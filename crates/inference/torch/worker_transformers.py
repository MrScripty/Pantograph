"""Transformers compatibility shims for PyTorch worker model loading."""

import logging
from pathlib import Path

import torch

logger = logging.getLogger("pantograph.pytorch")


def apply_compatibility_shims():
    """Patch transformers modules for cross-version compatibility.

    Models loaded via trust_remote_code=True, such as SDAR and TraDo, may
    import names that were removed in newer transformers versions. This injects
    aliases so the model code works regardless of installed version.
    """
    import importlib.metadata
    import transformers.cache_utils as cu
    import transformers.utils as tu

    version = importlib.metadata.version("transformers")
    major = int(version.split(".")[0])

    if not hasattr(cu, "SlidingWindowCache") and hasattr(cu, "DynamicSlidingWindowLayer"):
        cu.SlidingWindowCache = cu.DynamicSlidingWindowLayer
        logger.info(
            "Shimmed SlidingWindowCache -> DynamicSlidingWindowLayer (transformers %s)",
            version,
        )

    if not hasattr(tu, "LossKwargs"):
        from typing import Optional, TypedDict

        class LossKwargs(TypedDict, total=False):
            num_items_in_batch: Optional["torch.Tensor"]

        tu.LossKwargs = LossKwargs
        logger.info("Shimmed LossKwargs stub into transformers.utils (transformers %s)", version)

    import transformers.modeling_utils as modeling_utils

    if not getattr(modeling_utils, "_pantograph_dispatch_patched", False):
        original_dispatch = modeling_utils.dispatch_model

        def safe_dispatch(model, device_map, **kwargs):
            has_meta = any(p.device.type == "meta" for p in model.parameters())
            if not has_meta:
                return original_dispatch(model, device_map, **kwargs)
            if isinstance(device_map, dict) and len(device_map) == 1 and "" in device_map:
                device = device_map[""]
            elif isinstance(device_map, str):
                device = device_map
            else:
                return original_dispatch(model, device_map, **kwargs)

            meta_names = {n for n, p in model.named_parameters() if p.device.type == "meta"}

            if meta_names and hasattr(model.config, "_name_or_path"):
                import glob as glob_module
                from safetensors.torch import load_file

                model_dir = Path(model.config._name_or_path)
                if model_dir.is_dir():
                    expected_shapes = {
                        n: p.shape for n, p in model.named_parameters() if n in meta_names
                    }
                    loaded_count = 0
                    for shard in sorted(glob_module.glob(str(model_dir / "*.safetensors"))):
                        state_dict = load_file(shard, device=str(device))
                        for key, value in state_dict.items():
                            candidates = [key]
                            if key.startswith("language_model."):
                                candidates.append(key.replace("language_model.", "model.", 1))
                            for candidate in candidates:
                                if candidate in meta_names:
                                    expected_shape = expected_shapes.get(candidate)
                                    if expected_shape is not None and value.shape != expected_shape:
                                        slices = tuple(slice(0, size) for size in expected_shape)
                                        value = value[slices].contiguous()
                                    _set_nested_parameter(
                                        model,
                                        candidate,
                                        torch.nn.Parameter(value, requires_grad=False),
                                    )
                                    meta_names.discard(candidate)
                                    loaded_count += 1
                                    break
                        del state_dict
                    logger.info(
                        "  Reloaded %d params from safetensors (%d still meta)",
                        loaded_count,
                        len(meta_names),
                    )

            for name, param in list(model.named_parameters()):
                if param.device.type == "meta":
                    _set_nested_parameter(
                        model,
                        name,
                        torch.nn.Parameter(
                            torch.empty(param.shape, dtype=param.dtype, device=device),
                            requires_grad=param.requires_grad,
                        ),
                    )
                elif str(param.device) != str(torch.device(device)):
                    _set_nested_parameter(
                        model,
                        name,
                        torch.nn.Parameter(
                            param.data.to(device),
                            requires_grad=param.requires_grad,
                        ),
                    )
            for name, buffer in list(model.named_buffers()):
                if buffer.device.type == "meta":
                    _set_nested_attribute(
                        model,
                        name,
                        torch.empty(buffer.shape, dtype=buffer.dtype, device=device),
                    )
                elif str(buffer.device) != str(torch.device(device)):
                    _set_nested_attribute(model, name, buffer.to(device))
            model.tie_weights()
            logger.info("Shimmed dispatch_model: materialised meta tensors onto %s", device)
            return model

        modeling_utils.dispatch_model = safe_dispatch
        modeling_utils._pantograph_dispatch_patched = True
        logger.info("Shimmed dispatch_model for meta-tensor safety (transformers %s)", version)

    if major >= 5:
        from transformers import PretrainedConfig

        original_config_init = PretrainedConfig.__init__
        if not getattr(PretrainedConfig, "_pantograph_patched", False):
            token_defaults = {"pad_token_id": None, "bos_token_id": None, "eos_token_id": None}

            def patched_config_init(self, **kwargs):
                original_config_init(self, **kwargs)
                for attr, default in token_defaults.items():
                    if not hasattr(self, attr):
                        setattr(self, attr, default)

            PretrainedConfig.__init__ = patched_config_init
            PretrainedConfig._pantograph_patched = True
            logger.info("Shimmed PretrainedConfig token ID defaults (transformers %s)", version)


def _set_nested_parameter(model, name, parameter):
    _set_nested_attribute(model, name, parameter)


def _set_nested_attribute(model, name, value):
    parts = name.split(".")
    module = model
    for part in parts[:-1]:
        module = getattr(module, part)
    setattr(module, parts[-1], value)
