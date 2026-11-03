# Optional graph-authored text temperature

## Contract and ownership

This separate feature follows frozen composition `60197971` and the small
[CPU CI qualification follow-up](2026-10-05-cpu-sampler-hosted-qualification.md),
`a6d1fd15adf94949c2474e7afcd78ecb08c1c733`. Published frozen histories
`174c1950`, `3bf3eb45` and `60197971` remain unchanged. Parent coordinates
publication and independent review; no PR54 update or review request was made.

The canonical text-generation descriptor adds optional `temperature`: scalar
F64, range zero through the existing generation contract's finite `f32::MAX`,
no step and no default. This is a representation bound, not a product tuning
limit. Omission leaves the generation option absent and preserves PyTorch's
existing 0.7 default. Zero selects greedy decoding. Positive values sample;
the existing backend floors positive sampling temperatures at 0.01 in both
streaming and non-streaming generation. That backend policy is unchanged.

The shared host input gains additive `F64(serde_json::Number)`. This reuses the
existing scheduler JSON-number domain, keeps Eq-compatible finite values, and
rejects nonfinite/wrong-type wire values. Existing I64/U64 wire variants and
integer port validation stay intact. Execution contract version 2 remains;
older hosts can reject the new variant, so a graph using temperature requires
the updated host. No parallel scheduler schema, float wrapper or client DTO
was added.

`RuntimeHostExecutionInputValue::try_as_f32` checks the actual conversion into
the existing generation domain. It accepts exactly representable binary f32
values or their canonical shortest JSON decimal spelling, including ordinary
`0.7`. Extra authored precision such as `0.7000000000000001`, integer
`16777217`, `2^53+1`, overflow and positive-to-zero underflow fail explicitly.
Negative temperatures and wrong types fail before runtime loading or gateway
calls. Integer checks compare the original integer even when conversion to f64 would round.
The two contract crates enable serde_json `float_roundtrip`: testing exposed
default parser drift at the maximum representable bound. Dependency versions
and the lockfile are unchanged.

Floats pass from an existing selection-input or numeric JSON-filter result
through the scheduler's existing Json(Number) result to the host finite number.
The number-input remains an integer source; float values are never truncated
into token-limit or top-k inputs. An authored-source integration test traces
selection-input external bindings through source projection and host
materialization. Host projection maps only the optional sampling temperature
into GenerationOptions; the existing gateway and worker keep their established
mapping, default, cleanup and diagnostic behavior.

## Evidence

- Shared contracts: 21 inference-interface, 45 runtime-host unit and 7
  runtime-host accessor tests pass, including finite-number wire rejection,
  round trips and the actual checked f32 conversion.
- `cargo test -p inference --features backend-pytorch --lib`: 688 tests pass.
  New selected-text gateway coverage checks omission, zero, 0.7 and f32 maximum
  with optional token limits/system prompts, mapped diagnostics and cleanup.
  Backend envelope tests cover generate and stream operations; these are
  transport checks, not sampling proof.
- `HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 python -m unittest discover -s
  crates/inference/torch/tests -v`: five real CPU tests pass using installed
  Torch `2.14.1+cpu` and Transformers `4.53.3`. The new matrix uses the actual
  worker decoder and actual Transformers GenerationMixin loop on fixed tiny
  logits, alongside the actual streaming sampler. It compares generated
  tokens and every probability tensor while delegating draws to real
  torch.multinomial. Zero gives exact argmax and makes no sampling call;
  omission/positive/floored-small/large temperatures retain established
  semantics. No weights or model downloads are needed. The CI follow-up's
  discovery command now includes this fifth test.
- Normal-default embedded-runtime library/test Clippy compilation with
  backend-pytorch and `ORT_SKIP_DOWNLOAD=1` passes. This compiles host,
  descriptor, workflow mapping and authored-source tests; it does not execute
  them. Formatting, whitespace, critical, scheduler-only and traceability
  gates pass.

Logs: `/tmp/temperature-contract-tests.log`,
`/tmp/temperature-inference-tests.log`, `/tmp/temperature-real-sampling.log`
and `/tmp/temperature-embedded-final-clippy.log`. The earlier frozen review
packet remains available as Library
`libfile_cdf0c1d747c0819183dfb308ea0875d0`; its contents were not regenerated.

## Remaining qualification

Parent-hosted exact-head runs must execute:

```sh
cargo test -p pantograph-embedded-runtime --features backend-pytorch --lib runtime_host_text_execution::tests::
cargo test -p pantograph-embedded-runtime --features backend-pytorch --lib inference_interface_facts_provider::tests::
cargo test -p pantograph-workflow-service --lib workflow::runtime_host_task_input_mapping::tests::
cargo test -p pantograph-workflow-service --lib workflow::external_input_materialization::tests::
```

The two repaired public-session scenarios at `174c1950` and native host tests
at `60197971` retain their separate qualification requirement. Peer source
review accepts those two repairs; hosted execution and fresh CI CPU dependency
provisioning remain pending. Native/ONNX stays deferred. Required-real
text-to-image acceptance and DA-03/DA-07 remain open; this feature makes no
pretrained-model or complete-workflow acceptance claim. See [plan](../plan.md).
