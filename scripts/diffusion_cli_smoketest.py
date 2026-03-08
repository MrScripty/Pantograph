#!/usr/bin/env python3
"""Smoke test the Pantograph diffusion worker against a local model bundle.

This script loads the same worker module Pantograph uses for process-backed
diffusion inference and runs one image-generation request against a local
diffusers-style model directory such as tiny-sd-turbo.

Usage:
  ./.venv/bin/python scripts/diffusion_cli_smoketest.py \
    --model-path /path/to/tiny-sd-turbo \
    --prompt "paper lantern in the rain" \
    --output /tmp/tiny-sd-turbo-smoke.png
"""

from __future__ import annotations

import argparse
import base64
import importlib.util
import json
import sys
import traceback
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
WORKER_PATH = REPO_ROOT / "crates" / "inference" / "torch" / "worker.py"
DEFAULT_PROMPT = "paper lantern in the rain, cinematic lighting, detailed illustration"


def load_worker():
    spec = importlib.util.spec_from_file_location("pantograph_torch_worker_smoke", WORKER_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load worker module spec from {WORKER_PATH}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def write_image(output_path: Path, image_base64: str) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_bytes(base64.b64decode(image_base64))


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Pantograph diffusion worker smoke test")
    parser.add_argument("--model-path", required=True, help="Path to a local diffusers model directory")
    parser.add_argument("--prompt", default=DEFAULT_PROMPT, help="Positive prompt")
    parser.add_argument("--negative-prompt", default="", help="Optional negative prompt")
    parser.add_argument("--steps", type=int, default=4, help="Inference steps")
    parser.add_argument("--guidance-scale", type=float, default=0.0, help="CFG / guidance scale")
    parser.add_argument("--width", type=int, default=512, help="Image width")
    parser.add_argument("--height", type=int, default=512, help="Image height")
    parser.add_argument("--seed", type=int, default=42, help="Deterministic seed")
    parser.add_argument("--device", default="auto", help="Runtime device (default: auto)")
    parser.add_argument(
        "--torch-dtype",
        default=None,
        help="Optional torch dtype override (for example: float16, bfloat16, float32)",
    )
    parser.add_argument(
        "--output",
        default="",
        help="Optional PNG output path for the first generated image",
    )
    return parser


def main() -> int:
    args = build_parser().parse_args()
    model_path = Path(args.model_path).expanduser().resolve()
    if not model_path.exists():
        raise FileNotFoundError(f"Model path does not exist: {model_path}")

    worker = load_worker()
    worker.load_diffusion_model(
        model_path=str(model_path),
        device=args.device,
        torch_dtype=args.torch_dtype,
    )
    result = worker.generate_image(
        prompt=args.prompt,
        negative_prompt=args.negative_prompt or None,
        width=args.width,
        height=args.height,
        num_inference_steps=args.steps,
        guidance_scale=args.guidance_scale,
        seed=args.seed,
    )

    image_base64 = result.get("image_base64")
    if not isinstance(image_base64, str) or not image_base64:
        raise RuntimeError("Diffusion smoke test produced no primary image")

    if args.output:
        write_image(Path(args.output).expanduser(), image_base64)

    summary = {
        "model_path": str(model_path),
        "prompt": args.prompt,
        "steps": args.steps,
        "guidance_scale": args.guidance_scale,
        "seed_used": result.get("seed_used"),
        "width": result.get("width"),
        "height": result.get("height"),
        "mime_type": result.get("mime_type"),
        "images": len(result.get("images", [])),
        "output": str(Path(args.output).expanduser()) if args.output else None,
    }
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # pragma: no cover - CLI failure path
        print(f"[diffusion-smoketest] {exc}", file=sys.stderr)
        traceback.print_exc()
        raise SystemExit(1)
