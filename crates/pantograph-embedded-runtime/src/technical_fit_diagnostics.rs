use pantograph_runtime_registry::{
    RuntimeTechnicalFitDeviceClass, RuntimeTechnicalFitDeviceDiagnostic,
    RuntimeTechnicalFitDeviceDiagnosticCode, RuntimeTechnicalFitDeviceDiagnosticSeverity,
};
use pantograph_workflow_service::{
    WorkflowDeviceResolutionDiagnostic, WorkflowDeviceResolutionDiagnosticCode,
    WorkflowDeviceResolutionDiagnosticSeverity, WorkflowInferenceDeviceClass,
    WorkflowTechnicalFitDeviceClass, WorkflowTechnicalFitDeviceDiagnostic,
    WorkflowTechnicalFitDeviceDiagnosticCode, WorkflowTechnicalFitDeviceDiagnosticSeverity,
};

pub(crate) fn project_runtime_device_class(
    device_class: RuntimeTechnicalFitDeviceClass,
) -> WorkflowTechnicalFitDeviceClass {
    match device_class {
        RuntimeTechnicalFitDeviceClass::Cpu => WorkflowTechnicalFitDeviceClass::Cpu,
        RuntimeTechnicalFitDeviceClass::Cuda => WorkflowTechnicalFitDeviceClass::Cuda,
        RuntimeTechnicalFitDeviceClass::Metal => WorkflowTechnicalFitDeviceClass::Metal,
        RuntimeTechnicalFitDeviceClass::Mps => WorkflowTechnicalFitDeviceClass::Mps,
    }
}

pub(crate) fn project_workflow_runtime_variant_device_class(
    device_class: WorkflowInferenceDeviceClass,
) -> Option<RuntimeTechnicalFitDeviceClass> {
    match device_class {
        WorkflowInferenceDeviceClass::Cpu => Some(RuntimeTechnicalFitDeviceClass::Cpu),
        WorkflowInferenceDeviceClass::Cuda => Some(RuntimeTechnicalFitDeviceClass::Cuda),
        WorkflowInferenceDeviceClass::Metal => Some(RuntimeTechnicalFitDeviceClass::Metal),
        WorkflowInferenceDeviceClass::Mps => Some(RuntimeTechnicalFitDeviceClass::Mps),
        WorkflowInferenceDeviceClass::Unknown => None,
    }
}

pub(crate) fn project_runtime_device_diagnostic(
    diagnostic: &RuntimeTechnicalFitDeviceDiagnostic,
) -> WorkflowTechnicalFitDeviceDiagnostic {
    WorkflowTechnicalFitDeviceDiagnostic {
        code: project_runtime_device_diagnostic_code(diagnostic.code),
        severity: project_runtime_device_diagnostic_severity(diagnostic.severity),
        message: diagnostic.message.clone(),
        task_id: diagnostic.task_id.clone(),
        runtime_id: diagnostic.runtime_id.clone(),
        device_class: diagnostic.device_class.map(project_runtime_device_class),
        device_id: diagnostic.device_id.clone(),
        runtime_variant_id: diagnostic.runtime_variant_id.clone(),
        backend_key: diagnostic.backend_key.clone(),
        model_id: diagnostic.model_id.clone(),
        evidence_key: diagnostic.evidence_key.clone(),
        requested_runtime_key: diagnostic.requested_runtime_key.clone(),
    }
}

