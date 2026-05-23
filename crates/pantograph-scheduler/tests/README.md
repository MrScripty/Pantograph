# pantograph-scheduler tests

## Purpose
This directory contains public contract tests for Pantograph scheduler DTOs and
policy helpers. The tests prove that persisted and transported scheduler facts
decode, validate, reject legacy path-shaped success data, and preserve the
scheduler-owned boundary before runtime execution is wired.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `fixtures/` | Canonical JSON payloads and fixture producer contract documentation for machine-consumed scheduler contracts. |
| `schedulable_task_intent.rs` | Path-free task intent validation and legacy field rejection tests. |
| `capability_hint.rs` | Graph/editor capability hint availability and diagnostic validation tests. |
| `readiness_admission.rs` | Scheduler dependency readiness admission and policy mapping tests. |
| `queue_state.rs` | Durable queue-state validation, idempotent replay, legacy path rejection, and exhaustive transition matrix tests. |
| `dispatch_decision.rs` | Runtime/device/model/reservation/batch dispatch decision contract tests. |
| `runtime_handoff.rs` | Runtime-host handoff validation and path-free dispatch envelope tests. |
| `resource_residency.rs` | Platform-neutral resource observation and fit validation tests. |
| `batching_policy.rs` | Cross-workflow task compatibility and batch rejection tests. |
| `task_lifecycle.rs` | Backend-owned task lifecycle diagnostic compatibility tests. |
| `lifecycle_supervision.rs` | Scheduler lifecycle owner and bounded component supervision tests. |

## Problem
The scheduler contracts are consumed across queue storage, graph/editor hints,
runtime handoff, diagnostics, and future host execution. Tests in this
directory keep those contracts explicit and catch accidental reintroduction of
`ModelRefV2`, graph-visible model paths, frontend `modelPath`, or executable
load-target data before it reaches implementation slices.

## Constraints
- Tests must use deterministic fixtures and local validation only.
- Tests must not require installed runtimes, local model libraries, Pumas
  storage paths, GPU availability, network access, or platform-specific
  resource collectors.
- Fixture changes are contract changes and must be reviewed with matching
  source and README updates.
- Successful scheduler paths must remain no-fallback: old dependency resolver
  output and path-shaped model identity are rejected, not adapted.

## Decision
Keep tests as focused Rust integration tests backed by JSON fixtures. This
matches the current project testing choice for this milestone and avoids adding
another JavaScript or browser test platform for scheduler Rust contracts.

## Alternatives Rejected
- Browser or Playwright tests for scheduler DTOs: rejected because these
  contracts are Rust serde/validation contracts, not UI behavior.
- Runtime smoke tests in this directory: rejected because runtime host
  execution belongs to later integration slices after scheduler decisions and
  Pumas load-target resolution are wired.
- Snapshot-only fixture tests: rejected because validation behavior and
  typed error cases are as important as fixture decoding.

## Invariants
- Every fixture represents a stable contract and must validate through the
  same public API exported to consumers.
- Every negative test must assert the typed `SchedulerContractError` shape, not
  an incidental string.
- Tests must keep scheduler policy separate from graph editor, node-engine,
  runtime adapter, and Pumas path resolution responsibilities.
- New public scheduler DTOs require at least one fixture-backed positive test
  and focused negative validation coverage.

## Revisit Triggers
- Scheduler contracts gain generated bindings or persisted migrations that
  require a fixture compatibility matrix.
- Platform-specific resource observers are implemented and need target-gated
  compile or smoke tests outside the pure contract suite.
- Runtime host execution is wired and needs cross-crate acceptance tests from
  task intent to scheduler dispatch to host-local Pumas load-target resolution.

## Dependencies
**Internal:** `pantograph-scheduler` public exports and JSON fixtures in this
directory.

**External:** `serde_json` for fixture decoding; no external services or
runtime binaries are required.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```bash
cargo test -p pantograph-scheduler
```

## API Consumer Contract
- Inputs: checked-in JSON fixtures and Rust DTO values built inside focused
  tests.
- Outputs: pass/fail validation of public scheduler contract behavior,
  including accepted canonical shapes and rejected legacy or malformed shapes.
- Lifecycle: tests are stateless and may run in any order.
- Errors: expected failures assert `SchedulerContractError` values directly.
- Compatibility: a fixture update must land with source, README, and plan
  updates that explain the contract change.

## Structured Producer Contract
- Stable fixture files live under `fixtures/` and use snake_case serde field
  names matching public scheduler DTOs.
- Fixture ordering is not a contract except where a DTO explicitly documents
  ordered semantics.
- Defaults are tested through omitted optional fields where omission is part of
  the public contract.
- New fixtures should be minimal but complete enough to validate one public
  contract boundary.
- Regenerate or edit fixtures only in slices that also update the corresponding
  tests and source documentation.
