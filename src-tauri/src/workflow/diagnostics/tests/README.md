# src-tauri/src/workflow/diagnostics/tests

## Purpose
Focused diagnostics test modules for the Tauri workflow diagnostics projection boundary. The parent `tests.rs` file keeps shared fixtures and small request/trace tests, while behavior-heavy runtime/scheduler projection and replay coverage stays in submodules.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `runtime_projection.rs` | Runtime/scheduler snapshot projection, runtime lifecycle normalization, and trace-store attribution coverage. |
| `replay.rs` | Clear-history, restart, replay, overlay reset, progress detail, and duplicate snapshot coverage. |

## Problem
The diagnostics test harness was large enough to obscure which behavior area a regression covered. Splitting behavior groups keeps the projection boundary reviewable without changing production diagnostics APIs.

## Constraints
- Tests remain under the parent diagnostics module so they can exercise crate-private projection helpers.
- Shared fixtures stay in `tests.rs` unless a submodule-specific fixture becomes clearer.
- Test modules must not introduce production-only helper APIs.

## Decision
Keep `runtime_projection.rs` for runtime/scheduler snapshot projection coverage and `replay.rs` for clear-history, restart, replay, and overlay reset behavior. Leave request normalization, event timing, and small trace-filter assertions in the parent harness.

## Alternatives Rejected
- Leave all diagnostics tests in one file. Rejected because the file exceeded the decomposition threshold and mixed unrelated behavior groups.
- Move diagnostics tests to integration tests. Rejected because these tests intentionally exercise crate-private projection helpers.

## Invariants
- Submodules use `super::*` to share parent fixtures and keep fixture ownership local to the diagnostics test harness.
- Runtime/scheduler projection coverage stays in `runtime_projection.rs`.
- Replay and clear-history coverage stays in `replay.rs`.

## Revisit Triggers
- Diagnostics fixtures grow enough to justify a dedicated shared fixture module.
- Production diagnostics APIs become public enough to move these tests to integration tests.

## Dependencies
**Internal:** parent diagnostics test harness and workflow diagnostics modules.

**External:** none beyond existing crate test dependencies.
- Reason: diagnostics test modules reuse the parent crate's existing test
  dependencies and do not introduce a new package boundary.
- Revisit trigger: a future integration-test split needs independent fixtures
  or external harness dependencies.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Reason: diagnostics tests assert transport projection over backend-owned workflow and runtime state.

## Usage Examples
Run focused diagnostics tests through the Tauri crate test filter.

## API Consumer Contract
- These files are crate-local tests, not a public API.
- New diagnostics behavior tests should be placed in the smallest matching behavior module.

## Structured Producer Contract
- Tests assert that diagnostics projections preserve backend-owned trace, runtime, scheduler, and overlay semantics without adapter-local reinvention.
