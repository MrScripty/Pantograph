# Plan: Pantograph Milestone 5 Workflow And Adapter Integration

## Status
Active

Last updated: 2026-04-16

## Current Source-of-Truth Summary

This document is the dedicated source of truth for runtime-registry Milestone
5. It expands the short Milestone 5 section in
`IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
into a standards-reviewed execution plan for adapter-boundary hardening,
workflow diagnostics transport, and post-Milestone-4 workflow integration.

Milestone 5 planning and status should now be updated here first. The umbrella
runtime-registry plan and roadmap should reference this file instead of
duplicating Milestone 5 detail.

## Objective

Complete runtime-registry Milestone 5 by hardening the workflow and binding
integration path so Tauri, Rustler, UniFFI, and related transport code forward
backend-owned runtime-selection, scheduler, trace, and diagnostics semantics
without becoming policy owners, while refactoring the immediate insertion areas
to comply with Pantograph’s architecture, coding, documentation, testing,
concurrency, tooling, interop, and security standards.

## Scope

### In Scope

- Tauri workflow execution, diagnostics, and headless transport boundaries that
  forward backend-owned runtime-registry, scheduler, trace, and technical-fit
  results
- Binding and adapter review for UniFFI, Rustler, and frontend HTTP workflow
  request/response surfaces touched by runtime-selection and diagnostics work
- Refactors required to keep immediate Milestone 5 insertion points compliant
  before more adapter or diagnostics logic lands
- Machine-consumable workflow error, scheduler snapshot, trace snapshot, and
  diagnostics projection transport semantics
- Documentation and source-of-truth updates required by touched workflow and
  diagnostics boundaries

### Out of Scope

- New runtime-selection policy or candidate scoring rules owned by
  `pantograph-runtime-registry`
- New frontend-only workflow state, optimistic UI ownership, or TypeScript-side
  business logic
- Distributed scheduling, multi-host coordination, or cross-process registry
  ownership changes
- Replacing the existing embedded runtime or workflow-service public facades
  beyond additive contract work explicitly justified in this milestone
- Broad diagnostics feature expansion that belongs to Milestone 6 rather than
  transport/boundary integration

## Inputs

### Problem

Milestone 4 completed the backend-owned technical-fit path, but Milestone 5
still needs a dedicated plan for the remaining workflow and adapter integration
work. The codebase now has backend-owned runtime-registry selection, workflow
preflight integration, and additive explicit override transport, yet the
workflow transport and diagnostics integration surfaces still have two risks:

- oversized workflow adapter and trace files are already beyond decomposition
  thresholds and should not absorb more logic unchanged
- diagnostics, scheduler, trace, and runtime-not-ready transport paths still
  need a dedicated standards-compliant hardening pass so adapters remain
  transport wrappers rather than policy owners

Without a dedicated Milestone 5 plan, follow-on work would likely append more
logic into already-large files or let transport modules accumulate backend
decision reconstruction, which would violate the standards and re-open the
Tauri/core-boundary problems that the runtime-registry refactor was intended to
solve.

### Constraints

- Core runtime policy remains backend-owned in Rust crates; Tauri and bindings
  are transport/adaptation layers only.
- `src-tauri` may compose runtime and workflow services, but it must not become
  the owner of runtime admission, technical-fit, scheduler, or trace policy.
- Binding surfaces must preserve backend-owned request/response semantics and
  validate raw payloads only at the boundary.
- Public workflow facades should stay additive unless an explicit API break is
  approved and documented.
- Existing large files in the immediate insertion area require decomposition
  review before taking on more Milestone 5 logic.
- Diagnostics, trace, and scheduler outputs are machine-consumed contracts and
  must remain deterministic across adapters.

### Public Facade Preservation Note

Milestone 5 is a facade-first refactor. Existing public workflow commands and
binding entry points remain in place unless an additive backend-owned contract
change is required and explicitly documented. The default implementation choice
is internal extraction and delegation rather than API breakage.

### Assumptions

- Milestone 4 backend-owned technical-fit integration is the frozen foundation
  that Milestone 5 adapters must transport rather than reinterpret.
- `pantograph-workflow-service` remains the owner of canonical workflow trace
  and scheduler contracts, while `src-tauri/src/workflow/diagnostics.rs`
  remains a projection layer over backend-owned trace data.
- UniFFI, Rustler, and frontend HTTP workflow bindings should remain additive
  transport wrappers over the same core Rust request/response contracts.
- No new persistent datastore is required for Milestone 5; diagnostics and
  trace state can remain backend-owned in memory unless a later milestone
  explicitly changes that assumption.

### Dependencies

- `src-tauri/src/workflow`
- `crates/pantograph-workflow-service`
- `crates/pantograph-embedded-runtime`
- `crates/pantograph-uniffi`
- `crates/pantograph-rustler`
- `crates/pantograph-frontend-http-adapter`
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`

