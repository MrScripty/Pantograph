use serde::{Deserialize, Serialize};

use pantograph_scheduler::{SchedulerResourceFitState, SchedulerResourceKind};

use super::runtime_dispatch_selection::WorkflowRuntimeDispatchCandidateFact;
use crate::graph::WorkflowRuntimeSourceContext;

pub(super) const WORKFLOW_RUNTIME_TASK_ATTEMPT_FACT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[must_use]
pub(super) struct WorkflowRuntimeTaskAttemptFactRequest {
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) scheduler_task_id: String,
    pub(super) scheduler_task_attempt_id: String,
    pub(super) task_attempt_generation: u64,
    pub(super) selected_model_id: String,
    pub(super) selected_artifact_id: String,
    pub(super) selected_runtime_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) selected_runtime_variant_id: Option<String>,
    pub(super) backend_id: String,
    pub(super) runtime_family: String,
    pub(super) load_target: String,
    pub(super) runtime_residency_key: String,
    pub(super) loaded_runtime_memory_estimate_bytes: u64,
    pub(super) resource_fit: WorkflowRuntimeTaskAttemptResourceFitFacts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) reservations: Vec<WorkflowRuntimeTaskAttemptReservationFact>,
    pub(super) operation_type: String,
    pub(super) context_shape_key: String,
    pub(super) cancellation_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) timeout_ms: Option<u64>,
    pub(super) recorded_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeTaskAttemptSourceContextRequest {
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) scheduler_task_id: String,
    pub(super) task_attempt_generation: u64,
    pub(super) timeout_ms: Option<u64>,
    pub(super) runtime_source_context: WorkflowRuntimeSourceContext,
    pub(super) selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeTaskAttemptSourceContext {
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) scheduler_task_id: String,
    pub(super) task_attempt_generation: u64,
    pub(super) timeout_ms: Option<u64>,
    pub(super) runtime_source_context: WorkflowRuntimeSourceContext,
    pub(super) selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeTaskAttemptFactBuildRequest {
    pub(super) source_context: WorkflowRuntimeTaskAttemptSourceContext,
    pub(super) scheduler_task_attempt_id: String,
    pub(super) scheduler_task_attempt_started_at_ms: u64,
    pub(super) recorded_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[must_use]
pub(super) struct WorkflowRuntimeTaskAttemptFactRecord {
    pub(super) schema_version: u16,
    pub(super) workflow_id: String,
    pub(super) workflow_run_id: String,
    pub(super) scheduler_task_id: String,
    pub(super) scheduler_task_attempt_id: String,
    pub(super) task_attempt_generation: u64,
    pub(super) selected_model_id: String,
    pub(super) selected_artifact_id: String,
    pub(super) selected_runtime_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) selected_runtime_variant_id: Option<String>,
    pub(super) backend_id: String,
    pub(super) runtime_family: String,
    pub(super) load_target: String,
    pub(super) runtime_residency_key: String,
    pub(super) loaded_runtime_memory_estimate_bytes: u64,
    pub(super) resource_fit: WorkflowRuntimeTaskAttemptResourceFitFacts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) reservations: Vec<WorkflowRuntimeTaskAttemptReservationFact>,
    pub(super) operation_type: String,
    pub(super) context_shape_key: String,
    pub(super) cancellation_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) timeout_ms: Option<u64>,
    pub(super) recorded_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[must_use]
pub(super) struct WorkflowRuntimeTaskAttemptResourceFitFacts {
    pub(super) state: WorkflowRuntimeTaskAttemptResourceFitState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) diagnostic_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[must_use]
