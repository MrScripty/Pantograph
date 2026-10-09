"""Independent native Tokenizers/component-consumer units, without a model.

An ordinary unpatched installation skips this explicitly optional provider.
Qualification must set PANTOGRAPH_REQUIRE_BOUNDED_TOKENIZER_PROVIDER=1, making
missing capability a failure. These cases do not encode text, import Torch,
load Transformers, perform inference, or establish dispatch performance.
"""

import hashlib
import importlib.util
import os
from contextlib import ExitStack
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import patch

import tokenizers
from tokenizers import AddedToken, Tokenizer, decoders, models, normalizers, pre_tokenizers


API = "_pantograph_wordlevel_snapshot_v1"
PREFIX = b"pantograph-tokenizers-0.21.4-wordlevel-snapshot.v1\0"
ENTRIES = 4096
STRING = 16384
COPY_LIMIT = 4 * 1024 * 1024
WORK_LIMIT = 5 * 4 * ENTRIES * 14 * (STRING + 1) + 32 * COPY_LIMIT


def owner_helper():
    path = Path(__file__).resolve().parents[1] / "service_timing_owner.py"
    spec = importlib.util.spec_from_file_location("independent_bounded_owner_helper", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def native(vocab=None, unknown="[UNK]"):
    tokenizer = Tokenizer(models.WordLevel(
        {"[UNK]": 0, "a": 1} if vocab is None else vocab,
        unk_token=unknown,
    ))
    tokenizer.pre_tokenizer = pre_tokenizers.WhitespaceSplit()
    return tokenizer


class ControlledFastTokenizer:
    """Only the Python wrapper/settings seam is controlled; native data is real."""

    def __init__(self, tokenizer):
        self._tokenizer = tokenizer
        self.model_max_length = 16
        self.init_kwargs = {}

    @property
    def backend_tokenizer(self):
        return self._tokenizer


def controlled_transformers():
    module = types.ModuleType("transformers")
    module.PreTrainedTokenizerFast = ControlledFastTokenizer
    return module


class BoundedTokenizerSnapshotOracle(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if API not in vars(Tokenizer):
            if os.environ.get("PANTOGRAPH_REQUIRE_BOUNDED_TOKENIZER_PROVIDER") == "1":
                raise RuntimeError("required patched native tokenizer capability is missing")
            raise unittest.SkipTest("optional bounded Tokenizers provider is not installed")
        if tokenizers.__version__ != "0.21.4":
            raise RuntimeError("qualification requires the pinned Tokenizers 0.21.4 provider")
        cls.helper = owner_helper()

    def setUp(self):
        self.assertNotIn("torch", sys.modules)
        self.assertNotIn("transformers", sys.modules)

    def tearDown(self):
        self.assertNotIn("torch", sys.modules)
        self.assertNotIn("transformers", sys.modules)

    def inspect(self, tokenizer):
        receipt = vars(Tokenizer)[API](tokenizer)
        self.assertIs(type(receipt), tuple)
        self.assertEqual(len(receipt), 4)
        accepted, payload, work, copied = receipt
        self.assertIs(type(accepted), bool)
        self.assertIs(type(payload), bytes)
        self.assertIs(type(work), int)
        self.assertIs(type(copied), int)
        self.assertGreaterEqual(work, 0)
        self.assertLessEqual(work, WORK_LIMIT)
        self.assertGreaterEqual(copied, 0)
        self.assertLessEqual(copied, COPY_LIMIT)
        if accepted:
            self.assertTrue(payload.startswith(PREFIX))
            self.assertLessEqual(len(payload), COPY_LIMIT)
            self.assertGreaterEqual(copied, 2 * len(payload))
        else:
            self.assertEqual(payload, b"")
        return receipt

    def test_same_object_receipt_is_deterministic_and_order_is_canonical(self):
        first = native({"[UNK]": 0, "left": 1, "right": 2})
        second = native({"right": 2, "left": 1, "[UNK]": 0})
        receipt = self.inspect(first)
        self.assertTrue(receipt[0])
        self.assertEqual(receipt, self.inspect(first))
        other = self.inspect(second)
        self.assertTrue(other[0])
        self.assertEqual(receipt[1], other[1])
        # Different randomized map layouts can legitimately spend different
        # comparison counts. Only identical object receipts must match exactly.

    def test_native_model_and_unknown_token_mutations_change_payload(self):
        tokenizer = native()
        baseline = self.inspect(tokenizer)
        self.assertTrue(baseline[0])
        tokenizer.model = models.WordLevel({"[UNK]": 0, "b": 1}, unk_token="[UNK]")
        changed = self.inspect(tokenizer)
        self.assertTrue(changed[0])
        self.assertNotEqual(changed[1], baseline[1])
        tokenizer.model.unk_token = "absent-but-bounded"
        unknown = self.inspect(tokenizer)
        self.assertTrue(unknown[0])
        self.assertNotEqual(unknown[1], changed[1])

    def test_sparse_and_maximum_ids_refuse_before_variable_copy(self):
        for index in (100, (1 << 32) - 1):
            with self.subTest(index=index):
                receipt = self.inspect(native({"[UNK]": 0, "a": index}))
                self.assertFalse(receipt[0])
                self.assertLessEqual(receipt[3], ENTRIES)

    def test_duplicate_base_ids_do_not_hide_missing_dense_id(self):
        receipt = self.inspect(native({"[UNK]": 0, "alias": 0}))
        self.assertFalse(receipt[0])

    def test_added_overlap_and_added_only_have_actual_dense_semantics(self):
        overlap = native()
        overlap.add_tokens([AddedToken("a", normalized=False)])
        overlap.add_tokens([AddedToken("extra", normalized=False)])
        self.assertEqual(overlap.get_vocab_size(False), 2)
        self.assertEqual(overlap.get_vocab_size(True), 3)
        self.assertTrue(self.inspect(overlap)[0])
        added_only = native({}, unknown="outside")
        added_only.add_tokens([AddedToken("only", normalized=False)])
        self.assertEqual(added_only.get_vocab_size(False), 0)
        self.assertEqual(added_only.get_vocab_size(True), 1)
        self.assertTrue(self.inspect(added_only)[0])

    def test_oversized_unknown_vocab_and_padding_strings_refuse(self):
        excessive = "x" * (STRING + 1)
        fixtures = [native(unknown=excessive), native({"[UNK]": 0, excessive: 1})]
        padded = native()
        padded.enable_padding(pad_token=excessive)
        fixtures.append(padded)
        for tokenizer in fixtures:
            with self.subTest(tokenizer=type(tokenizer)):
                receipt = self.inspect(tokenizer)
                self.assertFalse(receipt[0])
                # Only fixed density scratch, if counted by the implementation,
                # may be materialized before oversized native data is rejected.
                self.assertLessEqual(receipt[3], ENTRIES)

    def test_aggregate_text_and_entry_limits_refuse(self):
        aggregate = {"[UNK]": 0}
        aggregate.update({str(i) + "x" * 12000: i + 1 for i in range(5)})
        self.assertFalse(self.inspect(native(aggregate))[0])
        too_many = {str(i): i for i in range(ENTRIES + 1)}
        self.assertFalse(self.inspect(native(too_many, unknown="0"))[0])

    def test_normalized_current_and_historical_tokens_refuse(self):
        tokenizer = native()
        tokenizer.add_tokens([AddedToken("extra", normalized=True)])
        self.assertFalse(self.inspect(tokenizer)[0])
        tokenizer.add_tokens([AddedToken("extra", normalized=False)])
        self.assertFalse(tokenizer.get_added_tokens_decoder()[2].normalized)
        # A later unnormalized decoder entry does not erase the prior matcher
        # history. The limited profile must still refuse that history.
        self.assertFalse(self.inspect(tokenizer)[0])

    def test_ordered_unnormalized_history_changes_identity(self):
        first = native()
        first.add_tokens([AddedToken("extra", normalized=False, lstrip=False)])
        second = native()
        second.add_tokens([AddedToken("extra", normalized=False, lstrip=True)])
        second.add_tokens([AddedToken("extra", normalized=False, lstrip=False)])
        self.assertEqual(first.get_vocab(), second.get_vocab())
        self.assertEqual(first.get_added_tokens_decoder()[2].lstrip,
                         second.get_added_tokens_decoder()[2].lstrip)
        a, b = self.inspect(first), self.inspect(second)
        self.assertTrue(a[0])
        self.assertTrue(b[0])
        self.assertNotEqual(a[1], b[1])

    def test_special_encoding_padding_and_truncation_change_identity(self):
        tokenizer = native()
        first = self.inspect(tokenizer)
        tokenizer.encode_special_tokens = True
        second = self.inspect(tokenizer)
        self.assertTrue(second[0])
        self.assertNotEqual(first[1], second[1])
        tokenizer.enable_padding(length=4, pad_id=0, pad_token="PAD", direction="left")
        third = self.inspect(tokenizer)
        self.assertTrue(third[0])
        self.assertNotEqual(second[1], third[1])
        tokenizer.enable_truncation(max_length=3, stride=0, direction="left")
        fourth = self.inspect(tokenizer)
        self.assertTrue(fourth[0])
        self.assertNotEqual(third[1], fourth[1])

    def test_unsupported_components_refuse_without_custom_callback(self):
        touched = []

        class Custom:
            def pre_tokenize(self, _):
                touched.append(True)
                raise AssertionError("inspection must not execute custom pretokenizer")

        fixtures = []
        sequence = native()
        sequence.pre_tokenizer = pre_tokenizers.Sequence([pre_tokenizers.WhitespaceSplit()])
        fixtures.append(sequence)
        custom = native()
        custom.pre_tokenizer = pre_tokenizers.PreTokenizer.custom(Custom())
        fixtures.append(custom)
        normalized = native()
        normalized.normalizer = normalizers.Lowercase()
        fixtures.append(normalized)
        decoded = native()
        decoded.decoder = decoders.WordPiece()
        fixtures.append(decoded)
        unsupported_model = Tokenizer(models.BPE())
        unsupported_model.pre_tokenizer = pre_tokenizers.WhitespaceSplit()
        fixtures.append(unsupported_model)
        for tokenizer in fixtures:
            self.assertFalse(self.inspect(tokenizer)[0])
        self.assertEqual(touched, [])

    def forbid_legacy(self):
        def forbidden(*_args, **_kwargs):
            raise AssertionError("bounded consumer invoked a legacy native getter/export")

        stack = ExitStack()
        for name in ("model", "pre_tokenizer", "normalizer", "post_processor", "decoder",
                     "padding", "truncation"):
            stack.enter_context(patch.object(Tokenizer, name, property(forbidden)))
        for name in ("get_vocab_size", "id_to_token", "to_str", "get_vocab"):
            stack.enter_context(patch.object(Tokenizer, name, forbidden))
        return stack

    def test_actual_consumer_uses_provider_before_all_legacy_getters(self):
        tokenizer = native()
        receipt = self.inspect(tokenizer)
        self.assertTrue(receipt[0])
        wrapper = ControlledFastTokenizer(tokenizer)
        settings = vars(Tokenizer)["_pantograph_settings_snapshot_v1"](tokenizer, vars(wrapper))
        self.assertTrue(settings[0])
        expected = hashlib.sha256(b"installed-wordlevel-bounded-components.v3\0")
        expected.update(receipt[1])
        expected.update(settings[1])
        with patch.dict(sys.modules, {"transformers": controlled_transformers()}):
            with self.forbid_legacy():
                actual = self.helper._tokenizer_state(wrapper)
        self.assertEqual(actual, expected.hexdigest())

    def test_actual_provider_refusal_never_falls_back_to_legacy(self):
        tokenizer = native({"[UNK]": 0, "sparse": 100})
        wrapper = ControlledFastTokenizer(tokenizer)
        with patch.dict(sys.modules, {"transformers": controlled_transformers()}):
            with self.forbid_legacy():
                with self.assertRaises(self.helper._Unknown):
                    self.helper._tokenizer_state(wrapper)

    def test_missing_api_explicitly_returns_advisory_legacy_sentinel(self):
        fake = types.ModuleType("tokenizers")
        fake.Tokenizer = type("UnpatchedTokenizer", (), {})
        fake.__version__ = "0.21.4"
        with patch.dict(sys.modules, {"tokenizers": fake}):
            self.assertIsNone(self.helper._bounded_tokenizer_native_state(object()))

    def test_unknown_api_version_refuses_before_descriptor_invocation(self):
        calls = []

        class WrongVersion:
            def _pantograph_wordlevel_snapshot_v1(self):
                calls.append(True)
                raise AssertionError("wrong-version native API must not execute")

        fake = types.ModuleType("tokenizers")
        fake.Tokenizer = WrongVersion
        fake.__version__ = "0.23.1"
        with patch.dict(sys.modules, {"tokenizers": fake}):
            with self.assertRaises(self.helper._Unknown):
                self.helper._bounded_tokenizer_native_state(WrongVersion())
        self.assertEqual(calls, [])

    def test_python_capability_substitution_refuses_before_callback(self):
        calls = []

        def fabricated(_native):
            calls.append(True)
            return True, PREFIX, 0, 0

        tokenizer = native()
        with patch.object(Tokenizer, API, fabricated):
            with self.assertRaises(self.helper._Unknown):
                self.helper._bounded_tokenizer_native_state(tokenizer)
        self.assertEqual(calls, [])

    def test_other_native_descriptor_cannot_impersonate_snapshot_capability(self):
        class Custom:
            def pre_tokenize(self, _pretok):
                raise AssertionError("no tokenization is permitted")

        tokenizer = native()
        tokenizer.pre_tokenizer = pre_tokenizers.PreTokenizer.custom(Custom())
        # to_str would enter the serializer and fail on the custom component;
        # id_to_token would fail its missing argument. Neither native error is
        # the helper's required pre-invocation capability refusal.
        for descriptor in (vars(Tokenizer)["to_str"], vars(Tokenizer)["id_to_token"]):
            self.assertIs(type(descriptor), types.MethodDescriptorType)
            with patch.object(Tokenizer, API, descriptor):
                with self.assertRaises(self.helper._Unknown):
                    self.helper._bounded_tokenizer_native_state(tokenizer)


if __name__ == "__main__":
    unittest.main(verbosity=2)
