use serde_json::json;

use super::{
    RuntimeSessionLoadProofContractError, ValidatedWorkflowSessionRuntimeLoadProof,
    WorkflowSessionRuntimeLoadProof, WorkflowSessionRuntimeLoadProofDiagnosticPhase,
    WorkflowSessionRuntimeLoadProofReadinessState, RUNTIME_SESSION_LOAD_PROOF_CONTRACT_VERSION,
};

#[test]
fn runtime_session_load_proof_fixture_decodes_and_validates() {
    let proof: WorkflowSessionRuntimeLoadProof = serde_json::from_value(json!({
        "contract_version": RUNTIME_SESSION_LOAD_PROOF_CONTRACT_VERSION,
        "workflow_id": "workflow-a",
        "task_id": "task-a",
        "backend_key": "llama_cpp",
        "runtime_id": "managed-llama-slot",
        "model_id": "pumas:models:maid",
        "artifact_id": "artifact:maid:gguf",
        "load_target_id": "load-target:maid:q4",
        "readiness_state": "ready",
        "diagnostic_phase": "runtime_model_load",
        "requested_model_active": true
    }))
    .expect("runtime session load proof fixture must decode");

    let validated = ValidatedWorkflowSessionRuntimeLoadProof::try_from(proof)
        .expect("runtime session load proof fixture must validate");

    assert_eq!(
        validated.as_ref().contract_version,
        RUNTIME_SESSION_LOAD_PROOF_CONTRACT_VERSION
    );
    assert_eq!(
        validated.as_ref().readiness_state,
        WorkflowSessionRuntimeLoadProofReadinessState::Ready
    );
    assert_eq!(
        validated.as_ref().diagnostic_phase,
        Some(WorkflowSessionRuntimeLoadProofDiagnosticPhase::RuntimeModelLoad)
    );
}

#[test]
fn runtime_session_load_proof_rejects_path_shaped_fields() {
    let error = serde_json::from_value::<WorkflowSessionRuntimeLoadProof>(json!({
        "workflow_id": "workflow-a",
        "backend_key": "llama_cpp",
        "active_model_path": "/models/model.gguf",
        "readiness_state": "ready",
        "requested_model_active": true
    }))
    .expect_err("runtime session load proof must reject executable path fields");

    assert!(
        error
            .to_string()
            .contains("unknown field `active_model_path`"),
        "{error}"
    );
}

#[test]
fn runtime_session_load_proof_rejects_ready_without_active_model() {
    let proof = WorkflowSessionRuntimeLoadProof {
        contract_version: RUNTIME_SESSION_LOAD_PROOF_CONTRACT_VERSION,
        workflow_id: "workflow-a".to_string(),
        task_id: None,
        backend_key: "llama_cpp".to_string(),
        runtime_id: Some("runtime-a".to_string()),
        model_id: Some("model-a".to_string()),
        artifact_id: None,
        load_target_id: None,
        readiness_state: WorkflowSessionRuntimeLoadProofReadinessState::Ready,
        diagnostic_phase: Some(WorkflowSessionRuntimeLoadProofDiagnosticPhase::RuntimeModelLoad),
        requested_model_active: false,
    };

    let error = ValidatedWorkflowSessionRuntimeLoadProof::try_from(proof)
        .expect_err("ready proofs must mark requested model active");

    assert_eq!(
        error,
        RuntimeSessionLoadProofContractError::InvalidField {
            field: "requested_model_active",
            reason: "ready runtime session load proofs must mark the requested model active"
        }
    );
}

#[test]
fn runtime_session_load_proof_rejects_invalid_workflow_id() {
    let proof = WorkflowSessionRuntimeLoadProof {
        contract_version: RUNTIME_SESSION_LOAD_PROOF_CONTRACT_VERSION,
        workflow_id: "workflow/a".to_string(),
        task_id: None,
        backend_key: "llama_cpp".to_string(),
        runtime_id: None,
        model_id: None,
        artifact_id: None,
        load_target_id: None,
        readiness_state: WorkflowSessionRuntimeLoadProofReadinessState::NotReady,
        diagnostic_phase: None,
        requested_model_active: false,
    };

    let error = ValidatedWorkflowSessionRuntimeLoadProof::try_from(proof)
        .expect_err("workflow ids must use bounded identifier syntax");

    assert_eq!(
        error,
        RuntimeSessionLoadProofContractError::InvalidIdentifier {
            field: "workflow_id"
        }
    );
}
