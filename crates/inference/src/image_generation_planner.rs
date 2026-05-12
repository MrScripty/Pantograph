use serde::{Deserialize, Serialize};

use crate::device_contracts::{
    BackendExecutionDecision, BackendId, DeviceResolutionDecision, InferenceDeviceClass,
    InferenceDeviceId, RuntimeVariantId,
};
use crate::model_contracts::{
    DiffusersComponentRole, ImageGenerationFamilyLabel, InferenceTaskId, PackageFactStatus,
    PumasModelRef, ResolvedModelPackageFacts,
};
use crate::types::ImageGenerationRequest;

const PYTORCH_BACKEND_ID: &str = "pytorch";
const IMAGE_PLANNER_MIN_DIMENSION: u32 = 1;
const IMAGE_PLANNER_BYTES_PER_RGBA_PIXEL: u64 = 4;

const STABLE_DIFFUSION_REQUIRED_COMPONENTS: &[DiffusersComponentRole] = &[
    DiffusersComponentRole::PipelineIndex,
    DiffusersComponentRole::Scheduler,
    DiffusersComponentRole::Tokenizer,
    DiffusersComponentRole::TextEncoder,
    DiffusersComponentRole::Unet,
    DiffusersComponentRole::Vae,
];

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
    pub scheduler: Option<String>,
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
    ResourceEstimateOverflow,
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
    validate_package_contract(input.package_facts, &mut diagnostics);
    validate_task_evidence(input.package_facts, &mut diagnostics);
    validate_image_request(input.request, &mut diagnostics);

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
    let required_components = family
        .and_then(|family| required_components_for_family(family, &mut diagnostics).map(Vec::from));
    validate_required_components(
        input.package_facts,
        required_components.as_deref(),
        &mut diagnostics,
    );

    let pipeline_class = diffusers.pipeline_class.as_deref().map(str::trim);
    if pipeline_class.is_none_or(str::is_empty) {
        diagnostics.push(diagnostic(
            ImageGenerationPlannerDiagnosticCode::MissingPipelineClass,
            "package_facts.diffusers.pipeline_class",
            "Diffusers pipeline class is required for image-generation planning",
        ));
    }

    let estimated_output_rgba_bytes = estimate_output_rgba_bytes(input.request, &mut diagnostics);

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
    let Some(required_components) = required_components else {
        return rejected(vec![diagnostic(
            ImageGenerationPlannerDiagnosticCode::MissingComponentRole,
            "package_facts.diffusers.components",
            "Diffusers component roles are required for image-generation planning",
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
            required_components,
            prompt: input.request.prompt.clone(),
            negative_prompt: input.request.negative_prompt.clone(),
            width: input.request.width,
            height: input.request.height,
            num_inference_steps: input.request.num_inference_steps,
            guidance_scale: input.request.guidance_scale,
            seed: input.request.seed,
            scheduler: input.request.scheduler.clone(),
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

fn validate_image_request(
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

fn required_components_for_family(
    family: ImageGenerationFamilyLabel,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) -> Option<&'static [DiffusersComponentRole]> {
    match family {
        ImageGenerationFamilyLabel::StableDiffusion => Some(STABLE_DIFFUSION_REQUIRED_COMPONENTS),
        ImageGenerationFamilyLabel::StableDiffusionXl
        | ImageGenerationFamilyLabel::Flux
        | ImageGenerationFamilyLabel::Flux2
        | ImageGenerationFamilyLabel::QwenImage
        | ImageGenerationFamilyLabel::LuminaImage
        | ImageGenerationFamilyLabel::GlmImage
        | ImageGenerationFamilyLabel::ZImage
        | ImageGenerationFamilyLabel::Unknown
        | ImageGenerationFamilyLabel::Ambiguous => {
            diagnostics.push(diagnostic(
                ImageGenerationPlannerDiagnosticCode::UnsupportedFamily,
                "package_facts.diffusers.family_evidence",
                "this planner slice only supports Stable Diffusion family facts",
            ));
            None
        }
    }
}

fn validate_required_components(
    package_facts: &ResolvedModelPackageFacts,
    required_components: Option<&[DiffusersComponentRole]>,
    diagnostics: &mut Vec<ImageGenerationPlannerDiagnostic>,
) {
    let Some(required_components) = required_components else {
        return;
    };
    let Some(diffusers) = package_facts.diffusers.as_ref() else {
        return;
    };

    for required in required_components {
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
