use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortDataType {
    Any,
    String,
    Image,
    Audio,
    AudioStream,
    Component,
    Stream,
    Prompt,
    Tools,
    Embedding,
    Document,
    Json,
    KvCache,
    Boolean,
    Number,
    VectorDb,
}

impl PortDataType {
    pub fn is_compatible_with(&self, target: &PortDataType) -> bool {
        if *self == PortDataType::Any || *target == PortDataType::Any {
            return true;
        }
        if self == target {
            return true;
        }
        if matches!(
            (self, target),
            (PortDataType::String, PortDataType::Prompt)
                | (PortDataType::Prompt, PortDataType::String)
        ) {
            return true;
        }
        if matches!(
            (self, target),
            (PortDataType::AudioStream, PortDataType::Stream)
                | (PortDataType::Stream, PortDataType::AudioStream)
        ) {
            return true;
        }
        if *target == PortDataType::String {
            return matches!(
                self,
                PortDataType::Json | PortDataType::Number | PortDataType::Boolean
            );
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortDefinition {
    pub id: String,
    pub label: String,
    pub data_type: PortDataType,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub multiple: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeCategory {
    Input,
    Processing,
    Tool,
    Output,
    Control,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IoBindingOrigin {
    ClientSession,
    Integrated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Reactive,
    Manual,
    Stream,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Reactive
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeDefinition {
    pub node_type: String,
    pub category: NodeCategory,
    pub label: String,
    pub description: String,
    pub io_binding_origin: IoBindingOrigin,
    pub inputs: Vec<PortDefinition>,
    pub outputs: Vec<PortDefinition>,
    #[serde(default)]
    pub execution_mode: ExecutionMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub position: Position,
    #[serde(default)]
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub source_handle: String,
    pub target: String,
    pub target_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct PortMapping {
    pub internal_node_id: String,
    pub internal_port_id: String,
    pub group_port_id: String,
    pub group_port_label: String,
    pub data_type: PortDataType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct NodeGroup {
    pub id: String,
    pub name: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub exposed_inputs: Vec<PortMapping>,
    pub exposed_outputs: Vec<PortMapping>,
    pub position: Position,
    pub collapsed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionAnchor {
    pub node_id: String,
    pub port_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTargetAnchorCandidate {
    pub port_id: String,
    pub port_label: String,
    pub data_type: PortDataType,
    pub multiple: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTargetNodeCandidate {
    pub node_id: String,
    pub node_type: String,
    pub node_label: String,
    pub position: Position,
    pub anchors: Vec<ConnectionTargetAnchorCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InsertableNodeTypeCandidate {
    pub node_type: String,
    pub category: NodeCategory,
    pub label: String,
    pub description: String,
    pub matching_input_port_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InsertNodePositionHint {
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionCandidatesResponse {
    pub graph_revision: String,
    pub revision_matches: bool,
    pub source_anchor: ConnectionAnchor,
    pub compatible_nodes: Vec<ConnectionTargetNodeCandidate>,
    pub insertable_node_types: Vec<InsertableNodeTypeCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionRejectionReason {
    StaleRevision,
    UnknownSourceAnchor,
    UnknownTargetAnchor,
    UnknownEdge,
    DuplicateConnection,
    TargetCapacityReached,
    SelfConnection,
    CycleDetected,
    IncompatibleTypes,
    UnknownInsertNodeType,
    NoCompatibleInsertInput,
    NoCompatibleInsertPath,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionRejection {
    pub reason: ConnectionRejectionReason,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionCommitResponse {
    pub accepted: bool,
    pub graph_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<WorkflowGraph>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_event: Option<node_engine::WorkflowEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_session_state: Option<super::session_contract::WorkflowGraphSessionStateView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection: Option<ConnectionRejection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InsertNodeConnectionResponse {
    pub accepted: bool,
    pub graph_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inserted_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<WorkflowGraph>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_event: Option<node_engine::WorkflowEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_session_state: Option<super::session_contract::WorkflowGraphSessionStateView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection: Option<ConnectionRejection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EdgeInsertionBridge {
    pub input_port_id: String,
    pub output_port_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EdgeInsertionPreviewResponse {
    pub accepted: bool,
    pub graph_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge: Option<EdgeInsertionBridge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection: Option<ConnectionRejection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InsertNodeOnEdgeResponse {
    pub accepted: bool,
    pub graph_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inserted_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge: Option<EdgeInsertionBridge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<WorkflowGraph>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_event: Option<node_engine::WorkflowEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_session_state: Option<super::session_contract::WorkflowGraphSessionStateView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection: Option<ConnectionRejection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WorkflowDerivedGraph {
    pub schema_version: u32,
    pub graph_fingerprint: String,
    pub consumer_count_map: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WorkflowGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_graph: Option<WorkflowDerivedGraph>,
}

impl WorkflowGraph {
    pub const DERIVED_GRAPH_SCHEMA_VERSION: u32 = 1;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn find_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn find_node_mut(&mut self, id: &str) -> Option<&mut GraphNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn has_edge_to(&self, node_id: &str, port_id: &str) -> bool {
        self.edges
            .iter()
            .any(|e| e.target == node_id && e.target_handle == port_id)
    }

    pub fn incoming_edges<'a>(
        &'a self,
        node_id: &'a str,
    ) -> impl Iterator<Item = &'a GraphEdge> + 'a {
        self.edges.iter().filter(move |e| e.target == node_id)
    }

    pub fn outgoing_edges<'a>(
        &'a self,
        node_id: &'a str,
    ) -> impl Iterator<Item = &'a GraphEdge> + 'a {
        self.edges.iter().filter(move |e| e.source == node_id)
    }

    pub fn compute_consumer_count_map(&self) -> HashMap<String, u32> {
        let mut out = HashMap::new();
        for edge in &self.edges {
            let key = format!("{}:{}", edge.source, edge.source_handle);
            out.entry(key).and_modify(|count| *count += 1).or_insert(1);
        }
        out
    }

    pub fn compute_fingerprint(&self) -> String {
        let mut node_rows = self
            .nodes
            .iter()
            .map(|n| format!("{}|{}", n.id, n.node_type))
            .collect::<Vec<_>>();
        node_rows.sort();

        let mut edge_rows = self
            .edges
            .iter()
            .map(|e| {
                format!(
                    "{}|{}|{}|{}",
                    e.source, e.source_handle, e.target, e.target_handle
                )
            })
            .collect::<Vec<_>>();
        edge_rows.sort();

        let mut digest = FNV64_OFFSET_BASIS;
        digest = fnv1a64_update(digest, b"v1");
        for row in node_rows {
            digest = fnv1a64_update(digest, row.as_bytes());
            digest = fnv1a64_update(digest, b"\n");
        }
        digest = fnv1a64_update(digest, b"--");
        for row in edge_rows {
            digest = fnv1a64_update(digest, row.as_bytes());
            digest = fnv1a64_update(digest, b"\n");
        }

        format!("{:016x}", digest)
    }

    pub fn build_derived_graph(&self) -> WorkflowDerivedGraph {
        WorkflowDerivedGraph {
            schema_version: Self::DERIVED_GRAPH_SCHEMA_VERSION,
            graph_fingerprint: self.compute_fingerprint(),
            consumer_count_map: self.compute_consumer_count_map(),
        }
    }

    pub fn refresh_derived_graph(&mut self) {
        self.derived_graph = Some(self.build_derived_graph());
    }

    pub fn effective_consumer_count_map(&self) -> HashMap<String, u32> {
        if let Some(derived) = &self.derived_graph {
            if derived.schema_version == Self::DERIVED_GRAPH_SCHEMA_VERSION
                && derived.graph_fingerprint == self.compute_fingerprint()
            {
                return derived.consumer_count_map.clone();
            }
        }
        self.compute_consumer_count_map()
    }
}

const FNV64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV64_PRIME: u64 = 0x100000001b3;

fn fnv1a64_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV64_PRIME);
    }
    hash
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowGraphMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created: String,
    pub modified: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orchestration_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowFile {
    pub version: String,
    pub metadata: WorkflowGraphMetadata,
    pub graph: WorkflowGraph,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,
}

impl WorkflowFile {
    pub const CURRENT_VERSION: &'static str = "1.0";

    pub fn new(name: impl Into<String>, graph: WorkflowGraph) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            version: Self::CURRENT_VERSION.to_string(),
            metadata: WorkflowGraphMetadata {
                id: None,
                name: name.into(),
                description: None,
                created: now.clone(),
                modified: now,
                orchestration_id: None,
            },
            graph,
            viewport: None,
        }
    }
}
