# Current Image Generation Implementation Recovery Sequence

## Status

**Active recovery overlay:** Approved for execution.

This document is the authoritative next-step sequence for the current image
generation plan until Milestone 0 below reconciles the older milestone status
and conflicting grouped-only dispatch text. Historical execution evidence
remains in the existing plan files; this document does not retroactively
rewrite that history.

## Objective

Restore a trustworthy implementation baseline and complete one real,
user-visible image-generation run through the workflow editor:

```text
workflow editor
  -> backend validation and submission
  -> durable scheduler task orchestration
  -> scheduler-selected runtime and device reservation
  -> worker-owned runtime-host dispatch
  -> real PyTorch/Diffusers inference
  -> retained backend artifact
  -> I/O Inspector presentation
```

The recovery must preserve canonical backend ownership and remove, rather than
mask, stale behavior. A valid runtime task may dispatch as a scheduler-selected
group of one or more compatible assignments. A group of one is not a fallback;
it is the bounded canonical decision when no additional compatible assignment
arrives during the collection window.

## Scope

### In Scope

- Reconcile current plan status, active decisions, and verification evidence.
- Repair backend-owned inference validation for the real saved workflow.
- Replace the grouped-only terminal policy with canonical non-empty `1..N`
  scheduler dispatch.
- Restore the active workflow-service and embedded-runtime test baseline.
- Complete Milestone 9 with a real workflow-editor image-generation run and a
  retained UI-visible artifact.
- Record discovered defects and deferred work without expanding the active
  slice.

### Out of Scope

- Compatibility shims for retired graph, runtime, device, scheduler, worker,
  or attribution behavior.
- Request-scoped or direct runtime execution.
- Frontend- or Tauri-owned validation, scheduling, runtime, device, model,
  retention, or artifact policy.
- Reintroducing attribution schema version 7.
- Carrying workflow inputs, run state, or ownership across runtime reuse.
- Broad scheduler optimization beyond the behavior needed to prove canonical
  one-run and compatible multi-run execution.

## Inputs

### Current State

Repository review on 2026-07-28 found:

- The current workflow smoke script passes.
- The frontend test suite passes with 456 tests.
- `pantograph-workflow-service` has 20 failing library tests. Failures cluster
  around production inference validation, technical-fit preflight, scheduler
  source-input readiness, and grouped batch expiry.
- `pantograph-embedded-runtime` has three failing library tests. Two are
  production validation/admission failures and one is a real execution test
  ending in `BatchWindowExpired`.
- The GUI harness launches the real desktop app with a fresh attribution
  schema 8 project root, loads the saved workflow, and stops at the backend
  submit gate with `Inference validation has blocking diagnostics`.
- Milestone 9 remains open because no configured workflow-editor run has yet
  produced a retained image visible through the app.
- Older plan sections intentionally expire solo assignments without dispatch.
  That decision conflicts with the approved end state in this document.
- Milestone status text is stale in several files and no longer provides a
  reliable current-state dashboard.

### Constraints

- Backend services are authoritative for graph validity, resolved inference
  interfaces, diagnostics, task state, scheduler policy, runtime/device
  selection, reservations, execution, and artifact retention.
- Svelte/TypeScript owns UI projection and interaction only.
- Tauri owns transport and composition bridging only; it must not own business
  rules.
- The scheduler owns collection-window policy and decides when a compatible
  non-empty assignment group is dispatched.
- The runtime host executes the scheduler handoff it receives. It does not
  choose scheduler policy or recover missing scheduler facts.
- Runtime residency is reusable across workflow runs but carries no workflow
  input or run-local state forward.
- A workflow may request persistence but does not own the persistent runtime.
- Physical compute devices and their capacity are tracked separately from
  runtime instances. A reservation may span devices, and a device may host
  multiple runtimes when capacity allows.
- Invalid or incomplete canonical facts produce typed diagnostics. They do not
  select a fallback.

### Dependencies

- A Pumas package and artifact that expose complete image-generation facts.
- A usable Python/PyTorch/Diffusers runtime for the real smoke.
- Backend resource estimates and runtime/device inventory sufficient for
  scheduler admission.
