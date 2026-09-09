"""Closed local Stable Diffusion construction; package metadata never selects code."""

import inspect
import json
from pathlib import Path
from typing import NamedTuple


class DiffusionLoadError(Exception):
    """A declared loader failure, projected by the worker transport."""

    def __init__(self, kind, message):
        super().__init__(message)
        self.kind = kind


class _Admission(NamedTuple):
    root: Path
    components: tuple
    requires_safety_checker: bool


_REQUIRED = {"unet", "vae", "text_encoder", "tokenizer", "scheduler"}
_IDENTITIES = {
    "unet": ("diffusers", {"UNet2DConditionModel"}),
    "vae": ("diffusers", {"AutoencoderKL"}),
    "text_encoder": ("transformers", {"CLIPTextModel"}),
    "tokenizer": ("transformers", {"CLIPTokenizer", "CLIPTokenizerFast"}),
    "scheduler": ("diffusers", None),
    "safety_checker": ("stable_diffusion", {"StableDiffusionSafetyChecker"}),
    "feature_extractor": ("transformers", {"CLIPImageProcessor", "CLIPFeatureExtractor"}),
    "image_encoder": ("transformers", {"CLIPVisionModelWithProjection"}),
}
_WEIGHT_SLOTS = {"unet", "vae", "text_encoder", "safety_checker", "image_encoder"}


def _fail(kind, message):
    raise DiffusionLoadError(kind, message)


def _object_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            _fail("invalid_request", "Duplicate diffusion configuration key")
        result[key] = value
    return result


def _read_config(path):
    try:
        value = json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_object_pairs)
    except (ValueError, UnicodeError) as exc:
        raise DiffusionLoadError("invalid_request", "Malformed diffusion configuration") from exc
    except OSError as exc:
        raise DiffusionLoadError("model_load_failed", "Cannot read local diffusion configuration") from exc
    if not isinstance(value, dict):
        _fail("invalid_request", "Diffusion configuration must be an object")
    return value


def _identifier(value):
    if not isinstance(value, str) or not value.isascii() or not value.isidentifier() or value.startswith("_") or "__" in value:
        _fail("invalid_request", "Unsafe diffusion implementation identifier")
    return value


def _runtime():
    try:
        import diffusers
        import transformers
        from diffusers.schedulers.scheduling_utils import KarrasDiffusionSchedulers, SchedulerMixin
        from transformers.utils.import_utils import check_torch_load_is_safe
        # This installed release uses weights_only=True for every torch shard load.
        # New Diffusers releases need the same dependency-source qualification.
        if diffusers.__version__ != "0.37.0":
            _fail("runtime_unavailable", "Diffusers version is not qualified for restricted weight loading")
        check_torch_load_is_safe()
        return diffusers, transformers, KarrasDiffusionSchedulers, SchedulerMixin
    except DiffusionLoadError:
        raise
    except Exception as exc:
        raise DiffusionLoadError("runtime_unavailable", "Installed diffusion runtime safety capability is unavailable") from exc


def _class_for(slot, name):
    diffusers, transformers, schedulers, scheduler_base = _runtime()
    try:
        if slot == "safety_checker":
            from diffusers.pipelines.stable_diffusion.safety_checker import StableDiffusionSafetyChecker
            return StableDiffusionSafetyChecker
        namespace = transformers if _IDENTITIES[slot][0] == "transformers" else diffusers
        if slot == "scheduler" and name not in schedulers.__members__:
            _fail("unsupported_task" if hasattr(diffusers, name) else "trust_policy_rejected",
                  "Diffusion scheduler is outside the admitted built-in set")
        cls = getattr(namespace, name)
        if not inspect.isclass(cls) or inspect.isabstract(cls):
            _fail("runtime_unavailable", "Installed diffusion component is not concrete")
        if slot == "scheduler" and not issubclass(cls, scheduler_base):
            _fail("runtime_unavailable", "Installed scheduler does not implement SchedulerMixin")
        return cls
    except DiffusionLoadError:
        raise
    except Exception as exc:
        raise DiffusionLoadError("runtime_unavailable", "Installed diffusion component is unavailable") from exc


