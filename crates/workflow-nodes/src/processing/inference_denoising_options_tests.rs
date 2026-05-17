use super::inference_denoising_options::{
    filter_options, scheduler_options_from_package_facts, DenoisingSchedulerOptionsProvider,
};
use crate::setup::{PumasSelectorAccess, PUMAS_SELECTOR_ACCESS};
use node_engine::{
    ExecutorExtensions, PortOptionAvailabilityState, PortOptionsContextId, PortOptionsProvider,
    PortOptionsQuery, PortOptionsQueryContext,
};
use serde_json::Value;
use std::sync::Arc;
use tempfile::TempDir;

const TEST_MODEL_ID: &str = "diffusion/imported/test-bundle";
const PUMAS_MODEL_REF_PREFIX: &str = "pumas://models/";
const OPTION_SUPPORT_NOT_IMPLEMENTED: &str = "explicit_override_not_implemented";
const DIAGNOSTIC_MISSING_SELECTED_MODEL_REF: &str = "missing_selected_model_ref";
const DIAGNOSTIC_MISSING_DIFFUSERS_EVIDENCE: &str = "missing_diffusers_evidence";

fn create_test_env() -> TempDir {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    std::fs::create_dir_all(temp_dir.path().join("launcher-data")).unwrap();
    std::fs::create_dir_all(temp_dir.path().join("launcher-data/metadata")).unwrap();
    std::fs::create_dir_all(temp_dir.path().join("launcher-data/cache")).unwrap();
    std::fs::create_dir_all(temp_dir.path().join("launcher-data/logs")).unwrap();
    std::fs::create_dir_all(temp_dir.path().join("shared-resources/models")).unwrap();
    temp_dir
}

fn write_test_diffusers_bundle(root: &std::path::Path) {
    std::fs::create_dir_all(root.join("scheduler")).unwrap();
    std::fs::create_dir_all(root.join("text_encoder")).unwrap();
    std::fs::create_dir_all(root.join("tokenizer")).unwrap();
    std::fs::create_dir_all(root.join("unet")).unwrap();
    std::fs::create_dir_all(root.join("vae")).unwrap();
    std::fs::write(
        root.join("model_index.json"),
        serde_json::json!({
            "_class_name": "StableDiffusionPipeline",
            "scheduler": ["diffusers", "EulerDiscreteScheduler"],
            "text_encoder": ["transformers", "CLIPTextModel"],
            "tokenizer": ["transformers", "CLIPTokenizer"],
            "unet": ["diffusers", "UNet2DConditionModel"],
            "vae": ["diffusers", "AutoencoderKL"]
        })
        .to_string(),
    )
    .unwrap();
}

fn write_imported_diffusion_metadata(model_dir: &std::path::Path, entry_path: &std::path::Path) {
    std::fs::create_dir_all(model_dir).unwrap();
    std::fs::write(
        model_dir.join("metadata.json"),
        serde_json::json!({
            "schema_version": 2,
            "model_id": TEST_MODEL_ID,
            "family": "imported",
            "model_type": "diffusion",
            "official_name": "test-bundle",
            "cleaned_name": "test-bundle",
            "source_path": entry_path.display().to_string(),
            "entry_path": entry_path.display().to_string(),
            "storage_kind": "external_reference",
            "bundle_format": "diffusers_directory",
            "pipeline_class": "StableDiffusionPipeline",
            "import_state": "ready",
            "validation_state": "valid",
            "pipeline_tag": "text-to-image",
            "task_type_primary": "text-to-image",
            "input_modalities": ["text"],
            "output_modalities": ["image"],
            "task_classification_source": "external-diffusers-import",
            "task_classification_confidence": 1.0,
            "model_type_resolution_source": "external-diffusers-import",
            "model_type_resolution_confidence": 1.0,
            "recommended_backend": "diffusers",
            "runtime_engine_hints": ["diffusers", "pytorch"]
        })
        .to_string(),
    )
    .unwrap();
}

async fn provider_with_diffusers_bundle() -> (
    DenoisingSchedulerOptionsProvider,
    ExecutorExtensions,
    TempDir,
) {
    let temp_dir = create_test_env();
    let bundle_root = temp_dir.path().join("external/tiny-sd-turbo");
    write_test_diffusers_bundle(&bundle_root);
    let model_dir = temp_dir
        .path()
        .join("shared-resources/models/diffusion/imported/test-bundle");
    write_imported_diffusion_metadata(&model_dir, &bundle_root);

    let api = Arc::new(
        pumas_library::PumasApi::builder(temp_dir.path())
            .build()
            .await
            .unwrap(),
    );
    api.rebuild_model_index().await.unwrap();

    let mut extensions = ExecutorExtensions::new();
    extensions.set(
        PUMAS_SELECTOR_ACCESS,
        Arc::new(PumasSelectorAccess::Owner(api)),
    );

    (DenoisingSchedulerOptionsProvider, extensions, temp_dir)
}

