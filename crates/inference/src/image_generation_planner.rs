use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::device_contracts::{
    BackendExecutionDecision, BackendId, DeviceResolutionDecision, InferenceDeviceClass,
    InferenceDeviceId, RuntimeVariantId,
};
use crate::image_generation_family_rules::{
    image_generation_family_rules, ImageGenerationFamilyRules,
};
use crate::model_contracts::{
    DiffusersComponentRole, ImageGenerationFamilyLabel, InferenceTaskId, PackageFactStatus,
    PumasModelRef, ResolvedModelPackageFacts,
};
use crate::types::ImageGenerationRequest;

const PYTORCH_BACKEND_ID: &str = "pytorch";
const IMAGE_PLANNER_MIN_DIMENSION: u32 = 1;
const IMAGE_PLANNER_BYTES_PER_RGBA_PIXEL: u64 = 4;
const DENOISING_SCHEDULER_OPTION_ID_MAX_LEN: usize = 96;

/// Side-effect-free inputs for canonical image-generation planning.
#[derive(Debug, Clone, Copy)]
pub struct ImageGenerationPlanningInput<'a> {
    /// User or workflow image-generation request.
    pub request: &'a ImageGenerationRequest,
    /// Current Pumas package facts for the selected model artifact.
    pub package_facts: &'a ResolvedModelPackageFacts,
    /// Scheduler-owned backend/runtime/device decision.
    pub backend_decision: &'a BackendExecutionDecision,
}

/// Planner result: exactly one execution plan or bounded diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ImageGenerationPlanningOutcome {
    /// The request has one canonical PyTorch/Diffusers execution plan.
    Planned {
        /// Validated execution plan.
        plan: ImageGenerationExecutionPlan,
    },
    /// Planning failed closed with typed diagnostics.
    Rejected {
        /// Blocking planner diagnostics.
        diagnostics: Vec<ImageGenerationPlannerDiagnostic>,
    },
}

impl ImageGenerationPlanningOutcome {
    /// Return true when planning produced an executable plan.
    #[must_use]
    pub fn is_planned(&self) -> bool {
        matches!(self, Self::Planned { .. })
    }
}

/// Error returned when an image denoising scheduler option id is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DenoisingSchedulerOptionIdError {
    /// Option ids must not be blank.
    Blank,
    /// Option ids are bounded to keep graph and worker contracts small.
    TooLong {
        /// Maximum accepted byte length.
        max_len: usize,
        /// Actual byte length.
        actual_len: usize,
    },
    /// Option ids use stable lowercase primitive ids, not display labels.
    InvalidShape { value: String },
}

impl fmt::Display for DenoisingSchedulerOptionIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Blank => f.write_str("denoising_scheduler option id must not be blank"),
            Self::TooLong {
                max_len,
                actual_len,
            } => write!(
                f,
                "denoising_scheduler option id must be at most {max_len} bytes, got {actual_len}"
            ),
            Self::InvalidShape { value } => write!(
                f,
                "denoising_scheduler option id must be a lowercase primitive id, got {value}"
            ),
        }
    }
}

impl std::error::Error for DenoisingSchedulerOptionIdError {}

/// Stable primitive id for a denoising scheduler option.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub struct DenoisingSchedulerOptionId(String);

impl DenoisingSchedulerOptionId {
    /// Parse and validate a primitive denoising scheduler option id.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DenoisingSchedulerOptionIdError> {
        let trimmed = value.as_ref().trim();
        if trimmed.is_empty() {
            return Err(DenoisingSchedulerOptionIdError::Blank);
        }
        if trimmed.len() > DENOISING_SCHEDULER_OPTION_ID_MAX_LEN {
            return Err(DenoisingSchedulerOptionIdError::TooLong {
                max_len: DENOISING_SCHEDULER_OPTION_ID_MAX_LEN,
                actual_len: trimmed.len(),
            });
        }

        let mut chars = trimmed.chars();
        let Some(first) = chars.next() else {
            return Err(DenoisingSchedulerOptionIdError::Blank);
        };
        if !first.is_ascii_lowercase() {
            return Err(DenoisingSchedulerOptionIdError::InvalidShape {
                value: trimmed.to_string(),
            });
        }

