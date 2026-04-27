# Typed Diagnostic Event Ledger Architecture

## Status

Accepted planning direction. Bootstrap implementation started in
`pantograph-diagnostics-ledger`.

Last updated: 2026-04-27.

## Implementation Progress

- `pantograph-diagnostics-ledger` owns the typed event contract, SQLite event
  storage, and projection cursor metadata.
- Initial implemented event families are `scheduler.estimate_produced`,
  `scheduler.queue_placement`, `run.snapshot_accepted`,
  `io.artifact_observed`, `library.asset_accessed`,
  `runtime.capability_observed`, and `retention.policy_changed`.
- SQLite schema version `7` adds `diagnostic_events` and `projection_state`.
- Event append assigns durable monotonic `event_seq`, hashes bounded typed
  payload JSON, stores payload references separately, and rejects disallowed
  source components or missing required correlation fields.
- `projection_state` persists projection version, status, rebuild timestamp,
  and `last_applied_event_seq` for incremental read-model drains.
- The workflow-service queued run snapshot path emits
  `run.snapshot_accepted` when a diagnostics ledger is configured.
- The workflow-session scheduler emits `scheduler.estimate_produced` and
  `scheduler.queue_placement` after queue insertion when a diagnostics ledger
  is configured.
- SQLite schema version `8` adds `scheduler_timeline_projection`, and the
  diagnostics ledger can drain `run.snapshot_accepted`,
  `scheduler.estimate_produced`, and `scheduler.queue_placement` events into
  that materialized hot projection by `projection_state` cursor.
- Warm projection tables, rebuild commands, page-facing API wiring, and
  remaining feature emitters remain plan work.

## Purpose

Define the diagnostics architecture used by the run-centric GUI workbench
plans. The goal is to make diagnostics extensible without opening the system to
unvalidated arbitrary data or requiring sweeping schema/API rewrites every time
Pantograph needs to track a new scheduler, runtime, node, I/O, retention, or
Library fact.

## Decision

Use a typed append-only diagnostic event ledger as the durable write model, and
derive page/query-specific read models from that ledger.

Projection rebuild cost is a primary design constraint. "Rebuildable" means a
projection can be reconstructed from the ledger during migration, corruption
repair, tests, or projection-version changes. It does not mean normal startup,
page load, or every query should replay all events. Normal operation uses
durable materialized projections with stored cursors and applies only new
events.

The database/storage shape may be flexible enough to accept future event
families, but the write contract is strict:

- events are emitted only by backend-owned subsystems
- event kinds are allowlisted
- payloads are typed Rust structures
- every payload has a schema version
- every event is validated before persistence
- every event records source ownership and correlation identifiers
- privacy and retention class are required
- large or sensitive payloads are stored by reference, not embedded blindly
- projections are materialized during normal operation and explicitly
  rebuildable from the ledger when needed

## Core Pattern

```text
Flexible storage shape, strict write contract.
```

Feature code must not write raw JSON directly to the ledger. Feature code uses
typed constructors or builder APIs owned by the event ledger boundary. The
ledger serializes validated payloads into durable storage.

## Event Envelope

The stable event envelope should include:

```text
event_seq
event_id
event_kind
schema_version
source_component
source_instance_id
occurred_at_ms
recorded_at_ms
workflow_run_id?
workflow_id?
workflow_version_id?
workflow_semver?
node_id?
node_type?
node_version?
runtime_id?
runtime_version?
model_id?
model_version?
client_id?
client_session_id?
bucket_id?
scheduler_policy_id?
retention_policy_id?
privacy_class
retention_class
payload_hash
payload_size_bytes
payload_ref?
payload_json
```

Optional fields are optional because some events are system-scoped rather than
run-scoped. Validation rules decide which fields are required for each event
kind.

## Event Families

Initial event families should include:

- `scheduler.*`: submission, estimate, queue placement, delay, promotion,
  cancellation, admission, reservation, runtime/device selection, model
  load/unload, retry, fallback, client action, admin override.
- `run.*`: run snapshot accepted, execution started, execution completed,
  execution failed, run status changed.
- `node.*`: node execution started, node execution completed, node failed,
  node input observed, node output observed.
- `io.*`: workflow input recorded, workflow output recorded, node artifact
  recorded, payload retained, payload truncated, payload expired, payload
  deleted, payload externalized.
- `library.*`: Pumas search, download, delete, asset access, asset used by
  run, cache hit/miss, network bytes observed where available.
- `runtime.*`: runtime capability observed, runtime health changed, local
  resource load observed.
- `retention.*`: policy changed, cleanup started, cleanup item processed,
  cleanup completed, cleanup failed.

Event family names are planning names. Implementation should freeze exact
strings in one contract module and test them if they cross API boundaries.

Event family ownership must prevent duplicate truth:

- `run.*` owns execution lifecycle status and terminal outcomes.
- `scheduler.*` owns scheduling decisions, estimates, queue placement,
  admission, reservations, resource/model load decisions, and authority
  actions.
- Scheduler projections may join `run.*` lifecycle events, but scheduler
  producers must not emit separate terminal lifecycle facts that compete with
  `run.*`.

## Validation Rules

Every event kind must define:

