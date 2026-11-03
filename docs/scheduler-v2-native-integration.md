# Native bounded completion ranking integration

Local branch: `scheduler/bounded-completion-v2`, based on verified remote main
`a8483e511dcec4f36e269e6e4debf181a318222f`. No push or main merge.
The scheduler is fresh native Rust work; no POC code was copied or translated.

## Opt-in and evidence ownership

`EmbeddedHostedStartupCompositionInput::with_completion_timing` injects an
`EmbeddedCompletionTimingOptIn`. It carries a native `Arc` timing source, owner
lifetime, maximum sample age and explicit configured-estimate permission.
Default is absent. Public session requests cannot submit timing numbers or turn
this on. Existing scheduler priority/FIFO/starvation/warm-reuse policy is unchanged.

The callback receives the exact admitted task/Ready identity, immutable artifact
fingerprint, runtime/backend/device/optional variant, effective trait settings,
runtime source context, materialized typed input values, residency key, current
runtime generation/state and admission-resource observation. Missing content
identity or unavailable/oversized inputs produces no usable evidence. Materialized
inputs are obtained read-only from the task's referenced persisted results using
the same scalar conversion as host execution; default providers do no extra work.

This is a trusted native injection boundary, not authentication or calibration.
The implementation must be bounded/nonblocking and know actual physical-device,
implementation/configuration, generation, workload size/content and comparable
resource conditions. Media references require owner-known content/shape; a
reference alone is insufficient. Return None for unqualified data. Existing
coarse trace averages and gateway service attempts cannot be used automatically:
they lack a compatible precommit workload/configuration/residency profile and
explicit transfer cost. Reused service timing value/outcome types distinguish
successful observations, owner-configured estimates, failed partial work and
unknown values. Query echo equality binds the current snapshot, not history.
Stopped/no-instance is explicitly observed unloaded state, never an invented
loaded generation. A native source must not blindly rebind an old loaded-generation
sample as a prediction for the current unloaded owner.

All eligible alternatives need complete preparation + required transfer +
execution costs under the same source class and convention. Preparation includes
load/warmup/owner delay; execution includes cleanup. Unknown transfer is never
zero. One complete successful record per alternative is the minimum; this does
not establish statistical calibration. Mixed provenance, unknown/failed stages,
foreign records, stale/future samples and disabled configured estimates retain
legacy sole-eligible/otherwise-ambiguous selection. Diagnostics retain the result
and explicitly identify Synthetic as owner-configured estimates. The comparison
clock is captured after callbacks to avoid classifying fresh records as future.

## Work bounds and dispatch safety

At most 64 alternatives/records, 8 device IDs per offer, 32 request diagnostics,
8 resource/domain observations per query, 32 scalar trait settings, 16 materialized
inputs and 64 KiB serialized input data; context text is at most 128 bytes, trait
strings at most 1024 bytes. String length checks precede scanning/serialization.
Cohort bounds precede query copies and source calls. Alternatives share one
immutable typed-input slice; its serialized bound is checked once per decision,
rather than repeating large-payload copies and scans per alternative. Oversized policy input
refuses before legacy scan; insufficient evidence preserves conservative behavior.
The wrapper bounds its own work; a trusted injected source must honor its stated
nonblocking work contract. Source refresh, request validation and registry
operations retain their separate existing contracts and costs.

The source receives resource evaluation only; no speculative lease is acquired.
Ranking picks an ID, then the unchanged selected commit revalidates current
capacity/generation/ownership before publication. The normal Ready-to-start,
dispatch validation, reservation binding and rollback/transfer custody remain.
Session/runtime ownership, dependency readiness, cancellation and artifact custody
are preserved. No GPU discovery, hardware execution or new batching capability
is introduced. The controlled image fixture traverses the existing singleton
batch envelope and fake backend, never a native multi-member GPU batch.

## Qualification

Three public-session integration tests cover 12 controlled runs:

- Configured cold 2s load + 3s execution beats warm 12s execution; changing only
  cold load to 10s selects warm instead. A third run structurally checks
  Observed/Completed value routing using authored constants; it is not a measured
  hardware latency sample.
- Unknown workload, unknown transfer, stale record, foreign device binding,
  failed stage, configured estimates disabled and default opt-in absent preserve
  ambiguity. No host execution or lease occurs.
- A test-only capacity change after evaluation prevents selected commit and
  dispatch. Public active-task cancellation at the real host boundary releases
  selected custody. Successful output artifact bodies remain readable after
  dispatch; completion leaves no reservation.

The tests use the actual embedded provider/resource source, provisional commit,
registry custody, Pumas local resolver/package facts, embedded host port and
reservation lifecycle port. Owner runtime/capacity metadata, timing and backend
execution are explicitly controlled fixtures. The package detail cache is prepared
locally before the Pumas local-only resolver; no model/runtime download occurs.

