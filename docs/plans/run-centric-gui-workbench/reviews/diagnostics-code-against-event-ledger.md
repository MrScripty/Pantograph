# Current Diagnostics Code Against Typed Event Ledger

## Status

Planning analysis. No source implementation yet.

Last updated: 2026-04-27.

## Purpose

Compare the current diagnostics implementation against the planned typed
append-only diagnostic event ledger in
`../diagnostic-event-ledger-architecture.md`.

The goal is to identify what can be reused, what must move, and where current
diagnostics behavior would conflict with the new architecture.

## Executive Summary

Pantograph already has several useful diagnostics building blocks, but they are
not one architecture:

- `pantograph-diagnostics-ledger` owns durable SQLite tables for model/license
  usage, timing observations, run summaries, and retention policy.
- `pantograph-workflow-service::trace` owns typed workflow trace events,
  in-memory reducers, timing expectations, and queue/runtime trace summaries.
- `src-tauri/src/workflow/diagnostics` owns a GUI projection and an in-memory
  event overlay with untyped JSON payloads derived from Tauri workflow events.
- `src-tauri/src/workflow/headless_diagnostics.rs` owns a host-facing
  diagnostics snapshot path that updates the same mutable diagnostics store
  from live scheduler/runtime queries.
- `pantograph-embedded-runtime` owns runtime-specific node diagnostics and
  model usage submission into the durable ledger.
- The frontend consumes backend projections, but it still renders raw
  diagnostics event payloads for the current debug-style event list.

The best reuse path is to keep the current reducers and projection DTOs as
read-model seeds, while replacing the current durable write model with typed
ledger events. Existing timing/run summary tables should become projections
derived from the event ledger, not primary diagnostic truth.

The biggest gaps are:

- no durable `diagnostic_events` append-only table
- no stable event envelope with event kind, payload schema version, source
  component, privacy class, retention class, payload hash, payload size, or
  payload reference
- no workflow version id or node version id in current timing/run projections
- current diagnostics grouping still depends on `graph_fingerprint`
- current scheduler diagnostics are inferred from snapshots instead of recorded
  as explicit scheduler decisions/events
- current Tauri overlay serializes workflow event payloads as untyped JSON
- current workflow trace persistence errors are ignored
- no projection rebuild contract
- no artifact/payload reference policy for node outputs, stream chunks, images,
  audio, or other large data

## Current Architecture Layers

### Durable Diagnostics Ledger

Current owner: `crates/pantograph-diagnostics-ledger`.

The existing ledger is typed and validates several records before persistence.
`ModelLicenseUsageEvent` captures client/session/bucket/run/workflow/model
identity, node lineage, license snapshot, output measurement, execution
guarantee, status, retention class, timestamps, and correlation id
(`records.rs`).

That is a strong precedent for the new architecture because it proves
backend-owned typed submission and validation already exist. It should not,
however, be treated as the final ledger pattern. It is a bespoke event table
family, not a generic typed event envelope.

Current storage is table-first:

- `model_license_usage_events` stores one durable usage-event family.
- `license_snapshots`, `model_output_measurements`, and `usage_lineage` hang
  from that event id.
- `workflow_timing_observations` stores timing facts keyed by workflow id and
  `graph_fingerprint`.
- `workflow_run_summaries` stores mutable/upserted run summary rows.
- `diagnostics_retention_policy` stores policy, but retention actions are not
  themselves ledger events.

Compatibility with the event-ledger target:

- Reuse the repository trait boundary and validation style.
- Reuse model/license usage structures as the seed payload for a
  `library.asset_used_by_run` or `model.license_usage_recorded` event family.
- Replace `schema_version` as table schema metadata with per-payload schema
  version in the event envelope.
- Replace direct timing/run summary persistence with projection writers derived
  from typed events.
- Replace prune-by-delete behavior for audit-relevant data with metadata
  retention plus payload expiration/externalization.

The current repository trait exposes record/query methods for usage events,
timing observations, run summaries, and pruning, but no generic append event,
projection rebuild, event validation, or projection checkpoint API.

### Workflow-Service Trace Store

Current owner: `crates/pantograph-workflow-service/src/trace`.

`WorkflowTraceEvent` is the closest current type to a future ledger event
contract. It covers run lifecycle, node lifecycle, progress, streams, waiting
for input, graph modification, incremental execution, runtime snapshots, and
scheduler snapshots.

