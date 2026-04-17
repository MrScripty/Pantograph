# Plan: Pantograph Phase 7 Runtime Adapter Unification

## Status
Active

Last updated: 2026-04-17

## Current Source-of-Truth Summary

This document is the dedicated source of truth for Phase 7, Runtime Adapter
Unification. It expands the short Phase 7 section in
`ROADMAP-pantograph-workflow-graph-scheduling-runtime.md` into a
standards-reviewed implementation plan for finishing runtime producer
convergence without reopening the backend ownership decisions already closed by
runtime-registry Milestones 1 through 6.

Phase 7 planning and status should now be updated here first. The roadmap
remains the cross-target summary, while milestone sequencing, immediate
standards refactors, and acceptance gates for the remaining producer-
convergence work are tracked in this dedicated plan.

The accurate implementation baseline at the current checkpoint is:

- backend-owned runtime identity normalization is already in place and is
  consumed by Tauri and embedded-runtime producer paths rather than re-derived
  separately per adapter
- backend-owned runtime health assessment and lifecycle-to-registry status
  translation already exist for gateway and dedicated embedding producers
- backend-owned runtime-registry reconciliation already handles execution-path
  runtime snapshot overrides and preserves matching unhealthy runtime state
  instead of silently downgrading it back to lifecycle-ready
- Tauri host lifecycle commands, recovery, and RAG flows already consume shared
  backend registry sync and embedding-endpoint refresh helpers rather than
  owning those policy rules locally
- direct embedded and headless workflow execution already reconcile Python-
  sidecar observations into the shared runtime registry, but this producer
  family is not yet fully converged with gateway and dedicated embedding health
  and capability behavior
- the remaining roadmap gaps are still the three explicit Phase 7 items:
  - full health-check, reconnect, and degraded-state hardening for runtime
    producers beyond gateway plus dedicated embedding
  - one shared registry-ready capability contract family across all runtime
    producers
  - broader backend-owned runtime-registry boundary coverage for producer
    observation and restore/reconciliation paths beyond gateway plus dedicated
    embedding

The Milestone 1 inventory frozen for implementation is:

- active host runtime producer:
  `HostRuntimeModeSnapshot.active_runtime` plus the normalized
  backend/runtime identity used for the live gateway-managed runtime
- dedicated embedding producer:
  `HostRuntimeModeSnapshot.embedding_runtime` plus the dedicated embedding
  runtime capability and health overlays already routed through backend Rust
- execution-observed Python-sidecar producer family:
  runtime snapshot overrides and execution diagnostics emitted for
  Python-backed runtimes such as `pytorch`, `diffusers`, `onnx-runtime`, and
  `stable_audio`
- external-capability-only runtime family:
  runtime capability entries that must stay in the same backend-owned
  capability contract family even when they do not participate in host-owned
  polling or registry reconciliation

The target contract family frozen in Milestone 1 is:

- registry observations continue to converge on backend-owned
  `pantograph_runtime_registry::RuntimeObservation` with canonical runtime id,
  display name, backend keys, optional model id, registry status,
  runtime-instance identity, and last-error semantics
- producer health overlays continue to converge on backend-owned
  `RuntimeHealthAssessment` and `RuntimeHealthAssessmentSnapshot` semantics for
  degraded versus unhealthy progression
- workflow-facing runtime capability publication continues to converge on
  backend-owned `WorkflowRuntimeCapability` fields rather than adapter-local
  capability variants, including canonical `runtime_id`, `backend_keys`,
  `source_kind`, readiness/install semantics, and stable unavailable reasons

## Objective

Complete Phase 7 by making every runtime producer that affects workflow
execution, scheduling, preflight, and diagnostics converge on backend-owned
Rust contracts for health, degraded state, reconnect behavior, capability
publication, and runtime-registry reconciliation, while refactoring the
immediate insertion areas so the resulting code and nearby codebase
surroundings stay compliant with the coding, architecture, concurrency,
interop, and documentation standards.

## Scope

### In Scope

- remaining Phase 7 producer-convergence work for runtime health, degraded
  state, reconnect handling, capability publication, and registry
  reconciliation
- refactors required to keep the immediate runtime insertion areas standards
  compliant before more Phase 7 logic lands
- backend-owned contract extraction or consolidation for runtime producer
  observations and registry-ready capability payloads
