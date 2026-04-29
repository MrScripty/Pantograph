# crates/inference/tests

## Purpose

This directory contains integration tests for the `inference` crate's managed
runtime and managed media dependency boundaries.

## Contents

| File | Description |
| ---- | ----------- |
| `managed_media_dependencies.rs` | Conversion dependency lease and media-tool/native-library planning tests. |
| `managed_redistributables.rs` | Managed redistributable catalog, status, activation, selection, and removal tests. |

## Problem

Managed runtime and media dependency behavior crosses filesystem state,
catalog metadata, platform-specific expected files, and durable activation
records. Unit-only tests would miss the integration behavior that determines
whether Settings and conversion jobs receive trustworthy readiness facts.

## Constraints

- Tests must use temporary app-owned roots and avoid mutating developer
  machines.
- Managed media dependency readiness must not rely on ambient host `PATH` or
  system library discovery.
- Tests may create placeholder files for expected managed artifacts, but they
  must not require real third-party media binaries.

## Decision

Keep managed redistributable and conversion dependency tests as integration
tests under this directory. They exercise the public crate contracts with
temporary storage so status, activation, and lease behavior remain auditable.

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

## Revisit Triggers

- Managed media dependencies gain real download/checksum operations.
- OpenColorIO ABI validation starts loading native libraries in tests.
- Conversion dependency leasing moves to another crate or process boundary.
- The holder convention changes or becomes a typed cross-crate contract.

## Dependencies

**Internal:** `inference::managed_redistributables` and
`inference::managed_media_dependencies`.

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
```

## API Consumer Contract

- Inputs: public managed redistributable ids, staging directories, activation
  requests, and conversion dependency plans.
- Outputs: status projections, state transitions, and explicit errors for
  invalid or unsafe operations.
- Lifecycle: each test creates temporary roots, writes only its own placeholder
  artifacts, and lets test cleanup remove them.
- Lease attribution: conversion dependency tests assert holder, dependency id,
  active version, lease id, install root, expected files, rollback, and release
  behavior.

## Structured Producer Contract

- Test fixtures are producer evidence for public status and lease DTOs.
- Any DTO shape changes require updating these integration tests and the
  workflow-service/frontend contract tests that consume the same facts.
