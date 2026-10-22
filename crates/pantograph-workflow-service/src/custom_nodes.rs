//! Host-authorized in-process custom nodes. See `docs/custom-nodes.md`.
//!
//! Admission is a host trust decision, never a graph/palette/code-loading API.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use futures_util::FutureExt;
use node_engine::{Context, ExecutorExtensions, NodeExecutor};
use pantograph_node_contracts::{
    compute_node_behavior_digest, NodeCategory, NodeExecutionSemantics, NodeTypeContract,
    PortCardinality, PortContract, PortRequirement, PortValueType,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::watch;

const MAX_PORTS: usize = 8;
pub const CUSTOM_NODE_STRING_MAX_BYTES: usize = 2048;
const MAX_PAYLOAD_BYTES: usize = 8192;
const INVOCATION_TIMEOUT: Duration = Duration::from_secs(30);
pub const CUSTOM_NODE_IDENTITY_FIELD: &str = "custom_node_identity";
pub const CUSTOM_NODE_INVOCATION_EXTENSION: &str = "pantograph.custom_node_invocation";

/// Host attestations, not measurements of callback code or ambient environment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CustomNodeIdentity {
    pub node_type: String,
    pub contract_version: String,
    pub contract_digest: String,
    pub implementation_digest: String,
    pub environment_digest: String,
    pub adapter_version: u16,
}

/// Scheduler-authored invocation correlation. Graph data cannot override it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomNodeInvocation {
    pub workflow_id: String,
    pub workflow_run_id: String,
    pub task_id: String,
    pub attempt_id: String,
}

type InvocationKey = (String, String, String);

