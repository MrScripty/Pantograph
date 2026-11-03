//! Host-admitted pure tool-loop contracts. See `docs/tool-loops.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::custom_nodes::{CustomNodeCatalog, CustomNodeError, CustomNodeIdentity};

pub const TOOL_LOOP_IDENTITY_FIELD: &str = "tool_loop_identity";
pub const TOOL_LOOP_PRIMITIVE_EXTENSION: &str = "pantograph.tool_loop_primitive";
pub(crate) const MAX_TURNS: u8 = 16;
pub(crate) const MAX_TOOLS: usize = 8;

/// Host composition only: a controller and explicitly granted pure transforms.
/// All implementations must already be admitted to the same CN-01 catalog.
/// This contract grants no inference, external effects or detached work.
pub struct ToolLoopRegistration {
    pub id: String,
    pub controller: CustomNodeIdentity,
    pub tools: BTreeMap<String, CustomNodeIdentity>,
    pub max_turns: u8,
}

/// Complete immutable authority pinned by graph and scheduler, never installed
/// by deserializing a graph. Adapter version owns the turn protocol and limits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolLoopIdentity {
    pub id: String,
    pub controller: CustomNodeIdentity,
    pub tools: BTreeMap<String, CustomNodeIdentity>,
    pub max_turns: u8,
    pub adapter_version: u16,
}

/// Bounded controller input, encoded as the controller's single string input.
/// Only the latest observation is retained; controllers cannot grow a history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolLoopTurnInput {
    pub prompt: String,
    pub turn: u8,
    pub observation: Option<ToolLoopObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolLoopObservation {
    pub tool: String,
    pub output: String,
}

/// Controller output, encoded as its single string output. Call IDs are minted
/// by the scheduler from attempt and turn; controller text cannot supply one.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ToolLoopAction {
    Complete { response: String },
    CallTool { tool: String, input: String },
}

/// Scheduler-authored primitive attribution supplements CN-01 parent identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolLoopPrimitiveInvocation {
    pub loop_id: String,
    pub turn: u8,
    pub identity: CustomNodeIdentity,
    pub kind: ToolLoopPrimitiveKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolLoopPrimitiveKind {
    Controller,
    PureStringTransform { tool: String, call_id: String },
}

impl ToolLoopIdentity {
    pub(crate) fn admit(
        registration: ToolLoopRegistration,
        catalog: &CustomNodeCatalog,
    ) -> Result<Self, CustomNodeError> {
        if !valid_name(&registration.id)
            || !(1..=MAX_TURNS).contains(&registration.max_turns)
            || registration.tools.len() > MAX_TOOLS
            || registration.tools.keys().any(|name| !valid_name(name))
        {
            return Err(CustomNodeError::InvalidLoop("invalid names or limits"));
        }
        for identity in std::iter::once(&registration.controller).chain(registration.tools.values())
        {
            let node = catalog
                .get(&identity.node_type)
                .ok_or(CustomNodeError::Unavailable)?;
            if node.identity() != identity {
                return Err(CustomNodeError::IdentityMismatch);
            }
            if serde_json::to_vec(identity)
                .map_err(|_| CustomNodeError::InvalidLoop("invalid identity"))?
                .len()
                > 1024
            {
                return Err(CustomNodeError::InvalidLoop(
                    "primitive identity exceeds 1024 bytes",
                ));
            }
            if node.contract().inputs.len() != 1 || node.contract().outputs.len() != 1 {
                return Err(CustomNodeError::InvalidLoop(
                    "primitives require one string input and output",
                ));
            }
        }
        Ok(Self {
            id: registration.id,
            controller: registration.controller,
            tools: registration.tools,
            max_turns: registration.max_turns,
            adapter_version: 1,
        })
    }

    pub(crate) fn behavior_digest(&self, descriptor_digest: &str) -> String {
        let bytes = serde_json::to_vec(&(descriptor_digest, self))
            .expect("tool-loop identity contains only JSON data");
        format!("tool-loop-blake3:{}", blake3::hash(&bytes))
    }
}

fn valid_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
pub(crate) mod tests;
