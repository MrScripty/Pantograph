"""PyTorch inference worker for Pantograph.

Embedded in the Rust process via PyO3. Provides model loading, generation,
and streaming token output for HuggingFace, dLLM, and Sherry models.

All public functions are called from Rust through PyO3's Python::with_gil.
Module-level globals hold the loaded model state.
"""

import json
import logging
from pathlib import Path

import torch

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
    import torch
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
            import torch
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

    if _model_type == "dllm":
        return _generate_dllm(prompt, system_prompt, max_tokens, temperature, top_p)
    return _generate_autoregressive(prompt, system_prompt, max_tokens, temperature, top_p)


def generate_tokens(prompt, system_prompt=None, max_tokens=512, temperature=0.7, top_p=1.0):
    """Generate tokens as a Python generator (for streaming).

    dLLM models generate block-by-block; each decoded block is yielded as a
    chunk. Autoregressive models yield one token at a time.
    """
    if _model is None:
        raise RuntimeError("No model loaded. Call load_model() first.")

    if _model_type == "dllm":
        yield from _generate_dllm_streaming(prompt, system_prompt, max_tokens, temperature, top_p)
    else:
        yield from _generate_autoregressive_streaming(prompt, system_prompt, max_tokens, temperature, top_p)


# --- Autoregressive generation (standard HuggingFace models) ---

def _generate_autoregressive(prompt, system_prompt, max_tokens, temperature, top_p):
    import torch

    text = _format_prompt(prompt, system_prompt)
    inputs = _tokenizer(text, return_tensors="pt").to(_device)

    with torch.no_grad():
        outputs = _model.generate(
            **inputs,
            max_new_tokens=max_tokens,
            temperature=max(temperature, 0.01),
            top_p=top_p,
            do_sample=temperature > 0,
        )

    input_len = inputs["input_ids"].shape[1]
    generated = outputs[0][input_len:]
    return _tokenizer.decode(generated, skip_special_tokens=True)


def _generate_autoregressive_streaming(prompt, system_prompt, max_tokens, temperature, top_p):
    import torch

    text = _format_prompt(prompt, system_prompt)
    inputs = _tokenizer(text, return_tensors="pt").to(_device)
    input_ids = inputs["input_ids"]

    for _ in range(max_tokens):
        with torch.no_grad():
            outputs = _model(input_ids)
            logits = outputs.logits[:, -1, :]

            if temperature > 0:
                logits = logits / max(temperature, 0.01)
                probs = torch.softmax(logits, dim=-1)
                next_token = torch.multinomial(probs, num_samples=1)
            else:
                next_token = logits.argmax(dim=-1, keepdim=True)

        if next_token.item() == _tokenizer.eos_token_id:
            break

        token_str = _tokenizer.decode(next_token[0], skip_special_tokens=True)
        yield token_str

        input_ids = torch.cat([input_ids, next_token], dim=-1)


# --- Block diffusion generation (dLLM / SDAR / TraDo models) ---

# Mask token ID used by SDAR-family models
_DLLM_MASK_ID = 151669

def _generate_dllm(prompt, system_prompt, max_tokens, temperature, top_p):
    """Generate using block diffusion (full response)."""
    import torch

    text = _format_prompt(prompt, system_prompt)
    tokens = _tokenizer(text, return_tensors="pt", padding=True, truncation=True).to(_device)
    prompt_length = tokens["input_ids"].shape[1]

    output_ids = _block_diffusion_generate(
        _model,
        prompt=tokens,
        mask_id=_DLLM_MASK_ID,
        gen_length=max_tokens,
        temperature=max(temperature, 0.01),
        top_p=top_p,
    )

    generated = output_ids[0][prompt_length:]
    return _tokenizer.decode(generated, skip_special_tokens=True)


def _generate_dllm_streaming(prompt, system_prompt, max_tokens, temperature, top_p):
    """Generate using block diffusion, yielding text after each block."""
    import torch

    text = _format_prompt(prompt, system_prompt)
    tokens = _tokenizer(text, return_tensors="pt", padding=True, truncation=True).to(_device)
    prompt_length = tokens["input_ids"].shape[1]

    # Use the streaming variant that yields per-block
    for block_ids in _block_diffusion_generate_blocks(
        _model,
        prompt=tokens,
        mask_id=_DLLM_MASK_ID,
        gen_length=max_tokens,
        temperature=max(temperature, 0.01),
        top_p=top_p,
    ):
        chunk = _tokenizer.decode(block_ids, skip_special_tokens=True)
        if chunk:
            yield chunk


