"""PyTorch inference worker for Pantograph.

Embedded in the Rust process via PyO3. Provides model loading, generation,
and streaming token output for HuggingFace, dLLM, and Sherry models.

All public functions are called from Rust through PyO3's Python::with_gil.
Module-level globals hold the loaded model state.

Generation logic is split into sibling modules:
  - block_diffusion: dLLM / SDAR / TraDo block diffusion generation
  - autoregressive: standard token-by-token HuggingFace generation
  - worker_runtime: shared device, model, dtype, and payload helpers
  - worker_transformers: cross-version transformers compatibility shims
"""

import base64
import inspect
import io
import json
import logging
import math
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
import torch

# When loaded from the filesystem, ensure sibling modules are importable.
# When embedded via PyO3's PyModule::from_code(), __file__ won't be a real
# path and this is a no-op — the Rust host must register siblings separately.
_self_path = Path(__file__).resolve()
if _self_path.parent.is_dir():
    _torch_dir = str(_self_path.parent)
    if _torch_dir not in sys.path:
        sys.path.insert(0, _torch_dir)

from block_diffusion import _generate_dllm_masked, _generate_dllm_masked_streaming
from autoregressive import (
    _generate_autoregressive,
    _generate_autoregressive_streaming,
    _continue_sdar_cached,
    _generate_sdar_cached,
)
from worker_runtime import (
    _decode_base64_image,
    _detect_diffusion_load_overrides,
    _detect_model_type,
    _dtype_name,
    _encode_image,
    _resolve_device,
    _resolve_model_directory,
    _resolve_torch_dtype,
)
from worker_transformers import apply_compatibility_shims
from worker_contract import (
    AUTOMATIC_SPEECH_RECOGNITION_LOADER,
    CAUSAL_LM_LOADER,
    clear_kv_cache_kwargs_from_envelope,
    GENERATE_TEXT_STREAM_OPERATION,
    generate_text_kwargs_from_envelope,
    get_loaded_info_kwargs_from_envelope,
    init_worker_kwargs_from_envelope,
    load_transformers_model_kwargs_from_envelope,
    restore_kv_cache_kwargs_from_envelope,
    save_kv_cache_kwargs_from_envelope,
    transcribe_audio_kwargs_from_envelope,
    truncate_kv_cache_kwargs_from_envelope,
    unload_model_kwargs_from_envelope,
)

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("pantograph.pytorch")

# Module-level state — one model loaded at a time
_model = None
_tokenizer = None
_device = None
_model_path = None
_model_type = None
_live_kv_state = None

_diffusion_pipeline = None
_diffusion_device = None
_diffusion_model_path = None
_diffusion_dtype = None

_asr_pipeline = None
_asr_device = None
_asr_model_path = None

_DIFFUSION_PREVIEW_MAX_EVENTS = 8
_DIFFUSION_PREVIEW_MAX_DIMENSION = 384


def init_worker_from_envelope(envelope):
    """Validate worker initialization from the Rust worker envelope contract."""
    request_id = "unknown"
    try:
        decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
        if isinstance(decoded, dict):
            request_id = str(decoded.get("request_id") or request_id)
        init_worker_kwargs_from_envelope(decoded)
        return json.dumps({
            "status": "ok",
            "request_id": request_id,
            "result": {"initialized": True},
        })
    except ValueError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "invalid_request",
                "message": str(exc),
                "canonical_code": "pytorch_worker_invalid_init_request",
            },
        })
    except Exception as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "internal",
                "message": str(exc),
                "canonical_code": "pytorch_worker_init_internal",
            },
        })


def _generate_dllm_autoregressive_safe(formatted_prompt, max_tokens, temperature, top_p, top_k=None):
    """Generate for TraDo/SDAR via native generate(), with empty-output retry.

    Some SDAR exports include chat delimiters in generation_config.eos_token_id,
    which can terminate immediately and decode to an empty string. Retry with a
    single EOS and a small min_new_tokens floor when that happens.
    """
    global _live_kv_state
    if _live_kv_state is not None:
        cached_token_ids = _live_kv_state.get("token_ids")
        cached_cache = _live_kv_state.get("cache")
        if isinstance(cached_token_ids, list) and cached_cache is not None:
            try:
                text, token_ids, cache = _continue_sdar_cached(
                    _model,
                    _tokenizer,
                    _device,
                    formatted_prompt,
                    max_tokens,
                    temperature,
                    top_p,
                    cached_token_ids,
                    cached_cache,
                    top_k=top_k,
                )
                _live_kv_state = {
                    "token_ids": token_ids,
                    "cache": cache,
                    "model_path": str(_model_path) if _model_path else None,
                    "model_type": _model_type,
                    "device": str(_device) if _device is not None else None,
                }
                return text
            except Exception as exc:
                logger.warning("Live KV reuse failed; falling back to fresh decode: %s", exc)
                _live_kv_state = None

    text, token_ids, cache = _generate_sdar_cached(
        _model, _tokenizer, _device, formatted_prompt, max_tokens, temperature, top_p, top_k=top_k,
    )
    _live_kv_state = {
        "token_ids": token_ids,
        "cache": cache,
        "model_path": str(_model_path) if _model_path else None,
        "model_type": _model_type,
        "device": str(_device) if _device is not None else None,
    }
    if text and text.strip():
        return text

    logger.warning("Empty dllm decode on SDAR path; retrying with stricter EOS settings")

    inputs = _tokenizer(formatted_prompt, return_tensors="pt").to(_device)
    retry_min_new = min(max_tokens, 24)
    eos_id = getattr(_tokenizer, "eos_token_id", None)
    pad_id = getattr(_tokenizer, "pad_token_id", eos_id)

    with torch.no_grad():
        outputs = _model.generate(
            **inputs,
            max_new_tokens=max_tokens,
            min_new_tokens=retry_min_new if retry_min_new > 0 else None,
            temperature=max(temperature, 0.01),
            top_p=top_p,
            top_k=int(top_k) if top_k is not None else getattr(_model.generation_config, "top_k", 0),
            do_sample=temperature > 0,
            eos_token_id=eos_id,
            pad_token_id=pad_id,
        )

    input_len = inputs["input_ids"].shape[1]
    generated = outputs[0][input_len:]
    _live_kv_state = None
    decoded = _tokenizer.decode(generated, skip_special_tokens=True)
    if decoded and decoded.strip():
        return decoded

    # Last resort: include special tokens so callers can see what happened.
    raw = _tokenizer.decode(generated, skip_special_tokens=False)
    return raw.strip()


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


