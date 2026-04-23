# crates/pantograph-embedded-runtime/src/workflow_runtime_tests

## Purpose
This directory keeps workflow-runtime helper tests split by behavior while the
root `workflow_runtime_tests.rs` file owns shared fixtures and module
registration.

## Contents
| File | Description |
| ---- | ----------- |
| `diagnostics_snapshot.rs` | Covers workflow execution diagnostics snapshot assembly and registry sync during snapshot construction. |
| `event_projection.rs` | Covers runtime event projection from stored and live active/embedding runtime snapshots. |
| `metrics.rs` | Covers runtime metric normalization, alias canonicalization, model-target selection, and diagnostics projection helpers. |
| `registry_reconciliation.rs` | Covers runtime-registry reconciliation for live gateway runtimes, stored sidecar runtimes, and execution overrides. |

## Problem
The original workflow-runtime test module mixed metric normalization, event
projection, diagnostics snapshot assembly, and registry reconciliation in one
large file. That made it harder to identify the behavior under test and pushed
the test module over the large-file threshold.

## Constraints
- Keep production `workflow_runtime.rs` focused on projection and
  reconciliation helpers.
- Preserve the existing mocked runtime-registry controller contract used by
  async tests.
- Keep runtime-registry assertions in tests, not in production projection
  helpers.
- Avoid duplicating scheduler/runtime fixtures across behavior modules.

## Decision
Keep shared imports and the mocked runtime-registry controller in
`workflow_runtime_tests.rs`, then split tests into behavior modules that use
`super::*` for the common fixture boundary.

## Alternatives Rejected
- Keep all workflow-runtime tests in the root module.
  Rejected because the root test module exceeded the large-file threshold and
  mixed separate diagnostics and registry concerns.
- Move the mocked controller into each behavior module.
  Rejected because duplicate controller implementations would make async
  registry sync tests harder to keep aligned.

## Invariants
- New tests should join the module matching the behavior under test before a
  new module is added.
- Registry reconciliation assertions must continue checking backend-owned
  runtime ids and instance ids after canonicalization.
- Diagnostics snapshot tests should keep scheduler and runtime facts explicit so
  projection ownership remains visible.

## Revisit Triggers
- Add a new module if workflow-runtime helper behavior grows beyond metrics,
  event projection, diagnostics snapshot, or registry reconciliation.
- Revisit fixture placement if tests require different host controller behavior
  that cannot be represented by `MockRuntimeRegistryController`.

## Dependencies
- `workflow_runtime.rs` declares this root test module and owns the helpers
  under test.
- `pantograph-runtime-registry` provides the registry state used by
  reconciliation assertions.
- `pantograph-workflow-service` provides scheduler/session and runtime metrics
  DTOs used by diagnostics snapshots.

## Related ADRs
- `crates/pantograph-embedded-runtime/src/README.md`
- `docs/standards-compliance-analysis/refactor-plan.md`

## Usage Examples
Run the focused moved coverage with:

```sh
cargo test -p pantograph-embedded-runtime workflow_runtime
```
