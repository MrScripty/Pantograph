# Local two-completion results

Branch: `scheduler/bounded-two-completion`, based on remote main verified as
`a8483e511dcec4f36e269e6e4debf181a318222f`. The design checkpoint is `a7cc0d5e`.
The source-reviewed native integration hardening candidate is
`51fdd7f8dce3764f6cc994bfcc9c1540f6a1b357`; its parent native opt-in is `42f31c65`.
No push or merge occurred. Cold-Candle `6338b71a` remains separate, including its
workflow_service_composition.rs overlap. This follow-up does not touch that file,
runtime_registry.rs, inference_interface_facts_provider.rs, inspector normalization
or rerank/audio adapters.

## Implemented scope

`select_scheduler_candidate_with_two_completions` is a fresh Rust pure evaluator
for a declared serialized two-task prefix. The admitted first task stays fixed.
Every first placement supplies its own qualified post-completion successor
capacity and timing snapshot, including release/reconciliation/retention scope.
The complete successor offer universe, intent, readiness proof and environment
are frozen. Unknown capacity or a missing branch/stage invalidates comparison.
Future capacity is not derived by subtracting a lease and warmth is not inferred
from equal model identity. These are trusted caller projections, not authenticated
production measurements or executable resource permissions.

Limits are four offers per snapshot, four branches, sixteen full path evaluations
and thirty-two completion-event evaluations, with bounds on consumed nested
collections and raw text. All feasible alternatives are qualified. A partial
population never wins. Checked integer scoring minimizes terminal prefix
completion, then the sum of its two completions, then stable placement IDs.
Missing/incomparable evidence preserves the exact one-decision result; oversized
comparisons refuse before either selector scans/clones them. No clock, runtime,
reservation, owner mutation or default policy change occurs in this evaluator.

The synthetic cold/reuse fixture explicitly separates preparation from execution:
first A is 0+2; first B is 4+1. After A, B needs 4+1; after B, B needs 0+1 under
the fixture's qualified retention assumption. One-decision ranking chooses A;
two-completion ranking chooses B, with terminal completion 6 instead of 7.
This demonstrates the scoped mechanism; it is not a production speedup claim.

Native hardening fences exact task, Ready/version, persisted dependency proof and
opt-in inputs under the same lock as start. Changed evidence cannot start an
attempt or transfer custody; prepared selected-only provisional claims roll back.
Provider callbacks stay outside the store lock. Raw diagnostic padding is rejected
before bounded ranking/fallback and native owner callbacks.

## Verification and independent review

All commands used `ORT_SKIP_DOWNLOAD=1`, `--locked --offline`; no model/runtime
downloads or ONNX build downloads occurred. Tests used the writable test config
directory. Native tests selected `--no-default-features --features backend-llamacpp`.

* Complete scheduler suite: 156 passed, one ignored cost probe.
* Workflow-service library suite: 926 passed, including snapshot-version,
  persisted-proof and scalar-input replacement with no attempt and custody rollback.
* Embedded-runtime library suite: 524 passed, one ignored native cost probe;
  controlled public-session dispatch, capacity change and cancellation still pass.
* Targeted policy suites: completion ranking 13 passed; two-completion 10 passed.
  Both are included in the complete scheduler count. Independent reviewer reran
  these 23 tests and found no remaining pure/local correctness blocker.
* Acceptance includes a separately written exhaustive oracle over 200 tiny
  matrices, the declared objective/tie distinction, capacity gained/lost/unknown,
  frozen successor proof, stage/context/provenance failures, pointer association,
  stable offer/branch/row permutations, both arithmetic overflows, exact budget
  exhaustion and oversized offers/rows/branches/budgets/raw diagnostics.
* All-target Clippy passed for scheduler, workflow-service and embedded-runtime
  with `-D warnings -A dead_code` (existing disabled-feature inference fields).
  Scheduler-only all-target Clippy passed without that allowance.
* Formatting and `git diff --check` passed.

The independent reviewer identified the original snapshot and raw-text holes,
then the conditional-context prevalidation and successor-proof comparison gaps;
all were fixed before committing the evaluator. A full runner re-Ready race using
real registry predecessor-lineage custody remains broader acceptance coverage.
Current tests directly exercise the authoritative store fence and rollback token,
while native regression tests retain the actual registry/session lifecycle path.

## Actual cost, distinct from modeled completion

Measurements used the available 5-CPU Xeon 8573C environment without concurrent
compilation. Optimized evaluator evidence was frozen before timing; it excludes
contract validation, profile collection, registry work and workflow snapshot fences.
These use controlled fixture fields, not maximum nested-length inputs.
Each row has 2,000 calls. Process totals are 0.072459 user CPU seconds, 0.010310
system CPU seconds, 0.088485 wall seconds and 8,704 KiB peak RSS.

| Offers per task | Median | p95 | Maximum |
| --- | ---: | ---: | ---: |
| 1 | 1.580 us | 1.639 us | 241.859 us |
| 2 | 4.525 us | 4.773 us | 2,471.759 us |
| 4 | 15.152 us | 21.663 us | 6,868.045 us |

The controlled native provider probe includes resource evaluation, request
validation, native queries/source lookup, one-decision ranking, selected commit,
facts construction and custody rollback. It excludes source refresh, the new
workflow snapshot fence, actual backend work and two-completion native wiring.
Process totals: 4.487856 user CPU seconds, 0.018927 system CPU seconds, 4.509568
wall seconds, 34,148 KiB peak RSS.

| Offers / input bytes | Samples | Median | p95 | Maximum |
| --- | ---: | ---: | ---: | ---: |
| 1 / 256 | 1,000 | 151 us | 539 us | 6,732 us |
| 2 / 256 | 1,000 | 209 us | 609 us | 2,632 us |
| 64 / 256 | 500 | 3,411 us | 6,489 us | 10,670 us |
| 64 / 61,440 | 200 | 6,482 us | 19,448 us | 168,990 us |

These are observations, not elapsed-time bounds or a proven default dispatch
budget. The larger native path still warrants cost reduction before default use.
Evidence logs are in `/workspace/pantograph-cache/two-completion-*.log`.

## Exact remaining boundaries

Native activation requires an owner-certified actual successor/cohort snapshot,
serialized execution semantics across branch/batch paths, conditional capacity
and acknowledged release/reconciliation/retention evidence, real calibrated timing
production and invalidation, and cohort/first-action version revalidation. Current
native queries describe only the current admitted task and cannot establish these.
PriorityThenFifoSchedulerPolicy, starvation boosts and its one-position warm-reuse
window remain unchanged. No CPU/GPU overlap, native batching, preload/eviction,
multi-device stage planning, remote worker placement or full workflow search is
implemented here; the detailed production/missing map remains in
[migration plan](scheduler-v2-production-migration.md).

Library thesis/evaluation text informed the objective, frozen population,
conditional service costs, nonanticipation, bounds and revalidation principles.
Archive materialization still returned `library file transfer failed: download failed`;
no readable archive bytes or C3 byte hash were verified locally. That missing input
does not block this independent design. No POC implementation was copied or translated.
