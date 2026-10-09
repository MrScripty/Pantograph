"""Opt-in native text timing evidence; no inference or residency authority.

The admitted v1 profile is deliberately small: ordinary CPU GPT2LMHeadModel,
inspectable contiguous tensors and a simple native WordLevel tokenizer. Unknown
profiles refuse. Stable hashes never include generation/object/request IDs.
Generation and weak object stamps separately fence every ordinary shared-worker
load/unload/shutdown, including failures and A -> B -> A transitions.
"""

import functools
import hashlib
import json
import math
import os
import platform
import sys
import threading
import types
import uuid
import weakref

MAX_GENERATION = (1 << 64) - 1
MAX_ACTIVE_TRANSITIONS = 64
MAX_COLLECTORS = 64
MAX_MODEL_BYTES = 8 * 1024 * 1024
MAX_MODULES = 1024
MAX_TENSORS = 4096
MAX_VOCAB = 4096
MAX_TEXT_BYTES = 64 * 1024
MAX_TOKENIZER_BYTES = 4 * 1024 * 1024
MAX_NODES = 4096
MAX_DEPTH = 8
MAX_STRING = 16 * 1024


class _Unknown(Exception):
    pass


def _text(value, limit=MAX_STRING):
    if type(value) is not str or len(value) > limit:
        raise _Unknown()
    encoded = value.encode("utf-8")
    if len(encoded) > limit:
        raise _Unknown()
    return value


def _canonical(value):
    """Inspect bounds before container copies, sorting, encoding or recursion."""
    remaining = [MAX_NODES, MAX_TEXT_BYTES]

    def visit(item, depth):
        remaining[0] -= 1
        if remaining[0] < 0 or depth > MAX_DEPTH:
            raise _Unknown()
        if item is None or type(item) is bool:
            return item
        if type(item) is int:
            if not -(1 << 64) <= item < (1 << 64):
                raise _Unknown()
            return item
        if type(item) is float:
            if not math.isfinite(item):
                raise _Unknown()
            return item
        if type(item) is str:
            value = _text(item)
            remaining[1] -= len(value.encode("utf-8"))
            if remaining[1] < 0:
                raise _Unknown()
            return value
        if type(item) in (set, frozenset):
            if len(item) > MAX_NODES:
                raise _Unknown()
            # Validate before sorting: opaque comparisons and oversized string
            # comparisons cannot run merely because this container is a set.
            if any(type(child) not in (str, int) for child in item):
                raise _Unknown()
            members = [visit(child, depth + 1) for child in item]
            return sorted(members, key=lambda child: (type(child).__name__, child))
        if type(item) in (list, tuple):
            if len(item) > MAX_NODES:
                raise _Unknown()
            return [visit(child, depth + 1) for child in item]
        if type(item) is dict:
            if len(item) > MAX_NODES:
                raise _Unknown()
            keys = [visit(key, depth + 1) for key in item]
            if any(type(key) not in (str, int) for key in keys):
                raise _Unknown()
            # GPT2's actual id2label uses integer keys. Preserve key types;
            # JSON object coercion would conflate 0 and "0".
            return {"$dict": [[type(key).__name__, key, visit(item[key], depth + 1)]
                              for key in sorted(keys, key=lambda key: (type(key).__name__, key))]}
        # Actual native scalar values with sealed, versioned implementations.
        if type(item) is sys.modules["torch"].dtype:
            return visit(str(item), depth + 1)
        if type(item) is sys.modules["torch"].device:
            return visit(str(item), depth + 1)
        if type(item) is sys.modules["tokenizers"].AddedToken:
            return visit({name: getattr(item, name) for name in
                          ("content", "single_word", "lstrip", "rstrip", "normalized", "special")}, depth + 1)
        from transformers import GPT2Config, GenerationConfig
        if type(item) in (GPT2Config, GenerationConfig):
            return visit(_configuration_state(item), depth + 1)
        raise _Unknown()

    return json.dumps(visit(value, 0), sort_keys=True, ensure_ascii=False,
                      separators=(",", ":"), allow_nan=False).encode("utf-8")


def _digest(value):
    return hashlib.sha256(_canonical(value)).hexdigest()


def _configuration_state(value):
    fields = vars(value)
    if len(fields) > MAX_NODES:
        raise _Unknown()
    # These installed native fields are locators/diagnostics, not execution
    # configuration. Content comes from actual tensors, never these labels.
    ignored = {"_name_or_path", "_commit_hash"}
    return {key: item for key, item in fields.items() if key not in ignored}


