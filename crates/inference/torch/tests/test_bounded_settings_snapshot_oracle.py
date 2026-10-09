"""Independent native component units; no encoding, model, Torch or Transformers.

Authored outside the repository. The consumer cases use a controlled Python
wrapper around a real native tokenizer, never a fabricated native receipt.
"""
import hashlib
import importlib.util
import math
import os
import struct
import sys
import types
import unittest
from pathlib import Path
from unittest.mock import patch

import tokenizers
from tokenizers import AddedToken, Tokenizer, models, pre_tokenizers

API = "_pantograph_settings_snapshot_v1"
PREFIX = b"pantograph-cpython-3.12.3-tokenizer-settings.v1\0"
COPY = 4 * 1024 * 1024
WORK = 5 * 4 * 4096 * 14 * (16384 + 1) + 32 * COPY
HELPER = Path(__file__).resolve().parents[1] / "service_timing_owner.py"


def native():
    value = Tokenizer(models.WordLevel({"[UNK]": 0, "a": 1}, unk_token="[UNK]"))
    value.pre_tokenizer = pre_tokenizers.WhitespaceSplit()
    return value


def decode(payload):
    """Independent structural reader, not the native sorting/inspection code."""
    cursor = len(PREFIX)

    def take(n):
        nonlocal cursor
        data = payload[cursor:cursor + n]
        assert len(data) == n
        cursor += n
        return data

    def read():
        tag = take(1)
        if tag == b"n":
            return None
        if tag in (b"t", b"f"):
            return tag == b"t"
        if tag == b"i":
            return int.from_bytes(take(16), "little", signed=True)
        if tag == b"r":
            return struct.unpack("<d", take(8))[0]
        size = int.from_bytes(take(8), "little")
        if tag == b"s":
            return take(size).decode("utf-8")
        if tag == b"a":
            return [read() for _ in range(size)]
        assert tag == b"d"
        return {read(): read() for _ in range(size)}

    result = read()
    assert cursor == len(payload)
    return result


class Opaque:
    calls = 0

    def bomb(self, *args):
        type(self).calls += 1
        raise AssertionError("opaque callback executed")

    __iter__ = __repr__ = __str__ = __sizeof__ = __eq__ = bomb

    def __hash__(self):
        type(self).calls += 1
        return 42


class ControlledFast:
    def __init__(self, backend):
        self._tokenizer = backend
        self.model_max_length = 16
        self.init_kwargs = {}

    @property
    def backend_tokenizer(self):
        return self._tokenizer


