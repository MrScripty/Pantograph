"""PyTorch inference worker for Pantograph.

Embedded in the Rust process via PyO3. Provides model loading, generation,
and streaming token output for HuggingFace, dLLM, and Sherry models.

All public functions are called from Rust through PyO3's Python::with_gil.
Module-level globals hold the loaded model state.

Generation logic is split into sibling modules:
  - block_diffusion: dLLM / SDAR / TraDo block diffusion generation
  - autoregressive: standard token-by-token HuggingFace generation
"""

import json
import logging
import sys
from pathlib import Path

import torch

# When loaded from the filesystem, ensure sibling modules are importable.
# When embedded via PyO3's PyModule::from_code(), __file__ won't be a real
# path and this is a no-op — the Rust host must register siblings separately.
_self_path = Path(__file__).resolve()
if _self_path.parent.is_dir():
    _torch_dir = str(_self_path.parent)
    if _torch_dir not in sys.path:
        sys.path.insert(0, _torch_dir)

from block_diffusion import (
    _generate_dllm,
    _generate_dllm_streaming,
)
from autoregressive import (
    _generate_autoregressive,
    _generate_autoregressive_streaming,
)

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("pantograph.pytorch")

# Module-level state — one model loaded at a time
_model = None
_tokenizer = None
_device = None
_model_path = None
_model_type = None


def is_loaded():
    """Check whether a model is currently loaded."""
    return _model is not None


def get_loaded_info():
    """Return metadata about the currently loaded model, or None."""
    if _model is None:
        return None
    return {
        "model_path": str(_model_path) if _model_path else None,
        "model_type": _model_type,
        "device": str(_device),
    }


def _apply_compatibility_shims():
    """Patch transformers modules for cross-version compatibility.

    Models loaded via trust_remote_code=True (e.g. SDAR/TraDo) may import
    names that were removed in newer transformers versions. This injects
    aliases so the model code works regardless of installed version.

    Known removals:
      - transformers 4.54: LossKwargs removed from transformers.utils
      - transformers 5.0:  SlidingWindowCache removed from cache_utils
      - transformers 5.0:  pad_token_id etc. removed from PretrainedConfig defaults
    """
    import importlib.metadata
    import transformers.cache_utils as cu
    import transformers.utils as tu

    version = importlib.metadata.version("transformers")
    major, minor = (int(x) for x in version.split(".")[:2])

    # SlidingWindowCache -> DynamicSlidingWindowLayer (removed in 5.0)
    if not hasattr(cu, "SlidingWindowCache") and hasattr(cu, "DynamicSlidingWindowLayer"):
        cu.SlidingWindowCache = cu.DynamicSlidingWindowLayer
        logger.info("Shimmed SlidingWindowCache -> DynamicSlidingWindowLayer (transformers %s)", version)

    # LossKwargs removed in 4.54 — stub it as a TypedDict
    if not hasattr(tu, "LossKwargs"):
        from typing import Optional, TypedDict
        class LossKwargs(TypedDict, total=False):
            num_items_in_batch: Optional["torch.Tensor"]
        tu.LossKwargs = LossKwargs
        logger.info("Shimmed LossKwargs stub into transformers.utils (transformers %s)", version)

    # PretrainedConfig no longer sets default token IDs in 5.x — patch __init__
    # so older custom config classes that never set these still work.
    if major >= 5:
        from transformers import PretrainedConfig
        _orig_config_init = PretrainedConfig.__init__
        if not getattr(PretrainedConfig, "_pantograph_patched", False):
            _TOKEN_DEFAULTS = {"pad_token_id": None, "bos_token_id": None, "eos_token_id": None}
            def _patched_config_init(self, **kwargs):
                _orig_config_init(self, **kwargs)
                for attr, default in _TOKEN_DEFAULTS.items():
                    if not hasattr(self, attr):
                        setattr(self, attr, default)
            PretrainedConfig.__init__ = _patched_config_init
            PretrainedConfig._pantograph_patched = True
            logger.info("Shimmed PretrainedConfig token ID defaults (transformers %s)", version)


