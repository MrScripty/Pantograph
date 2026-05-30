#[cfg(feature = "standalone")]
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_dependency_planning::{
    DependencyInventoryObservationFreshness, DependencyProviderSourceState,
    RuntimeFeatureProviderSourceRow, RuntimeFeatureProviderSourceSnapshot,
};

#[async_trait]
pub(crate) trait RuntimeFeatureProviderSource: Send + Sync {
    async fn snapshot(&self) -> Result<RuntimeFeatureProviderSourceSnapshot, String>;
}

#[cfg(feature = "standalone")]
pub(crate) struct GatewayRuntimeFeatureProviderSource {
    gateway: Arc<inference::InferenceGateway>,
}

#[cfg(feature = "standalone")]
impl GatewayRuntimeFeatureProviderSource {
    #[must_use]
    pub(crate) fn new(gateway: Arc<inference::InferenceGateway>) -> Self {
        Self { gateway }
    }
}

#[cfg(feature = "standalone")]
#[async_trait]
impl RuntimeFeatureProviderSource for GatewayRuntimeFeatureProviderSource {
    async fn snapshot(&self) -> Result<RuntimeFeatureProviderSourceSnapshot, String> {
        let selected_backend_key = pantograph_runtime_identity::canonical_runtime_backend_key(
            &self.gateway.current_backend_name().await,
        );
        let mut backends = self.gateway.available_backends();
        let current_backend = self.gateway.current_backend_info().await;
        if let Some(existing) = backends.iter_mut().find(|backend| {
            pantograph_runtime_identity::canonical_runtime_backend_key(&backend.backend_key)
                == selected_backend_key
        }) {
            *existing = current_backend;
        } else {
            backends.push(current_backend);
        }
        Ok(runtime_feature_source_snapshot_from_backends(&backends))
    }
}

pub(crate) fn runtime_feature_source_snapshot_from_backends(
    backends: &[inference::BackendInfo],
) -> RuntimeFeatureProviderSourceSnapshot {
    let mut rows = Vec::new();
    for backend in backends {
        let runtime_id = backend.backend_key.clone();
        let facts = &backend.capabilities.facts;
        rows.extend([
            feature_row(
                &runtime_id,
                pantograph_dependency_planning::RUNTIME_FEATURE_STREAMING,
                feature_state(backend, facts.features.streaming),
            ),
            feature_row(
                &runtime_id,
                pantograph_dependency_planning::RUNTIME_FEATURE_DEVICE_SELECTION,
                feature_state(backend, facts.features.device_selection),
            ),
            feature_row(
                &runtime_id,
                pantograph_dependency_planning::RUNTIME_FEATURE_EXTERNAL_CONNECTION,
                feature_state(backend, facts.features.external_connection),
            ),
            feature_row(
                &runtime_id,
                pantograph_dependency_planning::RUNTIME_FEATURE_KV_CACHE,
                feature_state(backend, facts.features.kv_cache),
            ),
            feature_row(
                &runtime_id,
                pantograph_dependency_planning::RUNTIME_FEATURE_CUSTOM_CODE,
                feature_state(backend, facts.model_sources.custom_code),
            ),
            feature_row(
                &runtime_id,
                pantograph_dependency_planning::RUNTIME_FEATURE_PREPROCESSING,
                component_state(backend, facts.preprocessing),
            ),
            feature_row(
                &runtime_id,
                pantograph_dependency_planning::RUNTIME_FEATURE_POSTPROCESSING,
                component_state(backend, facts.postprocessing),
            ),
            feature_row(
                &runtime_id,
                pantograph_dependency_planning::RUNTIME_FEATURE_REQUEST_LIFECYCLE,
                if facts.request_lifecycle_facts().phases.is_empty() {
                    DependencyProviderSourceState::Unknown
                } else if backend.available {
                    DependencyProviderSourceState::Ready
                } else {
                    DependencyProviderSourceState::Unavailable
                },
            ),
        ]);
    }
    RuntimeFeatureProviderSourceSnapshot {
        contract_version: 1,
        rows,
        diagnostics: Vec::new(),
    }
}

