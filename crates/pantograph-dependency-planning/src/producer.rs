use serde::{Deserialize, Serialize};

use crate::error::DependencyPlanningContractError;
use crate::model_ref::{ModelArtifactKind, PumasModelRef};
use crate::preflight::{validate_contract_version, DependencyPlanningIdentityKey};
use crate::request::{
    DependencyBindingId, DependencyOverrideFingerprint, DependencyOverridePatchV1,
    DependencyPlanningPlatformContext, DependencyPlanningRequest, DependencyRequirementsId,
    DependencyTaskId, DependencyTraitIntent, SchedulerIntent, ValidatedDependencyPlanningRequest,
};
use crate::result::{DependencyPlanningDiagnostic, DependencyPlanningState};

const DEPENDENCY_REQUIREMENTS_ID_PREFIX: &str = "dependency-requirements-blake3:";
const DEPENDENCY_OVERRIDE_FINGERPRINT_PREFIX: &str = "dependency-overrides-blake3:";

/// Optional typed availability facts supplied to the dependency requirements
/// producer.
///
/// These facts are supplied by the caller. This contract does not discover
/// Pumas package state, inspect local files, or select runtime/device policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyRequirementsAvailabilityFacts {
    #[serde(default = "default_dependency_requirements_producer_contract_version")]
    pub contract_version: u32,
    pub state: DependencyPlanningState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

impl DependencyRequirementsAvailabilityFacts {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_contract_version(
            self.contract_version,
            "dependency_requirements_availability_facts.contract_version",
            "only dependency requirements availability facts contract version 1 is supported",
        )?;
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

/// Path-free producer request for dependency requirements proof creation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyRequirementsProofRequest {
    #[serde(default = "default_dependency_requirements_producer_contract_version")]
    pub contract_version: u32,
    pub planning_request: DependencyPlanningRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub availability_facts: Option<DependencyRequirementsAvailabilityFacts>,
}

impl DependencyRequirementsProofRequest {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_contract_version(
            self.contract_version,
            "dependency_requirements_proof_request.contract_version",
            "only dependency requirements proof request contract version 1 is supported",
        )?;
        self.planning_request.validate()?;
        DependencyPlanningIdentityKey::from_planning_request(&self.planning_request)?;
        if let Some(availability_facts) = &self.availability_facts {
            availability_facts.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyRequirementsProofRequest(DependencyRequirementsProofRequest);

impl ValidatedDependencyRequirementsProofRequest {
    pub fn into_inner(self) -> DependencyRequirementsProofRequest {
        self.0
    }

    pub fn as_request(&self) -> &DependencyRequirementsProofRequest {
        &self.0
    }
}

impl TryFrom<DependencyRequirementsProofRequest> for ValidatedDependencyRequirementsProofRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyRequirementsProofRequest) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyRequirementsProofRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let request: DependencyRequirementsProofRequest =
            serde_json::from_value(value).map_err(|_| {
                DependencyPlanningContractError::InvalidField {
                    field: "dependency_requirements_proof_request",
                    reason: "request JSON did not match dependency requirements proof contract",
                }
            })?;
        Self::try_from(request)
    }
}

/// Status of a produced dependency requirements proof.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyRequirementsProofStatus {
    Current,
    Unavailable,
    Invalid,
    Stale,
    Ambiguous,
    NeedsDetail,
    Missing,
    NotImplemented,
}

/// Path-free proof produced from a validated dependency planning request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyRequirementsProof {
    #[serde(default = "default_dependency_requirements_producer_contract_version")]
    pub contract_version: u32,
    pub identity_key: DependencyPlanningIdentityKey,
    pub dependency_requirements_id: DependencyRequirementsId,
    pub dependency_override_fingerprint: DependencyOverrideFingerprint,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trait_intents: Vec<DependencyTraitIntent>,
    pub status: DependencyRequirementsProofStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

