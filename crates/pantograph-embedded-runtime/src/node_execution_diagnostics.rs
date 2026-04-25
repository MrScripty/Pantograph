//! Runtime-owned node diagnostics event projection.
//!
//! This module adapts lower-level node-engine workflow events into enriched
//! transient runtime facts. The facts are not durable ledger records; Stage 04
//! owns durable model/license persistence.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use pantograph_node_contracts::{PortId, PortValueType};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunId,
};
use serde::{Deserialize, Serialize};

use crate::{NodeExecutionContext, NodeExecutionGuarantee, NodeLineageContext, NodeOutputSummary};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeExecutionDiagnosticEventKind {
    Started,
    Completed,
    Failed,
    WaitingForInput,
    Progress,
    Stream,
    Cancelled,
    GraphModified,
    IncrementalExecutionStarted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeExecutionDiagnosticEvent {
    pub kind: NodeExecutionDiagnosticEventKind,
    pub client_id: ClientId,
    pub client_session_id: ClientSessionId,
    pub bucket_id: BucketId,
    pub workflow_id: WorkflowId,
    pub workflow_run_id: WorkflowRunId,
    pub node_id: pantograph_node_contracts::NodeInstanceId,
    pub node_type: pantograph_node_contracts::NodeTypeId,
    pub attempt: u32,
    pub occurred_at_ms: u64,
    pub guarantee: NodeExecutionGuarantee,
    pub lineage: NodeLineageContext,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<PortId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_summaries: Vec<NodeOutputSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress_detail: Option<node_engine::TaskProgressDetail>,
}

#[derive(Default)]
pub struct NodeExecutionDiagnosticsRecorder {
    contexts_by_node_id: Mutex<BTreeMap<String, NodeExecutionContext>>,
    events: Mutex<Vec<NodeExecutionDiagnosticEvent>>,
    inner: Option<Arc<dyn node_engine::EventSink>>,
}

impl NodeExecutionDiagnosticsRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn forwarding_to(inner: Arc<dyn node_engine::EventSink>) -> Self {
        Self {
            contexts_by_node_id: Mutex::new(BTreeMap::new()),
            events: Mutex::new(Vec::new()),
            inner: Some(inner),
        }
    }

    pub fn register_context(
        &self,
        context: NodeExecutionContext,
    ) -> Result<(), node_engine::EventError> {
        let mut contexts = self.contexts_by_node_id.lock().map_err(lock_error)?;
        contexts.insert(context.node_id().as_str().to_string(), context);
        Ok(())
    }

    pub fn events(&self) -> Result<Vec<NodeExecutionDiagnosticEvent>, node_engine::EventError> {
        self.events
            .lock()
            .map(|events| events.clone())
            .map_err(lock_error)
    }
}

impl node_engine::EventSink for NodeExecutionDiagnosticsRecorder {
    fn send(&self, event: node_engine::WorkflowEvent) -> Result<(), node_engine::EventError> {
        let diagnostic_event = {
            let contexts = self.contexts_by_node_id.lock().map_err(lock_error)?;
            contexts
                .values()
                .find_map(|context| adapt_node_engine_diagnostic_event(context, &event))
        };

        if let Some(diagnostic_event) = diagnostic_event {
            self.events
                .lock()
                .map_err(lock_error)?
                .push(diagnostic_event);
        }

        if let Some(inner) = &self.inner {
            inner.send(event)?;
        }

        Ok(())
    }
}

