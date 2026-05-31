use pantograph_dependency_planning::{DeviceIntentId, RuntimeIntentId};
use pantograph_runtime_registry::{
    RuntimeAdmissionResourceKind, RuntimeRegistryError, RuntimeReservationRequest,
    RuntimeReservationRequirements, RuntimeRetentionHint, SharedRuntimeRegistry,
};
use pantograph_scheduler::{
    SchedulerReservationLeaseId, SchedulerResourceDiagnostic, SchedulerResourceDiagnosticCode,
    SchedulerResourceDiagnosticSeverity, SchedulerResourceFitAssessment, SchedulerResourceFitState,
    SchedulerResourceKind, SchedulerResourceReservation, SchedulerTaskId, SchedulerWorkflowRunId,
};

#[derive(Debug, Clone)]
pub(crate) struct RuntimeDispatchResourceFactsSource {
    registry: SharedRuntimeRegistry,
}

impl RuntimeDispatchResourceFactsSource {
    pub(crate) fn new(registry: SharedRuntimeRegistry) -> Self {
        Self { registry }
    }

    pub(crate) fn reserve(
        &self,
        request: RuntimeDispatchResourceFactsRequest,
    ) -> RuntimeDispatchResourceFactsOutcome {
        let diagnostics = validate_request(&request);
        if !diagnostics.is_empty() {
            return RuntimeDispatchResourceFactsOutcome::Unavailable {
                fit_assessment: fit_assessment(
                    &request,
                    SchedulerResourceFitState::ImpossibleFit,
                    diagnostics
                        .iter()
                        .map(resource_diagnostic_from_source)
                        .collect(),
                ),
                diagnostics,
            };
        }

        match self
            .registry
            .acquire_reservation(runtime_reservation_request(&request))
        {
            Ok(lease) => RuntimeDispatchResourceFactsOutcome::Reserved {
                facts: RuntimeDispatchResourceFacts {
                    lease_id: lease.reservation_id,
                    reservations: scheduler_reservations(&request, lease.reservation_id),
                    fit_assessment: fit_assessment(
                        &request,
                        SchedulerResourceFitState::Fits,
                        Vec::new(),
                    ),
                },
                diagnostics: Vec::new(),
            },
            Err(error) => {
                let diagnostic = diagnostic_from_registry_error(&request, error);
                RuntimeDispatchResourceFactsOutcome::Unavailable {
                    fit_assessment: fit_assessment(
                        &request,
                        SchedulerResourceFitState::WaitingForResources,
                        vec![resource_diagnostic_from_source(&diagnostic)],
                    ),
                    diagnostics: vec![diagnostic],
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeDispatchResourceFactsRequest {
    pub runtime_id: RuntimeIntentId,
    pub selected_device_id: DeviceIntentId,
    pub workflow_id: String,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub task_id: SchedulerTaskId,
    pub reservation_owner_id: String,
    pub model_id: Option<String>,
    pub usage_profile: Option<String>,
    pub requirements: RuntimeReservationRequirements,
    pub retention_hint: RuntimeRetentionHint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeDispatchResourceFacts {
    pub lease_id: u64,
    pub reservations: Vec<SchedulerResourceReservation>,
    pub fit_assessment: SchedulerResourceFitAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeDispatchResourceFactsDiagnostic {
    pub code: RuntimeDispatchResourceFactsDiagnosticCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeDispatchResourceFactsDiagnosticCode {
    MissingWorkflowId,
    MissingReservationOwnerId,
    MissingResourceClaims,
    InvalidResourceClaimBytes,
    RuntimeRegistryRejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimeDispatchResourceFactsOutcome {
    Reserved {
        facts: RuntimeDispatchResourceFacts,
        diagnostics: Vec<RuntimeDispatchResourceFactsDiagnostic>,
    },
    Unavailable {
        fit_assessment: SchedulerResourceFitAssessment,
        diagnostics: Vec<RuntimeDispatchResourceFactsDiagnostic>,
    },
}

impl RuntimeDispatchResourceFactsOutcome {
    pub(crate) fn diagnostics(&self) -> &[RuntimeDispatchResourceFactsDiagnostic] {
        match self {
            Self::Reserved { diagnostics, .. } | Self::Unavailable { diagnostics, .. } => {
                diagnostics
            }
        }
    }
}

fn validate_request(
    request: &RuntimeDispatchResourceFactsRequest,
) -> Vec<RuntimeDispatchResourceFactsDiagnostic> {
    let mut diagnostics = Vec::new();
    if request.workflow_id.trim().is_empty() {
        diagnostics.push(diagnostic(
            RuntimeDispatchResourceFactsDiagnosticCode::MissingWorkflowId,
            "runtime dispatch resource reservation requires a workflow id",
        ));
    }
    if request.reservation_owner_id.trim().is_empty() {
        diagnostics.push(diagnostic(
            RuntimeDispatchResourceFactsDiagnosticCode::MissingReservationOwnerId,
            "runtime dispatch resource reservation requires a reservation owner id",
        ));
    }
    if request.requirements.claims.is_empty() {
        diagnostics.push(diagnostic(
            RuntimeDispatchResourceFactsDiagnosticCode::MissingResourceClaims,
            "runtime dispatch resource reservation requires at least one resource claim",
        ));
    }
    for claim in &request.requirements.claims {
        if claim.bytes == 0 {
            diagnostics.push(diagnostic(
                RuntimeDispatchResourceFactsDiagnosticCode::InvalidResourceClaimBytes,
                "runtime dispatch resource reservation claim bytes must be greater than zero",
            ));
        }
    }
    diagnostics
}

fn runtime_reservation_request(
    request: &RuntimeDispatchResourceFactsRequest,
) -> RuntimeReservationRequest {
    RuntimeReservationRequest {
        runtime_id: request.runtime_id.as_str().to_string(),
        workflow_id: request.workflow_id.clone(),
        reservation_owner_id: Some(request.reservation_owner_id.clone()),
        usage_profile: request.usage_profile.clone(),
        model_id: request.model_id.clone(),
        pin_runtime: false,
        requirements: Some(request.requirements.clone()),
        retention_hint: request.retention_hint,
    }
}

fn scheduler_reservations(
    request: &RuntimeDispatchResourceFactsRequest,
    lease_id: u64,
) -> Vec<SchedulerResourceReservation> {
    request
        .requirements
        .claims
        .iter()
        .map(|claim| SchedulerResourceReservation {
            reservation_lease_id: SchedulerReservationLeaseId::parse(format!(
                "runtime-registry.{lease_id}"
            ))
            .expect("runtime-registry reservation ids should be scheduler-safe"),
            workflow_run_id: request.workflow_run_id.clone(),
            task_id: request.task_id.clone(),
            device_id: request.selected_device_id.clone(),
            resource_kind: scheduler_resource_kind(claim.kind),
            reserved_bytes: claim.bytes,
        })
        .collect()
}

fn scheduler_resource_kind(kind: RuntimeAdmissionResourceKind) -> SchedulerResourceKind {
    match kind {
        RuntimeAdmissionResourceKind::RamBytes => SchedulerResourceKind::SystemRam,
        RuntimeAdmissionResourceKind::VramBytes => SchedulerResourceKind::DeviceVram,
    }
}

fn fit_assessment(
    request: &RuntimeDispatchResourceFactsRequest,
    state: SchedulerResourceFitState,
    diagnostics: Vec<SchedulerResourceDiagnostic>,
) -> SchedulerResourceFitAssessment {
    SchedulerResourceFitAssessment {
        workflow_run_id: request.workflow_run_id.clone(),
        task_id: request.task_id.clone(),
        state,
        diagnostics,
    }
}

fn resource_diagnostic_from_source(
    diagnostic: &RuntimeDispatchResourceFactsDiagnostic,
) -> SchedulerResourceDiagnostic {
    SchedulerResourceDiagnostic {
        severity: SchedulerResourceDiagnosticSeverity::Error,
        code: match diagnostic.code {
            RuntimeDispatchResourceFactsDiagnosticCode::RuntimeRegistryRejected => {
                SchedulerResourceDiagnosticCode::SchedulerResourcePolicyError
            }
            RuntimeDispatchResourceFactsDiagnosticCode::MissingResourceClaims
            | RuntimeDispatchResourceFactsDiagnosticCode::InvalidResourceClaimBytes => {
                SchedulerResourceDiagnosticCode::ImpossibleFit
            }
            RuntimeDispatchResourceFactsDiagnosticCode::MissingWorkflowId
            | RuntimeDispatchResourceFactsDiagnosticCode::MissingReservationOwnerId => {
                SchedulerResourceDiagnosticCode::ObservationUnavailable
            }
        },
        message: diagnostic.message.clone(),
        hint: None,
    }
}

fn diagnostic_from_registry_error(
    request: &RuntimeDispatchResourceFactsRequest,
    error: RuntimeRegistryError,
) -> RuntimeDispatchResourceFactsDiagnostic {
    RuntimeDispatchResourceFactsDiagnostic {
        code: RuntimeDispatchResourceFactsDiagnosticCode::RuntimeRegistryRejected,
        message: format!(
            "runtime registry rejected dispatch reservation for runtime '{}' and task '{}': {error}",
            request.runtime_id.as_str(),
            request.task_id.as_str()
        ),
    }
}

fn diagnostic(
    code: RuntimeDispatchResourceFactsDiagnosticCode,
    message: &str,
) -> RuntimeDispatchResourceFactsDiagnostic {
    RuntimeDispatchResourceFactsDiagnostic {
        code,
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pantograph_dependency_planning::{DeviceIntentId, RuntimeIntentId};
    use pantograph_runtime_registry::{
        RuntimeAdmissionBudget, RuntimeAdmissionResourceBudget, RuntimeReservationResourceClaim,
        RuntimeTransition,
    };
    use pantograph_scheduler::{SchedulerTaskId, SchedulerWorkflowRunId};

    use super::*;

    #[test]
    fn source_acquires_real_lease_and_projects_scheduler_reservation_facts() {
        let registry = Arc::new(pantograph_runtime_registry::RuntimeRegistry::new());
        registry.register_runtime(
            pantograph_runtime_registry::RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["pytorch".to_string()])
                .with_admission_budget(RuntimeAdmissionBudget::from_resources(vec![
                    RuntimeAdmissionResourceBudget::ram_bytes(Some(16 * mib())),
                    RuntimeAdmissionResourceBudget::vram_bytes(Some(8 * mib())),
                ])),
        );
        registry
            .transition_runtime(
                "pytorch",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime-instance.001".to_string()),
                },
            )
            .expect("ready runtime transition");
        let source = RuntimeDispatchResourceFactsSource::new(registry.clone());

        let outcome = source.reserve(resource_request(vec![
            RuntimeReservationResourceClaim::ram_bytes(mib()),
            RuntimeReservationResourceClaim::vram_bytes(2 * mib()),
        ]));

        let RuntimeDispatchResourceFactsOutcome::Reserved { facts, .. } = outcome else {
            panic!("resource source should reserve real registry lease");
        };
        assert_eq!(facts.reservations.len(), 2);
        assert!(facts
            .reservations
            .iter()
            .any(
                |reservation| reservation.resource_kind == SchedulerResourceKind::SystemRam
                    && reservation.reserved_bytes == mib()
            ));
        assert!(facts
            .reservations
            .iter()
            .any(
                |reservation| reservation.resource_kind == SchedulerResourceKind::DeviceVram
                    && reservation.reserved_bytes == 2 * mib()
            ));
        assert_eq!(facts.fit_assessment.state, SchedulerResourceFitState::Fits);
        assert_eq!(registry.snapshot().reservations.len(), 1);
        assert_eq!(
            registry.snapshot().reservations[0].reservation_id,
            facts.lease_id
        );
    }

    #[test]
    fn source_reuses_reservation_owner_for_same_runtime() {
        let registry = Arc::new(pantograph_runtime_registry::RuntimeRegistry::new());
        registry.register_runtime(
            pantograph_runtime_registry::RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["pytorch".to_string()]),
        );
        let source = RuntimeDispatchResourceFactsSource::new(registry);
        let request = resource_request(vec![RuntimeReservationResourceClaim::ram_bytes(mib())]);

        let first = source.reserve(request.clone());
        let second = source.reserve(request);

        let RuntimeDispatchResourceFactsOutcome::Reserved { facts: first, .. } = first else {
            panic!("first reservation should succeed");
        };
        let RuntimeDispatchResourceFactsOutcome::Reserved { facts: second, .. } = second else {
            panic!("second reservation should succeed");
        };
        assert_eq!(first.lease_id, second.lease_id);
    }

    #[test]
    fn source_returns_fit_diagnostics_for_admission_failure() {
        let registry = Arc::new(pantograph_runtime_registry::RuntimeRegistry::new());
        registry.register_runtime(
            pantograph_runtime_registry::RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["pytorch".to_string()])
                .with_admission_budget(RuntimeAdmissionBudget::from_resources(vec![
                    RuntimeAdmissionResourceBudget::vram_bytes(Some(mib())),
                ])),
        );
        let source = RuntimeDispatchResourceFactsSource::new(registry);

        let outcome = source.reserve(resource_request(vec![
            RuntimeReservationResourceClaim::vram_bytes(2 * mib()),
        ]));

        let RuntimeDispatchResourceFactsOutcome::Unavailable {
            fit_assessment,
            diagnostics,
        } = outcome
        else {
            panic!("oversized reservation should be unavailable");
        };
        assert_eq!(
            diagnostics[0].code,
            RuntimeDispatchResourceFactsDiagnosticCode::RuntimeRegistryRejected
        );
        assert_eq!(
            fit_assessment.state,
            SchedulerResourceFitState::WaitingForResources
        );
        assert!(!fit_assessment.diagnostics.is_empty());
    }

    #[test]
    fn source_rejects_empty_resource_claims_before_registry_reservation() {
        let registry = Arc::new(pantograph_runtime_registry::RuntimeRegistry::new());
        registry.register_runtime(
            pantograph_runtime_registry::RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["pytorch".to_string()]),
        );
        let source = RuntimeDispatchResourceFactsSource::new(registry.clone());

        let outcome = source.reserve(resource_request(Vec::new()));

        assert!(matches!(
            outcome,
            RuntimeDispatchResourceFactsOutcome::Unavailable { .. }
        ));
        assert!(outcome.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == RuntimeDispatchResourceFactsDiagnosticCode::MissingResourceClaims
        }));
        assert!(registry.snapshot().reservations.is_empty());
    }

    fn resource_request(
        claims: Vec<RuntimeReservationResourceClaim>,
    ) -> RuntimeDispatchResourceFactsRequest {
        RuntimeDispatchResourceFactsRequest {
            runtime_id: RuntimeIntentId::parse("pytorch").expect("runtime id"),
            selected_device_id: DeviceIntentId::parse("cuda:0").expect("device id"),
            workflow_id: "workflow.image".to_string(),
            workflow_run_id: SchedulerWorkflowRunId::parse("run.image.001")
                .expect("workflow run id"),
            task_id: SchedulerTaskId::parse("task.inference.001").expect("task id"),
            reservation_owner_id: "run.image.001:task.inference.001".to_string(),
            model_id: Some("diffusion/imported/test-bundle".to_string()),
            usage_profile: Some("image-generation".to_string()),
            requirements: RuntimeReservationRequirements::from_claims(claims),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        }
    }

    fn mib() -> u64 {
        1024 * 1024
    }
}