        let mut previous_was_separator = false;
        for ch in chars {
            if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
                previous_was_separator = false;
                continue;
            }
            if matches!(ch, '_' | '-' | '.') && !previous_was_separator {
                previous_was_separator = true;
                continue;
            }
            return Err(DenoisingSchedulerOptionIdError::InvalidShape {
                value: trimmed.to_string(),
            });
        }

        if previous_was_separator {
            return Err(DenoisingSchedulerOptionIdError::InvalidShape {
                value: trimmed.to_string(),
            });
        }

        Ok(Self(trimmed.to_string()))
    }

    /// Borrow the validated id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for DenoisingSchedulerOptionId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for DenoisingSchedulerOptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DenoisingSchedulerOptionId {
    type Err = DenoisingSchedulerOptionIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for DenoisingSchedulerOptionId {
    type Error = DenoisingSchedulerOptionIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for DenoisingSchedulerOptionId {
    type Error = DenoisingSchedulerOptionIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for DenoisingSchedulerOptionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DenoisingSchedulerOptionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Canonical Rust-owned image-generation plan consumed before worker execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ImageGenerationExecutionPlan {
    pub model_ref: PumasModelRef,
    pub artifact_entry_path: String,
    pub backend_id: BackendId,
    pub runtime_variant_id: RuntimeVariantId,
    pub selected_device_class: InferenceDeviceClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_device_id: Option<InferenceDeviceId>,
    pub device_decision: DeviceResolutionDecision,
    pub family: ImageGenerationFamilyLabel,
    pub pipeline_class: String,
    pub required_components: Vec<DiffusersComponentRole>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_inference_steps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guidance_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denoising_scheduler: Option<DenoisingSchedulerOptionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_images_per_prompt: Option<u32>,
    /// Conservative RGBA output byte estimate when width, height, and count are known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_output_rgba_bytes: Option<u64>,
}

/// Stable planner diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ImageGenerationPlannerDiagnostic {
    pub code: ImageGenerationPlannerDiagnosticCode,
    pub severity: ImageGenerationPlannerDiagnosticSeverity,
    pub field_path: String,
    pub message: String,
}

/// Stable image-generation planner diagnostic codes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ImageGenerationPlannerDiagnosticCode {
    StalePackageFactsContract,
    UnsupportedBackend,
    MissingDiffusersEvidence,
    DiffusersEvidenceUnavailable,
    UnsupportedTaskEvidence,
    MissingFamilyEvidence,
    AmbiguousFamilyEvidence,
    UnsupportedFamily,
    MissingPipelineClass,
    MissingComponentRole,
    MissingPrompt,
    InvalidNumericOption,
    InvalidDenoisingSchedulerOptionId,
    UnsupportedOption,
    ResourceEstimateOverflow,
    MissingSelectedModelRef,
    SelectedModelRefMismatch,
}

/// Planner diagnostic severity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageGenerationPlannerDiagnosticSeverity {
    /// Blocking planner failure.
    Error,
}

