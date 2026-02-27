"""Stable Audio generation pipeline.

Sibling module for the audio worker. Handles model loading and audio
generation using Stability AI's stable-audio-tools library.

All public functions are called from the audio worker module.
"""

import base64
import io
import logging

import torch

logger = logging.getLogger("pantograph.audio.stable_audio")


def load_stable_audio(model_path, device="auto"):
    """Load a Stable Audio model from disk.

    Args:
        model_path: Path to the model directory containing model.ckpt
            and vae_model.ckpt.
        device: Target device ("auto", "cuda", "mps", "cpu").

    Returns:
        Tuple of (model, model_config) ready for generation.
    """
    from stable_audio_tools import get_pretrained_model
    from stable_audio_tools.models.utils import load_ckpt_state_dict

    if device == "auto":
        if torch.cuda.is_available():
            device = "cuda"
        elif hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
            device = "mps"
        else:
            device = "cpu"

    logger.info("Loading Stable Audio model from %s on %s", model_path, device)

    model, model_config = get_pretrained_model(model_path)
    model = model.to(device)
    model.eval()

    logger.info("Stable Audio model loaded successfully")
    return model, model_config


def generate_audio(
    model,
    model_config,
    device,
    prompt,
    duration=30.0,
    steps=100,
    guidance_scale=7.0,
    seed=-1,
):
    """Generate audio from a text prompt.

    Args:
        model: Loaded Stable Audio model.
        model_config: Model configuration dict.
        device: Device the model is on.
        prompt: Text prompt describing the audio to generate.
        duration: Duration of generated audio in seconds.
        steps: Number of diffusion inference steps.
        guidance_scale: Classifier-free guidance scale.
        seed: Random seed (-1 for random).

    Returns:
        Dict with "audio_base64" (WAV), "duration_seconds", "sample_rate".
    """
    import torchaudio
    from stable_audio_tools.inference.generation import generate_diffusion_cond

    sample_rate = model_config.get("sample_rate", 44100)
    sample_size = int(duration * sample_rate)

    if seed >= 0:
        torch.manual_seed(seed)

    conditioning = [{"prompt": prompt, "seconds_start": 0, "seconds_total": duration}]

    logger.info(
        "Generating audio: prompt=%r, duration=%.1fs, steps=%d, cfg=%.1f",
        prompt[:80],
        duration,
        steps,
        guidance_scale,
    )

    with torch.no_grad():
        output = generate_diffusion_cond(
            model,
            steps=steps,
            cfg_scale=guidance_scale,
            conditioning=conditioning,
            sample_size=sample_size,
            sigma_min=0.3,
            sigma_max=500,
            sampler_type="dpmpp-3m-sde",
            device=device,
        )

    # Output shape: [batch, channels, samples] — take first item
    audio_tensor = output[0]
    if audio_tensor.dim() == 1:
        audio_tensor = audio_tensor.unsqueeze(0)

    # Normalize to prevent clipping
    peak = audio_tensor.abs().max()
    if peak > 0:
        audio_tensor = audio_tensor / peak * 0.95

    # Encode to WAV as base64
    buffer = io.BytesIO()
    torchaudio.save(buffer, audio_tensor.cpu(), sample_rate, format="wav")
    audio_base64 = base64.b64encode(buffer.getvalue()).decode("ascii")

    actual_duration = audio_tensor.shape[-1] / sample_rate

    logger.info(
        "Audio generated: %.1fs, %d Hz, %d bytes base64",
        actual_duration,
        sample_rate,
        len(audio_base64),
    )

    return {
        "audio_base64": audio_base64,
        "duration_seconds": actual_duration,
        "sample_rate": sample_rate,
    }