def get_live_kv_info():
    """Return metadata about the current live KV snapshot, or None."""
    if _live_kv_state is None:
        return None
    return {
        "token_count": int(len(_live_kv_state.get("token_ids", []))),
        "model_path": _live_kv_state.get("model_path"),
        "model_type": _live_kv_state.get("model_type"),
        "device": _live_kv_state.get("device"),
    }


def get_loaded_diffusion_info():
    """Return metadata about the currently loaded diffusion pipeline, or None."""
    if _diffusion_pipeline is None:
        return None
    return {
        "model_path": str(_diffusion_model_path) if _diffusion_model_path else None,
        "device": str(_diffusion_device) if _diffusion_device is not None else None,
        "torch_dtype": _dtype_name(_diffusion_dtype),
    }


def get_loaded_asr_info():
    """Return metadata about the currently loaded ASR pipeline, or None."""
    if _asr_pipeline is None:
        return None
    return {
        "model_path": str(_asr_model_path) if _asr_model_path else None,
        "device": str(_asr_device) if _asr_device is not None else None,
    }


def _transformers_package_requires_remote_code(path):
    """Return whether config metadata declares custom Transformers code."""
    if path.is_dir():
        config_path = path / "config.json"
    else:
        config_path = path.parent / "config.json"
    if not config_path.exists():
        return False
    try:
        with open(config_path, encoding="utf-8") as f:
            config = json.load(f)
    except (json.JSONDecodeError, OSError) as e:
        logger.warning("Failed to inspect config.json for trust policy: %s", e)
        return False

    auto_map = config.get("auto_map")
    if isinstance(auto_map, dict):
        return bool(auto_map)
    if isinstance(auto_map, list):
        return bool(auto_map)
    return False


def load_transformers_model_from_envelope(envelope):
    """Load a Transformers model from the Rust worker envelope contract."""
    request_id = "unknown"
    try:
        decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
        if isinstance(decoded, dict):
            request_id = str(decoded.get("request_id") or request_id)
        kwargs = load_transformers_model_kwargs_from_envelope(decoded)
        if not kwargs.get("model_path"):
            raise ValueError("PyTorch worker load envelope missing payload.entry_path")
        loader = kwargs.pop("loader", CAUSAL_LM_LOADER)
        _validate_transformers_loader(loader)
    except ValueError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "invalid_request",
                "message": str(exc),
                "canonical_code": "pytorch_worker_invalid_load_request",
            },
        })

    try:
        info = _load_transformers_model_from_kwargs(loader, kwargs)
        return json.dumps({
            "status": "ok",
            "request_id": request_id,
            "result": info,
        })
    except RuntimeError as exc:
        message = str(exc)
        if "trust policy is closed" in message:
            kind = "trust_policy_rejected"
            canonical_code = "pytorch_transformers_trust_policy_rejected"
        else:
            kind = "model_load_failed"
            canonical_code = "pytorch_worker_model_load_failed"
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": kind,
                "message": message,
                "canonical_code": canonical_code,
            },
        })
    except (FileNotFoundError, ValueError, OSError, ImportError) as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "model_load_failed",
                "message": str(exc),
                "canonical_code": "pytorch_worker_model_load_failed",
            },
        })
    except Exception as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "model_load_failed",
                "message": str(exc),
                "canonical_code": "pytorch_worker_model_load_failed",
            },
        })


def _validate_transformers_loader(loader):
    if loader not in {CAUSAL_LM_LOADER, AUTOMATIC_SPEECH_RECOGNITION_LOADER}:
        raise ValueError(f"Unsupported PyTorch worker Transformers loader: {loader}")


def _load_transformers_model_from_kwargs(loader, kwargs):
    if loader == CAUSAL_LM_LOADER:
        return load_model(**kwargs)

    if loader == AUTOMATIC_SPEECH_RECOGNITION_LOADER:
        kwargs.pop("model_type", None)
        info = load_asr_model_with_policy(**kwargs)
        return {
            "model_path": info.get("model_path"),
            "model_type": "audio_transcription",
            "device": info.get("device"),
        }

    _validate_transformers_loader(loader)


def generate_text_from_envelope(envelope):
    """Generate text from the Rust worker envelope contract."""
    request_id = "unknown"
    try:
        decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
        if isinstance(decoded, dict):
            request_id = str(decoded.get("request_id") or request_id)
        kwargs = generate_text_kwargs_from_envelope(decoded)
        text = generate(**kwargs)
        return json.dumps({
            "status": "ok",
            "request_id": request_id,
            "result": {"text": text},
        })
    except ValueError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "invalid_request",
                "message": str(exc),
                "canonical_code": "pytorch_worker_invalid_generate_text_request",
            },
        })
    except RuntimeError as exc:
        message = str(exc)
        kind = "runtime_unavailable" if "No model loaded" in message else "generation_failed"
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": kind,
                "message": message,
                "canonical_code": "pytorch_worker_generate_text_failed",
            },
        })
    except Exception as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "internal",
                "message": str(exc),
                "canonical_code": "pytorch_worker_generate_text_internal",
            },
        })


