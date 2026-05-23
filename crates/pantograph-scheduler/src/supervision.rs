use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SchedulerContractError;

const MAX_ID_LEN: usize = 128;
const MAX_TEXT_LEN: usize = 1024;
const MAX_QUEUE_BOUND: u32 = 1_000_000;

/// Current contract version for scheduler lifecycle supervision.
pub const SCHEDULER_LIFECYCLE_SUPERVISION_CONTRACT_VERSION: u16 = 1;

macro_rules! supervision_id {
    ($name:ident, $field:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[must_use]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl AsRef<str>) -> Result<Self, SchedulerContractError> {
                validate_identifier($field, value.as_ref()).map(Self)
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl FromStr for $name {
            type Err = SchedulerContractError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = SchedulerContractError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

supervision_id!(SchedulerLifecycleOwnerId, "lifecycle_owner_id");

/// Canonical long-running scheduler component owned by one lifecycle owner.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerLifecycleComponent {
    QueueWorker,
    DependencyReadinessAction,
    ResourceObservationLoop,
    RuntimeHostDispatch,
    RetryLoop,
    ReservationCleanup,
}

impl SchedulerLifecycleComponent {
    fn requires_bounded_queue(self) -> bool {
        matches!(
            self,
            SchedulerLifecycleComponent::QueueWorker
                | SchedulerLifecycleComponent::DependencyReadinessAction
                | SchedulerLifecycleComponent::ResourceObservationLoop
                | SchedulerLifecycleComponent::RuntimeHostDispatch
                | SchedulerLifecycleComponent::RetryLoop
        )
    }
}

/// Runtime state for one scheduler lifecycle component.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerLifecycleComponentState {
    NotStarted,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed,
}

/// Cancellation state observed by the scheduler lifecycle owner.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerLifecycleCancellationState {
    NotRequested,
    Requested,
    Draining,
    Completed,
}

/// Panic state observed for a supervised scheduler component.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerLifecyclePanicState {
    None,
    Observed,
    IsolatedRestartable,
    Terminal,
}

/// Bounded queue contract for supervised long-running scheduler components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerLifecycleQueueBound {
    pub max_in_flight: u32,
    pub max_buffered: u32,
}

impl SchedulerLifecycleQueueBound {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_bound("queue_bound.max_in_flight", self.max_in_flight)?;
        validate_bound("queue_bound.max_buffered", self.max_buffered)
    }
}

/// Diagnostic emitted by the scheduler lifecycle owner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerLifecycleOwnerDiagnostic {
    pub severity: SchedulerLifecycleOwnerDiagnosticSeverity,
    pub code: SchedulerLifecycleOwnerDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl SchedulerLifecycleOwnerDiagnostic {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_text("lifecycle_owner_diagnostic.message", &self.message)?;
        if let Some(hint) = &self.hint {
            validate_text("lifecycle_owner_diagnostic.hint", hint)?;
        }
        Ok(())
    }
}

/// Lifecycle diagnostic severity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerLifecycleOwnerDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Stable lifecycle diagnostic code.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerLifecycleOwnerDiagnosticCode {
    StartupPending,
    ShutdownRequested,
    ShutdownCompleted,
    ComponentPanicObserved,
    ComponentFailed,
    QueueDraining,
    RetryLoopTerminated,
    ReservationCleanupRequired,
    ReservationCleanupCompleted,
    LifecyclePolicyError,
}

/// One component registered under the scheduler lifecycle owner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerLifecycleComponentSnapshot {
    pub component: SchedulerLifecycleComponent,
    pub state: SchedulerLifecycleComponentState,
    pub cancellation: SchedulerLifecycleCancellationState,
    pub panic_state: SchedulerLifecyclePanicState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_bound: Option<SchedulerLifecycleQueueBound>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerLifecycleOwnerDiagnostic>,
}

impl SchedulerLifecycleComponentSnapshot {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        if self.component.requires_bounded_queue() && self.queue_bound.is_none() {
            return Err(SchedulerContractError::MissingField {
                field: "component.queue_bound",
            });
        }
        if let Some(queue_bound) = &self.queue_bound {
            queue_bound.validate()?;
        }
        if self.state == SchedulerLifecycleComponentState::Failed && self.diagnostics.is_empty() {
            return Err(SchedulerContractError::MissingField {
                field: "component.diagnostics",
            });
        }
        if self.panic_state != SchedulerLifecyclePanicState::None && self.diagnostics.is_empty() {
            return Err(SchedulerContractError::MissingField {
                field: "component.diagnostics",
            });
        }
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

/// Scheduler-owned lifecycle snapshot for all long-running scheduler services.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerLifecycleOwnerSnapshot {
    #[serde(default = "default_scheduler_lifecycle_supervision_contract_version")]
    pub contract_version: u16,
    pub owner_id: SchedulerLifecycleOwnerId,
    pub components: Vec<SchedulerLifecycleComponentSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerLifecycleOwnerDiagnostic>,
}