pub(super) enum WorkflowRuntimeTaskAttemptResourceFitState {
    Fits,
    WaitingForResources,
    ImpossibleFit,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[must_use]
pub(super) struct WorkflowRuntimeTaskAttemptReservationFact {
    pub(super) reservation_lease_id: String,
    pub(super) device_id: String,
    pub(super) resource_kind: WorkflowRuntimeTaskAttemptResourceKind,
    pub(super) reserved_bytes: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[must_use]
pub(super) enum WorkflowRuntimeTaskAttemptResourceKind {
    SystemRam,
    SystemSwap,
    DeviceVram,
    DeviceSharedMemory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowRuntimeTaskAttemptFactDiagnostic {
    pub(super) code: WorkflowRuntimeTaskAttemptFactDiagnosticCode,
    pub(super) field_path: String,
    pub(super) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowRuntimeTaskAttemptFactDiagnosticCode {
    MissingSelectedFact,
    InvalidAttemptIdentity,
    InvalidSourceContext,
    InvalidMemoryEstimate,
    InvalidReservationFact,
    InvalidResourceFitFact,
    InvalidTimeoutPolicy,
}

impl WorkflowRuntimeTaskAttemptSourceContext {
    pub(super) fn new(
        request: WorkflowRuntimeTaskAttemptSourceContextRequest,
    ) -> Result<Self, WorkflowRuntimeTaskAttemptFactDiagnostic> {
        validate_source_context_request(&request)?;
        Ok(Self {
            workflow_id: request.workflow_id,
            workflow_run_id: request.workflow_run_id,
            scheduler_task_id: request.scheduler_task_id,
            task_attempt_generation: request.task_attempt_generation,
            timeout_ms: request.timeout_ms,
            runtime_source_context: request.runtime_source_context,
            selected_candidate_fact: request.selected_candidate_fact,
        })
    }
}

impl WorkflowRuntimeTaskAttemptFactRecord {
    pub(super) fn new(
        request: WorkflowRuntimeTaskAttemptFactRequest,
    ) -> Result<Self, WorkflowRuntimeTaskAttemptFactDiagnostic> {
        validate_request(&request)?;
        Ok(Self {
            schema_version: WORKFLOW_RUNTIME_TASK_ATTEMPT_FACT_SCHEMA_VERSION,
            workflow_id: request.workflow_id,
            workflow_run_id: request.workflow_run_id,
            scheduler_task_id: request.scheduler_task_id,
            scheduler_task_attempt_id: request.scheduler_task_attempt_id,
            task_attempt_generation: request.task_attempt_generation,
            selected_model_id: request.selected_model_id,
            selected_artifact_id: request.selected_artifact_id,
            selected_runtime_id: request.selected_runtime_id,
            selected_runtime_variant_id: request.selected_runtime_variant_id,
            backend_id: request.backend_id,
            runtime_family: request.runtime_family,
            load_target: request.load_target,
            runtime_residency_key: request.runtime_residency_key,
            loaded_runtime_memory_estimate_bytes: request.loaded_runtime_memory_estimate_bytes,
            resource_fit: request.resource_fit,
            reservations: request.reservations,
            operation_type: request.operation_type,
            context_shape_key: request.context_shape_key,
            cancellation_mode: request.cancellation_mode,
            timeout_ms: request.timeout_ms,
            recorded_at_ms: request.recorded_at_ms,
        })
    }

    pub(super) fn from_source_context(
        request: WorkflowRuntimeTaskAttemptFactBuildRequest,
    ) -> Result<Self, WorkflowRuntimeTaskAttemptFactDiagnostic> {
        validate_non_blank(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
            "scheduler_task_attempt_id",
            &request.scheduler_task_attempt_id,
        )?;
        if request.scheduler_task_attempt_started_at_ms == 0 {
            return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
                WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
                "scheduler_task_attempt_started_at_ms",
                "runtime task-attempt scheduler start timestamp must be greater than zero",
            ));
        }

        let source_context = request.source_context;
        let selected_candidate_fact = source_context.selected_candidate_fact;
        let selected_artifact_id = selected_candidate_fact
            .selected_model_ref
            .selected_artifact_id
            .clone()
            .ok_or_else(|| {
                WorkflowRuntimeTaskAttemptFactDiagnostic::new(
                    WorkflowRuntimeTaskAttemptFactDiagnosticCode::MissingSelectedFact,
                    "selected_candidate_fact.selected_model_ref.selected_artifact_id",
                    "selected candidate fact must carry selected model artifact id",
                )
            })?;
        let reservations = selected_candidate_fact
            .reservations
            .iter()
            .enumerate()
            .map(|(index, reservation)| {
                Ok(WorkflowRuntimeTaskAttemptReservationFact {
                    reservation_lease_id: reservation.reservation_lease_id.as_str().to_string(),
                    device_id: reservation.device_id.as_str().to_string(),
                    resource_kind: task_attempt_resource_kind(index, &reservation.resource_kind)?,
                    reserved_bytes: reservation.reserved_bytes,
                })
            })
            .collect::<Result<Vec<_>, WorkflowRuntimeTaskAttemptFactDiagnostic>>()?;

        Self::new(WorkflowRuntimeTaskAttemptFactRequest {
            workflow_id: source_context.workflow_id,
            workflow_run_id: source_context.workflow_run_id,
            scheduler_task_id: source_context.scheduler_task_id,
            scheduler_task_attempt_id: request.scheduler_task_attempt_id,
            task_attempt_generation: source_context.task_attempt_generation,
            selected_model_id: selected_candidate_fact.selected_model_ref.model_id,
            selected_artifact_id,
            selected_runtime_id: selected_candidate_fact
                .selected_runtime_id
                .as_str()
                .to_string(),
            selected_runtime_variant_id: selected_candidate_fact
                .selected_runtime_variant_id
                .as_ref()
                .map(|variant_id| variant_id.as_str().to_string()),
            backend_id: selected_candidate_fact.selected_backend_key,
            runtime_family: selected_candidate_fact.runtime_family,
            load_target: selected_candidate_fact.resolved_load_target,
            runtime_residency_key: selected_candidate_fact.runtime_residency_key,
            loaded_runtime_memory_estimate_bytes: selected_candidate_fact
                .loaded_runtime_memory_estimate_bytes,
            resource_fit: task_attempt_resource_fit(
                selected_candidate_fact.resource_fit_assessment.state,
                &selected_candidate_fact.resource_fit_assessment.diagnostics,
            )?,
            reservations,
            operation_type: source_context.runtime_source_context.operation_type,
            context_shape_key: source_context.runtime_source_context.context_shape_key,
            cancellation_mode: source_context.runtime_source_context.cancellation_mode,
            timeout_ms: source_context.timeout_ms,
            recorded_at_ms: request.recorded_at_ms,
        })
    }
}

fn task_attempt_resource_fit(
    state: SchedulerResourceFitState,
    diagnostics: &[pantograph_scheduler::SchedulerResourceDiagnostic],
) -> Result<WorkflowRuntimeTaskAttemptResourceFitFacts, WorkflowRuntimeTaskAttemptFactDiagnostic> {
    Ok(WorkflowRuntimeTaskAttemptResourceFitFacts {
        state: match state {
            SchedulerResourceFitState::Fits => WorkflowRuntimeTaskAttemptResourceFitState::Fits,
            SchedulerResourceFitState::WaitingForResources => {
                WorkflowRuntimeTaskAttemptResourceFitState::WaitingForResources
            }
            SchedulerResourceFitState::ImpossibleFit => {
                WorkflowRuntimeTaskAttemptResourceFitState::ImpossibleFit
            }
            SchedulerResourceFitState::Unknown => {
                WorkflowRuntimeTaskAttemptResourceFitState::Unknown
            }
            _ => {
                return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
                    WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidResourceFitFact,
                    "resource_fit.state",
                    "runtime task-attempt resource-fit state is not supported by the fact contract",
                ));
            }
        },
        diagnostic_codes: diagnostics
            .iter()
            .map(|diagnostic| format!("{:?}", diagnostic.code))
            .collect(),
    })
}