def unload_model_from_envelope(envelope):
    """Unload the active model from the Rust worker envelope contract."""
    request_id = "unknown"
    try:
        decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
        if isinstance(decoded, dict):
            request_id = str(decoded.get("request_id") or request_id)
        unload_model_kwargs_from_envelope(decoded)
        unload_model()
        return json.dumps({
            "status": "ok",
            "request_id": request_id,
            "result": {"unloaded": True},
        })
    except ValueError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "invalid_request",
                "message": str(exc),
                "canonical_code": "pytorch_worker_invalid_unload_request",
            },
        })
    except RuntimeError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "generation_failed",
                "message": str(exc),
                "canonical_code": "pytorch_worker_unload_failed",
            },
        })
    except Exception as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "internal",
                "message": str(exc),
                "canonical_code": "pytorch_worker_unload_internal",
            },
        })


def get_loaded_info_from_envelope(envelope):
    """Return loaded model info from the Rust worker envelope contract."""
    request_id = "unknown"
    try:
        decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
        if isinstance(decoded, dict):
            request_id = str(decoded.get("request_id") or request_id)
        get_loaded_info_kwargs_from_envelope(decoded)
        info = get_loaded_info()
        if info is None:
            raise RuntimeError("No model loaded. Call load_model() first.")
        return json.dumps({
            "status": "ok",
            "request_id": request_id,
            "result": info,
        })
    except ValueError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "invalid_request",
                "message": str(exc),
                "canonical_code": "pytorch_worker_invalid_get_loaded_info_request",
            },
        })
    except RuntimeError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "runtime_unavailable",
                "message": str(exc),
                "canonical_code": "pytorch_worker_kv_loaded_info_failed",
            },
        })
    except Exception as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "internal",
                "message": str(exc),
                "canonical_code": "pytorch_worker_kv_loaded_info_internal",
            },
        })


def clear_live_kv_cache_from_envelope(envelope):
    """Clear live KV state from the Rust worker envelope contract."""
    request_id = "unknown"
    try:
        decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
        if isinstance(decoded, dict):
            request_id = str(decoded.get("request_id") or request_id)
        clear_kv_cache_kwargs_from_envelope(decoded)
        result = clear_live_kv_cache()
        cleared = bool(result.get("cleared")) if isinstance(result, dict) else False
        return json.dumps({
            "status": "ok",
            "request_id": request_id,
            "result": {"cleared": cleared},
        })
    except ValueError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "invalid_request",
                "message": str(exc),
                "canonical_code": "pytorch_worker_invalid_clear_kv_cache_request",
            },
        })
    except RuntimeError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "generation_failed",
                "message": str(exc),
                "canonical_code": "pytorch_worker_kv_clear_failed",
            },
        })
    except Exception as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "internal",
                "message": str(exc),
                "canonical_code": "pytorch_worker_kv_clear_internal",
            },
        })


def save_live_kv_cache_from_envelope(envelope):
    """Persist live KV state from the Rust worker envelope contract."""
    request_id = "unknown"
    try:
        decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
        if isinstance(decoded, dict):
            request_id = str(decoded.get("request_id") or request_id)
        kwargs = save_kv_cache_kwargs_from_envelope(decoded)
        result = save_live_kv_cache(**kwargs)
        return json.dumps({
            "status": "ok",
            "request_id": request_id,
            "result": result,
        })
    except ValueError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "invalid_request",
                "message": str(exc),
                "canonical_code": "pytorch_worker_invalid_save_kv_cache_request",
            },
        })
    except RuntimeError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "generation_failed",
                "message": str(exc),
                "canonical_code": "pytorch_worker_kv_save_failed",
            },
        })
    except Exception as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "internal",
                "message": str(exc),
                "canonical_code": "pytorch_worker_kv_save_internal",
            },
        })


def restore_live_kv_cache_from_envelope(envelope):
    """Restore live KV state from the Rust worker envelope contract."""
    request_id = "unknown"
    try:
        decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
        if isinstance(decoded, dict):
            request_id = str(decoded.get("request_id") or request_id)
        kwargs = restore_kv_cache_kwargs_from_envelope(decoded)
        result = restore_live_kv_cache(**kwargs)
        return json.dumps({
            "status": "ok",
            "request_id": request_id,
            "result": result,
        })
    except ValueError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "invalid_request",
                "message": str(exc),
                "canonical_code": "pytorch_worker_invalid_restore_kv_cache_request",
            },
        })
    except RuntimeError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "generation_failed",
                "message": str(exc),
                "canonical_code": "pytorch_worker_kv_restore_failed",
            },
        })
    except Exception as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "internal",
                "message": str(exc),
                "canonical_code": "pytorch_worker_kv_restore_internal",
            },
        })


def truncate_kv_cache_file_from_envelope(envelope):
    """Truncate persisted KV state from the Rust worker envelope contract."""
    request_id = "unknown"
    try:
        decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
        if isinstance(decoded, dict):
            request_id = str(decoded.get("request_id") or request_id)
        kwargs = truncate_kv_cache_kwargs_from_envelope(decoded)
        result = truncate_kv_cache_file(**kwargs)
        return json.dumps({
            "status": "ok",
            "request_id": request_id,
            "result": result,
        })
    except ValueError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "invalid_request",
                "message": str(exc),
                "canonical_code": "pytorch_worker_invalid_truncate_kv_cache_request",
            },
        })
    except RuntimeError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "generation_failed",
                "message": str(exc),
                "canonical_code": "pytorch_worker_kv_truncate_failed",
            },
        })
    except Exception as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "internal",
                "message": str(exc),
                "canonical_code": "pytorch_worker_kv_truncate_internal",
            },
        })


def generate_text_stream_from_envelope(envelope):
    """Stream text generation from the Rust worker envelope contract."""
    kwargs = generate_text_kwargs_from_envelope(
        envelope,
        expected_operation=GENERATE_TEXT_STREAM_OPERATION,
    )
    return generate_tokens(**kwargs)