- The existing schema 8 attribution store and retained artifact contracts.
- The existing GUI WebdriverIO and `tauri-driver` harness.

## Architecture Decisions

### Canonical Batch-Or-Solo Dispatch

The scheduler uses one dispatch mechanism for every runtime assignment:

1. A ready assignment enters a scheduler-owned bounded collection window.
2. Compatible assignments may join according to scheduler batching policy.
3. At an earlier readiness decision or at window expiry, the scheduler claims
   every compatible assignment currently selected.
4. The claimed group must be non-empty and may contain `1..N` members.
5. The worker sends that group through the same runtime-host batch envelope,
   reservation lifecycle, task-attempt state, diagnostics, cancellation, and
   result fan-out path.

This replaces the grouped-only rule. It does not restore a singleton helper,
direct runtime call, request-scoped executor, or secondary success path.
`BatchWindowExpired` remains valid only when policy cannot dispatch any
assignment, not merely because a valid group contains one member.

### Runtime Reuse And Isolation

- Runtime identity, model identity, and device placement remain separate
  concepts linked by scheduler decisions and reservations.
- A loaded runtime may serve multiple workflow runs.
- Active task/run usage is tracked through reservations and task attempts, not
  workflow ownership of the runtime.
- Persistence changes unload eligibility only. It does not preserve prompts,
  tensors, graph inputs, output targets, cancellation tokens, or any other
  run-local state.

### Validation Ownership

One backend-owned inference descriptor and validation path must govern:

- graph editor ports and options;
- draft and executable validation;
- scheduler task materialization and admission;
- pre-dispatch runtime validation.

The frontend renders typed projections. Tauri forwards commands and responses.
Neither layer may duplicate validation rules or parse diagnostic messages to
make execution decisions.

## Simplicity And Complection Review

- Independent concepts: graph validity, model facts, runtime residency,
  physical devices, reservations, batching policy, task attempts, run-local
  inputs, artifact retention, and UI projection.
- Intentionally coupled: a scheduler dispatch decision references a task
  attempt, selected runtime, selected device reservation, and non-empty member
  group.
- Accidental coupling to remove: batch eligibility with minimum cardinality
  two; runtime persistence with workflow ownership; runtime reuse with input
  reuse; Tauri transport with backend policy; UI availability with inferred
  diagnostics.
- Ownership: workflow service owns domain validation and orchestration;
  scheduler owns admission, collection, placement, and reservation policy;
  runtime registry owns live residency state; runtime host owns execution;
  artifact store owns retained bodies; frontend owns projection; Tauri owns
  transport/composition.
- Future batching, fairness, eviction, or placement policy changes should not
  require changes to workflow graph contracts, UI policy, or runtime-host
  inference semantics.

## Definition Of Done

- Plan entry points show one accurate active sequence and current status.
- The canonical saved image workflow passes backend executable validation or
  returns exact typed diagnostics for genuinely missing external facts.
- A valid single runtime assignment completes through scheduler-owned `1..N`
  dispatch.
- Multiple compatible assignments can still be grouped, while incompatible
  assignments remain separate.
- Active workflow-service and embedded-runtime library suites are green.
- The real desktop workflow editor can submit the configured workflow,
  complete inference, and navigate to a retained generated image in the I/O
  Inspector.
- Missing model/runtime/device prerequisites remain typed, backend-owned,
  user-visible failures.
- No retired or fallback execution path is reachable.

## Milestones

### Milestone 0: Reconcile The Plan Set

**Goal:** Make the active sequence legible and remove contradictory execution
instructions before source implementation resumes.

**Tasks:**

- [ ] Audit milestone status against committed implementation and current test
  evidence.
- [ ] Mark Milestones 0-4 and 7 according to verified closure evidence.
- [ ] Reconcile Milestones 5, 5a, 5b, 5c, and 5d as implemented, partially
  accepted, or still open instead of relying on stale headings.
- [ ] Mark Milestone 6 as implemented but acceptance-blocked until its
  production validation and runtime tests pass.
- [ ] Keep Milestone 8 explicitly headless/contract complete and Milestone 9
  active for real user readiness.
- [ ] Replace grouped-only/solo-expiry requirements with the canonical `1..N`
  decision in every authoritative active-plan location.