### Affected Structured Contracts

- Tauri workflow command request/response transport contracts
- Workflow diagnostics and trace snapshot request/response contracts
- Scheduler snapshot and runtime-not-ready machine-consumable error envelopes
- Binding request/response wrappers for UniFFI, Rustler, and frontend HTTP
- Backend-owned workflow trace and diagnostics projection DTOs when additive
  transport fields are required

### Affected Persisted Artifacts

- None required for the first Milestone 5 implementation slice
- README, roadmap, ADR, and implementation-plan documents touched to keep
  source-of-truth status aligned with code reality
- Any checked-in debug fixtures or snapshot examples added during Milestone 5
  become structured artifacts that must be validated with repo tooling

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate Milestone 5 insertion points already exceed coding-standards
decomposition thresholds and should not absorb more behavior without planned
refactor:

- `crates/pantograph-workflow-service/src/trace.rs` was approximately 2284
  lines before the Milestone 5 trace decomposition and should remain a focused
  `trace/` module tree rather than regressing into a catch-all file
- `crates/pantograph-workflow-service/src/workflow.rs` is approximately 6017
  lines
- `src-tauri/src/workflow/headless_workflow_commands.rs` is approximately 1688
  lines
- `src-tauri/src/workflow/diagnostics.rs` is approximately 1970 lines
- `src-tauri/src/workflow/commands.rs` is approximately 845 lines
- `src-tauri/src/workflow/workflow_execution_commands.rs` is approximately 670
  lines

Milestone 5 must therefore include explicit extraction work around workflow
trace ownership, diagnostics projection, and command transport surfaces before
adding more integration logic.

### Concurrency / Race-Risk Review

- Workflow diagnostics and trace updates overlap with session queue movement,
  runtime load/unload, runtime-registry observation, and event streaming.
- Scheduler snapshots, runtime snapshots, and trace projections must preserve
  backend-owned execution identity and must not invent adapter-local state when
  backend identity is absent or ambiguous.
- Diagnostics projection stores and trace stores hold shared mutable state that
  must keep related fields under one owner/lock and avoid splitting canonical
  state between backend and adapter overlays.
- Startup, shutdown, recovery, and session-removal flows must not leave stale
  diagnostics or trace ownership behind after a runtime or session disappears.
- Any background or event-driven diagnostics updates must document who starts
  the work, who stops it, and how overlap/cancellation is prevented.

### Ownership And Lifecycle Note

- `pantograph-workflow-service` remains the owner of canonical workflow trace,
  scheduler, and execution-state semantics.
- `pantograph-embedded-runtime` remains the owner of runtime/producer-aware
  workflow execution helpers that need backend-owned runtime facts.
- `src-tauri` remains the composition root and transport host; it may start or
  stop backend-owned workers but must not absorb their policy or canonical
  state.
- UniFFI, Rustler, and frontend HTTP adapters remain transport wrappers over
  backend-owned contracts. Generated bindings stay generated; hand-written
  changes belong in wrapper crates or backend crates only.
