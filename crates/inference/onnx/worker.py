"""ONNX text-to-audio worker for Pantograph.

This worker focuses on KittenTTS-compatible ONNX bundles:
- model ONNX file (e.g., *.onnx)
- voices file (voices.npz)

It is executed by the python runtime sidecar bridge and returns base64 WAV.
"""

from __future__ import annotations

import base64
import io
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, List, Tuple

import numpy as np
import soundfile as sf

_loaded = None
_loaded_key = None

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


def _ensure_prompt(inputs: Dict[str, Any]) -> str:
    prompt = inputs.get("prompt")
    if isinstance(prompt, str) and prompt.strip():
        return prompt.strip()
    raise RuntimeError("Missing prompt input")


def _resolve_bundle_paths(model_path: str) -> Tuple[Path, Path]:
    base = Path(model_path).expanduser().resolve()
    if base.is_file():
        if base.suffix.lower() != ".onnx":
            raise RuntimeError(f"Expected ONNX model file, got: {base}")
        model_file = base
        voices_file = base.with_name("voices.npz")
    else:
        if not base.exists():
            raise RuntimeError(f"Model path does not exist: {base}")
        model_candidates = sorted(base.glob("*.onnx"))
        if not model_candidates:
            raise RuntimeError(f"No .onnx file found in model directory: {base}")
        model_file = model_candidates[0]
        voices_file = base / "voices.npz"

    if not voices_file.exists():
        raise RuntimeError(
            f"voices.npz not found for ONNX model bundle (expected at {voices_file})"
        )

    return model_file, voices_file


def _to_wav_base64(audio: np.ndarray, sample_rate: int) -> Tuple[str, float]:
    arr = np.asarray(audio, dtype=np.float32)
    if arr.ndim > 1:
        arr = np.squeeze(arr, axis=0)

    buffer = io.BytesIO()
    sf.write(buffer, arr, sample_rate, format="WAV")
    duration = float(arr.shape[-1]) / float(sample_rate) if sample_rate > 0 else 0.0
    encoded = base64.b64encode(buffer.getvalue()).decode("ascii")
    return encoded, duration


def _runtime_hint_for_exception(exc: BaseException) -> str:
    lowered = str(exc).lower()
    hints: List[str] = []

    if "phonemizer" in lowered or "espeak" in lowered:
        hints.append(
            "Phonemizer runtime appears unavailable. Install an OS-level "
            "espeak/espeak-ng package and ensure it is discoverable from the "
            "selected Python dependency environment."
        )
    if "onnxruntime" in lowered or "onnx runtime" in lowered:
        hints.append(
            "ONNX Runtime load failed. Verify the dependency environment "
            "contains a compatible `onnxruntime` wheel for this platform."
        )

    return " ".join(hints)


def _load_model(model_path: str):
    global _loaded, _loaded_key

    model_file, voices_file = _resolve_bundle_paths(model_path)
    key = f"{model_file}:{voices_file}"
    if _loaded is not None and _loaded_key == key:
        return _loaded

    try:
        from kittentts.onnx_model import KittenTTS_1_Onnx
    except Exception as exc:
        hint = _runtime_hint_for_exception(exc)
        raise RuntimeError(
            "Failed to import KittenTTS ONNX runtime. Ensure the dependency environment "
            "includes `kittentts`, `onnxruntime`, `numpy`, `phonemizer`, and `soundfile`."
            + (f" {hint}" if hint else "")
        ) from exc

    try:
        _loaded = KittenTTS_1_Onnx(
            model_path=str(model_file),
            voices_path=str(voices_file),
        )
    except Exception as exc:
        hint = _runtime_hint_for_exception(exc)
        raise RuntimeError(
            "Failed to initialize KittenTTS ONNX model bundle."
            + (f" {hint}" if hint else "")
            + f" Original error: {exc}"
        ) from exc
    _loaded_key = key
    return _loaded


