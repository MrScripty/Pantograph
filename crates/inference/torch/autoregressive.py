"""Autoregressive (standard token-by-token) generation for HuggingFace models.

Provides non-streaming and streaming generation using the standard
HuggingFace generate API and manual token-by-token sampling.
"""

import copy
import math
from types import FunctionType, MethodType

import torch
from transformers import GenerationConfig, GenerationMixin
from transformers.generation.configuration_utils import GenerationMode
from transformers.cache_utils import DynamicCache
from transformers.generation.logits_process import (
    MinNewTokensLengthLogitsProcessor, RepetitionPenaltyLogitsProcessor,
)
from transformers.generation.stopping_criteria import StoppingCriteria, StoppingCriteriaList


class RepetitionPenaltyNumericsError(ValueError):
    """The requested repetition operation cannot produce usable f32 scores."""


class MinimumNewTokensError(ValueError):
    """The minimum length has no eligible non-EOS token to select."""


class SeedSamplingError(ValueError):
    """The requested seed cannot be isolated on this sampling route."""


class StopStringError(ValueError):
    """Stop text is invalid or cannot be honored on this decoding route."""


def _validate_stop_strings(value, *, inherited=False):
    if value is None:
        return ()
    if inherited and value == []:
        return ()
    if inherited and isinstance(value, str):
        value = [value]
    if (not isinstance(value, (list, tuple)) or not value
            or any(not isinstance(marker, str) or not marker for marker in value)):
        raise StopStringError("stop_strings must be a non-empty list of non-empty strings")
    return tuple(value)


def _resolve_stop_strings(model, stop_strings):
    if stop_strings is not None:
        return _validate_stop_strings(stop_strings)
    config = getattr(model, "generation_config", None)
    # Mirror the installed native resolver's legacy-refresh conditions without
    # changing the resident config. An edited generation config owns its defaults,
    # including an explicit None; an untouched legacy config can refresh [] too.
    model_config = getattr(model, "config", None)
    non_defaults = getattr(model_config, "_get_non_default_generation_parameters", None)
    if (type(config) is GenerationConfig and config._from_model_config
            and config._original_object_hash == hash(config)
            and callable(non_defaults) and non_defaults()):
        config = GenerationConfig.from_model_config(model_config)
    inherited = getattr(config, "stop_strings", None)
    return _validate_stop_strings(inherited, inherited=True)


class _GeneratedTextStop:
    """Match cumulative new-token decode, withholding possible marker prefixes."""

    def __init__(self, tokenizer, markers, floor):
        self.tokenizer = tokenizer
        self.markers = tuple(markers)
        self.floor = floor
        self.text = ""
        self.emitted = ""
        self.stopped = False

    def observe(self, token_ids):
        self.text = self.tokenizer.decode(token_ids, skip_special_tokens=True)
        if not isinstance(self.text, str):
            raise StopStringError("stop string decoding must return text")
        matches = [index for marker in self.markers
                   if (index := self.text.find(marker)) >= 0]
        if matches:
            if len(token_ids) < self.floor:
                raise MinimumNewTokensError("stop string conflicts with min_new_tokens")
            self.text = self.text[:min(matches)]
            self.stopped = True
        return self.stopped

    def take(self, *, final=False):
        text = self.text
        if not self.stopped and not final:
            hold = max((size for marker in self.markers
                        for size in range(1, min(len(marker), len(text) + 1))
                        if text.endswith(marker[:size])), default=0)
            # A held-only rewrite can move a possible marker prefix into text
            # already emitted. Withhold only the remaining suffix; a complete
            # marker crossing that boundary still fails the retraction guard.
            hold = min(hold, max(0, len(text) - len(self.emitted)))
            if hold:
                text = text[:-hold]
        if not text.startswith(self.emitted):
            raise StopStringError("stop string streaming cannot retract emitted decoded text")
        chunk = text[len(self.emitted):]
        self.emitted = text
        return chunk


class _GeneratedTextStopCriteria(StoppingCriteria):
    def __init__(self, matcher, prompt_length):
        self.matcher = matcher
        self.prompt_length = prompt_length
        self.calls = 0

    def __call__(self, input_ids, scores, **kwargs):
        if input_ids.shape[0] != 1:
            raise StopStringError("stop strings require one generated sequence")
        self.calls += 1
        matched = self.matcher.observe(input_ids[0, self.prompt_length:].tolist())
        return torch.tensor([matched], device=input_ids.device, dtype=torch.bool)


