use pantograph_runtime_registry::RuntimeRegistryError;
use pantograph_workflow_service::WorkflowServiceError;

use crate::runtime_registry;

pub(crate) fn workflow_service_error_from_runtime_registry(
    error: RuntimeRegistryError,
) -> WorkflowServiceError {
    match error {
        RuntimeRegistryError::RuntimeNotFound(_)
        | RuntimeRegistryError::ModelResidencyObservationChanged(_)
        | RuntimeRegistryError::ModelResidencyResourcesUnavailable { .. }
        | RuntimeRegistryError::ReservationRejected(_)
        | RuntimeRegistryError::ReservationCustodyPending(_)
        | RuntimeRegistryError::ReservationObservationChanged(_)
        | RuntimeRegistryError::AdmissionRejected { .. }
        | RuntimeRegistryError::ResourceDomainAdmissionRejected { .. } => {
            WorkflowServiceError::RuntimeNotReady(error.to_string())
        }
        RuntimeRegistryError::ReservationOwnerConflict { .. }
        | RuntimeRegistryError::InvalidModelResidencyResources { .. }
        | RuntimeRegistryError::InvalidResourceDomain { .. } => {
            WorkflowServiceError::InvalidRequest(error.to_string())
        }
        RuntimeRegistryError::ReservationNotFound(_)
        | RuntimeRegistryError::ReservationSequenceExhausted
        | RuntimeRegistryError::InvalidTransition { .. }
        | RuntimeRegistryError::ResourceAccountingOverflow { .. }
        | RuntimeRegistryError::ResourceDomainAccountingOverflow { .. }
        | RuntimeRegistryError::ResourceBudgetUnderflow { .. } => {
            WorkflowServiceError::Internal(error.to_string())
        }
    }
}

pub(crate) fn workflow_service_error_from_runtime_warmup_coordination(
    error: runtime_registry::RuntimeWarmupCoordinationError,
) -> WorkflowServiceError {
    match error {
        runtime_registry::RuntimeWarmupCoordinationError::Registry(error) => {
            workflow_service_error_from_runtime_registry(error)
        }
        runtime_registry::RuntimeWarmupCoordinationError::Timeout { runtime_id } => {
            WorkflowServiceError::RuntimeTimeout(format!(
                "timed out waiting for runtime '{}' to finish warmup or shutdown transition",
                runtime_id
            ))
        }
    }
}
