# pantograph-diagnostics-ledger

Durable diagnostics ledger for Pantograph.

This crate owns persisted model/license usage events, time-of-use license
snapshots, typed direct-output measurements, usage lineage, workflow timing
observations, timing expectation contracts, bounded query DTOs, and
retention/pruning commands. It is intentionally separate from transient runtime
trace storage while still accepting finalized trace-derived observations.

## Ownership

- `pantograph-diagnostics-ledger` owns durable SQLite storage and query
  semantics for model/license usage records and finalized workflow timing
  observations.
- `pantograph-embedded-runtime` submits validated usage facts through the
  ledger trait. Ordinary node implementations do not author compliance ledger
  records directly.
- `pantograph-workflow-service` may expose application-level query use cases by
  delegating to this crate, including timing expectation projections derived
  from canonical workflow trace state.
- GUI and binding layers consume projections only.

## Persistence

The initial implementation uses bundled SQLite through `rusqlite`, matching the
attribution persistence boundary. Version `1` stores schema migrations, usage
events, license snapshots, typed output measurements, lineage rows, and
retention policy records. Version `2` adds workflow timing observations,
idempotent observation keys, lookup indexes, and timing expectation projection.

Retention is explicit. The default local policy keeps `standard` usage events
for 365 days. Pruning deletes complete eligible events and associated rows in
one transaction and never rewrites retained historical facts.

Run-list and run-detail projections store scheduler queue position, priority,
estimate confidence, estimated wait/duration, and scheduler reason as typed
columns. The original scheduler payload JSON remains audit detail and rebuild
input, but GUI/API consumers should read the typed projection fields for normal
list and detail views.
The node-status projection stores the latest typed execution status per
`workflow_run_id` and `node_id` so graph overlays can render node state without
replaying scheduler timelines or parsing raw diagnostic event payloads.

Timing expectations are ranges over comparable completed observations, not
generic progress percentages. Consumers must treat insufficient history as a
first-class state and must not infer progress from a missing expectation.
Optional match fields such as runtime identity refine timing comparisons when
available, but unknown historical values remain comparable instead of resetting
history when later diagnostics include richer runtime facts.
If a runtime-refined lookup has too little history, expectation projection falls
back to the stable workflow/graph/node history before reporting insufficient
history.
Incomplete elapsed durations may report `within_expected_range` or
`slower_than_expected`; only completed durations can be classified as
`faster_than_expected`.
