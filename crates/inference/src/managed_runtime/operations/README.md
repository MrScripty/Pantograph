# managed_runtime/operations

## Purpose

This directory contains helper modules for the backend-managed runtime
operation entrypoint in `../operations.rs`. The parent module keeps the public
status, install, remove, catalog refresh, selection, and command-resolution
facade, while this directory holds behavior families that would otherwise make
the facade exceed the large-file threshold.

## Contents

| File | Description |
| ---- | ----------- |
| `download.rs` | Catalog version selection, retained artifact discovery, and HTTP resume/fresh response classification. |
| `projection.rs` | Runtime snapshot, readiness, retained artifact, and version-status projection from persisted state and capability facts. |
| `state_transitions.rs` | Persisted job, install/remove, selection, and runtime install-directory state transitions shared by operations and tests. |

## Problem

Managed-runtime operations need to coordinate filesystem installs, resumable
downloads, persisted state, capability projection, and launch command
resolution. Keeping all helper logic in `operations.rs` obscured these
responsibility boundaries and violated the source-size ratchet.

## Constraints

- The public managed-runtime API remains exported by the parent module.
- Helper modules must not move install or launch policy into Tauri or frontend
  adapters.
- State transitions must continue using the durable managed-runtime state file
  owned by `state.rs`.
- Download and projection helpers must preserve additive DTO behavior for host
  consumers.

## Decision

Split helper behavior by lifecycle responsibility while preserving the parent
facade. Download-source and retained-artifact decisions live in `download.rs`,
snapshot and version projection live in `projection.rs`, and persisted state
mutations live in `state_transitions.rs`.

## Alternatives Rejected

- Keeping helper logic in `operations.rs`: rejected because the file exceeded
  the large-file threshold and mixed download, projection, and persistence
  concerns.
- Moving the public operation functions into several public modules: rejected
  because callers already depend on the parent `managed_runtime` facade and do
  not need helper-module paths.

## Invariants

- Parent operation functions remain the only external entrypoints for install,
  removal, refresh, selection, status, and command resolution.
- Download helpers may classify transport behavior but must not finalize
  installs.
- Projection helpers may read persisted facts and filesystem readiness but must
  not mutate state.
- State transition helpers are responsible for durable mutations and history
  entries.

## Usage Examples

Callers should continue to use the parent managed-runtime facade:

```rust
use inference::managed_runtime::{ManagedBinaryId, managed_runtime_snapshot};

let snapshot = managed_runtime_snapshot(app_data_dir, ManagedBinaryId::LlamaCpp)?;
```

Tests under `operations_tests.rs` may import selected helpers through the
parent `operations.rs` module when they need direct coverage for resume
classification, persisted transition records, or snapshot projection.

## Revisit Triggers

- A new managed runtime family needs substantially different install or resume
  semantics.
- Host adapters require a new public managed-runtime operation that cannot be
  expressed through the existing parent facade.
- Full verification exposes duplicated state transition rules between these
  helpers and another backend runtime manager.

## Dependencies

**Internal:** `managed_runtime::contracts`, `managed_runtime::definitions`,
`managed_runtime::paths`, `managed_runtime::state`.

**External:** `reqwest`, `tokio`, `uuid`.

## Related ADRs

- [../../../../../../docs/standards-compliance-analysis/refactor-plan.md](../../../../../../docs/standards-compliance-analysis/refactor-plan.md)
