use inference::{
    ModelArtifactKind, ModelStorageKind, PumasArtifactLoadPathKind as InferenceLoadPathKind,
    PumasArtifactLoadTarget,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionRequest, ValidatedRuntimeHostExecutionRequest,
};
use pumas_library::models::{
    AssetValidationState, ModelArtifactState, ModelEntryPathState, PackageArtifactKind,
    PumasArtifactLoadPathKind, PumasArtifactLoadTarget as PumasLibraryArtifactLoadTarget,
    PumasArtifactLoadTargetDiagnostic, PumasArtifactLoadTargetDiagnosticCode,
    ResolveModelArtifactLoadTargetResponse, StorageKind, PACKAGE_FACTS_CONTRACT_VERSION,
};

use super::{
    build_runtime_host_artifact_load_target_request, build_runtime_host_model_requirement,
    ready_runtime_host_artifact_load_target, validate_runtime_host_producer_target_identity,
    RuntimeHostPumasLoadTargetError,
};

#[test]
fn load_target_request_uses_scheduler_selected_model_ref() {
    let request = validated_runtime_host_request();
    let pumas_request = build_runtime_host_artifact_load_target_request(&request)
        .expect("request builder must accept validated scheduler handoff");

    assert_eq!(pumas_request.model_ref.model_id, "juggernaut-xl-v10");
    assert_eq!(
        pumas_request.model_ref.selected_artifact_id.as_deref(),
        Some("diffusers-bundle")
    );
    assert_eq!(pumas_request.model_ref.selected_artifact_path, None);
    assert_eq!(pumas_request.caller_observed_entry_path, None);
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
fn intent_requirement_is_local_only_and_preserves_selected_identity() {
    let request = validated_runtime_host_request();
    let selected_model_ref = request
        .as_ref()
        .handoff
        .dispatch_decision
        .as_ref()
        .expect("fixture has dispatch decision")
        .selected_model_ref
        .clone();
    let requirement = build_runtime_host_model_requirement(&selected_model_ref)
        .expect("selected model should form a valid intent requirement");

    assert_eq!(
        requirement.acquisition_policy,
        pumas_library::intent::AcquisitionPolicy::LocalOnly
    );
    let pumas_library::intent::ModelSelector::LocalModel { model_ref } = requirement.selector
    else {
        panic!("runtime host must build a local-model requirement");
    };
    assert_eq!(model_ref.model_id, "juggernaut-xl-v10");
    assert_eq!(
        model_ref.selected_artifact_id.as_deref(),
        Some("diffusers-bundle")
    );
    assert_eq!(model_ref.selected_artifact_path, None);
}

#[test]
fn intent_requirement_does_not_promote_legacy_path_constraint() {
    let request = validated_runtime_host_request();
    let mut selected_model_ref = request
        .as_ref()
        .handoff
        .dispatch_decision
        .as_ref()
        .expect("fixture has dispatch decision")
        .selected_model_ref
        .clone();
    selected_model_ref.selected_artifact_path = Some("/producer/private/model".to_string());

    let requirement = build_runtime_host_model_requirement(&selected_model_ref)
        .expect("selected model should form a valid intent requirement");
    let pumas_library::intent::ModelSelector::LocalModel { model_ref } = requirement.selector
    else {
        panic!("runtime host must build a local-model requirement");
    };

    assert_eq!(model_ref.selected_artifact_path, None);
    assert_eq!(
        model_ref.selected_artifact_id.as_deref(),
        Some("diffusers-bundle")
    );
}

#[test]
fn ready_load_target_response_returns_host_only_target() {
    let response = ResolveModelArtifactLoadTargetResponse {
        artifact_state: ModelArtifactState::Ready,
        entry_path_state: ModelEntryPathState::Ready,
        target: Some(PumasLibraryArtifactLoadTarget {
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
            package_facts_contract_version: Some(PACKAGE_FACTS_CONTRACT_VERSION),
        }),
        diagnostics: Vec::new(),
    };

    let target = ready_runtime_host_artifact_load_target(response)
        .expect("ready Pumas response must return host-only load target");

    assert_eq!(target.artifact_kind, ModelArtifactKind::DiffusersBundle);
    assert_eq!(target.storage_kind, ModelStorageKind::LibraryOwned);
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

#[test]
fn producer_target_model_mismatch_fails_before_identity_normalization() {
    let request = validated_runtime_host_request();
    let selected_model_ref = request
        .as_ref()
        .handoff
        .dispatch_decision
        .as_ref()
        .expect("fixture has dispatch decision")
        .selected_model_ref
        .clone();
    let mut target = ready_target();
    target.model_ref.model_id = "pumas://models/other-model".to_string();

    let error = validate_runtime_host_producer_target_identity(&selected_model_ref, &target)
        .expect_err("producer model mismatch must fail before normalization");

    assert!(matches!(
        error,
        RuntimeHostPumasLoadTargetError::ProducerModelMismatch { .. }
    ));
}

#[test]
fn producer_target_revision_mismatch_fails_before_identity_normalization() {
    let request = validated_runtime_host_request();
    let mut selected_model_ref = request
        .as_ref()
        .handoff
        .dispatch_decision
        .as_ref()
        .expect("fixture has dispatch decision")
        .selected_model_ref
        .clone();
    selected_model_ref.revision = Some("selected-revision".to_string());
    let mut target = ready_target();
    target.model_ref.revision = Some("producer-revision".to_string());

    let error = validate_runtime_host_producer_target_identity(&selected_model_ref, &target)
        .expect_err("producer revision mismatch must fail before normalization");

    assert!(matches!(
        error,
        RuntimeHostPumasLoadTargetError::ProducerRevisionMismatch { .. }
    ));
}

fn ready_target() -> PumasArtifactLoadTarget {
    PumasArtifactLoadTarget {
        model_ref: inference::PumasModelRef {
            model_id: "pumas://models/juggernaut-xl-v10".to_string(),
            revision: None,
            selected_artifact_id: Some("diffusers-bundle".to_string()),
            selected_artifact_path: Some("juggernaut-xl-v10/diffusers".to_string()),
            migration_diagnostics: Vec::new(),
        },
        artifact_kind: ModelArtifactKind::DiffusersBundle,
        local_load_path: "/host-only/pumas/juggernaut-xl-v10".to_string(),
        load_path_kind: InferenceLoadPathKind::Directory,
        library_root_id: Some("default".to_string()),
        storage_kind: ModelStorageKind::LibraryOwned,
        validation_state: inference::ModelValidationState::Valid,
        verification_source_fingerprint: None,
        verification_observed_from_cache_at: None,
        content_fingerprint: Some("sha256:abc".to_string()),
        package_facts_contract_version: Some(PACKAGE_FACTS_CONTRACT_VERSION),
    }
}

fn validated_runtime_host_request() -> ValidatedRuntimeHostExecutionRequest {
    let request: RuntimeHostExecutionRequest = serde_json::from_str(include_str!(
        "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host execution request fixture must decode");
    ValidatedRuntimeHostExecutionRequest::try_from(request)
        .expect("runtime host execution request fixture must validate")
}
