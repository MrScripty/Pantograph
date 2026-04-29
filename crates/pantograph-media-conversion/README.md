# crates/pantograph-media-conversion

Managed media conversion contract crate.

## Purpose

This crate defines Pantograph's neutral media conversion boundary for Stage
`13`: typed conversion requests, target format metadata, command plans,
result/error contracts, dependency lease attribution, and the executor trait
used by host-owned converter implementations.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Crate metadata, workspace lint adoption, and dependency declarations. |
| `src/` | Public Rust contracts and source-level README for the conversion boundary. |

## Problem

Pantograph needs real media transcoding without letting workflow-service spawn
host processes or depend on managed dependency implementation modules. A
dedicated contract crate gives host adapters and future converter executors one
stable boundary for conversion metadata and errors.

## Constraints

- The crate must not depend on `pantograph-workflow-service`, `inference`,
  Tauri, generated bindings, or frontend code.
- Media bodies may exist in internal Rust request/result structs, but
  serialized attribution records must stay body-free.
- Converter executable paths and temporary files remain implementation details
  of later host-owned executors.
- Command plans use stdin/stdout stream markers and argv vectors only; they do
  not contain ArtifactStore paths, executable paths, shell command strings, or
  host filesystem assumptions.

## Decision

Keep Stage `13` conversion contracts in this standalone crate and wire concrete
process execution in later waves. This follows the repo's package-role pattern:
contracts first, implementation after the shared shape is frozen.

## Alternatives Rejected

- Add conversion contracts directly to workflow-service: rejected because it
  would make the workflow crate own host process execution concerns.
- Add conversion contracts to `inference`: rejected because non-inference hosts
  and workflow/binding surfaces still need a neutral conversion contract.

## Invariants

- Public ids are validated before construction.
- Per-conversion dependency attribution records the dependency id, active
  version, lease id, and lease holder used for that conversion.
- Command planning maps image targets to `oiiotool`, color-managed image
  targets to `ocioconvert` plus `OpenColorIO` support, and audio/video targets
  to `ffmpeg`.
- 3D conversion planning fails closed until Pantograph owns a concrete managed
  3D converter dependency.
- Request/result errors remain typed and bounded.
- No API in this crate accepts user-supplied executable paths.

## Revisit Triggers

- A later Stage `13` wave needs long-running converter workers instead of
  bounded process invocation.
- The conversion executor requires direct ArtifactStore ownership rather than
  source/output bytes supplied by a host adapter.
- Public binding projection needs a separate serialized DTO layer.

## Dependencies

**Internal:** None.
Reason: This crate is the neutral dependency target for later host adapters.
Revisit trigger: A converter implementation lands here and requires a narrow
internal dependency.

**External:** `async-trait`, `serde`, `thiserror`, `tokio`, and `uuid`.

## Related ADRs

- `None identified as of 2026-04-29.`
- `Reason: Stage 13 has not yet landed concrete process-executor ownership.`
- `Revisit trigger: The executor boundary is wired into workflow execution or
  changes ArtifactStore descriptor metadata.`

## Usage Examples

```rust
use pantograph_media_conversion::{
    plan_image_command, MediaConversionExecutor, MediaConversionRequest,
    MediaConversionResult,
};
```

## API Consumer Contract

- Inputs: validated Rust structs containing conversion identity, source
  artifact attribution, source bytes, target format metadata, and optional
  timeout.
- Plans: deterministic image, audio, and video command plans describe required
  managed dependency ids, stdin/stdout flow, and separate argv vectors.
- Outputs: converted bytes, conversion status, target format metadata,
  converter command identity, and per-conversion dependency attribution.
- Lifecycle: hosts construct validated requests, call an executor
  implementation, and write successful outputs back to ArtifactStore.
- Errors: invalid requests, unsupported conversions, dependency
  unavailability, process failures, timeouts, cancellation, and I/O errors are
  distinct variants.
- Compatibility: changes should be additive unless all consumers and tests are
  migrated together.

## Structured Producer Contract

- Stable fields: id wrappers, target metadata fields, status enum, and
  command/dependency attribution fields are machine-consumed.
- Defaults: omitted optional target fields mean the backend-selected default.
- Enum meanings: conversion status and dependency id variants are behaviorally
  meaningful and must not be renamed without migration.
- Ordering: dependency attribution and command-plan dependency ids preserve
  acquisition/order-of-use semantics.
- Compatibility: diagnostics may persist attribution records after artifact
  bodies are deleted.
