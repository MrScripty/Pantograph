"""DepthPro depth estimation pipeline.

Sibling module for the depth worker. Handles model loading and depth
estimation using Apple's DepthPro model via HuggingFace Transformers.

All public functions are called from the depth worker module.
"""

import base64
import io
import logging

import numpy as np
import torch
from PIL import Image

logger = logging.getLogger("pantograph.depth.estimation")


def load_model(model_path, device="auto"):
    """Load a DepthPro model from disk.

    Uses HuggingFace Transformers' DepthProForDepthEstimation. The model_path
    must contain config.json, model.safetensors, and preprocessor_config.json
    (the HF-format checkpoint from apple/DepthPro-hf).

    Args:
        model_path: Path to the HF-format model directory.
        device: Target device ("auto", "cuda", "mps", "cpu").

    Returns:
        Tuple of (model, processor) ready for inference.

    Raises:
        FileNotFoundError: If required HF-format files are missing.
    """
    from pathlib import Path

    from transformers import DepthProForDepthEstimation, DepthProImageProcessorFast

    model_dir = Path(model_path)
    required_files = ["config.json"]
    missing = [f for f in required_files if not (model_dir / f).exists()]
    if missing:
        raise FileNotFoundError(
            f"Model directory {model_path} is missing {missing}. "
            "DepthPro requires the HuggingFace-format checkpoint "
            "(apple/DepthPro-hf with config.json + model.safetensors)."
        )

    if device == "auto":
        if torch.cuda.is_available():
            device = "cuda"
        elif hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
            device = "mps"
        else:
            device = "cpu"

    logger.info("Loading DepthPro model from %s on %s", model_path, device)

    processor = DepthProImageProcessorFast.from_pretrained(model_path)
    model = DepthProForDepthEstimation.from_pretrained(
        model_path,
        torch_dtype=torch.float16 if device == "cuda" else torch.float32,
    ).to(device)
    model.eval()

    logger.info("DepthPro model loaded successfully")
    return model, processor


def estimate_depth(model, processor, device, image_base64):
    """Estimate depth from a base64-encoded image.

    Args:
        model: Loaded DepthPro model.
        processor: DepthPro image processor.
        device: Device the model is on.
        image_base64: Base64-encoded input image (PNG/JPEG).

    Returns:
        Dict with:
          - depth_map_base64: Grayscale depth map as base64 PNG
          - focal_length: Predicted focal length in pixels
          - width, height: Image dimensions
          - point_cloud: {positions: [[x,y,z],...], colors: [[r,g,b],...]}
            subsampled to ~16k points
    """
    # Decode input image
    image_bytes = base64.b64decode(image_base64)
    image = Image.open(io.BytesIO(image_bytes)).convert("RGB")
    width, height = image.size

    logger.info("Running depth estimation on %dx%d image", width, height)

    # Preprocess and run inference
    inputs = processor(images=image, return_tensors="pt").to(device)
    with torch.no_grad():
        outputs = model(**inputs)

    # Extract depth and focal length
    predicted_depth = outputs.predicted_depth.squeeze().cpu().numpy()
    focal_length = float(outputs.predicted_focal_length.item()) if hasattr(outputs, "predicted_focal_length") else width

    # Normalize depth to 0-255 for visualization
    depth_min = predicted_depth.min()
    depth_max = predicted_depth.max()
    if depth_max - depth_min > 0:
        depth_normalized = ((predicted_depth - depth_min) / (depth_max - depth_min) * 255).astype(np.uint8)
    else:
        depth_normalized = np.zeros_like(predicted_depth, dtype=np.uint8)

    # Encode depth map as grayscale PNG
    depth_image = Image.fromarray(depth_normalized, mode="L")
    depth_image = depth_image.resize((width, height), Image.BILINEAR)
    depth_buffer = io.BytesIO()
    depth_image.save(depth_buffer, format="PNG")
    depth_map_base64 = base64.b64encode(depth_buffer.getvalue()).decode("ascii")

    # Generate point cloud (subsample every 8th pixel for ~16k points)
    point_cloud = _generate_point_cloud(
        predicted_depth, image, focal_length, width, height, subsample=8
    )

    logger.info(
        "Depth estimation complete: focal_length=%.1f, %d point cloud points",
        focal_length,
        len(point_cloud["positions"]),
    )

    return {
        "depth_map_base64": depth_map_base64,
        "focal_length": focal_length,
        "width": width,
        "height": height,
        "point_cloud": point_cloud,
    }


def _generate_point_cloud(depth, image, focal_length, width, height, subsample=8):
    """Back-project pixels to 3D using pinhole camera model.

    Uses the formula: X = (u - cx) * Z / fx, Y = (v - cy) * Z / fy

    Args:
        depth: 2D depth array (H, W).
        image: PIL Image for color sampling.
        focal_length: Predicted focal length in pixels.
        width, height: Original image dimensions.
        subsample: Take every Nth pixel to reduce point count.

    Returns:
        Dict with "positions" and "colors" lists.
    """
    # Resize depth to match image dimensions
    depth_h, depth_w = depth.shape
    if depth_h != height or depth_w != width:
        depth_resized = np.array(
            Image.fromarray(depth).resize((width, height), Image.BILINEAR)
        )
    else:
        depth_resized = depth

    image_array = np.array(image)

    cx = width / 2.0
    cy = height / 2.0
    fx = fy = focal_length

    positions = []
    colors = []

    for v in range(0, height, subsample):
        for u in range(0, width, subsample):
            z = float(depth_resized[v, u])
            if z <= 0:
                continue

            x = (u - cx) * z / fx
            y = (v - cy) * z / fy

            r, g, b = image_array[v, u, :3]
            positions.append([x, y, z])
            colors.append([r / 255.0, g / 255.0, b / 255.0])

    return {"positions": positions, "colors": colors}
