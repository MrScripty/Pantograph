use std::panic::Location;

use pantograph_diagnostics_ledger::{
    sanitize_diagnostic_error_text, DiagnosticErrorLocation, DiagnosticErrorOccurredPayload,
    DiagnosticErrorRecoverability, DiagnosticErrorScopeKind, DiagnosticErrorSeverity,
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent,
    MAX_DIAGNOSTIC_ERROR_CAUSE_COUNT, MAX_DIAGNOSTIC_ERROR_CAUSE_LEN,
    MAX_DIAGNOSTIC_ERROR_TEXT_LEN,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunId, WorkflowVersionId,
};

use crate::scheduler::unix_timestamp_ms;

use super::{WorkflowErrorDiagnosticsLink, WorkflowService, WorkflowServiceError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowDiagnosticErrorPhase {
    RunSnapshot,
    SchedulerAdmission,
    RuntimePreflight,
    RuntimeModelLoad,
    RuntimeLaunch,
    ModelDependency,
    ManagedBinary,
    NodeExecution,
    OutputValidation,
    RunTimeout,
    Artifact,
    Projection,
    Transport,
}

impl WorkflowDiagnosticErrorPhase {
    fn registry_entry(self) -> &'static WorkflowDiagnosticErrorRegistryEntry {
        WORKFLOW_DIAGNOSTIC_ERROR_REGISTRY
            .iter()
            .find(|entry| entry.phase == self)
            .expect("workflow diagnostic error phase is registered")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkflowDiagnosticErrorRegistryEntry {
    pub phase: WorkflowDiagnosticErrorPhase,
    pub phase_id: &'static str,
    pub code: &'static str,
    pub scope_kind: DiagnosticErrorScopeKind,
    pub default_source: DiagnosticEventSourceComponent,
    pub allowed_sources: &'static [DiagnosticEventSourceComponent],
    pub default_severity: DiagnosticErrorSeverity,
    pub default_recoverability: DiagnosticErrorRecoverability,
    pub causality_policy: WorkflowDiagnosticCausalityPolicy,
    pub projection_effect: WorkflowDiagnosticProjectionEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowDiagnosticCausalityPolicy {
    DirectProducerKnowledgeOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowDiagnosticProjectionEffect {
    FatalRunFailure,
    DiagnosticsOnly,
}

const SCHEDULER_SOURCE: &[DiagnosticEventSourceComponent] =
    &[DiagnosticEventSourceComponent::Scheduler];
const RUNTIME_SOURCE: &[DiagnosticEventSourceComponent] =
    &[DiagnosticEventSourceComponent::Runtime];
const NODE_EXECUTION_SOURCE: &[DiagnosticEventSourceComponent] =
    &[DiagnosticEventSourceComponent::NodeExecution];
const WORKFLOW_SERVICE_SOURCE: &[DiagnosticEventSourceComponent] =
    &[DiagnosticEventSourceComponent::WorkflowService];

const WORKFLOW_DIAGNOSTIC_ERROR_REGISTRY: &[WorkflowDiagnosticErrorRegistryEntry] = &[
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::RunSnapshot,
        phase_id: "run_snapshot",
        code: "run_snapshot_failed",
        scope_kind: DiagnosticErrorScopeKind::Run,
        default_source: DiagnosticEventSourceComponent::WorkflowService,
        allowed_sources: WORKFLOW_SERVICE_SOURCE,
        default_severity: DiagnosticErrorSeverity::Fatal,
        default_recoverability: DiagnosticErrorRecoverability::Unknown,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::FatalRunFailure,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::SchedulerAdmission,
        phase_id: "scheduler_admission",
        code: "scheduler_admission_failed",
        scope_kind: DiagnosticErrorScopeKind::Scheduler,
        default_source: DiagnosticEventSourceComponent::Scheduler,
        allowed_sources: SCHEDULER_SOURCE,
        default_severity: DiagnosticErrorSeverity::Fatal,
        default_recoverability: DiagnosticErrorRecoverability::Retryable,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::FatalRunFailure,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::RuntimePreflight,
        phase_id: "runtime_preflight",
        code: "runtime_preflight_failed",
        scope_kind: DiagnosticErrorScopeKind::RuntimeModel,
        default_source: DiagnosticEventSourceComponent::Scheduler,
        allowed_sources: SCHEDULER_SOURCE,
        default_severity: DiagnosticErrorSeverity::Fatal,
        default_recoverability: DiagnosticErrorRecoverability::Retryable,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::FatalRunFailure,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::RuntimeModelLoad,
        phase_id: "runtime_model_load",
        code: "runtime_model_load_failed",
        scope_kind: DiagnosticErrorScopeKind::RuntimeModel,
        default_source: DiagnosticEventSourceComponent::Scheduler,
        allowed_sources: SCHEDULER_SOURCE,
        default_severity: DiagnosticErrorSeverity::Fatal,
        default_recoverability: DiagnosticErrorRecoverability::Retryable,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::FatalRunFailure,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::RuntimeLaunch,
        phase_id: "runtime_launch",
        code: "runtime_launch_failed",
        scope_kind: DiagnosticErrorScopeKind::RuntimeModel,
        default_source: DiagnosticEventSourceComponent::Runtime,
        allowed_sources: RUNTIME_SOURCE,
        default_severity: DiagnosticErrorSeverity::Fatal,
        default_recoverability: DiagnosticErrorRecoverability::Retryable,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::FatalRunFailure,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::ModelDependency,
        phase_id: "model_dependency",
        code: "model_dependency_failed",
        scope_kind: DiagnosticErrorScopeKind::RuntimeModel,
        default_source: DiagnosticEventSourceComponent::Runtime,
        allowed_sources: RUNTIME_SOURCE,
        default_severity: DiagnosticErrorSeverity::Fatal,
        default_recoverability: DiagnosticErrorRecoverability::Retryable,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::FatalRunFailure,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::ManagedBinary,
        phase_id: "managed_binary",
        code: "managed_binary_failed",
        scope_kind: DiagnosticErrorScopeKind::RuntimeModel,
        default_source: DiagnosticEventSourceComponent::Runtime,
        allowed_sources: RUNTIME_SOURCE,
        default_severity: DiagnosticErrorSeverity::Fatal,
        default_recoverability: DiagnosticErrorRecoverability::Retryable,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::FatalRunFailure,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::NodeExecution,
        phase_id: "node_execution",
        code: "node_execution_failed",
        scope_kind: DiagnosticErrorScopeKind::Node,
        default_source: DiagnosticEventSourceComponent::NodeExecution,
        allowed_sources: NODE_EXECUTION_SOURCE,
        default_severity: DiagnosticErrorSeverity::Fatal,
        default_recoverability: DiagnosticErrorRecoverability::Unknown,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::FatalRunFailure,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::OutputValidation,
        phase_id: "output_validation",
        code: "output_validation_failed",
        scope_kind: DiagnosticErrorScopeKind::Node,
        default_source: DiagnosticEventSourceComponent::WorkflowService,
        allowed_sources: WORKFLOW_SERVICE_SOURCE,
        default_severity: DiagnosticErrorSeverity::Fatal,
        default_recoverability: DiagnosticErrorRecoverability::Unrecoverable,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::FatalRunFailure,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::RunTimeout,
        phase_id: "run_timeout",
        code: "run_timeout",
        scope_kind: DiagnosticErrorScopeKind::Run,
        default_source: DiagnosticEventSourceComponent::WorkflowService,
        allowed_sources: WORKFLOW_SERVICE_SOURCE,
        default_severity: DiagnosticErrorSeverity::Fatal,
        default_recoverability: DiagnosticErrorRecoverability::Retryable,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::FatalRunFailure,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::Artifact,
        phase_id: "artifact",
        code: "artifact_failed",
        scope_kind: DiagnosticErrorScopeKind::Artifact,
        default_source: DiagnosticEventSourceComponent::WorkflowService,
        allowed_sources: WORKFLOW_SERVICE_SOURCE,
        default_severity: DiagnosticErrorSeverity::Fatal,
        default_recoverability: DiagnosticErrorRecoverability::Unknown,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::FatalRunFailure,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::Projection,
        phase_id: "projection",
        code: "projection_failed",
        scope_kind: DiagnosticErrorScopeKind::Projection,
        default_source: DiagnosticEventSourceComponent::WorkflowService,
        allowed_sources: WORKFLOW_SERVICE_SOURCE,
        default_severity: DiagnosticErrorSeverity::Error,
        default_recoverability: DiagnosticErrorRecoverability::Retryable,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::DiagnosticsOnly,
    },
    WorkflowDiagnosticErrorRegistryEntry {
        phase: WorkflowDiagnosticErrorPhase::Transport,
        phase_id: "transport",
        code: "transport_failed",
        scope_kind: DiagnosticErrorScopeKind::Transport,
        default_source: DiagnosticEventSourceComponent::WorkflowService,
        allowed_sources: WORKFLOW_SERVICE_SOURCE,
        default_severity: DiagnosticErrorSeverity::Error,
        default_recoverability: DiagnosticErrorRecoverability::Retryable,
        causality_policy: WorkflowDiagnosticCausalityPolicy::DirectProducerKnowledgeOnly,
        projection_effect: WorkflowDiagnosticProjectionEffect::DiagnosticsOnly,
    },
];

pub(crate) fn registered_workflow_diagnostic_error_phases(
) -> &'static [WorkflowDiagnosticErrorRegistryEntry] {
    WORKFLOW_DIAGNOSTIC_ERROR_REGISTRY
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowDiagnosticRunContext {
    pub workflow_run_id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_version_id: Option<WorkflowVersionId>,
    pub workflow_semantic_version: Option<String>,
    pub client_id: Option<ClientId>,
    pub client_session_id: Option<ClientSessionId>,
    pub bucket_id: Option<BucketId>,
    pub scheduler_policy_id: Option<String>,
    pub retention_policy_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowDiagnosticRunScope {
    pub run: WorkflowDiagnosticRunContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowDiagnosticNodeScope {
    pub run: WorkflowDiagnosticRunContext,
    pub node_id: String,
    pub node_type: Option<String>,
    pub node_version: Option<String>,
    pub runtime_id: Option<String>,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowDiagnosticRuntimeModelScope {
    pub run: WorkflowDiagnosticRunContext,
    pub runtime_id: String,
    pub runtime_version: Option<String>,
    pub model_id: Option<String>,
    pub model_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowDiagnosticSchedulerScope {
    pub run: WorkflowDiagnosticRunContext,
    pub selected_runtime_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowDiagnosticArtifactScope {
    pub run: WorkflowDiagnosticRunContext,
    pub node_id: Option<String>,
    pub payload_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowDiagnosticProjectionScope {
    pub workflow_run_id: Option<WorkflowRunId>,
    pub workflow_id: Option<WorkflowId>,
    pub projection_name: String,
    pub operation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowDiagnosticTransportScope {
    pub workflow_run_id: Option<WorkflowRunId>,
    pub workflow_id: Option<WorkflowId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkflowDiagnosticErrorScope {
    Run(WorkflowDiagnosticRunScope),
    Node(WorkflowDiagnosticNodeScope),
    RuntimeModel(WorkflowDiagnosticRuntimeModelScope),
    Scheduler(WorkflowDiagnosticSchedulerScope),
    Artifact(WorkflowDiagnosticArtifactScope),
    Projection(WorkflowDiagnosticProjectionScope),
    Transport(WorkflowDiagnosticTransportScope),
}

impl WorkflowDiagnosticErrorScope {
    fn kind(&self) -> DiagnosticErrorScopeKind {
        match self {
            Self::Run(_) => DiagnosticErrorScopeKind::Run,
            Self::Node(_) => DiagnosticErrorScopeKind::Node,
            Self::RuntimeModel(_) => DiagnosticErrorScopeKind::RuntimeModel,
            Self::Scheduler(_) => DiagnosticErrorScopeKind::Scheduler,
            Self::Artifact(_) => DiagnosticErrorScopeKind::Artifact,
            Self::Projection(_) => DiagnosticErrorScopeKind::Projection,
            Self::Transport(_) => DiagnosticErrorScopeKind::Transport,
        }
    }

    fn append_request_fields(&self, request: &mut DiagnosticEventAppendRequest) {
        match self {
            Self::Run(scope) => apply_run_context(request, &scope.run),
            Self::Node(scope) => {
                apply_run_context(request, &scope.run);
                request.node_id = Some(scope.node_id.clone());
                request.node_type = scope.node_type.clone();
                request.node_version = scope.node_version.clone();
                request.runtime_id = scope.runtime_id.clone();
                request.model_id = scope.model_id.clone();
            }
            Self::RuntimeModel(scope) => {
                apply_run_context(request, &scope.run);
                request.runtime_id = Some(scope.runtime_id.clone());
                request.runtime_version = scope.runtime_version.clone();
                request.model_id = scope.model_id.clone();
                request.model_version = scope.model_version.clone();
            }
            Self::Scheduler(scope) => {
                apply_run_context(request, &scope.run);
                request.runtime_id = scope.selected_runtime_id.clone();
            }
            Self::Artifact(scope) => {
                apply_run_context(request, &scope.run);
                request.node_id = scope.node_id.clone();
                request.payload_ref = scope.payload_ref.clone();
            }
            Self::Projection(scope) => {
                request.workflow_run_id = scope.workflow_run_id.clone();
                request.workflow_id = scope.workflow_id.clone();
            }
            Self::Transport(scope) => {
                request.workflow_run_id = scope.workflow_run_id.clone();
                request.workflow_id = scope.workflow_id.clone();
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowDiagnosticErrorRecordOutcome {
    pub event_id: Option<String>,
    pub diagnostics_unavailable: Option<String>,
}

impl WorkflowDiagnosticErrorRecordOutcome {
    fn recorded(event_id: String) -> Self {
        Self {
            event_id: Some(event_id),
            diagnostics_unavailable: None,
        }
    }

    fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            event_id: None,
            diagnostics_unavailable: Some(reason.into()),
        }
    }

    pub(crate) fn into_error_link<T>(
        self,
        workflow_run_id: Option<T>,
    ) -> WorkflowErrorDiagnosticsLink
    where
        T: ToString,
    {
        WorkflowErrorDiagnosticsLink {
            workflow_run_id: workflow_run_id.map(|value| value.to_string()),
            diagnostic_event_id: self.event_id,
            diagnostics_unavailable: self.diagnostics_unavailable,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowDiagnosticErrorRecordRequest {
    phase: WorkflowDiagnosticErrorPhase,
    scope: WorkflowDiagnosticErrorScope,
    message: String,
    technical_message: Option<String>,
    cause_chain: Vec<String>,
    source_component: Option<DiagnosticEventSourceComponent>,
    source_instance_id: Option<String>,
    severity: Option<DiagnosticErrorSeverity>,
    recoverability: Option<DiagnosticErrorRecoverability>,
    related_event_ids: Vec<String>,
    caused_by_event_id: Option<String>,
    location: DiagnosticErrorLocation,
}

impl WorkflowDiagnosticErrorRecordRequest {
    #[track_caller]
    pub(crate) fn run_snapshot_failed(
        scope: WorkflowDiagnosticRunScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::RunSnapshot,
            WorkflowDiagnosticErrorScope::Run(scope),
            error,
        )
    }

    #[track_caller]
    pub(crate) fn scheduler_admission_failed(
        scope: WorkflowDiagnosticSchedulerScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::SchedulerAdmission,
            WorkflowDiagnosticErrorScope::Scheduler(scope),
            error,
        )
    }

    #[track_caller]
    pub(crate) fn runtime_preflight_failed(
        scope: WorkflowDiagnosticRuntimeModelScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::RuntimePreflight,
            WorkflowDiagnosticErrorScope::RuntimeModel(scope),
            error,
        )
    }

    #[track_caller]
    pub(crate) fn runtime_model_load_failed(
        scope: WorkflowDiagnosticRuntimeModelScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::RuntimeModelLoad,
            WorkflowDiagnosticErrorScope::RuntimeModel(scope),
            error,
        )
    }

    #[track_caller]
    pub(crate) fn runtime_launch_failed(
        scope: WorkflowDiagnosticRuntimeModelScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::RuntimeLaunch,
            WorkflowDiagnosticErrorScope::RuntimeModel(scope),
            error,
        )
    }

    #[track_caller]
    pub(crate) fn model_dependency_failed(
        scope: WorkflowDiagnosticRuntimeModelScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::ModelDependency,
            WorkflowDiagnosticErrorScope::RuntimeModel(scope),
            error,
        )
    }

    #[track_caller]
    pub(crate) fn managed_binary_failed(
        scope: WorkflowDiagnosticRuntimeModelScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::ManagedBinary,
            WorkflowDiagnosticErrorScope::RuntimeModel(scope),
            error,
        )
    }

    #[track_caller]
    pub(crate) fn node_execution_failed(
        scope: WorkflowDiagnosticNodeScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::NodeExecution,
            WorkflowDiagnosticErrorScope::Node(scope),
            error,
        )
    }

    #[track_caller]
    pub(crate) fn output_validation_failed(
        scope: WorkflowDiagnosticNodeScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::OutputValidation,
            WorkflowDiagnosticErrorScope::Node(scope),
            error,
        )
    }

    #[track_caller]
    pub(crate) fn run_timeout(
        scope: WorkflowDiagnosticRunScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::RunTimeout,
            WorkflowDiagnosticErrorScope::Run(scope),
            error,
        )
    }

    #[track_caller]
    pub(crate) fn artifact_failed(
        scope: WorkflowDiagnosticArtifactScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::Artifact,
            WorkflowDiagnosticErrorScope::Artifact(scope),
            error,
        )
    }

    #[track_caller]
    pub(crate) fn projection_failed(
        scope: WorkflowDiagnosticProjectionScope,
        error: &WorkflowServiceError,
    ) -> Self {
        let operation = scope.operation.clone();
        let projection_name = scope.projection_name.clone();
        let mut request = Self::from_error(
            WorkflowDiagnosticErrorPhase::Projection,
            WorkflowDiagnosticErrorScope::Projection(scope),
            error,
        );
        request.location.component = Some("workflow-projection".to_string());
        request.location.operation = Some(format!("{projection_name}.{operation}"));
        request
    }

    #[track_caller]
    pub(crate) fn transport_failed(
        scope: WorkflowDiagnosticTransportScope,
        error: &WorkflowServiceError,
    ) -> Self {
        Self::from_error(
            WorkflowDiagnosticErrorPhase::Transport,
            WorkflowDiagnosticErrorScope::Transport(scope),
            error,
        )
    }

    #[track_caller]
    fn from_error(
        phase: WorkflowDiagnosticErrorPhase,
        scope: WorkflowDiagnosticErrorScope,
        error: &WorkflowServiceError,
    ) -> Self {
        let caller = Location::caller();
        Self {
            phase,
            scope,
            message: error.message().to_string(),
            technical_message: Some(error.to_string()),
            cause_chain: Vec::new(),
            source_component: None,
            source_instance_id: None,
            severity: None,
            recoverability: None,
            related_event_ids: Vec::new(),
            caused_by_event_id: None,
            location: DiagnosticErrorLocation {
                component: Some("workflow-service".to_string()),
                operation: Some(phase.registry_entry().phase_id.to_string()),
                module_path: Some(module_path!().to_string()),
                file: Some(caller.file().to_string()),
                line: Some(caller.line()),
            },
        }
    }

    pub(crate) fn with_source_instance_id(mut self, source_instance_id: impl Into<String>) -> Self {
        self.source_instance_id = Some(source_instance_id.into());
        self
    }

    pub(crate) fn with_source_component(
        mut self,
        source_component: DiagnosticEventSourceComponent,
    ) -> Self {
        self.source_component = Some(source_component);
        self
    }

    pub(crate) fn with_related_event_id(mut self, event_id: impl Into<String>) -> Self {
        self.related_event_ids.push(event_id.into());
        self
    }

    pub(crate) fn caused_by(mut self, event_id: impl Into<String>) -> Self {
        self.caused_by_event_id = Some(event_id.into());
        self
    }

    pub(crate) fn with_cause(mut self, cause: impl Into<String>) -> Self {
        self.cause_chain.push(cause.into());
        self
    }
}

impl WorkflowService {
    pub(crate) fn record_workflow_diagnostic_error_if_configured(
        &self,
        request: WorkflowDiagnosticErrorRecordRequest,
    ) -> Result<WorkflowDiagnosticErrorRecordOutcome, WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(WorkflowDiagnosticErrorRecordOutcome::unavailable(
                "diagnostics ledger is not configured",
            ));
        };
        let registry_entry = request.phase.registry_entry();
        if registry_entry.scope_kind != request.scope.kind() {
            return Err(WorkflowServiceError::Internal(format!(
                "diagnostic phase '{}' requires {:?} scope but received {:?}",
                registry_entry.phase_id,
                registry_entry.scope_kind,
                request.scope.kind()
            )));
        }
        let source_component = request
            .source_component
            .unwrap_or(registry_entry.default_source);
        if !registry_entry.allowed_sources.contains(&source_component) {
            return Err(WorkflowServiceError::Internal(format!(
                "diagnostic phase '{}' does not allow {:?} source",
                registry_entry.phase_id, source_component
            )));
        }

        let mut append_request = DiagnosticEventAppendRequest {
            source_component,
            source_instance_id: request.source_instance_id,
            occurred_at_ms: unix_timestamp_ms() as i64,
            workflow_run_id: None,
            workflow_id: None,
            workflow_version_id: None,
            workflow_semantic_version: None,
            node_id: None,
            node_type: None,
            node_version: None,
            runtime_id: None,
            runtime_version: None,
            model_id: None,
            model_version: None,
            client_id: None,
            client_session_id: None,
            bucket_id: None,
            scheduler_policy_id: None,
            retention_policy_id: None,
            privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
            retention_class: DiagnosticEventRetentionClass::AuditMetadata,
            payload_ref: None,
            payload: DiagnosticEventPayload::DiagnosticErrorOccurred(
                DiagnosticErrorOccurredPayload {
                    phase: registry_entry.phase_id.to_string(),
                    scope: registry_entry.scope_kind,
                    severity: request.severity.unwrap_or(registry_entry.default_severity),
                    code: registry_entry.code.to_string(),
                    message: sanitize_error_text(&request.message, MAX_DIAGNOSTIC_ERROR_TEXT_LEN),
                    technical_message: request
                        .technical_message
                        .as_deref()
                        .map(|value| sanitize_error_text(value, MAX_DIAGNOSTIC_ERROR_TEXT_LEN)),
                    cause_chain: sanitize_cause_chain(request.cause_chain),
                    recoverability: request
                        .recoverability
                        .unwrap_or(registry_entry.default_recoverability),
                    location: request.location,
                    related_event_ids: request.related_event_ids,
                    caused_by_event_id: request.caused_by_event_id,
                },
            ),
        };
        request.scope.append_request_fields(&mut append_request);

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        match self
            .append_diagnostic_event_and_request_projection_refresh(&mut *ledger, append_request)
        {
            Ok(record) => Ok(WorkflowDiagnosticErrorRecordOutcome::recorded(
                record.event_id,
            )),
            Err(error) => Ok(WorkflowDiagnosticErrorRecordOutcome::unavailable(format!(
                "diagnostics ledger append failed: {}",
                sanitize_error_text(&error.to_string(), MAX_DIAGNOSTIC_ERROR_TEXT_LEN)
            ))),
        }
    }
}

fn sanitize_error_text(value: &str, max_len: usize) -> String {
    sanitize_diagnostic_error_text(value, max_len)
}

fn sanitize_cause_chain(cause_chain: Vec<String>) -> Vec<String> {
    cause_chain
        .into_iter()
        .take(MAX_DIAGNOSTIC_ERROR_CAUSE_COUNT)
        .map(|cause| sanitize_error_text(&cause, MAX_DIAGNOSTIC_ERROR_CAUSE_LEN))
        .collect()
}

fn apply_run_context(
    request: &mut DiagnosticEventAppendRequest,
    context: &WorkflowDiagnosticRunContext,
) {
    request.workflow_run_id = Some(context.workflow_run_id.clone());
    request.workflow_id = Some(context.workflow_id.clone());
    request.workflow_version_id = context.workflow_version_id.clone();
    request.workflow_semantic_version = context.workflow_semantic_version.clone();
    request.client_id = context.client_id.clone();
    request.client_session_id = context.client_session_id.clone();
    request.bucket_id = context.bucket_id.clone();
    request.scheduler_policy_id = context.scheduler_policy_id.clone();
    request.retention_policy_id = context.retention_policy_id.clone();
}