fn task_attempt_resource_kind(
    index: usize,
    resource_kind: &SchedulerResourceKind,
) -> Result<WorkflowRuntimeTaskAttemptResourceKind, WorkflowRuntimeTaskAttemptFactDiagnostic> {
    match resource_kind {
        SchedulerResourceKind::SystemRam => Ok(WorkflowRuntimeTaskAttemptResourceKind::SystemRam),
        SchedulerResourceKind::SystemSwap => Ok(WorkflowRuntimeTaskAttemptResourceKind::SystemSwap),
        SchedulerResourceKind::DeviceVram => Ok(WorkflowRuntimeTaskAttemptResourceKind::DeviceVram),
        SchedulerResourceKind::DeviceSharedMemory => {
            Ok(WorkflowRuntimeTaskAttemptResourceKind::DeviceSharedMemory)
        }
        _ => Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidReservationFact,
            format!("reservations[{index}].resource_kind"),
            "runtime task-attempt reservation resource kind is not supported by the fact contract",
        )),
    }
}

impl WorkflowRuntimeTaskAttemptFactDiagnostic {
    fn new(
        code: WorkflowRuntimeTaskAttemptFactDiagnosticCode,
        field_path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            field_path: field_path.into(),
            message: message.into(),
        }
    }
}

fn validate_source_context_request(
    request: &WorkflowRuntimeTaskAttemptSourceContextRequest,
) -> Result<(), WorkflowRuntimeTaskAttemptFactDiagnostic> {
    validate_non_blank(
        WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
        "workflow_id",
        &request.workflow_id,
    )?;
    validate_non_blank(
        WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
        "workflow_run_id",
        &request.workflow_run_id,
    )?;
    validate_non_blank(
        WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
        "scheduler_task_id",
        &request.scheduler_task_id,
    )?;
    if request.task_attempt_generation == 0 {
        return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
            "task_attempt_generation",
            "runtime task-attempt generation must be greater than zero",
        ));
    }
    if request.timeout_ms == Some(0) {
        return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidTimeoutPolicy,
            "timeout_ms",
            "runtime task-attempt timeout must be greater than zero when present",
        ));
    }
    validate_runtime_source_context(&request.runtime_source_context)?;
    validate_selected_candidate_fact(&request.selected_candidate_fact)?;
    Ok(())
}

