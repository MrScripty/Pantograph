# crates/pantograph-diagnostics-ledger/src

Durable diagnostics ledger contracts and SQLite-backed persistence for
workflow runtime usage, timing observations, typed diagnostic events,
projection cursors, and workflow run summaries.

## Purpose

This directory owns diagnostics records that must survive process restarts.
The boundary exists so runtime producers, workflow trace code, and GUI
diagnostics projections can depend on one persistence contract instead of
writing ad hoc SQLite tables or keeping history in frontend memory.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Crate facade and public exports for diagnostics records, repositories, timing expectations, and SQLite storage. |
| `event.rs` | Typed diagnostic event envelope, payload families, source validation, retention/privacy classes, and projection cursor records. |
| `records.rs` | Model/license usage event records, query contracts, retention policy, lineage, and projection DTOs. |
| `timing.rs` | Workflow timing observation, timing expectation, and workflow run-summary contracts. |
| `repository.rs` | Host-facing diagnostics ledger repository trait. |
| `schema.rs` | SQLite schema version constants and migration ownership. |
| `sqlite.rs` and `sqlite/` | SQLite repository implementation split by model usage, timing, and run-summary persistence behavior. |
| `tests.rs` | Crate-level persistence, query, pruning, and migration regression tests. |

## Problem

Workflow diagnostics need durable history for timing comparisons and
restart-visible run summaries, while model/runtime usage diagnostics need an
auditable ledger. If these records are owned by transient trace stores or UI
state, the GUI loses previous workflow timing and runtime history after restart.

## Constraints

- Persisted records must use backend-owned identifiers, especially
  `workflow_id` and `workflow_run_id`.
- SQLite schema changes must be explicit and tested because user workspaces
  keep this database between application runs.
- Query contracts must remain deterministic so diagnostics views can compare
  current runs against prior observations.
- Retention/pruning must be caller-driven and auditable.
- Diagnostic events must use allowlisted typed payloads; raw arbitrary JSON is
  not accepted at the repository boundary.
- I/O artifact events must use typed artifact-role enums; callers must not
  submit arbitrary role labels for workflow or node artifacts.
- I/O artifact format metadata must use typed conversion status and typed
  dependency lease attribution records when real managed conversion occurs;
  callers must not encode conversion lifecycle or leases into free-form
  payload text.
- Library audit events must use typed operation and cache-status enums; callers
  must not submit arbitrary operation labels through payload strings.
- Retention policy change events must include typed actor scope so GUI admin
  and maintenance actions remain distinguishable without parsing source labels.
- Retention artifact state-change events must include typed actor scope so
  cleanup actions are auditable without parsing free-form reasons.
- Materialized projections are rebuildable, but normal read paths advance from
  stored `projection_state` cursors instead of replaying the full ledger.

## Decision

