# 01: Client Session, Bucket, And Workflow Run Attribution

## Purpose

Create the durable identity chain required before diagnostics and model/license
usage records can be reliable.

## Implementation Readiness Status

Ready for stage-start preflight after this file's decisions are recorded in the
start report required by `08-stage-start-implementation-gate.md`.

The remaining work before source edits is procedural rather than architectural:
inspect worktree status, confirm the intended write set, and record the
verification commands selected below.

## Implementation Notes

### 2026-04-24 Stage-Start Report

- Selected stage: Stage `01`, client session, bucket, and workflow-run
  attribution.
- Current branch: `main`.
- Git status before implementation: unrelated asset changes only:
  deleted `assets/3c842e69-080c-43ad-a9f0-14136e18761f.jpg`, deleted
  `assets/grok-image-6c435c73-11b8-4dcf-a8b2-f2735cc0c5d3.png`, deleted
  `assets/grok-image-e5979483-32c2-4cf5-b32f-53be66170132.png`,
  untracked `assets/banner_3.jpg`, `assets/banner_3.png`,
  `assets/github_social.jpg`, and `assets/reject/`.
- Dirty-file overlap: none. Stage `01` implementation must not touch `assets/`.
- Standards reviewed: `PLAN-STANDARDS.md`, `COMMIT-STANDARDS.md`,
  `DEPENDENCY-STANDARDS.md`, `SECURITY-STANDARDS.md`,
  `CONCURRENCY-STANDARDS.md`, `RUST-API-STANDARDS.md`,
  `RUST-SECURITY-STANDARDS.md`, `RUST-DEPENDENCY-STANDARDS.md`, and
  `RUST-TOOLING-STANDARDS.md`.
- Intended Wave `02` write set:
  `crates/pantograph-runtime-attribution/`,
  `crates/pantograph-workflow-service/`, and host-owned workspace manifests
  only for the reviewed attribution crate dependency additions.
- Adjacent inventory for later cutover:
  `crates/pantograph-uniffi/src/frontend_http.rs`,
  `crates/pantograph-uniffi/src/runtime.rs`,
  `crates/pantograph-rustler/src/frontend_http_nifs.rs`, and
  `crates/pantograph-frontend-http-adapter/src/lib.rs` expose or parse current
  workflow-session entry points and must be replaced, removed, or made internal
  before Stage `01` completes.
- Start outcome: `ready_with_recorded_assumptions`.
- Recorded assumptions:
  - Wave `02` may be executed serially by the host in this working tree when
    subagents are not explicitly authorized; the non-overlapping write-set and
    report rules still apply.
  - The first logical implementation step is the
    `attribution-domain-storage` slice, followed by targeted verification and
    an atomic commit before workflow-service cutover begins.
  - `pantograph-runtime-attribution` will use synchronous domain operations
    over SQLite transactions and avoid holding unrelated locks across
    persistence work.
- Expected verification for the first logical step:
  `cargo test -p pantograph-runtime-attribution`.
- Expected Stage `01` verification remains the command set listed in
  `Verification Commands`.

### 2026-04-24 Contract Freeze

- Attribution id newtypes: `ClientId`, `ClientCredentialId`,
  `ClientSessionId`, `BucketId`, `WorkflowRunId`, `WorkflowId`, and
  `UsageEventId`; each parses non-empty caller input at boundaries and exposes
  generated constructors for backend-owned ids.
- Lifecycle enums: `ClientStatus`, `ClientCredentialStatus`,
  `ClientSessionLifecycleState`, `BucketStatus`, and `WorkflowRunStatus`.
  Active client-session states are `Opening`, `Connected`, and
  `DisconnectedGrace`.
- Command/request names: `ClientRegistrationRequest`,
  `CredentialProofRequest`, `ClientSessionOpenRequest`,
  `ClientSessionResumeRequest`, `ClientSessionDisconnectRequest`,
  `ClientSessionExpireRequest`, `ClientSessionTakeoverRequest`,
  `BucketCreateRequest`, `BucketDeleteRequest`, `BucketSelection`, and
  `WorkflowRunStartRequest`.