def _iter_chunks(
    tts_model,
    text: str,
    voice: str,
    speed: float,
    clean_text: bool,
) -> Iterable[np.ndarray]:
    # Stream-like chunking when the internal helpers are available.
    if hasattr(tts_model, "preprocessor") and clean_text:
        text = tts_model.preprocessor(text)

    chunk_text = None
    try:
        from kittentts import onnx_model as onnx_model_mod

        chunk_text = getattr(onnx_model_mod, "chunk_text", None)
    except Exception:
        chunk_text = None

    if callable(chunk_text) and hasattr(tts_model, "generate_single_chunk"):
        for chunk in chunk_text(text):
            yield tts_model.generate_single_chunk(chunk, voice=voice, speed=speed)
        return

    # Fallback to single batch generation.
    yield tts_model.generate(text, voice=voice, speed=speed, clean_text=clean_text)


def _available_voice_names(tts_model: Any) -> List[str]:
    available = getattr(tts_model, "available_voices", None)
    if isinstance(available, (list, tuple)):
        names = [str(name) for name in available if str(name).strip()]
        if names:
            return names

    voices = getattr(tts_model, "voices", None)
    if isinstance(voices, dict):
        names = [str(name) for name in voices.keys() if str(name).strip()]
        if names:
            return names

    return []


def _resolve_voice(requested_voice: str, available_voices: List[str]) -> str:
    if not available_voices:
        return requested_voice

    requested = requested_voice.strip()
    if requested in available_voices:
        return requested

    return requested


def generate_audio(
    inputs: Dict[str, Any],
    emit_stream: Callable[[Dict[str, Any]], None] | None = None,
) -> Dict[str, Any]:
    model_path = inputs.get("model_path")
    if not isinstance(model_path, str) or not model_path.strip():
        raise RuntimeError("Missing model_path input. Connect a Puma-Lib node.")
    model_path = model_path.strip()

    prompt = _ensure_prompt(inputs)
    voice = str(inputs.get("voice", "expr-voice-5-m") or "expr-voice-5-m")
    speed = _as_float(inputs.get("speed", 1.0), 1.0)
    sample_rate = _as_int(inputs.get("sample_rate", 24000), 24000)
    clean_text = _as_bool(inputs.get("clean_text", True), True)

    tts = _load_model(model_path)
    available_voices = _available_voice_names(tts)
    voice = _resolve_voice(voice, available_voices)

    # Generate per-text-chunk so callers can surface progress if desired.
    chunk_audio: List[np.ndarray] = []
    chunk_payloads: List[Dict[str, Any]] = []
    pending_stream_chunk: Dict[str, Any] | None = None
    try:
        for sequence, chunk_arr in enumerate(_iter_chunks(tts, prompt, voice, speed, clean_text)):
            arr = np.asarray(chunk_arr, dtype=np.float32)
            chunk_audio.append(arr)
            chunk_b64, chunk_duration = _to_wav_base64(arr, sample_rate)
            chunk_payload = {
                "type": "audio_chunk",
                "mode": "append",
                "audio_base64": chunk_b64,
                "duration_seconds": chunk_duration,
                "sample_rate": sample_rate,
                "mime_type": "audio/wav",
                "sequence": sequence,
                "is_final": False,
            }
            chunk_payloads.append(chunk_payload)
            if callable(emit_stream):
                try:
                    # Delay emission by one chunk so we can mark terminal chunk deterministically.
                    if pending_stream_chunk is not None:
                        emit_stream(pending_stream_chunk)
                    pending_stream_chunk = chunk_payload
                except Exception:
                    # Streaming callbacks are best-effort and must not break inference.
                    pass
    except Exception as exc:
        hint = _runtime_hint_for_exception(exc)
        raise RuntimeError(
            f"KittenTTS ONNX generation failed: {exc}"
            + (f" {hint}" if hint else "")
        ) from exc

    if not chunk_audio:
        raise RuntimeError("ONNX worker generated no audio chunks")

    chunk_payloads[-1]["is_final"] = True
    if callable(emit_stream):
        try:
            emit_stream(chunk_payloads[-1])
        except Exception:
            # Streaming callbacks are best-effort and must not break inference.
            pass

    if len(chunk_audio) == 1:
        full_audio = chunk_audio[0]
    else:
        full_audio = np.concatenate(chunk_audio, axis=-1)

    full_b64, duration_seconds = _to_wav_base64(full_audio, sample_rate)
    return {
        "audio": full_b64,
        "duration_seconds": duration_seconds,
        "sample_rate": sample_rate,
        "stream": chunk_payloads,
        "voice_used": voice,
        "available_voices": available_voices,
        "speed_used": speed,
    }
