# Plan: Pantograph Workflow Diagnostics View

## Objective

Add a developer-facing diagnostics view inside the existing Pantograph GUI that
visualizes workflow metrics, traces, scheduler/runtime state, and execution
events without violating the repo's code, architecture, frontend, concurrency,
testing, or documentation standards.

The resulting implementation must preserve Pantograph's current layered
boundaries:

- `crates/node-engine` remains the producer of execution-domain events and
  metrics
- `crates/pantograph-workflow-service` remains the host-agnostic application
  orchestration layer
- `crates/pantograph-embedded-runtime` remains the owner of Pantograph-specific
  runtime integration
- Tauri remains a transport/adapter boundary
- the Svelte GUI consumes trace contracts through services/stores rather than
  reaching into runtime implementation details directly

## Scope

### In Scope

- Define a diagnostics contract for workflow run traces, node traces, runtime
  traces, scheduler decisions, graph mutation traces, and event records.
- Add additive backend/service diagnostics surfaces needed by the existing GUI.
- Add a GUI diagnostics view reachable from the current workflow experience.
- Add a dedicated diagnostics service/store boundary for the frontend.
- Implement the initial GUI tabs for `Overview`, `Timeline`, and `Events`.
- Define follow-on GUI tabs for `Scheduler`, `Runtime`, and `Graph`.
- Add documentation, README, and ADR traceability required by the standards.
- Add cross-layer verification for producer -> adapter -> store -> GUI output.

### Out of Scope

- Implementing Scheduler V2 itself.
- Implementing parallel demand execution itself.
- Implementing KV cache storage itself.
- Replacing the existing workflow editor with a diagnostics-first UI.
- Adding a production analytics product or remote telemetry backend.
- Introducing polling loops for UI synchronization when event-driven delivery
  is feasible.

## Inputs

### Problem

Pantograph does not yet provide an in-GUI way to inspect workflow execution in
terms of queue time, runtime load/reuse, per-node timing, graph invalidation,
or the raw event stream that drives execution state. Without that surface,
execution work remains difficult to measure and diagnose, which weakens the
next roadmap items such as parallel demand execution, scheduler work, and
incremental graph execution.

### Constraints

- Follow `PLAN-STANDARDS.md` for structure and execution.
- Follow `ARCHITECTURE-PATTERNS.md` and `CODING-STANDARDS.md` layered
  separation rules; GUI presentation must not own service/runtime business
  logic.
- Follow `FRONTEND-STANDARDS.md`; prefer event-driven synchronization over
  polling, keep rendering declarative, and add interaction checks for embedded
  controls inside the graph/canvas environment.
- Follow `CONCURRENCY-STANDARDS.md`; ownership of subscriptions, timers, and
  background event drains must be explicit and race-safe.
- Follow `TESTING-STANDARDS.md`; add at least one cross-layer acceptance path
  from trace production through adapter binding into GUI rendering.
- Follow `DOCUMENTATION-STANDARDS.md`; touched source directories must update
  README rationale, not only file listings.
- Keep the diagnostics view additive; existing workflow execution commands,
  stores, and toolbar actions should remain compatible.
- Prefer in-memory diagnostics state in v1 unless a later milestone justifies
  durable trace persistence.

### Assumptions

- The diagnostics view will be developer-facing and internal to the current GUI,
  not a user-facing product feature.
- The first useful GUI slice is `Overview`, `Timeline`, and `Events`; richer
  `Scheduler`, `Runtime`, and `Graph` tabs can remain additive follow-up within
  the same plan.
- Existing workflow ids, session ids, run ids, and execution ids can be aligned
  into a stable trace correlation model without an API break.
- Event-driven diagnostics synchronization is feasible through the existing
  workflow backend/event subscription shape.

### Dependencies

- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/ARCHITECTURE-PATTERNS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CODING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/FRONTEND-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CONCURRENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TESTING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`
- `crates/node-engine`
- `crates/pantograph-workflow-service`
- `crates/pantograph-embedded-runtime`
- `src-tauri/src/workflow`
- `src/backends/TauriWorkflowBackend.ts`
- `src/services/workflow`
- `src/stores`
- `src/components`
- `packages/svelte-graph`

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Diagnostics logic leaks business rules into Svelte components | High | Keep contracts and aggregation in service/store layers; components remain presentational. |
| Frontend introduces polling loops to approximate runtime state | High | Require event-driven updates by default; document and justify any unavoidable pull boundary. |
| Trace contracts drift between Rust producers, Tauri adapters, and TS consumers | High | Freeze additive DTO contracts before GUI work; add contract and acceptance tests. |
| Long-lived subscriptions or timers leak across workflow/session changes | High | Assign a single diagnostics store owner with explicit start/stop lifecycle and deterministic cleanup tests. |
| Diagnostics work breaks existing workflow toolbar or event consumers | Medium | Preserve current facades and add the view as an additive entrypoint. |
| Trace payload volume overwhelms the GUI or causes memory growth | Medium | Scope v1 to current workflow/session, bound retained event history, and add filtering policies. |
| The view becomes a dashboard that is hard to use for debugging | Medium | Start with linked `Overview`, `Timeline`, and `Events` rather than broad charting. |

### Affected Structured Contracts

- Workflow run trace DTOs
- Node execution trace DTOs
- Workflow event payloads surfaced to the frontend
- Scheduler/runtime diagnostic payloads
- Trace correlation identifiers (`workflow_id`, `session_id`, `run_id`,
  `execution_id`, `node_id`, `runtime_id`, graph revision/fingerprint)
- Frontend diagnostics store shape and tab/view-model contracts

### Affected Persisted Artifacts

- None required in v1.
- Reason: diagnostics state should remain in-memory and session-scoped unless a
  later milestone justifies durable trace persistence.
- Revisit trigger: export/replay requirements or restart-resilient diagnostics
  become a product requirement.

### Concurrency / Race-Risk Review

- Workflow execution events, runtime lifecycle events, and GUI tab state can
  update concurrently; one diagnostics store must own normalization and
  lifecycle transitions for the current workflow/session scope.
- Subscription start/stop must be deterministic on workflow change, session
  change, diagnostics view close, and app teardown.
- If a transport pull boundary exists, it must be isolated to the adapter/store
  boundary and must not become a global frontend polling loop.
- Related trace state such as selected run, retained events, and derived run
  summaries must move together under one owner to avoid partial updates.
- Trace retention must be bounded so unbounded event streams do not create
  memory pressure during long sessions.

### Ownership and Lifecycle Note

- Rust producers own trace emission for execution, runtime, and scheduler
  events.
- `pantograph-workflow-service` owns orchestration-level summaries and queue
  diagnostics that belong to the application-service layer.
- Tauri owns only transport mapping and subscription bridging.
- The frontend diagnostics service/store owns:
  - subscription start/stop
  - in-memory retention bounds
  - trace normalization into view models
  - selection state for workflow/session/run/node/runtime focus
- Svelte components render store state declaratively and must not own background
  subscriptions directly unless they are thin wrappers over the diagnostics
  store lifecycle.

### Public Facade Preservation Note

- Preserve existing workflow execution and toolbar facades; diagnostics is an
  additive GUI feature.
- Preserve the existing workflow backend subscription shape where possible;
  extend it additively for richer trace payloads.
- Avoid API-breaking rewrites in v1. Any contract expansion must be additive and
  version-compatible unless a separate ADR approves a break.

## Clarifying Questions (Only If Needed)

- None at this time.

## Definition of Done

- Pantograph has a developer-facing diagnostics view reachable from the existing
  workflow GUI.
- The view renders `Overview`, `Timeline`, and `Events` for the selected
  workflow/session/run without relying on ad hoc console logging.
- Trace contracts are additive, documented, and consistent across Rust, Tauri,
  and TypeScript boundaries.
- Frontend diagnostics synchronization is event-driven by default and has
  deterministic cleanup behavior.
- At least one cross-layer acceptance check verifies trace production through
  adapter binding into GUI rendering.
- Touched READMEs and any needed ADR updates are completed per documentation
  standards.

## Milestones

### Milestone 1: Freeze Diagnostics Boundary and Contracts

**Goal:** Define the diagnostics data model and ownership boundaries before
implementation spreads across layers.

**Tasks:**
- [x] Define the diagnostics contract families:
  - workflow run trace
  - node execution trace
  - workflow event record
  - runtime lifecycle trace
  - scheduler decision record
  - graph mutation trace
- [x] Freeze the correlation identifiers required across layers.
- [x] Decide which fields are stable versus intentionally volatile.
- [x] Record lifecycle ownership for producers, adapters, frontend service/store,
      and GUI components.
- [x] Decide whether a dedicated ADR is required for the diagnostics boundary or
      whether README updates are sufficient.

**Verification:**
- Manual contract review against `ARCHITECTURE-PATTERNS.md` and
  `CODING-STANDARDS.md`.
- Manual traceability review against `DOCUMENTATION-STANDARDS.md`.
- Confirm the contracts are additive and facade-preserving.

**Status:** Completed

### Milestone 2: Add Backend and Service Diagnostics Producers

**Goal:** Produce structured trace data in the correct Rust layers without
moving business logic into Tauri or the GUI.

**Tasks:**
- [ ] Add execution-domain trace production in `crates/node-engine`.
- [ ] Add orchestration/scheduler diagnostic production in
      `crates/pantograph-workflow-service`.
- [ ] Add runtime lifecycle diagnostics in
      `crates/pantograph-embedded-runtime` and/or `crates/inference`.
- [ ] Keep transport-free aggregation in service/runtime crates rather than
      emitting GUI-specific payloads from domain layers.
- [ ] Bound any retained trace history or snapshot generation so producer logic
      stays memory-safe.

**Verification:**
- `cargo check --workspace`
- Targeted Rust tests for trace DTO shape and producer behavior
- Cross-check against `CONCURRENCY-STANDARDS.md` for ownership and bounded
  in-memory retention

**Status:** Not started

### Milestone 3: Add Tauri Transport and Frontend Diagnostics Service Boundary

**Goal:** Bridge diagnostics into the GUI through additive adapters and a
single frontend owner.

**Tasks:**
- [x] Extend Tauri workflow transport with additive diagnostics payloads or
      retrieval endpoints as needed.
- [x] Add a dedicated frontend diagnostics service/store boundary rather than
      pushing trace normalization into `WorkflowToolbar.svelte` or app shell
      components.
- [x] Ensure subscription lifecycle is owned in one place and cleaned up on
      workflow/session/view transitions.
- [x] Bound retained event history and define filtering semantics.
- [x] Preserve existing workflow toolbar/event consumers during rollout.

**Verification:**
- `npm run typecheck`
- `npm run lint:full`
- Frontend lifecycle tests proving subscriptions/timers stop deterministically
- Manual review against `FRONTEND-STANDARDS.md` and
  `CONCURRENCY-STANDARDS.md`

**Status:** Completed

### Milestone 4: Add GUI Diagnostics View V1

**Goal:** Ship the first useful diagnostics view inside the existing GUI.

**Tasks:**
- [x] Add a diagnostics entrypoint from the existing workflow experience
      (`WorkflowToolbar.svelte` or adjacent shell entrypoint).
- [x] Implement `Overview`, `Timeline`, and `Events` tabs.
- [x] Add linked selection behavior between run summary, timeline rows, and
      event details.
- [x] Keep rendering declarative; do not drive the UI through imperative DOM
      mutation.
- [x] Keep gesture-heavy interactions compatible with the existing canvas
      environment.

**Verification:**
- `npm run typecheck`
- `npm run lint:full`
- Frontend tests for tab rendering, linked selection, and cleanup behavior
- Embedded interactive control smoke checks per `FRONTEND-STANDARDS.md`
- Accessibility-oriented selector checks for toolbar entrypoints and tab
  navigation

**Status:** Completed

### Milestone 5: Extend to Runtime, Scheduler, and Graph Diagnostics

**Goal:** Add deeper diagnostic tabs without violating layering or creating a
second source of truth for graph/runtime state.

**Tasks:**
- [ ] Add `Scheduler` tab backed by service-layer queue/admission diagnostics.
- [ ] Add `Runtime` tab backed by runtime lifecycle/state diagnostics.
- [ ] Add `Graph` tab or overlays backed by backend-owned graph revision and
      mutation traces rather than UI-local heuristics.
- [ ] Ensure graph overlays remain additive to existing graph rendering and do
      not create parallel graph state.
- [ ] Keep tab state and trace selection under the diagnostics store owner.

**Verification:**
- `npm run typecheck`
- `npm run lint:full`
- Targeted GUI tests for tab-specific rendering and linked navigation
- Cross-layer acceptance check covering scheduler/runtime/graph diagnostic data
  from producer to rendered view

**Status:** Not started

### Milestone 6: Hardening, Documentation, and Release Readiness

**Goal:** Close the diagnostics view with standards-compliant tests,
documentation, and rollout guidance.

**Tasks:**
- [x] Add README updates for all touched `src/` or equivalent source
      directories.
- [ ] Add or update ADR documentation if the diagnostics boundary becomes a
      stable architectural surface.
- [x] Add retention, performance, and non-goal notes so the view does not
      silently become an analytics subsystem.
- [ ] Add benchmark or stress checks for high-event-volume sessions if needed.
- [x] Confirm no milestone introduced file-size or multi-responsibility drift
      without decomposition review.

**Verification:**
- `npm run check`
- Targeted `cargo test` runs for touched Rust crates
- Cross-layer acceptance checks per `TESTING-STANDARDS.md`
- Manual documentation review against `DOCUMENTATION-STANDARDS.md`

**Status:** In progress

## Execution Notes

Update during implementation:
- 2026-04-12: Plan created for the GUI diagnostics view as the first concrete
  implementation slice of the roadmap's metrics/trace spine work.
- 2026-04-12: Plan intentionally keeps diagnostics additive and scoped to the
  existing workflow GUI rather than proposing a separate analytics product.
- 2026-04-12: Completed additive event-contract expansion across Tauri and
  TypeScript boundaries in commit `8d2e99e`.
- 2026-04-12: Completed the frontend diagnostics service/store boundary with
  bounded retention and targeted tests in commit `62e2847`.
- 2026-04-12: Completed the in-GUI diagnostics panel with `Overview`,
  `Timeline`, and `Events` tabs in commit `31eb452`.
- 2026-04-12: Updated architecture/docs traceability and recorded remaining
  follow-on work for backend runtime, scheduler, and graph diagnostics.

## Commit Cadence Notes

- Commit when a logical slice is complete and verified.
- Keep boundary-freeze, contract, backend producer, adapter/store, and GUI
  rendering work in separate reviewable commits where possible.
- Follow commit format/history cleanup rules from `COMMIT-STANDARDS.md`.

## Optional Subagent Assignment

Use only if needed.

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| None | None | None | None |

## Re-Plan Triggers

- The diagnostics feature needs durable trace persistence rather than in-memory
  state.
- Existing workflow backend contracts cannot carry the required additive trace
  payloads cleanly.
- The GUI needs polling for correctness because event-driven delivery is
  insufficient.
- The diagnostics view forces an API break in workflow execution or event
  contracts.
- The initial `Overview`/`Timeline`/`Events` scope proves insufficient for a
  useful first release and changes sequencing materially.

## Recommendations (Only If Better Option Exists)

- Recommendation 1: Ship `Overview`, `Timeline`, and `Events` before
  `Scheduler`, `Runtime`, and `Graph`.
  Why: This delivers the minimum useful debugging surface earlier, keeps risk
  lower, and preserves architecture while the trace contracts settle.
  Impact: Shorter time to first usable diagnostics view and less UI churn during
  early producer work.

- Recommendation 2: Keep diagnostics state session-scoped and in-memory in v1.
  Why: It avoids premature persistence design and keeps the feature aligned with
  the roadmap's immediate debugging purpose.
  Impact: Simpler scope and lower concurrency/persistence risk in the first
  implementation cycle.

## Completion Summary

### Completed

- Milestone 1: diagnostics boundary, identifiers, ownership notes, and README
  decisions frozen.
- Milestone 3: additive Tauri/frontend transport plus dedicated diagnostics
  service/store boundary implemented.
- Milestone 4: workflow diagnostics panel shipped with linked run/node
  selection across `Overview`, `Timeline`, and `Events`.
- README updates completed for touched frontend source directories.

### Deviations

- Milestone 2 remains open because backend-owned runtime, scheduler, and graph
  producer surfaces were not expanded beyond the additive workflow-event
  transport needed for the initial GUI.
- Milestone 5 remains open; `Scheduler`, `Runtime`, and `Graph` are shipped as
  explicit placeholders rather than completed tabs.
- No browser-automation acceptance test was added; current cross-layer coverage
  is contract/service/store plus rendered-panel integration through typecheck,
  lint, and targeted frontend tests.

### Follow-Ups

- Add backend/service diagnostics producers for scheduler, runtime, and graph
  invalidation state.
- Replace placeholder tabs with real scheduler/runtime/graph traces once the
  corresponding roadmap items publish stable diagnostics contracts.
- Add a broader acceptance check once a frontend harness exists for embedded
  workflow-panel interactions.

### Verification Summary

- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run typecheck`
- `npm run lint:full`
- `npm run test:frontend`

### Traceability Links

- Module README updated:
  - `src/services/README.md`
  - `src/services/diagnostics/README.md`
  - `src/stores/README.md`
  - `src/components/README.md`
  - `src/components/diagnostics/README.md`
- ADR added/updated: N/A yet
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A yet

## Brevity Note

Keep the plan concise. Expand detail only where execution decisions or risk
require it.
