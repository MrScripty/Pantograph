//! Read-only constructor facts for hosts; no execution capability or qualification.
use super::WorkflowService;
use std::time::Duration;

/// The execution path selected when the workflow service was constructed.
///
/// This value is configuration, not an owner handle or proof of CPU residency,
/// readiness, entitlement, cleanup, successful execution or qualification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowSchedulerExecutionOwner {
    /// Ordinary service construction. Independently installed providers, including
    /// generic completion timing, are outside this snapshot's scope.
    Ordinary,
    /// Dedicated construction owns serial CPU execution, drain and cleanup ports.
    SerialReadyCpu,
}

/// Immutable, nonserialized constructor configuration for host diagnostics.
///
/// Every `serial_*` field describes configured serial policy only. Enabled flags
/// do not attest an active feature, available native support, a current resident
/// owner, a cache hit, or milestone acceptance. Reading this snapshot invokes no
/// provider or native port and acquires no execution or store lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkflowSchedulerConfiguration {
    execution_owner: WorkflowSchedulerExecutionOwner,
    serial_completion_advisory: bool,
    serial_fair_opportunities: bool,
    serial_residency: bool,
    serial_key_reuse: bool,
    serial_minimum_samples: Option<usize>,
    serial_maximum_sample_age: Option<Duration>,
}

impl WorkflowSchedulerConfiguration {
    pub fn execution_owner(&self) -> WorkflowSchedulerExecutionOwner {
        self.execution_owner
    }

    /// Whether predecessor-conditioned serial completion advice was requested.
    pub fn serial_completion_advisory(&self) -> bool {
        self.serial_completion_advisory
    }

    /// Whether authenticated per-member serial service opportunities were requested.
    pub fn serial_fair_opportunities(&self) -> bool {
        self.serial_fair_opportunities
    }

    /// Whether the serial native preparation/residency policy was requested.
    pub fn serial_residency(&self) -> bool {
        self.serial_residency
    }

    /// Whether bounded immutable serial workload-key reuse was requested.
    pub fn serial_key_reuse(&self) -> bool {
        self.serial_key_reuse
    }

    /// Configured serial sample minimum (3 through 8), absent for ordinary services.
    pub fn serial_minimum_samples(&self) -> Option<usize> {
        self.serial_minimum_samples
    }

    /// Configured serial sample age in `(0, 1 hour]`, retaining nanosecond precision.
    /// Absent for ordinary services; this is not a workflow execution deadline.
    pub fn serial_maximum_sample_age(&self) -> Option<Duration> {
        self.serial_maximum_sample_age
    }
}

impl WorkflowService {
    /// Read the immutable constructor configuration without querying runtime state.
    ///
    /// This snapshot cannot change modes or establish execution qualification.
    /// Ordinary services report serial flags false and serial sample bounds absent;
    /// this says nothing about separately installed generic timing providers.
    pub fn scheduler_configuration(&self) -> WorkflowSchedulerConfiguration {
        match &self.serial_ready_mode {
            Some(mode) => WorkflowSchedulerConfiguration {
                execution_owner: WorkflowSchedulerExecutionOwner::SerialReadyCpu,
                serial_completion_advisory: mode.config.completion_lookahead,
                serial_fair_opportunities: mode.config.fair_service_opportunities,
                serial_residency: mode.config.serial_residency,
                serial_key_reuse: mode.config.bounded_grouping,
                serial_minimum_samples: Some(mode.config.minimum_samples),
                serial_maximum_sample_age: Some(mode.config.maximum_age),
            },
            None => WorkflowSchedulerConfiguration {
                execution_owner: WorkflowSchedulerExecutionOwner::Ordinary,
                serial_completion_advisory: false,
                serial_fair_opportunities: false,
                serial_residency: false,
                serial_key_reuse: false,
                serial_minimum_samples: None,
                serial_maximum_sample_age: None,
            },
        }
    }
}

#[cfg(test)]
#[path = "configuration_tests.rs"]
mod tests;
