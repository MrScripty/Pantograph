//! Scheduler-owned dynamic task dispatch contracts for Pantograph.
//!
//! This crate is the canonical owner for scheduler queue, policy, resource
//! admission, batching, dependency-readiness policy, dispatch, and lifecycle
//! boundaries. It does not execute workflow nodes, inspect Pumas storage, launch
//! runtimes, expose frontend actions, or resolve local model paths.

mod batching;
mod capability;
mod cohort_completion;
mod completion_ranking;
mod dependency_completion;
mod dispatch;
mod dispatch_selection;
mod dispatch_selection_policy;
mod dispatch_selection_validation;
mod error;
mod handoff;
mod intent;
mod lifecycle;
mod ownership;
mod queue;
mod readiness;
mod resource;
mod resource_types;
mod serial_admission;
mod serial_dispatch;
mod supervision;
mod two_completion;

pub use serial_admission::{
    SchedulerSerialAdmission, SchedulerSerialAdmissionRefusal, SchedulerSerialDispatch,
    SchedulerSerialPreparation,
};
pub use serial_dispatch::{
    SchedulerSerialAttemptIdentity, SchedulerSerialBoundDispatch, SchedulerSerialCleanupEvent,
    SchedulerSerialCleanupPending, SchedulerSerialCleanupState, SchedulerSerialDispatchRefusal,
    SchedulerSerialDrainState, SchedulerSerialDrainedDispatch, SchedulerSerialExecutingDispatch,
    SchedulerSerialOwnerSnapshot, SchedulerSerialRuntimeOwnerLease,
    SchedulerSerialVerifiedWarmDrain,
};

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
pub use cohort_completion::{
    evaluate_scheduler_cohort_completion, SchedulerCohortAction, SchedulerCohortBudget,
    SchedulerCohortCompletion, SchedulerCohortCosts, SchedulerCohortEvidence,
    SchedulerCohortIncomplete, SchedulerCohortPlacement, SchedulerCohortResult,
    SchedulerCohortSample, SchedulerCohortScore, SchedulerCohortTask, SchedulerCohortTaskRequest,
    SchedulerCohortTransition, SchedulerCohortWork, SchedulerFrozenCohort,
    SCHEDULER_COHORT_MAX_BRANCHES, SCHEDULER_COHORT_MAX_EVENTS, SCHEDULER_COHORT_MAX_EVIDENCE,
    SCHEDULER_COHORT_MAX_PLACEMENTS, SCHEDULER_COHORT_MAX_TASKS, SCHEDULER_COHORT_MAX_WORK,
};

pub use completion_ranking::{
    completion_diagnostics_bounded, select_scheduler_candidate_with_completion,
    SchedulerCompletionContext, SchedulerCompletionEvidence, SchedulerCompletionEvidenceSource,
    SchedulerCompletionRankingDiagnostic, SchedulerCompletionRankingPolicy,
    SchedulerCompletionRankingResult, SchedulerCompletionRefusalReason, SchedulerCompletionSample,
    SCHEDULER_COMPLETION_MAX_CANDIDATES,
};

pub use two_completion::{
    select_scheduler_candidate_with_two_completions, SchedulerTwoCompletionBudget,
    SchedulerTwoCompletionContinuation, SchedulerTwoCompletionDiagnostic,
    SchedulerTwoCompletionFallback, SchedulerTwoCompletionPrefix, SchedulerTwoCompletionResult,
    SchedulerTwoCompletionScore, SCHEDULER_TWO_COMPLETION_MAX_EVENTS,
    SCHEDULER_TWO_COMPLETION_MAX_OFFERS, SCHEDULER_TWO_COMPLETION_MAX_PLANS,
};

pub use dependency_completion::{
    select_scheduler_candidate_with_dependency_completion, SchedulerCompletionSuccessorRequest,
    SchedulerDependencyCompletionEvidence, ValidatedSchedulerCompletionSuccessor,
};
pub use dispatch::{
    SchedulerBatchingGroupId, SchedulerDispatchDecision, SchedulerDispatchDiagnostic,
    SchedulerDispatchDiagnosticCode, SchedulerDispatchDiagnosticSeverity,
    SchedulerReservationLeaseId, SchedulerRuntimeVariantId, ValidatedSchedulerDispatchDecision,
    SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION,
};
pub use dispatch_selection::{
    SchedulerDispatchCandidate, SchedulerDispatchCandidateId, SchedulerDispatchSelectionDecision,
    SchedulerDispatchSelectionDiagnostic, SchedulerDispatchSelectionDiagnosticCode,
    SchedulerDispatchSelectionDiagnosticSeverity, SchedulerDispatchSelectionRequest,
    SchedulerDispatchSelectionState, ValidatedSchedulerDispatchSelectionDecision,
    ValidatedSchedulerDispatchSelectionRequest, SCHEDULER_DISPATCH_SELECTION_CONTRACT_VERSION,
};
pub use dispatch_selection_policy::{
    select_scheduler_candidate_for_reservation, select_scheduler_dispatch,
    SchedulerDispatchReservationSelection,
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
    SchedulerNonRuntimeTaskKind, SchedulerSourceInputTaskIntent, SchedulerSourceInputTaskKind,
    SchedulerTaskExecutionIntent, SchedulerTaskState, SchedulerTaskStateDiagnostic,
    SchedulerTaskStateDiagnosticCode, SchedulerTaskStateDiagnosticSeverity, SchedulerTaskStateKind,
    SchedulerTaskStateRecord, SchedulerTaskStateTransition,
    SchedulerTaskStateTransitionApplyResult, SchedulerTaskStateTransitionId,
    ValidatedSchedulerTaskStateRecord, ValidatedSchedulerTaskStateTransition,
    SCHEDULER_TASK_STATE_CONTRACT_VERSION,
};
pub use readiness::{
    plan_scheduler_readiness_admission, SchedulerReadinessAdmissionAction,
    SchedulerReadinessAdmissionDecision, SchedulerReadinessAdmissionDiagnostic,
    SchedulerReadinessAdmissionDiagnosticCode, SchedulerReadinessAdmissionRequest,
    SchedulerReadinessAdmissionSeverity, SchedulerReadinessAdmissionState,
    ValidatedSchedulerReadinessAdmissionDecision, ValidatedSchedulerReadinessAdmissionRequest,
    SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION,
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
