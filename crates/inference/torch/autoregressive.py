"""Autoregressive (standard token-by-token) generation for HuggingFace models.

Provides non-streaming and streaming generation using the standard
HuggingFace generate API and manual token-by-token sampling.
"""

import copy
import math
from types import MethodType

import torch
from transformers.cache_utils import DynamicCache
from transformers.generation.logits_process import (
    MinNewTokensLengthLogitsProcessor, RepetitionPenaltyLogitsProcessor,
)


class RepetitionPenaltyNumericsError(ValueError):
    """The requested repetition operation cannot produce usable f32 scores."""


class MinimumNewTokensError(ValueError):
    """The minimum length has no eligible non-EOS token to select."""


class _CheckedMinimumNewTokens:
    def __init__(self, processor, floor=None):
        self.processor = processor
        self.floor = processor.min_new_tokens if floor is None else floor

    def __call__(self, input_ids, scores):
        adjusted = self.processor(input_ids, scores)
        active = input_ids.shape[-1] - self.processor.prompt_length_to_skip < self.floor
        if active and torch.any(~torch.isfinite(adjusted).any(dim=-1)):
            raise MinimumNewTokensError("min_new_tokens leaves no finite eligible token")
        return adjusted


class _MinimumSelectionGuard:
    """Refuse when later native processors invalidate an active EOS floor."""

    def __init__(self, processor, floor=None):
        self.processor = processor
        self.floor = processor.min_new_tokens if floor is None else floor

    def __call__(self, input_ids, scores):
        if input_ids.shape[-1] - self.processor.prompt_length_to_skip < self.floor:
            eos_mask = torch.isin(torch.arange(scores.shape[-1], device=scores.device),
                                  self.processor.eos_token_id.to(scores.device))
            if (torch.any(~torch.isfinite(scores).any(dim=-1))
                    or torch.any(~torch.isneginf(scores[..., eos_mask]))):
                raise MinimumNewTokensError(
                    "min_new_tokens leaves no finite eligible token or conflicts with later processors")
        return scores


class _CheckedRepetitionPenalty:
    """Apply the official operation in f32, preserving only existing -Inf masks."""

    def __init__(self, processor):
        self.processor = processor
        self.calls = 0

    def __call__(self, input_ids, scores):
        masked = torch.isneginf(scores)
        scores = scores.float()
        if torch.any(~torch.isfinite(scores) & ~masked):
            raise RepetitionPenaltyNumericsError(
                "repetition_penalty requires finite float32 logits or existing -inf masks")
        adjusted = self.processor(input_ids, scores)
        if (torch.any(~torch.isfinite(adjusted) & ~masked)
                or torch.any(masked & ~torch.isneginf(adjusted))):
            raise RepetitionPenaltyNumericsError(
                "repetition_penalty produced non-finite float32 logits")
        if torch.any(~torch.isfinite(adjusted).any(dim=-1)):
            raise RepetitionPenaltyNumericsError(
                "repetition_penalty has no finite selectable logits")
        self.calls += 1
        return adjusted


