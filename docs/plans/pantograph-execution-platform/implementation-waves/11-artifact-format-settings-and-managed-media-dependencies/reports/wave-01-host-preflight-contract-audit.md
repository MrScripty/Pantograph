# Wave 01 Host Report: Preflight Contract Audit

## Scope

Owned write set:

- `docs/plans/pantograph-execution-platform/11-artifact-format-settings-and-managed-media-dependencies.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/**`
- `crates/pantograph-workflow-service/src/workflow/artifact_contracts.rs`
- `crates/pantograph-workflow-service/src/workflow/media_capability_contracts.rs`
- workflow-service public re-exports and integration contract tests

## Source Audit Summary

Current inline media paths include:

- workflow-node base64 image/audio input and output values;
- embedded-runtime Python bridge `image_base64` and `audio_base64` mappings;
- Torch, ONNX, Stable Audio, and depth worker JSON media payloads;
- frontend data URL construction and base64 decoding in image/audio nodes;
- C# smoke and CLI examples that decode current image/audio payloads;
- workflow value-size validation that correctly rejects oversized JSON.

Migration decision: keep JSON payload caps and move media bodies to
ArtifactStore descriptors plus binary-safe read/stream handles.

## Changes

- Added `artifact_contracts.rs` for ArtifactStore descriptors, lifecycle
  states, retention state, policy, read/stream/consume handles, and default
  image/audio/video/3D format settings.
- Added `media_capability_contracts.rs` for backend-owned media format options
  and managed redistributable status categories.
- Re-exported the new DTOs through the workflow-service public facade.
- Added `tests/artifact_contract.rs` snapshots proving descriptors and handle
  responses do not carry inline media bodies.
- Added Stage `11` wave specs and report scaffolding for later parallel waves.

## Verification

Passed:

- `cargo test -p pantograph-workflow-service --test artifact_contract`
- `cargo test -p pantograph-workflow-service --test contract`
- `cargo fmt --all -- --check`

## Residual Risk

- Contract validation is not implemented yet; later waves must validate ids,
  handle shapes, format bounds, policy bounds, and managed dependency catalog
  metadata at ingress.
- ArtifactStore physical storage, restart recovery, streaming bodies, and
  cleanup are not implemented yet.
- Managed redistributable implementation ownership is still open; Wave `03`
  must decide whether to generalize the current runtime manager or split a new
  product-category boundary.