def _cpu_domain(torch):
    # A process-local Linux CPU execution domain, not portable hardware or
    # capacity equivalence. No CPU ordinal/name alone is authoritative.
    if sys.platform != "linux":
        raise _Unknown()
    with open("/proc/sys/kernel/random/boot_id", "rb") as source:
        raw = source.read(65)
    if len(raw) > 64:
        raise _Unknown()
    boot = str(uuid.UUID(raw.decode("ascii").strip()))
    affinity = None
    count = 0
    with os.scandir("/proc/self/task") as tasks:
        for task in tasks:
            count += 1
            if count > 256 or not task.name.isdecimal():
                raise _Unknown()
            mask = os.sched_getaffinity(int(task.name))
            if not mask or len(mask) > 256 or any(cpu < 0 or cpu > 1_000_000 for cpu in mask):
                raise _Unknown()
            current = sorted(mask)
            if affinity is not None and affinity != current:
                raise _Unknown()
            affinity = current
    if affinity is None:
        raise _Unknown()
    return _digest(("linux-process-cpu-domain.v1", boot, os.getpid(),
                    platform.machine(), affinity, torch.backends.cpu.get_cpu_capability()))


def _bounded_tokenizer_native_state(native):
    """Use only the pinned provider's compiled, fixed-envelope component API.

    The component owns admission before traversal and returns spent counters,
    including refused prefixes. This does not qualify the complete owner or
    admit NativeOwnerSnapshot in the Rust ledger. Missing API remains advisory.
    """
    import tokenizers

    operation = vars(tokenizers.Tokenizer).get("_pantograph_wordlevel_snapshot_v1")
    if operation is None:
        return None
    if (type(tokenizers.__version__) is not str or tokenizers.__version__ != "0.21.4"
            or type(operation) is not types.MethodDescriptorType
            or operation.__objclass__ is not tokenizers.Tokenizer
            or operation.__name__ != "_pantograph_wordlevel_snapshot_v1"):
        raise _Unknown()
    # Invoke the sealed type descriptor, never a per-instance override. It
    # enters before model/unk/id/export getters, retains native read guards,
    # and takes no caller-provided caps, callback, or timing declaration.
    result = operation(native)
    if type(result) is not tuple or len(result) != 4:
        raise _Unknown()
    accepted, payload, work, copied = result
    maximum_work = 5 * 4 * MAX_VOCAB * 14 * (MAX_STRING + 1) + 32 * MAX_TOKENIZER_BYTES
    if (type(accepted) is not bool or type(payload) is not bytes
            or type(work) is not int or not 0 <= work <= maximum_work
            or type(copied) is not int or not 0 <= copied <= MAX_TOKENIZER_BYTES
            or not accepted or len(payload) > MAX_TOKENIZER_BYTES
            or not payload.startswith(b"pantograph-tokenizers-0.21.4-wordlevel-snapshot.v1\0")):
        raise _Unknown()
    return payload, (work, copied)


def _tokenizer_state(tokenizer):
    import tokenizers
    from transformers import PreTrainedTokenizerFast

    if type(tokenizer) is not PreTrainedTokenizerFast:
        raise _Unknown()
    if type(tokenizer.model_max_length) is not int or not 0 < tokenizer.model_max_length <= 1_000_000:
        raise _Unknown()
    native = tokenizer.backend_tokenizer
    if type(native) is not tokenizers.Tokenizer:
        raise _Unknown()
    bounded = _bounded_tokenizer_native_state(native)
    if bounded is None and (type(native.model) is not tokenizers.models.WordLevel
            or type(native.pre_tokenizer) not in
               (tokenizers.pre_tokenizers.Whitespace, tokenizers.pre_tokenizers.WhitespaceSplit)
            or native.normalizer is not None or native.post_processor is not None
            or native.decoder is not None):
        raise _Unknown()
    # This scalar is outside the vocabulary; qualify it before whole export.
    # Native getters themselves may allocate before returning a Python string.
    # This bounds accepted content, not universal oversized-refusal copy cost.
    if bounded is None:
        _text(native.model.unk_token)
        size = native.get_vocab_size(with_added_tokens=True)
        if not 0 < size <= MAX_VOCAB:
            raise _Unknown()
        total = 0
        for index in range(size):
            token = _text(native.id_to_token(index))
            total += len(token.encode("utf-8"))
            if total > MAX_TEXT_BYTES:
                raise _Unknown()
    # WordLevel has no opaque merges/regex/processor graphs. Vocab, added-token
    # content and fixed scalar metadata are bounded above before native export.
    if bounded is None:
        _canonical((native.padding, native.truncation))
    fields = vars(tokenizer)
    if len(fields) > MAX_NODES:
        raise _Unknown()
    python_settings = {key: value for key, value in fields.items()
                       if key not in {"_tokenizer", "name_or_path", "deprecation_warnings"}}
    init = python_settings["init_kwargs"]
    if type(init) is not dict or len(init) > MAX_NODES:
        raise _Unknown()
    python_settings["init_kwargs"] = {key: value for key, value in init.items()
                                       if key not in {"name_or_path", "tokenizer_file", "_commit_hash"}}
    settings = _canonical(python_settings)
    if bounded is not None:
        payload, _component_cost = bounded
        # Explicitly version the new binary convention; old advisory hashes
        # cannot be comparable to this stronger installed-history identity.
        digest = hashlib.sha256(b"installed-wordlevel-bounded-component.v2\0")
        digest.update(payload)
        digest.update(settings)
        return digest.hexdigest()
    state = native.to_str(pretty=False)
    if len(state) > MAX_TOKENIZER_BYTES:
        raise _Unknown()
    encoded = state.encode("utf-8")
    if len(encoded) > MAX_TOKENIZER_BYTES:
        raise _Unknown()
    return hashlib.sha256(encoded + settings).hexdigest()


