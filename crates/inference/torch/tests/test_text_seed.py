"""Request-scoped token RNG on installed CPU Torch/Transformers, no models."""

from concurrent.futures import ThreadPoolExecutor
import copy
import json
import unittest
from unittest import mock

import torch
from transformers import GenerationConfig
from transformers.cache_utils import DynamicCache

from test_autoregressive import (
    autoregressive as ar, worker_contract, worker_text_functions,
    TinyGenerationModel, TokenizerFixture,
)


class CachedLogitsModel(TinyGenerationModel):
    def forward(self, input_ids, **kwargs):
        return super().forward(input_ids, **kwargs)


class TextSeedTests(unittest.TestCase):
    def native(self, seed, model=None, temperature=0.7):
        return ar._generate_autoregressive(
            model or TinyGenerationModel(), TokenizerFixture(), "cpu", "prompt",
            24, temperature, 1.0, sampling=ar._seeded_sampling(seed),
        )

    def stream(self, seed):
        return ar._generate_autoregressive_streaming(
            TinyGenerationModel(), TokenizerFixture(), "cpu", "prompt", 24, 0.7, 1.0,
            sampling=ar._seeded_sampling(seed),
        )

    def test_seeded_native_omitted_minimum_preserves_inherited_forced_eos(self):
        from test_pr58_compatibility import EOSDecoder, model_fixture

        for seed in [0, 42, (1 << 64) - 1]:
            with self.subTest(seed=seed):
                model = model_fixture(3, minimum=10)
                model.generation_config.forced_eos_token_id = 3
                tokenizer = EOSDecoder()
                original_config = copy.deepcopy(model.generation_config)
                before = torch.get_rng_state().clone()
                actual = ar._generate_autoregressive(
                    model, tokenizer, "cpu", "prompt", 2, 0.7, 1,
                    sampling=ar._seeded_sampling(seed))
                self.assertTrue(torch.equal(before, torch.get_rng_state()))
                self.assertEqual(model.generation_config, original_config)
                with torch.random.fork_rng(devices=[]):
                    torch.manual_seed(seed)
                    expected = model.generate(
                        input_ids=torch.tensor([[0, 1]]), max_new_tokens=2,
                        temperature=0.7, top_p=1, do_sample=True)[0, 2:]
                self.assertEqual(actual, tokenizer.decode(expected))
                with self.assertRaises(ar.MinimumNewTokensError):
                    ar._generate_autoregressive(
                        model, tokenizer, "cpu", "prompt", 2, 0.7, 1,
                        min_new_tokens=2, sampling=ar._seeded_sampling(seed))
                self.assertTrue(torch.equal(before, torch.get_rng_state()))

    def test_seeded_manual_routes_keep_tokenizer_eos_stopping(self):
        from test_pr58_compatibility import EOSDecoder, model_fixture

        before = torch.get_rng_state().clone()
        for token in [2, 3]:
            for minimum in [None, 0]:
                for route in ["stream", "fresh", "continued"]:
                    with self.subTest(token=token, minimum=minimum, route=route):
                        model = model_fixture(token)
                        model.logits[token] = 100
                        args = (model, EOSDecoder(), "cpu", "prompt", 4, 0.7, 1)
                        kwargs = {"min_new_tokens": minimum, "sampling": ar._seeded_sampling(42)}
                        tokens = [] if token == 2 else [3] * 4
                        if route == "stream":
                            actual = list(ar._generate_autoregressive_streaming(*args, **kwargs))
                            self.assertEqual(actual, [{"mode": "append", "text": "3"} for _ in tokens])
                        else:
                            if route == "fresh":
                                actual = ar._generate_sdar_cached(*args, **kwargs)
                                prompt_ids = [0, 1]
                            else:
                                actual = ar._continue_sdar_cached(*args, [0, 1], DynamicCache(), **kwargs)
                                prompt_ids = [0, 1, 0, 1]
                            self.assertEqual(actual[:2], (",".join(str(t) for t in tokens), prompt_ids + tokens))
                        self.assertTrue(torch.equal(before, torch.get_rng_state()))

    def test_seeded_retry_enforces_authored_floor_and_drops_failed_kv(self):
        from test_pr58_compatibility import EOSDecoder, model_fixture

        before = torch.get_rng_state().clone()
        for minimum in [None, 0, 2]:
            for streaming in [False, True]:
                with self.subTest(minimum=minimum, streaming=streaming):
                    worker = worker_text_functions()
                    worker["_model_type"] = "dllm"
                    worker["_model"] = model_fixture(3, minimum=10)
                    worker["_model"].logits[3] = 100
                    worker["_model"].generation_config.forced_eos_token_id = 2
                    worker["_tokenizer"] = EOSDecoder()
                    worker["_generate_sdar_cached"].return_value = ("", [0, 1], object())
                    operation = worker["generate_tokens" if streaming else "generate"]
                    result = operation("prompt", max_tokens=4, temperature=0.7,
                                       min_new_tokens=minimum, seed=42)
                    if streaming:
                        self.assertEqual(list(result), [{"mode": "replace", "text": "3,3,3"}])
                    else:
                        self.assertEqual(result, "3,3,3")
                    self.assertIsNone(worker["_live_kv_state"])
                    self.assertTrue(torch.equal(before, torch.get_rng_state()))
                    with self.assertRaises(ar.MinimumNewTokensError):
                        result = operation("prompt", max_tokens=2, temperature=0.7,
                                           min_new_tokens=2, seed=42)
                        if streaming:
                            list(result)
                    self.assertIsNone(worker["_live_kv_state"])
                    self.assertTrue(torch.equal(before, torch.get_rng_state()))

    def test_native_seed_matches_actual_transformers_and_preserves_ambient_state(self):
        for seed in [0, 42, (1 << 64) - 1]:
            model = TinyGenerationModel()
            original_config = copy.deepcopy(model.generation_config)
            original_method = model._sample.__func__
            before = torch.get_rng_state().clone()
            actual = self.native(seed, model)
            self.assertTrue(torch.equal(before, torch.get_rng_state()))
            self.assertEqual(actual, self.native(seed, model))
            self.assertEqual(model.generation_config, original_config)
            self.assertIs(model._sample.__func__, original_method)
            with torch.random.fork_rng(devices=[]):
                torch.manual_seed(seed)
                expected = model.generate(
                    input_ids=torch.tensor([[0, 1]]), max_new_tokens=24,
                    temperature=0.7, top_p=1.0, do_sample=True,
                )[0, 2:]
            self.assertEqual(actual, TokenizerFixture().decode(expected))
        self.assertNotEqual(self.native(0), self.native(42))

    def test_native_requests_overlap_without_reseeding_one_another(self):
        model = TinyGenerationModel()
        seeds = [0, 42, 42, (1 << 64) - 1]
        expected = [self.native(seed, model) for seed in seeds]
        before = torch.get_rng_state().clone()
        # Shared resident model; each generation receives its own request copy.
        with ThreadPoolExecutor(max_workers=4) as pool:
            actual = list(pool.map(lambda seed: self.native(seed, model), seeds))
        self.assertEqual(actual, expected)
        self.assertTrue(torch.equal(before, torch.get_rng_state()))

    def test_paused_and_closed_streams_leave_rng_private(self):
        before = torch.get_rng_state().clone()
        first, second = self.stream(42), self.stream(0)
        a, b = [], []
        for _ in range(24):
            a.append(next(first)["text"])
            self.assertTrue(torch.equal(before, torch.get_rng_state()))
            b.append(next(second)["text"])
        self.assertEqual(",".join(a), self.native(42))
        self.assertEqual(",".join(b), self.native(0))
        first.close(); second.close()
        abandoned = self.stream(42)
        next(abandoned); abandoned.close()
        self.assertTrue(torch.equal(before, torch.get_rng_state()))
        self.assertEqual(list(self.stream(42)), list(self.stream(42)))

    def test_omission_uses_existing_ambient_sampling_and_greedy_uses_no_draws(self):
        before = torch.get_rng_state().clone()
        self.native(None)
        self.assertFalse(torch.equal(before, torch.get_rng_state()))
        greedy = self.native(None, temperature=0)
        before = torch.get_rng_state().clone()
        self.assertEqual(self.native(42, temperature=0), greedy)
        self.assertTrue(torch.equal(before, torch.get_rng_state()))
        self.assertIsNone(ar._seeded_sampling(None))

    def test_native_inherited_defaults_survive_seed_and_beams_refuse_before_forward(self):
        model = TinyGenerationModel()
        model.generation_config.top_k = 1
        model.generation_config.repetition_penalty = 1.2
        self.assertEqual(self.native(42, model), self.native(0, model))
        model.generation_config.num_beams = 2
        with mock.patch.object(model, "forward", side_effect=AssertionError("dispatched")):
            with self.assertRaisesRegex(ar.SeedSamplingError, "generation mode"):
                self.native(42, model)

    def test_legacy_model_config_modes_are_resolved_before_seeded_dispatch(self):
        model = TinyGenerationModel()
        model.generation_config = GenerationConfig.from_model_config(model.config)
        model.config.num_beams = 2
        before = torch.get_rng_state().clone()
        with mock.patch.object(model, "forward", side_effect=AssertionError("dispatched")):
            with self.assertWarns(UserWarning):
                with self.assertRaisesRegex(ar.SeedSamplingError, "generation mode"):
                    self.native(42, model)
        self.assertTrue(torch.equal(before, torch.get_rng_state()))
        self.assertEqual(model.generation_config.num_beams, 1)

    def test_unimplemented_sampling_device_refuses_without_global_rng_changes(self):
        before = torch.get_rng_state().clone()
        with self.assertRaisesRegex(ar.SeedSamplingError, "does not support device"):
            ar._seeded_sampling(0).multinomial(torch.empty(1, 4, device="meta"), num_samples=1)
        self.assertTrue(torch.equal(before, torch.get_rng_state()))

    def test_generator_initialization_failure_is_an_explicit_seed_refusal(self):
        before = torch.get_rng_state().clone()
        with mock.patch.object(torch, "Generator", side_effect=RuntimeError("unavailable")):
            with self.assertRaisesRegex(ar.SeedSamplingError, "generator is unavailable"):
                ar._seeded_sampling(42).multinomial(torch.ones(1, 4), num_samples=1)
        self.assertTrue(torch.equal(before, torch.get_rng_state()))

    def test_custom_native_sampler_or_generate_refuses_before_model_execution(self):
        for name in ["generate", "_sample", "_prepare_generation_config"]:
            model = TinyGenerationModel()
            setattr(model, name, mock.Mock(side_effect=AssertionError("custom called")))
            with self.assertRaisesRegex(ar.SeedSamplingError, "canonical Transformers"):
                self.native(42, model)
            getattr(model, name).assert_not_called()

    def test_native_error_does_not_leak_rng_or_request_model_mutation(self):
        model = TinyGenerationModel()
        original_method = model._sample.__func__
        original_config = copy.deepcopy(model.generation_config)
        before = torch.get_rng_state().clone()
        with mock.patch.object(model, "forward", side_effect=RuntimeError("failed forward")):
            with self.assertRaisesRegex(RuntimeError, "failed forward"):
                self.native(42, model)
        self.assertTrue(torch.equal(before, torch.get_rng_state()))
        self.assertIs(model._sample.__func__, original_method)
        self.assertEqual(model.generation_config, original_config)

    def test_sdar_fresh_and_restored_starting_kv_replay_per_request(self):
        for seed in [0, 42, (1 << 64) - 1]:
            before = torch.get_rng_state().clone()
            def fresh():
                return ar._generate_sdar_cached(
                    CachedLogitsModel(), TokenizerFixture(), "cpu", "prompt", 24, 0.7, 1.0,
                    sampling=ar._seeded_sampling(seed),
                )
            one, two = fresh(), fresh()
            self.assertEqual(one[:2], two[:2])
            def continued():
                # Each replay starts from the same KV/context snapshot; RNG is
                # request scoped, not persisted alongside the cache.
                return ar._continue_sdar_cached(
                    CachedLogitsModel(), TokenizerFixture(), "cpu", "suffix", 24, 0.7, 1.0,
                    [0, 1], DynamicCache(), sampling=ar._seeded_sampling(seed),
                )
            one, two = continued(), continued()
            self.assertEqual(one[:2], two[:2])
            self.assertTrue(torch.equal(before, torch.get_rng_state()))

    def test_worker_envelope_preserves_seed_boundaries_and_refuses_invalid_authored_values(self):
        for operation in ["generate_text", "generate_text_stream"]:
            envelope = {"contract_version": 1, "operation": operation,
                        "payload": {"prompt": "prompt", "transformers_kwargs": {}}}
            self.assertNotIn("seed", worker_contract.generate_text_kwargs_from_envelope(envelope, operation))
            for seed in [0, 42, (1 << 64) - 1]:
                envelope["payload"]["transformers_kwargs"] = {"seed": seed}
                self.assertEqual(worker_contract.generate_text_kwargs_from_envelope(
                    json.dumps(envelope), operation)["seed"], seed)
            for seed in [-1, 1 << 64, 1.0, True, "42", None]:
                envelope["payload"]["transformers_kwargs"] = {"seed": seed}
                with self.assertRaisesRegex(ValueError, "seed must"):
                    worker_contract.generate_text_kwargs_from_envelope(envelope, operation)
                with self.assertRaisesRegex(ar.SeedSamplingError, "seed must"):
                    if seed is not None:
                        ar._seeded_sampling(seed)
                    else:
                        ar._SeededSampling(seed)

    def test_actual_worker_envelope_reaches_native_and_streaming_sampling(self):
        for streaming in [False, True]:
            worker = worker_text_functions()
            worker["_model"] = TinyGenerationModel()
            worker["_generate_autoregressive"] = ar._generate_autoregressive
            worker["_generate_autoregressive_streaming"] = ar._generate_autoregressive_streaming
            operation = "generate_text_stream" if streaming else "generate_text"
            envelope = {"contract_version": 1, "operation": operation, "request_id": "seed-test",
                        "payload": {"prompt": "prompt", "max_tokens": 24,
                                    "transformers_kwargs": {"seed": 42}}}
            before = torch.get_rng_state().clone()
            if streaming:
                result = list(worker["generate_text_stream_from_envelope"](envelope))
                self.assertEqual(",".join(chunk["text"] for chunk in result), self.native(42))
            else:
                result = json.loads(worker["generate_text_from_envelope"](envelope))
                self.assertEqual(result["status"], "ok", result)
                self.assertEqual(result["result"]["text"], self.native(42))
            self.assertTrue(torch.equal(before, torch.get_rng_state()))

    def test_worker_retries_share_one_request_rng_and_seed_failure_clears_kv(self):
        worker = worker_text_functions()
        worker["_model_type"] = "dllm"
        worker["_live_kv_state"] = {"token_ids": [0, 1], "cache": object()}
        worker["_continue_sdar_cached"].side_effect = RuntimeError("cache unavailable")
        worker["generate"]("prompt", seed=42)
        rng = worker["_continue_sdar_cached"].call_args.kwargs["sampling"]
        self.assertIs(worker["_generate_sdar_cached"].call_args.kwargs["sampling"], rng)
        worker["_continue_sdar_cached"].reset_mock()
        worker["_generate_sdar_cached"].reset_mock()
        worker["_continue_sdar_cached"].side_effect = ar.SeedSamplingError("unsupported seed route")
        with self.assertRaises(ar.SeedSamplingError):
            worker["generate"]("prompt", seed=42)
        self.assertIsNone(worker["_live_kv_state"])
        worker["_generate_sdar_cached"].assert_not_called()
        worker["_model"] = TinyGenerationModel()
        worker["_generate_sdar_cached"].return_value = ("", [0, 1], object())
        with mock.patch.dict(worker, {"_generate_native_checked": mock.Mock(return_value=torch.tensor([[0, 1, 2]]))}):
            worker["generate"]("prompt", seed=42)
            self.assertIs(worker["_generate_sdar_cached"].call_args.kwargs["sampling"],
                          worker["_generate_native_checked"].call_args.kwargs["sampling"])

    def test_empty_sdar_native_seed_refusal_discards_uncommitted_cache(self):
        for streaming in [False, True]:
            worker = worker_text_functions()
            worker["_model_type"] = "dllm"
            worker["_model"] = TinyGenerationModel()
            worker["_generate_sdar_cached"].return_value = ("", [0, 1], object())
            worker["_generate_native_checked"] = mock.Mock(
                side_effect=ar.SeedSamplingError("native seed route unsupported"))
            operation = worker["generate_tokens" if streaming else "generate"]
            with self.assertRaisesRegex(ar.SeedSamplingError, "native seed route unsupported"):
                result = operation("failed prompt", seed=42)
                if streaming:
                    list(result)
            self.assertIsNone(worker["_live_kv_state"])
            worker["_generate_sdar_cached"].return_value = ("fresh", [0, 1, 2], object())
            worker["_continue_sdar_cached"].reset_mock()
            result = operation("next prompt", seed=42)
            if streaming:
                list(result)
            worker["_continue_sdar_cached"].assert_not_called()

    def test_masked_seed_refuses_before_cache_or_diffusion_effects(self):
        for streaming in [False, True]:
            worker = worker_text_functions()
            worker["_model_type"] = "dllm"
            custody = worker["_live_kv_state"] = {"token_ids": [0, 1], "cache": object()}
            operation = worker["generate_tokens" if streaming else "generate"]
            with self.assertRaisesRegex(ar.SeedSamplingError, "Masked block-diffusion"):
                result = operation("prompt", masked_prompt_json="{}", seed=0)
                if streaming:
                    list(result)
            self.assertIs(worker["_live_kv_state"], custody)
            worker["_generate_dllm_masked"].assert_not_called()
            worker["_generate_dllm_masked_streaming"].assert_not_called()
            worker["_format_prompt"].assert_not_called()
