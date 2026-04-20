# Plan: Pantograph Milestone 6 Diagnostics, Documentation, And Rollout Safety

## Status
Completed

Last updated: 2026-04-16

## Current Source-of-Truth Summary

This document is the dedicated source of truth for runtime-registry Milestone
6. It expands the short Milestone 6 subsection in
`docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
into a standards-reviewed close-out plan for diagnostics visibility,
documentation traceability, and rollout-safety decisions after the Milestone 5
workflow/adapter refactor.

Milestone 6 planning and status should now be updated here first. The umbrella
runtime-registry plan and the broader workflow/runtime roadmap should reference
this file instead of duplicating Milestone 6 detail.

The accurate implementation baseline at plan creation time is:

- aggregate runtime debug snapshot transport already exists in
  `src-tauri/src/llm/commands/registry.rs` and synchronizes the shared runtime
  registry before returning backend-owned runtime mode, health, recovery, and
  latest workflow diagnostics facts
- that debug surface already supports optional workflow/session filtering and
  opt-in workflow trace reads by reusing backend-owned diagnostics and trace
  helpers rather than building a second adapter-local path
- synchronized runtime-registry snapshot and targeted reclaim command surfaces
  already exist and should be treated as completed inputs to Milestone 6, not
  new feature work
- immediate documentation compliance gaps still exist around the touched
  runtime/diagnostics areas:
  - `src-tauri/src/workflow/diagnostics/` has no `README.md`
  - `crates/pantograph-workflow-service/src/trace/` has no `README.md`
  - `docs/` has no `README.md`
  - `docs/logs/` has no `README.md`
- `src-tauri/src/llm/runtime_registry/` currently exists as an empty source
  directory with no documented ownership boundary; Milestone 6 must either
  remove it or give it a real documented purpose before more work lands nearby
- no dedicated Milestone 6 source-of-truth file existed before this plan
- no explicit written decision yet records whether rollout toggles are needed
  for the Milestone 6 close-out

## Objective

Close runtime-registry Milestone 6 by making Pantograph's runtime-registry,
recovery, and workflow-diagnostics surfaces observable and explainable,
bringing the immediate codebase surroundings up to documentation standards, and
recording rollout-safety decisions without moving business logic or long-lived
policy ownership into Tauri, bindings, or frontend code.

## Scope

### In Scope

- dedicated Milestone 6 source-of-truth planning and status tracking
- README compliance and boundary traceability for the runtime/diagnostics areas
  directly touched by runtime-registry work
- operator/developer documentation for runtime debug snapshot, runtime-registry
  snapshot, targeted reclaim, technical-fit observability, and recovery
  semantics
- explicit rollout-safety decision-making, including a documented "no new
  toggle" decision if no toggle is justified
- additive contract review for diagnostics/debug/recovery transport surfaces
- artifact-validation or tooling follow-through only if new persisted
  structured artifacts are introduced during Milestone 6
- cleanup of vestigial or generated directories in the immediate insertion
  areas when they would otherwise leave the codebase non-compliant

### Out of Scope

- new runtime-registry admission, retention, eviction, or technical-fit policy
- new scheduler features, KV cache work, or graph execution behavior
- frontend-owned diagnostics business logic or TypeScript-side runtime policy
- new persisted telemetry stores, replay logs, or trace databases unless a
  later milestone explicitly approves them
- broad UI redesign or new GUI features beyond documenting existing debug
  surfaces and their ownership

## Inputs

### Problem

Milestones 1 through 5 moved runtime policy, workflow diagnostics assembly, and
recovery semantics back behind backend-owned Rust boundaries, but the source of
truth for that work is still incomplete. The code now exposes richer runtime
debugging and recovery surfaces, yet the documentation around those surfaces
has not been fully reconciled with the standards, and the rollout posture is
not explicitly recorded. Without a dedicated Milestone 6 plan, follow-on work
would risk reopening boundary drift, leaving touched directories undocumented,
or adding speculative rollout toggles that create new state or configuration
owners without a concrete operational need.

### Constraints

- Backend-owned runtime policy remains in Rust backend crates, not in Tauri,
  TypeScript, UniFFI, Rustler, or other adapters.
- Tauri remains the desktop composition root and transport host; it may expose
  additive debug aggregation but must not become the owner of runtime truth.
- `pantograph-workflow-service` remains the owner of canonical workflow trace
  semantics.
- `pantograph-runtime-registry` remains the owner of runtime-registry policy
  state and machine-consumable snapshot semantics.
- Milestone 6 should prefer documenting and validating already-landed behavior
  over expanding the feature surface.
- Do not add rollout toggles "just in case"; they require explicit owner,
  default, compatibility, and configuration semantics.
- Every touched source directory must satisfy the README requirements in
  `DOCUMENTATION-STANDARDS.md`.
- Any new operator/developer documentation must describe real code paths and
  currently valid recovery/debug behavior, not planned behavior.

### Public Facade Preservation Note

Milestone 6 is a facade-first documentation and observability close-out. The
default path is to preserve existing runtime debug, workflow diagnostics, and
recovery command surfaces while clarifying their semantics and ownership.
Breaking contract changes are out of scope unless a separate re-plan records
the compatibility impact explicitly.

### Assumptions

- The existing aggregate runtime debug snapshot command is sufficient as the
  primary Milestone 6 observability surface unless acceptance review proves a
  concrete gap.
- ADR-002 already covers the core runtime-registry ownership boundary; Milestone
  6 may only need to update it for clarified consequences or rollout-safety
  notes rather than creating a new ADR.
- No new persisted schema-backed diagnostics artifact is required for Milestone
  6 unless documentation work intentionally adds checked-in examples or
  fixtures.
- Milestone 5 transport, binding, and recovery/idempotency hardening are the
  baseline; Milestone 6 should not reopen those refactors without a standards
  or correctness reason.

### Dependencies

- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-milestone-5-workflow-adapter-integration.md`
- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `src-tauri/src/llm`
- `src-tauri/src/workflow`
- `crates/pantograph-workflow-service/src`
- `crates/pantograph-embedded-runtime/src`
- `crates/pantograph-runtime-registry/src`
- documentation and tooling standards in
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`

### Affected Structured Contracts

- runtime debug snapshot request/response transport contracts
- runtime-registry snapshot and targeted-reclaim transport contracts
- workflow diagnostics snapshot and workflow trace snapshot transport contracts
- recovery debug state surfaced through Tauri host commands
- README/ADR documentation text that describes runtime-registry, recovery, and
  diagnostics contract ownership

### Affected Persisted Artifacts

- this dedicated Milestone 6 plan
- roadmap and umbrella-plan source-of-truth references
- README files added or updated in touched source and docs directories
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md` if Milestone
  6 clarifies accepted consequences there