def _prepare_native_stop(request_model, tokenizer, stop_strings, kwargs, floor):
    for name in ("generate", "_prepare_generation_config", "_sample", "_get_stopping_criteria"):
        if getattr(getattr(request_model, name, None), "__func__", None) is not getattr(GenerationMixin, name):
            raise StopStringError(f"stop strings require canonical Transformers {name}")
    if type(request_model.generation_config) is not GenerationConfig:
        raise StopStringError("stop strings require standard Transformers GenerationConfig")
    request_model.generation_config = copy.deepcopy(request_model.generation_config)
    provided = kwargs.pop("generation_config", None)
    if stop_strings is not None:
        kwargs["stop_strings"] = list(stop_strings)
    try:
        config, model_kwargs = request_model._prepare_generation_config(provided, **kwargs)
    except ValueError as exc:
        raise StopStringError(f"stop strings cannot use this generation configuration: {exc}") from exc
    config = copy.deepcopy(config)
    markers = _validate_stop_strings(config.stop_strings, inherited=True)
    if not markers:
        return {**model_kwargs, "generation_config": config, "use_model_defaults": False}, None
    if (config.get_generation_mode() not in (GenerationMode.SAMPLE, GenerationMode.GREEDY_SEARCH)
            or config.num_return_sequences != 1 or config.return_dict_in_generate
            or model_kwargs["input_ids"].shape[0] != 1):
        raise StopStringError("stop strings support single-sequence greedy/sampling generation only")
    # Our generated-only criterion owns matching; the HF tokenizer-wide default
    # would also match prompt/generated boundaries and duplicate stop semantics.
    config.stop_strings = None
    matcher = _GeneratedTextStop(tokenizer, markers, floor or 0)
    criterion = _GeneratedTextStopCriteria(matcher, model_kwargs["input_ids"].shape[-1])
    existing = model_kwargs.pop("stopping_criteria", [])
    return {**model_kwargs, "generation_config": config, "use_model_defaults": False,
            "stopping_criteria": StoppingCriteriaList([*existing, criterion])}, criterion


class _SeededSampling:
    """One generator per request, created on the actual sampling device.

    The seed covers token selection, including retries, but not randomness in
    arbitrary model forwards/processors. KV state does not own this RNG.
    """

    def __init__(self, seed):
        if isinstance(seed, bool) or not isinstance(seed, int) or not 0 <= seed < 1 << 64:
            raise SeedSamplingError("seed must be a non-negative u64 integer")
        self.seed = seed
        self.generator = None
        self.calls = 0

    def multinomial(self, probabilities, *args, **kwargs):
        device = probabilities.device
        if device.type not in ("cpu", "cuda"):
            raise SeedSamplingError(f"seed sampling does not support device {device}")
        if self.generator is None:
            try:
                self.generator = torch.Generator(device=device).manual_seed(self.seed)
            except (RuntimeError, TypeError) as exc:
                raise SeedSamplingError(f"seed generator is unavailable on {device}") from exc
        if self.generator.device != device:
            raise SeedSamplingError("seed sampling changed device within a request")
        self.calls += 1
        return torch.multinomial(probabilities, *args, generator=self.generator, **kwargs)

    def __getattr__(self, name):
        # Native _sample keeps the installed Transformers implementation. Only
        # its multinomial draw changes; neither torch nor the resident model does.
        return getattr(torch, name)


def _seeded_sampling(seed):
    return None if seed is None else _SeededSampling(seed)