fn query_with_model_ref() -> PortOptionsQuery {
    PortOptionsQuery {
        context: Some(PortOptionsQueryContext {
            selected_model_ref: Some(
                PortOptionsContextId::new(format!("{PUMAS_MODEL_REF_PREFIX}{TEST_MODEL_ID}"))
                    .unwrap(),
            ),
            task_kind: Some(PortOptionsContextId::new("image_generation").unwrap()),
            ..PortOptionsQueryContext::default()
        }),
        ..PortOptionsQuery::default()
    }
}

fn package_facts_with_scheduler_class(
    scheduler_class: &str,
) -> pumas_library::models::ResolvedModelPackageFacts {
    serde_json::from_value(serde_json::json!({
        "package_facts_contract_version": 2,
        "model_ref": {
            "model_ref_contract_version": 1,
            "model_id": TEST_MODEL_ID
        },
        "artifact": {
            "artifact_kind": "diffusers_bundle",
            "entry_path": TEST_MODEL_ID,
            "storage_kind": "library_owned",
            "validation_state": "valid"
        },
        "diffusers": {
            "status": "present",
            "pipeline_class": "StableDiffusionPipeline",
            "task": {
                "pipeline_tag": "text-to-image",
                "task_type_primary": "text-to-image",
                "input_modalities": ["text"],
                "output_modalities": ["image"]
            },
            "components": [
                {
                    "role": "scheduler",
                    "status": "present",
                    "relative_path": "scheduler",
                    "source_library": "diffusers",
                    "class_name": scheduler_class,
                    "config_path": "scheduler/scheduler_config.json"
                }
            ]
        },
        "task": {
            "pipeline_tag": "text-to-image",
            "task_type_primary": "text-to-image",
            "input_modalities": ["text"],
            "output_modalities": ["image"]
        },
        "generation_defaults": {
            "status": "missing"
        },
        "custom_code": {
            "requires_custom_code": false
        },
        "backend_hints": {
            "accepted": ["diffusers"],
            "raw": ["diffusers"]
        }
    }))
    .expect("package facts fixture should decode")
}

#[test]
fn package_facts_create_disabled_fact_derived_scheduler_option() {
    let facts = package_facts_with_scheduler_class("EulerDiscreteScheduler");
    let mut diagnostics = Vec::new();

    let options = scheduler_options_from_package_facts(&facts, &mut diagnostics);

    assert!(diagnostics.is_empty());
    let option = options.first().expect("scheduler option");
    assert_eq!(option.value, serde_json::json!("euler_discrete"));
    assert_eq!(option.label, "Euler Discrete");
    assert!(option.disabled);
    assert_eq!(
        option.unavailable_state,
        Some(PortOptionAvailabilityState::NotImplemented)
    );
    assert_eq!(
        option.unavailable_reason_code.as_deref(),
        Some(OPTION_SUPPORT_NOT_IMPLEMENTED)
    );
    assert_eq!(
        option
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("diffusersClassName")),
        Some(&serde_json::json!("EulerDiscreteScheduler"))
    );
}

#[test]
fn filter_options_matches_class_label_or_option_id() {
    let facts = package_facts_with_scheduler_class("EulerDiscreteScheduler");
    let mut diagnostics = Vec::new();
    let options = scheduler_options_from_package_facts(&facts, &mut diagnostics);

    let filtered = filter_options(options, Some("euler_discrete"));

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].value, serde_json::json!("euler_discrete"));
}

#[tokio::test]
async fn query_options_reports_missing_diffusers_evidence_from_current_pumas_import_shape() {
    let (provider, extensions, _temp_dir) = provider_with_diffusers_bundle().await;

    let result = provider
        .query_options(&query_with_model_ref(), &extensions)
        .await
        .expect("scheduler options should resolve with metadata diagnostics");

    assert_eq!(result.total_count, 0);
    assert!(result.options.is_empty());
    assert_eq!(
        first_diagnostic_code(&result.metadata),
        DIAGNOSTIC_MISSING_DIFFUSERS_EVIDENCE
    );
}

#[tokio::test]
async fn query_options_reports_missing_selected_model_ref() {
    let temp_dir = create_test_env();
    let api = Arc::new(
        pumas_library::PumasApi::builder(temp_dir.path())
            .build()
            .await
            .unwrap(),
    );
    let mut extensions = ExecutorExtensions::new();
    extensions.set(
        PUMAS_SELECTOR_ACCESS,
        Arc::new(PumasSelectorAccess::Owner(api)),
    );

    let result = DenoisingSchedulerOptionsProvider
        .query_options(&PortOptionsQuery::default(), &extensions)
        .await
        .expect("missing context should return metadata diagnostics");

    assert_eq!(result.total_count, 0);
    assert!(result.options.is_empty());
    assert_eq!(
        first_diagnostic_code(&result.metadata),
        DIAGNOSTIC_MISSING_SELECTED_MODEL_REF
    );
}

fn first_diagnostic_code(metadata: &Option<Value>) -> &str {
    metadata
        .as_ref()
        .and_then(|metadata| metadata.get("diagnostics"))
        .and_then(Value::as_array)
        .and_then(|diagnostics| diagnostics.first())
        .and_then(|diagnostic| diagnostic.get("code"))
        .and_then(Value::as_str)
        .expect("first diagnostic code")
}
