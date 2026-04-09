#!/usr/bin/env python3

"""Process bridge for python-backed Pantograph workflow nodes.

Reads a JSON request from stdin, executes the requested node by loading
Pantograph worker modules (torch/diffusion/audio/onnx) from explicit file
paths, and writes a JSON response to stdout.
"""

from __future__ import annotations

import importlib.util
import json
import os
import pathlib
import traceback
from typing import Any, Callable, Dict


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


def _resolve_setting_runtime_value(item: Dict[str, Any], value: Any) -> Any:
    if isinstance(value, dict) and "label" in value and "value" in value:
        return value.get("value")
    if isinstance(value, str):
        constraints = item.get("constraints")
        if isinstance(constraints, dict):
            allowed_values = constraints.get("allowed_values")
            if isinstance(allowed_values, list):
                for option in allowed_values:
                    if not isinstance(option, dict):
                        continue
                    option_label = (
                        option.get("label")
                        or option.get("name")
                        or option.get("key")
                    )
                    if option_label == value and "value" in option:
                        return option.get("value")
    return value


def _build_extra_settings(inputs: Dict[str, Any]) -> Dict[str, Any]:
    out: Dict[str, Any] = {}
    schema = _input_or_data(inputs, "inference_settings")
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
        value = _input_or_data(inputs, key)
        if value is None:
            value = item.get("default")
        value = _resolve_setting_runtime_value(item, value)
        if value is not None:
            out[key] = value
    return out


def _extract_prompt(inputs: Dict[str, Any]) -> str:
    prompt_value = inputs.get("prompt")
    if isinstance(prompt_value, str) and prompt_value.strip():
        return prompt_value
    if isinstance(prompt_value, dict) and prompt_value.get("type") == "masked_prompt":
        segments = prompt_value.get("segments")
        if not isinstance(segments, list):
            segments = []
        prompt = "".join(
            segment.get("text", "")
            for segment in segments
            if isinstance(segment, dict)
        )
        if prompt.strip():
            return prompt
    raise RuntimeError("Missing prompt input: expected string or masked_prompt object")


def _coalesce_setting(inputs: Dict[str, Any], extra: Dict[str, Any], *keys: str) -> Any:
    for key in keys:
        value = _input_or_data(inputs, key)
        if value is not None:
            return value
        if key in extra and extra.get(key) is not None:
            return extra.get(key)
    return None


def _input_or_data(inputs: Dict[str, Any], key: str) -> Any:
    if key in inputs and inputs.get(key) is not None:
        return inputs.get(key)
    data = inputs.get("_data")
    if isinstance(data, dict):
        return data.get(key)
    return None


def _as_bool(value: Any, default: bool) -> bool:
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        normalized = value.strip().lower()
        if normalized in {"1", "true", "yes", "on"}:
            return True
        if normalized in {"0", "false", "no", "off"}:
            return False
    return default


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

    task_type_primary = _input_or_data(inputs, "task_type_primary")
    if not isinstance(task_type_primary, str) or not task_type_primary.strip():
        task_type_primary = "text-generation"
    task_type_primary = task_type_primary.strip()

    if task_type_primary == "audio-to-text":
        model_path = inputs.get("model_path")
        if not isinstance(model_path, str) or not model_path.strip():
            raise RuntimeError("Missing model_path input. Connect a Puma-Lib node.")
        model_path = model_path.strip()

        audio = inputs.get("audio")
        if not isinstance(audio, str) or not audio.strip():
            raise RuntimeError("Missing audio input. Connect an audio-input node.")

        kwargs: Dict[str, Any] = {"audio_base64": audio.strip()}
        language = inputs.get("language")
        if isinstance(language, str) and language.strip():
            kwargs["language"] = language.strip()
        prompt = inputs.get("prompt")
        if isinstance(prompt, str) and prompt.strip():
            kwargs["prompt"] = prompt.strip()

        extra = _build_extra_settings(inputs)
        if "language" in extra and "language" not in kwargs:
            kwargs["language"] = extra["language"]
        if "task" in extra:
            kwargs["task"] = extra["task"]
        if "chunk_length_s" in extra:
            kwargs["chunk_length_s"] = extra["chunk_length_s"]

        result = worker.transcribe_audio(
            model_path=model_path,
            device=str(inputs.get("device", "auto") or "auto"),
            **kwargs,
        )
        if not isinstance(result, dict):
            raise RuntimeError("PyTorch ASR worker returned unexpected payload shape")

        outputs: Dict[str, Any] = {
            "response": result.get("text", ""),
            "language": result.get("language"),
            "duration_seconds": result.get("duration_seconds"),
        }
        outputs["model_ref"] = _input_model_ref(inputs) or _fallback_model_ref(
            "pytorch", model_path, task_type_primary
        )
        return outputs

    prompt_value = inputs.get("prompt")
    masked_prompt_json = None
    prompt = _extract_prompt(inputs)
    if isinstance(prompt_value, dict) and prompt_value.get("type") == "masked_prompt":
        masked_prompt_json = json.dumps(prompt_value)

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

    outputs: Dict[str, Any] = {"response": response_text}
    outputs["model_ref"] = _input_model_ref(inputs) or _fallback_model_ref(
        "pytorch", model_path, task_type_primary
    )
    return outputs


