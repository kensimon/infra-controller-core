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
//! Finite operation spans; a dropped in-flight future retains its cancellation outcome.

use std::future::Future;

use tracing::{Instrument, Span};

pub(crate) enum Outcome {
    Success,
    PartialSuccess,
    Error,
}

impl<T, E> From<&Result<T, E>> for Outcome {
    fn from(result: &Result<T, E>) -> Self {
        if result.is_ok() {
            Self::Success
        } else {
            Self::Error
        }
    }
}

/// Closes the span before returning, including on cancellation. Callers declare
/// `outcome` and `otel.status_code` as empty fields and record only safe metadata.
pub(crate) async fn operation<T>(
    span: Span,
    future: impl Future<Output = T>,
    outcome: impl FnOnce(&T) -> Outcome,
) -> T {
    let mut completion = Completion {
        span: span.clone(),
        outcome: None,
    };
    async {
        let result = future.await;
        completion.outcome = Some(outcome(&result));
        result
    }
    .instrument(span)
    .await
}

// Record each outcome once: the OTel SDK can retain repeated attributes rather
// than replacing them. The guard also handles a future dropped while pending.
struct Completion {
    span: Span,
    outcome: Option<Outcome>,
}

impl Drop for Completion {
    fn drop(&mut self) {
        let (outcome, otel_status_code) = match self.outcome.take() {
            Some(Outcome::Success) => ("success", "ok"),
            Some(Outcome::PartialSuccess) => ("partial_success", "error"),
            Some(Outcome::Error) => ("error", "error"),
            None => ("cancelled", "error"),
        };
        self.span.record("outcome", outcome);
        self.span.record("otel.status_code", otel_status_code);
    }
}

/// Keep URL credentials, paths, and query parameters out of span attributes.
pub(crate) fn endpoint_address(endpoint: &str) -> String {
    url::Url::parse(endpoint)
        .map(|url| url.origin().ascii_serialization())
        .unwrap_or_default()
}

#[cfg(test)]
pub(crate) mod tests {
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider, SpanData};
    use tracing::instrument::WithSubscriber;
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::SubscriberExt;

    pub(crate) struct SpanCapture {
        pub(crate) dispatch: tracing::Dispatch,
        provider: SdkTracerProvider,
        exporter: InMemorySpanExporter,
    }

    impl SpanCapture {
        pub(crate) fn new() -> Self {
            let exporter = InMemorySpanExporter::default();
            let provider = SdkTracerProvider::builder()
                .with_simple_exporter(exporter.clone())
                .build();
            let subscriber = tracing_subscriber::registry().with(
                tracing_opentelemetry::layer()
                    .with_tracer(provider.tracer("hardware-health-test"))
                    .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                        metadata.is_span()
                            && metadata.module_path().is_some_and(|path| {
                                path.starts_with("carbide_health::")
                                    || path == "rpc::grpc_client_tracing"
                            })
                    })),
            );
            Self {
                dispatch: tracing::Dispatch::new(subscriber),
                provider,
                exporter,
            }
        }

        pub(crate) fn spans(&self) -> Vec<SpanData> {
            self.provider.force_flush().unwrap();
            self.exporter.get_finished_spans().unwrap()
        }
    }

    pub(crate) fn attribute(span: &SpanData, key: &str) -> Option<String> {
        span.attributes
            .iter()
            .find(|attribute| attribute.key.as_str() == key)
            .map(|attribute| attribute.value.to_string())
    }

    #[tokio::test]
    async fn cancelling_an_operation_closes_its_span() {
        let capture = SpanCapture::new();
        async {
            let span = tracing::info_span!(
                parent: None, "cancelled_operation",
                outcome = tracing::field::Empty,
                otel.status_code = tracing::field::Empty,
            );
            let future = super::operation(span, std::future::pending::<()>(), |_| unreachable!());
            tokio::pin!(future);
            std::future::poll_fn(|cx| {
                assert!(future.as_mut().poll(cx).is_pending());
                std::task::Poll::Ready(())
            })
            .await;
            assert!(capture.spans().is_empty());
        }
        .with_subscriber(capture.dispatch.clone())
        .await;
        let spans = capture.spans();
        assert_eq!(spans.len(), 1);
        assert_eq!(
            attribute(&spans[0], "outcome").as_deref(),
            Some("cancelled")
        );
        assert!(matches!(
            spans[0].status,
            opentelemetry::trace::Status::Error { .. }
        ));
    }

    #[test]
    fn endpoint_attributes_omit_secrets() {
        carbide_test_support::value_scenarios!(super::endpoint_address:
            "URL components" {
                "https://user:password@collector.example:4317/private?token=secret#fragment"
                    => "https://collector.example:4317".to_string(),
                "https://[::1]:4317" => "https://[::1]:4317".to_string(),
                "invalid endpoint secret" => String::new(),
            }
        );
    }
}