/// Build a canonical image-generation execution plan or reject with diagnostics.
#[must_use]
pub fn plan_image_generation_execution(
    input: ImageGenerationPlanningInput<'_>,
) -> ImageGenerationPlanningOutcome {
    let mut diagnostics = Vec::new();

    validate_backend_decision(input.backend_decision, &mut diagnostics);
    validate_selected_model_ref(
        input.backend_decision,
        input.package_facts,
        &mut diagnostics,
    );
    validate_package_contract(input.package_facts, &mut diagnostics);
    validate_task_evidence(input.package_facts, &mut diagnostics);
    validate_image_request_shape(input.request, &mut diagnostics);

    let Some(diffusers) = input.package_facts.diffusers.as_ref() else {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::MissingDiffusersEvidence,
            "package_facts.diffusers",
            "image-generation planning requires structured Diffusers package facts",
        ));
        return rejected(diagnostics);
    };

    if diffusers.status != PackageFactStatus::Present {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::DiffusersEvidenceUnavailable,
            "package_facts.diffusers.status",
            "Diffusers package facts must be present before planning execution",
        ));
    }

    let family = resolve_family(input.package_facts, &mut diagnostics);
    let family_rules =
        family.and_then(|family| family_rules_for_planning(family, &mut diagnostics));
    validate_required_components(input.package_facts, family_rules, &mut diagnostics);

    let pipeline_class = diffusers.pipeline_class.as_deref().map(str::trim);
    if pipeline_class.is_none_or(str::is_empty) {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::MissingPipelineClass,
            "package_facts.diffusers.pipeline_class",
            "Diffusers pipeline class is required for image-generation planning",
        ));
    }

    let estimated_output_rgba_bytes = estimate_output_rgba_bytes(input.request, &mut diagnostics);
    let denoising_scheduler = validate_denoising_scheduler_id(input.request, &mut diagnostics);
    validate_family_option_support(
        family_rules,
        input.request,
        denoising_scheduler.as_ref(),
        &mut diagnostics,
    );

    if !diagnostics.is_empty() {
        return rejected(diagnostics);
    }

    let Some(family) = family else {
        return rejected(vec![diagnostic(
            ImageGenerationPlannerDiagnosticCode::MissingFamilyEvidence,
            "package_facts.diffusers.family_evidence",
            "Diffusers image-generation planning requires concrete family evidence",
        )]);
    };
    let Some(pipeline_class) = pipeline_class else {
        return rejected(vec![diagnostic(
            ImageGenerationPlannerDiagnosticCode::MissingPipelineClass,
            "package_facts.diffusers.pipeline_class",
            "Diffusers pipeline class is required for image-generation planning",
        )]);
    };
    let Some(family_rules) = family_rules else {
        return rejected(vec![diagnostic(
            ImageGenerationPlannerDiagnosticCode::UnsupportedFamily,
            "package_facts.diffusers.family_evidence",
            "Diffusers image-generation planning requires supported family rules",
        )]);
    };

    ImageGenerationPlanningOutcome::Planned {
        plan: ImageGenerationExecutionPlan {
            model_ref: input.package_facts.model_ref.clone(),
            artifact_entry_path: input.package_facts.artifact.entry_path.clone(),
            backend_id: input.backend_decision.selected_backend_id.clone(),
            runtime_variant_id: input.backend_decision.selected_runtime_variant_id.clone(),
            selected_device_class: input.backend_decision.selected_device_class,
            selected_device_id: input.backend_decision.selected_device_id.clone(),
            device_decision: input.backend_decision.device_decision.clone(),
            family,
            pipeline_class: pipeline_class.to_string(),
            required_components: family_rules.required_components.to_vec(),
            prompt: input.request.prompt.clone(),
            negative_prompt: input.request.negative_prompt.clone(),
            width: input.request.width,
            height: input.request.height,
            num_inference_steps: input.request.num_inference_steps,
            guidance_scale: input.request.guidance_scale,
            seed: input.request.seed,
            denoising_scheduler,
            num_images_per_prompt: input.request.num_images_per_prompt,
            estimated_output_rgba_bytes,
        },
    }
}

fn validate_backend_decision(
    backend_decision: &BackendExecutionDecision,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) {
    if backend_decision.selected_backend_id.as_str() != PYTORCH_BACKEND_ID {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::UnsupportedBackend,
            "backend_decision.selected_backend_id",
            "image-generation planning requires an explicit PyTorch backend decision",
        ));
    }
}

fn validate_selected_model_ref(
    backend_decision: &BackendExecutionDecision,
    package_facts: &ResolvedModelPackageFacts,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) {
    let Some(selected_model_ref) = backend_decision.selected_model_ref.as_ref() else {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::MissingSelectedModelRef,
            "backend_decision.selected_model_ref",
            "image-generation planning requires a scheduler-selected model ref",
        ));
        return;
    };

    let selected_model_id = canonical_pumas_model_id(selected_model_ref);
    let package_model_id = canonical_pumas_model_id(&package_facts.model_ref);
    if selected_model_id != package_model_id {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::SelectedModelRefMismatch,
            "backend_decision.selected_model_ref",
            format!(
                "scheduler-selected model ref '{selected_model_id}' does not match package facts model ref '{package_model_id}'"
            ),
        ));
    }
}

fn canonical_pumas_model_id(model_ref: &PumasModelRef) -> String {
    if model_ref.model_id.starts_with("pumas://models/") {
        model_ref.model_id.clone()
    } else {
        format!("pumas://models/{}", model_ref.model_id)
    }
}

