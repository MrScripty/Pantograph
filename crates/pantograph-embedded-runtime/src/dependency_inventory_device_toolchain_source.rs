#[cfg(feature = "standalone")]
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_dependency_planning::{
    DependencyInventoryObservationFreshness, DependencyProviderSourceAlternative,
    DependencyProviderSourceState, DeviceClassSourceId, DeviceToolchainProviderSourceRow,
    DeviceToolchainProviderSourceSnapshot, RuntimeSourceId,
};

#[async_trait]
pub(crate) trait DeviceToolchainProviderSource: Send + Sync {
    async fn snapshot(&self) -> Result<DeviceToolchainProviderSourceSnapshot, String>;
}

#[cfg(feature = "standalone")]
pub(crate) struct GatewayDeviceToolchainProviderSource {
    gateway: Arc<inference::InferenceGateway>,
}

#[cfg(feature = "standalone")]
impl GatewayDeviceToolchainProviderSource {
    #[must_use]
    pub(crate) fn new(gateway: Arc<inference::InferenceGateway>) -> Self {
        Self { gateway }
    }
}

#[cfg(feature = "standalone")]
#[async_trait]
impl DeviceToolchainProviderSource for GatewayDeviceToolchainProviderSource {
    async fn snapshot(&self) -> Result<DeviceToolchainProviderSourceSnapshot, String> {
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
        Ok(device_toolchain_source_snapshot_from_backends(&backends))
    }
}

pub(crate) fn device_toolchain_source_snapshot_from_backends(
    backends: &[inference::BackendInfo],
) -> DeviceToolchainProviderSourceSnapshot {
    let mut rows = Vec::new();
    for backend in backends {
        let runtime_id = RuntimeSourceId::parse(&backend.backend_key)
            .expect("backend runtime ids must be valid source ids");
        for variant in &backend.capabilities.facts.runtime_variants {
            let Some((toolchain_id, device_class)) = toolchain_and_class(variant.device_class)
            else {
                continue;
            };
            rows.push(DeviceToolchainProviderSourceRow {
                toolchain_id,
                runtime_id: Some(runtime_id.clone()),
                device_class: Some(device_class),
                device_id: None,
                state: if backend.available && variant.available {
                    DependencyProviderSourceState::Ready
                } else {
                    DependencyProviderSourceState::Unavailable
                },
                freshness: DependencyInventoryObservationFreshness::Fresh,
                checked_at_ms: None,
                diagnostics: Vec::new(),
                alternatives: Vec::new(),
            });
        }
    }
    let ready_alternatives = ready_alternatives_from_rows(&rows);
    for row in &mut rows {
        if row.state != DependencyProviderSourceState::Ready {
            row.alternatives = ready_alternatives
                .iter()
                .filter(|alternative| alternative.runtime_id == row.runtime_id)
                .take(8)
                .cloned()
                .collect();
        }
    }
    DeviceToolchainProviderSourceSnapshot {
        contract_version: 1,
        rows,
        diagnostics: Vec::new(),
    }
}

fn toolchain_and_class(
    device_class: inference::InferenceDeviceClass,
) -> Option<(
    pantograph_dependency_planning::DeviceToolchainSourceId,
    DeviceClassSourceId,
)> {
    let toolchain_id = match device_class {
        inference::InferenceDeviceClass::Cuda => {
            pantograph_dependency_planning::DEVICE_TOOLCHAIN_CUDA_RUNTIME
        }
        inference::InferenceDeviceClass::Metal => {
            pantograph_dependency_planning::DEVICE_TOOLCHAIN_METAL_RUNTIME
        }
        inference::InferenceDeviceClass::Mps => {
            pantograph_dependency_planning::DEVICE_TOOLCHAIN_MPS_RUNTIME
        }
        inference::InferenceDeviceClass::Cpu => return None,
        _ => return None,
    };
    Some((
        pantograph_dependency_planning::DeviceToolchainSourceId::parse(toolchain_id)
            .expect("canonical device toolchain ids must be valid source ids"),
        DeviceClassSourceId::parse(device_class.canonical_label())
            .expect("canonical device class ids must be valid source ids"),
    ))
}

fn ready_alternatives_from_rows(
    rows: &[DeviceToolchainProviderSourceRow],
) -> Vec<DependencyProviderSourceAlternative> {
    rows.iter()
        .filter(|row| row.state == DependencyProviderSourceState::Ready)
        .map(|row| DependencyProviderSourceAlternative {
            runtime_id: row.runtime_id.clone(),
            runtime_variant_id: None,
            feature_id: None,
            toolchain_id: Some(row.toolchain_id.clone()),
            device_class: row.device_class.clone(),
            device_id: row.device_id.clone(),
            system_package_id: None,
            package_manager_id: None,
            platform_id: None,
            reason: Some("Device toolchain is available on this runtime.".to_string()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_runtime_variants_project_toolchain_source_rows_with_alternatives() {
        let snapshot = device_toolchain_source_snapshot_from_backends(&[inference::BackendInfo {
            name: "PyTorch".to_string(),
            backend_key: "pytorch".to_string(),
            description: "PyTorch backend".to_string(),
            capabilities: inference::BackendCapabilities {
                facts: inference::BackendCapabilityFacts {
                    runtime_variants: vec![
                        inference::RuntimeVariantCapability {
                            runtime_variant_id: inference::RuntimeVariantId::parse("pytorch.cuda")
                                .expect("runtime variant id"),
                            device_class: inference::InferenceDeviceClass::Cuda,
                            available: true,
                            diagnostics: Vec::new(),
                        },
                        inference::RuntimeVariantCapability {
                            runtime_variant_id: inference::RuntimeVariantId::parse("pytorch.mps")
                                .expect("runtime variant id"),
                            device_class: inference::InferenceDeviceClass::Mps,
                            available: false,
                            diagnostics: Vec::new(),
                        },
                    ],
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

        let cuda = snapshot
            .rows
            .iter()
            .find(|row| row.toolchain_id.as_str() == "cuda_runtime")
            .expect("cuda row");
        assert_eq!(cuda.state, DependencyProviderSourceState::Ready);

        let mps = snapshot
            .rows
            .iter()
            .find(|row| row.toolchain_id.as_str() == "mps_runtime")
            .expect("mps row");
        assert_eq!(mps.state, DependencyProviderSourceState::Unavailable);
        assert_eq!(mps.alternatives.len(), 1);
        assert_eq!(
            mps.alternatives[0]
                .toolchain_id
                .as_ref()
                .map(|toolchain_id| toolchain_id.as_str()),
            Some("cuda_runtime")
        );
    }
}
