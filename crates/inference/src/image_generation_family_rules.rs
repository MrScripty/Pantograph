use crate::model_contracts::{DiffusersComponentRole, ImageGenerationFamilyLabel};
use crate::types::ImageGenerationRequest;

const STABLE_DIFFUSION_REQUIRED_COMPONENTS: &[DiffusersComponentRole] = &[
    DiffusersComponentRole::PipelineIndex,
    DiffusersComponentRole::Scheduler,
    DiffusersComponentRole::Tokenizer,
    DiffusersComponentRole::TextEncoder,
    DiffusersComponentRole::Unet,
    DiffusersComponentRole::Vae,
];

const STABLE_DIFFUSION_SUPPORTED_TRANSFORMERS_DTYPES: &[&str] = &["float32", "float16", "bfloat16"];

const STABLE_DIFFUSION_OPTION_RULES: ImageGenerationFamilyOptionRules =
    ImageGenerationFamilyOptionRules {
        supports_negative_prompt: true,
        supports_dimensions: true,
        supports_num_inference_steps: true,
        supports_guidance_scale: true,
        supports_seed: true,
        supports_num_images_per_prompt: true,
        supports_denoising_scheduler_override: false,
        supports_init_image: false,
        supports_mask_image: false,
        supports_strength: false,
        supports_extra_options: false,
    };

static STABLE_DIFFUSION_RULES: ImageGenerationFamilyRules = ImageGenerationFamilyRules {
    family: ImageGenerationFamilyLabel::StableDiffusion,
    required_components: STABLE_DIFFUSION_REQUIRED_COMPONENTS,
    supported_transformers_dtypes: STABLE_DIFFUSION_SUPPORTED_TRANSFORMERS_DTYPES,
    options: STABLE_DIFFUSION_OPTION_RULES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImageGenerationFamilyRules {
    pub(crate) family: ImageGenerationFamilyLabel,
    pub(crate) required_components: &'static [DiffusersComponentRole],
    supported_transformers_dtypes: &'static [&'static str],
    options: ImageGenerationFamilyOptionRules,
}

impl ImageGenerationFamilyRules {
    pub(crate) fn unsupported_request_options(
        &self,
        request: &ImageGenerationRequest,
        has_valid_denoising_scheduler: bool,
    ) -> Vec<UnsupportedImageGenerationOption> {
        let mut unsupported = Vec::new();

        push_unsupported(
            request.negative_prompt.is_some() && !self.options.supports_negative_prompt,
            "request.negative_prompt",
            "negative_prompt is not supported by this image family",
            &mut unsupported,
        );
        push_unsupported(
            (request.width.is_some() || request.height.is_some())
                && !self.options.supports_dimensions,
            "request.width/request.height",
            "explicit image dimensions are not supported by this image family",
            &mut unsupported,
        );
        push_unsupported(
            request.num_inference_steps.is_some() && !self.options.supports_num_inference_steps,
            "request.num_inference_steps",
            "num_inference_steps is not supported by this image family",
            &mut unsupported,
        );
        push_unsupported(
            request.guidance_scale.is_some() && !self.options.supports_guidance_scale,
            "request.guidance_scale",
            "guidance_scale is not supported by this image family",
            &mut unsupported,
        );
        push_unsupported(
            request.seed.is_some() && !self.options.supports_seed,
            "request.seed",
            "seed is not supported by this image family",
            &mut unsupported,
        );
        push_unsupported(
            request.num_images_per_prompt.is_some() && !self.options.supports_num_images_per_prompt,
            "request.num_images_per_prompt",
            "num_images_per_prompt is not supported by this image family",
            &mut unsupported,
        );
        push_unsupported(
            has_valid_denoising_scheduler && !self.options.supports_denoising_scheduler_override,
            "request.denoising_scheduler",
            "explicit denoising_scheduler changes require family/runtime support and are not supported by this planner slice",
            &mut unsupported,
        );
        push_unsupported(
            request.init_image.is_some() && !self.options.supports_init_image,
            "request.init_image",
            "init_image is reserved for later img2img support and is not supported by this image family",
            &mut unsupported,
        );
        push_unsupported(
            request.mask_image.is_some() && !self.options.supports_mask_image,
            "request.mask_image",
            "mask_image is reserved for later inpaint support and is not supported by this image family",
            &mut unsupported,
        );
        push_unsupported(
            request.strength.is_some() && !self.options.supports_strength,
            "request.strength",
            "strength is reserved for later img2img/inpaint support and is not supported by this image family",
            &mut unsupported,
        );
        push_unsupported(
            !request.extra_options.is_null() && !self.options.supports_extra_options,
            "request.extra_options",
            "image-generation extra_options require explicit family support and are not supported by this image family",
            &mut unsupported,
        );

        unsupported
    }