def _model_state(model, torch):
    from transformers import GPT2LMHeadModel

    if type(model) is not GPT2LMHeadModel or getattr(model, "is_quantized", False):
        raise _Unknown()
    digest = hashlib.sha256(b"installed-cpu-gpt2-state.v1\0")
    stack = [("", model)]
    seen = {}
    storages = {}
    modules = tensors = byte_count = metadata_bytes = visits = 0
    while stack:
        prefix, module = stack.pop()
        visits += 1
        if visits > MAX_MODULES:
            raise _Unknown()
        if id(module) in seen:
            # Tied graph aliases are explicitly described, not recursively copied.
            digest.update(_canonical(("module_alias", prefix, seen[id(module)])))
            continue
        seen[id(module)] = prefix
        modules += 1
        module_type = type(module)
        if (modules > MAX_MODULES or module.training
                or not module_type.__module__.startswith(
                    ("torch.nn.modules.", "transformers.models.gpt2.modeling_gpt2",
                     "transformers.activations", "transformers.pytorch_utils"))
                or getattr(sys.modules.get(module_type.__module__), module_type.__name__, None) is not module_type
                or getattr(module, "_hf_hook", None) is not None):
            raise _Unknown()
        hooks = ("_forward_hooks", "_forward_pre_hooks", "_backward_hooks", "_backward_pre_hooks",
                 "_forward_hooks_with_kwargs", "_forward_hooks_always_called", "_forward_pre_hooks_with_kwargs",
                 "_state_dict_hooks", "_state_dict_pre_hooks", "_load_state_dict_pre_hooks",
                 "_load_state_dict_post_hooks")
        for name in hooks:
            if getattr(module, name, None):
                raise _Unknown()
        digest.update(_canonical(("module", prefix, module_type.__module__, module_type.__name__)))
        fields = vars(module)
        if len(fields) > MAX_NODES:
            raise _Unknown()
        excluded = set(hooks) | {"_parameters", "_buffers", "_modules", "warnings_issued"}
        settings = _canonical({key: value for key, value in fields.items() if key not in excluded})
        metadata_bytes += len(settings)
        if metadata_bytes > MAX_TEXT_BYTES:
            raise _Unknown()
        digest.update(settings)
        for kind in ("_parameters", "_buffers"):
            values = getattr(module, kind)
            if type(values) is not dict or len(values) > MAX_TENSORS:
                raise _Unknown()
            names = [_text(name, 256) for name in values]
            for name in sorted(names):
                tensor = values[name]
                if tensor is None:
                    digest.update(_canonical((kind, prefix, name, None)))
                    continue
                tensors += 1
                if (tensors > MAX_TENSORS or type(tensor) not in (torch.Tensor, torch.nn.Parameter)
                        or tensor.device.type != "cpu"
                        or tensor.layout != torch.strided or not tensor.is_contiguous()
                        or tensor.dtype not in (torch.float32, torch.float16, torch.bfloat16,
                                               torch.int64, torch.int32, torch.int8,
                                               torch.uint8, torch.bool)
                        or tensor.ndim > 8):
                    raise _Unknown()
                size = tensor.numel() * tensor.element_size()
                byte_count += size
                if byte_count > MAX_MODEL_BYTES:
                    raise _Unknown()
                version = tensor._version
                # Addresses are used only to recognize actual sharing inside
                # this snapshot. Stable names/offsets, never addresses, enter K.
                storage = tensor.untyped_storage()
                storage_key = storage.data_ptr()
                first_name = storages.setdefault(storage_key, (kind, prefix, name))
                digest.update(_canonical((kind, prefix, name, str(tensor.dtype),
                                          list(tensor.shape), tensor.requires_grad,
                                          first_name, tensor.storage_offset())))
                view = memoryview(tensor.detach().reshape(-1).view(torch.uint8).numpy()).cast("B")
                for offset in range(0, size, 64 * 1024):
                    digest.update(view[offset:offset + 64 * 1024])
                if tensor._version != version:
                    raise _Unknown()
        children = module._modules
        if type(children) is not dict or len(children) > MAX_MODULES:
            raise _Unknown()
        names = [_text(name, 256) for name in children]
        for name in sorted(names, reverse=True):
            child = children[name]
            if child is not None:
                next_prefix = name if not prefix else prefix + "." + name
                _text(next_prefix, 256)
                stack.append((next_prefix, child))
                if len(stack) > MAX_MODULES:
                    raise _Unknown()
    return digest.hexdigest()