fn validate_package_contract(
    package_facts: &ResolvedModelPackageFacts,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) {
    if !package_facts.uses_current_contract() {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::StalePackageFactsContract,
            "package_facts.package_facts_contract_version",
            "package facts must use the current inference contract version",
        ));
    }
}

fn validate_task_evidence(
    package_facts: &ResolvedModelPackageFacts,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) {
    let task_entry = crate::model_contracts::resolve_task_registry_entry(
        InferenceTaskId::ImageGeneration.canonical_label(),
    );
    if !task_entry.is_some_and(|task_entry| task_entry.matches_task_evidence(&package_facts.task)) {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::UnsupportedTaskEvidence,
            "package_facts.task",
            "package task evidence must resolve to image_generation",
        ));
    }
}

fn validate_image_request_shape(
    request: &ImageGenerationRequest,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) {
    if request.prompt.trim().is_empty() {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::MissingPrompt,
            "request.prompt",
            "image-generation prompt must not be empty",
        ));
    }

    validate_non_zero(request.width, "request.width", diagnostics);
    validate_non_zero(request.height, "request.height", diagnostics);
    validate_non_zero(
        request.num_inference_steps,
        "request.num_inference_steps",
        diagnostics,
    );
    validate_non_zero(
        request.num_images_per_prompt,
        "request.num_images_per_prompt",
        diagnostics,
    );
    if request
        .guidance_scale
        .is_some_and(|guidance_scale| !guidance_scale.is_finite())
    {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::InvalidNumericOption,
            "request.guidance_scale",
            "image-generation guidance scale must be finite when provided",
        ));
    }
}

fn validate_denoising_scheduler_id(
    request: &ImageGenerationRequest,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) -> Option<DenoisingSchedulerOptionId> {
    request
        .denoising_scheduler
        .as_deref()
        .and_then(
            |scheduler| match DenoisingSchedulerOptionId::parse(scheduler) {
                Ok(option_id) => Some(option_id),
                Err(error) => {
                    diagnostics.push(diagnostic(
                        ImageGenerationPlannerDiagnosticCode::InvalidDenoisingSchedulerOptionId,
                        "request.denoising_scheduler",
                        error.to_string(),
                    ));
                    None
                }
            },
        )
}

fn validate_family_option_support(
    family_rules: Option<&ImageGenerationFamilyRules>,
    request: &ImageGenerationRequest,
    denoising_scheduler: Option<&DenoisingSchedulerOptionId>,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) {
    let Some(family_rules) = family_rules else {
        return;
    };
    for unsupported in
        family_rules.unsupported_request_options(request, denoising_scheduler.is_some())
    {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::UnsupportedOption,
            unsupported.field_path,
            unsupported.message,
        ));
    }
}

fn validate_non_zero(
    value: Option<u32>,
    field_path: &'static str,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) {
    if matches!(value, Some(0)) {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::InvalidNumericOption,
            field_path,
            "numeric image-generation options must be greater than zero when provided",
        ));
    }
}

fn resolve_family(
    package_facts: &ResolvedModelPackageFacts,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) -> Option<ImageGenerationFamilyLabel> {
    let families = package_facts
        .diffusers
        .as_ref()
        .map(|diffusers| {
            diffusers
                .family_evidence
                .iter()
                .filter_map(|evidence| match evidence.family {
                    ImageGenerationFamilyLabel::Unknown | ImageGenerationFamilyLabel::Ambiguous => {
                        None
                    }
                    family => Some(family),
                })
                .fold(
                    Vec::<ImageGenerationFamilyLabel>::new(),
                    |mut families, family| {
                        if !families.contains(&family) {
                            families.push(family);
                        }
                        families
                    },
                )
        })
        .unwrap_or_default();

    match families.as_slice() {
        [] => {
            diagnostics.push(diagnostic(
                ImageGenerationPlannerDiagnosticCode::MissingFamilyEvidence,
                "package_facts.diffusers.family_evidence",
                "Diffusers image-generation planning requires concrete family evidence",
            ));
            None
        }
        [family] => Some(*family),
        _ => {
            diagnostics.push(diagnostic(
                ImageGenerationPlannerDiagnosticCode::AmbiguousFamilyEvidence,
                "package_facts.diffusers.family_evidence",
                "Diffusers image-generation family evidence must resolve to one family",
            ));
            None
        }
    }
}

