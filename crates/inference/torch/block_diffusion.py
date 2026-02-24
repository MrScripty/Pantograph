"""Block diffusion generation for dLLM / SDAR / TraDo models.

Provides non-streaming and streaming block diffusion generation,
with a shared decode loop extracted to eliminate code duplication.
"""

import torch
from torch.nn import functional as F
from transformers.cache_utils import DynamicCache

# Mask token ID used by SDAR-family models
_DLLM_MASK_ID = 151669


def _sample_topk_topp(logits, temperature=1.0, top_k=0, top_p=1.0):
    """Sample from logits with top-k and top-p filtering."""
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
        scatter_mask = torch.scatter(
            torch.full_like(logits, False, dtype=torch.bool), -1, sorted_indices, mask
        )
        logits = logits.masked_fill(scatter_mask, float("-inf"))

    probs = F.softmax(logits, dim=-1)
    token = torch.multinomial(probs, num_samples=1)
    token_prob = torch.gather(probs, -1, token)
    return token.view(*orig_shape), token_prob.view(*orig_shape)


def _select_transfer(x0, x0_p, mask_index, num_to_transfer, strategy, confidence_threshold):
    """Select which mask positions to reveal based on confidence strategy."""
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


def _get_num_transfer_tokens(block_length, steps):
    """Compute number of tokens to transfer at each denoising step."""
    base = block_length // steps
    remainder = block_length % steps
    t = torch.zeros(steps, dtype=torch.int64) + base
    t[:remainder] += 1
    return t


@torch.no_grad()
def _block_diffusion_decode(
    model, x, prompt_length, past_key_values,
    attn_mask, position_ids, block_length,
    denoising_steps, mask_id, temperature,
    top_k, top_p, remasking_strategy,
    confidence_threshold, yield_blocks=False,
):
    """Shared prefill + decode loop for block diffusion generation.

    Handles the prefill stage and iterates over generation blocks, applying
    the denoising schedule to each block.

    When yield_blocks=False, returns the full x tensor after all blocks.
    When yield_blocks=True, is a generator that yields non-mask token IDs
    from each completed block.
    """
    num_blocks = x.shape[1] // block_length
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
                model(
                    cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                    past_key_values=past_key_values, use_cache=True, store_kv=True,
                )
                break

            logits = model(
                cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                past_key_values=past_key_values, use_cache=True, store_kv=False,
            ).logits

            x0, x0_p = _sample_topk_topp(logits, temperature, top_k, top_p)
            transfer_index = _select_transfer(
                x0, x0_p, mask_index, num_transfer[step],
                remasking_strategy, confidence_threshold,
            )
            cur_x[transfer_index] = x0[transfer_index]

        x[:, s:e] = cur_x

        if yield_blocks:
            block_tokens = cur_x[0][cur_x[0] != mask_id]
            if len(block_tokens) > 0:
                yield block_tokens

    if not yield_blocks:
        yield x


@torch.no_grad()
def _block_diffusion_decode_refining(
    model, x, prompt_length, past_key_values,
    attn_mask, position_ids, block_length,
    denoising_steps, mask_id, temperature,
    top_k, top_p, remasking_strategy,
    confidence_threshold, tokenizer,
    mask_placeholder="\u00b7",
):
    """Prefill + decode loop that yields the full decoded sequence after each
    denoising step, enabling in-place refinement display.

    Mask tokens are rendered as `mask_placeholder` (default: middle dot) so the
    user can see the full output length from the start.
    """
    num_blocks = x.shape[1] // block_length
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

    def _decode_full_sequence():
        """Decode the entire generated region, replacing mask tokens with placeholder."""
        non_mask = x[0, prompt_length:]
        parts = []
        i = 0
        tokens = non_mask.tolist()
        while i < len(tokens):
            if tokens[i] == mask_id:
                mask_count = 0
                while i < len(tokens) and tokens[i] == mask_id:
                    mask_count += 1
                    i += 1
                parts.append(mask_placeholder * mask_count)
            else:
                start = i
                while i < len(tokens) and tokens[i] != mask_id:
                    i += 1
                chunk_ids = torch.tensor(tokens[start:i], dtype=torch.long)
                parts.append(tokenizer.decode(chunk_ids, skip_special_tokens=True))
        return "".join(parts)

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
                model(
                    cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                    past_key_values=past_key_values, use_cache=True, store_kv=True,
                )
                break

            logits = model(
                cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                past_key_values=past_key_values, use_cache=True, store_kv=False,
            ).logits

            x0, x0_p = _sample_topk_topp(logits, temperature, top_k, top_p)
            transfer_index = _select_transfer(
                x0, x0_p, mask_index, num_transfer[step],
                remasking_strategy, confidence_threshold,
            )
            cur_x[transfer_index] = x0[transfer_index]

            # Write current block state back and yield full sequence
            x[:, s:e] = cur_x
            yield _decode_full_sequence()

        # Final write-back after block is done
        x[:, s:e] = cur_x


