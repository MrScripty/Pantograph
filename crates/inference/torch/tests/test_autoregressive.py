"""CPU sampling contracts; requires installed Torch and Transformers, no models."""

import ast
import importlib.util
import json
import logging
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest import mock

import torch
from transformers import BatchEncoding, GenerationConfig, GenerationMixin, PretrainedConfig
from transformers.modeling_outputs import CausalLMOutputWithPast
from transformers.generation.logits_process import (
    RepetitionPenaltyLogitsProcessor, TopKLogitsWarper, TopPLogitsWarper,
)


spec = importlib.util.spec_from_file_location(
    "pantograph_autoregressive", Path(__file__).parents[1] / "autoregressive.py"
)
autoregressive = importlib.util.module_from_spec(spec)
spec.loader.exec_module(autoregressive)
contract_spec = importlib.util.spec_from_file_location(
    "pantograph_worker_contract", Path(__file__).parents[1] / "worker_contract.py"
)
worker_contract = importlib.util.module_from_spec(contract_spec)
contract_spec.loader.exec_module(worker_contract)


def worker_text_functions():
    """Execute actual text entry functions without importing audio dependencies.

    This is a controlled worker boundary test, not full worker/module loading.
    """
    tree = ast.parse((Path(__file__).parents[1] / "worker.py").read_text())
    names = {"generate", "generate_tokens", "generate_text_from_envelope",
             "generate_text_stream_from_envelope", "generate_text_stream_setup_from_envelope",
             "_generate_dllm_autoregressive_safe", "clear_live_kv_cache"}
    selected = ast.Module(body=[node for node in tree.body
                               if isinstance(node, ast.FunctionDef) and node.name in names],
                          type_ignores=[])
    namespace = {"torch": torch, "json": json, "logger": logging.getLogger(__name__),
                 "_model": object(), "_model_type": "text-generation", "_live_kv_state": None,
                 "_tokenizer": TokenizerFixture(), "_device": "cpu", "_model_path": None,
                 "_format_prompt": mock.Mock(return_value="formatted prompt"),
                 "_generate_autoregressive": mock.Mock(return_value="ok"),
                 "_generate_autoregressive_streaming": mock.Mock(return_value=iter([])),
                 "_generate_dllm_masked": mock.Mock(),
                 "_generate_dllm_masked_streaming": mock.Mock(),
                 "_generate_sdar_cached": mock.Mock(return_value=("ok", [0, 1, 2], object())),
                 "_continue_sdar_cached": mock.Mock(return_value=("ok", [0, 1, 2], object()))}
    for name in ["decode_worker_envelope", "generate_text_kwargs_from_envelope",
                 "worker_success_response_json", "worker_error_response_json",
                 "GENERATE_TEXT_STREAM_OPERATION"]:
        namespace[name] = getattr(worker_contract, name)
    exec(compile(selected, "worker.py:text-entry-functions", "exec"), namespace)
    return namespace


class TokenizerFixture:
    eos_token_id = None

    def __call__(self, _prompt, **_kwargs):
        return BatchEncoding({"input_ids": torch.tensor([[0, 1]])})

    def decode(self, token_ids, **_kwargs):
        return ",".join(str(int(token)) for token in token_ids)


class LogitsModelFixture:
    def __init__(self, default_top_k=2):
        self.generation_config = SimpleNamespace(top_k=default_top_k)
        self.generate_kwargs = None

    def __call__(self, input_ids):
        logits = torch.tensor([[[0.0, 1.0, 2.0, 3.0]]])
        return SimpleNamespace(logits=logits.expand(1, input_ids.shape[1], 4))

    def generate(self, **kwargs):
        self.generate_kwargs = kwargs
        return torch.cat([kwargs["input_ids"], torch.tensor([[2]])], dim=-1)


