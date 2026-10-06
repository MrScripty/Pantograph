"""Actual CPU token generation/stop matching; no model weights or downloads."""
import copy
import json
import unittest
from unittest import mock

import torch
from transformers import BatchEncoding, GenerationConfig
from transformers.modeling_outputs import CausalLMOutputWithPast

from test_autoregressive import autoregressive as ar, TinyGenerationModel, worker_text_functions, worker_contract


class TextTokenizer:
    eos_token_id = 7
    pieces = {0: 'prompt', 1: '<END>', 2: 'hello ', 3: '<', 4: 'END', 5: '>tail', 6: '!', 7: ''}

    def __call__(self, _prompt, **_kwargs):
        return BatchEncoding({'input_ids': torch.tensor([[0, 1]])})

    def decode(self, ids, **_kwargs):
        return ''.join(self.pieces[int(token)] for token in ids)


class SequenceModel(TinyGenerationModel):
    def __init__(self, tokens=(2, 3, 4, 5, 6, 7)):
        super().__init__()
        self.tokens = tokens
        self.forward_lengths = []
        self.generation_config.eos_token_id = 7

    def forward(self, input_ids, **_kwargs):
        self.forward_lengths.append(input_ids.shape[-1])
        offset = min(input_ids.shape[-1] - 2, len(self.tokens) - 1)
        logits = torch.full((1, input_ids.shape[-1], 8), -100.0)
        logits[..., self.tokens[offset]] = 100
        return CausalLMOutputWithPast(logits=logits)