fn feature_state(
    backend: &inference::BackendInfo,
    support: inference::BackendFeatureSupport,
) -> DependencyProviderSourceState {
    match (backend.available, support) {
        (_, inference::BackendFeatureSupport::Unsupported) => {
            DependencyProviderSourceState::Unsupported
        }
        (true, inference::BackendFeatureSupport::Supported) => DependencyProviderSourceState::Ready,
        (false, inference::BackendFeatureSupport::Supported) => {
            DependencyProviderSourceState::Unavailable
        }
        (_, inference::BackendFeatureSupport::Unknown) => DependencyProviderSourceState::Unknown,
    }
}

fn component_state(
    backend: &inference::BackendInfo,
    support: inference::BackendComponentCapability,
) -> DependencyProviderSourceState {
    match (backend.available, support) {
        (_, inference::BackendComponentCapability::Unsupported) => {
            DependencyProviderSourceState::Unsupported
        }
        (_, inference::BackendComponentCapability::Unknown) => {
            DependencyProviderSourceState::Unknown
        }
        (true, _) => DependencyProviderSourceState::Ready,
        (false, _) => DependencyProviderSourceState::Unavailable,
    }
}

fn feature_row(
    runtime_id: &str,
    feature_id: &str,
    state: DependencyProviderSourceState,
) -> RuntimeFeatureProviderSourceRow {
    RuntimeFeatureProviderSourceRow {
        runtime_id: pantograph_dependency_planning::RuntimeSourceId::parse(runtime_id)
            .expect("backend runtime ids must be valid source ids"),
        feature_id: pantograph_dependency_planning::RuntimeFeatureSourceId::parse(feature_id)
            .expect("canonical runtime feature ids must be valid source ids"),
        runtime_variant_id: None,
        state,
        freshness: DependencyInventoryObservationFreshness::Fresh,
        checked_at_ms: None,
        diagnostics: Vec::new(),
        alternatives: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_feature_facts_project_to_provider_source_snapshot() {
        let snapshot = runtime_feature_source_snapshot_from_backends(&[inference::BackendInfo {
            name: "PyTorch".to_string(),
            backend_key: "pytorch".to_string(),
            description: "PyTorch backend".to_string(),
            capabilities: inference::BackendCapabilities {
                streaming: true,
                facts: inference::BackendCapabilityFacts {
                    features: inference::BackendFeatureCapabilityFacts {
                        streaming: inference::BackendFeatureSupport::Supported,
                        device_selection: inference::BackendFeatureSupport::Supported,
                        external_connection: inference::BackendFeatureSupport::Unsupported,
                        kv_cache: inference::BackendFeatureSupport::Unknown,
                    },
                    model_sources: inference::BackendModelSourceCapabilityFacts {
                        custom_code: inference::BackendFeatureSupport::Unsupported,
                        ..inference::BackendModelSourceCapabilityFacts::default()
                    },
                    preprocessing: inference::BackendComponentCapability::RequiresPackageComponent,
                    postprocessing: inference::BackendComponentCapability::BackendManaged,
                    ..inference::BackendCapabilityFacts::default()
                },
                ..inference::BackendCapabilities::default()
            },
            default_start_mode: inference::backend::BackendDefaultStartMode::Inference,
            active: true,
            available: true,
            unavailable_reason: None,
            can_install: false,
            runtime_binary_id: None,
        }]);

        let streaming = snapshot
            .rows
            .iter()
            .find(|row| row.feature_id.as_str() == "streaming")
            .expect("streaming feature row");
        assert_eq!(streaming.runtime_id.as_str(), "pytorch");
        assert_eq!(streaming.state, DependencyProviderSourceState::Ready);
        let external_connection = snapshot
            .rows
            .iter()
            .find(|row| row.feature_id.as_str() == "external_connection")
            .expect("external connection feature row");
        assert_eq!(
            external_connection.state,
            DependencyProviderSourceState::Unsupported
        );
    }
}
