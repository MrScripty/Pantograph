use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Mutex;

use pantograph_diagnostics_ledger::{
    sanitize_diagnostic_error_text, DiagnosticEventAppendRequest, DiagnosticEventPayload,
    DiagnosticEventPrivacyClass, DiagnosticEventRetentionClass, DiagnosticEventSourceComponent,
    DiagnosticsLedgerError, DiagnosticsLedgerRepository, ExecutionGuaranteeLevel,
    InferenceCompatibilityIssueDiagnosticSummary, InferenceCompatibilityReportDiagnosticSummary,
    InferenceExecutionDiagnosticObservedPayload, InferenceKvCacheDiagnosticSummary,
    InferenceOptionDiagnosticSummary, InferenceOptionSupportCounts,
    InferenceUsageDiagnosticSummary, LicenseSnapshot, ModelIdentity, ModelLicenseUsageEvent,
    ModelOutputMeasurement, NodeExecutionProjectionStatus, NodeExecutionStatusPayload,
    RetentionClass, UsageEventStatus, UsageLineage, MAX_DIAGNOSTIC_ERROR_TEXT_LEN,
    MAX_INFERENCE_COMPATIBILITY_ISSUES, MAX_INFERENCE_OPTION_DIAGNOSTICS,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, UsageEventId, WorkflowId, WorkflowRunId,
};
use thiserror::Error;

use crate::{
    ManagedCapabilityKind, ModelExecutionCapability, NodeExecutionContext, NodeExecutionGuarantee,
    SharedWorkflowService,
};

#[derive(Debug, Error)]
pub enum RuntimeLedgerSubmissionError {
    #[error("model execution capability route does not match node execution context")]
    ContextMismatch,
    #[error("model execution capability is unavailable")]
    CapabilityUnavailable,
    #[error("model usage completed before it started")]
    InvalidTimeRange,
    #[error("diagnostics ledger submission failed: {0}")]
    Ledger(#[from] DiagnosticsLedgerError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedModelUsageSubmission {
    pub model: ModelIdentity,
    pub license_snapshot: LicenseSnapshot,
    pub output_measurement: ModelOutputMeasurement,
    pub status: UsageEventStatus,
    pub started_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub output_port_ids: Vec<String>,
    pub correlation_id: Option<String>,
    pub retention_class: RetentionClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmittedModelUsageEvent {
    pub event: ModelLicenseUsageEvent,
}

pub fn inference_lifecycle_event_ledger_append_request(
    context: &NodeExecutionContext,
    event: &inference::InferenceRequestLifecycleEvent,
) -> Option<DiagnosticEventAppendRequest> {
    build_inference_lifecycle_event_ledger_append_request(
        &InferenceLifecycleLedgerAppendContext::from_node_execution_context(context),
        event,
        None,
    )
}

pub fn inference_diagnostic_event_ledger_append_request(
    context: &NodeExecutionContext,
    event: &inference::InferenceRequestLifecycleEvent,
) -> Option<DiagnosticEventAppendRequest> {
    inference_diagnostic_event_ledger_append_request_with_duration(context, event, None)
}

pub(crate) fn inference_diagnostic_event_ledger_append_request_with_duration(
    context: &NodeExecutionContext,
    event: &inference::InferenceRequestLifecycleEvent,
    duration_ms: Option<u64>,
) -> Option<DiagnosticEventAppendRequest> {
    build_inference_diagnostic_event_ledger_append_request(
        &InferenceLifecycleLedgerAppendContext::from_node_execution_context(context),
        event,
        duration_ms,
    )
}

#[derive(Debug, Default)]
pub struct InferenceLifecycleLedgerRecorder {
    recorder: InferenceLifecycleDurationRecorder,
}

impl InferenceLifecycleLedgerRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_request(
        &mut self,
        context: &NodeExecutionContext,
        event: &inference::InferenceRequestLifecycleEvent,
    ) -> Option<DiagnosticEventAppendRequest> {
        self.recorder.append_request(
            &InferenceLifecycleLedgerAppendContext::from_node_execution_context(context),
            event,
        )
    }
}

pub struct InferenceLifecycleWorkflowLedgerSink {
    workflow_service: SharedWorkflowService,
    execution_id: String,
    contexts_by_node_id: BTreeMap<String, InferenceLifecycleWorkflowNodeContext>,
    recorder: Mutex<InferenceLifecycleDurationRecorder>,
}

pub struct NodeExecutionWorkflowLedgerSink {
    workflow_service: SharedWorkflowService,
    workflow_id: WorkflowId,
    workflow_run_id: WorkflowRunId,
    execution_id: String,
    contexts_by_node_id: BTreeMap<String, NodeExecutionWorkflowLedgerNodeContext>,
    inner: Option<std::sync::Arc<dyn node_engine::EventSink>>,
}

impl InferenceLifecycleWorkflowLedgerSink {
    pub fn try_new(
        workflow_service: SharedWorkflowService,
        workflow_id: impl Into<String>,
        workflow_run_id: impl Into<String>,
        execution_id: impl Into<String>,
        graph: &node_engine::WorkflowGraph,
    ) -> Result<Self, String> {
        let workflow_id = WorkflowId::try_from(workflow_id.into()).map_err(|error| {
            format!("invalid workflow id for inference lifecycle ledger sink: {error}")
        })?;
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.into()).map_err(|error| {
            format!("invalid workflow run id for inference lifecycle ledger sink: {error}")
        })?;
        Ok(Self::new(
            workflow_service,
            workflow_id,
            workflow_run_id,
            execution_id,
            graph,
        ))
    }

