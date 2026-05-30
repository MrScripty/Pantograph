use std::sync::Arc;
use std::time::Duration;

use pantograph_dependency_environment_service::{
    DependencyEnvironmentReadinessSnapshot, DependencyEnvironmentReadinessSnapshotProvider,
    DependencyReadinessWorkQueue,
};

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

/// Embedded-runtime owner for future async dependency-readiness snapshot probes.
///
/// The current lifecycle is intentionally no-probe: it owns task startup and
/// shutdown without publishing fabricated snapshots. Real package/runtime
/// probes must be added behind this lifecycle in a later slice.
#[derive(Debug, Clone)]
pub struct EmbeddedDependencyReadinessSnapshotProducer {
    snapshot_provider: Arc<DependencyEnvironmentReadinessSnapshotProvider>,
    work_queue: Arc<DependencyReadinessWorkQueue>,
    config: EmbeddedDependencyReadinessSnapshotProducerConfig,
}

impl EmbeddedDependencyReadinessSnapshotProducer {
    #[must_use]
    pub fn new(
        snapshot_provider: Arc<DependencyEnvironmentReadinessSnapshotProvider>,
        work_queue: Arc<DependencyReadinessWorkQueue>,
    ) -> Self {
        Self {
            snapshot_provider,
            work_queue,
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
                            match DependencyEnvironmentReadinessSnapshot::unavailable_for_work_item(&item)
                                .and_then(|snapshot| snapshot_provider.insert_snapshot(snapshot))
                            {
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use pantograph_dependency_environment_service::{
        DependencyEnvironmentProvider, DependencyEnvironmentReadinessSnapshotProvider,
        DependencyReadinessTaskId, DependencyReadinessWorkItem,
        DependencyReadinessWorkItemProvenance, DependencyReadinessWorkQueue,
        DependencyReadinessWorkflowRunId, DependencyReadinessWorkflowSessionId,
    };
    use pantograph_dependency_planning::{
        DependencyEnvironmentReadinessState, ValidatedDependencyEnvironmentRequest,
    };

    use super::{
        EmbeddedDependencyReadinessSnapshotProducer,
        EmbeddedDependencyReadinessSnapshotProducerConfig,
    };

    #[tokio::test]
    async fn producer_lifecycle_shutdown_is_idempotent_and_does_not_publish_snapshots() {
        let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
        let work_queue = Arc::new(DependencyReadinessWorkQueue::new());
        let producer = EmbeddedDependencyReadinessSnapshotProducer::new(
            snapshot_provider.clone(),
            work_queue.clone(),
        )
        .with_config(EmbeddedDependencyReadinessSnapshotProducerConfig {
            poll_interval: Duration::from_millis(5),
        });
        let handle = producer
            .spawn(tokio::runtime::Handle::current())
            .expect("producer should spawn");

        tokio::time::sleep(Duration::from_millis(15)).await;
        assert_eq!(snapshot_provider.snapshot_count(), 0);

        handle.shutdown().await;
        handle.shutdown().await;
        assert_eq!(snapshot_provider.snapshot_count(), 0);
        assert!(work_queue.is_empty());
    }

    #[tokio::test]
    async fn producer_drains_work_queue_into_unavailable_snapshots() {
        let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
        let work_queue = Arc::new(DependencyReadinessWorkQueue::new());
        let request = validated_request();
        work_queue.enqueue(work_item(request.clone()));
        let producer = EmbeddedDependencyReadinessSnapshotProducer::new(
            snapshot_provider.clone(),
            work_queue.clone(),
        )
        .with_config(EmbeddedDependencyReadinessSnapshotProducerConfig {
            poll_interval: Duration::from_millis(5),
        });
        let handle = producer
            .spawn(tokio::runtime::Handle::current())
            .expect("producer should spawn");

        tokio::time::sleep(Duration::from_millis(20)).await;

        assert!(work_queue.is_empty());
        assert_eq!(snapshot_provider.snapshot_count(), 1);
        assert_eq!(
            snapshot_provider.resolve(&request).readiness_state,
            DependencyEnvironmentReadinessState::Unavailable
        );
        handle.shutdown().await;
    }

    #[test]
    fn producer_rejects_zero_poll_interval() {
        let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
        let work_queue = Arc::new(DependencyReadinessWorkQueue::new());
        let producer =
            EmbeddedDependencyReadinessSnapshotProducer::new(snapshot_provider, work_queue)
                .with_config(EmbeddedDependencyReadinessSnapshotProducerConfig {
                    poll_interval: Duration::ZERO,
                });
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let error = producer
            .spawn(runtime.handle().clone())
            .expect_err("zero interval should be rejected");

        assert!(error.to_string().contains("poll interval"));
    }

    fn work_item(request: ValidatedDependencyEnvironmentRequest) -> DependencyReadinessWorkItem {
        DependencyReadinessWorkItem::new(
            DependencyReadinessWorkItemProvenance::new(
                DependencyReadinessWorkflowSessionId::parse("session.001").expect("session id"),
                DependencyReadinessWorkflowRunId::parse("run.001").expect("run id"),
                DependencyReadinessTaskId::parse("infer").expect("task id"),
            ),
            request,
        )
    }

    fn validated_request() -> ValidatedDependencyEnvironmentRequest {
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../pantograph-dependency-planning/tests/fixtures/dependency_environment_resolve_request.json"
        ))
        .expect("request fixture should parse");
        ValidatedDependencyEnvironmentRequest::try_from(value)
            .expect("request fixture should validate")
    }
}
