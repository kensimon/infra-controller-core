/*
 * SPDX-FileCopyrightText: Copyright (c) 2021-2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: LicenseRef-NvidiaProprietary
 *
 * NVIDIA CORPORATION, its affiliates and licensors retain all intellectual
 * property and proprietary rights in and to this material, related
 * documentation and any modifications thereto. Any use, reproduction,
 * disclosure or distribution of this material and related documentation
 * without an express license agreement from NVIDIA CORPORATION or
 * its affiliates is strictly prohibited.
 */

//! Watcher for DPU resource events.
//!
//! Uses the repository `watch()` trait method to receive DPU events.
//! The repository implementation handles retries and requeuing when
//! handlers return `Err`.
//!
//! Callbacks may fire on any update to a DPU resource, not only on
//! phase transitions. All handlers must be idempotent.
//!
//! ## Example
//!
//! ```ignore
//! let watcher = DpuWatcherBuilder::new(repo, "dpf-operator-system")
//!     .on_dpu_event(|event| async move {
//!         println!("Phase: {:?}", event.phase);
//!         Ok(())
//!     })
//!     .on_reboot_required(|event| async move {
//!         enqueue_host_reboot(&event.host_bmc_ip).await?;
//!         Ok(())
//!     })
//!     .start();
//! ```

use std::sync::Arc;

use futures::FutureExt;
use futures::future::BoxFuture;

use crate::crds::dpus_generated::{DPU, DpuStatusPhase};
use crate::error::DpfError;
use crate::repository::DpuRepository;
use crate::types::{
    DpuErrorEvent, DpuEvent, DpuPhase, DpuReadyEvent, MaintenanceEvent, RebootRequiredEvent,
};

type DpuCallbackFn<T> = Box<dyn Fn(T) -> BoxFuture<'static, Result<(), DpfError>> + Send + Sync>;

struct Callbacks {
    dpu_event: DpuCallbackFn<DpuEvent>,
    reboot: DpuCallbackFn<RebootRequiredEvent>,
    ready: DpuCallbackFn<DpuReadyEvent>,
    maintenance: DpuCallbackFn<MaintenanceEvent>,
    error: DpuCallbackFn<DpuErrorEvent>,
}

/// The watcher only cares about how the events are translated into the callbacks,
/// not the actual event gathering. The repository implementation handles procuring
/// the events, as well as retries and requeuing when handlers return `Err`.
pub struct DpuWatcher {
    watcher_task: tokio::task::JoinHandle<()>,
}

/// The watcher continues running until this struct is dropped.
impl Drop for DpuWatcher {
    fn drop(&mut self) {
        self.watcher_task.abort();
    }
}

/// Builder for creating a DPU watcher.
pub struct DpuWatcherBuilder<R: DpuRepository> {
    repo: Arc<R>,
    namespace: String,
    cbs: Callbacks,
}

impl<R: DpuRepository> DpuWatcherBuilder<R> {
    pub fn new(repo: Arc<R>, namespace: impl Into<String>) -> Self {
        Self {
            repo,
            namespace: namespace.into(),
            cbs: Callbacks {
                dpu_event: Box::new(|_| std::future::ready(Ok(())).boxed()),
                reboot: Box::new(|_| std::future::ready(Ok(())).boxed()),
                ready: Box::new(|_| std::future::ready(Ok(())).boxed()),
                maintenance: Box::new(|_| std::future::ready(Ok(())).boxed()),
                error: Box::new(|_| std::future::ready(Ok(())).boxed()),
            },
        }
    }
}

/// This is a type state builder pattern. It's extra boilerplate, but we get generic
/// function types for the callbacks instead of boxing and pinning the closures.
impl<R: DpuRepository> DpuWatcherBuilder<R> {
    /// Register a callback for DPU events.
    ///
    /// The callback is invoked on every observed update to a DPU, not only
    /// on phase transitions. The handler must be idempotent.
    pub fn on_dpu_event<F, Fut>(self, callback: F) -> DpuWatcherBuilder<R>
    where
        F: Fn(DpuEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), DpfError>> + Send + 'static,
    {
        DpuWatcherBuilder {
            repo: self.repo,
            namespace: self.namespace,
            cbs: Callbacks {
                dpu_event: Box::new(move |event| callback(event).boxed()),
                reboot: self.cbs.reboot,
                ready: self.cbs.ready,
                maintenance: self.cbs.maintenance,
                error: self.cbs.error,
            },
        }
    }

    /// Register a callback for when a host reboot is required.
    ///
    /// Invoked on every update where the DPU is in the Rebooting phase, not
    /// only on transitions into that phase. The handler must be idempotent.
    ///
    /// Return `Ok(())` to acknowledge the event. Return `Err` to have the
    /// repository implementation retry after a backoff period.
    pub fn on_reboot_required<F, Fut>(self, callback: F) -> DpuWatcherBuilder<R>
    where
        F: Fn(RebootRequiredEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), DpfError>> + Send + 'static,
    {
        DpuWatcherBuilder {
            repo: self.repo,
            namespace: self.namespace,
            cbs: Callbacks {
                dpu_event: self.cbs.dpu_event,
                reboot: Box::new(move |event| callback(event).boxed()),
                ready: self.cbs.ready,
                maintenance: self.cbs.maintenance,
                error: self.cbs.error,
            },
        }
    }

