# Runtime observation qualification results

Local branch: `scheduler/service-observation-qualification`, based on preserved
`fd224c366645c19f868c36ed331264a82b79db68`. Design checkpoint: `e0044ae8`.
See [the scoped design and architectural stops](scheduler-v2-production-observation-design.md).

## Delivered

The existing selected-text timing guard now adds its monotonic clock epoch,
capture timestamp, constructor provenance and actual backend load disposition.
`Reloaded` separates load from execution but does not assert cold caches. A
read-only gateway clock snapshot and pure fresh-observation accessor refuse
missing legacy metadata, stale/future/different-clock observations, unknown load
state, injected owners/clocks, changed identity/generation, failed/abandoned
attempts, and missing/duplicate/synthetic/incomplete phases. Separate custody,
load, execution and worker-drain costs remain separate.

Optional identity hashing borrows the complete payload and preflights 64 KiB raw
bytes, 2048 nodes and depth 32. It leaves oversized identity unknown rather than
truncating it. Oversized request-ID correlation hashing is omitted while actual
request IDs continue unchanged. Clock regressions invalidate capture; completion
time cancellation excludes an observation without changing execution behavior.

`BuiltIn` describes trusted gateway construction, not calibrated production.
Capture occurs at guard drop immediately after finish, with no intervening await.
These are trusted in-process records, not authenticated replayable JSON evidence.
Production backends still supply no exact timing owner facts, so usable production
calibration remains unavailable. No native collector/store, aggregation, scheduler
timing-source bridge or default-policy activation was added.

## Validation

Combined-feature library suite: **2732 passed, zero failures, four existing
ignored tests** across inference (820), node-engine (343), embedded runtime (578),
host contracts (46), scheduler library (3), timing contracts (8), workflow service
(934), and interface contracts (0). This includes the existing owned audio,
rerank and bounded dependency-session paths. Ignored real-runtime tests were not
run. Deterministic fixtures cover separate phases, freshness boundaries, identity
and clock changes, synthetic refusal, actual load outcomes, cancellation/failure/
caller drop, raw budgets and disabled defaults.

All-target Clippy passed for the same eight packages and combined features with
`--locked --offline -- -D warnings -A dead_code`; the dead-code allowance is the
existing inference-field qualification allowance, with no new lint suppression.
`cargo fmt --all --check` and `git diff --check` passed. Clippy log:
`/workspace/pantograph-cache/service-observation-qualification-clippy.log`.

The independent reviewer approved the source and independently executed **23
focused cases**, all passing: 17 inference timing/bounds, five timing contracts
and the native selected-text host timing test. The native test proves a fake owner
with a steady clock stays injected and its original 1 MiB request ID still reaches
the response, while oversized telemetry correlation is omitted.

Reproduction (from the repository, after activating the existing local tools):

```sh
export ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1
export PYO3_PYTHON=/workspace/Pantograph/.venv/bin/python
export LD_LIBRARY_PATH=/workspace/pantograph-tools/python/cpython-3.12.3-linux-x86_64-gnu/lib
export PYTHONPATH=/workspace/Pantograph/.venv/lib/python3.12/site-packages
export XDG_CONFIG_HOME=/workspace/pantograph-cache/test-config
cargo test -p pantograph-timing-contracts -p inference -p node-engine \
  -p pantograph-scheduler -p pantograph-workflow-service \
  -p pantograph-embedded-runtime -p pantograph-runtime-host-contracts \
  -p pantograph-inference-interface-contracts --lib --no-default-features \
  --features backend-llamacpp,backend-candle,backend-pytorch,backend-audio,standalone \
  --locked --offline
```

Test log: `/workspace/pantograph-cache/service-observation-qualification-tests.log`.
No actual model execution, new model/runtime/ONNX download, public write, PR,
credential or network-setting change occurred. The baseline branch still points
to `fd224c36`; separate inspector/audio/rerank functional owners were not edited.

## Stop

The next production slice needs authoritative worker implementation/effective
configuration/physical-device facts and immutable model identity, a bounded
storage and cross-generation equivalence contract, and owner observations for
preparation/transfer/lease release/reconcile/conditional residency and capacity.
Gateway worker drain alone does not satisfy those contracts. No fabricated
timing numbers or cross-generation pooling were introduced. Library archive
materialization is still unavailable locally, so C3 hash verification and
simulator differential testing remain blocked. This is observation groundwork,
not the completed research scheduler or a measured production predictor.