    pub fn new(
        workflow_service: SharedWorkflowService,
        workflow_id: WorkflowId,
        workflow_run_id: WorkflowRunId,
        execution_id: impl Into<String>,
        graph: &node_engine::WorkflowGraph,
    ) -> Self {
        let contexts_by_node_id = graph
            .nodes
            .iter()
            .map(|node| {
                (
                    node.id.clone(),
                    InferenceLifecycleWorkflowNodeContext {
                        workflow_id: workflow_id.clone(),
                        workflow_run_id: workflow_run_id.clone(),
                        node_id: node.id.clone(),
                        node_type: node.node_type.clone(),
                    },
                )
            })
            .collect();

        Self {
            workflow_service,
            execution_id: execution_id.into(),
            contexts_by_node_id,
            recorder: Mutex::new(InferenceLifecycleDurationRecorder::default()),
        }
    }

    fn context_for_event(
        &self,
        event: &inference::InferenceRequestLifecycleEvent,
    ) -> Option<InferenceLifecycleLedgerAppendContext<'_>> {
        let request_id = event.request_id.as_deref()?;
        let request_suffix = request_id.strip_prefix(&format!("{}:", self.execution_id))?;
        self.contexts_by_node_id
            .iter()
            .find(|(node_id, _)| {
                request_suffix == node_id.as_str()
                    || request_suffix
                        .strip_prefix(node_id.as_str())
                        .is_some_and(|suffix| suffix.starts_with(':'))
            })
            .map(|(_, context)| {
                InferenceLifecycleLedgerAppendContext::from_workflow_node_context(context)
            })
    }
}

impl NodeExecutionWorkflowLedgerSink {
    pub fn try_new(
        workflow_service: SharedWorkflowService,
        workflow_id: impl Into<String>,
        workflow_run_id: impl Into<String>,
        execution_id: impl Into<String>,
        graph: &node_engine::WorkflowGraph,
        inner: Option<std::sync::Arc<dyn node_engine::EventSink>>,
    ) -> Result<Self, String> {
        let workflow_id = WorkflowId::try_from(workflow_id.into()).map_err(|error| {
            format!("invalid workflow id for node execution ledger sink: {error}")
        })?;
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.into()).map_err(|error| {
            format!("invalid workflow run id for node execution ledger sink: {error}")
        })?;
        let contexts_by_node_id = graph
            .nodes
            .iter()
            .map(|node| {
                (
                    node.id.clone(),
                    NodeExecutionWorkflowLedgerNodeContext {
                        node_id: node.id.clone(),
                        node_type: node.node_type.clone(),
                    },
                )
            })
            .collect();

        Ok(Self {
            workflow_service,
            workflow_id,
            workflow_run_id,
            execution_id: execution_id.into(),
            contexts_by_node_id,
            inner,
        })
    }

    fn record_kv_cache_diagnostic(&self, event: &node_engine::WorkflowEvent) {
        let Some(request) = build_kv_cache_diagnostic_event_ledger_append_request(
            &self.workflow_id,
            &self.workflow_run_id,
            &self.execution_id,
            &self.contexts_by_node_id,
            event,
        ) else {
            return;
        };

        if let Err(error) = self
            .workflow_service
            .workflow_diagnostic_event_record(request)
        {
            log::warn!("failed to record KV cache diagnostic event: {error}");
        }
    }
}

impl inference::InferenceRequestLifecycleEventSink for InferenceLifecycleWorkflowLedgerSink {
    fn record(&self, event: inference::InferenceRequestLifecycleEvent) {
        let Some(context) = self.context_for_event(&event) else {
            return;
        };
        let Some(request) = self
            .recorder
            .lock()
            .ok()
            .and_then(|mut recorder| recorder.append_request(&context, &event))
        else {
            return;
        };
        let duration_ms = inference_lifecycle_duration_ms_from_append_request(&request);

        if let Err(error) = self
            .workflow_service
            .workflow_diagnostic_event_record(request)
        {
            log::warn!("failed to record inference lifecycle diagnostic event: {error}");
        }

        if let Some(request) =
            build_inference_diagnostic_event_ledger_append_request(&context, &event, duration_ms)
        {
            if let Err(error) = self
                .workflow_service
                .workflow_diagnostic_event_record(request)
            {
                log::warn!("failed to record inference option diagnostic event: {error}");
            }
        }
    }
}

