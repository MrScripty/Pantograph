use super::*;
use crate::embedded_workflow_host_helpers::unresolved_llamacpp_device_decision_error;

const MIB: u64 = 1024 * 1024;

#[test]
fn reservation_requirements_returns_none_when_workflow_estimate_is_unknown() {
    assert_eq!(
        EmbeddedWorkflowHost::reservation_requirements(&WorkflowRuntimeRequirements::default())
            .expect("unknown estimates should not fail"),
        None
    );
}

#[test]
fn reservation_requirements_maps_workflow_memory_estimates() {
    let requirements =
        EmbeddedWorkflowHost::reservation_requirements(&WorkflowRuntimeRequirements {
            estimated_peak_vram_mb: Some(2048),
            estimated_peak_ram_mb: Some(1024),
            estimated_min_vram_mb: Some(1536),
            estimated_min_ram_mb: Some(768),
            estimation_confidence: "estimated_from_model_sizes".to_string(),
            required_models: vec!["model-a".to_string()],
            required_backends: vec!["llama_cpp".to_string()],
            required_extensions: Vec::new(),
        })
        .expect("requirements should be forwarded when estimates exist");

    let requirements = requirements.expect("claims");
    assert_eq!(
        requirements.claims,
        vec![
            pantograph_runtime_registry::RuntimeReservationResourceClaim::vram_bytes(2048 * MIB),
            pantograph_runtime_registry::RuntimeReservationResourceClaim::ram_bytes(1024 * MIB),
        ]
    );
}

#[test]
fn runtime_registry_admission_errors_map_to_runtime_not_ready() {
    let error = runtime_registry_errors::workflow_service_error_from_runtime_registry(
        RuntimeRegistryError::AdmissionRejected {
            runtime_id: "pytorch".to_string(),
            failure: pantograph_runtime_registry::RuntimeAdmissionFailure::InsufficientRam {
                requested_bytes: 1024 * MIB,
                available_bytes: 0,
                reserved_bytes: 2048 * MIB,
                total_bytes: 2048 * MIB,
                safety_margin_bytes: 0,
            },
        },
    );

    assert!(matches!(error, WorkflowServiceError::RuntimeNotReady(_)));
    assert_eq!(
        error.code(),
        pantograph_workflow_service::WorkflowErrorCode::RuntimeNotReady
    );
}

#[test]
fn runtime_registry_owner_conflicts_map_to_invalid_request() {
    let error = runtime_registry_errors::workflow_service_error_from_runtime_registry(
        RuntimeRegistryError::ReservationOwnerConflict {
            owner_id: "session-a".to_string(),
            existing_runtime_id: "llama_cpp".to_string(),
            requested_runtime_id: "pytorch".to_string(),
        },
    );

    assert!(matches!(error, WorkflowServiceError::InvalidRequest(_)));
    assert_eq!(
        error.code(),
        pantograph_workflow_service::WorkflowErrorCode::InvalidRequest
    );
}

#[test]
fn runtime_registry_resource_accounting_errors_map_to_internal() {
    let overflow = runtime_registry_errors::workflow_service_error_from_runtime_registry(
        RuntimeRegistryError::ResourceAccountingOverflow {
            runtime_id: "pytorch".to_string(),
            resource_kind: "ram",
        },
    );
    let underflow = runtime_registry_errors::workflow_service_error_from_runtime_registry(
        RuntimeRegistryError::ResourceBudgetUnderflow {
            runtime_id: "pytorch".to_string(),
            resource_kind: "vram",
            total_bytes: 1,
            safety_margin_bytes: 2,
            reserved_bytes: 0,
        },
    );

    assert!(matches!(overflow, WorkflowServiceError::Internal(_)));
    assert!(matches!(underflow, WorkflowServiceError::Internal(_)));
    assert_eq!(
        overflow.code(),
        pantograph_workflow_service::WorkflowErrorCode::InternalError
    );
    assert_eq!(
        underflow.code(),
        pantograph_workflow_service::WorkflowErrorCode::InternalError
    );
}

#[test]
fn unresolved_llamacpp_device_decision_blocks_host_owned_auto_start() {
    let error = unresolved_llamacpp_device_decision_error(Path::new("/models/model.gguf"));

    assert_eq!(
        error.code(),
        pantograph_workflow_service::WorkflowErrorCode::RuntimeNotReady
    );
    assert!(error
        .to_string()
        .contains("no canonical runtime/device decision"));
}
