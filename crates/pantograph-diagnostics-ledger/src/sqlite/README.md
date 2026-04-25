# crates/pantograph-diagnostics-ledger/src/sqlite

## Purpose
SQLite helpers that back focused repository behavior for the diagnostics
ledger.

## Contents
| File | Description |
| ---- | ----------- |
| `timing_sqlite.rs` | Workflow timing observation persistence, expectation lookup, and timing retention pruning. |

## Invariants
- Timing observation writes remain idempotent through `observation_key`.
- Expectation lookups require workflow id, graph fingerprint, and node id for
  node-scoped history.
- Optional fields such as node type and runtime id may narrow comparisons, but
  unknown historical values still match so timing history does not reset when
  later diagnostics include richer optional facts.

## Dependencies
**Internal:** parent `sqlite.rs`, diagnostics ledger timing contracts, and the
schema-owned `workflow_timing_observations` table.

**External:** `rusqlite` only.

## API Consumer Contract
This directory is not a public API. Public callers use
`DiagnosticsLedgerRepository` on `SqliteDiagnosticsLedger`.

## Structured Producer Contract
Callers submit validated `WorkflowTimingObservation` values. This module stores
the durable facts and projects `WorkflowTimingExpectation` without inventing
frontend-owned timing state.