- Error families: validation errors for malformed ids and bounded names,
  credential errors for missing, malformed, revoked, or mismatched proofs,
  lifecycle errors for duplicate active sessions, invalid transitions, and
  expired sessions, bucket errors for collisions, cross-client selection,
  default deletion, deletion protection, and unsupported rename, and storage
  errors for migration, unsupported schema version, transaction, and
  persistence failures.
- SQLite schema outline:
  `attribution_schema_migrations`, `clients`, `client_credentials`,
  `client_sessions`, `session_lifecycle_records`, `buckets`,
  `default_bucket_assignments`, and `workflow_runs`, with indexes for client,
  session, bucket, workflow, and workflow-run diagnostics lookup.
- Public workflow-session cutover inventory:
  - `WorkflowSessionCreateRequest`, `WorkflowSessionRunRequest`,
    `WorkflowSessionCloseRequest`, `WorkflowSessionStatusRequest`,
    `WorkflowSessionQueueListRequest`, `WorkflowSessionQueueCancelRequest`,
    `WorkflowSessionQueueReprioritizeRequest`,
    `WorkflowSessionKeepAliveRequest`, `WorkflowSessionInspectionRequest`, and
    stale-cleanup contracts are currently re-exported from
    `crates/pantograph-workflow-service/src/lib.rs`.
  - `WorkflowService::create_workflow_session`,
    `WorkflowService::run_workflow_session`,
    `WorkflowService::close_workflow_session`,
    `WorkflowService::workflow_get_session_status`, queue operations,
    keep-alive operations, and stale-cleanup worker entry points are current
    public workflow-service session APIs.
  - UniFFI and Rustler frontend wrappers expose the same workflow-session API
    shape and must not remain compatibility aliases after durable
    client-session APIs are introduced.
- Dependency review before manifest edits:
  - SQLite: `rusqlite` is already present in `Cargo.lock` at `0.32.1` through
    `pumas-library`; Stage `01` may add a direct crate-local dependency to
    `pantograph-runtime-attribution` using that locked version and SQLite
    transaction APIs. No new transitive dependency family is expected from the
    lockfile baseline.
  - Credential digest: `blake3` is already present in `Cargo.lock` at `1.8.3`.
    Stage `01` may add a direct crate-local dependency to store keyed or
    salted digest bytes instead of raw bearer secrets. Raw credential material
    remains response-only and must not be persisted or logged.
  - Credential secret generation: reuse the existing workspace `uuid` crate
    for backend-owned opaque ids and generated credential material in the first
    slice; revisit before release if a stronger dedicated secret generator is
    required by the security review.

## Required Identity Chain

```text
client
  -> session
    -> bucket
      -> workflow run
        -> model/license usage event
```

## Type Families To Define

### Identity Types

- `ClientId`
- `ClientCredentialId`
- `ClientSessionId`
- `BucketId`
- `WorkflowRunId`
- `WorkflowId`
- `UsageEventId`

### Durable Attribution Types

- `ClientRecord`
- `ClientCredential`
- `ClientSessionRecord`
- `SessionLifecycleRecord`
- `BucketRecord`
- `DefaultBucketAssignment`
- `WorkflowRunRecord`
- `WorkflowRunAttribution`
- `ClientSessionOpenRequest`
- `ClientSessionResumeRequest`
- `BucketSelection`

## Required Behavior

- Clients must register before opening sessions.
- Existing callers must prove they are the same client identity.
- A client may have at most one active session at a time.
- Sessions must persist across disconnect/reconnect cycles for diagnostics
  continuity.
- Every active session must have exactly one default bucket.
- Buckets must be durable attribution and scheduling groupings, not transient
  request-local labels.
- Every workflow run must attach to exactly one bucket.
- If no bucket is supplied, the run must attach to the session's default bucket.
- Every model/license usage event must attach to exactly one workflow run.
- A bucket used by a run must belong to the same client/session lineage as that
  run.

## Implementation Decisions

### Ownership And Layering

- `crates/pantograph-runtime-attribution` owns the canonical attribution
  domain:
  validated ids, records, credential verification policy, bucket lineage rules,
  session lifecycle state, workflow-run attribution, typed errors, and storage
  repository traits.
