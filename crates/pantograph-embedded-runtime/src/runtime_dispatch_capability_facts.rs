use std::sync::Arc;

use inference::{gateway::RuntimeOwnedDeviceCandidate, InferenceGateway};
use pantograph_runtime_registry::{
    RuntimeRegistryRuntimeSnapshot, RuntimeRegistryStatus, SharedRuntimeRegistry,
};

#[derive(Clone)]
pub(crate) struct RuntimeDispatchCapabilityFactsSource {
    registry: SharedRuntimeRegistry,
    gateway: Option<Arc<InferenceGateway>>,
}

impl RuntimeDispatchCapabilityFactsSource {
    pub(crate) fn new(registry: SharedRuntimeRegistry) -> Self {
        Self {
            registry,
            gateway: None,
        }
    }

    pub(crate) fn with_gateway(mut self, gateway: Arc<InferenceGateway>) -> Self {
        crate::runtime_registry::register_scheduler_loadable_candle(&self.registry, &gateway);
        crate::runtime_registry::register_backend_runtimes(
            &self.registry,
            &gateway.available_backends(),
        );
        self.gateway = Some(gateway);
        self
    }

    /// Read the actual gateway owner's task declaration, independently of
    /// registry enrollment and package metadata.
    pub(crate) async fn supports_candle_cpu_embeddings(&self) -> bool {
        let Some(gateway) = self.gateway.as_ref() else {
            return false;
        };
        let selected = gateway.current_backend_info().await;
        let candle = if inference::backend::canonical_backend_key(&selected.backend_key) == "candle"
        {
            Some(selected)
        } else {
            gateway.available_backends().into_iter().find(|backend| {
                inference::backend::canonical_backend_key(&backend.backend_key) == "candle"
            })
        };
        candle.is_some_and(|backend| {
            backend.available
                && backend.capabilities.embeddings
                && backend
                    .capabilities
                    .facts
                    .supports_task(inference::InferenceTaskId::Embedding)
        })
    }

