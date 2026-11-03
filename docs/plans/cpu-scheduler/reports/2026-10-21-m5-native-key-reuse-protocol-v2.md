# Fixed native CPU key-reuse comparison v2: prospective contract

Authority: continue docs/plans/cpu-scheduler/plan.md and finite checklist M5-CPU.
The user separately authorized correcting the reviewed candidate-identity oracle,
independent review, a prospective v2 freeze and one bounded new execution.
[The original v1 cohort](2026-10-21-m5-native-key-reuse-result.md) remains failed;
its source fb7ba0e3, protocol, archive, marker and outcomes are immutable. V2 uses
newly constructed baseline and enabled arms; no v1 baseline is reused. This
prospective document is not an execution receipt.

One baseline arm then one enabled arm. Each constructs a fresh protected
EmbeddedSerialCpuResidency using the existing baseline or explicit key-reuse
constructor, authors one two-branch graph, opens one authenticated attributed
session and executes exactly one run. Both arms use the committed untrained
BERT-8 fixture and golden texts[0]="hello world", texts[1]="hello", paired with
single_vectors[0] and [1]. Exactly two forwards per arm/four overall; minimum
three history samples prevents trained ranking. No seeded timing/Ready/proofs,
downloaded model, external backend or fake native delegate is admitted.

Each arm must observe a genuine two-Ready proposal. Enabled capture must contain
two compatible members. Mechanism oracle fixed before execution: baseline six
key encodings/zero hits, enabled four encodings/four hits, both healthy retained
entries zero after completed scope. These are the existing source-derived
n=2 controlled-worker formulas, not elapsed-time or total CPU improvement claims.
Poison/absent mode is unavailable and fails. A changed snapshot that defeats this
specific target is a retained failure, not permission to reinterpret the target.

Both returned and retained vectors must match their corresponding frozen golden
within fixed absolute tolerance <=1e-5 and identical dimensions. Corresponding
baseline/enabled returned vectors must also be exactly equal, as required by
the unchanged comparison oracle. Require exactly
two distinct member grants/Starts/drains/registry receipt ACKs with bijective
run/task/attempt/lease joins and observed native previous/current owner identity,
settings, one-thread and generation continuity checks; no injected capability.
The reviewed correction distinguishes provider identity from member custody:
production runtime_candidate_draft constructs candidate_id as runtime.<runtime_id>.
Each native drain must therefore have candidate_id exactly runtime.candle, and
that field must match its own registry ACK within the full unchanged identity
tuple. Ledger selected runtime candle / variant candle.cpu / device cpu checks
remain required. Candidate IDs may legitimately be shared across members; no
cross-member uniqueness is required for that provider field. Task, attempt,
execution-request, reservation-lease and grant-ticket identities must still be
distinct. Negative oracles must reject coordinated foreign candidates, mismatched
ACK candidates and each repeated custody field for the intended invariant.
Registry ACK is not relabeled
scheduler ACK. Scheduler matching remains explicitly source-assisted through the
existing adapter and successful public completion. Full raw private readiness
proof/owner lineage is not independently observed. Independent source and focused
negative tests retain the final owner/Start/cleanup fence proof.

Enabled diagnostics must contain available positive-call observations for all
five existing grouping phases and the actual captured pair. Nested spans overlap
and must not be summed; logging/observer overhead remains uncalibrated. Baseline
grouping instrumentation is disabled and reported absent, never measured zero.
No duration threshold or winner is chosen. Counts and correctness decide only
the disclosed two-member synthetic mechanism/ownership envelope.

Resource/termination bounds: one configured CPU thread, existing explicit
operator declarations (2 GiB RAM budget, 1 GiB VRAM budget, 32 MiB
resident and 600 MiB load envelope), eight graph nodes/six edges per arm, two short
fixed texts, four native forwards total. Declarations are not allocation
measurements. CPU dispatch may still carry conservative nonzero VRAM
reservation claims under existing policy; neither those claims nor declarations
are physical RAM/GPU allocation measurements. Record actual cgroup/affinity and fixture/binary sizes. Logger
capacity2048rows/4MiB; raw structured rows persist incrementally with sync so a
watchdog retains the observed prefix. Parse/overflow/I/O failure vetoes success. Each arm run plus
readiness resume waits at most60seconds, public shutdown at most30seconds. Session/graph closes share a five-second
absolute budget before the independent shutdown attempt.
A180second outer own-process execution watchdog is a hard global ceiling, not a guarantee
that both arms plus setup/teardown fit. At the deadline the supervisor kills only its own new process group and
attempts reaping for at most five more seconds. Watchdog failure/unknown shutdown
cannot be labeled cleanup success. Attempt public shutdown after every post-construction
body/oracle/I/O error; never start the next arm before acknowledged first teardown.

All actual actions use public facade graph/session/run/resume/readback/artifact/
close/shutdown methods. Private access is read-only scalar test-support evidence.
Pumas metadata uses one fresh process-local registry override, shared across arms;
each owns distinct local project/library/appdata and native RuntimeRegistry. The child has an8GiB virtual-address-space ceiling and the current observed
cgroup ceilings remain in force; this execution limit is not a footprint
measurement. No HOME, authentication, permission, sandbox or admission setting
changes.

Before execution: PR90 actual-main post-merge checks/workflows successful;
independent source/oracle/protocol review; ordinary model-free
oracles including shared candidate identity, foreign/mismatched candidates,
repeated custody identities, missing hits/phases/outputs/cleanup and poisoned
counter negatives; exact source commit/tree and clean checkout; compile without
running the held test; copy/hash the exact producer binary; hash every fixture
and protocol; record toolchain, settings, environment allowlist, limits and input
oracle into a create-new manifest/evidence root. Review actual binary discovery
with --list (no native execution). No code/build changes or concurrent local
compilation during the held run. Execute exactly the discovered held test
serial_cpu_residency::tests::key_reuse_qualification::held::fixed_two_ready_native_key_reuse_comparison_v2.
Use protocol ID m5-native-two-ready-key-reuse-v2 and the create-new evidence root
/workspace/scratch/pantograph-recovery/native-key-reuse-cohort-v2. Require the
static metadata override at that root/metadata.db. V1 paths and marker must never
be reused, overwritten or removed. The frozen
supervisor uses an exclusive execution-started marker, no automatic retry,
immutable hashes and the same cgroup/affinity at execution. Launch only the exact ignored v2 held test once, after independent review of
the full source/protocol/supervisor change and exact frozen manifest/binary.
Publish the correction through the existing isolated PR91 branch with normal
commits; keep it draft while qualification is incomplete. Hosted CI runs on
separate runners may overlap; no local compilation may overlap the held run.

Retain progress, raw rows, body error, teardown outcome and oracle outcome even
when failure occurs. Stop on the single terminal outcome, missing/unknown facts,
overflow, infrastructure failure or uncertain teardown. No rerun, post-outcome
input tuning, population growth, continued failed owners or retrospective
acceptance change. A genuine evidenced implementation defect may be repaired
separately only after re-planning; original cohort remains failed.

After execution independently review raw evidence against these frozen criteria.
Do not close overall M5/M6, performance, complete memory, native batching,
representative model quality or hardware claims from this narrow comparison.