- additive transport and binding changes only where backend-owned Rust
  contracts must cross Tauri, UniFFI, or other wrappers
- README, roadmap, and plan updates required by the touched source
  directories and runtime boundaries
- regression and acceptance coverage for producer health, recovery, restore,
  and capability parity behavior

### Out of Scope

- new frontend-owned runtime policy or TypeScript-side runtime truth
- reopening completed runtime-registry ownership boundaries or moving business
  logic back into Tauri
- scheduler policy changes except where scheduler-facing runtime contracts must
  remain aligned with Phase 7 outputs
- KV cache behavior, graph invalidation policy, or workflow event vocabulary
  expansion beyond runtime-contract compatibility points
- new distributed runtime coordination or remote fleet orchestration

## Inputs

### Problem

Pantograph has already moved most runtime-registry policy, producer mapping,
and diagnostics shaping behind backend-owned Rust boundaries, but the remaining
Phase 7 producer paths are still not fully converged. Gateway and dedicated
embedding are materially more complete than the execution-observed producer
paths, especially for health/degraded handling, reconnect semantics, registry-
ready capability publication, and post-restore reconciliation.

If the remaining work is implemented opportunistically without a dedicated
standards-reviewed plan, the likely regressions are:

- more producer-specific state handling leaking back into `src-tauri`
- capability semantics drifting across gateway, dedicated embedding, Python
  sidecar, and other execution-observed producers
- restore/recovery and runtime-registry synchronization being hardened for one
  producer family but not another
- additional logic being appended into already oversized runtime files without
  decomposition review or extraction

### Constraints

- backend Rust remains the owner of runtime producer policy, lifecycle
  assessment, registry reconciliation, and machine-consumable runtime facts
- Tauri remains a composition root and transport host only; it may poll, call
  backend helpers, and forward backend-owned contracts, but it must not become
  the owner of runtime health, reconnect, restore, or capability policy
- `pantograph-runtime-registry` remains the owner of runtime-registry state
  and status semantics
- `pantograph-embedded-runtime` remains the preferred owner for producer-aware
  runtime translation, execution-path reconciliation, and adapter-consumed
  runtime helper logic
- new Phase 7 work must preserve public facades by default and prefer additive
  contract changes over broad adapter-visible breakage
- every touched source directory must satisfy the README requirements in
  `DOCUMENTATION-STANDARDS.md`
- Phase 7 must explicitly address nearby non-compliance when new work would
  otherwise deepen it

### Public Facade Preservation Note

Phase 7 is a facade-first refactor. The default path is to preserve existing
runtime command and workflow-service facades while moving the remaining runtime
producer logic behind backend-owned helpers and additive contract fields.
Breaking transport changes are out of scope unless a re-plan records the
compatibility impact explicitly.

### Assumptions

- the existing runtime-registry ownership and diagnostics close-out from
  runtime-registry Milestones 5 and 6 remains the frozen boundary for this
  phase
- gateway and dedicated-embedding health/reconciliation behavior are the
  baseline semantics the remaining producer families should converge to, not a
  temporary special case to preserve forever
- direct embedded/headless workflow execution remains the best place to close
  non-gateway producer coverage gaps because it already consumes backend-owned
  runtime helper crates
- no new persisted runtime store is required to finish Phase 7
- the current Tauri wrappers can stay thin if the missing producer policy is
  pushed into backend crates instead of expanded in place

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
- `IMPLEMENTATION-PLAN-pantograph-milestone-5-workflow-adapter-integration.md`
- `IMPLEMENTATION-PLAN-pantograph-milestone-6-diagnostics-documentation-rollout-safety.md`
- `crates/pantograph-embedded-runtime`
- `crates/pantograph-runtime-registry`
- `crates/pantograph-runtime-identity`
- `crates/inference`
- `src-tauri/src/llm`
- `src-tauri/src/workflow`
- standards under
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Producer convergence work leaks new policy back into Tauri because it is the easiest insertion point | High | Treat backend extraction as a prerequisite, keep Tauri changes transport-only, and stop to re-plan if a slice requires Tauri-owned runtime state. |
| Oversized runtime files absorb more behavior before decomposition happens | High | Make extraction or explicit decomposition review part of Milestone 1 and Milestone 2 instead of deferring it to close-out. |
| Producer families converge only partially, leaving scheduler/preflight/diagnostics to infer different semantics | High | Freeze the shared capability contract family in Milestone 3 and add parity tests across producer families. |
| Restore, recovery, and execution-path registry updates race and overwrite unhealthy state or producer identity | High | Centralize reconciliation in backend-owned helpers and add targeted overlap tests in Milestone 4. |
| README and source-of-truth drift leaves the runtime boundary hard to audit after implementation | Medium | Keep README and roadmap reconciliation as explicit Milestone 5 tasks and update this plan first when scope changes. |

