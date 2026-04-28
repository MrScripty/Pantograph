# 04: API Projections And Frontend Data Boundaries

## Status

Draft plan. Not implemented.

## Objective

Expose backend-owned run, scheduler, diagnostics, retention, Library/Pumas, and
local Network facts through stable API projections so the Svelte GUI can render
the run-centric workbench without inventing backend truth in frontend stores.

## Scope

### In Scope

- Run list and run detail projections.
- Scheduler estimate and scheduler event query projections.
- Workflow version and presentation revision graph projections.
- I/O artifact metadata and retention-state projections.
- Node runtime-status projection for graph overlays and run diagnostics.
- Global retention policy read/update projection for privileged GUI surfaces.
- Library asset usage and Pumas audit projections.
- Projection contracts derived from the typed diagnostic event ledger.
- Local Network/system node state projection.
- Frontend TypeScript types and service adapters.
- Error categories for invalid workflow identity, version/fingerprint conflict,
  unauthorized queue action, retention errors, and missing/expired payloads.
- Immutable run submission, scoped client queue actions, and privileged GUI
  admin queue actions exposed through backend-owned command boundaries.
- Future Network peer pairing/trust projection placeholders without
  implementing Iroh discovery.

### Out of Scope

- Full page visual implementation.
- Network peer protocol design.
- Node Lab authoring API.
- Replacing all existing workflow graph mutation APIs unless needed to avoid
  ambiguity.

## Inputs

### Problem

The frontend needs stable, backend-owned DTOs for the new pages. Without a
projection stage, the app shell and page components would need to infer run
state, scheduler reasons, retention status, and library usage from unrelated
transport calls.

### Constraints

- Frontend stores may normalize DTOs but must not become policy owners.
- Backend errors must remain explicit so clients know why submissions/actions
  were rejected.
- Host-facing APIs require README updates documenting lifecycle, errors, and
  breaking contract cutovers.
- Event-driven synchronization is preferred over polling; any polling must be
  scoped, low-frequency, and cleaned up deterministically.
- API consumers use page/read-model projections by default. Raw diagnostic
  event access is not a normal page API and must remain a separate privileged
  developer/admin concern if added later.
- Ledger-derived page APIs read durable materialized projections and may expose
  projection freshness/status. They must not trigger full ledger replay during
  normal startup, page load, or query handling.

### Assumptions

- The GUI may consume Tauri commands, HTTP adapter endpoints, or both,
  depending on the existing app path chosen during implementation.
- Initial projections can be read-model oriented, but they do not need to
  preserve old workflow/run DTO compatibility.
- Active-run selection remains frontend-only and is not persisted.

### Dependencies

- Stages `01`, `02`, and `03`.
- `diagnostic-event-ledger-architecture.md`.
- `pantograph-frontend-http-adapter` and/or Tauri command modules.
- Frontend `src/services/`, `src/stores/`, and generated/manual type contracts.
- Existing diagnostics and workflow services.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| DTOs mirror storage internals too closely. | Medium | Define page/use-case projections at backend facade boundaries. |
| Raw event rows leak into normal page APIs. | High | Expose ledger-derived projections for pages; reserve raw event inspection for explicit privileged tooling. |
| API projections rebuild history on every page load. | High | Read materialized projection tables with cursors and expose stale/catching-up status for warm projections. |
| Frontend starts polling many endpoints. | Medium | Prefer event/subscription design; document any temporary polling owner and cleanup. |
| API errors are collapsed into generic failures. | High | Preserve explicit backend error categories through service adapters. |
| New projections inherit ambiguous old workflow graph API semantics. | High | Replace or delete old transport methods that conflict with run/version projections during the cutover stage. |
| Rust and TypeScript projection DTOs drift as the surface grows. | High | Add paired projection tests or a generated/schema-checked DTO workflow before pages consume new projections. |

## Definition of Done

- GUI can query run list, run detail, estimate, scheduler events, graph version,
  I/O metadata, Library usage, and local Network state through stable services.
- GUI page DTOs are projections derived from typed events or authoritative
  backend state, not raw ledger rows.
- Ledger-derived DTOs are served from materialized projections with recorded
  freshness/cursor state; normal DTO reads do not perform full event replay.
- Frontend service adapters preserve backend error categories.
- TypeScript DTOs and Rust/adapter DTOs are aligned and tested.
- DTO drift checks cover each new projection field, including defaults and
  optional/degraded-state behavior.
- Host-facing API README sections are updated.
- At least one cross-layer acceptance path proves backend projection reaches
  frontend service consumers with preserved semantics.

## Milestones

