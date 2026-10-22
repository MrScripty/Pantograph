use super::{WorkflowService, WorkflowServiceError};
use crate::custom_nodes::{authoring::*, CUSTOM_NODE_IDENTITY_FIELD};
use crate::{GraphEdge, GraphNode, Position, WorkflowGraph, WorkflowGraphEditSessionGraphResponse};
use serde_json::json;

impl WorkflowService {
    /// Empty means no host registrations; transport failure is a separate outcome.
    pub fn custom_node_authoring_catalog(&self) -> CustomNodeAuthoringCatalog {
        CustomNodeAuthoringCatalog {
            contract_version: 1,
            nodes: self
                .node_registry
                .custom_nodes
                .nodes()
                .map(|node| {
                    let ports = |ports: &[pantograph_node_contracts::PortContract]| {
                        ports
                            .iter()
                            .map(|port| CustomNodeAuthoringPort {
                                id: port.id.as_str().into(),
                                label: port.label.clone(),
                            })
                            .collect()
                    };
                    CustomNodeAuthoringEntry {
                        identity: node.identity().clone(),
                        label: node.contract().label.clone(),
                        description: node.contract().description.clone(),
                        trust: CustomNodeAuthoringTrust::TrustedInProcess,
                        inputs: ports(&node.contract().inputs),
                        outputs: ports(&node.contract().outputs),
                    }
                })
                .collect(),
        }
    }

    /// Resolve the user's complete selection before creating any session. Imported pins
    /// never pass through this operation, and are never silently updated.
    pub async fn create_custom_node_draft(
        &self,
        request: CustomNodeDraftRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        let custom = self
            .node_registry
            .custom_nodes
            .get(&request.identity.node_type)
            .ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(
                    crate::custom_nodes::CustomNodeError::Unavailable.to_string(),
                )
            })?;
        if custom.identity() != &request.identity {
            return Err(WorkflowServiceError::InvalidRequest(
                crate::custom_nodes::CustomNodeError::IdentityMismatch.to_string(),
            ));
        }
        let mut graph = WorkflowGraph {
            nodes: vec![GraphNode {
                id: "custom".into(),
                node_type: request.identity.node_type.clone(),
                position: Position { x: 320.0, y: 0.0 },
                data: json!({"label": custom.contract().label, CUSTOM_NODE_IDENTITY_FIELD: custom.identity()}),
            }],
            edges: vec![],
            derived_graph: None,
        };
        for (direction, ports, node_type) in [
            ("input", &custom.contract().inputs, "text-input"),
            ("output", &custom.contract().outputs, "text-output"),
        ] {
            let definition = self
                .node_registry
                .get_definition(node_type)
                .ok_or_else(|| {
                    WorkflowServiceError::Internal("text authoring contract is unavailable".into())
                })?;
            for (index, port) in ports.iter().enumerate() {
                let id = format!("{direction}-{index}");
                let input = direction == "input";
                graph.nodes.push(GraphNode {
                    id: id.clone(),
                    node_type: node_type.into(),
                    position: Position {
                        x: if input { 0.0 } else { 640.0 },
                        y: index as f64 * 160.0,
                    },
                    data: json!({"label": port.label, "text": "", "definition": definition}),
                });
                graph.edges.push(GraphEdge {
                    id: format!("edge-{id}"),
                    source: if input { id.clone() } else { "custom".into() },
                    source_handle: if input {
                        "text".into()
                    } else {
                        port.id.as_str().into()
                    },
                    target: if input { "custom".into() } else { id },
                    target_handle: if input {
                        port.id.as_str().into()
                    } else {
                        "text".into()
                    },
                });
            }
        }
        let session = self.graph_session_store.create_session(graph, None).await;
        self.graph_session_store
            .get_session_graph(&session.session_id)
            .await
    }
}
