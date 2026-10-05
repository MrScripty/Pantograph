# Optional graph-authored nucleus sampling

## Contract and actual sampling behavior

This separate feature starts from frozen temperature `e0293ebf` and incorporates
CI correction `fddae90d4e2802031a9f1c8e6d556626dcb69148`, tree
`c233101f95d2bb518c59d4411022cefdf47950ba`, parent `a6d1fd15`. The correction
remains a separate commit/branch; the integration merge is `4ce57295`, with
parents `e0293ebf` and `fddae90d`. Frozen source and Library evidence remain
unchanged. Parent owns review/publication and is holding publication for owner
confirmation. No PR or review requests were made.

The canonical text descriptor adds optional F64 `top_p`, range [0, 1], no step
and no default. The supported Transformers 4.53.3
[TopPLogitsWarper implementation](https://github.com/huggingface/transformers/blob/v4.53.3/src/transformers/generation/logits_process.py)
accepts both endpoints and retains at least one token at zero. Its implemented
range, actual generation execution and existing SamplingGenerationOptions
derive this contract; no arbitrary tuning limit is added.

Omission leaves the graph generation option absent and preserves the existing
PyTorch/worker default 1.0, which disables nucleus filtering. Existing worker
defaults override model-config top_p as before. Temperature zero bypasses all
sampling and returns argmax; positive temperature retains the existing 0.01
floor. Positive-temperature sampling applies temperature, then top_k, then
top_p. Explicit top_p zero retains one token and still samples that retained
distribution, so tied maxima need not choose the same index as greedy argmax.

Streaming already had a nucleus filter, but its descending cumulative/shift
algorithm disagreed with non-streaming Transformers at ties and cutoffs.
The expanded real tests produce 112 failures against the frozen sampler.
The bounded repair sorts ascending, removes the low-probability tail at
`cumulative <= 1 - p`, and retains at least one token, matching Transformers.
Sampling membership can therefore change at those ties/cutoffs; omission,
p=1, greedy temperature and established top_k semantics stay intact. The
same helper serves standard streaming and cached SDAR sampling; no model or
masked-diffusion acceptance is claimed.

## Projection and validation

The host reuses the frozen finite JSON-number value and checked f32 conversion.
The shared finite-number contract itself is unchanged, pending independent
temperature review. The text projection shares numeric-input handling for
temperature/top_p, checks [0, 1], and sets only the optional existing
SamplingGenerationOptions.top_p. Negative, above-one, wrong-type, overflow,
positive-to-zero underflow and extra authored precision fail before runtime
loading/gateway execution. Existing integer inputs retain their types and
validation. Existing selection-input graph values materialize as numeric host
inputs without truncation. No scheduler schema, client DTO or dependency pin
changed.

## Evidence and CI correction

- 74 shared tests pass: 22 inference-interface, 45 runtime-host unit and 7
  runtime-host accessor tests.
- 690 PyTorch-enabled inference library tests pass. New gateway and backend
  tests preserve optional/default/zero/unit-boundary values, mapped diagnostics,
  cleanup and both worker-envelope operations. These are transport evidence.
- Six real CPU tests pass with Torch 2.14.1+cpu and Transformers 4.53.3.
  The new test covers 375 combinations: three distinct/tied/uniform logits,
  five omitted/boundary/interior p choices, five omitted/greedy/floored/interior
  temperatures, and five omitted/zero/bounded/oversized/max k choices. It uses
  the actual worker decoder, actual Transformers GenerationMixin loop, actual
  streaming helper and real torch.multinomial draws. Every probability tensor
  and generated token sequence agrees. The expanded batched oracle separately
  checks filtering against Transformers at zero and cutoff boundaries.
- Embedded-runtime library/test Clippy with normal defaults, backend-pytorch
  and `ORT_SKIP_DOWNLOAD=1` passes without warnings. This compiles descriptor,
  host rejection/projection and authored-source/workflow tests; it does not
  execute those native-dependent tests.
- The corrected CI script runs all six real tests against installed CPU
  dependencies locally, with model Hub access disabled. Discovery now requires
  at least six tests. Fresh provisioning and hosted execution remain pending.

The original `a6d1fd15` step lacked pipefail and could mask unittest failure;
its previous failure-propagation claim was incorrect. Separate successor
`fddae90d` sets `shell: bash` and `set -euo pipefail`, and corrects its
[report](2026-10-05-cpu-sampler-hosted-qualification.md). The exact corrected
script's controlled red/green check returns 1 for failed tests and 0 for passed
tests while retaining both logs. A green run from the old step alone cannot
qualify real sampling.

```sh
HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 python -m unittest discover -s crates/inference/torch/tests -v
```

Logs: `/tmp/top-p-frozen-source-regression.log`, `/tmp/top-p-real-sampling.log`,
`/tmp/top-p-contract-tests.log`, `/tmp/top-p-inference-tests.log`,
`/tmp/top-p-embedded-clippy.log`, `/tmp/top-p-corrected-ci-local.log` and
`/tmp/cpu-sampler-pipeline-red-green.log`. No model weights or ONNX download
were used. Formatting, whitespace, critical, scheduler-only and traceability
checks pass. These results do not establish pretrained-model or complete
workflow acceptance.

## Remaining qualification

Parent must review the separate top_p/filtering milestone, integrate any finite
contract corrections without rewriting history, and run exact-head native:

```sh
cargo test -p pantograph-embedded-runtime --features backend-pytorch --lib runtime_host_text_execution::tests::
cargo test -p pantograph-embedded-runtime --features backend-pytorch --lib inference_interface_facts_provider::tests::
cargo test -p pantograph-workflow-service --lib workflow::runtime_host_task_input_mapping::tests::
cargo test -p pantograph-workflow-service --lib workflow::external_input_materialization::tests::
```

Earlier public-session and native host qualification requirements remain open.
Native/ONNX execution stays deferred. DA-03, DA-07 and required-real
text-to-image acceptance remain open; see [plan](../plan.md).