def _snapshot(worker, implementation_digest):
    import torch
    import transformers
    import tokenizers
    import numpy

    model, tokenizer = worker._model, worker._tokenizer
    if (model is None or tokenizer is None or str(worker._device) != "cpu"
            or worker._model_type not in {"text-generation", "gpt2"}
            or os.environ.get("TOKENIZERS_PARALLELISM") != "false"):
        raise _Unknown()
    build = _text(torch.__config__.show())
    implementation = _digest(("pytorch-native-cpu-wordlevel-owner.v1", implementation_digest,
                              sys.version, str(torch.__version__), transformers.__version__,
                              tokenizers.__version__, numpy.__version__, build, sys.byteorder))
    configuration = _digest((worker._model_type, _configuration_state(model.config), _configuration_state(model.generation_config),
                             getattr(model, "hf_device_map", None), torch.get_num_threads(),
                             torch.get_num_interop_threads(), torch.are_deterministic_algorithms_enabled(),
                             torch.is_deterministic_algorithms_warn_only_enabled(),
                             torch.backends.mkldnn.enabled, torch.backends.mkldnn.deterministic,
                             torch.get_default_dtype(), torch.get_float32_matmul_precision(),
                             "tokenizers-explicit-serial.v1"))
    content = _digest((_model_state(model, torch), _tokenizer_state(tokenizer)))
    return {"content_fingerprint": content, "implementation_fingerprint": implementation,
            "effective_configuration_fingerprint": configuration,
            "physical_device_fingerprint": _cpu_domain(torch), "device_id": "cpu"}


class OwnerStamp:
    __slots__ = ("facts_json", "_generation", "_model", "_tokenizer", "_tracker")

    def __init__(self, tracker, generation, model, tokenizer, facts):
        self._tracker = weakref.ref(tracker)
        self._generation = generation
        self._model = weakref.ref(model)
        self._tokenizer = weakref.ref(tokenizer)
        self.facts_json = json.dumps(facts, sort_keys=True, separators=(",", ":"))