- Any Milestone 5 background work must state who creates it, who tears it
  down, and how restart/recovery overlap reconciles back to backend-owned
  state.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Adapter code regains runtime-policy ownership | High | Keep Milestone 5 scoped to transport, validation, and projection only; policy stays in backend crates |
| Oversized workflow transport files keep growing | High | Make decomposition a required milestone before more command or diagnostics logic lands |
| Diagnostics projection diverges from backend-owned trace data | High | Build projections from canonical trace/store DTOs and test one end-to-end producer-to-consumer path |
| Binding surfaces drift from Rust transport contracts | High | Review UniFFI, Rustler, and frontend HTTP wrappers together and keep changes additive |
| Recovery or restart paths leave stale trace/diagnostics state | Medium | Add replay/recovery/idempotency coverage and explicit ownership cleanup tasks |
| New debug fixtures or artifacts drift from producer contracts | Medium | Gate any new artifacts behind tooling validation and README traceability updates |

## Standards Review Passes

### Pass 1: Plan Structure And Source-of-Truth

Reviewed against:
- `templates/PLAN-TEMPLATE.md`
- `PLAN-STANDARDS.md`

Corrections applied:
- Kept this as a dedicated plan instead of leaving Milestone 5 as a short
  umbrella-plan subsection.
- Added required affected-contract, persisted-artifact, concurrency, ownership,
  and facade-preservation notes for cross-layer work.
- Declared this file the Milestone 5 source of truth so status can be kept
  aligned across roadmap and umbrella-plan updates.

### Pass 2: Architecture And Code Ownership

Reviewed against:
- `ARCHITECTURE-PATTERNS.md`
- `CODING-STANDARDS.md`

Corrections applied:
- Locked workflow/runtime/trace/scheduler policy ownership to backend Rust
  crates and kept Tauri/bindings transport-only.
- Added explicit refactor scope for oversized insertion files so new work does
  not deepen existing catch-all modules.
- Recorded single-owner state and facade-first preservation requirements for
  recovery, diagnostics, and trace work.

### Pass 3: Interop And Language Bindings

Reviewed against:
- `INTEROP-STANDARDS.md`
- `LANGUAGE-BINDINGS-STANDARDS.md`

Corrections applied:
- Restricted binding work to validation, conversion, and transport alignment at
  the boundary.
- Added wrapper-audit tasks for Tauri, UniFFI, Rustler, and frontend HTTP
  surfaces to prevent contract drift or binding-local business logic.
- Recorded that generated bindings are not hand-edited and that wrapper crates
  or backend crates own additive contract updates.

### Pass 4: Testing And Concurrency

Reviewed against:
- `TESTING-STANDARDS.md`
- `CONCURRENCY-STANDARDS.md`

Corrections applied:
- Required cross-layer acceptance coverage from backend producers through
  adapter consumers instead of crate-local checks alone.
- Added replay/recovery/idempotency and startup/shutdown overlap checks for
  diagnostics and trace ownership.
- Recorded lifecycle notes for any background work so cancellation and restart
  behavior is explicit before implementation begins.

### Pass 5: Documentation And Tooling

Reviewed against:
- `DOCUMENTATION-STANDARDS.md`
- `TOOLING-STANDARDS.md`

Corrections applied:
- Required README updates for any new modules or shifted responsibilities in
  workflow, diagnostics, or binding directories.
- Required umbrella-plan and roadmap synchronization so the dedicated plan
  remains traceable from existing source-of-truth documents.
- Kept persisted-artifact validation in scope if Milestone 5 introduces debug
  fixtures or snapshot examples.

### Pass 6: Security And Dependencies

Reviewed against:
- `SECURITY-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`

Corrections applied:
- Reasserted validate-once-at-the-boundary behavior for raw adapter payloads,
  with backend code consuming validated contracts rather than duplicating
  checks.
- Added dependency review scope for binding/adapter work so new cross-layer
  crates or features are not introduced casually.
- Limited Milestone 5 to additive transport changes unless a documented break
  becomes unavoidable.

## Definition of Done