fn validate_request(
    request: &WorkflowRuntimeTaskAttemptFactRequest,
) -> Result<(), WorkflowRuntimeTaskAttemptFactDiagnostic> {
    validate_non_blank(
        WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
        "workflow_id",
        &request.workflow_id,
    )?;
    validate_non_blank(
        WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
        "workflow_run_id",
        &request.workflow_run_id,
    )?;
    validate_non_blank(
        WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
        "scheduler_task_id",
        &request.scheduler_task_id,
    )?;
    validate_non_blank(
        WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
        "scheduler_task_attempt_id",
        &request.scheduler_task_attempt_id,
    )?;
    if request.task_attempt_generation == 0 {
        return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
            "task_attempt_generation",
            "runtime task-attempt generation must be greater than zero",
        ));
    }

    validate_required_selected_fact("selected_model_id", &request.selected_model_id)?;
    validate_required_selected_fact("selected_artifact_id", &request.selected_artifact_id)?;
    validate_required_selected_fact("selected_runtime_id", &request.selected_runtime_id)?;
    if let Some(selected_runtime_variant_id) = &request.selected_runtime_variant_id {
        validate_required_selected_fact(
            "selected_runtime_variant_id",
            selected_runtime_variant_id,
        )?;
    }
    validate_required_selected_fact("backend_id", &request.backend_id)?;
    validate_required_selected_fact("runtime_family", &request.runtime_family)?;
    validate_required_selected_fact("load_target", &request.load_target)?;
    validate_required_selected_fact("runtime_residency_key", &request.runtime_residency_key)?;
    if request.loaded_runtime_memory_estimate_bytes == 0 {
        return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidMemoryEstimate,
            "loaded_runtime_memory_estimate_bytes",
            "loaded-runtime memory estimate must be greater than zero",
        ));
    }
    validate_resource_fit(&request.resource_fit)?;
    for (index, reservation) in request.reservations.iter().enumerate() {
        validate_reservation(index, reservation)?;
    }
    validate_required_selected_fact("operation_type", &request.operation_type)?;
    validate_required_selected_fact("context_shape_key", &request.context_shape_key)?;
    validate_required_selected_fact("cancellation_mode", &request.cancellation_mode)?;
    if request.timeout_ms == Some(0) {
        return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidTimeoutPolicy,
            "timeout_ms",
            "runtime task-attempt timeout must be greater than zero when present",
        ));
    }
    if request.recorded_at_ms == 0 {
        return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity,
            "recorded_at_ms",
            "runtime task-attempt fact timestamp must be greater than zero",
        ));
    }
    Ok(())
}

fn validate_runtime_source_context(
    context: &WorkflowRuntimeSourceContext,
) -> Result<(), WorkflowRuntimeTaskAttemptFactDiagnostic> {
    validate_required_selected_fact(
        "runtime_source_context.operation_type",
        &context.operation_type,
    )?;
    validate_required_selected_fact(
        "runtime_source_context.context_shape_key",
        &context.context_shape_key,
    )?;
    validate_required_selected_fact(
        "runtime_source_context.cancellation_mode",
        &context.cancellation_mode,
    )?;
    Ok(())
}

fn validate_selected_candidate_fact(
    fact: &WorkflowRuntimeDispatchCandidateFact,
) -> Result<(), WorkflowRuntimeTaskAttemptFactDiagnostic> {
    validate_required_selected_fact(
        "selected_candidate_fact.runtime_family",
        &fact.runtime_family,
    )?;
    validate_required_selected_fact(
        "selected_candidate_fact.selected_backend_key",
        &fact.selected_backend_key,
    )?;
    validate_required_selected_fact(
        "selected_candidate_fact.resolved_load_target",
        &fact.resolved_load_target,
    )?;
    validate_required_selected_fact(
        "selected_candidate_fact.runtime_residency_key",
        &fact.runtime_residency_key,
    )?;
    if fact.loaded_runtime_memory_estimate_bytes == 0 {
        return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidMemoryEstimate,
            "selected_candidate_fact.loaded_runtime_memory_estimate_bytes",
            "selected candidate loaded-runtime memory estimate must be greater than zero",
        ));
    }
    let selected_artifact_id = fact
        .selected_model_ref
        .selected_artifact_id
        .as_deref()
        .ok_or_else(|| {
            WorkflowRuntimeTaskAttemptFactDiagnostic::new(
                WorkflowRuntimeTaskAttemptFactDiagnosticCode::MissingSelectedFact,
                "selected_candidate_fact.selected_model_ref.selected_artifact_id",
                "selected candidate fact must carry selected model artifact id",
            )
        })?;
    validate_required_selected_fact(
        "selected_candidate_fact.selected_model_ref.selected_artifact_id",
        selected_artifact_id,
    )?;
    Ok(())
}