impl node_engine::EventSink for NodeExecutionWorkflowLedgerSink {
    fn send(&self, event: node_engine::WorkflowEvent) -> Result<(), node_engine::EventError> {
        self.record_kv_cache_diagnostic(&event);

        if let Some(inner) = &self.inner {
            inner.send(event)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct InferenceLifecycleWorkflowNodeContext {
    workflow_id: WorkflowId,
    workflow_run_id: WorkflowRunId,
    node_id: String,
    node_type: String,
}

#[derive(Debug, Clone)]
struct NodeExecutionWorkflowLedgerNodeContext {
    node_id: String,
    node_type: String,
}

#[derive(Debug, Clone)]
struct InferenceLifecycleLedgerAppendContext<'a> {
    workflow_id: &'a WorkflowId,
    workflow_run_id: &'a WorkflowRunId,
    node_id: &'a str,
    node_type: &'a str,
    node_version: Option<&'a String>,
    client_id: Option<&'a ClientId>,
    client_session_id: Option<&'a ClientSessionId>,
    bucket_id: Option<&'a BucketId>,
}

impl<'a> InferenceLifecycleLedgerAppendContext<'a> {
    fn from_node_execution_context(context: &'a NodeExecutionContext) -> Self {
        Self {
            workflow_id: context.workflow_id(),
            workflow_run_id: &context.attribution().workflow_run_id,
            node_id: context.node_id().as_str(),
            node_type: context.node_type().as_str(),
            node_version: context
                .effective_contract()
                .static_contract
                .contract_version
                .as_ref(),
            client_id: Some(&context.attribution().client_id),
            client_session_id: Some(&context.attribution().client_session_id),
            bucket_id: Some(&context.attribution().bucket_id),
        }
    }

    fn from_workflow_node_context(context: &'a InferenceLifecycleWorkflowNodeContext) -> Self {
        Self {
            workflow_id: &context.workflow_id,
            workflow_run_id: &context.workflow_run_id,
            node_id: &context.node_id,
            node_type: &context.node_type,
            node_version: None,
            client_id: None,
            client_session_id: None,
            bucket_id: None,
        }
    }
}

#[derive(Debug, Default)]
struct InferenceLifecycleDurationRecorder {
    started_at_ms_by_key: HashMap<InferenceLifecycleDurationKey, i64>,
}

impl InferenceLifecycleDurationRecorder {
    fn append_request(
        &mut self,
        context: &InferenceLifecycleLedgerAppendContext<'_>,
        event: &inference::InferenceRequestLifecycleEvent,
    ) -> Option<DiagnosticEventAppendRequest> {
        let occurred_at_ms = i64::try_from(event.occurred_at_ms).unwrap_or(i64::MAX);
        let duration_key = InferenceLifecycleDurationKey::from_append_context(context, event);
        let duration_ms = match event.kind {
            inference::InferenceRequestLifecycleEventKind::Started => {
                if let Some(key) = duration_key {
                    self.started_at_ms_by_key.insert(key, occurred_at_ms);
                }
                None
            }
            inference::InferenceRequestLifecycleEventKind::Completed
            | inference::InferenceRequestLifecycleEventKind::Failed
            | inference::InferenceRequestLifecycleEventKind::Cancelled => duration_key
                .and_then(|key| self.started_at_ms_by_key.remove(&key))
                .and_then(|started_at_ms| occurred_at_ms.checked_sub(started_at_ms))
                .map(|duration_ms| duration_ms as u64),
            inference::InferenceRequestLifecycleEventKind::CleanupCompleted => {
                if let Some(key) = duration_key {
                    self.started_at_ms_by_key.remove(&key);
                }
                None
            }
        };

        build_inference_lifecycle_event_ledger_append_request(context, event, duration_ms)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct InferenceLifecycleDurationKey {
    node_id: String,
    request_id: String,
    phase: &'static str,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
}

impl InferenceLifecycleDurationKey {
    fn from_append_context(
        context: &InferenceLifecycleLedgerAppendContext<'_>,
        event: &inference::InferenceRequestLifecycleEvent,
    ) -> Option<Self> {
        Some(Self {
            node_id: context.node_id.to_string(),
            request_id: event.request_id.clone()?,
            phase: inference_lifecycle_phase_key(&event.phase),
            backend_key: event.backend_key.clone(),
            runtime_id: event.runtime_id.clone(),
            runtime_instance_id: event.runtime_instance_id.clone(),
        })
    }
}

fn inference_lifecycle_phase_key(phase: &inference::InferenceLifecyclePhase) -> &'static str {
    match phase {
        inference::InferenceLifecyclePhase::ModelPackageResolution => "model_package_resolution",
        inference::InferenceLifecyclePhase::TaskValidation => "task_validation",
        inference::InferenceLifecyclePhase::Preprocessing => "preprocessing",
        inference::InferenceLifecyclePhase::BackendExecution => "backend_execution",
        inference::InferenceLifecyclePhase::Postprocessing => "postprocessing",
        inference::InferenceLifecyclePhase::ResultProjection => "result_projection",
    }
}

fn inference_lifecycle_event_kind_key(
    kind: &inference::InferenceRequestLifecycleEventKind,
) -> &'static str {
    match kind {
        inference::InferenceRequestLifecycleEventKind::Started => "started",
        inference::InferenceRequestLifecycleEventKind::Completed => "completed",
        inference::InferenceRequestLifecycleEventKind::Failed => "failed",
        inference::InferenceRequestLifecycleEventKind::Cancelled => "cancelled",
        inference::InferenceRequestLifecycleEventKind::CleanupCompleted => "cleanup_completed",
    }
}

fn build_inference_lifecycle_event_ledger_append_request(
    context: &InferenceLifecycleLedgerAppendContext<'_>,
    event: &inference::InferenceRequestLifecycleEvent,
    duration_ms: Option<u64>,
) -> Option<DiagnosticEventAppendRequest> {
    let status = match event.kind {
        inference::InferenceRequestLifecycleEventKind::Started => {
            NodeExecutionProjectionStatus::Running
        }
        inference::InferenceRequestLifecycleEventKind::Completed => {
            NodeExecutionProjectionStatus::Completed
        }
        inference::InferenceRequestLifecycleEventKind::Failed => {
            NodeExecutionProjectionStatus::Failed
        }
        inference::InferenceRequestLifecycleEventKind::Cancelled => {
            NodeExecutionProjectionStatus::Cancelled
        }
        inference::InferenceRequestLifecycleEventKind::CleanupCompleted => return None,
    };
    let occurred_at_ms = i64::try_from(event.occurred_at_ms).unwrap_or(i64::MAX);
    let terminal = matches!(
        status,
        NodeExecutionProjectionStatus::Completed
            | NodeExecutionProjectionStatus::Failed
            | NodeExecutionProjectionStatus::Cancelled
    );

    Some(DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::NodeExecution,
        source_instance_id: event.runtime_instance_id.clone(),
        occurred_at_ms,
        workflow_run_id: Some(context.workflow_run_id.clone()),
        workflow_id: Some(context.workflow_id.clone()),
        workflow_version_id: None,
        workflow_semantic_version: None,
        node_id: Some(context.node_id.to_string()),
        node_type: Some(context.node_type.to_string()),
        node_version: context.node_version.cloned(),
        runtime_id: event
            .runtime_id
            .clone()
            .or_else(|| event.backend_key.clone()),
        runtime_version: None,
        model_id: event.model_id.clone(),
        model_version: None,
        client_id: context.client_id.cloned(),
        client_session_id: context.client_session_id.cloned(),
        bucket_id: context.bucket_id.cloned(),
        scheduler_policy_id: None,
        retention_policy_id: None,
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::NodeExecutionStatus(NodeExecutionStatusPayload {
            status,
            started_at_ms: if event.kind == inference::InferenceRequestLifecycleEventKind::Started {
                Some(occurred_at_ms)
            } else {
                None
            },
            completed_at_ms: if terminal { Some(occurred_at_ms) } else { None },
            duration_ms,
            error: event
                .detail
                .as_deref()
                .map(sanitize_inference_lifecycle_error_detail),
            canonical_error_event_id: event.canonical_error_event_id.clone(),
            task_id: event.task_id.clone(),
            selected_backend_key: event.backend_key.clone(),
        }),
    })
}

fn sanitize_inference_lifecycle_error_detail(value: &str) -> String {
    sanitize_diagnostic_error_text(value, MAX_DIAGNOSTIC_ERROR_TEXT_LEN)
}

fn inference_lifecycle_duration_ms_from_append_request(
    request: &DiagnosticEventAppendRequest,
) -> Option<u64> {
    match &request.payload {
        DiagnosticEventPayload::NodeExecutionStatus(payload) => payload.duration_ms,
        _ => None,
    }
}

fn build_kv_cache_diagnostic_event_ledger_append_request(
    workflow_id: &WorkflowId,
    workflow_run_id: &WorkflowRunId,
    execution_id: &str,
    contexts_by_node_id: &BTreeMap<String, NodeExecutionWorkflowLedgerNodeContext>,
    event: &node_engine::WorkflowEvent,
) -> Option<DiagnosticEventAppendRequest> {
    let node_engine::WorkflowEvent::TaskProgress {
        task_id,
        execution_id: event_execution_id,
        detail: Some(node_engine::TaskProgressDetail::KvCache(detail)),
        occurred_at_ms,
        ..
    } = event
    else {
        return None;
    };
    if event_execution_id != execution_id {
        return None;
    }
    let context = contexts_by_node_id.get(task_id)?;
    let occurred_at_ms = occurred_at_ms
        .and_then(|value| i64::try_from(value).ok())
        .unwrap_or_else(|| {
            i64::try_from(crate::workflow_runtime::unix_timestamp_ms()).unwrap_or(i64::MAX)
        });

    Some(DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::NodeExecution,
        source_instance_id: None,
        occurred_at_ms,
        workflow_run_id: Some(workflow_run_id.clone()),
        workflow_id: Some(workflow_id.clone()),
        workflow_version_id: None,
        workflow_semantic_version: None,
        node_id: Some(context.node_id.clone()),
        node_type: Some(context.node_type.clone()),
        node_version: None,
        runtime_id: detail.backend_key.clone(),
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
        payload: DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(
            InferenceExecutionDiagnosticObservedPayload {
                request_id: format!("{task_id}:kv_cache"),
                task_id: "kv_cache".to_string(),
                lifecycle_phase: Some("kv_cache".to_string()),
                lifecycle_event_kind: Some("progress".to_string()),
                duration_ms: None,
                selected_backend_key: detail.backend_key.clone(),
                selected_backend_family: selected_backend_family(
                    detail.backend_key.as_deref(),
                    None,
                ),
                selected_device_id: None,
                selected_network_node_id: None,
                resolved_artifact_kind: None,
                usage: None,
                cache_handle_id: None,
                kv_cache: Some(kv_cache_diagnostic_summary(detail)),
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
                option_support_counts: kv_cache_option_support_counts(&detail.option_diagnostics),
                option_diagnostics: detail
                    .option_diagnostics
                    .iter()
                    .take(MAX_INFERENCE_OPTION_DIAGNOSTICS)
                    .map(kv_cache_option_diagnostic_summary)
                    .collect(),
            },
        ),
    })
}

