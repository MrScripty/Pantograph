# M3: Bounded Authenticated CPU Service Opportunities

Status: **Accepted**. Source and deterministic evidence independently reviewed;
fixed native cohort completed once; published-head CI and independent final acceptance now pass. M1/M2 remain accepted.
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
following the final-Start correction. Workflow all-target and CPU-example scoped `--no-deps` warning-deny Clippy
pass. Scheduler205 tests pass with one existing ignored cost probe; declared
pre-push tests pass261+168 with one existing node-engine ignore. Workspace
formatting, whitespace, critical/accessibility lint and explicit staged
traceability pass. Three existing actual native lifecycle regressions pass. Normal exact-head PR CI passes, including the actual final summary. Existing
Candle-only inference optional-consumer warnings (CPU-17) remain outside-slice
debt; no suppression or PR75-owned repair is claimed.

## Fixed actual CPU evidence

The fixed cohort executed exactly once from clean source commit
`e9d89eacf686b163539c08ebc6225a6c2750bc96`, tree
`f7ec94bd6f550975b58f30b96603bf0b80401ec7`. Release binary SHA256:
`74ad1349267b2251299e10c7c6b650575669893402b0fd00009db380766cb756`.
The original binary remains frozen outside Git at
`/workspace/m3-authenticated-service-v1/frozen-cpu_embedding_workflow_probe`.
[Source manifest](m3-service-v1/source-manifest.json),
[original invocation status](m3-service-v1/invocation-status.json),
[corrected analysis](m3-service-v1/reanalysis.json), and
[archive hashes](m3-service-v1/SHA256SUMS) retain exact provenance.

Five public sessions used two real authenticated `(client,bucket)` units:
four foreground pairs at priority100 and one background pair at priority-100.
The first foreground Running call paused before native forward until four actual
Ready waiters registered. The two background grants were positions3 and7
(zero-based), both protected at bypass2 with competing foreground priority100.
Ten distinct grants, actual Starts, completed native results, public vectors and
applied matching cleanup receipts agree. The independently recomputed Transformers
reference passed; maximum absolute native/reference error was
`8.940696716308594e-08`, below the declared `1e-5` bound. The environment had a
4-CPU-equivalent/16GiB limit and no accessible GPU interface. This is CPU protocol
and numerical evidence, with no speedup threshold or performance conclusion.

The eighth foreground call (tenth overall) paused after actual native drain while
holding its genuine result and linear receipt. Public Running cancellation
succeeded; the exact lease remained charged at request and until matched cleanup.
All ten calls retained536895107 charged bytes in each RAM/VRAM resource kind at the
pre-native and post-drain barriers. Completed output won as allowed. No interrupted
compute or public Ready cancellation is claimed. Shutdown has no cleanup errors,
no reservations and no native serial owner; conservative stopping/resident registry
metadata remains the already disclosed CPU-14 limitation.

### Original failed oracle and read-only correction

The original native process exited1 after all work and teardown because its final
checker incorrectly treated local `task_id` as globally unique across runs.
`infer_a`/`infer_b` intentionally repeat in five sessions. It rejected prior
successful `infer_b` observations when the last `infer_b` was cancelled. Preserve
that failed oracle and process status: they have not been rewritten as success.

Independent source and design reviewers verified the original raw data and
admitted a data-only correction, with no native/model retry. The corrected checker
first joins unique full native identities in serial result/return order, then
compares ring sequence/multiplicity with the nine uncancelled returns. Four pure
regressions cover repeated local names, missing/extra samples, foreign/duplicate
full identities and actual cancellation observation. Scoped warning-deny Clippy
and the read-only debug build pass. The separately created `reanalysis.json`
passes all original grant/Start/result/receipt/numerical gates plus this correction;
it embeds the original failed oracle and explicitly says native reexecution=false.

There are nine fresh positive observations (five `infer_a`, four `infer_b`) with
two workload digests and no possible ring eviction at five-per-key below eight.
Exclusion of the final cancelled attempt is **source-assisted aggregate evidence**:
frozen source checks the cancellation handle and Running state before appending,
and native lifecycle regressions cover cancellation training exclusion. The sample
API has no run/attempt identity. Neither the checker nor this report claims
independent per-attempt sample-duration attribution. This narrower evidence was
independently approved; no production scheduler correction followed the cohort.

The immutable raw JSON, wrapper, original status/stdout/stderr, reference and
manifests are in [raw-evidence.tar.gz](m3-service-v1/raw-evidence.tar.gz), with
member hashes in [raw-file-sha256.json](m3-service-v1/raw-file-sha256.json).
[Supporting gates](m3-service-v1/supporting-gates.json) and their logs include the
unsupported Candle-only whole-library test build failure (missing optional default
APIs), followed by the accepted default-feature native-test configuration passing
all three lifecycle tests. No hidden successful rerun of the fixed cohort occurred.

M3-C1/C2/C3/C4 are satisfied within these opportunity, ownership and evidence
limits. Independent `m3_source_review` verified all41 raw archive members against the
retained originals, all bundle/log hashes, the unchanged frozen production source
and report scope. Independent `m3_design_review` approved corrected producer
SHA256 `8fa52e568df966dd783d0d3f6930a17789765667b371e0e04e8276525e626820`.
M3-C5 is accepted after exact published-head required CI and independent final review; see the2026-10-20 acceptance report. Parent retains merge/integration authority.

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