impl SchedulerLifecycleOwnerSnapshot {
    /// Validates one scheduler lifecycle owner before composition roots use it.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        validate_required_components(&self.components)?;
        for component in &self.components {
            component.validate()?;
        }
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

/// Validated lifecycle owner snapshot for composition roots and health checks.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerLifecycleOwnerSnapshot(SchedulerLifecycleOwnerSnapshot);

impl ValidatedSchedulerLifecycleOwnerSnapshot {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerLifecycleOwnerSnapshot {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerLifecycleOwnerSnapshot {
        self.0
    }
}

impl TryFrom<SchedulerLifecycleOwnerSnapshot> for ValidatedSchedulerLifecycleOwnerSnapshot {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerLifecycleOwnerSnapshot) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn validate_required_components(
    components: &[SchedulerLifecycleComponentSnapshot],
) -> Result<(), SchedulerContractError> {
    if components.is_empty() {
        return Err(SchedulerContractError::MissingField {
            field: "components",
        });
    }
    let mut seen = BTreeSet::new();
    for component in components {
        if !seen.insert(component.component) {
            return Err(SchedulerContractError::InvalidField {
                field: "components",
                reason: "scheduler lifecycle components must not be duplicated",
            });
        }
    }
    for required in REQUIRED_COMPONENTS {
        if !seen.contains(required) {
            return Err(SchedulerContractError::MissingField {
                field: required_component_field(*required),
            });
        }
    }
    Ok(())
}

const REQUIRED_COMPONENTS: &[SchedulerLifecycleComponent] = &[
    SchedulerLifecycleComponent::QueueWorker,
    SchedulerLifecycleComponent::DependencyReadinessAction,
    SchedulerLifecycleComponent::ResourceObservationLoop,
    SchedulerLifecycleComponent::RuntimeHostDispatch,
    SchedulerLifecycleComponent::RetryLoop,
    SchedulerLifecycleComponent::ReservationCleanup,
];

fn required_component_field(component: SchedulerLifecycleComponent) -> &'static str {
    match component {
        SchedulerLifecycleComponent::QueueWorker => "components.queue_worker",
        SchedulerLifecycleComponent::DependencyReadinessAction => {
            "components.dependency_readiness_action"
        }
        SchedulerLifecycleComponent::ResourceObservationLoop => {
            "components.resource_observation_loop"
        }
        SchedulerLifecycleComponent::RuntimeHostDispatch => "components.runtime_host_dispatch",
        SchedulerLifecycleComponent::RetryLoop => "components.retry_loop",
        SchedulerLifecycleComponent::ReservationCleanup => "components.reservation_cleanup",
    }
}

fn default_scheduler_lifecycle_supervision_contract_version() -> u16 {
    SCHEDULER_LIFECYCLE_SUPERVISION_CONTRACT_VERSION
}

fn validate_contract_version(value: u16) -> Result<(), SchedulerContractError> {
    if value == SCHEDULER_LIFECYCLE_SUPERVISION_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported scheduler lifecycle supervision contract version",
        })
    }
}

fn validate_bound(field: &'static str, value: u32) -> Result<(), SchedulerContractError> {
    if value == 0 {
        return Err(SchedulerContractError::InvalidField {
            field,
            reason: "lifecycle queue bounds must be greater than zero",
        });
    }
    if value > MAX_QUEUE_BOUND {
        return Err(SchedulerContractError::InvalidField {
            field,
            reason: "lifecycle queue bounds exceed the maximum supported value",
        });
    }
    Ok(())
}

fn validate_identifier(field: &'static str, value: &str) -> Result<String, SchedulerContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SchedulerContractError::MissingField { field });
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(SchedulerContractError::FieldTooLong {
            field,
            max_len: MAX_ID_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(SchedulerContractError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

fn validate_text(field: &'static str, value: &str) -> Result<(), SchedulerContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SchedulerContractError::MissingField { field });
    }
    if trimmed.len() > MAX_TEXT_LEN {
        return Err(SchedulerContractError::FieldTooLong {
            field,
            max_len: MAX_TEXT_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(SchedulerContractError::InvalidText { field });
    }
    Ok(())
}
