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
    }
}
