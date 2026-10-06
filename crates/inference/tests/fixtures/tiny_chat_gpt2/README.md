# Synthetic CPU chat fixture

`model-11` and `model-37` contain locally initialized, **untrained** GPT-2 models
with 5,392 parameters each. `generate.py` constructs official installed
Transformers/Tokenizers classes directly; it downloads no weights, tokenizer,
config, or executable model code. These artifacts test routing and deterministic
execution, not language quality. They are test data generated for this repository.

Each directory includes an explicit character-tokenizer chat template, safe model
serialization, eight native `GPT2LMHeadModel.generate` oracles and a SHA-256
manifest. The tokenizer uses standard GPT-2 `input_ids`/`attention_mask` inputs;
segment-ID tokenizers are outside this fixture's coverage. Oracles cover two
prompt/system contexts and seeds 0, 42, 43 and `u64::MAX`, with four generated
tokens and fixed sampling controls. Reproduction is version-sensitive: the
recorded environment uses Torch 2.14.1+cpu and Transformers 4.53.3.

With those official dependencies, Tokenizers, Accelerate and SoundFile installed:

```sh
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1
python crates/inference/tests/fixtures/tiny_chat_gpt2/generate.py
python crates/inference/tests/fixtures/tiny_chat_gpt2/qualify.py
```

Regeneration intentionally rewrites fixtures and manifests; review the resulting
diff. `qualify.py` loads the actual full worker and local model, observes real
forward inputs, checks the explicit template and both streaming/nonstreaming
results, and confirms that seeded requests leave ambient Torch RNG unchanged.
It does not stub worker imports or replace model forwards.

The ignored Rust test
`saved_cpu_chat_graph_runs_native_owner_with_template_controls_and_scope_isolation`
must be run explicitly with `--ignored --nocapture` in a qualified CPU Python
environment. It saves/reopens public graphs and calls the actual selected loader,
gateway and worker with controlled package/readiness/dispatch facts. It also
checks separately scoped sequential envelope members and precancellation.
Live Pumas discovery, pretrained quality, GPU, desktop execution, post-start
native cancellation, other architectures and tokenizer fallbacks remain separate.
