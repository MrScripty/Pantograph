use std::collections::{BTreeMap, BTreeSet};

use pantograph_inference_interface_contracts::WorkflowGraphRevision;
use pantograph_runtime_attribution::{
    AttributionError, BucketCreateRequest, BucketDeleteRequest, BucketRecord,
    ClientRegistrationRequest, ClientRegistrationResponse, ClientSessionOpenRequest,
    ClientSessionOpenResponse, ClientSessionRecord, ClientSessionResumeRequest,
    WorkflowExecutableValidationSnapshotLookupRequest as AttributionWorkflowExecutableValidationSnapshotLookupRequest,
    WorkflowId, WorkflowPresentationRevisionRecord, WorkflowPresentationRevisionResolveRequest,
    WorkflowRunId, WorkflowRunSnapshotRecord, WorkflowRunVersionProjection, WorkflowVersionId,
    WorkflowVersionRecord, WorkflowVersionResolveRequest,
};

use crate::graph::{
    validate_workflow_graph_contract_diagnostics, workflow_executable_topology,
    workflow_execution_fingerprint_for_topology, workflow_presentation_fingerprint_for_metadata,
    workflow_presentation_metadata, workflow_presentation_metadata_json, GraphEdge, GraphNode,
    NodeRegistry, WorkflowExecutableTopology, WorkflowGraph, WorkflowGraphRunSettings,
    WorkflowPresentationMetadata,
};

use super::{
    validate_workflow_id, AttributionRepository,
    ValidatedWorkflowExecutableValidationSnapshotRecord, WorkflowExecutableValidationSnapshotError,
    WorkflowExecutableValidationSnapshotId, WorkflowExecutableValidationSnapshotLookupRequest,
    WorkflowExecutableValidationSnapshotPublishRequest, WorkflowExecutableValidationSnapshotRecord,
    WorkflowGraphSessionExecutableValidationSnapshotPublishRequest, WorkflowRunGraphProjection,
    WorkflowRunGraphQueryRequest, WorkflowRunGraphQueryResponse, WorkflowService,
    WorkflowServiceError,
};

