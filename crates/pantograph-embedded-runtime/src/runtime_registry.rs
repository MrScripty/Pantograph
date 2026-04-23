//! Backend-owned runtime-registry translation helpers.
//!
//! This module converts gateway lifecycle facts and producer-specific runtime
//! snapshots into `pantograph_runtime_registry::RuntimeObservation` values so
//! host adapters do not own registry-observation mapping logic.

use crate::HostRuntimeModeSnapshot;
use crate::runtime_health::RuntimeHealthAssessment;
pub use crate::runtime_registry_lifecycle::{
    HostRuntimeRegistryController, HostRuntimeRegistryLifecycleController,
    RuntimeWarmupCoordinationError, consume_active_runtime_warmup_disposition,
    reclaim_runtime_and_reconcile_runtime_registry,
    release_reservation_and_reconcile_runtime_registry,
    restore_runtime_and_reconcile_runtime_registry,
    run_runtime_transition_and_reconcile_runtime_registry, runtime_registry_snapshot,
    stop_all_runtime_producers_and_reconcile_runtime_registry, sync_runtime_registry,
    sync_runtime_registry_with_active_health_assessment,
    sync_runtime_registry_with_health_assessments,
};
pub use crate::runtime_registry_observations::{
    ActiveRuntimeDescriptor, active_runtime_descriptor, active_runtime_id,
    active_runtime_observation, active_runtime_observation_with_health_assessment,
    embedding_runtime_id, embedding_runtime_observation,
    embedding_runtime_observation_with_health_assessment, live_host_runtime_producer,
    observations_from_mode_info, observations_from_mode_info_with_active_health_assessment,
    observations_from_mode_info_with_health_assessments, reconcile_active_runtime_mode_info,
    reconcile_runtime_registry_mode_info_with_health_snapshot,
};
use pantograph_runtime_identity::{
    canonical_runtime_id, runtime_backend_key_aliases, runtime_display_name,
};
use pantograph_runtime_registry::{
    RuntimeObservation, RuntimeRegistration, RuntimeRegistry, RuntimeRegistryError,
    RuntimeRegistryRuntimeSnapshot, RuntimeRegistryStatus, RuntimeReservationRequest,
    RuntimeReservationRequirements, RuntimeRetentionHint, observed_runtime_status_from_lifecycle,
};
use pantograph_workflow_service::{
    WorkflowSchedulerRuntimeDiagnosticsRequest, WorkflowSchedulerRuntimeRegistryDiagnostics,
    WorkflowSchedulerRuntimeWarmupDecision, WorkflowSchedulerRuntimeWarmupReason,
    WorkflowSessionRuntimeUnloadCandidate,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostRuntimeProducer {
    Active,
    Embedding,
}

pub fn reconcile_runtime_registry_mode_info(
    registry: &RuntimeRegistry,
    mode_info: &HostRuntimeModeSnapshot,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    reconcile_runtime_registry_mode_info_with_health_assessments(registry, mode_info, None, None)
}

pub fn reconcile_runtime_registry_mode_info_with_active_health_assessment(
    registry: &RuntimeRegistry,
    mode_info: &HostRuntimeModeSnapshot,
    assessment: Option<&RuntimeHealthAssessment>,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    reconcile_runtime_registry_mode_info_with_health_assessments(
        registry, mode_info, assessment, None,
    )
}

pub fn reconcile_runtime_registry_mode_info_with_health_assessments(
    registry: &RuntimeRegistry,
    mode_info: &HostRuntimeModeSnapshot,
    active_assessment: Option<&RuntimeHealthAssessment>,
    embedding_assessment: Option<&RuntimeHealthAssessment>,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    registry.observe_runtimes(observations_from_mode_info_with_health_assessments(
        mode_info,
        active_assessment,
        embedding_assessment,
    ))
}

pub fn register_active_runtime(
    registry: &RuntimeRegistry,
    mode_info: &HostRuntimeModeSnapshot,
) -> ActiveRuntimeDescriptor {
    let descriptor = active_runtime_descriptor(mode_info);
    registry.register_runtime(
        RuntimeRegistration::new(
            descriptor.runtime_id.clone(),
            descriptor.display_name.clone(),
        )
        .with_backend_keys(descriptor.backend_keys.clone()),
    );
    descriptor
}

pub fn active_runtime_reservation_request(
    registry: &RuntimeRegistry,
    mode_info: &HostRuntimeModeSnapshot,
    workflow_id: &str,
    reservation_owner_id: Option<&str>,
    usage_profile: Option<&str>,
    requirements: Option<RuntimeReservationRequirements>,
    retention_hint: RuntimeRetentionHint,
) -> RuntimeReservationRequest {
    let descriptor = register_active_runtime(registry, mode_info);
    RuntimeReservationRequest {
        runtime_id: descriptor.runtime_id,
        workflow_id: workflow_id.to_string(),
        reservation_owner_id: reservation_owner_id.map(ToOwned::to_owned),
        usage_profile: usage_profile.map(ToOwned::to_owned),
        model_id: mode_info.active_model_target.clone(),
        pin_runtime: false,
        requirements,
        retention_hint,
    }
}

pub fn sync_runtime_reservation_retention_hint(
    registry: &RuntimeRegistry,
    reservation_id: u64,
    retention_hint: RuntimeRetentionHint,
) -> Result<(), RuntimeRegistryError> {
    registry
        .update_reservation_retention_hint_if_present(reservation_id, retention_hint)
        .map(|_| ())
}

pub fn scheduler_runtime_registry_diagnostics(
    registry: &RuntimeRegistry,
    mode_info: &HostRuntimeModeSnapshot,
    request: &WorkflowSchedulerRuntimeDiagnosticsRequest,
) -> Result<WorkflowSchedulerRuntimeRegistryDiagnostics, RuntimeRegistryError> {
    let descriptor = register_active_runtime(registry, mode_info);
    let reclaim_candidate =
        runtime_registry_reclaim_candidate_for_sessions(registry, &request.reclaim_candidates);
    let warmup_disposition = if request.next_admission_queue_id.is_some() {
        Some(registry.warmup_disposition(&descriptor.runtime_id)?)
    } else {
        None
    };

    Ok(WorkflowSchedulerRuntimeRegistryDiagnostics {
        target_runtime_id: Some(descriptor.runtime_id),
        reclaim_candidate_session_id: reclaim_candidate
            .as_ref()
            .map(|(session_id, _)| session_id.clone()),
        reclaim_candidate_runtime_id: reclaim_candidate.map(|(_, runtime_id)| runtime_id),
        next_warmup_decision: warmup_disposition
            .as_ref()
            .map(|disposition| workflow_scheduler_runtime_warmup_decision(disposition.decision)),
        next_warmup_reason: warmup_disposition
            .as_ref()
            .map(|disposition| workflow_scheduler_runtime_warmup_reason(disposition.reason)),
    })
}

pub fn runtime_registry_reclaim_candidate_for_sessions(
    runtime_registry: &RuntimeRegistry,
    candidates: &[WorkflowSessionRuntimeUnloadCandidate],
) -> Option<(String, String)> {
    let candidates_by_session_id = candidates
        .iter()
        .map(|candidate| (candidate.session_id.clone(), candidate))
        .collect::<std::collections::HashMap<_, _>>();
    let owner_ids = candidates_by_session_id
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let reservation = runtime_registry.eviction_reservation_candidate_for_owners(&owner_ids)?;
    let owner_id = reservation.reservation_owner_id?;
    let candidate = candidates_by_session_id.get(&owner_id)?;
    Some((candidate.session_id.clone(), reservation.runtime_id))
}

fn workflow_scheduler_runtime_warmup_decision(
    decision: pantograph_runtime_registry::RuntimeWarmupDecision,
) -> WorkflowSchedulerRuntimeWarmupDecision {
    match decision {
        pantograph_runtime_registry::RuntimeWarmupDecision::StartRuntime => {
            WorkflowSchedulerRuntimeWarmupDecision::StartRuntime
        }
        pantograph_runtime_registry::RuntimeWarmupDecision::ReuseLoadedRuntime => {
            WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime
        }
        pantograph_runtime_registry::RuntimeWarmupDecision::WaitForTransition => {
            WorkflowSchedulerRuntimeWarmupDecision::WaitForTransition
        }
    }
}

fn workflow_scheduler_runtime_warmup_reason(
    reason: pantograph_runtime_registry::RuntimeWarmupReason,
) -> WorkflowSchedulerRuntimeWarmupReason {
    match reason {
        pantograph_runtime_registry::RuntimeWarmupReason::NoLoadedInstance => {
            WorkflowSchedulerRuntimeWarmupReason::NoLoadedInstance
        }
        pantograph_runtime_registry::RuntimeWarmupReason::RecoveryRequired => {
            WorkflowSchedulerRuntimeWarmupReason::RecoveryRequired
        }
        pantograph_runtime_registry::RuntimeWarmupReason::LoadedInstanceReady => {
            WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady
        }
        pantograph_runtime_registry::RuntimeWarmupReason::LoadedInstanceBusy => {
            WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceBusy
        }
        pantograph_runtime_registry::RuntimeWarmupReason::WarmupInProgress => {
            WorkflowSchedulerRuntimeWarmupReason::WarmupInProgress
        }
        pantograph_runtime_registry::RuntimeWarmupReason::StopInProgress => {
            WorkflowSchedulerRuntimeWarmupReason::StopInProgress
        }
    }
}

pub fn reconcile_runtime_registry_snapshot_override(
    registry: &RuntimeRegistry,
    snapshot: &inference::RuntimeLifecycleSnapshot,
    model_id: Option<&str>,
) -> Option<RuntimeRegistryRuntimeSnapshot> {
    reconcile_runtime_registry_snapshot_override_with_health_assessment(
        registry, snapshot, model_id, None,
    )
}

pub fn reconcile_runtime_registry_snapshot_override_with_health_assessment(
    registry: &RuntimeRegistry,
    snapshot: &inference::RuntimeLifecycleSnapshot,
    model_id: Option<&str>,
    assessment: Option<&RuntimeHealthAssessment>,
) -> Option<RuntimeRegistryRuntimeSnapshot> {
    let runtime_id = snapshot
        .runtime_id
        .as_deref()
        .map(canonical_runtime_id)
        .filter(|runtime_id| !runtime_id.is_empty())?;
    let display_name = runtime_display_name(&runtime_id)
        .unwrap_or(runtime_id.as_str())
        .to_string();
    let backend_keys = runtime_backend_key_aliases(&display_name, &runtime_id);

    let observation = crate::runtime_registry_observations::observation_with_health_assessment(
        RuntimeObservation {
            runtime_id,
            display_name: display_name.clone(),
            backend_keys,
            model_id: model_id.map(ToOwned::to_owned),
            status: observed_runtime_status_from_lifecycle(
                snapshot.active,
                snapshot.warmup_started_at_ms,
                snapshot.warmup_completed_at_ms,
                snapshot.last_error.is_some(),
            ),
            runtime_instance_id: snapshot.runtime_instance_id.clone(),
            last_error: snapshot.last_error.clone(),
        },
        assessment,
    );

    let observation = preserve_matching_unhealthy_runtime(registry, observation);

    Some(registry.observe_runtime(observation))
}

pub fn reconcile_runtime_registry_stored_projection_overrides(
    registry: &RuntimeRegistry,
    stored_active_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_embedding_runtime_snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    stored_active_model_target: Option<&str>,
    stored_embedding_model_target: Option<&str>,
    gateway_mode_info: &HostRuntimeModeSnapshot,
) {
    reconcile_runtime_registry_snapshot_override_if_not_live_host_runtime(
        registry,
        stored_active_runtime_snapshot,
        stored_active_model_target,
        gateway_mode_info,
    );
    reconcile_runtime_registry_snapshot_override_if_not_live_host_runtime(
        registry,
        stored_embedding_runtime_snapshot,
        stored_embedding_model_target,
        gateway_mode_info,
    );
}

fn reconcile_runtime_registry_snapshot_override_if_not_live_host_runtime(
    registry: &RuntimeRegistry,
    snapshot: Option<&inference::RuntimeLifecycleSnapshot>,
    model_id: Option<&str>,
    gateway_mode_info: &HostRuntimeModeSnapshot,
) {
    let Some(snapshot) = snapshot else {
        return;
    };
    let Some(runtime_id) = snapshot
        .runtime_id
        .as_deref()
        .map(canonical_runtime_id)
        .filter(|runtime_id| !runtime_id.is_empty())
    else {
        return;
    };

    let matches_live_host_runtime = active_runtime_id(gateway_mode_info).as_deref()
        == Some(runtime_id.as_str())
        || embedding_runtime_id(gateway_mode_info).as_deref() == Some(runtime_id.as_str());
    if matches_live_host_runtime {
        return;
    }

    let _ = reconcile_runtime_registry_snapshot_override(registry, snapshot, model_id);
}

fn preserve_matching_unhealthy_runtime(
    registry: &RuntimeRegistry,
    mut observation: RuntimeObservation,
) -> RuntimeObservation {
    if matches!(
        observation.status,
        RuntimeRegistryStatus::Stopped | RuntimeRegistryStatus::Failed
    ) {
        return observation;
    }

    let Some(existing_runtime) = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == observation.runtime_id)
    else {
        return observation;
    };

    if existing_runtime.status != RuntimeRegistryStatus::Unhealthy {
        return observation;
    }

    if existing_runtime.runtime_instance_id != observation.runtime_instance_id {
        return observation;
    }

    observation.status = RuntimeRegistryStatus::Unhealthy;
    if observation.last_error.is_none() {
        observation.last_error = existing_runtime.last_error;
    }

    observation
}

#[cfg(test)]
#[path = "runtime_registry_tests.rs"]
mod tests;
