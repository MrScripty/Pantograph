use pantograph_dependency_planning::{DeviceIntentId, PumasModelRef, RuntimeIntentId};
use pantograph_scheduler::{
    SchedulerResourceFitAssessment, SchedulerResourceFitState, SchedulerResourceReservation,
};

pub(crate) const RUNTIME_DISPATCH_EVIDENCE_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(crate) struct RuntimeDispatchEvidenceRequest {
    pub(crate) selected_backend_key: String,
    pub(crate) runtime_family: String,
    pub(crate) resolved_load_target: String,
    pub(crate) runtime_residency_key: String,
    pub(crate) loaded_runtime_memory_estimate_bytes: u64,
    pub(crate) runtime_load_state: Option<RuntimeDispatchEvidenceLoadState>,
    pub(crate) runtime_instance_id: Option<String>,
    pub(crate) selected_runtime_id: RuntimeIntentId,
    pub(crate) selected_model_ref: PumasModelRef,
    pub(crate) selected_device_id: DeviceIntentId,
    pub(crate) reservations: Vec<SchedulerResourceReservation>,
    pub(crate) resource_fit_assessment: SchedulerResourceFitAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(crate) struct RuntimeDispatchEvidenceRecord {
    pub(crate) contract_version: u16,
    pub(crate) selected_backend_key: String,
    pub(crate) runtime_family: String,
    pub(crate) resolved_load_target: String,
    pub(crate) runtime_residency_key: String,
    pub(crate) loaded_runtime_memory_estimate_bytes: u64,
    pub(crate) runtime_load_state: RuntimeDispatchEvidenceLoadState,
    pub(crate) runtime_instance_id: Option<String>,
    pub(crate) selected_runtime_id: RuntimeIntentId,
    pub(crate) selected_model_ref: PumasModelRef,
    pub(crate) selected_device_id: DeviceIntentId,
    pub(crate) reservations: Vec<SchedulerResourceReservation>,
    pub(crate) resource_fit_assessment: SchedulerResourceFitAssessment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(crate) enum RuntimeDispatchEvidenceLoadState {
    NotLoaded,
    Loading,
    Loaded,
    Busy,
    Unloading,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(crate) struct RuntimeDispatchEvidenceDiagnostic {
    pub(crate) code: RuntimeDispatchEvidenceDiagnosticCode,
    pub(crate) field_path: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(crate) enum RuntimeDispatchEvidenceDiagnosticCode {
    MissingSelectedFact,
    InvalidMemoryEstimate,
    InvalidModelFact,
    InvalidRuntimeInstanceFact,
    InvalidReservationFact,
    InvalidResourceFitFact,
}

impl RuntimeDispatchEvidenceRecord {
    pub(crate) fn new(
        request: RuntimeDispatchEvidenceRequest,
    ) -> Result<Self, RuntimeDispatchEvidenceDiagnostic> {
        validate_request(&request)?;
        Ok(Self {
            contract_version: RUNTIME_DISPATCH_EVIDENCE_CONTRACT_VERSION,
            selected_backend_key: request.selected_backend_key,
            runtime_family: request.runtime_family,
            resolved_load_target: request.resolved_load_target,
            runtime_residency_key: request.runtime_residency_key,
            loaded_runtime_memory_estimate_bytes: request.loaded_runtime_memory_estimate_bytes,
            runtime_load_state: request
                .runtime_load_state
                .expect("runtime load state was validated as present"),
            runtime_instance_id: request.runtime_instance_id,
            selected_runtime_id: request.selected_runtime_id,
            selected_model_ref: request.selected_model_ref,
            selected_device_id: request.selected_device_id,
            reservations: request.reservations,
            resource_fit_assessment: request.resource_fit_assessment,
        })
    }
}

impl RuntimeDispatchEvidenceDiagnostic {
    fn new(
        code: RuntimeDispatchEvidenceDiagnosticCode,
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
    request: &RuntimeDispatchEvidenceRequest,
) -> Result<(), RuntimeDispatchEvidenceDiagnostic> {
    validate_selected_fact("selected_backend_key", &request.selected_backend_key)?;
    validate_selected_fact("runtime_family", &request.runtime_family)?;
    validate_selected_fact("resolved_load_target", &request.resolved_load_target)?;
    validate_selected_fact("runtime_residency_key", &request.runtime_residency_key)?;
    if request.loaded_runtime_memory_estimate_bytes == 0 {
        return Err(RuntimeDispatchEvidenceDiagnostic::new(
            RuntimeDispatchEvidenceDiagnosticCode::InvalidMemoryEstimate,
            "loaded_runtime_memory_estimate_bytes",
            "loaded-runtime memory estimate must be greater than zero",
        ));
    }
    validate_runtime_load_state(
        request.runtime_load_state,
        request.runtime_instance_id.as_deref(),
    )?;
    validate_model_ref(&request.selected_model_ref)?;
    validate_reservations(request)?;
    validate_resource_fit(&request.resource_fit_assessment)?;
    Ok(())
}

fn validate_selected_fact(
    field_path: &'static str,
    value: &str,
) -> Result<(), RuntimeDispatchEvidenceDiagnostic> {
    if value.trim().is_empty() {
        return Err(RuntimeDispatchEvidenceDiagnostic::new(
            RuntimeDispatchEvidenceDiagnosticCode::MissingSelectedFact,
            field_path,
            format!("{field_path} must be supplied by canonical dispatch evidence"),
        ));
    }
    Ok(())
}

fn validate_runtime_load_state(
    runtime_load_state: Option<RuntimeDispatchEvidenceLoadState>,
    runtime_instance_id: Option<&str>,
) -> Result<(), RuntimeDispatchEvidenceDiagnostic> {
    let Some(runtime_load_state) = runtime_load_state else {
        return Err(RuntimeDispatchEvidenceDiagnostic::new(
            RuntimeDispatchEvidenceDiagnosticCode::MissingSelectedFact,
            "runtime_load_state",
            "runtime load state must be supplied by canonical dispatch evidence",
        ));
    };
    if runtime_instance_id.is_some_and(|runtime_instance_id| runtime_instance_id.trim().is_empty())
    {
        return Err(RuntimeDispatchEvidenceDiagnostic::new(
            RuntimeDispatchEvidenceDiagnosticCode::InvalidRuntimeInstanceFact,
            "runtime_instance_id",
            "runtime instance id must not be blank when supplied",
        ));
    }
    if matches!(
        runtime_load_state,
        RuntimeDispatchEvidenceLoadState::Loaded | RuntimeDispatchEvidenceLoadState::Busy
    ) && runtime_instance_id.is_none()
    {
        return Err(RuntimeDispatchEvidenceDiagnostic::new(
            RuntimeDispatchEvidenceDiagnosticCode::InvalidRuntimeInstanceFact,
            "runtime_instance_id",
            "loaded or busy runtime evidence must include the selected runtime instance id",
        ));
    }
    Ok(())
}

fn validate_model_ref(
    selected_model_ref: &PumasModelRef,
) -> Result<(), RuntimeDispatchEvidenceDiagnostic> {
    if selected_model_ref.selected_artifact_path.is_some() {
        return Err(RuntimeDispatchEvidenceDiagnostic::new(
            RuntimeDispatchEvidenceDiagnosticCode::InvalidModelFact,
            "selected_model_ref.selected_artifact_path",
            "runtime dispatch evidence must not carry selected artifact paths",
        ));
    }
    if selected_model_ref
        .selected_artifact_id
        .as_ref()
        .is_none_or(|selected_artifact_id| selected_artifact_id.trim().is_empty())
    {
        return Err(RuntimeDispatchEvidenceDiagnostic::new(
            RuntimeDispatchEvidenceDiagnosticCode::MissingSelectedFact,
            "selected_model_ref.selected_artifact_id",
            "runtime dispatch evidence requires a selected artifact id",
        ));
    }
    selected_model_ref.validate().map_err(|source| {
        RuntimeDispatchEvidenceDiagnostic::new(
            RuntimeDispatchEvidenceDiagnosticCode::InvalidModelFact,
            "selected_model_ref",
            format!("selected model ref failed validation: {source}"),
        )
    })
}

fn validate_reservations(
    request: &RuntimeDispatchEvidenceRequest,
) -> Result<(), RuntimeDispatchEvidenceDiagnostic> {
    if request.reservations.is_empty() {
        return Err(RuntimeDispatchEvidenceDiagnostic::new(
            RuntimeDispatchEvidenceDiagnosticCode::InvalidReservationFact,
            "reservations",
            "runtime dispatch evidence requires at least one reservation fact",
        ));
    }
    for (index, reservation) in request.reservations.iter().enumerate() {
        if reservation.device_id != request.selected_device_id {
            return Err(RuntimeDispatchEvidenceDiagnostic::new(
                RuntimeDispatchEvidenceDiagnosticCode::InvalidReservationFact,
                format!("reservations[{index}].device_id"),
                "runtime dispatch reservation device must match the selected device",
            ));
        }
        if reservation.reserved_bytes == 0 {
            return Err(RuntimeDispatchEvidenceDiagnostic::new(
                RuntimeDispatchEvidenceDiagnosticCode::InvalidReservationFact,
                format!("reservations[{index}].reserved_bytes"),
                "runtime dispatch reservation bytes must be greater than zero",
            ));
        }
    }
    Ok(())
}

fn validate_resource_fit(
    resource_fit_assessment: &SchedulerResourceFitAssessment,
) -> Result<(), RuntimeDispatchEvidenceDiagnostic> {
    if matches!(
        resource_fit_assessment.state,
        SchedulerResourceFitState::WaitingForResources
            | SchedulerResourceFitState::ImpossibleFit
            | SchedulerResourceFitState::Unknown
    ) && resource_fit_assessment.diagnostics.is_empty()
    {
        return Err(RuntimeDispatchEvidenceDiagnostic::new(
            RuntimeDispatchEvidenceDiagnosticCode::InvalidResourceFitFact,
            "resource_fit_assessment.diagnostics",
            "non-fit runtime dispatch evidence must include resource-fit diagnostics",
        ));
    }
    for (index, diagnostic) in resource_fit_assessment.diagnostics.iter().enumerate() {
        if diagnostic.message.trim().is_empty() {
            return Err(RuntimeDispatchEvidenceDiagnostic::new(
                RuntimeDispatchEvidenceDiagnosticCode::InvalidResourceFitFact,
                format!("resource_fit_assessment.diagnostics[{index}].message"),
                "resource-fit diagnostic message must not be blank",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use pantograph_scheduler::{
        SchedulerReservationLeaseId, SchedulerResourceDiagnostic, SchedulerResourceDiagnosticCode,
        SchedulerResourceDiagnosticSeverity, SchedulerResourceFitAssessment,
        SchedulerResourceFitState, SchedulerResourceKind, SchedulerResourceReservation,
        SchedulerTaskId, SchedulerWorkflowRunId,
    };

    use super::*;

    #[test]
    fn record_accepts_complete_dispatch_evidence() {
        let record = RuntimeDispatchEvidenceRecord::new(dispatch_evidence_request())
            .expect("complete dispatch evidence should validate");

        assert_eq!(
            record.contract_version,
            RUNTIME_DISPATCH_EVIDENCE_CONTRACT_VERSION
        );
        assert_eq!(record.selected_backend_key, "diffusers");
        assert_eq!(record.runtime_family, "image-generation");
        assert_eq!(record.resolved_load_target, "cuda:0");
        assert_eq!(
            record.runtime_residency_key,
            "runtime:pytorch:model:pumas.model.sdxl:cuda:0"
        );
        assert_eq!(record.loaded_runtime_memory_estimate_bytes, 8 * mib());
        assert_eq!(
            record.runtime_load_state,
            RuntimeDispatchEvidenceLoadState::Loaded
        );
        assert_eq!(
            record.runtime_instance_id.as_deref(),
            Some("runtime-instance.pytorch.cuda0")
        );
    }

    #[test]
    fn record_rejects_missing_backend_key() {
        let mut request = dispatch_evidence_request();
        request.selected_backend_key = " ".to_string();

        let diagnostic = RuntimeDispatchEvidenceRecord::new(request)
            .expect_err("missing backend evidence must fail closed");

        assert_eq!(
            diagnostic.code,
            RuntimeDispatchEvidenceDiagnosticCode::MissingSelectedFact
        );
        assert_eq!(diagnostic.field_path, "selected_backend_key");
    }

    #[test]
    fn record_rejects_missing_load_target() {
        let mut request = dispatch_evidence_request();
        request.resolved_load_target.clear();

        let diagnostic = RuntimeDispatchEvidenceRecord::new(request)
            .expect_err("missing load target evidence must fail closed");

        assert_eq!(
            diagnostic.code,
            RuntimeDispatchEvidenceDiagnosticCode::MissingSelectedFact
        );
        assert_eq!(diagnostic.field_path, "resolved_load_target");
    }

    #[test]
    fn record_rejects_missing_residency_key() {
        let mut request = dispatch_evidence_request();
        request.runtime_residency_key.clear();

        let diagnostic = RuntimeDispatchEvidenceRecord::new(request)
            .expect_err("missing residency evidence must fail closed");

        assert_eq!(
            diagnostic.code,
            RuntimeDispatchEvidenceDiagnosticCode::MissingSelectedFact
        );
        assert_eq!(diagnostic.field_path, "runtime_residency_key");
    }

    #[test]
    fn record_rejects_zero_memory_estimate() {
        let mut request = dispatch_evidence_request();
        request.loaded_runtime_memory_estimate_bytes = 0;

        let diagnostic = RuntimeDispatchEvidenceRecord::new(request)
            .expect_err("missing memory estimate evidence must fail closed");

        assert_eq!(
            diagnostic.code,
            RuntimeDispatchEvidenceDiagnosticCode::InvalidMemoryEstimate
        );
        assert_eq!(
            diagnostic.field_path,
            "loaded_runtime_memory_estimate_bytes"
        );
    }

    #[test]
    fn record_rejects_missing_runtime_load_state() {
        let mut request = dispatch_evidence_request();
        request.runtime_load_state = None;

        let diagnostic = RuntimeDispatchEvidenceRecord::new(request)
            .expect_err("missing load-state evidence must fail closed");

        assert_eq!(
            diagnostic.code,
            RuntimeDispatchEvidenceDiagnosticCode::MissingSelectedFact
        );
        assert_eq!(diagnostic.field_path, "runtime_load_state");
    }

    #[test]
    fn record_rejects_loaded_runtime_without_instance_id() {
        let mut request = dispatch_evidence_request();
        request.runtime_instance_id = None;

        let diagnostic = RuntimeDispatchEvidenceRecord::new(request)
            .expect_err("loaded runtime evidence must identify the selected instance");

        assert_eq!(
            diagnostic.code,
            RuntimeDispatchEvidenceDiagnosticCode::InvalidRuntimeInstanceFact
        );
        assert_eq!(diagnostic.field_path, "runtime_instance_id");
    }

    #[test]
    fn record_allows_not_loaded_runtime_without_instance_id() {
        let mut request = dispatch_evidence_request();
        request.runtime_load_state = Some(RuntimeDispatchEvidenceLoadState::NotLoaded);
        request.runtime_instance_id = None;

        let record = RuntimeDispatchEvidenceRecord::new(request)
            .expect("not-loaded runtime can be selected before instance exists");

        assert_eq!(
            record.runtime_load_state,
            RuntimeDispatchEvidenceLoadState::NotLoaded
        );
        assert!(record.runtime_instance_id.is_none());
    }

    #[test]
    fn record_rejects_path_carrying_model_ref() {
        let mut request = dispatch_evidence_request();
        request.selected_model_ref.selected_artifact_path =
            Some("/models/sdxl/model_index.json".to_string());

        let diagnostic = RuntimeDispatchEvidenceRecord::new(request)
            .expect_err("dispatch evidence must not carry filesystem paths");

        assert_eq!(
            diagnostic.code,
            RuntimeDispatchEvidenceDiagnosticCode::InvalidModelFact
        );
        assert_eq!(
            diagnostic.field_path,
            "selected_model_ref.selected_artifact_path"
        );
    }

    #[test]
    fn record_rejects_missing_selected_artifact() {
        let mut request = dispatch_evidence_request();
        request.selected_model_ref.selected_artifact_id = None;

        let diagnostic = RuntimeDispatchEvidenceRecord::new(request)
            .expect_err("dispatch evidence must identify selected artifact");

        assert_eq!(
            diagnostic.code,
            RuntimeDispatchEvidenceDiagnosticCode::MissingSelectedFact
        );
        assert_eq!(
            diagnostic.field_path,
            "selected_model_ref.selected_artifact_id"
        );
    }

    #[test]
    fn record_rejects_reservation_for_other_device() {
        let mut request = dispatch_evidence_request();
        request.reservations[0].device_id = "cuda:1".parse().expect("device id");

        let diagnostic = RuntimeDispatchEvidenceRecord::new(request)
            .expect_err("reservation evidence must match selected device");

        assert_eq!(
            diagnostic.code,
            RuntimeDispatchEvidenceDiagnosticCode::InvalidReservationFact
        );
        assert_eq!(diagnostic.field_path, "reservations[0].device_id");
    }

    #[test]
    fn record_rejects_non_fit_without_diagnostics() {
        let mut request = dispatch_evidence_request();
        request.resource_fit_assessment.state = SchedulerResourceFitState::WaitingForResources;

        let diagnostic = RuntimeDispatchEvidenceRecord::new(request)
            .expect_err("non-fit resource evidence must include diagnostics");

        assert_eq!(
            diagnostic.code,
            RuntimeDispatchEvidenceDiagnosticCode::InvalidResourceFitFact
        );
        assert_eq!(diagnostic.field_path, "resource_fit_assessment.diagnostics");
    }

    #[test]
    fn record_accepts_non_fit_with_diagnostics() {
        let mut request = dispatch_evidence_request();
        request.runtime_load_state = Some(RuntimeDispatchEvidenceLoadState::NotLoaded);
        request.runtime_instance_id = None;
        request.resource_fit_assessment.state = SchedulerResourceFitState::WaitingForResources;
        request.resource_fit_assessment.diagnostics = vec![SchedulerResourceDiagnostic {
            severity: SchedulerResourceDiagnosticSeverity::Error,
            code: SchedulerResourceDiagnosticCode::RuntimeNotReady,
            message: "device memory is reserved by another task".to_string(),
            hint: None,
        }];

        let record = RuntimeDispatchEvidenceRecord::new(request)
            .expect("non-fit evidence can be recorded when diagnostics explain it");

        assert_eq!(
            record.resource_fit_assessment.state,
            SchedulerResourceFitState::WaitingForResources
        );
    }

    fn dispatch_evidence_request() -> RuntimeDispatchEvidenceRequest {
        let workflow_run_id: SchedulerWorkflowRunId =
            "run.dispatch-evidence".parse().expect("workflow run id");
        let task_id: SchedulerTaskId = "infer".parse().expect("task id");
        let selected_device_id: DeviceIntentId = "cuda:0".parse().expect("device id");
        RuntimeDispatchEvidenceRequest {
            selected_backend_key: "diffusers".to_string(),
            runtime_family: "image-generation".to_string(),
            resolved_load_target: "cuda:0".to_string(),
            runtime_residency_key: "runtime:pytorch:model:pumas.model.sdxl:cuda:0".to_string(),
            loaded_runtime_memory_estimate_bytes: 8 * mib(),
            runtime_load_state: Some(RuntimeDispatchEvidenceLoadState::Loaded),
            runtime_instance_id: Some("runtime-instance.pytorch.cuda0".to_string()),
            selected_runtime_id: "pytorch".parse().expect("runtime id"),
            selected_model_ref: PumasModelRef {
                model_id: "pumas.model.sdxl".to_string(),
                revision: Some("main".to_string()),
                selected_artifact_id: Some("diffusers".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            selected_device_id: selected_device_id.clone(),
            reservations: vec![SchedulerResourceReservation {
                reservation_lease_id: SchedulerReservationLeaseId::parse("runtime-registry.100")
                    .expect("reservation id"),
                workflow_run_id: workflow_run_id.clone(),
                task_id: task_id.clone(),
                device_id: selected_device_id,
                resource_kind: SchedulerResourceKind::DeviceVram,
                reserved_bytes: mib(),
            }],
            resource_fit_assessment: SchedulerResourceFitAssessment {
                workflow_run_id,
                task_id,
                state: SchedulerResourceFitState::Fits,
                diagnostics: Vec::new(),
            },
        }
    }

    fn mib() -> u64 {
        1024 * 1024
    }
}
