# Plan: Diagnostics Push Projection And SQLite Locking

## Objective

Eliminate intermittent diagnostics refresh failures caused by SQLite lock
contention, and replace constant or repeated diagnostics polling with a
backend-owned push invalidation path. The backend remains the source of truth
for diagnostics events, projection state, run detail, scheduler timeline, node
status, and IO artifact facts; the frontend subscribes to backend changes and
renders refreshed projections.

## Scope

### In Scope

- Configure diagnostics SQLite connections for concurrent app reads and writes.
- Split live diagnostics projection state from durable diagnostics ledger
  ownership so Tauri can keep runtime/scheduler/debug overlays without opening
  an uncoordinated durable writer.
- Move diagnostics projection draining out of read query commands and into a
  backend-owned projection refresh owner.
- Add durable projection health metadata to the diagnostics ledger
  `projection_state` table without recording projection failures as workflow
  diagnostic events.
- Add backend-owned diagnostics projection invalidation/update events.
- Add a Tauri transport bridge that forwards backend diagnostics update events
  without defining diagnostics semantics.
- Update Diagnostics, Scheduler, Network, IO Inspector, and Library refresh
  paths to use event-driven projection refreshes where they currently refresh
  diagnostics projections on page load, active-run change, or user action.
- Keep manual refresh buttons as fallback actions.
- Add focused contention, projection, Tauri transport, and frontend subscription
  tests.
- Update touched READMEs when ownership boundaries change.

### Out Of Scope

- Replacing the diagnostics ledger, diagnostics projections, ArtifactStore, or
  retention policy.
- Moving diagnostics truth into Tauri commands, frontend stores, or Svelte
  components.
- Reintroducing a frontend diagnostics accumulation store.
- Changing workflow execution, scheduler admission, inference backend
  selection, or artifact retention semantics except where they emit diagnostics
  update notifications.
- Replacing SQLite with another database.
- Recording projection refresh failures as workflow, inference, client, node,
  scheduler, or artifact diagnostic events.

## Inputs

### Problem

Refreshing the Diagnostics page can intermittently report:

`Internal error: diagnostics ledger storage error: database is locked`

The user also wants diagnostics-related UI pages to update automatically from
backend changes instead of relying on constant polling or repeated manual
refresh.

### Current Codebase Findings

- `src-tauri/src/app_setup.rs` opens `.pantograph/workflow-diagnostics.sqlite`
  twice: once for `WorkflowService` and once for `WorkflowDiagnosticsStore`.
- `WorkflowService` wraps its `SqliteDiagnosticsLedger` in one process-local
  mutex, but a second `SqliteDiagnosticsLedger` connection bypasses that mutex.
- Diagnostics workbench pages call several projection query commands per
  refresh. `DiagnosticsPage.svelte` currently calls `queryRunDetail` and
  `querySchedulerTimeline`, then `queryRunList`, `queryNodeStatus`, and
  `queryIoArtifacts`.
- `NetworkPage.svelte`, `SchedulerPage.svelte`, and `LibraryPage.svelte` also
  consume projection queries, including library usage and scheduler estimate
  projections.
- Projection query commands are not pure reads today. Methods including
  `workflow_run_detail_query`, `workflow_run_list_query`,
  `workflow_scheduler_timeline_query`, `workflow_node_status_query`,
  `workflow_io_artifact_query`, `workflow_scheduler_estimate_query`, and
  `workflow_library_usage_query` drain projections before reading them.
- Draining projections writes to SQLite. A diagnostics page refresh can
  therefore perform several write transactions while workflow execution is also
  appending diagnostics events.
- `SqliteDiagnosticsLedger::open` currently initializes the connection but does
  not visibly configure WAL mode or a busy timeout.
- Frontend standards prefer event-driven synchronization over polling when
  event or subscription hooks are feasible.

### Constraints

- Follow
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.
- Backend-owned diagnostics data remains authoritative.
- Tauri is an IPC and desktop event transport adapter, not the diagnostics
  semantic owner.
- Frontend may own transient UI state and subscription lifecycle, but must not
  own diagnostics facts or projection semantics.
- Projection refresh work must have a clear lifecycle owner, bounded work, and
  cancellation or shutdown behavior.