- `crates/pantograph-runtime-identity` currently owns runtime/backend alias
  normalization only. Stage `01` must not broaden that crate into client,
  credential, session, bucket, or workflow-run persistence because its existing
  README and public contract intentionally keep it dependency-light and
  lifecycle-free.
- `crates/pantograph-workflow-service` owns orchestration only: it resolves a
  validated client session and bucket into a workflow-run record before
  scheduling execution.
- `crates/pantograph-frontend-http-adapter` treats every request payload as
  untrusted, parses it into validated identity request types, and does not
  contain attribution policy.
- Runtime, GUI, bindings, and nodes consume projected attribution facts. They
  must not create trusted client/session/bucket/run ids from raw caller input.

### Persistence Model

- The first implementation uses SQLite as the durable local storage engine for
  attribution records.
- `pantograph-runtime-attribution` owns the SQLite schema, migrations,
  repository abstraction, and storage implementation for clients, credential
  digests, sessions, session lifecycle records, buckets, default bucket
  assignments, workflow runs, and indexes needed to enforce uniqueness and
  lineage rules on restart.
- Writes are serialized through one identity store owner. Each mutating command
  produces a complete validated state transition and persists it in one SQLite
  transaction.
- The persisted schema carries an explicit migration version. Unsupported
  future versions fail closed with a typed storage error.
- The SQLite dependency, feature selection, bundled/native-linking behavior,
  audit impact, and release artifact impact must be recorded before
  implementation.
- The stage-start report must inspect SQLite and credential-digest dependency
  cost using the Rust dependency standards before either dependency is added.

### Attribution SQLite Schema Decision

The first schema must include, at minimum:

- `clients`: `client_id`, display metadata, creation timestamp, status.
- `client_credentials`: `client_credential_id`, `client_id`, salt, digest,
  status, creation timestamp, optional revocation timestamp.
- `client_sessions`: `client_session_id`, `client_id`, opened timestamp,
  latest lifecycle state, optional grace deadline, optional superseded-by
  session id.
- `session_lifecycle_records`: event id, `client_session_id`, lifecycle state,
  timestamp, reason, optional related session id.
- `buckets`: `bucket_id`, `client_id`, immutable name, metadata, creation
  timestamp, optional deletion timestamp, deletion reason.
- `default_bucket_assignments`: `client_session_id`, `bucket_id`, assigned
  timestamp.
- `workflow_runs`: `workflow_run_id`, `workflow_id`, `client_id`,
  `client_session_id`, `bucket_id`, lifecycle/status, started timestamp,
  optional completed timestamp.

Required indexes and constraints:

- unique active session per `client_id` for states that count as active
- unique bucket name per `client_id` where the bucket is not deleted
- foreign keys preserving client/session/bucket/workflow-run lineage
- indexes for diagnostics queries by client, session, bucket, workflow, and
  workflow run

### Client Credential Decision

- Client credentials are opaque bearer secrets generated by the backend during
  client registration.
- The raw credential secret is returned only once to the caller and is never
  written to diagnostics, workflow-run records, usage events, logs, or test
  fixtures.
- Persistent credential storage keeps `ClientCredentialId`, per-credential salt,
  digest, creation timestamp, status, and optional revocation timestamp. It does
  not store the raw secret.
- Credential verification parses external credential material at the adapter or
  service boundary into a validated credential proof request, compares it
  through the identity domain, and returns typed errors for missing, malformed,
  revoked, or mismatched credentials.
- Any hash implementation or dependency added for credential digests must be
  documented in the stage dependency review before source edits. If no approved
  digest implementation is available, implementation must stop rather than
  persisting raw secrets.

### Bucket Ownership Decision

- Pantograph owns and persists bucket records.
- Clients request or define buckets through Pantograph APIs, but the resulting
  bucket is a backend-owned durable scheduling and attribution record.
- Buckets are scoped to a client identity for lineage and authorization. This
  means a bucket belongs to a Pantograph client namespace, not that the external
  client stores or controls the durable record.
- A bucket may be associated with the session that created it for audit, but it
  remains persisted by Pantograph and can be reused by later sessions from the
  same client.
- Bucket names are unique within a single `client_id` namespace and are not
  globally unique across clients.
- Bucket ids are generated by Pantograph. Clients may supply requested bucket
  names and metadata, but not trusted bucket ids.