Keep durable diagnostics contracts in this crate and expose
`SqliteDiagnosticsLedger` as the concrete persistence owner. Workflow timing
history and run summaries use `workflow_run_id` for one submitted execution and
`workflow_id` for cross-run comparisons. Typed diagnostic events add a shared
append-only audit boundary for scheduler, run, node execution, I/O, Library,
runtime, and retention facts. The scheduler timeline, run-list, run-detail,
I/O artifact, and node-status projections are durable materialized read models
advanced from the event ledger by cursor.
Node-status projection rows include selected runtime id, canonical inference
task id, selected backend key, and model id when producers provide them, so UI
and API consumers do not need to parse raw diagnostic payload JSON for common
inference execution context.
Node-status projection rows also preserve the latest bounded effective runtime
settings summary when producers emit one. Those summaries are backend
execution diagnostics, not frontend configuration state.
Run-list and run-detail projections also roll up node- and
inference-diagnostic-derived selected runtime id, selected backend key,
selected model id, selected task id, selected device class, and selected device
id where scheduler/run payloads do not already provide those facts.
Scheduler-owned selected runtime, device, and network-node fields keep their
native lifecycle semantics and are not overwritten by inference payloads.
Run-list and run-detail projections retain descriptor-level output artifact
measures for later scheduler learning by counting output artifact descriptors
and summing their reported byte sizes. These projections do not inspect
artifact payload bodies.
Run terminal events may carry typed resource observations for peak RAM, peak
VRAM, and explicit out-of-memory failures. Run-list and run-detail projections
persist those fields as scheduler-history facts; consumers must not infer OOM
from free-form terminal error text.
Inference option-support summaries use
`inference.execution_diagnostic_observed` events. These rows are bounded
system metadata for request id, task id, lifecycle phase/kind, selected backend,
selected device class/id, resolved artifact kind, support-state counts,
backend/model compatibility summaries, per-option compatibility summaries,
usage-count summaries, cache-handle ids, artifact refs, and structured
KV-cache action/outcome references.
Inference artifact refs and cache-handle ids are stable metadata only; direct
ledger appends reject local-path-shaped values so producer-side payload filters
cannot be bypassed by alternate append paths.
Duration-only completed
lifecycle rows are allowed for bounded phase timing even when a phase carries no
usage or compatibility details; they must not carry prompt text, messages,
generated content, embeddings, tensors, token arrays, Python kwargs, raw backend
process output, cache bytes, cache fingerprints, or temp paths.
I/O artifact projection queries can filter producer and consumer node
endpoints directly; callers should not scan projection pages client-side to
answer node-produced or node-consumed artifact questions.
Stable scheduler estimate and queue-placement facts are promoted into typed
projection columns; consumers do not parse payload JSON for queue position,
priority, estimate confidence, estimated wait/duration, scheduler reason, or
future/scheduled run status presentation.
Runtime and workflow services may write observations, summaries, and typed
events through repository methods, but they do not own the schema or query
semantics.
Workflow error diagnostics use `diagnostic.error_occurred` as the canonical
durable error fact. The payload carries phase, scope, severity, code, user
message, technical detail, cause-chain summaries, recoverability, location, and
optional directly-known causality links. Existing lifecycle rows such as
`run.terminal`, scheduler model lifecycle, and node execution status remain
their native facts and should link to canonical error events rather than
duplicating the detailed error payload.
Failed `run.terminal` payloads may carry `canonical_error_event_id` when the
workflow service knows the directly related `diagnostic.error_occurred` row.
Consumers should use that link for navigation and causality display instead of
deriving cause from adjacent timestamps.
Failed `node.execution_status` payloads may also carry
`canonical_error_event_id`; node-status projections preserve that link
separately from `error_event_id`, which remains reserved for fatal
node-scoped `diagnostic.error_occurred` rows projected directly as node status.
Embedded inference lifecycle adapters should pass through directly-known
canonical error links into node-status payloads rather than copying detailed
inference failure text into a second canonical error event.
When a secondary diagnostic append fails while handling an existing workflow or
inference error, service layers should preserve the original error and expose a
`diagnostics_unavailable` link instead of replacing the user-visible failure
with a ledger/storage failure.

## Alternatives Rejected

- Store timing history only in the frontend.
  Rejected because workflow history must be visible after GUI restart.
- Keep workflow run summaries in the transient trace store.
  Rejected because trace retention is process-local and not sufficient for
  restart-visible diagnostics.
- Let each producer create its own SQLite tables.
  Rejected because schema ownership, migrations, and retention would drift.

## Invariants

- `workflow_run_id` identifies one workflow execution in timing observations
  and run summaries.
- `workflow_id` is the stable workflow grouping key for comparable timing
  history.
- `diagnostic_events.event_seq` is the durable monotonic cursor for projection
  application.
- `diagnostic.error_occurred` rows use typed scope validation. Run, node,
  runtime/model, scheduler, artifact, and projection errors require enough
  backend-owned IDs to make the event navigable; transport-scoped errors may be
  recorded without run IDs only when no workflow run context exists.
- Error payload text must be sanitized and bounded before append. Control
  characters are not accepted by validation; callers can use
  `sanitize_diagnostic_error_text` before constructing an error payload.
- `projection_state` records the projection version and last applied event
  sequence so incremental projection drains can resume after restart.
- Schema migrations may recreate incompatible rebuildable projection tables and
  clear their projection cursor so the next drain repopulates them from the
  ledger instead of preserving stale materialized rows.
- Warm projection drains may return `rebuilding` with a persisted
  `last_applied_event_seq` when a bounded batch intentionally leaves later
  matching events for a subsequent drain.
- `scheduler_timeline_projection` is read directly by page/API consumers after
  an explicit incremental drain; normal reads do not replay raw event rows.
- Scheduler timeline events include typed delay and model lifecycle facts when
  those events are emitted. Delay state may also update the run-list status and
  scheduler reason through projection drains.
- Scheduler timeline projection may include `run.*` lifecycle and
  `node.execution_status` rows for audit visibility, but those rows remain
  their native lifecycle facts and must not be re-emitted as scheduler events.
- Scheduler timeline projection may also include
  `inference.execution_diagnostic_observed` rows for bounded inference
  lifecycle visibility. Timeline summaries may use typed task, backend,
  device, artifact-kind, compatibility, option-support, usage-count,
  cache-handle, artifact-ref count, and KV-cache action/outcome facts, but must
  not expose prompt/result bodies, tensors, raw media, Python kwargs, CLI flags,
  or local paths.
- Scheduler estimate and model lifecycle events carry typed model/cache state
  where known. Timeline projection details may include that state, but callers
  should still treat estimate facts and observed lifecycle facts as separate
  rows.
