# 03: Diagnostics, Retention, And Audit Ledgers

## Status

In progress. Stage `01` has started the version-aware diagnostics filter
cutover by adding workflow-version and node contract version/digest filters to
the existing model/license usage diagnostics path. Stage `03` has started the
typed event ledger bootstrap in `pantograph-diagnostics-ledger` with validated
event contracts, append-only SQLite storage, bounded payloads, monotonic
`event_seq`, and `projection_state` cursor persistence. The workflow-service
run snapshot path now emits typed `run.snapshot_accepted` events, and the
workflow-session scheduler emits typed `scheduler.estimate_produced` and
`scheduler.queue_placement` events when a diagnostics ledger is configured.
Retention, I/O, Library/Pumas emitters, and materialized projection tables
remain pending.

## Objective

Introduce a typed append-only diagnostic event ledger and durable
materialized projections so run-centric pages can query version-aware
diagnostics, I/O artifact metadata, retroactive global retention state,
retention cleanup events, and Pumas/Library usage audits without losing
auditability when payloads expire.

## Scope

### In Scope

- Workflow/node/model/runtime/version-aware diagnostics filters.
- Typed diagnostic event envelope, event families, payload schema versions,
  validation rules, and backend-owned event builders.
- I/O artifact metadata records for workflow inputs, workflow outputs, node
  inputs, node outputs, intermediate artifacts, and final artifacts.
- Payload retention state and deletion/expiration reasons.
- Global retention policy versioning and retroactive cleanup behavior.
- Pumas/Library audit records for search, download, deletion, asset access,
  run usage, network bytes where available, and cache hits/misses.
- Rebuildable materialized query projections for active-run page views,
  timelines, galleries, audit summaries, and aggregate diagnostics.
- Durable projection state, projection versions, and event cursors so normal
  page/API reads use incremental materialized projections rather than full
  ledger replay.

### Out of Scope

- Implementing every media preview renderer.
- Replacing all payload storage.
- Per-workflow, per-run, or per-artifact retention policies beyond future
  extensibility.
- Distributed asset transfer audit for Iroh peers.

## Inputs

### Problem

The I/O Inspector and Diagnostics pages need to show what data flowed through a
run and why some payloads may no longer be retained. Library and Pumas activity
also needs audit history because assets can affect scheduling, diagnostics,
network traffic, and model usage accountability.

### Constraints

- Retention policy is global initially.
- Policy changes are retroactive.
- Payload data may expire, but audit metadata must remain.
- Diagnostics grouping must distinguish workflow/node/model/runtime versions.
- Pumas content is manageable, not read-only.
- Diagnostic writes must use allowlisted typed event payloads. Free-form
  frontend or client-supplied diagnostic metadata is not accepted.
- Page APIs consume projections by default, not raw event rows.
- Rebuildable projections are a repair and migration capability. Normal
  startup, page load, and query paths must not replay all diagnostic events.
- Event payloads are bounded metadata and payload references. Stream chunks,
  raw media bytes, token-by-token output, and full artifact bodies do not
  belong in the event ledger.

### Assumptions

- `pantograph-diagnostics-ledger` remains the likely durable owner for
  the shared typed event ledger and projection metadata unless Milestone 1
  chooses a new shared diagnostics-event crate. Event storage ownership is
  decided once for all event families, not separately per feature.
- Payload blobs may live outside the diagnostics ledger; the ledger stores
  metadata, references, hashes, size/type facts, retention status, and deletion
  reasons.
- Initial retention cleanup can be explicit/admin-triggered before a background
  worker is added, if that reduces lifecycle risk.

### Dependencies