fn validate_required_selected_fact(
    field_path: &'static str,
    value: &str,
) -> Result<(), WorkflowRuntimeTaskAttemptFactDiagnostic> {
    validate_non_blank(
        WorkflowRuntimeTaskAttemptFactDiagnosticCode::MissingSelectedFact,
        field_path,
        value,
    )
}

fn validate_resource_fit(
    resource_fit: &WorkflowRuntimeTaskAttemptResourceFitFacts,
) -> Result<(), WorkflowRuntimeTaskAttemptFactDiagnostic> {
    if !matches!(
        resource_fit.state,
        WorkflowRuntimeTaskAttemptResourceFitState::Fits
    ) && resource_fit.diagnostic_codes.is_empty()
    {
        return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidResourceFitFact,
            "resource_fit.diagnostic_codes",
            "non-fit runtime task-attempt resource state requires diagnostics",
        ));
    }
    for (index, diagnostic_code) in resource_fit.diagnostic_codes.iter().enumerate() {
        validate_non_blank(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidResourceFitFact,
            format!("resource_fit.diagnostic_codes[{index}]"),
            diagnostic_code,
        )?;
    }
    Ok(())
}

fn validate_reservation(
    index: usize,
    reservation: &WorkflowRuntimeTaskAttemptReservationFact,
) -> Result<(), WorkflowRuntimeTaskAttemptFactDiagnostic> {
    validate_non_blank(
        WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidReservationFact,
        format!("reservations[{index}].reservation_lease_id"),
        &reservation.reservation_lease_id,
    )?;
    validate_non_blank(
        WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidReservationFact,
        format!("reservations[{index}].device_id"),
        &reservation.device_id,
    )?;
    if reservation.reserved_bytes == 0 {
        return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidReservationFact,
            format!("reservations[{index}].reserved_bytes"),
            "runtime task-attempt reservation bytes must be greater than zero",
        ));
    }
    Ok(())
}