- Scheduler estimate events may carry typed candidate runtime, device, and
  network-node id lists. Producers should leave unknown candidate classes empty
  instead of encoding them into free-form reason text only.
- Scheduler estimate events carry typed blocking conditions and missing asset
  ids separately from human-readable reasons so future scheduler policy can add
  richer blocking analysis without replacing the event family.
- Scheduler queue-control events use typed cancel, reprioritize, and
  push-front actions plus typed accepted/denied outcomes so refused queue
  mutations remain auditable without parsing service error text. Session-owned
  queue controls use `client_session` actor scope with requested/effective
  session ids; `gui_admin` controls carry the effective session id resolved by
  the scheduler store. Timeline projection rows format these typed action,
  outcome, actor-scope, authority context, position, and priority fields
  explicitly instead of relying on enum debug strings.
- Scheduler model lifecycle transitions include load failure and unload failure
  states so runtime admission and teardown errors can remain typed audit facts
  instead of free-form timeline text.
- Scheduler admission events may carry selected runtime, device/network-node,
  and reserved model ids as typed bounded fields. Timeline projection details
  may display those selections, but admission remains a scheduler decision
  event instead of a runtime execution lifecycle event.
- Scheduler reservation events record local runtime-slot reservation creation
  and release with a typed transition, resource kind, reservation id, selected
  runtime, and reserved model ids. They describe scheduler-held capacity, not
  runtime execution lifecycle.
- `run.snapshot_accepted` events carry bounded immutable snapshot metadata,
  including `workflow_run_snapshot_id`, `workflow_presentation_revision_id`,
  and `node_versions` entries with node id, node type, contract version, and
  behavior digest. Consumers audit the node-version set from those event
  fields instead of consulting mutable graph state.
- `run_detail_projection` is read directly by selected-run page/API consumers
  after an explicit incremental drain; normal detail reads do not replay raw
  event rows.
- `run_list_projection` and `run_detail_projection` expose stable scheduler
  estimate, queue-placement, scheduler model-cache posture, selected runtime,
  selected backend, selected device class, selected device id, and selected
  network-node facts as typed columns. Run-list rows also expose client,
  client-session, bucket, and workflow execution-session identifiers. Run-list
  queries can filter by those scope fields and accepted-at ranges. Payload JSON
  remains audit detail, not the normal GUI data path for those facts.
- `io_artifact_projection` exposes artifact producer and consumer node/port
  fields separately from the event node id so I/O browsing can distinguish
  workflow inputs, workflow outputs, and future node-to-node artifacts without
  parsing raw payload JSON.
- Run-list facet queries group materialized run-list rows by workflow version,
  status, scheduler policy, retention policy, selected runtime, selected
  backend, selected device class, selected device id, and selected network
  node. They must not derive mixed-version warnings from raw ledger events or
  client-side page samples.
- `io_artifact_projection` is read directly by I/O Inspector page/API
  consumers after an explicit incremental drain; normal artifact gallery reads
  do not replay raw event rows or load artifact bodies.
- `io_artifact_projection` keeps the latest current row per
  `workflow_run_id` and `artifact_fact_id`; append-only ledger events remain
  the source of historical observation and retention cleanup decisions.
- I/O artifact facts distinguish durable fact identity from retained body
  identity. `artifact_fact_id` identifies the projected observation row,
  `payload_artifact_id` identifies the ArtifactStore/read target, and
  `artifact_id` remains a compatibility field until workflow-service callers
  complete the read-target cutover.
- Multiple artifact facts may reference the same retained payload identity.
  Retention cleanup selects one representative row per workflow-run payload
  identity, emits one typed retention event, and applies the resulting
  retention state to every projected fact sharing that payload. Projection rows
  must not receive duplicate `event_seq` values for a single retention event.
- `io_artifact_projection` and durable I/O artifact events preserve
  conversion id, conversion status, conversion command id, and per-conversion
  dependency lease attribution in format metadata when producers provide those
  typed fields.
- I/O artifact projection may derive runtime, model, and selected-backend
  context from the latest producer-node execution status event at or before the
  artifact observation when the artifact event envelope does not already carry
  those facts. The projection must not infer runtime/model identity from
  artifact payloads, graph topology, or retention metadata.
- `io_artifact_projection.retention_state` is a typed retention summary.
  Consumers must not infer expired, deleted, external, truncated, or too-large
  payload states from `payload_ref` alone.
- `io_artifact_projection.artifact_role` stores canonical labels derived from
  `IoArtifactRole`, keeping workflow/node artifact roles typed at write time
  while preserving simple string filters for query contracts.
- Node-status projection runtime-settings summaries are copied from bounded
  `inference.execution_diagnostic_observed` payloads. The projection must not
  parse backend process logs or frontend setting controls to infer effective
  execution settings.
