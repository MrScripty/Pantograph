//! Execution evidence normalization for package facts and backend capabilities.
//!
//! This module interprets model package facts against backend capability facts
//! without ranking candidates, reserving resources, or dispatching execution.

use serde::{Deserialize, Serialize};

use crate::backend::{
    canonical_backend_key, BackendCompatibilityReport, BackendCompatibilityRequest, BackendInfo,
};
use crate::model_contracts::{
    resolve_task_registry_entry, BackendHintLabel, InferenceTaskId, ModelArtifactKind,
    PackageFactStatus, ResolvedModelPackageFacts,
};

/// Input for side-effect-free execution evidence normalization.
#[derive(Debug, Clone)]
pub struct ExecutionEvidenceRequest<'a> {
    pub task_id: InferenceTaskId,
    pub package_facts: &'a ResolvedModelPackageFacts,
    pub backends: &'a [BackendInfo],
    pub graph_runtime_requirement: Option<&'a GraphRuntimeRequirement>,
}

/// A graph-provided runtime requirement after boundary validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRuntimeRequirement {
    runtime_key: String,
}

impl GraphRuntimeRequirement {
    /// Parse a graph runtime value into the canonical backend/runtime key used
    /// for candidate filtering.
    ///
    /// # Errors
    ///
    /// Returns `GraphRuntimeRequirementParseError` when the graph value is
    /// blank after trimming.
    pub fn parse(value: &str) -> Result<Self, GraphRuntimeRequirementParseError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(GraphRuntimeRequirementParseError::Blank);
        }

        Ok(Self {
            runtime_key: canonical_backend_key(trimmed),
        })
    }

    #[must_use]
    pub fn runtime_key(&self) -> &str {
        &self.runtime_key
    }
}

/// Parse error for graph runtime requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GraphRuntimeRequirementParseError {
    #[error("graph runtime requirement must not be blank")]
    Blank,
}

/// Result of normalizing execution evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ExecutionEvidenceReport {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<ExecutionBackendCandidateEvidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub records: Vec<ExecutionEvidenceRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ExecutionEvidenceDiagnostic>,
}

impl ExecutionEvidenceReport {
    #[must_use]
    pub fn has_executable_candidates(&self) -> bool {
        !self.candidates.is_empty()
    }
}

/// One executable backend candidate plus the factual compatibility report that
/// made it eligible.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ExecutionBackendCandidateEvidence {
    pub backend_key: String,
    pub task_id: InferenceTaskId,
    pub model_id: String,
    pub compatibility_report: BackendCompatibilityReport,
}

/// One normalized evidence fact with a role that prevents package labels from
/// being reused as executable backend decisions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ExecutionEvidenceRecord {
    pub role: ExecutionEvidenceRole,
    pub source: ExecutionEvidenceSource,
    pub key: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
}

/// Stable role for one execution-evidence fact.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEvidenceRole {
    ExecutableBackendCandidate,
    DependencyPackageEvidence,
    RuntimeCapabilityEvidence,
    GraphRuntimeConstraint,
    DisplayLabel,
}

/// Source family for one execution-evidence fact.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEvidenceSource {
    PackageFacts,
    BackendCapabilities,
    GraphRuntimeRequest,
    CompatibilityReport,
}

/// Stable diagnostic for execution evidence normalization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ExecutionEvidenceDiagnostic {
    pub code: ExecutionEvidenceDiagnosticCode,
    pub severity: ExecutionEvidenceDiagnosticSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_runtime_key: Option<String>,
}

/// Diagnostic code emitted by execution evidence normalization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEvidenceDiagnosticCode {
    UnsupportedTask,
    BackendUnavailable,
    MissingRuntimeCapability,
    RequiredPackageEvidenceUnavailable,
    BackendCompatibilityRejected,
    GraphRuntimeRequirementUnsatisfied,
}