The trace store applies those events to an in-memory `WorkflowTraceState`,
builds snapshots, derives terminal timing observations, and upserts run
summaries into the diagnostics ledger.

This is useful because it already contains:

- a typed event enum
- deterministic reducer logic
- a run/node projection model
- timing expectation enrichment
- scheduler/runtime projection fields

It is not sufficient as the durable event ledger because it lacks:

- `event_id`
- stable string `event_kind`
- per-payload `schema_version`
- `source_component` and `source_instance_id`
- privacy and retention classes
- payload hash, size, and reference
- workflow version id
- node version id
- model/runtime version ids in the generic envelope
- source-specific validation rules
- append-only durable storage

The trace store currently records terminal timing facts after updating the
in-memory state, then writes run summary and timing records into the ledger.
Those writes ignore errors. That is acceptable for best-effort debug
projections, but not for an audit ledger that will explain scheduler, runtime,
retention, and Library behavior. The event ledger plan needs an explicit write
failure policy for audit events.

### Scheduler Diagnostics

Current scheduler diagnostics are snapshot-derived. `apply_scheduler_snapshot`
looks at the latest session summary and queue items, then infers queue state,
admission outcome, decision reason, and wait time.

That is a projection pattern, not a durable event source. The planned scheduler
ledger must record explicit scheduler events such as:

- run accepted into queue
- estimate produced
- queue placement selected
- delayed for cache/model state
- promoted by client/admin action
- reservation created
- model load started/completed
- model unload started/completed
- run admitted
- run cancelled
- retry/fallback chosen

The existing snapshot reducer can still be useful as a transitional projection,
but it should not be the source of truth for scheduler audit history.

### Tauri Diagnostics Store And Overlay

Current owner: `src-tauri/src/workflow/diagnostics`.

The Tauri diagnostics store composes workflow-service trace snapshots with a
GUI overlay. The overlay records `DiagnosticsEventRecord` values with a
synthetic id, sequence, timestamp, event type string, workflow run id,
workflow id, node id, summary, and `serde_json::Value` payload.

The overlay is useful as a GUI projection prototype. It should not become the
durable event ledger because:

- event ids are run-local synthetic sequence ids
- event type is a transport/display string, not an allowlisted ledger kind
- payload is untyped JSON
- payload is produced by serializing a Tauri `WorkflowEvent`
- retention is an in-memory count limit
- no event source, privacy class, retention class, payload hash, or payload
  reference is recorded
- runtime and scheduler snapshots are mutable last-write-wins state

The overlay can survive as a rebuilt projection, or as a developer/admin raw
event view fed by sanitized ledger events. Normal run-centric pages should
consume page projections rather than raw overlay payloads.

### Headless Diagnostics Projection

Current owner: `src-tauri/src/workflow/headless_diagnostics.rs` and
`src-tauri/src/workflow/headless_diagnostics_transport.rs`.

Headless diagnostics are another projection path into the same diagnostics
store. A diagnostics snapshot request can query the scheduler, inspect session
state, query runtime capabilities, read gateway/runtime lifecycle snapshots,
collect managed runtime views, and then update the diagnostics store before
returning a projection.

This is useful because it centralizes snapshot assembly for host-facing
commands. It is also another reason the new architecture needs to distinguish
observed state projections from ledger facts. Current headless diagnostics
mutate scheduler/runtime projection state while answering a read request. Under
the typed event-ledger model, reads should not silently create durable audit
truth. If a read captures useful runtime/scheduler observations, it should emit
explicit typed observation events, or keep them as non-durable live projection
refreshes with clear naming.

### Tauri Workflow Event Transport

Current owner: `src-tauri/src/workflow/events.rs` and
`src-tauri/src/workflow/event_adapter`.

Tauri `WorkflowEvent` is a live UI transport contract. It carries node stream
chunks, node outputs, full workflow outputs, runtime snapshots, scheduler
snapshots, and diagnostics snapshots.

This transport should remain separate from the durable ledger contract.
Persisting it as-is would violate the typed event-ledger architecture because
the stream/output payloads can be large, sensitive, or unbounded. The future
ledger should persist typed metadata and payload references for I/O data, not
blindly embed values from UI transport events.