def load_model(model_path, device="auto", model_type=None):
    """Load a model + tokenizer into module globals.

    Args:
        model_path: Filesystem path to model directory.
        device: Device string — "auto", "cpu", "cuda", "cuda:0", "mps", etc.
        model_type: Optional hint — "dllm", "sherry", or "text-generation".
                    If None, auto-detected from config.json.

    Returns:
        Dict with model_path, model_type, device.
    """
    from transformers import AutoModelForCausalLM, AutoTokenizer

    _apply_compatibility_shims()

    global _model, _tokenizer, _device, _model_path, _model_type

    # Unload previous model first
    if _model is not None:
        unload_model()

    path = Path(model_path)
    if not path.exists():
        raise FileNotFoundError(f"Model path does not exist: {model_path}")

    resolved_device = _resolve_device(device)
    detected_type = model_type or _detect_model_type(path)

    logger.info(
        "Loading %s model from %s onto %s", detected_type, model_path, resolved_device
    )

    tokenizer = AutoTokenizer.from_pretrained(
        str(path), trust_remote_code=True
    )

    model = AutoModelForCausalLM.from_pretrained(
        str(path),
        torch_dtype="auto",
        device_map=str(resolved_device),
        trust_remote_code=True,
        low_cpu_mem_usage=True,
    )
    model.eval()

    _model = model
    _tokenizer = tokenizer
    _device = resolved_device
    _model_path = path
    _model_type = detected_type

    logger.info("Model loaded: %s (%s)", path.name, detected_type)
    return {
        "model_path": str(path),
        "model_type": detected_type,
        "device": str(resolved_device),
    }


def unload_model():
    """Unload the current model and free GPU memory."""
    global _model, _tokenizer, _device, _model_path, _model_type

    if _model is not None:
        name = _model_path.name if _model_path else "unknown"
        del _model
        del _tokenizer
        _model = None
        _tokenizer = None
        _device = None
        _model_path = None
        _model_type = None

        try:
            if torch.cuda.is_available():
                torch.cuda.empty_cache()
        except Exception:
            pass

        logger.info("Model unloaded: %s", name)


def generate(prompt, system_prompt=None, max_tokens=512, temperature=0.7, top_p=1.0):
    """Generate a complete response (non-streaming).

    Routes to block diffusion for dLLM models, standard generate otherwise.
    """
    if _model is None:
        raise RuntimeError("No model loaded. Call load_model() first.")

    formatted = _format_prompt(prompt, system_prompt)

    if _model_type == "dllm":
        return _generate_dllm(
            _model, _tokenizer, _device, formatted, max_tokens, temperature, top_p,
        )
    return _generate_autoregressive(
        _model, _tokenizer, _device, formatted, max_tokens, temperature, top_p,
    )


def generate_tokens(prompt, system_prompt=None, max_tokens=512, temperature=0.7, top_p=1.0):
    """Generate tokens as a Python generator (for streaming).

    dLLM models generate block-by-block; each decoded block is yielded as a
    chunk. Autoregressive models yield one token at a time.
    """
    if _model is None:
        raise RuntimeError("No model loaded. Call load_model() first.")

    formatted = _format_prompt(prompt, system_prompt)

    if _model_type == "dllm":
        yield from _generate_dllm_streaming(
            _model, _tokenizer, _device, formatted, max_tokens, temperature, top_p,
        )
    else:
        yield from _generate_autoregressive_streaming(
            _model, _tokenizer, _device, formatted, max_tokens, temperature, top_p,
        )


# --- Internal helpers ---

def _format_prompt(prompt, system_prompt=None):
    """Format user + system prompt into a single string.

    If the tokenizer has a chat template, use it. Otherwise fall back
    to a simple text format.
    """
    messages = []
    if system_prompt:
        messages.append({"role": "system", "content": system_prompt})
    messages.append({"role": "user", "content": prompt})

    # Try chat template first (most HF models support this)
    if hasattr(_tokenizer, "apply_chat_template"):
        try:
            return _tokenizer.apply_chat_template(
                messages, tokenize=False, add_generation_prompt=True
            )
        except Exception:
            pass

    # Fallback: simple text format
    parts = []
    if system_prompt:
        parts.append(f"System: {system_prompt}")
    parts.append(f"User: {prompt}")
    parts.append("Assistant:")
    return "\n".join(parts)


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
        elif hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
            return torch.device("mps")
        else:
            return torch.device("cpu")

    return torch.device(device_str)
