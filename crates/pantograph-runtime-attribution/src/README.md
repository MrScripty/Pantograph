# crates/pantograph-runtime-attribution/src

## Purpose
This directory owns Pantograph's durable attribution domain for clients,
credentials, client sessions, buckets, workflow versions, and workflow runs.
Runtime execution, diagnostics, adapters, bindings, and nodes consume
validated attribution facts from this crate instead of trusting caller-supplied
ids.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Public crate facade for validated attribution ids, records, command requests, typed errors, and SQLite-backed state transitions. |

## Problem
Diagnostics and model/license usage events need stable caller and run lineage
before execution starts. Request-local workflow ids, workflow-session ids, or
node-authored metadata cannot provide restart-safe attribution, credential
verification, bucket lineage, or one-active-session enforcement.

## Constraints
- Raw credential secrets are returned once and are never persisted.
- Bucket records are Pantograph-owned durable attribution groupings scoped to a
  client namespace.
- Client-session lifecycle transitions are owned by one store and persisted in
  complete SQLite transactions.
- Workflow-run attribution must exist before runtime scheduling begins.
- Workflow-version records are resolved before immutable queue/run snapshot
  creation and enforce strict semantic-version/fingerprint agreement.
- This crate must not depend on GUI, binding, adapter, or runtime execution
  crates.

## Decision
Keep the first implementation in one crate with synchronous domain operations
over a SQLite connection. The crate owns the schema, migration version,
validated id newtypes, lifecycle enums, records, credential digesting,
transactional state transitions, and typed rejection errors. Callers compose
the store at service boundaries and pass only validated records into execution
paths.

## Alternatives Rejected
- Store raw bearer secrets. Rejected because credential material must not appear
  in persistence, diagnostics, logs, workflow-run records, or fixtures.
- Reuse `pantograph-runtime-identity`. Rejected because that crate is limited
  to dependency-light runtime alias normalization and must not grow lifecycle or
  persistence policy.
- Let workflow-service own attribution persistence. Rejected because attribution
  is a shared backend contract needed by diagnostics and later runtime surfaces.

## Invariants
- Every active client session has exactly one default bucket assignment.
- At most one active session exists per client.
- Every workflow run points to one client, one client session, and one bucket.
- Each `(workflow_id, semantic_version)` maps to exactly one execution
  fingerprint, and each `(workflow_id, execution_fingerprint)` maps to exactly
  one semantic version.
- Explicit bucket selection must stay inside the session client's namespace.
- Credential verification compares digest-only persistent state.

## Revisit Triggers
- A release security review requires a dedicated secret-generation dependency.
- Diagnostics ledger implementation needs shared retention or migration helpers.
- SQLite becomes insufficient for supported restart or concurrency targets.
- Bindings require generated DTOs for attribution records.

## Dependencies
**Internal:** None.

**External:** `rusqlite` for local durable persistence, `blake3` for digesting
high-entropy bearer credentials, `uuid` for backend-generated ids and
credential material, `chrono` for UTC timestamps, `serde` for DTO projection,
and `thiserror` for typed errors.

## Related ADRs
- `None yet.`
- Reason: Stage `01` implementation must add the durable attribution ADR before
  completion.
- Revisit trigger: Stage `01` reaches Wave `03`.

## Usage Examples
```rust
use pantograph_runtime_attribution::{
    ClientRegistrationRequest, ClientSessionOpenRequest, SqliteAttributionStore,
};

let mut store = SqliteAttributionStore::open_in_memory()?;
let registered = store.register_client(ClientRegistrationRequest {
    display_name: Some("local gui".to_string()),
    metadata_json: None,
})?;
let opened = store.open_session(ClientSessionOpenRequest {
    credential: registered.credential.proof_request(),
    takeover: false,
    reason: Some("launch".to_string()),
})?;

assert_eq!(opened.session.client_id, registered.client.client_id);
# Ok::<(), pantograph_runtime_attribution::AttributionError>(())
```

## API Consumer Contract
- Parse external strings into validated id and request types at the boundary.
- Treat `CredentialSecret` as response-only material. It redacts `Debug` output
  and must not be copied into diagnostics records.
- Use store commands for lifecycle transitions; do not mutate records directly.
- Use returned `WorkflowRunRecord` values as the trusted execution attribution.

## Structured Producer Contract
- SQLite schema version `2` is the current breaking-cutover schema version.
- Persisted credential rows contain credential id, client id, salt bytes,
  digest bytes, status, timestamps, and no raw secret.
- Persisted workflow-version rows contain workflow id, semantic version,
  execution fingerprint, canonical executable topology JSON, and creation
  timestamp.
- Lifecycle history is append-only through `session_lifecycle_records`.
- Diagnostic query indexes are maintained for client, session, bucket,
  workflow, and workflow-run lookup.
