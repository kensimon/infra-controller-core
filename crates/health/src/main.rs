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
#![cfg_attr(not(test), deny(dead_code_pub_in_binary))]

use carbide_health::{Config, HealthError};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

#[tokio::main]
async fn main() -> Result<(), HealthError> {
    let config_path = std::env::args().nth(1).map(std::path::PathBuf::from);
    let config = Config::load(config_path.as_deref()).map_err(HealthError::GenericError)?;

    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    // Counts events from startup; `run_service` registers the metric with the
    // framework registry its /metrics response exposes.
    let log_events = carbide_instrument::LogEventsMetric::new("nico-hardware-health");
    let (span_layer, tracing) = carbide_instrument::otlp_tracing::setup(
        carbide_instrument::otlp_tracing::Config::new("nico-hardware-health"),
    );
    tracing_subscriber::registry()
        .with(log_events.layer().with_filter(env_filter.clone()))
        // Collector logs may contain hardware payloads. Export operation spans
        // and the shared RPC client spans, without attaching log events to them.
        .with(
            span_layer.with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                metadata.is_span()
                    && metadata.module_path().is_some_and(|path| {
                        path.starts_with("carbide_health::") || path == "rpc::grpc_client_tracing"
                    })
            })),
        )
        .with(
            logfmt::layer()
                .with_event_fields([logfmt::EventField::with_default(
                    "component",
                    "nico-hardware-health",
                )])
                .with_filter(env_filter),
        )
        .init();
    tracing.report();

    tracing::info!(
        version = carbide_version::v!(build_version),
        config = ?config,
        "Started carbide-hw-health"
    );

    // SIGTERM and SIGINT cancel this token.
    let result = async {
        let cancel_token = carbide_utils::shutdown_handler::start().map_err(|e| {
            HealthError::GenericError(format!(
                "failed to install the shutdown signal handler: {e}"
            ))
        })?;

        carbide_health::run_service(config, cancel_token).await
    }
    .await;

    tracing::info!(
        version = carbide_version::v!(build_version),
        "Stopped carbide-hw-health"
    );

    tracing.shutdown().await;
    result
}