fn build_inference_diagnostic_event_ledger_append_request(
    context: &InferenceLifecycleLedgerAppendContext<'_>,
    event: &inference::InferenceRequestLifecycleEvent,
    duration_ms: Option<u64>,
) -> Option<DiagnosticEventAppendRequest> {
    let has_bounded_diagnostics = !event.option_diagnostics.is_empty()
        || event.compatibility_report.is_some()
        || !event.compatibility_issues.is_empty()
        || event.usage.is_some()
        || event.cache_handle_id.is_some()
        || duration_ms.is_some();
    if !has_bounded_diagnostics
        || !inference_diagnostic_phase_is_persistable(event)
        || !inference_diagnostic_event_kind_is_persistable(event)
    {
        return None;
    }
    let occurred_at_ms = i64::try_from(event.occurred_at_ms).unwrap_or(i64::MAX);

    Some(DiagnosticEventAppendRequest {
        source_component: DiagnosticEventSourceComponent::NodeExecution,
        source_instance_id: event.runtime_instance_id.clone(),
        occurred_at_ms,
        workflow_run_id: Some(context.workflow_run_id.clone()),
        workflow_id: Some(context.workflow_id.clone()),
        workflow_version_id: None,
        workflow_semantic_version: None,
        node_id: Some(context.node_id.to_string()),
        node_type: Some(context.node_type.to_string()),
        node_version: context.node_version.cloned(),
        runtime_id: event
            .runtime_id
            .clone()
            .or_else(|| event.backend_key.clone()),
        runtime_version: None,
        model_id: event.model_id.clone(),
        model_version: None,
        client_id: context.client_id.cloned(),
        client_session_id: context.client_session_id.cloned(),
        bucket_id: context.bucket_id.cloned(),
        scheduler_policy_id: None,
        retention_policy_id: None,
        privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
        retention_class: DiagnosticEventRetentionClass::AuditMetadata,
        payload_ref: None,
        payload: DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(
            InferenceExecutionDiagnosticObservedPayload {
                request_id: event
                    .request_id
                    .clone()
                    .unwrap_or_else(|| context.node_id.to_string()),
                task_id: event
                    .task_id
                    .clone()
                    .unwrap_or_else(|| inference_lifecycle_phase_key(&event.phase).to_string()),
                lifecycle_phase: Some(inference_lifecycle_phase_key(&event.phase).to_string()),
                lifecycle_event_kind: Some(
                    inference_lifecycle_event_kind_key(&event.kind).to_string(),
                ),
                duration_ms,
                selected_backend_key: event.backend_key.clone(),
                selected_backend_family: selected_backend_family(
                    event.backend_key.as_deref(),
                    event.runtime_id.as_deref(),
                ),
                selected_device_id: event.selected_device_id.clone(),
                selected_network_node_id: event.selected_network_node_id.clone(),
                resolved_artifact_kind: event.resolved_artifact_kind.clone(),
                usage: event.usage.as_ref().map(inference_usage_summary),
                cache_handle_id: event.cache_handle_id.clone(),
                kv_cache: None,
                compatibility_report: event
                    .compatibility_report
                    .as_ref()
                    .map(compatibility_report_summary),
                compatibility_issue_count: event.compatibility_issues.len().min(u32::MAX as usize)
                    as u32,
                compatibility_issues: event
                    .compatibility_issues
                    .iter()
                    .take(MAX_INFERENCE_COMPATIBILITY_ISSUES)
                    .map(|issue| compatibility_issue_summary(issue, event.model_id.as_deref()))
                    .collect(),
                option_support_counts: option_support_counts(&event.option_diagnostics),
                option_diagnostics: event
                    .option_diagnostics
                    .iter()
                    .take(MAX_INFERENCE_OPTION_DIAGNOSTICS)
                    .map(option_diagnostic_summary)
                    .collect(),
            },
        ),
    })
}

