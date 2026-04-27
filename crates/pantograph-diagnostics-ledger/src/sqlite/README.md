# crates/pantograph-diagnostics-ledger/src/sqlite

## Purpose
SQLite helpers that back focused repository behavior for the diagnostics
ledger.

## Contents
| File | Description |
| ---- | ----------- |
| `event_sqlite.rs` | Typed diagnostic event append/query persistence, scheduler timeline projection, and projection cursor storage. |
| `run_summary_sqlite.rs` | Workflow run-summary upsert and query persistence for restart-visible run lists. |
| `timing_sqlite.rs` | Workflow timing observation persistence, expectation lookup, and timing retention pruning. |

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
- Diagnostic event cursor queries require bounded page sizes and non-negative
  cursors.
- `projection_state` is the durable resume point for incremental materialized
  projections; full rebuilds should update the same cursor/version contract.
- Scheduler timeline drains apply only events after the stored projection
  cursor and write idempotent rows keyed by `event_seq`.
- Scheduler timeline page/query reads use `scheduler_timeline_projection`,
  not `diagnostic_events`.
- Run detail drains apply only events after the stored projection cursor and
  update one row per workflow run for selected-run page/query reads.
- I/O artifact drains apply only artifact observation events after the stored
  projection cursor and write bounded metadata/reference rows keyed by
  `event_seq`.

## Dependencies
**Internal:** parent `sqlite.rs`, diagnostics ledger event/timing/run-summary
contracts, and schema-owned diagnostics tables.

**External:** `rusqlite` only.

## API Consumer Contract
This directory is not a public API. Public callers use
`DiagnosticsLedgerRepository` on `SqliteDiagnosticsLedger`.

## Structured Producer Contract
Callers submit validated typed diagnostic events, `WorkflowTimingObservation`
values, and workflow run summaries. These modules store durable facts and
project query responses without inventing frontend-owned diagnostics state.