class Tracker:
    def __init__(self, worker, implementation_digest):
        self._worker = weakref.ref(worker)
        self._implementation_digest = _text(implementation_digest, 64)
        if len(implementation_digest) != 64 or any(c not in "0123456789abcdef" for c in implementation_digest):
            raise _Unknown()
        self._epoch = uuid.uuid4().hex
        self._generation = 0
        self._busy = 0
        self._exhausted = False
        self._state_lock = threading.RLock()
        self._collector_condition = threading.Condition(self._state_lock)
        self._collectors = 0
        self._active = []
        self._scope = threading.local()
        self._entrypoints = {}

    def _wrap(self, name, original):
        @functools.wraps(original)
        def transition(*args, **kwargs):
            thread = threading.get_ident()
            with self._state_lock:
                if self._generation >= MAX_GENERATION:
                    self._exhausted = True
                else:
                    self._generation += 1
                # Bound bookkeeping independently of native caller concurrency.
                # Overflow poisons evidence; execution continues normally.
                tracked = len(self._active) < MAX_ACTIVE_TRANSITIONS
                if not tracked:
                    self._exhausted = True
                for active in self._active:
                    if active["thread"] != thread:
                        active["contaminated"] = True
                    else:
                        active["expected"] = self._generation
                scope = {"thread": thread, "expected": self._generation, "contaminated": False}
                if tracked:
                    self._active.append(scope)
                self._busy += 1
            try:
                # Native collection may release the GIL while strongly borrowing
                # tensors. All shared-worker mutations drain it before effects,
                # including probes whose Rust caller has already been dropped.
                with self._collector_condition:
                    while self._collectors:
                        self._collector_condition.wait()
                result = original(*args, **kwargs)
                if name == "load_model" and hasattr(self._scope, "ack"):
                    with self._state_lock:
                        worker = self._worker()
                        try:
                            if (scope["contaminated"] or not tracked or self._exhausted
                                    or scope["expected"] != self._generation):
                                self._scope.ack = False
                            else:
                                self._scope.ack = ((self._generation, weakref.ref(worker._model),
                                                   weakref.ref(worker._tokenizer))
                                                  if self._scope.ack is None else False)
                        except Exception:
                            self._scope.ack = False
                return result
            finally:
                with self._state_lock:
                    self._busy -= 1
                    if tracked:
                        # Identity removal avoids comparing arbitrary scope data.
                        self._active[:] = [active for active in self._active if active is not scope]
        return transition

    def load_with_ack(self, envelope):
        """Pair the actual caller's response with its load ACK before hashing.

        Thread-local scope survives native GIL release without accepting another
        caller's latest ACK. Later current-owner lookup alone is insufficient.
        No response/exception is changed when attestation refuses.
        """
        worker = self._worker()
        previous = getattr(self._scope, "ack", None)
        self._scope.ack = None
        try:
            response = worker.load_transformers_model_from_envelope(envelope)
            ack = self._scope.ack
            stamp = self.capture()
            if (not ack or stamp is None or stamp._generation != ack[0]
                    or stamp._model() is not ack[1]() or stamp._tokenizer() is not ack[2]()):
                stamp = None
            return response, stamp
        finally:
            self._scope.ack = previous

    def capture(self):
        registered = False
        model = tokenizer = None
        try:
            worker = self._worker()
            with self._collector_condition:
                if (worker is None or self._busy or self._exhausted
                        or self._collectors >= MAX_COLLECTORS
                        or any(getattr(worker, name, None) is not function
                               for name, function in self._entrypoints.items())):
                    return None
                self._collectors += 1
                registered = True
                generation = self._generation
                model, tokenizer = worker._model, worker._tokenizer
            facts = _snapshot(worker, self._implementation_digest)
            with self._collector_condition:
                if (generation != self._generation or self._busy or self._exhausted
                        or worker._model is not model or worker._tokenizer is not tokenizer):
                    return None
                facts["owner_fence"] = self._epoch + ":" + str(generation)
                return OwnerStamp(self, generation, model, tokenizer, facts)
        except Exception as error:
            # Missing/unsupported facts are observation refusal, never load error.
            # An unwound tensor frame can survive in an exception traceback.
            # Release it before the collector's finally signals physical drain.
            error.__traceback__ = None
            error.__context__ = None
            error.__cause__ = None
            return None
        finally:
            # Drop actual native borrows before permitting physical release ACK.
            # _snapshot's frame/tensor locals have already returned or unwound.
            model = tokenizer = None
            if registered:
                with self._collector_condition:
                    self._collectors -= 1
                    self._collector_condition.notify_all()

    def revalidate(self, stamp):
        if (type(stamp) is not OwnerStamp or stamp._tracker() is not self
                or self._generation != stamp._generation or self._busy or self._exhausted):
            return None
        worker = self._worker()
        if (worker is None or stamp._model() is not worker._model
                or stamp._tokenizer() is not worker._tokenizer):
            return None
        current = self.capture()
        return stamp.facts_json if current is not None and current.facts_json == stamp.facts_json else None


def install(worker, implementation_digest):
    existing = getattr(worker, "_pantograph_text_timing_tracker_v1", None)
    if existing is not None:
        return existing if type(existing) is Tracker and existing._implementation_digest == implementation_digest else None
    tracker = Tracker(worker, implementation_digest)
    for name in ("load_model", "unload_model", "shutdown_worker"):
        wrapped = tracker._wrap(name, getattr(worker, name))
        setattr(worker, name, wrapped)
        tracker._entrypoints[name] = wrapped
    worker._pantograph_text_timing_tracker_v1 = tracker
    return tracker
