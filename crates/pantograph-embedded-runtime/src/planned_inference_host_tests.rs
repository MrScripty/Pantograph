use inference::{
    BackendExecutionDecision, BackendId, DeviceResolutionDecision, InferenceDeviceClass,
    InferenceDevicePolicy, InferenceTaskId, ModelArtifactKind, ModelStorageKind,
    ModelValidationState, ResolvedModelPackageFacts, RuntimeVariantId,
};
use pumas_library::models::{
    AssetValidationState, ModelArtifactState, ModelEntryPathState, PackageArtifactKind,
    PumasArtifactLoadTargetDiagnostic, PumasArtifactLoadTargetDiagnosticCode,
    ResolveModelArtifactLoadTargetResponse, StorageKind,
};

use super::{
    build_image_artifact_load_target_request, project_pumas_artifact_load_target,
    ready_pumas_artifact_load_target, EmbeddedPlannedInferenceHostError,
};

fn backend_decision() -> BackendExecutionDecision {
    let runtime_variant_id = RuntimeVariantId::parse("pytorch.cuda").expect("runtime variant id");
    BackendExecutionDecision {
        selected_backend_id: BackendId::parse("pytorch").expect("backend id"),
        selected_runtime_variant_id: runtime_variant_id.clone(),
        selected_device_class: InferenceDeviceClass::Cuda,
        selected_device_id: None,
        device_decision: DeviceResolutionDecision {
            policy: InferenceDevicePolicy::Auto,
            runtime_variant_id,
            selected_device_class: InferenceDeviceClass::Cuda,
            selected_device_id: None,
            diagnostics: Vec::new(),
        },
        selected_task_id: Some(InferenceTaskId::ImageGeneration),
        selected_model_ref: Some(inference::PumasModelRef {
            model_id: "pumas://models/image/example".to_string(),
            revision: None,
            selected_artifact_id: Some("artifact-1".to_string()),
            selected_artifact_path: Some("image/example".to_string()),
            migration_diagnostics: Vec::new(),
        }),
        diagnostics: Vec::new(),
        dependency_readiness: Vec::new(),
        selection_policy_trace: None,
    }
}

#[test]
fn image_load_target_request_preserves_scheduler_selected_model_ref() {
    let package_facts: ResolvedModelPackageFacts = serde_json::from_value(serde_json::json!({
        "package_facts_contract_version": 2,
        "model_ref": {
            "model_id": "pumas://models/image/example"
        },
        "artifact": {
            "artifact_kind": "diffusers_bundle",
            "entry_path": "image/example",
            "storage_kind": "library_owned",
            "validation_state": "valid"
        },
        "task": {"task_type_primary": "image_generation"},
        "generation_defaults": {"status": "uninspected"},
        "custom_code": {"requires_custom_code": false},
        "backend_hints": {}
    }))
    .expect("package facts");
    let decision = backend_decision();
    let request = build_image_artifact_load_target_request(
        decision.selected_model_ref.as_ref().expect("model ref"),
        &decision,
        &package_facts,
    );

    assert_eq!(request.model_ref.model_id, "pumas://models/image/example");
    assert_eq!(
        request.model_ref.selected_artifact_id.as_deref(),
        Some("artifact-1")
    );
    assert_eq!(
        request.expected_artifact_kind,
        Some(PackageArtifactKind::DiffusersBundle)
    );
    assert_eq!(
        request.caller_observed_entry_path.as_deref(),
        Some("image/example")
    );
    assert_eq!(
        request.caller_observed_package_facts_contract_version,
        Some(2)
    );
    assert_eq!(
        request.consumer.task_kind.as_deref(),
        Some("image_generation")
    );
    assert_eq!(
        request.consumer.runtime_family.as_deref(),
        Some("pytorch.cuda")
    );
}

#[test]
fn ready_load_target_projects_to_inference_contract_without_pumas_version_field() {
    let target = pumas_library::models::PumasArtifactLoadTarget {
        model_ref: pumas_library::models::PumasModelRef {
            model_id: "pumas://models/image/example".to_string(),
            selected_artifact_id: Some("artifact-1".to_string()),
            selected_artifact_path: Some("image/example".to_string()),
            ..Default::default()
        },
        artifact_kind: PackageArtifactKind::DiffusersBundle,
        local_load_path: "/models/image/example".to_string(),
        load_path_kind: pumas_library::models::PumasArtifactLoadPathKind::Directory,
        library_root_id: Some("default".to_string()),
        storage_kind: StorageKind::LibraryOwned,
        validation_state: AssetValidationState::Valid,
        content_fingerprint: Some("sha256:abc".to_string()),
        package_facts_contract_version: Some(2),
    };

    let projected = project_pumas_artifact_load_target(target);

    assert_eq!(projected.model_ref.model_id, "pumas://models/image/example");
    assert_eq!(
        projected.model_ref.selected_artifact_id.as_deref(),
        Some("artifact-1")
    );
    assert_eq!(projected.artifact_kind, ModelArtifactKind::DiffusersBundle);
    assert_eq!(projected.storage_kind, ModelStorageKind::LibraryOwned);
    assert_eq!(projected.validation_state, ModelValidationState::Valid);
    assert_eq!(projected.package_facts_contract_version, Some(2));
}

#[test]
fn unavailable_load_target_returns_typed_projection_error() {
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

    let error = ready_pumas_artifact_load_target(response).expect_err("missing target should fail");

    assert!(matches!(
        error,
        EmbeddedPlannedInferenceHostError::ArtifactLoadTargetUnavailable {
            artifact_state,
            entry_path_state,
            diagnostic_count: 1,
            ..
        } if artifact_state == "Missing" && entry_path_state == "Missing"
    ));
}
