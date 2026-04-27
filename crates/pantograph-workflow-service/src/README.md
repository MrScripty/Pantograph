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
| `workflow/` | Private workflow contracts, host traits, graph API methods, diagnostics ledger query methods, capability/preflight API methods, workflow run and session execution API methods, queue and lifecycle API methods, service configuration, request validation, I/O derivation, runtime preflight, and session-runtime helpers extracted from the main facade. |
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
Generic workflow run execution now lives behind the facade in the workflow run
API helper.
Service construction, capacity-limit configuration, diagnostics-provider setup,
diagnostics-ledger setup, and the session-store guard now live in the workflow
service configuration helper. The root workflow facade tests now live in
`workflow/tests.rs`; shared
test fixture families now live under `workflow/tests/fixtures/` and are
re-exported by `workflow/tests/fixtures.rs`. Scheduler snapshot facade coverage
now lives in `workflow/tests/scheduler_snapshot.rs`, while scheduler admission,
runtime-registry, and rebalance diagnostics coverage lives in
`workflow/tests/scheduler_snapshot_diagnostics.rs`,
and session queue item/admission coverage now lives in
`workflow/tests/session_queue.rs`. Workflow capability discovery and default
capability derivation coverage now lives in
`workflow/tests/workflow_capabilities.rs`. Workflow I/O discovery and validation
coverage now lives in `workflow/tests/workflow_io.rs`, and workflow preflight
coverage now lives in `workflow/tests/workflow_preflight.rs`. Runtime preflight
policy coverage now lives in `workflow/tests/runtime_preflight.rs`. Private
workflow run implementation coverage now lives in `workflow/tests/workflow_run.rs`.
Workflow DTO serialization and error-envelope coverage now lives in
`workflow/tests/contracts.rs`. Workflow session execution and retention-hint
coverage now lives in `workflow/tests/session_execution.rs`. Session and
runtime capacity limit/error coverage now lives in
`workflow/tests/session_capacity_limits.rs`, while runtime capacity rebalance
coverage lives in `workflow/tests/session_capacity.rs`.
Runtime capacity/admission wait coverage now lives in
`workflow/tests/session_admission.rs`. Session runtime preflight cache and
keep-alive preflight failure coverage now lives in
`workflow/tests/session_runtime_preflight.rs`. Session runtime loaded-state
invalidation coverage now lives in `workflow/tests/session_runtime_state.rs`.
Session stale cleanup, inspection, and stale cleanup worker coverage now lives in
`workflow/tests/session_stale_cleanup.rs`.
Validated workflow identity grammar now lives in `workflow/identity.rs` and is
used by workflow service validation plus saved graph persistence boundaries, so
Stage 01 versioning work has one stable id contract instead of filesystem
sanitization.
Session creation and queued session run methods now live behind the facade in
the workflow session execution API helper.
Session status, queue inspection, scheduler snapshot, cancellation, and
reprioritization methods now live behind the facade in the workflow session
queue API helper.
Stale cleanup, stale cleanup worker, keep-alive, and close-session methods now
live behind the facade in the workflow session lifecycle API helper.
Model/license usage diagnostics query methods now live behind the facade in the
workflow diagnostics API helper and delegate to `pantograph-diagnostics-ledger`
for storage and query semantics.

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
- Workflow identity validation is centralized through `WorkflowIdentity`; saved
  workflow names, execution requests, capabilities, I/O, preflight, and future
  version records must not use independent identity grammars.
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
- Service configuration API methods preserve constructor defaults, capacity
  bounds, and shared lock error mapping behind the facade.
- Workflow facade test coverage stays in the workflow helper directory so the
  production facade file remains small enough to review directly.
- Workflow run API methods preserve timeout cancellation, output validation,
  and runtime-readiness checks as backend-owned behavior.
- Session execution API methods preserve queue admission, runtime preflight,
  runtime load, and run finalization as one backend-owned workflow.
- Session queue API methods preserve the public facade while keeping direct
  scheduler-store access in the workflow session queue helper.
- Session lifecycle API methods preserve the public facade while keeping
  cleanup, keep-alive, close, and runtime unload side effects together.
- Diagnostics usage query API methods preserve the public facade while keeping
  durable ledger storage and retention semantics in
  `pantograph-diagnostics-ledger`.
- Diagnostics projection re-exports include typed I/O artifact retention state
  and retention summary records so adapter callers can preserve serialized
  service contracts without depending on private ledger modules.
- Diagnostics projection re-exports include run-list facet records so adapter
  callers can consume backend-owned comparison counts without depending on
  private ledger modules or sampled frontend pages.

## Revisit Triggers
- Public workflow DTOs need versioning rather than additive migration.
- Scheduler or diagnostics persistence becomes durable across app restarts.
- The `workflow.rs` facade decomposition requires public module changes.
- Runtime lifecycle supervision moves into a dedicated backend manager.

## Dependencies
**Internal:** `node-engine`, `workflow-nodes`, `pantograph-runtime-identity`,
`pantograph-diagnostics-ledger`, and sibling source modules in this crate.

**External:** `async-trait`, `serde`, `serde_json`, `thiserror`, `tokio`,
`uuid`, `chrono`, and `parking_lot`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `docs/adr/ADR-011-scheduler-only-workflow-execution.md`

## Usage Examples
```rust
use pantograph_workflow_service::{
    WorkflowExecutionSessionCreateRequest, WorkflowExecutionSessionRunRequest, WorkflowService,
};
```

## API Consumer Contract
- Inputs: public request DTOs, workflow ids, graph edit/session ids,
  host-trait implementations, runtime capabilities, and technical-fit override
  selections.
- Outputs: public response DTOs for session runs, capabilities, IO discovery,
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
