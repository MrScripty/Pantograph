# pantograph-diagnostics-ledger

Durable model and license usage ledger for Pantograph.

This crate owns persisted model/license usage events, time-of-use license
snapshots, typed direct-output measurements, usage lineage, bounded query DTOs,
and retention/pruning commands. It is intentionally separate from transient
runtime trace storage.

## Ownership

- `pantograph-diagnostics-ledger` owns durable SQLite storage and query
  semantics for model/license usage records.
- `pantograph-embedded-runtime` submits validated usage facts through the
  ledger trait. Ordinary node implementations do not author compliance ledger
  records directly.
- `pantograph-workflow-service` may expose application-level query use cases by
  delegating to this crate.
- GUI and binding layers consume projections only.

## Persistence

The initial implementation uses bundled SQLite through `rusqlite`, matching
the attribution persistence boundary. Version `1` stores schema migrations,
usage events, license snapshots, typed output measurements, lineage rows, and
retention policy records.

Retention is explicit. The default local policy keeps `standard` usage events
for 365 days. Pruning deletes complete eligible events and associated rows in
one transaction and never rewrites retained historical facts.