- operator/developer documentation added under `docs/` if needed
- tooling or hook configuration only if Milestone 6 introduces new persisted
  structured artifacts that require validation

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate Milestone 6 surroundings are not yet fully compliant with the
documentation standards:

- `src-tauri/src/workflow/diagnostics/` is a source directory without a
  `README.md`
- `crates/pantograph-workflow-service/src/trace/` is a source directory
  without a `README.md`
- `docs/` currently lacks a top-level `README.md` even though Milestone 6 work
  will place or update architecture/operator documents there
- `docs/logs/` contains many checked-in logs and has no `README.md`
- `src-tauri/src/llm/runtime_registry/` is an empty source directory with no
  README or owned files; Milestone 6 must not leave that boundary ambiguous
- generated cache directories such as `__pycache__/` are present under some
  source roots and should not survive as part of the documented source-tree
  contract if touched during Milestone 6 cleanup

Milestone 6 must therefore include surrounding documentation/cleanup refactors
before it can claim standards-compliant close-out.

### Concurrency / Race-Risk Review

- Runtime debug snapshot reads can overlap with health polling, automatic
  recovery, workflow execution, targeted reclaim, and registry reconciliation.
- Milestone 6 must preserve the existing rule that debug surfaces reread or
  synchronize backend-owned state instead of caching adapter-local truth.
