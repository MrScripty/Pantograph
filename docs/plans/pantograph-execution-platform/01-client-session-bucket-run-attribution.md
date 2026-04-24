# 01: Client Session, Bucket, And Workflow Run Attribution

## Purpose

Create the durable identity chain required before diagnostics and model/license
usage records can be reliable.

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
- `SessionLiveness`
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

## Open Design Decisions

- Whether non-default buckets are session-scoped or globally client-owned with
  session history.
- Client credential format and storage.
- Session liveness, expiry, and takeover rules.
- How public `client session` terminology is separated from existing internal
  workflow-session/keep-alive terminology.

## Affected Structured Contracts And Persisted Artifacts

- Client, credential, session, bucket, workflow-run, and usage-event identity
  records.
- Session liveness and takeover state.
- Default bucket assignment records.
- Workflow-run attribution records and indexes used by diagnostics queries.

## Standards Compliance Notes

- Rust API compliance requires validated newtypes for externally supplied ids,
  explicit enums for liveness/takeover states, and typed errors for rejected
  lineage, credential, or bucket-selection requests.
- Architecture compliance requires attribution to resolve before execution and
  remain owned by backend services; nodes, GUI, and bindings cannot supply
  trusted runtime attribution directly.
- Concurrency compliance requires one clear owner for session liveness,
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

- Define client registration and credential verification flow.
- Define session open, resume, liveness, expiry, and takeover behavior.
- Enforce at most one active session per client.
- Create the default bucket automatically for each active session.
- Define additional bucket creation and selection behavior.
- Ensure every workflow run is recorded before node execution starts.
- Assign runs without explicit bucket ids to the session's default bucket.
- Rename, privatize, or clearly separate existing "workflow session"
  terminology from public client-session terminology.

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
