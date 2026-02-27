"""Depth estimation worker for Pantograph.

Embedded in the Rust process via PyO3. Provides model loading and
depth estimation for Apple DepthPro models.

All public functions are called from Rust through PyO3's Python::with_gil.
Module-level globals hold the loaded model state.

Estimation logic is in the sibling module:
  - depth_estimation: DepthPro model loading and depth estimation
"""

import logging
import sys
from pathlib import Path

import torch

# When loaded from the filesystem, ensure sibling modules are importable.
_self_path = Path(__file__).resolve()
if _self_path.parent.is_dir():
    _depth_dir = str(_self_path.parent)
    if _depth_dir not in sys.path:
        sys.path.insert(0, _depth_dir)

from depth_estimation import load_model as _load_model
from depth_estimation import estimate_depth as _estimate_depth

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("pantograph.depth")

# Module-level state — one depth model loaded at a time
_model = None
_processor = None
_device = None
_model_path = None


def is_loaded():
    """Check whether a depth model is currently loaded."""
    return _model is not None


def get_loaded_info():
    """Return metadata about the currently loaded depth model, or None."""
    if _model is None:
        return None
    return {
        "model_path": _model_path,
        "device": _device,
    }


def load_model(model_path, device="auto"):
    """Load a DepthPro model.

    Args:
        model_path: Path to the HF-format model directory.
        device: Target device ("auto", "cuda", "mps", "cpu").

    Returns:
        Dict with model_path and device.
    """
    global _model, _processor, _device, _model_path

    # Unload previous model if different path
    if _model is not None and _model_path != model_path:
        unload_model()

    if _model is not None:
        logger.info("Depth model already loaded from %s", _model_path)
        return get_loaded_info()

    _model, _processor = _load_model(model_path, device)

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
    """Unload the current depth model and free resources."""
    global _model, _processor, _device, _model_path

    if _model is not None:
        logger.info("Unloading depth model from %s", _model_path)
        del _model
        _model = None
        _processor = None
        _device = None
        _model_path = None

        if torch.cuda.is_available():
            torch.cuda.empty_cache()


def estimate_depth(image_base64):
    """Estimate depth from a base64-encoded image.

    Args:
        image_base64: Base64-encoded input image (PNG/JPEG).

    Returns:
        Dict with depth_map_base64, focal_length, width, height, point_cloud.

    Raises:
        RuntimeError: If no model is loaded.
    """
    if _model is None:
        raise RuntimeError("No depth model loaded. Call load_model() first.")

    return _estimate_depth(_model, _processor, _device, image_base64)
