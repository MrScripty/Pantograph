# crates/pantograph-runtime-host-contracts/src

## Purpose
This crate owns the shared runtime-host execution boundary between scheduler
orchestration and host runtime execution. It provides the serialized request
and response contracts, validation wrappers, typed errors, runtime-host port
trait, and scheduler dispatch helper.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Crate-level contract documentation and public re-exports. |
| `runtime_host_execution.rs` | Runtime-host execution request/response DTOs, diagnostics, validation, and typed contract errors. |
| `runtime_host_execution_tests.rs` | Fixture-backed runtime-host execution contract tests. |
| `runtime_host_dispatch.rs` | Runtime-host execution port trait, scheduler dispatcher, response correlation checks, and typed dispatch errors. |
| `runtime_host_dispatch_tests.rs` | Focused dispatcher tests using a fake runtime-host port. |
| `../tests/fixtures/` | Serialized request/response contract fixtures. |

## Invariants
- This crate is a boundary/contract crate only.
- It must not own scheduler policy, workflow orchestration, runtime loading,
  Pumas load-target resolution, node-engine execution, concrete I/O, spawned
  task lifecycle, or Tokio runtime creation.
- Runtime execution requests must contain an actual dispatch-selected
  `SchedulerRuntimeHandoff`.
- Responses must correlate to the request and scheduler handoff ids.
- Path-shaped legacy fields are rejected by serde contract validation rather
  than accepted as compatibility aliases.