- Recovery, restart, restore, and reclaim docs must describe one owner for
  each state transition path so operator guidance does not imply duplicate
  manual reconciliation flows.
- If rollout toggles are introduced, they must have a single configuration
  owner and deterministic startup semantics; split frontend/backend toggle
  state is not acceptable.
- If new artifact validation hooks are added, they must be fast enough for the
  intended hook stage and must not rely on mutable shared test state.

### Ownership And Lifecycle Note

- `pantograph-runtime-registry` owns runtime-registry state, transition,
  admission, retention, warmup, technical-fit, and reclaim policy.
- `pantograph-workflow-service` owns canonical workflow trace and scheduler
  facts.
- `pantograph-embedded-runtime` owns producer-aware runtime translation,
  recovery-plan derivation, and execution-path reconciliation helpers that
  adapters consume.
- `src-tauri/src/llm` and `src-tauri/src/workflow` remain transport/composition
  layers that aggregate, synchronize, and forward backend-owned facts without
  becoming policy owners.
- Milestone 6 documentation must reinforce these ownership boundaries instead
  of introducing operator guidance that bypasses them.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Documentation drifts from the already-landed code paths | High | Treat current code as the baseline, inventory concrete commands/helpers first, and update roadmap/umbrella references in the same change as the dedicated plan |
| README cleanup stops at parent directories and leaves touched child source directories undocumented | High | Inventory all immediate source/doc directories up front and require README closure or directory removal before Milestone 6 is marked complete |
| A speculative rollout toggle creates a second policy/config owner | High | Default to no new toggle unless a concrete rollout risk is documented; if a toggle is required, keep it backend-owned with explicit defaults and lifecycle text |
| Operator docs describe Tauri-local workarounds instead of backend-owned reconciliation paths | High | Anchor all recovery, debug, and reclaim guidance to backend-owned contracts and existing synchronized helper paths |
| New checked-in examples or fixtures drift from producer contracts | Medium | Add lightweight artifact validation only if new persisted structured artifacts are actually introduced |
| Vestigial directories or generated cache directories remain in touched source roots | Medium | Treat them as compliance work in Milestone 6 rather than ignoring them as incidental clutter |

## Standards Review Passes

### Pass 1: Plan Structure And Source-of-Truth

Reviewed against:
- `templates/PLAN-TEMPLATE.md`
- `PLAN-STANDARDS.md`

Corrections applied:
- Created a dedicated Milestone 6 plan instead of leaving the milestone as a
  short umbrella-plan subsection.
- Added affected-contract, persisted-artifact, concurrency, ownership, and
  facade-preservation notes because Milestone 6 crosses runtime, workflow,
  documentation, and rollout boundaries.
- Updated the umbrella plan and roadmap to point at this file so status and
  sequencing do not keep drifting across documents.

### Pass 2: Architecture And Code Ownership

Reviewed against:
- `ARCHITECTURE-PATTERNS.md`
- `CODING-STANDARDS.md`

Corrections applied:
- Locked runtime-registry, trace, recovery-plan, and technical-fit ownership to
  backend Rust crates and kept Tauri/bindings transport-only.
- Scoped Milestone 6 to observability close-out and documentation rather than
  reopening policy implementation in adapters.
- Added explicit cleanup for undocumented immediate insertion directories so
  the resulting codebase surroundings comply with source-directory README
  requirements.

### Pass 3: Documentation And Traceability

Reviewed against:
- `DOCUMENTATION-STANDARDS.md`

Corrections applied:
- Recorded the concrete missing README gaps in touched source and docs
  directories instead of assuming the surroundings were already compliant.
- Required module README updates to include architectural rationale, consumer
  contracts, and structured producer contract text where the directory exposes
  machine-consumable runtime/diagnostics data.
- Added ADR and docs traceability requirements so Milestone 6 updates README,
  ADR, and roadmap/plan text together.

### Pass 4: Interop And Boundary Contracts

Reviewed against:
- `INTEROP-STANDARDS.md`
- `LANGUAGE-BINDINGS-STANDARDS.md`

