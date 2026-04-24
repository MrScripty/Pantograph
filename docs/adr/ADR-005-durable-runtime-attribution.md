# ADR-005: Durable Runtime Attribution

## Status
Accepted

## Context
Pantograph needs durable client, session, bucket, and workflow-run identity
before runtime diagnostics and model/license usage records can be trusted.
Earlier workflow execution APIs allowed caller-authored run identifiers and used
workflow-session terminology for scheduler-managed runtime reuse. That made it
too easy to confuse an execution-session id with a durable client-session id,
and it left later diagnostics without a stable attribution chain.

Stage `01` of the execution-platform plan implemented the first durable
attribution boundary.

## Decision
Create `pantograph-runtime-attribution` as the canonical owner of runtime
attribution state:

- validated client, credential, client-session, bucket, workflow, workflow-run,
  and usage-event identity types
- client and credential records
- client-session lifecycle records
- Pantograph-owned bucket records and default bucket assignments
- workflow-run attribution records
- typed validation, credential, lifecycle, bucket, and storage errors
- SQLite schema versioning, migrations, indexes, and transactional state
  transitions

Use SQLite as the first durable local attribution store. The schema records
clients, credential digests, sessions, session lifecycle history, buckets,
default bucket assignments, and workflow runs. Storage fails closed on
unsupported future schema versions.

Persist credential secrets as digest-only records. The backend generates the
raw bearer secret during client registration, returns it once, and stores only a
per-credential salt plus digest. Raw credential material must not be written to
diagnostics, workflow-run records, usage events, logs, or fixtures.

Make buckets Pantograph-owned attribution and scheduling groupings. Clients may
request bucket creation and select existing buckets, but bucket ids and
persistent bucket records are backend-owned. Bucket names are immutable,
unique only within a client namespace, and the default bucket cannot be renamed
or deleted.

Require workflow-run attribution to be created by the backend before attributed
execution starts. `pantograph-workflow-service` resolves a validated
client-session and bucket into a durable `WorkflowRunRecord`, rejects
caller-supplied run ids at public run boundaries, and passes the backend-owned
run id into execution.

Reserve `client session` terminology for the durable caller identity session.
Scheduler-managed runtime reuse is named `workflow execution session` in public
Rust, serialized diagnostics, and host adapter projections. Legacy
workflow-session public wrappers are removed instead of kept as compatibility
aliases.

Keep host adapters as projections. Tauri commands, UniFFI wrappers, Rustler
NIFs, and frontend HTTP adapters may parse transport payloads, inject host
resources, and call backend services, but they must not own attribution policy,
credential verification policy, bucket lineage rules, scheduler policy, or
workflow execution semantics.

## Consequences

### Positive
- Diagnostics and future model/license usage records can attach to a durable
  identity chain: client, client session, bucket, workflow run, and later usage
  event.
- Credential storage avoids raw secret persistence.
- Bucket and session lineage is enforced in one backend-owned domain instead of
  being reconstructed in adapters.
- Host bindings project one backend contract instead of inventing
  language-local attribution semantics.
- Workflow execution-session terminology no longer collides with durable
  client-session terminology.

### Negative
- Callers that used legacy workflow-session binding wrappers must migrate to
  durable attribution APIs or native execution-session management APIs.
- JSON field names for scheduler inspection and graph-state projections changed
  from `workflow_session_*` to `workflow_execution_session_*`.
- SQLite becomes part of the runtime attribution persistence baseline and must
  remain covered by dependency, migration, and release review.
- Non-attributed local workflow execution can still exist for direct execution
  and compatibility, but it must not be treated as durable diagnostics
  attribution unless it creates a `WorkflowRunRecord` through the attribution
  owner.

## Implementation Notes
- Implemented attribution crate:
  `crates/pantograph-runtime-attribution`
- Workflow orchestration integration:
  `crates/pantograph-workflow-service`
- Embedded runtime projection:
  `crates/pantograph-embedded-runtime`
- Host adapter projections:
  `src-tauri`, `crates/pantograph-uniffi`, and `crates/pantograph-rustler`
- Stage plan:
  `docs/plans/pantograph-execution-platform/01-client-session-bucket-run-attribution.md`
- Wave ledger:
  `docs/plans/pantograph-execution-platform/implementation-waves/01-client-session-bucket-run-attribution/coordination-ledger.md`

## Compliance Mapping
- Backend ownership: attribution policy and persistence live outside host
  adapters.
- Layered separation: workflow service orchestrates, runtime executes, adapters
  project.
- Security: raw credential secrets are response-only and digest-only at rest.
- Persistence: SQLite schema versioning and transaction boundaries are owned by
  the attribution crate.
- Interop: bindings expose durable attribution projections without
  workflow-session compatibility wrappers.

## Revisit Triggers
- A remote or multi-process attribution store replaces local SQLite.
- Credential handling requires key rotation, hardware-backed secrets, or a
  stronger dedicated secret-generation dependency.
- Durable usage events introduce retention rules that change bucket deletion or
  workflow-run lifecycle semantics.
- Non-attributed local workflow execution becomes part of diagnostics or
  model/license usage reporting.
