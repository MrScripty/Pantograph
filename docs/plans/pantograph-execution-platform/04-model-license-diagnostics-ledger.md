# 04: Model License Diagnostics Ledger

## Purpose

Make direct model execution observable and persistently attributable without
explicit diagnostics nodes.

## Implementation Readiness Status

Ready for stage-start preflight after stages `01`, `02`, and `03` are complete
and their stage-end refactor gates have been recorded.

## Implementation Notes

### 2026-04-24 Stage-Start Report

- Selected stage: Stage `04`, model license diagnostics ledger.
- Current branch: `main`.
- Stage base: `915e2c0a`, the Stage `03` closeout commit.
- Prior-stage gates: Stage `01`, Stage `02`, and Stage `03` coordination
  ledgers record completed wave integration and stage-end refactor gate
  outcomes of `not_warranted`.
- Git status before implementation: unrelated asset changes only:
  deleted `assets/3c842e69-080c-43ad-a9f0-14136e18761f.jpg`, deleted
  `assets/grok-image-6c435c73-11b8-4dcf-a8b2-f2735cc0c5d3.png`, deleted
  `assets/grok-image-e5979483-32c2-4cf5-b32f-53be66170132.png`,
  untracked `assets/banner_3.jpg`, `assets/banner_3.png`,
  `assets/github_social.jpg`, and `assets/reject/`.
- Dirty-file overlap: none. Stage `04` implementation must not touch
  `assets/`.