- Avoid holding synchronous mutex guards across async waits.
- Avoid hidden writes in read query commands once the projection refresh owner
  exists.
- Keep changes in thin validated vertical slices with atomic commits during
  implementation.

### Assumptions

- The intermittent lock error is caused by a combination of concurrent SQLite
  connections and projection-drain writes triggered by UI refresh reads.
- WAL plus busy timeout is a necessary short-term hardening step, but not
  sufficient as the final design.
- Existing diagnostics projection tables remain the correct read model.
- A compact invalidation event is preferable to pushing full diagnostics
  projection payloads through Tauri events.
- Manual refresh remains useful for recovery, debugging, and missed events.
- Push events are invalidations, not durable data. A UI event only means the
  displayed projection may be stale and should be re-read from the backend.

### Dependencies

- `crates/pantograph-diagnostics-ledger`
- `crates/pantograph-workflow-service`
- `src-tauri/src/app_setup.rs`
- `src-tauri/src/workflow/*`
- `src/services/workflow/*`
- `src/components/workbench/DiagnosticsPage.svelte`
- `src/components/workbench/SchedulerPage.svelte`
- `src/components/workbench/NetworkPage.svelte`
- `src/components/workbench/IoInspectorPage.svelte`
- `src/components/workbench/LibraryPage.svelte`

### Affected Contracts

- New backend diagnostics projection update DTO.
- New or extended workflow-service diagnostics subscription/invalidation
  facade.
- New typed projection-kind contract shared by workflow-service DTOs, Tauri
  transport payloads, and frontend service types.
- New workflow-service projection refresh request/result contract.
- New Tauri event payload, transported from backend DTOs.
- Frontend service subscription API for diagnostics projection invalidations.
- Additive frontend `ProjectionStateRecord` health fields.

### Affected Persisted Artifacts

- Existing `.pantograph/workflow-diagnostics.sqlite`.
- SQLite PRAGMA behavior for the diagnostics database.
- Existing projection tables and `projection_state` rows.
- `projection_state` gains durable operational health columns for projection
  refresh failures. These fields describe materialized read-model maintenance,
  not workflow-run, inference, client, node, scheduler, or artifact facts.
- The migration must be additive: existing rows remain valid and new health
  columns default to `NULL`.

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| WAL/busy timeout masks a deeper long transaction | Medium | Add contention tests and continue with projection-owner refactor. |
| Projection worker misses an event | High | Use ledger `event_seq` cursors and projection state as the durable source; notifications are invalidations, not truth. |
| Frontend misses a Tauri event while inactive | Medium | On mount and active-run change, fetch an initial snapshot; event stream only keeps it fresh afterward. |
| Too many backend events cause UI refresh storms | Medium | Coalesce projection update events by run id/projection kind and debounce frontend refreshes. |
| Moving projection drains breaks existing query expectations | High | Add tests proving read commands return existing materialized projections and projection worker advances state. |
| Second diagnostics ledger owner still writes to the same file | High | Audit `WorkflowDiagnosticsStore` usage and route durable timing/trace needs through one backend diagnostics owner or an explicit shared facade. |
| Background task shutdown leaks handles | Medium | Add a lifecycle owner with tracked task handle and deterministic shutdown tests. |
| UI misses invalidation events while inactive or not mounted | Medium | Initial page load and active-run changes always fetch backend projections; manual refresh remains. |
| Projection refresh fails but UI assumes success | High | Emit invalidations only after successful projection advancement and expose projection health/error metadata. |
| Multiple app windows duplicate refresh pressure | Medium | Backend owns one refresh worker; Tauri broadcasts compact invalidations and each window filters locally. |
| Startup opens with stale projections | Medium | Startup performs bounded catch-up or marks projection state stale until the refresh owner advances it. |
| Projection failures pollute workflow diagnostics | Medium | Store projection errors only on `projection_state`; do not append diagnostic events for projection maintenance failures. |
| Durable projection error metadata becomes stale | Medium | Clear `last_error` fields on successful refresh or rebuild and cover with migration/projection tests. |

## Definition Of Done

- Diagnostics SQLite connections use WAL and bounded busy timeout.
- App startup no longer creates competing uncoordinated durable diagnostics
  ledger owners for the same file.