- [ ] Correct ADR traceability and identify superseded historical decisions.
- [ ] Create a concise known-failure inventory with owner, test, diagnostic,
  and intended resolving milestone.
- [ ] Move future detailed chronological evidence to bounded execution reports;
  keep milestone files as checklists and current status, not append-only
  diaries.

**Allowed write set:** plan and report Markdown only. No source, test, fixture,
configuration, generated file, or lockfile changes.

**Verification:**

- Every active milestone has one status and one next action.
- Links and ADR references resolve.
- Searches find no active instruction requiring a valid solo assignment to
  expire solely because no peer arrived.
- `git diff --check`.

**Status:** In progress. This recovery overlay is the first reconciliation
artifact; the older plan files still require the checklist above.

### Milestone 1: Expose And Accept Backend Validation Diagnostics

**Goal:** Make the smallest real saved workflow reach a correct backend
validation decision before changing scheduler behavior.

**Tasks:**

- [ ] Capture the complete typed validation projection for the GUI smoke
  workflow through the existing backend command boundary.
- [ ] Classify every blocking diagnostic by canonical producer and owner.
- [ ] Add a production-composition acceptance test using the saved workflow and
  real model/package-fact projection shape.
- [ ] Fix the smallest canonical producer, resolver, estimator, or projection
  defect responsible for each false blocker.
- [ ] Update test fixtures whose declared purpose is executable success so
  they provide all newly required canonical facts.
- [ ] Preserve genuine missing facts as typed diagnostics and keep submission
  closed.

**Allowed write set:** declare the exact backend owner, focused tests/fixtures,
and this plan before each thin slice. Shared DTOs, saved workflows, generated
bindings, and lockfiles remain serial integration-owner work.

**No-fallback confirmation:** do not weaken executable validation, invent
resource estimates, infer model facts from names or paths, parse UI text for
policy, or add frontend/Tauri eligibility logic.

**Verification:**

- Focused resolver, estimator, validation projection, and production
  composition tests.
- `cargo test -p pantograph-workflow-service --lib`.
- Validation-related `pantograph-embedded-runtime` tests.
- GUI smoke reaches either an executable submit state or an exact external
  prerequisite diagnostic.

**Status:** Not started.

### Milestone 2: Implement Canonical Non-Empty Dispatch

**Goal:** Allow a valid assignment group of one to execute without reducing
the scheduler's ability to batch compatible simultaneous work.

**Tasks:**

- [ ] Verify the runtime-host group envelope can represent non-empty `1..N`
  members; re-plan a shared contract change if it cannot.
- [ ] Change scheduler collection-window expiry/readiness policy to claim all
  selected compatible assignments when at least one is dispatchable.
- [ ] Route one-member and multi-member groups through the same durable claim,
  reservation, worker, runtime-host, cancellation, diagnostic, and result
  fan-out path.
- [ ] Remove or replace diagnostics and state transitions that treat
  cardinality one as an unsatisfied batch.
- [ ] Prove run-local inputs and outputs are reconstructed per member and are
  never inherited from a resident runtime or another run.

**Allowed write set:** scheduler assignment/broker policy, canonical runtime
dispatch owner, focused tests, and plan status. Runtime-host contracts,
generated DTOs, and shared fixtures require an explicitly declared serial
slice.

**No-fallback confirmation:** there is no direct solo executor, singleton
compatibility adapter, request-scoped runtime call, fabricated peer, or second
success path.

**Verification:**

- One valid assignment dispatches after the bounded collection decision.
- Two compatible assignments group and execute together.
- Incompatible assignments dispatch separately.
- Empty groups fail with typed contract diagnostics.
- Timeout, cancellation, shutdown, reservation release, replay, and
  duplicate-dispatch tests remain deterministic.
- `cargo test -p pantograph-workflow-service --lib`.
- `cargo test -p pantograph-embedded-runtime --lib`.

**Status:** Not started.

### Milestone 3: Restore The Active-Lane Baseline

**Goal:** Remove known regressions and stale expectations before treating the
GUI as acceptance evidence.

**Tasks:**

