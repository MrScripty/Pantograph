"""Image-generation worker envelope validation helpers that do not import torch."""

import json

from worker_contract import WORKER_CONTRACT_VERSION, _worker_device_or_auto

GENERATE_IMAGE_OPERATION = "generate_image"
IMAGE_GENERATION_PAYLOAD_KEYS = {
    "model_ref",
    "artifact_entry_path",
    "family",
    "pipeline_class",
    "required_components",
    "device",
    "prompt",
    "negative_prompt",
    "width",
    "height",
    "num_inference_steps",
    "guidance_scale",
    "seed",
    "denoising_scheduler",
    "num_images_per_prompt",
}
MODEL_REF_KEYS = {
    "model_id",
    "revision",
    "selected_artifact_id",
    "selected_artifact_path",
    "migration_diagnostics",
}


def _reject_unknown_keys(payload, allowed_keys, context):
    unknown_keys = sorted(key for key in payload if key not in allowed_keys)
    if unknown_keys:
        joined = ", ".join(unknown_keys)
        raise ValueError(f"PyTorch worker {context} contains unsupported key(s): {joined}")


def _require_non_empty_string(payload, key, context):
    value = payload.get(key)
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"PyTorch worker {context}.{key} must be a non-empty string")
    return value.strip()


def _validate_positive_int(payload, key):
    value = payload.get(key)
    if value is not None and (
        isinstance(value, bool) or not isinstance(value, int) or value <= 0
    ):
        raise ValueError(
            f"PyTorch worker generate_image payload.{key} must be a positive integer"
        )
    return value


def generate_image_kwargs_from_envelope(envelope):
    """Validate a Rust-planned image-generation envelope and project call kwargs."""
    if isinstance(envelope, str):
        envelope = json.loads(envelope)
    if not isinstance(envelope, dict):
        raise ValueError("PyTorch worker generate_image envelope must be an object")
    contract_version = envelope.get("contract_version")
    if contract_version != WORKER_CONTRACT_VERSION:
        raise ValueError(f"Unsupported PyTorch worker contract_version: {contract_version}")
    operation = envelope.get("operation")
    if operation != GENERATE_IMAGE_OPERATION:
        raise ValueError(f"Unexpected PyTorch worker operation for generate_image: {operation}")

    payload = envelope.get("payload")
    if not isinstance(payload, dict):
        raise ValueError("PyTorch worker generate_image envelope payload must be an object")
    _reject_unknown_keys(payload, IMAGE_GENERATION_PAYLOAD_KEYS, "generate_image payload")

    model_ref = payload.get("model_ref")
    if not isinstance(model_ref, dict):
        raise ValueError("PyTorch worker generate_image payload.model_ref must be an object")
    _reject_unknown_keys(model_ref, MODEL_REF_KEYS, "generate_image payload.model_ref")
    _require_non_empty_string(model_ref, "model_id", "generate_image payload.model_ref")

    artifact_entry_path = _require_non_empty_string(
        payload, "artifact_entry_path", "generate_image payload"
    )
    family = _require_non_empty_string(payload, "family", "generate_image payload")
    pipeline_class = _require_non_empty_string(
        payload, "pipeline_class", "generate_image payload"
    )
    prompt = _require_non_empty_string(payload, "prompt", "generate_image payload")

    required_components = payload.get("required_components")
    if not isinstance(required_components, list) or not required_components:
        raise ValueError(
            "PyTorch worker generate_image payload.required_components must be a non-empty list"
        )
    if any(
        not isinstance(component, str) or not component.strip()
        for component in required_components
    ):
        raise ValueError(
            "PyTorch worker generate_image payload.required_components "
            "must contain non-empty strings"
        )

    if "device" not in payload:
        raise ValueError(
            "PyTorch worker generate_image payload.device must be selected by Rust"
        )
    device = _worker_device_or_auto(payload, "generate_image")

    for key in ("negative_prompt", "denoising_scheduler"):
        value = payload.get(key)
        if value is not None and not isinstance(value, str):
            raise ValueError(f"PyTorch worker generate_image payload.{key} must be a string")

    width = _validate_positive_int(payload, "width")
    height = _validate_positive_int(payload, "height")
    steps = _validate_positive_int(payload, "num_inference_steps")
    image_count = _validate_positive_int(payload, "num_images_per_prompt")
    seed = payload.get("seed")
    if seed is not None and (
        isinstance(seed, bool) or not isinstance(seed, int) or seed < 0
    ):
        raise ValueError(
            "PyTorch worker generate_image payload.seed must be a non-negative integer"
        )
    guidance_scale = payload.get("guidance_scale")
    if guidance_scale is not None and (
        isinstance(guidance_scale, bool) or not isinstance(guidance_scale, (int, float))
    ):
        raise ValueError("PyTorch worker generate_image payload.guidance_scale must be a number")

    generation_kwargs = {
        "prompt": prompt,
        "negative_prompt": payload.get("negative_prompt"),
        "width": width,
        "height": height,
        "num_inference_steps": steps,
        "guidance_scale": float(guidance_scale) if guidance_scale is not None else None,
        "seed": seed,
        "denoising_scheduler": payload.get("denoising_scheduler"),
        "num_images_per_prompt": image_count,
    }
    return {
        "model_ref": model_ref,
        "artifact_entry_path": artifact_entry_path,
        "family": family,
        "pipeline_class": pipeline_class,
        "required_components": [component.strip() for component in required_components],
        "device": device,
        "generation_kwargs": generation_kwargs,
    }
