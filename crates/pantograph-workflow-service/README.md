# pantograph-workflow-service

Host-agnostic workflow application service for Pantograph.

## Purpose
This crate owns workflow API contracts, graph-edit sessions, scheduler queues,
runtime preflight, diagnostics traces, and host-facing workflow orchestration.
The crate boundary exists so Tauri, UniFFI, Rustler, and future hosts can
delegate to one backend service instead of each owning workflow behavior.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Crate manifest for the workflow application-service package. |
| `src/` | Workflow service source modules and source-level README. |
| `tests/` | Public and cross-crate behavior tests for service contracts. |

## Problem
Pantograph workflow behavior crosses UI, runtime, and binding boundaries.
Without a host-agnostic service crate, execution ids, graph mutations,
diagnostics, queue policy, and runtime preflight can drift between frontend,
Tauri, and generated binding surfaces.

## Constraints
- No Tauri, UniFFI, Rustler, or UI dependencies.
- Host/runtime capabilities enter through traits and request DTOs.
- Graph mutation and diagnostics contracts are consumed by frontend and
  bindings, so shape changes must be coordinated.
- Scheduler and runtime-preflight decisions must stay backend-owned.

## Decision
Keep workflow orchestration in a reusable Rust service crate. Transport
adapters decode boundary payloads and call this crate; runtime hosts implement
narrow traits. Large internal modules may be decomposed, but public facades
should be preserved until a breaking API change is explicitly accepted.

## Alternatives Rejected
- Keep workflow behavior in Tauri commands: rejected because native bindings
  and tests need the same behavior without desktop runtime.
- Let Svelte stores own graph mutation truth: rejected because backend-owned
  data and no-optimistic-update rules require server confirmation.
- Put scheduler queue policy in runtime adapters: rejected because queue
  behavior is part of workflow service semantics.

## Invariants
- Workflow execution/session identity is backend-owned.
- Graph mutation responses come from this service or lower backend crates, not
  from frontend reconstruction.
- Diagnostics snapshots and traces must preserve backend producer identity.
- Transport adapters may translate payloads but must not change workflow
  decisions.

## Revisit Triggers
- A public binding needs a workflow operation that cannot be represented by the
  current service contracts.
- Scheduler or diagnostics persistence becomes durable across app restarts.
- `workflow.rs` facade decomposition would require public API changes.

## Dependencies
**Internal:** `node-engine`, `workflow-nodes`, and
`pantograph-runtime-identity`.

**External:** `async-trait`, `serde`, `serde_json`, `thiserror`, `tokio`,
`uuid`, `chrono`, and `parking_lot`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```rust
use pantograph_workflow_service::workflow::WorkflowRunRequest;
```

## API Consumer Contract
- Inputs: workflow run/session/graph/diagnostics request DTOs and host trait
  implementations.
- Outputs: response DTOs, graph mutation responses, diagnostics snapshots,
  queue state, and typed workflow service errors.
- Lifecycle: hosts create service dependencies and close sessions explicitly;
  the service owns scheduler and graph session semantics.
- Errors: invalid requests, runtime failures, cancellation, and host errors are
  represented as workflow service errors for adapters to project.
- Versioning: public DTOs should change additively unless all adapters and
  bindings migrate together.

## Structured Producer Contract
- Stable fields: workflow responses, diagnostics snapshots, graph mutation
  responses, queue records, and trace payloads are machine-consumed.
- Defaults: omitted optional request fields use service-defined defaults.
- Enums and labels: workflow states, runtime issue categories, and queue states
  carry behavior, not only display text.
- Ordering: queue and trace ordering are part of observable service behavior.
- Compatibility: saved workflows and generated bindings may consume these
  shapes across releases.
- Regeneration/migration: response-shape changes must update Tauri wire
  contracts, frontend stores, binding wrappers, and tests in the same slice.

## Testing
```bash
cargo test -p pantograph-workflow-service
```

## Notes
- `src/workflow.rs` and related workflow modules remain over the decomposition
  threshold and are tracked by the standards compliance plan.
