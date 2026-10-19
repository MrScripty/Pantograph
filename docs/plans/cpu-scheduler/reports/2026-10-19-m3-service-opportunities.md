# M3: Bounded Authenticated CPU Service Opportunities

Status: **Verifying**. Source and deterministic evidence independently reviewed;
fixed native cohort and published-head CI remain pending. M1/M2 remain accepted.
No default activation, speedup, GUI/GPU, native batching or model-quality claim.

## Owned change

Base main `4aec0d5c4927a6af2832d40e224be5c2136fc9ef`. The [canonical
contract](../plan.md#m3-bounded-service-contract-2026-10-19-independently-admitted)
was independently admitted before production changes. The branch
`feat/cpu-scheduler-m3-bounded-service` is isolated in `/workspace/pantograph-m3`;
original `work`, PR79 README and PR75 text adapter/model work are untouched.
Parent owns integration; no merge, history rewrite or branch deletion.

The opt-in serial config protects already Ready authenticated `(client,bucket)`
units after two other-unit grants. At most64 pending members across32 units;
oldest protected service order wins, otherwise active-run priority/FIFO. Every
service grant refreshes unit service order independently of member FIFO. That
refresh prevents the independently identified three-old-unit rotation from
starving a newer fourth. Unit opportunity is bounded by33 other-unit grants for
continuously pending eligible work; arbitrary priorities within one unit and
unbounded nonpreemptible durations have no new starvation/wall-time guarantee.

Session attribution is resolved by existing credential/session/bucket ownership
validation; no caller supplies fairness identity. Missing attribution refuses
before preparation or new runtime claims. Existing session priority, FIFO,
starvation boosts and warm window remain at their original admission boundary.
M1 still ranks only its current same-run pair, and M2 remains advisory.

Ready capture and bounded input/proof validation precede waiter registration.
The original identity/priority/first Ready member is revalidated after waiting
and under the final Ready-to-Running lock after awaited candidate preparation.
This final check closes the P1 gap found during independent source review while
preserving M1 selection of the second same-run member. New turns cannot release
physical resources: native drain/linear cleanup and poison fencing remain the
existing authority. Pending future drop removes scheduling state only; supervised
work survives caller loss. Public active cancellation still requires Running.

## Deterministic and static evidence

Workflow library:1008 passed,0 failed,0 ignored. Added13 exact trace policy tests,
2 async registration tests and5 fair-worker tests. They cover priority extrema,
FIFO, same-unit debt, always-backlogged A/B/C/D rotation, full32-unit trace,
capacity/counter exhaustion atomicity, drop/wakeup ordering, missing identity,
changed inputs during waiting, no-pair population change during candidate await,
and abort/poison refusal without another forward. Controlled test ports remain
protocol evidence, not real CPU execution.

Independent GPT-6.1 Sol / High `m3_source_review` approved source and these tests
following the final-Start correction. Exact affected lint, native lifecycle,
formatting/hooks and normal CI results will be recorded before closure.

## Fixed actual CPU evidence

Pending: five sessions from two authenticated units;4 foreground pairs at100,
1 background pair at-100. First foreground Running call pauses before native
forward until4 actual Ready waiters register. The third opportunity after
registration must protect B despite a high-priority A waiter. Minimum8 samples
keeps M1 on untrained FIFO for all five samples per workload, fixing the grant
representative/Start correspondence without changing policy.

The eighth foreground call pauses after actual native drain with its genuine
result/linear receipt held. A public Running cancellation request must retain
its exact charged lease until matched cleanup. Results/receipts return unchanged;
completed output may win and no interrupted compute is claimed. There is one
fixed ten-call cohort, no winner retries, timing tuning or speedup threshold.

Independent producer pre-run review admitted SHA256
`6d4e4d4a2cb512c44a661c56b032217c40f67e39a9701fb1f74cee0ce603fdc6` after checking
actual attribution grouping, charged cancellation lease and
native-drain/request/return/cleanup ordering. Build/source/binary freeze and
independent numerical reference precede the one admitted model invocation.

## M4 dependency boundary

Read-only identification after M3 source review: retain for the next same-model
Ready task; otherwise acknowledged serial stop, then cold-load the next selected
model. Native receipt-backed last-lease eviction exists, but the serial cleanup
port currently always retains and requires an existing resident owner/successor.
That is a real integration gap. Keepalive/live successors and pins cannot be
removed merely to make an eviction eligible.

M4 requires accepted M3 gates and a separately reviewed exact contract. Qualify
BERT8/BERT12 A,A,B,B,A with separate actual load/forward/stop/reload observations,
declared pressure, cancellation, failed-stop fencing and charges retained until
exact stop acknowledgement. M2 warm dispatch-through-cleanup samples do not
supply cold/load/stop costs. Existing generic model replacement loads the candidate
before retirement and therefore cannot implement the no-overlapping-load subset.
No M4 code or native/continuous batching is admitted by this report.