fn family_rules_for_planning(
    family: ImageGenerationFamilyLabel,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) -> Option<&'static ImageGenerationFamilyRules> {
    let rules = image_generation_family_rules(family);
    if rules.is_none() {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::UnsupportedFamily,
            "package_facts.diffusers.family_evidence",
            "this planner slice only supports Stable Diffusion family facts",
        ));
    }
    rules
}

fn validate_required_components(
    package_facts: &ResolvedModelPackageFacts,
    family_rules: Option<&ImageGenerationFamilyRules>,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) {
    let Some(family_rules) = family_rules else {
        return;
    };
    let Some(diffusers) = package_facts.diffusers.as_ref() else {
        return;
    };

    for required in family_rules.required_components {
        let present = diffusers.components.iter().any(|component| {
            component.role == *required && component.status == PackageFactStatus::Present
        });
        if !present {
            diagnostics.push(diagnostic(
                ImageGenerationPlannerDiagnosticCode::MissingComponentRole,
                format!(
                    "package_facts.diffusers.components.{}",
                    role_label(*required)
                ),
                format!(
                    "Diffusers component role '{}' is required for the selected image family",
                    role_label(*required)
                ),
            ));
        }
    }
}

fn estimate_output_rgba_bytes(
    request: &ImageGenerationRequest,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) -> Option<u64> {
    let (Some(width), Some(height)) = (request.width, request.height) else {
        return None;
    };
    let count = request.num_images_per_prompt.unwrap_or(1);
    if width < IMAGE_PLANNER_MIN_DIMENSION
        || height < IMAGE_PLANNER_MIN_DIMENSION
        || count < IMAGE_PLANNER_MIN_DIMENSION
    {
        return None;
    }

    let estimate = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(u64::from(count)))
        .and_then(|pixels| pixels.checked_mul(IMAGE_PLANNER_BYTES_PER_RGBA_PIXEL));
    if estimate.is_none() {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::ResourceEstimateOverflow,
            "request.width/request.height/request.num_images_per_prompt",
            "image-generation output byte estimate overflowed",
        ));
    }
    estimate
}

fn rejected(diagnostics: Vec<ImageGenerationPlannerDiagnostic>) -> ImageGenerationPlanningOutcome {
    ImageGenerationPlanningOutcome::Rejected { diagnostics }
}

fn diagnostic(
    code: ImageGenerationPlannerDiagnosticCode,
    field_path: impl Into<String>,
    message: impl Into<String>,
) -> ImageGenerationPlannerDiagnostic {
    ImageGenerationPlannerDiagnostic {
        code,
        severity: ImageGenerationPlannerDiagnosticSeverity::Error,
        field_path: field_path.into(),
        message: message.into(),
    }
}

fn role_label(role: DiffusersComponentRole) -> &'static str {
    match role {
        DiffusersComponentRole::PipelineIndex => "pipeline_index",
        DiffusersComponentRole::Scheduler => "scheduler",
        DiffusersComponentRole::Tokenizer => "tokenizer",
        DiffusersComponentRole::Tokenizer2 => "tokenizer2",
        DiffusersComponentRole::TextEncoder => "text_encoder",
        DiffusersComponentRole::TextEncoder2 => "text_encoder2",
        DiffusersComponentRole::TextEncoder3 => "text_encoder3",
        DiffusersComponentRole::ImageProcessor => "image_processor",
        DiffusersComponentRole::Processor => "processor",
        DiffusersComponentRole::Unet => "unet",
        DiffusersComponentRole::Transformer => "transformer",
        DiffusersComponentRole::Vae => "vae",
        DiffusersComponentRole::Controlnet => "controlnet",
        DiffusersComponentRole::Adapter => "adapter",
        DiffusersComponentRole::Weights => "weights",
        DiffusersComponentRole::GenerationConfig => "generation_config",
    }
}

#[cfg(test)]
#[path = "image_generation_planner_tests.rs"]
mod image_generation_planner_tests;
