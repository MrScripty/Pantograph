# Plan: CPU Scheduler Completion And Fair Progress

**Plan status:** `Active` — M1 implementation and qualification; independent design approved.

**Current phase:** M1 — retained CPU composition and real two-Ready qualification.

**Next slice:** Finish publication gates and hand off the independently reviewed
single C3 supplement. The original failed cohort and slower baseline remain
unchanged. No further experiment or M2 work is admitted.

**Acceptance status:** M1 evidence reviewed, final publication gates pending:
actual hosted CPU output, independent ranking and measured comparison pass;
second-position coverage passes in the separately admitted `c3-768b-v1` cohort.
The original release result stays negative. Ordinary CI passed at `938bc89e`;
changed heads require their applicable gates. No full research scheduler/default change.

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

## Objective And Authority

Make CPU-supported scheduler improvements usable through Pantograph's canonical
public execution sessions: select from measured comparable work, make bounded
progress across ready work and apps, and manage model residency without releasing
resources before the owner proves cleanup. Keep policy selection explicit until
the affected execution, safety and performance claims are qualified.

The user authorized this phased CPU work on 2026-10-12, after a standards-compliant
plan and independent design review. This plan independently owns CPU scheduler
policy sequencing and acceptance. The [domain/multimodal plan](../domain-architecture-and-multimodal/plan.md)
continues to own graph/inference feature completion and its paused work. The
[research paper](../../research/scheduler/resource-safe-learning-scheduler.md)
owns research rationale; [migration notes](../../scheduler-v2-production-migration.md)
and dated slice reports are source/evidence, not a second execution authority.

Implementation admission is this exact path, operation `start` after M0 review;
subsequent work uses `continue` while Active. User authorization covers that
transition without another permission request. Main merges, deployments,
security-setting changes and release remain unauthorized.

## Standards Route And Design Decision