/// Diagnostic severity emitted by execution evidence normalization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEvidenceDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Normalize package and backend facts into candidate evidence.
#[must_use]
pub fn normalize_execution_evidence(
    request: ExecutionEvidenceRequest<'_>,
) -> ExecutionEvidenceReport {
    let mut records = package_evidence_records(request.package_facts);
    if let Some(requirement) = request.graph_runtime_requirement {
        records.push(ExecutionEvidenceRecord {
            role: ExecutionEvidenceRole::GraphRuntimeConstraint,
            source: ExecutionEvidenceSource::GraphRuntimeRequest,
            key: "runtime".to_string(),
            value: requirement.runtime_key().to_string(),
            backend_key: None,
        });
    }

    let Some(task) = resolve_task_registry_entry(request.task_id.canonical_label()) else {
        return ExecutionEvidenceReport {
            candidates: Vec::new(),
            records,
            diagnostics: vec![ExecutionEvidenceDiagnostic {
                code: ExecutionEvidenceDiagnosticCode::UnsupportedTask,
                severity: ExecutionEvidenceDiagnosticSeverity::Error,
                message: format!(
                    "task {} is not registered for execution evidence normalization",
                    request.task_id.canonical_label()
                ),
                backend_key: None,
                requested_runtime_key: request
                    .graph_runtime_requirement
                    .map(|requirement| requirement.runtime_key().to_string()),
            }],
        };
    };

    let mut diagnostics = Vec::new();
    let mut candidates = Vec::new();
    let requested_runtime_key = request
        .graph_runtime_requirement
        .map(|requirement| requirement.runtime_key().to_string());

    for backend in request.backends {
        let backend_key = canonical_backend_key(&backend.backend_key);
        if request
            .graph_runtime_requirement
            .is_some_and(|requirement| requirement.runtime_key() != backend_key)
        {
            continue;
        }

        if !backend.available {
            diagnostics.push(ExecutionEvidenceDiagnostic {
                code: ExecutionEvidenceDiagnosticCode::BackendUnavailable,
                severity: ExecutionEvidenceDiagnosticSeverity::Error,
                message: format!("backend {} is not available", backend.backend_key),
                backend_key: Some(backend_key),
                requested_runtime_key: requested_runtime_key.clone(),
            });
            continue;
        }

        records.extend(runtime_capability_records(backend, &backend_key));
        if let Some(diagnostic) = required_package_evidence_diagnostic(
            request.package_facts,
            &backend_key,
            &requested_runtime_key,
        ) {
            diagnostics.push(diagnostic);
            continue;
        }

        if !backend
            .capabilities
            .facts
            .runtime_variants
            .iter()
            .any(|variant| variant.available)
        {
            diagnostics.push(ExecutionEvidenceDiagnostic {
                code: ExecutionEvidenceDiagnosticCode::MissingRuntimeCapability,
                severity: ExecutionEvidenceDiagnosticSeverity::Error,
                message: format!(
                    "backend {} has no available runtime variant facts",
                    backend.backend_key
                ),
                backend_key: Some(backend_key),
                requested_runtime_key: requested_runtime_key.clone(),
            });
            continue;
        }

        let compatibility_report = backend.capabilities.check_model_compatibility(
            Some(&backend_key),
            BackendCompatibilityRequest::new(&task, request.package_facts),
        );
        if !compatibility_report.compatible {
            diagnostics.push(ExecutionEvidenceDiagnostic {
                code: ExecutionEvidenceDiagnosticCode::BackendCompatibilityRejected,
                severity: ExecutionEvidenceDiagnosticSeverity::Info,
                message: format!(
                    "backend {} is not compatible with model {} for task {}",
                    backend.backend_key,
                    request.package_facts.model_ref.model_id,
                    request.task_id.canonical_label()
                ),
                backend_key: Some(backend_key),
                requested_runtime_key: requested_runtime_key.clone(),
            });
            continue;
        }

        records.push(ExecutionEvidenceRecord {
            role: ExecutionEvidenceRole::ExecutableBackendCandidate,
            source: ExecutionEvidenceSource::CompatibilityReport,
            key: "backend_key".to_string(),
            value: backend_key.clone(),
            backend_key: Some(backend_key.clone()),
        });
        candidates.push(ExecutionBackendCandidateEvidence {
            backend_key,
            task_id: request.task_id.clone(),
            model_id: request.package_facts.model_ref.model_id.clone(),
            compatibility_report,
        });
    }

    if let Some(requirement) = request.graph_runtime_requirement {
        if candidates.is_empty() {
            diagnostics.push(ExecutionEvidenceDiagnostic {
                code: ExecutionEvidenceDiagnosticCode::GraphRuntimeRequirementUnsatisfied,
                severity: ExecutionEvidenceDiagnosticSeverity::Error,
                message: format!(
                    "graph runtime requirement {} did not match a validated executable candidate",
                    requirement.runtime_key()
                ),
                backend_key: None,
                requested_runtime_key: Some(requirement.runtime_key().to_string()),
            });
        }
    }

    ExecutionEvidenceReport {
        candidates,
        records,
        diagnostics,
    }
}