Corrections applied:
- Kept debug/diagnostics/recovery transport review scoped to boundary-local
  validation and additive contract semantics.
- Explicitly prohibited Milestone 6 from introducing binding-local policy or
  alternate debug/recovery semantics.
- Required operator/developer docs to describe backend-owned contract owners
  rather than implying that Tauri or generated bindings are the source of
  truth.

### Pass 5: Testing, Concurrency, And Rollout Safety

Reviewed against:
- `TESTING-STANDARDS.md`
- `CONCURRENCY-STANDARDS.md`
- `SECURITY-STANDARDS.md`

Corrections applied:
- Required at least one acceptance path that exercises startup/shutdown,
  recovery, and debug visibility through the real boundaries rather than only
  reviewing docs statically.
- Added replay/recovery overlap and synchronized-state expectations to the
  rollout-safety review so Milestone 6 docs do not normalize stale local state
  handling.
- Recorded that any new rollout toggle must have a single owner, validated
  boundary inputs, and deterministic defaults.

### Pass 6: Tooling And Dependency Discipline

Reviewed against:
- `TOOLING-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`

Corrections applied:
- Limited tooling work to lightweight validation or hook follow-through only if
  Milestone 6 introduces new persisted structured artifacts.
- Avoided assuming new dependencies are justified for documentation or
  validation work; existing repo tooling should be preferred unless a concrete
  gap appears.
- Required repo traceability updates to stay in the same logical slices as the
  documentation changes they describe.

## Definition of Done

- this dedicated Milestone 6 plan exists and is referenced by the umbrella
  runtime-registry plan and the roadmap
- touched runtime/diagnostics/doc directories are standards-compliant: each
  required directory has a real `README.md` or the directory is removed as
  vestigial
- runtime debug, registry snapshot, targeted reclaim, recovery, and workflow
  diagnostics ownership are accurately documented without moving policy into
  adapters
- the rollout-toggle decision is explicit: either no new toggle is needed and
  that is recorded, or an additive backend-owned toggle is documented with
  defaults and compatibility semantics
- ADR/operator/developer documentation is reconciled with the actual
  Milestone 6 code path ownership
- any new persisted structured artifacts introduced by Milestone 6 have an
  appropriate validation/tooling story
- startup, shutdown, recovery, and diagnostics visibility are verified through
  at least one real acceptance/smoke path
- roadmap and umbrella-plan wording match Milestone 6 reality at close-out

## Milestones

### Milestone 1: Freeze Milestone 6 Source Of Truth

**Goal:** Turn Milestone 6 into a dedicated, standards-reviewed execution plan
before more close-out work lands.

**Tasks:**
- [x] Create this dedicated Milestone 6 plan and promote it to the source of
      truth for Milestone 6 sequencing.
- [x] Update the umbrella runtime-registry plan and roadmap so they reference
      this file instead of keeping Milestone 6 as a short inline checklist.
- [x] Inventory the already-landed diagnostics/debug/recovery surfaces that
      Milestone 6 documents rather than re-plans as new features.
- [x] Record the immediate documentation and directory-compliance gaps in the
      touched runtime/diagnostics surroundings.

**Verification:**
- Plan review against `PLAN-STANDARDS.md`
- Source-of-truth review confirms roadmap and umbrella plan reference this file

**Status:** Completed

### Milestone 2: Bring Immediate Surroundings To Documentation Compliance

**Goal:** Fix the README and boundary-traceability gaps in the directories that
Milestone 6 directly touches.

**Tasks:**
- [x] Add `README.md` coverage for `src-tauri/src/workflow/diagnostics/`,
      `crates/pantograph-workflow-service/src/trace/`, `docs/`, and
      `docs/logs/` with all required sections from
      `DOCUMENTATION-STANDARDS.md`.
- [x] Resolve `src-tauri/src/llm/runtime_registry/` by removing it if it is
      vestigial or giving it a real documented ownership boundary before future
      code lands there.
- [x] Remove or properly ignore generated cache directories from touched source
      roots instead of treating them as part of the architecture surface.
