use std::collections::BTreeMap;

use pantograph_inference_interface_contracts::{
    AuthoredInferenceInterfaceSnapshot, DeviceIntentId, DraftGraphEnqueueDisabledReason,
    DraftGraphValidationSessionId, DraftGraphValidationStatus, DraftGraphValidationSummary,
    InferenceConnectionSurface, InferenceConnectionSurfaceStatus, InferenceDiagnosticCode,
    InferenceDiagnosticSeverity, InferenceInterfaceContractError, InferenceInterfaceDescriptor,
    InferenceInterfaceDiagnostic, InferenceInterfaceFingerprint, RuntimeIntentId,
    WorkflowGraphRevision, WorkflowNodeId, INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::inference_interface_facts::missing_model_facts;
use super::inference_interface_projection::{
    resolve_inference_interface_projection, InferenceInterfaceProjectionError,
};
use super::inference_interface_request::{
    InferenceInterfaceGraphResolutionDiagnostic, InferenceInterfaceGraphResolutionInputs,
};
use super::inference_interface_resolver::InferenceInterfaceResolverFacts;
use super::inference_interface_validation::{
    InferenceInterfaceValidationSessionError, WorkflowGraphInferenceValidationEvent,
    WorkflowGraphInferenceValidationEventPayload, WorkflowGraphInferenceValidationEventScope,
    WorkflowGraphInferenceValidationSession,
};

pub(crate) const MAX_VALIDATION_NODE_PROJECTIONS: usize = 512;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum InferenceInterfacePublicationError {
    #[error("inference interface projection failed for node {node_id}: {source}")]
    Projection {
        node_id: String,
        source: InferenceInterfaceProjectionError,
    },
    #[error("inference interface contract error: {0}")]
    Contract(#[from] InferenceInterfaceContractError),
    #[error("inference validation session error: {0}")]
    ValidationSession(#[from] InferenceInterfaceValidationSessionError),
    #[error("validation node projection count {count} exceeds maximum {max}")]
    TooManyNodeProjections { count: usize, max: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowGraphInferenceValidationPublication {
    pub validation_session: WorkflowGraphInferenceValidationSession,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_projections: Vec<InferenceInterfaceNodeProjectionRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub request_diagnostics: Vec<InferenceInterfaceGraphResolutionDiagnostic>,
}

impl WorkflowGraphInferenceValidationPublication {
    pub fn connection_surfaces(
        &self,
    ) -> Result<Vec<InferenceConnectionSurface>, InferenceInterfacePublicationError> {
        self.node_projections
            .iter()
            .map(|projection| {
                connection_surface_for_projection(
                    &self.validation_session.validation_session_id,
                    &self.validation_session.graph_revision,
                    projection,
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceNodeProjectionRecord {
    pub node_id: WorkflowNodeId,
    pub descriptor: InferenceInterfaceDescriptor,
    pub authored_snapshot: AuthoredInferenceInterfaceSnapshot,
    pub validation_summary: DraftGraphValidationSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_constraint: Option<RuntimeIntentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_constraint: Option<DeviceIntentId>,
}

impl InferenceInterfaceNodeProjectionRecord {
    pub fn descriptor_fingerprint(&self) -> &InferenceInterfaceFingerprint {
        &self.descriptor.descriptor_fingerprint
    }
}

fn connection_surface_for_projection(
    validation_session_id: &DraftGraphValidationSessionId,
    graph_revision: &WorkflowGraphRevision,
    projection: &InferenceInterfaceNodeProjectionRecord,
) -> Result<InferenceConnectionSurface, InferenceInterfacePublicationError> {
    let status = connection_surface_status(&projection.validation_summary);
    let surface = InferenceConnectionSurface {
        contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
        graph_revision: graph_revision.clone(),
        validation_session_id: Some(validation_session_id.clone()),
        node_id: projection.node_id.clone(),
        descriptor_fingerprint: Some(projection.descriptor.descriptor_fingerprint.clone()),
        status,
        inputs: projection.descriptor.inputs.clone(),
        outputs: projection.descriptor.outputs.clone(),
        validation_summary: projection.validation_summary.clone(),
        drift_report: None,
        diagnostics: connection_surface_diagnostics(status, &projection.descriptor),
    };
    surface.validate()?;
    Ok(surface)
}

fn connection_surface_status(
    summary: &DraftGraphValidationSummary,
) -> InferenceConnectionSurfaceStatus {
    match summary.status {
        DraftGraphValidationStatus::Executable => InferenceConnectionSurfaceStatus::Current,
        DraftGraphValidationStatus::Pending => InferenceConnectionSurfaceStatus::Pending,
        DraftGraphValidationStatus::Stale => InferenceConnectionSurfaceStatus::Stale,
        DraftGraphValidationStatus::Unresolved => InferenceConnectionSurfaceStatus::Missing,
        DraftGraphValidationStatus::Unavailable => InferenceConnectionSurfaceStatus::Unavailable,
        DraftGraphValidationStatus::Blocked => InferenceConnectionSurfaceStatus::Blocked,
        _ => InferenceConnectionSurfaceStatus::Blocked,
    }
}

fn connection_surface_diagnostics(
    status: InferenceConnectionSurfaceStatus,
    descriptor: &InferenceInterfaceDescriptor,
) -> Vec<InferenceInterfaceDiagnostic> {
    let mut diagnostics = descriptor.diagnostics.clone();
    if status == InferenceConnectionSurfaceStatus::Current || !diagnostics.is_empty() {
        return diagnostics;
    }
    diagnostics.push(InferenceInterfaceDiagnostic {
        severity: InferenceDiagnosticSeverity::Error,
        code: connection_surface_diagnostic_code(status),
        message: connection_surface_diagnostic_message(status).to_string(),
        hint: None,
        port_id: None,
    });
    diagnostics
}

fn connection_surface_diagnostic_code(
    status: InferenceConnectionSurfaceStatus,
) -> InferenceDiagnosticCode {
    match status {
        InferenceConnectionSurfaceStatus::Pending => {
            InferenceDiagnosticCode::GraphValidationPending
        }
        InferenceConnectionSurfaceStatus::Missing => {
            InferenceDiagnosticCode::InferenceConnectionSurfaceMissing
        }
        InferenceConnectionSurfaceStatus::Stale => {
            InferenceDiagnosticCode::InferenceConnectionSurfaceStale
        }
        InferenceConnectionSurfaceStatus::Unavailable => {
            InferenceDiagnosticCode::DescriptorUnavailable
        }
        InferenceConnectionSurfaceStatus::Blocked
        | InferenceConnectionSurfaceStatus::DriftBlocked
        | InferenceConnectionSurfaceStatus::Current => {
            InferenceDiagnosticCode::InferenceConnectionSurfaceBlocked
        }
        _ => InferenceDiagnosticCode::InferenceConnectionSurfaceBlocked,
    }
}

fn connection_surface_diagnostic_message(status: InferenceConnectionSurfaceStatus) -> &'static str {
    match status {
        InferenceConnectionSurfaceStatus::Pending => {
            "Inference connection surface is waiting for validation."
        }
        InferenceConnectionSurfaceStatus::Missing => {
            "Inference connection surface has not been resolved for this graph revision."
        }
        InferenceConnectionSurfaceStatus::Stale => {
            "Inference connection surface is stale for this graph revision."
        }
        InferenceConnectionSurfaceStatus::Unavailable => {
            "Inference connection surface is unavailable for this descriptor."
        }
        InferenceConnectionSurfaceStatus::Blocked
        | InferenceConnectionSurfaceStatus::DriftBlocked
        | InferenceConnectionSurfaceStatus::Current => {
            "Inference connection surface is blocked by validation diagnostics."
        }
        _ => "Inference connection surface is blocked by validation diagnostics.",
    }
}

#[cfg(test)]
pub(crate) fn publish_inference_validation_for_graph(
    validation_session_id: DraftGraphValidationSessionId,
    graph_revision: WorkflowGraphRevision,
    graph: &super::types::WorkflowGraph,
    facts_by_node_id: BTreeMap<String, InferenceInterfaceResolverFacts>,
) -> Result<WorkflowGraphInferenceValidationPublication, InferenceInterfacePublicationError> {
    let resolution_inputs =
        super::inference_interface_request::inference_interface_resolution_inputs_from_graph(graph);
    publish_inference_validation_for_resolution_inputs(
        validation_session_id,
        graph_revision,
        resolution_inputs,
        facts_by_node_id,
    )
}

pub(crate) fn publish_inference_validation_for_resolution_inputs(
    validation_session_id: DraftGraphValidationSessionId,
    graph_revision: WorkflowGraphRevision,
    resolution_inputs: InferenceInterfaceGraphResolutionInputs,
    facts_by_node_id: BTreeMap<String, InferenceInterfaceResolverFacts>,
) -> Result<WorkflowGraphInferenceValidationPublication, InferenceInterfacePublicationError> {
    if resolution_inputs.requests.len() > MAX_VALIDATION_NODE_PROJECTIONS {
        return Err(InferenceInterfacePublicationError::TooManyNodeProjections {
            count: resolution_inputs.requests.len(),
            max: MAX_VALIDATION_NODE_PROJECTIONS,
        });
    }

    let mut node_projections = Vec::new();

    for input in &resolution_inputs.requests {
        let facts = facts_by_node_id
            .get(&input.node_id)
            .cloned()
            .unwrap_or_else(missing_model_facts);
        let projection = resolve_inference_interface_projection(input.request.clone(), facts)
            .map_err(|source| InferenceInterfacePublicationError::Projection {
                node_id: input.node_id.clone(),
                source,
            })?;
        node_projections.push(InferenceInterfaceNodeProjectionRecord {
            node_id: WorkflowNodeId::parse(&input.node_id)?,
            descriptor: projection.descriptor,
            authored_snapshot: projection.authored_snapshot,
            validation_summary: projection.validation_summary,
            runtime_constraint: input.request.runtime_constraint.clone(),
            device_constraint: input.request.device_constraint.clone(),
        });
    }

    let summary = aggregate_summary(&node_projections, &resolution_inputs.diagnostics)?;
    let events = validation_events(
        &validation_session_id,
        &graph_revision,
        &node_projections,
        &summary,
    );
    let latest_sequence = events.last().map(|event| event.sequence).unwrap_or(0);
    let validation_session = WorkflowGraphInferenceValidationSession {
        contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
        validation_session_id,
        graph_revision,
        latest_sequence,
        summary,
        events,
    };
    validation_session.validate()?;

    Ok(WorkflowGraphInferenceValidationPublication {
        validation_session,
        node_projections,
        request_diagnostics: resolution_inputs.diagnostics,
    })
}

fn aggregate_summary(
    node_projections: &[InferenceInterfaceNodeProjectionRecord],
    request_diagnostics: &[InferenceInterfaceGraphResolutionDiagnostic],
) -> Result<DraftGraphValidationSummary, InferenceInterfaceContractError> {
    let diagnostics_count = checked_u32(
        "validation_summary.diagnostics_count",
        node_projections
            .iter()
            .map(|projection| projection.validation_summary.diagnostics_count as usize)
            .sum::<usize>()
            + request_diagnostics.len(),
    )?;
    let blocking_diagnostics_count = checked_u32(
        "validation_summary.blocking_diagnostics_count",
        node_projections
            .iter()
            .map(|projection| projection.validation_summary.blocking_diagnostics_count as usize)
            .sum::<usize>()
            + request_diagnostics.len(),
    )?;
    let mut reasons = Vec::new();
    if !request_diagnostics.is_empty() {
        push_unique(
            &mut reasons,
            DraftGraphEnqueueDisabledReason::BlockingDiagnostics,
        );
    }
    for projection in node_projections {
        for reason in &projection.validation_summary.enqueue_disabled_reasons {
            push_unique(&mut reasons, *reason);
        }
    }
    let status = aggregate_status(node_projections, request_diagnostics);
    let executable = status == DraftGraphValidationStatus::Executable && reasons.is_empty();
    let summary = DraftGraphValidationSummary {
        status,
        executable,
        enqueue_disabled_reasons: reasons,
        diagnostics_count,
        blocking_diagnostics_count,
    };
    summary.validate()?;
    Ok(summary)
}

fn aggregate_status(
    node_projections: &[InferenceInterfaceNodeProjectionRecord],
    request_diagnostics: &[InferenceInterfaceGraphResolutionDiagnostic],
) -> DraftGraphValidationStatus {
    if !request_diagnostics.is_empty() {
        return DraftGraphValidationStatus::Blocked;
    }
    let mut status = DraftGraphValidationStatus::Executable;
    for projection in node_projections {
        status = more_restrictive_status(status, projection.validation_summary.status);
    }
    status
}

fn more_restrictive_status(
    current: DraftGraphValidationStatus,
    next: DraftGraphValidationStatus,
) -> DraftGraphValidationStatus {
    if status_rank(next) > status_rank(current) {
        next
    } else {
        current
    }
}

fn status_rank(status: DraftGraphValidationStatus) -> u8 {
    match status {
        DraftGraphValidationStatus::Executable => 0,
        DraftGraphValidationStatus::Pending => 1,
        DraftGraphValidationStatus::Stale => 2,
        DraftGraphValidationStatus::Unresolved => 3,
        DraftGraphValidationStatus::Unavailable => 4,
        DraftGraphValidationStatus::Blocked => 5,
        _ => 5,
    }
}

fn validation_events(
    validation_session_id: &DraftGraphValidationSessionId,
    graph_revision: &WorkflowGraphRevision,
    node_projections: &[InferenceInterfaceNodeProjectionRecord],
    summary: &DraftGraphValidationSummary,
) -> Vec<WorkflowGraphInferenceValidationEvent> {
    let mut events = Vec::with_capacity(node_projections.len() + 1);
    let mut sequence = 1;
    for projection in node_projections {
        events.push(WorkflowGraphInferenceValidationEvent {
            validation_session_id: validation_session_id.clone(),
            graph_revision: graph_revision.clone(),
            sequence,
            scope: WorkflowGraphInferenceValidationEventScope::Node {
                node_id: projection.node_id.clone(),
            },
            payload: WorkflowGraphInferenceValidationEventPayload::DescriptorResolved(
                projection.descriptor.descriptor_fingerprint.clone(),
            ),
        });
        sequence += 1;
    }
    events.push(WorkflowGraphInferenceValidationEvent {
        validation_session_id: validation_session_id.clone(),
        graph_revision: graph_revision.clone(),
        sequence,
        scope: WorkflowGraphInferenceValidationEventScope::Graph,
        payload: WorkflowGraphInferenceValidationEventPayload::Summary(summary.clone()),
    });
    events
}

fn push_unique(
    reasons: &mut Vec<DraftGraphEnqueueDisabledReason>,
    reason: DraftGraphEnqueueDisabledReason,
) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn checked_u32(field: &'static str, count: usize) -> Result<u32, InferenceInterfaceContractError> {
    u32::try_from(count).map_err(|_| InferenceInterfaceContractError::TooManyItems {
        field,
        actual_len: count,
        max_len: u32::MAX as usize,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_inference_interface_contracts::{
        InferenceAvailability, InferencePortDescriptor, InferencePortDirection, InferencePortId,
        InferencePortOptions, InferencePortRequirement, InferenceScalarType, InferenceTaskKind,
        InferenceValueType, RuntimeIntentId,
    };
    use serde_json::json;

    use crate::graph::{
        GraphEdge, GraphNode, InferenceCapabilityFacts, InferenceModelResolutionFacts,
        InferenceModelResolutionState, InferenceRuntimeAvailabilityFact,
        InferenceRuntimeAvailabilityState, Position, WorkflowGraph,
    };

    #[test]
    fn publication_projects_ready_inference_node_and_summary_event() {
        let graph = graph_with_connected_model();
        let publication = publish_inference_validation_for_graph(
            validation_session_id(),
            graph_revision(),
            &graph,
            BTreeMap::from([("infer".to_string(), ready_facts())]),
        )
        .expect("publication");

        assert!(publication.request_diagnostics.is_empty());
        assert_eq!(publication.node_projections.len(), 1);
        assert_eq!(publication.node_projections[0].node_id.as_str(), "infer");
        assert_eq!(
            publication.validation_session.summary.status,
            DraftGraphValidationStatus::Executable
        );
        assert!(publication.validation_session.summary.executable);
        assert_eq!(publication.validation_session.events.len(), 2);
    }

    #[test]
    fn publication_builds_current_connection_surface_from_projection() {
        let graph = graph_with_connected_model();
        let publication = publish_inference_validation_for_graph(
            validation_session_id(),
            graph_revision(),
            &graph,
            BTreeMap::from([("infer".to_string(), ready_facts())]),
        )
        .expect("publication");

        let surfaces = publication
            .connection_surfaces()
            .expect("connection surfaces should validate");

        assert_eq!(surfaces.len(), 1);
        assert_eq!(surfaces[0].node_id.as_str(), "infer");
        assert_eq!(
            surfaces[0].status,
            InferenceConnectionSurfaceStatus::Current
        );
        assert_eq!(
            surfaces[0]
                .descriptor_fingerprint
                .as_ref()
                .expect("current surface has descriptor fingerprint"),
            publication.node_projections[0].descriptor_fingerprint()
        );
        assert_eq!(surfaces[0].inputs.len(), 1);
        assert!(surfaces[0].validation_summary.executable);
    }

    #[test]
    fn publication_blocks_request_extraction_diagnostics() {
        let mut graph = graph_with_connected_model();
        graph.edges.push(GraphEdge {
            id: "duplicate".to_string(),
            source: "model".to_string(),
            source_handle: "other_handle".to_string(),
            target: "infer".to_string(),
            target_handle: "pumas_model_ref".to_string(),
        });

        let publication = publish_inference_validation_for_graph(
            validation_session_id(),
            graph_revision(),
            &graph,
            BTreeMap::from([("infer".to_string(), ready_facts())]),
        )
        .expect("publication");

        assert!(publication.node_projections.is_empty());
        assert_eq!(
            publication.validation_session.summary.status,
            DraftGraphValidationStatus::Blocked
        );
        assert!(!publication.validation_session.summary.executable);
        assert_eq!(publication.request_diagnostics.len(), 1);
    }

    #[test]
    fn publication_uses_unavailable_projection_when_facts_are_missing() {
        let graph = graph_with_connected_model();
        let publication = publish_inference_validation_for_graph(
            validation_session_id(),
            graph_revision(),
            &graph,
            BTreeMap::new(),
        )
        .expect("publication");

        assert_eq!(
            publication.validation_session.summary.status,
            DraftGraphValidationStatus::Blocked
        );
        assert!(!publication.validation_session.summary.executable);
        assert_eq!(publication.node_projections.len(), 1);
        assert_eq!(
            publication.node_projections[0].validation_summary.status,
            DraftGraphValidationStatus::Blocked
        );
    }

    #[test]
    fn publication_builds_fail_closed_connection_surface_for_missing_facts() {
        let graph = graph_with_connected_model();
        let publication = publish_inference_validation_for_graph(
            validation_session_id(),
            graph_revision(),
            &graph,
            BTreeMap::new(),
        )
        .expect("publication");

        let surfaces = publication
            .connection_surfaces()
            .expect("blocked connection surface should validate");

        assert_eq!(surfaces.len(), 1);
        assert_eq!(
            surfaces[0].status,
            InferenceConnectionSurfaceStatus::Blocked
        );
        assert!(!surfaces[0].validation_summary.executable);
        assert!(
            !surfaces[0].diagnostics.is_empty(),
            "non-current surfaces must explain why they are blocked"
        );
        assert!(surfaces[0]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == InferenceDiagnosticCode::DescriptorUnavailable));
    }

    #[test]
    fn publication_rejects_unbounded_node_projection_records() {
        let graph = graph_with_inference_node_count(MAX_VALIDATION_NODE_PROJECTIONS + 1);
        let error = publish_inference_validation_for_graph(
            validation_session_id(),
            graph_revision(),
            &graph,
            BTreeMap::new(),
        )
        .expect_err("projection count should be bounded");

        assert!(matches!(
            error,
            InferenceInterfacePublicationError::TooManyNodeProjections {
                count,
                max: MAX_VALIDATION_NODE_PROJECTIONS,
            } if count == MAX_VALIDATION_NODE_PROJECTIONS + 1
        ));
    }

    fn graph_with_connected_model() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "model".to_string(),
                    node_type: "puma-lib".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: json!({
                        "pumas_model_ref": {
                            "model_id": "image/example/tiny",
                            "selected_artifact_id": "diffusers"
                        }
                    }),
                },
                GraphNode {
                    id: "infer".to_string(),
                    node_type: "llm-inference".to_string(),
                    position: Position { x: 200.0, y: 0.0 },
                    data: json!({
                        "task_kind": "image_generation",
                        "runtime": "pytorch"
                    }),
                },
            ],
            edges: vec![GraphEdge {
                id: "model-to-infer".to_string(),
                source: "model".to_string(),
                source_handle: "pumas_model_ref".to_string(),
                target: "infer".to_string(),
                target_handle: "pumas_model_ref".to_string(),
            }],
            derived_graph: None,
        }
    }

    fn graph_with_inference_node_count(count: usize) -> WorkflowGraph {
        let mut graph = WorkflowGraph {
            nodes: vec![GraphNode {
                id: "model".to_string(),
                node_type: "puma-lib".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: json!({
                    "pumas_model_ref": {
                        "model_id": "image/example/tiny",
                        "selected_artifact_id": "diffusers"
                    }
                }),
            }],
            edges: Vec::new(),
            derived_graph: None,
        };
        for index in 0..count {
            let node_id = format!("infer-{index}");
            graph.nodes.push(GraphNode {
                id: node_id.clone(),
                node_type: "llm-inference".to_string(),
                position: Position {
                    x: 200.0,
                    y: index as f64,
                },
                data: json!({
                    "task_kind": "image_generation",
                    "runtime": "pytorch"
                }),
            });
            graph.edges.push(GraphEdge {
                id: format!("model-to-{node_id}"),
                source: "model".to_string(),
                source_handle: "pumas_model_ref".to_string(),
                target: node_id,
                target_handle: "pumas_model_ref".to_string(),
            });
        }
        graph
    }

    fn ready_facts() -> InferenceInterfaceResolverFacts {
        let runtime = runtime_id("pytorch");
        InferenceInterfaceResolverFacts {
            model: InferenceModelResolutionFacts {
                state: InferenceModelResolutionState::Ready,
            },
            capability: Some(InferenceCapabilityFacts {
                task_kind: task_kind("image_generation"),
                inputs: vec![prompt_port()],
                outputs: Vec::new(),
                runtime_conditions: Vec::new(),
                supported_runtime_ids: vec![runtime.clone()],
            }),
            runtimes: vec![InferenceRuntimeAvailabilityFact {
                runtime_id: runtime,
                state: InferenceRuntimeAvailabilityState::Available,
                device_ids: Vec::new(),
            }],
        }
    }

    fn prompt_port() -> InferencePortDescriptor {
        InferencePortDescriptor {
            port_id: InferencePortId::parse("prompt").expect("valid port"),
            label: "Prompt".to_string(),
            direction: InferencePortDirection::Input,
            requirement: InferencePortRequirement::Required,
            value_type: InferenceValueType::Scalar(InferenceScalarType::String),
            default: None,
            options: InferencePortOptions::None,
            availability: InferenceAvailability::available(),
            runtime_conditions: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn task_kind(value: &str) -> InferenceTaskKind {
        InferenceTaskKind::parse(value).expect("valid task kind")
    }

    fn runtime_id(value: &str) -> RuntimeIntentId {
        RuntimeIntentId::parse(value).expect("valid runtime id")
    }

    fn validation_session_id() -> DraftGraphValidationSessionId {
        DraftGraphValidationSessionId::parse("validation.session.1")
            .expect("valid validation session")
    }

    fn graph_revision() -> WorkflowGraphRevision {
        WorkflowGraphRevision::parse("aaaaaaaaaaaaaaaa").expect("valid graph revision")
    }
}