class TinyGenerationModel(GenerationMixin):
    """Real Transformers generation on fixed logits; no weights or Hub access."""

    main_input_name = "input_ids"
    _is_stateful = False
    _supports_cache_class = False
    device = torch.device("cpu")

    def __init__(self):
        self.config = PretrainedConfig(is_encoder_decoder=False)
        self.generation_config = GenerationConfig(
            use_cache=False, pad_token_id=0, bos_token_id=0, eos_token_id=None,
            top_k=0,
        )
        self.logits = torch.tensor([0.0, 0.01, 0.02, 0.03])

    @classmethod
    def can_generate(cls):
        return True

    def prepare_inputs_for_generation(self, input_ids, **_kwargs):
        return {"input_ids": input_ids}

    def forward(self, input_ids, **_kwargs):
        return CausalLMOutputWithPast(
            logits=self.logits.expand(1, input_ids.shape[1], 4),
        )

    __call__ = forward


class AutoregressiveSamplingTests(unittest.TestCase):
    def test_actual_worker_entries_propagate_and_refuse_before_inference(self):
        for model_type in ["text-generation", "sherry", "dllm"]:
            for streaming in [False, True]:
                with self.subTest(model_type=model_type, streaming=streaming):
                    worker = worker_text_functions()
                    worker["_model_type"] = model_type
                    operation = "generate_text_stream" if streaming else "generate_text"
                    envelope = {"contract_version": 1, "request_id": "repetition-entry",
                                "operation": operation, "payload": {"prompt": "prompt",
                                "transformers_kwargs": {"repetition_penalty": 2.0}}}
                    if streaming:
                        setup = json.loads(worker["generate_text_stream_setup_from_envelope"](envelope))
                        self.assertEqual(setup["status"], "ok")
                        list(worker["generate_text_stream_from_envelope"](envelope))
                    else:
                        result = json.loads(worker["generate_text_from_envelope"](envelope))
                        self.assertEqual(result["status"], "ok")
                    called = "_generate_sdar_cached" if model_type == "dllm" else (
                        "_generate_autoregressive_streaming" if streaming else "_generate_autoregressive")
                    self.assertEqual(worker[called].call_args.kwargs["repetition_penalty"], 2.0)
                    worker[called].reset_mock()
                    worker["_format_prompt"].reset_mock()
                    envelope["payload"]["transformers_kwargs"]["repetition_penalty"] = True
                    if streaming:
                        setup = json.loads(worker["generate_text_stream_setup_from_envelope"](envelope))
                        self.assertEqual(setup["error"]["kind"], "invalid_request")
                        with self.assertRaises(ValueError):
                            list(worker["generate_text_stream_from_envelope"](envelope))
                    else:
                        result = json.loads(worker["generate_text_from_envelope"](envelope))
                        self.assertEqual(result["error"]["kind"], "invalid_request")
                    worker[called].assert_not_called()
                    worker["_format_prompt"].assert_not_called()

    def test_masked_route_rejects_authored_penalty_without_touching_custody_or_model(self):
        for streaming in [False, True]:
            worker = worker_text_functions()
            worker["_model_type"] = "dllm"
            custody = {"token_ids": [2], "cache": object()}
            worker["_live_kv_state"] = custody
            operation = worker["generate_tokens" if streaming else "generate"]
            with self.assertRaisesRegex(ValueError, "Masked block-diffusion"):
                result = operation("prompt", masked_prompt_json="{}", repetition_penalty=1.0)
                if streaming:
                    list(result)
            self.assertIs(worker["_live_kv_state"], custody)
            for name in ["_format_prompt", "_generate_sdar_cached", "_continue_sdar_cached",
                         "_generate_dllm_masked", "_generate_dllm_masked_streaming"]:
                worker[name].assert_not_called()

    def test_sdar_worker_cached_and_empty_retry_preserve_request_or_model_default(self):
        for authored in [None, 1.0, 2.0]:
            worker = worker_text_functions()
            model = LogitsModelFixture()
            model.generation_config.repetition_penalty = 2.0
            worker["_model"] = model
            worker["_model_type"] = "dllm"
            worker["_live_kv_state"] = {"token_ids": [2], "cache": object()}
            worker["_generate_dllm_autoregressive_safe"](
                "prompt", 2, 0, 1, repetition_penalty=authored)
            self.assertEqual(worker["_continue_sdar_cached"].call_args.kwargs["repetition_penalty"], authored)
            worker["_live_kv_state"] = None
            worker["_generate_sdar_cached"].return_value = ("", [0, 1], object())
            worker["_generate_dllm_autoregressive_safe"](
                "prompt", 2, 0, 1, repetition_penalty=authored)
            if authored is None:
                self.assertNotIn("repetition_penalty", model.generate_kwargs)
            else:
                self.assertEqual(model.generate_kwargs["repetition_penalty"], authored)
            self.assertIsNone(worker["_live_kv_state"])

    def test_repetition_penalty_matches_real_transformers_with_full_history(self):
        for logits in [[-4.0, 4.0, 3.0, 2.5], [-4.0, -1.0, -1.5, -3.0]]:
            for authored in [None, 0.5, 1.0, 2.0]:
                for temperature in [0.0, 0.8]:
                    for top_k, top_p in [(0, 1.0), (2, 0.7)]:
                        with self.subTest(logits=logits, penalty=authored,
                                          temperature=temperature, top_k=top_k, top_p=top_p):
                            model = TinyGenerationModel()
                            model.logits = torch.tensor(logits)
                            model.generation_config.repetition_penalty = 2.0
                            results = []
                            for streaming in [False, True]:
                                with torch.random.fork_rng(devices=[]):
                                    torch.manual_seed(17)
                                    args = (model, TokenizerFixture(), "cpu", "prompt", 3,
                                            temperature, top_p)
                                    if streaming:
                                        chunks = autoregressive._generate_autoregressive_streaming(
                                            *args, top_k=top_k, repetition_penalty=authored)
                                        results.append(",".join(chunk["text"] for chunk in chunks))
                                    else:
                                        results.append(autoregressive._generate_autoregressive(
                                            *args, top_k=top_k, repetition_penalty=authored))
                            self.assertEqual(results[0], results[1])
                            # Independent official processor plus official warpers,
                            # with history advanced after each sampled token.
                            history = torch.tensor([[0, 1]])
                            expected = []
                            penalty = 2.0 if authored is None else authored
                            with torch.random.fork_rng(devices=[]):
                                torch.manual_seed(17)
                                for _ in range(3):
                                    scores = RepetitionPenaltyLogitsProcessor(penalty)(
                                        history, model.logits.unsqueeze(0))
                                    if temperature == 0:
                                        token = scores.argmax(dim=-1, keepdim=True)
                                    else:
                                        scores = scores / temperature
                                        if top_k:
                                            scores = TopKLogitsWarper(top_k)(history, scores)
                                        if top_p < 1:
                                            scores = TopPLogitsWarper(top_p)(history, scores)
                                        token = torch.multinomial(torch.softmax(scores, -1), 1)
                                    expected.append(str(int(token.item())))
                                    history = torch.cat([history, token], dim=-1)
                            self.assertEqual(results[1], ",".join(expected))
                            if logits[1] > 0 and temperature == 0 and not top_k:
                                self.assertEqual(results[1],
                                                 "2,3,1" if penalty == 2 else "1,1,1")

    def test_repetition_omission_leaves_generate_default_and_explicit_one_overrides(self):
        for authored in [None, 1.0, 0.5, 2.0]:
            model = LogitsModelFixture()
            model.generation_config.repetition_penalty = 2.0
            autoregressive._generate_autoregressive(
                model, TokenizerFixture(), "cpu", "prompt", 1, 0.8, 1.0,
                repetition_penalty=authored)
            if authored is None:
                self.assertNotIn("repetition_penalty", model.generate_kwargs)
            else:
                self.assertEqual(model.generate_kwargs["repetition_penalty"], authored)

    def test_sdar_fresh_and_cached_history_use_prompt_suffix_and_generated_tokens(self):
        class SDARModel:
            generation_config = SimpleNamespace(top_k=0, repetition_penalty=2.0)

            def __call__(self, input_ids, **_kwargs):
                return SimpleNamespace(logits=torch.tensor([[[-4.0, 4.0, 3.0, 2.5]]]))

        class Cache:
            def __init__(self):
                self.crops = []

            def crop(self, length):
                self.crops.append(length)

        class EmptySuffixTokenizer(TokenizerFixture):
            def __call__(self, _prompt, **_kwargs):
                return BatchEncoding({"input_ids": torch.empty((1, 0), dtype=torch.long)})

        for authored in [None, 1.0, 2.0]:
            penalty = 2.0 if authored is None else authored
            fresh, ids, cache = autoregressive._generate_sdar_cached(
                SDARModel(), TokenizerFixture(), "cpu", "prompt", 3, 0, 1,
                repetition_penalty=authored)
            self.assertEqual(fresh, "2,3,1" if penalty == 2 else "1,1,1")
            self.assertEqual(ids, [0, 1] + [int(token) for token in fresh.split(",")])
            self.assertIsNotNone(cache)
            continued, ids, _ = autoregressive._continue_sdar_cached(
                SDARModel(), TokenizerFixture(), "cpu", "suffix", 2, 0, 1,
                [2], Cache(), repetition_penalty=authored)
            self.assertEqual(continued, "3,1" if penalty == 2 else "1,1")
            self.assertEqual(ids, [2, 0, 1] + [int(token) for token in continued.split(",")])
            replay_cache = Cache()
            replayed, ids, _ = autoregressive._continue_sdar_cached(
                SDARModel(), EmptySuffixTokenizer(), "cpu", "", 2, 0, 1,
                [0, 1, 2], replay_cache, repetition_penalty=authored)
            self.assertEqual(replayed, "3,1" if penalty == 2 else "1,1")
            self.assertEqual(ids, [0, 1, 2] + [int(token) for token in replayed.split(",")])
            self.assertEqual(replay_cache.crops, [2])

    def test_worker_envelopes_propagate_only_valid_repetition_penalties(self):
        for operation in ["generate_text", "generate_text_stream"]:
            for penalty in [0.5, 1, 2.0]:
                kwargs = worker_contract.generate_text_kwargs_from_envelope(
                    {"contract_version": 1, "operation": operation,
                     "payload": {"prompt": "prompt", "transformers_kwargs": {
                         "repetition_penalty": penalty}}}, expected_operation=operation)
                self.assertEqual(kwargs["repetition_penalty"], penalty)
            for invalid in [True, False, None, "1.2", 0, -1, float("nan"),
                            float("inf"), -float("inf"), 10 ** 400]:
                with self.subTest(operation=operation, invalid=str(invalid)):
                    with self.assertRaisesRegex(ValueError, "finite positive"):
                        worker_contract.generate_text_kwargs_from_envelope(
                            {"contract_version": 1, "operation": operation,
                             "payload": {"prompt": "prompt", "transformers_kwargs": {
                                 "repetition_penalty": invalid}}}, expected_operation=operation)

    def test_top_p_boundaries_defaults_and_interactions_match_real_generation(self):
        for logits in [[0.0, 0.01, 0.02, 0.03], [0.03, 0.03, 0.01, 0.0], [0.0] * 4]:
            for authored_p in [None, 0.0, 0.5, 0.7, 1.0]:
                for authored_temperature in [None, 0.0, 0.001, 0.7, 2.0]:
                    for authored_k in [None, 0, 2, 5, (1 << 32) - 1]:
                        with self.subTest(logits=logits, top_p=authored_p,
                                          temperature=authored_temperature, top_k=authored_k):
                            model = TinyGenerationModel()
                            model.logits = torch.tensor(logits)
                            model.generation_config.top_k = 2
                            model.generation_config.top_p = 0.25
                            temperature = 0.7 if authored_temperature is None else authored_temperature
                            top_p = 1.0 if authored_p is None else authored_p
                            top_k = 2 if authored_k is None else authored_k
                            scores = model.logits.unsqueeze(0)
                            if temperature > 0:
                                scores = scores / max(temperature, 0.01)
                                if top_k > 0:
                                    scores = TopKLogitsWarper(top_k)(None, scores)
                                if top_p < 1.0:
                                    scores = TopPLogitsWarper(top_p)(None, scores)
                                expected_probs = torch.softmax(scores, dim=-1)

                            def generate(streaming):
                                operation = "generate_text_stream" if streaming else "generate_text"
                                payload = {"prompt": "prompt", "max_tokens": 2,
                                           "transformers_kwargs": {}}
                                if authored_p is not None:
                                    payload["top_p"] = authored_p
                                if authored_temperature is not None:
                                    payload["temperature"] = authored_temperature
                                if authored_k is not None:
                                    payload["transformers_kwargs"]["top_k"] = authored_k
                                kwargs = worker_contract.generate_text_kwargs_from_envelope(
                                    {"contract_version": 1, "request_id": "tiny-nucleus",
                                     "operation": operation, "payload": payload},
                                    expected_operation=operation,
                                )
                                self.assertEqual(kwargs["top_p"], top_p)
                                observed_probs = []
                                multinomial = torch.multinomial

                                def record_and_sample(probs, num_samples, **extra):
                                    observed_probs.append(probs.clone())
                                    return multinomial(probs, num_samples, **extra)

                                with torch.random.fork_rng(devices=[]):
                                    torch.manual_seed(41)
                                    with mock.patch.object(torch, "multinomial", side_effect=record_and_sample):
                                        args = (model, TokenizerFixture(), "cpu", "prompt", 2,
                                                kwargs["temperature"], kwargs["top_p"])
                                        if streaming:
                                            result = list(autoregressive._generate_autoregressive_streaming(
                                                *args, top_k=kwargs.get("top_k"),
                                            ))
                                            text = ",".join(chunk["text"] for chunk in result)
                                        else:
                                            text = autoregressive._generate_autoregressive(
                                                *args, top_k=kwargs.get("top_k"),
                                            )
                                if temperature == 0:
                                    self.assertEqual(observed_probs, [])
                                    expected = str(int(model.logits.argmax()))
                                    self.assertEqual(text, f"{expected},{expected}")
                                else:
                                    self.assertEqual(len(observed_probs), 2)
                                    for probs in observed_probs:
                                        torch.testing.assert_close(probs, expected_probs)
                                return text

                            self.assertEqual(generate(False), generate(True))

    def test_temperature_controls_real_streaming_and_transformers_generation(self):
        for authored in [None, 0.0, 0.001, 0.01, 0.7, 2.0, torch.finfo(torch.float32).max]:
            with self.subTest(temperature=authored):
                temperature = 0.7 if authored is None else authored
                model = TinyGenerationModel()
                logits = model.logits.unsqueeze(0)
                observed_probs = []
                multinomial = torch.multinomial

                def record_and_sample(probs, num_samples, **kwargs):
                    observed_probs.append(probs.clone())
                    return multinomial(probs, num_samples, **kwargs)

                def generate(streaming):
                    observed_probs.clear()
                    operation = "generate_text_stream" if streaming else "generate_text"
                    payload = {"prompt": "prompt", "max_tokens": 3, "top_p": 1.0,
                               "transformers_kwargs": {"top_k": 0}}
                    if authored is not None:
                        payload["temperature"] = authored
                    envelope = {"contract_version": 1, "request_id": "tiny-logits",
                                "operation": operation, "payload": payload}
                    kwargs = worker_contract.generate_text_kwargs_from_envelope(
                        envelope, expected_operation=operation,
                    )
                    self.assertEqual(kwargs["temperature"], temperature)
                    with torch.random.fork_rng(devices=[]):
                        torch.manual_seed(37)
                        with mock.patch.object(torch, "multinomial", side_effect=record_and_sample):
                            if streaming:
                                result = list(autoregressive._generate_autoregressive_streaming(
                                    model, TokenizerFixture(), "cpu", "prompt", 3,
                                    kwargs["temperature"], kwargs["top_p"], top_k=kwargs["top_k"],
                                ))
                                text = ",".join(chunk["text"] for chunk in result)
                            else:
                                text = autoregressive._generate_autoregressive(
                                    model, TokenizerFixture(), "cpu", "prompt", 3,
                                    kwargs["temperature"], kwargs["top_p"], top_k=kwargs["top_k"],
                                )
                    if temperature == 0:
                        self.assertEqual(observed_probs, [])
                        self.assertEqual(text, "3,3,3")
                    else:
                        self.assertEqual(len(observed_probs), 3)
                        expected_probs = torch.softmax(logits / max(temperature, 0.01), dim=-1)
                        for probs in observed_probs:
                            torch.testing.assert_close(probs, expected_probs)
                    return text

                self.assertEqual(generate(False), generate(True))

    def test_real_logits_match_transformers_top_k_and_top_p_probabilities(self):
        logits = torch.tensor([[0.0, 1.0, 2.0, 3.0], [2.0, 2.0, 0.0, -1.0], [0.0] * 4])
        for top_k in [0, 1, 2, 4, 5, (1 << 32) - 1]:
            for top_p in [0.0, 0.25, 0.5, 0.7, 1.0]:
                with self.subTest(top_k=top_k, top_p=top_p):
                    expected_scores = logits / 0.8
                    if top_k > 0:
                        expected_scores = TopKLogitsWarper(top_k)(None, expected_scores)
                    if top_p < 1.0:
                        expected_scores = TopPLogitsWarper(top_p)(None, expected_scores)
                    expected_probs = torch.softmax(expected_scores, dim=-1)
                    observed_probs = []
                    multinomial = torch.multinomial

                    def record_and_sample(probs, num_samples):
                        observed_probs.append(probs.clone())
                        return multinomial(probs, num_samples)

                    with torch.random.fork_rng(devices=[]):
                        torch.manual_seed(17)
                        expected_token = multinomial(expected_probs, num_samples=1)
                        torch.manual_seed(17)
                        with mock.patch.object(torch, "multinomial", side_effect=record_and_sample):
                            actual_token = autoregressive._sample_next_token(logits, 0.8, top_p, top_k)
                    self.assertEqual(len(observed_probs), 1)
                    torch.testing.assert_close(observed_probs[0], expected_probs)
                    torch.testing.assert_close(actual_token, expected_token)

    def test_greedy_sampling_preserves_argmax_for_large_k(self):
        logits = torch.tensor([[0.0, 1.0, 3.0, 2.0]])
        for top_k in [0, 5, (1 << 32) - 1]:
            with self.subTest(top_k=top_k):
                actual = autoregressive._sample_next_token(logits, 0.0, 1.0, top_k)
                torch.testing.assert_close(actual, torch.tensor([[2]]))

    def test_streaming_with_k_above_vocabulary_matches_disabled_filter(self):
        def generate(top_k):
            with torch.random.fork_rng(devices=[]):
                torch.manual_seed(23)
                return list(autoregressive._generate_autoregressive_streaming(
                    LogitsModelFixture(), TokenizerFixture(), "cpu", "prompt",
                    3, 0.8, 1.0, top_k=top_k,
                ))

        baseline = generate(0)
        self.assertEqual(len(baseline), 3)
        self.assertEqual(generate(None), generate(2))
        for top_k in [5, (1 << 32) - 1]:
            with self.subTest(top_k=top_k):
                self.assertEqual(generate(top_k), baseline)

    def test_nonstreaming_preserves_zero_and_model_default_kwargs(self):
        for default_top_k in [None, 2]:
            for top_k in [None, 0, 5, (1 << 32) - 1]:
                with self.subTest(default_top_k=default_top_k, top_k=top_k):
                    model = LogitsModelFixture(default_top_k)
                    result = autoregressive._generate_autoregressive(
                        model, TokenizerFixture(), "cpu", "prompt", 3, 0.8, 1.0,
                        top_k=top_k,
                    )
                    self.assertEqual(result, "2")
                    if top_k is None and default_top_k is None:
                        self.assertNotIn("top_k", model.generate_kwargs)
                    else:
                        self.assertEqual(model.generate_kwargs["top_k"],
                                         default_top_k if top_k is None else top_k)
                    self.assertEqual(autoregressive._resolve_top_k(model, top_k),
                                     (default_top_k or 0) if top_k is None else top_k)


if __name__ == "__main__":
    unittest.main()
