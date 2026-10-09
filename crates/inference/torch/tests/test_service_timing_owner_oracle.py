"""Independent native-owner protocol oracle; local tiny untrained CPU objects.

Authored from the frozen prospective contract before reading service_timing_owner.
These tests exercise real baseline loader functions and installed Torch/Tokenizers;
no downloads, wall-time assertions, trained-model quality or selector claim.
"""

from concurrent.futures import ThreadPoolExecutor
import gc
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
import uuid
import weakref

import torch
from tokenizers import Tokenizer, decoders, models, normalizers, pre_tokenizers
from transformers import GPT2Config, GPT2LMHeadModel, PreTrainedTokenizerFast

TORCH_DIR = Path(__file__).resolve().parents[1]
if str(TORCH_DIR) not in sys.path:
    sys.path.insert(0, str(TORCH_DIR))
import service_timing_owner as owner

FACT_FIELDS = {
    "owner_fence", "content_fingerprint", "implementation_fingerprint",
    "effective_configuration_fingerprint", "physical_device_fingerprint", "device_id",
}
STABLE_FIELDS = FACT_FIELDS - {"owner_fence"}
IMPLEMENTATION_DIGEST = hashlib.sha256(b"".join(
    (TORCH_DIR / name).read_bytes()
    for name in ("worker.py", "autoregressive.py", "worker_runtime.py", "worker_transformers.py")
)).hexdigest()


def build_native_package(path):
    """Create tiny actual GPT2 weights and the promised restricted tokenizer."""
    path.mkdir()
    vocab = {"<unk>": 0, "<pad>": 1, "hello": 2, "world": 3,
             "alpha": 4, "beta": 5, "gamma": 6, "delta": 7}
    tokenizer = Tokenizer(models.WordLevel(vocab, unk_token="<unk>"))
    tokenizer.pre_tokenizer = pre_tokenizers.Whitespace()
    fast = PreTrainedTokenizerFast(
        tokenizer_object=tokenizer, unk_token="<unk>", pad_token="<pad>",
        bos_token="<unk>", eos_token="<pad>", model_max_length=16,
    )
    with torch.random.fork_rng(devices=[]):
        torch.manual_seed(20261009)
        model = GPT2LMHeadModel(GPT2Config(
            vocab_size=len(vocab), n_positions=16, n_ctx=16, n_embd=8,
            n_layer=1, n_head=1, bos_token_id=0, eos_token_id=1, pad_token_id=1,
        ))
    model.eval()
    model.save_pretrained(path, safe_serialization=True)
    fast.save_pretrained(path)