- Clients may create buckets.
- Clients may delete non-default buckets when no active workflow-run or
  retention rule prevents deletion.
- Buckets cannot be renamed. A rename request must be rejected with a typed
  unsupported-operation error that explains bucket names are immutable.
- The default bucket cannot be renamed or deleted.
- Creating a bucket with a name that already exists for the same client is
  rejected with a typed name-collision error that is safe to return to the
  client.
- Deleting a missing bucket, a bucket owned by another client, the default
  bucket, or a bucket protected by active scheduling/retention state is rejected
  with a typed error explaining why the operation could not be completed.
- Every active session has exactly one default bucket assignment. The default
  assignment points to a Pantograph-owned bucket in the client's namespace and
  is created automatically when a session is opened if no reusable default
  exists.
- Non-default bucket selection is allowed only when the selected bucket belongs
  to the same client as the active session.
- A workflow run stores both `client_id` and `client_session_id` plus the
  selected `bucket_id`. This preserves session history while allowing
  Pantograph-owned bucket continuity across reconnects and later sessions.

### Session Lifecycle, Expiry, And Takeover Decision

- A client may have at most one active session. Active means `opening`,
  `connected`, or `disconnected_grace`.
- Session lifecycle state is represented by an explicit enum rather than
  booleans:
  `Opening`, `Connected`, `DisconnectedGrace`, `Expired`, `TakenOver`, and
  `Closed`.
- Every lifecycle transition is stored as a `SessionLifecycleRecord` so
  diagnostics can reconstruct session history instead of seeing only the latest
  session state.
- Resume is idempotent for the same client credential and session id while the
  session is `Connected` or `DisconnectedGrace`.
- Disconnect moves a session to `DisconnectedGrace` with a monotonic grace
  deadline. Expiry moves it to `Expired` after that deadline.
- Opening a new session while another session is active is rejected unless the
  request explicitly asks for takeover.
- Takeover is a single transactional transition: the previous active session is
  marked `TakenOver`, a new session is created with its own default bucket
  assignment, and the response includes the superseded session id.
- Workflow runs that already started under a taken-over session retain their
  original session attribution. New runs must use the new active session.

### Terminology Decision

- Public API and persisted attribution use `client session` only for the durable
  caller identity session described in this plan.
- Existing workflow execution session or keep-alive concepts must be renamed or
  kept private to their owning module before they cross public API,
  diagnostics, or persisted record boundaries.
- Existing workflow-session public surfaces are not maintained for backward
  compatibility. Stage `01` must replace or remove them cleanly at the
  boundary where durable client sessions become the supported API.
- If an existing type named `Session` cannot be renamed safely in the same
  stage, the public type must use an explicit prefix such as
  `ClientSessionRecord`, and the conflict must be recorded in the stage-start
  report.

### Public API Upgrade Decision

- The durable client-session API replaces legacy workflow-session entry points
  for callers affected by attribution.
- No compatibility shim may allow callers to keep using a workflow-session id
  as trusted client/session/bucket/run attribution.
- Stage `01` must identify each affected workflow, HTTP adapter, Rustler, and
  UniFFI-facing entry point in the stage-start report and classify it as:
  replaced by the durable client-session API, made crate-private/internal, or
  removed.
- Any persisted or generated artifacts that use the old workflow-session public
  shape must be regenerated or migrated in the same logical slice that changes
  the owning API. The final state must not carry dual public contracts.

### Concurrency And Lifecycle Decision

- The identity store is the single lifecycle owner for session state,
  credential status, default bucket assignment, and workflow-run attribution.
- Mutating operations are command-shaped and serialized by the owner rather
  than allowing independent modules to update shared state directly.
- State transitions are synchronous domain operations over validated records.
  Async is limited to the persistence shell.
- No lock may be held across blocking SQLite work. The implementation must use
  a single persistence owner and transaction boundary rather than allowing
  unrelated modules to hold state locks while performing database I/O.
- Cancellation during persistence must leave either the previous committed
  transaction or the next complete committed transaction, never partially
  applied attribution state.

### Stage Dependency Decision

- This stage should avoid new runtime dependencies unless credential digest
  support requires one.
- Any credential digest dependency must have an owner, feature/audit impact,
  transitive-cost note, and release impact recorded before implementation.
