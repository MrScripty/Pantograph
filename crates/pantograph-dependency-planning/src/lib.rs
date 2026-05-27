//! Shared dependency planning contracts for Pantograph.
//!
//! This crate owns typed request, result, diagnostic, and Pumas-facing model
//! reference contracts used across graph execution, host planning, frontend
//! actions, persisted fixtures, and backend/worker handoff boundaries. It does
//! not call Pumas, inspect files, select runtimes, or execute workers.

mod environment;
mod error;
mod model_ref;
mod preflight;
mod producer;
mod readiness;
mod request;
mod result;

pub use environment::{
    DependencyBindingProfileId, DependencyBindingStatusRow, DependencyBindingStatusState,
    DependencyEnvironmentAction, DependencyEnvironmentFailureState, DependencyEnvironmentId,
    DependencyEnvironmentInstallState, DependencyEnvironmentKind, DependencyEnvironmentManifestId,
    DependencyEnvironmentOperation, DependencyEnvironmentOperationState,
    DependencyEnvironmentReadinessState, DependencyEnvironmentRef, DependencyEnvironmentRequest,
    DependencyEnvironmentResult, DependencyEnvironmentValidationCode,
    DependencyEnvironmentValidationError, DependencyEnvironmentValidationState,
    DependencyOperationTimestampMs, DependencyRequirement, DependencyRequirementBinding,
    DependencyRequirementKind, DependencyRequirementName, DependencyValidationFieldPath,
    PythonBindingDetails, PythonPackageManagerKind, PythonRequirementDetails,
    ValidatedDependencyEnvironmentRequest,
};
pub use error::{DependencyPlanningContractError, PumasArtifactEntryPathError};
pub use model_ref::{
    ModelArtifactKind, ModelRefMigrationDiagnostic, ModelStorageKind, ModelValidationState,
    PumasArtifactEntryPath, PumasArtifactLoadPathKind, PumasArtifactLoadTarget, PumasModelRef,
};
pub use preflight::{
    DependencyPlanningIdentityKey, DependencyPreflightRequest, DependencyPreflightResult,
    ValidatedDependencyPreflightRequest, ValidatedDependencyPreflightResult,
};
pub use producer::{
    produce_dependency_requirements_proof, produce_dependency_requirements_proof_from_request,
    DependencyRequirementsAvailabilityFacts, DependencyRequirementsProof,
    DependencyRequirementsProofRequest, DependencyRequirementsProofStatus,
    ValidatedDependencyRequirementsProofRequest,
};
pub use readiness::{
    DependencyReadinessPolicy, DependencyReadinessRequest, ValidatedDependencyReadinessRequest,
};
pub use request::{
    DependencyBindingId, DependencyNodeTypeId, DependencyOverrideFieldsV1,
    DependencyOverrideFingerprint, DependencyOverridePatchV1, DependencyOverrideScope,
    DependencyPlanningCallerContext, DependencyPlanningPlatformContext, DependencyPlanningRequest,
    DependencyPlatformKey, DependencyRequirementsId, DependencyTaskId, DependencyTraitIntent,
    DependencyTraitIntentId, DependencyTraitIntentValue, DeviceIntentId, RuntimeIntentId,
    SchedulerIntent, ValidatedDependencyPlanningRequest,
};
pub use result::{
    DependencyPlanningDiagnostic, DependencyPlanningDiagnosticCode, DependencyPlanningResult,
    DependencyPlanningSeverity, DependencyPlanningState,
};
