//! Backend compatibility checks against resolved model package facts.
//!
//! These checks are factual and advisory. They do not select runtimes, reserve
//! resources, inspect scheduler queues, or decide workflow admission.

use serde::{Deserialize, Serialize};

use crate::model_contracts::{
    CacheGenerationOptions, InferenceLifecyclePhase, InferenceTaskId, ModelArtifactKind,
    ModelValidationState, OptionCompatibilityDiagnostic, OptionSupportState, PackageFactStatus,
    ProcessorComponentKind, ResolvedModelPackageFacts, TaskRegistryEntry,
};
use crate::types::{
    looks_like_local_artifact_ref, InferenceCompatibilityIssueSummary,
    InferenceCompatibilityReportSummary,
};

use super::{BackendCapabilities, BackendComponentCapability, BackendFeatureSupport};

/// Requested execution features used by backend compatibility checks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BackendCompatibilityOptions {
    pub streaming: bool,
    pub device_selection: bool,
    pub external_connection: bool,
    pub cache: CacheGenerationOptions,
}

/// Factual compatibility input for one backend/model/task boundary check.
#[derive(Debug, Clone)]
pub struct BackendCompatibilityRequest<'a> {
    pub task: &'a TaskRegistryEntry,
    pub package_facts: &'a ResolvedModelPackageFacts,
    pub options: BackendCompatibilityOptions,
}

impl<'a> BackendCompatibilityRequest<'a> {
    #[must_use]
    pub fn new(task: &'a TaskRegistryEntry, package_facts: &'a ResolvedModelPackageFacts) -> Self {
        Self {
            task,
            package_facts,
            options: BackendCompatibilityOptions::default(),
        }
    }

    #[must_use]
    pub fn with_options(mut self, options: BackendCompatibilityOptions) -> Self {
        self.options = options;
        self
    }
}

/// Result of checking static backend facts against resolved package facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BackendCompatibilityReport {
    pub compatible: bool,
    pub task: BackendCompatibilityStatus,
    pub model_source: BackendCompatibilityStatus,
    pub preprocessing: BackendCompatibilityStatus,
    pub postprocessing: BackendCompatibilityStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<BackendCompatibilityIssue>,
}

/// Coarse factual status for one compatibility dimension.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendCompatibilityStatus {
    Supported,
    Unsupported,
    Unknown,
}

/// One bounded compatibility issue suitable for diagnostics projection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct BackendCompatibilityIssue {
    pub kind: BackendCompatibilityIssueKind,
    pub phase: InferenceLifecyclePhase,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// Stable issue labels for factual backend/model compatibility reports.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendCompatibilityIssueKind {
    ContractVersionMismatch,
    InvalidModelArtifact,
    UnsupportedTask,
    TaskEvidenceMismatch,
    UnsupportedModelArtifact,
    UnsupportedBackendHint,
    CustomCodeUnsupported,
    MissingPreprocessingComponent,
    MissingPostprocessingComponent,
    UnsupportedOption,
}

impl BackendCompatibilityReport {
    /// Convert this backend-owned report into lifecycle-safe diagnostic
    /// metadata without exposing backend internals or scheduler policy.
    #[must_use]
    pub fn to_inference_compatibility_report_summary(&self) -> InferenceCompatibilityReportSummary {
        InferenceCompatibilityReportSummary {
            status: compatibility_report_status_label(self).to_string(),
            compatible: self.compatible,
            task: compatibility_status_label(self.task).to_string(),
            model_source: compatibility_status_label(self.model_source).to_string(),
            preprocessing: compatibility_status_label(self.preprocessing).to_string(),
            postprocessing: compatibility_status_label(self.postprocessing).to_string(),
        }
    }

    /// Convert issue facts into bounded lifecycle-safe diagnostic metadata.
    #[must_use]
    pub fn to_inference_compatibility_issue_summaries(
        &self,
        limit: usize,
    ) -> Vec<InferenceCompatibilityIssueSummary> {
        self.issues
            .iter()
            .take(limit)
            .map(|issue| InferenceCompatibilityIssueSummary {
                kind: compatibility_issue_kind_label(&issue.kind).to_string(),
                phase: issue.phase.clone(),
                message: issue.message.clone(),
                model_id: issue.model_id.clone(),
                path: lifecycle_safe_compatibility_issue_path(issue),
            })
            .collect()
    }
}