#[derive(Debug, Error)]
pub enum CustomNodeError {
    #[error("invalid custom-node admission: {0}")]
    InvalidAdmission(&'static str),
    #[error("custom-node implementation is unavailable")]
    Unavailable,
    #[error("custom-node identity does not match admitted implementation")]
    IdentityMismatch,
    #[error("custom-node {0} must match the bounded required string port contract")]
    InvalidPayload(&'static str),
    #[error("custom-node invocation was cancelled; callback polling stopped")]
    Cancelled,
    #[error("custom-node invocation timed out; callback polling stopped")]
    TimedOut,
    #[error("custom-node callback failed")]
    CallbackFailed,
    #[error("custom-node callback panicked")]
    Panicked,
    #[error("custom-node attempt is already running")]
    AlreadyRunning,
    #[error("invalid tool-loop admission: {0}")]
    InvalidLoop(&'static str),
    #[error("tool-loop controller returned an invalid bounded action or state")]
    InvalidLoopAction,
    #[error("tool-loop requested a tool outside its host grant")]
    UnauthorizedTool,
    #[error("tool-loop exhausted its admitted turn budget")]
    TurnLimit,
}

/// Immutable trusted implementation. No deserializer or mutable executor access.
///
/// The admitting host must attest that implementation, captured configuration
/// and environment remain immutable, and that the executor is nonblocking,
/// cancellation-safe, has no external effects, and starts no detached work.
/// In-process Rust cannot enforce these obligations or sandbox an executor.
pub struct TrustedCustomNode {
    contract: NodeTypeContract,
    identity: CustomNodeIdentity,
    executor: Arc<dyn NodeExecutor>,
}

impl TrustedCustomNode {
    pub fn admit(
        mut contract: NodeTypeContract,
        implementation_digest: String,
        environment_digest: String,
        executor: Arc<dyn NodeExecutor>,
    ) -> Result<Self, CustomNodeError> {
        contract
            .validate()
            .map_err(|_| CustomNodeError::InvalidAdmission("invalid canonical contract"))?;
        // Reparse IDs: canonical types currently deserialize unchecked strings.
        if contract
            .node_type
            .as_str()
            .parse::<pantograph_node_contracts::NodeTypeId>()
            .ok()
            .as_ref()
            != Some(&contract.node_type)
            || contract.category != NodeCategory::Processing
            || contract.execution_semantics != NodeExecutionSemantics::Batch
            || !contract.capability_requirements.is_empty()
            || !contract.inference_tasks.is_empty()
        {
            return Err(CustomNodeError::InvalidAdmission("only static Processing/Batch nodes without capabilities or inference are supported"));
        }
        validate_ports(&contract.inputs)?;
        validate_ports(&contract.outputs)?;
        for digest in [&implementation_digest, &environment_digest] {
            let hex = digest
                .strip_prefix("blake3:")
                .ok_or(CustomNodeError::InvalidAdmission(
                    "host digests must be blake3 plus 64 lowercase hexadecimal digits",
                ))?;
            if hex.len() != 64
                || !hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(CustomNodeError::InvalidAdmission("invalid host digest"));
            }
        }
        // Caller-supplied contract_digest cannot substitute for contract content.
        let contract_digest = compute_node_behavior_digest(&contract)
            .map_err(|_| CustomNodeError::InvalidAdmission("contract cannot be encoded"))?;
        let identity = CustomNodeIdentity {
            node_type: contract.node_type.as_str().to_string(),
            contract_version: contract.contract_version.clone().ok_or(
                CustomNodeError::InvalidAdmission("missing contract version"),
            )?,
            contract_digest,
            implementation_digest,
            environment_digest,
            adapter_version: 1,
        };
        let bytes = serde_json::to_vec(&identity)
            .map_err(|_| CustomNodeError::InvalidAdmission("identity cannot be encoded"))?;
        contract.contract_digest = Some(format!("custom-node-blake3:{}", blake3::hash(&bytes)));
        Ok(Self {
            contract,
            identity,
            executor,
        })
    }

    pub fn identity(&self) -> &CustomNodeIdentity {
        &self.identity
    }
    pub fn contract(&self) -> &NodeTypeContract {
        &self.contract
    }
}

fn validate_ports(ports: &[PortContract]) -> Result<(), CustomNodeError> {
    if !(1..=MAX_PORTS).contains(&ports.len()) {
        return Err(CustomNodeError::InvalidAdmission(
            "each direction requires 1..8 ports",
        ));
    }
    let mut ids = BTreeSet::new();
    for port in ports {
        if port
            .id
            .as_str()
            .parse::<pantograph_node_contracts::PortId>()
            .ok()
            .as_ref()
            != Some(&port.id)
            || !ids.insert(port.id.as_str())
            || port.id.as_str().starts_with('_')
            || port.value_type != PortValueType::String
            || port.requirement != PortRequirement::Required
            || port.cardinality != PortCardinality::Single
            || !port.constraints.is_empty()
            || !port.inference_payloads.is_empty()
            || port.options_provider.is_some()
        {
            return Err(CustomNodeError::InvalidAdmission("ports must be unique required single strings without constraints/options/inference"));
        }
    }
    Ok(())
}

/// One service-lifetime immutable catalog. There is no replacement or merge API.
#[derive(Default)]
pub struct CustomNodeCatalog {
    entries: BTreeMap<String, TrustedCustomNode>,
    pub(crate) tool_loops: BTreeMap<String, crate::tool_loops::ToolLoopIdentity>,
    active: Mutex<HashMap<InvocationKey, watch::Sender<bool>>>,
}

impl CustomNodeCatalog {
    pub fn try_new(nodes: Vec<TrustedCustomNode>) -> Result<Self, CustomNodeError> {
        let builtins =
            workflow_nodes::builtin_node_contracts().map_err(|_| CustomNodeError::Unavailable)?;
        let mut entries = BTreeMap::new();
        for node in nodes {
            let name = node.identity.node_type.clone();
            if builtins
                .iter()
                .any(|builtin| builtin.node_type.as_str() == name)
                || entries.contains_key(&name)
            {
                return Err(CustomNodeError::InvalidAdmission(
                    "duplicate or built-in node type",
                ));
            }
            entries.insert(name, node);
        }
        Ok(Self {
            entries,
            tool_loops: BTreeMap::new(),
            active: Mutex::new(HashMap::new()),
        })
    }

    /// Admit pure loops and all their primitives together before service creation.
    /// No live catalog mutation or graph-supplied registration is supported.
    pub fn try_new_with_tool_loops(
        nodes: Vec<TrustedCustomNode>,
        loops: Vec<crate::tool_loops::ToolLoopRegistration>,
    ) -> Result<Self, CustomNodeError> {
        let mut catalog = Self::try_new(nodes)?;
        for registration in loops {
            let identity = crate::tool_loops::ToolLoopIdentity::admit(registration, &catalog)?;
            if catalog
                .tool_loops
                .insert(identity.id.clone(), identity)
                .is_some()
            {
                return Err(CustomNodeError::InvalidLoop("duplicate loop id"));
            }
        }
        Ok(catalog)
    }

    pub fn tool_loop_identity(&self, id: &str) -> Option<&crate::tool_loops::ToolLoopIdentity> {
        self.tool_loops.get(id)
    }

    pub fn get(&self, node_type: &str) -> Option<&TrustedCustomNode> {
        self.entries.get(node_type)
    }
    pub(crate) fn nodes(&self) -> impl Iterator<Item = &TrustedCustomNode> {
        self.entries.values()
    }

    pub(crate) fn cancel(
        &self,
        run: &str,
        task: &str,
        attempt: &str,
    ) -> Result<(), CustomNodeError> {
        let active = self.active.lock();
        let sender = active
            .get(&(run.to_string(), task.to_string(), attempt.to_string()))
            .ok_or(CustomNodeError::Unavailable)?;
        sender.send_replace(true);
        Ok(())
    }

    pub(crate) fn prepare(
        self: &Arc<Self>,
        invocation: CustomNodeInvocation,
    ) -> Result<PreparedCustomNodeInvocation, CustomNodeError> {
        let key = (
            invocation.workflow_run_id.clone(),
            invocation.task_id.clone(),
            invocation.attempt_id.clone(),
        );
        let (sender, cancelled) = watch::channel(false);
        let mut active = self.active.lock();
        if active.contains_key(&key) {
            return Err(CustomNodeError::AlreadyRunning);
        }
        active.insert(key.clone(), sender);
        Ok(PreparedCustomNodeInvocation {
            catalog: self.clone(),
            invocation,
            key,
            cancelled,
        })
    }
}

pub(crate) struct PreparedCustomNodeInvocation {
    catalog: Arc<CustomNodeCatalog>,
    invocation: CustomNodeInvocation,
    key: InvocationKey,
    cancelled: watch::Receiver<bool>,
}

impl std::fmt::Debug for PreparedCustomNodeInvocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedCustomNodeInvocation")
            .field("invocation", &self.invocation)
            .finish_non_exhaustive()
    }
}