Regression validation: 1,528 library tests pass across embedded-runtime (524),
workflow-service (926), runtime-registry (75) and scheduler (3); the 12 completion
policy integration tests also pass. The controlled native cost probe passes when
run explicitly; it is excluded from ordinary test runs. Rust 1.92, locked/offline,
`ORT_SKIP_DOWNLOAD=1`, embedded feature `backend-llamacpp`, default features off.
Pumas test registry uses an isolated writable XDG config directory. Changed-file
rustfmt and `git diff --check` pass. All-target clippy passes with `-D warnings
-A dead_code`; the latter allows the existing feature-specific unused fields in
`inference::selected_text_execution`, without editing that worker's surface. Independent reviewer reran the three public
session tests and closed the workload/bounds/freshness/host-fixture findings.

The following native cost probe uses the unoptimized debug test binary on the
same 5-CPU Xeon8573C executor. It includes owned candidate preparation, resource
evaluation, request validation, timing lookup/association, completion ranking,
selected provisional commit, fact validation and custody rollback, plus registry
snapshot assertions. It excludes source refresh, session admission/input lookup,
real history qualification/calibration and runtime execution. All capacities,
workloads and timing values are controlled fixtures, not device measurements.
There were no concurrent builds during the final probe.

| Alternatives | Prompt bytes | Samples | Median | p95 | Maximum |
| --- | --- | --- | --- | --- | --- |
| 1 | 256 | 1,000 | 0.156 ms | 0.478 ms | 2.250 ms |
| 2 | 256 | 1,000 | 0.211 ms | 0.570 ms | 2.642 ms |
| 64 | 256 | 500 | 3.507 ms | 8.135 ms | 25.328 ms |
| 64 | 61,440 | 200 | 6.772 ms | 12.183 ms | 82.701 ms |

Process CPU: 4.414502s user + 0.009251s system; wall 4.448547s; peak RSS 34,824 KiB.
An earlier implementation repeatedly serialized/copied the large input across
alternatives and reached 258.279 ms p95 while a clippy build was active. That
finding prompted the shared immutable payload and once-per-decision validation;
the before/after runs are not a controlled hardware speedup comparison.

These results do not establish a production dispatch SLA or statistical bound.
The 64-alternative path is not demonstrated to meet a sub-millisecond default
budget; default scheduling stays unchanged. Default enablement requires optimized
whole-dispatch profiling with a real qualified source, admission/input lookup and
source refresh included, a stated cost budget, and conservative refusal under
that owner-defined budget. The fixed work caps here are deterministic operation
bounds; measured maxima show why they must not be described as elapsed-time
bounds.

## Composition overlap for deliberate integration

Separate reviewed cold-Candle candidate:
`6338b71a55371317c5ace6adbeacdec469c96950`, based on the same main. It is not
assumed merged or cherry-picked. This work does not edit `runtime_registry.rs`,
its tests or `inference_interface_facts_provider.rs`.

The overlapping `workflow_service_composition.rs` changes are limited to:

- Import the timing opt-in type; add optional timing fields to the private native
  factory/startup composition inputs, initialized to None.
- Add the public native startup builder `with_completion_timing`, without changing
  `EmbeddedHostedStartupConfig` or session request fields.
- Forward that option through startup to the private factory, and through both
  hosted composition calls to the resource-backed candidate provider.
- Add one optional argument to private `resource_backed`; its existing test passes
  None. A narrow argument-count lint allowance keeps explicit composition
  dependencies together.

No backend enrollment, cold availability or CPU capability projection is changed
here. Review those adjacent factory calls deliberately when integrating the
Candle candidate; do not replace either worker's composition file wholesale.

## Remaining work

No general production calibration/history store or owner-qualified cross-run
forecast producer is installed. Native owners can supply qualified bounded
records; unavailable owners retain ordinary behavior. Next work needs a real
owner-qualified measurement/profile source and admitted-workload conditioning
before any default enablement, followed by bounded joint ready-frontier/multi-event
planning and semantic differential acceptance. This slice ranks runtime/device
alternatives for one admitted Ready task and does not reorder the public queue.

CPU compute slots/throughput, GPU compute/link capacity, app-wide protected-service
accounting, native joint batching forecasts, model residency forecasts and remote
node placement remain missing beyond the existing production foundation mapped in
[the migration plan](scheduler-v2-production-migration.md). No remote-node support
or complete researched scheduler is claimed.

Library textual reports were read, but authorized local materialization of thesis,
evaluation and archive failed with `library file transfer failed: download failed`.
No local archive bytes/hash or simulator differential execution is verified here.
The supplied C3 hash and CPU audit remain supplied evidence. No alternative storage
routes or repeated retries were used.
# Reviewed follow-up

The follow-up on `scheduler/bounded-two-completion` fences the exact task,
Ready record/version, persisted readiness proof and opt-in materialized inputs
under the same store lock as task start. A changed snapshot rejects prepared
selection and drops its selected provisional custody before any attempt starts.
Provider callbacks remain outside that lock. Raw diagnostic message/hint lengths
are now checked before bounded ranking, fallback or native owner callbacks.
These close two findings from independent review of `42f31c65`.

The separately callable two-completion evaluator remains pure/local; see
[design checkpoint](scheduler-v2-two-completion-checkpoint.md). It does not
activate native successor forecasting or alter the default queue policy.
