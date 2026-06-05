// The first scheduler lifecycle diagnostics slice lands the typed synchronous
// registry before public snapshots, ledger events, and concrete component
// owners consume it in later slices.
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::BTreeMap;

use crate::workflow::WorkflowServiceError;

/// Workflow-service owned scheduler lifecycle component registry.
///
/// This registry is the synchronous state owner for scheduler lifecycle
/// component presence and coarse state. It intentionally does not expose a
/// public diagnostics snapshot or write ledger events; later slices must first
/// attach real component owners to these typed records.
#[derive(Debug, Clone)]
pub(crate) struct WorkflowSchedulerLifecycleComponentRegistry {
    owner_id: WorkflowSchedulerLifecycleOwnerId,
    components: BTreeMap<
        WorkflowSchedulerLifecycleComponentKind,
        WorkflowSchedulerLifecycleComponentRecord,
    >,
}

impl WorkflowSchedulerLifecycleComponentRegistry {
    pub(crate) fn new(owner_id: WorkflowSchedulerLifecycleOwnerId) -> Self {
        let components = WorkflowSchedulerLifecycleComponentKind::required_components()
            .iter()
            .copied()
            .map(|component| {
                (
                    component,
                    WorkflowSchedulerLifecycleComponentRecord {
                        owner_id: owner_id.clone(),
                        component,
                        state: WorkflowSchedulerLifecycleComponentState::NotStarted,
                    },
                )
            })
            .collect();

        Self {
            owner_id,
            components,
        }
    }

    pub(crate) fn owner_id(&self) -> &WorkflowSchedulerLifecycleOwnerId {
        &self.owner_id
    }

    pub(crate) fn component(
        &self,
        component: WorkflowSchedulerLifecycleComponentKind,
    ) -> Result<&WorkflowSchedulerLifecycleComponentRecord, WorkflowServiceError> {
        self.components.get(&component).ok_or_else(|| {
            lifecycle_error(WorkflowSchedulerLifecycleDiagnostic::error(
                WorkflowSchedulerLifecycleDiagnosticCode::RequiredComponentMissing,
                format!(
                    "scheduler lifecycle component '{}' is not owned by registry '{}'",
                    component.as_str(),
                    self.owner_id.as_str()
                ),
                Some(
                    "Register every required lifecycle component before projecting diagnostics."
                        .to_string(),
                ),
            ))
        })
    }

    pub(crate) fn update_component_state(
        &mut self,
        component: WorkflowSchedulerLifecycleComponentKind,
        state: WorkflowSchedulerLifecycleComponentState,
    ) -> Result<WorkflowSchedulerLifecycleComponentRecord, WorkflowServiceError> {
        let owner_id = self.owner_id.clone();
        let record = self.components.get_mut(&component).ok_or_else(|| {
            lifecycle_error(WorkflowSchedulerLifecycleDiagnostic::error(
                WorkflowSchedulerLifecycleDiagnosticCode::RequiredComponentMissing,
                format!(
                    "scheduler lifecycle component '{}' is not owned by registry '{}'",
                    component.as_str(),
                    owner_id.as_str()
                ),
                Some(
                    "Update only components owned by the workflow-service lifecycle registry."
                        .to_string(),
                ),
            ))
        })?;
        record.state = state;
        Ok(record.clone())
    }

    pub(crate) fn required_component_records(
        &self,
    ) -> Result<Vec<WorkflowSchedulerLifecycleComponentRecord>, WorkflowServiceError> {
        WorkflowSchedulerLifecycleComponentKind::required_components()
            .iter()
            .copied()
            .map(|component| self.component(component).cloned())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct WorkflowSchedulerLifecycleOwnerId(String);

impl WorkflowSchedulerLifecycleOwnerId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, WorkflowServiceError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(lifecycle_error(
                WorkflowSchedulerLifecycleDiagnostic::error(
                    WorkflowSchedulerLifecycleDiagnosticCode::InvalidLifecycleOwnerId,
                    "scheduler lifecycle owner id must not be blank",
                    Some(
                        "Use the workflow-service scheduler lifecycle owner identity.".to_string(),
                    ),
                ),
            ));
        }

        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum WorkflowSchedulerLifecycleComponentKind {
    QueueWorker,
    DependencyReadinessAction,
    ResourceObservationLoop,
    RuntimeHostDispatch,
    RetryLoop,
    ReservationCleanup,
}

impl WorkflowSchedulerLifecycleComponentKind {
    pub(crate) fn required_components() -> &'static [Self] {
        &[
            Self::QueueWorker,
            Self::DependencyReadinessAction,
            Self::ResourceObservationLoop,
            Self::RuntimeHostDispatch,
            Self::RetryLoop,
            Self::ReservationCleanup,
        ]
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::QueueWorker => "queue_worker",
            Self::DependencyReadinessAction => "dependency_readiness_action",
            Self::ResourceObservationLoop => "resource_observation_loop",
            Self::RuntimeHostDispatch => "runtime_host_dispatch",
            Self::RetryLoop => "retry_loop",
            Self::ReservationCleanup => "reservation_cleanup",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowSchedulerLifecycleComponentState {
    NotStarted,
    Running,
    ShuttingDown,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowSchedulerLifecycleComponentRecord {
    pub(crate) owner_id: WorkflowSchedulerLifecycleOwnerId,
    pub(crate) component: WorkflowSchedulerLifecycleComponentKind,
    pub(crate) state: WorkflowSchedulerLifecycleComponentState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowSchedulerLifecycleDiagnosticSeverity {
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowSchedulerLifecycleDiagnosticCode {
    InvalidLifecycleOwnerId,
    RequiredComponentMissing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowSchedulerLifecycleDiagnostic {
    pub(crate) severity: WorkflowSchedulerLifecycleDiagnosticSeverity,
    pub(crate) code: WorkflowSchedulerLifecycleDiagnosticCode,
    pub(crate) message: String,
    pub(crate) hint: Option<String>,
}

impl WorkflowSchedulerLifecycleDiagnostic {
    fn error(
        code: WorkflowSchedulerLifecycleDiagnosticCode,
        message: impl Into<String>,
        hint: Option<String>,
    ) -> Self {
        Self {
            severity: WorkflowSchedulerLifecycleDiagnosticSeverity::Error,
            code,
            message: message.into(),
            hint,
        }
    }
}

fn lifecycle_error(diagnostic: WorkflowSchedulerLifecycleDiagnostic) -> WorkflowServiceError {
    WorkflowServiceError::InvalidRequest(format!(
        "scheduler lifecycle error: {:?}: {}{}",
        diagnostic.code,
        diagnostic.message,
        diagnostic
            .hint
            .as_ref()
            .map(|hint| format!(" Hint: {hint}"))
            .unwrap_or_default()
    ))
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
