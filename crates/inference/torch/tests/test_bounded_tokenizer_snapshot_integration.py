"""Real Transformers wrapper integration; no model, encode or inference.

This optional provider is required for explicit native qualification. The
independent oracle tests the native kernel without importing Transformers;
these cases additionally exercise the exact production wrapper/settings seam.
"""
import importlib.util
import os
from pathlib import Path
import unittest

import tokenizers
from transformers import PreTrainedTokenizerFast


class BoundedTokenizerWrapperIntegration(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        if "_pantograph_wordlevel_snapshot_v1" not in vars(tokenizers.Tokenizer):
            if os.environ.get("PANTOGRAPH_REQUIRE_BOUNDED_TOKENIZER_PROVIDER") == "1":
                raise RuntimeError("required patched native provider is missing")
            raise unittest.SkipTest("optional patched native provider is absent")
        path = Path(__file__).resolve().parents[1] / "service_timing_owner.py"
        spec = importlib.util.spec_from_file_location("bounded_real_wrapper_owner", path)
        cls.helper = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.helper)

    def wrapper(self):
        native = tokenizers.Tokenizer(tokenizers.models.WordLevel(
            {"unk": 0, "hello": 1}, unk_token="unk"))
        native.pre_tokenizer = tokenizers.pre_tokenizers.Whitespace()
        return PreTrainedTokenizerFast(tokenizer_object=native,
                                       unk_token="unk", model_max_length=16)

    def test_actual_wrapper_has_stable_content_and_observes_native_mutation(self):
        one, two = self.wrapper(), self.wrapper()
        stamp = self.helper._tokenizer_state(one)
        self.assertEqual(stamp, self.helper._tokenizer_state(two))
        one.backend_tokenizer.model = tokenizers.models.WordLevel(
            {"unk": 0, "changed": 1}, unk_token="unk")
        self.assertNotEqual(stamp, self.helper._tokenizer_state(one))

    def test_native_refusal_remains_unknown_at_actual_wrapper(self):
        wrapper = self.wrapper()
        wrapper.backend_tokenizer.model = tokenizers.models.WordLevel(
            {"unk": 0, "sparse": 0xffffffff}, unk_token="unk")
        with self.assertRaises(self.helper._Unknown):
            self.helper._tokenizer_state(wrapper)

    def test_python_settings_remain_part_of_installed_identity(self):
        wrapper = self.wrapper()
        stamp = self.helper._tokenizer_state(wrapper)
        wrapper.chat_template = "{{ messages }}"
        self.assertNotEqual(stamp, self.helper._tokenizer_state(wrapper))


if __name__ == "__main__":
    unittest.main()
