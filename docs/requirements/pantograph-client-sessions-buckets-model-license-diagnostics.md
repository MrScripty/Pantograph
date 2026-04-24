# Pantograph Requirements: Client Sessions, Buckets, and Model License Diagnostics

## Status

Draft requirements note. This is not a full implementation plan.

## Purpose

Capture the agreed high-level requirements for:

- persistent client identity
- persistent client sessions
- persistent buckets
- workflow run attribution
- persistent model/license usage diagnostics
- diagnostics GUI support for model and license usage over time

This document exists so a later implementation plan can be written against a
stable set of feature expectations, terminology, and invariants.

## Scope

This note covers the required feature shape and architectural direction.

This note does not define:

- milestone ordering
- commit sequencing
- migration mechanics in detail
- exact database schema or SQL migration scripts
- exact API payload names for every transport surface

## Core Problem

Pantograph needs backend-owned, persistent diagnostics for legally important and
potentially financially important model usage. Workflow runs may use more than
one model. Pantograph must be able to record, retain, query, and display:

- what model was used
- what license applied to that model at time of use
- how much direct output that model produced
- what workflow node initiated that model use
- what workflow run produced the usage
- what bucket and session the run belonged to
- what client owned the session
- how model and license usage changed over time

This feature must not require an explicit workflow node and must not be
disableable in ordinary operation.

## Terminology

### Client

A durable registered caller identity. A client is the long-lived identity of an
application or integration that uses Pantograph.

### Session

A persistent, live instance opened by a client. A session is not the client
identity itself; it contains the client identity and represents the current
active connection/instance through which the client uses Pantograph.

Only one active session may exist per client at a time.

### Bucket

A persistent attribution and scheduling grouping associated with a client
session. A session always has a default bucket. Additional buckets may be used
to distinguish traffic lanes such as agents, projects, workload classes, or
other caller-defined groupings.

### Workflow Run

A single submitted workflow execution. All workflow runs must attach to exactly
one bucket. If the caller does not provide a bucket id, Pantograph must assign
the run to the session's default bucket automatically.

### Model License Usage Event

A persistent backend-owned record describing one direct model-output-producing
execution together with the model identity, license snapshot, output metrics,
and workflow/run attribution context.

## Required Identity Model

Pantograph must introduce durable records for:

- clients
- sessions
- buckets
- workflow runs

Required identity chain:

`client -> session -> bucket -> workflow run -> model license usage events`

Required behavior:

- a client must register itself before opening a session
- a client may have at most one active session at a time
- different clients may have concurrent sessions
- if client `C` already has an active session, another session must reject an
  attempt to attach as client `C` until the original session is closed or no
  longer considered active
- sessions must be persistent across disconnect/reconnect cycles for
  diagnostics continuity
- buckets must be persistent first-class records, not transient request fields
- each session must have exactly one default bucket
- every workflow run must belong to exactly one bucket

## Client Verification and Session Continuity

Pantograph must support a lightweight, cryptographically secure way to verify
that a session is being opened by the same client identity without imposing
unnecessary operational overhead.

High-level requirements:

- new callers must be able to register as new clients
- existing callers must be able to prove they are the same client identity
- client verification must be sufficient to preserve diagnostics continuity over
  time
- the feature is primarily about accurate and durable attribution, not about
  redesigning all runtime transport security

The later implementation plan must define:

- client registration flow
- client credential issuance and storage approach
- session open/resume flow
- session liveness and expiry rules
- what counts as a disconnected or expired session for takeover purposes

## Public Naming Direction

The repo currently uses "workflow session" terminology for an existing runtime
/ keep-alive concept. This feature introduces client sessions as a first-class
concept, so the naming must be refactored to avoid ambiguity.

Required direction:

- public-facing execution terminology should use `workflow run`
- client identity terminology should use `client` and `session`
- the existing internal workflow-session concept should be renamed, privatized,
  or otherwise decoupled from public client session terminology

This refactor is required before or alongside implementation so diagnostics,
storage, and API contracts do not become ambiguous.

## Model and License Diagnostics Requirements

Pantograph must persistently record direct model usage for workflow runs.

For each model usage record, Pantograph must be able to retain:

- client identity
- session identity
- bucket identity
- workflow run identity
- workflow identity
- graph node identity that initiated the model execution
- model identity
- model revision/hash when available
- model type when available
- model license value from Pumas
- license source metadata indicating the license came from Pumas
- license/model metadata snapshot at time of use
- direct output metrics for the model execution
- timestamps needed for querying and graphing over time
- lineage metadata sufficient to show where in the workflow graph the usage
  occurred

## Source of Truth for Model and License Data

License data must come from Pumas model metadata and be snapshotted at time of
use.