- Diagnostics projection query commands do not perform hidden projection drains.
- A backend-owned projection refresh owner advances projections after diagnostic
  events are appended or after explicit rebuild/refresh requests.
- Projection refresh requests are coalesced and drained in bounded batches
  rather than running one full projection drain per diagnostic event.
- Projection refresh failures are stored as durable `projection_state`
  operational health metadata and are not appended as workflow diagnostic
  events.
- Backend emits compact diagnostics projection update events after projection
  state advances.
- Tauri forwards backend diagnostics update events without owning diagnostics
  semantics.
- Workbench diagnostics consumers subscribe to backend invalidations and refresh
  affected projections automatically.
- Manual refresh remains available.
- Initial page load and active-run changes fetch snapshots even when no event
  has been received.
- Event bursts are coalesced so repeated diagnostics events for the same run
  and projection kind do not produce unbounded UI refreshes.
- Stale frontend responses are discarded when active run or request serial
  changes.
- Projection refresh failures are visible through backend projection metadata
  instead of silent event loss.
- Successful projection refreshes clear stale durable projection error metadata.
- Tests cover ledger contention, projection-owner behavior, event transport,
  frontend subscription cleanup, and at least one cross-layer update path.
- Touched module READMEs document the new ownership and lifecycle boundaries.

## Milestones

### Milestone 1: SQLite Lock Hardening

**Goal:** Reduce immediate lock failures while preserving current behavior.

**Tasks:**

- [x] Add a focused diagnostics SQLite connection configuration helper in
      `pantograph-diagnostics-ledger`.
- [x] Configure `foreign_keys = ON`, `journal_mode = WAL`,
      `busy_timeout = 5000`, and `synchronous = NORMAL` for file-backed
      diagnostics ledgers.
- [x] Keep in-memory test ledgers compatible.
- [x] Add tests with two file-backed ledger connections where one connection
      writes and another reads or waits instead of immediately returning
      `database is locked`.
- [x] Record any long-transaction or retry behavior discovered during tests in
      this plan.

**Verification:**

- `cargo test -p pantograph-diagnostics-ledger sqlite`
- Targeted contention test by exact name.
- `cargo check -p pantograph-diagnostics-ledger`

**Status:** Completed 2026-05-09.

### Milestone 2: Live Diagnostics And Durable Ledger Ownership Split

**Goal:** Preserve Tauri live diagnostics overlays while removing the second
uncoordinated durable diagnostics ledger owner.

**Responsibility split:**

| Responsibility | Owner after this milestone |
| -------------- | -------------------------- |
| Live runtime/scheduler snapshots | `WorkflowDiagnosticsStore` |
| Retained GUI/debug overlays | `WorkflowDiagnosticsStore` |
| Tauri diagnostics snapshot event payload shaping | `WorkflowDiagnosticsStore` and Tauri event adapter |
| Durable timing observations | Workflow-service diagnostics persistence facade |
| Durable workflow run summaries | Workflow-service diagnostics persistence facade |
| Durable node-status diagnostic events | Workflow-service diagnostics persistence facade |
| Diagnostics SQLite connection ownership | Workflow-service diagnostics owner |

**Tasks:**

- [ ] Audit `WorkflowDiagnosticsStore` and Tauri workflow diagnostics modules
      for durable ledger reads/writes still needed by active UI paths.
- [ ] Keep `WorkflowDiagnosticsStore` as the live in-memory runtime,
      scheduler, trace, and debug-overlay projection boundary where those paths
      are still needed.
- [ ] Add or reuse a narrow workflow-service diagnostics persistence facade for
      durable timing, run-summary, and node-status event writes.
- [ ] Route `WorkflowDiagnosticsStore` durable write needs through that facade
      instead of giving it its own `SqliteDiagnosticsLedger`.
- [ ] Update `src-tauri/src/app_setup.rs` so app startup does not open two
      independent durable diagnostics ledger connections for the same file
      unless they are explicitly configured and owned.
- [ ] Add tests or startup assertions covering single-owner setup.
- [ ] Update Tauri workflow diagnostics README with the new boundary.

**Verification:**

- `cargo check --manifest-path src-tauri/Cargo.toml`
- Targeted Tauri workflow diagnostics tests.
- `cargo test -p pantograph-workflow-service diagnostics`