fn lifecycle_safe_compatibility_issue_path(issue: &BackendCompatibilityIssue) -> Option<String> {
    let has_stable_model_id = issue
        .model_id
        .as_deref()
        .is_some_and(|model_id| !model_id.trim().is_empty());

    issue.path.as_ref().and_then(|path| {
        if has_stable_model_id && looks_like_local_artifact_ref(path) {
            None
        } else {
            Some(path.clone())
        }
    })
}

impl BackendCapabilities {
    /// Check whether this backend's static facts can consume a resolved model
    /// package for a canonical task and option set.
    #[must_use]
    pub fn check_model_compatibility(
        &self,
        backend_key: Option<&str>,
        request: BackendCompatibilityRequest<'_>,
    ) -> BackendCompatibilityReport {
        let mut issues = Vec::new();
        let task = self.check_task_compatibility(&request, &mut issues);
        let model_source = self.check_model_source_compatibility(&request, &mut issues);
        let preprocessing = self.check_component_compatibility(
            self.facts.preprocessing,
            InferenceLifecyclePhase::Preprocessing,
            BackendCompatibilityIssueKind::MissingPreprocessingComponent,
            &request,
            &mut issues,
        );
        let postprocessing = self.check_component_compatibility(
            self.facts.postprocessing,
            InferenceLifecyclePhase::Postprocessing,
            BackendCompatibilityIssueKind::MissingPostprocessingComponent,
            &request,
            &mut issues,
        );
        let option_diagnostics = self.option_compatibility_diagnostics(backend_key, &request);
        issues.extend(option_diagnostics.iter().filter_map(|diagnostic| {
            matches!(
                &diagnostic.state,
                OptionSupportState::Unsupported | OptionSupportState::Rejected
            )
            .then(|| BackendCompatibilityIssue {
                kind: BackendCompatibilityIssueKind::UnsupportedOption,
                phase: InferenceLifecyclePhase::TaskValidation,
                message: diagnostic.message.clone().unwrap_or_else(|| {
                    format!("option {} is not supported", diagnostic.option_path)
                }),
                model_id: Some(request.package_facts.model_ref.model_id.clone()),
                path: None,
            })
        }));
        let compatible = [task, model_source, preprocessing, postprocessing]
            .into_iter()
            .all(|status| status == BackendCompatibilityStatus::Supported)
            && option_diagnostics.iter().all(|diagnostic| {
                !matches!(
                    &diagnostic.state,
                    OptionSupportState::Unsupported | OptionSupportState::Rejected
                )
            });

        BackendCompatibilityReport {
            compatible,
            task,
            model_source,
            preprocessing,
            postprocessing,
            option_diagnostics,
            issues,
        }
    }

    fn check_task_compatibility(
        &self,
        request: &BackendCompatibilityRequest<'_>,
        issues: &mut Vec<BackendCompatibilityIssue>,
    ) -> BackendCompatibilityStatus {
        if !self.facts.supports_task(request.task.task_id.clone()) {
            issues.push(compatibility_issue(
                request,
                BackendCompatibilityIssueKind::UnsupportedTask,
                InferenceLifecyclePhase::TaskValidation,
                format!("backend does not declare task {:?}", request.task.task_id),
                None,
            ));
            return BackendCompatibilityStatus::Unsupported;
        }

        if !request
            .task
            .matches_task_evidence(&request.package_facts.task)
            || !request
                .task
                .matches_modality_evidence(&request.package_facts.task)
        {
            issues.push(compatibility_issue(
                request,
                BackendCompatibilityIssueKind::TaskEvidenceMismatch,
                InferenceLifecyclePhase::TaskValidation,
                "package task evidence does not match the requested task registry entry"
                    .to_string(),
                None,
            ));
            return BackendCompatibilityStatus::Unsupported;
        }

        BackendCompatibilityStatus::Supported
    }