- No binding, GUI, or runtime-observability dependencies are added in this
  stage.

## Affected Structured Contracts And Persisted Artifacts

- Client, credential, session, bucket, workflow-run, and usage-event identity
  records.
- Session lifecycle and takeover state.
- Default bucket assignment records.
- Workflow-run attribution records and indexes used by diagnostics queries.

## Standards Compliance Notes

- Rust API compliance requires validated newtypes for externally supplied ids,
  explicit enums for lifecycle/takeover states, and typed errors for rejected
  lineage, credential, or bucket-selection requests.
- Architecture compliance requires attribution to resolve before execution and
  remain owned by backend services; nodes, GUI, and bindings cannot supply
  trusted runtime attribution directly.
- Concurrency compliance requires one clear owner for session lifecycle,
  takeover, expiry, and reconnect races. Durable state transitions must be
  transactional, idempotent, or explicitly compensating across cancellation
  points.
- Security compliance requires credential parsing at the boundary, no raw
  credential material in diagnostics records, bounded request payloads, and
  rejection of cross-client bucket/run lineage mismatches.
- Testing compliance requires restart/reconnect tests, duplicate active-session
  rejection tests, lineage mismatch tests, and persistence recovery tests.

## Risks And Mitigations

- Risk: the public client-session model conflicts with existing workflow
  session terminology. Mitigation: rename or isolate internal terminology
  before adding public contracts.
- Risk: reconnect and takeover create race conditions. Mitigation: define a
  single durable session-state transition owner before implementation.
- Risk: diagnostics become unreliable if workflow runs are created lazily after
  node execution starts. Mitigation: require workflow-run records before
  scheduling nodes.

## Tasks

- Record the stage-start report required by
  `08-stage-start-implementation-gate.md`, including dirty-file status,
  intended write set, standards reviewed, and the verification commands below.
- Add `crates/pantograph-runtime-attribution` with README coverage required for
  a new source crate.
- Define client registration and credential verification flow.
- Define session open, resume, lifecycle state, expiry, and takeover behavior.
- Enforce at most one active session per client.
- Create the default bucket automatically for each active session.
- Define bucket creation, deletion, immutable-name, default-bucket protection,
  name-collision, and selection behavior.
- Ensure every workflow run is recorded before node execution starts.
- Assign runs without explicit bucket ids to the session's default bucket.
- Rename, privatize, or clearly separate existing "workflow session"
  terminology from public client-session terminology.
- Remove or replace legacy workflow-session public API entry points instead of
  preserving backward-compatible wrappers.

## Intended Write Set

- Primary:
  - `crates/pantograph-runtime-attribution/`
  - `crates/pantograph-workflow-service/`
- Adjacent only if required by existing call sites:
  - `crates/pantograph-embedded-runtime/`
  - `crates/pantograph-frontend-http-adapter/`
  - `crates/pantograph-rustler/`
  - workspace manifests for an approved credential digest dependency
- Forbidden for this stage unless the plan is updated first:
  - GUI implementation files
  - host binding crates
  - model/license diagnostics ledger implementation
  - node contract registry implementation

## Existing Code Impact

- `crates/pantograph-runtime-identity/` currently contains runtime/backend
  alias normalization helpers and must remain limited to that role.
- `crates/pantograph-workflow-service/src/workflow/session_lifecycle_api.rs`
  currently manages workflow execution session keep-alive and stale cleanup.
  Stage `01` must either make this internal to the workflow execution cache or
  replace the public surface with durable client-session lifecycle records.
- `crates/pantograph-workflow-service/src/scheduler/store.rs` currently owns
  in-memory `WorkflowSessionStore` state for workflow scheduling, queueing,
  warm runtime reuse, and loaded/unloaded runtime state. It must either consume
  resolved durable attribution from `pantograph-runtime-attribution` or be
  renamed/private enough that callers cannot confuse it with client sessions.
- `crates/pantograph-workflow-service/src/workflow/workflow_run_api.rs`
  currently creates or accepts `run_id` after execution completes. Stage `01`
  must change this boundary so a durable workflow-run attribution record exists
  before host execution starts.