fn selected_backend_family(backend_key: Option<&str>, runtime_id: Option<&str>) -> Option<String> {
    let evidence = backend_key
        .into_iter()
        .chain(runtime_id)
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();

    if evidence
        .iter()
        .any(|value| value.contains("llama.cpp") || value.contains("llamacpp"))
    {
        return Some("llama_cpp".to_string());
    }
    if evidence
        .iter()
        .any(|value| value.contains("transformers") || value.contains("pytorch"))
    {
        return Some("transformers_pytorch".to_string());
    }
    if evidence.iter().any(|value| value.contains("vllm")) {
        return Some("vllm".to_string());
    }
    if evidence.iter().any(|value| value.contains("mlx")) {
        return Some("mlx".to_string());
    }
    if evidence.iter().any(|value| value.contains("candle")) {
        return Some("candle".to_string());
    }
    if evidence.iter().any(|value| value.contains("diffusers")) {
        return Some("diffusers".to_string());
    }
    if evidence
        .iter()
        .any(|value| value.contains("onnxruntime") || value.contains("onnx-runtime"))
    {
        return Some("onnxruntime".to_string());
    }

    backend_key
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn inference_diagnostic_phase_is_persistable(
    event: &inference::InferenceRequestLifecycleEvent,
) -> bool {
    match event.phase {
        inference::InferenceLifecyclePhase::ModelPackageResolution
        | inference::InferenceLifecyclePhase::Preprocessing
        | inference::InferenceLifecyclePhase::Postprocessing
        | inference::InferenceLifecyclePhase::ResultProjection => true,
        inference::InferenceLifecyclePhase::TaskValidation => {
            event.compatibility_report.is_some()
                || !event.compatibility_issues.is_empty()
                || !event.option_diagnostics.is_empty()
        }
        inference::InferenceLifecyclePhase::BackendExecution => true,
    }
}

fn inference_diagnostic_event_kind_is_persistable(
    event: &inference::InferenceRequestLifecycleEvent,
) -> bool {
    matches!(
        event.kind,
        inference::InferenceRequestLifecycleEventKind::Completed
            | inference::InferenceRequestLifecycleEventKind::Failed
            | inference::InferenceRequestLifecycleEventKind::Cancelled
    )
}

fn inference_usage_summary(usage: &inference::InferenceUsage) -> InferenceUsageDiagnosticSummary {
    InferenceUsageDiagnosticSummary {
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        total_tokens: usage.total_tokens,
    }
}

fn kv_cache_diagnostic_summary(
    detail: &node_engine::KvCacheExecutionDiagnostics,
) -> InferenceKvCacheDiagnosticSummary {
    InferenceKvCacheDiagnosticSummary {
        action: kv_cache_action_label(&detail.action).to_string(),
        outcome: kv_cache_outcome_label(&detail.outcome).to_string(),
        cache_id: detail.cache_id.clone(),
        backend_key: detail.backend_key.clone(),
        reuse_source: detail.reuse_source.clone(),
        token_count: detail
            .token_count
            .and_then(|token_count| u64::try_from(token_count).ok()),
        reason: detail.reason.clone(),
    }
}

fn kv_cache_action_label(action: &node_engine::KvCacheEventAction) -> &'static str {
    match action {
        node_engine::KvCacheEventAction::RestoreInput => "restore_input",
        node_engine::KvCacheEventAction::CaptureOutput => "capture_output",
        node_engine::KvCacheEventAction::Truncate => "truncate",
    }
}