impl PreparedCustomNodeInvocation {
    pub(crate) fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow()
    }
    pub(crate) async fn invoke(
        &self,
        identity: &CustomNodeIdentity,
        inputs: HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>, CustomNodeError> {
        self.invoke_primitive(identity, inputs, None).await
    }

    pub(crate) fn catalog(&self) -> &CustomNodeCatalog {
        &self.catalog
    }

    pub(crate) fn invocation(&self) -> &CustomNodeInvocation {
        &self.invocation
    }

    pub(crate) async fn invoke_primitive(
        &self,
        identity: &CustomNodeIdentity,
        inputs: HashMap<String, Value>,
        primitive: Option<crate::tool_loops::ToolLoopPrimitiveInvocation>,
    ) -> Result<HashMap<String, Value>, CustomNodeError> {
        let invocation = &self.invocation;
        let mut cancelled = self.cancelled.clone();
        if *cancelled.borrow() {
            return Err(CustomNodeError::Cancelled);
        }
        let node = self
            .catalog
            .get(&identity.node_type)
            .ok_or(CustomNodeError::Unavailable)?;
        if &node.identity != identity {
            return Err(CustomNodeError::IdentityMismatch);
        }
        validate_payload(&node.contract.inputs, &inputs, "inputs")?;
        let context = Context::new();
        let mut extensions = ExecutorExtensions::new();
        extensions.set(CUSTOM_NODE_INVOCATION_EXTENSION, invocation.clone());
        if let Some(primitive) = primitive {
            extensions.set(crate::tool_loops::TOOL_LOOP_PRIMITIVE_EXTENSION, primitive);
        }
        let callback = AssertUnwindSafe(async {
            node.executor
                .execute(&invocation.task_id, inputs, &context, &extensions)
                .await
        })
        .catch_unwind();
        let outputs = tokio::select! {
            biased;
            _ = cancelled.wait_for(|value| *value) => return Err(CustomNodeError::Cancelled),
            result = tokio::time::timeout(INVOCATION_TIMEOUT, callback) => match result {
                Err(_) => return Err(CustomNodeError::TimedOut),
                Ok(Err(_)) => return Err(CustomNodeError::Panicked),
                Ok(Ok(Err(_))) => return Err(CustomNodeError::CallbackFailed),
                Ok(Ok(Ok(outputs))) => outputs,
            }
        };
        if *cancelled.borrow() {
            return Err(CustomNodeError::Cancelled);
        }
        validate_payload(&node.contract.outputs, &outputs, "outputs")?;
        Ok(outputs)
    }
}

impl Drop for PreparedCustomNodeInvocation {
    fn drop(&mut self) {
        self.catalog.active.lock().remove(&self.key);
    }
}

pub(crate) fn validate_payload(
    ports: &[PortContract],
    values: &HashMap<String, Value>,
    direction: &'static str,
) -> Result<(), CustomNodeError> {
    if values.len() != ports.len() {
        return Err(CustomNodeError::InvalidPayload(direction));
    }
    let mut total = 0usize;
    for port in ports {
        let value = values
            .get(port.id.as_str())
            .and_then(Value::as_str)
            .ok_or(CustomNodeError::InvalidPayload(direction))?;
        total += value.len();
        if value.len() > CUSTOM_NODE_STRING_MAX_BYTES || total > MAX_PAYLOAD_BYTES {
            return Err(CustomNodeError::InvalidPayload(direction));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
