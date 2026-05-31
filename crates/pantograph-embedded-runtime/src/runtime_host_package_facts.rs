use std::sync::Arc;

use async_trait::async_trait;
use inference::ResolvedModelPackageFacts;
use pantograph_runtime_host_contracts::ValidatedRuntimeHostExecutionRequest;
use pumas_library::PumasError;
use thiserror::Error;

pub(crate) struct RuntimeHostPumasPackageFactsResolver {
    pumas_api: Arc<pumas_library::PumasApi>,
}

#[async_trait]
pub(crate) trait RuntimeHostPackageFactsResolver: Send + Sync {
    async fn resolve(
        &self,
        request: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<ResolvedModelPackageFacts, RuntimeHostPumasPackageFactsError>;
}

impl RuntimeHostPumasPackageFactsResolver {
    pub(crate) fn new(pumas_api: Arc<pumas_library::PumasApi>) -> Self {
        Self { pumas_api }
    }
}

#[async_trait]
impl RuntimeHostPackageFactsResolver for RuntimeHostPumasPackageFactsResolver {
    async fn resolve(
        &self,
        request: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<ResolvedModelPackageFacts, RuntimeHostPumasPackageFactsError> {
        let selected_model_ref = selected_pumas_model_ref(request)?;
        let raw_facts = self
            .pumas_api
            .resolve_model_package_facts(selected_model_ref.model_id.as_str())
            .await?;
        let package_facts = normalize_runtime_host_package_fact_identity(
            selected_model_ref,
            decode_pumas_package_facts(raw_facts)?,
        );
        validate_runtime_host_package_facts(selected_model_ref, package_facts)
    }
}

fn selected_pumas_model_ref(
    request: &ValidatedRuntimeHostExecutionRequest,
) -> Result<&pantograph_dependency_planning::PumasModelRef, RuntimeHostPumasPackageFactsError> {
    request
        .as_ref()
        .handoff
        .dispatch_decision
        .as_ref()
        .map(|decision| &decision.selected_model_ref)
        .ok_or(RuntimeHostPumasPackageFactsError::MissingDispatchDecision)
}

fn validate_runtime_host_package_facts(
    selected_model_ref: &pantograph_dependency_planning::PumasModelRef,
    package_facts: ResolvedModelPackageFacts,
) -> Result<ResolvedModelPackageFacts, RuntimeHostPumasPackageFactsError> {
    if !package_facts.uses_current_contract() {
        return Err(
            RuntimeHostPumasPackageFactsError::StalePackageFactsContract {
                model_id: selected_model_ref.model_id.clone(),
                package_facts_contract_version: package_facts.package_facts_contract_version,
            },
        );
    }
    if selected_model_ref.selected_artifact_id.is_some()
        && package_facts.model_ref.selected_artifact_id != selected_model_ref.selected_artifact_id
    {
        return Err(
            RuntimeHostPumasPackageFactsError::SelectedArtifactMismatch {
                model_id: selected_model_ref.model_id.clone(),
                selected_artifact_id: selected_model_ref.selected_artifact_id.clone(),
                package_artifact_id: package_facts.model_ref.selected_artifact_id.clone(),
            },
        );
    }
    Ok(package_facts)
}

fn normalize_runtime_host_package_fact_identity(
    selected_model_ref: &pantograph_dependency_planning::PumasModelRef,
    mut package_facts: ResolvedModelPackageFacts,
) -> ResolvedModelPackageFacts {
    package_facts.artifact.entry_path = runtime_host_package_fact_entry_path(selected_model_ref);
    package_facts.model_ref.selected_artifact_path = selected_model_ref
        .selected_artifact_path
        .as_deref()
        .filter(|path| is_path_free_artifact_entry(path))
        .map(str::to_string);
    package_facts
}

fn runtime_host_package_fact_entry_path(
    selected_model_ref: &pantograph_dependency_planning::PumasModelRef,
) -> String {
    selected_model_ref
        .selected_artifact_path
        .as_deref()
        .filter(|path| is_path_free_artifact_entry(path))
        .map(str::to_string)
        .unwrap_or_else(|| path_free_model_entry_path(&selected_model_ref.model_id))
}

fn path_free_model_entry_path(model_id: &str) -> String {
    model_id
        .strip_prefix("pumas://models/")
        .unwrap_or(model_id)
        .trim_matches('/')
        .to_string()
}

fn is_path_free_artifact_entry(path: &str) -> bool {
    let trimmed = path.trim();
    !trimmed.is_empty()
        && !std::path::Path::new(trimmed).is_absolute()
        && !trimmed
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
}

fn decode_pumas_package_facts(
    facts: pumas_library::models::ResolvedModelPackageFacts,
) -> Result<ResolvedModelPackageFacts, RuntimeHostPumasPackageFactsError> {
    let mut value = serde_json::to_value(facts).map_err(|error| {
        RuntimeHostPumasPackageFactsError::PackageFactsDecodeFailed {
            message: error.to_string(),
        }
    })?;
    strip_pumas_model_ref_contract_versions(&mut value);
    serde_json::from_value(value).map_err(|error| {
        RuntimeHostPumasPackageFactsError::PackageFactsDecodeFailed {
            message: error.to_string(),
        }
    })
}

fn strip_pumas_model_ref_contract_versions(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            if map.contains_key("model_id") {
                map.remove("model_ref_contract_version");
            }
            for child in map.values_mut() {
                strip_pumas_model_ref_contract_versions(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_pumas_model_ref_contract_versions(item);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Error)]
pub(crate) enum RuntimeHostPumasPackageFactsError {
    #[error("runtime host execution request is missing scheduler dispatch decision")]
    MissingDispatchDecision,
    #[error("Pumas package facts could not be decoded into the inference contract: {message}")]
    PackageFactsDecodeFailed { message: String },
    #[error(
        "Pumas package facts for model '{model_id}' use stale contract version {package_facts_contract_version}"
    )]
    StalePackageFactsContract {
        model_id: String,
        package_facts_contract_version: u32,
    },
    #[error(
        "Pumas package facts for model '{model_id}' do not match selected artifact {selected_artifact_id:?}; got {package_artifact_id:?}"
    )]
    SelectedArtifactMismatch {
        model_id: String,
        selected_artifact_id: Option<String>,
        package_artifact_id: Option<String>,
    },
    #[error(transparent)]
    Pumas(#[from] PumasError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_runtime_host_contracts::{
        RuntimeHostExecutionRequest, ValidatedRuntimeHostExecutionRequest,
    };

    #[test]
    fn selected_model_ref_comes_from_scheduler_dispatch_decision() {
        let request = validated_runtime_host_request();

        let model_ref =
            selected_pumas_model_ref(&request).expect("validated fixture has dispatch decision");

        assert_eq!(model_ref.model_id, "pumas://models/juggernaut-xl-v10");
        assert_eq!(
            model_ref.selected_artifact_id.as_deref(),
            Some("diffusers-bundle")
        );
        assert_eq!(model_ref.selected_artifact_path, None);
    }

    #[test]
    fn decoded_package_facts_strip_pumas_model_ref_contract_versions() {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../inference/tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
        ))
        .expect("fixture should parse");
        value["model_ref"]["model_ref_contract_version"] = serde_json::json!(2);
        let raw_facts: pumas_library::models::ResolvedModelPackageFacts =
            serde_json::from_value(value).expect("fixture should decode into Pumas facts");

        let package_facts =
            decode_pumas_package_facts(raw_facts).expect("Pumas facts should decode");

        assert_eq!(
            package_facts.model_ref.model_id,
            "image/stable-diffusion/tiny-sd"
        );
        assert!(package_facts.uses_current_contract());
    }

    #[test]
    fn stale_package_facts_contract_fails_closed() {
        let request = validated_runtime_host_request();
        let selected_model_ref = selected_pumas_model_ref(&request).expect("selected model ref");
        let mut package_facts = image_package_facts_for_request(selected_model_ref);
        package_facts.package_facts_contract_version = 1;

        let error = validate_runtime_host_package_facts(selected_model_ref, package_facts)
            .expect_err("stale package facts must fail closed");

        assert!(matches!(
            error,
            RuntimeHostPumasPackageFactsError::StalePackageFactsContract {
                model_id,
                package_facts_contract_version: 1
            } if model_id == "pumas://models/juggernaut-xl-v10"
        ));
    }

    #[test]
    fn selected_artifact_mismatch_fails_closed() {
        let request = validated_runtime_host_request();
        let selected_model_ref = selected_pumas_model_ref(&request).expect("selected model ref");
        let mut package_facts = image_package_facts_for_request(selected_model_ref);
        package_facts.model_ref.selected_artifact_id = Some("other-artifact".to_string());

        let error = validate_runtime_host_package_facts(selected_model_ref, package_facts)
            .expect_err("artifact mismatch must fail closed");

        assert!(matches!(
            error,
            RuntimeHostPumasPackageFactsError::SelectedArtifactMismatch {
                selected_artifact_id,
                package_artifact_id,
                ..
            } if selected_artifact_id.as_deref() == Some("diffusers-bundle")
                && package_artifact_id.as_deref() == Some("other-artifact")
        ));
    }

    #[test]
    fn runtime_host_package_fact_identity_removes_owner_local_entry_paths() {
        let request = validated_runtime_host_request();
        let selected_model_ref = selected_pumas_model_ref(&request).expect("selected model ref");
        let mut package_facts = image_package_facts_for_request(selected_model_ref);
        package_facts.artifact.entry_path = "/host-only/pumas/juggernaut-xl-v10".to_string();
        package_facts.model_ref.selected_artifact_path =
            Some("/host-only/pumas/juggernaut-xl-v10".to_string());

        let normalized =
            normalize_runtime_host_package_fact_identity(selected_model_ref, package_facts);

        assert_eq!(normalized.artifact.entry_path, "juggernaut-xl-v10");
        assert_eq!(normalized.model_ref.selected_artifact_path, None);
    }

    #[test]
    fn runtime_host_package_fact_identity_preserves_path_free_selected_artifact_path() {
        let request = validated_runtime_host_request();
        let selected_model_ref = selected_pumas_model_ref(&request).expect("selected model ref");
        let mut selected_model_ref = selected_model_ref.clone();
        selected_model_ref.selected_artifact_path =
            Some("image/stable-diffusion/tiny-sd/diffusers".to_string());
        let mut package_facts = image_package_facts_for_request(&selected_model_ref);
        package_facts.artifact.entry_path = "/host-only/pumas/tiny-sd".to_string();

        let normalized =
            normalize_runtime_host_package_fact_identity(&selected_model_ref, package_facts);

        assert_eq!(
            normalized.artifact.entry_path,
            "image/stable-diffusion/tiny-sd/diffusers"
        );
        assert_eq!(
            normalized.model_ref.selected_artifact_path.as_deref(),
            Some("image/stable-diffusion/tiny-sd/diffusers")
        );
    }

    fn validated_runtime_host_request() -> ValidatedRuntimeHostExecutionRequest {
        let request: RuntimeHostExecutionRequest = serde_json::from_str(include_str!(
            "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
        ))
        .expect("runtime host execution request fixture must decode");
        ValidatedRuntimeHostExecutionRequest::try_from(request)
            .expect("runtime host execution request fixture must validate")
    }

    fn image_package_facts_for_request(
        selected_model_ref: &pantograph_dependency_planning::PumasModelRef,
    ) -> ResolvedModelPackageFacts {
        let mut package_facts: ResolvedModelPackageFacts = serde_json::from_str(include_str!(
            "../../inference/tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
        ))
        .expect("image package facts fixture should decode");
        package_facts.model_ref = inference::PumasModelRef {
            model_id: selected_model_ref.model_id.clone(),
            revision: selected_model_ref.revision.clone(),
            selected_artifact_id: selected_model_ref.selected_artifact_id.clone(),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        };
        package_facts
    }
}