fn kv_cache_outcome_label(outcome: &node_engine::KvCacheEventOutcome) -> &'static str {
    match outcome {
        node_engine::KvCacheEventOutcome::Hit => "hit",
        node_engine::KvCacheEventOutcome::Miss => "miss",
        node_engine::KvCacheEventOutcome::Saved => "saved",
        node_engine::KvCacheEventOutcome::Invalidated => "invalidated",
        node_engine::KvCacheEventOutcome::Unsupported => "unsupported",
        node_engine::KvCacheEventOutcome::Truncated => "truncated",
    }
}

fn compatibility_report_summary(
    report: &inference::InferenceCompatibilityReportSummary,
) -> InferenceCompatibilityReportDiagnosticSummary {
    InferenceCompatibilityReportDiagnosticSummary {
        status: report.status.clone(),
        compatible: report.compatible,
        task: report.task.clone(),
        model_source: report.model_source.clone(),
        preprocessing: report.preprocessing.clone(),
        postprocessing: report.postprocessing.clone(),
    }
}

fn compatibility_issue_summary(
    issue: &inference::InferenceCompatibilityIssueSummary,
    event_model_id: Option<&str>,
) -> InferenceCompatibilityIssueDiagnosticSummary {
    InferenceCompatibilityIssueDiagnosticSummary {
        kind: issue.kind.clone(),
        phase: inference_lifecycle_phase_key(&issue.phase).to_string(),
        message: issue.message.clone(),
        model_id: issue.model_id.clone(),
        path: bounded_compatibility_issue_path(issue, event_model_id),
    }
}