def _generate_native_checked(model, *, minimum_to_enforce=None, **kwargs):
    """Guard native repetition in place without changing resident model state.

    Public custom processors run after sanitizers and masks in Transformers.
    A request-local shallow copy instead wraps the resolved built-in processor
    at its original position, retaining generation defaults and processor order.
    Weights remain shared. Custom generation that bypasses this builder is
    unsupported and fails closed rather than returning unchecked output.
    """
    # Inherited native floors retain Transformers' warning/forced-EOS semantics.
    # SDAR retries supply an internal floor and explicitly preserve authorship.
    if minimum_to_enforce is None:
        minimum_to_enforce = kwargs.get("min_new_tokens")
    penalty = _resolve_repetition_penalty(model, kwargs.get("repetition_penalty"))
    request_model = copy.copy(model)
    if request_model is model or not callable(getattr(request_model, "_get_logits_processor", None)):
        raise RepetitionPenaltyNumericsError(
            "native repetition checking requires the Transformers logits processor builder")
    build_processors = request_model._get_logits_processor
    checked = []
    built = False

    def build_checked(_self, *args, **builder_kwargs):
        nonlocal built
        processors = build_processors(*args, **builder_kwargs)
        minimum_guards = []
        for index, processor in enumerate(processors):
            if type(processor) is RepetitionPenaltyLogitsProcessor:
                guard = _CheckedRepetitionPenalty(processor)
                checked.append(guard)
                processors[index] = guard
            elif minimum_to_enforce and type(processor) is MinNewTokensLengthLogitsProcessor:
                processors[index] = _CheckedMinimumNewTokens(processor, minimum_to_enforce)
                minimum_guards.append(_MinimumSelectionGuard(processor, minimum_to_enforce))
        if penalty != 1.0 and not checked:
            raise RepetitionPenaltyNumericsError(
                "native generation did not provide its repetition processor")
        processors.extend(minimum_guards)
        built = True
        return processors

    request_model._get_logits_processor = MethodType(build_checked, request_model)
    outputs = request_model.generate(**kwargs)
    if not built or (penalty != 1.0 and not any(guard.calls for guard in checked)):
        raise RepetitionPenaltyNumericsError(
            "native generation bypassed repetition checking")
    return outputs


def _resolve_repetition_penalty(model, repetition_penalty):
    """Use the model's generation default only when the request omits it."""
    if repetition_penalty is None:
        repetition_penalty = getattr(getattr(model, "generation_config", None),
                                     "repetition_penalty", 1.0)
    if isinstance(repetition_penalty, bool) or not isinstance(repetition_penalty, (int, float)):
        raise ValueError("repetition_penalty must be a finite positive number")
    try:
        penalty = float(repetition_penalty)
    except OverflowError as exc:
        raise ValueError("repetition_penalty must be a finite positive number") from exc
    if not math.isfinite(penalty) or penalty <= 0:
        raise ValueError("repetition_penalty must be a finite positive number")
    return penalty


def _resolve_top_k(model, top_k):
    """Resolve top_k from arg or model generation config."""
    if top_k is not None:
        try:
            return int(top_k)
        except Exception:
            return 0
    gen_cfg = getattr(model, "generation_config", None)
    cfg_top_k = getattr(gen_cfg, "top_k", None) if gen_cfg is not None else None
    try:
        return int(cfg_top_k) if cfg_top_k is not None else 0
    except Exception:
        return 0


def _resolve_min_new_tokens(model, min_new_tokens, max_tokens):
    """Resolve the EOS floor without allowing a promise beyond the token budget."""
    authored = min_new_tokens is not None
    if min_new_tokens is None:
        min_new_tokens = getattr(getattr(model, "generation_config", None),
                                 "min_new_tokens", None)
        if min_new_tokens is None:
            min_new_tokens = 0
    if (isinstance(min_new_tokens, bool) or not isinstance(min_new_tokens, int)
            or not 0 <= min_new_tokens <= (1 << 32) - 1):
        raise ValueError("min_new_tokens must be a non-negative u32 integer")
    if authored or min_new_tokens:
        if (isinstance(max_tokens, bool) or not isinstance(max_tokens, int)
                or not 0 < max_tokens <= (1 << 32) - 1):
            raise ValueError("max_tokens must be a positive u32 integer for min_new_tokens")
    if authored and min_new_tokens > max_tokens:
        raise ValueError("min_new_tokens must not exceed max_tokens")
    # Manual loops stop at the request budget even for a larger model default.
    return min(min_new_tokens, max_tokens)


def _minimum_processor(prompt_length, floor, eos_ids, device):
    if not floor or not eos_ids:
        return None
    return _CheckedMinimumNewTokens(MinNewTokensLengthLogitsProcessor(
        prompt_length, floor, sorted(eos_ids), device=device))