class SettingsOracle(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        assert tokenizers.__version__ == "0.21.4"
        operation = vars(Tokenizer).get(API)
        if operation is None:
            if os.environ.get("PANTOGRAPH_REQUIRE_BOUNDED_TOKENIZER_PROVIDER") == "1":
                raise RuntimeError("required patched settings capability is missing")
            raise unittest.SkipTest("optional bounded settings provider is not installed")
        assert type(operation) is types.MethodDescriptorType
        assert operation.__name__ == API and operation.__objclass__ is Tokenizer
        spec = importlib.util.spec_from_file_location(
            "independent_settings_owner", HELPER
        )
        cls.helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.helper)

    def setUp(self):
        self.assertNotIn("torch", sys.modules)
        self.assertNotIn("transformers", sys.modules)
        self.native = native()

    def tearDown(self):
        self.assertNotIn("torch", sys.modules)
        self.assertNotIn("transformers", sys.modules)

    def receipt(self, fields, accepted=True):
        result = vars(Tokenizer)[API](self.native, fields)
        self.assertEqual(tuple(type(v) for v in result), (bool, bytes, int, int))
        good, payload, work, copied = result
        self.assertIs(good, accepted)
        self.assertTrue(0 <= work <= WORK)
        self.assertTrue(0 <= copied <= COPY)
        if good:
            self.assertTrue(payload.startswith(PREFIX))
            self.assertGreaterEqual(copied, 3 * len(payload))
        else:
            self.assertEqual(payload, b"")
        return result

    def test_primitive_binary_facts(self):
        fields = {"init_kwargs": {}, "values": [None, True, False, -(1 << 64),
                   (1 << 64) - 1, 1.25, "é🙂"]}
        self.assertEqual(decode(self.receipt(fields)[1]), fields)

    def test_order_is_canonical_without_input_mutation(self):
        a = {"z": {"b": 2, "a": 1}, "a": [3, 4], "init_kwargs": {}}
        b = {"init_kwargs": {}, "a": [3, 4], "z": {"a": 1, "b": 2}}
        order = tuple(a)
        self.assertEqual(self.receipt(a)[1], self.receipt(b)[1])
        self.assertEqual(tuple(a), order)
        self.assertEqual(self.receipt(a), self.receipt(a))

    def test_list_tuple_equivalence_and_changed_values(self):
        self.assertEqual(self.receipt({"x": [1, 2]})[1], self.receipt({"x": (1, 2)})[1])
        self.assertNotEqual(self.receipt({"x": [1, 2]})[1], self.receipt({"x": [2, 1]})[1])

    def test_locators_excluded_only_at_declared_scopes(self):
        bomb = Opaque()
        fields = {"_tokenizer": bomb, "name_or_path": bomb, "deprecation_warnings": bomb,
                  "init_kwargs": {"name_or_path": bomb, "tokenizer_file": bomb,
                                  "_commit_hash": bomb, "ordinary": 1}}
        Opaque.calls = 0
        self.assertEqual(decode(self.receipt(fields)[1]), {"init_kwargs": {"ordinary": 1}})
        self.assertEqual(Opaque.calls, 0)
        self.assertEqual(decode(self.receipt({"other": {"name_or_path": "kept"}})[1]),
                         {"other": {"name_or_path": "kept"}})

    def test_unknown_key_never_hashes_compares_or_formats(self):
        fields = {Opaque(): 1}
        Opaque.calls = 0
        self.receipt(fields, False)
        self.assertEqual(Opaque.calls, 0)

    def test_opaque_values_added_tokens_and_sets_refuse(self):
        for value in [Opaque(), AddedToken("x"), set(), frozenset()]:
            with self.subTest(kind=type(value).__name__):
                Opaque.calls = 0
                self.receipt({"x": value}, False)
                self.assertEqual(Opaque.calls, 0)

    def test_exact_types_required_before_subclass_callbacks(self):
        class BadDict(dict):
            __sizeof__ = __iter__ = Opaque.bomb
        class BadString(str):
            __str__ = Opaque.bomb
        for value in [BadDict(), {"x": BadDict()}, {BadString("x"): 1}]:
            self.receipt(value, False)

    def test_key_types_and_init_kwargs_type(self):
        self.receipt({1: "integer", "1": "string"})
        for fields in [{True: 1}, {1.0: 1}, {None: 1}, {"init_kwargs": []}]:
            self.receipt(fields, False)

    def test_surrogates_refuse_without_conversion_leak(self):
        for fields in [{"x": "\ud800"}, {"\udfff": 1}]:
            self.receipt(fields, False)

    def test_integer_and_float_limits(self):
        for value in [-(1 << 64) - 1, 1 << 64, 1 << 100000, math.nan, math.inf, -math.inf]:
            self.receipt({"x": value}, False)
        self.assertNotEqual(self.receipt({"x": 0.0})[1], self.receipt({"x": -0.0})[1])

    def test_string_and_aggregate_limits(self):
        self.receipt({"x": "a" * 16383})  # key + text exactly 16,384 codepoints
        self.receipt({"x": "a" * 16384}, False)  # aggregate worst-case UTF-8 admission
        self.receipt({"x": "a" * 16385}, False)
        self.receipt({"x": "🙂" * 4096})
        self.receipt({"x": "🙂" * 4097}, False)  # actual per-string bytes >16KiB

    def test_oversize_excluded_key_still_qualified(self):
        self.receipt({"name_or_path": Opaque(), "x" * 16385: 1}, False)

    def test_node_budget_not_just_container_length(self):
        self.receipt({"x": [None] * 4093})
        self.receipt({"x": [None] * 4094}, False)
        self.receipt({"x": [None] * 4097}, False)

    def test_depth_and_cycles_refuse(self):
        value = None
        for _ in range(7):
            value = [value]
        self.receipt({"x": value})
        self.receipt({"x": [value]}, False)
        cyclic = []
        cyclic.append(cyclic)
        self.receipt({"x": cyclic}, False)

    def test_retained_dictionary_storage_refuses_before_payload(self):
        retained = {i: None for i in range(20000)}
        for i in range(1, 20000):
            del retained[i]
        self.assertGreater(retained.__sizeof__(), 256 * 1024)
        self.assertEqual(len(retained), 1)
        receipt = self.receipt(retained, False)
        self.assertLess(receipt[3], 1024)
        self.receipt({0: None})

    def test_shared_instance_dictionary_storage_allowance(self):
        class Fields:
            def __init__(self):
                self.a = 1
                self.b = "x"
        instances = [Fields() for _ in range(64)]
        self.assertEqual(decode(self.receipt(vars(instances[0]))[1]), {"a": 1, "b": "x"})

    def consumer(self, wrapper):
        module = types.ModuleType("transformers")
        module.PreTrainedTokenizerFast = ControlledFast
        with patch.dict(sys.modules, {"transformers": module}):
            return self.helper._tokenizer_state(wrapper)

    def test_actual_consumer_uses_both_native_receipts_without_legacy_codec(self):
        wrapper = ControlledFast(self.native)
        a = vars(Tokenizer)["_pantograph_wordlevel_snapshot_v1"](self.native)[1]
        b = self.receipt(vars(wrapper))[1]
        expected = hashlib.sha256(b"installed-wordlevel-bounded-components.v3\0" + a + b).hexdigest()
        with patch.object(self.helper, "_canonical", side_effect=AssertionError("legacy codec called")):
            self.assertEqual(self.consumer(wrapper), expected)

    def test_consumer_refusal_has_no_legacy_fallback(self):
        wrapper = ControlledFast(self.native)
        wrapper.unknown = Opaque()
        Opaque.calls = 0
        with patch.object(self.helper, "_canonical", side_effect=AssertionError("fallback called")):
            with self.assertRaises(self.helper._Unknown):
                self.consumer(wrapper)
        self.assertEqual(Opaque.calls, 0)

    def test_consumer_mutation_changes_identity_and_locators_do_not(self):
        wrapper = ControlledFast(self.native)
        first = self.consumer(wrapper)
        wrapper.name_or_path = "new/location"
        wrapper.init_kwargs["_commit_hash"] = "different label"
        self.assertEqual(first, self.consumer(wrapper))
        wrapper.model_max_length = 17
        self.assertNotEqual(first, self.consumer(wrapper))
        wrapper.model_max_length = 16
        self.native.model.unk_token = "new unknown"
        self.assertNotEqual(first, self.consumer(wrapper))


if __name__ == "__main__":
    unittest.main(verbosity=2)
