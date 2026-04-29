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
