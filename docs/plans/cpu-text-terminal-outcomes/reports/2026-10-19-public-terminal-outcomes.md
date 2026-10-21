# Public CPU text terminal outcomes

The bounded T4 defect is fixed and qualified on clean source
`511374122d82a07d953baf322bbd137a5002bad7`. Public workflow calls preserve the
runtime failure cause and return typed cancellation. Independent GPT-6.1 Sol / High
review admitted source and exact runtime evidence without material findings.
Trained-model T3 remains blocked; this report does not qualify model readiness.

| Public scenario | Observed result | Cleanup |
| --- | --- | --- |
| Generate | nonempty `:::eeeeeeeeeeeeeeeeeeeee` through text-output | passed |
| Corrupt disposable copied weights | `InternalError`, retaining `runtime_host.execution_failed`, `pytorch_worker_model_load_failed` and header-deserialization cause | passed |
| Cancel actual Running task | typed `Cancelled` with `runtime_host.cancellation_requested`; matching run/task/ack active-attempt/terminal event | passed |

All three report `probe_passed`, `integration_passed` and
`public_result_projection_qualified` true, with `t3_qualified` false and no
truncated diagnostics. Every session queue is empty and idle/unloaded; session
close and shutdown returned; no runtime instance, model, reservation, active
claim or uncertain residency remains. Failed loader history is retained as
terminal Failed metadata with resources released. Cancellation is early active
cancellation; it does not prove interruption after generation begins.

## Scope and implementation

The existing finalization owner now reads staged results before its completeness
guard. A TerminalFailed record must match a Failed result's workflow, run, node
and task identity. Its first Error diagnostic preserves producer order. Exact
runtime cancellation/shutdown codes project to Cancelled; other errors project
to Internal with code/message. Incomplete or incoherent facts retain the generic
InvalidRequest guard. Failed producers are inspected before blocked descendants.
The actual runtime result remains authoritative; no phase hint is invented.

Serial, resumed and batch continuation already use this helper. No scheduler
state, serialized schema, admission/fairness policy, task-result mapping, cleanup
sequence or M3 caller changed. The new probe exits unsuccessfully if public
projection is unqualified, after owner cleanup is observed.

## Exact evidence and reproduction

[Source/results](source-and-results-2026-10-19.json) record code hashes and full
scenario outcomes. [Retained evidence](evidence/2026-10-19/) contains only this
isolated synthetic run's public JSON/logs and affected test/static logs, compressed
with gzip; its manifest hashes every uncompressed record. No weights, databases,
credentials or operator catalogs are committed. Retained records may contain isolated evidence and source-worktree paths.

```bash
source /workspace/pantograph-tools/activate.sh
export LD_LIBRARY_PATH=/workspace/pantograph-tools/python/cpython-3.12.3-linux-x86_64-gnu/lib
export PYTHONPATH=/workspace/Pantograph/.venv/lib/python3.12/site-packages
bash scripts/qualify-cpu-text-workflow.sh \
  511374122d82a07d953baf322bbd137a5002bad7 \
  /workspace/text-t4-evidence/exact-checkpoint
```

The command completed exit 0 on that clean checkpoint; source-status.txt is
empty. Reproduction requires a new absolute evidence root and a clean checkout
at the requested SHA. The runbook documents bounds and prerequisites. This used
Rust 1.92.0, Python 3.12.3, CPU-only Torch and the already prepared seven-package
linked interpreter, with two Cargo jobs. Generation is capped at 24 new tokens,
one loaded model session and a 60-second execution timeout. No model download,
remote code or Python-direct substitute bypasses public Resolve/Check/scheduling.

| Check | Result |
| --- | --- |
| Focused finalization owner | 8 passed |
| Full workflow-service library | 992 passed |
| Full embedded library, PyTorch + host inventory | 619 passed, 3 ignored |
| Both owners lib/tests/example Clippy, warnings denied | exit 0 |
| Affected rustfmt, Bash syntax, staged traceability, whitespace | pass |
| Clean-checkpoint public probe, all three scenarios | exit 0 |
| Independent source and exact evidence review | admitted |

The default backend-disabled service suite emits the existing inference unused
SelectedTextLoad-fields warning. The selected CPU-feature Clippy gate passes
with warnings denied. Its command is:

```bash
cargo clippy --locked -p pantograph-workflow-service -p pantograph-embedded-runtime \
  --features pantograph-embedded-runtime/backend-pytorch,pantograph-embedded-runtime/host-dependency-inventory \
  --lib --tests --example cpu_text_generation_workflow_probe -- -D warnings
```

## Baseline, coordination and remaining inputs

This is stacked on frozen PR80 head
`040969de5245001e0c13d96caa46fb86b2c1df3a`, main
`4aec0d5c4927a6af2832d40e224be5c2136fc9ef`. PR80 source and evidence, original
worktree, README and M3 Start caller files remain unchanged. Parent confirmed M3
owns session_scheduler_runner.rs/task_execution_worker.rs at e9d89eac and leaves
our finalization owner untouched. No branch was merged, rewritten or deleted.
The new protected worktree/branch remains for review and parent integration.

PR80 CI monitor most recently showed CPU Composition success, Headless Workflow
and Runtime Separation running, and Quality Gates/CPU Text queued. This is not a
CI-green claim. The focused stacked fix PR must retain its own exact-head CI
status; pending jobs require follow-up rather than a fabricated success.

The official DistilGPT2 revision
`2290a62682d06624634c1f46a6ad5be0f47f38aa` config endpoint previously failed at
proxy CONNECT HTTP403; that denial was not retried or bypassed. Missing input:
verified local trained safetensors/config/tokenizer, immutable upstream revision
and authoritative license evidence. An equivalent licensed trained ordinary local
causal LM within the same CPU/no-custom-code/600 MiB contract could satisfy T3
with its own provenance. The untrained fixture cannot substitute.

Full text-to-image qualification additionally needs verified licensed text and
image model assets, an actual composed public text-to-image execution producing
image artifacts/output presentation, and failure/cancellation/cleanup at those
real boundaries. No GPU, native UI, quality or performance claim is made.
