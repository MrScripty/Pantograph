# 03: Diagnostics, Retention, And Audit Ledgers

## Status

Draft plan. Not implemented.

## Objective

Introduce a typed append-only diagnostic event ledger and rebuildable
projections so run-centric pages can query version-aware diagnostics, I/O
artifact metadata, retroactive global retention state, retention cleanup
events, and Pumas/Library usage audits without losing auditability when
payloads expire.

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
- Rebuildable query projections for active-run page views, timelines,
  galleries, audit summaries, and aggregate diagnostics.

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
| Event-family owners create their own durable stores. | High | Keep one shared event envelope, append path, validation boundary, and projection rebuild model. |

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
- Projection facets preserve future comparison keys for run-vs-run,
  workflow-version, runtime-version, model-version, device, and input-profile
  comparisons even when first-pass comparison workflows are out of scope.
- Tests cover event validation, retention metadata survival, projection rebuild,
  and version-aware filtering.

## Milestones

### Milestone 1: Typed Event Contract And Storage Ownership

**Goal:** Decide durable ownership and freeze the typed event ledger contract
before implementation.

**Tasks:**

- [ ] Decide whether the typed diagnostic event ledger lives in
  `pantograph-diagnostics-ledger` or a new shared diagnostics-event crate.
  This decision applies to all event families; do not approve per-family
  sibling repositories.
- [ ] Define event envelope fields, event id behavior, timestamps, source
  ownership, correlation identifiers, privacy classes, retention classes,
  payload hashes, embedded payload size limits, and payload references.
- [ ] Define initial event families: `scheduler.*`, `run.*`, `node.*`,
  `io.*`, `library.*`, `runtime.*`, and `retention.*`.
- [ ] Define typed payload structs and schema versions for first-pass event
  kinds.
- [ ] Define event builders and validation errors. Direct raw event writes
  should be test/migration-only.
- [ ] Define I/O artifact metadata contract.
- [ ] Define retention policy/version and artifact retention-state contract.
- [ ] Define Pumas/Library audit event contract.
- [ ] Define centralized validators for artifact payload references,
  Library/Pumas resource identifiers, external references, and any filesystem
  paths accepted by download/delete/access operations.
- [ ] Define ledger indexes, projection tables, and migration strategy for
  version-aware diagnostics.
- [ ] Define event family ownership: `run.*` owns execution lifecycle,
  `scheduler.*` owns scheduling decisions/control/resource events, and
  projections join families instead of duplicating facts.

**Verification:**

- Schema/contract tests compile.
- Validation tests reject unsupported event kinds, missing required correlation
  fields, unsupported schema versions, disallowed producers, and oversized
  embedded payloads.
- Validation tests reject unsafe artifact references, invalid Library/Pumas
  resource identifiers, and paths that do not resolve through approved
  workspace/cache roots.
- README or ADR updates record ownership decisions.

**Status:** Not started.

### Milestone 2: Ledger Persistence And Projection Rebuild

**Goal:** Persist typed events and rebuild first-pass projections from the
ledger.

**Tasks:**

- [ ] Implement append-only event persistence.
- [ ] Implement projection rebuild for run summary, run detail, scheduler
  timeline, diagnostics summary, I/O artifact gallery, retention state, and
  Library usage where first-pass event families exist.
- [ ] Add workflow execution version and node version fields to projections.
- [ ] Add model/runtime/version and scheduler policy filters where not already
  present.
- [ ] Add retention-completeness filter/projection.
- [ ] Add query outputs that report mixed-version counts or facets.
- [ ] Preserve comparison-ready facets for workflow version, node version,
  model/runtime version, device/network node, scheduler policy, graph settings,
  and input profile where available.

**Verification:**

- Repository tests cover event append, query, and projection rebuild.
- Repository tests cover each new filter.
- Tests cover mixed-version result metadata.
- Persistence, migration, replay, and projection tests each own isolated
  durable resources: SQLite/database files, temporary roots, payload stores,
  and cache paths must not be shared between parallel tests.

**Status:** Not started.

### Milestone 3: I/O Artifact Metadata And Retention

**Goal:** Persist I/O metadata and retention state independently from payload
availability.

**Tasks:**

- [ ] Record workflow inputs/outputs and node input/output metadata.
- [ ] Store artifact type, size, content hash where available, producer node,
  consumer node, run id, timestamps, and payload reference.
- [ ] Emit typed `io.*` events for artifact observation, retention state
  changes, truncation, externalization, expiration, and deletion.
- [ ] Add global retention policy record and policy version.
- [ ] Define first-pass global retention settings for final outputs, workflow
  inputs, intermediate node I/O, failed-run data, maximum artifact size,
  maximum total storage, media behavior, compression/archive behavior, and
  cleanup trigger/status.
- [ ] Add retroactive cleanup command that updates metadata before deleting or
  expiring payloads.
- [ ] Emit typed `retention.*` events with policy version, timestamp, actor,
  affected artifact, and reason.

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
- [ ] Add Library usage projections: used by active run, used by N runs, last
  accessed, total access count, linked workflow/node versions.
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

## Re-Plan Triggers

- Payload storage is not discoverable from run/artifact metadata.
- Pumas APIs do not expose enough information for download/delete/access audit.
- Retention cleanup needs a background worker earlier than expected.
- Version-aware query performance requires a larger storage redesign.
- Event volume requires projection snapshotting, compaction, or partitioning.
- Raw developer event inspection is needed earlier than planned.

## Completion Summary

### Completed

- None. Draft plan only.

### Deviations

- None.

### Follow-Ups

- Decide typed event ledger storage ownership in Milestone 1.
- Decide whether pinning belongs in first implementation or stays future.

### Verification Summary

- Not run. Draft plan only.

### Traceability Links

- Requirement sections: Diagnostics Requirements, I/O Inspector Requirements,
  Retention Policy Requirements, Library Requirements, Pumas Audit
  Requirements.
