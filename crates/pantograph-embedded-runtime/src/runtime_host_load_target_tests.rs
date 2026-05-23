use pumas_library::models::{
    AssetValidationState, ModelArtifactState, ModelEntryPathState, PackageArtifactKind,
    PumasArtifactLoadPathKind, PumasArtifactLoadTarget, PumasArtifactLoadTargetDiagnostic,
    PumasArtifactLoadTargetDiagnosticCode, ResolveModelArtifactLoadTargetResponse, StorageKind,
};

use crate::runtime_host_execution::{
    RuntimeHostExecutionRequest, ValidatedRuntimeHostExecutionRequest,
};

use super::{
    build_runtime_host_artifact_load_target_request, ready_runtime_host_artifact_load_target,
    RuntimeHostPumasLoadTargetError,
};

#[test]
fn load_target_request_uses_scheduler_selected_model_ref() {
    let request = validated_runtime_host_request();
    let pumas_request = build_runtime_host_artifact_load_target_request(&request)
        .expect("request builder must accept validated scheduler handoff");

    assert_eq!(
        pumas_request.model_ref.model_id,
        "pumas://models/juggernaut-xl-v10"
    );
    assert_eq!(
        pumas_request.model_ref.selected_artifact_id.as_deref(),
        Some("diffusers-bundle")
    );
    assert_eq!(
        pumas_request.model_ref.selected_artifact_path.as_deref(),
        Some("juggernaut-xl-v10/diffusers")
    );
    assert_eq!(
        pumas_request.caller_observed_entry_path.as_deref(),
        Some("juggernaut-xl-v10/diffusers")
    );
    assert_eq!(pumas_request.expected_artifact_kind, None);
    assert_eq!(
        pumas_request.consumer.runtime_family.as_deref(),
        Some("diffusers-pytorch.cuda")
    );
    assert_eq!(
        pumas_request.consumer.task_kind.as_deref(),
        Some("image_generation")
    );
}

#[test]
fn ready_load_target_response_returns_host_only_target() {
    let response = ResolveModelArtifactLoadTargetResponse {
        artifact_state: ModelArtifactState::Ready,
        entry_path_state: ModelEntryPathState::Ready,
        target: Some(PumasArtifactLoadTarget {
            model_ref: pumas_library::models::PumasModelRef {
                model_id: "pumas://models/juggernaut-xl-v10".to_string(),
                selected_artifact_id: Some("diffusers-bundle".to_string()),
                selected_artifact_path: Some("juggernaut-xl-v10/diffusers".to_string()),
                ..Default::default()
            },
            artifact_kind: PackageArtifactKind::DiffusersBundle,
            local_load_path: "/host-only/pumas/juggernaut-xl-v10".to_string(),
            load_path_kind: PumasArtifactLoadPathKind::Directory,
            library_root_id: Some("default".to_string()),
            storage_kind: StorageKind::LibraryOwned,
            validation_state: AssetValidationState::Valid,
            content_fingerprint: Some("sha256:abc".to_string()),
            package_facts_contract_version: Some(2),
        }),
        diagnostics: Vec::new(),
    };

    let target = ready_runtime_host_artifact_load_target(response)
        .expect("ready Pumas response must return host-only load target");

    assert_eq!(target.artifact_kind, PackageArtifactKind::DiffusersBundle);
    assert_eq!(target.storage_kind, StorageKind::LibraryOwned);
}

#[test]
fn unavailable_load_target_response_returns_typed_error() {
    let response = ResolveModelArtifactLoadTargetResponse {
        artifact_state: ModelArtifactState::Missing,
        entry_path_state: ModelEntryPathState::Missing,
        target: None,
        diagnostics: vec![PumasArtifactLoadTargetDiagnostic {
            code: PumasArtifactLoadTargetDiagnosticCode::ArtifactMissing,
            field_path: Some("artifact".to_string()),
            message: "artifact is missing".to_string(),
        }],
    };

    let error = ready_runtime_host_artifact_load_target(response)
        .expect_err("unavailable Pumas response must fail with typed error");

    assert!(matches!(
        error,
        RuntimeHostPumasLoadTargetError::Unavailable {
            artifact_state,
            entry_path_state,
            diagnostic_count: 1,
            ..
        } if artifact_state == "Missing" && entry_path_state == "Missing"
    ));
}

fn validated_runtime_host_request() -> ValidatedRuntimeHostExecutionRequest {
    let request: RuntimeHostExecutionRequest = serde_json::from_str(include_str!(
        "runtime_host_execution_tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host execution request fixture must decode");
    ValidatedRuntimeHostExecutionRequest::try_from(request)
        .expect("runtime host execution request fixture must validate")
}
