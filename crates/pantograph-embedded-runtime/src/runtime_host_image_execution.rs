use inference::{
    BackendExecutionDecision, BackendId, CapabilityAvailabilityState, DependencyReadinessFact,
    DependencyReadinessResolverOwner, DeviceResolutionDecision, ImageGenerationPlanningInput,
    ImageGenerationRequest, InferenceDeviceClass, InferenceDeviceId, InferenceDevicePolicy,
    InferenceTaskId, PlannedImageGenerationLaunchHandoff, PumasArtifactLoadPathKind,
    PumasArtifactLoadTarget, ResolvedModelPackageFacts, RuntimeVariantId,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionInputValue, ValidatedRuntimeHostExecutionRequest,
};
use pantograph_scheduler::SchedulerDispatchDecision;
use thiserror::Error;

const IMAGE_GENERATION_TASK: &str = "image_generation";
pub(crate) const PROMPT_PORT: &str = "prompt";
pub(crate) const NEGATIVE_PROMPT_PORT: &str = "negative_prompt";
pub(crate) const WIDTH_PORT: &str = "width";
pub(crate) const HEIGHT_PORT: &str = "height";
pub(crate) const STEPS_PORT: &str = "num_inference_steps";
pub(crate) const SEED_PORT: &str = "seed";
pub(crate) const GUIDANCE_SCALE_PORT: &str = "guidance_scale";
pub(crate) const NUM_IMAGES_PORT: &str = "num_images_per_prompt";
const DENOISING_SCHEDULER_PORT: &str = "denoising_scheduler";
const PYTORCH_BACKEND_ID: &str = "pytorch";
const PYTORCH_RUNTIME_ID: &str = "pytorch";
const DIFFUSERS_PYTORCH_RUNTIME_ID: &str = "diffusers-pytorch";

#[derive(Debug)]
pub(crate) struct RuntimeHostImageGenerationProjection {
    request: ImageGenerationRequest,
    launch_handoff: PlannedImageGenerationLaunchHandoff,
}

impl RuntimeHostImageGenerationProjection {
    pub(crate) fn planning_input(&self) -> ImageGenerationPlanningInput<'_> {
        ImageGenerationPlanningInput {
            request: &self.request,
            package_facts: self.launch_handoff.package_facts(),
            artifact_load_target: self.launch_handoff.artifact_load_target(),
            backend_decision: self.launch_handoff.backend_decision(),
        }
    }

    pub(crate) fn request(&self) -> &ImageGenerationRequest {
        &self.request
    }

    pub(crate) fn launch_handoff(&self) -> &PlannedImageGenerationLaunchHandoff {
        &self.launch_handoff
    }
}

pub(crate) fn project_runtime_host_image_generation(
    request: &ValidatedRuntimeHostExecutionRequest,
    package_facts: ResolvedModelPackageFacts,
    load_target: pumas_library::models::PumasArtifactLoadTarget,
) -> Result<RuntimeHostImageGenerationProjection, RuntimeHostImageGenerationProjectionError> {
    let request = request.as_ref();
    if request.handoff.task_intent.task_type.as_str() != IMAGE_GENERATION_TASK {
        return Err(RuntimeHostImageGenerationProjectionError::UnsupportedTask {
            task_type: request.handoff.task_intent.task_type.as_str().to_string(),
        });
    }
    let dispatch_decision = request
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or(RuntimeHostImageGenerationProjectionError::MissingDispatchDecision)?;
    let image_request = image_generation_request(request, dispatch_decision)?;
    let backend_decision = image_backend_decision(dispatch_decision)?;
    let load_target = project_pumas_artifact_load_target(load_target);
    let launch_handoff =
        PlannedImageGenerationLaunchHandoff::new(package_facts, load_target, backend_decision)
            .map_err(RuntimeHostImageGenerationProjectionError::InvalidLaunchHandoff)?;

    Ok(RuntimeHostImageGenerationProjection {
        request: image_request,
        launch_handoff,
    })
}

