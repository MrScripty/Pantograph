# Typed Diagnostic Event Ledger Architecture

## Status

Accepted planning direction. Not implemented.

Last updated: 2026-04-27.

## Purpose

Define the diagnostics architecture used by the run-centric GUI workbench
plans. The goal is to make diagnostics extensible without opening the system to
unvalidated arbitrary data or requiring sweeping schema/API rewrites every time
Pantograph needs to track a new scheduler, runtime, node, I/O, retention, or
Library fact.

## Decision

Use a typed append-only diagnostic event ledger as the durable write model, and
derive page/query-specific read models from that ledger.

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
- projections are rebuildable from the ledger

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

## Retention And Privacy

Payload retention and audit retention are separate.

Retention may delete or externalize payload bodies. It must not delete the
audit metadata needed to explain that an event happened, which run it belonged
to, which workflow/node/runtime/model versions were involved, and why payload
data is unavailable.

Large media, binary data, model outputs, and sensitive data should use
`payload_ref` plus size/hash/type metadata. Embedded JSON is for bounded typed
payloads only.

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
  artifact metadata, and projection rebuild behavior.
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
- Projections cannot be rebuilt within acceptable time from the ledger.
- Privacy policy requires cryptographic erasure or tenant-level separation.
- A remote/multi-node deployment changes event producer trust boundaries.
- Raw developer event inspection becomes a product feature.
