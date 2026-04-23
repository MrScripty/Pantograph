"""Shared runtime helpers for the PyTorch worker facade."""

import base64
import io
import json
import logging
from pathlib import Path

import torch

logger = logging.getLogger("pantograph.pytorch")


def _resolve_model_directory(model_path):
    """Return the containing model directory for a resolved model artifact path."""
    path = Path(model_path)
    if path.is_file():
        return path.parent
    return path


def _detect_model_type(path):
    """Auto-detect model type from config.json."""
    config_path = path / "config.json" if path.is_dir() else path.parent / "config.json"

    if config_path.exists():
        try:
            with open(config_path) as f:
                config = json.load(f)

            architectures = config.get("architectures", [])
            model_type_field = config.get("model_type", "")

            if any("dllm" in arch.lower() or "sdar" in arch.lower() for arch in architectures):
                return "dllm"
            if "dllm" in model_type_field.lower() or "sdar" in model_type_field.lower():
                return "dllm"

            if any("sherry" in arch.lower() for arch in architectures):
                return "sherry"
            if "sherry" in model_type_field.lower():
                return "sherry"

        except (json.JSONDecodeError, OSError) as e:
            logger.warning("Failed to read config.json: %s", e)

    return "text-generation"


def _resolve_device(device_str):
    """Resolve a device string to a torch.device.

    "auto" picks the best available: cuda > mps > cpu.
    """
    if device_str == "auto":
        if torch.cuda.is_available():
            return torch.device("cuda")
        if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
            return torch.device("mps")
        return torch.device("cpu")

    return torch.device(device_str)


def _resolve_torch_dtype(device, requested_dtype=None):
    if isinstance(requested_dtype, str):
        normalized = requested_dtype.strip().lower()
        if normalized in {"bf16", "bfloat16"}:
            return torch.bfloat16
        if normalized in {"fp16", "float16", "half"}:
            return torch.float16
        if normalized in {"fp32", "float32", "float"}:
            return torch.float32

    if isinstance(device, torch.device):
        device_type = device.type
    else:
        device_type = str(device)

    if device_type == "cuda":
        if hasattr(torch.cuda, "is_bf16_supported") and torch.cuda.is_bf16_supported():
            return torch.bfloat16
        return torch.float16
    if device_type == "mps":
        return torch.float16
    return torch.float32


def _dtype_name(dtype):
    if dtype is None:
        return None
    for name in ("bfloat16", "float16", "float32"):
        if getattr(torch, name) == dtype:
            return name
    return str(dtype)


def _detect_diffusion_load_overrides(bundle_root):
    """Infer narrow from_pretrained overrides from a diffusers bundle layout."""
    safetensor_variants = set()
    saw_safetensors = False

    for child in bundle_root.iterdir():
        if not child.is_dir():
            continue
        for candidate in child.iterdir():
            if not candidate.is_file():
                continue
            name = candidate.name
            if not name.endswith(".safetensors"):
                continue
            saw_safetensors = True
            stem = candidate.stem
            if "." not in stem:
                continue
            variant = stem.rsplit(".", 1)[-1].strip().lower()
            if variant:
                safetensor_variants.add(variant)

    overrides = {}
    if saw_safetensors:
        overrides["use_safetensors"] = True
    if len(safetensor_variants) == 1:
        overrides["variant"] = next(iter(safetensor_variants))
    return overrides


def _decode_base64_image(value):
    from PIL import Image

    if isinstance(value, dict):
        encoded = value.get("data_base64")
    else:
        encoded = value
    if not isinstance(encoded, str) or not encoded.strip():
        raise RuntimeError("Expected base64 image payload")
    raw = base64.b64decode(encoded)
    return Image.open(io.BytesIO(raw)).convert("RGB")


def _encode_image(image):
    buffer = io.BytesIO()
    image.save(buffer, format="PNG")
    return {
        "data_base64": base64.b64encode(buffer.getvalue()).decode("ascii"),
        "mime_type": "image/png",
        "width": getattr(image, "width", None),
        "height": getattr(image, "height", None),
    }