    /// Register a callback for when a DPU is in the Ready phase.
    ///
    /// Invoked on every update where the DPU is in the Ready phase, not
    /// only on transitions into that phase. The handler must be idempotent.
    pub fn on_dpu_ready<F, Fut>(self, callback: F) -> DpuWatcherBuilder<R>
    where
        F: Fn(DpuReadyEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), DpfError>> + Send + 'static,
    {
        DpuWatcherBuilder {
            repo: self.repo,
            namespace: self.namespace,
            cbs: Callbacks {
                dpu_event: self.cbs.dpu_event,
                reboot: self.cbs.reboot,
                ready: Box::new(move |event| callback(event).boxed()),
                maintenance: self.cbs.maintenance,
                error: self.cbs.error,
            },
        }
    }

    /// Register a callback for when the DPU is in the NodeEffect phase.
    ///
    /// Invoked on every update where the DPU is in the NodeEffect phase, not
    /// only on transitions into that phase. The handler must be idempotent.
    ///
    /// Return `Ok(())` to acknowledge the event. Return `Err` to have the
    /// repository implementation retry after a backoff period.
    pub fn on_maintenance_needed<F, Fut>(self, callback: F) -> DpuWatcherBuilder<R>
    where
        F: Fn(MaintenanceEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), DpfError>> + Send + 'static,
    {
        DpuWatcherBuilder {
            repo: self.repo,
            namespace: self.namespace,
            cbs: Callbacks {
                dpu_event: self.cbs.dpu_event,
                reboot: self.cbs.reboot,
                ready: self.cbs.ready,
                maintenance: Box::new(move |event| callback(event).boxed()),
                error: self.cbs.error,
            },
        }
    }

    /// Register a callback for when a DPU is in the Error phase.
    ///
    /// Invoked on every update where the DPU is in the Error phase, not
    /// only on transitions into that phase. The handler must be idempotent.
    ///
    /// Return `Ok(())` to acknowledge the event. Return `Err` to have the
    /// repository implementation retry after a backoff period.
    pub fn on_error<F, Fut>(self, callback: F) -> DpuWatcherBuilder<R>
    where
        F: Fn(DpuErrorEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), DpfError>> + Send + 'static,
    {
        DpuWatcherBuilder {
            repo: self.repo,
            namespace: self.namespace,
            cbs: Callbacks {
                dpu_event: self.cbs.dpu_event,
                reboot: self.cbs.reboot,
                ready: self.cbs.ready,
                maintenance: self.cbs.maintenance,
                error: Box::new(move |event| callback(event).boxed()),
            },
        }
    }
}

impl<R> DpuWatcherBuilder<R>
where
    R: DpuRepository,
{
    /// Start watching for events.
    ///
    /// Returns a handle that stops the watcher when dropped.
    pub fn start(self) -> DpuWatcher {
        let cbs = Arc::new(self.cbs);

        let handler = move |dpu: Arc<DPU>| {
            let cbs = cbs.clone();
            async move {
                let Some(status) = &dpu.status else {
                    return Ok(());
                };
                let Some(dpu_name) = &dpu.metadata.name else {
                    return Ok(());
                };

                let device_name = dpu.spec.dpu_device_name.clone();
                let phase = DpuPhase::from(status.phase.clone());
                let node_name = dpu.spec.dpu_node_name.clone();

                (cbs.dpu_event)(DpuEvent {
                    dpu_name: dpu_name.clone(),
                    device_name: device_name.clone(),
                    node_name: node_name.clone(),
                    phase,
                })
                .await?;

                if matches!(status.phase, DpuStatusPhase::NodeEffect) {
                    (cbs.maintenance)(MaintenanceEvent {
                        dpu_name: dpu_name.clone(),
                        node_name: node_name.clone(),
                    })
                    .await?;
                }

                if matches!(status.phase, DpuStatusPhase::Ready) {
                    (cbs.ready)(DpuReadyEvent {
                        dpu_name: dpu_name.clone(),
                        device_name: device_name.clone(),
                        node_name: node_name.clone(),
                    })
                    .await?;
                }

                if matches!(status.phase, DpuStatusPhase::Error) {
                    (cbs.error)(DpuErrorEvent {
                        dpu_name: dpu_name.clone(),
                        device_name: device_name.clone(),
                        node_name: node_name.clone(),
                    })
                    .await?;
                }

                if matches!(status.phase, DpuStatusPhase::Rebooting) {
                    (cbs.reboot)(RebootRequiredEvent {
                        dpu_name: dpu_name.clone(),
                        node_name: node_name.clone(),
                        host_bmc_ip: dpu.spec.bmc_ip.clone().unwrap_or_default(),
                    })
                    .await?;
                }

                Ok(())
            }
        };

        DpuWatcher {
            watcher_task: tokio::spawn(self.repo.watch(&self.namespace, handler)),
        }
    }
}