Canonical Coding-Standards revision: [`188beda1fa477d21d576c233dd8c7f4c4c267d23`](https://github.com/MrScripty/Coding-Standards/tree/188beda1fa477d21d576c233dd8c7f4c4c267d23).
This is an explicit new baseline for this effort; the older broad plan's
`366c1d90` baseline is not silently replaced. No repository AGENTS.md or local
SKILL.md applies; `/workspace/.agents` and `/workspace/.codex` are empty.

Core/Router select Planning/discovery, Implementation, Verification/oracles,
Development Proportionality, Documentation and Commit; Architecture, Contracts,
Concurrency, Resilience, Performance, Diagnostics and Code Design; Library and
Rust base/API/async profiles. Their required closure is reviewed at M0.
Re-route when a milestone changes persisted policy/profile state, IPC/bindings,
UI, dependencies or build/CI mechanisms; those boundaries are not implied by an
in-process opt-in. No release, GPU, remote or graphical acceptance is inferred.

The standards cause concrete choices: retain one policy owner and the existing
linear native cleanup capability; preserve default construction; distinguish
measured service cost from unmeasured cold/transfer phases; use actual public
session output as acceptance; keep one serial integration owner and bounded
ordinary tests. Do not introduce a plugin framework, new scheduler daemon,
generic verifier, duplicate profile database or Python simulator dependency.

Alternatives considered: (a) merely add another shadow wrapper — rejected for M1
because actual retained CPU execution already exists and the composition gap is
concrete; (b) replace all scheduling with the research search — deferred because
native phase evidence, safe action coverage and fairness obligations are not
complete; (c) wire the existing narrow serial owner and qualify it — selected.
The existing pure Rust kernels remain candidates for later bounded extensions;
a generic queue library cannot supply Pantograph's owner/readiness/custody proof.

## Source-Backed Starting State

Admitted main: `1fba975ab20cb032949aa7221e373f0719e3264b`, tree
`6d3c8f96039af1616c3ea3ac97c177b5fdbc922d`. Its four automatic workflows completed
successfully before this effort. This is a baseline, not new-feature evidence.

| Concern | Current source and consequence |
| --- | --- |
| Normal application path | `src/components/WorkflowToolbar.svelte`, `src-tauri/src/workflow/headless_workflow_commands.rs` and ADR-011 submit execution sessions. This is production infrastructure, not a Python research executor. |
| Default policy | `workflow/service_config.rs` constructs `serial_ready_mode: None`; `scheduler/policy.rs` uses PriorityThenFifo, starvation boosts and a one-position warm window. Preserve it. Paths in this table are under `crates/pantograph-workflow-service/src/` unless qualified. |
| Native composition gap | `crates/pantograph-embedded-runtime/src/workflow_service_composition.rs::configure_workflow_service` replaces batch/execution/lifecycle ports; `workflow/service_config.rs` deliberately rejects replacement of constructor-owned serial batch/cleanup ports. |
| Restricted CPU implementation | `workflow/serial_ready.rs`, `workflow/task_execution_worker.rs` and `crates/pantograph-embedded-runtime/src/serial_cpu_port.rs` already implement singleton admission, bounded first-two comparison, actual native warm/drain receipts and retained cleanup. Native public-session tests inject readiness/candidates and do not yet prove an actual two-Ready ranking path with ordinary hosted providers. |
| Timing | The serial ring records actual dispatch through drain and matched cleanup, keyed to exact inputs/owner/profile/settings. It is capped at 64 keys, eight samples/key; default minimum three samples and maximum age 60 seconds. It cannot satisfy generic preparation+transfer+execution forecasts. |
| Lookahead | `pantograph-scheduler/src/{completion_ranking,dependency_completion,two_completion,cohort_completion}.rs` contain bounded Rust algorithms. Generic native timing is explicit opt-in; four-task cohort coverage is read-only and does not install the kernel. |
| Resources/residency | `pantograph-runtime-registry` owns RAM/VRAM/shared-domain claims and resident charges. Native last-lease eviction exists separately; the serial port still requires a resident owner and a retaining successor at admission. No release on predicted finish or cancellation request. |
| Parallelism/batching | TaskWorker supervises concurrent commands, while run progression selects the first Ready runtime task. Text/chat/embedding/rerank/audio batch envelopes execute members sequentially in `runtime_host_execution_port.rs`. Neither general parallel DAG optimization nor native/continuous batching is established. |
| App fairness/remote | Client/session/bucket attribution exists, but app-wide service debt/protected opportunities do not. `workflow/local_network_api.rs` returns LocalOnly and no peers. |

Historical reports contain superseded pin/publication blockers. Current code and
the public pinned Pumas contract are authoritative; do not revive obsolete patches
or treat all old blockers as current. Do not change Pumas or the private catalog.

## Environment And Explicit Exclusions

Observed 2026-10-12: Linux x86_64; AMD EPYC 9V74; five exposed logical CPUs,
`cpu.max=400000 100000` (four CPU-equivalents), `memory.max=17179869184` (16 GiB),
no swap. `/dev/nvidia0`, `/dev/nvidiactl`, `/dev/dri`, `/dev/kfd`,
`/proc/driver/nvidia` and `/sys/class/drm` are absent; `nvidia-smi`, `rocm-smi` and
`lspci` are unavailable. This establishes no accessible GPU path here, not that
the physical host cannot contain a GPU. Recheck environment facts before execution.

Use committed untrained BERT-8/BERT-12 CPU fixtures and existing toolchains. These
prove numerical execution/ownership, not semantic model quality. No model
download or paid inference is required. Preserve license qualifications in the
[CPU guide](../../../examples/cpu-embedding-demo/README.md).

Explicitly Deferred: GPU/CPU split execution and GPU qualification (requires
verified suitable hardware and a separately reviewed plan); remote workers
(requires an available test machine and new authorization); native/continuous
batching without a real capable CPU backend; general kernel preemption, load
overlap, distributed recovery, pretrained quality and default-policy activation.
No provisioning or attempts to bypass environment/security limitations.

## Binding Interfaces And Invariants

| Owner | Contract for this rollout |
| --- | --- |
| Workflow service | Own graph readiness, immutable input/proof snapshots, run/attempt identity, ready-frontier capture, serial decision observation and task/terminal publication. Captures are bounded before cloning and revalidated before Start. |
| Scheduler | Own finite ranking, fairness and action proposals. A proposal cannot mint a Ready proof, executable lease, future output, cleanup acknowledgement or successful terminal state. |
| Runtime registry | Own physical/resident charges, reservation commit, retention, exclusive execution custody and reclaim eligibility. Revalidate current facts at commitment; unknown capacity is not free capacity. |
| Native host/inference | Own actual load, forward, cancellation/drain, private instance/generation and linear cleanup receipt. Cancellation or caller loss cannot release an executing allocation. |
| Pumas | Own immutable model/package/artifact/load-target facts. Read only its current public pinned contracts; no guessed paths or unpublished local changes. |
| App/transport | Select an explicit supported host mode and display backend diagnostics; never supply measured timing, inferred readiness or scheduling authority. |

M1 reuses `EmbeddedRetainedCpuSerialPort::new`,
`WorkflowService::new_serial_ready_cpu` and
`EmbeddedHostedStartupCompositionInput::with_workflow_service`. Add a read-only
service capability query so hosted composition preserves **all** constructor-owned
serial execution/cleanup ports while installing the ordinary source refresher,
candidate, facts and readiness dependencies. Ordinary construction still installs
ordinary ports. Keep replacement assertions; do not retrofit a mode into an
already-exposed service. Explicitly reject serial mode combined with generic
`EmbeddedCompletionTimingOptIn` for this milestone, before partially composing it.

Use the same actual gateway, registry and authoritative selector in the native
port and hosted composition. This is a caller composition precondition: the
read-only mode query does not bind these dependencies or detect every possible
foreign constructor combination. Test the foreign owner/registry cases rejected
by existing native custody checks; do not promise broader automatic detection.
Observed custody mismatches refuse before forward, never silently switch backends.
Bootstrap through an ordinary public CPU session with `keep_alive=true` and the
calibrated gateway; keep a genuine separate retaining session lease alive while
the serial service runs. Its configured resident RAM envelope is an explicit
operator/test declaration, not a physical allocator measurement. Cold bootstrap
and retention are real prerequisites, not synthetic proof fixtures.

Replace the serial selector's opaque Option internally with one bounded typed
decision outcome: FIFO with reason, or ranked first/second with the comparable
sample counts, ages and observed maxima. Retain the existing maximum 128 graph/
record entries, two compared Ready members, 64 keys by eight samples, and current
preflighted text/proof limits. The diagnostic contains at most two bounded task
identities, scalar counts/costs/ages and an enum reason; never raw text, serialized
proof, an owner Arc or unbounded decision history. Keep it with the existing serial policy and
project through existing diagnostics; no second history store, caller-authored
timing input or new public IPC endpoint. Distinguish ranking proposal, actual Start
and matched cleanup events. The exact diagnostic projection
must pass M1 review before expanding any public contract. Inspect its actual event
representation: changing a persisted diagnostic schema or meaning routes
persistence/evolution even when no new IPC endpoint is added. Prefer an existing
bounded diagnostic slot where its current contract fits; do not silently change
durable consumers under the label of an internal observation.

M1 projection review (2026-10-12): the private typed proposal uses the existing
`log` facade under `workflow::serial_ready`; authoritative Start/cleanup remain
the existing ledger events. The workflow crate adds a direct dependency on the
already locked workspace log facade, without a new dependency version. Existing
ledger payloads cannot hold the full proposal without changing semantics or
schema. The isolated qualification example collects a bounded in-memory log and
writes JSONL after execution; this is invocation evidence, not production state.
It correlates workflow/run/task IDs, refuses overflow/missing evidence, and emits
outside owner/store/ring locks. Independent standards review accepted this small
projection correction; no durable schema or public IPC is changed.

Actual hosted probing also exposed a readiness-resume gap: initial submission
uses the supervised TaskWorker, but EmbeddedRuntime's resume path passes no runtime
owner. Add a facade resume method forwarding the existing private owner-aware
resume implementation, and select it only for serial mode in EmbeddedRuntime.
The ordinary resume branch is preserved. Independent review admitted this narrow
addition before edits; it reuses the same active run, inputs and proofs, without
new request/schema/storage or a readiness bypass. The additional write set is
`task_execution_facade.rs`, `session_execution_api.rs` and embedded
`embedded_workflow_service_api.rs`, with delegation and actual same-run resume
verification.

Missing/stale/incomparable samples retain explicit FIFO on the same serial port;
ranked owner mismatch refuses before execution. Ties preserve first position.
Poison/cleanup uncertainty stays fenced. Samples are recorded only after actual
successful completion and matched cleanup; cancellation, failure and abandoned
work do not train. No duration is a worst-case or future-completion guarantee.

## Milestones And Runnable Checkpoints

Each feature gets its own branch. Only the next slice is admitted for detailed
implementation; later milestones are ordered goals whose exact shared contracts
and write sets receive an independent review before their first code change.

| Milestone / state | Usable result and bounded write set | Dependencies and acceptance gate |
| --- | --- | --- |
| M0 / Accepted on publication | Reviewed plan, source/hardware facts, ledger/issues; navigation pointers in `docs/plans/README.md`, broad plan and scheduler migration notes. No production code. | Two independent reviewers approved M1's shared interface decisions; links, exact scope and history reviewed. This plan commit is the publication checkpoint. |
| M1 / In progress | Existing retained CPU first-two-Ready mode composes with ordinary hosted providers. Writes: embedded `workflow_service_composition.rs` and its focused tests; workflow `serial_ready.rs`, `task_execution_worker.rs`, relevant tests/exports; new focused native integration test and registration; bounded CPU example/guide only if needed for repeatable invocation; this plan's evidence. | M0; real resident CPU owner and retaining lease. Actual public sessions with two ready embeddings and retained outputs; measured sample-driven order; no injected candidate/readiness/timing/outputs; default and negative lifecycle regressions. Exact-head local gates, independent source/evidence review, normal CI. |
| M2 / Planned | Bounded completion lookahead for a disclosed CPU cohort, initially advisory on actual work. Extend existing scheduler kernels, workflow capture and native evidence producer only. | M1; complete comparable evidence for every admitted alternative/transition, including unfinished obligations. Hand-computed independent oracles, work-budget refusal, missing/stale evidence, owner invalidation and actual CPU trace correspondence. No forecasts authorize execution. Promote only a separately qualified action subset; do not fill missing cold/transfer costs with zero. |
| M3 / Planned | Explicit per-app/bucket service opportunities at an authoritative ready-work boundary, alongside priorities. Writes confined to scheduler/workflow queue/attribution integration and tests after its design review. | M1 and a bounded eligible population; use authenticated existing attribution, not arbitrary caller identities. Fixed modeled traces prove opportunity/order, then two actual CPU sessions prove progress under repeated foreground arrival and cancellation. No wall-clock guarantee for unbounded nonpreemptible jobs. Missing entitlement identity has an explicit owned disposition. |
| M4 / Planned | CPU residency actions improve a specified repeated-model workflow without violating cleanup; serial preparation/eviction only. Writes scheduler policy, registry/host action integration and tests after review. | M2 evidence, M3 fairness obligations where actions delay other apps. Measure load/unload/reload separately; full charge remains until acknowledged stop; pinned/live allocations protected. Actual two-model CPU alternation, pressure, cancellation and failed-stop cases. No overlapping loads, speculative lease release or automatic recovery of unknown execution. |
| M5 / Planned | Fair bounded grouping on the existing supported sequential CPU batch envelope; optimize only demonstrated scheduler overhead/reuse. Writes existing batch policy/broker and tests after review. | M3/M4 where grouping changes retention or fairness. Group 1..8 compatible members without waiting indefinitely, preserve per-member outputs/attempts/cancellation, mixed/incompatible tasks refuse grouping. Compare actual CPU grouped versus singleton runs. Label sequential envelope honestly; native/continuous execution remains Deferred unless independently capable and exercised. |
| M6 / Planned | Qualified modes become an explicit documented host/application choice with truthful diagnostics and bounded rollback. | Accepted upstream milestones; exact consumer contract and UI/IPC/persistence routing reviewed if added. Real consuming-path evidence, regression and performance report, normal CI. Default activation is a separate decision; no main merge/deployment authorized. |

M2–M5 may be reordered only when measured dependencies change; the integrator
updates the one next slice and affected design review, rather than implementing
unsettled shared contracts in parallel. A milestone unable to prove its named
native action remains advisory/Blocked; contract tests never substitute for it.

### M1 Evidence And Oracle Plan

| ID | Observable criterion / deciding oracle | Kind / environment / mode | Status | Evidence |
| --- | --- | --- | --- | --- |
| C1 | Default construction/dispatch remains unchanged; serial port replacement still refuses; conflicting modes reject before composition. Existing default tests plus focused composition regressions. | contract / representative / automated | passed | 25 focused composition tests; default/port protection/early conflicts pass. See [M1 report](reports/2026-10-12-retained-ready-m1.md). |
| C2 | From public graph/session submission, ordinary readiness/candidate providers dispatch real resident Candle CPU forwards for both Ready tasks and retain exact numerical outputs. Committed fixture/independent reference vectors and actual ledger owner events are oracles. | integration / required-real CPU / automated | passed | Final corrected-source release: all 32 vectors from public hosted 8+8 runs match independent reference; ordinary readiness/candidates, same-run resume and native output verified. |
| C3 | After at least three fresh successful samples per exact workload, the recorded first-two decision matches independently calculated maxima and actual start order. Use real monotonic measurements; assert the observed winner, not an assumed faster text. If ties/no usable separation prevent proving second-position selection, record that claim pending; never inject clocks/delays or retry until a desired winner. | integration / required-real CPU / automated | passed by admitted supplement; original cohort negative | Original release raw counts/maxima/order pass but no second-position Start (exit 1), unchanged. One prospectively published `c3-768b-v1` cohort independently verifies all raw samples/outputs and two actual second-position Starts, followed by three correct first-position Starts; exit 0, no retry. See [supplement](reports/2026-10-12-c3-768b-results.md). |
| C4a | Missing/stale/foreign-generation observations use explicit FIFO or fail closed as declared; owner replacement after ranking, cancellation, dropped receiver and failed cleanup retain custody and exclude training. | focused / simulated protocol / automated | passed | Full workflow-service 979 tests pass, including private InvalidClock partial-state reset and direct/supervised timed run/resume no-side-effect refusal. |
| C4b | Actual CPU cancellation preserves custody and prevents cancelled training. Reuse requires matched cleanup; otherwise the correct result is fenced/poisoned and the next attempt refuses. No new recovery is authorized. | integration / required-real CPU / automated | passed | Two actual native post-drain cancellation/next-attempt regressions pass; controlled metadata remains separate from C2; matched cleanup or conservative fencing, no compute-cancellation claim. |
| C5 | Record a representative paired-workload baseline and opt-in comparison: outputs, selected order, sample provenance, queue/service/workflow elapsed times, selector-only scheduling cost and environment (total scheduler CPU remains unisolated). Repeated release observations disclose variability; do not promise general speedup. | integration / required-real CPU / automated | passed (scoped) | Fixed final release 8+8 comparison, all raw values and min/median/max in [M1 report](reports/2026-10-12-retained-ready-m1.md). Selector-only cost disclosed; total scheduler CPU unisolated, no speedup observed. |
| C6 | Independent review and applicable repository gates pass on the published feature head. Normal CI validates broad regressions; it does not establish GUI/GPU or native batching. | contract / representative / automated + review | passed at `938bc89e`; recheck changed head | Independent source/evidence review, local gates and all four ordinary CI workflows passed on draft PR #73. Later changes must receive applicable checks; this does not close C3. |

Predeclare the real run count and use the same unchanged graph throughout: two
independent embedding tasks, exact texts, same model/environment/descriptor, with
both genuine Ready proofs available concurrently. At least three successful fresh
samples per text are required. Obtain raw durations through a bounded test-only
view of the existing ring or existing sample events; independently aggregate them
outside the selector. Re-comparing only the selector's reported maxima is not an
independent oracle. This observation seam cannot inject or modify history or add
a second store. The fixed run set must demonstrate at least one second-position
ranked Start to accept that claim. Otherwise leave it pending and report the
observed result, without repeating runs until a favorable order appears.

M1's current claims stop at Rust public sessions and native CPU output. Existing
browser demos are baseline evidence, not proof of the opt-in scheduler. Any new
GUI claim requires actual interface capture through that mode and a separately
admitted verification route. No unsandboxed browser permission is inferred.

Commands use the repository-pinned toolchain and cached dependencies. In this
executor activate `/workspace/pantograph-tools/activate.sh`; do not infer GPU
availability from compilation. Initial existing checkpoints are runnable now:

```sh
ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
  cargo test --locked --offline -p pantograph-scheduler
ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
  cargo test --locked --offline -p pantograph-workflow-service --lib serial_ready
ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 \
  cargo test --locked --offline -p pantograph-embedded-runtime \
  --no-default-features --features backend-candle,host-dependency-inventory \
  --lib actual_cpu_worker_consumes_linear_warm_receipt_and_preserves_session_successor
cargo fmt --all --check
git diff --check
```

These are scoped starting checkpoints, not M1 acceptance by themselves. M1 adds
the hosted two-Ready test to normal discovery, records its exact command and
nonzero executed test count, runs affected complete crate suites and all-target
Clippy with the intended features, and checks unchanged default features.
Use process-local writable Pumas registry/test roots; no user's catalog.
If cached/native dependencies prevent reaching the new path, report that exact
failure; do not suppress it or claim the filtered suite executed.

## Simplicity And Ownership Review

**Applicability:** `applicable` — hosted composition and policy observation change.

1. **Independent concerns:** workflow readiness (service, per graph/run), policy
   ordering (scheduler/service, at admission), physical execution/cleanup (native
   owner, per attempt), resource accounting (registry, across attempts), model
   facts (Pumas, immutable artifact), and diagnostics (workflow ledger, projection).
   Their what/who/when/where/how differ because graph validity, policy and physical
   allocation change independently. They meet at the existing validated handoff.
2. **Interleavings:** readiness/input/owner generation must be rechecked before
   Start; measured cost requires real drain plus cleanup; release requires the
   same linear receipt. Incidental coupling of generic timing with the serial
   ring or default composition overwriting owned ports is rejected. No new state
   persistence/version envelope in M1; exact owner/input invalidation stays local.
3. **Caller knowledge:** an opted-in native host constructs one port/service with
   the same gateway/registry/selector and supplies ordinary artifact/ledger
   dependencies. Callers know retained-owner preconditions and typed refusal,
   not backend locks, mutable timing history or cleanup protocol internals.
4. **Change paths:** changing serial sample policy touches its owner/tests;
   changing native cleanup touches native receipt/registry contract consumers;
   changing host wiring touches composition and its tests; changing GUI display
   later consumes diagnostics without reimplementing selection. A change forcing
   unrelated graph/binding policy edits reopens the design.
5. **Stable dependencies:** existing execution/cleanup traits and immutable model,
   input and attempt identities carry authority. The capability query is a read-only
   composition fact, not access to the private ring or raw backend. Protect existing
   port replacement and metadata validation; do not pass lock/generation internals
   into UI or normal callers.
6. **Independent evolution/failure:** pure ranking, composition and native cleanup
   have separate focused tests plus the real path test. Bad history affects ranking;
   invalid ownership stops execution; registry state does not follow diagnostic
   success. A disabled opt-in leaves ordinary construction unchanged.
7. **Deletion test:** no new policy manager/store/registry. Removing the composition
   branch recreates the demonstrated protected-port conflict; removing typed decision
   outcome hides the distinction between FIFO fallback and actual ranked dispatch.
   Existing serial owner contains necessary admission/drain complexity. Decline any
   additional permanent mechanism without a named reachable gap and independent value.
8. **Cumulative complexity:** reuse one serial admission owner, one bounded ring,
   one native attempt/receipt and one registry. Finite lookahead, fairness and
   residency policy are later separately accepted mechanisms, not prerequisites
   silently accumulated in M1. No daemon, new execution authority or simulator port.

## Migration, Rollback And Branches

M1 is additive native Rust composition. No persisted mode/profile migration,
session JSON toggle, generated binding or default change. Existing callers retain
the same behavior. A host opts out by constructing the default service for the
next service lifetime; never switch modes under active work. Shutdown must drain
owned tasks. Poison/uncertain execution remains fenced until an existing proven
owner cleanup or separately reviewed recovery; rollback cannot erase claims.

Every later public/persisted change must name old/new consumers, migration and
rollback before implementation. Do not keep duplicate contract authorities.

Plan branch: `plan/cpu-scheduler-2026-10-12`, based on the exact main above;
worktree `/workspace/Pantograph-cpu-scheduler`. Owner/integrator: this delegated
scheduler worker, with parent coordinating future integration. Publish without
rewriting history. Target is a future reviewed main integration, not authorized
here. M1 uses `feat/cpu-scheduler-retained-ready-2026-10-12`; each later feature
gets a separate branch with its admitted base recorded when opened.

The plan worktree is `retained-protected`: it remains for plan/M1 coordination,
with commits reachable from its named published branch. Retire it only after
parent handoff, clean state and verified retained commit reachability; do not
delete or prune unrelated worktrees. `/workspace/Pantograph` and its paused
multimodal branch are forbidden write sets. Existing main remains unchanged.

## Review, Blockers And Re-Plan Triggers

One integrator owns all shared contract edits. Independent reviewers may inspect
in parallel; disjoint implementation is admitted only after shared contracts are
settled, with exact write sets and one integration order. Read-only reviews do not
require a separate concurrent-plan protocol. Review the full feature range before
push and use normal CI for completed source changes; no forced updates or security
suppression. Where CI triggers only on PR/main, report that limitation and use a
review PR only when authorized, never an unauthorized main push to obtain CI.

Current M1 design admission is approved by independent feasibility and standards
reviewers; findings and disposition are in the execution ledger. Publication through the
authorized GitHub connector is available; Git CLI credentials are unavailable.
No evidence is accepted merely because source compiles or a historical report is
green. Full simulator differential acceptance remains unavailable without the
verified archive and is not claimed for the independently qualified CPU subset.

Re-plan for a changed native owner contract; unavoidable private/Pumas dependency;
failure to establish real retained CPU prerequisites; inability to collect comparable
real first-two evidence; lifecycle/custody weakening; unbounded cost/population;
new public/persisted consumer; required cross-owner edits beyond this composition;
or measured regression that invalidates the promised useful outcome. Fix routine
implementation defects inside the admitted boundary without repeating design
ceremonies. GPU/remote remain Deferred until the stated environment and authority.

## Final Acceptance

M0 is accepted. M1 implementation and CPU/ranking/measurement evidence are
independently reviewed. The original release cohort remains negative for
second-position coverage and retains exit 1. The separately admitted single
`c3-768b-v1` supplement closes that C3 case with two real second-position Starts
and independent raw sample/output/order verification; neither cohort shows
speedup. C6 passed at `938bc89e`; final changed-head gates still govern handoff.
The implementation remains draft PR #73. No M2 or broader scheduler completion is
admitted by these results. See the original [M1 report](reports/2026-10-12-retained-ready-m1.md)
and the separate [supplement](reports/2026-10-12-c3-768b-results.md).

### M1 resident-accounting qualification correction (2026-10-12)

The first genuinely Ready native pair refused with `UnknownResidentAccounting`: the
ordinary candidate conservatively claims RAM and VRAM, while the probe declared
only RAM for its retained allocation. Independent review confirmed the existing
registry contract distinguishes omitted/unknown from explicit known zero. The
probe now declares 32 MiB RAM (an operator envelope, not an allocator measurement)
and zero VRAM for the exact calibrated Candle CPU F32 backend, whose tensors use
`Device::Cpu`. It preserves the ordinary task claims and matched cleanup checks.
No candidate, validator, registry algorithm, Pumas contract or default was changed.
The caller must share the same gateway, registry and selector across the retaining
bootstrap, native serial port and hosted composition. The composition does not
claim to infer or prove identity for arbitrary user-supplied port implementations.

### M1 timeout capability correction (2026-10-12)

Independent implementation review found the supervised worker path carries
`timeout_ms` without the direct resume path's deadline enforcement; native serial
execution does not implement that deadline. Before editing, standards review
approved an explicit fail-closed restriction in the already admitted execution
API: serial runs reject `Some(timeout_ms)` before admission/host calls; resume
rejects stored timed runs before recovery-event creation/worker enqueue. Default
behavior is unchanged. Focused no-side-effect run/resume tests qualify refusal.
The probe uses `None` and a 30-second caller watchdog; caller cancellation/drop
is not evidence of native stop or permission to release custody. No new deadline
machinery, recovery, or deadline-performance claim is admitted.

### M1 C3 supplemental coverage amendment (2026-10-12)

The user authorized exactly one bounded supplemental coverage experiment after
the [causal investigation](reports/2026-10-12-c3-causal-investigation.md). The
original release result remains negative: eight baseline plus eight serial runs,
all five ranked decisions correctly selecting position zero, probe exit 1,
baseline median 54.031291 ms and serial median 63.954510 ms. Preserve all 13
original evidence checksums under `reports/m1-evidence/SHA256SUMS`; do not replace
those exports, summaries, manifests or the original report.

**Protocol `c3-768b-v1`, frozen before execution:**

- A, bound to `prompt_a` / `infer_a`: ASCII `"hello world "` repeated exactly 64
  times, including its trailing space; 768 UTF-8 bytes; SHA-256
  `37abb1bb44478b85ca20fe74213169f68d833875a43317f10d76733465ef82b1`.
- B, bound to `prompt_b` / `infer_b`: ASCII `"hello"`, no trailing space;
  five UTF-8 bytes; SHA-256
  `2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824`.
- Preserve graph/task IDs, candidate order, exact model/weights, tokenizer, one
  configured private CPU thread, ordinary hosted providers, retaining owner
  prerequisites, cleanup validation and all capability limits. In particular,
  String embedding text is limited to 1,024 bytes. No TranscriptText substitution.
- Before hosted execution, use the cached independent Transformers reference with
  the unchanged BERT-8 fixture. Verify exact new input token IDs, attention masks
  and type IDs against the committed truncation reference, then execute a CPU
  reference forward and compare all eight output dimensions within `1e-5`.
  Record package/model/input hashes and actual reference vectors. No weights are
  regenerated and no model/package acquisition is permitted for this protocol.
- One fresh isolated Pumas owner, registry, gateway and empty serial history;
  ordinary retaining bootstrap; exactly eight baseline runs followed by eight
  serial runs with unchanged graph/texts. Keep both hosted compositions/readiness
  producers alive throughout both phases. Use the release binary; finish builds
  and reference execution before measurement and avoid concurrent model work.
- Three successful fresh samples per exact workload before ranking. Capture every
  successful raw per-sample dispatch-through-matched-cleanup duration through the
  existing read-only observation view, all proposals, both actual Start IDs,
  outputs, run/resume IDs and shutdown state. Independently recompute counts and
  maxima from raw samples and compare proposed winner and actual order.
- Preserve the existing numerical, readiness, owner, freshness and cleanup
  oracles. Use `timeout_ms: None` and the existing 30-second caller watchdog,
  which confers no native cancellation or resource-release authority.
- Store new inputs, reference, source/binary hashes and full raw baseline/serial
  records in a separate `c3-768b-v1` evidence directory. Report every run and
  measurement regardless of ordering. This supplement neither replaces C5's
  negative/slower comparison nor establishes general speedup or isolated
  preprocessing cost. The tensor input shapes remain 16 and three tokens.

Hypothesis: the supported 3.2-fold raw-input increase for A may create a measured
service-cost contrast that covers second-position selection. This is conditional,
not an assumed faster-text oracle. The selection policy and C3 criterion do not
change. The original negative cohort remains negative even if the supplement
passes the missing second-position case.

**Unconditional stop:** this protocol allows at most one hosted cohort. Complete
the fixed run count regardless of winner unless capability/reference/readiness,
execution, freshness, oracle or infrastructure failure stops it earlier; preserve
partial evidence and do not restart. Do not tune inputs, choose seeds, inject
delays/history/clocks, swap positions, or retry to seek a desired order. If C3 is
still unmet, retain it as an open coverage limitation and propose no further
experiment without a materially different reason. A discovered code defect
requires targeted regression/review; no automatic second cohort is admitted.

Publish this investigation and amendment, then obtain independent confirmation of
the published protocol before execution. An explicitly selected example mode and
reference/evidence scripts may implement this fixed protocol; production policy,
defaults and original `--serial-ready` behavior remain unchanged. Independent
review of the resulting implementation and evidence is required. No M2, merge,
GPU, remote worker or deployment is admitted by this amendment.