    pub(crate) async fn collect(&self) -> RuntimeDispatchCapabilityFactsOutcome {
        let (backends, mode_info) = if let Some(gateway) = &self.gateway {
            let mut backends = gateway.available_backends();
            let current = gateway.current_backend_info().await;
            backends.retain(|backend| backend.backend_key != current.backend_key);
            backends.push(current);
            (backends, Some(gateway.mode_info().await))
        } else {
            (Vec::new(), None)
        };
        let snapshot = self.registry.snapshot();
        let owned_devices = self
            .gateway
            .as_ref()
            .map(|gateway| gateway.runtime_owned_device_candidates())
            .unwrap_or_default();
        let mut diagnostics = Vec::new();
        if snapshot.runtimes.is_empty() {
            diagnostics.push(diagnostic(
                RuntimeDispatchCapabilityFactsDiagnosticCode::NoRegisteredRuntimes,
                "runtime registry has no registered runtimes for dispatch capability projection",
            ));
            return RuntimeDispatchCapabilityFactsOutcome::Unavailable { diagnostics };
        }

        let runtimes = snapshot
            .runtimes
            .into_iter()
            .filter_map(|runtime| {
                project_runtime(
                    runtime,
                    &backends,
                    mode_info.as_ref(),
                    &owned_devices,
                    &mut diagnostics,
                )
            })
            .collect::<Vec<_>>();
        if runtimes.is_empty() {
            return RuntimeDispatchCapabilityFactsOutcome::Unavailable { diagnostics };
        }

        RuntimeDispatchCapabilityFactsOutcome::Projected {
            facts: RuntimeDispatchCapabilityFactsProjection {
                generated_at_ms: snapshot.generated_at_ms,
                runtimes,
            },
            diagnostics,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeDispatchCapabilityFactsProjection {
    pub generated_at_ms: u64,
    pub runtimes: Vec<RuntimeDispatchRuntimeCapabilityFacts>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeDispatchRuntimeCapabilityFacts {
    pub runtime_id: String,
    pub backend_keys: Vec<String>,
    pub backend_capabilities: inference::BackendCapabilityFacts,
    pub device_ids: Vec<inference::InferenceDeviceId>,
    pub runtime_family: String,
    pub runtime_residency_key: String,
    pub status: RuntimeRegistryStatus,
    pub runtime_instance_id: Option<String>,
    pub loaded_model_ids: Vec<String>,
    pub active_reservation_ids: Vec<u64>,
    pub has_admission_budget: bool,
    /// Additive discovery facts; this CPU slice is not an inventory of GPUs.
    pub automatic_device_candidates: Vec<RuntimeOwnedDeviceCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeDispatchCapabilityFactsDiagnostic {
    pub code: RuntimeDispatchCapabilityFactsDiagnosticCode,
    pub runtime_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeDispatchCapabilityFactsDiagnosticCode {
    NoRegisteredRuntimes,
    RuntimeMissingBackendKeys,
    RuntimeMissingDispatchIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimeDispatchCapabilityFactsOutcome {
    Projected {
        facts: RuntimeDispatchCapabilityFactsProjection,
        diagnostics: Vec<RuntimeDispatchCapabilityFactsDiagnostic>,
    },
    Unavailable {
        diagnostics: Vec<RuntimeDispatchCapabilityFactsDiagnostic>,
    },
}

impl RuntimeDispatchCapabilityFactsOutcome {
    pub(crate) fn diagnostics(&self) -> &[RuntimeDispatchCapabilityFactsDiagnostic] {
        match self {
            Self::Projected { diagnostics, .. } | Self::Unavailable { diagnostics } => diagnostics,
        }
    }
}

fn project_runtime(
    runtime: RuntimeRegistryRuntimeSnapshot,
    backends: &[inference::BackendInfo],
    mode_info: Option<&inference::ServerModeInfo>,
    owned_devices: &[RuntimeOwnedDeviceCandidate],
    diagnostics: &mut Vec<RuntimeDispatchCapabilityFactsDiagnostic>,
) -> Option<RuntimeDispatchRuntimeCapabilityFacts> {
    if runtime.backend_keys.is_empty() {
        diagnostics.push(RuntimeDispatchCapabilityFactsDiagnostic {
            code: RuntimeDispatchCapabilityFactsDiagnosticCode::RuntimeMissingBackendKeys,
            runtime_id: Some(runtime.runtime_id),
            message:
                "runtime registry record has no backend keys for dispatch capability projection"
                    .to_string(),
        });
        return None;
    }

    let (Some(runtime_family), Some(runtime_residency_key)) =
        (runtime.runtime_family, runtime.runtime_residency_key)
    else {
        diagnostics.push(RuntimeDispatchCapabilityFactsDiagnostic {
            code: RuntimeDispatchCapabilityFactsDiagnosticCode::RuntimeMissingDispatchIdentity,
            runtime_id: Some(runtime.runtime_id),
            message: "runtime registry record has no dispatch identity for capability projection"
                .to_string(),
        });
        return None;
    };

    let automatic_device_candidates = owned_devices
        .iter()
        .filter(|device| {
            runtime
                .backend_keys
                .iter()
                .any(|key| inference::backend::canonical_backend_key(key) == device.backend_key)
        })
        .cloned()
        .collect();
    let backend = backends.iter().find(|backend| {
        backend.available
            && runtime
                .backend_keys
                .iter()
                .any(|key| inference::backend::canonical_backend_key(key) == backend.backend_key)
    });
    let backend_capabilities = backend
        .map(|backend| backend.capabilities.facts.clone())
        .unwrap_or_default();
    let mut device_ids = Vec::new();
    for variant in backend_capabilities
        .runtime_variants
        .iter()
        .filter(|variant| variant.available)
    {
        // CPU and MPS name singleton devices. Indexed device classes alone do
        // not establish an inventory or authorize inventing e.g. cuda:0.
        let singleton = match variant.device_class {
            inference::InferenceDeviceClass::Cpu => Some("cpu"),
            inference::InferenceDeviceClass::Mps => Some("mps"),
            _ => None,
        };
        if let Some(device) = singleton {
            device_ids.push(
                inference::InferenceDeviceId::parse(device).expect("canonical singleton device ID"),
            );
        }
    }
    if let Some(mode) = mode_info.filter(|mode| {
        backend.is_some_and(|backend| {
            mode.backend_key.as_deref() == Some(backend.backend_key.as_str())
        }) && mode.active_runtime.as_ref().is_some_and(|active| {
            active.active
                && active
                    .runtime_id
                    .as_deref()
                    .map(pantograph_runtime_identity::canonical_runtime_id)
                    .as_deref()
                    == Some(runtime.runtime_id.as_str())
                && active.runtime_instance_id.is_some()
                && active.runtime_instance_id == runtime.runtime_instance_id
        })
    }) {
        if let Some(device) = mode.active_resolved_device.as_ref().filter(|device| {
            backend_capabilities.runtime_variants.iter().any(|variant| {
                variant.available && device_matches_class(device.as_str(), variant.device_class)
            })
        }) {
            device_ids.push(device.clone());
        }
    }
    device_ids.sort();
    device_ids.dedup();
    let backend_keys = backend
        .map(|backend| vec![backend.backend_key.clone()])
        .unwrap_or(runtime.backend_keys);

    Some(RuntimeDispatchRuntimeCapabilityFacts {
        runtime_id: runtime.runtime_id,
        backend_keys,
        backend_capabilities,
        device_ids,
        runtime_family,
        runtime_residency_key,
        status: runtime.status,
        runtime_instance_id: runtime.runtime_instance_id,
        loaded_model_ids: runtime
            .models
            .into_iter()
            .map(|model| model.model_id)
            .collect(),
        active_reservation_ids: runtime.active_reservation_ids,
        has_admission_budget: runtime.admission_budget.is_some(),
        automatic_device_candidates,
    })
}

pub(crate) fn device_matches_class(
    device_id: &str,
    class: inference::InferenceDeviceClass,
) -> bool {
    match class {
        inference::InferenceDeviceClass::Cpu => device_id == "cpu",
        inference::InferenceDeviceClass::Mps => device_id == "mps",
        inference::InferenceDeviceClass::Cuda => device_id
            .strip_prefix("cuda:")
            .is_some_and(|index| index.parse::<u32>().is_ok()),
        inference::InferenceDeviceClass::Metal => device_id
            .strip_prefix("metal:")
            .is_some_and(|index| index.parse::<u32>().is_ok()),
        _ => false,
    }
}

fn diagnostic(
    code: RuntimeDispatchCapabilityFactsDiagnosticCode,
    message: &str,
) -> RuntimeDispatchCapabilityFactsDiagnostic {
    RuntimeDispatchCapabilityFactsDiagnostic {
        code,
        runtime_id: None,
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pantograph_runtime_registry::{
        RuntimeDispatchIdentity, RuntimeRegistration, RuntimeRegistry, RuntimeTransition,
    };

    use super::*;

    #[cfg(feature = "backend-pytorch")]
    #[tokio::test]
    async fn projection_joins_registered_backend_aliases_to_gateway_owned_cpu_candidates() {
        let registry = Arc::new(RuntimeRegistry::new());
        for (id, key) in [("pytorch", "torch"), ("other", "unsupported")] {
            registry.register_runtime(
                RuntimeRegistration::new(id, id)
                    .with_backend_keys(vec![key.into()])
                    .with_dispatch_identity(
                        RuntimeDispatchIdentity::new(
                            "transformers",
                            format!("runtime.{id}.shared"),
                        )
                        .expect("valid dispatch identity"),
                    ),
            );
        }
        let source = RuntimeDispatchCapabilityFactsSource::new(registry.clone());
        let RuntimeDispatchCapabilityFactsOutcome::Projected { facts, .. } = source.collect().await
        else {
            panic!("registered facts");
        };
        assert!(facts
            .runtimes
            .iter()
            .all(|runtime| runtime.automatic_device_candidates.is_empty()));
        let gateway = Arc::new(InferenceGateway::new());
        assert!(gateway
            .runtime_owned_device_candidates()
            .iter()
            .any(|device| device.backend_key == "pytorch"));
        let RuntimeDispatchCapabilityFactsOutcome::Projected { facts, .. } =
            source.with_gateway(gateway).collect().await
        else {
            panic!("owned facts");
        };
        let pytorch = facts
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "pytorch")
            .unwrap();
        assert_eq!(pytorch.automatic_device_candidates.len(), 1);
        assert_eq!(
            pytorch.automatic_device_candidates[0].device_id.as_str(),
            "cpu"
        );
        assert!(facts
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "other")
            .unwrap()
            .automatic_device_candidates
            .is_empty());
        assert!(registry.snapshot().reservations.is_empty());
    }

    #[tokio::test]
    async fn source_projects_path_free_runtime_registry_facts() {
        let registry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(
            RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["torch".to_string(), "pytorch".to_string()])
                .with_dispatch_identity(dispatch_identity()),
        );
        registry
            .transition_runtime(
                "pytorch",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime-instance.001".to_string()),
                },
            )
            .expect("ready runtime transition");
        let source = RuntimeDispatchCapabilityFactsSource::new(registry);

        let outcome = source.collect().await;

        let RuntimeDispatchCapabilityFactsOutcome::Projected { facts, .. } = outcome else {
            panic!("runtime registry source should project facts");
        };
        assert_eq!(facts.runtimes.len(), 1);
        let runtime = &facts.runtimes[0];
        assert_eq!(runtime.runtime_id, "pytorch");
        assert_eq!(runtime.runtime_family, "diffusers");
        assert_eq!(
            runtime.runtime_residency_key,
            "runtime.diffusers.pytorch.shared"
        );
        assert_eq!(runtime.status, RuntimeRegistryStatus::Ready);
        assert_eq!(
            runtime.runtime_instance_id.as_deref(),
            Some("runtime-instance.001")
        );
        assert!(runtime.backend_keys.iter().any(|key| key == "pytorch"));
        assert!(runtime.active_reservation_ids.is_empty());
        assert!(!runtime.has_admission_budget);
    }

    #[tokio::test]
    async fn source_reports_no_registered_runtimes() {
        let source = RuntimeDispatchCapabilityFactsSource::new(Arc::new(RuntimeRegistry::new()));

        let outcome = source.collect().await;

        assert!(matches!(
            outcome,
            RuntimeDispatchCapabilityFactsOutcome::Unavailable { .. }
        ));
        assert!(outcome.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == RuntimeDispatchCapabilityFactsDiagnosticCode::NoRegisteredRuntimes
        }));
    }

    #[tokio::test]
    async fn source_rejects_runtime_without_backend_keys() {
        let registry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(
            RuntimeRegistration::new("custom-runtime", "Custom Runtime")
                .with_dispatch_identity(dispatch_identity()),
        );
        let source = RuntimeDispatchCapabilityFactsSource::new(registry);

        let outcome = source.collect().await;

        assert!(matches!(
            outcome,
            RuntimeDispatchCapabilityFactsOutcome::Unavailable { .. }
        ));
        assert!(outcome.diagnostics().iter().any(|diagnostic| {
            diagnostic.code
                == RuntimeDispatchCapabilityFactsDiagnosticCode::RuntimeMissingBackendKeys
                && diagnostic.runtime_id.as_deref() == Some("custom-runtime")
        }));
    }

    #[tokio::test]
    async fn source_rejects_dispatch_runtime_without_dispatch_identity() {
        let registry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(
            RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["pytorch".to_string()]),
        );
        let source = RuntimeDispatchCapabilityFactsSource::new(registry);

        let outcome = source.collect().await;

        assert!(matches!(
            outcome,
            RuntimeDispatchCapabilityFactsOutcome::Unavailable { .. }
        ));
        assert!(outcome.diagnostics().iter().any(|diagnostic| {
            diagnostic.code
                == RuntimeDispatchCapabilityFactsDiagnosticCode::RuntimeMissingDispatchIdentity
                && diagnostic.runtime_id.as_deref() == Some("pytorch")
        }));
    }

    #[cfg(feature = "backend-pytorch")]
    #[tokio::test]
    async fn indexed_devices_require_matching_live_owner_observation() {
        let registry = Arc::new(RuntimeRegistry::new());
        let gateway = inference::InferenceGateway::with_backend(
            Box::new(inference::PyTorchBackend::new()),
            "PyTorch",
        );
        let mut backend = gateway.current_backend_info().await;
        backend.capabilities.facts.runtime_variants =
            inference::PyTorchBackend::runtime_variants_from_device_probe(
                inference::backend::pytorch::PyTorchDeviceProbeSnapshot {
                    cuda_available: true,
                    mps_available: false,
                },
            );
        crate::runtime_registry::register_backend_runtimes(
            &registry,
            std::slice::from_ref(&backend),
        );
        registry
            .transition_runtime(
                "pytorch",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("observed.1".into()),
                },
            )
            .unwrap();
        let runtime = registry.snapshot().runtimes.remove(0);
        let mut mode = gateway.mode_info().await;
        mode.active_runtime = Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("pytorch".into()),
            runtime_instance_id: Some("observed.1".into()),
            active: true,
            ..Default::default()
        });
        mode.active_resolved_device = Some("cuda:1".parse().unwrap());
        let facts = project_runtime(
            runtime.clone(),
            std::slice::from_ref(&backend),
            Some(&mode),
            &[],
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(
            facts
                .device_ids
                .iter()
                .map(|device| device.as_str())
                .collect::<Vec<_>>(),
            vec!["cpu", "cuda:1"]
        );
        for invalidation in [
            "unobserved",
            "instance",
            "runtime",
            "inactive",
            "unavailable",
        ] {
            let mut changed_mode = mode.clone();
            let mut changed_backend = backend.clone();
            match invalidation {
                "unobserved" => changed_mode.active_resolved_device = None,
                "instance" => {
                    changed_mode
                        .active_runtime
                        .as_mut()
                        .unwrap()
                        .runtime_instance_id = Some("other.2".into())
                }
                "runtime" => {
                    changed_mode.active_runtime.as_mut().unwrap().runtime_id = Some("other".into())
                }
                "inactive" => changed_mode.active_runtime.as_mut().unwrap().active = false,
                "unavailable" => changed_backend
                    .capabilities
                    .facts
                    .runtime_variants
                    .iter_mut()
                    .filter(|variant| variant.device_class == inference::InferenceDeviceClass::Cuda)
                    .for_each(|variant| variant.available = false),
                _ => unreachable!(),
            }
            let facts = project_runtime(
                runtime.clone(),
                &[changed_backend],
                Some(&changed_mode),
                &[],
                &mut Vec::new(),
            )
            .unwrap();
            assert_eq!(
                facts
                    .device_ids
                    .iter()
                    .map(|device| device.as_str())
                    .collect::<Vec<_>>(),
                vec!["cpu"],
                "{invalidation}"
            );
        }
    }

    fn dispatch_identity() -> RuntimeDispatchIdentity {
        RuntimeDispatchIdentity::new("diffusers", "runtime.diffusers.pytorch.shared")
            .expect("dispatch identity fixture")
    }
}