- Standards reviewed through the execution-platform standards map:
  `PLAN-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
  `CODING-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`,
  `TESTING-STANDARDS.md`, `CONCURRENCY-STANDARDS.md`,
  `TOOLING-STANDARDS.md`, `DEPENDENCY-STANDARDS.md`,
  `SECURITY-STANDARDS.md`, `RELEASE-STANDARDS.md`,
  `COMMIT-STANDARDS.md`, `languages/rust/RUST-DEPENDENCY-STANDARDS.md`,
  `languages/rust/RUST-API-STANDARDS.md`, and
  `languages/rust/RUST-TOOLING-STANDARDS.md`.
- Intended Wave `02` write sets:
  - `ledger-storage-retention`: `crates/pantograph-diagnostics-ledger/`.
  - `runtime-ledger-submission`: `crates/pantograph-embedded-runtime/`
    ledger submission boundaries.
  - `workflow-service-query-projections`:
    `crates/pantograph-workflow-service/` diagnostics query use cases.
- Host-owned shared files: workspace manifests, lockfiles, public facade
  exports, ADR files, and any integration documentation.
- Forbidden write set for this stage unless the plan is updated first:
  GUI diagnostics views, host binding projections, and node factoring or
  migration logic.
- Start outcome: `ready_with_recorded_assumptions`.
- Recorded assumptions:
  - Wave `02` may be executed serially by the host in this shared workspace
    when subagents are not explicitly authorized; the recorded worker write
    sets and reports still apply.
  - The first implementation step after Wave `01` is
    `ledger-storage-retention`: create `pantograph-diagnostics-ledger` with
    canonical event types, SQLite schema/migrations, retention/pruning,
    bounded query DTOs, tests, and README coverage.
  - GUI diagnostics/history views remain deferred. Stage `04` may define
    backend query projections that GUI can consume, but must not implement GUI
    files or host binding projections in this stage.

### 2026-04-24 Wave 01 Ledger Schema And Retention Freeze

- SQLite dependency decision: use `rusqlite` `0.32.1` with the existing
  `bundled` feature already present in `pantograph-runtime-attribution`.
  Stage `04` does not introduce a second SQLite crate or a different SQLite
  feature set.
- Workspace ownership decision: because `rusqlite` will be directly used by
  both `pantograph-runtime-attribution` and
  `pantograph-diagnostics-ledger`, implementation should move the shared
  version/features to `[workspace.dependencies]` and have both crates declare
  `rusqlite = { workspace = true }`.
- Dependency review commands run:
  `cargo tree -i rusqlite`,
  `cargo tree -p pantograph-runtime-attribution --depth 1`,
  `cargo tree -p rusqlite --depth 1`, and
  `cargo tree -p pantograph-runtime-attribution --prefix none --no-dedupe | sort -u | wc -l`.
- Dependency review results:
  `rusqlite` is already in the tree via `pantograph-runtime-attribution` and
  `pumas-library`; direct `rusqlite` dependencies are `bitflags`,
  `fallible-iterator`, `fallible-streaming-iterator`, `hashlink`,
  `libsqlite3-sys`, and `smallvec`; the attribution crate reports 48 unique
  dependency lines under the inspected tree, below the 100+ written-justification
  threshold.
- Linking/release decision: keep `bundled` SQLite for deterministic local
  builds and to match existing attribution persistence. Release artifact impact
  is unchanged in kind from Stage `01`: native artifacts carry bundled SQLite
  through `libsqlite3-sys` rather than depending on host SQLite availability.
- Audit decision: no new storage-family dependency is added in Wave `01`; the
  implementation still must keep `Cargo.lock` updated and rerun workspace
  check, clippy, and tests after manifest changes.
- Migration strategy: `pantograph-diagnostics-ledger` owns a
  `ledger_schema_migrations` table with monotonically increasing integer
  versions, applied timestamp in milliseconds, and a schema checksum string for
  the migration text or equivalent embedded migration identity.
- Unsupported schema versions must fail open attempts with a typed ledger
  error rather than silently applying unknown migrations or truncating data.
- Version `1` schema freeze:
  - `model_license_usage_events`: usage event id, attribution ids, workflow id,
    node id/type, model id/version/hash/modality, runtime/backend id, guarantee
    level, status, started/completed timestamps, schema version, retention
    class, and correlation ids.
  - `license_snapshots`: usage event id, Pumas license value, source metadata,
    model metadata snapshot JSON, and unavailable reason.
  - `model_output_measurements`: usage event id, modality, typed count/size/
    duration/dimension/token fields, unavailable fields, and unavailable
    reasons.
  - `usage_lineage`: usage event id, node id/type, port ids, composed-parent
    chain JSON, effective contract version or digest, and lineage metadata.
  - `diagnostics_retention_policy`: policy id/version, retention class,
    retention duration in days, applied timestamp, and explanation.
- Required indexes remain the plan-level indexes for time range, model,
  license, client, session, bucket, workflow, workflow run, node, guarantee
  level, per-run drilldown, and pruning lookup by timestamp and retention
  class.
- Retention default: local usage events use a `standard` retention class with
  a default retention duration of 365 days. This avoids unbounded local growth
  while preserving enough history for compliance and diagnostics review.
- Retention re-plan trigger: if product policy requires a different retention
  period, legal hold, export-before-prune, or per-client retention, update this
  plan and ledger schema policy before changing implementation behavior.
- Pruning command semantics: pruning is explicit and command-shaped, accepts a
  validated cutoff or retention policy id, deletes only complete eligible usage
  events plus associated license snapshot, measurement, and lineage rows in one
  transaction, and returns counts plus retention metadata.
- Pruning must not rewrite retained license snapshots, output measurements,
  lineage, attribution ids, or guarantee values.
- Query bounds: query inputs use validated optional filters, inclusive start
  and exclusive end timestamps, stable sort fields, and explicit pagination.
  Default page size is 100, maximum page size is 500, and time-series bucket
  count must be bounded to at most 366 buckets per query.
- Unavailable measurement reasons are frozen as typed values:
  `NotProduced`, `UnsupportedModality`, `TokenizerUnavailable`,
  `MetadataUnavailable`, `RuntimeDidNotReport`, `OutputTruncated`,
  `OutputRedacted`, `ExecutionFailedBeforeMeasurement`, and `Unknown`.
- Unavailable license reasons are frozen as typed values:
  `PumasMetadataUnavailable`, `LicenseFieldMissing`, `ModelNotInLibrary`,
  `RuntimeOnlyModel`, `LookupFailed`, and `Unknown`.
- Wave `02` non-overlap decision: `ledger-storage-retention` owns canonical
  ledger DTOs and persistence; `runtime-ledger-submission` may consume the
  ledger trait and DTOs but does not define storage; `workflow-service-query-
  projections` may delegate to the ledger but does not own ledger persistence
  semantics.
- Wave `01` outcome: complete. Source implementation may begin with the
  `ledger-storage-retention` slice.

## Diagnostics Products

Diagnostics has two related products:

- runtime trace diagnostics for explaining execution behavior
- durable model/license usage ledger records for compliance, cost, and history

Runtime traces answer: "What happened during this run?" Durable usage records
answer: "Which client/session/bucket/workflow/node used which model and license
over time, and how much direct output did it produce?"

## Type Families To Define

- `NodeExecutionStarted`
- `NodeExecutionCompleted`
- `NodeExecutionFailed`
- `NodeExecutionSkipped`
- `NodeExecutionGuaranteeChanged`
- `NodeInputSummary`
- `NodeOutputSummary`
- `NodeDiagnosticsAnnotation`
- `NodeProgressEvent`
- `NodeCancellationObserved`
- `ModelLicenseUsageEvent`
- `ModelOutputMeasurement`
- `ModelUsageAttribution`
- `LicenseSnapshot`
- `UsageLineage`
- `DiagnosticsRetentionPolicy`
- `DiagnosticsQuery`
- `DiagnosticsProjection`
- `UsageSummaryProjection`
- `UsageTimeSeriesProjection`
- `WorkflowRunUsageDetail`

## Durable Model And License Usage Event

Each `ModelLicenseUsageEvent` must retain:

- `usage_event_id`
- `client_id`
- `client_session_id`
- `bucket_id`
- `workflow_run_id`
- `workflow_id`
- initiating `node_id`
- initiating `node_type`
- optional composed-node parent chain
- model id
- model revision, hash, or resolved version when available
- model type or modality
- backend/runtime that executed the model when available
- Pumas license value at time of use
- Pumas license source metadata
- model metadata snapshot needed to explain the license later
- direct output measurement
- execution guarantee level
- lineage metadata sufficient to locate the usage in the graph
- started and completed timestamps where available
- failure or partial-output status when usage occurred but did not complete

License data must be snapshotted at time of use. Later changes to Pumas model
metadata must not rewrite the historical license value attached to an existing
usage event.

## Output Measurement Rules

Output measurement must be explicit and typed by modality:

- text: characters, bytes, token count when tokenizer facts are available
- image: image count, dimensions, pixel count, encoded byte size when available
- audio: item count, duration, sample rate, channels, encoded byte size when
  available
- video: item count, duration, frame count, dimensions, encoded byte size when
  available
- embeddings: vector count, dimensions, numeric representation, byte size when
  available
- structured output: byte size, top-level shape, schema id or schema digest
  when available

Measurements represent direct model output, not downstream transformed content.
When a measurement is unavailable, the record must say which measurement fields
are unavailable and why instead of silently storing zeros.

## Diagnostics Query Projections

The diagnostics query surface must support:

- clients with diagnostics-visible metadata
- sessions by client
- session lifecycle history by session
- buckets by client
- default bucket assignment history
- workflow runs by client, session, bucket, and workflow
- model usage over time
- license usage over time
- usage grouped by client
- usage grouped by session
- usage grouped by bucket
- usage grouped by workflow
- usage grouped by workflow run
- usage grouped by model
- usage grouped by license
- per-run detail for workflows that use multiple models
- graph-node drilldown for the node that initiated each usage event
- reduced-guarantee or escape-hatch filtering

Query projections must preserve stable ids so GUI, native Rust, C#, Python, and
Elixir/BEAM consumers can correlate summaries back to durable records and graph
diagnostics.

## GUI Diagnostics And Attribution History

The GUI must be able to inspect attribution and diagnostics history through
backend-owned query projections. It must not reconstruct this history from
local state, logs, host-local catalogs, or optimistic client-side assumptions.

Required GUI-visible views:

- client diagnostics summary: `client_id`, display metadata safe for UI,
  active/latest session state, bucket count, run count, and last activity.
- client session history: sessions for a client with current lifecycle state,
  opened timestamp, latest transition timestamp, and takeover/expiry/close
  reason where available.
- session lifecycle timeline: ordered `SessionLifecycleRecord` entries for a
  session, including `Opening`, `Connected`, `DisconnectedGrace`, `Expired`,
  `TakenOver`, and `Closed` transitions.
- bucket list: buckets for a client, including immutable name, default-bucket
  marker, deletion state, creation timestamp, deletion timestamp, and deletion
  reason where available.
- default bucket assignment history: session-to-default-bucket assignments.
- workflow-run attribution list: runs filtered by client, session, bucket,
  workflow, status, and time range.
- usage ledger drilldown: model/license usage events filtered by client,
  session, bucket, workflow run, workflow, node, model, license, time range,
  and execution guarantee level.
- event-to-context drilldown: usage event -> workflow run -> bucket -> session
  lifecycle -> client summary.

Allowed GUI actions:

- request creation of a non-default bucket through the backend API.
- request deletion of a non-default bucket through the backend API.
- select filters, time ranges, grouping, sort order, and pagination.
- open drilldowns between usage events, runs, buckets, sessions, and clients.

Forbidden GUI behavior:

- renaming buckets.
- deleting the default bucket.
- locally mutating bucket/session/run state before backend confirmation.
- treating missing local cache entries as proof that a client, session, bucket,
  or run does not exist.
- showing reduced-guarantee records as complete compliance data.
- displaying credential material or raw client secrets.

Frontend and accessibility requirements:

- lists and tables use semantic table/list controls where practical.
- timelines are keyboard navigable and expose transition names, timestamps, and
  reasons to assistive technology.
- filter controls have labels and deterministic keyboard operation.
- destructive bucket deletion uses a semantic button, explicit confirmation,
  backend-confirmed completion, and accessible error messages.
- loading, empty, partial-data, reduced-guarantee, and error states are
  explicitly represented.
- tests use accessible queries for controls and include keyboard interaction for
  filters, drilldowns, and deletion confirmation.

## Storage Boundary

The implementation plan must define which diagnostics are retained only in
memory and which are persisted.

Default direction:

- transient trace stream: live run inspection and recent diagnostics
- persisted ledger: client/session/bucket/run/model/license usage history
- persisted run index: finding usage records by workflow run and graph node

Retention behavior, pruning policy, and migration rules must be defined before
this feature is considered complete.

## Implementation Decisions

### Ledger Ownership

- Stage `04` creates `crates/pantograph-diagnostics-ledger` as the canonical
  backend owner for durable model/license usage records, output measurements,
  license snapshots, query DTOs, retention policy, and persisted indexes.
- `crates/pantograph-embedded-runtime` owns managed model capability
  interception and submits validated usage events to the ledger through a
  narrow trait.
- `crates/pantograph-workflow-service` exposes application-level diagnostics
  query use cases by delegating to the ledger. It does not own ledger storage
  semantics.
- GUI and binding layers consume projections only and cannot reinterpret
  license values, output measurements, or reduced-guarantee semantics.

### Persistence Decision

- The first ledger implementation uses SQLite as the durable local storage
  engine.
- The SQLite schema stores model/license usage events, license snapshots,
  typed output measurements, lineage fields, guarantee classification, and
  query indexes required for time-series, grouping, and per-run drilldown.
- Every persisted event includes a schema version or migration version and
  stable ids needed for replay, migration, and correlation with workflow runs
  and graph nodes.
- Inserting one usage event and its associated measurement/snapshot rows is the
  durable transaction boundary.
- Retention and pruning operate on complete events. Pruning must never rewrite
  license snapshots or output measurement values for retained events.
- SQLite migration files or equivalent schema migration records are required
  before the ledger is considered complete.
- The SQLite dependency, feature selection, bundled/native-linking behavior,
  audit impact, and release artifact impact must be recorded before
  implementation.
- The stage-start report must inspect SQLite dependency cost using the Rust
  dependency standards before adding or changing SQLite crates/features.

### Ledger SQLite Schema Decision

The first schema must include, at minimum:

- `ledger_schema_migrations`: migration id, applied timestamp, checksum or
  equivalent integrity marker.
- `model_license_usage_events`: `usage_event_id`, client/session/bucket/run/
  workflow/node/model identifiers, guarantee level, status, timestamps, and
  correlation ids.
- `license_snapshots`: `usage_event_id`, Pumas license value, source metadata,
  model metadata snapshot, unavailable reason when license facts are missing.
- `model_output_measurements`: `usage_event_id`, modality, typed measurement
  fields, unavailable measurement fields, unavailable reasons.
- `usage_lineage`: `usage_event_id`, node id, node type, port ids where
  relevant, composed-parent chain, effective contract version or digest.

Required indexes:

- time range plus model, license, client, session, bucket, workflow, workflow
  run, node, and guarantee level
- per-run drilldown by `workflow_run_id` and graph node id
- retention/pruning lookup by timestamp and retention class

### License Snapshot Decision

- The ledger stores the Pumas license value and source metadata exactly as
  resolved at time of use.
- Later Pumas metadata changes do not mutate existing ledger events.
- Missing or unavailable license metadata is represented as a typed unavailable
  reason, not as an empty license string.

### Output Measurement Decision

- Output measurements are modality-specific enums with explicit unavailable
  field reasons.
- Direct model output is measured before downstream graph transformations.
- A zero value is valid only when the measured output is actually zero. Missing
  measurement data must use an unavailable reason.

### Query Boundary Decision

- Query inputs are bounded by typed time ranges, optional grouping filters, and
  explicit pagination or limit fields where result size can grow.
- Query projections preserve stable event ids, workflow-run ids, graph node
  ids, model ids, license values, and guarantee levels so consumers can drill
  down without recomputing ledger semantics.

### Retention And Pruning Decision

- `pantograph-diagnostics-ledger` owns retention policy evaluation and pruning
  commands.
- The first implementation must provide an explicit default retention policy
  rather than leaving retention unbounded. If product policy is not finalized at
  stage start, the stage-start report must record a conservative local default,
  the rationale, and the re-plan trigger for changing it.
- Retention policy is stored with a schema version so future policy changes can
  be explained and migrated.
- Pruning is command-shaped and operates on complete usage events plus their
  measurement, license snapshot, and lineage rows in one transaction.
- Pruning may delete eligible events, but it must not rewrite retained license
  snapshots, output measurements, lineage, or attribution values.
- Query APIs must expose enough retention metadata for consumers to distinguish
  "no matching usage" from "usage may have been pruned by policy."

## Affected Structured Contracts And Persisted Artifacts

- `ModelLicenseUsageEvent` records, license snapshots, output measurements,
  usage lineage, diagnostics query DTOs, client/session/bucket attribution
  history projections, summary projections, time-series projections,
  run-detail projections, retention policy, and persisted indexes.

## Standards Compliance Notes

- Architecture compliance requires durable ledger facts to be backend-owned and
  independent from transient trace storage.
- Rust API compliance requires typed output measurements by modality,
  unavailable-measurement reasons, non-stringly license/model identifiers where
  bug cost justifies it, and typed query errors.
- Security and privacy compliance require explicit retention/pruning behavior,
  bounded query ranges, controlled exposure of model metadata, and no secret
  credential material in usage records.
- Dependency compliance requires any tokenizer, media-inspection, storage, or
  analytics dependency to be justified by owner, feature gate, transitive cost,
  and audit impact.
- Testing compliance requires time-of-use license snapshot tests, historical
  stability tests, multi-model run tests, unavailable-measurement tests,
  reduced-guarantee filtering tests, and replay/recovery tests for persisted
  records.

## Risks And Mitigations

- Risk: missing measurements are stored as zeros and misread as real output.
  Mitigation: require typed unavailable reasons.
- Risk: license history changes when Pumas metadata changes. Mitigation:
  snapshot license facts at time of use and test historical stability.
- Risk: diagnostics storage grows without policy. Mitigation: define retention,
  pruning, and migration before marking the ledger complete.

## Tasks

- Add `crates/pantograph-diagnostics-ledger` with README coverage required for
  a new source crate.
- Define managed model execution capability.
- Define the usage attribution context passed into every managed model call.
- Require managed model calls to receive resolved client/session/bucket/workflow
  run attribution from the runtime, not from node-authored arguments.
- Integrate Pumas license snapshot lookup.
- Define exact `ModelLicenseUsageEvent` fields and persistence ownership.
- Define direct output measurement rules by modality.
- Define unavailable-measurement reasons so missing metrics are explicit.
- Classify execution guarantee levels for full, partial, escape-hatch, and
  unsafe/unobserved paths.
- Persist model/license usage events.
- Define query projections for summaries, time series, and per-run details.
- Define retention, pruning, and migration expectations for the persisted
  ledger.
- Implement command-shaped pruning over complete events and tests for retained
  snapshot immutability after pruning.
- Expose typed query projections.
- Expose GUI-consumable attribution history projections for clients, sessions,
  session lifecycle records, buckets, default bucket assignments, workflow
  runs, and usage-event drilldowns.

## Intended Write Set

- Primary:
  - `crates/pantograph-diagnostics-ledger/`
  - workspace manifests for the new crate
- Adjacent only if required by integration:
  - `crates/pantograph-embedded-runtime/`
  - `crates/pantograph-workflow-service/`
  - `crates/pantograph-runtime-attribution/`
- Forbidden for this stage unless the plan is updated first:
  - GUI diagnostics views
  - host binding projections
  - node factoring or migration logic

## Existing Code Impact

- No existing workspace crate currently owns SQLite diagnostics ledger
  persistence. Stage `04` must add a dedicated crate instead of placing ledger
  storage in workflow service, embedded runtime, bindings, or GUI adapters.
- `crates/pantograph-embedded-runtime/src/task_executor/` and
  `crates/workflow-nodes/src/processing/` contain direct model execution paths
  and Pumas/library integration points. Stage `04` must route model-producing
  calls through managed capabilities or explicit ledger submission boundaries
  so ordinary nodes do not hand-author compliance records.
- `crates/pantograph-embedded-runtime/src/workflow_runtime.rs` already exposes
  transient runtime diagnostics snapshots. Stage `04` must keep the durable
  SQLite ledger separate from that transient snapshot path while preserving
  correlation ids.
- `crates/pantograph-workflow-service/src/trace/` currently owns trace query
  and store concepts. Stage `04` must either keep those trace-only or adapt
  them as projections over `pantograph-diagnostics-ledger`; they must not own
  model/license ledger persistence semantics.

## Verification Commands

Expected stage verification:

```bash
cargo test -p pantograph-diagnostics-ledger
cargo test -p pantograph-embedded-runtime
cargo check --workspace --all-features
```

If workflow-service query integration is touched, also run:

```bash
cargo test -p pantograph-workflow-service
```

Stage completion also requires the Rust baseline verification from
`RUST-TOOLING-STANDARDS.md` unless the stage-start report records an existing
repo-owned equivalent:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Verification

- Model-producing nodes using the managed capability create durable usage
  records automatically.
- License snapshots are time-of-use facts.
- License values remain historically stable if Pumas metadata later changes.
- Multi-model workflow runs produce multiple attributable usage records.
- Unavailable measurements are represented explicitly rather than as misleading
  zero values.
- Reduced-guarantee records are visible and filterable in diagnostics queries.
- Diagnostics queries can filter by client, session, bucket, run, workflow,
  model, license, and time range.
- GUI-facing projections can show client/session/bucket/run history without
  exposing credential material or requiring local reconstruction.
- Frontend tests for any implemented GUI diagnostics/history surface cover
  semantic controls, keyboard drilldown, backend-confirmed mutation behavior,
  reduced-guarantee display, and accessible error states.
- Ledger records survive process restart, replay, and migration according to
  the selected persistence model.
- SQLite persistence tests cover transaction atomicity, restart recovery,
  migration application, unsupported schema version rejection, retention
  pruning, and indexed query correctness.

## Completion Criteria

- Managed model execution automatically records durable model/license usage
  events.
- Durable usage records preserve client/session/bucket/workflow-run/node/model
  attribution, Pumas license snapshots, typed output measurements, timestamps,
  lineage, and execution guarantee classification.
- Diagnostics query projections support model/license time series, grouped
  summaries, per-run multi-model detail, and graph-node drilldown.
- GUI diagnostics/history projections support client, session lifecycle,
  bucket, default bucket assignment, workflow-run, and usage-event drilldown
  views.
- The stage-start implementation gate in
  `08-stage-start-implementation-gate.md` is recorded before source edits.
- The stage-end refactor gate in `09-stage-end-refactor-gate.md` is completed
  or explicitly recorded as not warranted for this stage.

## Re-Plan Triggers

- Direct model output measurement requires per-node boilerplate instead of
  managed capability interception.
- The selected storage engine cannot support retention, pruning, or indexed
  query requirements.
- SQLite linking, migration, or release-artifact requirements conflict with the
  supported platform or dependency standards.
- Pumas license metadata cannot provide stable time-of-use snapshots.
