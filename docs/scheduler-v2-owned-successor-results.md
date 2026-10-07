# Owned dependency completion slice

Implemented on `scheduler/owned-dependency-lookahead`, following the
[design checkpoint](scheduler-v2-owned-successor-design.md). This is an opt-in,
local two-task prefix, not the full researched scheduler. The production default
remains `PriorityThenFifoSchedulerPolicy`, including starvation boosts and the
one-position warm-reuse window. Timing source capability defaults false and its
conditional forecast defaults `None`; no production calibration source was added.

## Caller and ownership

The actual session scheduler captures one Ready predecessor and its unique
AwaitingInputs runtime successor. It supports exactly two unfinished runtime
tasks, identical immutable model/task/environment configuration, completed other
dependencies, known scalar inputs and explicit symbolic predecessor-output
bindings. Unsupported cohorts decline forecasting. The graph/record scan is
capped at 128 entries before the two task snapshots are cloned. Each task has at
most 16 dependencies/bindings, 32 traits and a capped 64 KiB serialized snapshot;
raw collection/string checks precede bounded serialization and equality.

The successor offer catalogue is advisory: no Ready proof, fabricated input,
environment preparation, resource evaluation or lease is created for the future
task. Native forecast queries identify the exact current admitted workload,
owned successor snapshot, descriptor/configuration, artifact, runtime/device and
post-release state. The source must qualify successful Ephemeral release and
reconciliation, retained/unloaded residency, known resource accounting and
complete preparation/transfer/execution costs. Execution includes cleanup under
the existing serialized-service convention. Echoes, fingerprints and sample
counters establish association; they do not authenticate calibration history.

The pure policy enumerates at most four first offers by four successor offers:
16 plans and 32 represented completion events. It orders by terminal prefix
completion, then represented completion sum, then deterministic candidate IDs.
It requires a complete legal comparison matrix. Unknown, stale, mismatched,
uncalibrated or contradictory evidence falls back to freshly checked frozen
one-decision evidence. Overflow or exceeded work limits fails conservatively.
Measured native comparisons require at least three qualified successful samples;
authored configured fixtures require explicit synthetic opt-in. The adapter caps
conditional callbacks at 16. Trusted source callbacks must be bounded, nonblocking
in-memory lookups; this code cannot preempt arbitrary producer code or enforce a
wall-clock deadline on it.

Both existing dispatch paths recheck the actual first Ready/input/proof snapshot
and the complete owned pair under the first start lock. They install an
attempt-bound successor cleanup gate atomically with start. The gate also applies
when a captured opt-in cohort's forecast falls back. It blocks input advancement
and the authoritative task-start method until an actual cleanup application
matches task, attempt and bound lease. `AlreadyApplied`, no release intent,
nonterminal events and old retry acknowledgements cannot open it. In-memory
state replacement preserves it; durable crash recovery is outside this slice.

The existing native singleton batch envelope reports `DeferredToScheduler` and
uses `RetryDeferred` cleanup. That path awaits real release/reconciliation and
can produce the same typed cleanup acknowledgement. This introduces no batching
capability. Actual predecessor outputs must then satisfy ordinary dependency
input guards, and the successor undergoes fresh admission and placement. Failed
cleanup remains blocked even if the underlying lease release happened.

## Qualification before current-main integration

All builds used Rust 1.92, offline locked dependencies, `ORT_SKIP_DOWNLOAD=1`
and the cached `backend-llamacpp` native feature. No model/runtime downloads or
ONNX downloads occurred.

- Scheduler: 162 passed, one existing ignored cost probe.
- Workflow library: 932 passed.
- Embedded runtime library: 527 passed, two ignored cost probes.
- All-target Clippy for those three crates: passed with `-D warnings -A dead_code`
  (the dead-code allowance covers existing inference backend code).
- Formatting and `git diff --check`: passed.

New pure fixtures cover first-choice reversal, all-infeasible/qualified capacity
deficits, ties, permutation, hard successor constraints, incomplete/duplicate/
foreign/stale/unqualified evidence, overflow and raw-size refusal. Across 256
small matrices (one through four offers), the advisory successor evaluator agrees
with the previously committed Ready-prefix evaluator. This is local evaluator
equivalence, not differential execution against the inaccessible C3 archive.

The public-session fixtures exercise the actual native provider, selected text
host mapping and reservation lifecycle with controlled CPU backends. They prove
the forecast changes the first placement, real predecessor output becomes the
successor's typed input, and the successor gets fresh resource admission. Paused,
failed and cancelled paths do not dispatch the successor early or leak leases.
An exact pair mutation passes the original first-task fence but fails the new
pair fence; provisional custody rolls back. Additional cases cover missing,
unknown, stale, incoherent residency/accounting and default source opt-out.

Independent review ran 7 pure dependency tests, 5 snapshot/gate tests, the
callback-outside-lock regression and 3 public-session tests from compiled
binaries. It found no remaining ownership/execution-safety blocker and approved
only this bounded opt-in scope.

## Dispatch cost