- `crates/pantograph-workflow-service/src/graph/session*.rs` currently owns
  graph edit sessions. These are editing/runtime sessions, not durable client
  sessions, and must not become the attribution source of truth.
- `crates/pantograph-embedded-runtime/src/workflow_session_execution.rs`
  currently keys warm workflow executors by workflow session id. Stage `01`
  must decide whether that id remains an internal workflow-session id or is
  explicitly related to, but distinct from, `ClientSessionId`.
- `crates/pantograph-rustler/src/frontend_http_nifs.rs` currently exposes
  workflow-session NIFs. Stage `01` must remove or replace those public NIFs
  when durable client-session APIs take over; they must not remain as
  compatibility aliases.

## Milestones

1. Identity contracts and errors:
   define validated ids, request types, lifecycle enum, record types, and typed
   errors in `pantograph-runtime-attribution`.
2. Durable identity repository:
   implement SQLite schema migrations, repository validation, transactional
   writes, restart recovery tests, and unsupported-version rejection.
3. Credential registration and verification:
   generate opaque credentials, store digest-only credential records, verify
   proofs, and reject malformed, revoked, or mismatched credentials.
4. Session and bucket lifecycle:
   implement open, resume, disconnect grace, expiry, explicit takeover, default
   bucket assignment, and Pantograph-owned non-default bucket selection.
5. Workflow-run attribution integration:
   require workflow-run records before execution scheduling and reject
   cross-client bucket/session lineage mismatches.
6. Terminology cleanup:
   rename, privatize, or remove conflicting workflow-session names before they
   reach public API, diagnostics, or persistence boundaries.
7. Public API cutover:
   replace affected public workflow-session entry points with durable
   client-session APIs and regenerate or migrate any dependent artifacts without
   leaving backward-compatible residual wrappers.

## Verification Commands

Expected stage verification:

```bash
cargo test -p pantograph-runtime-attribution
cargo test -p pantograph-workflow-service
cargo check --workspace --all-features
```

If a public feature is added to any touched crate, also run:

```bash
cargo check --workspace --no-default-features
```

If implementation touches HTTP adapter request parsing, also run the adapter's
targeted tests or add the missing targeted command to the stage-start report
before editing adapter code.

Stage completion also requires the Rust baseline verification from
`RUST-TOOLING-STANDARDS.md` unless the stage-start report records an existing
repo-owned equivalent:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Verification

- A registered client can open and resume a durable session.
- A second active session for the same client is rejected until takeover rules
  allow it.
- Every session has exactly one default bucket.
- Every workflow run has exactly one bucket.
- Bucket/client/session lineage mismatches are rejected.
- Workflow run records survive disconnect/reconnect and process restart in the
  persistence model selected by the implementation.
- Credential, lineage, and bucket-selection failures return typed errors
  without leaking secret material.
- Reconnect, takeover, and expiry tests cover concurrent requests and
  cancellation during durable state transitions.
- SQLite persistence tests prove restart recovery, unsupported schema version
  rejection, migration application, and transaction atomicity.
- Credential tests prove raw secrets are not present in diagnostics-facing
  records, workflow-run records, or persisted credential records.
- Bucket API tests prove per-client name uniqueness, cross-client duplicate
  names are allowed, bucket rename is rejected, default bucket deletion is
  rejected, non-default bucket deletion records the correct state, and error
  messages explain the rejection reason.

## Completion Criteria

- Durable client, session, bucket, and workflow-run attribution records exist.
- Workflow-run attribution is resolved before node execution.
- Runtime and diagnostics records never rely on node-authored client/session/
  bucket/run ids.
- The stage-start implementation gate in
  `08-stage-start-implementation-gate.md` is recorded before source edits.
- The stage-end refactor gate in `09-stage-end-refactor-gate.md` is completed
  or explicitly recorded as not warranted for this stage.

## Re-Plan Triggers

- The selected persistence model cannot enforce one active session per client.
- Client credential storage requires a security model outside this plan.
- Session takeover semantics conflict with existing workflow keep-alive
  behavior.
- A required caller cannot move from workflow-session public APIs to durable
  client-session APIs in the same stage.
- SQLite cannot provide atomic restart-safe updates on supported targets.
- Credential digest support requires an unapproved dependency or raw secret
  persistence.
