/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
//! Exercise the real binary subscriber and its shutdown flush against an OTLP receiver.
#![cfg(unix)]

use std::time::Duration;

use opentelemetry_proto::tonic::collector::trace::v1::trace_service_server::{
    TraceService, TraceServiceServer,
};
use opentelemetry_proto::tonic::collector::trace::v1::{
    ExportTraceServiceRequest, ExportTraceServiceResponse,
};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::sync::CancellationToken;

struct Receiver(tokio::sync::mpsc::UnboundedSender<ExportTraceServiceRequest>);

#[tonic::async_trait]
impl TraceService for Receiver {
    async fn export(
        &self,
        request: tonic::Request<ExportTraceServiceRequest>,
    ) -> Result<tonic::Response<ExportTraceServiceResponse>, tonic::Status> {
        self.0.send(request.into_inner()).unwrap();
        Ok(tonic::Response::new(ExportTraceServiceResponse::default()))
    }
}

#[tokio::test]
async fn binary_exports_spans_with_logs_off_and_flushes_on_sigterm()
-> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let otlp_address = listener.local_addr()?;
    let cancel = CancellationToken::new();
    let shutdown = cancel.clone();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(TraceServiceServer::new(Receiver(tx)))
            .serve_with_incoming_shutdown(
                TcpListenerStream::new(listener),
                shutdown.cancelled_owned(),
            )
            .await
    });

    // Reachability collection only needs an open TCP port. The HTTP response
    // deliberately fails Redfish discovery; no physical BMC is needed.
    let bmc_listener = TcpListener::bind("127.0.0.1:0").await?;
    let bmc_port = bmc_listener.local_addr()?.port();
    let bmc_shutdown = cancel.clone();
    let bmc = tokio::spawn(async move {
        axum::serve(
            bmc_listener,
            axum::Router::new().fallback(|| async { http::StatusCode::SERVICE_UNAVAILABLE }),
        )
        .with_graceful_shutdown(bmc_shutdown.cancelled_owned())
        .await
    });
    // Empty successful inventory responses let the real discovery loop exercise
    // the shared gRPC client without requiring a full NICo API deployment.
    let api_listener = TcpListener::bind("127.0.0.1:0").await?;
    let api_address = api_listener.local_addr()?;
    let api_shutdown = cancel.clone();
    let (api_tx, mut api_rx) = tokio::sync::mpsc::unbounded_channel();
    let api = tokio::spawn(async move {
        axum::serve(
            api_listener,
            axum::Router::new().fallback(move |request: axum::extract::Request| {
                let api_tx = api_tx.clone();
                async move {
                    api_tx
                        .send(
                            request
                                .headers()
                                .get("traceparent")
                                .and_then(|value| value.to_str().ok())
                                .map(str::to_string),
                        )
                        .unwrap();
                    http::Response::builder()
                        .header("content-type", "application/grpc")
                        .header("grpc-status", "0")
                        .body(axum::body::Body::from(vec![0_u8; 5]))
                        .unwrap()
                }
            }),
        )
        .with_graceful_shutdown(api_shutdown.cancelled_owned())
        .await
    });
    let metrics_listener = TcpListener::bind("127.0.0.1:0").await?;
    let metrics_address = metrics_listener.local_addr()?;
    drop(metrics_listener);
    let dir = tempfile::tempdir()?;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = std::fs::read_to_string(root.join("crates/health/example/config.bmc-mock.toml"))?
        .replace("port = 1266", &format!("port = {bmc_port}"));
    let config_path = dir.path().join("health.toml");
    std::fs::write(
        &config_path,
        format!(
            "{config}\n[metrics]\nendpoint = \"{metrics_address}\"\n[collectors.reachability]\nenabled = true\ninterval = \"60s\"\nlog_mode = \"all\"\n"
        ),
    )?;

    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_forge-hw-health"))
        .arg(config_path)
        .current_dir(root)
        .env("RUST_LOG", "off")
        .env(
            "CARBIDE_HEALTH__ENDPOINT_SOURCES__NICO_API__ENABLED",
            "true",
        )
        .env(
            "CARBIDE_HEALTH__ENDPOINT_SOURCES__NICO_API__API_URL",
            format!("http://{api_address}"),
        )
        .env(
            "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
            format!("http://{otlp_address}"),
        )
        .env("OTEL_TRACES_SAMPLER", "always_on")
        .env("NICO_TRACES_SPAN_LEVEL", "info")
        .env("OTEL_BSP_SCHEDULE_DELAY", "600000")
        .env("OTEL_BSP_MAX_EXPORT_BATCH_SIZE", "2048")
        .kill_on_drop(true)
        .spawn()?;

    let client = reqwest::Client::new();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                panic!("health exited before collection: {status}");
            }
            if let Ok(response) = client
                .get(format!("http://{metrics_address}/telemetry"))
                .send()
                .await
                && response.text().await.unwrap().contains("tcp_port")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await?;
    assert!(rx.try_recv().is_err(), "the batch timer has not fired");
    let status = tokio::process::Command::new("kill")
        .args(["-TERM", &child.id().unwrap().to_string()])
        .status()
        .await?;
    assert!(status.success());
    let status = tokio::time::timeout(Duration::from_secs(15), child.wait()).await??;
    assert!(status.success(), "{status}");

    let request = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await?
        .unwrap();
    let resource = request.resource_spans.iter().find(|resource| {
        resource.resource.as_ref().is_some_and(|resource| resource.attributes.iter().any(|attribute| {
            attribute.key == "service.name"
                && attribute.value.as_ref().is_some_and(|value| matches!(&value.value,
                    Some(opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(value))
                        if value == "nico-hardware-health"))
        }))
    }).expect("hardware-health service resource");
    let spans: Vec<_> = resource
        .scope_spans
        .iter()
        .flat_map(|scope| &scope.spans)
        .collect();
    assert!(spans.iter().any(|span| span.name == "health_collection"));
    assert!(spans.iter().any(|span| span.name == "health_discovery"));
    let rpc = spans
        .iter()
        .find(|span| span.name == "grpc_client_request")
        .expect("shared client spans are exported with RUST_LOG=off");
    let discovery = spans
        .iter()
        .find(|span| span.name == "health_discovery" && span.trace_id == rpc.trace_id)
        .unwrap();
    assert_eq!(rpc.parent_span_id, discovery.span_id);
    let expected = format!(
        "00-{}-{}-01",
        rpc.trace_id
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        rpc.span_id
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let mut parents = Vec::new();
    while let Ok(parent) = api_rx.try_recv() {
        parents.push(parent);
    }
    assert!(
        parents.contains(&Some(expected)),
        "the exported client context reaches the API"
    );
    assert!(
        spans
            .iter()
            .all(|span| span.end_time_unix_nano >= span.start_time_unix_nano)
    );
    assert!(
        spans.iter().all(|span| span.events.is_empty()),
        "hardware log payloads stay out of traces"
    );
    cancel.cancel();
    server.await??;
    bmc.await??;
    api.await??;
    Ok(())
}
