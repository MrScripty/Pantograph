"""Autoregressive (standard token-by-token) generation for HuggingFace models.

Provides non-streaming and streaming generation using the standard
HuggingFace generate API and manual token-by-token sampling.
"""

import torch
from transformers.cache_utils import DynamicCache


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


def _sample_next_token(logits, temperature, top_p, top_k=0):
    """Sample one token from logits with temperature + top-k + top-p."""
    if temperature <= 0:
        return logits.argmax(dim=-1, keepdim=True)

    logits = logits / max(temperature, 0.01)
    if top_k and top_k > 0:
        values, _ = torch.topk(logits, top_k)
        logits = torch.where(logits < values[..., -1, None], float("-inf"), logits)
    if top_p < 1.0:
        sorted_logits, sorted_indices = torch.sort(logits, descending=True)
        cum_probs = torch.cumsum(torch.softmax(sorted_logits, dim=-1), dim=-1)
        mask = cum_probs > top_p
        mask[..., 1:] = mask[..., :-1].clone()
        mask[..., 0] = False
        scatter_mask = torch.zeros_like(logits, dtype=torch.bool).scatter(-1, sorted_indices, mask)
        logits = logits.masked_fill(scatter_mask, float("-inf"))

    probs = torch.softmax(logits, dim=-1)
    return torch.multinomial(probs, num_samples=1)


def _eos_ids(tokenizer):
    eos = getattr(tokenizer, "eos_token_id", None)
    if eos is None:
        return set()
    if isinstance(eos, int):
        return {int(eos)}
    if isinstance(eos, (list, tuple)):
        return {int(x) for x in eos if x is not None}
    return set()


def _generate_sdar_cached(model, tokenizer, device, formatted_prompt,
                          max_tokens, temperature, top_p, top_k=None):
    """TraDo/SDAR decode loop with explicit store_kv=True cache updates."""
    inputs = tokenizer(formatted_prompt, return_tensors="pt").to(device)
    input_ids = inputs["input_ids"]
    prompt_len = input_ids.shape[1]
    eos_ids = _eos_ids(tokenizer)
    resolved_top_k = _resolve_top_k(model, top_k)

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
        next_token = _sample_next_token(logits, temperature, top_p, resolved_top_k)
        token_id = int(next_token.item())
        if token_id in eos_ids:
            break
        generated_ids.append(token_id)

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
        return ""
    return tokenizer.decode(generated_ids, skip_special_tokens=True)


def _generate_autoregressive(model, tokenizer, device, formatted_prompt,
                             max_tokens, temperature, top_p, top_k=None):
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
    inputs = tokenizer(formatted_prompt, return_tensors="pt").to(device)

    resolved_top_k = _resolve_top_k(model, top_k)

    with torch.no_grad():
        gen_kwargs = {
            "max_new_tokens": max_tokens,
            "temperature": max(temperature, 0.01),
            "top_p": top_p,
            "do_sample": temperature > 0,
        }
        if resolved_top_k and resolved_top_k > 0:
            gen_kwargs["top_k"] = resolved_top_k
        outputs = model.generate(**inputs, **gen_kwargs)

    input_len = inputs["input_ids"].shape[1]
    generated = outputs[0][input_len:]
    return tokenizer.decode(generated, skip_special_tokens=True)


def _generate_autoregressive_streaming(model, tokenizer, device,
                                       formatted_prompt, max_tokens,
                                       temperature, top_p, top_k=None):
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
    inputs = tokenizer(formatted_prompt, return_tensors="pt").to(device)
    input_ids = inputs["input_ids"]
    resolved_top_k = _resolve_top_k(model, top_k)

    for _ in range(max_tokens):
        with torch.no_grad():
            outputs = model(input_ids)
            logits = outputs.logits[:, -1, :]

            next_token = _sample_next_token(logits, temperature, top_p, resolved_top_k)

        if next_token.item() == tokenizer.eos_token_id:
            break

        token_str = tokenizer.decode(next_token[0], skip_special_tokens=True)
        yield {"mode": "append", "text": token_str}

        input_ids = torch.cat([input_ids, next_token], dim=-1)
