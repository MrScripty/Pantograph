# crates/pantograph-media-conversion/src

Host-owned managed media conversion contracts for Pantograph.

## Purpose

This directory owns the neutral conversion boundary used between workflow
artifact descriptors, managed media dependencies, and host process execution.
It exists so `pantograph-workflow-service` can stay host-agnostic while later
Stage `13` slices add real `ffmpeg`, `oiiotool`, `ocioconvert`, and OCIO-backed
conversion.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Public conversion request/result/error contracts, typed ids, target format metadata, lease attribution records, and executor trait. |

## Problem

Stage `11` can attribute selected media formats to ArtifactStore descriptors,
but it does not invoke conversion tools. Without a separate boundary, real
conversion would either leak host process execution into workflow-service or
force adapters to invent incompatible conversion metadata.

## Constraints

- `pantograph-workflow-service` must not depend on host process execution or
  the `inference` crate.
- Converter executables and OCIO/OIIO assets must come from Pantograph-managed
  dependencies, not system PATH discovery.
- Public diagnostics must receive bounded metadata and lease attribution, not
  raw media bodies or filesystem paths.
- Conversion contracts must be usable by Tauri, embedded runtime adapters, and
  tests without requiring a live converter binary.

## Decision

Define a neutral crate for conversion contracts and executor traits. The crate
stores in-memory source/output bodies only in internal request/result types and
keeps serialized attribution records body-free. Later host slices can implement
the trait with managed dependency leases and safe process invocation.

## Alternatives Rejected

- Put conversion process execution in `pantograph-workflow-service`: rejected
  because workflow-service is the host-agnostic orchestration facade.
- Store converter paths in workflow descriptors: rejected because descriptors
  must not expose backend storage or executable paths to clients.
- Let each adapter define its own conversion DTOs: rejected because diagnostic
  attribution and binding projections need one stable contract.

## Invariants

- Conversion ids, artifact ids, workflow run ids, node ids, port ids,
  dependency ids, dependency versions, and lease ids are validated before use.
- Quality, CRF, bitrate, timeout, and stderr summaries are bounded.
- Lease attribution is recorded per conversion, not inferred from ambient
  active dependency snapshots.
- Serialized conversion attribution does not contain media bodies.

## Revisit Triggers

- Conversion needs long-running worker processes instead of bounded tool
  invocations.
- A converter requires unmanaged host PATH discovery.
- ArtifactStore cannot provide private temporary inputs or outputs without a
  storage API refactor.

## Dependencies

**Internal:** None.
Reason: The contract crate is intentionally neutral so workflow-service,
inference, Tauri, and bindings can depend on or adapt it without reverse
coupling.
Revisit trigger: A concrete converter implementation lands in this crate and
requires a dependency on managed redistributable or ArtifactStore modules.

**External:** `async-trait` for the executor trait, `serde` for attribution
records, `thiserror` for typed errors, and `uuid` for conversion ids.

## Related ADRs

- `None identified as of 2026-04-29.`
- `Reason: Stage 13 is still defining the implementation boundary; promote an
  ADR when the concrete host process executor and public metadata contract land
  across workflow-service, diagnostics, and Tauri.`
- `Revisit trigger: The conversion executor is wired into workflow execution or
  changes a public ArtifactStore descriptor field.`

## Usage Examples

```rust
use pantograph_media_conversion::{
    ConversionMediaKind, MediaConversionExecutor, MediaConversionRequest,
};
```

## API Consumer Contract

- Inputs: validated conversion ids, artifact attribution, source bytes from a
  backend-owned ArtifactStore read, and target format metadata from backend
  capability settings.
- Outputs: converted bytes, target format metadata, typed status, and
  per-conversion dependency lease attribution.
- Lifecycle: callers acquire source bytes, call an executor implementation, and
  write the returned bytes back to ArtifactStore. Executor implementations own
  converter process timeout, cancellation, temporary files, and lease release.
- Errors: invalid requests, unsupported conversion pairs, dependency
  unavailability, process failures, timeout, cancellation, and I/O failures are
  distinct typed errors.
- Compatibility: DTO changes should be additive unless workflow-service,
  diagnostics, Tauri, bindings, and frontend projections migrate together.

## Structured Producer Contract

- Stable fields: conversion ids, artifact attribution, target format metadata,
  status, and dependency attribution are machine-consumed.
- Defaults: omitted codec, quality, bitrate, CRF, bit depth, and color profile
  mean the backend-selected default for the target format.
- Enum meanings: `converted` means a converter produced new bytes;
  `passed_through` means no conversion was needed; `failed` is reserved for
  diagnostic projection of unsuccessful attempts.
- Ordering: dependency attribution preserves acquisition order.
- Compatibility: persisted diagnostics may retain attribution records after
  artifact bodies are deleted.