- [x] Reconcile parent and child README text so runtime-registry, workflow
      trace, embedded-runtime, and Tauri transport ownership statements do not
      contradict each other.

**Verification:**
- README review against `DOCUMENTATION-STANDARDS.md`
- Directory scan confirms no touched required directory is left undocumented

**Status:** Completed

### Milestone 3: Close Diagnostics Contracts And Rollout-Safety Decisions

**Goal:** Freeze what the debug and recovery close-out surfaces mean and decide
whether any rollout toggle is actually necessary.

**Tasks:**
- [x] Audit runtime debug snapshot, runtime-registry snapshot, targeted
      reclaim, and workflow diagnostics/trace transport contracts for
      additive-only semantics and boundary-local validation.
- [x] Decide whether a rollout toggle is required. Default to no new toggle
      unless a concrete migration or operational risk is documented.
- [x] If no toggle is needed, record that decision with rationale and revisit
      triggers in the Milestone 6 docs/README surfaces.
- [x] Record that no new toggle is required today and freeze the fallback rule
      that any future toggle must stay backend-owned with explicit config
      ownership, defaults, ordering, upgrade semantics, and restart behavior.
- [x] Ensure recovery, restart, restore, and reclaim guidance points to the
      synchronized backend-owned helper paths rather than Tauri-local
      bookkeeping or manual drift-prone workarounds.

**Verification:**
- Interop/boundary review against `INTEROP-STANDARDS.md`
- Targeted smoke verification for startup, shutdown, recovery, runtime debug
  snapshot, and workflow trace visibility through real command paths

**Status:** Completed

### Milestone 4: ADR, Operator Docs, Tooling, And Final Reconciliation

**Goal:** Close the durable traceability and tooling follow-through for
Milestone 6 and reconcile all source-of-truth documents.

**Tasks:**
- [x] Update `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md` if
      Milestone 6 needs to record clarified accepted consequences around
      diagnostics visibility, recovery ownership, or rollout posture.
- [x] Add a new ADR only if Milestone 6 uncovers a new long-lived boundary
      decision not already covered by ADR-002.
- [x] Add or update operator/developer documentation under `docs/` covering
      runtime debug snapshot usage, registry snapshot interpretation, technical-fit
      observability, and recovery expectations.
- [x] Add lightweight artifact validation or hook documentation only if
      Milestone 6 introduces new persisted structured examples, fixtures, or
      machine-consumed docs artifacts.
- [x] Reconcile milestone status and wording across this plan, the umbrella
      plan, and the roadmap once Milestone 6 verification passes.

**Verification:**
- Documentation review against `DOCUMENTATION-STANDARDS.md`
- Tooling/traceability review against `TOOLING-STANDARDS.md`
- Acceptance/recovery close-out review against `TESTING-STANDARDS.md`

**Status:** Completed

## Execution Notes

Update during implementation:
- 2026-04-16: Dedicated Milestone 6 draft created after reviewing the roadmap,
  umbrella runtime-registry plan, current runtime/debug/recovery code
  surfaces, and the planning, architecture, coding, documentation, interop,
  testing, concurrency, tooling, dependency, and security standards.
- 2026-04-16: Draft review identified immediate documentation non-compliance in
  `src-tauri/src/workflow/diagnostics/`,
  `crates/pantograph-workflow-service/src/trace/`, `docs/`, `docs/logs/`, and
  the vestigial empty `src-tauri/src/llm/runtime_registry/` directory.
- 2026-04-16: Source-of-truth references were updated so the umbrella plan and
  roadmap can point to this file instead of keeping Milestone 6 detail inline.
- 2026-04-16: Completed the Milestone 6 documentation-compliance slice by
  adding README coverage for `src-tauri/src/workflow/diagnostics/`,
  `crates/pantograph-workflow-service/src/trace/`, `docs/`, and `docs/logs/`,
  removing the vestigial empty `src-tauri/src/llm/runtime_registry/`
  directory, and ignoring/removing generated cache directories from the touched
  source roots.