    fn check_model_source_compatibility(
        &self,
        request: &BackendCompatibilityRequest<'_>,
        issues: &mut Vec<BackendCompatibilityIssue>,
    ) -> BackendCompatibilityStatus {
        let package = request.package_facts;
        let mut status = BackendCompatibilityStatus::Supported;

        if !package.uses_current_contract() {
            issues.push(compatibility_issue(
                request,
                BackendCompatibilityIssueKind::ContractVersionMismatch,
                InferenceLifecyclePhase::ModelPackageResolution,
                "model package facts use an unsupported contract version".to_string(),
                None,
            ));
            status = BackendCompatibilityStatus::Unsupported;
        }

        if matches!(
            &package.artifact.validation_state,
            ModelValidationState::Invalid | ModelValidationState::Unknown
        ) {
            issues.push(compatibility_issue(
                request,
                BackendCompatibilityIssueKind::InvalidModelArtifact,
                InferenceLifecyclePhase::ModelPackageResolution,
                "model artifact is not valid".to_string(),
                Some(package.artifact.entry_path.clone()),
            ));
            status = BackendCompatibilityStatus::Unsupported;
        }

        if self.facts.model_sources.artifact_kinds.is_empty() {
            status = BackendCompatibilityStatus::Unknown;
        } else if !self
            .facts
            .model_sources
            .artifact_kinds
            .contains(&package.artifact.artifact_kind)
        {
            issues.push(compatibility_issue(
                request,
                BackendCompatibilityIssueKind::UnsupportedModelArtifact,
                InferenceLifecyclePhase::ModelPackageResolution,
                format!(
                    "backend does not declare support for {:?} artifacts",
                    package.artifact.artifact_kind
                ),
                Some(package.artifact.entry_path.clone()),
            ));
            status = BackendCompatibilityStatus::Unsupported;
        }

        if !package.backend_hints.accepted.is_empty()
            && !self.facts.model_sources.backend_hints.is_empty()
            && !package
                .backend_hints
                .accepted
                .iter()
                .any(|hint| self.facts.model_sources.backend_hints.contains(hint))
        {
            issues.push(compatibility_issue(
                request,
                BackendCompatibilityIssueKind::UnsupportedBackendHint,
                InferenceLifecyclePhase::ModelPackageResolution,
                "package backend hints do not overlap backend source capabilities".to_string(),
                Some(package.artifact.entry_path.clone()),
            ));
            status = BackendCompatibilityStatus::Unsupported;
        }

        if package.custom_code.requires_custom_code {
            match self.facts.model_sources.custom_code {
                BackendFeatureSupport::Supported => {}
                BackendFeatureSupport::Unsupported => {
                    issues.push(compatibility_issue(
                        request,
                        BackendCompatibilityIssueKind::CustomCodeUnsupported,
                        InferenceLifecyclePhase::ModelPackageResolution,
                        "package requires custom code but backend does not support it".to_string(),
                        Some(package.artifact.entry_path.clone()),
                    ));
                    status = BackendCompatibilityStatus::Unsupported;
                }
                BackendFeatureSupport::Unknown => {
                    status = BackendCompatibilityStatus::Unknown;
                }
            }
        }

        status
    }

    fn check_component_compatibility(
        &self,
        capability: BackendComponentCapability,
        phase: InferenceLifecyclePhase,
        issue_kind: BackendCompatibilityIssueKind,
        request: &BackendCompatibilityRequest<'_>,
        issues: &mut Vec<BackendCompatibilityIssue>,
    ) -> BackendCompatibilityStatus {
        match capability {
            BackendComponentCapability::NotRequired
            | BackendComponentCapability::BackendManaged => BackendCompatibilityStatus::Supported,
            BackendComponentCapability::Unsupported => {
                issues.push(compatibility_issue(
                    request,
                    issue_kind,
                    phase,
                    "backend declares this lifecycle component unsupported".to_string(),
                    Some(request.package_facts.artifact.entry_path.clone()),
                ));
                BackendCompatibilityStatus::Unsupported
            }
            BackendComponentCapability::Unknown => BackendCompatibilityStatus::Unknown,
            BackendComponentCapability::RequiresPackageComponent => {
                if package_components_available_for_task(request.package_facts, request.task) {
                    BackendCompatibilityStatus::Supported
                } else {
                    issues.push(compatibility_issue(
                        request,
                        issue_kind,
                        phase,
                        "required model package component is missing, invalid, or unsupported"
                            .to_string(),
                        Some(request.package_facts.artifact.entry_path.clone()),
                    ));
                    BackendCompatibilityStatus::Unsupported
                }
            }
        }
    }