def admit_diffusion_bundle(model_path):
    """Validate fresh metadata before any loading effect or resident reuse."""
    try:
        root = Path(model_path).resolve(strict=True)
    except (OSError, TypeError, ValueError, RuntimeError) as exc:
        raise DiffusionLoadError("model_load_failed", "Local diffusion bundle is unavailable") from exc
    config = _read_config(root / "model_index.json")
    pipeline = config.get("_class_name")
    if isinstance(pipeline, list) and pipeline:
        _fail("trust_policy_rejected", "Custom diffusion pipelines are not authorized")
    _identifier(pipeline)
    if pipeline != "StableDiffusionPipeline":
        _fail("unsupported_task", "Only built-in StableDiffusionPipeline bundles are supported")
    for field in config:
        if field not in _IDENTITIES and field not in {"_class_name", "_diffusers_version", "_name_or_path", "requires_safety_checker"}:
            _fail("trust_policy_rejected" if field in {"auto_map", "custom_pipeline", "trust_remote_code"} else "unsupported_task", "Unsupported diffusion configuration directive")
    for field in ("_diffusers_version", "_name_or_path"):
        if field in config and not isinstance(config[field], str):
            _fail("invalid_request", "Diffusion metadata must be a string")
    requires_safety_checker = config.get("requires_safety_checker", True)
    if type(requires_safety_checker) is not bool:
        _fail("invalid_request", "requires_safety_checker must be boolean")
    components = []
    for slot, (library, names) in _IDENTITIES.items():
        pair = config.get(slot)
        if pair is None and slot not in _REQUIRED and slot not in config:
            continue
        if not isinstance(pair, list) or len(pair) != 2:
            _fail("invalid_request", "Missing or malformed diffusion component: " + slot)
        if pair == [None, None] and slot not in _REQUIRED:
            continue
        declared_library, name = map(_identifier, pair)
        if declared_library != library:
            _fail("trust_policy_rejected", "Custom diffusion component library is not authorized: " + slot)
        if names is not None and name not in names:
            namespace = _runtime()[1 if library == "transformers" else 0]
            if not hasattr(namespace, name):
                _fail("trust_policy_rejected", "Custom diffusion component class is not authorized: " + slot)
            _fail("unsupported_task", "Unsupported built-in diffusion component: " + slot)
        _class_for(slot, name)
        files = ("tokenizer_config.json", "config.json") if slot == "tokenizer" else (("preprocessor_config.json",) if slot == "feature_extractor" else (("scheduler_config.json",) if slot == "scheduler" else ("config.json",)))
        for filename in files:
            path = root / slot / filename
            if path.exists():
                component_config = _read_config(path)
                if any(component_config.get(key) for key in ("auto_map", "custom_pipeline")):
                    _fail("trust_policy_rejected", "Custom component code mapping is not authorized: " + slot)
        components.append((slot, name))
    if "safety_checker" in dict(components) and "feature_extractor" not in dict(components):
        _fail("invalid_request", "A safety checker requires its feature extractor")
    return _Admission(root, tuple(components), requires_safety_checker)


def construct_diffusion_pipeline(admission, torch_dtype, variant=None):
    """Load only explicit installed classes, locally, with safetensors weights."""
    diffusers = _runtime()[0]
    try:
        pipeline_class = diffusers.StableDiffusionPipeline
    except Exception as exc:
        raise DiffusionLoadError("runtime_unavailable", "Installed StableDiffusionPipeline is unavailable") from exc
    components = {slot: None for slot in _IDENTITIES if slot not in _REQUIRED}
    try:
        for slot, name in admission.components:
            kwargs = {"local_files_only": True}
            if _IDENTITIES[slot][0] == "transformers" or slot == "safety_checker":
                kwargs["trust_remote_code"] = False
            if slot in _WEIGHT_SLOTS:
                kwargs.update(torch_dtype=torch_dtype, use_safetensors=True)
                if _IDENTITIES[slot][0] == "transformers" or slot == "safety_checker":
                    kwargs["weights_only"] = True
                if variant is not None:
                    kwargs["variant"] = variant
            components[slot] = _class_for(slot, name).from_pretrained(str(admission.root / slot), **kwargs)
        return pipeline_class(**components, requires_safety_checker=admission.requires_safety_checker)
    except DiffusionLoadError:
        raise
    except Exception as exc:
        raise DiffusionLoadError("model_load_failed", "Failed to load local safetensors diffusion bundle") from exc
