use std::collections::{BTreeMap, BTreeSet};

use pantograph_runtime_attribution::{
    BucketCreateRequest, BucketDeleteRequest, BucketRecord, ClientRegistrationRequest,
    ClientRegistrationResponse, ClientSessionOpenRequest, ClientSessionOpenResponse,
    ClientSessionRecord, ClientSessionResumeRequest, WorkflowId,
    WorkflowPresentationRevisionRecord, WorkflowPresentationRevisionResolveRequest, WorkflowRunId,
    WorkflowRunSnapshotRecord, WorkflowRunVersionProjection, WorkflowVersionId,
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
    validate_workflow_id, AttributionRepository, WorkflowRunGraphProjection,
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