fn package_evidence_records(
    package_facts: &ResolvedModelPackageFacts,
) -> Vec<ExecutionEvidenceRecord> {
    let mut records = vec![
        ExecutionEvidenceRecord {
            role: ExecutionEvidenceRole::DependencyPackageEvidence,
            source: ExecutionEvidenceSource::PackageFacts,
            key: "artifact_kind".to_string(),
            value: artifact_kind_label(&package_facts.artifact.artifact_kind).to_string(),
            backend_key: None,
        },
        ExecutionEvidenceRecord {
            role: ExecutionEvidenceRole::DisplayLabel,
            source: ExecutionEvidenceSource::PackageFacts,
            key: "model_id".to_string(),
            value: package_facts.model_ref.model_id.clone(),
            backend_key: None,
        },
    ];

    if package_facts.diffusers.is_some() {
        records.push(ExecutionEvidenceRecord {
            role: ExecutionEvidenceRole::DependencyPackageEvidence,
            source: ExecutionEvidenceSource::PackageFacts,
            key: "diffusers_package_facts".to_string(),
            value: "present".to_string(),
            backend_key: None,
        });
    }

    records.extend(package_facts.backend_hints.accepted.iter().map(|hint| {
        ExecutionEvidenceRecord {
            role: ExecutionEvidenceRole::DependencyPackageEvidence,
            source: ExecutionEvidenceSource::PackageFacts,
            key: "backend_hint".to_string(),
            value: backend_hint_label(*hint).to_string(),
            backend_key: None,
        }
    }));

    records
}

fn runtime_capability_records(
    backend: &BackendInfo,
    backend_key: &str,
) -> Vec<ExecutionEvidenceRecord> {
    backend
        .capabilities
        .facts
        .runtime_variants
        .iter()
        .map(|variant| ExecutionEvidenceRecord {
            role: ExecutionEvidenceRole::RuntimeCapabilityEvidence,
            source: ExecutionEvidenceSource::BackendCapabilities,
            key: "runtime_variant_id".to_string(),
            value: variant.runtime_variant_id.as_str().to_string(),
            backend_key: Some(backend_key.to_string()),
        })
        .collect()
}

fn required_package_evidence_diagnostic(
    package_facts: &ResolvedModelPackageFacts,
    backend_key: &str,
    requested_runtime_key: &Option<String>,
) -> Option<ExecutionEvidenceDiagnostic> {
    if package_facts.artifact.artifact_kind != ModelArtifactKind::DiffusersBundle {
        return None;
    }

    if package_facts
        .diffusers
        .as_ref()
        .is_some_and(|diffusers| diffusers.status == PackageFactStatus::Present)
    {
        return None;
    }

    Some(ExecutionEvidenceDiagnostic {
        code: ExecutionEvidenceDiagnosticCode::RequiredPackageEvidenceUnavailable,
        severity: ExecutionEvidenceDiagnosticSeverity::Error,
        message: "Diffusers bundle execution requires present Diffusers package evidence"
            .to_string(),
        backend_key: Some(backend_key.to_string()),
        requested_runtime_key: requested_runtime_key.clone(),
    })
}

fn artifact_kind_label(kind: &ModelArtifactKind) -> &'static str {
    match kind {
        ModelArtifactKind::Gguf => "gguf",
        ModelArtifactKind::HfCompatibleDirectory => "hf_compatible_directory",
        ModelArtifactKind::Safetensors => "safetensors",
        ModelArtifactKind::DiffusersBundle => "diffusers_bundle",
        ModelArtifactKind::Onnx => "onnx",
        ModelArtifactKind::Adapter => "adapter",
        ModelArtifactKind::Shard => "shard",
        ModelArtifactKind::Unknown => "unknown",
    }
}

fn backend_hint_label(hint: BackendHintLabel) -> &'static str {
    match hint {
        BackendHintLabel::Transformers => "transformers",
        BackendHintLabel::LlamaCpp => "llama.cpp",
        BackendHintLabel::Vllm => "vllm",
        BackendHintLabel::Mlx => "mlx",
        BackendHintLabel::Candle => "candle",
        BackendHintLabel::Diffusers => "diffusers",
        BackendHintLabel::OnnxRuntime => "onnxruntime",
    }
}

#[cfg(test)]
#[path = "execution_evidence_tests.rs"]
mod execution_evidence_tests;
