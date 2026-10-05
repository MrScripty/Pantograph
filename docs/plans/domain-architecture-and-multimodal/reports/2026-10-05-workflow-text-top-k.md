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
- Base: PR54 `d6e9fcd15b135bedf36437ab2eceba229a0c9e2c`, verified against
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

The parent's separate accepted PR54 repair series (`c83d5179`, `029ac704`,
`cd54e12`) is not included or duplicated. Graph qualification, reservation and
observation files remain outside this write set. Parent owns composition, PRs,
review and merge.

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
