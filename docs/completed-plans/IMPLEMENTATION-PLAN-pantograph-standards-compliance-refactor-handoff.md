# Plan: Pantograph Standards Compliance Refactor Handoff

## Objective

Realign the current diagnostics and workflow-execution implementation work with
the coding standards so backend-owned workflow behavior returns to Rust-owned
contracts and the TypeScript frontend is reduced to presentation, transport,
and view-model concerns.

## Scope

### In Scope

- Audit the recent workflow and diagnostics slices against the coding standards
- Record which recent commits are compliant, partially compliant, or temporary
  frontend stopgaps
- Define the Rust-side refactor targets needed to restore backend ownership
- Preserve the roadmap sequence so work can resume smoothly after the refactor

### Out of Scope

- Performing the full Rust refactor in this document
- Rewriting unrelated dirty work already present in the repository
- Replacing the broader roadmap with a new roadmap

## Inputs

### Problem

Recent work improved workflow diagnostics and event handling, but several slices
implemented execution ownership and scheduler-facing decisions in TypeScript.
Under the coding standards, that creates drift from the intended architecture:
the backend is the single source of truth, services own business logic, and the
frontend should only own transient UI state or pure presentation concerns.

### Constraints

- `CODING-STANDARDS.md` requires backend-owned data and business logic to stay
  out of the frontend.
- `ARCHITECTURE-PATTERNS.md` states that frontend state is allowed only when
  the backend has no concept of that state.
- `LANGUAGE-BINDINGS-STANDARDS.md` treats TypeScript/Tauri as a host binding
  layer over a Rust-owned implementation rather than a primary logic surface.
- Existing roadmap sequencing should remain intact after the refactor.
- Unrelated dirty work in the repository must remain untouched.

### Assumptions

- The Rust workflow service and Tauri adapter remain the correct ownership
  boundary for canonical workflow execution, diagnostics, and scheduler state.
- Some of the recent TypeScript work is still useful as a temporary sketch of
  desired behavior, even where final ownership should move to Rust.
- The diagnostics UI remains optional and may continue to consume projections,
  but it should not own canonical workflow state transitions.

### Dependencies

- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `src-tauri/src/workflow/*`
- `crates/pantograph-workflow-service/src/*`
- `crates/node-engine/src/events.rs`
- `src/services/workflow/*`
- `src/stores/diagnosticsStore.ts`
- `packages/svelte-graph/src/*`

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Leaving execution ownership in TypeScript hardens a second state machine outside Rust | High | Move canonical execution/session identity and scheduler distinctions into Rust-owned contracts |
| Reverting too broadly loses useful event-contract and diagnostics progress | Medium | Classify commits by keep/supersede status instead of blanket rollback |
| Refactor sequencing drifts from the roadmap and stalls progress | Medium | Record the exact resume order after compliance work |
| Frontend docs/tests start asserting temporary behavior as permanent | Medium | Mark frontend-owned slices as transitional and superseded by Rust-owned replacements |

## Definition of Done

- The recent workflow/diagnostics slices are classified against the standards
- A concrete Rust-first refactor order is recorded
- The next roadmap targets after the refactor are explicitly listed
- The repository contains a handoff artifact that future work can follow

## Milestones

### Milestone 1: Audit Recent Slices

**Goal:** Capture what landed and whether each slice aligns with the standards.

**Tasks:**
- [x] Review recent commits affecting diagnostics, workflow events, and session
      execution
- [x] Compare those slices against the coding standards
- [x] Classify each recent slice as keep, supersede, or reassess

**Verification:**
- Read `CODING-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`, and
  `LANGUAGE-BINDINGS-STANDARDS.md`
- Compare against the current Pantograph commit history

**Status:** Complete

### Milestone 2: Record Rust Refactor Targets

**Goal:** Define what must move out of TypeScript and back into Rust-owned
contracts.

**Tasks:**
- [ ] Move canonical session kind / run kind ownership into Rust or Tauri DTOs
- [ ] Move stale-event rejection rules into a backend-owned execution/session
      contract or canonical projection
- [ ] Move edit-session vs persisted-run scheduler distinctions into the
      workflow service or Tauri adapter
- [ ] Reduce frontend diagnostics stores to projection and rendering only

**Verification:**
- Refactor plan maps each frontend-owned rule to a Rust-owned contract or
  service
- Frontend remains a consumer of contracts rather than an owner of behavior

**Status:** Not started

### Milestone 3: Resume Roadmap After Compliance Pass

**Goal:** Continue the roadmap without re-opening the same ownership mistakes.

