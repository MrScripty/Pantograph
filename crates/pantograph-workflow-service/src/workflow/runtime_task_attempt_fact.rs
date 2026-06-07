use serde::{Deserialize, Serialize};

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
    pub(super) resolved_device_id: String,
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
    pub(super) resolved_device_id: String,
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
    pub(super) reservation_id: String,
    pub(super) lease_id: String,
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
    InvalidMemoryEstimate,
    InvalidReservationFact,
    InvalidResourceFitFact,
    InvalidTimeoutPolicy,
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
            resolved_device_id: request.resolved_device_id,
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
    validate_required_selected_fact("resolved_device_id", &request.resolved_device_id)?;
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
        format!("reservations[{index}].reservation_id"),
        &reservation.reservation_id,
    )?;
    validate_non_blank(
        WorkflowRuntimeTaskAttemptFactDiagnosticCode::InvalidReservationFact,
        format!("reservations[{index}].lease_id"),
        &reservation.lease_id,
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
        assert_eq!(record.resolved_device_id, "cuda:0");
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
        assert_eq!(record.operation_type, "image-generation.txt2img");
        assert_eq!(record.context_shape_key, "txt2img.1024x1024.steps30");
        assert_eq!(record.cancellation_mode, "per-run-fanout");
        assert_eq!(record.timeout_ms, Some(30_000));
        assert_eq!(record.recorded_at_ms, 1_000);
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
            resolved_device_id: "cuda:0".to_string(),
            load_target: "cuda:0".to_string(),
            runtime_residency_key: "runtime.diffusers.model.sdxl.cuda0".to_string(),
            loaded_runtime_memory_estimate_bytes: 8_589_934_592,
            resource_fit: WorkflowRuntimeTaskAttemptResourceFitFacts {
                state: WorkflowRuntimeTaskAttemptResourceFitState::Fits,
                diagnostic_codes: Vec::new(),
            },
            reservations: vec![WorkflowRuntimeTaskAttemptReservationFact {
                reservation_id: "reservation.runtime.1".to_string(),
                lease_id: "reservation-lease.runtime.1".to_string(),
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
