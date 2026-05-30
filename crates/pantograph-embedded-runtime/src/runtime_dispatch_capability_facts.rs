use pantograph_runtime_registry::{
    RuntimeRegistryRuntimeSnapshot, RuntimeRegistryStatus, SharedRuntimeRegistry,
};

#[derive(Clone)]
pub(crate) struct RuntimeDispatchCapabilityFactsSource {
    registry: SharedRuntimeRegistry,
}

impl RuntimeDispatchCapabilityFactsSource {
    pub(crate) fn new(registry: SharedRuntimeRegistry) -> Self {
        Self { registry }
    }

    pub(crate) fn collect(&self) -> RuntimeDispatchCapabilityFactsOutcome {
        let snapshot = self.registry.snapshot();
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
            .filter_map(|runtime| project_runtime(runtime, &mut diagnostics))
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
    pub status: RuntimeRegistryStatus,
    pub runtime_instance_id: Option<String>,
    pub loaded_model_ids: Vec<String>,
    pub active_reservation_ids: Vec<u64>,
    pub has_admission_budget: bool,
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

    Some(RuntimeDispatchRuntimeCapabilityFacts {
        runtime_id: runtime.runtime_id,
        backend_keys: runtime.backend_keys,
        status: runtime.status,
        runtime_instance_id: runtime.runtime_instance_id,
        loaded_model_ids: runtime
            .models
            .into_iter()
            .map(|model| model.model_id)
            .collect(),
        active_reservation_ids: runtime.active_reservation_ids,
        has_admission_budget: runtime.admission_budget.is_some(),
    })
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

    use pantograph_runtime_registry::{RuntimeRegistration, RuntimeRegistry, RuntimeTransition};

    use super::*;

    #[test]
    fn source_projects_path_free_runtime_registry_facts() {
        let registry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(
            RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["torch".to_string(), "pytorch".to_string()]),
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

        let outcome = source.collect();

        let RuntimeDispatchCapabilityFactsOutcome::Projected { facts, .. } = outcome else {
            panic!("runtime registry source should project facts");
        };
        assert_eq!(facts.runtimes.len(), 1);
        let runtime = &facts.runtimes[0];
        assert_eq!(runtime.runtime_id, "pytorch");
        assert_eq!(runtime.status, RuntimeRegistryStatus::Ready);
        assert_eq!(
            runtime.runtime_instance_id.as_deref(),
            Some("runtime-instance.001")
        );
        assert!(runtime.backend_keys.iter().any(|key| key == "pytorch"));
        assert!(runtime.active_reservation_ids.is_empty());
        assert!(!runtime.has_admission_budget);
    }

    #[test]
    fn source_reports_no_registered_runtimes() {
        let source = RuntimeDispatchCapabilityFactsSource::new(Arc::new(RuntimeRegistry::new()));

        let outcome = source.collect();

        assert!(matches!(
            outcome,
            RuntimeDispatchCapabilityFactsOutcome::Unavailable { .. }
        ));
        assert!(outcome.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == RuntimeDispatchCapabilityFactsDiagnosticCode::NoRegisteredRuntimes
        }));
    }

    #[test]
    fn source_rejects_runtime_without_backend_keys() {
        let registry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(RuntimeRegistration::new("custom-runtime", "Custom Runtime"));
        let source = RuntimeDispatchCapabilityFactsSource::new(registry);

        let outcome = source.collect();

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
}
