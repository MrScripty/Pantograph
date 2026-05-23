//! Scheduler-owned dynamic task dispatch contracts for Pantograph.
//!
//! This crate is the canonical owner for scheduler queue, policy, resource
//! admission, batching, dependency-readiness policy, dispatch, and lifecycle
//! boundaries. It does not execute workflow nodes, inspect Pumas storage, launch
//! runtimes, expose frontend actions, or resolve local model paths.

mod capability;
mod error;
mod intent;
mod ownership;

pub use capability::{
    CapabilityAvailabilityState, SchedulerCapabilityDiagnostic, SchedulerCapabilityDiagnosticCode,
    SchedulerCapabilityHintSnapshot, SchedulerCapabilitySeverity, SchedulerDeviceCapabilityHint,
    SchedulerRuntimeCapabilityHint, SchedulerTraitOptionHint, SchedulerTraitOptionValue,
    ValidatedSchedulerCapabilityHintSnapshot, SCHEDULER_CAPABILITY_HINT_CONTRACT_VERSION,
};
pub use error::SchedulerContractError;
pub use intent::{
    SchedulableTaskIntent, SchedulerEstimateHint, SchedulerEstimateHintKind, SchedulerFairnessKey,
    SchedulerNodeId, SchedulerRuntimeDeviceConstraints, SchedulerTaskId, SchedulerTraitId,
    SchedulerTraitSetting, SchedulerTraitValue, SchedulerWorkflowId, SchedulerWorkflowRunId,
    ValidatedSchedulableTaskIntent, SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
};
pub use ownership::{
    owner_for_capability, SchedulerBoundaryConsumer, SchedulerBoundaryOwner,
    SchedulerOwnedCapability, SCHEDULER_CONTRACT_VERSION,
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