fn project_runtime_device_diagnostic_code(
    code: RuntimeTechnicalFitDeviceDiagnosticCode,
) -> WorkflowTechnicalFitDeviceDiagnosticCode {
    match code {
        RuntimeTechnicalFitDeviceDiagnosticCode::InvalidDevicePolicy => {
            WorkflowTechnicalFitDeviceDiagnosticCode::InvalidDevicePolicy
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::InvalidDeviceId => {
            WorkflowTechnicalFitDeviceDiagnosticCode::InvalidDeviceId
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::InvalidRuntimeVariantId => {
            WorkflowTechnicalFitDeviceDiagnosticCode::InvalidRuntimeVariantId
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::InvalidBackendId => {
            WorkflowTechnicalFitDeviceDiagnosticCode::InvalidBackendId
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::CandidateUnavailable => {
            WorkflowTechnicalFitDeviceDiagnosticCode::CandidateUnavailable
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::ExplicitDeviceUnavailable => {
            WorkflowTechnicalFitDeviceDiagnosticCode::ExplicitDeviceUnavailable
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::NoValidCandidate => {
            WorkflowTechnicalFitDeviceDiagnosticCode::NoValidCandidate
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::AmbiguousAutoResolution => {
            WorkflowTechnicalFitDeviceDiagnosticCode::AmbiguousAutoResolution
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::BackendIncompatible => {
            WorkflowTechnicalFitDeviceDiagnosticCode::BackendIncompatible
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::UnsupportedDeviceClass => {
            WorkflowTechnicalFitDeviceDiagnosticCode::UnsupportedDeviceClass
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::MissingRuntimeVariant => {
            WorkflowTechnicalFitDeviceDiagnosticCode::MissingRuntimeVariant
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::MissingModelPackageFacts => {
            WorkflowTechnicalFitDeviceDiagnosticCode::MissingModelPackageFacts
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::CandidateSetOverflow => {
            WorkflowTechnicalFitDeviceDiagnosticCode::CandidateSetOverflow
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::LegacyDeviceRejected => {
            WorkflowTechnicalFitDeviceDiagnosticCode::LegacyDeviceRejected
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceUnsupportedTask => {
            WorkflowTechnicalFitDeviceDiagnosticCode::EvidenceUnsupportedTask
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceBackendUnavailable => {
            WorkflowTechnicalFitDeviceDiagnosticCode::EvidenceBackendUnavailable
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceMissingRuntimeCapability => {
            WorkflowTechnicalFitDeviceDiagnosticCode::EvidenceMissingRuntimeCapability
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceRequiredPackageUnavailable => {
            WorkflowTechnicalFitDeviceDiagnosticCode::EvidenceRequiredPackageUnavailable
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceBackendCompatibilityRejected => {
            WorkflowTechnicalFitDeviceDiagnosticCode::EvidenceBackendCompatibilityRejected
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceGraphRuntimeUnsatisfied => {
            WorkflowTechnicalFitDeviceDiagnosticCode::EvidenceGraphRuntimeUnsatisfied
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceNoAcceptedCandidate => {
            WorkflowTechnicalFitDeviceDiagnosticCode::EvidenceNoAcceptedCandidate
        }
        _ => WorkflowTechnicalFitDeviceDiagnosticCode::NoValidCandidate,
    }
}

fn project_runtime_device_diagnostic_severity(
    severity: RuntimeTechnicalFitDeviceDiagnosticSeverity,
) -> WorkflowTechnicalFitDeviceDiagnosticSeverity {
    match severity {
        RuntimeTechnicalFitDeviceDiagnosticSeverity::Advisory => {
            WorkflowTechnicalFitDeviceDiagnosticSeverity::Advisory
        }
        RuntimeTechnicalFitDeviceDiagnosticSeverity::Warning => {
            WorkflowTechnicalFitDeviceDiagnosticSeverity::Warning
        }
        RuntimeTechnicalFitDeviceDiagnosticSeverity::Error => {
            WorkflowTechnicalFitDeviceDiagnosticSeverity::Error
        }
    }
}

pub(crate) fn project_workflow_device_diagnostic(
    diagnostic: &WorkflowDeviceResolutionDiagnostic,
) -> RuntimeTechnicalFitDeviceDiagnostic {
    RuntimeTechnicalFitDeviceDiagnostic {
        code: project_workflow_device_diagnostic_code(diagnostic.code),
        severity: project_workflow_device_diagnostic_severity(diagnostic.severity),
        message: diagnostic.message.clone(),
        task_id: None,
        runtime_id: None,
        device_class: diagnostic
            .device_class
            .and_then(project_workflow_runtime_variant_device_class),
        device_id: diagnostic.device_id.clone(),
        runtime_variant_id: diagnostic.runtime_variant_id.clone(),
        backend_key: diagnostic.backend_id.clone(),
        model_id: None,
        evidence_key: None,
        requested_runtime_key: None,
    }
}

fn project_workflow_device_diagnostic_code(
    code: WorkflowDeviceResolutionDiagnosticCode,
) -> RuntimeTechnicalFitDeviceDiagnosticCode {
    match code {
        WorkflowDeviceResolutionDiagnosticCode::Unknown
        | WorkflowDeviceResolutionDiagnosticCode::NoValidCandidate => {
            RuntimeTechnicalFitDeviceDiagnosticCode::NoValidCandidate
        }
        WorkflowDeviceResolutionDiagnosticCode::InvalidDevicePolicy => {
            RuntimeTechnicalFitDeviceDiagnosticCode::InvalidDevicePolicy
        }
        WorkflowDeviceResolutionDiagnosticCode::InvalidDeviceId => {
            RuntimeTechnicalFitDeviceDiagnosticCode::InvalidDeviceId
        }
        WorkflowDeviceResolutionDiagnosticCode::InvalidRuntimeVariantId => {
            RuntimeTechnicalFitDeviceDiagnosticCode::InvalidRuntimeVariantId
        }
        WorkflowDeviceResolutionDiagnosticCode::InvalidBackendId => {
            RuntimeTechnicalFitDeviceDiagnosticCode::InvalidBackendId
        }
        WorkflowDeviceResolutionDiagnosticCode::CandidateUnavailable => {
            RuntimeTechnicalFitDeviceDiagnosticCode::CandidateUnavailable
        }
        WorkflowDeviceResolutionDiagnosticCode::ExplicitDeviceUnavailable => {
            RuntimeTechnicalFitDeviceDiagnosticCode::ExplicitDeviceUnavailable
        }
        WorkflowDeviceResolutionDiagnosticCode::AmbiguousAutoResolution => {
            RuntimeTechnicalFitDeviceDiagnosticCode::AmbiguousAutoResolution
        }
        WorkflowDeviceResolutionDiagnosticCode::BackendIncompatible => {
            RuntimeTechnicalFitDeviceDiagnosticCode::BackendIncompatible
        }
        WorkflowDeviceResolutionDiagnosticCode::UnsupportedDeviceClass => {
            RuntimeTechnicalFitDeviceDiagnosticCode::UnsupportedDeviceClass
        }
        WorkflowDeviceResolutionDiagnosticCode::MissingRuntimeVariant => {
            RuntimeTechnicalFitDeviceDiagnosticCode::MissingRuntimeVariant
        }
        WorkflowDeviceResolutionDiagnosticCode::LegacyDeviceRejected => {
            RuntimeTechnicalFitDeviceDiagnosticCode::LegacyDeviceRejected
        }
    }
}

fn project_workflow_device_diagnostic_severity(
    severity: WorkflowDeviceResolutionDiagnosticSeverity,
) -> RuntimeTechnicalFitDeviceDiagnosticSeverity {
    match severity {
        WorkflowDeviceResolutionDiagnosticSeverity::Unknown
        | WorkflowDeviceResolutionDiagnosticSeverity::Error => {
            RuntimeTechnicalFitDeviceDiagnosticSeverity::Error
        }
        WorkflowDeviceResolutionDiagnosticSeverity::Advisory => {
            RuntimeTechnicalFitDeviceDiagnosticSeverity::Advisory
        }
        WorkflowDeviceResolutionDiagnosticSeverity::Warning => {
            RuntimeTechnicalFitDeviceDiagnosticSeverity::Warning
        }
    }
}
