//! Read-only host catalog and exactly pinned graph authoring. No execution/loading API.
use super::CustomNodeIdentity;
use serde::{Deserialize, Serialize};

/// Version 1 is a deliberately narrow projection of admitted string-port contracts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CustomNodeAuthoringCatalog {
    pub contract_version: u16,
    pub nodes: Vec<CustomNodeAuthoringEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CustomNodeAuthoringEntry {
    pub identity: CustomNodeIdentity,
    pub label: String,
    pub description: String,
    pub trust: CustomNodeAuthoringTrust,
    pub inputs: Vec<CustomNodeAuthoringPort>,
    pub outputs: Vec<CustomNodeAuthoringPort>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CustomNodeAuthoringTrust {
    TrustedInProcess,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CustomNodeAuthoringPort {
    pub id: String,
    pub label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CustomNodeDraftRequest {
    pub identity: CustomNodeIdentity,
}
