//! Scheduler-owned dynamic task dispatch contracts for Pantograph.
//!
//! This crate is the canonical owner for scheduler queue, policy, resource
//! admission, batching, dependency-readiness policy, dispatch, and lifecycle
//! boundaries. It does not execute workflow nodes, inspect Pumas storage, launch
//! runtimes, expose frontend actions, or resolve local model paths.

use serde::{Deserialize, Serialize};

/// Current scheduler contract version for persisted or transported scheduler DTOs.
pub const SCHEDULER_CONTRACT_VERSION: u16 = 1;

/// Scheduler-owned capabilities that must not be implemented in adapters or graph code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SchedulerOwnedCapability {
    QueueState,
    SchedulingPolicy,
    ResourceAdmission,
    RuntimeDeviceSelection,
    DependencyReadinessPolicy,
    DispatchTiming,
    DispatchDecision,
    BatchingPolicy,
    Lifecycle,
}

/// Non-scheduler components that may consume scheduler facts but must not own policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SchedulerBoundaryConsumer {
    GraphEditor,
    NodeEngine,
    FrontendAdapter,
    TauriCommand,
    RuntimeAdapter,
    RuntimeHost,
    DependencyReadinessService,
    CapabilityService,
    DiagnosticsLedger,
}

impl SchedulerBoundaryConsumer {
    /// Returns whether this consumer may own scheduler policy or dispatch decisions.
    #[must_use]
    pub const fn may_own_scheduler_capability(self, capability: SchedulerOwnedCapability) -> bool {
        let _ = (self, capability);
        false
    }
}

/// Canonical boundary owner for scheduler-owned capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SchedulerBoundaryOwner {
    Scheduler,
}

/// Returns the component that owns the supplied scheduler capability.
#[must_use]
pub const fn owner_for_capability(capability: SchedulerOwnedCapability) -> SchedulerBoundaryOwner {
    let _ = capability;
    SchedulerBoundaryOwner::Scheduler
}

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