`diagnostics_bridge.rs` is still an important migration point: it already
observes node-engine events, translates them to Tauri events, and records
diagnostics snapshots. Future implementation should add typed ledger event
builders at this boundary or lower in the runtime where richer node context is
available.

### Embedded Runtime Diagnostics

Current owner: `crates/pantograph-embedded-runtime`.

The embedded runtime has two important diagnostics paths:

- `node_execution_diagnostics.rs` adapts node-engine events into enriched,
  typed `NodeExecutionDiagnosticEvent` values.
- `node_execution_ledger.rs` builds and submits typed model-usage events into
  the durable diagnostics ledger.

This is the cleanest current source for typed node/runtime facts. It already
has attribution, workflow id, workflow run id, node id, node type, attempt,
occurred timestamp, guarantee level, lineage, contract version/digest, port id,
progress/message/error, output summaries, and structured progress details.

That makes it a better seed for `node.*` and `io.*` events than the Tauri
transport events. It still needs the ledger envelope, source component,
payload schema version, privacy/retention classification, payload hashes, and
workflow/node version identifiers.

### Frontend Diagnostics

Current owner: `src/services/diagnostics`, `src/stores/diagnosticsStore.ts`,
and `src/components/diagnostics`.

The frontend mostly consumes backend projections, which matches the planned
architecture. The main conflict is that `DiagnosticsEventRecord.payload` is
typed as `unknown` and the diagnostics event component renders raw payload JSON.

That can remain as a temporary developer-facing view, but the run-centric GUI
should not depend on raw event payloads. Scheduler, Diagnostics, Graph,
I/O Inspector, Library, and Network pages should receive specific projections
with typed fields. Raw event inspection should be privileged and separate.

There is also existing type drift: Rust `DiagnosticsSchedulerSnapshot` includes
scheduler diagnostics, while the frontend diagnostics type omits that field.
The new projection APIs should freeze generated or directly tested DTO
contracts to avoid repeating that drift.

## Envelope Compatibility Matrix

| Planned envelope field | Current support | Required change |
| --- | --- | --- |
| `event_id` | Usage events have `usage_event_id`; overlay has synthetic run-local ids; trace events have none. | Add global event id at ledger boundary. |
| `event_kind` | Trace enum variants and Tauri display strings exist; usage events are implicit by table. | Freeze allowlisted string event kinds in a contract module. |
| `schema_version` | Ledger table stores storage schema version on usage rows. | Add per-event payload schema version separate from DB schema version. |
| `source_component` | Absent. | Require backend source ownership per event kind. |
| `source_instance_id` | Absent. | Add optional/required source instance for runtime, scheduler, retention, and future network nodes. |
| `occurred_at_ms` | Present in runtime diagnostic events and many workflow events. | Normalize into all event builders. |
| `recorded_at_ms` | Present on timing/run summary records; not universal. | Add at append boundary. |
| `workflow_run_id` | Present widely. | Keep required for run/node/io/scheduler run-scoped events. |
| `workflow_id` | Present widely, sometimes optional. | Keep and validate by event scope. |
| `workflow_version_id` | Absent. | Stage 01 must make this available before diagnostics are authoritative. |
| `workflow_semver` | Absent. | Add from workflow versioning contract. |
| `node_id` / `node_type` | Present in trace/runtime/usage paths. | Keep and validate by event kind. |
| `node_version` | Partially represented by contract version/digest. | Normalize into node version id/semver/fingerprint fields. |
| `runtime_id` / `runtime_version` | Runtime id appears in timing and runtime metrics; version is inconsistent. | Add stable runtime identity/version contract to events and projections. |
| `model_id` / `model_version` | Model usage has id/revision/hash. | Map to normalized model version fields. |
| `client_id` / `client_session_id` / `bucket_id` | Present in model usage and runtime node diagnostics. | Require where attribution exists; propagate to workflow trace events. |
| `scheduler_policy_id` | Absent. | Add from scheduler policy/version contract. |
| `retention_policy_id` | Retention policy has id, but event rows do not. | Add to retention and payload events. |
| `privacy_class` | Absent. | Require validation per event kind. |
| `retention_class` | Present for model usage only. | Require for all events. |
| `payload_hash` | Absent for generic payloads. | Compute at append boundary. |
| `payload_size_bytes` | Model output measurements include sizes, but not generic payload size. | Compute for embedded payload or referenced artifact metadata. |
| `payload_ref` | Absent from diagnostics ledger; managed runtime has unrelated job artifact state. | Add payload/artifact reference contract before retaining large I/O. |
| `payload_json` | Overlay stores raw JSON; model usage stores typed normalized rows. | Only store bounded, typed, validated payload JSON. |