    pub(crate) fn supports_transformers_dtype(&self, dtype: &str) -> bool {
        normalize_transformers_dtype(dtype).is_some_and(|normalized| {
            self.supported_transformers_dtypes
                .iter()
                .any(|supported| *supported == normalized)
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImageGenerationFamilyOptionRules {
    supports_negative_prompt: bool,
    supports_dimensions: bool,
    supports_num_inference_steps: bool,
    supports_guidance_scale: bool,
    supports_seed: bool,
    supports_num_images_per_prompt: bool,
    supports_denoising_scheduler_override: bool,
    supports_init_image: bool,
    supports_mask_image: bool,
    supports_strength: bool,
    supports_extra_options: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnsupportedImageGenerationOption {
    pub(crate) field_path: &'static str,
    pub(crate) message: &'static str,
}

pub(crate) fn image_generation_family_rules(
    family: ImageGenerationFamilyLabel,
) -> Option<&'static ImageGenerationFamilyRules> {
    match family {
        ImageGenerationFamilyLabel::StableDiffusion => Some(&STABLE_DIFFUSION_RULES),
        ImageGenerationFamilyLabel::StableDiffusionXl
        | ImageGenerationFamilyLabel::Flux
        | ImageGenerationFamilyLabel::Flux2
        | ImageGenerationFamilyLabel::QwenImage
        | ImageGenerationFamilyLabel::LuminaImage
        | ImageGenerationFamilyLabel::GlmImage
        | ImageGenerationFamilyLabel::ZImage
        | ImageGenerationFamilyLabel::Unknown
        | ImageGenerationFamilyLabel::Ambiguous => None,
    }
}

fn push_unsupported(
    requested: bool,
    field_path: &'static str,
    message: &'static str,
    unsupported: &mut Vec<UnsupportedImageGenerationOption>,
) {
    if requested {
        unsupported.push(UnsupportedImageGenerationOption {
            field_path,
            message,
        });
    }
}

fn normalize_transformers_dtype(dtype: &str) -> Option<String> {
    let normalized = dtype.trim().to_ascii_lowercase().replace('_', "");
    match normalized.strip_prefix("torch.") {
        Some(dtype) if !dtype.is_empty() => Some(dtype.to_string()),
        _ if !normalized.is_empty() => Some(normalized),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::EncodedImage;

    fn image_request() -> ImageGenerationRequest {
        ImageGenerationRequest {
            model: "image/stable-diffusion/tiny-sd".to_string(),
            prompt: "a compact test image".to_string(),
            negative_prompt: Some("blur".to_string()),
            width: Some(512),
            height: Some(512),
            num_inference_steps: Some(8),
            guidance_scale: Some(7.5),
            seed: Some(42),
            denoising_scheduler: None,
            num_images_per_prompt: Some(1),
            init_image: None,
            mask_image: None,
            strength: None,
            extra_options: serde_json::Value::Null,
        }
    }

    #[test]
    fn stable_diffusion_rules_keep_current_required_components_in_table_data() {
        let rules =
            image_generation_family_rules(ImageGenerationFamilyLabel::StableDiffusion).unwrap();

        assert_eq!(rules.family, ImageGenerationFamilyLabel::StableDiffusion);
        assert_eq!(
            rules.required_components,
            STABLE_DIFFUSION_REQUIRED_COMPONENTS
        );
    }

    #[test]
    fn stable_diffusion_rules_reject_unsupported_family_options() {
        let rules =
            image_generation_family_rules(ImageGenerationFamilyLabel::StableDiffusion).unwrap();
        let request = ImageGenerationRequest {
            denoising_scheduler: Some("euler".to_string()),
            init_image: Some(EncodedImage {
                data_base64: "aW1hZ2U=".to_string(),
                mime_type: "image/png".to_string(),
                width: Some(32),
                height: Some(32),
            }),
            mask_image: Some(EncodedImage {
                data_base64: "bWFzaw==".to_string(),
                mime_type: "image/png".to_string(),
                width: Some(32),
                height: Some(32),
            }),
            strength: Some(0.5),
            extra_options: serde_json::json!({ "adapter:opaque_option": true }),
            ..image_request()
        };

        let field_paths = rules
            .unsupported_request_options(&request, true)
            .into_iter()
            .map(|unsupported| unsupported.field_path)
            .collect::<Vec<_>>();

        assert_eq!(
            field_paths,
            vec![
                "request.denoising_scheduler",
                "request.init_image",
                "request.mask_image",
                "request.strength",
                "request.extra_options",
            ]
        );
    }

    #[test]
    fn stable_diffusion_rules_accept_current_text_to_image_options() {
        let rules =
            image_generation_family_rules(ImageGenerationFamilyLabel::StableDiffusion).unwrap();

        assert_eq!(
            rules.unsupported_request_options(&image_request(), false),
            Vec::new()
        );
        assert!(rules.supports_transformers_dtype("float32"));
        assert!(rules.supports_transformers_dtype("torch.float16"));
        assert!(rules.supports_transformers_dtype("bfloat16"));
        assert!(!rules.supports_transformers_dtype("int8"));
    }
}
