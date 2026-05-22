//! Planned inference contracts consumed by inference nodes.
//!
//! Node-engine forwards validated graph intent through host-owned execution
//! traits. Hosts own scheduler plan lookup, Pumas artifact resolution, runtime
//! launch facts, and backend execution.

use std::collections::HashMap;

use async_trait::async_trait;
use inference::{BackendExecutionDecision, InferenceTaskId};
use thiserror::Error;

/// Inference execution decisions available for one workflow run.
#[derive(Debug, Clone)]
pub struct PlannedInferenceDecisionContext {
    workflow_run_id: String,
    node_decisions: HashMap<String, BackendExecutionDecision>,
}

impl PlannedInferenceDecisionContext {
    pub fn new(
        workflow_run_id: impl Into<String>,
        node_decisions: HashMap<String, BackendExecutionDecision>,
    ) -> Result<Self, PlannedInferenceDecisionContextError> {
        let workflow_run_id = validate_required_text("workflow_run_id", workflow_run_id.into())?;
        if node_decisions.is_empty() {
            return Err(PlannedInferenceDecisionContextError::MissingDecisions);
        }
        for node_id in node_decisions.keys() {
            validate_required_text("node_id", node_id.clone())?;
        }

        Ok(Self {
            workflow_run_id,
            node_decisions,
        })
    }

    #[must_use]
    pub fn workflow_run_id(&self) -> &str {
        &self.workflow_run_id
    }

    pub fn decision_for_node(
        &self,
        workflow_run_id: &str,
        node_id: &str,
        expected_task_id: InferenceTaskId,
    ) -> Result<&BackendExecutionDecision, PlannedInferenceDecisionContextError> {
        if self.workflow_run_id != workflow_run_id {
            return Err(PlannedInferenceDecisionContextError::StaleRunContext {
                expected_workflow_run_id: workflow_run_id.to_string(),
                actual_workflow_run_id: self.workflow_run_id.clone(),
            });
        }

        let decision = self.node_decisions.get(node_id).ok_or_else(|| {
            PlannedInferenceDecisionContextError::MissingNodeDecision {
                workflow_run_id: workflow_run_id.to_string(),
                node_id: node_id.to_string(),
            }
        })?;
        if decision.selected_task_id.as_ref() != Some(&expected_task_id) {
            return Err(PlannedInferenceDecisionContextError::TaskMismatch {
                node_id: node_id.to_string(),
                expected_task_id,
                actual_task_id: decision.selected_task_id.clone(),
            });
        }
        Ok(decision)
    }
}

/// Host-owned planned execution boundary for inference nodes.
///
/// Node-engine passes run/node correlation and user intent through this trait.
/// Hosts own scheduler-plan lookup, Pumas load-target resolution, runtime
/// launch facts, and backend execution.
#[async_trait]
pub trait PlannedInferenceExecutionHost: Send + Sync {
    async fn generate_image(
        &self,
        request: PlannedImageGenerationRequest,
    ) -> Result<inference::ImageGenerationResult, PlannedInferenceExecutionError>;
}

/// Path-free image-generation request sent from node-engine to the host.
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct PlannedImageGenerationRequest {
    workflow_run_id: String,
    node_id: String,
    request_id: String,
    image_request: inference::ImageGenerationRequest,
}

impl PlannedImageGenerationRequest {
    pub fn new(
        workflow_run_id: impl Into<String>,
        node_id: impl Into<String>,
        request_id: impl Into<String>,
        image_request: inference::ImageGenerationRequest,
    ) -> Result<Self, PlannedImageGenerationRequestError> {
        Ok(Self {
            workflow_run_id: validate_planned_request_text(
                "workflow_run_id",
                workflow_run_id.into(),
            )?,
            node_id: validate_planned_request_text("node_id", node_id.into())?,
            request_id: validate_planned_request_text("request_id", request_id.into())?,
            image_request,
        })
    }

    #[must_use]
    pub fn workflow_run_id(&self) -> &str {
        &self.workflow_run_id
    }

    #[must_use]
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    #[must_use]
    pub fn image_request(&self) -> &inference::ImageGenerationRequest {
        &self.image_request
    }