### Milestone 1: Projection Contract Inventory

**Goal:** Define projection families and decide transport ownership.

**Tasks:**

- [ ] Inventory existing workflow, diagnostics, runtime, and frontend adapter
  APIs.
- [ ] Define run list/detail DTOs.
- [ ] Define scheduler estimate/event DTOs.
- [ ] Define which DTOs are direct authoritative-state projections and which
  are rebuilt from typed diagnostic ledger events.
- [ ] Define which ledger-derived DTOs are hot, warm, or cold projections and
  how projection freshness/catching-up/degraded states appear in API responses.
- [x] Define graph-version DTOs for historic run view.
- [ ] Define I/O artifact and retention DTOs.
  - First-pass I/O artifact DTOs cover bounded metadata and payload
    references. Retention-state DTOs remain pending with retention policy work.
- [ ] Define Library usage/Pumas audit DTOs.
- [x] Define local Network node DTOs.
- [x] Define future peer pairing/trust DTO placeholders for Network so Iroh
  can extend the model without replacing the page contract.
- [ ] Define explicit error taxonomy.
- [x] Define local Network/system metrics behind a platform abstraction with
  degraded-state DTOs for unavailable or unauthorized metrics.
- [ ] Choose the DTO parity mechanism before page work begins: generated
  bindings/schema checks, or paired Rust serialization tests plus TypeScript
  normalization/fixture tests for every projection.
- [x] If any new dependency is needed for DTO generation, media metadata,
  system metrics, or projection plumbing, record the owner, reason, alternatives
  considered, and lockfile impact before adding it.
  - Added `sysinfo = "0.32"` to `pantograph-workflow-service` for local
    CPU/memory/disk/network-interface metrics. Owner: workflow-service local
    Network status provider. Alternatives considered: std-only host facts
    without resource metrics, or routing through the inference crate. Direct
    `sysinfo` use keeps the API provider focused and avoids coupling Network
    state to inference. Lockfile impact: no new transitive package version;
    `sysinfo` was already present through `crates/inference`.

**Verification:**

- Contract tests cover serialization and default semantics.
- DTO parity tests or generated binding checks cover Rust/TypeScript field
  names, optional states, defaults, and degraded-state behavior.
- Local Network/system metrics tests cover platform-specific provider
  abstraction and graceful degraded states.
- Documentation records transport ownership and breaking-contract decisions.

**Status:** Not started.

### Milestone 2: Backend Projection Implementation

**Goal:** Implement backend read models and command boundaries without moving
policy into adapters.

**Tasks:**

- [x] Add backend queries for run list.
- [x] Add backend queries for run detail.
- [x] Add scheduler estimate and event queries.
- [x] Add workflow-version graph lookup by run id.
- [x] Add I/O metadata and retention policy queries/commands.
  - I/O artifact metadata query is implemented. Retention policy
    query and global update command are implemented.
- [x] Add node runtime-status query.
- [x] Add Library/Pumas usage audit queries.
- [x] Add projection rebuild/query boundaries for typed event ledger derived
  views.
- [x] Ensure backend projection queries read materialized projection tables or
  authoritative state, not raw event replay, during ordinary API requests.
- [x] Add explicit admin/maintenance command boundaries for projection rebuild
  where Stage `03` exposes them.
- [x] Add local Network/system-node status query.
- [ ] Add immutable run submission and cancel/resubmit command boundaries.
- [x] Add scoped client queue action command boundaries.
- [ ] Add privileged/admin command boundaries for GUI-only actions.
- [ ] Remove or rename old projection APIs that would expose stale
  graph-fingerprint or current-graph semantics for historic runs.

**Verification:**

- Rust unit/integration tests cover projection shape and error mapping.
- Tests prove normal run list/detail and scheduler timeline projection reads do
  not trigger full diagnostic-event replay.
- Tests prove adapters forward policy decisions instead of recomputing them.
- If Rustler, UniFFI, Tauri commands, or HTTP adapter binding contracts are
  touched, native and host-language binding checks cover the changed projection
  and command DTOs.

