# M1: hosted retained CPU first-two-Ready qualification

The opt-in serial CPU service now composes with ordinary hosted readiness and
candidate providers. Its constructor-owned execution, batch and linear cleanup
ports survive composition. A readiness-deferred run resumes through the same
supervised TaskWorker path as submission. Default dispatch remains unchanged.

This milestone starts at Rust public graph and execution-session APIs. It does
not qualify the editor/UI with this scheduler mode. The earlier browser demo
remains distinct baseline evidence; no new screenshot or GUI claim is made.
The authoritative scope and C1–C6 criteria are in the [plan](../plan.md).

## Shared owner and accounting prerequisites

The caller must use the same `Arc` gateway, registry and Pumas selector for the
ordinary retaining bootstrap, `EmbeddedRetainedCpuSerialPort`, and hosted
composition. Keep an ordinary public `keep_alive: true` execution session alive
through the serial runs. The port requires a resident native owner and a separate
retaining successor; this change adds neither cold bootstrap nor sole-lease
recovery to it. Arbitrary supplied ports cannot be identity-checked by the
composition. Combining serial Ready mode with generic completion timing fails
before registry/selector/readiness-producer side effects.

The probe declares a 32 MiB retained RAM envelope for the fixture and known-zero
VRAM for the calibrated CPU-only Candle implementation. These are explicit
operator declarations, not allocator measurements. The ordinary conservative
RAM/VRAM task claims and cleanup validation remain intact. An omitted resource
kind is unknown; an exploratory run correctly refused it.

## Reproduce the actual cohort

From the repository root, using Rust 1.92.0 and the locked dependencies:

```sh
ORT_SKIP_DOWNLOAD=1 cargo build --release --locked \
  -p pantograph-embedded-runtime --no-default-features \
  --features backend-candle,host-dependency-inventory \
  --example cpu_embedding_workflow_probe
probe_parent="$(mktemp -d)"
probe_root="$probe_parent/run"
PUMAS_REGISTRY_DB_PATH="$probe_root/registry.db" \
HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
  "${CARGO_TARGET_DIR:-target}/release/examples/cpu_embedding_workflow_probe" \
  "$probe_root" --serial-ready
```

Use `--offline` when the locked build dependencies are already cached, as they
were for this qualification. Finish compilation before running the cohort. The
probe requires a new absolute directory; do not point it at a private model
library. Its temporary published Pumas owner disables networking and process
management and contains only the copied committed fixture. No local/unpublished
Pumas source or private catalog is used.

The fixed graph contains two independent embedding tasks, separate ordinary
dependency sidecars, and vector outputs. Inputs come from the committed
`truncation_texts`: 16 tokens after truncation for `infer_a`, three for `infer_b`.
Authoring obtains the actual backend interface, requires executable validation,
resolves dependencies and publishes validation through the existing public
APIs. It does not inject Ready proofs, candidates, clocks, history or outputs.
Pending readiness resumes the same run ID.

Serial mode requires `timeout_ms: None`: its supervised native path does not
enforce workflow deadlines and rejects timed submissions/resumes explicitly.
The probe wraps caller waiting in a 30-second watchdog; that bound is not a
backend deadline or permission to release resources while native work drains.

The process performs one ordinary retaining bootstrap, eight baseline runs, then
eight serial runs with exactly the same graph, texts and shared owner. Both
compositions/readiness producers remain alive throughout both measured cohorts;
ordering is baseline first and serial second, so thermal/time-order effects are
not randomized. Each
serial run must yield one genuine first-two-Ready proposal and one actual Start
for each pair member. The oracle independently groups the existing ring's raw
durations by workload and compares maxima/counts with the proposal and actual
Start order. Three fresh successful samples are required before ranking; ties
choose position zero. No run is repeated to obtain a preferred winner. An absent
second-position winner leaves that claim pending and exits nonzero.

Each output is checked against the independent committed Transformers reference
with absolute tolerance `1e-5`. Only successful native drain plus matched cleanup
can add a sample. After closing all sessions, shutdown must leave no reservation
or native serial owner. This is the precise meaning of `shutdown_ok`; the
registry can conservatively retain `stopping` plus the declared resident charge.
It does not assert a cleared runtime row or zero resident accounting. On failure,
evidence is retained and the process exits nonzero.

Evidence includes `baseline-run-N.json`, `serial-run-N.json`, both diagnostic
event exports, `serial-proposals.jsonl`, `serial-result.json`, validation and
Resolve snapshots, and registry snapshots before/after shutdown. The proposal
collector is process-local and bounded to 64 rows, 8 KiB per row and 128 KiB total;
it writes files after execution. The production projection uses the existing
log facade after store/history locks are released. It is a proposal, not a new
durable execution event or cleanup capability.

## Backend, model and environment

- Candle CPU F32, one configured private CPU thread; pinned Candle commit
  `88ed7911de9e88196b1f55b199145d22647f415e`.
- Published Pumas `aa1d79ae2d53f0acb088bc85223ecc2ff46af12b`, read-only contracts.
- Committed untrained one-layer BERT-8 fixture, 6,464-byte weights, SHA-256
  `549cf713305780cd5af92cf397e6be34ada0e3650f967bb9772fa08ba9cedd44`.
  Independent reference: Transformers 4.53.3 / Torch 2.14.1+cpu.