**Status:** Not started.

### Milestone 3: Backend Projection Refresh Owner And Durable Health Contract

**Goal:** Define the backend-owned projection refresh and invalidation contract
before moving query behavior, including durable operational health for
projection maintenance failures.

**Additive migration contract:**

```sql
ALTER TABLE projection_state ADD COLUMN last_error TEXT;
ALTER TABLE projection_state ADD COLUMN last_error_at_ms INTEGER;
ALTER TABLE projection_state ADD COLUMN last_failed_event_seq INTEGER;
```

- Existing rows with `NULL` health fields are valid.
- Successful projection drain, refresh, or rebuild clears all three health
  fields.
- Failed projection refresh sets `status = failed`, preserves the last
  successful `last_applied_event_seq`, and records `last_error`,
  `last_error_at_ms`, and `last_failed_event_seq`.
- Projection maintenance failures must not append diagnostic events.
- Implement projection-state SQL through centralized helpers instead of
  repeating success/failure `INSERT ... ON CONFLICT` statements in every drain:
  `query_projection_state`, `write_projection_success_state`, and
  `write_projection_failure_state`.
- The health-column migration should be available through an idempotent helper,
  such as `ensure_projection_state_health_columns`, used by both versioned
  migration and current-version schema repair.

**Workflow-service refresh contract sketch:**

```rust
pub enum DiagnosticsProjectionKind {
    SchedulerTimeline,
    RunList,
    RunDetail,
    IoArtifact,
    NodeStatus,
    LibraryUsage,
}

pub enum DiagnosticsProjectionRefreshReason {
    DiagnosticEventAppended,
    ExplicitRefresh,
    ProjectionRebuild,
    StartupCatchUp,
    RetentionCleanup,
}

pub struct DiagnosticsProjectionRefreshRequest {
    pub projections: Vec<DiagnosticsProjectionKind>,
    pub workflow_run_id: Option<String>,
    pub workflow_id: Option<String>,
    pub reason: DiagnosticsProjectionRefreshReason,
    pub batch_size: u32,
}

pub struct DiagnosticsProjectionRefreshResult {
    pub advanced: Vec<DiagnosticsProjectionAdvance>,
    pub failed: Vec<DiagnosticsProjectionFailure>,
    pub invalidations: Vec<DiagnosticsProjectionInvalidation>,
}
```

- `pantograph-workflow-service` owns this contract and synchronous refresh
  semantics.
- Tauri may own async task scheduling and event broadcast, but not projection
  semantics.
- Refresh work is bounded by `batch_size` and may be called repeatedly until
  projection state is current.
- Query methods must not call this refresh API internally.
- Invalidations are emitted only for successful projection advancement.
- `workflow_scheduler_estimate_query` reads the `RunDetail` projection and must
  subscribe to or refresh `DiagnosticsProjectionKind::RunDetail`; it is not a
  separate materialized projection kind.
- IO artifact retention summaries read the `IoArtifact` projection and must
  subscribe to or refresh `DiagnosticsProjectionKind::IoArtifact`; retention
  summary is not a separate materialized projection kind.
- The public `workflow_projection_rebuild` command may keep its current string
  request shape for the first implementation pass, but workflow-service should
  map that string into the typed projection-kind contract internally.

**Tasks:**

- [x] Add a diagnostics-ledger schema migration that extends
      `projection_state` with durable operational health fields:
      `last_error`, `last_error_at_ms`, and `last_failed_event_seq`.
- [x] Add migration tests from a pre-health `projection_state` schema fixture.
- [x] Add centralized projection-state read/write helpers and route projection
      drains, rebuilds, and refresh failures through them.
- [x] Update `ProjectionStateRecord` and `ProjectionStateUpdate` so health
      fields can be recorded, cleared, serialized, and queried consistently.
- [x] Ensure successful projection refreshes and rebuilds clear stale projection
      error metadata.
- [x] Ensure failed projection refreshes mark the affected projection as
      `failed` and persist the error metadata without appending a diagnostic
      event.
- [x] Add DTOs for diagnostics projection update/invalidation events, including
      `workflow_run_id`, `last_event_seq`, affected typed projection kinds, and
      refresh reason.
