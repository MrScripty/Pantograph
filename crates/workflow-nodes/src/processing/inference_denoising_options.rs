//! Backend-owned denoising scheduler port options for canonical inference.
//!
//! This provider exposes fact-derived option rows for graph editors. It does
//! not decide execution policy and does not make explicit scheduler overrides
//! executable; the image planner remains the authority for accepted requests.

use async_trait::async_trait;
use inference::DenoisingSchedulerOptionId;
use node_engine::{
    ExecutorExtensions, NodeEngineError, PortOption, PortOptionAvailabilityState,
    PortOptionsProvider, PortOptionsQuery, PortOptionsResult,
};
use serde_json::Value;
use std::sync::Arc;

use crate::processing::inference::InferenceTask;
use crate::setup::{PumasSelectorAccess, PUMAS_SELECTOR_ACCESS};

const PROVIDER_CONTRACT_VERSION: u32 = 1;
const PUMAS_MODEL_REF_PREFIX: &str = "pumas://models/";
const OPTION_SUPPORT_NOT_IMPLEMENTED: &str = "explicit_override_not_implemented";
const DIAGNOSTIC_MISSING_SELECTED_MODEL_REF: &str = "missing_selected_model_ref";
const DIAGNOSTIC_MISSING_PACKAGE_FACTS: &str = "missing_package_facts";
const DIAGNOSTIC_MISSING_DIFFUSERS_EVIDENCE: &str = "missing_diffusers_evidence";
const DIAGNOSTIC_MISSING_SCHEDULER_COMPONENT: &str = "missing_scheduler_component";
const DIAGNOSTIC_UNSUPPORTED_SCHEDULER_CLASS: &str = "unsupported_scheduler_class";

/// Provides model/package-fact-derived denoising scheduler options.
pub(crate) struct DenoisingSchedulerOptionsProvider;

struct SchedulerOptionSpec {
    diffusers_class_name: &'static str,
    option_id: &'static str,
    label: &'static str,
}

const SCHEDULER_OPTION_SPECS: &[SchedulerOptionSpec] = &[
    SchedulerOptionSpec {
        diffusers_class_name: "EulerDiscreteScheduler",
        option_id: "euler_discrete",
        label: "Euler Discrete",
    },
    SchedulerOptionSpec {
        diffusers_class_name: "FlowMatchEulerDiscreteScheduler",
        option_id: "flow_match_euler_discrete",
        label: "Flow Match Euler Discrete",
    },
];

#[async_trait]
impl PortOptionsProvider for DenoisingSchedulerOptionsProvider {
    async fn query_options(
        &self,
        query: &PortOptionsQuery,
        extensions: &ExecutorExtensions,
    ) -> node_engine::Result<PortOptionsResult> {
        let Some(selected_model_id) = selected_model_id(query) else {
            return Ok(metadata_only_result(
                diagnostic_metadata(
                    DIAGNOSTIC_MISSING_SELECTED_MODEL_REF,
                    "denoising scheduler options require selectedModelRef provider context",
                ),
                Vec::new(),
            ));
        };

        let selector_access = extensions
            .get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
            .cloned()
            .ok_or_else(|| {
                NodeEngineError::ExecutionFailed(
                    "Pumas selector access not available for denoising scheduler options"
                        .to_string(),
                )
            })?;

        let package_facts = match selector_access
            .resolve_model_package_facts(&selected_model_id)
            .await
        {
            Ok(facts) => facts,
            Err(error) => {
                return Ok(metadata_only_result(
                    diagnostic_metadata(
                        DIAGNOSTIC_MISSING_PACKAGE_FACTS,
                        format!("failed to resolve Pumas package facts: {error}"),
                    ),
                    Vec::new(),
                ));
            }
        };

        let mut diagnostics = Vec::new();
        let options = scheduler_options_from_package_facts(&package_facts, &mut diagnostics);
        let options = filter_options(options, query.search.as_deref());
        let metadata = serde_json::json!({
            "providerContractVersion": PROVIDER_CONTRACT_VERSION,
            "selectedModelRef": selected_model_ref(query),
            "selectedModelId": selected_model_id,
            "packageFactsContractVersion": package_facts.package_facts_contract_version,
            "optionSupport": OPTION_SUPPORT_NOT_IMPLEMENTED,
            "diagnostics": diagnostics,
        });

        Ok(PortOptionsResult {
            total_count: options.len(),
            options,
            searchable: true,
            metadata: Some(metadata),
        })
    }
}

inventory::submit!(node_engine::PortQueryFn {
    node_type: "llm-inference",
    port_id: InferenceTask::PORT_DENOISING_SCHEDULER,
    provider: || Box::new(DenoisingSchedulerOptionsProvider),
});

fn selected_model_ref(query: &PortOptionsQuery) -> Option<&str> {
    query
        .context
        .as_ref()
        .and_then(|context| context.selected_model_ref.as_ref())
        .map(|model_ref| model_ref.as_str())
}