impl DependencyRequirementsProof {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_contract_version(
            self.contract_version,
            "dependency_requirements_proof.contract_version",
            "only dependency requirements proof contract version 1 is supported",
        )?;
        self.identity_key.validate()?;
        for intent in &self.trait_intents {
            intent.validate()?;
        }
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

/// Produce a path-free dependency requirements proof.
///
/// # Errors
///
/// Returns `DependencyPlanningContractError` when the validated request cannot
/// be converted into path-free dependency requirements identity.
pub fn produce_dependency_requirements_proof(
    request: &ValidatedDependencyPlanningRequest,
    availability_facts: Option<&DependencyRequirementsAvailabilityFacts>,
) -> Result<DependencyRequirementsProof, DependencyPlanningContractError> {
    if let Some(availability_facts) = availability_facts {
        availability_facts.validate()?;
    }
    produce_from_parts(request.as_request(), availability_facts)
}

/// Produce a path-free dependency requirements proof from a validated producer
/// request.
///
/// # Errors
///
/// Returns `DependencyPlanningContractError` when the producer request is not
/// path-free or canonical identity cannot be serialized.
pub fn produce_dependency_requirements_proof_from_request(
    request: &ValidatedDependencyRequirementsProofRequest,
) -> Result<DependencyRequirementsProof, DependencyPlanningContractError> {
    produce_from_parts(
        &request.as_request().planning_request,
        request.as_request().availability_facts.as_ref(),
    )
}

fn produce_from_parts(
    request: &DependencyPlanningRequest,
    availability_facts: Option<&DependencyRequirementsAvailabilityFacts>,
) -> Result<DependencyRequirementsProof, DependencyPlanningContractError> {
    let identity_key = DependencyPlanningIdentityKey::from_planning_request(request)?;
    let dependency_override_fingerprint = hash_override_fingerprint(
        "dependency_override_patches",
        &CanonicalOverrideIdentity {
            patches: &request.dependency_override_patches,
        },
    )?;
    let dependency_requirements_id = hash_requirements_id(
        "dependency_requirements_identity",
        &CanonicalRequirementsIdentity {
            model_ref: CanonicalPumasModelRef::from(&request.model_ref),
            task_id: &request.task_id,
            task_type: request.task_type.as_ref(),
            expected_artifact_kind: request.expected_artifact_kind.as_ref(),
            scheduler_intent: &request.scheduler_intent,
            platform_context: request.platform_context.as_ref(),
            selected_binding_ids: &request.selected_binding_ids,
            dependency_override_fingerprint: dependency_override_fingerprint.as_str(),
            trait_intents: &request.trait_intents,
        },
    )?;

    let status = availability_facts
        .map(|facts| proof_status_from_planning_state(facts.state))
        .unwrap_or(DependencyRequirementsProofStatus::Current);
    let diagnostics = availability_facts
        .map(|facts| facts.diagnostics.clone())
        .unwrap_or_default();

    let proof = DependencyRequirementsProof {
        contract_version: 1,
        identity_key,
        dependency_requirements_id,
        dependency_override_fingerprint,
        trait_intents: request.trait_intents.clone(),
        status,
        diagnostics,
    };
    proof.validate()?;
    Ok(proof)
}

fn proof_status_from_planning_state(
    state: DependencyPlanningState,
) -> DependencyRequirementsProofStatus {
    match state {
        DependencyPlanningState::Ready => DependencyRequirementsProofStatus::Current,
        DependencyPlanningState::Unavailable => DependencyRequirementsProofStatus::Unavailable,
        DependencyPlanningState::Invalid => DependencyRequirementsProofStatus::Invalid,
        DependencyPlanningState::Stale => DependencyRequirementsProofStatus::Stale,
        DependencyPlanningState::Ambiguous => DependencyRequirementsProofStatus::Ambiguous,
        DependencyPlanningState::NeedsDetail => DependencyRequirementsProofStatus::NeedsDetail,
        DependencyPlanningState::Missing => DependencyRequirementsProofStatus::Missing,
        DependencyPlanningState::NotImplemented => {
            DependencyRequirementsProofStatus::NotImplemented
        }
    }
}

fn hash_requirements_id<T: Serialize>(
    field: &'static str,
    value: &T,
) -> Result<DependencyRequirementsId, DependencyPlanningContractError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| DependencyPlanningContractError::CanonicalSerializationFailed { field })?;
    DependencyRequirementsId::parse(format!(
        "{DEPENDENCY_REQUIREMENTS_ID_PREFIX}{}",
        blake3::hash(&bytes).to_hex()
    ))
}

fn hash_override_fingerprint<T: Serialize>(
    field: &'static str,
    value: &T,
) -> Result<DependencyOverrideFingerprint, DependencyPlanningContractError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| DependencyPlanningContractError::CanonicalSerializationFailed { field })?;
    DependencyOverrideFingerprint::parse(format!(
        "{DEPENDENCY_OVERRIDE_FINGERPRINT_PREFIX}{}",
        blake3::hash(&bytes).to_hex()
    ))
}

#[derive(Serialize)]
struct CanonicalRequirementsIdentity<'a> {
    model_ref: CanonicalPumasModelRef<'a>,
    task_id: &'a DependencyTaskId,
    #[serde(skip_serializing_if = "Option::is_none")]
    task_type: Option<&'a DependencyTaskId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_artifact_kind: Option<&'a ModelArtifactKind>,
    scheduler_intent: &'a SchedulerIntent,
    #[serde(skip_serializing_if = "Option::is_none")]
    platform_context: Option<&'a DependencyPlanningPlatformContext>,
    selected_binding_ids: &'a [DependencyBindingId],
    dependency_override_fingerprint: &'a str,
    trait_intents: &'a [DependencyTraitIntent],
}

#[derive(Serialize)]
struct CanonicalOverrideIdentity<'a> {
    patches: &'a [DependencyOverridePatchV1],
}

#[derive(Serialize)]
struct CanonicalPumasModelRef<'a> {
    model_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_artifact_id: Option<&'a str>,
}

impl<'a> From<&'a PumasModelRef> for CanonicalPumasModelRef<'a> {
    fn from(value: &'a PumasModelRef) -> Self {
        Self {
            model_id: value.model_id.as_str(),
            revision: value.revision.as_deref(),
            selected_artifact_id: value.selected_artifact_id.as_deref(),
        }
    }
}

fn default_dependency_requirements_producer_contract_version() -> u32 {
    1
}
