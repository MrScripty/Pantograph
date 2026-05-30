use std::sync::Arc;
use std::time::Duration;

use pantograph_dependency_environment_service::DependencyEnvironmentReadinessSnapshotProvider;

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
    config: EmbeddedDependencyReadinessSnapshotProducerConfig,
}

impl EmbeddedDependencyReadinessSnapshotProducer {
    #[must_use]
    pub fn new(snapshot_provider: Arc<DependencyEnvironmentReadinessSnapshotProvider>) -> Self {
        Self {
            snapshot_provider,
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
                        log::trace!(
                            "dependency-readiness snapshot producer heartbeat: {} snapshots available",
                            snapshot_provider.snapshot_count()
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

    use pantograph_dependency_environment_service::DependencyEnvironmentReadinessSnapshotProvider;

    use super::{
        EmbeddedDependencyReadinessSnapshotProducer,
        EmbeddedDependencyReadinessSnapshotProducerConfig,
    };

    #[tokio::test]
    async fn producer_lifecycle_shutdown_is_idempotent_and_does_not_publish_snapshots() {
        let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
        let producer = EmbeddedDependencyReadinessSnapshotProducer::new(snapshot_provider.clone())
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
    }

    #[test]
    fn producer_rejects_zero_poll_interval() {
        let snapshot_provider = Arc::new(DependencyEnvironmentReadinessSnapshotProvider::new());
        let producer = EmbeddedDependencyReadinessSnapshotProducer::new(snapshot_provider)
            .with_config(EmbeddedDependencyReadinessSnapshotProducerConfig {
                poll_interval: Duration::ZERO,
            });
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let error = producer
            .spawn(runtime.handle().clone())
            .expect_err("zero interval should be rejected");

        assert!(error.to_string().contains("poll interval"));
    }
}
