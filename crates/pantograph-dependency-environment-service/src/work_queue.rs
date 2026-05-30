use std::collections::VecDeque;
use std::fmt;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use pantograph_dependency_planning::{
    DependencyEnvironmentAction, DependencyEnvironmentRef, DependencyPlanningContractError,
    DependencyPlanningIdentityKey, DependencyRequirementsId, ValidatedDependencyEnvironmentRequest,
};

const MAX_ID_LEN: usize = 128;
const MAX_DIAGNOSTIC_CONTEXT_LEN: usize = 512;

macro_rules! readiness_id {
    ($name:ident, $field:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[must_use]
        pub struct $name(String);

        impl $name {
            pub fn parse(
                value: impl AsRef<str>,
            ) -> Result<Self, DependencyReadinessWorkQueueError> {
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
            type Err = DependencyReadinessWorkQueueError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = DependencyReadinessWorkQueueError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }
    };
}

readiness_id!(
    DependencyReadinessWorkflowSessionId,
    "dependency_readiness_work_item.session_id"
);
readiness_id!(
    DependencyReadinessWorkflowRunId,
    "dependency_readiness_work_item.workflow_run_id"
);
readiness_id!(
    DependencyReadinessTaskId,
    "dependency_readiness_work_item.task_id"
);
readiness_id!(
    DependencyReadinessCancellationScopeId,
    "dependency_readiness_work_item.cancellation_scope_id"
);

/// Backend task provenance for a dependency-readiness work item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyReadinessWorkItemProvenance {
    pub session_id: DependencyReadinessWorkflowSessionId,
    pub workflow_run_id: DependencyReadinessWorkflowRunId,
    pub task_id: DependencyReadinessTaskId,
}

impl DependencyReadinessWorkItemProvenance {
    #[must_use]
    pub fn new(
        session_id: DependencyReadinessWorkflowSessionId,
        workflow_run_id: DependencyReadinessWorkflowRunId,
        task_id: DependencyReadinessTaskId,
    ) -> Self {
        Self {
            session_id,
            workflow_run_id,
            task_id,
        }
    }
}

/// Freshness and retry policy carried with a readiness work item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyReadinessFreshnessPolicy {
    pub deadline_epoch_ms: Option<u64>,
    pub max_attempts: u16,
}

impl DependencyReadinessFreshnessPolicy {
    pub fn new(
        deadline_epoch_ms: Option<u64>,
        max_attempts: u16,
    ) -> Result<Self, DependencyReadinessWorkQueueError> {
        if max_attempts == 0 {
            return Err(DependencyReadinessWorkQueueError::InvalidField {
                field: "dependency_readiness_work_item.max_attempts",
                reason: "max attempts must be greater than zero",
            });
        }
        Ok(Self {
            deadline_epoch_ms,
            max_attempts,
        })
    }
}

impl Default for DependencyReadinessFreshnessPolicy {
    fn default() -> Self {
        Self {
            deadline_epoch_ms: None,
            max_attempts: 3,
        }
    }
}

/// Bounded diagnostic context for queue consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyReadinessDiagnosticContext(String);

impl DependencyReadinessDiagnosticContext {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DependencyReadinessWorkQueueError> {
        let value = value.as_ref();
        if value.is_empty() {
            return Err(DependencyReadinessWorkQueueError::InvalidField {
                field: "dependency_readiness_work_item.diagnostic_context",
                reason: "diagnostic context must not be empty",
            });
        }
        if value.len() > MAX_DIAGNOSTIC_CONTEXT_LEN {
            return Err(DependencyReadinessWorkQueueError::InvalidField {
                field: "dependency_readiness_work_item.diagnostic_context",
                reason: "diagnostic context is too long",
            });
        }
        if value.chars().any(char::is_control) {
            return Err(DependencyReadinessWorkQueueError::InvalidField {
                field: "dependency_readiness_work_item.diagnostic_context",
                reason: "diagnostic context must not contain control characters",
            });
        }
        Ok(Self(value.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Typed backend-owned unit of dependency-readiness producer work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyReadinessWorkItem {
    pub provenance: DependencyReadinessWorkItemProvenance,
    pub request: ValidatedDependencyEnvironmentRequest,
    pub freshness_policy: DependencyReadinessFreshnessPolicy,
    pub cancellation_scope_id: Option<DependencyReadinessCancellationScopeId>,
    pub diagnostic_context: Option<DependencyReadinessDiagnosticContext>,
}

impl DependencyReadinessWorkItem {
    #[must_use]
    pub fn new(
        provenance: DependencyReadinessWorkItemProvenance,
        request: ValidatedDependencyEnvironmentRequest,
    ) -> Self {
        Self {
            provenance,
            request,
            freshness_policy: DependencyReadinessFreshnessPolicy::default(),
            cancellation_scope_id: None,
            diagnostic_context: None,
        }
    }

    #[must_use]
    pub fn with_freshness_policy(
        mut self,
        freshness_policy: DependencyReadinessFreshnessPolicy,
    ) -> Self {
        self.freshness_policy = freshness_policy;
        self
    }

    #[must_use]
    pub fn with_cancellation_scope_id(
        mut self,
        cancellation_scope_id: DependencyReadinessCancellationScopeId,
    ) -> Self {
        self.cancellation_scope_id = Some(cancellation_scope_id);
        self
    }

    #[must_use]
    pub fn with_diagnostic_context(
        mut self,
        diagnostic_context: DependencyReadinessDiagnosticContext,
    ) -> Self {
        self.diagnostic_context = Some(diagnostic_context);
        self
    }

    fn key(&self) -> DependencyReadinessWorkItemKey {
        DependencyReadinessWorkItemKey::from(self)
    }
}

/// Result returned by readiness work queue insertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyReadinessWorkQueueEvent {
    Enqueued,
    Replaced,
}

/// Synchronous, path-free queue for backend-owned dependency-readiness work.
#[derive(Debug, Clone, Default)]
pub struct DependencyReadinessWorkQueue {
    items: Arc<Mutex<VecDeque<DependencyReadinessWorkItem>>>,
}

impl DependencyReadinessWorkQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(&self, item: DependencyReadinessWorkItem) -> DependencyReadinessWorkQueueEvent {
        let key = item.key();
        let mut items = self.items.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(existing) = items.iter_mut().find(|existing| existing.key() == key) {
            *existing = item;
            return DependencyReadinessWorkQueueEvent::Replaced;
        }
        items.push_back(item);
        DependencyReadinessWorkQueueEvent::Enqueued
    }

    #[must_use]
    pub fn pop_next(&self) -> Option<DependencyReadinessWorkItem> {
        self.items
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .pop_front()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Errors returned while constructing dependency-readiness work queue inputs.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DependencyReadinessWorkQueueError {
    #[error("dependency readiness work item field is invalid: {field}: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("dependency readiness work item request is invalid: {0}")]
    InvalidRequest(DependencyPlanningContractError),
}

impl From<DependencyPlanningContractError> for DependencyReadinessWorkQueueError {
    fn from(value: DependencyPlanningContractError) -> Self {
        Self::InvalidRequest(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DependencyReadinessWorkItemKey {
    session_id: DependencyReadinessWorkflowSessionId,
    workflow_run_id: DependencyReadinessWorkflowRunId,
    task_id: DependencyReadinessTaskId,
    action: DependencyEnvironmentAction,
    identity_key: DependencyPlanningIdentityKey,
    dependency_requirements_id: Option<DependencyRequirementsId>,
    environment_ref: Option<DependencyEnvironmentRef>,
}

impl From<&DependencyReadinessWorkItem> for DependencyReadinessWorkItemKey {
    fn from(value: &DependencyReadinessWorkItem) -> Self {
        let request = value.request.as_request();
        Self {
            session_id: value.provenance.session_id.clone(),
            workflow_run_id: value.provenance.workflow_run_id.clone(),
            task_id: value.provenance.task_id.clone(),
            action: request.action,
            identity_key: request.identity_key.clone(),
            dependency_requirements_id: request.dependency_requirements_id.clone(),
            environment_ref: request.environment_ref.clone(),
        }
    }
}

fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, DependencyReadinessWorkQueueError> {
    if value.is_empty() {
        return Err(DependencyReadinessWorkQueueError::InvalidField {
            field,
            reason: "identifier must not be empty",
        });
    }
    if value.len() > MAX_ID_LEN {
        return Err(DependencyReadinessWorkQueueError::InvalidField {
            field,
            reason: "identifier is too long",
        });
    }
    if value.chars().any(char::is_control) {
        return Err(DependencyReadinessWorkQueueError::InvalidField {
            field,
            reason: "identifier must not contain control characters",
        });
    }
    Ok(value.to_string())
}