- Stage `01` workflow/run version snapshots.
- Stage `02` scheduler events and policy ids.
- `diagnostic-event-ledger-architecture.md`.
- `pantograph-diagnostics-ledger` schema/repository.
- Pumas APIs and metadata.
- Library/model/runtime registry metadata.
- Frontend I/O Inspector and Library API projections from Stage `04`.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Retention deletes payloads without retaining enough metadata. | High | Persist metadata and deletion reason before deleting payloads. |
| Diagnostics become arbitrary unvalidated JSON. | High | Require typed Rust payloads, event kind allowlists, schema versions, and validation before persistence. |
| Feature code bypasses ledger ownership. | High | Expose narrow event builder APIs and reject direct raw event writes outside ledger tests/migrations. |
| Retroactive cleanup races with active inspection or execution. | High | Define cleanup ownership, locking, and active-run exclusions. |
| Audit tables grow without bounds. | Medium | Define metadata retention invariant and separate payload cleanup from audit compaction. |
| Pumas network/use audits miss caller/run attribution. | Medium | Require run/session/bucket/client actor where available and explicit unknown actor otherwise. |
| Query filters become slow as history grows. | Medium | Add indexes with migration tests for version/time/status filters. |
| Event-family owners create their own durable stores. | High | Keep one shared event envelope, append path, validation boundary, and incremental materialized projection model. |
| Naive projection rebuild replays all events for every page or projection. | High | Store monotonic event cursors in `projection_state`, update hot projections incrementally, and reserve full rebuild for migration, repair, projection-version changes, and tests. |
| Event volume grows because producers log overly granular stream or payload data. | High | Persist bounded metadata and payload references only; reject oversized embedded payloads and define event granularity rules per family. |

## Definition of Done

- Diagnostics queries can filter by workflow, workflow version, node version,
  model version, runtime version, scheduler policy, graph settings, session,
  bucket, input profile or input characteristics, date range, status, and
  retention completeness where data exists.
- Typed event ledger persists only validated events from allowlisted backend
  producers.
- Each event kind has a typed payload, schema version, required envelope fields,
  privacy class, retention class, and maximum embedded payload size.
- I/O artifact metadata records survive payload expiration.
- Global retention policy changes can retroactively mark/delete payloads and
  audit what changed.
- Library/Pumas audit events are persistently queryable.
- Projections for active-run diagnostics, scheduler timelines, I/O galleries,
  retention state, and Library usage are rebuildable from the ledger.
- Normal projection operation is incremental: every event has a monotonic
  `event_seq`, every projection stores `projection_version`,
  `last_applied_event_seq`, and status, and page/API reads use materialized
  projection tables.
- Full projection rebuild is available only for explicit rebuild commands,
  migration, repair, projection-version changes, and tests.
- Terminal runs have compact summary rows for normal run-list and run-detail
  reads so old completed runs do not require timeline replay.
- Projection facets preserve future comparison keys for run-vs-run,
  workflow-version, runtime-version, model-version, device, and input-profile
  comparisons even when first-pass comparison workflows are out of scope.
- Tests cover event validation, retention metadata survival, incremental
  projection application, projection rebuild, cursor recovery, idempotency,
  non-trivial event counts, and version-aware filtering.

## Milestones

### Milestone 1: Typed Event Contract And Storage Ownership

**Goal:** Decide durable ownership and freeze the typed event ledger contract
before implementation.

**Tasks:**

- [x] Decide whether the typed diagnostic event ledger lives in
  `pantograph-diagnostics-ledger` or a new shared diagnostics-event crate.
  This decision applies to all event families; do not approve per-family
  sibling repositories.
- [x] Define event envelope fields, event id behavior, timestamps, source
  ownership, correlation identifiers, privacy classes, retention classes,
  payload hashes, embedded payload size limits, payload references, and
  monotonic `event_seq`.
- [x] Define initial event families: `scheduler.*`, `run.*`, `node.*`,
  `io.*`, `library.*`, `runtime.*`, and `retention.*`.
- [x] Define typed payload structs and schema versions for first-pass event
  kinds.
- [x] Define event builders and validation errors. Direct raw event writes
  should be test/migration-only.
- [ ] Define I/O artifact metadata contract.
- [ ] Define retention policy/version and artifact retention-state contract.
- [ ] Define Pumas/Library audit event contract.
- [x] Define centralized validators for artifact payload references,
  Library/Pumas resource identifiers, external references, and any filesystem
  paths accepted by download/delete/access operations.
- [x] Define ledger indexes, projection tables, and migration strategy for
  version-aware diagnostics.
