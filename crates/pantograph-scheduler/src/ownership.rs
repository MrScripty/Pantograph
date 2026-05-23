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
