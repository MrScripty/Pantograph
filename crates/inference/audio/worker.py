"""Audio generation worker for Pantograph.

Embedded in the Rust process via PyO3. Provides model loading and
audio generation for Stable Audio Open models.

All public functions are called from Rust through PyO3's Python::with_gil.
Module-level globals hold the loaded model state.

Generation logic is in the sibling module:
  - stable_audio: Stable Audio model loading and generation
"""

import logging
import sys
from pathlib import Path

import torch

# When loaded from the filesystem, ensure sibling modules are importable.
_self_path = Path(__file__).resolve()
if _self_path.parent.is_dir():
    _audio_dir = str(_self_path.parent)
    if _audio_dir not in sys.path:
        sys.path.insert(0, _audio_dir)

from stable_audio import load_stable_audio, generate_audio

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("pantograph.audio")

# Module-level state — one audio model loaded at a time
_model = None
_model_config = None
_device = None
_model_path = None


def is_loaded():
    """Check whether an audio model is currently loaded."""
    return _model is not None


def get_loaded_info():
    """Return metadata about the currently loaded audio model, or None."""
    if _model is None:
        return None
    return {
        "model_path": _model_path,
        "device": _device,
        "sample_rate": _model_config.get("sample_rate", 44100) if _model_config else None,
    }


def load_model(model_path, device="auto"):
    """Load a Stable Audio model.

    Args:
        model_path: Path to the model directory.
        device: Target device ("auto", "cuda", "mps", "cpu").

    Returns:
        Dict with model_path, device, sample_rate.
    """
    global _model, _model_config, _device, _model_path

    # Unload previous model if different path
    if _model is not None and _model_path != model_path:
        unload_model()

    if _model is not None:
        logger.info("Audio model already loaded from %s", _model_path)
        return get_loaded_info()

    _model, _model_config = load_stable_audio(model_path, device)

    if device == "auto":
        if torch.cuda.is_available():
            _device = "cuda"
        elif hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
            _device = "mps"
        else:
            _device = "cpu"
    else:
        _device = device

    _model_path = model_path
    return get_loaded_info()


def unload_model():
    """Unload the current audio model and free resources."""
    global _model, _model_config, _device, _model_path

    if _model is not None:
        logger.info("Unloading audio model from %s", _model_path)
        del _model
        _model = None
        _model_config = None
        _device = None
        _model_path = None

        if torch.cuda.is_available():
            torch.cuda.empty_cache()


def generate_audio_from_text(
    prompt,
    duration=30.0,
    steps=100,
    guidance_scale=7.0,
    seed=-1,
):
    """Generate audio from a text prompt using the loaded model.

    Args:
        prompt: Text prompt describing the audio to generate.
        duration: Duration of generated audio in seconds (1-47).
        steps: Number of diffusion inference steps (10-500).
        guidance_scale: Classifier-free guidance scale (1-20).
        seed: Random seed (-1 for random).

    Returns:
        Dict with "audio_base64" (WAV), "duration_seconds", "sample_rate".

    Raises:
        RuntimeError: If no model is loaded.
    """
    if _model is None:
        raise RuntimeError("No audio model loaded. Call load_model() first.")

    return generate_audio(
        _model,
        _model_config,
        _device,
        prompt=prompt,
        duration=duration,
        steps=steps,
        guidance_scale=guidance_scale,
        seed=seed,
    )
