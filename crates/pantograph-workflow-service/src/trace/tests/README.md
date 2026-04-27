# crates/pantograph-workflow-service/src/trace/tests

## Purpose
This directory contains behavior-focused workflow trace test modules split out
of the root trace test index. The boundary keeps long lifecycle and
scheduler/runtime attribution scenarios reviewable without moving production
trace ownership out of `pantograph-workflow-service`.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lifecycle.rs` | Trace store restart, cancellation, duplicate completion, and incremental resume behavior. |
| `scheduler_runtime.rs` | Graph reconciliation, waiting pause duration, queue attribution, scheduler decision, runtime metric selection, and backend timestamp capture behavior. |
| `timing.rs` | Timing expectation, run-summary persistence, and bounded node-status ledger producer behavior. |

## Problem
Trace store tests cover several independent behavior families. Keeping every
scenario in one file obscures lifecycle regressions from scheduler/runtime
attribution regressions and violates the large-file decomposition target.

## Constraints
- Tests must keep using the backend-owned trace contracts from the parent
  module.
- The parent `tests.rs` module remains the shared fixture/import surface so
  behavior slices do not duplicate setup helpers.
- Test movement must preserve behavior and public trace DTO expectations.

## Decision
Keep shared trace fixtures and smaller contract/filter tests in the parent
`tests.rs` module. Move larger lifecycle/restart scenarios into `lifecycle.rs`
and scheduler/runtime attribution scenarios into `scheduler_runtime.rs`.
Keep durable timing and node-status ledger producer tests in `timing.rs`
because both validate trace-store writes to `pantograph-diagnostics-ledger`
without introducing frontend or Tauri dependencies.

## Alternatives Rejected
- Keep appending scenarios to `tests.rs`.
  Rejected because the file had grown past the decomposition threshold and
  mixed unrelated behavior families.

## Invariants
- Each test module must focus on one trace behavior family.
- Test modules may use parent imports and helpers through `super::*`.
- Production trace modules remain independent of test-only fixtures.
- Durable diagnostic ledger producer tests must assert bounded event emission
  and projection results rather than inspecting raw frontend state.

## Revisit Triggers
- A behavior slice grows past the review threshold.
- New trace persistence or durable replay behavior needs a separate fixture
  family.

## Dependencies
**Internal:** parent trace tests module, workflow-service trace store contracts,
and workflow scheduler/session DTOs.

**External:** Rust unit test harness only.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- Reason: trace tests protect backend-owned workflow-service diagnostics
  contracts rather than Tauri-only behavior.
- Revisit trigger: trace ownership moves out of `pantograph-workflow-service`.

## Usage Examples
```bash
cargo test -p pantograph-workflow-service trace::tests
```

## API Consumer Contract
- These modules are crate-local tests and do not expose public APIs.
- Test names should describe the workflow trace contract they protect.

## Structured Producer Contract
- Test fixtures must continue producing backend-owned trace events and
  scheduler/runtime snapshots through the same DTOs consumed by production
  trace code.