## Clarifying Questions (Only If Needed)

- None at this time.

### Affected Structured Contracts

- runtime producer lifecycle snapshots and runtime observation translation
- runtime health assessment snapshots and degraded/unhealthy reason semantics
- registry-ready runtime capability payloads published to workflow and
  diagnostics consumers
- runtime-registry snapshot, restore, and reclaim/reconciliation behavior
- Tauri and binding-layer transport payloads that forward backend-owned runtime
  capability or health facts

### Affected Persisted Artifacts

- this dedicated Phase 7 plan
- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- README files in touched runtime source directories
- existing ADRs only if implementation reveals a new long-lived ownership or
  compatibility decision that is not already covered

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate Phase 7 surroundings are not fully compliant today and must be
included in the plan rather than ignored:

- `crates/pantograph-runtime-identity/src/` is a source directory without the
  required `README.md`
- `crates/pantograph-embedded-runtime/src/task_executor.rs` is far above the
  500-line decomposition threshold and currently mixes multiple responsibilities
  around workflow execution and runtime observation handling
- `crates/pantograph-embedded-runtime/src/workflow_runtime.rs` is above the
  decomposition threshold and now carries multiple runtime projection and
  capability concerns
- `crates/pantograph-embedded-runtime/src/runtime_registry.rs` is above the
  decomposition threshold and is the primary insertion area for producer-
  convergence logic
- `src-tauri/src/llm/health_monitor.rs` and
  `src-tauri/src/llm/runtime_registry.rs` are both above the decomposition
  threshold; they should not accumulate new backend-owned policy

Phase 7 therefore must include extraction and README work in the immediate
surroundings before or alongside the remaining producer-convergence slices.

### Concurrency / Race-Risk Review

- runtime health polling, recovery restart loops, restore paths, workflow
  execution, and runtime-registry synchronization can overlap in time
- each producer family must have one backend-owned source of truth for health,
  degraded-state progression, reconnect sequencing, and registry reconciliation
- Tauri polling loops may gather probe inputs and emit events, but they must
  not race with backend state by maintaining a second lifecycle state machine
- restore and reclaim flows must synchronize registry state after lifecycle
  transitions using backend-owned helpers rather than ad hoc adapter-local
  sequencing
- execution-path runtime observations must not overwrite backend-owned
  unhealthy state or erase producer identity when a matching runtime instance
  is already known
- any newly extracted async helpers must define who starts background work, who
  stops it, and how overlap or duplicate retries are prevented

### Ownership And Lifecycle Note

- `pantograph-runtime-registry` owns registry state, observation admission, and
  final status semantics
- `pantograph-embedded-runtime` owns producer-aware observation translation,
  execution-path reconciliation, backend-ready capability shaping, and shared
  recovery/reconnect helper logic consumed by adapters
- `crates/inference` owns runtime start/stop/reuse mechanics for managed
  runtimes where those mechanics already live there
- `src-tauri/src/llm` owns polling cadence, HTTP transport, command wiring, and
  event emission, but not producer policy or runtime truth
- `src-tauri/src/workflow` owns host composition and transport for workflow
  execution, but not runtime-registry policy or capability derivation

### Milestone 1 Decomposition Review

The immediate insertion-area decomposition decisions frozen before Milestones 2
through 4 are:

- `crates/pantograph-embedded-runtime/src/runtime_registry.rs` should be split
  by responsibility before more Phase 7 behavior lands there:
  observation builders and health overlays separate from sync/reclaim/restore
  orchestration and execution-override reconciliation
- `crates/pantograph-embedded-runtime/src/workflow_runtime.rs` should be split
  into runtime diagnostics projection, runtime event projection, and capability
  fallback helpers rather than continuing as one mixed projection module