A debug-build probe ran 100 real controlled public sessions without concurrent
compilation on an Intel Xeon Platinum 8573C. It times the actual native provider:
offer gathering, current resource evaluation, 2x2 conditional evidence checks,
ranking and provisional reservation publication. It excludes workflow snapshot
capture/start fencing, queue wait and backend execution. Source forecasts and
capacity are explicitly synthetic fixture data.

| Timed provider call | Median | P95 | Maximum |
| --- | ---: | ---: | ---: |
| Owned 2x2 first decision | 860.906 microseconds | 2,106.884 microseconds | 4,042.965 microseconds |
| Fresh actual successor decision | 376.581 microseconds | 715.673 microseconds | 1,033.275 microseconds |

The final run on the current-main integration tree used 3.798727 CPU seconds,
3.803283 wall seconds and 52,956 KiB maximum RSS for the whole test process.
These are controlled dispatch measurements, not a
production latency guarantee, maximum-field measurement or model-speed claim.
The existing separate pure four-offer cost probe remains available. Real profile
qualification and representative production/maximum-field cost acceptance are
required before any default change.

## Current-main preservation and final qualification

Implementation commit: `09a3dd46a1c781170b830b30bd51c9b9355fd4c5`.
Current-main incorporation is local feature-branch commit
`7d878b4481b694a57911b4f5d4305b6790f06bfc`, code tree
`046766098864594da028a1ba666415d3745ea743`. Remote main was rechecked after
qualification and remains `beb6c2630f1b6d9308b6a6e12fcdabbe9d696f46`.

The two merge resolutions preserve all test includes and supply the new rerank
task-context argument to the bounded scalar input helper. Relative to current
main, the shared input mapper differs only in `pub(crate)` visibility. Rerank
functional inference/backend/gateway, native host/descriptor code, contract
changes, CPU rerank fixtures and tests match current main. Inspector and
cold-Candle work were not incorporated. Independent review approved both merge
resolutions and checked these preservation properties.

After incorporation, the affected full libraries pass: 933 workflow tests,
528 embedded-runtime tests (two cost probes remain ignored by default),
46 runtime-host contract tests and the zero-test interface-contract library.
All-target Clippy for scheduler/workflow/embedded-runtime, formatting and
`git diff --check` pass. Scheduler's earlier 162 passed/one ignored remains
applicable because current main did not change scheduler sources.

The additional `backend-candle` offline test attempt stops before compilation:
`failed to download axum v0.7.9` / `attempting to make an HTTP request, but
--offline was specified`. Thus Candle-gated CPU rerank/embedding regression
execution was not requalified here. Its source and feature guards are preserved;
this limitation is distinct from the passing llama.cpp-feature qualification.

Reproduction uses `source /workspace/pantograph-tools/activate.sh`,
`ORT_SKIP_DOWNLOAD=1`, and
`XDG_CONFIG_HOME=/workspace/pantograph-cache/test-config`:

```bash
cargo test -p pantograph-scheduler --locked --offline
cargo test -p pantograph-workflow-service -p pantograph-embedded-runtime \
  -p pantograph-runtime-host-contracts -p pantograph-inference-interface-contracts \
  --lib --no-default-features --features backend-llamacpp --locked --offline
cargo clippy -p pantograph-scheduler -p pantograph-workflow-service \
  -p pantograph-embedded-runtime --all-targets --no-default-features \
  --features backend-llamacpp --locked --offline -- -D warnings -A dead_code
cargo fmt --all --check
git diff --check
```

Logs are local executor evidence under `/workspace/pantograph-cache/`:
`owned-scheduler-full.log`, `owned-current-main-full.log`,
`owned-current-main-clippy.log`, `owned-current-main-cost.log`,
`owned-current-main-candle.log`. The cost probe is
`dependency_native_dispatch_cost_probe`; run its compiled test binary with
`--ignored --nocapture --test-threads=1` after compilation has stopped.

## Remaining prerequisites

The [migration map](scheduler-v2-production-migration.md) still identifies the
missing app-wide priority accounting, phase/device capacity discovery, global
queues, true batching, durable residency transitions, comparable calibrated
timing and remote-node ownership. None is supplied by this caller. Future-output
classes that cannot be qualified must return `None`; equal model identity never
authorizes reuse or timing estimates.

The Library materialization attempt for `pantograph-scheduler-poc-v2.zip`
(`libfile_cfe8b8ef4fbc81918d2155af3100c90f`) failed with
`library file transfer failed: download failed`. No readable local archive bytes
or independently verified SHA256 exist in this executor. The parent-supplied C3
hash/audit are not recast as local verification. No unbounded Python search was
embedded, and no simulator differential test is claimed. This missing input does
not block the independently qualified local ownership slice.

Remote main was reverified as `beb6c2630f1b6d9308b6a6e12fcdabbe9d696f46`
after CPU rerank PR63 merged. Its runtime dispatch changes must remain preserved;
cold-Candle PR65 and inspector PR64 remain separate. No public writes, pushes or
merge into main are authorized by this work.