def generate_text_stream_setup_from_envelope(envelope):
    """Validate streaming generation setup and return a worker response JSON."""
    request_id = "unknown"
    try:
        decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
        if isinstance(decoded, dict):
            request_id = str(decoded.get("request_id") or request_id)
        generate_text_kwargs_from_envelope(
            decoded,
            expected_operation=GENERATE_TEXT_STREAM_OPERATION,
        )
        if _model is None:
            raise RuntimeError("No model loaded. Call load_model() first.")
        return json.dumps({
            "status": "ok",
            "request_id": request_id,
            "result": {"ready": True},
        })
    except ValueError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "invalid_request",
                "message": str(exc),
                "canonical_code": "pytorch_worker_invalid_generate_text_stream_request",
            },
        })
    except RuntimeError as exc:
        message = str(exc)
        kind = "runtime_unavailable" if "No model loaded" in message else "generation_failed"
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": kind,
                "message": message,
                "canonical_code": "pytorch_worker_generate_text_stream_failed",
            },
        })
    except Exception as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "internal",
                "message": str(exc),
                "canonical_code": "pytorch_worker_generate_text_stream_internal",
            },
        })


def transcribe_audio_from_envelope(envelope):
    """Transcribe audio from the Rust worker envelope contract."""
    request_id = "unknown"
    try:
        decoded = json.loads(envelope) if isinstance(envelope, str) else envelope
        if isinstance(decoded, dict):
            request_id = str(decoded.get("request_id") or request_id)
        kwargs = transcribe_audio_kwargs_from_envelope(decoded)
        result = transcribe_audio(**kwargs)
        return json.dumps({
            "status": "ok",
            "request_id": request_id,
            "result": result,
        })
    except ValueError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "invalid_request",
                "message": str(exc),
                "canonical_code": "pytorch_worker_invalid_audio_transcription_request",
            },
        })
    except RuntimeError as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "generation_failed",
                "message": str(exc),
                "canonical_code": "pytorch_worker_audio_transcription_failed",
            },
        })
    except Exception as exc:
        return json.dumps({
            "status": "error",
            "request_id": request_id,
            "error": {
                "kind": "internal",
                "message": str(exc),
                "canonical_code": "pytorch_worker_audio_transcription_internal",
            },
        })


