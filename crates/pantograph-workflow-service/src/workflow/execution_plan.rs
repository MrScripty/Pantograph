use std::collections::BTreeMap;

use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{
    WorkflowExecutionPlanBackendKey, WorkflowExecutionPlanDeviceId, WorkflowExecutionPlanModelRef,
    WorkflowExecutionPlanRuntimeId, WorkflowExecutionPlanRuntimeVariantId,
};
use super::{WorkflowInferenceDeviceClass, WorkflowInferenceTaskId};

pub const WORKFLOW_EXECUTION_PLAN_SCHEMA_VERSION: u32 = 1;
pub const WORKFLOW_EXECUTION_PLAN_MAX_NODE_DECISIONS: usize = 1024;
pub const WORKFLOW_EXECUTION_PLAN_MAX_DIAGNOSTICS: usize = 32;
pub const WORKFLOW_EXECUTION_PLAN_MAX_POLICY_TRACE_IDS: usize = 32;

const MAX_EXECUTION_PLAN_ID_LEN: usize = 128;
const MAX_EXECUTION_PLAN_DIAGNOSTIC_MESSAGE_LEN: usize = 512;

/// Run-scoped workflow execution plan produced by scheduler admission.
///
/// This contract is intentionally smaller than scheduler/runtime internals and
/// must not contain graph inputs, full Pumas facts, worker envelopes, or raw
/// node payloads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", try_from = "WorkflowExecutionPlanUnchecked")]
pub struct WorkflowExecutionPlan {
    schema_version: u32,
    workflow_run_id: WorkflowRunId,
    workflow_id: WorkflowId,
    node_decisions: BTreeMap<String, WorkflowExecutionPlanNodeDecision>,
}

impl WorkflowExecutionPlan {
    pub fn new(
        workflow_run_id: WorkflowRunId,
        workflow_id: WorkflowId,
        decisions: Vec<WorkflowExecutionPlanNodeDecision>,
    ) -> Result<Self, WorkflowExecutionPlanError> {
        if decisions.is_empty() {
            return Err(WorkflowExecutionPlanError::MissingNodeDecisions);
        }
        if decisions.len() > WORKFLOW_EXECUTION_PLAN_MAX_NODE_DECISIONS {
            return Err(WorkflowExecutionPlanError::TooManyNodeDecisions {
                count: decisions.len(),
                max: WORKFLOW_EXECUTION_PLAN_MAX_NODE_DECISIONS,
            });
        }

        let mut node_decisions = BTreeMap::new();
        for decision in decisions {
            let node_id = decision.node_id().to_string();
            if node_decisions.insert(node_id.clone(), decision).is_some() {
                return Err(WorkflowExecutionPlanError::DuplicateNodeDecision { node_id });
            }
        }

        Ok(Self {
            schema_version: WORKFLOW_EXECUTION_PLAN_SCHEMA_VERSION,
            workflow_run_id,
            workflow_id,
            node_decisions,
        })
    }

    #[must_use]
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    #[must_use]
    pub fn workflow_run_id(&self) -> &WorkflowRunId {
        &self.workflow_run_id
    }

    #[must_use]
    pub fn workflow_id(&self) -> &WorkflowId {
        &self.workflow_id
    }

    #[must_use]
    pub fn node_decision(&self, node_id: &str) -> Option<&WorkflowExecutionPlanNodeDecision> {
        self.node_decisions.get(node_id)
    }

    #[must_use]
    pub fn node_decisions(&self) -> &BTreeMap<String, WorkflowExecutionPlanNodeDecision> {
        &self.node_decisions
    }
}

impl TryFrom<WorkflowExecutionPlanUnchecked> for WorkflowExecutionPlan {
    type Error = WorkflowExecutionPlanError;