- [x] Define `projection_state` with projection name, projection version,
  last applied event sequence, status, and rebuild timestamp.
- [ ] Define hot, warm, and cold projection classes and which component owns
  synchronous, asynchronous, lazy, and explicit rebuild application.
- [ ] Define event granularity rules that reject chunk/token/raw-artifact event
  spam and require bounded metadata plus payload references.
- [ ] Define event family ownership: `run.*` owns execution lifecycle,
  `scheduler.*` owns scheduling decisions/control/resource events, and
  projections join families instead of duplicating facts.

**Verification:**

- Schema/contract tests compile.
- Validation tests reject unsupported event kinds, missing required correlation
  fields, unsupported schema versions, disallowed producers, and oversized
  embedded payloads.
- Validation tests reject event payloads that exceed embedded payload limits or
  attempt to store raw stream/media/artifact bodies instead of references.
- Validation tests reject unsafe artifact references, invalid Library/Pumas
  resource identifiers, and paths that do not resolve through approved
  workspace/cache roots.
- README or ADR updates record ownership decisions.

**Status:** In progress. `pantograph-diagnostics-ledger` is the accepted
storage owner. First-pass scheduler/run/I/O/library/runtime/retention event
contracts, validation errors, source allowlists, payload hashes, embedded
payload limits, SQLite `diagnostic_events`, `projection_state`, and safe
payload-reference scheme validation have been implemented. Detailed I/O
retention contracts, Pumas download/delete path validators, and hot/warm
projection ownership details are partially pending; scheduler timeline
projection ownership is implemented as the first hot projection.

### Milestone 2: Ledger Persistence And Incremental Projections

**Goal:** Persist typed events and maintain first-pass materialized
projections incrementally, while preserving explicit full-rebuild support for
repair, migration, projection-version changes, and tests.

**Tasks:**

- [x] Implement append-only event persistence.
- [x] Assign each event a monotonic durable `event_seq` and index event
  queries by `event_seq`, `event_kind`, `workflow_run_id`, version ids, node
  ids, model/runtime ids, and status fields needed by projections.
- [x] Implement `projection_state` persistence and cursor updates for every
  first-pass projection.
- [x] Implement hot projection updates for run summary, run detail/current
  status, scheduler timeline, and active-run I/O artifact metadata.
  - Run summary, run list, scheduler timeline, and selected-run detail/current
    status are implemented as incremental materialized projections. I/O
    artifact metadata is implemented as a bounded metadata/reference projection
    for selected-run and active-run I/O Inspector reads.
- [ ] Implement warm projection drains for diagnostics summary, retention
  completeness, workflow-version performance, model/runtime comparison facets,
  and Library usage where first-pass event families exist.
  - First-pass Library usage warm projection is implemented from
    `library.asset_accessed` events. Diagnostics summary, retention
    completeness, workflow-version performance, and model/runtime comparison
    facets remain pending.
- [x] Implement explicit full-rebuild commands for migration, corruption
  repair, projection-version changes, and tests. Ordinary startup and page
  load must not call these commands.
- [ ] Add compact terminal run summary rows for completed/failed/cancelled runs
  so normal historic run-list and run-detail reads do not replay timelines.
- [x] Add workflow execution version and node version fields to projections.
- [x] Add model/runtime/version and scheduler policy filters where not already
  present.
- [x] Add retention-completeness filter/projection.
- [x] Add query outputs that report mixed-version counts or facets.
- [ ] Preserve comparison-ready facets for workflow version, node version,
  model/runtime version, device/network node, scheduler policy, graph settings,
  and input profile where available.

**Verification:**

- Repository tests cover event append, query, incremental projection
  application, cursor persistence, duplicate-application idempotency, and
  explicit projection rebuild.
- Repository tests cover each new filter.
- Tests cover mixed-version result metadata.
- Tests cover startup/reopen recovery applying only events after
  `last_applied_event_seq`.
- Projection rebuild tests include a non-trivial event count and assert normal
  query paths use materialized projections rather than full event replay.
- Persistence, migration, replay, and projection tests each own isolated
  durable resources: SQLite/database files, temporary roots, payload stores,
  and cache paths must not be shared between parallel tests.