fn validate_non_blank(
    code: WorkflowRuntimeTaskAttemptFactDiagnosticCode,
    field_path: impl Into<String>,
    value: &str,
) -> Result<(), WorkflowRuntimeTaskAttemptFactDiagnostic> {
    let field_path = field_path.into();
    if value.trim().is_empty() {
        return Err(WorkflowRuntimeTaskAttemptFactDiagnostic::new(
            code,
            field_path.clone(),
            format!("runtime task-attempt fact '{field_path}' must not be blank"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use pantograph_dependency_planning::{
        DependencyEnvironmentId, DependencyEnvironmentRef, DeviceIntentId, PumasModelRef,
    };
    use pantograph_scheduler::{
        SchedulerDispatchCandidateId, SchedulerReservationLeaseId, SchedulerResourceFitAssessment,
        SchedulerResourceFitState, SchedulerResourceKind, SchedulerResourceReservation,
        SchedulerRuntimeVariantId, SchedulerTaskId, SchedulerWorkflowRunId,
    };

    use super::super::runtime_dispatch_selection::WorkflowRuntimeDispatchLoadState;
    use super::*;

    #[test]
    fn runtime_task_attempt_fact_records_complete_selected_runtime_resource_facts() {
        let record =
            WorkflowRuntimeTaskAttemptFactRecord::new(fact_request()).expect("valid fact record");

        assert_eq!(
            record.schema_version,
            WORKFLOW_RUNTIME_TASK_ATTEMPT_FACT_SCHEMA_VERSION
        );
        assert_eq!(record.workflow_id, "workflow.image");
        assert_eq!(record.workflow_run_id, "run.image.1");
        assert_eq!(record.scheduler_task_id, "image-task");
        assert_eq!(record.scheduler_task_attempt_id, "attempt.image.1");
        assert_eq!(record.task_attempt_generation, 1);
        assert_eq!(record.selected_model_id, "model.sdxl");
        assert_eq!(record.selected_artifact_id, "artifact.sdxl.diffusers");
        assert_eq!(record.selected_runtime_id, "runtime.diffusers");
        assert_eq!(record.selected_runtime_variant_id.as_deref(), Some("cuda"));
        assert_eq!(record.backend_id, "backend.diffusers");
        assert_eq!(record.runtime_family, "diffusers");
        assert_eq!(record.load_target, "cuda:0");
        assert_eq!(
            record.runtime_residency_key,
            "runtime.diffusers.model.sdxl.cuda0"
        );
        assert_eq!(record.loaded_runtime_memory_estimate_bytes, 8_589_934_592);
        assert_eq!(
            record.resource_fit.state,
            WorkflowRuntimeTaskAttemptResourceFitState::Fits
        );
        assert_eq!(record.reservations.len(), 1);
        assert_eq!(
            record.reservations[0].reservation_lease_id,
            "reservation-lease.runtime.1"
        );
        assert_eq!(record.reservations[0].device_id, "cuda:0");
        assert_eq!(record.operation_type, "image-generation.txt2img");
        assert_eq!(record.context_shape_key, "txt2img.1024x1024.steps30");
        assert_eq!(record.cancellation_mode, "per-run-fanout");
        assert_eq!(record.timeout_ms, Some(30_000));
        assert_eq!(record.recorded_at_ms, 1_000);
    }

    #[test]
    fn runtime_task_attempt_fact_builds_from_source_context_selected_candidate_reservations() {
        let source_context = WorkflowRuntimeTaskAttemptSourceContext::new(source_context_request())
            .expect("valid source context");

        let record = WorkflowRuntimeTaskAttemptFactRecord::from_source_context(
            WorkflowRuntimeTaskAttemptFactBuildRequest {
                source_context,
                scheduler_task_attempt_id: "attempt.image.1".to_string(),
                scheduler_task_attempt_started_at_ms: 900,
                recorded_at_ms: 1_000,
            },
        )
        .expect("valid projected fact record");

        assert_eq!(record.workflow_id, "workflow.image");
        assert_eq!(record.workflow_run_id, "run.image.1");
        assert_eq!(record.scheduler_task_id, "image-task");
        assert_eq!(record.scheduler_task_attempt_id, "attempt.image.1");
        assert_eq!(record.task_attempt_generation, 1);
        assert_eq!(record.selected_model_id, "model.sdxl");
        assert_eq!(record.selected_artifact_id, "artifact.sdxl.diffusers");
        assert_eq!(record.selected_runtime_id, "runtime.diffusers");
        assert_eq!(record.selected_runtime_variant_id.as_deref(), Some("cuda"));
        assert_eq!(record.backend_id, "backend.diffusers");
        assert_eq!(record.runtime_family, "diffusers");
        assert_eq!(record.load_target, "cuda:0");
        assert_eq!(
            record.runtime_residency_key,
            "runtime.diffusers.model.sdxl.cuda0"
        );
        assert_eq!(record.loaded_runtime_memory_estimate_bytes, 8_589_934_592);
        assert_eq!(
            record.resource_fit.state,
            WorkflowRuntimeTaskAttemptResourceFitState::Fits
        );
        assert_eq!(record.resource_fit.diagnostic_codes, Vec::<String>::new());
        assert_eq!(record.reservations.len(), 1);
        assert_eq!(
            record.reservations[0].reservation_lease_id,
            "reservation-lease.runtime.1"
        );
        assert_eq!(record.reservations[0].device_id, "cuda:0");
        assert_eq!(
            record.reservations[0].resource_kind,
            WorkflowRuntimeTaskAttemptResourceKind::DeviceVram
        );
        assert_eq!(record.reservations[0].reserved_bytes, 8_589_934_592);
        assert_eq!(record.operation_type, "image-generation.txt2img");
        assert_eq!(record.context_shape_key, "txt2img.1024x1024.steps30");
        assert_eq!(record.cancellation_mode, "per-run-fanout");
        assert_eq!(record.timeout_ms, Some(30_000));
        assert_eq!(record.recorded_at_ms, 1_000);
    }

    #[test]
    fn runtime_task_attempt_fact_build_rejects_missing_scheduler_attempt_start() {
        let source_context = WorkflowRuntimeTaskAttemptSourceContext::new(source_context_request())
            .expect("valid source context");

        let error = WorkflowRuntimeTaskAttemptFactRecord::from_source_context(
            WorkflowRuntimeTaskAttemptFactBuildRequest {
                source_context,
                scheduler_task_attempt_id: "attempt.image.1".to_string(),
                scheduler_task_attempt_started_at_ms: 0,
                recorded_at_ms: 1_000,
            },
        )
        .expect_err("missing scheduler attempt start must fail");

        assert_eq!(
            error.code,
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity
        );
        assert_eq!(error.field_path, "scheduler_task_attempt_started_at_ms");
    }

    #[test]
    fn runtime_task_attempt_fact_rejects_missing_selected_runtime_fact() {
        let mut request = fact_request();
        request.selected_runtime_id = " ".to_string();

        let error = WorkflowRuntimeTaskAttemptFactRecord::new(request)
            .expect_err("missing runtime id must fail closed");

        assert_eq!(
            error.code,
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::MissingSelectedFact
        );
        assert_eq!(error.field_path, "selected_runtime_id");
    }

    #[test]
    fn runtime_task_attempt_fact_rejects_zero_memory_estimate() {
        let mut request = fact_request();
        request.loaded_runtime_memory_estimate_bytes = 0;

        let error = WorkflowRuntimeTaskAttemptFactRecord::new(request)
            .expect_err("zero memory estimate must fail");

        assert_eq!(
            error.code,
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidMemoryEstimate
        );
        assert_eq!(error.field_path, "loaded_runtime_memory_estimate_bytes");
    }

    #[test]
    fn runtime_task_attempt_fact_requires_resource_diagnostics_for_non_fit_state() {
        let mut request = fact_request();
        request.resource_fit = WorkflowRuntimeTaskAttemptResourceFitFacts {
            state: WorkflowRuntimeTaskAttemptResourceFitState::WaitingForResources,
            diagnostic_codes: Vec::new(),
        };

        let error = WorkflowRuntimeTaskAttemptFactRecord::new(request)
            .expect_err("non-fit resource state requires diagnostics");

        assert_eq!(
            error.code,
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidResourceFitFact
        );
        assert_eq!(error.field_path, "resource_fit.diagnostic_codes");
    }

    #[test]
    fn runtime_task_attempt_fact_rejects_invalid_reservation() {
        let mut request = fact_request();
        request.reservations[0].reserved_bytes = 0;

        let error = WorkflowRuntimeTaskAttemptFactRecord::new(request)
            .expect_err("zero reservation bytes must fail");

        assert_eq!(
            error.code,
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidReservationFact
        );
        assert_eq!(error.field_path, "reservations[0].reserved_bytes");
    }

    #[test]
    fn runtime_task_attempt_fact_rejects_reservation_without_device() {
        let mut request = fact_request();
        request.reservations[0].device_id = " ".to_string();

        let error = WorkflowRuntimeTaskAttemptFactRecord::new(request)
            .expect_err("blank reservation device must fail");

        assert_eq!(
            error.code,
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidReservationFact
        );
        assert_eq!(error.field_path, "reservations[0].device_id");
    }

    #[test]
    fn runtime_task_attempt_fact_rejects_zero_timeout() {
        let mut request = fact_request();
        request.timeout_ms = Some(0);

        let error =
            WorkflowRuntimeTaskAttemptFactRecord::new(request).expect_err("zero timeout must fail");

        assert_eq!(
            error.code,
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidTimeoutPolicy
        );
        assert_eq!(error.field_path, "timeout_ms");
    }

    #[test]
    fn source_context_groups_runtime_source_context_and_selected_candidate_fact() {
        let context = WorkflowRuntimeTaskAttemptSourceContext::new(source_context_request())
            .expect("source context should validate");

        assert_eq!(context.workflow_id, "workflow.image");
        assert_eq!(context.workflow_run_id, "run.image.1");
        assert_eq!(context.scheduler_task_id, "image-task");
        assert_eq!(context.task_attempt_generation, 1);
        assert_eq!(context.timeout_ms, Some(30_000));
        assert_eq!(
            context.runtime_source_context.context_shape_key,
            "txt2img.1024x1024.steps30"
        );
        assert_eq!(
            context.selected_candidate_fact.runtime_residency_key,
            "runtime.diffusers.model.sdxl.cuda0"
        );
    }

    #[test]
    fn source_context_rejects_missing_runtime_source_context_field() {
        let mut request = source_context_request();
        request.runtime_source_context.operation_type = " ".to_string();

        let error = WorkflowRuntimeTaskAttemptSourceContext::new(request)
            .expect_err("missing operation type must fail closed");

        assert_eq!(
            error.code,
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::MissingSelectedFact
        );
        assert_eq!(error.field_path, "runtime_source_context.operation_type");
    }

    #[test]
    fn source_context_rejects_missing_selected_candidate_fact_field() {
        let mut request = source_context_request();
        request.selected_candidate_fact.runtime_family = " ".to_string();

        let error = WorkflowRuntimeTaskAttemptSourceContext::new(request)
            .expect_err("missing selected runtime family must fail closed");

        assert_eq!(
            error.code,
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::MissingSelectedFact
        );
        assert_eq!(error.field_path, "selected_candidate_fact.runtime_family");
    }

    #[test]
    fn source_context_rejects_zero_attempt_generation() {
        let mut request = source_context_request();
        request.task_attempt_generation = 0;

        let error = WorkflowRuntimeTaskAttemptSourceContext::new(request)
            .expect_err("zero attempt generation must fail closed");

        assert_eq!(
            error.code,
            WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidAttemptIdentity
        );
        assert_eq!(error.field_path, "task_attempt_generation");
    }

    fn source_context_request() -> WorkflowRuntimeTaskAttemptSourceContextRequest {
        WorkflowRuntimeTaskAttemptSourceContextRequest {
            workflow_id: "workflow.image".to_string(),
            workflow_run_id: "run.image.1".to_string(),
            scheduler_task_id: "image-task".to_string(),
            task_attempt_generation: 1,
            timeout_ms: Some(30_000),
            runtime_source_context: runtime_source_context(),
            selected_candidate_fact: selected_candidate_fact(),
        }
    }

    fn runtime_source_context() -> crate::graph::WorkflowRuntimeSourceContext {
        crate::graph::WorkflowRuntimeSourceContext {
            context_shape_key: "txt2img.1024x1024.steps30".to_string(),
            operation_type: "image-generation.txt2img".to_string(),
            cancellation_mode: "per-run-fanout".to_string(),
        }
    }

    fn selected_candidate_fact() -> WorkflowRuntimeDispatchCandidateFact {
        let workflow_run_id: SchedulerWorkflowRunId = "run.image.1".parse().expect("run id");
        let task_id: SchedulerTaskId = "image-task".parse().expect("task id");
        let device_id: DeviceIntentId = "cuda:0".parse().expect("device id");
        WorkflowRuntimeDispatchCandidateFact {
            candidate_id: SchedulerDispatchCandidateId::parse("candidate.diffusers.cuda0")
                .expect("candidate id"),
            selected_runtime_id: "runtime.diffusers".parse().expect("runtime id"),
            selected_runtime_variant_id: Some(
                SchedulerRuntimeVariantId::parse("cuda").expect("runtime variant id"),
            ),
            selected_backend_key: "backend.diffusers".to_string(),
            runtime_family: "diffusers".to_string(),
            resolved_load_target: "cuda:0".to_string(),
            runtime_residency_key: "runtime.diffusers.model.sdxl.cuda0".to_string(),
            loaded_runtime_memory_estimate_bytes: 8_589_934_592,
            runtime_load_state: WorkflowRuntimeDispatchLoadState::Loaded,
            runtime_instance_id: Some("runtime.diffusers.001".to_string()),
            selected_device_ids: vec![device_id.clone()],
            selected_model_ref: PumasModelRef {
                model_id: "model.sdxl".to_string(),
                revision: Some("main".to_string()),
                selected_artifact_id: Some("artifact.sdxl.diffusers".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            runtime_trait_settings: Vec::new(),
            environment_ref: DependencyEnvironmentRef {
                environment_id: DependencyEnvironmentId::parse("env.runtime")
                    .expect("environment id"),
                manifest_id: None,
            },
            reservations: vec![SchedulerResourceReservation {
                reservation_lease_id: SchedulerReservationLeaseId::parse(
                    "reservation-lease.runtime.1",
                )
                .expect("reservation lease id"),
                workflow_run_id: workflow_run_id.clone(),
                task_id: task_id.clone(),
                device_id,
                resource_kind: SchedulerResourceKind::DeviceVram,
                reserved_bytes: 8_589_934_592,
            }],
            resource_fit_assessment: SchedulerResourceFitAssessment {
                workflow_run_id,
                task_id,
                state: SchedulerResourceFitState::Fits,
                diagnostics: Vec::new(),
            },
            batching_group_id: None,
        }
    }

    fn fact_request() -> WorkflowRuntimeTaskAttemptFactRequest {
        WorkflowRuntimeTaskAttemptFactRequest {
            workflow_id: "workflow.image".to_string(),
            workflow_run_id: "run.image.1".to_string(),
            scheduler_task_id: "image-task".to_string(),
            scheduler_task_attempt_id: "attempt.image.1".to_string(),
            task_attempt_generation: 1,
            selected_model_id: "model.sdxl".to_string(),
            selected_artifact_id: "artifact.sdxl.diffusers".to_string(),
            selected_runtime_id: "runtime.diffusers".to_string(),
            selected_runtime_variant_id: Some("cuda".to_string()),
            backend_id: "backend.diffusers".to_string(),
            runtime_family: "diffusers".to_string(),
            load_target: "cuda:0".to_string(),
            runtime_residency_key: "runtime.diffusers.model.sdxl.cuda0".to_string(),
            loaded_runtime_memory_estimate_bytes: 8_589_934_592,
            resource_fit: WorkflowRuntimeTaskAttemptResourceFitFacts {
                state: WorkflowRuntimeTaskAttemptResourceFitState::Fits,
                diagnostic_codes: Vec::new(),
            },
            reservations: vec![WorkflowRuntimeTaskAttemptReservationFact {
                reservation_lease_id: "reservation-lease.runtime.1".to_string(),
                device_id: "cuda:0".to_string(),
                resource_kind: WorkflowRuntimeTaskAttemptResourceKind::DeviceVram,
                reserved_bytes: 8_589_934_592,
            }],
            operation_type: "image-generation.txt2img".to_string(),
            context_shape_key: "txt2img.1024x1024.steps30".to_string(),
            cancellation_mode: "per-run-fanout".to_string(),
            timeout_ms: Some(30_000),
            recorded_at_ms: 1_000,
        }
    }
}