- I/O retention completeness queries group the materialized artifact projection
  by typed retention state. They must not scan raw ledger events during normal
  page reads.
- The artifact retention cleanup command uses the active global policy to
  drain the I/O artifact projection, select retained rows older than the policy
  cutoff, append typed `retention.artifact_state_changed` audit events, and
  leave artifact metadata queryable after payload references expire. Cleanup
  commands must pass typed actor scope into those audit events.
- The active global retention policy exposes first-pass typed setting groups
  for final outputs, workflow inputs, intermediate node I/O, failed-run data,
  maximum artifact size, total storage, media behavior, compression behavior,
  and cleanup trigger. These settings are currently derived from the standard
  retention window unless a future policy-version migration persists finer
  granularity.
- Library usage projection queries may filter by `workflow_run_id` through the
  materialized run-link table so active-run Library pages do not scan raw
  `library.asset_accessed` events.
- Schema migrations are forward-only and covered by repository tests.
- Query results must not require frontend-side identity repair or workflow-name
  side channels.
- Pruning commands return explicit counts so callers can audit data removal.

## Revisit Triggers

- Diagnostics storage moves from local SQLite to a shared service.
- Run-summary records need to include additional scheduler/runtime lifecycle
  phases beyond the current status and timing facts.
- Timing comparison policy needs configurable percentile or window selection.
- Projection rebuild APIs are added for migration, repair, or projection
  version changes.

## Dependencies

### Internal

- `pantograph-runtime-attribution` for canonical workflow and run id value
  semantics used by producers.

### External

- `rusqlite` for the local durable store.
- `serde` for persisted/query DTO projection.
- `thiserror` for repository error contracts.

## Related ADRs

- `docs/adr/ADR-012-canonical-workflow-run-identity.md` - Canonical workflow
  run identity across scheduler, runtime, traces, and diagnostics history.

## Usage Examples

```rust
use pantograph_diagnostics_ledger::{
    SqliteDiagnosticsLedger, WorkflowRunSummaryQuery,
};

let ledger = SqliteDiagnosticsLedger::open(path)?;
let history = ledger.query_workflow_run_summaries(&WorkflowRunSummaryQuery {
    workflow_id: Some("workflow-1".to_string()),
    workflow_run_id: Some("run-1".to_string()),
    limit: Some(10),
})?;
```

## API Consumer Contract

- Inputs: repository methods accept strongly named query/record structs; blank
  ids are rejected by callers before persistence where applicable.
- Outputs: query responses preserve backend field names and deterministic
  ordering for diagnostics projection.
- Lifecycle: callers open one ledger for a workspace database and reuse it
  through workflow/runtime services.
- Errors: repository failures return `DiagnosticsLedgerError` without hiding
  SQLite migration or query failures.
- Versioning: schema changes require migration code, tests, and README updates.

## Structured Producer Contract

- Stable fields: `workflow_id`, `workflow_run_id`, timing status, timing
  durations, usage-event identity, model identity, workflow-version fields,
  run snapshot node-version payloads, and lineage node contract
  version/digest facts are machine-consumed by diagnostics projections.
- Inference diagnostic summary fields are bounded metadata only: request id,
  task id, lifecycle phase/kind, duration when known, selected backend
  key/family, resolved artifact kind, compatibility summaries, option-support
  summaries, usage counts, cache-handle ids, and KV-cache action/outcome
  references.
  Producers may emit duration-only completed lifecycle summaries for phase
  timing when a matching start/terminal pair exists.
  Producers must keep raw request/result bodies, embeddings, tensors, token
  arrays, Python kwargs, backend CLI flags, and unbounded process output out of
  these payloads.
- Legacy fields: timing `graph_fingerprint` remains a compatibility facet for
  existing timing expectation history only. New diagnostics grouping must use
  workflow-version and node behavior-version correlation from immutable run
  snapshots and the typed event ledger.
- Defaults: omitted optional filters mean unfiltered queries within the
  caller-provided limit.
- Enums and labels: timing statuses, run-summary statuses, and usage statuses
  are persisted semantic labels.
- Ordering: timing and run-summary queries return most-recent compatible
  records first unless a narrower query defines otherwise. Scheduler timeline
  projection queries return event-sequence order for replayable page timelines.
- Compatibility: old incompatible identity records may be ignored when a plan
  intentionally changes the schema contract.
- Regeneration/migration: schema version bumps must include migration tests and
  update this README or the SQLite module README when persisted query contract
  ownership changes.

## Testing

```bash
cargo test -p pantograph-diagnostics-ledger
```

## Notes

- `src/sqlite/README.md` documents the split inside the SQLite implementation
  modules.
