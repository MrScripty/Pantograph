//! Scheduler-owned dynamic task dispatch contracts for Pantograph.
//!
//! This crate is the canonical owner for scheduler queue, policy, resource
//! admission, batching, dependency-readiness policy, dispatch, and lifecycle
//! boundaries. It does not execute workflow nodes, inspect Pumas storage, launch
//! runtimes, expose frontend actions, or resolve local model paths.

mod batching;
mod capability;
mod dispatch;
mod error;
mod handoff;
mod intent;
mod lifecycle;
mod ownership;
mod queue;
mod readiness;
mod resource;
mod resource_types;
mod supervision;

pub use batching::{
    SchedulerBatchCandidate, SchedulerBatchDiagnostic, SchedulerBatchDiagnosticCode,
    SchedulerBatchDiagnosticSeverity, SchedulerBatchMemoryImpact, SchedulerBatchPolicyDecision,
    SchedulerBatchPolicyState, ValidatedSchedulerBatchPolicyDecision,
    SCHEDULER_BATCHING_POLICY_CONTRACT_VERSION,
};
pub use capability::{
    CapabilityAvailabilityState, SchedulerCapabilityDiagnostic, SchedulerCapabilityDiagnosticCode,
    SchedulerCapabilityHintSnapshot, SchedulerCapabilitySeverity, SchedulerDeviceCapabilityHint,
    SchedulerRuntimeCapabilityHint, SchedulerTraitOptionHint, SchedulerTraitOptionValue,
    ValidatedSchedulerCapabilityHintSnapshot, SCHEDULER_CAPABILITY_HINT_CONTRACT_VERSION,
};
pub use dispatch::{
    SchedulerBatchingGroupId, SchedulerDispatchDecision, SchedulerDispatchDiagnostic,
    SchedulerDispatchDiagnosticCode, SchedulerDispatchDiagnosticSeverity,
    SchedulerReservationLeaseId, SchedulerRuntimeVariantId, ValidatedSchedulerDispatchDecision,
    SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION,
};
pub use error::SchedulerContractError;
pub use handoff::{
    SchedulerRuntimeHandoff, SchedulerRuntimeHandoffState, ValidatedSchedulerRuntimeHandoff,
    SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION,
};
pub use intent::{
    SchedulableTaskIntent, SchedulerEstimateHint, SchedulerEstimateHintKind, SchedulerFairnessKey,
    SchedulerNodeId, SchedulerRuntimeDeviceConstraints, SchedulerTaskId, SchedulerTraitId,
    SchedulerTraitSetting, SchedulerTraitValue, SchedulerWorkflowId, SchedulerWorkflowRunId,
    ValidatedSchedulableTaskIntent, SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
};
pub use lifecycle::{
    SchedulerTaskLifecycleDiagnostic, SchedulerTaskLifecycleDiagnosticCode,
    SchedulerTaskLifecycleDiagnosticSeverity, SchedulerTaskLifecycleDiagnosticSnapshot,
    ValidatedSchedulerTaskLifecycleDiagnosticSnapshot,
    SCHEDULER_TASK_LIFECYCLE_DIAGNOSTIC_CONTRACT_VERSION,
};
pub use ownership::{
    owner_for_capability, SchedulerBoundaryConsumer, SchedulerBoundaryOwner,
    SchedulerOwnedCapability, SCHEDULER_CONTRACT_VERSION,
};
pub use queue::{
    apply_scheduler_task_state_transition, SchedulerNonRuntimeTaskIntent,
    SchedulerNonRuntimeTaskKind, SchedulerTaskExecutionIntent, SchedulerTaskState,
    SchedulerTaskStateDiagnostic, SchedulerTaskStateDiagnosticCode,
    SchedulerTaskStateDiagnosticSeverity, SchedulerTaskStateKind, SchedulerTaskStateRecord,
    SchedulerTaskStateTransition, SchedulerTaskStateTransitionApplyResult,
    SchedulerTaskStateTransitionId, ValidatedSchedulerTaskStateRecord,
    ValidatedSchedulerTaskStateTransition, SCHEDULER_TASK_STATE_CONTRACT_VERSION,
};
pub use readiness::{
    plan_scheduler_readiness_admission, SchedulerDependencyReadinessProof,
    SchedulerReadinessAdmissionAction, SchedulerReadinessAdmissionDecision,
    SchedulerReadinessAdmissionDiagnostic, SchedulerReadinessAdmissionDiagnosticCode,
    SchedulerReadinessAdmissionRequest, SchedulerReadinessAdmissionSeverity,
    SchedulerReadinessAdmissionState, ValidatedSchedulerReadinessAdmissionDecision,
    ValidatedSchedulerReadinessAdmissionRequest, SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION,
};
pub use resource::{
    SchedulerBatchingMemoryImpact, SchedulerDeviceResourceSnapshot, SchedulerLoadWarmupEstimate,
    SchedulerModelResidency, SchedulerResourceFitAssessment, SchedulerResourceObservationError,
    SchedulerResourceObserver, SchedulerResourceReservation, SchedulerResourceResidencySnapshot,
    SchedulerRuntimeReadiness, ValidatedSchedulerResourceResidencySnapshot,
    SCHEDULER_RESOURCE_RESIDENCY_CONTRACT_VERSION,
};
pub use resource_types::{
    SchedulerModelResidencyState, SchedulerResourceDiagnostic, SchedulerResourceDiagnosticCode,
    SchedulerResourceDiagnosticSeverity, SchedulerResourceFitState, SchedulerResourceKind,
    SchedulerRuntimeReadinessState,
};
pub use supervision::{
    SchedulerLifecycleCancellationState, SchedulerLifecycleComponent,
    SchedulerLifecycleComponentSnapshot, SchedulerLifecycleComponentState,
    SchedulerLifecycleOwnerDiagnostic, SchedulerLifecycleOwnerDiagnosticCode,
    SchedulerLifecycleOwnerDiagnosticSeverity, SchedulerLifecycleOwnerId,
    SchedulerLifecycleOwnerSnapshot, SchedulerLifecyclePanicState, SchedulerLifecycleQueueBound,
    ValidatedSchedulerLifecycleOwnerSnapshot, SCHEDULER_LIFECYCLE_SUPERVISION_CONTRACT_VERSION,
};