**Status:** In progress. Existing model/license usage projections now carry
workflow-version fields and can filter by node contract version/digest. Typed
event append and projection cursor persistence are implemented. The first
incremental hot read model, `scheduler_timeline_projection`, now drains
`run.snapshot_accepted`, `scheduler.estimate_produced`, and
`scheduler.queue_placement` events by cursor into materialized rows for page/API
queries. Typed `run.started` and `run.terminal` events are added to the same
timeline so selected-run history can cover execution start and terminal
status without replaying raw legacy diagnostics. A first `run_list_projection`
now materializes one row per run from the same cursor-drained lifecycle events
for scheduler-page list reads. `run_detail_projection` now materializes the
selected run's lifecycle payloads, snapshot identity, client/session/bucket
identity, current status, terminal summary, and timeline event count for
selected-run pages. `io_artifact_projection` now materializes current bounded
artifact metadata and payload references from `io.artifact_observed` events by
run, node, role, and event cursor for I/O Inspector reads. It also applies
typed `retention.artifact_state_changed` events incrementally so cleanup,
expiration, deletion, externalization, and truncation decisions remain audited
in the ledger while the gallery reads one current row per run artifact.
Projection filters now cover workflow version, scheduler policy,
runtime/model ids, media type, retention policy/state, and artifact role where
those facts exist in the read model. Retention completeness is now queryable as
state/count summaries over the I/O artifact projection for the same run and
artifact-scope filters. Run-list responses now include backend projection
facets for workflow version, run status, scheduler policy, and retention
policy so mixed-version diagnostics do not depend on a client-paged sample.
The first warm projection, `library_usage_projection`, now aggregates
Library/Pumas asset access counts, distinct run counts, network bytes, last
access facts, and workflow-version run links. Remaining warm drains and
mixed-version facet outputs remain pending.

### Milestone 3: I/O Artifact Metadata And Retention

**Goal:** Persist I/O metadata and retention state independently from payload
availability.

**Tasks:**

- [ ] Record workflow inputs/outputs and node input/output metadata.
- [ ] Store artifact type, size, content hash where available, producer node,
  consumer node, run id, timestamps, and payload reference.
  - First-pass projection stores artifact role, media type, size, content
    hash, event node identity, run id, timestamps, payload reference, typed
    retention state, and retention reason. Explicit producer/consumer edge
    roles remain pending with richer I/O event payloads.
- [x] Emit typed artifact events for observation, retention state changes,
  truncation, externalization, expiration, and deletion.
- [ ] Add global retention policy record and policy version.
  - Existing standard local retention policy is now exposed as a first-class
    backend/API query. Policy mutation/version history remains pending.
- [ ] Define first-pass global retention settings for final outputs, workflow
  inputs, intermediate node I/O, failed-run data, maximum artifact size,
  maximum total storage, media behavior, compression/archive behavior, and
  cleanup trigger/status.
- [ ] Add retroactive cleanup command that updates metadata before deleting or
  expiring payloads.
- [ ] Emit typed `retention.*` events with policy version, timestamp, actor,
  affected artifact, and reason.
- [x] Update affected hot/warm projections through event cursors rather than
  direct page-time artifact ledger scans.

**Verification:**

- Tests prove payload deletion leaves metadata queryable.
- Tests cover global policy changes affecting old runs.
- Tests cover active or pinned data behavior if pinning is introduced.
- Tests cover artifact reference/path validation before payload metadata or
  retention state can be persisted.

**Status:** Not started.

### Milestone 4: Library And Pumas Audit

**Goal:** Record asset operations and usage with enough context for Library,
Scheduler, and Diagnostics pages.

**Tasks:**

- [ ] Wrap or instrument Pumas model search/download/delete/access paths.
- [ ] Emit typed `library.*` events for asset access by run, session, bucket,
  client, or GUI actor where available.
- [ ] Emit typed cache hit/miss and network byte observations where available.
- [x] Add Library usage projections: used by active run, used by N runs, last
  accessed, total access count, linked workflow/node versions.
