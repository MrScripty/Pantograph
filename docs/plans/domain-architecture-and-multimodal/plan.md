# Plan: Domain Architecture And End-To-End Multimodal Workflows

**Plan status:** `Active`

**Current phase:** M3 owner/client consumer source is implemented. Native CPU sampling and the supported inference/embedded host route are qualified separately; the desktop-authored dependent text-to-image graph and full review remain open.

**Next slice:** Parent reviews frozen text-controls composition `9e8cd64c` and its
separate [native parallel test repair](reports/2026-10-06-diffusers-native-parallel-isolation.md).
Exact original qualification evidence is published separately at `fdb2bf2b`.
The original parallel invocation now passes with deterministic foreign-thread
coverage, without scheduler class mutation or reduced parallel coverage. Parent
reports scoped independent acceptance of repetition/KV source `59992b9b` and
minimum-token source `64b81b0b`; frozen sources/evidence remain unchanged. The next
bounded inference capability is existing typed text sampling seed, on its separate
branch after this qualified publication gate. Parent owns PR/review/merge and
fresh hosted CI. Preserve full peaks, custody, uncertainty, known zero,
source/instance fences and the no-build-download contract. Native desktop,
GPU and pretrained/custom-model acceptance remain separate.

**Prior native qualification:** Parent reviews frozen text-control source `a8c6970f` and its separate [exact-head native qualification route](reports/2026-10-05-text-control-native-qualification-route.md). Earlier local native host/workflow attempts failed before execution: the pinned ONNX dependency download returned HTTP 403, and no-download builds lacked linker symbols. Subsequent [hosted run 37360010163, job 111931961024](https://github.com/MrScripty/Pantograph/actions/runs/37360010163/job/111931961024) at exact head `815bceffbb0d877644184b71be6584f586be2e6e` passed 145 Rust tests across eight groups and six actual CPU sampler tests; all individual quality checks and all three workflows passed at that head. The focused native host/workflow qualification is complete for that head. Next qualify current owner-produced Pumas identities/facts/load targets/devices and the desktop-authored dependent text-to-image graph (DA-03). Frozen temperature `e0293ebf`, CI correction `fddae90d`, fixture `174c1950`, top-k `3bf3eb45` and composition `60197971` remain unchanged with their stated qualification requirements. Parent holds publication for owner confirmation and integrates review corrections without rewriting history. Current Pumas pin: `26a84e323cae566a46a8f76bef48fa1010aed48b`. DA-03/DA-07 remain open.

The next bounded scheduler capability is a fresh host RAM ceiling for explicitly declared backing pools, after CPU candidate discovery. See [host RAM ceilings](reports/2026-10-05-runtime-owned-host-ram-ceilings.md). It may only lower the configured budget, must preserve live task/resident charges after shrink, and must reread missing or changed owner facts at authoritative admission. It does not infer GPU placement, external-consumer allowance or a resident/transient discount.

CPU `394d4748` passed independent bounded source review. Frozen RAM `d7903010`
received two correctness findings: real cgroup hierarchy roots lack `memory.max`,
and over-capacity RAM incorrectly blocked unrelated VRAM resident publication.
The [narrow review repair](reports/2026-10-05-host-ram-ceiling-review-repairs.md)
distinguishes verified real roots from namespace-visible roots and limits resident
validation to charged pools. Its native source remains unavailable in this
container's unresolved `/..` mount layout. The independent [exact runtime service timing slice](reports/2026-10-05-exact-runtime-service-timing.md)
adds opt-in actual gateway/host observations with strict identity and provenance,
deterministic lifecycle tests, and disabled-by-default collection. Native owners
without immutable content and loaded implementation/configuration/device facts
remain unknown; controlled tests do not qualify real model latency. Ranking and
calibration remain separate until comparable native owner evidence is available.

Independent review accepted RAM repair `43f0a777`. The
[service timing retention repair](reports/2026-10-06-service-timing-correlation-review-repair.md)
replaces raw caller ID retention with bounded correlation digests, preserving
actual IDs through direct gateway and host execution, including rejected calls.
Frozen timing and device checkpoints remain separate. Parent coordinates normal
history-preserving integration with approved main `4153772634269e342a8b0cca797f1cd6716f18a5`;
its prior descriptor/sampler repairs will not be duplicated on feature ancestry.

The [runtime-owned CUDA fact slice](reports/2026-10-05-runtime-owned-cuda-device-facts.md)
uses the existing embedded PyTorch owner for explicit UUID/property observations.
The real CPU-only runtime qualifies the unavailable path; positive UUID tests are
controlled and GPU execution/capacity remain unqualified. Configured labels, Pumas
monitor aggregates and llama.cpp selectors cannot become physical backing facts.
Automatic GPU admission and completion ranking remain pending authoritative owner
placement, shared backing/capacity and comparable timing evidence.

The [bounded integration checkpoint](reports/2026-10-06-runtime-owner-integration-readiness.md)
combines both owner successors with approved main through ordinary merges. The
historical checkpoint records its two suite failures; the descriptor repair
arrives from main. Owner-authorized resumption produced the
[tested integration successor and draft review packet](reports/2026-10-06-runtime-owner-integration-successor.md):
canonical backend registration and real-owner abandoned warmup cleanup now pass
all affected library/host suites. The remaining model-contract failure is closed
by the [Pumas wire/projection qualification](reports/2026-10-06-pumas-wire-projection-qualification.md).
The producer version field remains current; the stale test had bypassed the
existing typed host adapter. Full mixed-backend inference and embedded suites now
pass. Parent coordinates PR publication, review and merge; native GPU, exact-model
and GTK qualification limits remain unchanged.

**Acceptance status:** `blocked`

**Independent scheduler slice:** The parent authorized continued feature delivery
while frozen native sampling qualification is blocked. The
[shared resource admission candidate](reports/2026-10-05-shared-resource-admission.md)
adds explicit backing domains across runtime RAM/VRAM claims, including unified
memory and authoritative commit/custody checks. Its separate
[startup composition successor](reports/2026-10-05-shared-resource-composition.md)
activates explicit persisted declarations through actual AppConfig/desktop setup;
portable config-factory/registry tests execute without ONNX. Native desktop qualification and automatic physical bindings remain pending;
parent owns coordinated review. Explicit-domain capacity now has an independent
host RAM ceiling successor; it does not establish complete available capacity.

The separate [AppConfig qualification successor](reports/2026-10-05-app-config-startup-qualification.md)
preserves frozen `73211ddc`, moves the actual persisted settings/startup composition
owner into `pantograph-app-config`, and fixes `Path::exists` hiding filesystem
failures. Real AppConfig cold-load/save/composition and filesystem regressions
execute locally. Authorized official dependency installation is blocked by OS
permission denial; the real desktop build fails at missing GLib prerequisites.
Tauri setup/IPC and a running desktop remain unqualified.

The independent [resident-resource successor](reports/2026-10-05-resident-resource-accounting.md)
adds explicit model/producer resident envelopes to the registry's existing local
and shared accounting, preserving charges after task release until confirmed
stop. Missing shared-pool resident estimates produce typed unavailable outcomes.
Portable lifecycle/contended admission tests qualify this owner capability;
the [producer bridge](reports/2026-10-05-runtime-producer-resident-estimates.md)
now publishes explicitly configured estimates from the active PyTorch lifecycle
owner with ordered-generation and known-zero handling. The bounded lifecycle repair at `b9e83208` establishes resident accounting or
uncertainty before terminal custody release, reconciles Retain, and makes failed
owned allocations ordinarily reclaimable. `dedbef88` adds acknowledged llama
child shutdown and ordered resident batches across PyTorch and llama owners.
Controlled real-host regressions qualify those paths. A proven resident/transient
split, GPU execution and real model execution remain separate gaps. Frozen
`eabbcc83` and `254aef1` are unchanged.

The [CPU candidate successor](reports/2026-10-05-runtime-owned-cpu-device-candidates.md)
projects available owner CPU variants into automatic selection and real selected
text execution while preserving full peak claims. Its full embedded suite has two
explicit frozen-parent failures: the four-versus-six descriptor assertion and the
warmup-timeout test receiving success. The descriptor repair `9ec3c2e` is already
an ancestor of PR55 qualification `ff84841`, but is absent from CPU `394d4748`.
Do not duplicate it on this ancestry or increase the warmup timeout.

Scheduler thesis priorities after the RAM ceiling slice are comparable exact-model
load/execution observations, actual per-device GPU ownership/backing facts, and
then completion-oriented ranking evaluated on frozen matched cohorts. Existing
technical-fit history ranking is already present. Warm-first heuristics, GPU class
counts as physical devices, and unproven resident/transient discounts do not meet
the thesis's physical-feasibility or timing-evidence requirements.

**Execution ledger:** [execution-ledger.md](execution-ledger.md)

**Issues:** [issues.md](issues.md)

**Reports:** [planning baseline](reports/planning-baseline.md)

**Related ADRs:** [existing decisions](../../adr/README.md), especially ADR-001, ADR-006, and ADR-011–ADR-016. These remain binding until explicitly replaced with consumer and migration evidence.

## Objective

Make Pantograph understandable and maintainable through coherent domain ownership
and small, useful interfaces, while making real text and image model execution
work together in a desktop-authored workflow and return retained results.
Review the complete maintained repository against the newly selected standards;
repair applicable violations and architectural entanglement without rewriting
sound code merely because it was produced by an older agent.

Use **GPT-5.3 Codex Spark** for evaluation on real small, well-defined changes
and bounded repairs; **GPT-5.6 Luna max** for larger enumerated changes with
settled contracts; **GPT-6 Astra low** for complex implementation, integration
and rescue; and **GPT-6 Astra medium** for consequential analysis/design and
substantive independent review. GPT-5.6 Sol medium is additionally authorized for bounded implementation and
verification where it can reduce complete accepted-change costs. These are the user's current routing choices,
superseding the earlier two-model implementation preference. Preserve exact
requested models and record availability or fallback explicitly. Select work by
complete API dollars per accepted change, including checks, review, repairs,
rescue and shared coordination; differing task classes are not a controlled
benchmark. These settings apply to delegated tasks, not the primary session's
configuration.

## Standards And Plan Authority

Standards source:
`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards`

Planning baseline: `366c1d90a24bbfb50973f62b155a5f3396c0f107`.
Read Core and Router, then canonical workflows/topics/profiles selected by actual
task facts and their Requires dependencies. Legacy `*-STANDARDS.md` navigation
files do not restore retired policy. Record material local standards changes or
later revisions before using them; do not silently mix acceptance baselines.

This plan is active under the user's instruction to begin with subagents and
the cost-effectiveness pilot. M0 superseded the [old portfolio](../current-standards-remediation/plan.md),
its five child plans and the [image plan](../current-image-generation-graphs/plan.md).
All old findings/claims are mapped in the [coverage report](reports/repository-review.md);
none are accepted by supersession. The old audit and accepted cleanup remain history.

## Documentation Proportionality

Apply the existing Planning, Documentation, Implementation and Development
Proportionality workflows; this policy requires no standards exception.

- Keep this single plan as sequencing authority. Do not create a child plan per
  agent, crate, finding or commit. Create another plan only when independently
  owned migration or coordination facts actually require it.
- Update current decisions, phase, next slice, blockers and acceptance when
  their meaning changes. Record those changes with the coherent implementation
  or evidence that caused them; no standalone lifecycle commits or per-command,
  per-agent-turn or per-commit documentation updates.
- Keep one concise ledger entry per accepted slice, plus material deviations,
  failed acceptance gates or verification changes. Link deciding evidence rather
  than copying command transcripts, diffs or test output into several documents.
  Successful routine checks need command/scope, result and relevant revision or
  environment, not a permanent raw-log report. Retain detailed evidence only
  where it decides a required claim or explains a failure.
- Agent reports contain source-backed findings, decisions and review coverage;
  no chronological work diaries or separate polished summary of the same facts.
  Consolidate duplicate findings into issues; update their disposition there.
  Completed reports remain dated evidence, not documents kept synchronized with
  implementation. Current policy lives only in its declared owner.
- Keep ordinary reversible design choices in code, tests and the change
  description. Update a canonical guide or contract document only when its
  durable knowledge changes. Use an ADR for a consequential durable decision,
  not every extraction, rename, test or interface-preserving refactor. Do not
  create a source-directory README without a real boundary-documentation need.
- Use one composed-design record per coherent changed composition; downstream
  slices link to it. Revisit affected answers only when ownership, caller
  knowledge, lifecycle, compatibility or observed change propagation changes.
  Do not copy the full probe into every task or repeat unchanged reviews.
- At M0, supersede old plan authority once and leave its history alone. At wave
  closure, compact obsolete narration only when it obscures current decisions;
  avoid cosmetic rewrites and unrelated formatting.

These reductions do not excuse stale current authority, missing coverage,
undispositioned findings, absent migration rationale, or unavailable acceptance
evidence. No new documentation generator, checker or registry is implied.

## Product Contract And Scope

Initial fixture assumption, subject to Milestone 0 confirmation:

`text input -> real text inference -> generated prompt -> real image inference`

The desktop user can author/save or load that graph, select real models, submit
one run, observe progress, and retrieve both generated text and a decodable image
in I/O Inspector. The image task must consume the text task's actual output;
two unrelated successful runs do not prove this contract. Both tasks share the
same workflow run identity, with distinct task/attempt identities.

The concrete port/fixture and policy-qualified cold-reopen procedure is in the
[desktop report](reports/desktop-review.md#executable-fixture-contract). Real
model/artifact IDs and qualified runtime/display remain missing prerequisites;
the selected text output port is `text`, not the retired `response` port.

An image-plus-text input to a vision model is a different capability. Record it
as required if selected by the user; do not claim vision acceptance from image
generation. Model families, supported input representations, streaming promises,
hardware, and persistence lifetime are established from real requirements in M0.

In scope: all maintained first-party Rust, Python, TypeScript/Svelte, bindings,
reusable packages, launchers, scripts, build/CI configuration, tests, and durable
documentation. Inventory generated, vendored, obsolete, and external material
separately, with source owners and reasons for exclusions. Review all maintained
areas, but change only evidenced violations or justified design problems.

Out of scope: publishing releases, changing external repositories without a
separate authorized write set, speculative new model families, restoring live
generated-UI execution without an accepted isolation design, and a wholesale
framework or distributed-services rewrite. Existing release/support obligations
are reviewed even though publication is excluded.

## Objective Acceptance

| ID | Observable criterion | Kind | Environment | Mode | Status | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| DA-01 | Every maintained area and reachable interface is reviewed against applicable current routes; each finding has evidence, owner, severity and disposition; no required violation remains open. | focused | not-applicable | either | pending | M1/M5 coverage and findings reconciliation |
| DA-02 | Changed domain interfaces have one owner per invariant/state/lifecycle; representative changes demonstrate reduced unrelated caller knowledge; replaced authorities and unsupported paths are removed or have real migration obligations. | integration | representative | either | pending | M2/M4 composed-design reviews and interface tests |
| DA-03 | One real desktop-submitted mixed graph runs real text and image models, passes generated text along its edge, and exposes both retained outputs with correct run/task identity. | user-workflow | required-real | automated | blocked | EX-04 and D-02 controlled integration accepted; real fixture/runtime prerequisites remain |
| DA-04 | Missing models/dependencies, denied model-code trust, invalid graphs, worker failure, cancellation and shutdown produce the declared terminal outcomes without false success, leaked processes/reservations, or cross-run input leakage. | system | representative | automated | pending | Focused lifecycle contracts and real worker process evidence |
| DA-05 | Accepted saved graphs and retained outputs obey their declared compatibility/lifetime contract through cold reopen; affected bindings and generated/IPC contracts agree with their owners. | contract | representative | automated | pending | Consumer fixtures and cold-process persistence checks |
| DA-06 | Model selection, submit-to-dispatch overhead and runtime reuse meet workload-specific budgets recorded before optimization; measurement separates model computation from orchestration and includes resource use. | system | required-real | automated | pending | M0 baseline and M4 comparison |
| DA-07 | Required supported-target, static, test, accessibility and artifact claims pass on the final integrated material revision; required unavailable evidence remains visibly blocked. | system | required-real | either | pending | M5 claim-specific evidence bundle |

DA-07 aggregates links, not proof kinds: any required release-artifact,
user-workflow, contract, or platform claim retains its own kind and environment
in the acceptance matrix created in M0/M1. A local build cannot close those claims.
An exception is explicitly owned and reported; it is not full compliance with
the overridden obligation. Product success and full review closure are reported
separately until all seven claims are satisfied.

## Constraints And Binding Decisions

| Decision | Owner | Rationale / replaced assumption |
| --- | --- | --- |
| One integrator controls shared contracts, lifecycle and acceptance. | Integrator with Astra medium design/review | Parallel effort must not duplicate authority. |
| Product execution is demonstrated early, before unrelated repository cleanup. | This plan | Replaces the old portfolio's broad prerequisite queue after M0 handoff; necessary trust/contract prerequisites remain. |
| Existing code and ADRs are evidence, not automatic proof of good design or defects. | Domain reviewers | Fresh review replaces inherited diagnosis and blanket rewrite assumptions. |
| Pumas owns model/package/artifact facts; Pantograph owns workflow and execution policy. | Existing architecture and affected contracts | External load-target gaps remain explicit dependency work, never local path guessing. |
| Runtime host executes a scheduler decision; UI and transports project owned outcomes. | Existing ADRs | No special demo executor or frontend-selected runtime policy. |
| Prefer ordinary tests and existing tools over new compliance infrastructure. | Verification owner | Every permanent checker/registry requires distinct deciding value and lifecycle justification. |
| Preserve unrelated local work and real public/persisted contracts. | Integrator | Protected Pumas proposals are outside this plan's write set. |

## Review Method And Domain Map

Start from behavior and trace producers, state transitions, consumers, failures,
and lifecycle. Do not allocate refactors by file size or presume each crate is
already a sound domain. The initial investigation map is:

| Concern | Question to settle |
| --- | --- |
| Graph authoring and node contracts | Who owns graph validity, inference ports, edits, graph revisions and materialization? |
| Workflow run and task orchestration | Who owns run identity, dependency readiness, attempts, cancellation and terminal results? |
| Scheduler and resource admission | Who owns placement, compatible grouping, reservations, waiting, fairness and dispatch? |
| Model library and dependencies | Which facts belong to Pumas, and how are freshness, readiness and authorized load targets conveyed? |
| Runtime host and inference | Who owns execution protocols, model residency, trust, worker processes and shutdown? |
| Media and artifacts | Who owns values versus references, conversions, retention, retrieval and run attribution? |
| Desktop and graph package | Which state is user intent versus backend projection; how are errors, stale updates and accessibility handled? |
| Bindings and infrastructure | Which consumers, compatibility promises, build targets and operator lifecycles are actually supported? |

For every material interface record only useful facts: responsibility and
non-responsibilities; owned values/invariants; callers; inputs/outputs; lifecycle,
ordering and errors; persistence/version obligations; hidden implementation;
and focused acceptance evidence. Put durable rationale in its canonical
architecture/contract document, not duplicated across reports.

Compare keep-and-repair, consolidate/delete, and a new seam only where a material
decision needs alternatives. Once a reversible design satisfies the contract
and standards, implement it. Additional review needs a named unresolved decision
and stopping condition. No recurring review tournament or arbitrary score target.

## Simplicity And Ownership Review

**Applicability:** `applicable`

This planning composition retains current execution owners provisionally; it
does not yet admit a redesigned runtime. M2/M4 must replace these provisional
answers with inspected artifact evidence before each structural implementation.

- **Independent concepts and dimensions:** Graph describes what depends on what;
  a run identifies requested execution; scheduler decides when/where; model facts
  describe available artifacts; runtime host owns how execution occurs; artifacts
  own retained results; desktop owns interaction. The concrete division inside
  workflow-service and embedded-runtime is an explicit M1/M2 investigation.
- **State, identity, value, time, policy, mechanism:** Keep graph revision, run,
  task attempt, model/artifact identity, runtime residency, reservations, user
  input and projection freshness distinct. Scheduling references library facts
  without owning their meaning. Artifact retention does not own run scheduling.
  Document actual version roles and consumer overlaps in M1; do not invent one
  global version. Graph edits must not mutate submitted run snapshots; model
  changes invalidate dependent execution facts, not unrelated UI identity.
- **Caller and composition-root knowledge:** Submission callers should supply
  workflow/session identity and typed inputs, not Python paths, GPU placement or
  worker setup. The existing embedded composition factory is a starting point
  to inspect for excess lifecycle/configuration knowledge, not proof of depth.
- **Representative changes and forced owners:** Changing a model-load protocol
  should affect its adapter/host contract, not graph interaction. Changing
  batching policy should stay with scheduler policy and its evidence. Adding an
  image display should consume an artifact contract without choosing retention.
  Updating a model selector should not require scheduler policy edits. Compare
  actual before/after change propagation for these cases.
- **Stable interfaces versus hidden knowledge:** Graph/session operations,
  inference descriptors, scheduler handoff and artifact retrieval are candidates
  for stable interfaces. Storage layout, Python kwargs and concrete runtime
  wiring remain implementation details unless a real consumer contract says otherwise.
- **Independent evolution, testing, failure and replacement:** Test each owner
  through the same interface its consumers use, then exercise the full desktop
  path. Cross-process execution requires real process failure/lifecycle evidence;
  synthetic inference cannot establish model functionality.
- **Deletion result:** This plan introduces documentation only, no production
  registry, generator, framework or adapter. For each later permanent mechanism,
  record what necessary complexity returns to callers if it is deleted; delete
  pass-through machinery whose complexity simply disappears.
- **Necessary complexity and cumulative machinery:** Resource admission, model
  trust, async execution and retained artifacts are inherent. Keep them contained
  in their owners; reassess the complete composition if their knowledge spreads
  across callers or the refactor adds competing authorities.

## Evidence And Oracle Plan

- DA-03 uses a saved fixture and actual model/runtime identities, a trace of the
  materialized edge value, real text output and image decoding, plus assertions
  on I/O Inspector. Exact generated wording/pixels are not required unless a
  selected model contract promises determinism. Nonempty output alone does not
  prove that the edge was used.
- DA-04 uses deliberate failures at the relevant boundary with assertions on the
  intended diagnostic/terminal state, cleanup, and reservation release. Merely
  observing any exception does not satisfy the negative case.
- DA-05 exercises real consumers and cold reopen against authoritative persisted
  data. A re-created in-memory object is not cold-process evidence.
- DA-06 records machine/model/runtime, cold/warm conditions, workload, latency
  distribution and resource use. Initial engineering targets are warm local
  model-selector rows p95 <=250 ms and already-ready task dispatch overhead p95
  <=500 ms, excluding declared batching/admission wait and model computation.
  These bound interactive overhead, not model speed or universal hardware
  guarantees. Model/runtime/device qualification is still unavailable; measure
  baseline before optimization and revisit targets only with recorded workload
  evidence. Run model work serially when contention would invalidate measurements.
- DA-01/02 use source-backed coverage and concrete change-path comparisons.
  File counts, grep scans and passing lint are supporting evidence only.
- Reuse existing scripts and runners after checking their actual proof scope.
  Extend only what the selected claims need. Keep expensive real runs for
  behavior-affecting integration points and final acceptance.

## Systemic Finding Audit

For a repeated defect, bound its invariant, owner, representations and all
reachable consumers before fixing examples. Each occurrence is repaired,
consolidated, removed, retained with evidence, or blocked with an owner. Expand
only for a newly reachable consumer, authority, material risk or supported
contract. Compare the repaired whole composition and remove superseded evidence
machinery. Stop when the bounded family and its acceptance claims are closed.

## Milestones

### M0 — Establish scope, authority and executable fixture

**Goal:** One current plan authority and a concrete product/verification contract.

**Allowed write set:** this plan directory; `docs/plans/README.md`;
`docs/README.md`; the `plan.md` and `execution-ledger.md` of the old remediation
portfolio, its five child plans, and old image plan. Product source is read-only.

**Tasks:** Reconcile old findings/claims without silently dropping any; retire
overlapping plan authority; record standards revision and local modifications;
inventory maintained source roots, consumers and supported targets; identify
real model IDs, runtime/dependency availability, trust requirements, saved
fixture and desktop runner. Record baseline commands and results only where
they decide immediate prerequisites. Select initial performance budgets and
the actual persistence promise. Route standards from observable facts.

**Gate:** DA-01 coverage population is bounded; DA-03/05/06 acceptance contract
is executable or exact external prerequisites are recorded. Each old claim has
a disposition. Missing model/hardware evidence blocks the dependent real run,
not independent architecture review. Review stops when these decisions are possible.

**Status:** `Accepted`

### M1 — Review the domains and repository coverage

**Goal:** Source-backed ownership map and prioritized findings, with execution-path findings delivered first.

**Allowed write set:** this plan's `reports/`, `issues.md`, `execution-ledger.md`
and `plan.md`; product source remains read-only.

**Tasks:** Trace the mixed graph through every real owner. In parallel review
other maintained areas using the table below. Map applicable standards to
actual evidence; distinguish defects, design weaknesses and optional preferences.
For each finding identify the owned invariant, affected consumers, consequence,
proposed disposition and deciding test. Inventory old tests/checkers for coverage
and obsolete guarantees. Do not defer the first production slice until every
unrelated finding is exhausted.

**Gate:** Execution-path review provides enough facts to admit M2 with exact
files and tests. Full repository review may continue alongside M2/M3 but must
close before M5. Record examined populations and unresolved areas explicitly.

**Status:** `Active`

### M2 — Repair the canonical execution path

**Goal:** One coherent backend path can execute the selected mixed graph safely.

**Allowed write set:** plan reports/control files plus RT-01: new
`crates/inference/torch/worker_diffusion.py`, `crates/inference/torch/worker.py`,
`crates/inference/src/backend/pytorch_worker.rs`,
`crates/inference/src/backend/pytorch_worker_image_python_tests.rs`, and
`scripts/diffusion_cli_smoketest.py` (support/help statement only).
Also admitted: `crates/inference/src/backend/pytorch_tests.rs`, registering the
new real helper and completing the existing batch-projector stub in its text-worker fixture; isolated consumer testing
proved that the new import otherwise breaks four existing text/lifecycle tests.
The admission decision in `reports/runtime-review.md` defines the finite built-in
component construction, preserved scheduler configuration, local weights with restricted
deserialization (see the report correction), typed failures and admission-before-cache invariants. Custom code and
other pipeline variants remain unavailable; no configurable authorization is
invented. Astra low implements; independent Astra medium review and real-loader
regressions decide this slice. Other slices still require exact admission.
Candidate owners: workflow-service, scheduler, embedded-runtime, inference,
runtime-host contracts, node contracts and their existing consumers. This list
is an investigation scope, not blanket source-write authority.

EX-01/EX-03 also admits these five inline-test/source files under
`crates/pantograph-workflow-service/src/workflow/`:
`runtime_branch_task_event.rs`, `runtime_dispatch_assignment.rs`,
`task_execution_worker.rs`, `runtime_branch_batch_execution.rs`, and
`session_execution_api.rs`. The final scoped-live-claims decision in
`reports/execution-review.md` governs ownership transfer, both event and batch
fences, immediate compatible grouping, supervised failure and deterministic
tests. Keep proofs out of immutable snapshots; retain finite expiry for unowned
work. Post-dispatch abandonment with unproved host stop is failed/fenced against
replay, not successful cancellation or resource-release evidence. No new timer,
service, schema or scheduler API is admitted. Missing lifecycle capability
returns to medium design before expanding the slice.

Consumer migration additionally admits
`crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`:
six session assertions assumed two independently submitted runs always shared
one host request. Validate nonempty bounded groups and exactly-once run/member
identities across all requests, retaining existing prompt, result, recovery and
diagnostic checks. Do not replace the old timing assumption with exactly two calls.

RT-03 is admitted under the runtime report plus the bounded consumer expansion
found during source verification. Core owner (Astra low):
`crates/inference/src/backend/mod.rs`, `backend/pytorch.rs`,
`backend/pytorch_tests.rs`, optional private `backend/pytorch_text_job.rs`,
`gateway.rs`, `gateway_tests.rs`, and `gateway_tests/start_config.rs` (all paths
after the first share `crates/inference/src/`). Replace backend stop with
`async fn stop(&mut self) -> Result<(), BackendError>` and gateway stop with
`pub async fn stop(&self) -> Result<(), GatewayError>`; preserve failures,
producer completion and coherent metadata under existing ownership.

Consumer owner (Luna max): `crates/inference/src/backend/llamacpp.rs` and
`candle.rs`; `crates/node-engine/src/core_executor/kv_cache_test_support.rs`
and `inference_tests.rs`; these files under
`crates/pantograph-embedded-runtime/src/`: `embedded_runtime_lifecycle.rs`,
`runtime_registry_controller.rs`, `runtime_registry_lifecycle.rs`,
`runtime_registry.rs` (exports), `reservation_lifecycle.rs`,
`embedded_workflow_host_helpers.rs`, `lib.rs` (exports only), `lib_tests.rs`,
`runtime_host_execution_port.rs`, `runtime_registry_tests.rs`,
`runtime_registry_tests/lifecycle.rs`, `reservation_lifecycle_tests.rs`,
`workflow_runtime_tests.rs`, `lib_tests/session_runtime_lifecycle_tests.rs`,
`lib_tests/workflow_run_execution_tests.rs`, and
`lib_tests/runtime_lifecycle_capability_tests.rs`; these files under
`src-tauri/src/`: `llm/gateway.rs`, `llm/runtime_registry.rs`,
`llm/rag_sync.rs`, `llm/recovery.rs`, `app_lifecycle.rs`, and
`llm/commands/server.rs`. Only stop-contract propagation, relevant tests and
necessary exports are admitted, including producer/stop-all controller Result
signatures and a local lifecycle error wrapping registry/gateway failures.
Reconcile observed state even on stop failure; do not return successful reclaim,
restart recovery, invalidate residency or log successful shutdown on that failure.
Additional actual callers require bounded source-backed admission.

This is a useful delegation evaluation of different task classes, not a matched
model benchmark. One integrated medium review covers composition; record both
implementation lanes, design, all review/repair/verification agent calls and
root integration/orchestration for RT-03. Shared slice costs remain explicit,
not zero or an invented per-model allocation. API-dollar estimates use the
ledger's verified rate assumptions and disclose the measurement cutoff.

RT-02 is admitted using the runtime report's selected-text contract and the
completed RT-03 lifecycle gate. Inference owner (Astra low), exact paths under
`crates/inference/src/`: `gateway.rs`, `gateway_tests.rs`, `backend/mod.rs`,
`backend/pytorch.rs`, `backend/pytorch_tests.rs`, new `selected_text_execution.rs`,
and `lib.rs` (module/export only). Host owner (Luna max), exact paths under
`crates/pantograph-embedded-runtime/src/`: new `runtime_host_text_execution.rs`,
`lib.rs` (module only), `runtime_host_execution_port.rs`, and
`lib_tests/workflow_run_execution_tests.rs`, and `runtime_host_image_execution.rs`
(only expose the existing Pumas-target converter as `pub(crate)` for shared use).
Additionally admit `lib_tests/runtime_preflight_tests.rs` solely to gate its
Candle-specific test with `cfg(feature = "backend-candle")`; its unconditional
reference prevents the requested no-Candle test matrix from compiling. Astra low
owns the remaining host integration and this bounded repair after the Luna handoff.
Root owns these plan/control files.
No other image projection, worker Python, external Pumas, wire schema, or other source
changes are admitted without a source-backed expansion.

The shared gateway operation is `execute_selected_text_with_cancellation`,
accepting existing `InferenceExecutionRequest`, separate `PumasArtifactLoadTarget`,
`BackendExecutionDecision`, and `InferenceExecutionCancellationHandle`, returning
`Result<InferenceExecutionResult, GatewayError>`. Inference validates identity,
PyTorch runtime, concrete device, directory target and denied custom-code policy
before effects, and owns residency through observed terminal completion and
cancellation cleanup. Host projects exact `prompt: String` to `text: String`,
uses existing generation defaults, rejects unsupported inputs and strings over
1024 bytes without truncation, and requires no image sink for text. Existing
image and batch behavior remains part of verification. Canonical EX-01/03 sends
even singleton text through the batch entrypoint: admit sequential awaited text
member execution there, validating text member shapes before effects, preserving
member identities and existing retry/reservation outcomes. Completed earlier
members remain retained; failed/cancelled members emit no partial output and
remaining cancelled members do not load. This reuses the selected-text operation,
not a new native text batching mechanism. Independent Astra medium review judges
integrated source and deciding tests, including service→batch-host text retention. Cargo ownership is serialized;
implementation, checks, repairs, review and shared root costs are counted together
with an explicit reporting cutoff. Real-model acceptance remains outstanding.

EX-04 is admitted after medium confirmation of the canonical worker path.
Astra low owns exactly these files under
`crates/pantograph-workflow-service/src/workflow/`, including inline tests:
`session_scheduler_runner.rs`, `runtime_branch_task_event.rs`,
`runtime_branch_batch_execution.rs`, and `task_execution_worker.rs`.
Core additionally owns `session_execution_api.rs` for composed recovery to skip
validated scheduler-completed upstream tasks without replay while preserving
mismatch/failed diagnostics, plus its focused recovery regression.
`task_execution_facade.rs` and `task_execution_runtime.rs` are admitted only for
two diagnostic test expectation migrations caused by session validation before
event claim. Pre-proof dependency-pending events remain Ready/unclaimed; this
does not change post-dispatch deferred settlement/retry policy.
Luna max initially owned only
`crates/pantograph-embedded-runtime/src/lib_tests/workflow_run_execution_tests.rs`
for dependent canonical retained-result fixtures, preserving the accepted singleton
case. Astra low completed the fixture integration, bounded repairs and composed
recovery evidence after the explicit ownership handoff; the shared write set did
not expand. Root owns this plan, issues and ledger. No wire/persisted schema change or
other source file is admitted without source-backed expansion.

Move scheduler progress/admission before claim, select only a Ready task with its
dependency proof, and claim its matching run/task event. Classify next-task
readiness versus all-complete instead of requiring the entire graph Ready.
After observed host completion, retain task results and settle the current
assignment/event; unfinished work is a typed continuation. The worker retains
and rebinds the same run responder under its existing supervised ownership,
then advances downstream inputs and selects the next task. No detached
continuation, premature whole-run success, copied-proof authorization, or lost
cross-run batch responder is allowed. Existing failed/deferred/cancelled and
shutdown/fencing semantics remain. Call the existing whole-run finalizer only
when all tasks complete. Canonical dependent retained text and text→image
fixtures, cancellation/failure and claim-ownership regressions plus independent
medium review decide acceptance; controlled adapters do not prove real models.
Cargo ownership is serialized between core and fixture checks.

**Tasks:** Fix confirmed trust authorization before real model execution;
consolidate inference descriptor/validation authority; settle scheduler versus
run orchestration ownership; repair Pumas load-target/dependency handoff;
preserve task identity, real edge materialization and artifact attribution.
Support solo dispatch without requiring a peer; preserve compatible batch
semantics. Exercise real text and image model adapters behind canonical entry
points. Delete replaced paths after consumer migration is covered.

**Gate:** Focused contracts and affected static checks pass; canonical backend
mixed workflow returns both outputs; denied trust and missing facts fail at
their proper owner. This is intermediate evidence, not DA-03 desktop acceptance.

**Status:** `Active`

### Pumas owner/client integration — implemented source, qualification open

The selected design below is retained as rationale. Its producer operation and
consumer migration are already present at the current pin; they are no longer
the next implementation slice. `PumasSelectorAccess` routes full facts and
targets through Owner/LocalClient, while dispatch, host resolution and hosted
composition consume that facade and preserve ReadOnly execution refusal.
The [full-facts consumer report](reports/2026-10-04-pumas-full-facts-consumer.md)
records the six boundaries, exact producer identity and outstanding native
acceptance. Its earlier source-only receipts do not establish real model or
desktop acceptance. The [configured-owner report](reports/2026-10-04-pumas-configured-owner-client.md)
records accepted attachment and configured-root ownership behavior.

Workflow text generation already supports `max_new_tokens` and `system_prompt`.
The [top-k candidate](reports/2026-10-05-workflow-text-top-k.md) adds the missing
optional integer sampling input through existing U64 values and typed options,
with explicit zero and unchanged omission semantics. This is a bounded input
projection; scheduler redesign and broader backend support are not admitted.
The original feature composition with PR54 repair head `78bc7193` remains
frozen. Its successors preserve both histories: the [public-session fixture repair](reports/2026-10-05-pr54-session-output-discovery.md)
persists real built-in I/O definitions, and the [sampling repair](reports/2026-10-05-top-k-vocabulary-sampling.md)
caps positive k at vocabulary width and preserves non-streaming explicit zero.
Parent review and final native qualification remain outstanding.

**Decision:** Reuse the existing Pumas facts/target operations and Pantograph
access facade. Pumas owns model identity, inspection, freshness, cache and path
resolution. Pantograph owns scheduling, device/runtime choice, code permission,
execution and output retention. The two applications share an existing library
owner; an attached client never owns that process's shutdown.

**Producer change:** Pumas already implements full package facts in its owning
interface, HTTP RPC and an internal dispatch branch. Complete the typed local
IPC operation/client exposure with connection-token validation, bounded model
ID decoding and the existing public diagnostic conventions. Reuse the domain
resolver and cache. Do not add an aggregate execution API, HTTP endpoint,
independent cache, model-copy mechanism or general connection framework.

**Consumer change:** Use the existing Owner/LocalClient/ReadOnly facade for
facts and targets instead of concrete owner dependencies in dispatch and host
composition. Execution accepts Owner or a client attached to the configured
launcher root; ReadOnly remains explicitly non-executable. Match Pumas's
advertised canonical launcher-root identity and try that owner before a
read-only fallback. Never select another library merely because it connects.
Retain lifecycle cleanup only for owners constructed by Pantograph.

**Facts and ordering:** Keep model-list snapshots cheap and hydrate full facts
only for selected execution candidates. `OwnerFresh` target resolution refreshes
external validation but does not hydrate package facts. Validate selected
artifact identity across facts/target and preserve typed stale, missing,
invalid and unsupported outcomes. Schema-version equality is not evidence
identity; the current target supplies no content fingerprint or file lease.
The two operations do not promise atomic immutable contents. File-open failure
remains an execution error; a future immutable-result requirement needs its own
producer-owned contract. Keep executable paths out of scheduler projections.

**Change locality and deletion:** Existing Pumas owner/client adapters contain
transport differences. Completing them removes owner-specific rejection and
branching from Pantograph's facts/target consumers. A Pumas storage/cache change
stays in Pumas; a Pantograph scheduler-policy change stays in Pantograph.
Keep existing internal resolver traits for host tests; no new permanent module
is needed. Separate dispatch and host freshness checks remain until measured
reuse can preserve their semantics; no speculative cache is admitted.

**Bounded implementation candidates:** In Pumas, paths under
`rust/crates/pumas-core/src/`: `ipc/protocol.rs`, `ipc/local_client.rs`,
`api/state.rs` and their existing focused contract tests. In Pantograph:
`crates/workflow-nodes/src/setup.rs`; embedded-runtime's
`pumas_dispatch_package_facts.rs`, `runtime_dispatch_load_target_facts.rs`,
`runtime_host_package_facts.rs`, `runtime_host_load_target.rs`,
`workflow_service_composition.rs`, and affected existing tests; dependency
pins/locks and actual independently pinned consumers must be enumerated before
editing. This design selects scope, but does not authorize overwriting either
repository's active work; freeze the concrete revision and exact test/pin
write set when admitting implementation. The existing domain-architecture plan
remains the sole Pantograph sequencing authority.

**Gate:** Real owner/local-client contract equivalence, authentication/invalid
request/owner-loss failures, correct-root selection, ReadOnly refusal, selected
artifact changes and invalid/missing size/target shape rejection; actual
producer facts through canonical dispatch/host, no scheduler path leakage,
cheap snapshots and measured selected hydration. Target transport completion
is separate from current-model identity/path qualification and real inference.
Pumas path/identity repairs require reproduced current producer evidence.

### M3 — Prove the desktop mixed workflow

**Goal:** The user authors/submits the graph and receives text and image results.

**Allowed write set (D-02):** `src/components/workbench/ioInspectorPresenters.ts`,
`src/components/workbench/ioInspectorPresenters.test.ts`,
`src/components/workbench/IoInspectorPage.svelte`, and the existing
`tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs`
(decode/dimension assertions only). Luna max owns these settled repairs.
Sol medium owns the disjoint controlled browser fixture
`tests/e2e/io-inspector-image-preview/run.mjs` and `fixture.ts`.
Root owns this plan, issues and ledger; Astra medium independently reviews.

D-02 uses the existing full-body request for image Read while retaining bounded
text previews. Both Read and Read Stream reject incomplete images before
preview installation. Inspector owns image decode errors, matching-URL cleanup,
and stale request disposal across selection changes and unmount. Download
remains independent of display decoding. The new harness mounts the actual
Inspector with controlled service responses and a valid PNG larger than 64 KiB
in real WebKit; it supplies the decoder/lifecycle evidence absent from Node
presenter checks without requiring models. Retain it while those browser
contracts lack equivalent coverage in the canonical desktop harness. This
repair preserves artifact storage and IPC contracts. D-02 is accepted after focused requests, browser success/failure/lifecycle
checks, affected static checks and independent medium review; it does not
satisfy DA-03. Further mixed-workflow source writes require
separate concrete admission.

**Tasks:** Integrate actual node ports and model selection, run progress/errors,
artifact retrieval and I/O Inspector. Extend the existing Tauri/WebKit harness
to prove the dependent text/image graph. Verify save/reopen, cancellation and
failure presentation. An inaccessible required interaction is part of this slice.

**Gate:** DA-03 passes on recorded real models/hardware/runtime; affected DA-04
and DA-05 cases pass. Preserve raw bounded evidence in reports. If blocked,
record the exact missing prerequisite and continue unrelated admitted work.

**Status:** `Planned`

### M4 — Complete domain remediation and measured efficiency

**Goal:** Close the remaining full-repository findings while preserving the working product path.

**Allowed write set:** P01/P02 exact paths in First Pilot Admission, plus P03
`crates/inference/src/gateway_tests.rs` for the existing PyTorch alias test's
feature-independent Mock backend constructor only, plus plan
reports/control files. Other finding families require exact production/test/
documentation write sets before implementation.

P04 also admits only
`crates/inference/tests/fixtures/pytorch_worker_contract/load_transformers_model_request.json`:
change nested `payload.model_source.source_contract_version` from 2 to 3.
Medium reviewed all 19 worker JSON fixtures and the source-contract migration;
no other field or validator change is required. This is a low-complexity positive
fixture repair assigned to Astra low, comparable in scope to Luna's P03 fixture
repair. Preserve outer worker version 1 and negative stale-version cases.
Existing load-envelope consumers and invalid-source tests decide acceptance;
independent medium review remains required. It is the fourth useful pilot task,
not a matched test of multi-file architecture implementation.

**Tasks:** Refactor independently changing concerns identified in M1; address
binding/runtime lifecycles, frontend state/accessibility, dependency ownership,
test discovery, launcher/build/release obligations and stale documentation as
applicable. Measure selector performance, scheduling overhead and reuse; fix
demonstrated problems at the owner. Pumas changes require their own authorized
work; do not duplicate its semantics. Remove dead paths and unjustified checks.

**Gate:** DA-01/02/04/05/06 evidence closes for each admitted finding family;
affected tests and static gates pass; the mixed workflow remains functional.
No arbitrary file-size target, new crate quota, or mandatory universal abstraction.

**Status:** `Active`

### CI-01 — Session Capacity Observability Repair

**Decision (2026-10-02):** Explicit keep-alive creation/enablement owns the
existing session-count capacity limit, host-selected eviction and affinity.
Runtime-task dispatch continues to own dependency admission and resource
reservations; non-runtime runs must not acquire a session runtime. The five
legacy capacity tests reached the wrong owner after this split. Moving only
their trigger would lose previously asserted eviction diagnostics and timing:
`session_runtime.rs` currently receives no diagnostic context from its sole
keep-alive caller, while that context requires a run that does not exist.

**Admitted repair:** Emit eviction lifecycle facts with genuine target and
unloaded execution-session identities and optional real run attribution. Extend
the existing typed diagnostic payloads additively, retaining old payload decoding
and validation of run-only events. Add a registered session-runtime error scope
so an unload failure retains its original code/message and reports diagnostic
unavailability without inventing a workflow run. Preserve host selection,
rollback, cleanup, event order and a shared timing-attempt identity. Session-only
facts belong in the ledger; they must not create fabricated run projections.

**Write set:** workflow-service `session_runtime.rs`, `diagnostic_errors.rs`,
`tests/session_capacity.rs`, `tests/session_capacity_faults.rs`, test module
registration and focused diagnostic tests if needed; diagnostics
ledger `event.rs` and `tests.rs`; this plan, its scoped report, and the owning
`docs/headless-workflow.md` guide. No runtime-task admission, resource reservation,
non-runtime execution, public run facade, or unrelated baseline repair changes.
The integrator owns this decision and independent review. Implementation is on
`fix/session-capacity-observability-2026-10-02`, separate from fixture PR #7.

**Gate:** Direct keep-alive lifecycle tests preserve victim selection, all three
affinity cases, explicit target cleanup, rollback and original unload failure;
available and unavailable diagnostics are tested. Ledger tests accept valid
session-only events, reject missing/blank identity, preserve old run-event
contracts, and demonstrate no fabricated run projection. The non-runtime no-load
regression and focused runtime-dispatch tests remain green. Full CI gaps remain
explicit; independent review precedes integration.

### M5 — Integrated acceptance and maintainer handoff

**Goal:** One honest, current product and compliance result.

**Allowed write set:** this plan's files and exact canonical guide paths declared
at M4 closure; source fixes return to a bounded M2/M4 slice.

**Tasks:** Reconcile complete coverage, all old/new findings, supported consumers
and required target claims. Review changed architecture independently from its
implementer. Run the selected integrated checks and real desktop workflow on
the final material revision; update architecture/consumer/run guidance with
actual behavior. Explain remaining limits without labeling them compliant.

**Gate:** DA-01–DA-07 satisfied, no undispositioned area or required violation,
no unavailable evidence represented as passing, and no competing plan authority.

**Status:** `Planned`

## Concurrent Work

### First Pilot Admission (P01/P02)

**Status:** `Accepted` for these two scoped repairs; see ledger for evidence.
This does not accept the complete M4 milestone or establish a cost winner.

Source-backed review establishes these two independent M4 prerequisite repairs
can proceed while M2's execution design is resolved. This changes milestone
ordering only; it does not permit real model execution before trust closure.
The integrator owns current contracts and plan writes; workers deliver code,
not outstanding proposals to mutate shared authority. No conflicting write sets
or stale admission facts have been identified for this pair.

| Slice | Model | Contract and exact allowed write set | Acceptance |
| --- | --- | --- | --- |
| P01: D-01 GUI smoke temporary-root cleanup | Luna max | `scripts/check-workflow-editor-image-generation-gui-smoke.sh`; `scripts/check-workflow-editor-image-generation-gui-smoke.test.mjs`. Keep shell lifecycle ownership through direct child completion, preserve status and remove only its own allocated root. Preserve existing INT/TERM delivery addressed to wrapper PID, wait/reap child before cleanup and return 130/143. Whole driver/app process-tree supervision is not newly promised. | Copied real wrapper in isolated fixture with fake wdio verifies child input/root, success/nonzero exit and cleanup; ready-marker INT/TERM tests assert child receives signal while root exists, terminates before cleanup and leaves no direct child; `bash -n`; independent medium review. This is wrapper contract evidence, not GUI/model acceptance. |
| P02: COV-01 frontend test discovery | Astra low | `package.json`; only if Node's built-in discovery is insufficient, `scripts/run-frontend-tests.mjs` and `scripts/run-frontend-tests.test.mjs`. Discover `.test.ts` under maintained `src/` and `packages/` roots, including new nested files; exclude dependencies/generated output; preserve test failures. Prefer existing Node glob support over a custom runner. | All 90 currently tracked frontend files selected plus a temporary nested regression; intentional failure propagates; excluded-root fixture not selected; complete discovered suite run and independent medium review. Newly exposed failures are reported and assigned, never hidden to make this patch pass. |

These are different tasks with unequal possible repair effort. Their results
start the four-task pilot but cannot establish a causal model ranking. P03/P04
remain unassigned until comparable admitted tasks are available; D-02 large-image
preview and COV-02 packaging are findings, not automatic new write authority.
Per-agent billing is unavailable from current tools; first-pass quality and
repair rounds are observable. Total agent wall time was not instrumented for
this pair; measured test runtime is not model latency. No dollar saving is claimed.

The composition remains the existing shell launcher and Node test runner;
P01 restores an existing resource owner, P02 removes a duplicate manual test
inventory. No new domain owner, public contract or persisted schema is admitted.
Any proposed custom runner must justify why the built-in mechanism cannot own
discovery; it must not become a new test registry.

Use one integrator, Astra medium analysis/review workers, and
implementation workers selected by the policy below. The restarted session exposes ten subagent slots;
that is capacity, not a utilization target. Start with the four bounded review
lanes below and add workers only for independently useful admitted tasks:

| Owner | Primary write set | Adjacent write set | Forbidden/shared | Output | Integration order |
| --- | --- | --- | --- | --- | --- |
| Execution reviewer | `reports/execution-review.md` | none | Product and plan-control files | Run/scheduler/graph ownership and failures | First for M2 |
| Runtime reviewer | `reports/runtime-review.md` | none | Product and plan-control files | Inference, Pumas, trust, worker lifecycle | First for M2 |
| Desktop reviewer | `reports/desktop-review.md` | none | Product and plan-control files | UI/graph package, IPC, artifacts, accessibility | For M3 |
| Coverage reviewer | `reports/repository-review.md` | none | Product and plan-control files | Bindings, tooling, dependencies, docs, remaining coverage | M1/M4 |
| Integrator | Plan-control files and admitted slice | Explicitly declared consumer files | Protected user changes | Current decisions, integrated code/evidence | Serial |

Each delegated task supplies: model/effort, bounded question, relevant standards
routes, authoritative contracts, exact write set, prerequisites, acceptance
command/claim, forbidden shared paths and stop condition. Review reports cite
source locations and uncertainties; they do not authorize a competing redesign.

Implementation parallelism is admitted only after contracts and disjoint writes
are established. One owner edits shared contracts, manifests, composition roots
and plan controls. Reviewers may work alongside that implementation. If multiple
outstanding implementation proposals can become stale before integration, first
route and apply the standards' concurrent-plan-integration profile and record
its required revision/reconciliation facts. Otherwise keep serial integration;
agent count alone does not justify new coordination machinery. Reserve real
model/GPU evidence runs to avoid contention and misleading measurements.

### COV-02 Spark Admission

Under the current four-model policy, Spark owns exactly four literal replacements
of `docs/headless-native-bindings.md` with `docs/headless-workflow.md` in
`scripts/package-uniffi-csharp-artifacts.sh`. Preserve all other bytes and behavior.
The latter is the existing live guide and agrees with the package README. Medium
confirmed the route at `snapshot:v1:1356b67e-26ae-4c5b-9e65-6e3e8f9ef58e`:
Core/Router, implementation/verification/build/documentation/library, zero
unresolved facts. Verify the actual script in an isolated temporary fixture with
stub native build/generation, real archive/checksum tools, and both archived
manifests resolving to the exact guide bytes. No real binding build/release claim.
Independent medium review may be combined with EX-04 review. Root owns plan,
issues and ledger; no other packaging or documentation source changes admitted.

### Cost And Quality Controls

- Route delegated work using the four-model policy above with explicit model
  and supported effort settings and bounded context. Spark begins on real small
  changes; its quality and complete accepted-change cost remain under evaluation.
  Do not silently replace an unavailable requested model or expand a small repair
  into consequential design to fill a model lane.
- A medium analysis task stops once the owner, interface, invariants, relevant
  consumer migrations, write set and acceptance evidence are sufficiently clear
  for reversible implementation. Reuse its findings across dependent slices;
  do not commission duplicate broad investigations.
- An implementation task receives that contract and implements the complete
  coherent slice, including focused tests and affected checks. It may make
  ordinary local choices but must not silently change ownership, compatibility,
  trust, lifecycle or acceptance semantics to make the patch pass.
- If implementation exposes a material design contradiction, return the exact
  evidence and decision to a medium analysis task. Do not repeat blind repair
  attempts or automatically raise all implementation work to medium. After the
  decision is resolved, hand the bounded implementation back to the selected
  implementation model; record any rescue by a different model in pilot evidence.
- Keep root orchestration context and output bounded: avoid dumping tool schemas
  or whole reports, batch independent reads/checks, and delegate routine check
  execution with one consolidated result packet where independent integration
  evidence permits. Preserve medium decision/review gates; reducing their
  necessary coverage is not the cost optimization. RT-03 showed root overhead
  dominates implementation cost.
- Medium review examines the resulting diff against the admitted design and
  applicable standards. Review coherent integrated changes together where safe;
  avoid separate full reviews per file or trivial edit. Re-review only material
  changes and unresolved findings, not unchanged accepted work.
- Run focused checks during implementation and broader evidence at the integration
  points that require it. Neither low reasoning nor cost reduction changes the
  acceptance criteria or permits substituting simulated success for real model runs.
- Prioritize API dollars per accepted change over elapsed time and raw tokens.
  Price recorded uncached input, cache reads/writes and output at verified
  model/service-tier/context rates; count reasoning within output once. Include
  attributable design/review/repair/rescue and show shared overhead separately.
  Distinguish API-equivalent estimates from actual invoices; never infer a cost
  winner from fewer tokens alone. Use existing usage logs and ledger rather
  than building a tracking framework. Use fewer workers when decisions, files
  or hardware are shared.

### Bounded Implementation Model Pilot

**Status:** `Complete` (P01–P04). The ledger records quality, usage and the
verified API-price correction. Luna max has lower observed API-equivalent
implementation cost and lower attributable design/review subtotals in this
unmatched sample; prefer it for qualified bounded repairs. Astra low remains
provisional for consequential classes beyond the pilot, on quality grounds,
not a demonstrated cost advantage. Shared overhead and actual billing remain
unallocated. Continue useful product-task evaluations under the user’s latest instruction;
record quality gates and complete attributable costs without artificial benchmark tasks.

The decision is whether Luna max can lower the total cost of accepted changes
for particular task classes without weakening design or correctness. Official
model positioning and reasoning labels do not establish Pantograph performance.

- After M1 provides admitted production slices, select four useful, reversible
  implementation tasks, two per model, with comparable domain complexity and
  evidence requirements. Include both a local behavior repair and a bounded
  multi-file contract-consumer change per model where available. Do not compare
  trivial Luna edits against difficult Astra redesigns or split coherent work
  artificially to fill the sample. Keep unresolved security, persistence and
  lifecycle design with Astra medium analysis; initially use Astra low for
  consequential implementation outside the pilot's tested scope.
- Record task class/difficulty before assignment. Give both models equally
  explicit contracts, relevant source context, write sets and preselected
  acceptance tests; preserve initial results before repair. Use disjoint tasks
  for useful production progress, not duplicate implementations by default.
- Astra medium reviews both against the same behavior, ownership, failure,
  lifecycle, maintainability and standards criteria. Omit model attribution
  from review packets where practical. Tests are run independently by the
  integrator as required; self-reported success and tests that merely mirror
  implementation do not decide quality.
- Keep one compact table in the existing ledger: task/model/effort, first-pass
  gate results, substantive findings by severity, repair rounds or rescue,
  implementation/review/repair usage when exposed, elapsed time, and final
  acceptance. Record later discovered regressions against their originating
  task. No new benchmark framework or separate recurring report.
- Compare total implementation + review + repair + rescue cost per accepted
  task, including failed attempts. Use actual billed cost when available;
  otherwise distinguish token/latency proxies and API-price estimates from
  session billing. Missing usage means cost is unknown, not proven lower.
- Stop after these four tasks and select a provisional routing policy by task
  class. A small unmatched sample is operational evidence, not a causal model
  ranking. Expand only if one named uncertainty could change the selection and
  a further useful task can resolve it cheaply. Do not delay product progress
  for a statistically strong benchmark.
- Adopt Luna for a tested class only when required gates pass, review reveals
  no unresolved material defects and observed total effort/cost supports it.
  Retain Astra low for classes where Luna needs substantial rescue or design
  repair. One material trust/data/lifecycle violation stops expansion into that
  class pending review; passing local tasks does not establish suitability for
  the scheduler or cross-process architecture. Keep medium review and the same
  acceptance gates for either implementation model.

## Blockers

- No blocker to M0 documentation/read-only work.
- Real mixed-workflow model/runtime/hardware and desktop-runner availability
  have not been verified in this planning pass; M0 owns the check.
- Full consumer/target inventory and project licensing authority remain to be
  reconciled; unresolved external authority blocks only dependent claims.
- Exact production write sets are intentionally not admitted before source-backed
  findings. Later milestones cannot use candidate owner lists as write permission.

## Re-Plan Triggers

Change the current decision when the user selects a different modality contract;
source evidence contradicts an owner/ADR; a real external consumer or persisted
promise changes migration needs; a missing Pumas contract changes sequencing;
required execution evidence is unavailable; or new machinery/change propagation
invalidates the composed-design review. Re-run only affected review and gates.

## Implementation Invocation

`Continue docs/plans/domain-architecture-and-multimodal/plan.md, operation continue.
Admit D-02's complete retained-image retrieval repair from the existing desktop
report after confirming its exact consumer write set; preserve artifact identity,
retention and cancellation semantics. Then qualify concrete owner-fresh Pumas
text/image targets, runtime/device and the existing desktop fixture before real
mixed-workflow execution. EX-04's controlled text→image, pending-response and
readiness-recovery evidence is accepted, not a real-model/desktop substitute.
Continue the four-model policy and record full implementation, review, rescue,
commit/coordination and unknown-rate costs. Full audit/static and existing test
failures remain tracked independently.

Subsequent invocations supply this same canonical plan path with the operation
appropriate to its recorded lifecycle; the next-slice field is not independent
execution authority. The user has now authorized starting the plan with subagents
and the bounded implementation pilot.

## UniFFI Validation Publication Bridge (2026-10-03)

Hosted headless qualification now reaches the UniFFI runtime tests. The FFI
constructor owns an ephemeral attribution store, and execution correctly requires
an owner-published executable validation snapshot. The embedding API exposes
edit sessions but omits validation refresh and snapshot publication, so a valid
text graph cannot currently complete the public create/run flow.

Accepted bounded repair: expose typed EmbeddedRuntime delegates and strict JSON
UniFFI entrypoints for the existing workflow-service current-validation refresh
and graph-session snapshot publication methods. Keep graph/proof construction,
revision checks, descriptor compatibility and dependency-proof freshness at their
existing owners. A client supplies session/version identifiers, never trusted
snapshot content. It loads/saves a graph, creates an edit session, refreshes the
returned graph revision, publishes its current executable validation session,
then creates/runs an execution session. Reopening the ephemeral FFI runtime
requires publication again. Missing, stale and unavailable validation still fail
closed; runtime inference facts and readiness are not fabricated.

Required evidence: actual public JSON text execution; missing/stale/reopened
runtime rejection; existing error-envelope fidelity; UniFFI metadata and generated
C# surface checks. Independent read-only review is required before publication.
Canonical selector fixtures and managed-runtime readiness evidence stay separate.

Independent review found that inference-only snapshots rejected every legitimate
zero-inference graph and that real C# callers had not been migrated. The accepted
correction keeps publication mandatory and checks exact canonical inference-node
coverage at owner publication and executable projection, including imported
records. Empty coverage is valid only for a graph with no inference nodes. Missing,
extra and duplicate runtime projections fail closed; per-node proofs and existing
freshness checks remain. The canonical inference request owner supplies the shared
classification; unsupported inference-bearing task types remain rejected. Both
NativeSmoke and the packaged DirectRuntimeQuickstart must perform the real public
load/edit/refresh/publish lifecycle before execution.

## Final Acceptance

- Acceptance status: `blocked`
- Deferred follow-ups: new model families and live generated-UI execution outside the selected product contract; release publication.
- Final status: `Active`