@torch.no_grad()
def _block_diffusion_generate(
    model, prompt, mask_id,
    gen_length=128, block_length=8, denoising_steps=8,
    temperature=1.0, top_k=0, top_p=1.0,
    remasking_strategy="low_confidence_dynamic",
    confidence_threshold=0.85,
):
    """Block diffusion generation adapted from dLLM-RL/generate.py."""
    import torch
    from torch.nn import functional as F
    from transformers.cache_utils import DynamicCache

    model.eval()
    input_ids = prompt["input_ids"]
    prompt_length = input_ids.shape[1]
    past_key_values = DynamicCache()

    num_blocks = (prompt_length + gen_length + block_length - 1) // block_length
    total_length = num_blocks * block_length

    block_mask = torch.tril(torch.ones(num_blocks, num_blocks, device=model.device))
    attn_mask = (
        block_mask.repeat_interleave(block_length, dim=0)
        .repeat_interleave(block_length, dim=1)
        .unsqueeze(0)
    )
    position_ids = torch.arange(total_length, device=model.device).unsqueeze(0)

    x = torch.full((1, total_length), mask_id, dtype=torch.long, device=model.device)
    x[:, :prompt_length] = input_ids
    prefill_blocks = prompt_length // block_length
    prefill_length = prefill_blocks * block_length

    # Prefill stage
    if prefill_length > 0:
        cur_attn = attn_mask[:, :prefill_length, :prefill_length]
        if cur_attn.dim() == 3:
            cur_attn = cur_attn[:, None, :, :]
        model(
            x[:, :prefill_length],
            attention_mask=cur_attn,
            position_ids=position_ids[:, :prefill_length],
            past_key_values=past_key_values,
            use_cache=True,
            store_kv=True,
        )

    num_transfer = _get_num_transfer_tokens(block_length, denoising_steps)

    # Decode stage
    for nb in range(prefill_blocks, num_blocks):
        s, e = nb * block_length, (nb + 1) * block_length
        cur_x = x[:, s:e].clone()
        cur_attn = attn_mask[:, s:e, :e]
        if cur_attn.dim() == 3:
            cur_attn = cur_attn[:, None, :, :]
        cur_pos = position_ids[:, s:e]

        for step in range(denoising_steps + 1):
            mask_index = cur_x == mask_id
            if mask_index.sum() == 0:
                model(cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                      past_key_values=past_key_values, use_cache=True, store_kv=True)
                break

            logits = model(cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                           past_key_values=past_key_values, use_cache=True, store_kv=False).logits

            x0, x0_p = _sample_topk_topp(logits, temperature, top_k, top_p)
            transfer_index = _select_transfer(
                x0, x0_p, mask_index, num_transfer[step], remasking_strategy, confidence_threshold
            )
            cur_x[transfer_index] = x0[transfer_index]

        x[:, s:e] = cur_x

    return x


@torch.no_grad()
def _block_diffusion_generate_blocks(
    model, prompt, mask_id,
    gen_length=128, block_length=8, denoising_steps=8,
    temperature=1.0, top_k=0, top_p=1.0,
    remasking_strategy="low_confidence_dynamic",
    confidence_threshold=0.85,
):
    """Like _block_diffusion_generate but yields decoded block token IDs."""
    import torch
    from transformers.cache_utils import DynamicCache

    model.eval()
    input_ids = prompt["input_ids"]
    prompt_length = input_ids.shape[1]
    past_key_values = DynamicCache()

    num_blocks = (prompt_length + gen_length + block_length - 1) // block_length
    total_length = num_blocks * block_length

    block_mask = torch.tril(torch.ones(num_blocks, num_blocks, device=model.device))
    attn_mask = (
        block_mask.repeat_interleave(block_length, dim=0)
        .repeat_interleave(block_length, dim=1)
        .unsqueeze(0)
    )
    position_ids = torch.arange(total_length, device=model.device).unsqueeze(0)

    x = torch.full((1, total_length), mask_id, dtype=torch.long, device=model.device)
    x[:, :prompt_length] = input_ids
    prefill_blocks = prompt_length // block_length
    prefill_length = prefill_blocks * block_length

    if prefill_length > 0:
        cur_attn = attn_mask[:, :prefill_length, :prefill_length]
        if cur_attn.dim() == 3:
            cur_attn = cur_attn[:, None, :, :]
        model(
            x[:, :prefill_length],
            attention_mask=cur_attn,
            position_ids=position_ids[:, :prefill_length],
            past_key_values=past_key_values,
            use_cache=True,
            store_kv=True,
        )

    num_transfer = _get_num_transfer_tokens(block_length, denoising_steps)

    for nb in range(prefill_blocks, num_blocks):
        s, e = nb * block_length, (nb + 1) * block_length
        cur_x = x[:, s:e].clone()
        cur_attn = attn_mask[:, s:e, :e]
        if cur_attn.dim() == 3:
            cur_attn = cur_attn[:, None, :, :]
        cur_pos = position_ids[:, s:e]

        for step in range(denoising_steps + 1):
            mask_index = cur_x == mask_id
            if mask_index.sum() == 0:
                model(cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                      past_key_values=past_key_values, use_cache=True, store_kv=True)
                break

            logits = model(cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                           past_key_values=past_key_values, use_cache=True, store_kv=False).logits

            x0, x0_p = _sample_topk_topp(logits, temperature, top_k, top_p)
            transfer_index = _select_transfer(
                x0, x0_p, mask_index, num_transfer[step], remasking_strategy, confidence_threshold
            )
            cur_x[transfer_index] = x0[transfer_index]

        x[:, s:e] = cur_x
        # Yield only the non-mask tokens from this block
        block_tokens = cur_x[0][cur_x[0] != mask_id]
        if len(block_tokens) > 0:
            yield block_tokens