def _prepare_seeded_native(request_model, sampling, kwargs):
    for name in ("generate", "_sample", "_prepare_generation_config"):
        if getattr(getattr(request_model, name, None), "__func__", None) is not getattr(GenerationMixin, name):
            raise SeedSamplingError(f"seed requires canonical Transformers {name}")
    if type(request_model.generation_config) is not GenerationConfig:
        raise SeedSamplingError("seed requires the standard Transformers GenerationConfig")
    # The native resolver also handles legacy model.config defaults. Resolve on
    # the request copy, then pass that exact configuration back to generate.
    request_model.generation_config = copy.deepcopy(request_model.generation_config)
    provided = kwargs.pop("generation_config", None)
    config, model_kwargs = request_model._prepare_generation_config(provided, **kwargs)
    mode = config.get_generation_mode()
    if mode not in (GenerationMode.SAMPLE, GenerationMode.GREEDY_SEARCH):
        raise SeedSamplingError(f"seed does not support native generation mode {mode}")
    function = GenerationMixin._sample
    if function.__closure__ is not None or "multinomial" not in function.__code__.co_names:
        raise SeedSamplingError("seed does not support this Transformers sampling implementation")
    native_globals = dict(function.__globals__, torch=sampling)
    native_sample = FunctionType(function.__code__, native_globals, function.__name__, function.__defaults__)
    native_sample.__kwdefaults__ = function.__kwdefaults__
    entered = []

    def sample(_self, *args, **sample_kwargs):
        entered.append(True)
        return native_sample(_self, *args, **sample_kwargs)

    request_model._sample = MethodType(sample, request_model)
    return {**model_kwargs, "generation_config": config, "use_model_defaults": False}, entered, mode


class _CheckedMinimumNewTokens:
    def __init__(self, processor, floor=None):
        self.processor = processor
        self.floor = processor.min_new_tokens if floor is None else floor

    def __call__(self, input_ids, scores):
        adjusted = self.processor(input_ids, scores)
        active = input_ids.shape[-1] - self.processor.prompt_length_to_skip < self.floor
        if active and torch.any(~torch.isfinite(adjusted).any(dim=-1)):
            raise MinimumNewTokensError("min_new_tokens leaves no finite eligible token")
        return adjusted


class _MinimumSelectionGuard:
    """Refuse when later native processors invalidate an active EOS floor."""

    def __init__(self, processor, floor=None):
        self.processor = processor
        self.floor = processor.min_new_tokens if floor is None else floor

    def __call__(self, input_ids, scores):
        if input_ids.shape[-1] - self.processor.prompt_length_to_skip < self.floor:
            eos_mask = torch.isin(torch.arange(scores.shape[-1], device=scores.device),
                                  self.processor.eos_token_id.to(scores.device))
            if (torch.any(~torch.isfinite(scores).any(dim=-1))
                    or torch.any(~torch.isneginf(scores[..., eos_mask]))):
                raise MinimumNewTokensError(
                    "min_new_tokens leaves no finite eligible token or conflicts with later processors")
        return scores


class _CheckedRepetitionPenalty:
    """Apply the official operation in f32, preserving only existing -Inf masks."""

    def __init__(self, processor):
        self.processor = processor
        self.calls = 0

    def __call__(self, input_ids, scores):
        masked = torch.isneginf(scores)
        scores = scores.float()
        if torch.any(~torch.isfinite(scores) & ~masked):
            raise RepetitionPenaltyNumericsError(
                "repetition_penalty requires finite float32 logits or existing -inf masks")
        adjusted = self.processor(input_ids, scores)
        if (torch.any(~torch.isfinite(adjusted) & ~masked)
                or torch.any(masked & ~torch.isneginf(adjusted))):
            raise RepetitionPenaltyNumericsError(
                "repetition_penalty produced non-finite float32 logits")
        if torch.any(~torch.isfinite(adjusted).any(dim=-1)):
            raise RepetitionPenaltyNumericsError(
                "repetition_penalty has no finite selectable logits")
        self.calls += 1
        return adjusted


