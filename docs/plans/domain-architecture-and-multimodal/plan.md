# Plan: Domain Architecture And End-To-End Multimodal Workflows

**Plan status:** `Active`

**Current phase:** M1 review remains open; M2 selected text and dependent runtime continuation accepted under controlled integration; M3 real-model/desktop qualification pending; M4 ongoing complete-cost model evaluation.

**Next slice:** D-02 complete retained-image retrieval in I/O Inspector, then qualify the real model/runtime/device and desktop fixture for the dependent text→image workflow. EX-04 now proves scheduler continuation, retained outputs and readiness recovery through controlled canonical integration. Full audit, baseline failures and required-real acceptance remain open.

**Acceptance status:** `blocked`

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
substantive independent review. These are the user's current routing choices,
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
| DA-03 | One real desktop-submitted mixed graph runs real text and image models, passes generated text along its edge, and exposes both retained outputs with correct run/task identity. | user-workflow | required-real | automated | blocked | EX-04 controlled integration accepted; D-02 and real fixture/runtime prerequisites remain |
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

### M3 — Prove the desktop mixed workflow

**Goal:** The user authors/submits the graph and receives text and image results.

**Allowed write set:** plan reports/control files until M2 declares the exact
desktop transport, graph, result-projection and existing GUI-fixture files.

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

## Final Acceptance

- Acceptance status: `blocked`
- Deferred follow-ups: new model families and live generated-UI execution outside the selected product contract; release publication.
- Final status: `Active`
