use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionRequest, ValidatedRuntimeHostExecutionRequest,
};
use pumas_library::models::{
    AssetValidationState, ModelArtifactState, ModelEntryPathState, PackageArtifactKind,
    PumasArtifactLoadPathKind, PumasArtifactLoadTarget, PumasArtifactLoadTargetDiagnostic,
    PumasArtifactLoadTargetDiagnosticCode, ResolveModelArtifactLoadTargetResponse, StorageKind,
    PACKAGE_FACTS_CONTRACT_VERSION,
};

use super::{
    build_model_artifact_load_target_request, ready_runtime_host_artifact_load_target,
    runtime_family, RuntimeHostLoadTargetResolver, RuntimeHostPumasLoadTargetError,
    RuntimeHostPumasLoadTargetResolver,
};

#[test]
fn load_target_request_uses_scheduler_selected_model_ref() {
    let request = validated_runtime_host_request();
    let handoff = &request.as_ref().handoff;
    let decision = handoff.dispatch_decision.as_ref().unwrap();
    let pumas_request = build_model_artifact_load_target_request(
        &decision.selected_model_ref,
        handoff.task_intent.task_type.as_str(),
        &runtime_family(decision),
    );

    assert_eq!(
        pumas_request.model_ref.model_id,
        "pumas://models/juggernaut-xl-v10"
    );
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
fn model_target_request_preserves_owner_fresh_policy_revision_and_runtime_fallback() {
    let request = validated_runtime_host_request();
    let mut decision = request.as_ref().handoff.dispatch_decision.clone().unwrap();
    decision.selected_runtime_variant_id = None;
    let mut selected = decision.selected_model_ref.clone();
    selected.selected_artifact_path = Some("relative/model/artifact".into());
    let request = build_model_artifact_load_target_request(
        &selected,
        "text_generation",
        &runtime_family(&decision),
    );
    assert_eq!(request.model_ref.revision, selected.revision);
    assert_eq!(
        request.model_ref.selected_artifact_path,
        selected.selected_artifact_path
    );
    assert_eq!(
        request.caller_observed_entry_path,
        selected.selected_artifact_path
    );
    assert_eq!(
        request.consumer.task_kind.as_deref(),
        Some("text_generation")
    );
    assert_eq!(
        request.consumer.runtime_family.as_deref(),
        Some("diffusers-pytorch")
    );
    assert_eq!(
        request.consumer.consumer_name,
        "pantograph-embedded-runtime"
    );
    assert!(matches!(
        request.resolution_mode,
        pumas_library::models::PumasArtifactLoadTargetResolutionMode::OwnerFresh
    ));
}

#[tokio::test]
async fn model_reference_resolvers_match_selected_wrappers_and_refuse_identity_changes() {
    use crate::runtime_host_package_facts::{
        RuntimeHostPackageFactsResolver, RuntimeHostPumasPackageFactsResolver,
    };
    use std::sync::Arc;
    let directory = tempfile::tempdir().unwrap();
    for path in [
        "launcher-data/metadata",
        "launcher-data/cache",
        "launcher-data/logs",
        "shared-resources/models",
    ] {
        std::fs::create_dir_all(directory.path().join(path)).unwrap();
    }
    let owner = Arc::new(
        crate::pumas_test_support::builder(directory.path())
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .unwrap(),
    );
    // Metadata-only Diffusers bundle, inspected by Pumas, never executed.
    let model_id = "diffusion/fixture/metadata-parity";
    let bundle = directory
        .path()
        .join("shared-resources/models")
        .join(model_id);
    for component in ["scheduler", "text_encoder", "tokenizer", "unet", "vae"] {
        std::fs::create_dir_all(bundle.join(component)).unwrap();
    }
    std::fs::write(bundle.join("model_index.json"),serde_json::json!({
        "_class_name":"StableDiffusionPipeline","_diffusers_version":"0.32.0",
        "scheduler":["diffusers","EulerDiscreteScheduler"],"text_encoder":["transformers","CLIPTextModel"],
        "tokenizer":["transformers","CLIPTokenizer"],"unet":["diffusers","UNet2DConditionModel"],"vae":["diffusers","AutoencoderKL"]
    }).to_string()).unwrap();
    std::fs::write(bundle.join("metadata.json"),serde_json::json!({
        "schema_version":2,"model_id":model_id,"family":"fixture","model_type":"diffusion",
        "official_name":"Metadata Parity","cleaned_name":"metadata-parity","source_path":bundle,"entry_path":bundle,
        "storage_kind":"library_owned","bundle_format":"diffusers_directory","pipeline_class":"StableDiffusionPipeline",
        "selected_artifact_id":"diffusers","import_state":"ready","validation_state":"valid",
        "pipeline_tag":"text-to-image","task_type_primary":"text-to-image","input_modalities":["text"],
        "output_modalities":["image"],"recommended_backend":"diffusers","runtime_engine_hints":["diffusers","pytorch"]
    }).to_string()).unwrap();
    owner.rebuild_model_index().await.unwrap();
    let model = owner.resolve_model_package_facts(model_id).await.unwrap();
    let selected = pantograph_dependency_planning::PumasModelRef {
        model_id: format!("pumas://models/{}", model.model_ref.model_id),
        revision: model.model_ref.revision.clone(),
        selected_artifact_id: model.model_ref.selected_artifact_id.clone(),
        selected_artifact_path: None,
        migration_diagnostics: Vec::new(),
    };
    let access = Arc::new(workflow_nodes::setup::PumasSelectorAccess::Owner(owner));
    let packages = RuntimeHostPumasPackageFactsResolver::new(access.clone());
    let targets = RuntimeHostPumasLoadTargetResolver::new(access);
    let selected_request = request_with_model(&selected);
    let direct_package = packages.resolve_for_model(&selected).await.unwrap();
    let wrapped_package = packages.resolve(&selected_request).await.unwrap();
    assert_eq!(
        serde_json::to_value(&direct_package).unwrap(),
        serde_json::to_value(wrapped_package).unwrap()
    );
    assert!(direct_package.uses_current_contract());
    assert!(direct_package.artifact.logical_size.is_some());
    assert!(!std::path::Path::new(&direct_package.artifact.entry_path).is_absolute());
    let direct_target = targets
        .resolve_for_model(&selected, "image_generation", "diffusers-pytorch.cuda")
        .await
        .unwrap();
    let wrapped_target = targets.resolve(&selected_request).await.unwrap();
    assert_eq!(
        serde_json::to_value(&direct_target).unwrap(),
        serde_json::to_value(wrapped_target).unwrap()
    );
    assert_eq!(direct_target.model_ref.selected_artifact_path, None);
    assert!(std::path::Path::new(&direct_target.local_load_path).is_absolute());
    for revision_change in [true, false] {
        let mut changed = selected.clone();
        if revision_change {
            changed.revision = Some("foreign-revision".into());
        } else {
            changed.selected_artifact_id = Some("foreign-artifact".into());
        }
        let request = request_with_model(&changed);
        assert!(packages.resolve_for_model(&changed).await.is_err());
        assert!(packages.resolve(&request).await.is_err());
        assert!(targets
            .resolve_for_model(&changed, "image_generation", "diffusers-pytorch.cuda")
            .await
            .is_err());
        assert!(targets.resolve(&request).await.is_err());
    }
}

fn request_with_model(
    model: &pantograph_dependency_planning::PumasModelRef,
) -> ValidatedRuntimeHostExecutionRequest {
    fn replace(value: &mut serde_json::Value, model: &serde_json::Value) {
        match value {
            serde_json::Value::Object(fields) if fields.contains_key("model_id") => {
                *value = model.clone();
            }
            serde_json::Value::Object(fields) => {
                for value in fields.values_mut() {
                    replace(value, model);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    replace(value, model);
                }
            }
            _ => {}
        }
    }
    let mut request = serde_json::to_value(validated_runtime_host_request().as_ref()).unwrap();
    replace(&mut request, &serde_json::to_value(model).unwrap());
    ValidatedRuntimeHostExecutionRequest::try_from(
        serde_json::from_value::<RuntimeHostExecutionRequest>(request).unwrap(),
    )
    .unwrap()
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
            package_facts_contract_version: Some(PACKAGE_FACTS_CONTRACT_VERSION),
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
        "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .expect("runtime host execution request fixture must decode");
    ValidatedRuntimeHostExecutionRequest::try_from(request)
        .expect("runtime host execution request fixture must validate")
}
