# task_executor_tests

## Purpose

This directory contains behavior-focused tests for the Pantograph host task
executor. The parent `../task_executor_tests.rs` file owns shared fixtures and
module indexing, while these modules keep Python runtime, dependency preflight,
input normalization, and Puma-Lib coverage below the large-file threshold.

## Contents

| File | Description |
| ---- | ----------- |
| `dependency_fallback.rs` | Environment-ref gate and local Python fallback behavior for dependency preflight. |
| `dependency_preflight.rs` | Blocking, successful model-ref resolution, diffusion routing, and ONNX routing through dependency preflight. |
| `input_helpers.rs` | Inference setting defaults, runtime environment id collection, stable hashing, and dependency request shaping. |
| `puma_lib.rs` | Puma-Lib model lookup and stale model-path rebinding coverage. |
| `recorder_stream.rs` | Python runtime recorder identity/health coverage and stream event replay behavior. |

## Problem

The host task-executor test file had grown into a mixed fixture and behavior
suite. That made Python adapter behavior, dependency preflight, input helper,
and Puma-Lib changes harder to review and kept the file above the source-size
ratchet.

## Constraints

- Tests must continue exercising `TauriTaskExecutor` behavior through the same
  public executor path used by workflow execution.
- Shared fixtures should stay in the parent test file unless they become a
  dedicated reusable test support boundary.
- Dependency fallback tests must remain separate from recorder/stream tests so
  preflight policy changes are not mixed with runtime health assertions.

## Decision

Keep fixture setup in `../task_executor_tests.rs` and split behavior coverage
into modules aligned with the production `task_executor/` decomposition. Each
module imports the shared parent test scope and covers one behavior family.

## Alternatives Rejected

- Leaving all tests in `task_executor_tests.rs`: rejected because the file
  exceeded the large-file threshold and mixed unrelated behavior families.
- Splitting fixtures into every test module: rejected because it would duplicate
  model dependency resolver and Python adapter setup.

## Invariants

- Shared resolver and Python adapter fixtures remain single-source in the parent
  test module.
- Dependency preflight tests cover both blocking and allowed execution paths.
- Recorder and stream tests continue asserting runtime identity, health, and
  buffered replay behavior.
- Puma-Lib tests remain tied to model metadata rebinding rather than dependency
  installation policy.

## Usage Examples

Run the focused host task-executor tests through the existing crate filter:

```bash
cargo test -p pantograph-embedded-runtime task_executor
```

Add new coverage to the module that matches the behavior family being asserted.

## Revisit Triggers

- A new host task-executor behavior family needs a dedicated test module.
- Shared fixtures become complex enough to justify a separate test support
  module.
- Production task-executor dispatch changes in a way that requires new
  cross-family integration coverage.

## Dependencies

**Internal:** `task_executor`, `python_runtime`, runtime extension keys, and
node-engine dependency contracts.

**External:** `tempfile`, `serde_json`, `tokio`.

## Related ADRs

- [../../../../../docs/standards-compliance-analysis/refactor-plan.md](../../../../../docs/standards-compliance-analysis/refactor-plan.md)