    fn option_compatibility_diagnostics(
        &self,
        backend_key: Option<&str>,
        request: &BackendCompatibilityRequest<'_>,
    ) -> Vec<OptionCompatibilityDiagnostic> {
        let mut diagnostics = Vec::new();
        push_feature_option_diagnostic(
            &mut diagnostics,
            backend_key,
            "streaming",
            request.options.streaming,
            self.facts.features.streaming,
        );
        push_feature_option_diagnostic(
            &mut diagnostics,
            backend_key,
            "device_selection",
            request.options.device_selection,
            self.facts.features.device_selection,
        );
        push_feature_option_diagnostic(
            &mut diagnostics,
            backend_key,
            "external_connection",
            request.options.external_connection,
            self.facts.features.external_connection,
        );
        push_feature_option_diagnostic(
            &mut diagnostics,
            backend_key,
            "cache.use_cache",
            request.options.cache.use_cache == Some(true),
            self.facts.features.kv_cache,
        );
        push_feature_option_diagnostic(
            &mut diagnostics,
            backend_key,
            "cache.kv_cache_checkpoint_requested",
            request.options.cache.kv_cache_checkpoint_requested == Some(true),
            self.facts.features.kv_cache,
        );
        diagnostics
    }
}

fn compatibility_status_label(status: BackendCompatibilityStatus) -> &'static str {
    match status {
        BackendCompatibilityStatus::Supported => "supported",
        BackendCompatibilityStatus::Unsupported => "unsupported",
        BackendCompatibilityStatus::Unknown => "unknown",
    }
}

fn compatibility_report_status_label(report: &BackendCompatibilityReport) -> &'static str {
    if report.compatible {
        return "accepted";
    }

    let statuses = [
        report.task,
        report.model_source,
        report.preprocessing,
        report.postprocessing,
    ];
    if statuses
        .iter()
        .any(|status| *status == BackendCompatibilityStatus::Unsupported)
    {
        "rejected"
    } else {
        "degraded"
    }
}

fn compatibility_issue_kind_label(kind: &BackendCompatibilityIssueKind) -> &'static str {
    match kind {
        BackendCompatibilityIssueKind::ContractVersionMismatch => "contract_version_mismatch",
        BackendCompatibilityIssueKind::InvalidModelArtifact => "invalid_model_artifact",
        BackendCompatibilityIssueKind::UnsupportedTask => "unsupported_task",
        BackendCompatibilityIssueKind::TaskEvidenceMismatch => "task_evidence_mismatch",
        BackendCompatibilityIssueKind::UnsupportedModelArtifact => "unsupported_model_artifact",
        BackendCompatibilityIssueKind::UnsupportedBackendHint => "unsupported_backend_hint",
        BackendCompatibilityIssueKind::CustomCodeUnsupported => "custom_code_unsupported",
        BackendCompatibilityIssueKind::MissingPreprocessingComponent => {
            "missing_preprocessing_component"
        }
        BackendCompatibilityIssueKind::MissingPostprocessingComponent => {
            "missing_postprocessing_component"
        }
        BackendCompatibilityIssueKind::UnsupportedOption => "unsupported_option",
    }
}

fn compatibility_issue(
    request: &BackendCompatibilityRequest<'_>,
    kind: BackendCompatibilityIssueKind,
    phase: InferenceLifecyclePhase,
    message: String,
    path: Option<String>,
) -> BackendCompatibilityIssue {
    BackendCompatibilityIssue {
        kind,
        phase,
        message,
        model_id: Some(request.package_facts.model_ref.model_id.clone()),
        path,
    }
}

fn push_feature_option_diagnostic(
    diagnostics: &mut Vec<OptionCompatibilityDiagnostic>,
    backend_key: Option<&str>,
    option_path: &str,
    requested: bool,
    support: BackendFeatureSupport,
) {
    if !requested {
        return;
    }

    let (state, message) = match support {
        BackendFeatureSupport::Supported => (
            OptionSupportState::Honored,
            format!("option {option_path} is supported"),
        ),
        BackendFeatureSupport::Unsupported => (
            OptionSupportState::Unsupported,
            format!("option {option_path} is not supported by this backend"),
        ),
        BackendFeatureSupport::Unknown => (
            OptionSupportState::Unsupported,
            format!("option {option_path} support has not been declared by this backend"),
        ),
    };

    diagnostics.push(OptionCompatibilityDiagnostic {
        option_path: option_path.to_string(),
        state,
        backend_key: backend_key.map(ToOwned::to_owned),
        message: Some(message),
    });
}