- `crates/pantograph-embedded-runtime/src/task_executor.rs` should keep a thin
  facade and move Python-sidecar execution, RAG execution, and shared
  execution-context helpers into narrower modules before producer-convergence
  changes add more behavior
- `src-tauri/src/llm/health_monitor.rs` and
  `src-tauri/src/llm/runtime_registry.rs` may receive transport-only edits, but
  any new backend-owned producer policy must be extracted into backend crates
  instead of being added there

## Definition of Done

- all runtime producers that materially affect workflow execution publish one
  backend-owned registry-ready capability contract family
- all runtime producers that remain in scope for local execution share backend-
  owned health, degraded-state, and reconnect assessment semantics
- backend-owned registry reconciliation covers the remaining restore and
  execution-observed producer paths that still bypass the shared boundary today
- no new backend-owned runtime policy is added to Tauri or other binding
  layers
- touched runtime source directories satisfy README requirements
- the immediate oversized insertion areas have either been decomposed or had an
  explicit extraction review reflected in the implementation slices
- roadmap, plan, README, and any needed ADR references are consistent at
  close-out

## Milestones

### Milestone 1: Freeze Phase 7 Boundaries And Surrounding Compliance

**Goal:** Turn Phase 7 into a dedicated source of truth and fix the immediate
standards blockers before more producer logic lands.

**Tasks:**
- [x] Keep this file and the roadmap aligned, with this file as the detailed
      source of truth for Phase 7 sequencing.
- [x] Add the missing `crates/pantograph-runtime-identity/src/README.md`
      before Milestones 2 through 4 rely on that source directory as part of
      the runtime identity contract.
- [x] Inventory the exact runtime producer families still in scope for Phase 7
      and freeze the target contract family they must converge on.
- [x] Perform decomposition review for the oversized insertion areas and decide
      which responsibilities must move out before Milestones 2 through 4 add
      more code.
- [x] Confirm that remaining Phase 7 work stays backend-owned and facade-first
      rather than expanding adapter-local state machines.

**Verification:**
- Manual standards review against `PLAN-STANDARDS.md`,
  `CODING-STANDARDS.md`, and `ARCHITECTURE-PATTERNS.md`
- Direct read-through of roadmap and this plan to confirm source-of-truth
  alignment

**Status:** Complete

### Milestone 2: Converge Producer Health, Reconnect, And Degraded Semantics

**Goal:** Make remaining runtime producers use one backend-owned health and
reconnect policy family instead of a gateway-only special case.

**Tasks:**
- [ ] Extract or consolidate backend-owned producer health assessment helpers in
      `pantograph-embedded-runtime` and related runtime crates so Tauri health
      polling stays transport-only.
- [ ] Extend the shared health/degraded contract to the remaining execution-
      observed producer families, including Python-sidecar-style runtime paths
      that currently rely more on execution snapshots than on unified health
      overlays.
- [ ] Ensure reconnect and degraded-state progression rules are expressed once
      in backend-owned code and reused by recovery, restore, and health-monitor
      callers.
- [ ] Refactor the immediate oversized runtime health and registry insertion
      areas as needed so new policy does not deepen the existing file-size and
      mixed-responsibility debt.

**Verification:**
- Targeted Rust unit tests for health assessment, degraded/unhealthy
  progression, and reconnect sequencing
- `cargo check` and focused runtime-registry or embedded-runtime test slices
  for touched crates

**Status:** In progress

### Milestone 3: Unify The Registry-Ready Capability Contract Family

**Goal:** Publish one backend-owned capability vocabulary across gateway,
dedicated embedding, and execution-observed runtime producers.

**Tasks:**
- [ ] Freeze or extend a single backend-owned capability contract family that
      covers canonical runtime id, backend requirements, producer identity,
      readiness state, and degraded-state semantics.
- [ ] Remove any remaining adapter-local capability shaping that duplicates or
      diverges from backend-owned runtime capability builders.
- [ ] Keep transport and binding updates additive so Tauri, UniFFI, and other
      wrappers forward the same backend-owned semantics rather than inferring
      new ones locally.
- [ ] Add coverage that compares producer families against the same capability
      expectations for scheduler, preflight, and diagnostics consumers.

**Verification:**
- Contract-focused Rust tests in backend crates plus any thin transport tests
  needed at boundaries