    #[must_use]
    pub fn into_image_request(self) -> inference::ImageGenerationRequest {
        self.image_request
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlannedImageGenerationRequestError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PlannedInferenceExecutionError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error(
        "planned inference execution failed for workflow run '{workflow_run_id}', node '{node_id}', request '{request_id}': {message}"
    )]
    ExecutionFailed {
        workflow_run_id: String,
        node_id: String,
        request_id: String,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl PlannedInferenceExecutionError {
    pub fn execution_failed(
        request: &PlannedImageGenerationRequest,
        message: impl Into<String>,
    ) -> Self {
        Self::ExecutionFailed {
            workflow_run_id: request.workflow_run_id().to_string(),
            node_id: request.node_id().to_string(),
            request_id: request.request_id().to_string(),
            message: message.into(),
            source: None,
        }
    }

    pub fn execution_failed_with_source(
        request: &PlannedImageGenerationRequest,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::ExecutionFailed {
            workflow_run_id: request.workflow_run_id().to_string(),
            node_id: request.node_id().to_string(),
            request_id: request.request_id().to_string(),
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlannedInferenceDecisionContextError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error("planned inference context must contain at least one node decision")]
    MissingDecisions,
    #[error(
        "planned inference context belongs to workflow run '{actual_workflow_run_id}', not '{expected_workflow_run_id}'"
    )]
    StaleRunContext {
        expected_workflow_run_id: String,
        actual_workflow_run_id: String,
    },
    #[error(
        "workflow run '{workflow_run_id}' has no planned inference decision for node '{node_id}'"
    )]
    MissingNodeDecision {
        workflow_run_id: String,
        node_id: String,
    },
    #[error("planned inference decision for node '{node_id}' has task mismatch")]
    TaskMismatch {
        node_id: String,
        expected_task_id: InferenceTaskId,
        actual_task_id: Option<InferenceTaskId>,
    },
}

fn validate_required_text(
    field: &'static str,
    value: String,
) -> Result<String, PlannedInferenceDecisionContextError> {
    let value = value.trim();
    if value.is_empty() {
        Err(PlannedInferenceDecisionContextError::MissingField { field })
    } else {
        Ok(value.to_string())
    }
}

fn validate_planned_request_text(
    field: &'static str,
    value: String,
) -> Result<String, PlannedImageGenerationRequestError> {
    let value = value.trim();
    if value.is_empty() {
        Err(PlannedImageGenerationRequestError::MissingField { field })
    } else {
        Ok(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inference::{
        BackendExecutionDecision, BackendId, DeviceResolutionDecision, InferenceDeviceClass,
        InferenceDevicePolicy, RuntimeVariantId,
    };

    fn backend_decision(task_id: Option<InferenceTaskId>) -> BackendExecutionDecision {
        let runtime_variant_id =
            RuntimeVariantId::parse("pytorch.cuda").expect("runtime variant id");
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
            selected_task_id: task_id,
            selected_model_ref: None,
            diagnostics: Vec::new(),
            dependency_readiness: Vec::new(),
            selection_policy_trace: None,
        }
    }

    #[test]
    fn planned_context_returns_matching_node_decision() {
        let context = PlannedInferenceDecisionContext::new(
            "run-a",
            HashMap::from([(
                "image-node-1".to_string(),
                backend_decision(Some(InferenceTaskId::ImageGeneration)),
            )]),
        )
        .expect("planned context");

        let decision = context
            .decision_for_node("run-a", "image-node-1", InferenceTaskId::ImageGeneration)
            .expect("node decision");

        assert_eq!(decision.selected_backend_id.as_str(), "pytorch");
    }

    #[test]
    fn planned_context_rejects_stale_run_id() {
        let context = PlannedInferenceDecisionContext::new(
            "run-a",
            HashMap::from([(
                "image-node-1".to_string(),
                backend_decision(Some(InferenceTaskId::ImageGeneration)),
            )]),
        )
        .expect("planned context");

        let error = context
            .decision_for_node("run-b", "image-node-1", InferenceTaskId::ImageGeneration)
            .expect_err("stale run id must fail");

        assert!(matches!(
            error,
            PlannedInferenceDecisionContextError::StaleRunContext { .. }
        ));
    }

    #[test]
    fn planned_context_rejects_missing_node_decision() {
        let context = PlannedInferenceDecisionContext::new(
            "run-a",
            HashMap::from([(
                "image-node-1".to_string(),
                backend_decision(Some(InferenceTaskId::ImageGeneration)),
            )]),
        )
        .expect("planned context");

        let error = context
            .decision_for_node("run-a", "other-node", InferenceTaskId::ImageGeneration)
            .expect_err("missing node decision must fail");

        assert!(matches!(
            error,
            PlannedInferenceDecisionContextError::MissingNodeDecision { .. }
        ));
    }

    #[test]
    fn planned_context_rejects_task_mismatch() {
        let context = PlannedInferenceDecisionContext::new(
            "run-a",
            HashMap::from([(
                "image-node-1".to_string(),
                backend_decision(Some(InferenceTaskId::Embedding)),
            )]),
        )
        .expect("planned context");

        let error = context
            .decision_for_node("run-a", "image-node-1", InferenceTaskId::ImageGeneration)
            .expect_err("task mismatch must fail");

        assert!(matches!(
            error,
            PlannedInferenceDecisionContextError::TaskMismatch { .. }
        ));
    }
}
