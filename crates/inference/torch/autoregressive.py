"""Autoregressive (standard token-by-token) generation for HuggingFace models.

Provides non-streaming and streaming generation using the standard
HuggingFace generate API and manual token-by-token sampling.
"""

import torch


def _generate_autoregressive(model, tokenizer, device, formatted_prompt,
                             max_tokens, temperature, top_p):
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

    with torch.no_grad():
        outputs = model.generate(
            **inputs,
            max_new_tokens=max_tokens,
            temperature=max(temperature, 0.01),
            top_p=top_p,
            do_sample=temperature > 0,
        )

    input_len = inputs["input_ids"].shape[1]
    generated = outputs[0][input_len:]
    return tokenizer.decode(generated, skip_special_tokens=True)


def _generate_autoregressive_streaming(model, tokenizer, device,
                                       formatted_prompt, max_tokens,
                                       temperature, top_p):
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

    for _ in range(max_tokens):
        with torch.no_grad():
            outputs = model(input_ids)
            logits = outputs.logits[:, -1, :]

            if temperature > 0:
                logits = logits / max(temperature, 0.01)
                probs = torch.softmax(logits, dim=-1)
                next_token = torch.multinomial(probs, num_samples=1)
            else:
                next_token = logits.argmax(dim=-1, keepdim=True)

        if next_token.item() == tokenizer.eos_token_id:
            break

        token_str = tokenizer.decode(next_token[0], skip_special_tokens=True)
        yield {"mode": "append", "text": token_str}

        input_ids = torch.cat([input_ids, next_token], dim=-1)
