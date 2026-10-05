# Workflow text top-k

Text-generation graph descriptors now expose optional `top_k` as an existing
U64 input, with integer steps from zero through `u32::MAX` and no default.
The host converts it to the existing `GenerationOptions.sampling.top_k`.
Omission preserves backend defaults; explicit zero disables top-k filtering.
A supplied top-k value creates generation options even without a token limit.
Token limits retain their positive-only range, and system prompts retain their
exact string content and existing byte bounds.

## Source and ownership

- Branch: `feat/workflow-text-top-k`.
- Feature base: PR54 `d6e9fcd15b135bedf36437ab2eceba229a0c9e2c`, verified against
  `refs/pull/54/head` before creating the branch.
- Pumas pin: `2243a2b6909fcf4fe4b4ffa0f7f0a2b4ca027d32`, unchanged.
- Pattern: [workflow system prompt](2026-10-04-workflow-system-prompt.md).
- Write set: descriptor producer, text host projection/tests, shared descriptor
  fixture/contract tests, existing recording gateway and PyTorch envelope tests,
  and this evidence report.

No new value variant, wire version, scheduler mechanism, or backend capability
is introduced. The text host still admits only the existing selected PyTorch
Transformers runtime/device path. The gateway already forwards typed top-k into
chat JSON, and the PyTorch adapter already forwards it into the worker's
allowlisted Transformers kwargs. Passing those contracts does not establish
support for other runtimes, model families, or dLLM/Sherry sampling behavior.

The feature milestone excludes the parent's separate accepted PR54 repair
series (original local identities `c83d5179`, `029ac704`, `cd54e12`). Graph
qualification, reservation and observation implementation remain outside the
feature write set. The later published composition is recorded below. Parent
owns PRs, review and merge.

## Qualification

- `cargo test --locked -p pantograph-inference-interface-contracts`: 20 passed.
  The shared descriptor validates and round-trips the optional zero-inclusive
  integer port alongside the existing token limit and system prompt.
- `cargo test --locked -p inference --features backend-pytorch --lib`: 686
  passed, none ignored. The selected recording gateway covers omitted, zero,
  positive and maximum top-k values crossed with omitted/present token limit
  and system prompt; exact JSON omission/value, system/user order, mapped option
  diagnostics and completed cleanup are asserted. Worker-envelope tests cover
  zero and `u32::MAX` in both generation operations.
- Embedded host tests cover the same option combinations through the host port
  and recording gateway, plus signed/unsigned boundaries, negative values,
  wrong types and overflow. Invalid execution must reject with no outputs or
  backend load/generation calls. Native execution remains deferred.
- `ORT_SKIP_DOWNLOAD=1 cargo clippy --locked -p pantograph-embedded-runtime
  --features backend-pytorch --lib --tests`: passed with normal defaults intact,
  no warnings. This compiler check covers the descriptor producer and new host
  tests; it neither downloads nor links/executes ONNX. The initial narrowed
  profile lacked the baseline gateway constructor and was replaced with this
  supported profile; the new recording-field fixture initializer was fixed
  before the passing check.
- Formatting, diff whitespace, critical antipattern and accessibility gates
  passed; 27 accessibility checker and 28 traceability checker tests passed.

Commands use the existing activated Rust 1.92.0 and Python 3.12 toolchain with
bounded Cargo jobs. Local logs: `/tmp/top-k-contract.log`,
`/tmp/top-k-inference.log`, `/tmp/top-k-embedded-default-clippy.log` and
`/tmp/top-k-{critical,a11y,traceability-tests}.log`.

Hosted/native follow-through must run the embedded descriptor and host tests at
the final composition, including the parent's repair series:

```sh
cargo test --locked -p pantograph-embedded-runtime --lib inference_interface_facts_provider::tests
cargo test --locked -p pantograph-embedded-runtime --lib runtime_host_text_execution::tests
cargo test --locked -p pantograph-embedded-runtime --lib
```

These native commands were not run locally. ONNX download/link execution and
its existing CDN 403 remain deferred; there was no download retry, runtime
substitution, dependency repin or network-policy change. GUI, real model
inference and dependent text-to-image acceptance remain open.

## Published PR54 repair composition

The feature remains independently available at
`9d6646a47c0dda8d391266970e160f0fb53aacb3`, tree
`6b4e09ae6b7ad80cf010babb1096072ce6a1edad`, with plan reconciliation at
`b08fdb61b9c7cb93f9f888960340f0cd1452cb70`.

The parent subsequently published the repair series as
`f56e2b5a` → `f9fb470c` → `78bc71931772d891a6b5555076a072a63fe969a7`,
tree `141a0eae52ec7aa587536da297dd14a652836431`.
The actual PR54 remote head and source ancestry were verified before its
conflict-free merge into this feature branch. All five repair paths match the
published repair bytes, including graph diagnostics, session attribution,
bounded host observations and the runtime operations guide. No repair was
reimplemented or edited. The parent's hosted CI on the repair head remains
separate from final composed-feature acceptance.

The composed source passes the same normal-default embedded library/test
compiler-Clippy command with ONNX downloading disabled, with no warnings
(`/tmp/top-k-composed-clippy.log`). Formatting, whitespace and critical gates
also pass. The six feature code/fixture paths still match milestone `9d6646a4`
exactly, so its 20 contract and 686 inference test results retain that unchanged
scope. The composed native host tests remain unexecuted locally; successful
compilation is not native or real-model acceptance.
