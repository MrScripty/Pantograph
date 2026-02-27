#!/usr/bin/env python3
"""Standalone TraDo smoke test using the official dLLM-RL generation path.

This script intentionally avoids Pantograph's worker wrapper and runs the
official block diffusion generation loop directly against the local model.

Usage:
  ./.venv/bin/python scripts/trado_cli_smoketest.py
"""

from __future__ import annotations

import argparse
import gc
import importlib.util
import time
import traceback
from pathlib import Path

import torch
import torch.nn.functional as F
from transformers.cache_utils import DynamicCache
from transformers import AutoModelForCausalLM, AutoTokenizer


DEFAULT_MODEL_PATH = Path(
    "/media/jeremy/OrangeCream/Linux Software/Pumas-Library/shared-resources/models/llm/gen-verse/trado-8b-instruct"
)
DEFAULT_PROMPT = "Serve me a cup of oolong tea"
DEFAULT_SYSTEM_PROMPT = "You are a maid who loves to serve her master"
REPO_ROOT = Path(__file__).resolve().parent.parent
WORKER_PATH = REPO_ROOT / "crates" / "inference" / "torch" / "worker.py"


def top_k_logits(logits: torch.Tensor, k: int):
    if k <= 0:
        return logits
    values, _ = torch.topk(logits, k)
    min_values = values[..., -1, None]
    return torch.where(logits < min_values, torch.full_like(logits, float("-inf")), logits)


def top_p_logits(logits: torch.Tensor, p: float):
    sorted_logits, sorted_indices = torch.sort(logits, descending=True)
    cumulative_probs = torch.cumsum(F.softmax(sorted_logits, dim=-1), dim=-1)
    sorted_mask = cumulative_probs > p
    sorted_mask[..., 1:] = sorted_mask[..., :-1].clone()
    sorted_mask[..., 0] = False
    mask_indices = torch.scatter(
        torch.full_like(logits, False, dtype=torch.bool), -1, sorted_indices, sorted_mask
    )
    return logits.masked_fill(mask_indices, float("-inf"))


def sample_with_temperature_topk_topp(
    logits: torch.Tensor, temperature: float = 1.0, top_k: int = 0, top_p: float = 1.0
):
    orig_shape = logits.shape[:-1]
    vocab_size = logits.shape[-1]
    logits = logits.reshape(-1, vocab_size)
    if temperature != 1.0:
        logits = logits / temperature
    if top_k > 0:
        logits = top_k_logits(logits, top_k)
    if top_p < 1.0:
        logits = top_p_logits(logits, top_p)
    probs = F.softmax(logits, dim=-1)
    token = torch.multinomial(probs, num_samples=1)
    token_prob = torch.gather(probs, -1, token)
    return token.view(*orig_shape), token_prob.view(*orig_shape)


def get_num_transfer_tokens(block_length: int, steps: int):
    base = block_length // steps
    remainder = block_length % steps
    num_transfer_tokens = torch.zeros(steps, dtype=torch.int64) + base
    num_transfer_tokens[:remainder] += 1
    return num_transfer_tokens


