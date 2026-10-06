"""PR58 omission compatibility against exact main and installed CPU native code."""

import subprocess
from types import ModuleType
import unittest
from unittest import mock

import torch
from transformers.cache_utils import DynamicCache

from test_autoregressive import (
    autoregressive as candidate, TinyGenerationModel, TokenizerFixture,
    worker_text_functions,
)

MAIN_SHA = "763e8d4b13ba311ff245c75a370e56dafb417d8e"
main = ModuleType("pantograph_main_autoregressive")
exec(compile(subprocess.check_output(
    ["git", "show", f"{MAIN_SHA}:crates/inference/torch/autoregressive.py"], text=True,
), f"{MAIN_SHA}:autoregressive.py", "exec"), main.__dict__)


class EOSDecoder(TokenizerFixture):
    eos_token_id = 2

    def decode(self, tokens, skip_special_tokens=True):
        return ",".join(str(int(token)) for token in tokens
                        if not skip_special_tokens or int(token) != self.eos_token_id)


def model_fixture(token, minimum=0):
    model = TinyGenerationModel()
    model.generation_config.eos_token_id = [1, 3]
    model.generation_config.min_new_tokens = minimum
    model.logits = torch.tensor([0.0, 1.0, 2.0, 3.0])
    model.logits[token] = 10
    return model


class PR58CompatibilityTests(unittest.TestCase):
    def test_omitted_and_zero_minimum_manual_stops_match_main_tokenizer_eos(self):
        for predicted in [2, 3]:
            for authored in [None, 0]:
                for route in ["stream", "fresh", "continued"]:
                    with self.subTest(predicted=predicted, authored=authored, route=route):
                        def execute(module, kwargs):
                            args = (model_fixture(predicted), EOSDecoder(), "cpu", "prompt", 4, 0, 1)
                            if route == "stream":
                                return list(module._generate_autoregressive_streaming(*args, **kwargs))
                            if route == "fresh":
                                return module._generate_sdar_cached(*args, **kwargs)[:2]
                            return module._continue_sdar_cached(
                                *args, [0, 1], DynamicCache(), **kwargs)[:2]
                        expected = execute(main, {})
                        actual = execute(candidate, {"min_new_tokens": authored})
                        self.assertEqual(actual, expected)

    def test_inherited_infeasible_native_minimum_matches_main_including_forced_eos(self):
        for forced_eos in [None, 3]:
            for route in ["helper", "worker"]:
                with self.subTest(forced_eos=forced_eos, route=route):
                    def model():
                        result = model_fixture(3, minimum=10)
                        result.generation_config.forced_eos_token_id = forced_eos
                        return result
                    args = (model(), EOSDecoder(), "cpu", "prompt", 2, 0, 1)
                    expected = main._generate_autoregressive(*args)
                    if route == "helper":
                        actual = candidate._generate_autoregressive(model(), *args[1:])
                    else:
                        worker = worker_text_functions()
                        worker["_model"] = model()
                        worker["_tokenizer"] = EOSDecoder()
                        worker["_generate_autoregressive"] = candidate._generate_autoregressive
                        actual = worker["generate"]("prompt", max_tokens=2, temperature=0)
                    self.assertEqual(actual, expected)

    def test_inherited_manual_minimum_is_capped_but_authored_minimum_stays_strict(self):
        for route in ["stream", "fresh", "continued"]:
            with self.subTest(route=route):
                args = (model_fixture(3, minimum=10), EOSDecoder(), "cpu", "prompt", 2, 0, 1)
                if route == "stream":
                    result = list(candidate._generate_autoregressive_streaming(*args))
                    self.assertEqual(len(result), 2)
                    with self.assertRaisesRegex(ValueError, "must not exceed"):
                        list(candidate._generate_autoregressive_streaming(*args, min_new_tokens=3))
                elif route == "fresh":
                    result = candidate._generate_sdar_cached(*args)
                    self.assertEqual(len(result[1]), 4)
                    with self.assertRaisesRegex(ValueError, "must not exceed"):
                        candidate._generate_sdar_cached(*args, min_new_tokens=3)
                else:
                    result = candidate._continue_sdar_cached(*args, [0, 1], DynamicCache())
                    self.assertEqual(len(result[1]), 6)
                    with self.assertRaisesRegex(ValueError, "must not exceed"):
                        candidate._continue_sdar_cached(*args, [0, 1], DynamicCache(), min_new_tokens=3)

    def test_authored_minimum_suppresses_union_without_using_delimiters_to_stop(self):
        args = (model_fixture(3), EOSDecoder(), "cpu", "prompt", 4, 0, 1)
        result = list(candidate._generate_autoregressive_streaming(*args, min_new_tokens=2))
        self.assertEqual(result, [{"mode": "append", "text": str(token)} for token in [0, 0, 3, 3]])
        for route in ["fresh", "continued"]:
            if route == "fresh":
                result = candidate._generate_sdar_cached(*args, min_new_tokens=2)
            else:
                result = candidate._continue_sdar_cached(*args, [0, 1], DynamicCache(), min_new_tokens=2)
            self.assertEqual(result[1][-4:], [0, 0, 3, 3])

    def test_inherited_sdar_retry_keeps_delimiter_safeguard_without_authored_promise(self):
        for streaming in [False, True]:
            worker = worker_text_functions()
            worker["_model_type"] = "dllm"
            worker["_model"] = model_fixture(3, minimum=10)
            worker["_model"].generation_config.forced_eos_token_id = 3
            worker["_tokenizer"] = EOSDecoder()
            worker["_generate_sdar_cached"].return_value = ("", [0, 1], object())
            worker["_generate_native_checked"] = candidate._generate_native_checked
            operation = worker["generate_tokens" if streaming else "generate"]
            result = operation("prompt", max_tokens=2, temperature=0)
            if streaming:
                result = list(result)
                self.assertEqual(result[0]["mode"], "replace")
                result = result[0]["text"]
            self.assertTrue(result)
            self.assertIsNone(worker["_live_kv_state"])

    def test_authored_native_floor_still_rejects_budget_and_forced_eos_conflicts(self):
        model = model_fixture(3)
        with self.assertRaisesRegex(ValueError, "must not exceed"):
            candidate._generate_autoregressive(model, EOSDecoder(), "cpu", "prompt", 2, 0, 1, min_new_tokens=3)
        model.generation_config.forced_eos_token_id = 3
        with self.assertRaises(candidate.MinimumNewTokensError):
            candidate._generate_autoregressive(model, EOSDecoder(), "cpu", "prompt", 2, 0, 1, min_new_tokens=2)


if __name__ == "__main__":
    unittest.main(verbosity=2)