def _setup_block_diffusion(model, input_ids, gen_length, block_length, mask_id):
    """Common setup for block diffusion: create tensors, attention mask, positions.

    Returns (x, attn_mask, position_ids, past_key_values, prompt_length).
    """
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

    return x, attn_mask, position_ids, past_key_values, prompt_length


def _build_masked_sequence(segments, tokenizer, mask_id):
    """Build a pre-anchored token sequence from masked prompt segments.

    Anchored segments get their real token IDs, masked segments get mask_id.
    Returns (input_ids tensor, anchor_mask tensor).
    """
    all_ids = []
    anchor_mask = []
    for seg in segments:
        ids = tokenizer.encode(seg["text"], add_special_tokens=False)
        all_ids.extend(ids)
        if seg["masked"]:
            anchor_mask.extend([False] * len(ids))
        else:
            anchor_mask.extend([True] * len(ids))

    input_ids = torch.tensor([all_ids], dtype=torch.long)
    # Replace masked positions with mask_id
    anchor_t = torch.tensor(anchor_mask, dtype=torch.bool)
    input_ids[0, ~anchor_t] = mask_id

    return input_ids, anchor_t


def _generate_dllm_masked(model, tokenizer, device, segments, **kwargs):
    """Generate with masked prompt for block diffusion models.

    Anchored segments are preserved, masked segments are regenerated.
    """
    mask_id = getattr(model.config, 'mask_token_id', _DLLM_MASK_ID)
    input_ids, anchor_mask = _build_masked_sequence(segments, tokenizer, mask_id)
    input_ids = input_ids.to(device)
    anchor_mask = anchor_mask.to(device)

    max_tokens = kwargs.get("max_tokens", 512)
    temperature = kwargs.get("temperature", 0.2)
    top_p = kwargs.get("top_p", 0.9)
    block_length = 8
    denoising_steps = 8

    # Pad to block boundary
    seq_len = input_ids.shape[1]
    num_blocks = (seq_len + block_length - 1) // block_length
    total_length = num_blocks * block_length
    if total_length > seq_len:
        pad = torch.full((1, total_length - seq_len), mask_id, dtype=torch.long, device=device)
        input_ids = torch.cat([input_ids, pad], dim=1)
        anchor_pad = torch.zeros(total_length - seq_len, dtype=torch.bool, device=device)
        anchor_mask = torch.cat([anchor_mask, anchor_pad])

    # Build attention mask and positions
    block_mask = torch.tril(torch.ones(num_blocks, num_blocks, device=device))
    attn_mask = (
        block_mask.repeat_interleave(block_length, dim=0)
        .repeat_interleave(block_length, dim=1)
        .unsqueeze(0)
    )
    position_ids = torch.arange(total_length, device=device).unsqueeze(0)

    x = input_ids.clone()
    num_transfer = _get_num_transfer_tokens(block_length, denoising_steps)
    past_key_values = DynamicCache()

    for nb in range(num_blocks):
        s, e = nb * block_length, (nb + 1) * block_length
        cur_x = x[:, s:e].clone()
        cur_anchor = anchor_mask[s:e]
        cur_attn = attn_mask[:, s:e, :e]
        if cur_attn.dim() == 3:
            cur_attn = cur_attn[:, None, :, :]
        cur_pos = position_ids[:, s:e]

        # Only denoise blocks that have masked positions
        has_masks = (cur_x[0] == mask_id).any()
        if not has_masks:
            # Pure anchor block — just run forward for KV cache
            with torch.no_grad():
                model(
                    cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                    past_key_values=past_key_values, use_cache=True, store_kv=True,
                )
            continue

        for step in range(denoising_steps + 1):
            mask_index = cur_x == mask_id
            if mask_index.sum() == 0:
                with torch.no_grad():
                    model(
                        cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                        past_key_values=past_key_values, use_cache=True, store_kv=True,
                    )
                break

            with torch.no_grad():
                logits = model(
                    cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                    past_key_values=past_key_values, use_cache=True, store_kv=False,
                ).logits

            x0, x0_p = _sample_topk_topp(logits, max(temperature, 0.01), 0, top_p)
            transfer_index = _select_transfer(
                x0, x0_p, mask_index, num_transfer[step],
                "low_confidence_dynamic", 0.85,
            )
            # Preserve anchored positions
            transfer_index[:, cur_anchor] = False
            cur_x[transfer_index] = x0[transfer_index]

        x[:, s:e] = cur_x

    output_ids = x[0].tolist()
    return tokenizer.decode(output_ids, skip_special_tokens=True)