- No model acquisition or paid inference: zero additional model bytes. The
  [existing fixture/license qualification](../../../real-inference-workflow-probe.md#model-and-acquisition)
  applies; no pretrained quality or separate model license is asserted.
- Linux x86_64, AMD EPYC 9V74, five exposed logical CPUs, four-CPU cgroup quota,
  16 GiB memory limit and no swap. No exposed GPU path.

## Evidence and measurement disposition

The [release summary](m1-evidence/release-summary.json) records the complete
fixed 8+8 cohort on the final corrected source. All 32 eight-dimensional outputs
match the reference (maximum absolute error `8.940696716308594e-08`); independent
recalculation confirms raw sample counts/maxima and both actual Start IDs exactly
once in the selected order. Three FIFO training runs were followed by five
ranked runs. All five correctly selected position **zero**. The first ranked
maxima were 13,054,876 ns for `infer_a` and 13,102,873 ns for `infer_b`; later
maxima were 15,165,646 ns and 16,592,732 ns. Selecting the second position would
have disagreed with the observed evidence.

The release probe therefore exits **1**, with
`second_position_claim: pending_no_observed_second_winner`. This is a failed
second-position qualification criterion, not a model/runtime failure. **M1 is
not fully accepted: C3 remains partial pending final-source second-position
evidence.** No cohort was rerun to seek a preferred winner. The prior
[dev cohort](m1-evidence/dev-summary.json) observed five actual second-position
starts and validates that integration checkpoint, but predates the timeout
restriction and symmetric producer setup. It does not substitute for the final
release qualification.

| Measurement | Baseline min / median / max | Serial min / median / max |
| --- | --- | --- |
| Workflow wall ms, 8 runs | 49.647 / 54.031 / 71.099 | 50.219 / 63.955 / 108.311 |
| Queue placement to inference Start ms, 16 tasks | 9 / 21 / 42 | 11 / 32.5 / 89 |
| Inference attempt ms, 16 tasks | 10 / 12 / 14 | 10 / 12.5 / 19 |
| Cleanup-qualified service ms, 16 samples | not recorded by default mode | 9.946 / 12.205 / 16.593 |
| Selector-only cost µs, 8 proposals | not applicable | 16.505 / 19.089 / 41.403 |
| Wall minus serial service ms, 8 runs | not applicable | 28.756 / 36.483 / 84.414 |

These are repeated descriptive observations, with baseline first, no concurrent
compilation/model work, and no statistical significance or speedup claim. The
serial median is higher in this cohort. Both live compositions/readiness
producers were present in both phases. The fixed fixture is too small to treat
text length as an assumed service-time winner.

Workflow wall time uses a monotonic clock around public submission/readiness
resume through response; it excludes session creation/close and subsequent
vector verification. Queue and attempt durations come from existing wall-clock
ledger timestamps at millisecond resolution. Queue-to-Start includes readiness
and preceding work; attempt duration includes task lifecycle. Service samples
use real monotonic dispatch-through-drain and matched cleanup. `decision_ns`
measures only `mode.select`, excluding pair capture, log projection and Start
fencing. **Total scheduler CPU remains unisolated.** Wall minus service includes
readiness, orchestration, output/diagnostic work and scheduling; it is not a
scheduler CPU measurement. These explicit scopes satisfy C5's limited reporting
obligation, not a latency improvement criterion.

The [compressed source evidence and checksums](m1-evidence/SHA256SUMS) preserve
both full result files, proposals, baseline/serial events and final registry
snapshots. The compact summaries retain all 16 raw serial durations, workload
digests, per-run ages/counts/maxima, exact Start identities and final vectors.
`gzip -dc` reads each original JSON/JSONL export; its hash is recorded in the
summary. Final reservations are empty and native owner absence is asserted by
the probe; conservative `stopping`/32 MiB declared residency remains recorded.

## Review and gates

Two independent reviewers approved the restricted implementation and recomputed
the real output/order evidence. Their findings produced the InvalidClock
regression, explicit timeout refusal, same-background-work comparison and precise
shutdown/measurement scopes. Both require C3's remaining claim to stay pending.
Final local tests: workflow-service **979 passed**; embedded-runtime **592 passed,
two pre-existing tests ignored**. New native cancellation and all composition
regressions are included. Declared pre-push `npm test`: **261 node-engine passed / one existing ignored**,
**168 workflow-nodes passed**. `cargo fmt --all -- --check`, staged whitespace and
decision traceability pass; no hook was disabled. The ignored tests are existing
explicit cost probes plus the existing node-engine ignore. No new ignore was
added. External CI is recorded in the final handoff. Normal CI must still run on
the published draft head; C6 is not marked complete before that evidence exists.
No merge, deployment, GPU, remote-worker, native batching, full research scheduler
or default-policy activation is implied.

### CI test-scope correction

Initial published head `743e783cad979940bb0eea12fb642d36bd1614f2` failed the
warning-deny Clippy audit on the new timeout-refusal test: an explicit
`drop(store)` did not express the mutex guard's lexical lifetime to Clippy.
The follow-up encloses only those store assertions in a block before awaiting
serial admission. It changes no production/inference behavior and adds no lint
suppression. Focused warning-deny Clippy and the two timeout tests validate the
correction; normal CI runs again on the successor head.

The [measurement source manifest](m1-evidence/source-manifest.json) and raw
evidence remain unchanged and bind the qualified source snapshot at `743e783c`.
The later delta is test scope plus this publication record; it is not a new
measurement cohort, and the C3 pending disposition remains unchanged.
