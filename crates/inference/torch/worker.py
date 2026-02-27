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
    _generate_dllm_masked,
    _generate_dllm_masked_streaming,
)
from autoregressive import (
    _generate_autoregressive,
    _generate_autoregressive_streaming,
    _generate_sdar_cached,
)

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("pantograph.pytorch")

# Module-level state — one model loaded at a time
_model = None
_tokenizer = None
_device = None
_model_path = None
_model_type = None


def _generate_dllm_autoregressive_safe(formatted_prompt, max_tokens, temperature, top_p, top_k=None):
    """Generate for TraDo/SDAR via native generate(), with empty-output retry.

    Some SDAR exports include chat delimiters in generation_config.eos_token_id,
    which can terminate immediately and decode to an empty string. Retry with a
    single EOS and a small min_new_tokens floor when that happens.
    """
    text = _generate_sdar_cached(
        _model, _tokenizer, _device, formatted_prompt, max_tokens, temperature, top_p, top_k=top_k,
    )
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

    # accelerate's dispatch_model calls model.to(device) which explodes when
    # any parameter is still on the meta device (common with trust_remote_code
    # models whose __init__ creates extra buffers or padded embeddings).
    # Patch it to materialise meta tensors via to_empty before moving.
    import transformers.modeling_utils as _mu
    if not getattr(_mu, "_pantograph_dispatch_patched", False):
        _orig_dispatch = _mu.dispatch_model

        def _safe_dispatch(model, device_map, **kwargs):
            has_meta = any(p.device.type == "meta" for p in model.parameters())
            if not has_meta:
                return _orig_dispatch(model, device_map, **kwargs)
            # Single-device map: materialise then move
            if isinstance(device_map, dict) and len(device_map) == 1 and "" in device_map:
                device = device_map[""]
            elif isinstance(device_map, str):
                device = device_map
            else:
                return _orig_dispatch(model, device_map, **kwargs)

            # Collect names of meta parameters that need loading
            meta_names = {n for n, p in model.named_parameters()
                          if p.device.type == "meta"}

            if meta_names and hasattr(model.config, "_name_or_path"):
                # Reload missing weights from safetensors with key remapping.
                # We assign tensors directly via setattr to bypass shape
                # validation (embed_tokens may differ between checkpoint and
                # config) and avoid resize_token_embeddings on meta tensors.
                import glob as _glob
                from safetensors.torch import load_file as _load_file
                model_dir = Path(model.config._name_or_path)
                if model_dir.is_dir():
                    # Expected shapes from meta params — truncate oversized
                    # checkpoint tensors (e.g. padded embeddings) to match.
                    expected_shapes = {n: p.shape for n, p in model.named_parameters()
                                       if n in meta_names}
                    loaded_count = 0
                    for shard in sorted(_glob.glob(str(model_dir / "*.safetensors"))):
                        sd = _load_file(shard, device=str(device))
                        for k, v in sd.items():
                            candidates = [k]
                            if k.startswith("language_model."):
                                candidates.append(k.replace("language_model.", "model.", 1))
                            for cand in candidates:
                                if cand in meta_names:
                                    exp = expected_shapes.get(cand)
                                    if exp is not None and v.shape != exp:
                                        slices = tuple(slice(0, s) for s in exp)
                                        v = v[slices].contiguous()
                                    parts = cand.split(".")
                                    mod = model
                                    for p in parts[:-1]:
                                        mod = getattr(mod, p)
                                    setattr(mod, parts[-1], torch.nn.Parameter(
                                        v, requires_grad=False,
                                    ))
                                    meta_names.discard(cand)
                                    loaded_count += 1
                                    break
                        del sd
                    logger.info("  Reloaded %d params from safetensors (%d still meta)",
                                loaded_count, len(meta_names))

            # Move any remaining real params to device; zero-fill any still-meta
            for name, param in list(model.named_parameters()):
                if param.device.type == "meta":
                    parts = name.split(".")
                    mod = model
                    for p in parts[:-1]:
                        mod = getattr(mod, p)
                    setattr(mod, parts[-1], torch.nn.Parameter(
                        torch.empty(param.shape, dtype=param.dtype, device=device),
                        requires_grad=param.requires_grad,
                    ))
                elif str(param.device) != str(torch.device(device)):
                    parts = name.split(".")
                    mod = model
                    for p in parts[:-1]:
                        mod = getattr(mod, p)
                    setattr(mod, parts[-1], torch.nn.Parameter(
                        param.data.to(device), requires_grad=param.requires_grad,
                    ))
            for name, buf in list(model.named_buffers()):
                if buf.device.type == "meta":
                    parts = name.split(".")
                    mod = model
                    for p in parts[:-1]:
                        mod = getattr(mod, p)
                    setattr(mod, parts[-1],
                            torch.empty(buf.shape, dtype=buf.dtype, device=device))
                elif str(buf.device) != str(torch.device(device)):
                    parts = name.split(".")
                    mod = model
                    for p in parts[:-1]:
                        mod = getattr(mod, p)
                    setattr(mod, parts[-1], buf.to(device))
            model.tie_weights()
            logger.info("Shimmed dispatch_model: materialised meta tensors onto %s", device)
            return model

        _mu.dispatch_model = _safe_dispatch
        _mu._pantograph_dispatch_patched = True
        logger.info("Shimmed dispatch_model for meta-tensor safety (transformers %s)", version)

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
        yield from _generate_autoregressive_streaming(
            _model, _tokenizer, _device, formatted, max_tokens, temperature, top_p, top_k=top_k,
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
