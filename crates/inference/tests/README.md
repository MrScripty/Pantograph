# crates/inference/tests

## Purpose

This directory contains integration tests for the `inference` crate's managed
runtime boundary, transitional managed media planning adapter, and adapters
into neutral managed dependency DTOs.

## Contents

| File | Description |
| ---- | ----------- |
| `device_contracts.rs` | Public device/runtime contract fixture tests for runtime variant capabilities, backend execution decisions, typed diagnostics, and invalid raw identifier rejection. |
| `managed_media_dependencies.rs` | Transitional conversion dependency lease and media-tool/native-library planning adapter tests over the media-conversion owner. |
| `managed_redistributables.rs` | Managed redistributable owner contract and inference neutral status projection tests. |
| `model_contracts.rs` | Public contract fixture tests for Pumas model refs, package facts, task evidence, generation defaults, option diagnostics, lifecycle phases, package-facts summary snapshots, and model-library update feeds. |
| `fixtures/device_contracts/` | Stable JSON fixtures for device policy, runtime variant, backend candidate/decision, and diagnostics DTOs. |
| `fixtures/inference_package_facts/` | Named JSON fixtures matching the inference execution boundary plan. |

## Problem

Managed runtime and media dependency behavior crosses filesystem state,
catalog metadata, platform-specific expected files, and durable activation
records. Unit-only tests would miss the integration behavior that determines
whether Settings and conversion jobs receive trustworthy readiness facts.
Device/runtime contracts and model package facts also cross producer/consumer
boundaries, so fixture tests prove that Pantograph can decode Pumas-style and
scheduler-facing DTOs without depending on Pumas storage internals or raw
backend device strings.

## Constraints

- Tests must use temporary app-owned roots and avoid mutating developer
  machines.
- Managed media dependency readiness must not rely on ambient host `PATH` or
  system library discovery.
- Tests may create placeholder files for expected managed artifacts, but they
  must not require real third-party media binaries.
- Model package fixtures must be stable JSON DTOs and must not expose Pumas
  SQLite layout, `models.metadata_json`, or search-cache internals.
- Device contract fixtures must use canonical lowercase ids and reject raw
  backend-local selectors such as llama.cpp `CUDA0`.

## Decision

Keep managed redistributable owner coverage and conversion dependency adapter
coverage as integration tests under this directory while inference still
publishes neutral dependency projection helpers and legacy media-planning DTOs.
They exercise the public crate contracts with temporary storage so status,
activation, lease behavior, and neutral DTO projections remain auditable.
Keep model package fixtures here as public contract tests because later
workflow, backend, and diagnostics slices will consume the same shapes.
Keep device/runtime fixtures here for the same reason: later Tauri, frontend,
diagnostics-ledger, worker, and persisted-state slices must consume these
canonical shapes rather than inventing raw-device compatibility paths.

## Alternatives Rejected

- Put these tests only beside implementation modules: rejected because the
  behavior depends on public integration across catalog, paths, state, and
  operations modules.
- Require real `ffmpeg`, `ocioconvert`, `oiiotool`, or OpenColorIO installs:
  rejected because tests should validate Pantograph's managed boundary without
  depending on host tools.

## Invariants

- Test roots are temporary and app-owned.
- Placeholder managed files stand in for managed artifacts only when the test
  is about Pantograph readiness logic, not third-party binary execution.
- Active-version removal and lease conflicts must fail closed.
- Conversion dependency plans must validate holder attribution before any lease
  is acquired and must roll back earlier leases if a later dependency fails.
- Executable path resolution must accept only managed tool dependencies and
  reject native library artifacts such as OpenColorIO.

## Revisit Triggers

- Managed media dependencies gain real download/checksum operations.
- OpenColorIO ABI validation starts loading native libraries in tests.
- The inference media-planning compatibility adapter is removed.
- Inference no longer aggregates managed dependency status for runtime,
  media-tool, and native-artifact dependencies.
- The holder convention changes or becomes a typed cross-crate contract.
- Pumas package-fact fixtures move to a shared contract crate or schema package.

## Dependencies

**Internal:** inference neutral dependency projection helpers,
`inference::managed_media_dependencies`, `pantograph-media-conversion`, and
`pantograph-managed-dependencies` DTOs.

**External:** temporary filesystem support from the Rust test environment.

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`
- Reason: these tests protect backend-owned dependency facts consumed by the
  workbench Settings page.
- Revisit trigger: workbench starts deriving media dependency readiness outside
  backend DTOs.

## Usage Examples

Run the focused integration tests from the workspace root:

```bash
cargo test -p inference --test managed_redistributables
cargo test -p inference --test managed_media_dependencies
cargo test -p inference --test model_contracts
cargo test -p inference --test device_contracts
```

## API Consumer Contract

- Inputs: public device/runtime DTOs, managed redistributable ids,
  managed-dependency staging and activation requests, and legacy inference
  conversion dependency plans.
- Outputs: managed-dependency owner state transitions, inference neutral status
  projections, and explicit errors for invalid or unsafe operations.
- Lifecycle: each test creates temporary roots, writes only its own placeholder
  artifacts, and lets test cleanup remove them.
- Lease attribution: conversion dependency tests assert holder, dependency id,
  active version, lease id, install root, expected files, rollback, and release
  behavior.
- Executable resolution: conversion dependency tests assert that tool
  dependencies resolve to app-owned managed executable paths and that native
  library artifacts cannot be treated as tools.

## Structured Producer Contract

- Test fixtures are producer evidence for public status and lease DTOs.
- Package-fact fixtures are producer evidence for public model/task/generation
  contracts, summary snapshots, and update-feed cache invalidation facts.
- Device contract fixtures are producer evidence for scheduler-facing runtime
  variant capability, selected backend execution decision, and typed diagnostic
  payloads.
- Any DTO shape changes require updating these integration tests and the
  workflow-service/frontend contract tests that consume the same facts.