    fn try_from(value: WorkflowExecutionPlanUnchecked) -> Result<Self, Self::Error> {
        if value.schema_version != WORKFLOW_EXECUTION_PLAN_SCHEMA_VERSION {
            return Err(WorkflowExecutionPlanError::UnsupportedSchemaVersion {
                actual: value.schema_version,
                expected: WORKFLOW_EXECUTION_PLAN_SCHEMA_VERSION,
            });
        }
        if value.node_decisions.is_empty() {
            return Err(WorkflowExecutionPlanError::MissingNodeDecisions);
        }
        if value.node_decisions.len() > WORKFLOW_EXECUTION_PLAN_MAX_NODE_DECISIONS {
            return Err(WorkflowExecutionPlanError::TooManyNodeDecisions {
                count: value.node_decisions.len(),
                max: WORKFLOW_EXECUTION_PLAN_MAX_NODE_DECISIONS,
            });
        }

        let workflow_run_id = WorkflowRunId::try_from(value.workflow_run_id).map_err(|error| {
            WorkflowExecutionPlanError::InvalidAttributionId {
                field: "workflow_run_id",
                message: error.to_string(),
            }
        })?;
        let workflow_id = WorkflowId::try_from(value.workflow_id).map_err(|error| {
            WorkflowExecutionPlanError::InvalidAttributionId {
                field: "workflow_id",
                message: error.to_string(),
            }
        })?;

        for (key, decision) in &value.node_decisions {
            if key != decision.node_id() {
                return Err(WorkflowExecutionPlanError::NodeDecisionKeyMismatch {
                    key: key.clone(),
                    node_id: decision.node_id().to_string(),
                });
            }
        }

        Ok(Self {
            schema_version: value.schema_version,
            workflow_run_id,
            workflow_id,
            node_decisions: value.node_decisions,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct WorkflowExecutionPlanUnchecked {
    schema_version: u32,
    workflow_run_id: String,
    workflow_id: String,
    #[serde(default)]
    node_decisions: BTreeMap<String, WorkflowExecutionPlanNodeDecision>,
}

/// Reduced per-node scheduler decision consumed by execution-time adapters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    rename_all = "snake_case",
    try_from = "WorkflowExecutionPlanNodeDecisionUnchecked"
)]
pub struct WorkflowExecutionPlanNodeDecision {
    node_id: String,
    selected_backend_key: WorkflowExecutionPlanBackendKey,
    selected_runtime_id: WorkflowExecutionPlanRuntimeId,
    selected_runtime_variant_id: WorkflowExecutionPlanRuntimeVariantId,
    selected_device_class: WorkflowInferenceDeviceClass,
    selected_device_id: Option<WorkflowExecutionPlanDeviceId>,
    selected_task_id: WorkflowInferenceTaskId,
    selected_model_ref: Option<WorkflowExecutionPlanModelRef>,
    diagnostics: Vec<WorkflowExecutionPlanDiagnostic>,
    policy_trace_ids: Vec<String>,
}

impl WorkflowExecutionPlanNodeDecision {
    pub fn new(
        node_id: impl Into<String>,
        selected_backend_key: impl AsRef<str>,
        selected_runtime_id: impl AsRef<str>,
        selected_runtime_variant_id: impl AsRef<str>,
        selected_device_class: WorkflowInferenceDeviceClass,
        selected_task_id: WorkflowInferenceTaskId,
    ) -> Result<Self, WorkflowExecutionPlanError> {
        let node_id = validate_required_text("node_id", node_id.into(), MAX_EXECUTION_PLAN_ID_LEN)?;
        let selected_backend_key = parse_selected_backend_key(selected_backend_key.as_ref())?;
        let selected_runtime_id = parse_selected_runtime_id(selected_runtime_id.as_ref())?;
        let selected_runtime_variant_id =
            parse_selected_runtime_variant_id(selected_runtime_variant_id.as_ref())?;
        validate_selected_device_class(selected_device_class)?;
        validate_selected_task_id(selected_task_id)?;

        Ok(Self {
            node_id,
            selected_backend_key,
            selected_runtime_id,
            selected_runtime_variant_id,
            selected_device_class,
            selected_device_id: None,
            selected_task_id,
            selected_model_ref: None,
            diagnostics: Vec::new(),
            policy_trace_ids: Vec::new(),
        })
    }

    pub fn with_selected_device_id(
        mut self,
        selected_device_id: impl AsRef<str>,
    ) -> Result<Self, WorkflowExecutionPlanError> {
        self.selected_device_id = Some(parse_selected_device_id(selected_device_id.as_ref())?);
        Ok(self)
    }

    pub fn with_selected_model_ref(
        mut self,
        selected_model_ref: impl AsRef<str>,
    ) -> Result<Self, WorkflowExecutionPlanError> {
        self.selected_model_ref = Some(parse_selected_model_ref(selected_model_ref.as_ref())?);
        Ok(self)
    }

    pub fn with_diagnostics(
        mut self,
        diagnostics: Vec<WorkflowExecutionPlanDiagnostic>,
    ) -> Result<Self, WorkflowExecutionPlanError> {
        validate_count(
            "diagnostics",
            diagnostics.len(),
            WORKFLOW_EXECUTION_PLAN_MAX_DIAGNOSTICS,
        )?;
        self.diagnostics = diagnostics;
        Ok(self)
    }

    pub fn with_policy_trace_ids(
        mut self,
        policy_trace_ids: Vec<String>,
    ) -> Result<Self, WorkflowExecutionPlanError> {
        validate_count(
            "policy_trace_ids",
            policy_trace_ids.len(),
            WORKFLOW_EXECUTION_PLAN_MAX_POLICY_TRACE_IDS,
        )?;
        self.policy_trace_ids = policy_trace_ids
            .into_iter()
            .map(|trace_id| {
                validate_required_text("policy_trace_ids", trace_id, MAX_EXECUTION_PLAN_ID_LEN)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(self)
    }

    #[must_use]
    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    #[must_use]
    pub fn selected_backend_key(&self) -> &str {
        self.selected_backend_key.as_str()
    }

    #[must_use]
    pub fn selected_runtime_id(&self) -> &str {
        self.selected_runtime_id.as_str()
    }

    #[must_use]
    pub fn selected_runtime_variant_id(&self) -> &str {
        self.selected_runtime_variant_id.as_str()
    }

    #[must_use]
    pub fn selected_device_class(&self) -> WorkflowInferenceDeviceClass {
        self.selected_device_class
    }

    #[must_use]
    pub fn selected_device_id(&self) -> Option<&str> {
        self.selected_device_id
            .as_ref()
            .map(WorkflowExecutionPlanDeviceId::as_str)
    }

    #[must_use]
    pub fn selected_task_id(&self) -> WorkflowInferenceTaskId {
        self.selected_task_id
    }

    #[must_use]
    pub fn selected_model_ref(&self) -> Option<&str> {
        self.selected_model_ref
            .as_ref()
            .map(WorkflowExecutionPlanModelRef::as_str)
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[WorkflowExecutionPlanDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn policy_trace_ids(&self) -> &[String] {
        &self.policy_trace_ids
    }
}

impl TryFrom<WorkflowExecutionPlanNodeDecisionUnchecked> for WorkflowExecutionPlanNodeDecision {
    type Error = WorkflowExecutionPlanError;

    fn try_from(value: WorkflowExecutionPlanNodeDecisionUnchecked) -> Result<Self, Self::Error> {
        let mut decision = Self::new(
            value.node_id,
            value.selected_backend_key,
            value.selected_runtime_id,
            value.selected_runtime_variant_id,
            value.selected_device_class,
            value.selected_task_id,
        )?;
        if let Some(selected_device_id) = value.selected_device_id {
            decision = decision.with_selected_device_id(selected_device_id)?;
        }
        if let Some(selected_model_ref) = value.selected_model_ref {
            decision = decision.with_selected_model_ref(selected_model_ref)?;
        }
        decision = decision.with_diagnostics(value.diagnostics)?;
        decision.with_policy_trace_ids(value.policy_trace_ids)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct WorkflowExecutionPlanNodeDecisionUnchecked {
    node_id: String,
    selected_backend_key: String,
    selected_runtime_id: String,
    selected_runtime_variant_id: String,
    selected_device_class: WorkflowInferenceDeviceClass,
    #[serde(default)]
    selected_device_id: Option<String>,
    selected_task_id: WorkflowInferenceTaskId,
    #[serde(default)]
    selected_model_ref: Option<String>,
    #[serde(default)]
    diagnostics: Vec<WorkflowExecutionPlanDiagnostic>,
    #[serde(default)]
    policy_trace_ids: Vec<String>,
}

/// Bounded diagnostic carried with a workflow execution-plan decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    rename_all = "snake_case",
    try_from = "WorkflowExecutionPlanDiagnosticUnchecked"
)]
pub struct WorkflowExecutionPlanDiagnostic {
    code: WorkflowExecutionPlanDiagnosticCode,
    severity: WorkflowExecutionPlanDiagnosticSeverity,
    message: String,
}

impl WorkflowExecutionPlanDiagnostic {
    pub fn new(
        code: WorkflowExecutionPlanDiagnosticCode,
        severity: WorkflowExecutionPlanDiagnosticSeverity,
        message: impl Into<String>,
    ) -> Result<Self, WorkflowExecutionPlanError> {
        Ok(Self {
            code,
            severity,
            message: validate_required_text(
                "diagnostic.message",
                message.into(),
                MAX_EXECUTION_PLAN_DIAGNOSTIC_MESSAGE_LEN,
            )?,
        })
    }

    #[must_use]
    pub fn code(&self) -> WorkflowExecutionPlanDiagnosticCode {
        self.code
    }

    #[must_use]
    pub fn severity(&self) -> WorkflowExecutionPlanDiagnosticSeverity {
        self.severity
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl TryFrom<WorkflowExecutionPlanDiagnosticUnchecked> for WorkflowExecutionPlanDiagnostic {
    type Error = WorkflowExecutionPlanError;

    fn try_from(value: WorkflowExecutionPlanDiagnosticUnchecked) -> Result<Self, Self::Error> {
        Self::new(value.code, value.severity, value.message)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct WorkflowExecutionPlanDiagnosticUnchecked {
    code: WorkflowExecutionPlanDiagnosticCode,
    severity: WorkflowExecutionPlanDiagnosticSeverity,
    message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowExecutionPlanDiagnosticCode {
    MissingNodeDecision,
    MissingSelectedDecisionFact,
    InvalidSelectedDecisionFact,
    AmbiguousNodeMapping,
    StaleRunContext,
    ProjectionFailed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowExecutionPlanDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkflowExecutionPlanError {
    #[error("unsupported workflow execution-plan schema version {actual}; expected {expected}")]
    UnsupportedSchemaVersion { actual: u32, expected: u32 },
    #[error("workflow execution plan must contain at least one node decision")]
    MissingNodeDecisions,
    #[error("workflow execution plan has {count} node decisions; maximum is {max}")]
    TooManyNodeDecisions { count: usize, max: usize },
    #[error("workflow execution plan contains duplicate node decision '{node_id}'")]
    DuplicateNodeDecision { node_id: String },
    #[error(
        "workflow execution-plan node decision key '{key}' does not match node id '{node_id}'"
    )]
    NodeDecisionKeyMismatch { key: String, node_id: String },
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error("{field} exceeds maximum length {max_len}")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("{field} contains invalid characters")]
    InvalidField { field: &'static str },
    #[error("{field} has {count} entries; maximum is {max}")]
    TooManyEntries {
        field: &'static str,
        count: usize,
        max: usize,
    },
    #[error("{field} is not a valid attribution id: {message}")]
    InvalidAttributionId {
        field: &'static str,
        message: String,
    },
    #[error("technical-fit decision is missing selected fact {field}")]
    MissingSelectedDecisionFact { field: &'static str },
    #[error("selected decision fact '{field}' value '{value}' is invalid: {message}")]
    InvalidSelectedDecisionFact {
        field: &'static str,
        value: String,
        message: String,
    },
    #[error("selected model '{model_id}' was not present in workflow capabilities")]
    SelectedModelNotFound { model_id: String },
    #[error("selected model '{model_id}' matched {count} workflow capability records")]
    AmbiguousSelectedModel { model_id: String, count: usize },
    #[error("selected model '{model_id}' maps to {count} nodes")]
    AmbiguousNodeMapping { model_id: String, count: usize },
    #[error("selected model '{model_id}' maps to {count} inference tasks")]
    AmbiguousSelectedTask { model_id: String, count: usize },
    #[error("selected model ref '{value}' is invalid: {message}")]
    InvalidSelectedModelRef { value: String, message: String },
}

fn validate_required_text(
    field: &'static str,
    value: String,
    max_len: usize,
) -> Result<String, WorkflowExecutionPlanError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(WorkflowExecutionPlanError::MissingField { field });
    }
    if value.len() > max_len {
        return Err(WorkflowExecutionPlanError::FieldTooLong { field, max_len });
    }
    if value.chars().any(char::is_control) {
        return Err(WorkflowExecutionPlanError::InvalidField { field });
    }
    Ok(value.to_string())
}

fn validate_count(
    field: &'static str,
    count: usize,
    max: usize,
) -> Result<(), WorkflowExecutionPlanError> {
    if count > max {
        Err(WorkflowExecutionPlanError::TooManyEntries { field, count, max })
    } else {
        Ok(())
    }
}

fn validate_selected_device_class(
    selected_device_class: WorkflowInferenceDeviceClass,
) -> Result<(), WorkflowExecutionPlanError> {
    if selected_device_class == WorkflowInferenceDeviceClass::Unknown {
        Err(WorkflowExecutionPlanError::InvalidField {
            field: "selected_device_class",
        })
    } else {
        Ok(())
    }
}

fn validate_selected_task_id(
    selected_task_id: WorkflowInferenceTaskId,
) -> Result<(), WorkflowExecutionPlanError> {
    if selected_task_id == WorkflowInferenceTaskId::Unknown {
        Err(WorkflowExecutionPlanError::InvalidField {
            field: "selected_task_id",
        })
    } else {
        Ok(())
    }
}

fn parse_selected_model_ref(
    value: &str,
) -> Result<WorkflowExecutionPlanModelRef, WorkflowExecutionPlanError> {
    WorkflowExecutionPlanModelRef::parse(value).map_err(|error| {
        WorkflowExecutionPlanError::InvalidSelectedModelRef {
            value: value.to_string(),
            message: error.to_string(),
        }
    })
}

fn parse_selected_backend_key(
    value: &str,
) -> Result<WorkflowExecutionPlanBackendKey, WorkflowExecutionPlanError> {
    WorkflowExecutionPlanBackendKey::parse(value)
        .map_err(|error| invalid_selected_decision_fact("selected_backend_key", value, error))
}

fn parse_selected_runtime_id(
    value: &str,
) -> Result<WorkflowExecutionPlanRuntimeId, WorkflowExecutionPlanError> {
    WorkflowExecutionPlanRuntimeId::parse(value)
        .map_err(|error| invalid_selected_decision_fact("selected_runtime_id", value, error))
}

fn parse_selected_runtime_variant_id(
    value: &str,
) -> Result<WorkflowExecutionPlanRuntimeVariantId, WorkflowExecutionPlanError> {
    WorkflowExecutionPlanRuntimeVariantId::parse(value).map_err(|error| {
        invalid_selected_decision_fact("selected_runtime_variant_id", value, error)
    })
}

fn parse_selected_device_id(
    value: &str,
) -> Result<WorkflowExecutionPlanDeviceId, WorkflowExecutionPlanError> {
    WorkflowExecutionPlanDeviceId::parse(value)
        .map_err(|error| invalid_selected_decision_fact("selected_device_id", value, error))
}

fn invalid_selected_decision_fact(
    field: &'static str,
    value: &str,
    error: impl std::error::Error,
) -> WorkflowExecutionPlanError {
    WorkflowExecutionPlanError::InvalidSelectedDecisionFact {
        field,
        value: value.to_string(),
        message: error.to_string(),
    }
}