def _apply_minimum(processor, history, scores):
    return scores if processor is None else processor(history, scores)


def _sample_next_token(logits, temperature, top_p, top_k=0):
    """Sample one token from logits with temperature + top-k + top-p."""
    if temperature <= 0:
        return logits.argmax(dim=-1, keepdim=True)

    logits = logits / max(temperature, 0.01)
    if top_k and top_k > 0:
        # Match Transformers: k above the vocabulary retains every token.
        values, _ = torch.topk(logits, min(top_k, logits.size(-1)))
        logits = torch.where(logits < values[..., -1, None], float("-inf"), logits)
    if top_p < 1.0:
        # Match Transformers TopPLogitsWarper, including ties and p=0:
        # remove the low-probability tail and retain at least one token.
        sorted_logits, sorted_indices = torch.sort(logits, descending=False)
        cum_probs = torch.cumsum(torch.softmax(sorted_logits, dim=-1), dim=-1)
        mask = cum_probs <= (1.0 - top_p)
        mask[..., -1] = False
        scatter_mask = torch.zeros_like(logits, dtype=torch.bool).scatter(-1, sorted_indices, mask)
        logits = logits.masked_fill(scatter_mask, float("-inf"))

    probs = torch.softmax(logits, dim=-1)
    return torch.multinomial(probs, num_samples=1)


def _eos_ids(tokenizer, model=None):
    eos = getattr(getattr(model, "generation_config", None), "eos_token_id", None)
    if eos is None:
        eos = getattr(tokenizer, "eos_token_id", None)
    if eos is None:
        return set()
    if isinstance(eos, int):
        return {int(eos)}
    if isinstance(eos, (list, tuple)):
        return {int(x) for x in eos if x is not None}
    return set()


def _generate_sdar_cached(model, tokenizer, device, formatted_prompt,
                          max_tokens, temperature, top_p, top_k=None, repetition_penalty=None,
                          min_new_tokens=None):
    """TraDo/SDAR decode loop with explicit store_kv=True cache updates."""
    floor = _resolve_min_new_tokens(model, min_new_tokens, max_tokens)
    inputs = tokenizer(formatted_prompt, return_tensors="pt").to(device)
    input_ids = inputs["input_ids"]
    prompt_len = input_ids.shape[1]
    eos_ids = _eos_ids(tokenizer)
    minimum_processor = _minimum_processor(
        prompt_len, floor, eos_ids | _eos_ids(tokenizer, model), device)
    resolved_top_k = _resolve_top_k(model, top_k)
    penalty_processor = _CheckedRepetitionPenalty(RepetitionPenaltyLogitsProcessor(
        _resolve_repetition_penalty(model, repetition_penalty)))
    token_history = input_ids

    past_key_values = DynamicCache()
    position_ids = torch.arange(prompt_len, device=device).unsqueeze(0)
    causal = torch.tril(torch.ones(prompt_len, prompt_len, device=device, dtype=torch.bool))
    attention_mask = causal.unsqueeze(0).unsqueeze(0)  # [B,1,Q,K]

    with torch.no_grad():
        outputs = model(
            input_ids,
            attention_mask=attention_mask,
            position_ids=position_ids,
            past_key_values=past_key_values,
            use_cache=True,
            store_kv=True,
        )
        logits = outputs.logits[:, -1, :]

    generated_ids = []
    cur_pos = prompt_len

    for _ in range(max_tokens):
        next_token = _sample_next_token(_apply_minimum(
            minimum_processor, token_history, penalty_processor(token_history, logits)),
                                        temperature, top_p, resolved_top_k)
        token_id = int(next_token.item())
        if token_id in eos_ids:
            break
        generated_ids.append(token_id)
        token_history = torch.cat([token_history, next_token], dim=-1)

        with torch.no_grad():
            outputs = model(
                next_token,
                position_ids=torch.tensor([[cur_pos]], device=device),
                past_key_values=past_key_values,
                use_cache=True,
                store_kv=True,
            )
            logits = outputs.logits[:, -1, :]
        cur_pos += 1

    full_sequence = torch.cat(
        [input_ids[0].detach().cpu(), torch.tensor(generated_ids, dtype=input_ids.dtype)],
        dim=0,
    ).tolist()
    if not generated_ids:
        return "", full_sequence, past_key_values
    return tokenizer.decode(generated_ids, skip_special_tokens=True), full_sequence, past_key_values


