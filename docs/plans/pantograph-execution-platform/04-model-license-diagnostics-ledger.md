# 04: Model License Diagnostics Ledger

## Purpose

Make direct model execution observable and persistently attributable without
explicit diagnostics nodes.

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

## Storage Boundary

The implementation plan must define which diagnostics are retained only in
memory and which are persisted.

Default direction:

- transient trace stream: live run inspection and recent diagnostics
- persisted ledger: client/session/bucket/run/model/license usage history
- persisted run index: finding usage records by workflow run and graph node

Retention behavior, pruning policy, and migration rules must be defined before
this feature is considered complete.

## Affected Structured Contracts And Persisted Artifacts

- `ModelLicenseUsageEvent` records, license snapshots, output measurements,
  usage lineage, diagnostics query DTOs, summary projections, time-series
  projections, run-detail projections, retention policy, and persisted indexes.

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
- Expose typed query projections.

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
- Ledger records survive process restart, replay, and migration according to
  the selected persistence model.

## Completion Criteria

- Managed model execution automatically records durable model/license usage
  events.
- Durable usage records preserve client/session/bucket/workflow-run/node/model
  attribution, Pumas license snapshots, typed output measurements, timestamps,
  lineage, and execution guarantee classification.
- Diagnostics query projections support model/license time series, grouped
  summaries, per-run multi-model detail, and graph-node drilldown.
- The stage-start implementation gate in
  `08-stage-start-implementation-gate.md` is recorded before source edits.
- The stage-end refactor gate in `09-stage-end-refactor-gate.md` is completed
  or explicitly recorded as not warranted for this stage.

## Re-Plan Triggers

- Direct model output measurement requires per-node boilerplate instead of
  managed capability interception.
- The selected storage engine cannot support retention, pruning, or indexed
  query requirements.
- Pumas license metadata cannot provide stable time-of-use snapshots.