- required envelope fields
- typed payload structure
- payload schema version
- allowed source components
- privacy class
- retention class
- maximum embedded payload size
- whether `payload_ref` is allowed or required
- whether the event is run-scoped, workflow-scoped, node-scoped, system-scoped,
  or Library-scoped

Invalid events must be rejected before persistence. Rejected events should not
be partially written.

## Projection Model

GUI pages and API consumers should not query raw event rows by default. They
should consume rebuildable projections such as:

- run list projection
- run detail projection
- scheduler timeline projection
- scheduler estimate projection
- diagnostics summary projection
- node timeline projection
- I/O artifact gallery projection
- retention policy/status projection
- Library usage projection
- workflow-version performance projection
- local Network/system state projection

Raw event access can exist later for developer/admin inspection, but it must be
separate from user-facing page contracts and protected by the same validation,
privacy, and retention model.

Projection storage should follow this pattern:

```text
diagnostic_events
  event_seq
  event_id
  event_kind
  occurred_at_ms
  workflow_run_id?
  workflow_version_id?
  node_id?
  model_id?
  runtime_id?
  payload_json?
  payload_ref?

projection_state
  projection_name
  projection_version
  last_applied_event_seq
  status
  rebuilt_at_ms?

run_list_projection
run_detail_projection
scheduler_timeline_projection
io_artifact_projection
library_usage_projection
retention_status_projection
diagnostics_summary_projection
```

The append path writes a typed event once, then updates affected hot
projections synchronously or near-synchronously inside the same durable
boundary where practical. Warm projections may drain new events asynchronously
or lazily from:

```sql
SELECT *
FROM diagnostic_events
WHERE event_seq > ?
ORDER BY event_seq;
```

Each projection owns a `projection_version` and `last_applied_event_seq`.
Projection code must be idempotent for duplicate application attempts and
recover by replaying only events after the last committed cursor. Full replay
is reserved for explicit rebuild commands, migration, repair, projection
version changes, and test fixtures.

Projection classes:

- Hot projections: run list, run detail, current run status, scheduler
  timeline, and active-run I/O artifact metadata. These feed the default GUI
  pages and should be cheap, indexed, and current enough for page rendering.
- Warm projections: workflow-version performance summaries, model/runtime
  comparison facets, Library usage counts, and retention completeness
  summaries. These can update asynchronously or lazily and expose freshness
  status when not caught up.
- Cold rebuilds: full diagnostics summary rebuilds, all-runs artifact gallery
  rebuilds, and historical aggregate recomputation. These are admin,
  migration, repair, or test paths, not page-load behavior.

Terminal runs should write or project compact summary rows so normal run-list
and run-detail views for old completed runs do not need to replay full
timelines.

## Retention And Privacy

Payload retention and audit retention are separate.

Retention may delete or externalize payload bodies. It must not delete the
audit metadata needed to explain that an event happened, which run it belonged
to, which workflow/node/runtime/model versions were involved, and why payload
data is unavailable.

Large media, binary data, model outputs, and sensitive data should use
`payload_ref` plus size/hash/type metadata. Embedded JSON is for bounded typed
payloads only.

The ledger must not store every stream chunk, token, image byte, audio sample,
or raw artifact body as diagnostic events. Event payloads are bounded metadata
and references. High-volume data belongs in payload stores or streaming
channels with ledger events recording durable audit facts and retained
references.

## Security Boundary

The frontend cannot write diagnostic events directly. External clients cannot
submit arbitrary diagnostic events as part of workflow execution.

Allowed event producers are backend subsystems such as scheduler, workflow
service, runtime execution, node execution, retention cleanup, Pumas/Library
service wrappers, and local system/runtime observers.

## Implementation Implications

- Stage `01` must make run snapshots and workflow version ids available as
  stable event correlation fields.
- Stage `02` scheduler events should be typed ledger events, not a separate
  unstructured log stream.
- Stage `03` owns ledger schema, typed event contracts, retention classes,
  artifact metadata, projection state/cursors, hot/warm/cold projection
  classes, incremental projection application, and explicit rebuild behavior.
- Stage `04` exposes projections derived from the ledger, not storage tables.
- Stages `05` and `06` render projections and may show timelines/galleries,
  but they do not own diagnostic truth.
- Stage `07` must add source-audit and test gates proving new event kinds are
  typed, validated, and covered by projections or intentionally internal.

## Alternatives Rejected

- Free-form diagnostic JSON written by feature code.
  Rejected because it weakens validation, privacy, indexing, and long-term
  maintainability.
- Entity-attribute-value diagnostics tables.
  Rejected because they are flexible but difficult to type, validate, secure,
  and query predictably.
- One bespoke table per new diagnostic concern.
  Rejected because it forces broad schema/API/frontend changes for each new
  diagnostic family.

## Revisit Triggers

- Event volume requires compaction, partitioning, or snapshotting.
- Projections cannot be incrementally maintained or rebuilt within acceptable
  time from the ledger.
- Startup, page load, or ordinary query paths begin replaying full event
  history instead of reading durable materialized projections.
- Privacy policy requires cryptographic erasure or tenant-level separation.
- A remote/multi-node deployment changes event producer trust boundaries.
- Raw developer event inspection becomes a product feature.
