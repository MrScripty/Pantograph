"""Worker envelope validation helpers that do not import torch."""

import json

WORKER_CONTRACT_VERSION = 1
LOAD_TRANSFORMERS_MODEL_OPERATION = "load_transformers_model"
UNLOAD_MODEL_OPERATION = "unload_model"
GET_LOADED_INFO_OPERATION = "get_loaded_info"
GENERATE_TEXT_OPERATION = "generate_text"
GENERATE_TEXT_STREAM_OPERATION = "generate_text_stream"
TRANSCRIBE_AUDIO_OPERATION = "transcribe_audio"
CLEAR_KV_CACHE_OPERATION = "clear_kv_cache"
SAVE_KV_CACHE_OPERATION = "save_kv_cache"
RESTORE_KV_CACHE_OPERATION = "restore_kv_cache"
ALLOWED_TRANSFORMERS_GENERATE_KWARGS = {"top_k"}
CAUSAL_LM_LOADER = "causal_lm"
AUTOMATIC_SPEECH_RECOGNITION_LOADER = "automatic_speech_recognition"
SUPPORTED_TRANSFORMERS_LOADERS = {
    CAUSAL_LM_LOADER,
    AUTOMATIC_SPEECH_RECOGNITION_LOADER,
}


def load_transformers_model_kwargs_from_envelope(envelope):
    """Validate a Rust worker envelope and project it to load_model kwargs."""
    if isinstance(envelope, str):
        envelope = json.loads(envelope)
    if not isinstance(envelope, dict):
        raise ValueError("PyTorch worker load envelope must be an object")
    contract_version = envelope.get("contract_version")
    if contract_version != WORKER_CONTRACT_VERSION:
        raise ValueError(f"Unsupported PyTorch worker contract_version: {contract_version}")
    operation = envelope.get("operation")
    if operation != LOAD_TRANSFORMERS_MODEL_OPERATION:
        raise ValueError(f"Unexpected PyTorch worker operation for load: {operation}")

    payload = envelope.get("payload")
    if not isinstance(payload, dict):
        raise ValueError("PyTorch worker load envelope payload must be an object")

    trust_policy = payload.get("trust_policy")
    if trust_policy is None:
        trust_policy = {}
    if not isinstance(trust_policy, dict):
        raise ValueError("PyTorch worker load trust_policy must be an object")

    task_profile = payload.get("task_profile")
    if task_profile is None:
        task_profile = {}
    if not isinstance(task_profile, dict):
        raise ValueError("PyTorch worker load task_profile must be an object")
    loader = task_profile.get("loader") or CAUSAL_LM_LOADER
    if loader not in SUPPORTED_TRANSFORMERS_LOADERS:
        raise ValueError(f"Unsupported PyTorch worker Transformers loader: {loader}")

    return {
        "model_path": payload.get("entry_path"),
        "device": payload.get("device") or "auto",
        "model_type": payload.get("model_type_hint"),
        "loader": loader,
        "trust_remote_code": bool(trust_policy.get("allow_remote_code", False)),
        "trust_policy_decision_id": trust_policy.get("decision_id"),
        "local_files_only": bool(trust_policy.get("local_files_only", True)),
        "revision": trust_policy.get("revision"),
        "code_revision": trust_policy.get("code_revision"),
        "cache_policy": trust_policy.get("cache_policy", "backend_default"),
    }