def _get_num_transfer_tokens(block_length, steps):
    import torch
    base = block_length // steps
    remainder = block_length % steps
    t = torch.zeros(steps, dtype=torch.int64) + base
    t[:remainder] += 1
    return t


def _sample_topk_topp(logits, temperature=1.0, top_k=0, top_p=1.0):
    import torch
    from torch.nn import functional as F

    orig_shape = logits.shape[:-1]
    vocab_size = logits.shape[-1]
    logits = logits.reshape(-1, vocab_size)

    if temperature != 1.0:
        logits = logits / temperature
    if top_k > 0:
        values, _ = torch.topk(logits, top_k)
        logits = torch.where(logits < values[..., -1, None], float("-inf"), logits)
    if top_p < 1.0:
        sorted_logits, sorted_indices = torch.sort(logits, descending=True)
        cum_probs = torch.cumsum(F.softmax(sorted_logits, dim=-1), dim=-1)
        mask = cum_probs > top_p
        mask[..., 1:] = mask[..., :-1].clone()
        mask[..., 0] = False
        scatter_mask = torch.scatter(torch.full_like(logits, False, dtype=torch.bool), -1, sorted_indices, mask)
        logits = logits.masked_fill(scatter_mask, float("-inf"))

    probs = F.softmax(logits, dim=-1)
    token = torch.multinomial(probs, num_samples=1)
    token_prob = torch.gather(probs, -1, token)
    return token.view(*orig_shape), token_prob.view(*orig_shape)


def _select_transfer(x0, x0_p, mask_index, num_to_transfer, strategy, confidence_threshold):
    import torch

    if strategy == "low_confidence_dynamic":
        confidence = torch.where(mask_index, x0_p, -torch.inf)
        transfer = torch.zeros_like(x0, dtype=torch.bool)
        for j in range(confidence.shape[0]):
            high_conf = confidence[j] > confidence_threshold
            if high_conf.sum() >= num_to_transfer:
                transfer[j] = high_conf
            else:
                _, idx = torch.topk(confidence[j], num_to_transfer)
                transfer[j, idx] = True
        return transfer
    elif strategy == "low_confidence_static":
        confidence = torch.where(mask_index, x0_p, -torch.inf)
        transfer = torch.zeros_like(x0, dtype=torch.bool)
        for j in range(confidence.shape[0]):
            _, idx = torch.topk(confidence[j], num_to_transfer)
            transfer[j, idx] = True
        return transfer
    else:  # sequential
        transfer = torch.zeros_like(x0, dtype=torch.bool)
        for j in range(x0.shape[0]):
            if mask_index[j].any():
                first = mask_index[j].nonzero(as_tuple=True)[0].min().item()
                transfer[j, first:first + num_to_transfer] = True
        return transfer


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
    import torch

    if device_str == "auto":
        if torch.cuda.is_available():
            return torch.device("cuda")
        elif hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
            return torch.device("mps")
        else:
            return torch.device("cpu")

    return torch.device(device_str)