def _generate_native_checked(model, *, minimum_to_enforce=None, sampling=None,
                             stop_tokenizer=None, stop_strings=None, stop_result=None, **kwargs):
    """Guard native repetition in place without changing resident model state.

    Public custom processors run after sanitizers and masks in Transformers.
    A request-local shallow copy instead wraps the resolved built-in processor
    at its original position, retaining generation defaults and processor order.
    Weights remain shared. Custom generation that bypasses this builder is
    unsupported and fails closed rather than returning unchecked output.
    """
    # Inherited native floors retain Transformers' warning/forced-EOS semantics.
    # SDAR retries supply an internal floor and explicitly preserve authorship.
    if minimum_to_enforce is None:
        minimum_to_enforce = kwargs.get("min_new_tokens")
    penalty = _resolve_repetition_penalty(model, kwargs.get("repetition_penalty"))
    request_model = copy.copy(model)
    if request_model is model or not callable(getattr(request_model, "_get_logits_processor", None)):
        raise RepetitionPenaltyNumericsError(
            "native repetition checking requires the Transformers logits processor builder")
    stop_criterion = None
    if stop_tokenizer is not None and _resolve_stop_strings(model, stop_strings):
        kwargs, stop_criterion = _prepare_native_stop(
            request_model, stop_tokenizer, stop_strings, kwargs, minimum_to_enforce)
        penalty = _resolve_repetition_penalty(request_model, kwargs["generation_config"].repetition_penalty)
    entered = None
    if sampling is not None:
        kwargs, entered, mode = _prepare_seeded_native(request_model, sampling, kwargs)
        penalty = _resolve_repetition_penalty(request_model, kwargs["generation_config"].repetition_penalty)
        draws_before = sampling.calls
    build_processors = request_model._get_logits_processor
    checked = []
    built = False

    def build_checked(_self, *args, **builder_kwargs):
        nonlocal built
        processors = build_processors(*args, **builder_kwargs)
        minimum_guards = []
        for index, processor in enumerate(processors):
            if type(processor) is RepetitionPenaltyLogitsProcessor:
                guard = _CheckedRepetitionPenalty(processor)
                checked.append(guard)
                processors[index] = guard
            elif minimum_to_enforce and type(processor) is MinNewTokensLengthLogitsProcessor:
                processors[index] = _CheckedMinimumNewTokens(processor, minimum_to_enforce)
                minimum_guards.append(_MinimumSelectionGuard(processor, minimum_to_enforce))
        if penalty != 1.0 and not checked:
            raise RepetitionPenaltyNumericsError(
                "native generation did not provide its repetition processor")
        processors.extend(minimum_guards)
        built = True
        return processors

    request_model._get_logits_processor = MethodType(build_checked, request_model)
    outputs = request_model.generate(**kwargs)
    if entered is not None and (not entered or (mode == GenerationMode.SAMPLE and sampling.calls == draws_before)):
        raise SeedSamplingError("native generation bypassed isolated seed sampling")
    if not built or (penalty != 1.0 and not any(guard.calls for guard in checked)):
        raise RepetitionPenaltyNumericsError(
            "native generation bypassed repetition checking")
    if stop_criterion is not None and not stop_criterion.calls:
        raise StopStringError("native generation bypassed stop string checking")
    if stop_criterion is not None and stop_result is not None:
        stop_result.append(stop_criterion.matcher)
    return outputs


def _resolve_repetition_penalty(model, repetition_penalty):
    """Use the model's generation default only when the request omits it."""
    if repetition_penalty is None:
        repetition_penalty = getattr(getattr(model, "generation_config", None),
                                     "repetition_penalty", 1.0)
    if isinstance(repetition_penalty, bool) or not isinstance(repetition_penalty, (int, float)):
        raise ValueError("repetition_penalty must be a finite positive number")
    try:
        penalty = float(repetition_penalty)
    except OverflowError as exc:
        raise ValueError("repetition_penalty must be a finite positive number") from exc
    if not math.isfinite(penalty) or penalty <= 0:
        raise ValueError("repetition_penalty must be a finite positive number")
    return penalty


def _resolve_top_k(model, top_k):
    """Resolve top_k from arg or model generation config."""
    if top_k is not None:
        try:
            return int(top_k)
        except Exception:
            return 0
    gen_cfg = getattr(model, "generation_config", None)
    cfg_top_k = getattr(gen_cfg, "top_k", None) if gen_cfg is not None else None
    try:
        return int(cfg_top_k) if cfg_top_k is not None else 0
    except Exception:
        return 0


def _resolve_min_new_tokens(model, min_new_tokens, max_tokens):
    """Resolve the EOS floor without allowing a promise beyond the token budget."""
    authored = min_new_tokens is not None
    if min_new_tokens is None:
        min_new_tokens = getattr(getattr(model, "generation_config", None),
                                 "min_new_tokens", None)
        if min_new_tokens is None:
            min_new_tokens = 0
    if (isinstance(min_new_tokens, bool) or not isinstance(min_new_tokens, int)
            or not 0 <= min_new_tokens <= (1 << 32) - 1):
        raise ValueError("min_new_tokens must be a non-negative u32 integer")
    if authored or min_new_tokens:
        if (isinstance(max_tokens, bool) or not isinstance(max_tokens, int)
                or not 0 < max_tokens <= (1 << 32) - 1):
            raise ValueError("max_tokens must be a positive u32 integer for min_new_tokens")
    if authored and min_new_tokens > max_tokens:
        raise ValueError("min_new_tokens must not exceed max_tokens")
    # Manual loops stop at the request budget even for a larger model default.
    return min(min_new_tokens, max_tokens)