- [ ] Re-run and classify every workflow-service and embedded-runtime failure.
- [ ] Fix active image-generation, validation, scheduler-input, runtime-host,
  reservation, and artifact failures in owner-bounded slices.
- [ ] Update stale tests only when their old expectation contradicts an
  approved canonical contract.
- [ ] Record unrelated failures separately; do not hide active-lane failures
  behind filters.
- [ ] Run the repository's affected lint, format, contract, frontend, and smoke
  gates.

**Allowed write set:** one failure cluster and its focused tests per slice,
plus plan status. Unrelated cleanup stays separate.

**No-fallback confirmation:** tests must assert canonical behavior and typed
diagnostics; no test-only bypass, alternate execution setup, or compatibility
fixture may preserve retired behavior.

**Verification:**

- `cargo test -p pantograph-workflow-service --lib`.
- `cargo test -p pantograph-embedded-runtime --lib`.
- `npm run test:frontend`.
- `npm run typecheck`.
- `npm run lint:full`.
- `node scripts/check-current-image-workflow-smoke.mjs`.
- Applicable runtime-separation and binding/contract checks from
  `docs/testing-and-release-strategy.md`.

**Status:** Not started.

### Milestone 4: Complete Real Workflow-Editor Image Generation

**Goal:** Close Milestone 9 with real desktop, scheduler, worker, runtime, and
artifact evidence.

**Tasks:**

- [ ] Provision and verify the documented Pumas model/artifact, Python,
  PyTorch/Diffusers runtime, and compute-device prerequisites.
- [ ] Run the schema 8 isolated desktop GUI harness against the canonical saved
  workflow.
- [ ] Submit through the workflow editor and observe the durable run/task
  lifecycle through scheduler and worker dispatch.
- [ ] Verify the scheduler selected a runtime and physical-device reservation
  without workflow ownership or input carry-forward.
- [ ] Verify real inference returns one retained image body with canonical
  descriptor and attribution.
- [ ] Verify the frontend navigates to and renders the retained artifact in the
  I/O Inspector.
- [ ] Run missing-prerequisite negative cases and assert typed backend
  diagnostics.
- [ ] Update Milestone 9 and the top-level completion criteria with commands,
  environment facts, artifact evidence, and any residual limitations.

**Allowed write set:** declare each remaining backend, adapter transport,
frontend projection, harness, fixture, and plan slice separately. Tauri changes
may expose backend commands or transport DTOs only.

**No-fallback confirmation:** success must use the real saved workflow, Pumas
facts, scheduler, reservation, worker, runtime host, PyTorch/Diffusers
inference, artifact store, and frontend projection. Direct scripts may verify
prerequisites but cannot satisfy end-to-end acceptance.

**Verification:**

- Focused backend, Tauri transport, frontend projection, and E2E harness tests.
- Full active-lane baseline from Milestone 3.
- Successful configured GUI smoke with retained artifact evidence.
- Fail-closed negative GUI smoke for at least missing model/runtime and
  unsatisfied device constraints.

**Status:** Not started.

### Milestone 5: Close And Compact The Plan

**Goal:** Leave a concise, accurate implementation record after user readiness
is proven.

**Tasks:**

- [ ] Mark all completion criteria with direct verification evidence.
- [ ] Reconcile final milestone statuses and remaining deferred work.
- [ ] Move superseded chronological notes to historical reports without
  changing their meaning.
- [ ] Keep only current architecture decisions and active follow-ups in the
  plan entry points.
- [ ] Confirm README, ADR, module documentation, and generated contract
  traceability.

**Verification:**

- Plan links and status agree across entry points.
- No active unchecked task is described elsewhere as complete.
- No superseded grouped-only or fallback instruction remains authoritative.
- `git diff --check`.

**Status:** Not started.

## Execution Procedure

For every source slice:

1. Inspect `git status` and stop for dirty implementation files outside the
   approved slice.
2. Name the smallest useful vertical behavior and exact allowed write set.
3. State how the slice preserves the no-fallback/no-legacy rule.
4. Implement the owner-bounded behavior.
5. Add or update focused tests and only the fixtures owned by that behavior.
6. Run focused verification, then the milestone gate when the slice closes.
7. Update this document with status, verification, deviations, discovered
   issues, and follow-ups.
