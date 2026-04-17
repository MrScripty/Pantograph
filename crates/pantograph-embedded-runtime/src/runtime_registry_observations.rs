use crate::runtime_health::{
    RuntimeHealthAssessment, RuntimeHealthAssessmentRecord, RuntimeHealthAssessmentSnapshot,
    RuntimeHealthState,
};
use crate::runtime_registry::HostRuntimeProducer;
use crate::HostRuntimeModeSnapshot;
use pantograph_runtime_identity::canonical_runtime_id;
use pantograph_runtime_registry::{
    observed_runtime_status_from_lifecycle, RuntimeObservation, RuntimeRegistry,
    RuntimeRegistryRuntimeSnapshot,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRuntimeDescriptor {
    pub runtime_id: String,
    pub display_name: String,
    pub backend_keys: Vec<String>,
    pub runtime_instance_id: Option<String>,
}

pub fn active_runtime_descriptor(mode_info: &HostRuntimeModeSnapshot) -> ActiveRuntimeDescriptor {
    let runtime_id = mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_id.clone())
        .or_else(|| mode_info.backend_key.clone())
        .or_else(|| mode_info.backend_name.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let display_name = mode_info
        .backend_name
        .clone()
        .unwrap_or_else(|| runtime_id.clone());
    let backend_keys = mode_info
        .backend_key
        .clone()
        .into_iter()
        .collect::<Vec<_>>();
    let runtime_instance_id = mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_instance_id.clone());

    ActiveRuntimeDescriptor {
        runtime_id,
        display_name,
        backend_keys,
        runtime_instance_id,
    }
}

pub fn active_runtime_id(mode_info: &HostRuntimeModeSnapshot) -> Option<String> {
    mode_info
        .active_runtime
        .as_ref()
        .and_then(|snapshot| snapshot.runtime_id.as_deref())
        .or(mode_info.backend_key.as_deref())
        .or(mode_info.backend_name.as_deref())
        .map(canonical_runtime_id)
}

pub fn embedding_runtime_id(mode_info: &HostRuntimeModeSnapshot) -> Option<String> {
    mode_info.embedding_runtime.as_ref().map(|snapshot| {
        snapshot
            .runtime_id
            .as_deref()
            .map(canonical_runtime_id)
            .unwrap_or_else(|| "llama.cpp.embedding".to_string())
    })
}

pub fn live_host_runtime_producer(
    mode_info: &HostRuntimeModeSnapshot,
    runtime_id: &str,
) -> Option<HostRuntimeProducer> {
    let runtime_id = canonical_runtime_id(runtime_id);

    if mode_info
        .active_runtime
        .as_ref()
        .map(|snapshot| snapshot.active)
        .unwrap_or(false)
        && active_runtime_id(mode_info).as_deref() == Some(runtime_id.as_str())
    {
        return Some(HostRuntimeProducer::Active);
    }

    if mode_info
        .embedding_runtime
        .as_ref()
        .map(|snapshot| snapshot.active)
        .unwrap_or(false)
        && embedding_runtime_id(mode_info).as_deref() == Some(runtime_id.as_str())
    {
        return Some(HostRuntimeProducer::Embedding);
    }

    None
}

pub fn active_runtime_observation(
    mode_info: &HostRuntimeModeSnapshot,
    include_stopped: bool,
) -> Option<RuntimeObservation> {
    let snapshot = mode_info
        .active_runtime
        .as_ref()
        .cloned()
        .unwrap_or_default();
    let descriptor = active_runtime_descriptor(mode_info);
    let status = observed_runtime_status_from_lifecycle(
        snapshot.active,
        snapshot.warmup_started_at_ms,
        snapshot.warmup_completed_at_ms,
        snapshot.last_error.is_some(),
    );

    if !include_stopped
        && matches!(
            status,
            pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
        )
        && snapshot.last_error.is_none()
    {
        return None;
    }

    Some(RuntimeObservation {
        runtime_id: descriptor.runtime_id,
        display_name: descriptor.display_name,
        backend_keys: descriptor.backend_keys,
        model_id: mode_info.active_model_target.clone(),
        status,
        runtime_instance_id: snapshot.runtime_instance_id,
        last_error: snapshot.last_error,
    })
}

pub fn active_runtime_observation_with_health_assessment(
    mode_info: &HostRuntimeModeSnapshot,
    include_stopped: bool,
    assessment: Option<&RuntimeHealthAssessment>,
) -> Option<RuntimeObservation> {
    active_runtime_observation(mode_info, include_stopped)
        .map(|observation| observation_with_health_assessment(observation, assessment))
}

