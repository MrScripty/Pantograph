use std::collections::BTreeMap;

use async_trait::async_trait;
use thiserror::Error;

use super::inference_interface_request::InferenceInterfaceGraphResolutionInput;
use super::inference_interface_resolver::{
    InferenceInterfaceResolverFacts, InferenceModelResolutionFacts, InferenceModelResolutionState,
};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum InferenceInterfaceFactsProviderError {
    #[error("failed to resolve inference interface facts: {0}")]
    Resolve(String),
}

#[async_trait]
pub trait InferenceInterfaceFactsProvider: std::fmt::Debug + Send + Sync {
    /// Resolve authoritative requirements independently of host readiness.
    /// Returning None preserves the existing dependency-provider path. Resolved
    /// requirements alone are never executable readiness evidence.
    async fn resolved_dependency_requirements(
        &self,
        _request: &pantograph_dependency_planning::ValidatedDependencyEnvironmentRequest,
    ) -> Result<
        Option<pantograph_dependency_planning::ValidatedDependencyEnvironmentResult>,
        InferenceInterfaceFactsProviderError,
    > {
        Ok(None)
    }

    async fn facts_for_resolution_inputs(
        &self,
        inputs: &[InferenceInterfaceGraphResolutionInput],
    ) -> Result<
        BTreeMap<String, InferenceInterfaceResolverFacts>,
        InferenceInterfaceFactsProviderError,
    >;
}

#[derive(Debug, Default)]
pub struct UnavailableInferenceInterfaceFactsProvider;

#[async_trait]
impl InferenceInterfaceFactsProvider for UnavailableInferenceInterfaceFactsProvider {
    async fn facts_for_resolution_inputs(
        &self,
        inputs: &[InferenceInterfaceGraphResolutionInput],
    ) -> Result<
        BTreeMap<String, InferenceInterfaceResolverFacts>,
        InferenceInterfaceFactsProviderError,
    > {
        Ok(inputs
            .iter()
            .map(|input| (input.node_id.clone(), missing_model_facts()))
            .collect())
    }
}

pub(crate) fn missing_model_facts() -> InferenceInterfaceResolverFacts {
    InferenceInterfaceResolverFacts {
        model: InferenceModelResolutionFacts {
            state: InferenceModelResolutionState::MissingModelFacts,
        },
        capability: None,
        runtimes: Vec::new(),
        estimate_hints: Vec::new(),
    }
}