def _continue_sdar_cached(model, tokenizer, device, formatted_prompt,
                          max_tokens, temperature, top_p,
                          cached_token_ids, past_key_values, top_k=None, repetition_penalty=None,
                          min_new_tokens=None):
    """Continue SDAR/TraDo decoding from a previously captured KV cache.

    `formatted_prompt` is treated as a suffix to append to the existing cached
    context represented by `cached_token_ids` + `past_key_values`.
    """
    floor = _resolve_min_new_tokens(model, min_new_tokens, max_tokens)
    if not cached_token_ids:
        raise RuntimeError("Live KV cache is missing cached token_ids")
    if past_key_values is None:
        raise RuntimeError("Live KV cache is missing cache data")
    if not hasattr(past_key_values, "crop"):
        raise RuntimeError("Live KV cache does not support crop-based replay")

    resolved_top_k = _resolve_top_k(model, top_k)
    penalty_processor = _CheckedRepetitionPenalty(RepetitionPenaltyLogitsProcessor(
        _resolve_repetition_penalty(model, repetition_penalty)))
    eos_ids = _eos_ids(tokenizer)
    full_sequence = [int(token_id) for token_id in cached_token_ids]
    cur_pos = len(full_sequence)

    suffix_inputs = tokenizer(
        formatted_prompt,
        return_tensors="pt",
        add_special_tokens=False,
    ).to(device)
    suffix_token_ids = suffix_inputs["input_ids"][0].detach().cpu().tolist()

    logits = None

    if suffix_token_ids:
        for token_id in suffix_token_ids:
            next_input = torch.tensor([[int(token_id)]], device=device)
            with torch.no_grad():
                outputs = model(
                    next_input,
                    position_ids=torch.tensor([[cur_pos]], device=device),
                    past_key_values=past_key_values,
                    use_cache=True,
                    store_kv=True,
                )
                logits = outputs.logits[:, -1, :]
            full_sequence.append(int(token_id))
            cur_pos += 1
    else:
        replay_position = cur_pos - 1
        past_key_values.crop(replay_position)
        replay_token = torch.tensor([[full_sequence[-1]]], device=device)
        with torch.no_grad():
            outputs = model(
                replay_token,
                position_ids=torch.tensor([[replay_position]], device=device),
                past_key_values=past_key_values,
                use_cache=True,
                store_kv=True,
            )
            logits = outputs.logits[:, -1, :]

    minimum_processor = _minimum_processor(
        len(full_sequence), floor, eos_ids | _eos_ids(tokenizer, model), logits.device)
    generated_ids = []
    for _ in range(max_tokens):
        token_history = torch.tensor([full_sequence], device=logits.device, dtype=torch.long)
        next_token = _sample_next_token(_apply_minimum(
            minimum_processor, token_history, penalty_processor(token_history, logits)),
                                        temperature, top_p, resolved_top_k)
        token_id = int(next_token.item())
        if token_id in eos_ids:
            break
        generated_ids.append(token_id)
        full_sequence.append(token_id)

        with torch.no_grad():
            outputs = model(
                next_token,
                position_ids=torch.tensor([[cur_pos]], device=device),
                past_key_values=past_key_values,
                use_cache=True,
                store_kv=True,
            )
            logits = outputs.logits[:, -1, :]
        cur_pos += 1

    if not generated_ids:
        return "", full_sequence, past_key_values
    return tokenizer.decode(generated_ids, skip_special_tokens=True), full_sequence, past_key_values


