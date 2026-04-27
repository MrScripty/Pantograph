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
Stable scheduler estimate and queue-placement facts are promoted into typed
projection columns; consumers do not parse payload JSON for queue position,
priority, estimate confidence, estimated wait/duration, or scheduler reason.
Runtime and workflow services may write observations, summaries, and typed
events through repository methods, but they do not own the schema or query
semantics.

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
- `projection_state` records the projection version and last applied event
  sequence so incremental projection drains can resume after restart.
- Warm projection drains may return `rebuilding` with a persisted
  `last_applied_event_seq` when a bounded batch intentionally leaves later
  matching events for a subsequent drain.
- `scheduler_timeline_projection` is read directly by page/API consumers after
  an explicit incremental drain; normal reads do not replay raw event rows.
- Scheduler timeline events include typed delay and model lifecycle facts when
  those events are emitted. Delay state may also update the run-list status and
  scheduler reason through projection drains.
- `run.snapshot_accepted` events carry bounded immutable snapshot metadata,
  including `workflow_run_snapshot_id`, `workflow_presentation_revision_id`,
  and `node_versions` entries with node id, node type, contract version, and
  behavior digest. Consumers audit the node-version set from those event
  fields instead of consulting mutable graph state.
- `run_detail_projection` is read directly by selected-run page/API consumers
  after an explicit incremental drain; normal detail reads do not replay raw
  event rows.
- `run_list_projection` and `run_detail_projection` expose stable scheduler
  estimate and queue-placement facts as typed columns. Payload JSON remains
  audit detail, not the normal GUI data path for those facts.
- Run-list facet queries group materialized run-list rows by workflow version,
  status, scheduler policy, and retention policy. They must not derive
  mixed-version warnings from raw ledger events or client-side page samples.
- `io_artifact_projection` is read directly by I/O Inspector page/API
  consumers after an explicit incremental drain; normal artifact gallery reads
  do not replay raw event rows or load artifact bodies.
- `io_artifact_projection` keeps the latest current row per
  `workflow_run_id` and `artifact_id`; append-only ledger events remain the
  source of historical observation and retention cleanup decisions.
- `io_artifact_projection.retention_state` is a typed retention summary.
  Consumers must not infer expired, deleted, external, truncated, or too-large
  payload states from `payload_ref` alone.
- I/O retention completeness queries group the materialized artifact projection
  by typed retention state. They must not scan raw ledger events during normal
  page reads.
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