fn image_generation_request(
    request: &pantograph_runtime_host_contracts::RuntimeHostExecutionRequest,
    dispatch_decision: &SchedulerDispatchDecision,
) -> Result<ImageGenerationRequest, RuntimeHostImageGenerationProjectionError> {
    validate_supported_inputs(request)?;
    let prompt = required_string_input(request, PROMPT_PORT)?;
    if prompt.trim().is_empty() {
        return Err(RuntimeHostImageGenerationProjectionError::BlankPrompt);
    }
    Ok(ImageGenerationRequest {
        model: dispatch_decision.selected_model_ref.model_id.clone(),
        prompt,
        negative_prompt: optional_string_input(request, NEGATIVE_PROMPT_PORT)?,
        width: optional_u32_input(request, WIDTH_PORT)?,
        height: optional_u32_input(request, HEIGHT_PORT)?,
        num_inference_steps: optional_u32_input(request, STEPS_PORT)?,
        guidance_scale: optional_guidance_scale(request)?,
        seed: optional_u64_input(request, SEED_PORT)?,
        denoising_scheduler: denoising_scheduler(request, dispatch_decision)?,
        num_images_per_prompt: optional_image_count(request)?,
        init_image: None,
        mask_image: None,
        strength: None,
        extra_options: serde_json::Value::Null,
    })
}

fn image_backend_decision(
    decision: &SchedulerDispatchDecision,
) -> Result<BackendExecutionDecision, RuntimeHostImageGenerationProjectionError> {
    let selected_backend_id = image_backend_id(decision)?;
    let selected_runtime_variant_id = selected_runtime_variant_id(decision)?;
    let selected_device_id = selected_device_id(decision)?;
    let selected_device_class = device_class(selected_device_id.as_str())?;
    let device_decision = DeviceResolutionDecision {
        policy: InferenceDevicePolicy::Auto,
        runtime_variant_id: selected_runtime_variant_id.clone(),
        selected_device_class,
        selected_device_id: Some(selected_device_id.clone()),
        diagnostics: Vec::new(),
    };
    Ok(BackendExecutionDecision {
        selected_backend_id,
        selected_runtime_variant_id,
        selected_device_class,
        selected_device_id: Some(selected_device_id),
        device_decision,
        selected_task_id: Some(InferenceTaskId::ImageGeneration),
        selected_model_ref: Some(inference::PumasModelRef {
            model_id: decision.selected_model_ref.model_id.clone(),
            revision: decision.selected_model_ref.revision.clone(),
            selected_artifact_id: decision.selected_model_ref.selected_artifact_id.clone(),
            selected_artifact_path: None,
            migration_diagnostics: decision
                .selected_model_ref
                .migration_diagnostics
                .iter()
                .map(|diagnostic| inference::ModelRefMigrationDiagnostic {
                    code: diagnostic.code.clone(),
                    message: diagnostic.message.clone(),
                    input: diagnostic.input.clone(),
                })
                .collect(),
        }),
        diagnostics: Vec::new(),
        dependency_readiness: image_dependency_readiness_facts(),
        selection_policy_trace: None,
    })
}

fn image_backend_id(
    decision: &SchedulerDispatchDecision,
) -> Result<BackendId, RuntimeHostImageGenerationProjectionError> {
    let runtime_id = decision.selected_runtime_id.as_str();
    let runtime_variant_id = decision
        .selected_runtime_variant_id
        .as_ref()
        .map(std::string::ToString::to_string);
    if is_pytorch_image_runtime(runtime_id, runtime_variant_id.as_deref()) {
        return BackendId::parse(PYTORCH_BACKEND_ID).map_err(|error| {
            RuntimeHostImageGenerationProjectionError::InvalidBackendId {
                value: PYTORCH_BACKEND_ID.to_string(),
                message: error.to_string(),
            }
        });
    }
    Err(
        RuntimeHostImageGenerationProjectionError::UnsupportedRuntime {
            runtime_id: runtime_id.to_string(),
            runtime_variant_id,
        },
    )
}

fn is_pytorch_image_runtime(runtime_id: &str, runtime_variant_id: Option<&str>) -> bool {
    matches!(
        runtime_id,
        PYTORCH_RUNTIME_ID | DIFFUSERS_PYTORCH_RUNTIME_ID
    ) || runtime_variant_id.is_some_and(|value| {
        value == PYTORCH_RUNTIME_ID
            || value == DIFFUSERS_PYTORCH_RUNTIME_ID
            || value
                .strip_prefix(PYTORCH_RUNTIME_ID)
                .is_some_and(|suffix| suffix.starts_with('.'))
            || value
                .strip_prefix(DIFFUSERS_PYTORCH_RUNTIME_ID)
                .is_some_and(|suffix| suffix.starts_with('.'))
    })
}

