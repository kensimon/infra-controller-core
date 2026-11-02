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
//! Carry producing context with the latest report, without retaining an open span.

use std::fmt::Display;
use std::future::Future;
use std::sync::Arc;

use opentelemetry::Context;
use opentelemetry::trace::{SpanContext, TraceContextExt};
use tracing::Span;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use super::{HealthReport, HealthReportTarget};
use crate::HealthError;

pub(super) struct QueuedHealthReport {
    pub(super) report: Arc<HealthReport>,
    parent: SpanContext,
}

impl QueuedHealthReport {
    pub(super) fn new(report: Arc<HealthReport>) -> Self {
        Self {
            report,
            parent: Span::current().context().span().span_context().clone(),
        }
    }

    pub(super) async fn submit(
        &self,
        target: HealthReportTarget,
        target_id: &impl Display,
        future: impl Future<Output = Result<(), HealthError>>,
    ) -> Result<(), HealthError> {
        let span = tracing::info_span!(
            parent: None,
            "health_report_submit",
            report_target = ?target,
            report_target_id = %target_id,
            report_source = ?self.report.source,
            outcome = tracing::field::Empty,
            otel.status_code = tracing::field::Empty,
            logfmt.suppress = true,
        );
        let _ = span.set_parent(Context::new().with_remote_span_context(self.parent.clone()));
        crate::operation_tracing::operation(span, future, |result| {
            crate::operation_tracing::Outcome::from(result)
        })
        .await
    }
}