impl WorkflowService {
    pub fn register_attribution_client(
        &self,
        request: ClientRegistrationRequest,
    ) -> Result<ClientRegistrationResponse, WorkflowServiceError> {
        let mut store = self.attribution_store_guard()?;
        store
            .register_client(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn open_client_session(
        &self,
        request: ClientSessionOpenRequest,
    ) -> Result<ClientSessionOpenResponse, WorkflowServiceError> {
        let mut store = self.attribution_store_guard()?;
        store
            .open_session(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn resume_client_session(
        &self,
        request: ClientSessionResumeRequest,
    ) -> Result<ClientSessionRecord, WorkflowServiceError> {
        let mut store = self.attribution_store_guard()?;
        store
            .resume_session(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn create_client_bucket(
        &self,
        request: BucketCreateRequest,
    ) -> Result<BucketRecord, WorkflowServiceError> {
        let mut store = self.attribution_store_guard()?;
        store
            .create_bucket(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn delete_client_bucket(
        &self,
        request: BucketDeleteRequest,
    ) -> Result<BucketRecord, WorkflowServiceError> {
        let mut store = self.attribution_store_guard()?;
        store
            .delete_bucket(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn resolve_workflow_graph_version(
        &self,
        workflow_id: &str,
        semantic_version: &str,
        graph: &WorkflowGraph,
    ) -> Result<WorkflowVersionRecord, WorkflowServiceError> {
        validate_workflow_id(workflow_id)?;
        let topology = workflow_executable_topology(graph)?;
        let execution_fingerprint = workflow_execution_fingerprint_for_topology(&topology)?;
        let executable_topology_json = serde_json::to_string(&topology).map_err(|error| {
            WorkflowServiceError::CapabilityViolation(format!(
                "failed to encode workflow executable topology: {error}"
            ))
        })?;
        let request = WorkflowVersionResolveRequest {
            workflow_id: WorkflowId::try_from(workflow_id.to_string())?,
            semantic_version: semantic_version.to_string(),
            execution_fingerprint,
            executable_topology_json,
        };
        let mut store = self.attribution_store_guard()?;
        store
            .resolve_workflow_version(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn resolve_workflow_graph_presentation_revision(
        &self,
        workflow_id: &str,
        workflow_version_id: &str,
        graph: &WorkflowGraph,
    ) -> Result<WorkflowPresentationRevisionRecord, WorkflowServiceError> {
        validate_workflow_id(workflow_id)?;
        let metadata = workflow_presentation_metadata(graph);
        let presentation_fingerprint = workflow_presentation_fingerprint_for_metadata(&metadata)?;
        let presentation_metadata_json = workflow_presentation_metadata_json(&metadata)?;
        let request = WorkflowPresentationRevisionResolveRequest {
            workflow_id: WorkflowId::try_from(workflow_id.to_string())?,
            workflow_version_id: WorkflowVersionId::try_from(workflow_version_id.to_string())?,
            presentation_fingerprint,
            presentation_metadata_json,
        };
        let mut store = self.attribution_store_guard()?;
        store
            .resolve_workflow_presentation_revision(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn workflow_run_snapshot(
        &self,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowRunSnapshotRecord>, WorkflowServiceError> {
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let store = self.attribution_store_guard()?;
        store
            .workflow_run_snapshot(&workflow_run_id)
            .map_err(WorkflowServiceError::from)
    }

    pub fn workflow_run_version_projection(
        &self,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowRunVersionProjection>, WorkflowServiceError> {
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let store = self.attribution_store_guard()?;
        store
            .workflow_run_version_projection(&workflow_run_id)
            .map_err(WorkflowServiceError::from)
    }

    pub fn store_workflow_executable_validation_snapshot(
        &self,
        snapshot: WorkflowExecutableValidationSnapshotRecord,
    ) -> Result<ValidatedWorkflowExecutableValidationSnapshotRecord, WorkflowServiceError> {
        let snapshot = ValidatedWorkflowExecutableValidationSnapshotRecord::try_from(snapshot)
            .map_err(workflow_executable_validation_snapshot_service_error)?;
        let request = snapshot
            .to_attribution_store_request()
            .map_err(workflow_executable_validation_snapshot_service_error)?;
        let mut store = self.attribution_store_guard()?;
        let stored = store
            .store_workflow_executable_validation_snapshot(request)
            .map_err(WorkflowServiceError::from)?;
        let lookup_request = WorkflowExecutableValidationSnapshotLookupRequest {
            workflow_version_id: stored.workflow_version_id.clone(),
            workflow_execution_fingerprint: stored.workflow_execution_fingerprint.clone(),
            descriptor_contract_version: stored.descriptor_contract_version,
        };
        ValidatedWorkflowExecutableValidationSnapshotRecord::from_attribution_record(
            stored,
            &lookup_request,
        )
        .map_err(workflow_executable_validation_snapshot_service_error)
    }

    pub fn workflow_executable_validation_snapshot(
        &self,
        request: WorkflowExecutableValidationSnapshotLookupRequest,
    ) -> Result<ValidatedWorkflowExecutableValidationSnapshotRecord, WorkflowServiceError> {
        request
            .validate()
            .map_err(workflow_executable_validation_snapshot_service_error)?;
        let store = self.attribution_store_guard()?;
        let stored = match store.workflow_executable_validation_snapshot(
            AttributionWorkflowExecutableValidationSnapshotLookupRequest {
                workflow_version_id: request.workflow_version_id.clone(),
            },
        ) {
            Ok(stored) => stored,
            Err(AttributionError::NotFound {
                entity: "workflow_executable_validation_snapshot",
            }) => {
                return Err(WorkflowServiceError::InvalidRequest(
                    "saved executable validation snapshot was not found for workflow version"
                        .to_string(),
                ));
            }
            Err(error) => return Err(WorkflowServiceError::from(error)),
        };
        ValidatedWorkflowExecutableValidationSnapshotRecord::from_attribution_record(
            stored, &request,
        )
        .map_err(workflow_executable_validation_snapshot_service_error)
    }

    pub fn publish_workflow_executable_validation_snapshot(
        &self,
        request: WorkflowExecutableValidationSnapshotPublishRequest,
    ) -> Result<ValidatedWorkflowExecutableValidationSnapshotRecord, WorkflowServiceError> {
        if !request.validation_publication.node_projections.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "runtime executable validation snapshots must be published from the current graph-session validation state"
                    .to_string(),
            ));
        }
        validate_workflow_id(&request.workflow_id)?;
        crate::graph::lower_groups(&request.graph, &crate::graph::NodeRegistry::new())?;
        let graph_revision = WorkflowGraphRevision::parse(request.graph.compute_fingerprint())
            .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
        if graph_revision
            != request
                .validation_publication
                .validation_session
                .graph_revision
        {
            return Err(WorkflowServiceError::InvalidRequest(
                "validation publication graph revision does not match executable publish graph"
                    .to_string(),
            ));
        }
        let workflow_version = self.resolve_workflow_graph_version(
            &request.workflow_id,
            &request.workflow_semantic_version,
            &request.graph,
        )?;
        let snapshot_id = request
            .validation_snapshot_id
            .unwrap_or_else(WorkflowExecutableValidationSnapshotId::generate);
        let snapshot = WorkflowExecutableValidationSnapshotRecord::from_validation_publication(
            &workflow_version,
            snapshot_id,
            &request.validation_publication,
        )
        .map_err(workflow_executable_validation_snapshot_service_error)?;
        self.store_workflow_executable_validation_snapshot(snapshot)
    }

    pub async fn publish_graph_session_executable_validation_snapshot(
        &self,
        request: WorkflowGraphSessionExecutableValidationSnapshotPublishRequest,
    ) -> Result<ValidatedWorkflowExecutableValidationSnapshotRecord, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        self.graph_session_store
            .with_executable_validation_snapshot_source_for_session(
                &request.graph_session_id,
                request.validation_session_id.clone(),
                |graph, source| {
                    let graph_revision = WorkflowGraphRevision::parse(graph.compute_fingerprint())
                        .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
                    if graph_revision != source.graph_revision {
                        return Err(WorkflowServiceError::InvalidRequest(
                            "current graph revision does not match executable snapshot source"
                                .to_string(),
                        ));
                    }
                    let workflow_version = self.resolve_workflow_graph_version(
                        &request.workflow_id,
                        &request.workflow_semantic_version,
                        &graph,
                    )?;
                    let snapshot_id = request
                        .validation_snapshot_id
                        .unwrap_or_else(WorkflowExecutableValidationSnapshotId::generate);
                    let snapshot =
                        WorkflowExecutableValidationSnapshotRecord::from_snapshot_source(
                            &workflow_version,
                            snapshot_id,
                            &source,
                            &graph,
                        )
                        .map_err(workflow_executable_validation_snapshot_service_error)?;
                    self.reject_changed_dependency_proof_freshness(&snapshot)?;
                    self.store_workflow_executable_validation_snapshot(snapshot)
                },
            )
            .await
    }

    fn reject_changed_dependency_proof_freshness(
        &self,
        snapshot: &WorkflowExecutableValidationSnapshotRecord,
    ) -> Result<(), WorkflowServiceError> {
        let request = WorkflowExecutableValidationSnapshotLookupRequest {
            workflow_version_id: snapshot.workflow_version_id.clone(),
            workflow_execution_fingerprint: snapshot.workflow_execution_fingerprint.clone(),
            descriptor_contract_version: snapshot.descriptor_contract_version,
        };
        request
            .validate()
            .map_err(workflow_executable_validation_snapshot_service_error)?;
        let stored = {
            let store = self.attribution_store_guard()?;
            match store.workflow_executable_validation_snapshot(
                AttributionWorkflowExecutableValidationSnapshotLookupRequest {
                    workflow_version_id: request.workflow_version_id.clone(),
                },
            ) {
                Ok(stored) => stored,
                Err(AttributionError::NotFound {
                    entity: "workflow_executable_validation_snapshot",
                }) => return Ok(()),
                Err(error) => return Err(WorkflowServiceError::from(error)),
            }
        };
        let existing =
            ValidatedWorkflowExecutableValidationSnapshotRecord::from_attribution_record(
                stored, &request,
            )
            .map_err(workflow_executable_validation_snapshot_service_error)?;
        snapshot
            .validate_dependency_proof_freshness_matches(existing.as_record())
            .map_err(workflow_executable_validation_snapshot_service_error)
    }

    pub fn workflow_run_graph_query(
        &self,
        request: WorkflowRunGraphQueryRequest,
    ) -> Result<WorkflowRunGraphQueryResponse, WorkflowServiceError> {
        let Some(projection) = self.workflow_run_version_projection(&request.workflow_run_id)?
        else {
            return Ok(WorkflowRunGraphQueryResponse { run_graph: None });
        };
        let run_graph = workflow_run_graph_projection_from_version(projection)?;
        Ok(WorkflowRunGraphQueryResponse {
            run_graph: Some(run_graph),
        })
    }
}

fn workflow_run_graph_projection_from_version(
    projection: WorkflowRunVersionProjection,
) -> Result<WorkflowRunGraphProjection, WorkflowServiceError> {
    let executable_topology: WorkflowExecutableTopology = decode_run_graph_json(
        "workflow executable topology",
        &projection.workflow_version.executable_topology_json,
    )?;
    let presentation_metadata: WorkflowPresentationMetadata = decode_run_graph_json(
        "workflow presentation metadata",
        &projection.presentation_revision.presentation_metadata_json,
    )?;
    let graph_settings: WorkflowGraphRunSettings = decode_run_graph_json(
        "workflow graph run settings",
        &projection.snapshot.graph_settings_json,
    )?;
    let graph = reconstruct_workflow_graph(
        &executable_topology,
        &presentation_metadata,
        &graph_settings,
    )?;
    let graph_diagnostics =
        validate_workflow_graph_contract_diagnostics(&graph, &NodeRegistry::new());

    Ok(WorkflowRunGraphProjection {
        workflow_run_id: projection.snapshot.workflow_run_id.as_str().to_string(),
        workflow_id: projection.snapshot.workflow_id.as_str().to_string(),
        workflow_version_id: projection.snapshot.workflow_version_id.as_str().to_string(),
        workflow_presentation_revision_id: projection
            .snapshot
            .workflow_presentation_revision_id
            .as_str()
            .to_string(),
        workflow_semantic_version: projection.snapshot.workflow_semantic_version,
        workflow_execution_fingerprint: projection.snapshot.workflow_execution_fingerprint,
        snapshot_created_at_ms: projection.snapshot.created_at_ms,
        workflow_version_created_at_ms: projection.workflow_version.created_at_ms,
        presentation_revision_created_at_ms: projection.presentation_revision.created_at_ms,
        graph,
        graph_diagnostics,
        executable_topology,
        presentation_metadata,
        graph_settings,
    })
}

fn workflow_executable_validation_snapshot_service_error(
    error: WorkflowExecutableValidationSnapshotError,
) -> WorkflowServiceError {
    match error {
        WorkflowExecutableValidationSnapshotError::SnapshotSerialization { .. } => {
            WorkflowServiceError::Internal(error.to_string())
        }
        _ => WorkflowServiceError::InvalidRequest(error.to_string()),
    }
}

fn decode_run_graph_json<T: serde::de::DeserializeOwned>(
    label: &str,
    json: &str,
) -> Result<T, WorkflowServiceError> {
    serde_json::from_str(json).map_err(|error| {
        WorkflowServiceError::Internal(format!("stored {label} JSON is invalid: {error}"))
    })
}

fn reconstruct_workflow_graph(
    executable_topology: &WorkflowExecutableTopology,
    presentation_metadata: &WorkflowPresentationMetadata,
    graph_settings: &WorkflowGraphRunSettings,
) -> Result<WorkflowGraph, WorkflowServiceError> {
    let mut positions_by_node_id = BTreeMap::new();
    for node in &presentation_metadata.nodes {
        if positions_by_node_id
            .insert(node.node_id.clone(), node.position.clone())
            .is_some()
        {
            return Err(WorkflowServiceError::Internal(format!(
                "stored workflow presentation metadata has duplicate node id '{}'",
                node.node_id
            )));
        }
    }

    let mut settings_by_node_id = BTreeMap::new();
    for node in &graph_settings.nodes {
        if settings_by_node_id
            .insert(
                node.node_id.clone(),
                (node.node_type.clone(), node.data.clone()),
            )
            .is_some()
        {
            return Err(WorkflowServiceError::Internal(format!(
                "stored workflow graph settings have duplicate node id '{}'",
                node.node_id
            )));
        }
    }

    if graph_settings
        .nodes
        .iter()
        .any(|node| node.node_type == "node-group")
    {
        if graph_settings.schema_version != 1
            || presentation_metadata.schema_version != 1
            || executable_topology.schema_version != 1
        {
            return Err(WorkflowServiceError::Internal(
                "stored grouped workflow graph uses an unsupported schema version".into(),
            ));
        }
        return reconstruct_grouped_workflow_graph(
            executable_topology,
            presentation_metadata,
            positions_by_node_id,
            settings_by_node_id,
        );
    }

    let mut nodes = Vec::with_capacity(executable_topology.nodes.len());
    for node in &executable_topology.nodes {
        let Some(position) = positions_by_node_id.remove(&node.node_id) else {
            return Err(WorkflowServiceError::Internal(format!(
                "stored workflow presentation metadata is missing node '{}'",
                node.node_id
            )));
        };
        let Some((settings_node_type, data)) = settings_by_node_id.remove(&node.node_id) else {
            return Err(WorkflowServiceError::Internal(format!(
                "stored workflow graph settings are missing node '{}'",
                node.node_id
            )));
        };
        if settings_node_type != node.node_type {
            return Err(WorkflowServiceError::Internal(format!(
                "stored workflow graph settings node '{}' type '{}' does not match executable topology type '{}'",
                node.node_id, settings_node_type, node.node_type
            )));
        }
        nodes.push(GraphNode {
            id: node.node_id.clone(),
            node_type: node.node_type.clone(),
            position,
            data,
        });
    }

    if let Some(extra_node_id) = positions_by_node_id.keys().next() {
        return Err(WorkflowServiceError::Internal(format!(
            "stored workflow presentation metadata contains extra node '{extra_node_id}'"
        )));
    }
    if let Some(extra_node_id) = settings_by_node_id.keys().next() {
        return Err(WorkflowServiceError::Internal(format!(
            "stored workflow graph settings contain extra node '{extra_node_id}'"
        )));
    }

    let executable_edges = executable_topology
        .edges
        .iter()
        .map(|edge| {
            (
                edge.source_node_id.as_str(),
                edge.source_port_id.as_str(),
                edge.target_node_id.as_str(),
                edge.target_port_id.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();
    let mut seen_presentation_edges = BTreeSet::new();
    let mut edges = Vec::with_capacity(presentation_metadata.edges.len());
    for edge in &presentation_metadata.edges {
        let edge_key = (
            edge.source_node_id.as_str(),
            edge.source_port_id.as_str(),
            edge.target_node_id.as_str(),
            edge.target_port_id.as_str(),
        );
        if !executable_edges.contains(&edge_key) {
            return Err(WorkflowServiceError::Internal(format!(
                "stored workflow presentation edge '{}' is missing from executable topology",
                edge.edge_id
            )));
        }
        if !seen_presentation_edges.insert(edge_key) {
            return Err(WorkflowServiceError::Internal(format!(
                "stored workflow presentation metadata has duplicate edge '{}'",
                edge.edge_id
            )));
        }
        edges.push(GraphEdge {
            id: edge.edge_id.clone(),
            source: edge.source_node_id.clone(),
            source_handle: edge.source_port_id.clone(),
            target: edge.target_node_id.clone(),
            target_handle: edge.target_port_id.clone(),
        });
    }

    if seen_presentation_edges.len() != executable_edges.len() {
        return Err(WorkflowServiceError::Internal(
            "stored workflow presentation metadata is missing executable edges".to_string(),
        ));
    }

    Ok(WorkflowGraph {
        nodes,
        edges,
        derived_graph: None,
    })
}

fn reconstruct_grouped_workflow_graph(
    executable_topology: &WorkflowExecutableTopology,
    presentation_metadata: &WorkflowPresentationMetadata,
    mut positions_by_node_id: BTreeMap<String, crate::graph::Position>,
    settings_by_node_id: BTreeMap<String, (String, serde_json::Value)>,
) -> Result<WorkflowGraph, WorkflowServiceError> {
    let mut nodes = Vec::with_capacity(settings_by_node_id.len());
    for (node_id, (node_type, data)) in settings_by_node_id {
        let position = positions_by_node_id.remove(&node_id).ok_or_else(|| {
            WorkflowServiceError::Internal(format!(
                "stored workflow presentation metadata is missing authored node '{node_id}'"
            ))
        })?;
        nodes.push(GraphNode {
            id: node_id,
            node_type,
            position,
            data,
        });
    }
    if let Some(extra_node_id) = positions_by_node_id.keys().next() {
        return Err(WorkflowServiceError::Internal(format!(
            "stored workflow presentation metadata contains extra authored node '{extra_node_id}'"
        )));
    }
    let graph = WorkflowGraph {
        nodes,
        edges: presentation_metadata
            .edges
            .iter()
            .map(|edge| GraphEdge {
                id: edge.edge_id.clone(),
                source: edge.source_node_id.clone(),
                source_handle: edge.source_port_id.clone(),
                target: edge.target_node_id.clone(),
                target_handle: edge.target_port_id.clone(),
            })
            .collect(),
        derived_graph: None,
    };
    let primitive = crate::graph::lower_groups(&graph, &NodeRegistry::new())
        .map_err(|error| {
            WorkflowServiceError::Internal(format!(
                "stored authored group graph cannot be lowered: {error}"
            ))
        })?
        .executable_graph;
    let mut primitive_nodes = primitive
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node.node_type.as_str()))
        .collect::<Vec<_>>();
    let mut stored_nodes = executable_topology
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node.node_type.as_str()))
        .collect::<Vec<_>>();
    primitive_nodes.sort();
    stored_nodes.sort();
    if primitive_nodes != stored_nodes {
        return Err(WorkflowServiceError::Internal(
            "stored authored group nodes do not match executable topology".into(),
        ));
    }
    let mut primitive_edges = primitive
        .edges
        .iter()
        .map(|edge| {
            (
                edge.source.as_str(),
                edge.source_handle.as_str(),
                edge.target.as_str(),
                edge.target_handle.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let mut stored_edges = executable_topology
        .edges
        .iter()
        .map(|edge| {
            (
                edge.source_node_id.as_str(),
                edge.source_port_id.as_str(),
                edge.target_node_id.as_str(),
                edge.target_port_id.as_str(),
            )
        })
        .collect::<Vec<_>>();
    primitive_edges.sort();
    stored_edges.sort();
    if primitive_edges != stored_edges {
        return Err(WorkflowServiceError::Internal(
            "stored authored group edges do not match executable topology".into(),
        ));
    }
    // Return the authored view. The caller returns the original primitive
    // topology/versions separately; nothing here rewrites version or lineage facts.
    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::WorkflowGraphDiagnosticCode;
    use pantograph_runtime_attribution::{
        WorkflowPresentationRevisionId, WorkflowRunSnapshotId, WorkflowVersionId,
    };

    #[test]
    fn run_graph_projection_includes_stale_graph_diagnostics() {
        let projection =
            workflow_run_graph_projection_from_version(stale_diffusion_version_projection())
                .expect("run graph projection");

        assert!(projection.graph_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == WorkflowGraphDiagnosticCode::RetiredNodeType
                && diagnostic.node_id.as_deref() == Some("diffusion")
        }));
    }

    fn grouped_record_fixture() -> (WorkflowRunVersionProjection, WorkflowGraph) {
        use crate::graph::{NodeGroup, PortDataType, PortMapping, Position};
        let group = NodeGroup {
            id: "wrapper".into(),
            name: "Stored group".into(),
            nodes: vec![GraphNode {
                id: "extract".into(),
                node_type: "json-filter".into(),
                position: Position { x: 12.5, y: -77.25 },
                data: serde_json::json!({"path":"results[0].text","opaque":[null,7,"🐾"]}),
            }],
            edges: vec![],
            exposed_inputs: vec![PortMapping {
                internal_node_id: "extract".into(),
                internal_port_id: "json".into(),
                group_port_id: "input-json".into(),
                group_port_label: "Input".into(),
                data_type: PortDataType::Any,
            }],
            exposed_outputs: vec![PortMapping {
                internal_node_id: "extract".into(),
                internal_port_id: "value".into(),
                group_port_id: "selected".into(),
                group_port_label: "Selected".into(),
                data_type: PortDataType::Any,
            }],
            position: Position { x: 1.0, y: 2.0 },
            collapsed: true,
            description: Some("Exact stored authoring data".into()),
            color: Some("#abcdef".into()),
        };
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "source".into(),
                    node_type: "selection-input".into(),
                    position: Position { x: -41.25, y: 87.0 },
                    data: serde_json::json!({}),
                },
                GraphNode {
                    id: "wrapper".into(),
                    node_type: "node-group".into(),
                    position: Position { x: 155.0, y: -42.5 },
                    data: serde_json::json!({"group":group,"opaque":{"unchanged":true}}),
                },
                GraphNode {
                    id: "sink".into(),
                    node_type: "text-output".into(),
                    position: Position {
                        x: 801.0,
                        y: 291.25,
                    },
                    data: serde_json::json!({}),
                },
            ],
            edges: vec![
                GraphEdge {
                    id: "authored-input".into(),
                    source: "source".into(),
                    source_handle: "value".into(),
                    target: "wrapper".into(),
                    target_handle: "input-json".into(),
                },
                GraphEdge {
                    id: "authored-output".into(),
                    source: "wrapper".into(),
                    source_handle: "selected".into(),
                    target: "sink".into(),
                    target_handle: "text".into(),
                },
            ],
            derived_graph: None,
        };
        // Synthetic typed record identities, with content produced by the actual
        // public graph owners. These are unit fixtures, not observed execution IDs.
        let mut record = stale_diffusion_version_projection();
        let topology = workflow_executable_topology(&graph).unwrap();
        let fingerprint = workflow_execution_fingerprint_for_topology(&topology).unwrap();
        record.workflow_version.executable_topology_json =
            serde_json::to_string(&topology).unwrap();
        record.workflow_version.execution_fingerprint = fingerprint.clone();
        record.snapshot.workflow_execution_fingerprint = fingerprint;
        record.presentation_revision.presentation_metadata_json =
            workflow_presentation_metadata_json(&workflow_presentation_metadata(&graph)).unwrap();
        record.snapshot.graph_settings_json = crate::graph::workflow_graph_run_settings_json(
            &crate::graph::workflow_graph_run_settings(&graph),
        )
        .unwrap();
        (record, graph)
    }

    #[test]
    fn grouped_history_retains_authored_data_and_stored_behavior_identity() {
        let (mut record, authored) = grouped_record_fixture();
        let mut topology: WorkflowExecutableTopology =
            serde_json::from_str(&record.workflow_version.executable_topology_json).unwrap();
        for node in &mut topology.nodes {
            node.contract_version = "9.8.7".into();
            node.behavior_digest = format!("stored-historical:{}", node.node_id);
        }
        record.workflow_version.executable_topology_json =
            serde_json::to_string(&topology).unwrap();
        let fingerprint = workflow_execution_fingerprint_for_topology(&topology).unwrap();
        record.workflow_version.execution_fingerprint = fingerprint.clone();
        record.snapshot.workflow_execution_fingerprint = fingerprint;
        let actual = workflow_run_graph_projection_from_version(record.clone()).unwrap();
        assert_eq!(actual.executable_topology, topology);
        for node in &authored.nodes {
            assert_eq!(actual.graph.find_node(&node.id), Some(node));
        }
        assert_eq!(actual.graph.edges.len(), authored.edges.len());
        for edge in &authored.edges {
            assert_eq!(
                actual.graph.edges.iter().find(|e| e.id == edge.id),
                Some(edge)
            );
        }
        assert_eq!(
            actual.workflow_version_id,
            record.snapshot.workflow_version_id.as_str()
        );
        assert_eq!(
            actual.workflow_presentation_revision_id,
            record.snapshot.workflow_presentation_revision_id.as_str()
        );
        assert_eq!(
            actual.workflow_execution_fingerprint,
            record.snapshot.workflow_execution_fingerprint
        );
        assert!(actual
            .graph_diagnostics
            .iter()
            .any(|d| d.node_type.as_deref() == Some("node-group") && d.blocking_submission));
    }

    fn stored_group_mut(value: &mut serde_json::Value) -> &mut serde_json::Value {
        &mut value["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|node| node["node_type"] == "node-group")
            .unwrap()["data"]["group"]
    }

    #[test]
    fn grouped_history_refuses_malformed_or_mismatched_stored_records() {
        use serde_json::{json, Value};
        #[derive(Clone, Copy)]
        enum Record {
            Settings,
            Presentation,
            Topology,
        }
        type CorruptionCase = (&'static str, Record, fn(&mut Value), &'static str);
        let cases: Vec<CorruptionCase> = vec![
            (
                "duplicate-settings",
                Record::Settings,
                |v| {
                    let n = v["nodes"][0].clone();
                    v["nodes"].as_array_mut().unwrap().push(n);
                },
                "duplicate node id",
            ),
            (
                "extra-settings",
                Record::Settings,
                |v| {
                    v["nodes"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"node_id":"phantom","node_type":"selection-input","data":{}}));
                },
                "missing authored node 'phantom'",
            ),
            (
                "missing-settings",
                Record::Settings,
                |v| {
                    v["nodes"]
                        .as_array_mut()
                        .unwrap()
                        .retain(|n| n["node_id"] != "source");
                },
                "extra authored node 'source'",
            ),
            (
                "duplicate-position",
                Record::Presentation,
                |v| {
                    let n = v["nodes"][0].clone();
                    v["nodes"].as_array_mut().unwrap().push(n);
                },
                "duplicate node id",
            ),
            (
                "missing-position",
                Record::Presentation,
                |v| {
                    v["nodes"]
                        .as_array_mut()
                        .unwrap()
                        .retain(|n| n["node_id"] != "wrapper");
                },
                "missing authored node 'wrapper'",
            ),
            (
                "extra-position",
                Record::Presentation,
                |v| {
                    v["nodes"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"node_id":"phantom","position":{"x":1,"y":2}}));
                },
                "extra authored node 'phantom'",
            ),
            (
                "malformed-position",
                Record::Presentation,
                |v| {
                    v["nodes"][0].as_object_mut().unwrap().remove("position");
                },
                "JSON is invalid",
            ),
            (
                "wrapper-identity",
                Record::Settings,
                |v| {
                    stored_group_mut(v)["id"] = json!("wrong");
                },
                "cannot be lowered",
            ),
            (
                "malformed-group",
                Record::Settings,
                |v| {
                    *stored_group_mut(v) = json!("broken");
                },
                "cannot be lowered",
            ),
            (
                "child-id-collision",
                Record::Settings,
                |v| {
                    stored_group_mut(v)["nodes"][0]["id"] = json!("source");
                },
                "cannot be lowered",
            ),
            (
                "dangling-mapping",
                Record::Settings,
                |v| {
                    stored_group_mut(v)["exposed_inputs"][0]["internal_node_id"] = json!("missing");
                },
                "cannot be lowered",
            ),
            (
                "wrong-node-type",
                Record::Topology,
                |v| {
                    v["nodes"][0]["node_type"] = json!("merge");
                },
                "nodes do not match",
            ),
            (
                "missing-primitive-node",
                Record::Topology,
                |v| {
                    v["nodes"].as_array_mut().unwrap().pop();
                },
                "nodes do not match",
            ),
            (
                "duplicate-primitive-node",
                Record::Topology,
                |v| {
                    let n = v["nodes"][0].clone();
                    v["nodes"].as_array_mut().unwrap().push(n);
                },
                "nodes do not match",
            ),
            (
                "missing-primitive-edge",
                Record::Topology,
                |v| {
                    v["edges"].as_array_mut().unwrap().pop();
                },
                "edges do not match",
            ),
            (
                "duplicate-primitive-edge",
                Record::Topology,
                |v| {
                    let e = v["edges"][0].clone();
                    v["edges"].as_array_mut().unwrap().push(e);
                },
                "edges do not match",
            ),
            (
                "wrong-primitive-port",
                Record::Topology,
                |v| {
                    v["edges"][0]["target_port_id"] = json!("other");
                },
                "edges do not match",
            ),
            (
                "missing-root-edge",
                Record::Presentation,
                |v| {
                    v["edges"].as_array_mut().unwrap().pop();
                },
                "edges do not match",
            ),
            (
                "dangling-root-edge",
                Record::Presentation,
                |v| {
                    v["edges"][0]["target_node_id"] = json!("missing");
                },
                "cannot be lowered",
            ),
            (
                "duplicate-authored-edge",
                Record::Presentation,
                |v| {
                    let e = v["edges"][0].clone();
                    v["edges"].as_array_mut().unwrap().push(e);
                },
                "cannot be lowered",
            ),
            (
                "unknown-settings-schema",
                Record::Settings,
                |v| {
                    v["schema_version"] = json!(99);
                },
                "unsupported schema",
            ),
            (
                "unknown-presentation-schema",
                Record::Presentation,
                |v| {
                    v["schema_version"] = json!(99);
                },
                "unsupported schema",
            ),
            (
                "unknown-topology-schema",
                Record::Topology,
                |v| {
                    v["schema_version"] = json!(99);
                },
                "unsupported schema",
            ),
        ];
        for (name, field, mutate, expected) in cases {
            let (mut record, _) = grouped_record_fixture();
            let text = match field {
                Record::Settings => &mut record.snapshot.graph_settings_json,
                Record::Presentation => {
                    &mut record.presentation_revision.presentation_metadata_json
                }
                Record::Topology => &mut record.workflow_version.executable_topology_json,
            };
            let mut value: Value = serde_json::from_str(text).unwrap();
            mutate(&mut value);
            *text = value.to_string();
            let error = workflow_run_graph_projection_from_version(record).unwrap_err();
            assert!(
                matches!(error, WorkflowServiceError::Internal(ref message) if message.contains(expected)),
                "{name}: expected Internal containing {expected}, got {error}"
            );
        }
        let (mut record, _) = grouped_record_fixture();
        record.snapshot.graph_settings_json = "{".into();
        assert!(matches!(workflow_run_graph_projection_from_version(record),
            Err(WorkflowServiceError::Internal(message)) if message.contains("JSON is invalid")));
    }

    fn stale_diffusion_version_projection() -> WorkflowRunVersionProjection {
        let workflow_run_id = WorkflowRunId::try_from("run-stale".to_string()).unwrap();
        let workflow_id = WorkflowId::try_from("workflow-stale".to_string()).unwrap();
        let workflow_version_id = WorkflowVersionId::try_from("wfver-stale".to_string()).unwrap();
        let workflow_presentation_revision_id =
            WorkflowPresentationRevisionId::try_from("wfpres-stale".to_string()).unwrap();

        WorkflowRunVersionProjection {
            snapshot: WorkflowRunSnapshotRecord {
                workflow_run_snapshot_id: WorkflowRunSnapshotId::try_from(
                    "runsnap-stale".to_string(),
                )
                .unwrap(),
                workflow_run_id: workflow_run_id.clone(),
                workflow_id: workflow_id.clone(),
                workflow_version_id: workflow_version_id.clone(),
                workflow_presentation_revision_id: workflow_presentation_revision_id.clone(),
                workflow_semantic_version: "1.0.0".to_string(),
                workflow_execution_fingerprint: "workflow-exec-blake3:stale".to_string(),
                client_id: None,
                client_session_id: None,
                bucket_id: None,
                workflow_execution_session_id: "session-stale".to_string(),
                workflow_execution_session_kind: "local".to_string(),
                usage_profile: None,
                keep_alive: false,
                retention_policy: "standard".to_string(),
                scheduler_policy: "priority_then_fifo".to_string(),
                priority: 0,
                timeout_ms: None,
                inputs_json: "[]".to_string(),
                output_targets_json: None,
                override_selection_json: None,
                graph_settings_json: serde_json::json!({
                    "schema_version": 1,
                    "nodes": [{
                        "node_id": "diffusion",
                        "node_type": "diffusion-inference",
                        "data": {}
                    }]
                })
                .to_string(),
                runtime_requirements_json: "{}".to_string(),
                capability_models_json: "[]".to_string(),
                runtime_capabilities_json: "[]".to_string(),
                created_at_ms: 1_000,
            },
            workflow_version: WorkflowVersionRecord {
                workflow_version_id,
                workflow_id: workflow_id.clone(),
                semantic_version: "1.0.0".to_string(),
                execution_fingerprint: "workflow-exec-blake3:stale".to_string(),
                executable_topology_json: serde_json::json!({
                    "schema_version": 1,
                    "nodes": [{
                        "node_id": "diffusion",
                        "node_type": "diffusion-inference",
                        "contract_version": "0.1.0",
                        "behavior_digest": "retired"
                    }],
                    "edges": []
                })
                .to_string(),
                created_at_ms: 900,
            },
            presentation_revision: WorkflowPresentationRevisionRecord {
                workflow_presentation_revision_id,
                workflow_id,
                workflow_version_id: WorkflowVersionId::try_from("wfver-stale".to_string())
                    .unwrap(),
                presentation_fingerprint: "workflow-presentation-blake3:stale".to_string(),
                presentation_metadata_json: serde_json::json!({
                    "schema_version": 1,
                    "nodes": [{
                        "node_id": "diffusion",
                        "position": { "x": 0.0, "y": 0.0 }
                    }],
                    "edges": []
                })
                .to_string(),
                created_at_ms: 950,
            },
        }
    }
}