## Event Family Mapping

### `run.*`

Current sources:

- `WorkflowTraceEvent::RunStarted`
- `WorkflowTraceEvent::RunCompleted`
- `WorkflowTraceEvent::RunFailed`
- `WorkflowTraceEvent::RunCancelled`
- Tauri `WorkflowEvent::Started`, `Completed`, `Failed`, `Cancelled`

Migration:

- Create typed run event payloads in the diagnostics ledger crate.
- Include workflow identity, workflow version id, workflow semver,
  run snapshot id, scheduler policy id, client/session/bucket, and run status.
- Derive run list and run detail projections from these events.
- Convert `workflow_run_summaries` into a projection table.

### `node.*`

Current sources:

- `WorkflowTraceEvent::NodeStarted`
- `WorkflowTraceEvent::NodeProgress`
- `WorkflowTraceEvent::NodeStream`
- `WorkflowTraceEvent::NodeCompleted`
- `WorkflowTraceEvent::NodeFailed`
- `NodeExecutionDiagnosticEvent`
- node-engine workflow events through Tauri adapters

Migration:

- Prefer embedded runtime `NodeExecutionDiagnosticEvent` as the typed source
  because it has attribution, guarantee, lineage, contract version/digest, and
  output summaries.
- Normalize contract version/digest into node version fields supplied by Stage
  01.
- Treat node stream/output data as metadata plus payload reference, not raw
  embedded values.
- Derive node timelines and timing expectations from `node.*` events.

### `io.*`

Current sources:

- Tauri `NodeStream.chunk`
- Tauri `NodeCompleted.outputs`
- Tauri `Completed.outputs`
- embedded runtime `NodeOutputSummary`
- workflow service I/O contracts for declared inputs/outputs

Migration:

- Add typed I/O artifact payloads with MIME/type/category, byte size, hash,
  port id, node id, workflow run id, retention class, privacy class, and
  payload reference.
- Store only bounded previews inline.
- Make I/O Inspector projections use artifact metadata and payload references.
- Record payload truncation, expiration, deletion, and externalization events.

### `scheduler.*`

Current sources:

- workflow-service scheduler snapshots
- Tauri `SchedulerSnapshot`
- inferred queue state in `apply_scheduler_snapshot`

Migration:

- Add explicit scheduler event kinds for estimates, queue placement, delay,
  admission, reservation, cancellation, promotion, model load/unload, retry,
  fallback, client action, and admin override.
- Preserve snapshots as projections or periodic observation events, but do not
  rely on snapshots as the only audit source.
- Include scheduler policy id and source authority on every scheduler event.

### `runtime.*`

Current sources:

- `WorkflowTraceEvent::RuntimeSnapshotCaptured`
- Tauri `RuntimeSnapshot`
- managed runtime lifecycle snapshots
- runtime capability observations

Migration:

- Add runtime capability, health, load, active model target, warmup, reuse,
  and lifecycle events.
- Include source instance id so future local-network nodes can report distinct
  runtime state.
- Use Network page local-node projections from these events before Iroh exists.

### `library.*`

Current sources:

- model/license usage ledger
- Pumas-facing model/library APIs
- model execution capability usage submissions

Migration:

- Convert current model usage event into a typed library/model event family.
- Add Pumas search, download, delete, asset access, asset used by run,
  cache hit/miss, and network byte observation events.
- Keep model/license diagnostics typed and validated; do not loosen them into
  free-form JSON.

### `retention.*`

Current sources:

- retention policy table
- usage/timing prune commands

Migration:

- Record policy changes, cleanup started/completed/failed, and per-item
  payload-retention actions.
- Preserve audit metadata after payload bodies expire.
- Avoid deleting the only row that explains an audit-relevant event happened.

## Migration Slices

1. Add typed event envelope types in `pantograph-diagnostics-ledger`: event id,
   kind enum/string, schema version, source component, timestamps, correlation
   fields, privacy class, retention class, payload metadata, and validation.