- [x] Define typed projection-kind DTOs instead of adding new stringly typed
      projection names at workflow-service, Tauri, or frontend boundaries.
- [x] Add internal mapping from current string projection names to typed
      projection kinds for existing rebuild and compatibility paths.
- [x] Define projection health metadata for stale or failed refreshes using the
      durable `projection_state` health fields.
- [x] Add a workflow-service projection refresh owner interface that can drain
      selected projections by batch size.
- [x] Keep refresh logic synchronous in workflow-service core unless async is
      required by the outer Tauri event transport.
- [x] Add tests proving the refresh owner advances projection state and returns
      affected projection names.
- [ ] Add tests proving failed refreshes do not emit success invalidations and
      do update projection health/error state.
- [ ] Add tests proving projection refresh failures do not append workflow
      diagnostic events.
- [x] Document that notifications are invalidations and projection queries
      remain the source of full facts.

**Verification:**

- `cargo test -p pantograph-diagnostics-ledger projection_state`
- `cargo test -p pantograph-workflow-service diagnostics_projection_refresh`
- `cargo check -p pantograph-diagnostics-ledger`
- `cargo check -p pantograph-workflow-service`

**Status:** In progress. The workflow-service core now has typed projection
kinds, refresh reasons, refresh request/response DTOs, successful invalidation
payloads, internal string-name mapping for rebuild, and a synchronous bounded
refresh method. Remaining work in this milestone is focused failure-path
coverage for refresh errors.

### Milestone 4: Read-Only Projection Query Slice

**Goal:** Make one diagnostics read path query materialized projections without
hidden writes, while a refresh owner updates those projections.

**Tasks:**

- [x] Convert one vertical slice first, preferably selected-run detail:
      refresh owner drains run detail and node status; `workflow_run_detail_query`
      only reads.
- [x] Add a test that fails if `workflow_run_detail_query` mutates projection
      state.
- [x] Add a full-path test: append diagnostics event, run refresh owner, query
      run detail, observe updated projection.
- [x] Add a stale-start test where a page/query can detect projection lag from
      projection state before the refresh owner catches up.
- [x] Keep current command response DTO shape stable.
- [x] Record any projection freshness behavior in response metadata.
- [x] Update tests that currently encode "query drains projection" behavior so
      they explicitly refresh first and then assert read-only query behavior.

**Verification:**

- `cargo test -p pantograph-workflow-service workflow_run_detail_query`
- Targeted mutation/freshness tests.
- `cargo check -p pantograph-workflow-service`

**Status:** Completed 2026-05-09. `workflow_run_detail_query` now reads
materialized `RunDetail` and `NodeStatus` projections without draining them,
returns non-persisted `NeedsRebuild` projection state when no materialized
state exists, and direct tests explicitly refresh through the projection
refresh owner before reading.

### Milestone 5: Expand Read-Only Queries Horizontally

**Goal:** Move all diagnostics projection query commands off hidden drains.

**Tasks:**

- [x] Convert scheduler timeline query to read-only.
- [x] Convert run list and facets query to read-only.
- [ ] Convert IO artifact query and retention summary query to read-only.
- [x] Convert node status query to read-only.
- [ ] Convert scheduler estimate query to read-only.
- [ ] Convert library usage query to read-only.
- [ ] Keep explicit projection rebuild/refresh commands as the write path.
- [ ] Keep startup repair, projection rebuild, retention cleanup, and explicit
      refresh as named maintenance/write paths rather than hidden query work.
- [ ] Update tests that currently expect query commands to drain projections.

**Verification:**

- `cargo test -p pantograph-workflow-service diagnostics`
- `cargo test -p pantograph-workflow-service workflow_run_inspection_query`
- `cargo test -p pantograph-workflow-service workflow_library_usage_query`
- `cargo check -p pantograph-workflow-service`

**Status:** In progress. Scheduler timeline, run list and facets, and node
status now read materialized projection state without hidden drains and use the
refresh owner in tests.

### Milestone 6: Backend Push Invalidation Transport

**Goal:** Emit compact projection update events from backend-owned refreshes to
the desktop frontend.

**Tasks:**