**Status:** In progress. Workflow service now exposes
`workflow_scheduler_timeline_query`, which advances the scheduler timeline
projection incrementally through the diagnostics ledger cursor and then reads
materialized `scheduler_timeline_projection` rows. It also exposes
`workflow_run_list_query` over durable `run_list_projection` rows for dense
scheduler-page run lists. The Tauri app now configures the shared
`WorkflowService` with the persistent diagnostics ledger and exposes these
projection queries with frontend service/type boundaries. `workflow_run_detail_query`
now exposes selected-run detail over durable `run_detail_projection` rows with
projection freshness state. `workflow_io_artifact_query` now exposes bounded
artifact metadata/reference rows for I/O Inspector reads. Projection query
DTOs now expose backend-owned filters for workflow version, scheduler policy,
runtime/model ids, media type, retention policy, node id, artifact role,
client, client session, bucket, and accepted-at time ranges where those fields
exist. The run-list projection now supports server-side retention-policy,
scope, and accepted-time filtering in addition to returning those fields on
each run row. `workflow_library_usage_query` now exposes warm
Library/Pumas usage aggregates with projection freshness state. Retention
policy query is exposed for GUI retention settings/inspectors. Local Network
status query is exposed with local-only CPU/memory/disk/network-interface
facts, scheduler load, future peer DTO placeholders, and explicit degraded GPU
state. Frontend queue cancel/reprioritize/push-front methods now call the
backend-owned execution-session queue commands, and stale frontend session
command names were corrected. Retention policy updates now use a backend
command that changes the global standard policy and records a typed
`retention.policy_changed` audit event. Run-list and run-detail projection DTOs
now expose typed scheduler queue position, priority, estimate confidence,
estimated queue wait, estimated
duration, and scheduler reason fields rather than requiring consumers to parse
estimate or queue-placement payload JSON for those facts. A narrow
`workflow_scheduler_estimate_query` command now exposes the same hot projection
estimate facts for callers that need estimate-only reads. Broader command
boundaries remain pending.
`workflow_node_status_query` now exposes the hot `node_status` projection over
typed `node.execution_status` ledger events for graph runtime-status overlays.
`WorkflowTraceStore` now produces bounded node-status events for traced node
lifecycle transitions, while progress and stream observations remain outside
the typed event ledger.
`workflow_io_artifact_query` now carries typed artifact `retention_state` and
`retention_reason` fields so I/O pages do not infer retention from
`payload_ref`. The I/O projection now treats retention changes as audited
ledger events and materializes the latest current state per run artifact,
including payload-reference removal after expiration or deletion. The same API
response now includes retention-state summary counts from the materialized
artifact projection so pages can show retention completeness without issuing
raw ledger scans.
Workflow-service diagnostics tests now include an expired-retention artifact
fixture that proves `workflow_io_artifact_query` returns the expired
`retention_state`, clears the payload reference, and reports matching
retention-summary counts through the public API.
Frontend workflow projection tests now use Tauri mock IPC to prove
`WorkflowProjectionService` forwards `workflow_scheduler_timeline_query`,
`workflow_run_list_query`, and `workflow_run_detail_query` requests under the
native `{ request }` envelope. The tests preserve backend-authored typed
scheduler timeline events, bounded payload JSON, run-list facets, delayed
status, workflow version, scheduler estimate fields, queue-placement fields,
and projection freshness state for GUI consumers.
`workflow_library_usage_query` now reports `rebuilding` projection status when
a bounded warm-projection batch applies only part of the pending Library usage
event cursor. Frontend projection tests preserve that warm catching-up state
through Tauri mock IPC so Library pages can display freshness without reading
raw ledger rows.
`workbenchStore.ts` owns transient active-run selection and
`schedulerRunListStore.ts` now owns Scheduler table filters, sort order, and
column visibility, so the Scheduler page consumes backend projections without
turning UI preferences into backend queue policy.
`workflow_run_list_query` now returns backend-owned comparison facets for
workflow version, status, scheduler policy, and retention policy from the
run-list projection. Diagnostics pages use those scoped facets for
mixed-version warnings instead of rebuilding counts from global page state.
`workflow_projection_rebuild`
provides the first explicit admin maintenance boundary for hot projection
repair and projection-version rebuild scenarios. `workflow_run_graph_query`
now reconstructs historic run graphs from immutable run snapshot, executable
topology, graph settings, and presentation revision records instead of reading
current graph files.

### Milestone 3: Frontend Services And Stores

**Goal:** Add frontend service adapters and UI stores that consume backend
projections while owning only transient UI state.

**Tasks:**

- [x] Add or extend `src/services/` modules for run, scheduler, I/O, Library,
  and Network projections.
- [x] Add the initial run list projection service method and TypeScript DTOs.
- [x] Add the initial scheduler timeline projection service method and
  TypeScript DTOs.
- [x] Add the initial selected-run detail projection service method and
  TypeScript DTOs.
- [x] Add the initial historic run graph query service method and TypeScript
  DTOs.
- [x] Add the initial I/O artifact projection service method and TypeScript
  DTOs.