fn selected_runtime_variant_id(
    decision: &SchedulerDispatchDecision,
) -> Result<RuntimeVariantId, RuntimeHostImageGenerationProjectionError> {
    let value = decision.selected_runtime_variant_id.as_ref().map_or_else(
        || decision.selected_runtime_id.to_string(),
        ToString::to_string,
    );
    RuntimeVariantId::parse(&value).map_err(|error| {
        RuntimeHostImageGenerationProjectionError::InvalidRuntimeVariantId {
            value,
            message: error.to_string(),
        }
    })
}

fn selected_device_id(
    decision: &SchedulerDispatchDecision,
) -> Result<InferenceDeviceId, RuntimeHostImageGenerationProjectionError> {
    let value = decision
        .selected_device_ids
        .first()
        .ok_or(RuntimeHostImageGenerationProjectionError::MissingSelectedDevice)?;
    InferenceDeviceId::parse(value.as_str()).map_err(|error| {
        RuntimeHostImageGenerationProjectionError::InvalidDeviceId {
            value: value.as_str().to_string(),
            message: error.to_string(),
        }
    })
}

fn device_class(
    device_id: &str,
) -> Result<InferenceDeviceClass, RuntimeHostImageGenerationProjectionError> {
    if device_id == "cpu" {
        return Ok(InferenceDeviceClass::Cpu);
    }
    if device_id.starts_with("cuda:") {
        return Ok(InferenceDeviceClass::Cuda);
    }
    if device_id == "mps" {
        return Ok(InferenceDeviceClass::Mps);
    }
    if device_id.starts_with("metal:") {
        return Ok(InferenceDeviceClass::Metal);
    }
    Err(
        RuntimeHostImageGenerationProjectionError::UnsupportedDevice {
            device_id: device_id.to_string(),
        },
    )
}

fn image_dependency_readiness_facts() -> Vec<DependencyReadinessFact> {
    inference::pytorch_diffusers_image_generation_package_requirements()
        .into_iter()
        .map(|declaration| {
            declaration.to_readiness_fact(
                CapabilityAvailabilityState::Available,
                DependencyReadinessResolverOwner::EmbeddedRuntime,
            )
        })
        .collect()
}

fn denoising_scheduler(
    request: &pantograph_runtime_host_contracts::RuntimeHostExecutionRequest,
    dispatch_decision: &SchedulerDispatchDecision,
) -> Result<Option<String>, RuntimeHostImageGenerationProjectionError> {
    if let Some(value) = optional_string_input(request, DENOISING_SCHEDULER_PORT)? {
        return Ok(Some(value));
    }
    dispatch_decision
        .runtime_trait_settings
        .iter()
        .find(|setting| setting.trait_id.as_str() == DENOISING_SCHEDULER_PORT)
        .map(|setting| match &setting.value {
            pantograph_scheduler::SchedulerTraitValue::String(value) => Ok(value.clone()),
            _ => Err(
                RuntimeHostImageGenerationProjectionError::InvalidTraitValue {
                    trait_id: DENOISING_SCHEDULER_PORT,
                    expected: "string",
                },
            ),
        })
        .transpose()
}

fn required_string_input(
    request: &pantograph_runtime_host_contracts::RuntimeHostExecutionRequest,
    port_id: &'static str,
) -> Result<String, RuntimeHostImageGenerationProjectionError> {
    optional_string_input(request, port_id)?
        .ok_or(RuntimeHostImageGenerationProjectionError::MissingRequiredInput { port_id })
}

fn optional_string_input(
    request: &pantograph_runtime_host_contracts::RuntimeHostExecutionRequest,
    port_id: &'static str,
) -> Result<Option<String>, RuntimeHostImageGenerationProjectionError> {
    optional_input(request, port_id)
        .map(|value| match value {
            RuntimeHostExecutionInputValue::String(value) => Ok(value.clone()),
            _ => Err(
                RuntimeHostImageGenerationProjectionError::InvalidInputType {
                    port_id,
                    expected: "string",
                },
            ),
        })
        .transpose()
}

fn optional_u64_input(
    request: &pantograph_runtime_host_contracts::RuntimeHostExecutionRequest,
    port_id: &'static str,
) -> Result<Option<u64>, RuntimeHostImageGenerationProjectionError> {
    optional_input(request, port_id)
        .map(|value| match value {
            RuntimeHostExecutionInputValue::U64(value) => Ok(*value),
            RuntimeHostExecutionInputValue::I64(value) if *value >= 0 => Ok(*value as u64),
            _ => Err(
                RuntimeHostImageGenerationProjectionError::InvalidInputType {
                    port_id,
                    expected: "u64",
                },
            ),
        })
        .transpose()
}