def _generate_autoregressive(model, tokenizer, device, formatted_prompt,
                             max_tokens, temperature, top_p, top_k=None, repetition_penalty=None,
                             min_new_tokens=None):
    """Generate a complete response using standard autoregressive decoding.

    Args:
        model: The loaded model.
        tokenizer: The loaded tokenizer.
        device: torch.device to use.
        formatted_prompt: Already-formatted prompt string.
        max_tokens: Maximum number of new tokens.
        temperature: Sampling temperature.
        top_p: Nucleus sampling threshold.

    Returns:
        Decoded string of generated text.
    """
    if min_new_tokens is not None:
        _resolve_min_new_tokens(model, min_new_tokens, max_tokens)
    inputs = tokenizer(formatted_prompt, return_tensors="pt").to(device)

    resolved_top_k = _resolve_top_k(model, top_k)
    # Validation also covers direct worker calls; omitted kwargs leave the
    # Transformers generation_config default in charge.
    resolved_penalty = _resolve_repetition_penalty(model, repetition_penalty)

    with torch.no_grad():
        gen_kwargs = {
            "max_new_tokens": max_tokens,
            "temperature": max(temperature, 0.01),
            "top_p": top_p,
            "do_sample": temperature > 0,
        }
        if top_k is not None or resolved_top_k > 0:
            gen_kwargs["top_k"] = resolved_top_k
        if repetition_penalty is not None:
            gen_kwargs["repetition_penalty"] = resolved_penalty
        if min_new_tokens is not None:
            gen_kwargs["min_new_tokens"] = min_new_tokens
        outputs = _generate_native_checked(model, **inputs, **gen_kwargs)

    input_len = inputs["input_ids"].shape[1]
    generated = outputs[0][input_len:]
    return tokenizer.decode(generated, skip_special_tokens=True)


def _generate_autoregressive_streaming(model, tokenizer, device,
                                       formatted_prompt, max_tokens,
                                       temperature, top_p, top_k=None, repetition_penalty=None,
                                       min_new_tokens=None):
    """Generate tokens one at a time for streaming output.

    Args:
        model: The loaded model.
        tokenizer: The loaded tokenizer.
        device: torch.device to use.
        formatted_prompt: Already-formatted prompt string.
        max_tokens: Maximum number of new tokens.
        temperature: Sampling temperature.
        top_p: Nucleus sampling threshold.

    Yields:
        Dicts with {"mode": "append", "text": ...} for each token.
    """
    floor = _resolve_min_new_tokens(model, min_new_tokens, max_tokens)
    inputs = tokenizer(formatted_prompt, return_tensors="pt").to(device)
    input_ids = inputs["input_ids"]
    eos_ids = _eos_ids(tokenizer)
    minimum_processor = _minimum_processor(
        input_ids.shape[-1], floor, eos_ids | _eos_ids(tokenizer, model), device)
    resolved_top_k = _resolve_top_k(model, top_k)
    penalty_processor = _CheckedRepetitionPenalty(RepetitionPenaltyLogitsProcessor(
        _resolve_repetition_penalty(model, repetition_penalty)))

    for _ in range(max_tokens):
        with torch.no_grad():
            outputs = model(input_ids)
            logits = outputs.logits[:, -1, :]

            next_token = _sample_next_token(_apply_minimum(
                minimum_processor, input_ids, penalty_processor(input_ids, logits)),
                                            temperature, top_p, resolved_top_k)

        if next_token.item() in eos_ids:
            break

        token_str = tokenizer.decode(next_token[0], skip_special_tokens=True)
        yield {"mode": "append", "text": token_str}

        input_ids = torch.cat([input_ids, next_token], dim=-1)