- [ ] Add a Tauri event bridge that subscribes to workflow-service projection
      update events.
- [ ] Ensure Tauri payloads mirror backend DTOs and do not create new
      diagnostics semantics.
- [ ] Coalesce bursts by projection kind and workflow run id.
- [ ] Broadcast one backend invalidation to all open windows while keeping one
      backend refresh owner.
- [ ] Ensure invalidations are emitted only after projection state advances.
- [ ] Ensure event sender lifecycle is started at app setup and stopped on app
      shutdown.
- [ ] Add transport tests for payload shape and coalescing.
- [ ] Add lifecycle tests for shutdown/cleanup of the bridge owner.

**Verification:**

- `cargo check --manifest-path src-tauri/Cargo.toml`
- Targeted `src-tauri` workflow event adapter tests.

**Status:** Not started.

### Milestone 7: Frontend Subscription Service

**Goal:** Provide a typed frontend service API for diagnostics invalidation
events with deterministic cleanup.

**Frontend helper contract:**

```ts
type DiagnosticsProjectionSubscriptionOptions = {
  projections: DiagnosticsProjectionKind[];
  getActiveRunId?: () => string | null;
  refresh: (event: DiagnosticsProjectionInvalidation) => Promise<void> | void;
};
```

- The helper owns Tauri `listen`, payload normalization, projection-kind/run
  filtering, debounce/coalescing, and unsubscribe cleanup.
- Pages keep ownership of selected run state, loading/error state, query
  selection, rendering, and manual refresh buttons.
- The helper must not cache diagnostics facts or become a frontend diagnostics
  truth store.
- Add optional health fields to the frontend `ProjectionStateRecord` mirror:
  `last_error`, `last_error_at_ms`, and `last_failed_event_seq`.
- Add a small `mockProjectionState` helper for frontend service mocks and tests
  so additive projection-state fields are not repeated across every fixture.
- Add or extend one shared projection-health/freshness presenter utility so
  pages render failed/stale projection state consistently.

**Tasks:**

- [ ] Add a diagnostics projection subscription helper in the frontend workflow
      service boundary.
- [ ] Normalize Tauri event payloads into typed diagnostics invalidation DTOs.
- [ ] Extend frontend diagnostics types with optional projection health fields.
- [ ] Add `mockProjectionState` or equivalent helper for mock projection
      responses and affected tests.
- [ ] Add or extend shared projection health/freshness presentation helpers.
- [ ] Provide unsubscribe cleanup and stale-response guards.
- [ ] Preserve initial snapshot fetches on page mount and active-run changes.
- [ ] Debounce/coalesce refresh requests per active run and projection kind.
- [ ] Ignore invalidations that do not affect the active page/run/filter scope.
- [ ] Add unit tests proving subscriptions call listeners, coalesce refreshes
      where owned by the service, and unsubscribe cleanly.
- [ ] Add unit tests proving missed events are recovered by initial snapshot and
      manual refresh paths.
- [ ] Update `src/services/workflow/README.md` or diagnostics README with the
      new event-driven boundary.

**Verification:**

- `npm run typecheck`
- `npm run test:frontend -- --runInBand` if supported, otherwise targeted
  frontend test command used by this repo.

**Status:** Not started.

### Milestone 8: Workbench Page Conversion

**Goal:** Replace diagnostics projection polling/repeated refresh behavior with
event-driven refreshes on affected pages.

**Tasks:**

- [ ] Update `DiagnosticsPage.svelte` to load initial data once on mount or
      active-run change, then subscribe to projection invalidations for the
      active run and affected comparison scopes.
- [ ] Update `SchedulerPage.svelte` run list/timeline refresh behavior to
      respond to run-list and scheduler-timeline invalidations.
- [ ] Update `NetworkPage.svelte` selected-run timeline/node-status refreshes
      and library-usage refreshes to respond to invalidations.
- [ ] Update `IoInspectorPage.svelte` selected-run graph/node/artifact refresh
      behavior to respond to invalidations.
- [ ] Update `LibraryPage.svelte` library-usage refresh behavior to respond to
      library-usage invalidations.
- [ ] Keep manual refresh buttons and active-run change refreshes.
- [ ] Remove any redundant timers or repeated polling loops introduced by these
      pages.