def generate_text_kwargs_from_envelope(envelope, expected_operation=GENERATE_TEXT_OPERATION):
    """Validate a Rust worker envelope and project it to generate kwargs."""
    if isinstance(envelope, str):
        envelope = json.loads(envelope)
    if not isinstance(envelope, dict):
        raise ValueError("PyTorch worker generate_text envelope must be an object")
    contract_version = envelope.get("contract_version")
    if contract_version != WORKER_CONTRACT_VERSION:
        raise ValueError(f"Unsupported PyTorch worker contract_version: {contract_version}")
    operation = envelope.get("operation")
    if operation != expected_operation:
        raise ValueError(
            f"Unexpected PyTorch worker operation for {expected_operation}: {operation}"
        )

    payload = envelope.get("payload")
    if not isinstance(payload, dict):
        raise ValueError("PyTorch worker generate_text envelope payload must be an object")
    prompt = payload.get("prompt")
    if not isinstance(prompt, str) or not prompt.strip():
        raise ValueError("PyTorch worker generate_text payload.prompt must be a non-empty string")

    transformers_kwargs = payload.get("transformers_kwargs") or {}
    if not isinstance(transformers_kwargs, dict):
        raise ValueError("PyTorch worker generate_text transformers_kwargs must be an object")
    unsupported_kwargs = sorted(
        key for key in transformers_kwargs if key not in ALLOWED_TRANSFORMERS_GENERATE_KWARGS
    )
    if unsupported_kwargs:
        joined = ", ".join(unsupported_kwargs)
        raise ValueError(
            f"PyTorch worker generate_text transformers_kwargs contains unsupported key(s): {joined}"
        )

    kwargs = {
        "prompt": prompt,
        "system_prompt": payload.get("system_prompt"),
        "max_tokens": int(payload.get("max_tokens", 512)),
        "temperature": float(payload.get("temperature", 0.7)),
        "top_p": float(payload.get("top_p", 1.0)),
        "masked_prompt_json": payload.get("masked_prompt_json"),
        "denoising_steps": payload.get("denoising_steps"),
        "block_length": payload.get("block_length"),
    }
    for key, value in transformers_kwargs.items():
        if value is not None:
            kwargs[key] = value
    return kwargs


def unload_model_kwargs_from_envelope(envelope):
    """Validate a Rust worker envelope and project it to unload kwargs."""
    if isinstance(envelope, str):
        envelope = json.loads(envelope)
    if not isinstance(envelope, dict):
        raise ValueError("PyTorch worker unload envelope must be an object")
    contract_version = envelope.get("contract_version")
    if contract_version != WORKER_CONTRACT_VERSION:
        raise ValueError(f"Unsupported PyTorch worker contract_version: {contract_version}")
    operation = envelope.get("operation")
    if operation != UNLOAD_MODEL_OPERATION:
        raise ValueError(f"Unexpected PyTorch worker operation for unload: {operation}")

    payload = envelope.get("payload")
    if not isinstance(payload, dict):
        raise ValueError("PyTorch worker unload envelope payload must be an object")
    return {}


def get_loaded_info_kwargs_from_envelope(envelope):
    """Validate a Rust worker envelope and project it to loaded-info kwargs."""
    if isinstance(envelope, str):
        envelope = json.loads(envelope)
    if not isinstance(envelope, dict):
        raise ValueError("PyTorch worker get_loaded_info envelope must be an object")
    contract_version = envelope.get("contract_version")
    if contract_version != WORKER_CONTRACT_VERSION:
        raise ValueError(f"Unsupported PyTorch worker contract_version: {contract_version}")
    operation = envelope.get("operation")
    if operation != GET_LOADED_INFO_OPERATION:
        raise ValueError(f"Unexpected PyTorch worker operation for get_loaded_info: {operation}")

    payload = envelope.get("payload")
    if not isinstance(payload, dict):
        raise ValueError("PyTorch worker get_loaded_info envelope payload must be an object")
    return {}


def clear_kv_cache_kwargs_from_envelope(envelope):
    """Validate a Rust worker envelope and project it to KV clear kwargs."""
    if isinstance(envelope, str):
        envelope = json.loads(envelope)
    if not isinstance(envelope, dict):
        raise ValueError("PyTorch worker clear_kv_cache envelope must be an object")
    contract_version = envelope.get("contract_version")
    if contract_version != WORKER_CONTRACT_VERSION:
        raise ValueError(f"Unsupported PyTorch worker contract_version: {contract_version}")
    operation = envelope.get("operation")
    if operation != CLEAR_KV_CACHE_OPERATION:
        raise ValueError(f"Unexpected PyTorch worker operation for clear_kv_cache: {operation}")

    payload = envelope.get("payload")
    if not isinstance(payload, dict):
        raise ValueError("PyTorch worker clear_kv_cache envelope payload must be an object")
    return {}