fn bounded_compatibility_issue_path(
    issue: &inference::InferenceCompatibilityIssueSummary,
    event_model_id: Option<&str>,
) -> Option<String> {
    let has_stable_model_id = issue
        .model_id
        .as_deref()
        .or(event_model_id)
        .is_some_and(|model_id| !model_id.trim().is_empty());
    issue.path.as_ref().and_then(|path| {
        if has_stable_model_id && Path::new(path).is_absolute() {
            None
        } else {
            Some(path.clone())
        }
    })
}

fn option_support_counts(
    diagnostics: &[inference::OptionCompatibilityDiagnostic],
) -> InferenceOptionSupportCounts {
    let mut counts = InferenceOptionSupportCounts::default();
    for diagnostic in diagnostics {
        match diagnostic.state {
            inference::OptionSupportState::Honored => counts.honored += 1,
            inference::OptionSupportState::Mapped => counts.mapped += 1,
            inference::OptionSupportState::Defaulted => counts.defaulted += 1,
            inference::OptionSupportState::Ignored => counts.ignored += 1,
            inference::OptionSupportState::Unsupported => counts.unsupported += 1,
            inference::OptionSupportState::Rejected => counts.rejected += 1,
            inference::OptionSupportState::Conflict => counts.conflict += 1,
            inference::OptionSupportState::ModelUnavailable => counts.model_unavailable += 1,
            inference::OptionSupportState::BackendUnavailable => counts.backend_unavailable += 1,
            inference::OptionSupportState::RequiresModelSupport => {
                counts.requires_model_support += 1;
            }
            inference::OptionSupportState::RequiresBackendSupport => {
                counts.requires_backend_support += 1;
            }
        }
    }
    counts
}

fn kv_cache_option_support_counts(
    diagnostics: &[node_engine::KvCacheOptionDiagnostic],
) -> InferenceOptionSupportCounts {
    let mut counts = InferenceOptionSupportCounts::default();
    for diagnostic in diagnostics {
        match diagnostic.state {
            node_engine::KvCacheOptionSupportState::Honored => counts.honored += 1,
            node_engine::KvCacheOptionSupportState::Ignored => counts.ignored += 1,
            node_engine::KvCacheOptionSupportState::Rejected => counts.rejected += 1,
            node_engine::KvCacheOptionSupportState::Conflict => counts.conflict += 1,
        }
    }
    counts
}

fn kv_cache_option_diagnostic_summary(
    diagnostic: &node_engine::KvCacheOptionDiagnostic,
) -> InferenceOptionDiagnosticSummary {
    InferenceOptionDiagnosticSummary {
        option_path: diagnostic.option_path.clone(),
        state: kv_cache_option_support_state_label(diagnostic.state).to_string(),
        backend_key: diagnostic.backend_key.clone(),
        message: diagnostic.message.clone(),
    }
}

fn kv_cache_option_support_state_label(
    state: node_engine::KvCacheOptionSupportState,
) -> &'static str {
    match state {
        node_engine::KvCacheOptionSupportState::Honored => "honored",
        node_engine::KvCacheOptionSupportState::Ignored => "ignored",
        node_engine::KvCacheOptionSupportState::Rejected => "rejected",
        node_engine::KvCacheOptionSupportState::Conflict => "conflict",
    }
}

fn option_diagnostic_summary(
    diagnostic: &inference::OptionCompatibilityDiagnostic,
) -> InferenceOptionDiagnosticSummary {
    InferenceOptionDiagnosticSummary {
        option_path: diagnostic.option_path.clone(),
        state: option_support_state_label(diagnostic.state).to_string(),
        backend_key: diagnostic.backend_key.clone(),
        message: diagnostic.message.clone(),
    }
}

fn option_support_state_label(state: inference::OptionSupportState) -> &'static str {
    match state {
        inference::OptionSupportState::Honored => "honored",
        inference::OptionSupportState::Mapped => "mapped",
        inference::OptionSupportState::Defaulted => "defaulted",
        inference::OptionSupportState::Ignored => "ignored",
        inference::OptionSupportState::Unsupported => "unsupported",
        inference::OptionSupportState::Rejected => "rejected",
        inference::OptionSupportState::Conflict => "conflict",
        inference::OptionSupportState::ModelUnavailable => "model_unavailable",
        inference::OptionSupportState::BackendUnavailable => "backend_unavailable",
        inference::OptionSupportState::RequiresModelSupport => "requires_model_support",
        inference::OptionSupportState::RequiresBackendSupport => "requires_backend_support",
    }
}