#[cfg(test)]
mod tests {
    use super::{
        owner_for_capability, SchedulerBoundaryConsumer, SchedulerBoundaryOwner,
        SchedulerOwnedCapability, SCHEDULER_CONTRACT_VERSION,
    };

    const CAPABILITIES: &[SchedulerOwnedCapability] = &[
        SchedulerOwnedCapability::QueueState,
        SchedulerOwnedCapability::SchedulingPolicy,
        SchedulerOwnedCapability::ResourceAdmission,
        SchedulerOwnedCapability::RuntimeDeviceSelection,
        SchedulerOwnedCapability::DependencyReadinessPolicy,
        SchedulerOwnedCapability::DispatchTiming,
        SchedulerOwnedCapability::DispatchDecision,
        SchedulerOwnedCapability::BatchingPolicy,
        SchedulerOwnedCapability::Lifecycle,
    ];

    const CONSUMERS: &[SchedulerBoundaryConsumer] = &[
        SchedulerBoundaryConsumer::GraphEditor,
        SchedulerBoundaryConsumer::NodeEngine,
        SchedulerBoundaryConsumer::FrontendAdapter,
        SchedulerBoundaryConsumer::TauriCommand,
        SchedulerBoundaryConsumer::RuntimeAdapter,
        SchedulerBoundaryConsumer::RuntimeHost,
        SchedulerBoundaryConsumer::DependencyReadinessService,
        SchedulerBoundaryConsumer::CapabilityService,
        SchedulerBoundaryConsumer::DiagnosticsLedger,
    ];

    #[test]
    fn scheduler_contract_version_is_explicit() {
        assert_eq!(SCHEDULER_CONTRACT_VERSION, 1);
    }

    #[test]
    fn scheduler_owns_all_canonical_capabilities() {
        for capability in CAPABILITIES {
            assert_eq!(
                owner_for_capability(*capability),
                SchedulerBoundaryOwner::Scheduler
            );
        }
    }

    #[test]
    fn external_consumers_do_not_own_scheduler_capabilities() {
        for consumer in CONSUMERS {
            for capability in CAPABILITIES {
                assert!(
                    !consumer.may_own_scheduler_capability(*capability),
                    "{consumer:?} must not own {capability:?}"
                );
            }
        }
    }
}