def new_actual_worker():
    """Separate module namespace using unmodified actual embedded worker source."""
    name = "native_timing_oracle_worker_" + uuid.uuid4().hex
    spec = importlib.util.spec_from_file_location(name, TORCH_DIR / "worker.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    try:
        spec.loader.exec_module(module)
    except BaseException:
        sys.modules.pop(name, None)
        raise
    return module


class NativeServiceTimingOwnerOracle(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="native-timing-owner-oracle-")
        self.addCleanup(self.directory.cleanup)
        self.package = Path(self.directory.name) / "model"
        build_native_package(self.package)
        self.worker = new_actual_worker()
        self.addCleanup(sys.modules.pop, self.worker.__name__, None)
        # Register teardown before installing instrumentation or loading models.
        self.addCleanup(lambda: self.worker.shutdown_worker())
        self.tracker = owner.install(self.worker, IMPLEMENTATION_DIGEST)

    def load(self, path=None):
        result = self.worker.load_model(
            str(path or self.package), device="cpu", local_files_only=True,
            trust_remote_code=False,
        )
        self.assertEqual(result["device"], "cpu")
        self.assertEqual(result["model_path"], str(path or self.package))
        self.assertIs(type(self.worker._model), GPT2LMHeadModel)
        self.assertIs(type(self.worker._tokenizer), PreTrainedTokenizerFast)
        return result

    def capture(self):
        stamp = self.tracker.capture()
        self.assertIsNotNone(stamp, "real tiny supported installed profile must qualify")
        facts = json.loads(stamp.facts_json)
        self.assertEqual(set(facts), FACT_FIELDS)
        self.assertLessEqual(len(stamp.facts_json.encode("utf-8")), 4096)
        for field, value in facts.items():
            self.assertIsInstance(value, str, field)
            self.assertTrue(value, field)
            self.assertLessEqual(len(value.encode("utf-8")), 256, field)
        for field in STABLE_FIELDS - {"device_id"}:
            self.assertRegex(facts[field], r"^[0-9a-f]{64}$", field)
        self.assertEqual(facts["device_id"], "cpu")
        self.assertEqual(self.tracker.revalidate(stamp), stamp.facts_json)
        return stamp, facts

    @staticmethod
    def stable(facts):
        return {field: facts[field] for field in STABLE_FIELDS}

    def test_actual_reload_preserves_stable_facts_but_not_live_owner(self):
        self.load()
        first, a = self.capture()
        # Repeated observation of one actual resident is not a claim of loader reuse.
        repeated, same = self.capture()
        self.assertEqual(a, same)
        self.worker.unload_model()
        self.assertIsNone(self.tracker.capture())
        self.assertIsNone(self.tracker.revalidate(first))
        self.assertIsNone(self.tracker.revalidate(repeated))
        self.load()
        second, b = self.capture()
        self.assertEqual(self.stable(a), self.stable(b))
        self.assertNotEqual(a["owner_fence"], b["owner_fence"])
        self.assertIsNone(self.tracker.revalidate(first))
        self.assertEqual(self.tracker.revalidate(second), second.facts_json)

    def test_same_content_foreign_reload_and_aba_cannot_restore_old_stamp(self):
        self.load()
        old, a = self.capture()
        self.load()  # Ordinary direct worker replacement with byte-identical content.
        middle, equivalent = self.capture()
        self.assertEqual(self.stable(a), self.stable(equivalent))
        self.assertIsNone(self.tracker.revalidate(old))
        other = Path(self.directory.name) / "other"
        build_native_package(other)
        # Give B a genuinely different installed weight, not a different path label.
        changed = GPT2LMHeadModel.from_pretrained(other, local_files_only=True)
        with torch.no_grad():
            changed.transformer.wte.weight[0, 0].add_(0.5)
        changed.save_pretrained(other, safe_serialization=True)
        del changed
        self.load(other)
        b_stamp, b = self.capture()
        self.assertNotEqual(a["content_fingerprint"], b["content_fingerprint"])
        self.load(self.package)
        final, restored = self.capture()
        self.assertEqual(self.stable(a), self.stable(restored))
        for stale in (old, middle, b_stamp):
            self.assertIsNone(self.tracker.revalidate(stale))
        self.assertEqual(self.tracker.revalidate(final), final.facts_json)
        self.assertNotEqual(a["owner_fence"], restored["owner_fence"])

    def test_native_load_ack_cannot_be_reattached_after_foreign_return_gap(self):
        template = json.loads((TORCH_DIR.parent / "tests" / "fixtures" /
            "pytorch_worker_contract" / "load_transformers_model_request.json").read_text())
        template["request_id"] = "independent-native-ack"
        payload = template["payload"]
        payload["entry_path"] = str(self.package)
        payload["model_source"]["entry_path"] = str(self.package)
        payload["device"] = "cpu"
        payload["trust_policy"]["revision"] = None
        envelope = json.dumps(template)
        response, stamp = self.tracker.load_with_ack(envelope)
        self.assertEqual(json.loads(response)["status"], "ok")
        self.assertIsNotNone(stamp)
        self.assertEqual(self.tracker.revalidate(stamp), stamp.facts_json)
        other = Path(self.directory.name) / "foreign"
        build_native_package(other)
        model = GPT2LMHeadModel.from_pretrained(other, local_files_only=True)
        with torch.no_grad():
            model.transformer.wte.weight[0, 0].add_(0.75)
        model.save_pretrained(other, safe_serialization=True)
        del model
        original_facade = self.worker.load_transformers_model_from_envelope
        entered = []

        def with_foreign_after_ack(raw):
            # Real original A load/ACK, then a different native caller installs B
            # before the original response is returned to load_with_ack.
            result = original_facade(raw)
            entered.append(True)
            with ThreadPoolExecutor(max_workers=1) as pool:
                pool.submit(self.worker.load_model, str(other), device="cpu",
                            local_files_only=True, trust_remote_code=False).result(timeout=10)
            return result

        self.worker.load_transformers_model_from_envelope = with_foreign_after_ack
        try:
            response, stale = self.tracker.load_with_ack(envelope)
        finally:
            self.worker.load_transformers_model_from_envelope = original_facade
        self.assertEqual(entered, [True], "actual return-gap interleaving must execute")
        self.assertEqual(json.loads(response)["status"], "ok")
        self.assertEqual(self.worker.get_loaded_info()["model_path"], str(other))
        self.assertIsNone(stale, "successful A response cannot adopt B's current owner")
        self.assertIsNone(self.tracker.revalidate(stamp))
        response, fresh = self.tracker.load_with_ack(envelope)
        self.assertEqual(json.loads(response)["status"], "ok")
        self.assertIsNotNone(fresh)
        self.assertEqual(self.tracker.revalidate(fresh), fresh.facts_json)

    def test_installed_weight_change_cannot_hide_behind_same_path(self):
        self.load()
        old, original = self.capture()
        with torch.no_grad():
            self.worker._model.transformer.wte.weight[0, 0].add_(0.25)
        self.assertIsNone(self.tracker.revalidate(old))
        self.worker._model.save_pretrained(self.package, safe_serialization=True)
        self.load()
        _new, changed = self.capture()
        self.assertNotEqual(original["content_fingerprint"], changed["content_fingerprint"])

    def test_native_tokenizer_template_and_effective_config_are_not_ignored(self):
        for mutation in ("tokenizer", "template", "model-config", "generation-config"):
            with self.subTest(mutation=mutation):
                self.load()
                old, original = self.capture()
                if mutation == "tokenizer":
                    vocab = self.worker._tokenizer.get_vocab()
                    value = vocab.pop("delta")
                    vocab["changed"] = value
                    self.worker._tokenizer.backend_tokenizer.model = models.WordLevel(
                        vocab, unk_token="<unk>")
                elif mutation == "template":
                    self.worker._tokenizer.chat_template = (
                        "{{ messages | map(attribute='content') | join(' ') }}")
                elif mutation == "model-config":
                    self.worker._model.config.use_cache = not self.worker._model.config.use_cache
                else:
                    self.worker._model.generation_config.max_length += 1
                self.assertIsNone(self.tracker.revalidate(old))
                self.worker._tokenizer.save_pretrained(self.package)
                self.worker._model.save_pretrained(self.package, safe_serialization=True)
                self.load()
                _new, changed = self.capture()
                self.assertNotEqual(self.stable(original), self.stable(changed))

    def test_failed_native_replacement_does_not_republish_previous_owner(self):
        self.load()
        old, original = self.capture()
        with self.assertRaises(FileNotFoundError):
            self.worker.load_model(
                str(Path(self.directory.name) / "missing"), device="cpu",
                local_files_only=True, trust_remote_code=False)
        self.assertIsNone(self.worker.get_loaded_info())
        self.assertIsNone(self.tracker.capture())
        self.assertIsNone(self.tracker.revalidate(old))
        self.load()
        _new, restored = self.capture()
        self.assertEqual(self.stable(original), self.stable(restored))
        self.assertNotEqual(original["owner_fence"], restored["owner_fence"])

    def test_shutdown_and_label_only_metadata_do_not_create_owner_facts(self):
        self.load()
        old, _facts = self.capture()
        self.worker.shutdown_worker()
        self.assertIsNone(self.tracker.capture())
        self.assertIsNone(self.tracker.revalidate(old))
        # Path/type/device labels are not installed model/tokenizer authority.
        self.worker._model_path = self.package
        self.worker._model_type = "text-generation"
        self.worker._device = torch.device("cpu")
        self.assertIsNone(self.tracker.capture())

    def test_install_is_idempotent_and_preserves_actual_function_results(self):
        wrappers = tuple(getattr(self.worker, name)
                         for name in ("load_model", "unload_model", "shutdown_worker"))
        self.assertIs(owner.install(self.worker, IMPLEMENTATION_DIGEST), self.tracker)
        self.assertEqual(wrappers, tuple(getattr(self.worker, name)
                         for name in ("load_model", "unload_model", "shutdown_worker")))
        result = self.load()
        self.assertEqual(set(result), {"model_path", "model_type", "device"})
        self.assertIsNone(self.worker.unload_model())
        self.assertIsNone(self.worker.shutdown_worker())
        with self.assertRaises(FileNotFoundError):
            self.worker.load_model(str(self.package / "missing"), device="cpu",
                                   local_files_only=True, trust_remote_code=False)

    def test_tracker_and_stamps_do_not_retain_unloaded_native_objects(self):
        self.load()
        stamp, _facts = self.capture()
        model_ref = weakref.ref(self.worker._model)
        tokenizer_ref = weakref.ref(self.worker._tokenizer)
        self.worker.unload_model()
        gc.collect()
        self.assertIsNone(model_ref())
        self.assertIsNone(tokenizer_ref())
        self.assertIsNone(self.tracker.revalidate(stamp))

    def test_unsupported_native_tokenizer_shapes_and_bounds_refuse(self):
        for shape in ("normalizer", "decoder", "pretokenizer", "bpe",
                      "large-vocab", "large-token", "sentinel-length"):
            with self.subTest(shape=shape):
                self.load()
                old, _facts = self.capture()
                tokenizer = self.worker._tokenizer
                if shape == "normalizer":
                    tokenizer.backend_tokenizer.normalizer = normalizers.Lowercase()
                elif shape == "decoder":
                    tokenizer.backend_tokenizer.decoder = decoders.WordPiece()
                elif shape == "pretokenizer":
                    tokenizer.backend_tokenizer.pre_tokenizer = pre_tokenizers.ByteLevel()
                elif shape == "bpe":
                    tokenizer.backend_tokenizer.model = models.BPE(
                        {"<unk>": 0, "hello": 1}, [], unk_token="<unk>")
                elif shape == "large-vocab":
                    tokenizer.backend_tokenizer.model = models.WordLevel(
                        {"<unk>": 0, **{f"token{i}": i + 1 for i in range(4096)}},
                        unk_token="<unk>")
                elif shape == "large-token":
                    tokenizer.backend_tokenizer.model = models.WordLevel(
                        {"<unk>": 0, "x" * 65537: 1}, unk_token="<unk>")
                else:
                    tokenizer.model_max_length = 10**30
                self.assertIsNone(self.tracker.capture())
                self.assertIsNone(self.tracker.revalidate(old))
                self.assertIsNotNone(self.worker.get_loaded_info())

    def test_unsupported_native_tensor_hook_and_training_state_refuse(self):
        for shape in ("oversized", "noncontiguous", "hook", "training"):
            with self.subTest(shape=shape):
                self.load()
                old, _facts = self.capture()
                model = self.worker._model
                handle = None
                if shape == "oversized":
                    model.register_buffer("oracle_large_buffer", torch.zeros(2_097_153))
                elif shape == "noncontiguous":
                    weight = model.transformer.wte.weight
                    model.transformer.wte.weight = torch.nn.Parameter(
                        weight.detach().transpose(0, 1).contiguous().transpose(0, 1))
                    self.assertFalse(model.transformer.wte.weight.is_contiguous())
                elif shape == "hook":
                    handle = model.register_forward_hook(lambda _module, _args, output: output)
                else:
                    model.train()
                try:
                    self.assertIsNone(self.tracker.capture())
                    self.assertIsNone(self.tracker.revalidate(old))
                    self.assertIsNotNone(self.worker.get_loaded_info())
                finally:
                    if handle is not None:
                        handle.remove()

    def test_effective_torch_thread_setting_is_revalidated(self):
        self.load()
        old, original = self.capture()
        saved = torch.get_num_threads()
        try:
            torch.set_num_threads(1 if saved != 1 else 2)
            self.assertIsNone(self.tracker.revalidate(old))
            current = self.tracker.capture()
            if current is not None:
                changed = json.loads(current.facts_json)
                self.assertNotEqual(self.stable(original), self.stable(changed))
        finally:
            torch.set_num_threads(saved)

    def test_native_generation_exhaustion_never_wraps_or_restores_old_owner(self):
        self.load()
        old, _facts = self.capture()
        # Explicitly private test-only counter seed; never native positive authority.
        self.assertTrue(hasattr(self.tracker, "_generation"))
        self.tracker._generation = (1 << 64) - 1
        result = self.load()
        self.assertEqual(result["device"], "cpu")  # Refusal must not fail inference load.
        self.assertIsNone(self.tracker.capture())
        self.assertIsNone(self.tracker.revalidate(old))
        self.load()
        self.assertIsNone(self.tracker.capture())


if __name__ == "__main__":
    unittest.main()