@torch.no_grad()
def block_diffusion_generate(
    model,
    prompt: dict[str, torch.Tensor],
    gen_length: int = 128,
    block_length: int = 8,
    denoising_steps: int = 8,
    temperature: float = 1.0,
    top_k: int = 0,
    top_p: float = 1.0,
    remasking_strategy: str = "low_confidence_dynamic",
    confidence_threshold: float = 0.85,
    mask_id: int = 151669,
    stopping_criteria_idx: list[int] | None = None,
):
    """Official block diffusion generation loop from dLLM-RL generate.py."""
    model.eval()
    input_ids = prompt["input_ids"]
    prompt_length = input_ids.shape[1]
    past_key_values = DynamicCache()

    num_blocks = (prompt_length + gen_length + block_length - 1) // block_length
    total_length = num_blocks * block_length

    block_mask = torch.tril(torch.ones(num_blocks, num_blocks, device=model.device))
    block_diffusion_attention_mask = (
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
        cur_x = x[:, :prefill_length]
        cur_attn_mask = block_diffusion_attention_mask[:, :prefill_length, :prefill_length]
        if cur_attn_mask.dim() == 3:
            cur_attn_mask = cur_attn_mask[:, None, :, :]
        cur_position_ids = position_ids[:, :prefill_length]
        model(
            cur_x,
            attention_mask=cur_attn_mask,
            position_ids=cur_position_ids,
            past_key_values=past_key_values,
            use_cache=True,
            store_kv=True,
        )

    num_transfer_tokens = get_num_transfer_tokens(block_length, denoising_steps)

    for num_block in range(prefill_blocks, num_blocks):
        cur_x = x[:, num_block * block_length : (num_block + 1) * block_length].clone()
        cur_attn_mask = block_diffusion_attention_mask[
            :,
            num_block * block_length : (num_block + 1) * block_length,
            : (num_block + 1) * block_length,
        ]
        if cur_attn_mask.dim() == 3:
            cur_attn_mask = cur_attn_mask[:, None, :, :]
        cur_position_ids = position_ids[:, num_block * block_length : (num_block + 1) * block_length]

        for step in range(denoising_steps + 1):
            transfer_budget_idx = min(step, len(num_transfer_tokens) - 1)
            mask_index = cur_x == mask_id
            if mask_index.sum() == 0:
                model(
                    cur_x,
                    attention_mask=cur_attn_mask,
                    position_ids=cur_position_ids,
                    past_key_values=past_key_values,
                    use_cache=True,
                    store_kv=True,
                )
                break

            logits = model(
                cur_x,
                attention_mask=cur_attn_mask,
                position_ids=cur_position_ids,
                past_key_values=past_key_values,
                use_cache=True,
                store_kv=False,
            ).logits

            x0, x0_p = sample_with_temperature_topk_topp(
                logits, temperature=temperature, top_k=top_k, top_p=top_p
            )

            if remasking_strategy == "sequential":
                transfer_index = torch.zeros_like(x0, dtype=torch.bool)
                for j in range(cur_x.shape[0]):
                    if mask_index[j].any():
                        first_mask_index = mask_index[j].nonzero(as_tuple=True)[0].min().item()
                        transfer_index[j, first_mask_index : first_mask_index + num_transfer_tokens[transfer_budget_idx]] = True
                    else:
                        raise ValueError("No mask tokens found in the current block.")
            elif remasking_strategy == "low_confidence_static":
                confidence = torch.where(mask_index, x0_p, -torch.inf)
                transfer_index = torch.zeros_like(x0, dtype=torch.bool)
                for j in range(confidence.shape[0]):
                    _, idx = torch.topk(confidence[j], num_transfer_tokens[transfer_budget_idx])
                    transfer_index[j, idx] = True
            elif remasking_strategy == "low_confidence_dynamic":
                confidence = torch.where(mask_index, x0_p, -torch.inf)
                transfer_index = torch.zeros_like(x0, dtype=torch.bool)
                for j in range(confidence.shape[0]):
                    high_conf_mask = confidence[j] > confidence_threshold
                    num_high_confidence = high_conf_mask.sum()
                    if num_high_confidence >= num_transfer_tokens[transfer_budget_idx]:
                        transfer_index[j] = high_conf_mask
                    else:
                        _, idx = torch.topk(confidence[j], num_transfer_tokens[transfer_budget_idx])
                        transfer_index[j, idx] = True
            else:
                raise ValueError(f"Unknown remasking strategy: {remasking_strategy}")

            cur_x[transfer_index] = x0[transfer_index]
            x[:, num_block * block_length : (num_block + 1) * block_length] = cur_x

        if stopping_criteria_idx is not None:
            gen_region = x[:, prompt_length:]
            should_stop = any((gen_region == int(stop_idx)).any().item() for stop_idx in stopping_criteria_idx)
            if should_stop:
                break

    return x


def resolve_mask_id(tokenizer, explicit_mask_id: int | None) -> int:
    if explicit_mask_id is not None:
        return explicit_mask_id
    mask_id = tokenizer.convert_tokens_to_ids("<|MASK|>")
    if isinstance(mask_id, int) and mask_id >= 0:
        return mask_id
    return 151669


def clear_cuda_memory():
    gc.collect()
    if torch.cuda.is_available():
        torch.cuda.empty_cache()
        try:
            torch.cuda.ipc_collect()
        except Exception:
            pass


def apply_pantograph_compat_shims():
    """Apply the same meta-safe transformers shims Pantograph uses."""
    if not WORKER_PATH.exists():
        return
    spec = importlib.util.spec_from_file_location("pantograph_torch_worker_shims", WORKER_PATH)
    if spec is None or spec.loader is None:
        return
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    shims = getattr(module, "_apply_compatibility_shims", None)
    if callable(shims):
        shims()


def coerce_model_dtype(model, dtype: torch.dtype):
    """Force all floating params/buffers to a single dtype after load."""
    cast_params = 0
    cast_buffers = 0

    for p in model.parameters():
        if p.is_floating_point() and p.dtype != dtype:
            p.data = p.data.to(dtype=dtype)
            cast_params += 1

    for b in model.buffers():
        if b.is_floating_point() and b.dtype != dtype:
            b.data = b.data.to(dtype=dtype)
            cast_buffers += 1

    return cast_params, cast_buffers


def load_model(args):
    if args.device == "auto":
        device = "cuda" if torch.cuda.is_available() else "cpu"
    else:
        device = args.device

    if device != "cuda":
        raise RuntimeError(
            "Official TraDo inference path requires CUDA (flash-attn model code). "
            "Run this on your GPU machine with --device cuda."
        )

    dtype_map = {
        "bfloat16": torch.bfloat16,
        "float16": torch.float16,
        "float32": torch.float32,
    }
    dtype = dtype_map[args.dtype]

    load_kwargs = {
        "trust_remote_code": True,
        "torch_dtype": dtype,
        "attn_implementation": args.attn_impl,
        "device_map": str(device),
        "low_cpu_mem_usage": True,
    }
    try:
        model = AutoModelForCausalLM.from_pretrained(args.model_path, **load_kwargs)
    except Exception as exc:
        if args.attn_impl == "flash_attention_2":
            print(f"warning: failed with flash_attention_2 ({exc}); retrying with sdpa")
            clear_cuda_memory()
            load_kwargs["attn_implementation"] = "sdpa"
            model = AutoModelForCausalLM.from_pretrained(args.model_path, **load_kwargs)
        else:
            raise

    cast_params, cast_buffers = coerce_model_dtype(model, dtype)
    if cast_params or cast_buffers:
        print(f"dtype_fixups:  params={cast_params}, buffers={cast_buffers}, target={dtype}")

    model.eval()
    tokenizer = AutoTokenizer.from_pretrained(args.model_path, trust_remote_code=True)
    return model, tokenizer, device


def build_messages(prompt: str, system_prompt: str):
    messages = []
    if system_prompt.strip():
        messages.append({"role": "system", "content": system_prompt})
    messages.append({"role": "user", "content": prompt})
    return messages


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description="TraDo CLI smoke test (official block diffusion path)")
    p.add_argument("--model-path", default=str(DEFAULT_MODEL_PATH))
    p.add_argument("--device", default="auto", help="auto|cuda")
    p.add_argument("--dtype", default="float16", choices=["bfloat16", "float16", "float32"])
    p.add_argument("--attn-impl", default="flash_attention_2", choices=["flash_attention_2", "sdpa", "eager"])
    p.add_argument("--prompt", default=DEFAULT_PROMPT)
    p.add_argument("--system-prompt", default=DEFAULT_SYSTEM_PROMPT)
    p.add_argument(
        "--use-chat-template",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Wrap prompt with tokenizer chat template (README snippet for instruct models)",
    )
    p.add_argument("--max-tokens", type=int, default=200, help="Generated token count (gen_length)")
    p.add_argument("--block-length", type=int, default=4)
    p.add_argument("--denoising-steps", type=int, default=4)
    p.add_argument("--temperature", type=float, default=1.0)
    p.add_argument("--top-k", type=int, default=0)
    p.add_argument("--top-p", type=float, default=1.0)
    p.add_argument(
        "--remasking-strategy",
        default="low_confidence_dynamic",
        choices=["low_confidence_dynamic", "low_confidence_static", "sequential"],
    )
    p.add_argument("--confidence-threshold", type=float, default=0.9)
    p.add_argument("--mask-id", type=int, default=None)
    p.add_argument("--seed", type=int, default=1234)
    return p