pub fn adapt_node_engine_diagnostic_event(
    context: &NodeExecutionContext,
    event: &node_engine::WorkflowEvent,
) -> Option<NodeExecutionDiagnosticEvent> {
    match event {
        node_engine::WorkflowEvent::TaskStarted {
            task_id,
            execution_id,
            occurred_at_ms,
        } if matches_context(context, execution_id, task_id) => Some(base_event(
            context,
            NodeExecutionDiagnosticEventKind::Started,
            *occurred_at_ms,
        )),
        node_engine::WorkflowEvent::TaskCompleted {
            task_id,
            execution_id,
            output,
            occurred_at_ms,
        } if matches_context(context, execution_id, task_id) => {
            let mut event = base_event(
                context,
                NodeExecutionDiagnosticEventKind::Completed,
                *occurred_at_ms,
            );
            event.output_summaries = output_summaries(context, output.as_ref());
            Some(event)
        }
        node_engine::WorkflowEvent::TaskFailed {
            task_id,
            execution_id,
            error,
            occurred_at_ms,
        } if matches_context(context, execution_id, task_id) => {
            let mut event = base_event(
                context,
                NodeExecutionDiagnosticEventKind::Failed,
                *occurred_at_ms,
            );
            event.error = Some(error.clone());
            Some(event)
        }
        node_engine::WorkflowEvent::WaitingForInput {
            execution_id,
            task_id,
            prompt,
            occurred_at_ms,
            ..
        } if matches_context(context, execution_id, task_id) => {
            let mut event = base_event(
                context,
                NodeExecutionDiagnosticEventKind::WaitingForInput,
                *occurred_at_ms,
            );
            event.message = prompt.clone();
            Some(event)
        }
        node_engine::WorkflowEvent::TaskProgress {
            task_id,
            execution_id,
            progress,
            message,
            detail,
            occurred_at_ms,
        } if matches_context(context, execution_id, task_id) => {
            let mut event = base_event(
                context,
                NodeExecutionDiagnosticEventKind::Progress,
                *occurred_at_ms,
            );
            event.progress = Some(*progress);
            event.message = message.clone();
            event.progress_detail = detail.clone();
            Some(event)
        }
        node_engine::WorkflowEvent::TaskStream {
            task_id,
            execution_id,
            port,
            data,
            occurred_at_ms,
        } if matches_context(context, execution_id, task_id) => {
            let mut event = base_event(
                context,
                NodeExecutionDiagnosticEventKind::Stream,
                *occurred_at_ms,
            );
            if let Ok(port_id) = PortId::try_from(port.clone()) {
                event.port_id = Some(port_id.clone());
                event
                    .output_summaries
                    .push(summary_for_port(context, port_id, data));
            }
            Some(event)
        }
        node_engine::WorkflowEvent::WorkflowCancelled {
            execution_id,
            error,
            occurred_at_ms,
            ..
        } if execution_id == context.workflow_run_id().as_str() => {
            let mut event = base_event(
                context,
                NodeExecutionDiagnosticEventKind::Cancelled,
                *occurred_at_ms,
            );
            event.error = Some(error.clone());
            Some(event)
        }
        node_engine::WorkflowEvent::GraphModified {
            execution_id,
            dirty_tasks,
            occurred_at_ms,
            ..
        } if execution_id == context.workflow_run_id().as_str()
            && dirty_tasks
                .iter()
                .any(|task_id| task_id == context.node_id().as_str()) =>
        {
            Some(base_event(
                context,
                NodeExecutionDiagnosticEventKind::GraphModified,
                *occurred_at_ms,
            ))
        }
        node_engine::WorkflowEvent::IncrementalExecutionStarted {
            execution_id,
            tasks,
            occurred_at_ms,
            ..
        } if execution_id == context.workflow_run_id().as_str()
            && tasks
                .iter()
                .any(|task_id| task_id == context.node_id().as_str()) =>
        {
            Some(base_event(
                context,
                NodeExecutionDiagnosticEventKind::IncrementalExecutionStarted,
                *occurred_at_ms,
            ))
        }
        _ => None,
    }
}

fn matches_context(context: &NodeExecutionContext, execution_id: &str, task_id: &str) -> bool {
    execution_id == context.workflow_run_id().as_str() && task_id == context.node_id().as_str()
}

fn base_event(
    context: &NodeExecutionContext,
    kind: NodeExecutionDiagnosticEventKind,
    occurred_at_ms: Option<u64>,
) -> NodeExecutionDiagnosticEvent {
    let attribution = context.attribution();
    NodeExecutionDiagnosticEvent {
        kind,
        client_id: attribution.client_id.clone(),
        client_session_id: attribution.client_session_id.clone(),
        bucket_id: attribution.bucket_id.clone(),
        workflow_id: context.workflow_id().clone(),
        workflow_run_id: attribution.workflow_run_id.clone(),
        node_id: context.node_id().clone(),
        node_type: context.node_type().clone(),
        attempt: context.attempt(),
        occurred_at_ms: occurred_at_ms.unwrap_or_else(crate::workflow_runtime::unix_timestamp_ms),
        guarantee: context.guarantee(),
        lineage: context.lineage().clone(),
        contract_version: context
            .effective_contract()
            .static_contract
            .contract_version
            .clone(),
        contract_digest: context
            .effective_contract()
            .static_contract
            .contract_digest
            .clone(),
        port_id: None,
        progress: None,
        message: None,
        error: None,
        output_summaries: Vec::new(),
        progress_detail: None,
    }
}

fn output_summaries(
    context: &NodeExecutionContext,
    output: Option<&serde_json::Value>,
) -> Vec<NodeOutputSummary> {
    let Some(serde_json::Value::Object(output)) = output else {
        return context
            .effective_contract()
            .outputs
            .iter()
            .map(|port| {
                NodeOutputSummary::unavailable(port.base.id.clone(), Some(port.base.value_type))
            })
            .collect();
    };

    context
        .effective_contract()
        .outputs
        .iter()
        .map(|port| {
            output
                .get(port.base.id.as_str())
                .map(|value| summary_for_port(context, port.base.id.clone(), value))
                .unwrap_or_else(|| {
                    NodeOutputSummary::unavailable(port.base.id.clone(), Some(port.base.value_type))
                })
        })
        .collect()
}

fn summary_for_port(
    context: &NodeExecutionContext,
    port_id: PortId,
    value: &serde_json::Value,
) -> NodeOutputSummary {
    let mut summary = NodeOutputSummary::from_value(port_id.clone(), value);
    summary.value_type = context
        .effective_contract()
        .outputs
        .iter()
        .find(|port| port.base.id == port_id)
        .map(|port| port.base.value_type)
        .or(Some(PortValueType::Any));
    summary
}

fn lock_error<T>(_error: std::sync::PoisonError<T>) -> node_engine::EventError {
    node_engine::EventError {
        message: "node execution diagnostics recorder lock poisoned".to_string(),
    }
}

#[cfg(test)]
#[path = "node_execution_diagnostics_tests.rs"]
mod tests;