fn package_components_available_for_task(
    package: &ResolvedModelPackageFacts,
    task: &TaskRegistryEntry,
) -> bool {
    if matches!(&package.artifact.artifact_kind, ModelArtifactKind::Gguf) {
        return true;
    }

    match task.task_id {
        InferenceTaskId::TextGeneration
        | InferenceTaskId::ChatCompletion
        | InferenceTaskId::Embedding
        | InferenceTaskId::Rerank => {
            component_family_available(package, &[ProcessorComponentKind::Tokenizer])
        }
        InferenceTaskId::ImageGeneration => component_family_available(
            package,
            &[
                ProcessorComponentKind::Processor,
                ProcessorComponentKind::ImageProcessor,
                ProcessorComponentKind::ModelIndex,
            ],
        ),
        InferenceTaskId::ImageUnderstanding => component_family_available(
            package,
            &[
                ProcessorComponentKind::Processor,
                ProcessorComponentKind::ImageProcessor,
            ],
        ),
        InferenceTaskId::DepthEstimation => component_family_available(
            package,
            &[
                ProcessorComponentKind::Processor,
                ProcessorComponentKind::ImageProcessor,
            ],
        ),
        InferenceTaskId::AudioTranscription => component_family_available(
            package,
            &[
                ProcessorComponentKind::Processor,
                ProcessorComponentKind::FeatureExtractor,
                ProcessorComponentKind::AudioFeatureExtractor,
            ],
        ),
        InferenceTaskId::VideoUnderstanding => component_family_available(
            package,
            &[
                ProcessorComponentKind::Processor,
                ProcessorComponentKind::VideoProcessor,
            ],
        ),
        InferenceTaskId::MultimodalGeneration => component_family_available(
            package,
            &[
                ProcessorComponentKind::Processor,
                ProcessorComponentKind::ImageProcessor,
                ProcessorComponentKind::VideoProcessor,
                ProcessorComponentKind::AudioFeatureExtractor,
                ProcessorComponentKind::FeatureExtractor,
            ],
        ),
        InferenceTaskId::Unknown => false,
    }
}

