#!/usr/bin/env python3

"""Process bridge for python-backed Pantograph workflow nodes.

Reads a JSON request from stdin, executes the requested node by loading
Pantograph worker modules (torch/audio/onnx) from explicit file paths, and
writes a JSON response to stdout.
"""

from __future__ import annotations

import importlib.util
import json
import os
import pathlib
import traceback
from typing import Any, Dict


def _load_module(module_name: str, module_path: str):
    spec = importlib.util.spec_from_file_location(module_name, module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load module spec for {module_name} at {module_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _as_float(value: Any, default: float) -> float:
    try:
        return float(value)
    except Exception:
        return default


def _as_int(value: Any, default: int) -> int:
    try:
        return int(value)
    except Exception:
        return default


def _build_extra_settings(inputs: Dict[str, Any]) -> Dict[str, Any]:
    out: Dict[str, Any] = {}
    schema = inputs.get("inference_settings")
    if not isinstance(schema, list):
        return out

    for item in schema:
        if not isinstance(item, dict):
            continue
        key = item.get("key")
        if not isinstance(key, str):
            continue
        key = key.strip()
        if not key:
            continue
        value = inputs.get(key, item.get("default"))
        if value is not None:
            out[key] = value
    return out


def _input_model_ref(inputs: Dict[str, Any]) -> Dict[str, Any] | None:
    model_ref = inputs.get("model_ref")
    if isinstance(model_ref, dict):
        return model_ref
    return None


def _fallback_model_ref(engine: str, model_path: str, task_type_primary: str) -> Dict[str, Any]:
    model_name = os.path.basename(os.path.normpath(model_path)) or f"{engine}-model"
    return {
        "contractVersion": 2,
        "engine": engine,
        "modelId": model_name,
        "modelPath": model_path,
        "taskTypePrimary": task_type_primary,
    }


def _run_pytorch(inputs: Dict[str, Any], torch_worker_path: str) -> Dict[str, Any]:
    worker = _load_module("pantograph_torch_worker_process", torch_worker_path)

    prompt_value = inputs.get("prompt")
    masked_prompt_json = None
    if isinstance(prompt_value, str):
        prompt = prompt_value
    elif isinstance(prompt_value, dict) and prompt_value.get("type") == "masked_prompt":
        segments = prompt_value.get("segments")
        if not isinstance(segments, list):
            segments = []
        prompt = "".join(
            segment.get("text", "")
            for segment in segments
            if isinstance(segment, dict)
        )
        masked_prompt_json = json.dumps(prompt_value)
    else:
        raise RuntimeError("Missing prompt input: expected string or masked_prompt object")

    model_path = inputs.get("model_path")
    if not isinstance(model_path, str) or not model_path.strip():
        raise RuntimeError("Missing model_path input. Connect a Puma-Lib node.")
    model_path = model_path.strip()

    model_info = worker.get_loaded_info()
    loaded_path = ""
    if isinstance(model_info, dict):
        loaded_path = str(model_info.get("model_path", ""))
    if loaded_path != model_path:
        load_kwargs: Dict[str, Any] = {
            "model_path": model_path,
            "device": str(inputs.get("device", "auto") or "auto"),
        }
        model_type = inputs.get("model_type")
        if isinstance(model_type, str) and model_type.strip():
            load_kwargs["model_type"] = model_type.strip()
        worker.load_model(**load_kwargs)

    max_tokens = _as_int(inputs.get("max_tokens", 512), 512)
    temperature = _as_float(inputs.get("temperature", 0.7), 0.7)
    top_p = _as_float(inputs.get("top_p", 0.95), 0.95)

    kwargs: Dict[str, Any] = {
        "prompt": prompt,
        "max_tokens": max_tokens,
        "temperature": temperature,
        "top_p": top_p,
    }
    system_prompt = inputs.get("system_prompt")
    if isinstance(system_prompt, str) and system_prompt.strip():
        kwargs["system_prompt"] = system_prompt
    if masked_prompt_json is not None:
        kwargs["masked_prompt_json"] = masked_prompt_json
    kwargs.update(_build_extra_settings(inputs))

    response = worker.generate(**kwargs)
    response_text = response if isinstance(response, str) else str(response)

    task_type_primary = inputs.get("task_type_primary")
    if not isinstance(task_type_primary, str) or not task_type_primary.strip():
        task_type_primary = "text-generation"

    outputs: Dict[str, Any] = {"response": response_text}
    outputs["model_ref"] = _input_model_ref(inputs) or _fallback_model_ref(
        "pytorch", model_path, task_type_primary
    )
    return outputs


def _run_audio(inputs: Dict[str, Any], audio_worker_path: str) -> Dict[str, Any]:
    worker = _load_module("pantograph_audio_worker_process", audio_worker_path)

    model_path = inputs.get("model_path")
    if not isinstance(model_path, str) or not model_path.strip():
        raise RuntimeError("Missing model_path input. Connect a Puma-Lib node.")
    model_path = model_path.strip()

    prompt = inputs.get("prompt")
    if not isinstance(prompt, str) or not prompt.strip():
        raise RuntimeError("Missing prompt input")

    model_info = worker.get_loaded_info()
    loaded_path = ""
    if isinstance(model_info, dict):
        loaded_path = str(model_info.get("model_path", ""))
    if loaded_path != model_path:
        worker.load_model(model_path=model_path, device="auto")

    result = worker.generate_audio_from_text(
        prompt=prompt,
        duration=_as_float(inputs.get("duration", 30.0), 30.0),
        steps=_as_int(inputs.get("num_inference_steps", 100), 100),
        guidance_scale=_as_float(inputs.get("guidance_scale", 7.0), 7.0),
        seed=_as_int(inputs.get("seed", -1), -1),
    )
    if not isinstance(result, dict):
        raise RuntimeError("Audio worker returned unexpected payload shape")

    outputs: Dict[str, Any] = {
        "audio": result.get("audio_base64", ""),
        "duration_seconds": result.get("duration_seconds", 0.0),
        "sample_rate": result.get("sample_rate", 44100),
    }
    outputs["model_ref"] = _input_model_ref(inputs) or _fallback_model_ref(
        "stable_audio", model_path, "text-to-audio"
    )
    return outputs


def _run_onnx(inputs: Dict[str, Any], onnx_worker_path: str) -> Dict[str, Any]:
    worker = _load_module("pantograph_onnx_worker_process", onnx_worker_path)

    model_path = inputs.get("model_path")
    if not isinstance(model_path, str) or not model_path.strip():
        raise RuntimeError("Missing model_path input. Connect a Puma-Lib node.")
    model_path = model_path.strip()

    if not isinstance(inputs.get("prompt"), str) or not str(inputs.get("prompt")).strip():
        raise RuntimeError("Missing prompt input")

    result = worker.generate_audio(inputs)
    if not isinstance(result, dict):
        raise RuntimeError("ONNX worker returned unexpected payload shape")

    outputs: Dict[str, Any] = {
        "audio": result.get("audio", ""),
        "duration_seconds": result.get("duration_seconds", 0.0),
        "sample_rate": result.get("sample_rate", 24000),
        "stream": result.get("stream", []),
        "voice_used": result.get("voice_used"),
        "speed_used": result.get("speed_used"),
    }
    outputs["model_ref"] = _input_model_ref(inputs) or _fallback_model_ref(
        "onnxruntime", model_path, "text-to-audio"
    )
    return outputs


def _ensure_worker_path(path: Any, label: str) -> str:
    if not isinstance(path, str) or not path.strip():
        raise RuntimeError(f"Missing worker path for {label}")
    resolved = pathlib.Path(path).expanduser().resolve()
    if not resolved.exists():
        raise RuntimeError(f"Worker path for {label} does not exist: {resolved}")
    return str(resolved)


def _main() -> int:
    raw = input_stream = ""
    try:
        import sys

        raw = sys.stdin.read()
        payload = json.loads(raw if raw else "{}")

        node_type = payload.get("node_type")
        if not isinstance(node_type, str) or not node_type.strip():
            raise RuntimeError("Missing node_type in python runtime bridge payload")
        node_type = node_type.strip()

        inputs = payload.get("inputs")
        if not isinstance(inputs, dict):
            inputs = {}

        worker_paths = payload.get("worker_paths")
        if not isinstance(worker_paths, dict):
            raise RuntimeError("Missing worker_paths in python runtime bridge payload")

        torch_worker = _ensure_worker_path(worker_paths.get("torch_worker"), "torch")
        audio_worker = _ensure_worker_path(worker_paths.get("audio_worker"), "audio")
        onnx_worker = _ensure_worker_path(worker_paths.get("onnx_worker"), "onnx")

        if node_type == "pytorch-inference":
            outputs = _run_pytorch(inputs, torch_worker)
        elif node_type == "audio-generation":
            outputs = _run_audio(inputs, audio_worker)
        elif node_type == "onnx-inference":
            outputs = _run_onnx(inputs, onnx_worker)
        else:
            raise RuntimeError(f"Unsupported python runtime node_type '{node_type}'")

        print(json.dumps({"ok": True, "outputs": outputs}, separators=(",", ":")))
        return 0
    except Exception as exc:
        trace = traceback.format_exc()
        print(
            json.dumps(
                {"ok": False, "error": str(exc), "traceback": trace},
                separators=(",", ":"),
            )
        )
        return 1


if __name__ == "__main__":
    raise SystemExit(_main())