fn optional_u32_input(
    request: &pantograph_runtime_host_contracts::RuntimeHostExecutionRequest,
    port_id: &'static str,
) -> Result<Option<u32>, RuntimeHostImageGenerationProjectionError> {
    optional_u64_input(request, port_id)?
        .map(|value| {
            u32::try_from(value).map_err(|_| {
                RuntimeHostImageGenerationProjectionError::IntegerInputOutOfRange { port_id, value }
            })
        })
        .transpose()
}

fn optional_image_count(
    request: &pantograph_runtime_host_contracts::RuntimeHostExecutionRequest,
) -> Result<Option<u32>, RuntimeHostImageGenerationProjectionError> {
    let count = optional_u32_input(request, NUM_IMAGES_PORT)?;
    if count.is_some_and(|count| {
        count == 0 || count as usize > pantograph_runtime_host_contracts::MAX_RUNTIME_HOST_OUTPUTS
    }) {
        return Err(RuntimeHostImageGenerationProjectionError::InvalidImageCount);
    }
    Ok(count)
}

fn optional_guidance_scale(
    request: &pantograph_runtime_host_contracts::RuntimeHostExecutionRequest,
) -> Result<Option<f32>, RuntimeHostImageGenerationProjectionError> {
    optional_input(request, GUIDANCE_SCALE_PORT)
        .map(|value| match value {
            RuntimeHostExecutionInputValue::F64(_)
            | RuntimeHostExecutionInputValue::I64(_)
            | RuntimeHostExecutionInputValue::U64(_) => value
                .try_as_f32()
                .map_err(|_| RuntimeHostImageGenerationProjectionError::InvalidGuidanceScale),
            _ => Err(
                RuntimeHostImageGenerationProjectionError::InvalidInputType {
                    port_id: GUIDANCE_SCALE_PORT,
                    expected: "finite number",
                },
            ),
        })
        .transpose()
}

fn optional_input<'a>(
    request: &'a pantograph_runtime_host_contracts::RuntimeHostExecutionRequest,
    port_id: &str,
) -> Option<&'a RuntimeHostExecutionInputValue> {
    request
        .materialized_inputs
        .iter()
        .find(|input| input.port_id == port_id)
        .map(|input| &input.value)
}

fn validate_supported_inputs(
    request: &pantograph_runtime_host_contracts::RuntimeHostExecutionRequest,
) -> Result<(), RuntimeHostImageGenerationProjectionError> {
    for input in &request.materialized_inputs {
        if !matches!(
            input.port_id.as_str(),
            PROMPT_PORT
                | NEGATIVE_PROMPT_PORT
                | WIDTH_PORT
                | HEIGHT_PORT
                | STEPS_PORT
                | SEED_PORT
                | GUIDANCE_SCALE_PORT
                | NUM_IMAGES_PORT
                | DENOISING_SCHEDULER_PORT
        ) {
            return Err(
                RuntimeHostImageGenerationProjectionError::UnsupportedInputPort {
                    port_id: input.port_id.clone(),
                },
            );
        }
    }
    Ok(())
}

pub(crate) fn project_pumas_artifact_load_target(
    target: pumas_library::models::PumasArtifactLoadTarget,
) -> PumasArtifactLoadTarget {
    PumasArtifactLoadTarget {
        model_ref: inference::PumasModelRef {
            model_id: target.model_ref.model_id,
            revision: target.model_ref.revision,
            selected_artifact_id: target.model_ref.selected_artifact_id,
            selected_artifact_path: target.model_ref.selected_artifact_path,
            migration_diagnostics: target
                .model_ref
                .migration_diagnostics
                .into_iter()
                .map(|diagnostic| inference::ModelRefMigrationDiagnostic {
                    code: diagnostic.code,
                    message: diagnostic.message,
                    input: diagnostic.input,
                })
                .collect(),
        },
        artifact_kind: project_artifact_kind(target.artifact_kind),
        local_load_path: target.local_load_path,
        load_path_kind: match target.load_path_kind {
            pumas_library::models::PumasArtifactLoadPathKind::Directory => {
                PumasArtifactLoadPathKind::Directory
            }
            pumas_library::models::PumasArtifactLoadPathKind::File => {
                PumasArtifactLoadPathKind::File
            }
        },
        library_root_id: target.library_root_id,
        storage_kind: match target.storage_kind {
            pumas_library::models::StorageKind::LibraryOwned => {
                inference::ModelStorageKind::LibraryOwned
            }
            pumas_library::models::StorageKind::ExternalReference => {
                inference::ModelStorageKind::ExternalReference
            }
        },
        validation_state: match target.validation_state {
            pumas_library::models::AssetValidationState::Valid => {
                inference::ModelValidationState::Valid
            }
            pumas_library::models::AssetValidationState::Degraded => {
                inference::ModelValidationState::Degraded
            }
            pumas_library::models::AssetValidationState::Invalid => {
                inference::ModelValidationState::Invalid
            }
        },
        content_fingerprint: target.content_fingerprint,
        package_facts_contract_version: target.package_facts_contract_version,
    }
}

