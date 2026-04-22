# pantograph-workflow-service/src/workflow

Workflow contract, runtime-readiness, and session-runtime helper modules.

## Purpose
This directory holds focused helpers extracted from the main workflow service
facade. These modules define host-facing workflow contracts, evaluate runtime
preflight readiness, and coordinate session runtime loading without moving
public exports out of the service crate.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `contracts.rs` | Public workflow request/response/error DTO definitions re-exported by the parent facade. |
| `graph_api.rs` | Graph edit-session, mutation, connection, persistence, and runtime snapshot facade methods. |
| `host.rs` | Host trait defaults and scheduler diagnostics provider contracts re-exported by the parent facade. |
| `io_contract.rs` | Workflow input/output surface derivation and host-response validation helpers. |
| `preflight_api.rs` | Workflow capability, I/O discovery, and preflight facade methods. |
| `runtime_preflight.rs` | Runtime requirement matching, issue formatting, and preflight warning collection. |
| `session_execution_api.rs` | Workflow session creation and queued session run orchestration facade methods. |
| `session_lifecycle_api.rs` | Workflow stale cleanup, stale cleanup worker, keep-alive, and close-session facade methods. |
| `session_queue_api.rs` | Workflow session status, queue inspection, scheduler snapshot, cancel, and reprioritize facade methods. |
| `session_runtime.rs` | Session runtime preflight cache checks, runtime-capability fingerprinting, runtime loaded-state invalidation, runtime loading, unload-candidate selection, and affinity refresh helpers. |
| `service_config.rs` | Workflow service construction, capacity-limit configuration, diagnostics-provider setup, and session-store guard helpers. |
| `tests/` | Behavior-focused workflow facade test modules split from the legacy monolithic test module. |
| `tests.rs` | Legacy workflow facade and scheduler/session behavior tests extracted from the root facade file. |
| `validation.rs` | Request, binding, output-target, and produced-output validation helpers shared by facade operations. |
| `workflow_run_api.rs` | Generic workflow run facade, run timeout handling, output validation, and internal session-run handoff. |

## Problem
`src/workflow.rs` remains a large public facade with service methods. Public
DTO definitions, graph edit-session APIs, capability/preflight APIs,
host/runtime trait defaults, workflow I/O derivation, runtime readiness,
request validation, and session-runtime loading are cohesive enough to isolate,
but they still preserve the parent facade as the compatibility export point.

## Constraints
- Preserve the public `WorkflowService` API while decomposing internals.
- Keep runtime capability matching deterministic.
- Keep scheduler capacity and session runtime decisions backend-owned.
- Avoid introducing adapter-specific types into service internals.

## Decision
Use this directory for workflow-service helper modules behind the parent
facade. The parent facade remains the public export point while helpers own
cohesive contract definitions, host/runtime trait defaults, request
validation, graph edit-session methods, capability/preflight methods, session
execution methods, session queue inspection methods, session lifecycle methods,
service configuration methods, workflow run execution, workflow I/O derivation,
runtime readiness, session-runtime workflows, and the root facade test module.

## Alternatives Rejected
- Leave all helpers in `workflow.rs`: rejected because runtime readiness and
  session loading are large enough to obscure the public facade.
- Move runtime preflight into adapters: rejected because runtime readiness is a
  service contract consumed by multiple hosts.
- Move session runtime loading into scheduler modules: rejected because the
  logic coordinates host runtime calls and session-store state together.

## Invariants
- Runtime matching uses canonical backend keys from
  `pantograph-runtime-identity`.
- Runtime warning and blocking-issue lists remain deterministic and deduped.
- Service configuration owns constructor defaults, loaded runtime capacity
  bounds, and the shared session-store lock error mapping.
- Workflow facade tests live outside `workflow.rs` so production facade imports
  and service shape remain reviewable; behavior-specific test modules live
  under `workflow/tests/`.
- Host calls occur outside session-store locks.
- Generic workflow run execution owns timeout cancellation, output validation,
  and direct runtime-not-ready checks behind the public facade.