def _minimum_processor(prompt_length, floor, eos_ids, device):
    if not floor or not eos_ids:
        return None
    return _CheckedMinimumNewTokens(MinNewTokensLengthLogitsProcessor(
        prompt_length, floor, sorted(eos_ids), device=device))


def _apply_minimum(processor, history, scores):
    return scores if processor is None else processor(history, scores)


def _sample_next_token(logits, temperature, top_p, top_k=0, sampling=None):
    """Sample one token from logits with temperature + top-k + top-p."""
    if temperature <= 0:
        return logits.argmax(dim=-1, keepdim=True)

    logits = logits / max(temperature, 0.01)
    if top_k and top_k > 0:
        # Match Transformers: k above the vocabulary retains every token.
        values, _ = torch.topk(logits, min(top_k, logits.size(-1)))
        logits = torch.where(logits < values[..., -1, None], float("-inf"), logits)
    if top_p < 1.0:
        # Match Transformers TopPLogitsWarper, including ties and p=0:
        # remove the low-probability tail and retain at least one token.
        sorted_logits, sorted_indices = torch.sort(logits, descending=False)
        cum_probs = torch.cumsum(torch.softmax(sorted_logits, dim=-1), dim=-1)
        mask = cum_probs <= (1.0 - top_p)
        mask[..., -1] = False
        scatter_mask = torch.zeros_like(logits, dtype=torch.bool).scatter(-1, sorted_indices, mask)
        logits = logits.masked_fill(scatter_mask, float("-inf"))

    probs = torch.softmax(logits, dim=-1)
    return (torch if sampling is None else sampling).multinomial(probs, num_samples=1)


def _eos_ids(tokenizer, model=None):
    eos = getattr(getattr(model, "generation_config", None), "eos_token_id", None)
    if eos is None:
        eos = getattr(tokenizer, "eos_token_id", None)
    if eos is None:
        return set()
    if isinstance(eos, int):
        return {int(eos)}
    if isinstance(eos, (list, tuple)):
        return {int(x) for x in eos if x is not None}
    return set()


def _generate_sdar_cached(model, tokenizer, device, formatted_prompt,
                          max_tokens, temperature, top_p, top_k=None, repetition_penalty=None,
                          min_new_tokens=None, sampling=None):
    """TraDo/SDAR decode loop with explicit store_kv=True cache updates."""
    floor = _resolve_min_new_tokens(model, min_new_tokens, max_tokens)
    inputs = tokenizer(formatted_prompt, return_tensors="pt").to(device)
    input_ids = inputs["input_ids"]
    prompt_len = input_ids.shape[1]
    eos_ids = _eos_ids(tokenizer)
    minimum_processor = _minimum_processor(
        prompt_len, floor, eos_ids | _eos_ids(tokenizer, model), device)
    resolved_top_k = _resolve_top_k(model, top_k)
    penalty_processor = _CheckedRepetitionPenalty(RepetitionPenaltyLogitsProcessor(
        _resolve_repetition_penalty(model, repetition_penalty)))
    token_history = input_ids

    past_key_values = DynamicCache()
    position_ids = torch.arange(prompt_len, device=device).unsqueeze(0)
    causal = torch.tril(torch.ones(prompt_len, prompt_len, device=device, dtype=torch.bool))
    attention_mask = causal.unsqueeze(0).unsqueeze(0)  # [B,1,Q,K]

    with torch.no_grad():
        outputs = model(
            input_ids,
            attention_mask=attention_mask,
            position_ids=position_ids,
            past_key_values=past_key_values,
            use_cache=True,
            store_kv=True,
        )
        logits = outputs.logits[:, -1, :]

    generated_ids = []
    cur_pos = prompt_len

    for _ in range(max_tokens):
        next_token = _sample_next_token(_apply_minimum(
            minimum_processor, token_history, penalty_processor(token_history, logits)),
                                        temperature, top_p, resolved_top_k, sampling)
        token_id = int(next_token.item())
        if token_id in eos_ids:
            break
        generated_ids.append(token_id)
        token_history = torch.cat([token_history, next_token], dim=-1)

        with torch.no_grad():
            outputs = model(
                next_token,
                position_ids=torch.tensor([[cur_pos]], device=device),
                past_key_values=past_key_values,
                use_cache=True,
                store_kv=True,
            )
            logits = outputs.logits[:, -1, :]
        cur_pos += 1

    full_sequence = torch.cat(
        [input_ids[0].detach().cpu(), torch.tensor(generated_ids, dtype=input_ids.dtype)],
        dim=0,
    ).tolist()
    if not generated_ids:
        return "", full_sequence, past_key_values
    return tokenizer.decode(generated_ids, skip_special_tokens=True), full_sequence, past_key_values