- Manual interop review against `INTEROP-STANDARDS.md` and
  `LANGUAGE-BINDINGS-STANDARDS.md` for additive payload compatibility

**Status:** In progress

### Milestone 4: Broaden Backend-Owned Registry Boundary Coverage

**Goal:** Finish registry reconciliation and restore coverage for the remaining
non-gateway producer paths.

**Tasks:**
- [ ] Move remaining producer observation translation and post-restore
      reconciliation rules into backend-owned runtime helpers where Tauri still
      owns too much sequencing.
- [ ] Ensure execution-path runtime observations preserve producer identity,
      unhealthy state, and runtime-instance matching across restore/recovery
      transitions.
- [ ] Close remaining direct-host or workflow-execution paths that still update
      registry state through adapter-local logic instead of the shared backend
      registry boundary.
- [ ] Add deterministic tests for restore, reclaim, recovery restart, and
      overlapping execution-observation scenarios that previously differed by
      producer family.

**Verification:**
- Focused runtime-registry and embedded-runtime tests for restore/reclaim/
  recovery sequencing
- `cargo check` for all touched crates and Tauri transport modules

**Status:** Not started

### Milestone 5: Documentation, Acceptance, And Source-Of-Truth Reconciliation

**Goal:** Close Phase 7 with standards-compliant documentation and acceptance
coverage.

**Tasks:**
- [ ] Update touched README files so the runtime identity, registry, workflow,
      and Tauri adapter boundaries describe the final Phase 7 ownership model.
- [ ] Add or update an ADR only if Phase 7 accepts a new long-lived runtime
      ownership or compatibility consequence that is not already covered.
- [ ] Reconcile this plan, the roadmap, and any touched runtime README files so
      Phase 7 completion status and remaining follow-ups are consistent.
- [ ] Confirm no unnecessary new dependencies were introduced for Phase 7; if a
      new dependency is justified, document the reason at the ownership
      boundary that actually uses it.

**Verification:**
- Manual documentation pass against `DOCUMENTATION-STANDARDS.md`
- Acceptance review of roadmap, README, and plan alignment plus targeted test
  summary from Milestones 2 through 4

**Status:** Not started

## Standards Review Passes

- Pass 1, plan structure plus architecture/coding review:
  - Checked against `PLAN-STANDARDS.md`, `CODING-STANDARDS.md`, and
    `ARCHITECTURE-PATTERNS.md`.
  - Corrections applied to the draft:
    - added explicit scope, constraints, affected contracts/artifacts, risks,
      re-plan triggers, and completion criteria
    - added public-facade preservation guidance so Phase 7 stays additive by
      default
    - pulled immediate surrounding non-compliance into scope instead of leaving
      oversized runtime files and the missing `crates/pantograph-runtime-
      identity/src/README.md` as untracked debt
- Pass 2, concurrency plus interop review:
  - Checked against `CONCURRENCY-STANDARDS.md`,
    `INTEROP-STANDARDS.md`, and `LANGUAGE-BINDINGS-STANDARDS.md`.
  - Corrections applied to the draft:
    - added single-owner lifecycle and race-risk rules for polling, restore,
      recovery, and registry reconciliation
    - explicitly prohibited Tauri or bindings from owning producer policy or a
      second runtime state machine
    - required additive transport updates so cross-boundary capability and
      health payloads stay backend-owned and compatible
- Pass 3, documentation plus dependency review:
  - Checked against `DOCUMENTATION-STANDARDS.md` and
    `DEPENDENCY-STANDARDS.md`.
  - Corrections applied to the draft:
    - added README compliance as a done criterion and milestone task
    - limited ADR work to cases where Phase 7 accepts a truly new long-lived
      architectural consequence
    - added an explicit no-unnecessary-dependencies rule tied to the package or
      crate that actually owns the runtime logic

## Execution Notes

Update during implementation:
- 2026-04-17: Dedicated Phase 7 draft created after reconciling the roadmap's
  remaining runtime-adapter gaps with the current runtime-registry and
  producer-convergence baseline.
- 2026-04-17: First standards pass pulled architecture, decomposition, and
  immediate surrounding-compliance work into the plan instead of leaving them
  as informal follow-ups.
- 2026-04-17: Second standards pass tightened lifecycle ownership and interop
  requirements so polling, recovery, restore, and binding updates cannot split
  runtime truth across backend and adapters.