2. Add an append-only event table. Because legacy support is not required, use
   a clean schema cutover instead of a compatibility migration if that yields a
   cleaner design.
3. Add event-family payload modules for `run.*`, `node.*`, `io.*`,
   `scheduler.*`, `runtime.*`, `library.*`, and `retention.*`.
4. Add projection tables and rebuild APIs. Current run summaries, timing
   expectations, scheduler summaries, and diagnostics overlays should become
   rebuildable projections.
5. Bridge workflow-service trace events into typed ledger events. Keep the
   current reducer as a projection reducer, not as the durable event contract.
6. Bridge embedded runtime node diagnostics into `node.*` and `io.*` typed
   events. Use output summaries and payload refs instead of raw Tauri payloads.
7. Convert model/license usage submissions into the library/model event family.
8. Add explicit scheduler/runtime/library/retention producers.
9. Expose API projections only for normal GUI pages. Add raw event access only
   later as a protected developer/admin feature.

## Source Findings

Key current source points:

- `crates/pantograph-diagnostics-ledger/src/schema.rs` defines schema version
  4 and concrete tables for model usage, timing observations, run summaries,
  and retention policy.
- `crates/pantograph-diagnostics-ledger/src/repository.rs` exposes typed
  usage/timing/summary repository methods, but no append-only event method.
- `crates/pantograph-diagnostics-ledger/src/timing.rs` still keys timing
  history by `workflow_id` plus `graph_fingerprint`.
- `crates/pantograph-workflow-service/src/trace/types.rs` defines typed
  `WorkflowTraceEvent` variants that can seed run/node/runtime/scheduler
  event builders.
- `crates/pantograph-workflow-service/src/trace/store.rs` derives timing and
  run summaries after in-memory trace updates, then ignores persistence errors.
- `crates/pantograph-workflow-service/src/trace/scheduler.rs` infers scheduler
  state from snapshots.
- `src-tauri/src/workflow/diagnostics/overlay.rs` builds GUI event records by
  serializing Tauri workflow events into untyped JSON payloads.
- `src-tauri/src/workflow/events.rs` carries live UI stream/output payloads and
  should stay separate from durable ledger contracts.
- `src-tauri/src/workflow/event_adapter/diagnostics_bridge.rs` is a practical
  bridge point for transitional diagnostics projections.
- `src-tauri/src/workflow/headless_diagnostics.rs` updates diagnostics
  scheduler/runtime projection state while constructing host-facing snapshot
  responses.
- `crates/pantograph-embedded-runtime/src/node_execution_diagnostics.rs`
  already builds enriched typed node diagnostics from node-engine events.
- `crates/pantograph-embedded-runtime/src/node_execution_ledger.rs` already
  submits typed model usage facts into the current durable diagnostics ledger.
- `src/services/diagnostics/types.ts` exposes raw diagnostics payloads as
  `unknown`, confirming that frontend pages are not yet protected by typed
  projection contracts.

## Architectural Risks

### Persisting Tauri Events As Ledger Events

Risk: Tauri events include raw stream chunks, node outputs, and workflow outputs.
Persisting them directly would create unbounded payload and privacy problems.

Control: Durable ledger producers must use typed backend event builders and
payload reference policy. Tauri events remain live transport.

### Treating Graph Fingerprints As Workflow Versions

Risk: Timing expectations and run summaries currently group by
`graph_fingerprint`, but planned diagnostics need workflow version ids based on
graph topology plus node versions.

Control: Stage 01 must land workflow version/run snapshot ids before Stage 03
turns diagnostics into authoritative historical comparisons.

### Silent Audit Write Failure

Risk: The trace store currently ignores timing/run summary persistence errors.
That can hide data loss once diagnostics become an audit surface.

Control: Event append APIs need explicit error handling. Implementation should
decide which event families are must-record, best-effort, or buffered for
retry, and tests should cover the behavior.

### Projection Drift

Risk: Current frontend and Rust diagnostics DTOs already show drift around
scheduler diagnostics fields.

Control: New projection APIs should use contract tests or generated bindings
for route DTOs that cross the Tauri/frontend boundary.

### Read Requests Creating Diagnostic State

Risk: Headless diagnostics snapshot reads currently refresh scheduler/runtime
projection state as part of request handling. In an event-ledger world, that
can blur whether a fact was observed during execution, observed during a user
inspection, or simply projected from current runtime state.