- Tauri workflow command handlers and other bindings remain transport/adaptation
  code and do not implement runtime-selection, scheduler, or trace policy.
- Workflow diagnostics, scheduler snapshots, trace snapshots, and
  runtime-not-ready/admission failures preserve backend-owned machine-consumable
  semantics across adapters.
- Immediate Milestone 5 insertion points are decomposed enough that new logic
  lands in focused, ownership-aligned modules rather than enlarging catch-all
  files.
- Cross-layer acceptance coverage proves producer input to adapter output
  preserves backend-owned workflow runtime and diagnostics semantics.
- README and plan/roadmap/ADR traceability is current for the touched
  workflow/diagnostics boundaries.

## Milestones

### Milestone 1: Freeze Milestone 5 Boundaries

**Goal:** Record the exact adapter-boundary ownership and transport semantics
before more workflow integration changes land.

**Tasks:**
- [x] Freeze which workflow runtime, diagnostics, trace, and error semantics are
      backend-owned versus adapter-owned overlays.
- [x] Record which binding surfaces are in scope for Milestone 5 and which stay
      out of scope.
- [x] Record the immediate insertion-point refactor scope required before more
      behavior is added.
- [x] Update the umbrella runtime-registry plan so this dedicated file becomes
      the Milestone 5 source of truth.
- [x] Confirm that no planned Milestone 5 behavior requires new TypeScript- or
      Tauri-owned business logic and record the backend owner for every new
      contract touched by this milestone.

**Verification:**
- Architecture review against ADR-001 and ADR-002
- Contract review against `PLAN-STANDARDS.md`
- README/plan review confirms no adapter is described as a policy owner

**Status:** Completed

### Milestone 2: Refactor Immediate Insertion Points To Compliance

**Goal:** Reduce oversized workflow transport and diagnostics files before
Milestone 5 behavior work expands them further.

**Tasks:**
- [x] Extract workflow trace request/contract, trace state-machine/store, and
      runtime/scheduler merge helpers out of
      `crates/pantograph-workflow-service/src/trace.rs` into focused modules.
- [x] Extract Tauri headless workflow runtime-building, diagnostics snapshot,
      and trace/debug helpers out of
      `src-tauri/src/workflow/headless_workflow_commands.rs`.
- [x] Extract Tauri diagnostics projection/store responsibilities out of
      `src-tauri/src/workflow/diagnostics.rs` into focused modules by concern.
- [x] Reduce `src-tauri/src/workflow/commands.rs` and
      `workflow_execution_commands.rs` to thin command-group façades over those
      focused helpers.
- [x] Keep extracted workflow/runtime policy in backend Rust crates and limit
      Tauri extractions to transport routing, boundary parsing, and projection
      helpers only.
- [x] Update touched `README.md` files for any new directories or shifted
      responsibilities.

