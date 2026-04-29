# artifact_store

## Purpose

This directory contains focused implementation helpers for the workflow
ArtifactStore. The public ArtifactStore facade stays in `artifact_store.rs`;
submodules own manifest persistence/recovery, memory-cache accounting, and
stream lifecycle behavior.

## Contents

| File | Description |
| ---- | ----------- |
| `cache.rs` | In-memory cache budget checks, rebuild, insertion, and eviction helpers. |
| `manifest.rs` | Manifest DTOs, body-file naming, descriptor recovery, deletion, range, size-limit, and persistence helpers. |
| `stream.rs` | Stream open, chunk append, finalize, stream-handle, and file hashing lifecycle. |

## Invariants

- Serialized manifests store descriptor metadata and bounded stream metadata,
  never binary body bytes.
- Body files remain under the private `bodies/` directory and are addressed
  through opaque artifact handles.
- The public facade exposes descriptors and binary-safe read responses while
  keeping storage tier details private.

## Problem

Workflow outputs can contain image, audio, video, 3D, table, or generic binary
payloads that are too large and too sensitive for inline workflow JSON or
diagnostic event payloads. The workflow service needs durable payload storage
without exposing whether a body is cached in memory, on disk, or moved later.

## Constraints

- Artifact descriptors are public workflow-service contracts; body-file paths
  are private implementation details.
- Retention may remove physical bytes while preserving queryable descriptor and
  audit metadata.
- Stream lifecycle operations must append bounded chunks and expose read
  handles without placing media bytes in projection rows.
- The store must be restart-recoverable from manifests and body files under the
  configured root.

## Decision

Keep storage helpers in this subdirectory and keep the public facade in
`../artifact_store.rs`. Submodules own manifest persistence, cache accounting,
and stream lifecycle details while the facade validates calls and returns
backend-owned descriptor/read DTOs.

## Alternatives Rejected

- Store payload bodies in diagnostics rows: rejected because diagnostics rows
  must remain bounded audit metadata.
- Expose filesystem paths to clients: rejected because clients should not know
  whether payloads are in memory, on disk, or future remote storage.
- Keep stream chunks only in memory: rejected because finalized streams must
  survive process restart when policy allows retention.

## Revisit Triggers

- Artifact storage gains remote/object-store tiers.
- Retention policy becomes per-workflow, per-node, or per-artifact instead of
  global.
- Stream reads need resumable multi-client subscription semantics beyond range
  reads.

## Dependencies

**Internal:** workflow ArtifactStore facade, ArtifactStore contract DTOs, and
diagnostics I/O artifact projections.

**External:** standard filesystem APIs, `serde`, and hashing utilities used by
the workflow service.

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`
- Reason: ArtifactStore descriptors feed workbench projections while payload
  bodies stay outside raw diagnostic event and projection JSON.
- Revisit trigger: workbench pages start reading payload bodies outside typed
  ArtifactStore APIs.

## Usage Examples

ArtifactStore callers use the facade, not these helper modules directly:

Reason: callers should depend on the validated facade contract instead of
submodule storage details.

Revisit trigger: update this example if artifact reads move to async streaming
or a remote storage tier changes the facade call shape.

```rust
let descriptor = artifact_store.descriptor(&artifact_id)?;
let body = artifact_store.read_body(&artifact_id, None)?;
```

## API Consumer Contract

- Inputs: validated artifact ids, stream handles, byte ranges, and descriptor
  metadata supplied through the public facade.
- Outputs: descriptors, bounded stream state, binary body reads, and consume
  acknowledgement results.
- Lifecycle: callers create or stream artifacts through the facade, then allow
  retention cleanup to manage physical body lifetime.
- Errors: missing, expired, deleted, finalizing, and invalid-handle states must
  fail closed through typed workflow-service errors.

## Structured Producer Contract

- Manifest records are machine-consumed recovery data and must remain
  schema-versioned.
- Descriptor metadata is the durable audit shape consumed by diagnostics and
  workbench projections.
- Body files are opaque payload storage and must not be referenced directly by
  frontend or binding clients.