impl ManagedModelUsageSubmission {
    pub fn completed(
        model: ModelIdentity,
        license_snapshot: LicenseSnapshot,
        output_measurement: ModelOutputMeasurement,
        started_at_ms: i64,
        completed_at_ms: i64,
    ) -> Self {
        Self {
            model,
            license_snapshot,
            output_measurement,
            status: UsageEventStatus::Completed,
            started_at_ms,
            completed_at_ms: Some(completed_at_ms),
            output_port_ids: Vec::new(),
            correlation_id: None,
            retention_class: RetentionClass::Standard,
        }
    }
}

impl ModelExecutionCapability {
    pub fn submit_usage_event(
        &self,
        ledger: &mut impl DiagnosticsLedgerRepository,
        context: &NodeExecutionContext,
        submission: ManagedModelUsageSubmission,
    ) -> Result<SubmittedModelUsageEvent, RuntimeLedgerSubmissionError> {
        let event = self.build_usage_event(context, submission)?;
        ledger.record_usage_event(event.clone())?;
        Ok(SubmittedModelUsageEvent { event })
    }

    pub fn build_usage_event(
        &self,
        context: &NodeExecutionContext,
        submission: ManagedModelUsageSubmission,
    ) -> Result<ModelLicenseUsageEvent, RuntimeLedgerSubmissionError> {
        self.validate_for_context(context)?;
        if let Some(completed_at_ms) = submission.completed_at_ms {
            if completed_at_ms < submission.started_at_ms {
                return Err(RuntimeLedgerSubmissionError::InvalidTimeRange);
            }
        }

        Ok(ModelLicenseUsageEvent {
            usage_event_id: UsageEventId::generate(),
            client_id: context.attribution().client_id.clone(),
            client_session_id: context.attribution().client_session_id.clone(),
            bucket_id: context.attribution().bucket_id.clone(),
            workflow_run_id: context.attribution().workflow_run_id.clone(),
            workflow_id: context.workflow_id().clone(),
            workflow_version_id: None,
            workflow_semantic_version: None,
            model: submission.model,
            lineage: usage_lineage(context, submission.output_port_ids),
            license_snapshot: submission.license_snapshot,
            output_measurement: submission.output_measurement.clone(),
            guarantee_level: guarantee_level(context, &submission.output_measurement),
            status: submission.status,
            retention_class: submission.retention_class,
            started_at_ms: submission.started_at_ms,
            completed_at_ms: submission.completed_at_ms,
            correlation_id: submission.correlation_id,
        })
    }

    fn validate_for_context(
        &self,
        context: &NodeExecutionContext,
    ) -> Result<(), RuntimeLedgerSubmissionError> {
        if self.route.kind != ManagedCapabilityKind::ModelExecution {
            return Err(RuntimeLedgerSubmissionError::ContextMismatch);
        }
        if !self.route.available {
            return Err(RuntimeLedgerSubmissionError::CapabilityUnavailable);
        }
        if self.route.workflow_id != *context.workflow_id()
            || self.route.attribution != *context.attribution()
            || self.route.node_id != *context.node_id()
            || self.route.node_type != *context.node_type()
        {
            return Err(RuntimeLedgerSubmissionError::ContextMismatch);
        }
        Ok(())
    }
}

fn usage_lineage(context: &NodeExecutionContext, output_port_ids: Vec<String>) -> UsageLineage {
    let port_ids = if output_port_ids.is_empty() {
        context
            .effective_contract()
            .outputs
            .iter()
            .map(|port| port.base.id.as_str().to_string())
            .collect()
    } else {
        output_port_ids
    };

    UsageLineage {
        node_id: context.node_id().as_str().to_string(),
        node_type: context.node_type().as_str().to_string(),
        port_ids,
        composed_parent_chain: context
            .lineage()
            .composed_node_stack
            .iter()
            .map(|node_id| node_id.as_str().to_string())
            .collect(),
        effective_contract_version: context
            .effective_contract()
            .static_contract
            .contract_version
            .clone(),
        effective_contract_digest: context
            .effective_contract()
            .static_contract
            .contract_digest
            .clone(),
        metadata_json: context
            .lineage()
            .lineage_segment_id
            .as_ref()
            .map(|segment_id| serde_json::json!({ "lineageSegmentId": segment_id }).to_string()),
    }
}

fn guarantee_level(
    context: &NodeExecutionContext,
    output_measurement: &ModelOutputMeasurement,
) -> ExecutionGuaranteeLevel {
    match context.guarantee() {
        NodeExecutionGuarantee::ManagedFull
            if !output_measurement.unavailable_reasons.is_empty() =>
        {
            ExecutionGuaranteeLevel::ManagedPartial
        }
        NodeExecutionGuarantee::ManagedFull => ExecutionGuaranteeLevel::ManagedFull,
        NodeExecutionGuarantee::ManagedPartial => ExecutionGuaranteeLevel::ManagedPartial,
        NodeExecutionGuarantee::EscapeHatchDetected => ExecutionGuaranteeLevel::EscapeHatchDetected,
        NodeExecutionGuarantee::UnsafeOrUnobserved => ExecutionGuaranteeLevel::UnsafeOrUnobserved,
    }
}

#[cfg(test)]
#[path = "node_execution_ledger_tests.rs"]
mod tests;