8. Review unpushed history for regression/fix pairs.
9. Create one atomic conventional commit.

Do not start a new source slice while the previous slice has uncommitted
implementation changes. Do not mix plan reconciliation, shared contract
changes, generated outputs, lockfiles, or saved workflow fixture changes into a
worker-owned parallel slice.

## Ownership And Lifecycle

- The composition root starts and stops scheduler/worker background tasks.
- The scheduler owns collection deadlines, task claims, reservations,
  dispatch decisions, retries, and terminal/deferred task transitions.
- The worker owns claimed-task processing and runtime-host invocation, but not
  scheduling policy.
- Cancellation and shutdown must release reservations, complete or defer
  responders through durable state, and prevent duplicate dispatch.
- Restart/replay must use durable task-attempt state and must not reconstruct
  authority from frontend, request, or runtime-resident state.

## Risks And Mitigations

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Fixing validation by weakening the gate | High | Require typed diagnostic capture and repair the canonical producer/fixture. |
| Treating one-member dispatch as a legacy fallback | High | Use one non-empty group contract and one scheduler-owned lifecycle for `1..N`. |
| Losing batching across simultaneous runs | High | Keep the bounded collection window and add explicit compatible-pair tests. |
| Runtime reuse leaks prior workflow state | High | Keep inputs/results on task attempts and test cross-run isolation. |
| Device and runtime state become coupled | High | Keep physical inventory, runtime residency, and reservations as separate contracts. |
| GUI work masks backend failures | High | Require green backend suites before final E2E acceptance. |
| Plan history obscures current work | Medium | Maintain this bounded dashboard, then archive historical diaries at closure. |
| External model/runtime setup blocks acceptance | Medium | Verify prerequisites separately and surface exact typed diagnostics. |

## Re-Plan Triggers

Stop before source changes and revise this sequence if:

- The canonical runtime-host request cannot represent non-empty `1..N`
  assignment groups without a shared contract change.
- A valid single assignment cannot dispatch through the same claim,
  reservation, worker, and runtime-host lifecycle as a larger group.
- Validation cannot produce a valid decision from canonical Pumas,
  runtime/device, resource-estimate, and graph facts.
- Runtime reuse requires retaining workflow inputs or granting workflow
  ownership of a runtime.
- Multi-device placement cannot be represented by the current reservation and
  physical-device contracts.
- A fix would move business logic into Tauri or the frontend.
- A required change reaches outside the declared slice, changes the objective,
  or preserves retired behavior.
- An external dependency is missing and cannot be represented as a typed
  prerequisite diagnostic.

## Commit Cadence

- Milestone 0 documentation reconciliation may use one or more documentation
  commits, each with a bounded plan-file write set.
- Source work uses one verified vertical slice per commit.
- Shared contracts, generated DTOs, saved workflow fixtures, lockfiles, ADRs,
  READMEs, and this recovery overlay remain serial integration-owner work.
- Commit messages follow `COMMIT-STANDARDS.md`; verification results stay in
  this plan rather than commit bodies.

## Execution Notes

- 2026-07-28: Created this recovery overlay from repository, plan, and test
  baseline review. Selected sequence: plan reconciliation, backend validation
  acceptance, canonical `1..N` dispatch, active-lane baseline restoration,
  real workflow-editor image generation, and plan compaction.
- 2026-07-28 documentation slice verification: `git diff --check` passed for
  the recovery overlay and both plan entry-point updates. All referenced ADR,
  testing-strategy, and active-milestone files exist. The pre-existing modified
  `PROPOSAL-pumas-artifact-load-target-resolution.md` and untracked
  `PROPOSAL-pumas-library-fast-model-snapshot.md` remained outside the slice
  and were not modified or staged.

## Traceability

- Architecture:
  `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`,
  `docs/adr/ADR-011-scheduler-only-workflow-execution.md`,
  `docs/adr/ADR-013-workflow-version-registry-and-run-snapshots.md`, and
  `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`.
- Active milestone:
  `milestones/09-workflow-editor-e2e-image-generation.md`.
- Testing strategy: `docs/testing-and-release-strategy.md`.
- Historical execution entry point: `plan.md`.