fn project_artifact_kind(
    kind: pumas_library::models::PackageArtifactKind,
) -> inference::ModelArtifactKind {
    match kind {
        pumas_library::models::PackageArtifactKind::Gguf => inference::ModelArtifactKind::Gguf,
        pumas_library::models::PackageArtifactKind::HfCompatibleDirectory => {
            inference::ModelArtifactKind::HfCompatibleDirectory
        }
        pumas_library::models::PackageArtifactKind::Safetensors => {
            inference::ModelArtifactKind::Safetensors
        }
        pumas_library::models::PackageArtifactKind::DiffusersBundle => {
            inference::ModelArtifactKind::DiffusersBundle
        }
        pumas_library::models::PackageArtifactKind::Onnx => inference::ModelArtifactKind::Onnx,
        pumas_library::models::PackageArtifactKind::Adapter => {
            inference::ModelArtifactKind::Adapter
        }
        pumas_library::models::PackageArtifactKind::Shard => inference::ModelArtifactKind::Shard,
        pumas_library::models::PackageArtifactKind::Unknown => {
            inference::ModelArtifactKind::Unknown
        }
    }
}

#[derive(Debug, Error, PartialEq)]
pub(crate) enum RuntimeHostImageGenerationProjectionError {
    #[error("runtime-host image execution supports image_generation only, got {task_type}")]
    UnsupportedTask { task_type: String },
    #[error("runtime-host image execution requires a scheduler dispatch decision")]
    MissingDispatchDecision,
    #[error("runtime-host image execution requires materialized input '{port_id}'")]
    MissingRequiredInput { port_id: &'static str },
    #[error("runtime-host image execution prompt must not be blank")]
    BlankPrompt,
    #[error("runtime-host image input '{port_id}' must be {expected}")]
    InvalidInputType {
        port_id: &'static str,
        expected: &'static str,
    },
    #[error("runtime-host image execution does not support materialized input '{port_id}'")]
    UnsupportedInputPort { port_id: String },
    #[error("runtime-host image input '{port_id}' value {value} exceeds u32")]
    IntegerInputOutOfRange { port_id: &'static str, value: u64 },
    #[error(
        "runtime-host image input 'guidance_scale' must retain authored precision in finite f32"
    )]
    InvalidGuidanceScale,
    #[error("runtime-host image input 'num_images_per_prompt' exceeds the positive per-task output limit")]
    InvalidImageCount,
    #[error("runtime-host image trait '{trait_id}' must be {expected}")]
    InvalidTraitValue {
        trait_id: &'static str,
        expected: &'static str,
    },
    #[error("runtime-host image execution does not support runtime {runtime_id} variant {runtime_variant_id:?}")]
    UnsupportedRuntime {
        runtime_id: String,
        runtime_variant_id: Option<String>,
    },
    #[error("invalid image backend id '{value}': {message}")]
    InvalidBackendId { value: String, message: String },
    #[error("invalid runtime variant id '{value}': {message}")]
    InvalidRuntimeVariantId { value: String, message: String },
    #[error("runtime-host image execution requires a selected device")]
    MissingSelectedDevice,
    #[error("invalid selected device id '{value}': {message}")]
    InvalidDeviceId { value: String, message: String },
    #[error("unsupported selected image device '{device_id}'")]
    UnsupportedDevice { device_id: String },
    #[error("invalid planned image launch handoff: {0}")]
    InvalidLaunchHandoff(#[from] inference::PlannedImageGenerationLaunchHandoffError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_runtime_host_contracts::{
        RuntimeHostExecutionInput, RuntimeHostExecutionRequest,
        ValidatedRuntimeHostExecutionRequest,
    };