def _run_diffusion(inputs: Dict[str, Any], torch_worker_path: str) -> Dict[str, Any]:
    worker = _load_module("pantograph_torch_worker_process", torch_worker_path)

    prompt = _extract_prompt(inputs)

    model_path = inputs.get("model_path")
    if not isinstance(model_path, str) or not model_path.strip():
        raise RuntimeError("Missing model_path input. Connect a Puma-Lib node.")
    model_path = model_path.strip()

    extra_settings = _build_extra_settings(inputs)

    model_info = worker.get_loaded_diffusion_info()
    loaded_path = ""
    if isinstance(model_info, dict):
        loaded_path = str(model_info.get("model_path", ""))
    if loaded_path != model_path:
        worker.load_diffusion_model(
            model_path=model_path,
            device=str(inputs.get("device", "auto") or "auto"),
            torch_dtype=_coalesce_setting(inputs, extra_settings, "torch_dtype", "dtype"),
            enable_attention_slicing=_as_bool(
                _coalesce_setting(inputs, extra_settings, "enable_attention_slicing"),
                False,
            ),
            enable_vae_slicing=_as_bool(
                _coalesce_setting(inputs, extra_settings, "enable_vae_slicing"),
                False,
            ),
            enable_vae_tiling=_as_bool(
                _coalesce_setting(inputs, extra_settings, "enable_vae_tiling"),
                False,
            ),
            model_cpu_offload=_as_bool(
                _coalesce_setting(inputs, extra_settings, "model_cpu_offload"),
                False,
            ),
            sequential_cpu_offload=_as_bool(
                _coalesce_setting(inputs, extra_settings, "sequential_cpu_offload"),
                False,
            ),
        )

    generation_kwargs: Dict[str, Any] = {
        "prompt": prompt,
        "negative_prompt": _coalesce_setting(inputs, extra_settings, "negative_prompt"),
        "width": _coalesce_setting(inputs, extra_settings, "width"),
        "height": _coalesce_setting(inputs, extra_settings, "height"),
        "num_inference_steps": _coalesce_setting(
            inputs, extra_settings, "steps", "num_inference_steps"
        ),
        "guidance_scale": _coalesce_setting(
            inputs, extra_settings, "cfg_scale", "guidance_scale"
        ),
        "seed": _coalesce_setting(inputs, extra_settings, "seed"),
        "scheduler": _coalesce_setting(inputs, extra_settings, "scheduler"),
        "num_images_per_prompt": _coalesce_setting(
            inputs, extra_settings, "num_images_per_prompt"
        ),
        "init_image": _coalesce_setting(inputs, extra_settings, "init_image"),
        "mask_image": _coalesce_setting(inputs, extra_settings, "mask_image"),
        "strength": _coalesce_setting(inputs, extra_settings, "strength"),
    }

    reserved_keys = {
        "steps",
        "num_inference_steps",
        "cfg_scale",
        "guidance_scale",
        "seed",
        "width",
        "height",
        "negative_prompt",
        "scheduler",
        "num_images_per_prompt",
        "init_image",
        "mask_image",
        "strength",
        "torch_dtype",
        "dtype",
        "enable_attention_slicing",
        "enable_vae_slicing",
        "enable_vae_tiling",
        "model_cpu_offload",
        "sequential_cpu_offload",
    }
    for key, value in extra_settings.items():
        if key not in reserved_keys and value is not None:
            generation_kwargs[key] = value

    result = worker.generate_image(**generation_kwargs)
    if not isinstance(result, dict):
        raise RuntimeError("Diffusion worker returned unexpected payload shape")

    task_type_primary = inputs.get("task_type_primary")
    if not isinstance(task_type_primary, str) or not task_type_primary.strip():
        task_type_primary = "text-to-image"

    outputs: Dict[str, Any] = {
        "image": result.get("image_base64", ""),
        "images": result.get("images", []),
        "mime_type": result.get("mime_type", "image/png"),
        "seed_used": result.get("seed_used"),
        "width": result.get("width"),
        "height": result.get("height"),
    }
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


def _run_onnx(
    inputs: Dict[str, Any],
    onnx_worker_path: str,
    emit_stream: Callable[[Dict[str, Any]], None] | None = None,
) -> Dict[str, Any]:
    worker = _load_module("pantograph_onnx_worker_process", onnx_worker_path)

    model_path = inputs.get("model_path")
    if not isinstance(model_path, str) or not model_path.strip():
        raise RuntimeError("Missing model_path input. Connect a Puma-Lib node.")
    model_path = model_path.strip()

    if not isinstance(inputs.get("prompt"), str) or not str(inputs.get("prompt")).strip():
        raise RuntimeError("Missing prompt input")

    result = worker.generate_audio(inputs, emit_stream=emit_stream)
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
        "onnx-runtime", model_path, "text-to-audio"
    )
    return outputs


def _ensure_worker_path(path: Any, label: str) -> str:
    if not isinstance(path, str) or not path.strip():
        raise RuntimeError(f"Missing worker path for {label}")
    resolved = pathlib.Path(path).expanduser().resolve()
    if not resolved.exists():
        raise RuntimeError(f"Worker path for {label} does not exist: {resolved}")
    return str(resolved)


def _emit_stream_event(port: str, chunk: Dict[str, Any]) -> None:
    print(
        json.dumps(
            {"event": "stream", "port": port, "chunk": chunk},
            separators=(",", ":"),
        ),
        flush=True,
    )


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
        elif node_type == "diffusion-inference":
            outputs = _run_diffusion(inputs, torch_worker)
        elif node_type == "audio-generation":
            outputs = _run_audio(inputs, audio_worker)
        elif node_type == "onnx-inference":
            outputs = _run_onnx(
                inputs,
                onnx_worker,
                emit_stream=lambda chunk: _emit_stream_event("stream", chunk),
            )
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