def _continue_sdar_cached(model, tokenizer, device, formatted_prompt,
                          max_tokens, temperature, top_p,
                          cached_token_ids, past_key_values, top_k=None, repetition_penalty=None,
                          min_new_tokens=None, sampling=None):
    """Continue SDAR/TraDo decoding from a previously captured KV cache.

    `formatted_prompt` is treated as a suffix to append to the existing cached
    context represented by `cached_token_ids` + `past_key_values`.
    """
    floor = _resolve_min_new_tokens(model, min_new_tokens, max_tokens)
    if not cached_token_ids:
        raise RuntimeError("Live KV cache is missing cached token_ids")
    if past_key_values is None:
        raise RuntimeError("Live KV cache is missing cache data")
    if not hasattr(past_key_values, "crop"):
        raise RuntimeError("Live KV cache does not support crop-based replay")

    resolved_top_k = _resolve_top_k(model, top_k)
    penalty_processor = _CheckedRepetitionPenalty(RepetitionPenaltyLogitsProcessor(
        _resolve_repetition_penalty(model, repetition_penalty)))
    eos_ids = _eos_ids(tokenizer)
    full_sequence = [int(token_id) for token_id in cached_token_ids]
    cur_pos = len(full_sequence)

    suffix_inputs = tokenizer(
        formatted_prompt,
        return_tensors="pt",
        add_special_tokens=False,
    ).to(device)
    suffix_token_ids = suffix_inputs["input_ids"][0].detach().cpu().tolist()

    logits = None

    if suffix_token_ids:
        for token_id in suffix_token_ids:
            next_input = torch.tensor([[int(token_id)]], device=device)
            with torch.no_grad():
                outputs = model(
                    next_input,
                    position_ids=torch.tensor([[cur_pos]], device=device),
                    past_key_values=past_key_values,
                    use_cache=True,
                    store_kv=True,
                )
                logits = outputs.logits[:, -1, :]
            full_sequence.append(int(token_id))
            cur_pos += 1
    else:
        replay_position = cur_pos - 1
        past_key_values.crop(replay_position)
        replay_token = torch.tensor([[full_sequence[-1]]], device=device)
        with torch.no_grad():
            outputs = model(
                replay_token,
                position_ids=torch.tensor([[replay_position]], device=device),
                past_key_values=past_key_values,
                use_cache=True,
                store_kv=True,
            )
            logits = outputs.logits[:, -1, :]

    minimum_processor = _minimum_processor(
        len(full_sequence), floor, eos_ids | _eos_ids(tokenizer, model), logits.device)
    generated_ids = []
    for _ in range(max_tokens):
        token_history = torch.tensor([full_sequence], device=logits.device, dtype=torch.long)
        next_token = _sample_next_token(_apply_minimum(
            minimum_processor, token_history, penalty_processor(token_history, logits)),
                                        temperature, top_p, resolved_top_k, sampling)
        token_id = int(next_token.item())
        if token_id in eos_ids:
            break
        generated_ids.append(token_id)
        full_sequence.append(token_id)

        with torch.no_grad():
            outputs = model(
                next_token,
                position_ids=torch.tensor([[cur_pos]], device=device),
                past_key_values=past_key_values,
                use_cache=True,
                store_kv=True,
            )
            logits = outputs.logits[:, -1, :]
        cur_pos += 1

    if not generated_ids:
        return "", full_sequence, past_key_values
    return tokenizer.decode(generated_ids, skip_special_tokens=True), full_sequence, past_key_values