    #[test]
    fn versioned_pumas_load_target_wire_projects_without_losing_consumer_facts() {
        let wire: serde_json::Value = serde_json::from_str(include_str!(
            "../../inference/tests/fixtures/runtime_load/pumas_artifact_load_target_wire.json"
        ))
        .expect("producer wire fixture");
        // Decode with the actual pinned producer DTO, never the strict consumer
        // mirror. This also catches fixture drift against producer serialization.
        let producer: pumas_library::models::PumasArtifactLoadTarget =
            serde_json::from_value(wire.clone()).expect("pinned Pumas wire must decode");
        assert_eq!(
            producer.model_ref.model_ref_contract_version,
            pumas_library::models::PUMAS_MODEL_REF_CONTRACT_VERSION
        );
        assert_eq!(serde_json::to_value(&producer).unwrap(), wire);
        assert!(serde_json::from_value::<PumasArtifactLoadTarget>(wire).is_err());

        let projected = project_pumas_artifact_load_target(producer);
        projected
            .validate_for_handoff()
            .expect("handoff must validate");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../inference/tests/fixtures/runtime_load/pantograph_artifact_load_target_projected.json"
        ))
        .expect("consumer projection fixture");
        assert_eq!(
            serde_json::to_value(&projected).unwrap(),
            expected,
            "the real host adapter preserves every declared consumer fact"
        );
        assert_eq!(
            serde_json::from_value::<PumasArtifactLoadTarget>(expected).unwrap(),
            projected
        );
    }

    #[test]
    fn advertised_basic_image_controls_reach_canonical_planning_without_loss() {
        let ports: Vec<pantograph_inference_interface_contracts::InferencePortDescriptor> =
            serde_json::from_str(include_str!(
                "../../pantograph-inference-interface-contracts/tests/fixtures/image_generation_basic_inputs.json"
            ))
            .expect("shared image controls fixture");
        let mut request = runtime_host_request_fixture();
        request
            .handoff
            .dispatch_decision
            .as_mut()
            .unwrap()
            .runtime_trait_settings
            .clear();
        request.materialized_inputs = ports
            .into_iter()
            .zip([
                RuntimeHostExecutionInputValue::String("a red cube".into()),
                RuntimeHostExecutionInputValue::String(" blur, low quality ".into()),
                RuntimeHostExecutionInputValue::U64(512),
                RuntimeHostExecutionInputValue::U64(768),
                RuntimeHostExecutionInputValue::U64(12),
                RuntimeHostExecutionInputValue::U64(u64::MAX),
                RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(7.5).unwrap()),
                RuntimeHostExecutionInputValue::U64(3),
            ])
            .map(|(port, value)| input(port.port_id.as_str(), value))
            .collect();
        let request = ValidatedRuntimeHostExecutionRequest::try_from(request).unwrap();
        let package = image_package_facts();
        let target = image_load_target(&package);
        let projection = project_runtime_host_image_generation(&request, package, target).unwrap();
        let inference::ImageGenerationPlanningOutcome::Planned { plan } =
            inference::plan_image_generation_execution(projection.planning_input())
        else {
            panic!("advertised controls must produce an image plan");
        };
        assert_eq!(plan.prompt, "a red cube");
        assert_eq!(plan.negative_prompt.as_deref(), Some(" blur, low quality "));
        assert_eq!(plan.width, Some(512));
        assert_eq!(plan.height, Some(768));
        assert_eq!(plan.num_inference_steps, Some(12));
        assert_eq!(plan.seed, Some(u64::MAX));
        assert_eq!(plan.guidance_scale, Some(7.5));
        assert_eq!(plan.num_images_per_prompt, Some(3));
    }

    #[test]
    fn image_guidance_preserves_existing_finite_f32_semantics_without_clamping() {
        for expected in [f32::MIN, -1.0, 0.0, 1.0, 7.5, f32::MAX] {
            let mut request = runtime_host_request_fixture();
            request.materialized_inputs.push(input(
                GUIDANCE_SCALE_PORT,
                RuntimeHostExecutionInputValue::F64(
                    serde_json::Number::from_f64(f64::from(expected)).unwrap(),
                ),
            ));
            assert_eq!(optional_guidance_scale(&request).unwrap(), Some(expected));
        }
        for value in [
            RuntimeHostExecutionInputValue::I64(-1),
            RuntimeHostExecutionInputValue::U64(1),
        ] {
            let mut request = runtime_host_request_fixture();
            request
                .materialized_inputs
                .push(input(GUIDANCE_SCALE_PORT, value.clone()));
            assert_eq!(
                optional_guidance_scale(&request).unwrap(),
                Some(value.try_as_f32().unwrap())
            );
        }
        for value in [f64::MAX, 1e-300, 1.0000000000000002] {
            let mut request = runtime_host_request_fixture();
            request.materialized_inputs.push(input(
                GUIDANCE_SCALE_PORT,
                RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(value).unwrap()),
            ));
            assert_eq!(
                optional_guidance_scale(&request),
                Err(RuntimeHostImageGenerationProjectionError::InvalidGuidanceScale)
            );
        }
        let mut request = runtime_host_request_fixture();
        request.materialized_inputs.push(input(
            GUIDANCE_SCALE_PORT,
            RuntimeHostExecutionInputValue::String("7.5".into()),
        ));
        assert!(matches!(
            optional_guidance_scale(&request),
            Err(
                RuntimeHostImageGenerationProjectionError::InvalidInputType {
                    port_id: GUIDANCE_SCALE_PORT,
                    ..
                }
            )
        ));
    }

    #[test]
    fn omitted_basic_image_controls_preserve_backend_defaults() {
        let mut request = runtime_host_request_fixture();
        request
            .materialized_inputs
            .retain(|input| input.port_id == PROMPT_PORT);
        let request = ValidatedRuntimeHostExecutionRequest::try_from(request).unwrap();
        let package = image_package_facts();
        let target = image_load_target(&package);
        let projection = project_runtime_host_image_generation(&request, package, target).unwrap();
        let image = projection.request();
        assert_eq!(image.negative_prompt, None);
        assert_eq!(image.width, None);
        assert_eq!(image.height, None);
        assert_eq!(image.num_inference_steps, None);
        assert_eq!(image.seed, None);
        assert_eq!(image.guidance_scale, None);
    }

    #[test]
    fn projects_valid_runtime_host_image_request_to_planning_input() {
        let request = validated_runtime_host_request();
        let package_facts = image_package_facts();
        let load_target = image_load_target(&package_facts);

        let projection =
            project_runtime_host_image_generation(&request, package_facts, load_target)
                .expect("runtime-host image request should project");
        let planning_input = projection.planning_input();

        assert_eq!(
            projection.request().prompt,
            "a cinematic image of a red cube on a white table"
        );
        assert_eq!(projection.request().seed, Some(42));
        assert_eq!(
            projection.request().denoising_scheduler.as_deref(),
            Some("EulerDiscreteScheduler")
        );
        assert_eq!(
            planning_input.backend_decision.selected_backend_id.as_str(),
            "pytorch"
        );
        assert_eq!(
            planning_input
                .backend_decision
                .selected_runtime_variant_id
                .as_str(),
            "diffusers-pytorch.cuda"
        );
        assert_eq!(
            planning_input
                .backend_decision
                .selected_device_id
                .as_ref()
                .map(InferenceDeviceId::as_str),
            Some("cuda:0")
        );
        assert_eq!(
            planning_input.backend_decision.dependency_readiness.len(),
            5
        );
        assert_eq!(
            projection
                .launch_handoff()
                .artifact_load_target()
                .local_load_path,
            "/pumas/models/image/stable-diffusion/tiny-sd"
        );
    }

    #[test]
    fn rejects_non_image_runtime_tasks_before_gateway_projection() {
        let mut request = runtime_host_request_fixture();
        request.handoff.task_intent.task_type = "text_generation".parse().expect("task type");
        request
            .handoff
            .readiness_proof
            .preflight_result
            .identity_key
            .task_id = "text_generation".parse().expect("task id");
        request
            .handoff
            .dispatch_decision
            .as_mut()
            .expect("dispatch decision")
            .task_intent
            .task_type = "text_generation".parse().expect("task type");
        request
            .handoff
            .dispatch_decision
            .as_mut()
            .expect("dispatch decision")
            .readiness_proof = request.handoff.readiness_proof.clone();
        let request = ValidatedRuntimeHostExecutionRequest::try_from(request)
            .expect("request remains contract-valid");

        let error = project_runtime_host_image_generation(
            &request,
            image_package_facts(),
            image_load_target(&image_package_facts()),
        )
        .expect_err("non-image tasks should not project");

        assert!(matches!(
            error,
            RuntimeHostImageGenerationProjectionError::UnsupportedTask { task_type }
                if task_type == "text_generation"
        ));
    }

    #[test]
    fn rejects_missing_prompt_without_using_defaults_or_legacy_inputs() {
        let mut request = runtime_host_request_fixture();
        request
            .materialized_inputs
            .retain(|input| input.port_id != PROMPT_PORT);
        let request = ValidatedRuntimeHostExecutionRequest::try_from(request)
            .expect("request remains contract-valid");

        let error = project_runtime_host_image_generation(
            &request,
            image_package_facts(),
            image_load_target(&image_package_facts()),
        )
        .expect_err("missing prompt should fail projection");

        assert!(matches!(
            error,
            RuntimeHostImageGenerationProjectionError::MissingRequiredInput {
                port_id: PROMPT_PORT
            }
        ));
    }

    #[test]
    fn rejects_unsupported_materialized_inputs_instead_of_ignoring_them() {
        let mut request = runtime_host_request_fixture();
        request.materialized_inputs.push(input(
            "guidance_rescale",
            RuntimeHostExecutionInputValue::String("7.5".to_string()),
        ));
        let request = ValidatedRuntimeHostExecutionRequest::try_from(request)
            .expect("request remains contract-valid");

        let error = project_runtime_host_image_generation(
            &request,
            image_package_facts(),
            image_load_target(&image_package_facts()),
        )
        .expect_err("unsupported inputs should fail projection");

        assert!(matches!(
            error,
            RuntimeHostImageGenerationProjectionError::UnsupportedInputPort { port_id }
                if port_id == "guidance_rescale"
        ));
    }

    #[test]
    fn rejects_non_pytorch_runtime_without_backend_guessing() {
        let mut request = runtime_host_request_fixture();
        request.handoff.task_intent.constraints.requested_runtime_id =
            Some("mlx".parse().expect("runtime id"));
        let decision = request
            .handoff
            .dispatch_decision
            .as_mut()
            .expect("dispatch decision");
        decision.task_intent.constraints.requested_runtime_id =
            Some("mlx".parse().expect("runtime id"));
        decision.selected_runtime_id = "mlx".parse().expect("runtime id");
        decision.selected_runtime_variant_id = Some("mlx.metal".parse().expect("variant id"));
        let request = ValidatedRuntimeHostExecutionRequest::try_from(request)
            .expect("request remains contract-valid");

        let error = project_runtime_host_image_generation(
            &request,
            image_package_facts(),
            image_load_target(&image_package_facts()),
        )
        .expect_err("unsupported runtimes should fail projection");

        assert!(matches!(
            error,
            RuntimeHostImageGenerationProjectionError::UnsupportedRuntime {
                runtime_id,
                runtime_variant_id
            } if runtime_id == "mlx" && runtime_variant_id.as_deref() == Some("mlx.metal")
        ));
    }

    fn validated_runtime_host_request() -> ValidatedRuntimeHostExecutionRequest {
        ValidatedRuntimeHostExecutionRequest::try_from(runtime_host_request_fixture())
            .expect("runtime-host request fixture should validate")
    }

    fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
        serde_json::from_str(include_str!(
            "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
        ))
        .expect("runtime host request fixture should deserialize")
    }

    fn image_package_facts() -> ResolvedModelPackageFacts {
        let mut package_facts: ResolvedModelPackageFacts = serde_json::from_str(include_str!(
            "../../inference/tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
        ))
        .expect("image package facts fixture should decode");
        package_facts.model_ref = inference::PumasModelRef {
            model_id: "pumas://models/juggernaut-xl-v10".to_string(),
            revision: Some("main".to_string()),
            selected_artifact_id: Some("diffusers-bundle".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        };
        package_facts
    }

    fn image_load_target(
        package_facts: &ResolvedModelPackageFacts,
    ) -> pumas_library::models::PumasArtifactLoadTarget {
        pumas_library::models::PumasArtifactLoadTarget {
            model_ref: pumas_library::models::PumasModelRef {
                model_id: package_facts.model_ref.model_id.clone(),
                revision: package_facts.model_ref.revision.clone(),
                selected_artifact_id: package_facts.model_ref.selected_artifact_id.clone(),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
                ..Default::default()
            },
            artifact_kind: pumas_library::models::PackageArtifactKind::DiffusersBundle,
            local_load_path: "/pumas/models/image/stable-diffusion/tiny-sd".to_string(),
            load_path_kind: pumas_library::models::PumasArtifactLoadPathKind::Directory,
            library_root_id: Some("test-root".to_string()),
            storage_kind: pumas_library::models::StorageKind::LibraryOwned,
            validation_state: pumas_library::models::AssetValidationState::Valid,
            content_fingerprint: None,
            package_facts_contract_version: Some(package_facts.package_facts_contract_version),
        }
    }

    #[allow(dead_code)]
    fn input(port_id: &str, value: RuntimeHostExecutionInputValue) -> RuntimeHostExecutionInput {
        RuntimeHostExecutionInput {
            port_id: port_id.to_string(),
            value,
        }
    }
}