class StopStringTests(unittest.TestCase):
    def native(self, model=None, markers=('<END>',), minimum=None, seed=None, budget=8, tokenizer=None):
        return ar._generate_autoregressive(
            model or SequenceModel(), tokenizer or TextTokenizer(), 'cpu', '<END>', budget, 0, 1,
            min_new_tokens=minimum, sampling=ar._seeded_sampling(seed),
            stop_strings=None if markers is None else list(markers))

    def stream(self, model=None, markers=('<END>',), minimum=None, seed=None, budget=8, tokenizer=None):
        return ar._generate_autoregressive_streaming(
            model or SequenceModel(), tokenizer or TextTokenizer(), 'cpu', '<END>', budget, 0, 1,
            min_new_tokens=minimum, sampling=ar._seeded_sampling(seed),
            stop_strings=None if markers is None else list(markers))

    def test_cross_token_marker_withholds_marker_and_same_token_tail_on_both_routes(self):
        for streaming in [False, True]:
            model = SequenceModel()
            result = list(self.stream(model)) if streaming else self.native(model)
            self.assertEqual(result, [{'mode': 'append', 'text': 'hello '}] if streaming else 'hello ')
            self.assertEqual(model.forward_lengths, [2, 3, 4, 5])

    def test_prompt_and_prompt_generated_boundary_do_not_match(self):
        # Prompt ends in <END>; generated starts >tail. No prompt bytes are searched.
        for streaming in [False, True]:
            model = SequenceModel((5, 7))
            result = list(self.stream(model)) if streaming else self.native(model)
            self.assertEqual(result, [{'mode': 'append', 'text': '>tail'}] if streaming else '>tail')
            tokenizer = TextTokenizer()
            tokenizer.pieces = {**tokenizer.pieces, 1: '<'}
            model = SequenceModel((4, 5, 7))
            result = list(self.stream(model, tokenizer=tokenizer)) if streaming else self.native(model, tokenizer=tokenizer)
            self.assertEqual(result, [{'mode': 'append', 'text': 'END'}, {'mode': 'append', 'text': '>tail'}]
                             if streaming else 'END>tail')

    def test_overlapping_markers_earliest_match_and_exact_whitespace(self):
        tokenizer = TextTokenizer()
        tokenizer.pieces = {**tokenizer.pieces, 2: 'safe  雪\nEND more'}
        for streaming in [False, True]:
            kwargs = {'model': SequenceModel((2, 7)), 'tokenizer': tokenizer,
                      'markers': ['END', '  雪\nEND', 'END more']}
            result = list(self.stream(**kwargs)) if streaming else self.native(**kwargs)
            self.assertEqual(result, [{'mode': 'append', 'text': 'safe'}] if streaming else 'safe')

    def test_unmatched_prefix_flushes_at_eos_and_budget(self):
        for tokens, budget in [((2, 3, 7), 8), ((2, 3, 4), 2)]:
            self.assertEqual(self.native(SequenceModel(tokens), budget=budget), 'hello <')
            chunks = list(self.stream(SequenceModel(tokens), budget=budget))
            self.assertEqual(''.join(chunk['text'] for chunk in chunks), 'hello <')

    def test_authored_floor_conflicts_refuse_without_continuing_or_marker_emission(self):
        for streaming in [False, True]:
            model = SequenceModel()
            if streaming:
                stream = self.stream(model, minimum=5)
                self.assertEqual(next(stream), {'mode': 'append', 'text': 'hello '})
                with self.assertRaisesRegex(ar.MinimumNewTokensError, 'stop string conflicts'):
                    list(stream)
            else:
                with self.assertRaisesRegex(ar.MinimumNewTokensError, 'stop string conflicts'):
                    self.native(model, minimum=5)
            self.assertEqual(model.forward_lengths, [2, 3, 4, 5])
            # Exactly four generated IDs satisfies the floor even though marker text is withheld.
            self.assertEqual(self.native(minimum=4), 'hello ')

    def test_native_inherited_floor_retains_semantics_manual_resolves_floor_and_zero_overrides(self):
        for minimum in [None, 0]:
            model = SequenceModel()
            model.generation_config.min_new_tokens = 5
            self.assertEqual(self.native(model, minimum=minimum), 'hello ')
            model = SequenceModel()
            model.generation_config.min_new_tokens = 5
            if minimum is None:
                with self.assertRaises(ar.MinimumNewTokensError):
                    list(self.stream(model, minimum=minimum))
            else:
                self.assertEqual(list(self.stream(model, minimum=minimum)), [{'mode': 'append', 'text': 'hello '}])

    def test_stop_route_preserves_eos_and_native_forced_eos_floor_priority(self):
        for streaming in [False, True]:
            model = SequenceModel((2, 7))
            result = list(self.stream(model)) if streaming else self.native(model)
            self.assertEqual(result, [{'mode': 'append', 'text': 'hello '}] if streaming else 'hello ')
        for seed in [None, 0]:
            for minimum in [None, 0, 3]:
                model = SequenceModel()
                model.generation_config.min_new_tokens = 5
                model.generation_config.forced_eos_token_id = 7
                if minimum == 3:
                    with self.assertRaisesRegex(ar.MinimumNewTokensError, 'later processors'):
                        self.native(model, markers=['absent'], budget=3, minimum=minimum, seed=seed)
                else:
                    self.assertEqual(self.native(model, markers=['absent'], budget=3,
                                                 minimum=minimum, seed=seed), 'hello <')

    def test_inherited_string_defaults_and_authored_precedence_do_not_mutate_resident(self):
        for streaming in [False, True]:
            for marker in ['<END>', ['<END>']]:
                model = SequenceModel()
                model.generation_config.stop_strings = marker
                original = copy.deepcopy(model.generation_config)
                result = list(self.stream(model, markers=None)) if streaming else self.native(model, markers=None)
                self.assertEqual(result, [{'mode': 'append', 'text': 'hello '}] if streaming else 'hello ')
                self.assertEqual(model.generation_config, original)
                result = list(self.stream(model, markers=['hello'])) if streaming else self.native(model, markers=['hello'])
                self.assertEqual(result, [] if streaming else '')
                self.assertEqual(model.generation_config, original)
            self.assertEqual(self.native(markers=None), 'hello <END>tail!')

    def test_legacy_native_model_defaults_are_resolved_before_stop_and_not_mutated(self):
        model = SequenceModel()
        model.config.stop_strings = ['<END>']
        model.config.top_k = 0
        model.config.use_cache = False
        model.generation_config = GenerationConfig.from_model_config(model.config)
        model.generation_config.use_cache = False
        # Preserve the native legacy-default refresh conditions, then change the owner config.
        model.generation_config._original_object_hash = hash(model.generation_config)
        model.config.stop_strings = ['hello']
        original = copy.deepcopy(model.generation_config)
        self.assertEqual(self.native(model, markers=None), '')
        self.assertEqual(model.generation_config, original)

    def test_legacy_empty_defaults_refresh_and_edited_none_owns_omission(self):
        for streaming in [False, True]:
            for edited in [False, True]:
                model = SequenceModel()
                model.config.stop_strings = ['<END>'] if edited else []
                model.config.top_k = 0
                model.config.use_cache = False
                model.generation_config = GenerationConfig.from_model_config(model.config)
                if edited:
                    model.generation_config.stop_strings = None
                else:
                    model.config.stop_strings = ['hello']
                original = copy.deepcopy(model.generation_config)
                expected = 'hello <END>tail!' if edited else ''
                result = ''.join(x['text'] for x in self.stream(model, markers=None)) if streaming else self.native(model, markers=None)
                self.assertEqual(result, expected)
                self.assertEqual(model.generation_config, original)
                worker = worker_text_functions()
                worker['_model_type'] = 'dllm'
                worker['_model'] = model
                if edited:
                    worker['generate']('prompt')
                    worker['_generate_sdar_cached'].assert_called_once()
                else:
                    with self.assertRaises(ar.StopStringError):
                        worker['generate']('prompt')
                    worker['_format_prompt'].assert_not_called()

    def test_seed_replay_is_private_and_stop_adds_no_draws_or_resident_mutation(self):
        for seed in [0, 42, (1 << 64) - 1]:
            before = torch.get_rng_state().clone()
            model = SequenceModel()
            model.generation_config.do_sample = True
            # Real native multinomial with isolated request RNG, deterministic fixed logits.
            sampling = ar._seeded_sampling(seed)
            result = ar._generate_autoregressive(model, TextTokenizer(), 'cpu', 'prompt', 8, 0.7, 1,
                                                 sampling=sampling, stop_strings=['<END>'])
            self.assertEqual(result, 'hello ')
            self.assertEqual(sampling.calls, 4)
            self.assertTrue(torch.equal(before, torch.get_rng_state()))
            streaming_sampling = ar._seeded_sampling(seed)
            chunks = list(ar._generate_autoregressive_streaming(
                SequenceModel(), TextTokenizer(), 'cpu', 'prompt', 8, 0.7, 1,
                sampling=streaming_sampling, stop_strings=['<END>']))
            self.assertEqual(chunks, [{'mode': 'append', 'text': 'hello '}])
            self.assertEqual(streaming_sampling.calls, 4)
            self.assertTrue(torch.equal(before, torch.get_rng_state()))
            self.assertEqual(self.native(seed=seed), 'hello ')

    def test_decode_rewrite_is_allowed_in_holdback_but_refuses_retracting_emitted_text(self):
        class RewritingTokenizer(TextTokenizer):
            def decode(self, ids, **kwargs):
                if len(ids) == 1:
                    return '<'
                return 'safe'
        self.assertEqual(''.join(x['text'] for x in self.stream(SequenceModel((3, 2)), budget=2,
                                                               tokenizer=RewritingTokenizer())), 'safe')
        class RetractingTokenizer(TextTokenizer):
            def decode(self, ids, **kwargs):
                return 'safe' if len(ids) == 1 else 'changed'
        stream = self.stream(SequenceModel((2, 3)), budget=2, tokenizer=RetractingTokenizer())
        self.assertEqual(next(stream)['text'], 'safe')
        with self.assertRaisesRegex(ar.StopStringError, 'cannot retract'):
            list(stream)

    def test_native_custom_beam_multiple_sequence_and_dict_routes_refuse_before_forward(self):
        for changes in [{'num_beams': 2}, {'num_return_sequences': 2, 'do_sample': True},
                        {'return_dict_in_generate': True}]:
            model = SequenceModel()
            for key, value in changes.items():
                setattr(model.generation_config, key, value)
            with self.assertRaises(ar.StopStringError):
                self.native(model)
            self.assertEqual(model.forward_lengths, [])
        model = SequenceModel()
        model.generate = mock.Mock()
        with self.assertRaisesRegex(ar.StopStringError, 'canonical'):
            self.native(model)
        model.generate.assert_not_called()

    def test_worker_forwarding_save_load_and_invalid_or_dllm_refusal_before_effects(self):
        marker = '  雪\nEND '
        for operation in ['generate_text', 'generate_text_stream']:
            envelope = {'contract_version': 1, 'operation': operation, 'request_id': 'stop',
                        'payload': {'prompt': 'prompt', 'transformers_kwargs': {'stop_strings': [marker]}}}
            loaded = json.loads(json.dumps(envelope))
            kwargs = worker_contract.generate_text_kwargs_from_envelope(loaded, operation)
            self.assertEqual(kwargs['stop_strings'], [marker])
            worker = worker_text_functions()
            result = worker['generate_tokens' if operation.endswith('stream') else 'generate'](**kwargs)
            if operation.endswith('stream'):
                list(result)
            target = worker['_generate_autoregressive_streaming' if operation.endswith('stream') else '_generate_autoregressive']
            self.assertEqual(target.call_args.kwargs['stop_strings'], [marker])
            for invalid in ['', [], [''], [1], ['END', None], True]:
                loaded['payload']['transformers_kwargs']['stop_strings'] = invalid
                with self.assertRaises(ValueError):
                    worker_contract.generate_text_kwargs_from_envelope(loaded, operation)
        for streaming in [False, True]:
            for inherited in [False, True]:
                worker = worker_text_functions()
                worker['_model_type'] = 'dllm'
                worker['_model'] = SequenceModel()
                worker['_live_kv_state'] = {'token_ids': [0, 1], 'cache': object()}
                original = worker['_live_kv_state']
                kwargs = {} if inherited else {'stop_strings': ['<END>']}
                if inherited:
                    worker['_model'].generation_config.stop_strings = '<END>'
                for masked in [None, '{}']:
                    with self.assertRaisesRegex(ar.StopStringError, 'SDAR or masked'):
                        result = worker['generate_tokens' if streaming else 'generate']('prompt', masked_prompt_json=masked, **kwargs)
                        if streaming:
                            list(result)
                    worker['_format_prompt'].assert_not_called()
                    worker['_generate_sdar_cached'].assert_not_called()
                    self.assertIs(worker['_live_kv_state'], original)


if __name__ == '__main__':
    unittest.main()