def load_model(
    model_path,
    device="auto",
    model_type=None,
    trust_remote_code=False,
    trust_policy_decision_id=None,
    local_files_only=True,
    revision=None,
    code_revision=None,
    cache_policy="backend_default",
):
    """Load a model + tokenizer into module globals.

    Args:
        model_path: Filesystem path to model directory.
        device: Device string — "auto", "cpu", "cuda", "cuda:0", "mps", etc.
        model_type: Optional hint — "dllm", "sherry", or "text-generation".
                    If None, auto-detected from config.json.
        trust_remote_code: Explicit custom-code policy decision. Defaults closed.
        trust_policy_decision_id: Optional Rust-side policy record id for logs.
        local_files_only: Whether Transformers must avoid registry/network access.
        revision: Optional model revision passed to Transformers.
        code_revision: Optional remote-code revision passed to Transformers.
        cache_policy: Rust-owned cache policy label for logs and future mapping.

    Returns:
        Dict with model_path, model_type, device.
    """
    from transformers import AutoModelForCausalLM, AutoTokenizer

    apply_compatibility_shims()

    global _model, _tokenizer, _device, _model_path, _model_type

    # Unload previous model first
    if _model is not None:
        unload_model()

    raw_path = Path(model_path)
    if not raw_path.exists():
        raise FileNotFoundError(f"Model path does not exist: {raw_path}")
    path = _resolve_model_directory(raw_path)

    resolved_device = _resolve_device(device)
    detected_type = model_type or _detect_model_type(path)
    trust_remote_code = bool(trust_remote_code)
    local_files_only = bool(local_files_only)
    force_download = cache_policy == "bypass_cache" and not local_files_only

    if _transformers_package_requires_remote_code(path) and not trust_remote_code:
        raise RuntimeError(
            "Model package requires custom Transformers code but trust policy is closed."
        )

    logger.info(
        "Loading %s model from %s onto %s (trust_remote_code=%s, local_files_only=%s, revision=%s, code_revision=%s, cache_policy=%s, trust_policy_decision_id=%s)",
        detected_type,
        model_path,
        resolved_device,
        trust_remote_code,
        local_files_only,
        revision,
        code_revision,
        cache_policy,
        trust_policy_decision_id,
    )

    tokenizer = AutoTokenizer.from_pretrained(
        str(path),
        trust_remote_code=trust_remote_code,
        local_files_only=local_files_only,
        revision=revision,
        code_revision=code_revision,
        force_download=force_download,
    )
    # Some local model exports ship chat_template.jinja without wiring it into
    # tokenizer_config.json. Load it explicitly so apply_chat_template works.
    if not getattr(tokenizer, "chat_template", None):
        chat_template_path = path / "chat_template.jinja"
        if chat_template_path.exists():
            try:
                tokenizer.chat_template = chat_template_path.read_text(encoding="utf-8")
                logger.info("Loaded chat template from %s", chat_template_path)
            except OSError as e:
                logger.warning("Failed to read chat template %s: %s", chat_template_path, e)

    model = AutoModelForCausalLM.from_pretrained(
        str(path),
        torch_dtype="auto",
        device_map=str(resolved_device),
        trust_remote_code=trust_remote_code,
        local_files_only=local_files_only,
        revision=revision,
        code_revision=code_revision,
        force_download=force_download,
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
    global _model, _tokenizer, _device, _model_path, _model_type, _live_kv_state

    if _model is not None:
        name = _model_path.name if _model_path else "unknown"
        del _model
        del _tokenizer
        _model = None
        _tokenizer = None
        _device = None
        _model_path = None
        _model_type = None
        _live_kv_state = None

        try:
            if torch.cuda.is_available():
                torch.cuda.empty_cache()
        except Exception:
            pass

        logger.info("Model unloaded: %s", name)


def clear_live_kv_cache():
    """Drop any live KV snapshot held by the worker."""
    global _live_kv_state
    _live_kv_state = None
    return {"cleared": True}


def _require_live_kv_state():
    if _live_kv_state is None:
        raise RuntimeError("No live KV cache captured. Run a KV-capable generation first.")
    return _live_kv_state


def _truncate_kv_payload(payload, token_position):
    token_ids = payload.get("token_ids")
    cache = payload.get("cache")
    if not isinstance(token_ids, list):
        raise RuntimeError("KV payload is missing token_ids")
    if cache is None:
        raise RuntimeError("KV payload is missing cache data")
    if token_position < 0:
        raise RuntimeError("token_position must be non-negative")
    if token_position > len(token_ids):
        raise RuntimeError(
            f"token_position {token_position} exceeds cache token_count {len(token_ids)}"
        )
    if not hasattr(cache, "crop"):
        raise RuntimeError(
            "transformers DynamicCache.crop is unavailable; KV truncation is not supported"
        )
    cache.crop(token_position)
    payload["token_ids"] = token_ids[:token_position]
    return payload


def save_live_kv_cache(path):
    """Persist the current live KV snapshot to disk."""
    payload = _require_live_kv_state()
    torch.save(payload, path)
    return get_live_kv_info()


def restore_live_kv_cache(path):
    """Restore a live KV snapshot from disk."""
    global _live_kv_state
    payload = torch.load(path, map_location="cpu")
    if not isinstance(payload, dict):
        raise RuntimeError("KV payload must deserialize to a dict")
    _live_kv_state = payload
    return get_live_kv_info()


def truncate_kv_cache_file(path, token_position):
    """Truncate a persisted KV snapshot in place."""
    payload = torch.load(path, map_location="cpu")
    if not isinstance(payload, dict):
        raise RuntimeError("KV payload must deserialize to a dict")
    truncated = _truncate_kv_payload(payload, int(token_position))
    torch.save(truncated, path)
    return {
        "token_count": int(len(truncated.get("token_ids", []))),
    }


def unload_diffusion_model():
    """Unload the current diffusion pipeline and free GPU memory."""
    global _diffusion_pipeline, _diffusion_device, _diffusion_model_path, _diffusion_dtype

    if _diffusion_pipeline is not None:
        name = _diffusion_model_path.name if _diffusion_model_path else "unknown"
        del _diffusion_pipeline
        _diffusion_pipeline = None
        _diffusion_device = None
        _diffusion_model_path = None
        _diffusion_dtype = None

        try:
            if torch.cuda.is_available():
                torch.cuda.empty_cache()
        except Exception:
            pass

        logger.info("Diffusion pipeline unloaded: %s", name)


def unload_asr_model():
    """Unload the current ASR pipeline and free GPU memory."""
    global _asr_pipeline, _asr_device, _asr_model_path

    if _asr_pipeline is not None:
        name = _asr_model_path.name if _asr_model_path else "unknown"
        del _asr_pipeline
        _asr_pipeline = None
        _asr_device = None
        _asr_model_path = None

        try:
            if torch.cuda.is_available():
                torch.cuda.empty_cache()
        except Exception:
            pass

        logger.info("ASR pipeline unloaded: %s", name)


def load_diffusion_model(
    model_path,
    device="auto",
    torch_dtype=None,
    enable_attention_slicing=False,
    enable_vae_slicing=False,
    enable_vae_tiling=False,
    model_cpu_offload=False,
    sequential_cpu_offload=False,
):
    """Load a diffusion pipeline into module globals for process-backed use."""
    global _diffusion_pipeline, _diffusion_device, _diffusion_model_path, _diffusion_dtype

    path = Path(model_path)
    if not path.exists():
        raise FileNotFoundError(f"Model path does not exist: {model_path}")

    if _diffusion_pipeline is not None and _diffusion_model_path == path:
        return get_loaded_diffusion_info()

    if _diffusion_pipeline is not None:
        unload_diffusion_model()

    try:
        from diffusers import DiffusionPipeline
    except Exception as exc:
        raise RuntimeError(
            "Failed to import diffusers runtime. Ensure the selected dependency environment "
            "includes `diffusers`, `transformers`, `accelerate`, `torch`, and Pillow."
        ) from exc

    resolved_device = _resolve_device(device)
    resolved_dtype = _resolve_torch_dtype(resolved_device, torch_dtype)

    logger.info(
        "Loading diffusion pipeline from %s onto %s (dtype=%s)",
        model_path,
        resolved_device,
        _dtype_name(resolved_dtype),
    )

    load_overrides = _detect_diffusion_load_overrides(path)
    pipeline = DiffusionPipeline.from_pretrained(
        str(path),
        torch_dtype=resolved_dtype,
        trust_remote_code=True,
        **load_overrides,
    )
    pipeline.set_progress_bar_config(disable=True)

    if enable_attention_slicing and hasattr(pipeline, "enable_attention_slicing"):
        pipeline.enable_attention_slicing()
    if enable_vae_slicing and hasattr(pipeline, "enable_vae_slicing"):
        pipeline.enable_vae_slicing()
    if enable_vae_tiling and hasattr(pipeline, "enable_vae_tiling"):
        pipeline.enable_vae_tiling()

    offload_active = bool(model_cpu_offload or sequential_cpu_offload)
    if offload_active:
        if not torch.cuda.is_available():
            raise RuntimeError("CPU offload options require CUDA to be available")
        if sequential_cpu_offload and hasattr(pipeline, "enable_sequential_cpu_offload"):
            pipeline.enable_sequential_cpu_offload()
        elif hasattr(pipeline, "enable_model_cpu_offload"):
            pipeline.enable_model_cpu_offload()
        else:
            raise RuntimeError("Selected diffusion pipeline does not support CPU offload")
        runtime_device = "cuda-offload"
    else:
        pipeline.to(resolved_device)
        runtime_device = str(resolved_device)

    _diffusion_pipeline = pipeline
    _diffusion_device = runtime_device
    _diffusion_model_path = path
    _diffusion_dtype = resolved_dtype

    return get_loaded_diffusion_info()


def _call_accepts_kwarg(callable_obj, name):
    try:
        signature = inspect.signature(callable_obj)
    except (TypeError, ValueError):
        return False
    return any(
        param.kind == inspect.Parameter.VAR_KEYWORD or param_name == name
        for param_name, param in signature.parameters.items()
    )


def _resize_preview_image(image, max_dimension):
    width = getattr(image, "width", None)
    height = getattr(image, "height", None)
    if not width or not height or max(width, height) <= max_dimension:
        return image
    resized = image.copy()
    resized.thumbnail((max_dimension, max_dimension))
    return resized


def _extract_callback_preview_image(callback_kwargs=None, latents=None):
    callback_kwargs = callback_kwargs if isinstance(callback_kwargs, dict) else {}

    for key in ("image", "images"):
        candidate = callback_kwargs.get(key)
        if isinstance(candidate, list) and candidate:
            candidate = candidate[0]
        if hasattr(candidate, "save") and hasattr(candidate, "width") and hasattr(candidate, "height"):
            return _resize_preview_image(candidate, _DIFFUSION_PREVIEW_MAX_DIMENSION)

    if latents is None:
        latents = callback_kwargs.get("latents")
    if latents is None or not torch.is_tensor(latents):
        return None

    vae = getattr(_diffusion_pipeline, "vae", None)
    image_processor = getattr(_diffusion_pipeline, "image_processor", None)
    if vae is None or image_processor is None or not hasattr(vae, "decode"):
        return None

    with torch.no_grad():
        preview_latents = latents[:1].detach()
        vae_device = getattr(vae, "device", preview_latents.device)
        vae_dtype = getattr(vae, "dtype", preview_latents.dtype)
        preview_latents = preview_latents.to(device=vae_device, dtype=vae_dtype)

        vae_config = getattr(vae, "config", None)
        scaling_factor = float(getattr(vae_config, "scaling_factor", 0.18215))
        preview_latents = preview_latents / scaling_factor

        decoded = vae.decode(preview_latents)
        decoded_sample = getattr(decoded, "sample", decoded)
        images = image_processor.postprocess(decoded_sample, output_type="pil")
        if not images:
            return None
        return _resize_preview_image(images[0], _DIFFUSION_PREVIEW_MAX_DIMENSION)


def _attach_diffusion_preview_callback(call_kwargs, total_steps, emit_stream):
    if not callable(emit_stream):
        return

    pipeline_call = getattr(_diffusion_pipeline, "__call__", None)
    if pipeline_call is None:
        return

    max_events = max(1, _DIFFUSION_PREVIEW_MAX_EVENTS)
    interval = max(1, math.ceil(max(1, int(total_steps)) / max_events))
    state = {"sequence": 0, "emitted": 0}

    def normalized_step(step):
        try:
            return int(step)
        except Exception:
            return state["emitted"]

    def should_emit(step):
        if state["emitted"] >= max_events:
            return False
        step_number = normalized_step(step)
        return (
            step_number == 0
            or (step_number + 1) % interval == 0
            or (step_number + 1) >= total_steps
        )

    def emit_preview(step, image):
        encoded = _encode_image(image)
        payload = {
            "type": "diffusion_preview",
            "preview_role": "revision",
            "artifact_role": "diffusion_preview",
            "image_base64": encoded["data_base64"],
            "media_type": encoded["mime_type"],
            "sequence": state["sequence"],
            "revision_index": state["sequence"],
            "step": normalized_step(step),
            "total_steps": int(total_steps),
            "is_final": False,
        }
        if encoded.get("width") is not None:
            payload["width"] = encoded["width"]
        if encoded.get("height") is not None:
            payload["height"] = encoded["height"]
        emit_stream(payload)
        state["sequence"] += 1
        state["emitted"] += 1

    def handle_step(step, callback_kwargs=None, latents=None):
        if not should_emit(step):
            return
        try:
            image = _extract_callback_preview_image(callback_kwargs, latents=latents)
            if image is not None:
                emit_preview(step, image)
        except Exception:
            # Streaming previews are opportunistic and must not affect final generation.
            pass

    if _call_accepts_kwarg(pipeline_call, "callback_on_step_end"):
        def callback_on_step_end(_pipeline, step, _timestep, callback_kwargs):
            handle_step(step, callback_kwargs=callback_kwargs)
            return callback_kwargs

        call_kwargs["callback_on_step_end"] = callback_on_step_end
        if _call_accepts_kwarg(pipeline_call, "callback_on_step_end_tensor_inputs"):
            tensor_inputs = getattr(_diffusion_pipeline, "_callback_tensor_inputs", None)
            if isinstance(tensor_inputs, (list, tuple, set)) and "latents" in tensor_inputs:
                call_kwargs["callback_on_step_end_tensor_inputs"] = ["latents"]
        return

    if _call_accepts_kwarg(pipeline_call, "callback"):
        def callback(step, _timestep, latents):
            handle_step(step, latents=latents)

        call_kwargs["callback"] = callback
        if _call_accepts_kwarg(pipeline_call, "callback_steps"):
            call_kwargs["callback_steps"] = interval


def load_asr_model(model_path, device="auto", chunk_length_s=None):
    """Load a speech-to-text pipeline into module globals for process-backed use."""
    return load_asr_model_with_policy(
        model_path,
        device=device,
        chunk_length_s=chunk_length_s,
    )


def load_asr_model_with_policy(
    model_path,
    device="auto",
    chunk_length_s=None,
    trust_remote_code=False,
    trust_policy_decision_id=None,
    local_files_only=True,
    revision=None,
    code_revision=None,
    cache_policy="backend_default",
):
    """Load a speech-to-text pipeline with the Rust-owned Transformers policy."""
    global _asr_pipeline, _asr_device, _asr_model_path

    path = _resolve_model_directory(model_path)
    if not path.exists():
        raise FileNotFoundError(f"Model path does not exist: {path}")

    if _asr_pipeline is not None and _asr_model_path == path:
        return get_loaded_asr_info()

    if _asr_pipeline is not None:
        unload_asr_model()

    try:
        from transformers import AutoModelForSpeechSeq2Seq, AutoProcessor, pipeline
    except Exception as exc:
        raise RuntimeError(
            "Failed to import transformers ASR runtime. Ensure the selected dependency environment "
            "includes `transformers`, `torch`, `numpy`, and `soundfile`."
        ) from exc

    resolved_device = _resolve_device(device)
    torch_dtype = torch.float16 if resolved_device.type == "cuda" else torch.float32
    trust_remote_code = bool(trust_remote_code)
    local_files_only = bool(local_files_only)
    force_download = cache_policy == "bypass_cache" and not local_files_only

    if _transformers_package_requires_remote_code(path) and not trust_remote_code:
        raise RuntimeError(
            "Model package requires custom Transformers code but trust policy is closed."
        )

    logger.info(
        "Loading ASR pipeline from %s onto %s (trust_remote_code=%s, local_files_only=%s, revision=%s, code_revision=%s, cache_policy=%s, trust_policy_decision_id=%s)",
        path,
        resolved_device,
        trust_remote_code,
        local_files_only,
        revision,
        code_revision,
        cache_policy,
        trust_policy_decision_id,
    )

    model = AutoModelForSpeechSeq2Seq.from_pretrained(
        str(path),
        torch_dtype=torch_dtype,
        low_cpu_mem_usage=True,
        use_safetensors=True,
        trust_remote_code=trust_remote_code,
        local_files_only=local_files_only,
        revision=revision,
        code_revision=code_revision,
        force_download=force_download,
    )
    processor = AutoProcessor.from_pretrained(
        str(path),
        trust_remote_code=trust_remote_code,
        local_files_only=local_files_only,
        revision=revision,
        code_revision=code_revision,
        force_download=force_download,
    )

    if resolved_device.type != "cpu":
        model.to(resolved_device)

    pipeline_device = 0 if resolved_device.type == "cuda" else -1
    pipe_kwargs = {
        "task": "automatic-speech-recognition",
        "model": model,
        "tokenizer": processor.tokenizer,
        "feature_extractor": processor.feature_extractor,
        "device": pipeline_device,
    }
    if chunk_length_s is not None:
        pipe_kwargs["chunk_length_s"] = float(chunk_length_s)

    _asr_pipeline = pipeline(**pipe_kwargs)
    _asr_device = resolved_device
    _asr_model_path = path
    return get_loaded_asr_info()


def transcribe_audio(
    model_path,
    audio_base64,
    device="auto",
    language=None,
    prompt=None,
    task=None,
    chunk_length_s=None,
):
    """Transcribe an in-memory WAV payload with the loaded ASR pipeline."""
    if not isinstance(audio_base64, str) or not audio_base64.strip():
        raise RuntimeError("Missing audio_base64 input")

    load_asr_model(model_path, device=device, chunk_length_s=chunk_length_s)
    if _asr_pipeline is None:
        raise RuntimeError("No ASR pipeline loaded. Call load_asr_model() first.")

    try:
        raw_bytes = base64.b64decode(audio_base64)
    except Exception as exc:
        raise RuntimeError("Failed to decode base64 audio payload") from exc

    try:
        audio, sample_rate = sf.read(io.BytesIO(raw_bytes), dtype="float32")
    except Exception as exc:
        raise RuntimeError("Failed to decode audio payload as WAV") from exc

    audio = np.asarray(audio, dtype=np.float32)
    if audio.ndim > 1:
        audio = np.mean(audio, axis=1, dtype=np.float32)

    duration_seconds = float(audio.shape[0]) / float(sample_rate) if sample_rate else 0.0

    generate_kwargs = {}
    if isinstance(language, str) and language.strip():
        generate_kwargs["language"] = language.strip()
    if isinstance(prompt, str) and prompt.strip():
        generate_kwargs["prompt"] = prompt.strip()
    if isinstance(task, str) and task.strip():
        generate_kwargs["task"] = task.strip()

    result = _asr_pipeline(
        {"array": audio, "sampling_rate": int(sample_rate)},
        generate_kwargs=generate_kwargs or None,
    )

    if isinstance(result, dict):
        text = result.get("text", "")
        chunks = result.get("chunks")
    else:
        text = str(result)
        chunks = None

    return {
        "text": text.strip(),
        "language": language.strip() if isinstance(language, str) and language.strip() else None,
        "duration_seconds": duration_seconds,
        "chunks": chunks,
    }


def generate_image(
    prompt,
    negative_prompt=None,
    width=None,
    height=None,
    num_inference_steps=30,
    guidance_scale=None,
    seed=None,
    num_images_per_prompt=1,
    scheduler=None,
    init_image=None,
    mask_image=None,
    strength=None,
    emit_stream=None,
    **kwargs,
):
    """Generate one or more images from the loaded diffusion pipeline."""
    del scheduler  # Reserved for later scheduler swapping support.

    if _diffusion_pipeline is None:
        raise RuntimeError("No diffusion pipeline loaded. Call load_diffusion_model() first.")

    resolved_steps = 30 if num_inference_steps is None else int(num_inference_steps)
    resolved_num_images = 1 if num_images_per_prompt is None else int(num_images_per_prompt)
    call_kwargs = {
        "prompt": prompt,
        "num_inference_steps": resolved_steps,
        "num_images_per_prompt": resolved_num_images,
    }
    if isinstance(negative_prompt, str) and negative_prompt.strip():
        call_kwargs["negative_prompt"] = negative_prompt.strip()
    if width is not None:
        call_kwargs["width"] = int(width)
    if height is not None:
        call_kwargs["height"] = int(height)
    if guidance_scale is not None:
        call_kwargs["guidance_scale"] = float(guidance_scale)
    if strength is not None:
        call_kwargs["strength"] = float(strength)
    if seed is not None:
        call_kwargs["generator"] = torch.Generator(device="cpu").manual_seed(int(seed))
    if init_image is not None:
        call_kwargs["image"] = _decode_base64_image(init_image)
    if mask_image is not None:
        call_kwargs["mask_image"] = _decode_base64_image(mask_image)

    for key, value in kwargs.items():
        if value is not None:
            call_kwargs[key] = value

    _attach_diffusion_preview_callback(call_kwargs, resolved_steps, emit_stream)

    result = _diffusion_pipeline(**call_kwargs)
    images = getattr(result, "images", None)
    if not images:
        raise RuntimeError("Diffusion pipeline returned no images")

    encoded_images = [_encode_image(image) for image in images]
    primary = encoded_images[0]
    return {
        "image_base64": primary["data_base64"],
        "mime_type": primary["mime_type"],
        "width": primary.get("width"),
        "height": primary.get("height"),
        "images": encoded_images,
        "seed_used": int(seed) if seed is not None else None,
    }


def generate(prompt, system_prompt=None, max_tokens=512, temperature=0.7, top_p=1.0,
             masked_prompt_json=None, denoising_steps=None, block_length=None,
             **kwargs):
    """Generate a complete response (non-streaming).

    Routes to block diffusion for dLLM models, standard generate otherwise.
    When masked_prompt_json is provided and the model is dLLM, uses masked
    generation that preserves anchored segments.
    """
    if _model is None:
        raise RuntimeError("No model loaded. Call load_model() first.")

    # Masked prompt routing for dLLM models
    if masked_prompt_json is not None and _model_type == "dllm":
        clear_live_kv_cache()
        mp = json.loads(masked_prompt_json)
        segments = mp.get("segments", [])
        return _generate_dllm_masked(
            _model, _tokenizer, _device, segments,
            max_tokens=max_tokens, temperature=temperature, top_p=top_p,
            denoising_steps=denoising_steps, block_length=block_length,
        )

    formatted = _format_prompt(prompt, system_prompt)
    top_k = kwargs.get("top_k")

    if _model_type == "dllm":
        # For TraDo/SDAR instruct models in Pantograph, the model's native
        # autoregressive generation path is significantly more stable than the
        # experimental custom block-diffusion decode path.
        return _generate_dllm_autoregressive_safe(
            formatted, max_tokens, temperature, top_p, top_k=top_k,
        )
    clear_live_kv_cache()
    return _generate_autoregressive(
        _model, _tokenizer, _device, formatted, max_tokens, temperature, top_p, top_k=top_k,
    )


def generate_tokens(prompt, system_prompt=None, max_tokens=512, temperature=0.7, top_p=1.0,
                     masked_prompt_json=None, denoising_steps=None, block_length=None,
                     **kwargs):
    """Generate tokens as a Python generator (for streaming).

    dLLM models generate block-by-block; each decoded block is yielded as a
    chunk. Autoregressive models yield one token at a time.
    When masked_prompt_json is provided and the model is dLLM, uses masked
    streaming generation that preserves anchored segments.
    """
    if _model is None:
        raise RuntimeError("No model loaded. Call load_model() first.")

    # Masked prompt streaming routing for dLLM models
    if masked_prompt_json is not None and _model_type == "dllm":
        clear_live_kv_cache()
        mp = json.loads(masked_prompt_json)
        segments = mp.get("segments", [])
        yield from _generate_dllm_masked_streaming(
            _model, _tokenizer, _device, segments,
            max_tokens=max_tokens, temperature=temperature, top_p=top_p,
            denoising_steps=denoising_steps, block_length=block_length,
        )
        return

    formatted = _format_prompt(prompt, system_prompt)
    top_k = kwargs.get("top_k")

    if _model_type == "dllm":
        # Stream a single final replacement for stability on TraDo/SDAR.
        final_text = _generate_dllm_autoregressive_safe(
            formatted, max_tokens, temperature, top_p, top_k=top_k,
        )
        yield {"mode": "replace", "text": final_text}
    else:
        clear_live_kv_cache()
        yield from _generate_autoregressive_streaming(
            _model, _tokenizer, _device, formatted, max_tokens, temperature, top_p, top_k=top_k,
        )


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

    # Fallback for Qwen/TraDo-style chat models that use ChatML tokens.
    try:
        special_tokens = set(getattr(_tokenizer, "additional_special_tokens", []) or [])
    except Exception:
        special_tokens = set()
    if "<|im_start|>" in special_tokens and "<|im_end|>" in special_tokens:
        parts = []
        if system_prompt:
            parts.append(f"<|im_start|>system\n{system_prompt}<|im_end|>")
        parts.append(f"<|im_start|>user\n{prompt}<|im_end|>")
        parts.append("<|im_start|>assistant\n")
        return "\n".join(parts)

    # Fallback: simple text format
    parts = []
    if system_prompt:
        parts.append(f"System: {system_prompt}")
    parts.append(f"User: {prompt}")
    parts.append("Assistant:")
    return "\n".join(parts)