- [x] Update Library usage counts through warm projection drains with recorded
  projection freshness.
- [ ] Ensure audit events are queryable without requiring payload retention.

**Verification:**

- Repository tests cover typed Pumas audit event validation, persistence, and
  queries.
- Integration tests cover representative model search/download/delete audit
  paths if available locally.
- Tests cover Pumas/Library resource validation for search/download/delete and
  prove rejected operations do not emit misleading usage audit events.

**Status:** Not started.

## Ownership And Lifecycle Note

If retention cleanup becomes a background task, it must have one owner, explicit
start/stop lifecycle, cancellation behavior, overlap prevention, and tests that
prove cleanup cannot run concurrently against the same workspace. Manual/admin
cleanup may be used first to avoid unowned timers.

Diagnostic event production must also have explicit owners. Scheduler, runtime,
node execution, retention cleanup, Pumas/Library wrappers, and local observers
may produce events only through their approved typed builders.

Projection application has separate ownership. The diagnostics ledger owns the
append boundary, event sequence, projection state rows, and explicit rebuild
commands. Hot projection updates run synchronously or near-synchronously with
event append when they are required for Scheduler, run detail, current status,
or active-run I/O pages. Warm projection drains must have one lifecycle owner,
record their cursor before yielding ownership, prevent overlapping drains for
the same projection, and expose stale/catching-up status to API projections.
Cold rebuild commands are admin/migration/repair paths and must not run
implicitly on page load.

## Re-Plan Triggers

- Payload storage is not discoverable from run/artifact metadata.
- Pumas APIs do not expose enough information for download/delete/access audit.
- Retention cleanup needs a background worker earlier than expected.
- Version-aware query performance requires a larger storage redesign.
- Event volume requires projection snapshotting, compaction, or partitioning.
- A normal startup, page load, or API query requires full ledger replay instead
  of reading materialized projections.
- Hot projections cannot stay current without blocking event append or run
  execution for unacceptable time.
- Warm projection lag cannot be represented clearly enough for API/frontend
  consumers.
- Raw developer event inspection is needed earlier than planned.

## Completion Summary

### Completed

- 2026-04-27: Added workflow execution version and node contract
  version/digest filters to the existing model/license usage diagnostics
  projection. This is a transitional Stage `01` filter cutover, not the full
  typed event ledger from this stage.
- 2026-04-27: Updated the diagnostics plan to treat projections as durable
  materialized read models with event cursors. Rebuildable now means explicit
  rebuild support for migration, repair, projection-version changes, and tests,
  not full replay during normal startup or page reads.

### Deviations

- None.

### Follow-Ups

- Decide typed event ledger storage ownership in Milestone 1.
- Decide whether pinning belongs in first implementation or stays future.
- During implementation, audit any already-added diagnostics filters and
  summaries so they either become materialized projections with cursors or stay
  explicitly documented transitional query paths until Stage `03` cutover.

### Verification Summary

- 2026-04-27: `cargo test -p pantograph-diagnostics-ledger
  query_usage_events_filters_by_node_contract_version_and_digest` passed.
- 2026-04-27: `cargo test -p pantograph-workflow-service
  workflow_diagnostics_usage_query_delegates_to_ledger_and_summarizes_events`
  passed.
- 2026-04-27: `cargo test -p pantograph-diagnostics-ledger` and
  `cargo test -p pantograph-workflow-service` passed.
- 2026-04-27: Documentation-only projection strategy update. No code tests
  required.
- 2026-04-27: `cargo test -p pantograph-diagnostics-ledger
  diagnostic_event_ledger_rejects_unsafe_payload_refs` passed.
- 2026-04-27: `cargo test -p pantograph-diagnostics-ledger
  diagnostic_event_ledger_rejects_unsafe_library_asset_ids` passed.
- 2026-04-27: `cargo test -p pantograph-workflow-service
  workflow_library_usage_query_validates_bounds` passed.

### Traceability Links

- Requirement sections: Diagnostics Requirements, I/O Inspector Requirements,
  Retention Policy Requirements, Library Requirements, Pumas Audit
  Requirements.