fn selected_model_id(query: &PortOptionsQuery) -> Option<String> {
    selected_model_ref(query)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .strip_prefix(PUMAS_MODEL_REF_PREFIX)
                .unwrap_or(value)
                .to_string()
        })
}

pub(crate) fn scheduler_options_from_package_facts(
    facts: &pumas_library::models::ResolvedModelPackageFacts,
    diagnostics: &mut Vec<Value>,
) -> Vec<PortOption> {
    let Some(diffusers) = facts.diffusers.as_ref() else {
        diagnostics.push(diagnostic_metadata(
            DIAGNOSTIC_MISSING_DIFFUSERS_EVIDENCE,
            "Pumas package facts do not include Diffusers evidence",
        ));
        return Vec::new();
    };

    let scheduler_components = diffusers
        .components
        .iter()
        .filter(|component| {
            component.role == pumas_library::models::DiffusersComponentRole::Scheduler
        })
        .collect::<Vec<_>>();

    if scheduler_components.is_empty() {
        diagnostics.push(diagnostic_metadata(
            DIAGNOSTIC_MISSING_SCHEDULER_COMPONENT,
            "Pumas Diffusers facts do not include a scheduler component",
        ));
        return Vec::new();
    }

    let mut options = Vec::new();
    for component in scheduler_components {
        let Some(class_name) = component.class_name.as_deref() else {
            diagnostics.push(diagnostic_metadata(
                DIAGNOSTIC_MISSING_SCHEDULER_COMPONENT,
                "Pumas scheduler component is missing a Diffusers class name",
            ));
            continue;
        };

        let Some(spec) = scheduler_option_spec(class_name) else {
            diagnostics.push(serde_json::json!({
                "code": DIAGNOSTIC_UNSUPPORTED_SCHEDULER_CLASS,
                "message": format!("Diffusers scheduler class '{class_name}' is not exposed as a selectable denoising_scheduler option"),
                "schedulerClass": class_name,
            }));
            continue;
        };

        if DenoisingSchedulerOptionId::parse(spec.option_id).is_err() {
            diagnostics.push(serde_json::json!({
                "code": DIAGNOSTIC_UNSUPPORTED_SCHEDULER_CLASS,
                "message": format!("Denoising scheduler option id '{}' is invalid", spec.option_id),
                "schedulerClass": class_name,
            }));
            continue;
        }

        options.push(PortOption {
            value: serde_json::json!(spec.option_id),
            label: spec.label.to_string(),
            description: Some("Detected from Pumas Diffusers package facts".to_string()),
            metadata: Some(serde_json::json!({
                "providerContractVersion": PROVIDER_CONTRACT_VERSION,
                "source": "pumas_package_facts",
                "diffusersClassName": spec.diffusers_class_name,
                "componentConfigPath": component.config_path,
                "componentRelativePath": component.relative_path,
                "optionSupport": OPTION_SUPPORT_NOT_IMPLEMENTED,
            })),
            disabled: true,
            unavailable_state: Some(PortOptionAvailabilityState::NotImplemented),
            unavailable_reason_code: Some(OPTION_SUPPORT_NOT_IMPLEMENTED.to_string()),
            unavailable_reason: Some(
                "Explicit denoising_scheduler selection is not executable in the current image planner; leave this input unset to use the model default."
                    .to_string(),
            ),
        });
    }

    options
}

fn scheduler_option_spec(class_name: &str) -> Option<&'static SchedulerOptionSpec> {
    SCHEDULER_OPTION_SPECS
        .iter()
        .find(|spec| spec.diffusers_class_name == class_name)
}

pub(crate) fn filter_options(options: Vec<PortOption>, search: Option<&str>) -> Vec<PortOption> {
    let Some(search) = search.map(str::trim).filter(|value| !value.is_empty()) else {
        return options;
    };
    let search = search.to_ascii_lowercase();
    options
        .into_iter()
        .filter(|option| {
            option.label.to_ascii_lowercase().contains(&search)
                || option
                    .value
                    .as_str()
                    .is_some_and(|value| value.to_ascii_lowercase().contains(&search))
                || option
                    .metadata
                    .as_ref()
                    .and_then(|metadata| metadata.get("diffusersClassName"))
                    .and_then(Value::as_str)
                    .is_some_and(|value| value.to_ascii_lowercase().contains(&search))
        })
        .collect()
}

fn metadata_only_result(metadata: Value, options: Vec<PortOption>) -> PortOptionsResult {
    PortOptionsResult {
        total_count: options.len(),
        options,
        searchable: true,
        metadata: Some(serde_json::json!({
            "providerContractVersion": PROVIDER_CONTRACT_VERSION,
            "optionSupport": OPTION_SUPPORT_NOT_IMPLEMENTED,
            "diagnostics": [metadata],
        })),
    }
}

fn diagnostic_metadata(code: impl Into<String>, message: impl Into<String>) -> Value {
    serde_json::json!({
        "code": code.into(),
        "message": message.into(),
    })
}
