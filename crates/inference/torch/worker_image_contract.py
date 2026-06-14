"""Image-generation worker envelope validation helpers that do not import torch."""

import json

from worker_contract import (
    GENERATE_IMAGE_BATCH_OPERATION,
    GENERATE_IMAGE_OPERATION,
    WORKER_CONTRACT_VERSION,
    _worker_device_or_auto,
)

IMAGE_GENERATION_PAYLOAD_KEYS = {
    "model_ref",
    "artifact_load_target",
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
IMAGE_GENERATION_BATCH_PAYLOAD_KEYS = {
    "batch_execution_id",
    "anchor_member_id",
    "members",
}
IMAGE_GENERATION_BATCH_MEMBER_KEYS = {
    "member_id",
    "request",
}
MODEL_REF_KEYS = {
    "model_id",
    "revision",
    "selected_artifact_id",
    "selected_artifact_path",
    "migration_diagnostics",
}
ARTIFACT_LOAD_TARGET_KEYS = {
    "model_ref",
    "artifact_kind",
    "local_load_path",
    "load_path_kind",
    "library_root_id",
    "storage_kind",
    "validation_state",
    "content_fingerprint",
    "package_facts_contract_version",
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


def _validate_positive_int(payload, key, context):
    value = payload.get(key)
    if value is not None and (
        isinstance(value, bool) or not isinstance(value, int) or value <= 0
    ):
        raise ValueError(
            f"PyTorch worker {context}.{key} must be a positive integer"
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
    return _generate_image_kwargs_from_payload(payload, "generate_image payload")


def generate_image_batch_kwargs_from_envelope(envelope):
    """Validate a Rust-planned image batch envelope and project member call kwargs."""
    if isinstance(envelope, str):
        envelope = json.loads(envelope)
    if not isinstance(envelope, dict):
        raise ValueError("PyTorch worker generate_image_batch envelope must be an object")
    contract_version = envelope.get("contract_version")
    if contract_version != WORKER_CONTRACT_VERSION:
        raise ValueError(f"Unsupported PyTorch worker contract_version: {contract_version}")
    operation = envelope.get("operation")
    if operation != GENERATE_IMAGE_BATCH_OPERATION:
        raise ValueError(
            f"Unexpected PyTorch worker operation for generate_image_batch: {operation}"
        )

    payload = envelope.get("payload")
    if not isinstance(payload, dict):
        raise ValueError(
            "PyTorch worker generate_image_batch envelope payload must be an object"
        )
    _reject_unknown_keys(
        payload,
        IMAGE_GENERATION_BATCH_PAYLOAD_KEYS,
        "generate_image_batch payload",
    )
    batch_execution_id = _require_non_empty_string(
        payload, "batch_execution_id", "generate_image_batch payload"
    )
    anchor_member_id = _require_non_empty_string(
        payload, "anchor_member_id", "generate_image_batch payload"
    )
    members = payload.get("members")
    if not isinstance(members, list) or not members:
        raise ValueError(
            "PyTorch worker generate_image_batch payload.members must be a non-empty list"
        )

    seen_member_ids = set()
    projected_members = []
    for member in members:
        if not isinstance(member, dict):
            raise ValueError(
                "PyTorch worker generate_image_batch payload.members must contain objects"
            )
        _reject_unknown_keys(
            member,
            IMAGE_GENERATION_BATCH_MEMBER_KEYS,
            "generate_image_batch member",
        )
        member_id = _require_non_empty_string(
            member, "member_id", "generate_image_batch member"
        )
        if member_id in seen_member_ids:
            raise ValueError(
                "PyTorch worker generate_image_batch payload.members contains "
                f"duplicate member_id: {member_id}"
            )
        seen_member_ids.add(member_id)
        request = member.get("request")
        if not isinstance(request, dict):
            raise ValueError(
                "PyTorch worker generate_image_batch member.request must be an object"
            )
        projected_members.append(
            {
                "member_id": member_id,
                "planned": _generate_image_kwargs_from_payload(
                    request,
                    f"generate_image_batch member {member_id}.request",
                ),
            }
        )

    if anchor_member_id not in seen_member_ids:
        raise ValueError(
            "PyTorch worker generate_image_batch payload.anchor_member_id must reference "
            "a member"
        )

    return {
        "batch_execution_id": batch_execution_id,
        "anchor_member_id": anchor_member_id,
        "members": projected_members,
    }


def _generate_image_kwargs_from_payload(payload, context):
    _reject_unknown_keys(payload, IMAGE_GENERATION_PAYLOAD_KEYS, context)

    model_ref = payload.get("model_ref")
    if not isinstance(model_ref, dict):
        raise ValueError(f"PyTorch worker {context}.model_ref must be an object")
    _reject_unknown_keys(model_ref, MODEL_REF_KEYS, f"{context}.model_ref")
    _require_non_empty_string(model_ref, "model_id", f"{context}.model_ref")

    artifact_load_target = payload.get("artifact_load_target")
    if not isinstance(artifact_load_target, dict):
        raise ValueError(
            f"PyTorch worker {context}.artifact_load_target must be an object"
        )
    _reject_unknown_keys(
        artifact_load_target,
        ARTIFACT_LOAD_TARGET_KEYS,
        f"{context}.artifact_load_target",
    )
    local_load_path = _require_non_empty_string(
        artifact_load_target,
        "local_load_path",
        f"{context}.artifact_load_target",
    )
    load_path_kind = _require_non_empty_string(
        artifact_load_target,
        "load_path_kind",
        f"{context}.artifact_load_target",
    )
    if load_path_kind != "directory":
        raise ValueError(
            f"PyTorch worker {context}.artifact_load_target.load_path_kind "
            "must be directory"
        )
    artifact_kind = _require_non_empty_string(
        artifact_load_target,
        "artifact_kind",
        f"{context}.artifact_load_target",
    )
    if artifact_kind != "diffusers_bundle":
        raise ValueError(
            f"PyTorch worker {context}.artifact_load_target.artifact_kind "
            "must be diffusers_bundle"
        )
    family = _require_non_empty_string(payload, "family", context)
    pipeline_class = _require_non_empty_string(payload, "pipeline_class", context)
    prompt = _require_non_empty_string(payload, "prompt", context)

    required_components = payload.get("required_components")
    if not isinstance(required_components, list) or not required_components:
        raise ValueError(
            f"PyTorch worker {context}.required_components must be a non-empty list"
        )
    if any(
        not isinstance(component, str) or not component.strip()
        for component in required_components
    ):
        raise ValueError(
            f"PyTorch worker {context}.required_components "
            "must contain non-empty strings"
        )

    if "device" not in payload:
        raise ValueError(f"PyTorch worker {context}.device must be selected by Rust")
    device_context = (
        context[: -len(" payload")] if context.endswith(" payload") else context
    )
    device = _worker_device_or_auto(payload, device_context)

    for key in ("negative_prompt", "denoising_scheduler"):
        value = payload.get(key)
        if value is not None and not isinstance(value, str):
            raise ValueError(f"PyTorch worker {context}.{key} must be a string")

    width = _validate_positive_int(payload, "width", context)
    height = _validate_positive_int(payload, "height", context)
    steps = _validate_positive_int(payload, "num_inference_steps", context)
    image_count = _validate_positive_int(payload, "num_images_per_prompt", context)
    seed = payload.get("seed")
    if seed is not None and (
        isinstance(seed, bool) or not isinstance(seed, int) or seed < 0
    ):
        raise ValueError(
            f"PyTorch worker {context}.seed must be a non-negative integer"
        )
    guidance_scale = payload.get("guidance_scale")
    if guidance_scale is not None and (
        isinstance(guidance_scale, bool) or not isinstance(guidance_scale, (int, float))
    ):
        raise ValueError(f"PyTorch worker {context}.guidance_scale must be a number")

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
        "artifact_load_target": artifact_load_target,
        "local_load_path": local_load_path,
        "family": family,
        "pipeline_class": pipeline_class,
        "required_components": [component.strip() for component in required_components],
        "device": device,
        "generation_kwargs": generation_kwargs,
    }