Output metrics should come from the inference/runtime execution path where
Pantograph can observe actual model output behavior.

Workflow attribution context must come from the workflow runtime / node engine,
not from the graph definition alone.

The later implementation plan should preserve this split:

- inference/runtime path produces direct output stats
- workflow/runtime layer adds node/run/bucket/session/client context
- backend diagnostics store persists the enriched record

## Output Measurement Requirements

Pantograph must track the amount of output produced by each model per workflow
run.

The later implementation plan must define typed measurement rules for at least:

- text
- image
- audio
- video
- embeddings
- structured/JSON outputs

The requirements here are:

- measurement must be based on direct model output usage, not a vague estimate
- the stored metrics must be queryable over time
- the metrics must be usable in diagnostics graphs and per-run detail views
- the system must support workflows using `n` models in a single run

## Lineage and Graph Attribution

This feature will use direct model output usage with lineage metadata, not
attempt full exact downstream content attribution in the first pass.

Pantograph must be able to identify where in the workflow graph the model usage
occurred.

Minimum requirements:

- identify the node that initiated the model execution
- retain enough context to locate that node in diagnostics views
- support runs that contain multiple model-using nodes
- preserve additive lineage metadata for later expansion without overclaiming
  final-output ownership

## Persistence Requirements

This feature requires a persistent backend-owned database. SQLite is an
acceptable intended direction for later planning, but the exact database choice
may still be finalized later.

Persistence requirements:

- records must survive process restarts and disconnects
- diagnostics history must remain queryable over time
- session and bucket history must remain durable after disconnect
- model/license usage history must remain durable after run completion
- the system must support later graphing, filtering, and reporting without
  relying only on in-memory trace state

This is broader than the current retained in-memory diagnostics projection.

## Diagnostics GUI Requirements

Pantograph must add a diagnostics GUI surface for model and license usage.

Required capabilities:

- graph model usage over time
- graph license usage over time
- filter by client
- filter by session
- filter by bucket
- filter by workflow run
- filter by workflow
- filter by model
- filter by license
- show per-run detail for multi-model workflow runs
- show what node in the workflow graph initiated each recorded model use

The GUI should remain a renderer over backend-owned diagnostics projections, in
the same general spirit as current diagnostics surfaces.

## Diagnostics API and Binding Requirements

Persistent usage diagnostics must not be GUI-only. Native Rust consumers and
supported host-language bindings need backend-owned query surfaces over the same
stored diagnostics facts.

Required direction:

- the native Rust API must expose typed query operations for client, session,
  bucket, workflow run, model, license, and time-range diagnostics
- C#, Python, and Elixir bindings must expose supported projections of those
  diagnostics queries when their support tier includes workflow execution
- host bindings must not reconstruct model/license usage by parsing transient
  trace text or static workflow graphs
- diagnostics query responses must preserve stable ids needed to correlate
  usage records back to clients, sessions, buckets, workflow runs, nodes, and
  ports
- unsupported or experimental host-language diagnostics gaps must be documented
  as support-tier limitations
- GUI diagnostics views must consume the same backend-owned projections that
  external API and binding consumers can rely on, unless a GUI-only projection
  is explicitly documented as presentation-only

## Required Invariants

- A client may have at most one active session at a time.
- A session must belong to exactly one client.
- Every active session must have exactly one default bucket.
- Every bucket used by a run must belong to the same client/session lineage as
  that run.
- Every workflow run must belong to exactly one bucket.
- If no bucket id is supplied for a run, Pantograph must route the run to the
  session's default bucket.
- Every model/license usage record must belong to exactly one workflow run.
- Every usage record must be attributable to a graph node that initiated the
  model execution.
- License information must be preserved as time-of-use diagnostics data, not
  reconstructed lazily later from mutable current metadata.
- This diagnostics capability must not require an explicit workflow node.
- This diagnostics capability must not be an ordinary optional toggle.

## Relationship to Existing Diagnostics

The current diagnostics system already demonstrates backend-owned observability
for node/run/runtime/scheduler events. This feature should extend that model
rather than move diagnostics ownership into workflow nodes or frontend state.

The later implementation plan should define:

- what new diagnostics event or ledger record shape is needed
- how it integrates with current trace-style observability
- which information remains transient trace data versus durable compliance data

## Non-Goals for This Note

This note does not yet settle:

- exact API field names
- exact SQL schema
- exact cryptographic credential format
- whether buckets are globally client-owned across all historical sessions or
  session-scoped with durable history
- exact heartbeat / lease / timeout values for session liveness
- exact export/reporting formats

Those choices should be made by a later implementation plan, but that plan must
remain within the constraints and invariants described here.