def _generate_autoregressive(model, tokenizer, device, formatted_prompt,
                             max_tokens, temperature, top_p, top_k=None, repetition_penalty=None,
                             min_new_tokens=None, sampling=None, stop_strings=None):
    """Generate a complete response using standard autoregressive decoding.

    Args:
        model: The loaded model.
        tokenizer: The loaded tokenizer.
        device: torch.device to use.
        formatted_prompt: Already-formatted prompt string.
        max_tokens: Maximum number of new tokens.
        temperature: Sampling temperature.
        top_p: Nucleus sampling threshold.

    Returns:
        Decoded string of generated text.
    """
    if min_new_tokens is not None:
        _resolve_min_new_tokens(model, min_new_tokens, max_tokens)
    _resolve_stop_strings(model, stop_strings)
    inputs = tokenizer(formatted_prompt, return_tensors="pt").to(device)

    resolved_top_k = _resolve_top_k(model, top_k)
    # Validation also covers direct worker calls; omitted kwargs leave the
    # Transformers generation_config default in charge.
    resolved_penalty = _resolve_repetition_penalty(model, repetition_penalty)

    with torch.no_grad():
        gen_kwargs = {
            "max_new_tokens": max_tokens,
            "temperature": max(temperature, 0.01),
            "top_p": top_p,
            "do_sample": temperature > 0,
        }
        if top_k is not None or resolved_top_k > 0:
            gen_kwargs["top_k"] = resolved_top_k
        if repetition_penalty is not None:
            gen_kwargs["repetition_penalty"] = resolved_penalty
        if min_new_tokens is not None:
            gen_kwargs["min_new_tokens"] = min_new_tokens
        stop_result = []
        outputs = _generate_native_checked(model, sampling=sampling, stop_tokenizer=tokenizer,
                                           stop_strings=stop_strings, stop_result=stop_result,
                                           **inputs, **gen_kwargs)

    input_len = inputs["input_ids"].shape[1]
    generated = outputs[0][input_len:]
    text = tokenizer.decode(generated, skip_special_tokens=True)
    # Use the matcher from the exact resolved request configuration, including
    # legacy native defaults, rather than resolving the resident defaults again.
    return stop_result[0].text if stop_result else text


def _generate_autoregressive_streaming(model, tokenizer, device,
                                       formatted_prompt, max_tokens,
                                       temperature, top_p, top_k=None, repetition_penalty=None,
                                       min_new_tokens=None, sampling=None, stop_strings=None):
    """Generate tokens one at a time for streaming output.

    Args:
        model: The loaded model.
        tokenizer: The loaded tokenizer.
        device: torch.device to use.
        formatted_prompt: Already-formatted prompt string.
        max_tokens: Maximum number of new tokens.
        temperature: Sampling temperature.
        top_p: Nucleus sampling threshold.

    Yields:
        Dicts with {"mode": "append", "text": ...} for each token.
    """
    floor = _resolve_min_new_tokens(model, min_new_tokens, max_tokens)
    markers = _resolve_stop_strings(model, stop_strings)
    matcher = _GeneratedTextStop(tokenizer, markers, floor) if markers else None
    inputs = tokenizer(formatted_prompt, return_tensors="pt").to(device)
    input_ids = inputs["input_ids"]
    eos_ids = _eos_ids(tokenizer)
    minimum_processor = _minimum_processor(
        input_ids.shape[-1], floor, eos_ids | _eos_ids(tokenizer, model), device)
    resolved_top_k = _resolve_top_k(model, top_k)
    penalty_processor = _CheckedRepetitionPenalty(RepetitionPenaltyLogitsProcessor(
        _resolve_repetition_penalty(model, repetition_penalty)))

    generated_ids = []
    for _ in range(max_tokens):
        with torch.no_grad():
            outputs = model(input_ids)
            logits = outputs.logits[:, -1, :]

            next_token = _sample_next_token(_apply_minimum(
                minimum_processor, input_ids, penalty_processor(input_ids, logits)),
                                            temperature, top_p, resolved_top_k, sampling)

        if next_token.item() in eos_ids:
            break

        if matcher is None:
            token_str = tokenizer.decode(next_token[0], skip_special_tokens=True)
            yield {"mode": "append", "text": token_str}
        else:
            generated_ids.append(int(next_token.item()))
            matcher.observe(generated_ids)
            chunk = matcher.take()
            if chunk:
                yield {"mode": "append", "text": chunk}
            if matcher.stopped:
                return

        input_ids = torch.cat([input_ids, next_token], dim=-1)
    if matcher is not None:
        chunk = matcher.take(final=True)
        if chunk:
            yield {"mode": "append", "text": chunk}
