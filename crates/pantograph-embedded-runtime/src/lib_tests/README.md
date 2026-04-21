# crates/pantograph-embedded-runtime/src/lib_tests

## Purpose
This directory contains behavior-focused test modules split out of the legacy
embedded-runtime root test module. It exists to keep large integration and unit
test groups reviewable while preserving access to the crate-private embedded
runtime test harness.

## Problem
`lib_tests.rs` still contains a large mixed set of embedded-runtime integration
tests, helper fixtures, and unit tests. Adding more tests to that file makes
runtime behavior changes harder to review and keeps unrelated test concerns
coupled.

## Constraints
- The remaining legacy test harness still lives in `lib_tests.rs` during the
  iterative split, so focused modules may temporarily import it with
  `super::*`.
- Tests in this directory must not create alternate production-only APIs just to
  make private state easier to inspect.
- Runtime-registry and workflow-service assertions must keep checking
  caller-visible error codes where those codes are part of the adapter contract.

## Decision
Split focused embedded-runtime test groups into this directory as behavior
boundaries become clear. Start with host-helper and runtime-registry
error-mapping unit tests because they are independent of the larger integration
fixtures and provide a safe first boundary.

## Alternatives Rejected
- Keep all embedded-runtime tests in `lib_tests.rs`.
  Rejected because the file is already large enough to obscure behavior-specific
  test ownership.
- Split every test group in one change.
  Rejected because the fixture sharing is still broad and a one-shot move would
  make regressions harder to isolate.

## Contents
| File | Description |
| ---- | ----------- |
| `data_graph_execution_tests.rs` | Integration tests for embedded data-graph execution, Python sidecar runtime observation, multi-runtime registry projection, and waiting-for-input propagation. |
| `edit_session_execution_tests.rs` | Integration tests for embedded edit-session graph execution, embedding runtime prepare/restore reconciliation, runtime trace metrics, and waiting-for-input event behavior. |
| `host_helper_tests.rs` | Unit tests for embedded workflow host helper contracts and workflow-facing runtime-registry error mapping. |
| `session_checkpoint_capacity_tests.rs` | Integration tests for keep-alive workflow-session checkpoint preservation across capacity rebalance and repeated unloads. |
| `session_checkpoint_recovery_tests.rs` | Integration tests for keeping workflow-session checkpoints intact across failed restore, runtime-not-ready resume, and scheduler reclaim recovery. |
| `session_execution_state_tests.rs` | Integration tests for keep-alive workflow-session executor reuse, carried inputs, graph-change reconciliation, and inspection state. |
| `session_runtime_lifecycle_tests.rs` | Integration tests for embedded workflow-session runtime reservation, warmup, preflight, unload, and non-keep-alive release behavior. |
| `workflow_run_execution_tests.rs` | Integration tests for embedded workflow runs, session runs, cancellation, human-input validation, and Python sidecar runtime observation. |

## Invariants
- Test modules in this directory may use `super::*` to share the legacy
  embedded-runtime test harness while the remaining root test module is split.
- New embedded-runtime tests should prefer a focused module in this directory
  over growing `lib_tests.rs`.
- Runtime-registry error mapping tests must assert workflow-service error codes
  as well as variants so adapters do not drift on caller-visible failures.

## Revisit Triggers
- `lib_tests.rs` stops owning shared fixtures and this directory can switch from
  `super::*` imports to explicit local fixtures.
- A test module in this directory grows beyond a single behavior area.
- Runtime-registry error mapping moves out of embedded-runtime ownership.

## Dependencies
- `pantograph-workflow-service` for workflow-facing contracts and error codes.
- `pantograph-runtime-registry` for runtime admission and reservation errors.
- The legacy embedded-runtime test harness in `lib_tests.rs` until the remaining
  integration tests are split by behavior area.

## Related ADRs
- `docs/standards-compliance-analysis/refactor-plan.md`
- `crates/pantograph-embedded-runtime/src/README.md`

## Usage Examples
```rust
#[path = "lib_tests/host_helper_tests.rs"]
mod host_helper_tests;
```