def save_kv_cache_kwargs_from_envelope(envelope):
    """Validate a Rust worker envelope and project it to KV save kwargs."""
    if isinstance(envelope, str):
        envelope = json.loads(envelope)
    if not isinstance(envelope, dict):
        raise ValueError("PyTorch worker save_kv_cache envelope must be an object")
    contract_version = envelope.get("contract_version")
    if contract_version != WORKER_CONTRACT_VERSION:
        raise ValueError(f"Unsupported PyTorch worker contract_version: {contract_version}")
    operation = envelope.get("operation")
    if operation != SAVE_KV_CACHE_OPERATION:
        raise ValueError(f"Unexpected PyTorch worker operation for save_kv_cache: {operation}")

    payload = envelope.get("payload")
    if not isinstance(payload, dict):
        raise ValueError("PyTorch worker save_kv_cache envelope payload must be an object")
    path = payload.get("path")
    if not isinstance(path, str) or not path.strip():
        raise ValueError("PyTorch worker save_kv_cache payload.path must be a non-empty string")
    return {"path": path}


def restore_kv_cache_kwargs_from_envelope(envelope):
    """Validate a Rust worker envelope and project it to KV restore kwargs."""
    if isinstance(envelope, str):
        envelope = json.loads(envelope)
    if not isinstance(envelope, dict):
        raise ValueError("PyTorch worker restore_kv_cache envelope must be an object")
    contract_version = envelope.get("contract_version")
    if contract_version != WORKER_CONTRACT_VERSION:
        raise ValueError(f"Unsupported PyTorch worker contract_version: {contract_version}")
    operation = envelope.get("operation")
    if operation != RESTORE_KV_CACHE_OPERATION:
        raise ValueError(f"Unexpected PyTorch worker operation for restore_kv_cache: {operation}")

    payload = envelope.get("payload")
    if not isinstance(payload, dict):
        raise ValueError("PyTorch worker restore_kv_cache envelope payload must be an object")
    path = payload.get("path")
    if not isinstance(path, str) or not path.strip():
        raise ValueError("PyTorch worker restore_kv_cache payload.path must be a non-empty string")
    return {"path": path}


def transcribe_audio_kwargs_from_envelope(envelope):
    """Validate a Rust worker envelope and project it to audio transcription kwargs."""
    if isinstance(envelope, str):
        envelope = json.loads(envelope)
    if not isinstance(envelope, dict):
        raise ValueError("PyTorch worker audio_transcription envelope must be an object")
    contract_version = envelope.get("contract_version")
    if contract_version != WORKER_CONTRACT_VERSION:
        raise ValueError(f"Unsupported PyTorch worker contract_version: {contract_version}")
    operation = envelope.get("operation")
    if operation != TRANSCRIBE_AUDIO_OPERATION:
        raise ValueError(
            f"Unexpected PyTorch worker operation for audio transcription: {operation}"
        )

    payload = envelope.get("payload")
    if not isinstance(payload, dict):
        raise ValueError("PyTorch worker audio_transcription envelope payload must be an object")

    model_path = payload.get("model_path")
    if not isinstance(model_path, str) or not model_path.strip():
        raise ValueError(
            "PyTorch worker audio_transcription payload.model_path must be a non-empty string"
        )
    audio_base64 = payload.get("audio_base64")
    if not isinstance(audio_base64, str) or not audio_base64.strip():
        raise ValueError(
            "PyTorch worker audio_transcription payload.audio_base64 must be a non-empty string"
        )

    device = payload.get("device") or "auto"
    if not isinstance(device, str) or not device.strip():
        raise ValueError(
            "PyTorch worker audio_transcription payload.device must be a non-empty string"
        )

    extra_options = payload.get("extra_options")
    if extra_options is not None:
        raise ValueError("PyTorch worker audio_transcription extra_options are not supported yet")

    kwargs = {
        "model_path": model_path,
        "audio_base64": audio_base64.strip(),
        "device": device,
        "language": payload.get("language"),
        "prompt": payload.get("prompt"),
        "task": payload.get("task"),
        "chunk_length_s": payload.get("chunk_length_s"),
    }

    for optional_string in ("language", "prompt", "task"):
        value = kwargs[optional_string]
        if value is not None and not isinstance(value, str):
            raise ValueError(
                f"PyTorch worker audio_transcription payload.{optional_string} must be a string when present"
            )
    chunk_length_s = kwargs["chunk_length_s"]
    if chunk_length_s is not None:
        try:
            kwargs["chunk_length_s"] = float(chunk_length_s)
        except (TypeError, ValueError) as exc:
            raise ValueError(
                "PyTorch worker audio_transcription payload.chunk_length_s must be a number when present"
            ) from exc

    return kwargs
