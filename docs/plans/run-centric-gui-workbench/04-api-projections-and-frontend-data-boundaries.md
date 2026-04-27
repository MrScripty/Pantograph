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
- [ ] Define graph-version DTOs for historic run view.
- [ ] Define I/O artifact and retention DTOs.
  - First-pass I/O artifact DTOs cover bounded metadata and payload
    references. Retention-state DTOs remain pending with retention policy work.
- [ ] Define Library usage/Pumas audit DTOs.
- [ ] Define local Network node DTOs.
- [ ] Define future peer pairing/trust DTO placeholders for Network so Iroh
  can extend the model without replacing the page contract.
- [ ] Define explicit error taxonomy.
- [ ] Define local Network/system metrics behind a platform abstraction with
  degraded-state DTOs for unavailable or unauthorized metrics.
- [ ] Choose the DTO parity mechanism before page work begins: generated
  bindings/schema checks, or paired Rust serialization tests plus TypeScript
  normalization/fixture tests for every projection.
- [ ] If any new dependency is needed for DTO generation, media metadata,
  system metrics, or projection plumbing, record the owner, reason, alternatives
  considered, and lockfile impact before adding it.

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
- [ ] Add workflow-version graph lookup by run id.
- [ ] Add I/O metadata and retention policy queries/commands.
  - I/O artifact metadata query is implemented. Retention policy
    queries/commands remain pending.
- [ ] Add Library/Pumas usage audit queries.
- [x] Add projection rebuild/query boundaries for typed event ledger derived
  views.
- [x] Ensure backend projection queries read materialized projection tables or
  authoritative state, not raw event replay, during ordinary API requests.
- [ ] Add explicit admin/maintenance command boundaries for projection rebuild
  where Stage `03` exposes them.
- [ ] Add local Network/system-node status query.
- [ ] Add immutable run submission and cancel/resubmit command boundaries.
- [ ] Add scoped client queue action command boundaries.
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
runtime/model ids, media type, retention policy, node id, and artifact role
where those fields exist. Retention policy, Library/Pumas, Network/system-node,
admin maintenance, and broader command boundaries remain pending.

### Milestone 3: Frontend Services And Stores

**Goal:** Add frontend service adapters and UI stores that consume backend
projections while owning only transient UI state.

**Tasks:**

- [ ] Add or extend `src/services/` modules for run, scheduler, I/O, Library,
  and Network projections.
- [x] Add the initial run list projection service method and TypeScript DTOs.
- [x] Add the initial scheduler timeline projection service method and
  TypeScript DTOs.
- [x] Add the initial selected-run detail projection service method and
  TypeScript DTOs.
- [x] Add the initial I/O artifact projection service method and TypeScript
  DTOs.
- [ ] Add active-run store as transient UI state.
- [ ] Add focused stores for run list filters/sort/column state.
- [ ] Preserve backend error categories through presenters.
- [ ] Avoid optimistic updates for backend-owned queue and retention state.

**Verification:**

- TypeScript unit tests cover normalization and error preservation.
- Typecheck passes.
- Polling/subscription lifecycle tests exist if any recurring update loop is
  introduced.

**Status:** Not started.

### Milestone 4: Cross-Layer Acceptance

**Goal:** Prove at least one end-to-end projection path works before page
implementation depends on it.

**Tasks:**

- [ ] Add an acceptance path for run list projection from backend fixture/state
  to frontend service consumer.
- [ ] Add an acceptance path for selected run detail with workflow version and
  scheduler estimate.
- [ ] Add fixture data for expired-retention artifact behavior.
- [ ] Add fixture data for no-active-run retained artifact browsing where
  supported.
- [ ] Add an acceptance path proving a typed event reaches a backend projection
  and then a frontend service without exposing raw ledger storage details.
- [ ] Add an acceptance path proving projection freshness/catching-up state is
  preserved for a warm projection when it has not yet applied the latest event
  cursor.

**Verification:**

- Cross-layer acceptance checks pass according to `TESTING-STANDARDS.md`.
- If transport or language bindings changed, cross-layer acceptance includes
  the binding path used by the GUI rather than only in-process Rust fixtures.

**Status:** Not started.

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

- None. Draft plan only.

### Deviations

- None.

### Follow-Ups

- Decide transport owner in Milestone 1.
- Decide whether DTO generation is warranted before implementation.

### Verification Summary

- Not run. Draft plan only.

### Traceability Links

- Requirement section: API Requirements.
- Standards: Frontend Standards, Architecture Patterns, Testing Standards,
  Documentation Standards.
