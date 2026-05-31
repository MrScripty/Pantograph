use std::sync::Arc;

use async_trait::async_trait;
use pantograph_runtime_host_contracts::{
    ReservationLifecycleApplication, ReservationLifecycleApplicationState,
    ReservationLifecycleContractError, ReservationLifecycleDiagnostic,
    ReservationLifecycleDiagnosticCode, ReservationLifecycleDiagnosticSeverity,
    ReservationLifecycleEvent, ReservationLifecycleOutcome, ReservationLifecyclePort,
    ReservationLifecyclePortError, ValidatedReservationLifecycleEvent,
    RESERVATION_LIFECYCLE_CONTRACT_VERSION,
};
use pantograph_runtime_registry::{RuntimeRegistryError, SharedRuntimeRegistry};

use crate::runtime_registry::{
    release_reservation_and_reconcile_runtime_registry, HostRuntimeRegistryController,
};

const RUNTIME_REGISTRY_LEASE_PREFIX: &str = "runtime-registry.";

#[derive(Clone)]
pub(crate) struct EmbeddedReservationLifecyclePort<C> {
    registry: SharedRuntimeRegistry,
    controller: Arc<C>,
}

impl<C> EmbeddedReservationLifecyclePort<C> {
    pub(crate) fn new(registry: SharedRuntimeRegistry, controller: Arc<C>) -> Self {
        Self {
            registry,
            controller,
        }
    }
}

#[async_trait]
impl<C> ReservationLifecyclePort for EmbeddedReservationLifecyclePort<C>
where
    C: HostRuntimeRegistryController + Send + Sync,
{
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        let event = ValidatedReservationLifecycleEvent::try_from(event)
            .map_err(port_error_from_contract)?;
        let event = event.into_inner();
        let reservation_id = match runtime_registry_reservation_id(&event) {
            Ok(reservation_id) => reservation_id,
            Err(application) => return Ok(application),
        };

        match event.outcome {
            ReservationLifecycleOutcome::DispatchStarted => Ok(application(
                &event,
                ReservationLifecycleApplicationState::Applied,
                Vec::new(),
            )),
            ReservationLifecycleOutcome::DuplicateReplay => Ok(application(
                &event,
                ReservationLifecycleApplicationState::AlreadyApplied,
                Vec::new(),
            )),
            ReservationLifecycleOutcome::CandidateUnselected
            | ReservationLifecycleOutcome::CandidateRequestRejected
            | ReservationLifecycleOutcome::RuntimeHostDispatchRejected
            | ReservationLifecycleOutcome::RuntimeHostCompleted
            | ReservationLifecycleOutcome::RuntimeHostFailed
            | ReservationLifecycleOutcome::WorkflowCancelled
            | ReservationLifecycleOutcome::RetryDeferred
            | ReservationLifecycleOutcome::SessionClosed => {
                release_reservation(&event, reservation_id, &self.registry, &self.controller).await
            }
            _ => Ok(application(
                &event,
                ReservationLifecycleApplicationState::Failed,
                vec![diagnostic(
                    ReservationLifecycleDiagnosticSeverity::Error,
                    ReservationLifecycleDiagnosticCode::RequestRejected,
                    "unsupported reservation lifecycle outcome",
                )],
            )),
        }
    }
}

async fn release_reservation<C>(
    event: &ReservationLifecycleEvent,
    reservation_id: u64,
    registry: &SharedRuntimeRegistry,
    controller: &Arc<C>,
) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError>
where
    C: HostRuntimeRegistryController + Send + Sync,
{
    release_reservation_and_reconcile_runtime_registry(
        controller.as_ref(),
        registry,
        reservation_id,
    )
    .await
    .map(|disposition| match disposition {
        Some(_) => application(
            event,
            ReservationLifecycleApplicationState::Applied,
            Vec::new(),
        ),
        None => application(
            event,
            ReservationLifecycleApplicationState::AlreadyApplied,
            vec![diagnostic(
                ReservationLifecycleDiagnosticSeverity::Warning,
                ReservationLifecycleDiagnosticCode::LeaseNotFound,
                "runtime-registry reservation lease was already released or is unknown",
            )],
        ),
    })
    .map_err(|error| ReservationLifecyclePortError::Failed {
        message: registry_error_message(event, error),
    })
}

fn runtime_registry_reservation_id(
    event: &ReservationLifecycleEvent,
) -> Result<u64, ReservationLifecycleApplication> {
    let lease_id = event.reservation_lease_id.as_str();
    let Some(raw_id) = lease_id.strip_prefix(RUNTIME_REGISTRY_LEASE_PREFIX) else {
        return Err(application(
            event,
            ReservationLifecycleApplicationState::Failed,
            vec![diagnostic(
                ReservationLifecycleDiagnosticSeverity::Error,
                ReservationLifecycleDiagnosticCode::LeaseOwnerMismatch,
                "reservation lease id was not issued by runtime-registry resource facts",
            )],
        ));
    };
    raw_id.parse::<u64>().map_err(|_| {
        application(
            event,
            ReservationLifecycleApplicationState::Failed,
            vec![diagnostic(
                ReservationLifecycleDiagnosticSeverity::Error,
                ReservationLifecycleDiagnosticCode::LeaseOwnerMismatch,
                "runtime-registry reservation lease id has an invalid numeric suffix",
            )],
        )
    })
}

fn application(
    event: &ReservationLifecycleEvent,
    state: ReservationLifecycleApplicationState,
    diagnostics: Vec<ReservationLifecycleDiagnostic>,
) -> ReservationLifecycleApplication {
    ReservationLifecycleApplication {
        contract_version: RESERVATION_LIFECYCLE_CONTRACT_VERSION,
        lifecycle_event_id: event.lifecycle_event_id.clone(),
        reservation_lease_id: event.reservation_lease_id.clone(),
        state,
        diagnostics,
    }
}

fn diagnostic(
    severity: ReservationLifecycleDiagnosticSeverity,
    code: ReservationLifecycleDiagnosticCode,
    message: impl Into<String>,
) -> ReservationLifecycleDiagnostic {
    ReservationLifecycleDiagnostic {
        severity,
        code,
        message: message.into(),
        hint: None,
    }
}

fn port_error_from_contract(
    error: ReservationLifecycleContractError,
) -> ReservationLifecyclePortError {
    ReservationLifecyclePortError::Failed {
        message: format!("reservation lifecycle event contract rejected: {error}"),
    }
}

fn registry_error_message(
    event: &ReservationLifecycleEvent,
    error: RuntimeRegistryError,
) -> String {
    format!(
        "runtime-registry reservation lifecycle failed for lease '{}' and task '{}': {error}",
        event.reservation_lease_id.as_str(),
        event.task_id.as_str()
    )
}

#[cfg(test)]
#[path = "reservation_lifecycle_tests.rs"]
mod tests;