Control: Event kinds should distinguish execution facts from observer facts.
Read-time observations should either remain non-durable projection refreshes or
be recorded as explicit `runtime.*`/`scheduler.*` observation events with
source component and occurred/recorded timestamps.

### Bespoke Tables Reappearing

Risk: Adding one table per new diagnostic concern would recreate the current
fragmentation.

Control: New diagnostic concerns add typed payload modules and projection
reducers under the event-ledger boundary. Bespoke tables are allowed only as
read projections, not independent write models.

## Compatibility Verdict

The current codebase is compatible with the planned typed event-ledger
architecture if the implementation treats the new ledger as the durable source
of truth and demotes current timing summaries, run summaries, overlays, and
frontend event lists to projections.

The architecture should not try to preserve old graph-fingerprint diagnostics
semantics. Since backwards compatibility is not required, the cleanest path is
to introduce a new ledger schema and rebuild the diagnostics API around
workflow version ids, run snapshots, typed event families, retention-aware
payload metadata, and rebuildable projections.

## Implementation Implications For Existing Plans

- Stage 01 must produce stable workflow version ids, node version ids, and run
  snapshot ids before diagnostic events become authoritative.
- Stage 02 scheduler work must emit typed scheduler ledger events instead of
  only improving snapshot projections.
- Stage 03 should own the event envelope, payload modules, SQLite append table,
  projection rebuild APIs, retention event policy, and artifact reference
  contract.
- Stage 04 must expose run-centric projections rather than raw event rows.
- Stages 05 and 06 should consume those projections and avoid frontend-owned
  interpretation of event payload JSON.
- Stage 07 should add source-audit gates: every new diagnostics producer must
  identify its event kinds, allowed source component, validation rules,
  retention/privacy class, projection ownership, and test coverage.

## Evidence Map

The analysis is grounded in the following current source locations:

- `crates/pantograph-diagnostics-ledger/src/schema.rs`: schema version 4,
  usage tables, timing tables, run summary tables, and retention policy.
- `crates/pantograph-diagnostics-ledger/src/repository.rs`: repository methods
  for usage, timing, run summary, and pruning.
- `crates/pantograph-diagnostics-ledger/src/timing.rs`: timing observations
  and run summaries keyed by workflow id and graph fingerprint.
- `crates/pantograph-diagnostics-ledger/src/records.rs`: typed model/license
  usage event and current diagnostics projection.
- `crates/pantograph-workflow-service/src/trace/types.rs`: typed
  `WorkflowTraceEvent` variants and trace projection DTOs.
- `crates/pantograph-workflow-service/src/trace/store.rs`: event recording,
  timing/run-summary derivation, and ignored persistence errors.
- `crates/pantograph-workflow-service/src/trace/timing.rs`: conversion from
  terminal trace events into timing observations and run summaries.
- `crates/pantograph-workflow-service/src/trace/scheduler.rs`: scheduler state
  inferred from snapshots.
- `src-tauri/src/workflow/diagnostics/overlay.rs`: in-memory GUI event overlay
  and untyped serialized payload extraction.
- `src-tauri/src/workflow/diagnostics/store.rs`: composition of trace snapshots
  with overlay and mutable runtime/scheduler state.
- `src-tauri/src/workflow/events.rs`: live Tauri workflow event transport
  carrying node streams, node outputs, workflow outputs, runtime snapshots,
  scheduler snapshots, and diagnostics snapshots.
- `src-tauri/src/workflow/event_adapter/diagnostics_bridge.rs`: current bridge
  from node-engine events to Tauri events plus diagnostics snapshots.
- `src-tauri/src/workflow/headless_diagnostics.rs`: host-facing diagnostics
  projection assembly and read-time scheduler/runtime projection updates.
- `src-tauri/src/workflow/headless_diagnostics_transport.rs`: Tauri command
  path for diagnostics snapshot responses.
- `crates/pantograph-embedded-runtime/src/node_execution_diagnostics.rs`:
  enriched typed node diagnostics events with attribution and contract facts.
- `crates/pantograph-embedded-runtime/src/node_execution_ledger.rs`: typed
  model usage submission into the current durable diagnostics ledger.
- `src/services/diagnostics/types.ts`: frontend diagnostics projection types
  and raw `unknown` payload exposure.
