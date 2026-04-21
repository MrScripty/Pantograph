# crates/pantograph-workflow-service/src

Host-agnostic workflow service source boundary.

## Purpose
This directory owns Pantograph workflow application-service contracts and
orchestration entrypoints. It keeps workflow execution, graph mutation,
scheduler queues, runtime preflight, technical-fit request shaping, and trace
diagnostics reusable across Tauri, UniFFI, Rustler, and tests.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Public module exports for the workflow service crate. |
| `workflow.rs` | Public workflow facade exports, execution/session facade methods, and orchestration logic. |
| `workflow/` | Private workflow contracts, host traits, graph API methods, capability/preflight API methods, session queue API methods, request validation, I/O derivation, runtime preflight, and session-runtime helpers extracted from the main facade. |
| `scheduler/` | Backend-owned workflow-session queue/store contracts used by the workflow facade. |
| `trace/` | Workflow trace contracts, request validation, in-memory trace state, and runtime/scheduler snapshot merge helpers. |
| `graph/` | Graph DTOs and session-kind contracts shared by service operations. |
| `technical_fit.rs` | Technical-fit request/decision DTOs, normalization helpers, session context assembly, and runtime-preflight integration. |
| `capabilities.rs` | Shared workflow capability and validation utilities. |

## Problem
Workflow behavior crosses UI, runtime, diagnostics, and binding boundaries.
Without one host-agnostic source owner, adapters can drift on execution ids,
session affinity, graph edits, runtime readiness, queue policy, and trace
semantics.

## Constraints
- No transport-framework dependencies such as Tauri, UniFFI, Rustler, or UI
  packages.
- Host and runtime dependencies enter through traits and DTOs.
- Public response shapes are consumed by frontend stores and generated/native
  bindings.
- Runtime install/remove/status mutations remain outside this crate.
- Runtime preflight and scheduler decisions must stay backend-owned.

## Decision
Keep public workflow orchestration in this source directory. `workflow.rs`
remains the compatibility facade while cohesive contracts and internals move
into focused private modules. Adapters may translate transport payloads but
must delegate workflow decisions to this crate.
Session runtime preflight cache fingerprinting now lives with the
session-runtime helper that owns cache lookup and refresh behavior.
Session runtime loaded-state invalidation now lives with the same helper that
owns runtime load and unload transitions.
Graph edit-session and persistence methods now live behind the facade in the
workflow graph API helper.
Workflow capability, I/O discovery, and preflight methods now live behind the
facade in the workflow preflight API helper.
Session status, queue inspection, scheduler snapshot, cancellation, and
reprioritization methods now live behind the facade in the workflow session
queue API helper.

## Alternatives Rejected
- Keep workflow behavior in Tauri commands: rejected because native bindings
  and tests need the same backend behavior without desktop transport.
- Let frontend stores reconstruct graph mutation or diagnostics truth:
  rejected because backend-owned responses are the source of truth.
- Move runtime readiness into runtime adapters: rejected because preflight and
  runtime-not-ready semantics are workflow service contracts.

## Invariants
- Workflow execution/session identity is owned here and exposed through public
  DTOs.
- Edit-session graph mutations, including collapsed node group create,
  ungroup, and port-mapping changes, return backend-owned snapshots that
  adapters render directly.
- `workflow_get_io` exposes only nodes marked as input/output with
  `io_binding_origin == "client_session"`.
- Workflow execution never triggers runtime installation implicitly.
- Session runtime preflight cache keys include graph fingerprint, runtime
  capability fingerprint, and normalized technical-fit override selection.
- Host calls that load/unload runtimes occur outside session-store locks.
- Trace stores own canonical event timestamps, idempotent terminal replay, and
  retry/reset behavior for repeated execution ids.
- Scheduler snapshots omit execution attribution when identity is ambiguous.
- Session queue API methods preserve the public facade while keeping direct
  scheduler-store access in the workflow session queue helper.

## Revisit Triggers
- Public workflow DTOs need versioning rather than additive migration.
- Scheduler or diagnostics persistence becomes durable across app restarts.
- The `workflow.rs` facade decomposition requires public module changes.
- Runtime lifecycle supervision moves into a dedicated backend manager.

## Dependencies
**Internal:** `node-engine`, `workflow-nodes`, `pantograph-runtime-identity`,
and sibling source modules in this crate.

**External:** `async-trait`, `serde`, `serde_json`, `thiserror`, `tokio`,
`uuid`, `chrono`, and `parking_lot`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```rust
use pantograph_workflow_service::{WorkflowRunRequest, WorkflowService};
```

## API Consumer Contract
- Inputs: public request DTOs, workflow ids, graph edit/session ids,
  host-trait implementations, runtime capabilities, and technical-fit override
  selections.
- Outputs: public response DTOs for runs, capabilities, IO discovery,
  preflight, sessions, queues, graph mutations, traces, and diagnostics.
- Lifecycle: hosts create a service, call workflow/session operations, and
  explicitly close sessions; the service owns scheduler and graph-session state.
- Errors: invalid requests, missing workflows/sessions, runtime-not-ready
  conditions, cancellations, capacity exhaustion, and host failures surface as
  `WorkflowServiceError`.
- Versioning: DTO changes should be additive unless Tauri, frontend, UniFFI,
  Rustler, examples, and contract tests migrate together.

## Structured Producer Contract
- Stable fields: workflow responses, graph mutation responses, queue records,
  runtime issues, technical-fit decisions, trace snapshots, and scheduler
  diagnostics are machine-consumed.
- Defaults: omitted optional fields use service-defined defaults and must be
  covered by contract tests when observable.
- Enums and labels: workflow/session states, runtime readiness states, queue
  statuses, trace statuses, and issue categories carry behavior.
- Ordering: queue, trace, runtime issue, and diagnostics ordering are part of
  observable behavior where clients display or compare sequences.
- Compatibility: saved workflows, frontend stores, and binding consumers may
  depend on serialized field names and semantics across releases.
- Regeneration/migration: response-shape changes must update Tauri wire
  contracts, frontend stores, binding wrappers, examples, and contract tests in
  the same slice.

## Testing
```bash
cargo test -p pantograph-workflow-service
```

## Notes
- `workflow.rs` remains over the decomposition threshold and is tracked in the
  standards compliance plan.
