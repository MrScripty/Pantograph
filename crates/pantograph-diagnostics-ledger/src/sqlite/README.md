# crates/pantograph-diagnostics-ledger/src/sqlite

## Purpose
SQLite helpers that back focused repository behavior for the diagnostics
ledger.

## Contents
| File | Description |
| ---- | ----------- |
| `event_sqlite.rs` | Typed diagnostic event append/query persistence, scheduler timeline, run, node, I/O artifact, Library usage projections, and projection cursor storage. |
| `run_summary_sqlite.rs` | Workflow run-summary upsert and query persistence for restart-visible run lists. |
| `timing_sqlite.rs` | Workflow timing observation persistence, expectation lookup, and timing retention pruning. |

## Problem
The SQLite repository implementation has several persistence responsibilities:
timing history, run summaries, typed diagnostic events, projection cursors, and
projection read models. Keeping all of that logic in one large file makes schema
ownership and projection behavior harder to audit.

## Constraints
- SQLite schema ownership stays in the parent diagnostics ledger crate.
- Helpers in this directory must not expose a second public repository API.
- Projection drains must be incremental from stored `projection_state` cursors.
- Page/query reads must use projection tables instead of replaying
  `diagnostic_events`.
- Event and artifact payload bodies must stay bounded metadata or references,
  not unbounded raw artifact storage.

## Decision
Split SQLite behavior by persistence responsibility inside this directory while
keeping `SqliteDiagnosticsLedger` as the public repository owner. Event helpers
own typed event append/query storage and projection cursor application. Timing
helpers own timing observations and timing expectation lookups. Run-summary
helpers own restart-visible workflow run summaries.

## Alternatives Rejected
- Keep all SQLite behavior in the parent `sqlite.rs` file.
  Rejected because typed event ledger and projection cursor logic make one file
  too broad to review safely.
- Expose each helper module as a public API.
  Rejected because public callers should depend on one repository trait and not
  on storage layout details.

## Invariants
- Timing observation writes remain idempotent through `observation_key`.
- Expectation lookups require workflow id, graph fingerprint, and node id for
  node-scoped history.
- Optional fields such as node type and runtime id may narrow comparisons, but
  unknown historical values still match so timing history does not reset when
  later diagnostics include richer optional facts.
- Runtime-refined lookups fall back to stable workflow/graph/node history when
  the refined bucket has too little history for an expectation.
- Diagnostic event writes assign one SQLite-owned monotonic `event_seq` and
  store bounded typed payload JSON plus payload hashes/references.
- Library asset events persist canonical operation/cache labels derived from
  enums, preserving flexible Library/Pumas audit coverage without accepting
  unvalidated action names.
- Diagnostic event cursor queries require bounded page sizes and non-negative
  cursors.
- `projection_state` is the durable resume point for incremental materialized
  projections; full rebuilds should update the same cursor/version contract.
- Warm projection drains that stop because a bounded batch leaves matching
  events unapplied report `rebuilding` status with the last applied cursor so
  API/GUI callers can show catching-up state without reading raw event rows.
- Scheduler timeline drains apply only events after the stored projection
  cursor and write idempotent rows keyed by `event_seq`.
- Scheduler timeline page/query reads use `scheduler_timeline_projection`,
  not `diagnostic_events`.
- Scheduler delay and model lifecycle events are typed ledger events and become
  timeline rows through the same incremental projection cursor. Delay events
  may also update run-list/run-detail scheduler status and reason fields.
- Run detail drains apply only events after the stored projection cursor and
  update one row per workflow run for selected-run page/query reads.
- Run-list facet reads group `run_list_projection` rows and must not replay
  `diagnostic_events` or depend on client-side page limits for comparison
  counts.
- I/O artifact drains apply artifact observation and retention state-change
  events after the stored projection cursor and write the latest bounded
  metadata/reference row per `workflow_run_id` and `artifact_id`.
- I/O artifact roles are persisted from typed event enums to canonical labels;
  SQLite projection code must not accept ad hoc role strings from payloads.
- I/O artifact drains persist typed retention-state columns so page/API
  consumers can distinguish retained, metadata-only, external, truncated,
  too-large, expired, and deleted payload states without parsing event payloads.
- I/O retention summary reads group `io_artifact_projection` rows and must not
  replay `diagnostic_events` for normal retention-completeness display.
- Artifact retention cleanup selects candidate rows from
  `io_artifact_projection`, emits typed retention audit events, and advances
  the same artifact projection path so cleanup remains observable without
  page-time ledger replay.
- Library usage drains apply asset-access events incrementally and report
  `rebuilding` while a limited batch has not caught up to all pending
  `library.asset_accessed` events.
- Library usage reads filter active-run assets through
  `library_usage_run_projection.workflow_run_id`; they must not replay raw
  event rows to answer selected-run Library queries.

## Revisit Triggers
- Diagnostics storage moves away from local SQLite.
- Projection drains require asynchronous workers or background scheduling.
- Artifact payload storage moves into a separate approved-root payload store
  with its own repository boundary.
- Schema migrations need per-module migration owners instead of the parent
  schema module.

## Dependencies
**Internal:** parent `sqlite.rs`, diagnostics ledger event/timing/run-summary
contracts, and schema-owned diagnostics tables.

**External:** `rusqlite` only.

## Related ADRs
- `docs/adr/ADR-008-durable-model-license-diagnostics-ledger.md`
- `docs/adr/ADR-012-canonical-workflow-run-identity.md`
- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples
Public callers do not use these modules directly. They open
`SqliteDiagnosticsLedger` and call `DiagnosticsLedgerRepository` methods.

## API Consumer Contract
This directory is not a public API. Public callers use
`DiagnosticsLedgerRepository` on `SqliteDiagnosticsLedger`.

## Structured Producer Contract
Callers submit validated typed diagnostic events, `WorkflowTimingObservation`
values, and workflow run summaries. These modules store durable facts and
project query responses without inventing frontend-owned diagnostics state.