pub fn embedding_runtime_observation(
    mode_info: &HostRuntimeModeSnapshot,
) -> Option<RuntimeObservation> {
    let snapshot = mode_info.embedding_runtime.as_ref()?.clone();
    let runtime_id = snapshot
        .runtime_id
        .clone()
        .unwrap_or_else(|| "llama.cpp.embedding".to_string());

    Some(RuntimeObservation {
        runtime_id,
        display_name: "Dedicated embedding runtime".to_string(),
        backend_keys: mode_info.backend_key.clone().into_iter().collect(),
        model_id: mode_info.embedding_model_target.clone(),
        status: observed_runtime_status_from_lifecycle(
            snapshot.active,
            snapshot.warmup_started_at_ms,
            snapshot.warmup_completed_at_ms,
            snapshot.last_error.is_some(),
        ),
        runtime_instance_id: snapshot.runtime_instance_id,
        last_error: snapshot.last_error,
    })
}

pub fn embedding_runtime_observation_with_health_assessment(
    mode_info: &HostRuntimeModeSnapshot,
    assessment: Option<&RuntimeHealthAssessment>,
) -> Option<RuntimeObservation> {
    embedding_runtime_observation(mode_info)
        .map(|observation| observation_with_health_assessment(observation, assessment))
}

pub fn observations_from_mode_info(mode_info: &HostRuntimeModeSnapshot) -> Vec<RuntimeObservation> {
    observations_from_mode_info_with_health_assessments(mode_info, None, None)
}

pub fn observations_from_mode_info_with_active_health_assessment(
    mode_info: &HostRuntimeModeSnapshot,
    assessment: Option<&RuntimeHealthAssessment>,
) -> Vec<RuntimeObservation> {
    observations_from_mode_info_with_health_assessments(mode_info, assessment, None)
}

pub fn observations_from_mode_info_with_health_assessments(
    mode_info: &HostRuntimeModeSnapshot,
    active_assessment: Option<&RuntimeHealthAssessment>,
    embedding_assessment: Option<&RuntimeHealthAssessment>,
) -> Vec<RuntimeObservation> {
    let mut observations = Vec::new();

    if let Some(observation) =
        active_runtime_observation_with_health_assessment(mode_info, true, active_assessment)
    {
        observations.push(observation);
    }

    if let Some(observation) =
        embedding_runtime_observation_with_health_assessment(mode_info, embedding_assessment)
    {
        observations.push(observation);
    }

    observations
}

pub fn reconcile_runtime_registry_mode_info_with_health_snapshot(
    registry: &RuntimeRegistry,
    mode_info: &HostRuntimeModeSnapshot,
    health_assessments: &RuntimeHealthAssessmentSnapshot,
) -> Vec<RuntimeRegistryRuntimeSnapshot> {
    super::runtime_registry::reconcile_runtime_registry_mode_info_with_health_assessments(
        registry,
        mode_info,
        matched_runtime_health_assessment(
            active_runtime_observation(mode_info, true).as_ref(),
            health_assessments.active.as_ref(),
        ),
        matched_runtime_health_assessment(
            embedding_runtime_observation(mode_info).as_ref(),
            health_assessments.embedding.as_ref(),
        ),
    )
}

pub fn reconcile_active_runtime_mode_info(
    registry: &RuntimeRegistry,
    mode_info: &HostRuntimeModeSnapshot,
    include_stopped: bool,
) -> Option<RuntimeRegistryRuntimeSnapshot> {
    active_runtime_observation(mode_info, include_stopped)
        .map(|observation| registry.observe_runtime(observation))
}

fn observation_with_health_assessment(
    mut observation: RuntimeObservation,
    assessment: Option<&RuntimeHealthAssessment>,
) -> RuntimeObservation {
    if let Some(RuntimeHealthAssessment {
        state: RuntimeHealthState::Unhealthy { reason },
        error,
        ..
    }) = assessment
    {
        observation.status = pantograph_runtime_registry::RuntimeRegistryStatus::Unhealthy;
        observation.last_error = error.clone().or_else(|| Some(reason.clone()));
    }

    observation
}

fn matched_runtime_health_assessment<'a>(
    observation: Option<&RuntimeObservation>,
    record: Option<&'a RuntimeHealthAssessmentRecord>,
) -> Option<&'a RuntimeHealthAssessment> {
    let observation = observation?;
    let record = record?;

    if observation.runtime_id != record.runtime_id {
        return None;
    }

    if observation.runtime_instance_id != record.runtime_instance_id {
        return None;
    }

    Some(&record.assessment)
}