- [x] Add frontend retention policy update method and TypeScript DTOs.
- [x] Add the initial local Network status service method and TypeScript DTOs.
- [x] Add frontend methods and DTOs for scoped queue cancel/reprioritize/
  push-front actions.
- [x] Add active-run store as transient UI state.
- [x] Add focused stores for run list filters/sort/column state.
- [x] Preserve backend error categories through presenters.
- [x] Avoid optimistic updates for backend-owned queue and retention state.

**Verification:**

- TypeScript unit tests cover normalization and error preservation.
- Typecheck passes.
- Polling/subscription lifecycle tests exist if any recurring update loop is
  introduced.

**Status:** Complete. Projection invoke wiring is now split into
`WorkflowProjectionService`, with `WorkflowService` inheriting that boundary
for existing GUI callers. The adapter covers scheduler timeline, run-list,
selected-run, I/O artifact, and warm Library usage reads. Workbench-facing
workflow command paths now normalize backend JSON error envelopes into typed
`WorkflowServiceError` values, and workbench pages format failures through a
shared presenter so categories such as `invalid_request`, `scheduler_busy`, and
`queue_item_not_found` are not collapsed into generic strings. Broader
queue and retention command tests now prove service methods return
backend-authored responses exactly rather than synthesizing local replacement
state.
Active-run selection is already transient in `workbenchStore.ts`; Scheduler
run-list filters, sort order, and column visibility now live in
`schedulerRunListStore.ts`.

### Milestone 4: Cross-Layer Acceptance

**Goal:** Prove at least one end-to-end projection path works before page
implementation depends on it.

**Tasks:**

- [x] Add an acceptance path for run list projection from backend fixture/state
  to frontend service consumer.
- [x] Add an acceptance path for selected run detail with workflow version and
  scheduler estimate.
- [x] Add fixture data for expired-retention artifact behavior.
- [x] Add fixture data for no-active-run retained artifact browsing where
  supported.
- [x] Add an acceptance path proving a typed event reaches a backend projection
  and then a frontend service without exposing raw ledger storage details.
- [x] Add an acceptance path proving projection freshness/catching-up state is
  preserved for a warm projection when it has not yet applied the latest event
  cursor.

**Verification:**

- Cross-layer acceptance checks pass according to `TESTING-STANDARDS.md`.
- If transport or language bindings changed, cross-layer acceptance includes
  the binding path used by the GUI rather than only in-process Rust fixtures.

**Status:** In progress. Run-list, selected-run detail, and typed scheduler
timeline event acceptance now cover the frontend service boundary with Tauri
mock IPC. Run-list projection DTOs now include client, client-session, bucket,
workflow execution-session scope fields, and server-side filters for client,
client-session, bucket, and accepted-at ranges so Scheduler and Diagnostics
pages do not recover authority or time-scope context from raw events. Backend
fixture coverage for typed event projection, retained artifact browsing,
expired I/O artifact state, I/O artifact endpoint filters, and warm Library
usage catching-up state is also in place. Frontend error-envelope coverage now
proves projection service calls preserve backend error categories through typed
service errors and shared workbench presenters. Queue and retention command
coverage proves frontend service consumers receive backend-owned command DTOs
without optimistic replacement.

## Ownership And Lifecycle Note

Any frontend polling introduced in this stage must be owned by one store or
component, stopped on unmount/shutdown, and covered by cleanup tests. Prefer a
single scheduler/run projection subscription or event drain if backend support
exists.

If event-driven synchronization is added, the frontend subscribes to projection
updates or event-derived invalidation hints. It must not become a raw diagnostic
event consumer for normal page state.

## Re-Plan Triggers

- Transport ownership must move between Tauri commands and HTTP adapter.
- DTO generation becomes necessary to prevent frontend/backend drift.
- Backend projections expose too many storage details.
- Subscription/event delivery is required before Scheduler table can be usable.

## Completion Summary

### Completed

- Backend projection boundaries implemented so far: run list, run detail,
  scheduler timeline, I/O artifact metadata, Library/Pumas usage, retention
  policy query/update, projection rebuild, historic run graph lookup by run id,
  and local Network status query.
- Frontend service/type boundaries implemented so far for those projection
  reads, the historic run graph lookup, and local Network status query.

### Deviations

- None.

### Follow-Ups

- Decide transport owner in Milestone 1.
- Decide whether DTO generation is warranted before implementation.

### Verification Summary

- Focused Rust workflow-service tests and frontend typecheck are run with each
  committed implementation slice.

### Traceability Links

- Requirement section: API Requirements.
- Standards: Frontend Standards, Architecture Patterns, Testing Standards,
  Documentation Standards.