- Workflow run handles use the same constructor for explicit and default
  creation so cancellation state starts from one backend-owned shape.
- Session execution APIs keep queue admission, runtime preflight, runtime load,
  and run finalization in one helper behind the public facade.
- Session lifecycle APIs keep cleanup, keep-alive, and close-session behavior
  together so runtime unload side effects remain visible in one helper.
- Session queue inspection and scheduler snapshot APIs stay behind the public
  facade while delegating their store access to the session queue helper.
- Session runtime preflight cache fingerprints are derived in the
  session-runtime helper that consumes them.
- Session runtime loaded-state invalidation stays with the session-runtime
  helper that owns load-state transitions.
- Session runtime loaded state is updated only after host load/unload calls
  succeed or return a service error.

## Revisit Triggers
- Runtime preflight becomes a public reusable crate-level policy.
- Session lifecycle supervision moves to a dedicated backend runtime manager.
- Workflow I/O schema handling needs to support a second bindable-origin model.
- `workflow.rs` facade decomposition exposes these helpers through a narrower
  public module structure.
- More `workflow/tests.rs` behavior areas need extraction into
  `workflow/tests/` modules after production facade decomposition is complete.

## Dependencies
**Internal:** parent workflow facade exports, scheduler queue and preflight
cache contracts, technical-fit overrides, host trait helpers, and
`pantograph-runtime-identity`.

**External:** none beyond parent crate dependencies.

Reason: helper modules inherit the parent crate dependency surface so extracted
workflow internals do not grow new package-level coupling.

Revisit trigger: add a direct external dependency here only when a helper owns a
stable reusable policy that cannot remain behind the parent facade.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
These helpers are reached through the workflow service facade:

```rust
service.ensure_session_runtime_loaded(host, session_id).await?;
```

## API Consumer Contract
- Inputs: workflow runtime requirements, runtime capability DTOs, session ids,
  workflow ids, and host trait methods.
- Outputs: request/response DTOs, bindable I/O node surfaces, runtime issues,
  scheduler diagnostics contracts, preflight cache records, and service errors
  consumed by public workflow operations.
- Lifecycle: helpers run inside public workflow/session operations and do not
  own long-lived runtime resources directly.
- Errors: capacity exhaustion, missing sessions, runtime-not-ready conditions,
  and host failures are returned as `WorkflowServiceError`.
- Versioning: helper behavior is private, but its observable responses are part
  of the public workflow service contract.

## Structured Producer Contract
- Stable fields: bindable I/O node ids, port ids, runtime issue messages,
  runtime ids, required backend keys, and preflight cache facts flow into public
  response DTOs.
- Defaults: blank required backend keys are ignored during matching.
- Validation: blank workflow ids, empty binding endpoints, duplicate endpoints,
  invalid output targets, oversized values, and missing produced outputs keep
  the same error codes as the parent facade.
- Enums and labels: runtime install/readiness states retain the parent service
  contract semantics.
- Ordering: runtime issues are sorted and deduplicated before public exposure.
- Ordering: bindable workflow I/O nodes and ports are sorted before public
  exposure.
- Compatibility: changing matching or issue formatting can affect frontend,
  adapter, and binding consumers.
- Regeneration/migration: update public contract tests, frontend runtime
  diagnostics, adapters, and this README when observable behavior changes.

## Testing
```bash
cargo test -p pantograph-workflow-service runtime_preflight
cargo test -p pantograph-workflow-service workflow::tests::runtime_preflight
cargo test -p pantograph-workflow-service session_runtime
cargo test -p pantograph-workflow-service workflow_io
cargo test -p pantograph-workflow-service workflow_get_io
cargo test -p pantograph-workflow-service workflow_preflight
cargo test -p pantograph-workflow-service workflow_get_scheduler_snapshot
cargo test -p pantograph-workflow-service workflow_session_queue
```

## Notes
- This directory is part of the staged decomposition of `workflow.rs`; keep new
  helper modules focused and re-exported through the facade unless an explicit
  public module API is accepted.
