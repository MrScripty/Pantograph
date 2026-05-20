//! Shared dependency planning contracts for Pantograph.
//!
//! This crate owns typed request, result, diagnostic, and Pumas-facing model
//! reference contracts used across graph execution, host planning, frontend
//! actions, persisted fixtures, and backend/worker handoff boundaries. It does
//! not call Pumas, inspect files, select runtimes, or execute workers.

mod error;
mod model_ref;
mod request;
mod result;

pub use error::{DependencyPlanningContractError, PumasArtifactEntryPathError};
pub use model_ref::{
    ModelArtifactKind, ModelRefMigrationDiagnostic, ModelStorageKind, ModelValidationState,
    PumasArtifactEntryPath, PumasArtifactLoadPathKind, PumasArtifactLoadTarget, PumasModelRef,
};
pub use request::{
    DependencyBindingId, DependencyPlanningCallerContext, DependencyPlanningRequest,
    DependencyTaskId, DeviceIntentId, RuntimeIntentId, SchedulerIntent,
    ValidatedDependencyPlanningRequest,
};
pub use result::{
    DependencyPlanningDiagnostic, DependencyPlanningDiagnosticCode, DependencyPlanningResult,
    DependencyPlanningSeverity, DependencyPlanningState,
};