def _generate_dllm_masked_streaming(model, tokenizer, device, segments, **kwargs):
    """Streaming version of masked generation. Yields intermediate results."""
    mask_id = getattr(model.config, 'mask_token_id', _DLLM_MASK_ID)
    input_ids, anchor_mask = _build_masked_sequence(segments, tokenizer, mask_id)
    input_ids = input_ids.to(device)
    anchor_mask = anchor_mask.to(device)

    max_tokens = kwargs.get("max_tokens", 512)
    temperature = kwargs.get("temperature", 0.2)
    top_p = kwargs.get("top_p", 0.9)
    block_length = 8
    denoising_steps = 8
    mask_placeholder = "\u00b7"

    # Pad to block boundary
    seq_len = input_ids.shape[1]
    num_blocks = (seq_len + block_length - 1) // block_length
    total_length = num_blocks * block_length
    if total_length > seq_len:
        pad = torch.full((1, total_length - seq_len), mask_id, dtype=torch.long, device=device)
        input_ids = torch.cat([input_ids, pad], dim=1)
        anchor_pad = torch.zeros(total_length - seq_len, dtype=torch.bool, device=device)
        anchor_mask = torch.cat([anchor_mask, anchor_pad])

    block_mask = torch.tril(torch.ones(num_blocks, num_blocks, device=device))
    attn_mask = (
        block_mask.repeat_interleave(block_length, dim=0)
        .repeat_interleave(block_length, dim=1)
        .unsqueeze(0)
    )
    position_ids = torch.arange(total_length, device=device).unsqueeze(0)

    x = input_ids.clone()
    num_transfer = _get_num_transfer_tokens(block_length, denoising_steps)
    past_key_values = DynamicCache()

    def _decode_full():
        tokens = x[0].tolist()
        parts = []
        i = 0
        while i < len(tokens):
            if tokens[i] == mask_id:
                count = 0
                while i < len(tokens) and tokens[i] == mask_id:
                    count += 1
                    i += 1
                parts.append(mask_placeholder * count)
            else:
                start = i
                while i < len(tokens) and tokens[i] != mask_id:
                    i += 1
                chunk = torch.tensor(tokens[start:i], dtype=torch.long)
                parts.append(tokenizer.decode(chunk, skip_special_tokens=True))
        return "".join(parts)

    for nb in range(num_blocks):
        s, e = nb * block_length, (nb + 1) * block_length
        cur_x = x[:, s:e].clone()
        cur_anchor = anchor_mask[s:e]
        cur_attn = attn_mask[:, s:e, :e]
        if cur_attn.dim() == 3:
            cur_attn = cur_attn[:, None, :, :]
        cur_pos = position_ids[:, s:e]

        has_masks = (cur_x[0] == mask_id).any()
        if not has_masks:
            with torch.no_grad():
                model(
                    cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                    past_key_values=past_key_values, use_cache=True, store_kv=True,
                )
            continue

        for step in range(denoising_steps + 1):
            mask_index = cur_x == mask_id
            if mask_index.sum() == 0:
                with torch.no_grad():
                    model(
                        cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                        past_key_values=past_key_values, use_cache=True, store_kv=True,
                    )
                break

            with torch.no_grad():
                logits = model(
                    cur_x, attention_mask=cur_attn, position_ids=cur_pos,
                    past_key_values=past_key_values, use_cache=True, store_kv=False,
                ).logits

            x0, x0_p = _sample_topk_topp(logits, max(temperature, 0.01), 0, top_p)
            transfer_index = _select_transfer(
                x0, x0_p, mask_index, num_transfer[step],
                "low_confidence_dynamic", 0.85,
            )
            transfer_index[:, cur_anchor] = False
            cur_x[transfer_index] = x0[transfer_index]

            x[:, s:e] = cur_x
            yield {"mode": "replace", "text": _decode_full()}

        x[:, s:e] = cur_x