**Tasks:**
- [ ] Re-land diagnostics/event work through Rust-owned contracts
- [ ] Resume Metrics/trace spine implementation in Rust-first order
- [ ] Continue into Scheduler V2 and Incremental Graph Execution only after the
      backend contract is authoritative

**Verification:**
- The next implementation slice after the refactor touches Rust/Tauri first for
  canonical workflow behavior
- Frontend changes are limited to consuming the new contract

**Status:** Not started

## Execution Notes

- 2026-04-12: Reviewed the recent Pantograph commits after the user raised a
  standards concern about TypeScript ownership.
- 2026-04-12: Confirmed that the current work improved behavior, but some of it
  crossed from view-model coordination into backend-owned workflow rules.
- 2026-04-12: Recorded the refactor handoff below so implementation can resume
  without losing roadmap continuity.

## Commit Cadence Notes

- The compliance refactor should supersede the temporary frontend-owned slices
  with atomic Rust-first commits.
- Prefer one commit for each ownership correction boundary:
  Rust contract/service change first, frontend consumption second, docs/tests
  third when they do not fit cleanly into the first two slices.

## Re-Plan Triggers

- A Rust-side contract cannot represent the needed execution/session semantics
  without broader service changes
- Scheduler V2 work reveals that diagnostics should consume a projection API
  instead of raw event streams
- The UI must support a mode where no frontend exists and diagnostics remain
  fully available headlessly

## Recommendations

- Recommendation 1: Treat `11f49a6`, `3552af0`, and `9dcebed` as transitional
  frontend stopgaps rather than final architecture.
  Why: they encode execution ownership, session-kind behavior, or stale-event
  policy in TypeScript, which the standards place on the backend side.
  Impact: these slices should be superseded during the Rust compliance pass.

- Recommendation 2: Keep and build on the Rust/Tauri contract work from
  `d2530cb`, `ab9d43f`, `1f1bba5`, and `5ae6333`.
  Why: these commits moved event shape, diagnostics snapshots, and session-first
  execution toward backend-owned contracts instead of away from them.
  Impact: they remain the strongest base for the refactor.

- Recommendation 3: Rework edit-session diagnostics so the workflow service
  publishes canonical scheduler/execution projections instead of requiring the
  frontend to infer them.
  Why: the frontend should render projections, not decide what kind of run owns
  a session or whether an event is stale.
  Impact: adds Rust/Tauri work first, then simplifies the Svelte stores.

- Recommendation 4: Keep frontend logic limited to transient UI concerns such
  as panel open state, selected diagnostics tab, and selected run/node in the
  already-produced diagnostics projection.
  Why: these states have no backend equivalent and remain compliant frontend
  ownership.
  Impact: no architectural conflict with the standards.

## Completion Summary

### Completed

- Audited the recent diagnostics/workflow slices against the coding standards
- Identified the specific commits that should be treated as temporary frontend
  ownership
- Recorded the Rust-first refactor order and the roadmap resume order

### Deviations

- The recent implementation sequence used TypeScript for some workflow
  execution ownership and diagnostics coordination that should instead live in
  Rust-owned contracts

### Follow-Ups

- Replace frontend-owned `currentSessionKind` behavior with backend-provided run
  classification or a canonical execution/session projection
- Replace frontend stale-event filtering with Rust/Tauri-owned execution
  identity or projection rules
- Publish canonical diagnostics/session projections from the backend before
  resuming more scheduler and incremental-execution work
- Revisit diagnostics README files after the refactor so they describe the
  final ownership model rather than the current transitional one

### Verification Summary

- Reviewed:
  `CODING-STANDARDS.md`
- Reviewed:
  `ARCHITECTURE-PATTERNS.md`
- Reviewed:
  `LANGUAGE-BINDINGS-STANDARDS.md`
- Audited recent commits:
  `69f4b17`, `ab9d43f`, `70b0eca`, `1f1bba5`, `5ae6333`, `11f49a6`,
  `3552af0`, `9dcebed`

### Traceability Links

- Roadmap reference:
  `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- This handoff plan:
  `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-standards-compliance-refactor-handoff.md`
- ADR added/updated:
  N/A

## Resume Order After Refactor

1. Re-establish execution identity, session kind, and stale-event handling as a
   Rust/Tauri-owned contract.
2. Rebuild the diagnostics store as a consumer of that contract rather than an
   owner of workflow rules.
3. Re-run standards compliance and update the affected READMEs.
4. Resume the roadmap at Phase 1 with Rust-first metrics/trace work.
5. Continue to Scheduler V2 and Incremental Graph Execution only after the
   event and diagnostics ownership boundary is clean.
