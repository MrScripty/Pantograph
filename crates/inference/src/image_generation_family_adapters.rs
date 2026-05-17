use crate::image_generation_family_rules::{
    image_generation_family_rules, ImageGenerationFamilyRules, UnsupportedImageGenerationOption,
};
use crate::model_contracts::{
    DiffusersComponentRole, ImageGenerationFamilyLabel, PackageFactStatus,
    ResolvedModelPackageFacts,
};
use crate::types::ImageGenerationRequest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImageGenerationFamilyAdapter {
    family: ImageGenerationFamilyLabel,
    rules: &'static ImageGenerationFamilyRules,
}

impl ImageGenerationFamilyAdapter {
    pub(crate) fn family(&self) -> ImageGenerationFamilyLabel {
        self.family
    }

    pub(crate) fn required_components(&self) -> &'static [DiffusersComponentRole] {
        self.rules.required_components
    }

    pub(crate) fn unsupported_request_options(
        &self,
        request: &ImageGenerationRequest,
        has_valid_denoising_scheduler: bool,
    ) -> Vec<UnsupportedImageGenerationOption> {
        self.rules
            .unsupported_request_options(request, has_valid_denoising_scheduler)
    }

    pub(crate) fn validate_required_components(
        &self,
        package_facts: &ResolvedModelPackageFacts,
    ) -> Vec<ImageGenerationFamilyAdapterDiagnostic> {
        let Some(diffusers) = package_facts.diffusers.as_ref() else {
            return Vec::new();
        };

        let mut diagnostics = Vec::new();
        for required in self.required_components() {
            let present = diffusers
                .components
                .iter()
                .filter(|component| {
                    component.role == *required && component.status == PackageFactStatus::Present
                })
                .collect::<Vec<_>>();
            let role = role_label(*required);
            if present.is_empty() {
                diagnostics.push(ImageGenerationFamilyAdapterDiagnostic {
                    code: ImageGenerationFamilyAdapterDiagnosticCode::MissingComponentRole,
                    field_path: format!("package_facts.diffusers.components.{role}"),
                    message: format!(
                        "Diffusers component role '{role}' is required for the selected image family"
                    ),
                });
            } else if present.len() > 1 {
                diagnostics.push(ImageGenerationFamilyAdapterDiagnostic {
                    code: ImageGenerationFamilyAdapterDiagnosticCode::AmbiguousComponentRole,
                    field_path: format!("package_facts.diffusers.components.{role}"),
                    message: format!(
                        "Diffusers component role '{role}' resolved to multiple present sources for the selected image family"
                    ),
                });
            }
        }
        diagnostics
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ImageGenerationFamilyAdapterResolution {
    Resolved(ImageGenerationFamilyAdapter),
    Rejected(Vec<ImageGenerationFamilyAdapterDiagnostic>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ImageGenerationFamilyAdapterDiagnostic {
    pub(crate) code: ImageGenerationFamilyAdapterDiagnosticCode,
    pub(crate) field_path: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageGenerationFamilyAdapterDiagnosticCode {
    MissingFamilyEvidence,
    AmbiguousFamilyEvidence,
    UnsupportedFamily,
    MissingComponentRole,
    AmbiguousComponentRole,
}

pub(crate) fn resolve_image_generation_family_adapter(
    package_facts: &ResolvedModelPackageFacts,
) -> ImageGenerationFamilyAdapterResolution {
    let mut diagnostics = Vec::new();
    let Some(family) = resolve_family(package_facts, &mut diagnostics) else {
        return ImageGenerationFamilyAdapterResolution::Rejected(diagnostics);
    };
    let Some(rules) = image_generation_family_rules(family) else {
        diagnostics.push(ImageGenerationFamilyAdapterDiagnostic {
            code: ImageGenerationFamilyAdapterDiagnosticCode::UnsupportedFamily,
            field_path: "package_facts.diffusers.family_evidence".to_string(),
            message: "this planner slice only supports Stable Diffusion family facts".to_string(),
        });
        return ImageGenerationFamilyAdapterResolution::Rejected(diagnostics);
    };

    ImageGenerationFamilyAdapterResolution::Resolved(ImageGenerationFamilyAdapter { family, rules })
}

fn resolve_family(
    package_facts: &ResolvedModelPackageFacts,
    diagnostics: &mut Vec<ImageGenerationFamilyAdapterDiagnostic>,
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
            diagnostics.push(ImageGenerationFamilyAdapterDiagnostic {
                code: ImageGenerationFamilyAdapterDiagnosticCode::MissingFamilyEvidence,
                field_path: "package_facts.diffusers.family_evidence".to_string(),
                message: "Diffusers image-generation planning requires concrete family evidence"
                    .to_string(),
            });
            None
        }
        [family] => Some(*family),
        _ => {
            diagnostics.push(ImageGenerationFamilyAdapterDiagnostic {
                code: ImageGenerationFamilyAdapterDiagnosticCode::AmbiguousFamilyEvidence,
                field_path: "package_facts.diffusers.family_evidence".to_string(),
                message: "Diffusers image-generation family evidence must resolve to one family"
                    .to_string(),
            });
            None
        }
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
mod tests {
    use super::*;

    fn package_fixture() -> ResolvedModelPackageFacts {
        serde_json::from_str(include_str!(
            "../tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
        ))
        .expect("fixture should decode")
    }

    #[test]
    fn resolves_stable_diffusion_adapter_from_package_facts() {
        let facts = package_fixture();

        let ImageGenerationFamilyAdapterResolution::Resolved(adapter) =
            resolve_image_generation_family_adapter(&facts)
        else {
            panic!("expected stable diffusion adapter");
        };

        assert_eq!(
            adapter.family(),
            ImageGenerationFamilyLabel::StableDiffusion
        );
        assert!(adapter
            .required_components()
            .contains(&DiffusersComponentRole::Unet));
        assert!(adapter.validate_required_components(&facts).is_empty());
    }

    #[test]
    fn reports_exact_missing_required_component_role() {
        let mut facts = package_fixture();
        facts
            .diffusers
            .as_mut()
            .expect("diffusers facts")
            .components
            .retain(|component| component.role != DiffusersComponentRole::Vae);

        let ImageGenerationFamilyAdapterResolution::Resolved(adapter) =
            resolve_image_generation_family_adapter(&facts)
        else {
            panic!("expected stable diffusion adapter");
        };

        let diagnostics = adapter.validate_required_components(&facts);
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == ImageGenerationFamilyAdapterDiagnosticCode::MissingComponentRole
                && diagnostic.field_path == "package_facts.diffusers.components.vae"
        }));
    }
}