- 2026-04-17: Third standards pass added README/ADR/dependency closure rules so
  Phase 7 implementation leaves the touched runtime boundaries auditable.
- 2026-04-17: Milestone 1 source-of-truth freeze now records the producer
  families still in scope, freezes the shared registry and capability contract
  family, and captures the decomposition decisions required before more Phase 7
  runtime behavior lands.
- 2026-04-17: Added the missing `crates/pantograph-runtime-identity/src/README.md`
  so the shared runtime-identity boundary meets documentation standards before
  the next extraction and producer-convergence slices.
- 2026-04-17: Began the first Milestone 2 backend extraction by moving
  runtime-registry observation builders and health-overlay matching into a
  dedicated embedded-runtime module so later producer-convergence changes can
  land behind backend Rust boundaries instead of expanding Tauri transport
  files.
- 2026-04-17: Continued the Milestone 2 decomposition pass by moving Python
  runtime execution metadata and recorder ownership out of `task_executor.rs`
  into a dedicated backend module, keeping execution-observed producer facts
  reusable by diagnostics and registry code without expanding the executor
  facade.
- 2026-04-17: Extended the execution-observed Python runtime path so failed
  Python-sidecar executions now carry a backend-owned unhealthy assessment into
  registry reconciliation instead of relying on lifecycle snapshot errors
  alone.
- 2026-04-17: Moved Python-sidecar runtime capability construction into the
  shared backend capability helper module so the execution-observed producer
  family no longer depends on `EmbeddedWorkflowHost`-local capability shaping.
- 2026-04-17: Moved capability-driven lifecycle fallback selection into the
  shared backend capability helper module so diagnostics and workflow fallback
  paths reuse the same backend-owned capability contract instead of
  `workflow_runtime.rs`-local selection logic.

## Commit Cadence Notes

- Commit after each completed logical Phase 7 slice or milestone-level refactor
  once the relevant focused verification passes.
- Keep code and documentation changes for the same Phase 7 slice together in
  one atomic commit.
- Follow `COMMIT-STANDARDS.md` for message structure and history cleanup.

## Optional Subagent Assignment

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| None | None | None | None |

## Re-Plan Triggers

- a remaining runtime producer is found to require a new ownership boundary not
  covered by the current runtime-registry and embedded-runtime crates
- capability contract changes would require a breaking transport or binding
  change instead of an additive one
- restore/recovery hardening reveals a new persisted state or replay
  requirement
- decomposition of the oversized insertion areas materially changes the
  milestone dependency order

## Recommendations

- Recommendation 1: Start Phase 7 with the smallest backend extraction that
  removes new producer policy from the oversized Tauri runtime files. This
  reduces boundary drift risk before any additional behavior changes land.
- Recommendation 2: Treat gateway plus dedicated-embedding behavior as the
  contract baseline and migrate the remaining producer families toward it,
  instead of inventing a second producer-specific contract family.
- Recommendation 3: Close registry-boundary coverage before polishing
  diagnostics wording, because restore/recovery sequencing changes can still
  alter what the diagnostics surfaces are allowed to claim.

## Completion Summary

### Completed

- Dedicated Phase 7 source-of-truth plan created.
- Multiple standards-review passes completed and corrections folded into the
  plan.
- Milestone 1 source-of-truth freeze completed for roadmap alignment, producer
  inventory, contract-family freeze, and decomposition review.
- Milestone 1 surrounding-compliance closure completed with the missing
  runtime-identity source README.
- Milestone 2 is now in progress with the first embedded-runtime extraction
  slice for runtime-registry observation building.

### Deviations

- None yet. Implementation has not started from this plan.

### Follow-Ups

- Begin the first backend extraction slice from the recorded decomposition
  review before Milestone 2 producer-policy changes land.

### Verification Summary

- Manual roadmap and standards review completed during planning.

### Traceability Links

- Module README updated: N/A during planning; required in Milestones 1 and 5
- ADR added/updated: N/A during planning; only if implementation reveals a new
  long-lived decision
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A during
  planning

## Brevity Note

This plan is intentionally detailed because Phase 7 spans multiple crates,
runtime producers, and standards categories. Implementation updates should stay
concise and use this file as the detailed source of truth instead of
duplicating milestone reasoning elsewhere.