fn component_family_available(
    package: &ResolvedModelPackageFacts,
    kinds: &[ProcessorComponentKind],
) -> bool {
    let mut present = false;
    for component in &package.components {
        if !kinds.contains(&component.kind) {
            continue;
        }
        match &component.status {
            PackageFactStatus::Present => present = true,
            PackageFactStatus::Missing
            | PackageFactStatus::Invalid
            | PackageFactStatus::Unsupported => return false,
            PackageFactStatus::Uninspected => {}
        }
    }
    present
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_contracts::{
        BackendHintLabel, InferenceModality, SupportTier, TaskExecutionBehavior, TaskFamily,
        TaskModalitySignature, TaskStreamingSupport,
    };
    use crate::{
        BackendCapabilityFacts, BackendFeatureCapabilityFacts, BackendModelSourceCapabilityFacts,
        BackendTaskCapability,
    };

    fn text_generation_task() -> TaskRegistryEntry {
        TaskRegistryEntry {
            task_id: InferenceTaskId::TextGeneration,
            aliases: vec!["text-generation".to_string()],
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Text],
                vec![InferenceModality::Text],
            ),
            result_family: "text".to_string(),
            task_family: TaskFamily::Generative,
            execution_behavior: TaskExecutionBehavior::Generates,
            streaming_support: TaskStreamingSupport::BackendDependent,
            support_tier: SupportTier::Stable,
            required_components: Vec::new(),
            upstream_task_ids: Vec::new(),
        }
    }

    fn image_generation_task() -> TaskRegistryEntry {
        TaskRegistryEntry {
            task_id: InferenceTaskId::ImageGeneration,
            aliases: vec!["text-to-image".to_string()],
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Text],
                vec![InferenceModality::Image],
            ),
            result_family: "generated_image".to_string(),
            task_family: TaskFamily::Generative,
            execution_behavior: TaskExecutionBehavior::Generates,
            streaming_support: TaskStreamingSupport::Unsupported,
            support_tier: SupportTier::Experimental,
            required_components: Vec::new(),
            upstream_task_ids: vec!["text-to-image".to_string()],
        }
    }

    fn embedding_task() -> TaskRegistryEntry {
        TaskRegistryEntry {
            task_id: InferenceTaskId::Embedding,
            aliases: vec!["feature-extraction".to_string()],
            modality_signature: TaskModalitySignature::new(
                vec![InferenceModality::Text],
                vec![InferenceModality::Embedding],
            ),
            result_family: "embedding_vector".to_string(),
            task_family: TaskFamily::Embedding,
            execution_behavior: TaskExecutionBehavior::ExtractsFeatures,
            streaming_support: TaskStreamingSupport::Unsupported,
            support_tier: SupportTier::Stable,
            required_components: Vec::new(),
            upstream_task_ids: vec!["feature-extraction".to_string()],
        }
    }

    fn backend_for_text_generation() -> BackendCapabilities {
        BackendCapabilities {
            facts: BackendCapabilityFacts {
                tasks: vec![BackendTaskCapability::stable(
                    InferenceTaskId::TextGeneration,
                    vec![InferenceModality::Text],
                    vec![InferenceModality::Text],
                )],
                preprocessing: BackendComponentCapability::RequiresPackageComponent,
                postprocessing: BackendComponentCapability::BackendManaged,
                model_sources: BackendModelSourceCapabilityFacts {
                    artifact_kinds: vec![
                        ModelArtifactKind::Gguf,
                        ModelArtifactKind::HfCompatibleDirectory,
                    ],
                    backend_hints: vec![BackendHintLabel::LlamaCpp, BackendHintLabel::Transformers],
                    custom_code: BackendFeatureSupport::Supported,
                },
                features: BackendFeatureCapabilityFacts {
                    streaming: BackendFeatureSupport::Supported,
                    device_selection: BackendFeatureSupport::Supported,
                    external_connection: BackendFeatureSupport::Unsupported,
                    kv_cache: BackendFeatureSupport::Supported,
                },
            },
            ..BackendCapabilities::default()
        }
    }

    fn backend_for_embedding_source(
        artifact_kind: ModelArtifactKind,
        backend_hint: BackendHintLabel,
    ) -> BackendCapabilities {
        BackendCapabilities {
            facts: BackendCapabilityFacts {
                tasks: vec![BackendTaskCapability::stable(
                    InferenceTaskId::Embedding,
                    vec![InferenceModality::Text],
                    vec![InferenceModality::Embedding],
                )],
                preprocessing: BackendComponentCapability::RequiresPackageComponent,
                postprocessing: BackendComponentCapability::BackendManaged,
                model_sources: BackendModelSourceCapabilityFacts {
                    artifact_kinds: vec![artifact_kind],
                    backend_hints: vec![backend_hint],
                    custom_code: BackendFeatureSupport::Unsupported,
                },
                features: BackendFeatureCapabilityFacts::default(),
            },
            ..BackendCapabilities::default()
        }
    }

    fn backend_for_diffusers_image_generation() -> BackendCapabilities {
        BackendCapabilities {
            facts: BackendCapabilityFacts {
                tasks: vec![BackendTaskCapability {
                    task_id: InferenceTaskId::ImageGeneration,
                    support_tier: SupportTier::Experimental,
                    modality_signature: TaskModalitySignature::new(
                        vec![InferenceModality::Text],
                        vec![InferenceModality::Image],
                    ),
                }],
                preprocessing: BackendComponentCapability::RequiresPackageComponent,
                postprocessing: BackendComponentCapability::BackendManaged,
                model_sources: BackendModelSourceCapabilityFacts {
                    artifact_kinds: vec![ModelArtifactKind::DiffusersBundle],
                    backend_hints: vec![BackendHintLabel::Diffusers],
                    custom_code: BackendFeatureSupport::Unsupported,
                },
                features: BackendFeatureCapabilityFacts::default(),
            },
            ..BackendCapabilities::default()
        }
    }

    fn fixture(raw: &str) -> ResolvedModelPackageFacts {
        serde_json::from_str(raw).expect("fixture should decode")
    }

    #[test]
    fn accepts_supported_package_task_and_options() {
        let package = fixture(include_str!(
            "../../tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
        ));
        let task = text_generation_task();
        let report = backend_for_text_generation().check_model_compatibility(
            Some("llama_cpp"),
            BackendCompatibilityRequest::new(&task, &package).with_options(
                BackendCompatibilityOptions {
                    streaming: true,
                    cache: CacheGenerationOptions {
                        use_cache: Some(true),
                        kv_cache_checkpoint_requested: Some(true),
                    },
                    ..BackendCompatibilityOptions::default()
                },
            ),
        );

        assert!(report.compatible);
        assert_eq!(report.task, BackendCompatibilityStatus::Supported);
        assert_eq!(report.model_source, BackendCompatibilityStatus::Supported);
        assert!(report.issues.is_empty());
        assert!(report
            .option_diagnostics
            .iter()
            .all(|diagnostic| matches!(&diagnostic.state, OptionSupportState::Honored)));
    }

    #[test]
    fn rejects_missing_required_preprocessing_component() {
        let package = fixture(include_str!(
            "../../tests/fixtures/inference_package_facts/missing_tokenizer_package_facts.json"
        ));
        let task = text_generation_task();
        let report = backend_for_text_generation().check_model_compatibility(
            Some("transformers"),
            BackendCompatibilityRequest::new(&task, &package),
        );

        assert!(!report.compatible);
        assert_eq!(
            report.preprocessing,
            BackendCompatibilityStatus::Unsupported
        );
        assert!(report.issues.iter().any(|issue| {
            issue.kind == BackendCompatibilityIssueKind::MissingPreprocessingComponent
                && issue.phase == InferenceLifecyclePhase::Preprocessing
        }));
    }

    #[test]
    fn diffusers_bundle_model_index_satisfies_image_generation_preprocessing() {
        let package = fixture(include_str!(
            "../../tests/fixtures/inference_package_facts/diffusers_bundle_package_facts.json"
        ));
        let task = image_generation_task();
        let report = backend_for_diffusers_image_generation().check_model_compatibility(
            Some("diffusers"),
            BackendCompatibilityRequest::new(&task, &package),
        );

        assert!(report.compatible, "unexpected issues: {:?}", report.issues);
        assert_eq!(report.task, BackendCompatibilityStatus::Supported);
        assert_eq!(report.model_source, BackendCompatibilityStatus::Supported);
        assert_eq!(report.preprocessing, BackendCompatibilityStatus::Supported);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn safetensors_and_onnx_embedding_fixtures_match_declared_backend_sources() {
        let task = embedding_task();

        for (raw, artifact_kind, backend_hint, backend_key) in [
            (
                include_str!(
                    "../../tests/fixtures/inference_package_facts/safetensors_package_facts.json"
                ),
                ModelArtifactKind::Safetensors,
                BackendHintLabel::Candle,
                "candle",
            ),
            (
                include_str!(
                    "../../tests/fixtures/inference_package_facts/onnx_package_facts.json"
                ),
                ModelArtifactKind::Onnx,
                BackendHintLabel::OnnxRuntime,
                "onnxruntime",
            ),
        ] {
            let package = fixture(raw);
            let report = backend_for_embedding_source(artifact_kind, backend_hint)
                .check_model_compatibility(
                    Some(backend_key),
                    BackendCompatibilityRequest::new(&task, &package),
                );

            assert!(report.compatible, "unexpected issues: {:?}", report.issues);
            assert_eq!(report.task, BackendCompatibilityStatus::Supported);
            assert_eq!(report.model_source, BackendCompatibilityStatus::Supported);
            assert_eq!(report.preprocessing, BackendCompatibilityStatus::Supported);
            assert!(report.issues.is_empty());
        }
    }

    #[test]
    fn hf_candle_embedding_fixture_matches_declared_backend_source() {
        let package = fixture(include_str!(
            "../../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
        ));
        let task = embedding_task();
        let report = backend_for_embedding_source(
            ModelArtifactKind::HfCompatibleDirectory,
            BackendHintLabel::Candle,
        )
        .check_model_compatibility(
            Some("candle"),
            BackendCompatibilityRequest::new(&task, &package),
        );

        assert!(report.compatible, "unexpected issues: {:?}", report.issues);
        assert_eq!(report.task, BackendCompatibilityStatus::Supported);
        assert_eq!(report.model_source, BackendCompatibilityStatus::Supported);
        assert_eq!(report.preprocessing, BackendCompatibilityStatus::Supported);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn rejects_unsupported_requested_options() {
        let package = fixture(include_str!(
            "../../tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
        ));
        let task = text_generation_task();
        let mut backend = backend_for_text_generation();
        backend.facts.features.kv_cache = BackendFeatureSupport::Unsupported;

        let report = backend.check_model_compatibility(
            Some("llama_cpp"),
            BackendCompatibilityRequest::new(&task, &package).with_options(
                BackendCompatibilityOptions {
                    cache: CacheGenerationOptions {
                        use_cache: Some(true),
                        kv_cache_checkpoint_requested: Some(true),
                    },
                    ..BackendCompatibilityOptions::default()
                },
            ),
        );

        assert!(!report.compatible);
        assert!(report.option_diagnostics.iter().any(|diagnostic| {
            diagnostic.option_path == "cache.use_cache"
                && matches!(&diagnostic.state, OptionSupportState::Unsupported)
        }));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == BackendCompatibilityIssueKind::UnsupportedOption));
    }

    #[test]
    fn compatibility_report_projects_lifecycle_safe_summary() {
        let package = fixture(include_str!(
            "../../tests/fixtures/inference_package_facts/missing_tokenizer_package_facts.json"
        ));
        let task = text_generation_task();
        let report = backend_for_text_generation().check_model_compatibility(
            Some("transformers"),
            BackendCompatibilityRequest::new(&task, &package),
        );

        let summary = report.to_inference_compatibility_report_summary();
        let issues = report.to_inference_compatibility_issue_summaries(1);

        assert!(!summary.compatible);
        assert_eq!(summary.status, "rejected");
        assert_eq!(summary.task, "supported");
        assert_eq!(summary.preprocessing, "unsupported");
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].kind, "missing_preprocessing_component");
        assert_eq!(issues[0].phase, InferenceLifecyclePhase::Preprocessing);
        assert_eq!(
            issues[0].model_id.as_deref(),
            Some("llm/example/missing-tokenizer")
        );
    }

    #[test]
    fn compatibility_issue_summary_drops_local_path_when_model_id_is_stable() {
        let report = BackendCompatibilityReport {
            compatible: false,
            task: BackendCompatibilityStatus::Supported,
            model_source: BackendCompatibilityStatus::Unsupported,
            preprocessing: BackendCompatibilityStatus::Supported,
            postprocessing: BackendCompatibilityStatus::Supported,
            option_diagnostics: Vec::new(),
            issues: vec![BackendCompatibilityIssue {
                kind: BackendCompatibilityIssueKind::InvalidModelArtifact,
                phase: InferenceLifecyclePhase::ModelPackageResolution,
                message: "model artifact is not valid".to_string(),
                model_id: Some("pumas://models/private".to_string()),
                path: Some("/media/models/private/model.gguf".to_string()),
            }],
        };

        let issues = report.to_inference_compatibility_issue_summaries(1);

        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].model_id.as_deref(),
            Some("pumas://models/private")
        );
        assert!(issues[0].path.is_none());
    }

    #[test]
    fn compatibility_issue_summary_keeps_relative_component_path() {
        let report = BackendCompatibilityReport {
            compatible: false,
            task: BackendCompatibilityStatus::Supported,
            model_source: BackendCompatibilityStatus::Supported,
            preprocessing: BackendCompatibilityStatus::Unsupported,
            postprocessing: BackendCompatibilityStatus::Supported,
            option_diagnostics: Vec::new(),
            issues: vec![BackendCompatibilityIssue {
                kind: BackendCompatibilityIssueKind::MissingPreprocessingComponent,
                phase: InferenceLifecyclePhase::Preprocessing,
                message: "required tokenizer component is missing".to_string(),
                model_id: Some("pumas://models/private".to_string()),
                path: Some("tokenizer.json".to_string()),
            }],
        };

        let issues = report.to_inference_compatibility_issue_summaries(1);

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].path.as_deref(), Some("tokenizer.json"));
    }
}
