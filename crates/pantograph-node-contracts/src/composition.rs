use serde::{Deserialize, Serialize};

use crate::{
    migration::ContractUpgradeRecord, validate_display_text, NodeContractError, NodeInstanceId,
    NodeTypeContract, NodeTypeId, PortId, PortKind, MAX_ID_LEN, MAX_LABEL_LEN,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ComposedNodeContract {
    pub external_contract: NodeTypeContract,
    pub internal_graph: ComposedInternalGraph,
    pub port_mappings: ComposedPortMappings,
    pub trace_policy: ComposedTracePolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upgrade_metadata: Option<ContractUpgradeRecord>,
}

impl ComposedNodeContract {
    pub fn validate(&self) -> Result<(), NodeContractError> {
        self.external_contract.validate()?;
        self.internal_graph.validate()?;
        self.port_mappings
            .validate(&self.external_contract, &self.internal_graph)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ComposedInternalGraph {
    pub graph_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<ComposedInternalNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<ComposedInternalEdge>,
}

impl ComposedInternalGraph {
    pub fn validate(&self) -> Result<(), NodeContractError> {
        validate_display_text("composed.graph_id", &self.graph_id, MAX_ID_LEN)?;
        for node in &self.nodes {
            validate_display_text("composed.node.label", &node.label, MAX_LABEL_LEN)?;
        }
        for edge in &self.edges {
            self.require_node(&edge.source_node_id)?;
            self.require_node(&edge.target_node_id)?;
        }
        Ok(())
    }

    fn contains_node(&self, node_id: &NodeInstanceId) -> bool {
        self.nodes.iter().any(|node| &node.node_id == node_id)
    }

    fn require_node(&self, node_id: &NodeInstanceId) -> Result<(), NodeContractError> {
        if self.contains_node(node_id) {
            Ok(())
        } else {
            Err(NodeContractError::UnknownCompositionInternalNode {
                node_id: node_id.clone(),
            })
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ComposedInternalNode {
    pub node_id: NodeInstanceId,
    pub node_type: NodeTypeId,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ComposedInternalEdge {
    pub source_node_id: NodeInstanceId,
    pub source_port_id: PortId,
    pub target_node_id: NodeInstanceId,
    pub target_port_id: PortId,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ComposedPortMappings {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<ComposedPortMapping>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<ComposedPortMapping>,
}

impl ComposedPortMappings {
    pub fn validate(
        &self,
        external_contract: &NodeTypeContract,
        internal_graph: &ComposedInternalGraph,
    ) -> Result<(), NodeContractError> {
        for port in &external_contract.inputs {
            if !self
                .inputs
                .iter()
                .any(|mapping| mapping.external_port_id == port.id)
            {
                return Err(NodeContractError::MissingCompositionPortMapping {
                    port_id: port.id.clone(),
                    kind: PortKind::Input,
                });
            }
        }
        for port in &external_contract.outputs {
            if !self
                .outputs
                .iter()
                .any(|mapping| mapping.external_port_id == port.id)
            {
                return Err(NodeContractError::MissingCompositionPortMapping {
                    port_id: port.id.clone(),
                    kind: PortKind::Output,
                });
            }
        }
        for mapping in &self.inputs {
            require_external_port(
                external_contract,
                &mapping.external_port_id,
                PortKind::Input,
            )?;
            internal_graph.require_node(&mapping.internal_node_id)?;
        }
        for mapping in &self.outputs {
            require_external_port(
                external_contract,
                &mapping.external_port_id,
                PortKind::Output,
            )?;
            internal_graph.require_node(&mapping.internal_node_id)?;
        }
        Ok(())
    }
}

fn require_external_port(
    contract: &NodeTypeContract,
    port_id: &PortId,
    kind: PortKind,
) -> Result<(), NodeContractError> {
    let found = match kind {
        PortKind::Input => contract.input(port_id),
        PortKind::Output => contract.output(port_id),
    };
    if found.is_some() {
        Ok(())
    } else {
        Err(NodeContractError::UnknownCompositionExternalPort {
            port_id: port_id.clone(),
            kind,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ComposedPortMapping {
    pub external_port_id: PortId,
    pub internal_node_id: NodeInstanceId,
    pub internal_port_id: PortId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComposedTracePolicy {
    PreservePrimitiveFacts,
    SummarizeOnly,
}
