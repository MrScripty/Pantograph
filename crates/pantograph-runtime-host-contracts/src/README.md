# crates/pantograph-runtime-host-contracts/src

## Purpose
This crate owns the shared runtime-host execution boundary between scheduler
orchestration and host runtime execution. It provides the serialized request
and response contracts, validation wrappers, typed errors, runtime-host port
trait, scheduler dispatch helper, and the reservation lifecycle contract used
to release or retain runtime-registry leases acquired for dispatch candidates.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Crate-level contract documentation and public re-exports. |
| `reservation_lifecycle.rs` | Runtime dispatch reservation lifecycle event/application DTOs, validation wrappers, lifecycle port trait, and typed contract/port errors. |
| `reservation_lifecycle_tests.rs` | Fixture-backed reservation lifecycle contract tests. |
| `runtime_host_execution.rs` | Runtime-host execution request/response DTOs, typed materialized input values, typed output values, diagnostics, validation, and typed contract errors. |
| `runtime_host_execution_tests.rs` | Fixture-backed runtime-host execution contract tests. |
| `runtime_host_dispatch.rs` | Runtime-host execution port trait, scheduler dispatcher, response correlation checks, and typed dispatch errors. |
| `runtime_host_dispatch_tests.rs` | Focused dispatcher tests using a fake runtime-host port. |
| `runtime_session_load.rs` | Path-free backend-owned runtime session load-proof DTO, readiness state, diagnostic phase, validation wrapper, and typed contract errors. |
| `runtime_session_load_tests.rs` | Runtime session load-proof contract tests for validation, path-field rejection, and ready-state invariants. |
| `../tests/fixtures/` | Serialized request/response contract fixtures. |

## Invariants
- This crate is a boundary/contract crate only.
- It must not own scheduler policy, workflow orchestration, runtime loading,
  Pumas load-target resolution, node-engine execution, concrete I/O, spawned
  task lifecycle, or Tokio runtime creation.
- Runtime execution requests must contain an actual dispatch-selected
  `SchedulerRuntimeHandoff`.
- Runtime execution requests must explicitly carry materialized, path-free input
  values. The request contract rejects missing `materialized_inputs` and bounds
  the number and size of values; runtime-specific validation can still decide
  whether an explicit empty input set is valid for a task family.
- Responses must correlate to the request and scheduler handoff ids.
- Response outputs must be typed, bounded, and path-free. They are runtime-host
  contract values that workflow-service maps into scheduler task results; this
  crate must not depend on workflow-service DTOs.
- Runtime session load proofs carry workflow/task correlation, backend/runtime
  identity, model/artifact/load-target identity, readiness state, diagnostic
  phase, and requested-model-active state without exposing executable paths.
  Ready proofs must explicitly mark the requested model active.
- Reservation lifecycle events carry scheduler lease ids, task/run
  correlation, typed outcomes, optional candidate ids, diagnostics, and
  idempotency event ids. Workflow-service emits these events, but the
  embedded-runtime infrastructure owner implements runtime-registry release,
  retention, and reclaim/reconcile side effects.
- Reservation lifecycle events must not carry graph paths, `ModelRefV2`,
  reduced execution-plan fields, runtime-host load targets, Pumas load
  targets, or provider-private source facts.
- Path-shaped legacy fields are rejected by serde contract validation rather
  than accepted as compatibility aliases.