- [ ] Display projection stale/error metadata returned by backend projections
      without inventing frontend projection truth.
- [ ] Keep current `requestSerial` or equivalent stale-response guards on all
      refreshed page data.

**Verification:**

- `npm run typecheck`
- `npm run test:frontend`
- Focused component/service tests for subscription cleanup and stale response
  handling.

**Status:** Not started.

### Milestone 9: Cross-Layer Acceptance And Release Verification

**Goal:** Prove the backend-to-frontend event-driven diagnostics path works
without lock errors.

**Tasks:**

- [ ] Add or update a cross-layer test that appends diagnostics events, refreshes
      projections, emits an invalidation, and causes the frontend service/page
      path to refresh projection data.
- [ ] Add a contention regression that runs append, projection refresh, and
      query workloads concurrently against a temp diagnostics database.
- [ ] Add a burst regression showing many events for one run produce bounded
      refresh calls.
- [ ] Add a missed-event regression showing page initial load recovers without
      receiving an event.
- [ ] Add a retention cleanup regression showing affected run/artifact
      projections are invalidated.
- [ ] Run full affected Rust and frontend verification.
- [ ] Build release artifacts.
- [ ] Update completion summary with exact validation results.

**Verification:**

- `cargo test -p pantograph-diagnostics-ledger`
- `cargo test -p pantograph-workflow-service diagnostics`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run typecheck`
- `npm run test:frontend`
- `bash launcher.sh --build-release`

**Status:** Not started.

## Ownership And Lifecycle

- `pantograph-diagnostics-ledger` owns SQLite connection behavior, schema, and
  low-level persistence guarantees, including durable projection operational
  health stored on `projection_state`.
- `pantograph-workflow-service` owns diagnostics event append semantics,
  projection refresh semantics, projection state, typed projection-kind DTOs,
  and invalidation DTOs.
- Tauri owns event transport and app lifecycle wiring only.
- Frontend services own subscription setup/teardown and stale response guards.
- Svelte pages own initial fetch timing, selected-run UI state, loading/error
  display, and manual refresh buttons.
- The projection refresh owner must have one lifecycle owner. If implemented as
  a background task, it must store its task handle, support shutdown, coalesce
  refresh requests, and log task errors at the owner.

## Concurrency And Race Review

- Do not hold `std::sync::Mutex` guards across async waits.
- Do not let read commands perform hidden writes after Milestone 5.
- Projection refresh must be idempotent and cursor-based so duplicate
  invalidations do not corrupt state.
- Frontend event handlers must discard stale refresh results when active run or
  request serial changes.
- Notifications are not durable truth. Pages must always fetch backend
  projections after receiving a notification.
- If an event is missed, initial load, manual refresh, or next invalidation must
  recover by reading projection state from backend.
- Backend invalidations must be emitted after projection state advances, not
  merely after event append.
- Backend refresh requests must be coalesced by affected projection and
  workflow run scope so event bursts cannot create unbounded background work.
- Multiple windows may receive the same invalidation, but only one backend
  projection refresh owner may mutate projection state.
- Projection refresh failure must update backend-owned projection health and
  must not emit a success invalidation.
- Startup must either run bounded projection catch-up or expose stale/pending
  projection state until catch-up completes.
- Retention cleanup must be treated as a diagnostics projection producer and
  must invalidate run detail, IO artifact, retention summary, and affected
  scheduler/run-list projections as needed.
- Projection maintenance failures must update only projection operational
  health metadata. They must not append workflow, inference, client, scheduler,
  node, or artifact diagnostic events.
- Library usage and scheduler estimate projections must follow the same
  refresh/read-only split as the primary diagnostics projections.
- Scheduler estimate invalidations are `RunDetail` invalidations. Retention
  summary invalidations are `IoArtifact` invalidations.
- Projection-state success/failure writes should flow through shared ledger
  helpers so stale error clearing and failure persistence cannot diverge across
  projection kinds.

## Public Facade Preservation

- Existing query command names and response DTOs should remain stable through
  the first implementation pass.
- New event/subscription DTOs are additive.
- Projection health fields added to `ProjectionStateRecord` are additive and
  must remain backward-compatible for existing frontend projection state
  consumers.
- Existing `workflow_projection_rebuild` command shape should remain stable for
  the first pass while internally using typed projection-kind mapping.
- Any breaking DTO change requires a plan update and synchronized frontend,
  Tauri, and workflow-service contract changes.

## Optional Subagent Assignment

Use subagents only if thread capacity is available and the worktree is clean.

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| Ledger worker | `pantograph-diagnostics-ledger` SQLite PRAGMA and contention tests | Commit-ready patch and test results | After Milestone 1 |
| Workflow-service worker | Projection refresh owner and read-only query conversion | Commit-ready patch, changed DTO list, verification output | After Milestones 3-5 |
| Frontend worker | Subscription service and workbench page conversion | Commit-ready patch, service/component tests, cleanup notes | After Milestones 7-8 |

Shared files such as Tauri app setup, public DTO exports, command registration,
and documentation are integration-owner files and should not be edited in
parallel by multiple workers.

## Re-Plan Triggers

- SQLite remains locked after WAL/busy-timeout and single-owner changes.
- `WorkflowDiagnosticsStore` cannot be split into live-only projection state and
  workflow-service-owned durable writes without breaking active functionality.
- Projection refresh cannot be separated from query commands without changing
  public response semantics.
- Durable projection health requires more than additive `projection_state`
  fields or introduces workflow diagnostic event pollution.
- Tauri event transport cannot provide reliable lifecycle cleanup.
- Frontend pages require a shared subscription store that would recreate a
  second diagnostics truth path.
- Contention tests expose long-running write transactions that require schema or
  batching changes.

## Execution Notes

- 2026-05-09: Plan created from read-only investigation. Current evidence
  points to concurrent durable ledger owners plus projection-drain writes inside
  query commands as the likely cause of intermittent `database is locked`
  errors during diagnostics refresh.
- 2026-05-09: Plan updated so projection refresh failures are durable
  `projection_state` operational health, not workflow diagnostic events. The
  implementation target now preserves Tauri live diagnostics overlays while
  moving durable diagnostics writes behind one workflow-service-owned boundary,
  includes library usage and scheduler estimate projections, and uses typed
  projection kinds for invalidation contracts.
- 2026-05-09: Follow-up review added implementation simplifications:
  centralized ledger projection-state helpers, internal typed projection-kind
  mapping for existing string rebuild paths, `RunDetail` invalidations for
  scheduler estimate, `IoArtifact` invalidations for retention summaries, and
  frontend helper/presenter reuse for projection-state health fields.
- 2026-05-09: Milestone 1 implemented diagnostics SQLite hardening with
  file-backed `busy_timeout`, WAL journal mode, and NORMAL synchronous mode.
  Focused contention coverage proves a second writer waits for a held write
  lock instead of immediately returning `database is locked`.
- 2026-05-09: Milestone 3 ledger foundation added additive projection health
  columns, centralized projection-state success writes, schema migration and
  current-version repair coverage, and tests proving failure metadata persists
  and successful state writes clear stale error metadata.

## Commit Cadence Notes

- Commit each milestone or smaller verified vertical slice atomically.
- Commit code, tests, and plan status together when they belong to the same
  slice.
- Keep fixture repairs separate from feature commits unless the fixture is part
  of the same validated slice.

## Completion Summary

### Completed

- Milestone 1: SQLite lock hardening.
- Milestone 3 partial: diagnostics-ledger projection health schema and
  projection-state helper foundation.

### Deviations

- None.

### Follow-Ups

- None yet.

### Verification Summary

- `cargo test -p pantograph-diagnostics-ledger file_backed_connection_waits_for_busy_writer`
- `cargo check -p pantograph-diagnostics-ledger`
- `cargo test -p pantograph-diagnostics-ledger projection_state`
- `cargo test -p pantograph-diagnostics-ledger`

### Traceability Links

- Module README updates required during implementation:
  - `crates/pantograph-diagnostics-ledger/src/sqlite/README.md`
  - `crates/pantograph-workflow-service/src/workflow/README.md`
  - `src-tauri/src/workflow/README.md`
  - `src/services/workflow/README.md`
  - `src/components/workbench/README.md`
- ADR: N/A unless implementation discovers a durable architecture decision that
  cannot be captured in module READMEs.
