//! Shared dependency planning contracts for Pantograph.
//!
//! This crate owns typed request, result, diagnostic, and Pumas-facing model
//! reference contracts used across graph execution, host planning, frontend
//! actions, persisted fixtures, and backend/worker handoff boundaries. It does
//! not call Pumas, inspect files, select runtimes, or execute workers.

mod environment;
mod error;
mod execution;
mod model_ref;
mod preflight;
mod producer;
mod readiness;
mod request;
mod result;

pub use environment::{
    dependency_environment_result_from_inventory_observations, known_device_classes,
    known_device_toolchain_ids, known_runtime_feature_ids, DependencyBindingProfileId,
    DependencyBindingStatusRow, DependencyBindingStatusState, DependencyEnvironmentAction,
    DependencyEnvironmentFailureState, DependencyEnvironmentId, DependencyEnvironmentInstallState,
    DependencyEnvironmentKind, DependencyEnvironmentManifestId, DependencyEnvironmentOperation,
    DependencyEnvironmentOperationState, DependencyEnvironmentReadinessState,
    DependencyEnvironmentRef, DependencyEnvironmentRequest, DependencyEnvironmentResult,
    DependencyEnvironmentValidationCode, DependencyEnvironmentValidationError,
    DependencyEnvironmentValidationState, DependencyInventoryObservationFreshness,
    DependencyInventoryObservationProjection, DependencyInventoryObservationRow,
    DependencyInventoryObservationState, DependencyOperationTimestampMs,
    DependencyProviderSourceAlternative, DependencyProviderSourceState, DependencyRequirement,
    DependencyRequirementBinding, DependencyRequirementKind, DependencyRequirementName,
    DependencyValidationFieldPath, DeviceClassSourceId, DeviceObservationId,
    DeviceToolchainBindingDetails, DeviceToolchainProviderSourceRow,
    DeviceToolchainProviderSourceSnapshot, DeviceToolchainRequirementDetails,
    DeviceToolchainSourceId, HostPlatformSourceId, ManagedRuntimeBindingDetails,
    ManagedRuntimeRequirementDetails, ManagedRuntimeSourceId, PythonBindingDetails,
    PythonPackageManagerKind, PythonRequirementDetails, RuntimeFeatureBindingDetails,
    RuntimeFeatureProviderSourceRow, RuntimeFeatureProviderSourceSnapshot,
    RuntimeFeatureRequirementDetails, RuntimeFeatureSourceId, RuntimeSourceId,
    RuntimeVariantSourceId, SystemPackageBindingDetails, SystemPackageManagerSourceId,
    SystemPackageProviderSourceRow, SystemPackageProviderSourceSnapshot,
    SystemPackageRequirementDetails, SystemPackageSourceId, ValidatedDependencyEnvironmentRequest,
    ValidatedDependencyEnvironmentResult, ValidatedDependencyInventoryObservationProjection,
    ValidatedDeviceToolchainProviderSourceSnapshot, ValidatedRuntimeFeatureProviderSourceSnapshot,
    ValidatedSystemPackageProviderSourceSnapshot, DEVICE_CLASS_CPU, DEVICE_CLASS_CUDA,
    DEVICE_CLASS_METAL, DEVICE_CLASS_MPS, DEVICE_TOOLCHAIN_CUDA_RUNTIME,
    DEVICE_TOOLCHAIN_LLAMACPP_DEVICE_INVENTORY, DEVICE_TOOLCHAIN_METAL_RUNTIME,
    DEVICE_TOOLCHAIN_MPS_RUNTIME, DEVICE_TOOLCHAIN_PYTORCH_DEVICE_PROBE,
    RUNTIME_FEATURE_CUSTOM_CODE, RUNTIME_FEATURE_DEVICE_SELECTION,
    RUNTIME_FEATURE_EXTERNAL_CONNECTION, RUNTIME_FEATURE_KV_CACHE, RUNTIME_FEATURE_POSTPROCESSING,
    RUNTIME_FEATURE_PREPROCESSING, RUNTIME_FEATURE_REQUEST_LIFECYCLE, RUNTIME_FEATURE_STREAMING,
};
pub use error::{DependencyPlanningContractError, PumasArtifactEntryPathError};
pub use execution::{
    DependencyReadinessCorrelationId, DependencyReadinessDescriptorFingerprint,
    DependencyReadinessExecutionContext, DependencyReadinessGraphRevision,
    DependencyReadinessNodeId, DependencyReadinessProofEnvelope, DependencyReadinessProofId,
    DependencyReadinessProofVersion, DependencyReadinessRequestEnvelope,
    DependencyReadinessSchedulerTaskId, DependencyReadinessValidationSessionId,
    DependencyReadinessValidationSnapshotId, DependencyReadinessWorkflowId,
    DependencyReadinessWorkflowRunId, ValidatedDependencyReadinessExecutionContext,
    ValidatedDependencyReadinessProofEnvelope, ValidatedDependencyReadinessRequestEnvelope,
};
pub use model_ref::{
    ModelArtifactKind, ModelRefMigrationDiagnostic, ModelStorageKind, ModelValidationState,
    PumasArtifactEntryPath, PumasArtifactLoadPathKind, PumasArtifactLoadTarget, PumasModelRef,
};
pub use preflight::{
    dependency_preflight_result_from_environment_result, DependencyPlanningIdentityKey,
    DependencyPreflightRequest, DependencyPreflightResult, ValidatedDependencyPreflightRequest,
    ValidatedDependencyPreflightResult,
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
