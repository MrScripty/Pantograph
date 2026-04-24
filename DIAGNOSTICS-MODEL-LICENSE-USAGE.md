# Pantograph Diagnostics, Model Usage, And License Tracking

## Status

Planning and requirements summary.

This root-level note exists to make the diagnostics and compliance direction
easy to find from the repository root. The detailed source artifacts are:

- `docs/requirements/pantograph-client-sessions-buckets-model-license-diagnostics.md`
- `docs/requirements/pantograph-node-system.md`
- `docs/plans/pantograph-execution-platform/01-client-session-bucket-run-attribution.md`
- `docs/plans/pantograph-execution-platform/03-managed-runtime-observability.md`
- `docs/plans/pantograph-execution-platform/04-model-license-diagnostics-ledger.md`

## Core Direction

Pantograph must treat model usage and license tracking as backend-owned runtime
facts, not as optional workflow nodes, frontend inference, or node-authored
boilerplate.

The normal execution path should be:

```text
client
  -> session
    -> bucket
      -> workflow run
        -> node execution
          -> managed model capability
            -> model/license usage event
```

If a node directly produces model output through Pantograph's managed model
capability, Pantograph records model usage, license data, output measurement,
lineage, and run attribution automatically.

## Durable Attribution Model

Model and license diagnostics depend on durable client/session/bucket/workflow
run identity. These are not optional request fields and should not be recreated
from transient trace state.

Required identity chain:

```text
client
  -> session
    -> bucket
      -> workflow run
        -> model/license usage event
```

Required behavior:

- A client is the durable registered caller identity.
- A session is a persistent live instance opened by a client.
- A client may have at most one active session at a time.
- A session must have exactly one default bucket.
- Buckets are persistent attribution and scheduling groupings, not transient
  request labels.
- Every workflow run belongs to exactly one bucket.
- If a caller does not supply a bucket id, Pantograph assigns the workflow run
  to the session's default bucket.
- Every model/license usage event belongs to exactly one workflow run.
- Every bucket used by a run must belong to the same client/session lineage as
  that run.
- Session and bucket history must remain durable after disconnect so
  diagnostics continuity survives reconnects and process restarts.

The public terminology should distinguish client sessions from workflow runs.
Execution records should use `workflow run`; caller identity should use
`client`, `session`, and `bucket`.

## What Must Be Automatic

Normal node authors should not manually implement:

- diagnostics span creation
- run, session, bucket, or client attribution
- model usage event creation
- Pumas license lookup or snapshotting
- output measurement for standard model outputs
- lineage attachment for ordinary managed execution
- compliance-grade diagnostics event emission

Those are runtime responsibilities.

Node authors may add optional diagnostics annotations, progress details, or
node-specific summaries, but those additions must enrich the baseline rather
than replace it.

## Runtime-Owned Diagnostics

Every standard node execution should automatically produce backend-owned
diagnostics for:

- execution start
- execution completion
- execution failure
- cancellation or timeout
- input summary
- output summary
- effective contract used for execution
- validation or compatibility failure
- execution guarantee level
- composed-node parent context when relevant

These diagnostics must use stable workflow, node, port, and run identifiers so
GUI, native Rust, C#, Python, and Elixir/BEAM consumers can correlate events
without guessing.

## Durable Model And License Usage Records

Pantograph must persist durable usage records for direct model-output-producing
execution.

Each durable model/license usage record should retain:

- usage event id
- client id
- session id
- bucket id
- workflow run id
- workflow id
- initiating graph node id
- initiating node type
- composed-node parent context when relevant
- model id
- model revision, hash, or resolved version when available
- model type or modality
- backend/runtime that executed the model when available
- Pumas license value at time of use
- Pumas license source metadata
- model metadata snapshot needed to explain the license later
- direct output measurement
- execution guarantee level
- lineage metadata
- timestamps
- failure or partial-output status when relevant

License values must be snapshotted at time of use. Later changes to Pumas model
metadata must not rewrite historical usage records.

## Output Measurement

Pantograph should measure direct model output by modality:

- text: characters, bytes, and token count when tokenizer facts are available
- image: image count, dimensions, pixel count, and encoded bytes when available
- audio: item count, duration, sample rate, channels, and encoded bytes when
  available
- video: item count, duration, frame count, dimensions, and encoded bytes when
  available
- embeddings: vector count, dimensions, representation, and byte size when
  available
- structured output: byte size, top-level shape, and schema id or schema digest
  when available

Missing measurements must be explicit. The system should record why a metric is
unavailable instead of silently storing a misleading zero.

## Diagnostics Queries

The diagnostics API should support:

- model usage over time
- license usage over time
- filtering by client
- filtering by session
- filtering by bucket
- filtering by workflow run
- filtering by workflow
- filtering by graph node
- filtering by model
- filtering by license
- per-run detail for multi-model workflows
- reduced-guarantee or escape-hatch filtering

The GUI should render backend-owned projections from the same diagnostics data
that native and host-language consumers can query.

## Guarantee Levels

Diagnostics and usage records must show whether the runtime had full
observability:

- `managed_full`: managed runtime path with complete required attribution and
  measurement facts
- `managed_partial`: managed runtime path with explicit unavailable measurement
  fields
- `escape_hatch_detected`: runtime-mediated escape hatch reduced guarantees
- `unsafe_or_unobserved`: required observability was bypassed or could not be
  proven

Reduced-guarantee records must never be presented as complete compliance data.

## Non-Negotiable Invariants

- Model/license diagnostics must not require explicit observability nodes.
- Model/license diagnostics must not be an ordinary optional toggle.
- The runtime, not individual node code, owns baseline diagnostics.
- Pumas is the source for model license metadata.
- License data is snapshotted at time of use.
- Direct model output measurement is based on runtime facts, not static graph
  guesses.
- Bindings expose projections of backend-owned diagnostics; they do not
  reconstruct usage from host-local node catalogs or trace text.

## Relationship To The Node System

This tracking model depends on the stronger node-runtime design:

- nodes execute through a managed runtime boundary
- model calls go through managed capabilities
- the runtime injects attribution and lineage context
- escape hatches are detectable and classified
- composed nodes remain traceable to primitive model-producing behavior

The practical goal is simple: a useful node should be easy to write, while
Pantograph still produces durable, queryable, compliance-grade model and license
usage history automatically.
