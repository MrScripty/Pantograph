"""Worker envelope validation helpers that do not import torch."""

import json

WORKER_CONTRACT_VERSION = 1
LOAD_TRANSFORMERS_MODEL_OPERATION = "load_transformers_model"
GENERATE_TEXT_OPERATION = "generate_text"
GENERATE_TEXT_STREAM_OPERATION = "generate_text_stream"
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
    }
    for key, value in transformers_kwargs.items():
        if value is not None:
            kwargs[key] = value
    return kwargs