- 2026-04-16: Completed the Milestone 6 rollout-safety slice by documenting
  the current runtime debug, reclaim, and recovery surfaces; recording the
  explicit "no new Milestone 6 rollout toggle" decision with revisit triggers;
  and tying the operator guidance back to backend-owned synchronization and
  reclaim helpers rather than Tauri-local bookkeeping.
- 2026-04-16: Completed the Milestone 6 close-out slice by clarifying ADR-002
  with additive observability guidance, reusing that ADR instead of creating a
  new one, recording that no new artifact-validation hook was required for
  narrative docs-only additions, and reconciling milestone status back into
  the umbrella plan and roadmap.

## Commit Cadence Notes

- Commit Milestone 6 documentation, cleanup, and rollout-safety slices as
  atomic changes that update code and traceability artifacts together.
- Keep README/ADR/doc-only refactors separate from behavior changes when that
  improves review clarity.
- Follow `COMMIT-STANDARDS.md` for detailed, atomic commit messages.

## Re-Plan Triggers

- Milestone 6 work reveals that a new persisted telemetry or diagnostics
  artifact is required.
- A rollout-safety review finds a concrete need for feature gating, staged
  migration, or compatibility toggles.
- README or ADR review reveals an ownership boundary that is not actually
  covered by ADR-002.
- Closing the documentation gaps would require moving policy back into Tauri,
  bindings, or frontend code.
- New operator workflows or non-Tauri hosts need a shared runtime-debug or
  recovery contract not captured in the current Milestone 6 scope.

## Recommendations

- Recommendation 1: Default Milestone 6 to "no new rollout toggle" unless a
  real operational hazard is demonstrated. This keeps configuration ownership
  simple and avoids inventing a second state source.
- Recommendation 2: Remove vestigial directories instead of documenting dead
  boundaries. A deleted empty source directory is more standards-compliant than
  a README that explains a boundary Pantograph does not actually use.
- Recommendation 3: Prefer updating ADR-002 plus focused READMEs/operator docs
  over creating a new ADR unless Milestone 6 truly accepts a new architectural
  boundary.

## Completion Summary

### Completed

- Dedicated Milestone 6 draft created.
- Standards review passes completed across planning, architecture, coding,
  documentation, interop, testing, concurrency, tooling, dependency, and
  security expectations.
- Immediate Milestone 6 documentation and source-tree compliance gaps were
  identified and recorded as explicit work items rather than being left as
  unstated assumptions.
- The umbrella runtime-registry plan and the broader roadmap now point at this
  file as the Milestone 6 source of truth.
- Milestone 2 documentation-compliance work is complete: the touched
  diagnostics, trace, and docs directories now have README coverage, generated
  cache directories are ignored and removed from the touched source roots, and
  the vestigial empty `src-tauri/src/llm/runtime_registry/` directory has been
  deleted.
- Milestone 3 rollout-safety work is complete: the no-new-toggle decision is
  now documented, and runtime debug/recovery/reclaim guidance explicitly
  points to the backend-owned synchronization paths already used by the code.
- Milestone 4 close-out work is complete: ADR-002 now covers additive
  observability surfaces, no new ADR or artifact-validation hook was needed,
  and the roadmap plus umbrella plan can be reconciled to Milestone 6
  completion.

### Deviations

- None.

### Follow-Ups

- Scheduler V2 remains the next major roadmap target after this Milestone 6
  close-out.

### Verification Summary

- Planning and standards review completed against the current roadmap,
  umbrella-plan, README, ADR, and touched runtime/diagnostics source
  boundaries.
- `cargo test --manifest-path src-tauri/Cargo.toml runtime_debug_snapshot_includes_synced_runtime_and_recovery_state`
- `cargo test --manifest-path src-tauri/Cargo.toml reclaim_runtime_returns_updated_registry_snapshot`
- `cargo test -p pantograph-runtime-registry reclaim_runtime_requests_stop_for_active_evictable_runtime`

### Traceability Links

- Dedicated plan: `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-milestone-6-diagnostics-documentation-rollout-safety.md`
- Umbrella plan: `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
- Roadmap: `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- Related ADR: `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