**Verification:**
- Decomposition review against `CODING-STANDARDS.md`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check -p pantograph-workflow-service`
- Review confirms no business logic moved from backend into Tauri during the
  extraction

**Status:** Completed

### Milestone 3: Harden Backend-Owned Workflow Diagnostics And Trace Transport

**Goal:** Ensure diagnostics, scheduler, runtime, and trace transport paths
preserve backend-owned semantics without adapter-local reconstruction.

**Tasks:**
- [ ] Normalize diagnostics and trace snapshot boundary validation into shared
      backend-owned request helpers where raw adapter input still leaks through.
- [x] Ensure scheduler snapshot, runtime snapshot, and trace snapshot transport
      keeps backend-owned execution/session identity semantics when identifiers
      are absent or ambiguous.
- [ ] Ensure Tauri diagnostics overlays remain additive UI/transport state and
      do not become canonical workflow or runtime state.
- [ ] Add explicit post-restart and post-restore reconciliation coverage so
      diagnostics and trace consumers resynchronize from backend-owned state
      after recovery-shaped transitions.
- [ ] Verify that runtime-not-ready, admission failure, and cancellation-shaped
      failures preserve machine-consumable backend envelopes through transport
      wrappers.
- [ ] Add or tighten acceptance coverage from backend trace/scheduler producers
      through Tauri/binding consumers.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- Cross-layer acceptance checks per `TESTING-STANDARDS.md`
- Interop review against `INTEROP-STANDARDS.md`

**Status:** In progress

### Milestone 4: Binding And Adapter Consistency Review

**Goal:** Keep UniFFI, Rustler, frontend HTTP, and Tauri workflow surfaces
aligned with the same backend-owned contracts.

**Tasks:**
- [ ] Audit UniFFI, Rustler, frontend HTTP, and Tauri workflow request/response
      wrappers for duplicate validation, local reconstruction, or contract
      drift.
- [ ] Apply additive transport-only changes where one binding still lags the
      backend-owned workflow/diagnostics contracts.
- [ ] Move any newly discovered binding-adjacent business logic back into the
      relevant backend Rust crate or wrapper crate before continuing.
- [ ] Ensure generated or wrapper-level binding docs identify the core Rust
      contract owner instead of implying binding-local business logic.
- [ ] Review dependency usage and avoid adding new cross-layer dependencies
      unless they are justified under `DEPENDENCY-STANDARDS.md`.

**Verification:**
- `cargo check -p pantograph-uniffi`
- `cargo check -p pantograph-rustler`
- `cargo check -p pantograph-frontend-http-adapter`
- Binding review against `LANGUAGE-BINDINGS-STANDARDS.md`

**Status:** Not started

### Milestone 5: Documentation, Recovery Hardening, And Close-Out

**Goal:** Finish Milestone 5 with standards-compliant docs, replay/recovery
coverage, and accurate source-of-truth updates.

**Tasks:**
- [ ] Add replay/recovery/idempotency verification for diagnostics/trace state
      where session restart, cleanup, or runtime recovery overlaps with
      retained workflow traces.
- [ ] Update `src-tauri/src/workflow/README.md`,
      `crates/pantograph-workflow-service/src/README.md`, and any newly created
      module READMEs to reflect final Milestone 5 ownership boundaries.
- [ ] Update the umbrella runtime-registry plan and roadmap wording so
      completed/in-progress milestone status remains accurate.
- [ ] Validate any checked-in debug fixtures or persisted snapshot examples with
      repo tooling if Milestone 5 introduces them.

**Verification:**
- Documentation review against `DOCUMENTATION-STANDARDS.md`
- Tooling review against `TOOLING-STANDARDS.md`
- Replay/recovery checks required by `TESTING-STANDARDS.md`
- Final compile/test pass for touched crates and adapter boundaries

**Status:** Not started

## Execution Notes

Update during implementation:
- 2026-04-16: Dedicated Milestone 5 draft created after Milestone 4 completion
  and a codebase/standards review of the remaining workflow adapter and
  diagnostics integration work.
- 2026-04-16: Draft reviewed and corrected against planning, architecture,
  coding, interop, bindings, testing, concurrency, documentation, tooling,
  security, and dependency standards; dedicated plan promoted to active source
  of truth.
- 2026-04-16: Began Milestone 5 implementation by extracting headless
  diagnostics projection and trace/scheduler snapshot helpers out of
  `src-tauri/src/workflow/headless_workflow_commands.rs` into
  `src-tauri/src/workflow/headless_diagnostics.rs`, reducing command-file
  ownership pressure while keeping backend-owned semantics intact.
- 2026-04-16: Extracted shared embedded-runtime construction out of
  `src-tauri/src/workflow/headless_workflow_commands.rs` into
  `src-tauri/src/workflow/headless_runtime.rs` so headless workflow,
  workflow-execution, and orchestration entry points reuse one host-runtime
  composition path instead of coupling that wiring to one command module.
- 2026-04-16: Extracted diagnostics snapshot, trace snapshot, and
  clear-history transport responses out of
  `src-tauri/src/workflow/headless_workflow_commands.rs` into
  `src-tauri/src/workflow/headless_diagnostics_transport.rs`, so workflow
  command wrappers and runtime debug commands share a focused diagnostics
  boundary instead of importing the broader headless workflow adapter.
- 2026-04-16: Decomposed `src-tauri/src/workflow/diagnostics.rs` into a
  dedicated `src-tauri/src/workflow/diagnostics/` module tree split across
  contracts, trace/projection helpers, store state, and preserved tests, so
  diagnostics projection and retained overlay logic no longer accumulate in one
  oversized Tauri transport file.
- 2026-04-16: Split Tauri execution/edit-session command registration out of
  `src-tauri/src/workflow/commands.rs` into
  `src-tauri/src/workflow/workflow_execution_tauri_commands.rs`, and reduced
  `workflow_execution_commands.rs` to a thin facade over focused runtime and
  edit-session helpers in `workflow_execution_runtime.rs` and
  `workflow_edit_session.rs`.

## Commit Cadence Notes

- Commit when each logical Milestone 5 slice is complete and verified.
- Keep refactor commits separate from behavior hardening commits whenever that
  separation preserves review clarity.
- Follow `COMMIT-STANDARDS.md` for detailed, atomic commit messages.

## Re-Plan Triggers

- Runtime-registry or workflow-service contract changes invalidate the frozen
  Milestone 5 transport assumptions.
- A required Milestone 5 fix would move policy back into Tauri or binding
  layers.
- Recovery, replay, or diagnostics ownership requirements expand beyond the
  current milestone scope and need Milestone 6 work pulled forward.
- New persisted artifacts, dependency additions, or API breaks become necessary
  and materially change sequencing or risk.

## Completion Summary

### Completed

- Dedicated Milestone 5 draft created.
- Standards review passes completed across the draft and the plan corrected to
  reflect boundary ownership, refactor-first sequencing, and source-of-truth
  linkage needs.
- Milestone 2 extraction work has already split headless diagnostics helpers,
  shared headless runtime construction, and diagnostics transport responses out
  of `headless_workflow_commands.rs` while keeping runtime policy backend-
  owned.
- Milestone 2 extraction work now also splits the workflow diagnostics module
  by concern, reducing one of the largest remaining Tauri-owned diagnostics
  insertion points without moving canonical trace or runtime policy out of the
  backend crates.
- Milestone 2 extraction work now also removes the edit-session execution block
  from the general workflow command root and leaves `workflow_execution_commands.rs`
  as a facade over focused helpers, reducing both remaining oversized Tauri
  command insertion points called out by this milestone.
- Milestone 2 extraction work now also replaces
  `crates/pantograph-workflow-service/src/trace.rs` with a focused `trace/`
  module tree that separates backend-owned trace contracts, request
  validation, in-memory trace state ownership, and runtime/scheduler merge
  helpers without moving diagnostics policy into adapter code.
- Milestone 3 transport hardening now preserves ambiguous scheduler identity at
  the Tauri diagnostics boundary: when backend scheduler snapshots omit
  `trace_execution_id`, the adapter keeps scheduler/runtime state as
  overlay-only and does not invent a run id from `session_id`.
- Milestone 3 boundary cleanup now routes diagnostics snapshot filter
  normalization through `WorkflowDiagnosticsSnapshotRequest` so the Tauri
  transport reuses one request-normalization rule instead of trimming fields in
  scattered command helpers.

### Deviations

- None yet.

### Follow-Ups

- Keep the roadmap and umbrella plan synchronized with this file as Milestone 5
  status changes during implementation.

### Verification Summary

- Initial repo inspection completed against the immediate Milestone 5 workflow,
  diagnostics, trace, and adapter surfaces.

### Traceability Links

- Dedicated plan: `IMPLEMENTATION-PLAN-pantograph-milestone-5-workflow-adapter-integration.md`
- Umbrella plan: `IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
- Roadmap: `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- Related ADRs: `docs/adr/ADR-001-headless-embedding-service-boundary.md`,
  `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
