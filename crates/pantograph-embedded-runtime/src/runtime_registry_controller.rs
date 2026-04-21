use async_trait::async_trait;

use crate::{HostRuntimeModeSnapshot, runtime_registry};

#[async_trait]
impl runtime_registry::HostRuntimeRegistryController for inference::InferenceGateway {
    async fn mode_info_snapshot(&self) -> HostRuntimeModeSnapshot {
        HostRuntimeModeSnapshot::from_mode_info(&self.mode_info().await)
    }

    async fn stop_runtime_producer(&self, producer: runtime_registry::HostRuntimeProducer) {
        match producer {
            runtime_registry::HostRuntimeProducer::Active => self.stop().await,
            runtime_registry::HostRuntimeProducer::Embedding => {
                debug_assert!(
                    false,
                    "embedded inference gateway cannot stop a dedicated embedding producer"
                );
            }
        }
    }
}

#[async_trait]
impl runtime_registry::HostRuntimeRegistryLifecycleController for inference::InferenceGateway {
    async fn stop_all_runtime_producers(&self) {
        self.stop().await;
    }

    async fn restore_runtime(
        &self,
        restore_config: Option<inference::BackendConfig>,
    ) -> Result<(), inference::GatewayError> {
        self.restore_inference_runtime(restore_config).await
    }
}
