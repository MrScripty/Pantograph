use pantograph_node_contracts::{NodeCapabilityRequirement, NodeInstanceId, NodeTypeId};
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunAttribution};
use serde::{Deserialize, Serialize};

use super::{NodeExecutionContext, NodeExecutionError};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ManagedCapabilityKind {
    ModelExecution,
    ResourceAccess,
    Cache,
    Progress,
    Diagnostics,
    ExternalTool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManagedCapabilityRoute {
    pub kind: ManagedCapabilityKind,
    pub capability_id: String,
    pub workflow_id: WorkflowId,
    pub attribution: WorkflowRunAttribution,
    pub node_id: NodeInstanceId,
    pub node_type: NodeTypeId,
    pub required: bool,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

impl ManagedCapabilityRoute {
    pub fn from_context(
        kind: ManagedCapabilityKind,
        capability_id: impl Into<String>,
        context: &NodeExecutionContext,
        required: bool,
        available: bool,
        unavailable_reason: Option<String>,
    ) -> Self {
        Self {
            kind,
            capability_id: capability_id.into(),
            workflow_id: context.workflow_id.clone(),
            attribution: context.attribution.clone(),
            node_id: context.node_id().clone(),
            node_type: context.node_type().clone(),
            required,
            available,
            unavailable_reason,
        }
    }

    pub fn ensure_available(&self) -> Result<(), NodeExecutionError> {
        if self.available {
            Ok(())
        } else {
            Err(NodeExecutionError::CapabilityUnavailable {
                capability_id: self.capability_id.clone(),
            })
        }
    }
}

macro_rules! managed_capability {
    ($name:ident) => {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "camelCase")]
        pub struct $name {
            pub route: ManagedCapabilityRoute,
        }

        impl $name {
            pub fn new(route: ManagedCapabilityRoute) -> Self {
                Self { route }
            }

            pub fn ensure_available(&self) -> Result<(), NodeExecutionError> {
                self.route.ensure_available()
            }
        }
    };
}

managed_capability!(ModelExecutionCapability);
managed_capability!(ResourceAccessCapability);
managed_capability!(CacheCapability);
managed_capability!(ProgressCapability);
managed_capability!(DiagnosticsCapability);
managed_capability!(ExternalToolCapability);

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeManagedCapabilities {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_execution: Vec<ModelExecutionCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_access: Vec<ResourceAccessCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cache: Vec<CacheCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub progress: Vec<ProgressCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DiagnosticsCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_tool: Vec<ExternalToolCapability>,
}

impl NodeManagedCapabilities {
    pub fn from_requirements(
        context: &NodeExecutionContext,
        requirements: &[NodeCapabilityRequirement],
    ) -> Self {
        let mut capabilities = Self {
            progress: vec![ProgressCapability::new(
                ManagedCapabilityRoute::from_context(
                    ManagedCapabilityKind::Progress,
                    "progress",
                    context,
                    true,
                    true,
                    None,
                ),
            )],
            diagnostics: vec![DiagnosticsCapability::new(
                ManagedCapabilityRoute::from_context(
                    ManagedCapabilityKind::Diagnostics,
                    "diagnostics",
                    context,
                    true,
                    true,
                    None,
                ),
            )],
            ..Self::default()
        };

        for requirement in requirements {
            capabilities.push_requirement(context, requirement);
        }

        capabilities
    }

    fn push_requirement(
        &mut self,
        context: &NodeExecutionContext,
        requirement: &NodeCapabilityRequirement,
    ) {
        let route = ManagedCapabilityRoute::from_context(
            managed_capability_kind(&requirement.capability_id),
            requirement.capability_id.clone(),
            context,
            requirement.required,
            true,
            None,
        );
        match route.kind {
            ManagedCapabilityKind::ModelExecution => {
                self.model_execution
                    .push(ModelExecutionCapability::new(route));
            }
            ManagedCapabilityKind::ResourceAccess => {
                self.resource_access
                    .push(ResourceAccessCapability::new(route));
            }
            ManagedCapabilityKind::Cache => self.cache.push(CacheCapability::new(route)),
            ManagedCapabilityKind::Progress => {
                self.progress.push(ProgressCapability::new(route));
            }
            ManagedCapabilityKind::Diagnostics => {
                self.diagnostics.push(DiagnosticsCapability::new(route));
            }
            ManagedCapabilityKind::ExternalTool => {
                self.external_tool.push(ExternalToolCapability::new(route));
            }
        }
    }
}

fn managed_capability_kind(capability_id: &str) -> ManagedCapabilityKind {
    match capability_id {
        "llm" | "embedding" | "image_generation" | "audio_generation" | "model_library" => {
            ManagedCapabilityKind::ModelExecution
        }
        "cache" | "kv_cache" => ManagedCapabilityKind::Cache,
        "progress" => ManagedCapabilityKind::Progress,
        "diagnostics" => ManagedCapabilityKind::Diagnostics,
        capability if capability.starts_with("resource") => ManagedCapabilityKind::ResourceAccess,
        _ => ManagedCapabilityKind::ExternalTool,
    }
}
