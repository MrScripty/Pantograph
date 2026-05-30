use std::sync::Arc;
use std::time::Duration;

use pantograph_dependency_environment_service::{
    resolve_dependency_requirements_payload, DependencyEnvironmentReadinessSnapshot,
    DependencyEnvironmentReadinessSnapshotProvider, DependencyReadinessWorkQueue,
    DependencyRequirementsRegistry,
};

use crate::dependency_inventory::DependencyInventoryService;
use crate::EmbeddedRuntimeError;

/// Configuration for the embedded dependency-readiness snapshot producer loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedDependencyReadinessSnapshotProducerConfig {
    pub poll_interval: Duration,
}

impl Default for EmbeddedDependencyReadinessSnapshotProducerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(60),
        }
    }
}

/// Embedded-runtime owner for async dependency-readiness snapshot probes.
#[derive(Clone)]
pub struct EmbeddedDependencyReadinessSnapshotProducer {
    snapshot_provider: Arc<DependencyEnvironmentReadinessSnapshotProvider>,
    work_queue: Arc<DependencyReadinessWorkQueue>,
    requirements_registry: Arc<dyn DependencyRequirementsRegistry>,
    dependency_inventory: Arc<DependencyInventoryService>,
    config: EmbeddedDependencyReadinessSnapshotProducerConfig,
}

impl EmbeddedDependencyReadinessSnapshotProducer {
    #[must_use]
    pub fn new(
        snapshot_provider: Arc<DependencyEnvironmentReadinessSnapshotProvider>,
        work_queue: Arc<DependencyReadinessWorkQueue>,
        requirements_registry: Arc<dyn DependencyRequirementsRegistry>,
    ) -> Self {
        Self {
            snapshot_provider,
            work_queue,
            requirements_registry,
            dependency_inventory: Arc::new(DependencyInventoryService::default()),
            config: EmbeddedDependencyReadinessSnapshotProducerConfig::default(),
        }
    }

    #[must_use]
    pub fn with_config(
        mut self,
        config: EmbeddedDependencyReadinessSnapshotProducerConfig,
    ) -> Self {
        self.config = config;
        self
    }

    #[must_use]
    #[cfg(any(test, feature = "standalone"))]
    pub(crate) fn with_dependency_inventory(
        mut self,
        dependency_inventory: Arc<DependencyInventoryService>,
    ) -> Self {
        self.dependency_inventory = dependency_inventory;
        self
    }

    pub fn spawn(
        self,
        runtime_handle: tokio::runtime::Handle,
    ) -> Result<EmbeddedDependencyReadinessSnapshotProducerHandle, EmbeddedRuntimeError> {
        if self.config.poll_interval.is_zero() {
            return Err(EmbeddedRuntimeError::Config {
                message:
                    "dependency-readiness snapshot producer poll interval must be greater than zero"
                        .to_string(),
            });
        }

        let snapshot_provider = self.snapshot_provider;
        let work_queue = self.work_queue;
        let requirements_registry = self.requirements_registry;
        let dependency_inventory = self.dependency_inventory;
        let poll_interval = self.config.poll_interval;
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
        let join_handle = runtime_handle.spawn(async move {
            let mut interval = tokio::time::interval(poll_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    _ = interval.tick() => {
                        while let Some(item) = work_queue.pop_next() {
                            let snapshot = match resolve_dependency_requirements_payload(
                                requirements_registry.as_ref(),
                                &item.request,
                            ) {
                                Ok(payload) => {
                                    dependency_inventory
                                        .snapshot_for_work_item(&item, payload)
                                    .await
                                }
                                Err(error) => {
                                    DependencyEnvironmentReadinessSnapshot::unavailable_for_work_item_registry_error(
                                        &item,
                                        &error,
                                    )
                                }
                            };
                            match snapshot.and_then(|snapshot| snapshot_provider.insert_snapshot(snapshot)) {
                                Ok(()) => {}
                                Err(error) => {
                                    log::error!(
                                        "dependency-readiness snapshot producer failed to publish queued unavailable snapshot: {error}"
                                    );
                                }
                            }
                        }
                        log::trace!(
                            "dependency-readiness snapshot producer heartbeat: {} snapshots available, {} work items queued",
                            snapshot_provider.snapshot_count(),
                            work_queue.len()
                        );
                    }
                }
            }
        });

        Ok(EmbeddedDependencyReadinessSnapshotProducerHandle::new(
            shutdown_tx,
            join_handle,
        ))
    }
}

/// Handle for a tracked dependency-readiness snapshot producer task.
#[derive(Debug)]
pub struct EmbeddedDependencyReadinessSnapshotProducerHandle {
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    join_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl EmbeddedDependencyReadinessSnapshotProducerHandle {
    fn new(
        shutdown_tx: tokio::sync::watch::Sender<bool>,
        join_handle: tokio::task::JoinHandle<()>,
    ) -> Self {
        Self {
            shutdown_tx,
            join_handle: tokio::sync::Mutex::new(Some(join_handle)),
        }
    }

    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        if let Some(join_handle) = self.join_handle.lock().await.take() {
            match join_handle.await {
                Ok(()) => {}
                Err(error) if error.is_panic() => {
                    log::error!(
                        "dependency-readiness snapshot producer panicked during shutdown: {error}"
                    );
                }
                Err(error) => {
                    log::warn!(
                        "dependency-readiness snapshot producer was cancelled during shutdown: {error}"
                    );
                }
            }
        }
    }
}