def main() -> int:
    args = build_parser().parse_args()

    if args.seed is not None:
        torch.manual_seed(args.seed)
        if torch.cuda.is_available():
            torch.cuda.manual_seed_all(args.seed)

    print("=== TraDo CLI Smoke Test (Official Path) ===")
    print(f"model_path:    {args.model_path}")
    print(f"device:        {args.device}")
    print(f"dtype:         {args.dtype}")
    print(f"attn_impl:     {args.attn_impl}")
    print(f"max_tokens:    {args.max_tokens}")
    print(f"block_length:  {args.block_length}")
    print(f"denoising:     {args.denoising_steps}")
    print(f"temperature:   {args.temperature}")
    print(f"top_k:         {args.top_k}")
    print(f"top_p:         {args.top_p}")
    print(f"remasking:     {args.remasking_strategy}")
    print(f"conf_thresh:   {args.confidence_threshold}")
    print("prompt:")
    print(f"  system: {args.system_prompt!r}")
    print(f"  user:   {args.prompt!r}")
    print()

    if args.max_tokens <= 0:
        raise ValueError("--max-tokens must be > 0")
    if args.block_length <= 0:
        raise ValueError("--block-length must be > 0")
    if args.denoising_steps <= 0:
        raise ValueError("--denoising-steps must be > 0")

    try:
        apply_pantograph_compat_shims()
        t0 = time.time()
        model, tokenizer, real_device = load_model(args)
        print(f"model_loaded:  ok (device={real_device})")
        print(f"load_time_s:   {time.time() - t0:.2f}")

        messages = build_messages(args.prompt, args.system_prompt)
        if args.use_chat_template:
            prompt_text = tokenizer.apply_chat_template(messages, tokenize=False, add_generation_prompt=True)
        else:
            # Official dLLM-RL generate.py uses raw prompt text.
            prompt_text = args.prompt

        tokens = tokenizer.batch_encode_plus(
            [prompt_text],
            return_tensors="pt",
            padding=True,
            truncation=True,
            max_length=200,
        )
        tokens = {k: v.to(model.device) for k, v in tokens.items()}
        prompt_ids = tokens["input_ids"]

        mask_id = resolve_mask_id(tokenizer, args.mask_id)
        print(f"mask_id:       {mask_id}")
        print(f"prompt_tokens: {prompt_ids.shape[1]}")

        t1 = time.time()
        out = block_diffusion_generate(
            model=model,
            prompt=tokens,
            gen_length=args.max_tokens,
            block_length=args.block_length,
            denoising_steps=args.denoising_steps,
            temperature=args.temperature,
            top_k=args.top_k,
            top_p=args.top_p,
            remasking_strategy=args.remasking_strategy,
            confidence_threshold=args.confidence_threshold,
            mask_id=mask_id,
        )
        gen_time = time.time() - t1

        output_text = tokenizer.decode(out[0], skip_special_tokens=False)
        text = output_text.replace("<|MASK|>", "").replace("<|endoftext|>", "")

        print()
        print("--- RESULT (repr) ---")
        print(repr(text))
        print("--- RESULT (plain) ---")
        print(text)
        print("----------------------")
        print(f"gen_time_s:    {gen_time:.2f}")
        return 0
    except Exception as exc:
        print(f"ERROR: {exc}")
        if isinstance(exc, ModuleNotFoundError) and ("transformers" in str(exc) or "torch" in str(exc)):
            print("Hint: run with project venv:")
            print("  ./.venv/bin/python scripts/trado_cli_smoketest.py")
        traceback.print_exc()
        return 1
    finally:
        clear_cuda_memory()


if __name__ == "__main__":
    raise SystemExit(main())