def _generate_dllm(model, tokenizer, device, formatted_prompt, max_tokens,
                   temperature, top_p):
    """Generate using block diffusion (full response).

    Args:
        model: The loaded model.
        tokenizer: The loaded tokenizer.
        device: torch.device to use.
        formatted_prompt: Already-formatted prompt string.
        max_tokens: Maximum generation length.
        temperature: Sampling temperature.
        top_p: Nucleus sampling threshold.

    Returns:
        Decoded string of generated text.
    """
    tokens = tokenizer(
        formatted_prompt, return_tensors="pt", padding=True, truncation=True,
    ).to(device)

    x, attn_mask, position_ids, past_key_values, prompt_length = (
        _setup_block_diffusion(
            model, tokens["input_ids"], max_tokens, block_length=8,
            mask_id=_DLLM_MASK_ID,
        )
    )

    # _block_diffusion_decode with yield_blocks=False yields the final x once
    for result in _block_diffusion_decode(
        model, x, prompt_length, past_key_values,
        attn_mask, position_ids, block_length=8,
        denoising_steps=8, mask_id=_DLLM_MASK_ID,
        temperature=max(temperature, 0.01), top_k=0, top_p=top_p,
        remasking_strategy="low_confidence_dynamic",
        confidence_threshold=0.85, yield_blocks=False,
    ):
        output_ids = result

    generated = output_ids[0][prompt_length:]
    return tokenizer.decode(generated, skip_special_tokens=True)


def _generate_dllm_streaming(model, tokenizer, device, formatted_prompt,
                             max_tokens, temperature, top_p):
    """Generate using block diffusion, yielding full text after each denoising step.

    Each yield is the entire decoded sequence so far (replace mode), allowing the
    UI to show text refining in-place as denoising progresses.

    Args:
        model: The loaded model.
        tokenizer: The loaded tokenizer.
        device: torch.device to use.
        formatted_prompt: Already-formatted prompt string.
        max_tokens: Maximum generation length.
        temperature: Sampling temperature.
        top_p: Nucleus sampling threshold.

    Yields:
        Dicts with {"mode": "replace", "text": ...} for each refinement step.
    """
    tokens = tokenizer(
        formatted_prompt, return_tensors="pt", padding=True, truncation=True,
    ).to(device)

    x, attn_mask, position_ids, past_key_values, prompt_length = (
        _setup_block_diffusion(
            model, tokens["input_ids"], max_tokens, block_length=8,
            mask_id=_DLLM_MASK_ID,
        )
    )

    for full_text in _block_diffusion_decode_refining(
        model, x, prompt_length, past_key_values,
        attn_mask, position_ids, block_length=8,
        denoising_steps=8, mask_id=_DLLM_MASK_ID,
        temperature=max(temperature, 0.01), top_k=0, top_p=top_p,
        remasking_strategy="low_confidence_dynamic",
        confidence_threshold=0.85, tokenizer=tokenizer,
    ):
        yield {"mode": "replace", "text": full_text}
