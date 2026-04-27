# ADR-008: Durable Model License Diagnostics Ledger

## Status

Accepted.

## Context

Pantograph must record direct model execution as durable compliance and
diagnostics facts without requiring explicit diagnostics nodes or node-authored
ledger boilerplate.

Stages 01, 02, and 03 established durable runtime attribution, canonical node
contracts, and embedded-runtime-owned managed execution observability. Stage 04
connects those facts to persistent model/license usage records while preserving
the separation between runtime execution, workflow use cases, host bindings,
and GUI presentation.

The ledger must preserve time-of-use license facts, typed output measurements,
lineage, retention policy, and reduced-guarantee status so later diagnostics
queries can explain what was observed and what was unavailable.

## Decision

`pantograph-diagnostics-ledger` owns durable model/license diagnostics storage.
It defines model/license usage events, license snapshots, typed output
measurements, usage lineage, query DTOs, retention/pruning commands, and the
SQLite schema/migration boundary.

The ledger uses SQLite through the workspace-owned `rusqlite` dependency with
the existing `bundled` feature set. This keeps the diagnostics ledger aligned
with existing attribution persistence and avoids introducing another storage
family or host SQLite dependency.

Ledger schema version 1 persists usage events, license snapshots, typed output
measurements, usage lineage, retention policy, and schema migration records.
Unsupported schema versions fail with typed ledger errors instead of being
silently ignored or rewritten.

License metadata is captured as a time-of-use snapshot. Retention defaults to
the `standard` class for 365 days. Pruning is explicit and transactional: it
deletes complete eligible usage events and their dependent snapshot,
measurement, and lineage rows, but never rewrites retained historical facts.
The standard retention policy is a versioned ledger record. Policy version
starts at `1`, increments on every update, and is included in typed
`retention.policy_changed` audit events so future cleanup decisions can be
traced to a concrete policy revision.

`pantograph-embedded-runtime` submits validated usage facts through the managed
model capability and runtime-created node execution context boundary. It owns
the mapping from runtime guarantee classification into durable ledger guarantee
levels and downgrades durable records when measurement facts are unavailable.

`pantograph-workflow-service` delegates durable diagnostics usage queries to
the ledger and projects workflow-facing summaries, event rows, retention
metadata, and pruned-usage context. It does not own ledger persistence
semantics.

Workflow-service may emit typed Library audit events when it has authoritative
run snapshot context. First-pass model asset usage is recorded as
`library.asset_accessed` with `run_usage` operation and `pumas://models/<id>`
asset ids, linked to workflow run, workflow version, client/session/bucket,
scheduler policy, retention policy, and model id/version metadata. This does
not replace Pumas-owned search/download/delete audit paths, which remain
separate typed Library event producers.

GUI adapters and host bindings consume backend-owned projections only. They do
not reconstruct model/license semantics, retention behavior, guarantee levels,
or ledger storage policy.

Stage 04 does not implement GUI diagnostics views or host binding projections.
Those surfaces remain future consumers of workflow-service projections.

## Consequences

- Durable model/license usage records have one persistence owner.
- Runtime-created attribution, node contract, lineage, and guarantee facts flow
  into persisted usage events without moving policy into individual nodes.
- License changes in Pumas metadata do not mutate historical usage records.
- Missing output measurements remain explicit unavailable facts rather than
  misleading zero values.
- Workflow diagnostics queries can group and filter usage records without
  coupling workflow service to SQLite schema details.
- Future GUI and binding work must project existing backend-owned query DTOs
  instead of inventing local compliance semantics.
- Changes to retention duration, legal hold, export-before-prune, per-client
  retention, or schema policy are re-plan triggers.
