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
| `attribution_api.rs` | Client/session/bucket facade methods plus workflow-version and presentation-revision resolution against the durable attribution store. |
| `contracts.rs` | Public workflow request/response/error DTO definitions re-exported by the parent facade. |
| `graph_api.rs` | Graph edit-session, mutation, connection, persistence, and runtime snapshot facade methods. |
| `host.rs` | Host trait defaults and scheduler diagnostics provider contracts re-exported by the parent facade. |
| `identity.rs` | Validated workflow identity value object and grammar used by workflow submission and saved graph boundaries. |
| `io_contract.rs` | Workflow input/output surface derivation and host-response validation helpers. |
| `diagnostics_api.rs` | Diagnostics, scheduler timeline, scheduler estimate, run projection, I/O artifact, Library usage, retention, and projection rebuild facade methods. |
| `preflight_api.rs` | Workflow capability, I/O discovery, and preflight facade methods. |
| `runtime_preflight.rs` | Runtime requirement matching, issue formatting, and preflight warning collection. |
| `session_execution_api.rs` | Workflow session creation and queued session run orchestration facade methods. |
| `session_lifecycle_api.rs` | Workflow stale cleanup, stale cleanup worker, keep-alive, and close-session facade methods. |
| `session_queue_api.rs` | Workflow session status, queue inspection, scheduler snapshot, session-scoped queue controls, and first-pass GUI-admin queued-run cancel facade methods. |
| `session_runtime.rs` | Session runtime preflight cache checks, runtime-capability fingerprinting, runtime loaded-state invalidation, runtime loading, unload-candidate selection, and affinity refresh helpers. |
| `service_config.rs` | Workflow service construction, capacity-limit configuration, diagnostics-provider setup, and session-store guard helpers. |
| `tests/` | Behavior-focused workflow facade test modules split from the legacy monolithic test module. |
| `tests.rs` | Legacy workflow facade and scheduler/session behavior tests extracted from the root facade file. |
| `validation.rs` | Request, binding, output-target, and produced-output validation helpers shared by facade operations. |
| `workflow_run_api.rs` | Private scheduler-owned workflow run internals, run timeout handling, output validation, and session-run handoff. |

## Problem
`src/workflow.rs` remains a large public facade with service methods. Public
DTO definitions, graph edit-session APIs, capability/preflight APIs,
host/runtime trait defaults, workflow I/O derivation, runtime readiness,
request validation, and session-runtime loading are cohesive enough to isolate,
but they still preserve the parent facade as the compatibility export point.

## Constraints
- Preserve the public `WorkflowService` API while decomposing internals, except
  where the canonical workflow-run identity contract intentionally removes
  direct scheduler-bypass execution surfaces.
- Keep runtime capability matching deterministic.
- Keep scheduler capacity and session runtime decisions backend-owned.
- Avoid introducing adapter-specific types into service internals.

## Decision
Use this directory for workflow-service helper modules behind the parent
facade. The parent facade remains the public export point while helpers own
cohesive contract definitions, host/runtime trait defaults, request
validation, graph edit-session methods, capability/preflight methods, session
execution methods, session queue inspection methods, session lifecycle methods,
service configuration methods, diagnostics projection and audit helpers,
workflow run execution, workflow I/O derivation, runtime readiness,
session-runtime workflows, and the root facade test module.

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
  under `workflow/tests/`, including capacity, stale cleanup, and worker
  lifecycle coverage.
- Session capacity limit/error coverage is separate from runtime rebalance
  coverage so scheduler-bound capacity assertions stay focused.
- Scheduler snapshot shape coverage is separate from scheduler diagnostics
  projection coverage.
- Workflow diagnostics projection tests cover Library usage warm projection
  catching-up state so service callers preserve backend projection freshness
  instead of inferring it from raw ledger rows.
- Workflow Library asset access audit writes must enter through the diagnostics
  API helper, use diagnostics-ledger typed operation/cache-status enums, and
  remain optional when diagnostics storage is not configured.
- Workflow Library usage queries accept `workflow_run_id` filters for
  active-run Library views and delegate that filtering to diagnostics-ledger
  projections.
- Workflow run-list projection queries accept client, client-session, bucket,
  and accepted-at range filters and delegate that filtering to
  diagnostics-ledger projections.
- Workflow retention cleanup applies through a typed diagnostics facade command
  that delegates to the ledger cleanup policy and returns backend cleanup
  counts without client-side artifact mutation.
- Shared workflow facade test fixtures live under `workflow/tests/fixtures/`
  and are re-exported by `workflow/tests/fixtures.rs`, keeping
  `workflow/tests.rs` as the module index for behavior slices.
- Session runtime loaded-state invalidation tests live with the
  session-runtime-state behavior slice.
- Workflow capability discovery tests live with the workflow-capabilities
  behavior slice.
- Host calls occur outside session-store locks.
- Generic workflow run execution owns timeout cancellation, output validation,
  and runtime-not-ready checks behind private scheduler/session handoff.
- Public workflow-run request DTOs must not accept caller-authored `run_id`
  fields. The backend scheduler creates `workflow_run_id` exactly once for a
  submitted run, and response DTOs expose that id as `workflow_run_id`.
- Workflow ids accepted at service or saved-graph boundaries must parse through
  `WorkflowIdentity`; callers receive explicit invalid-request errors instead
  of filesystem name sanitization or whitespace-only acceptance.
- Workflow run handles use the same constructor for explicit and default
  creation so cancellation state starts from one backend-owned shape.
- Session execution APIs keep queue admission, runtime preflight, runtime load,
  and run finalization in one helper behind the public facade.
- Session run submission generates the backend workflow run id before enqueue
  and, when attribution storage is configured, records the immutable workflow
  version/run snapshot and emits a `run.snapshot_accepted` event with the node
  behavior-version set and workflow execution-session id before handing the
  run to scheduler admission.
- Session run I/O artifact events use diagnostics-ledger typed artifact roles
  for workflow inputs and outputs. Workflow-service should pass role enums to
  the ledger and use string labels only for deterministic artifact ids.
- Session run Library audit events use diagnostics-ledger typed operation and
  cache-status enums. Workflow-service must not author free-form Library action
  labels when emitting run-linked model usage facts.
- Attributed session creation validates the caller credential, client session,
  and bucket through `pantograph-runtime-attribution`; queued run snapshots and
  scheduler/run diagnostic events inherit those validated ids instead of
  trusting caller-authored client fields.
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
- Workflow-session runtime admission, run-triggered capacity rebalances, and
  ephemeral teardown emit scheduler model lifecycle events for required models
  through the diagnostics ledger when configured. These events use
  preflight/cache model and backend facts instead of raw host internals.
- Workflow version resolution validates `WorkflowIdentity`, computes
  `WorkflowExecutableTopology`, and persists semantic-version/fingerprint
  agreement through the attribution store.
- Workflow presentation revision resolution validates `WorkflowIdentity`,
  computes `WorkflowPresentationMetadata`, and persists display-metadata
  fingerprint agreement through the attribution store without changing workflow
  execution identity.

## Revisit Triggers
- Runtime preflight becomes a public reusable crate-level policy.
- Session lifecycle supervision moves to a dedicated backend runtime manager.
- Workflow I/O schema handling needs to support a second bindable-origin model.
- `workflow.rs` facade decomposition exposes these helpers through a narrower
  public module structure.
- Remaining `workflow/tests.rs` behavior areas need extraction into
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
- Snapshotting: queued workflow execution sessions require an explicit
  `workflow_semantic_version` and use it when resolving the immutable workflow
  version and presentation revision snapshot.
- Snapshotting: queued run snapshots also capture backend-derived graph
  settings, runtime requirements, capability model inventory, and runtime
  capabilities before scheduler admission.
- Snapshotting: `run.snapshot_accepted` diagnostic events include
  `node_versions` entries with node id, node type, contract version, and
  behavior digest so event consumers can audit the node-version set without
  reading the current graph.
- Presentation revisions: display metadata is resolved through the attribution
  facade after workflow-version resolution; callers must keep the returned
  presentation revision id separate from workflow-version ids in diagnostics
  queries.
- Historic run graphs: graph lookup by workflow run id reconstructs a
  `WorkflowGraph` from the immutable run snapshot, workflow executable
  topology, and presentation revision records. It must not read current graph
  files for historic run views.
- Local Network status: `workflow_local_network_status_query` reports
  local-only system and scheduler-load facts through a provider abstraction.
  Scheduler-load facts include active and queued workflow run ids for local
  selected-run placement display, but they do not claim model/cache residency.
  Run-placement records include the owning workflow execution session, run
  state, runtime-loaded posture, and required backend/model facts so the GUI can
  show selected-run requirements without querying raw scheduler internals.
  Future peer records must extend the peer DTOs instead of changing local-node
  semantics.
- Retention policy updates: `workflow_retention_policy_update` changes the
  global standard diagnostics retention policy and records a typed
  `retention.policy_changed` audit event with `gui_admin` actor scope.
- Retention cleanup: `workflow_retention_cleanup_apply` records typed
  `retention.artifact_state_changed` audit events with `gui_admin` actor scope
  before expired payload references disappear from the projection.
- I/O artifact queries expose typed retention state from the diagnostics
  projection. Callers must treat `retention_state` as authoritative instead of
  deriving payload state from `payload_ref`; retention cleanup updates arrive
  through typed ledger events and are materialized into the current artifact
  projection row. Query responses also include retention-state summary counts
  from the same materialized projection.
- I/O artifact projection rows expose producer and consumer node/port endpoint
  fields separately from the event node id so workflow input/output metadata
  and future node-to-node I/O can share one query contract.
- I/O artifact query requests expose producer and consumer node filters. The
  workflow facade forwards those filters to diagnostics-ledger projection and
  retention-summary queries instead of filtering response pages locally.
- Run-list query responses include comparison facets from backend
  `run_list_projection` rows for workflow version, status, scheduler policy,
  and retention policy.
- Scheduler estimate queries expose the selected run's hot run-detail
  projection estimate fields without making frontend callers parse full run
  detail or raw ledger payload rows.
- Queue cancel, reprioritize, and push-front commands emit typed scheduler
  queue-control events when diagnostics are configured. Accepted and denied
  outcomes must be recorded after the scheduler store makes the authority
  decision. Session-scoped commands emit `client_session` actor scope. The
  GUI-admin queued-run cancel, priority override, and push-front boundaries
  resolve the owning session through the scheduler store and emit `gui_admin`
  actor scope.
- Workflow-session execution emits typed scheduler delay events for runtime
  admission waits when diagnostics ledger storage is configured. The event is
  recorded outside scheduler-store locks and is projected into run status,
  scheduler reason, and timeline rows by the diagnostics ledger.
- Diagnostics: usage diagnostics accept workflow-version and node contract
  version/digest filters so historic comparisons can avoid mixing different
  executable node behavior.
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
cargo test -p pantograph-workflow-service workflow::tests::contracts
cargo test -p pantograph-workflow-service runtime_preflight
cargo test -p pantograph-workflow-service workflow::tests::runtime_preflight
cargo test -p pantograph-workflow-service workflow::tests::session_admission
cargo test -p pantograph-workflow-service workflow::tests::session_capacity
cargo test -p pantograph-workflow-service workflow::tests::session_execution
cargo test -p pantograph-workflow-service session_runtime
cargo test -p pantograph-workflow-service workflow_io
cargo test -p pantograph-workflow-service workflow_get_io
cargo test -p pantograph-workflow-service workflow_preflight
cargo test -p pantograph-workflow-service workflow::tests::workflow_run
cargo test -p pantograph-workflow-service workflow_get_scheduler_snapshot
cargo test -p pantograph-workflow-service workflow_session_queue
cargo test -p pantograph-workflow-service workflow_version
```

## Notes
- This directory is part of the staged decomposition of `workflow.rs`; keep new
  helper modules focused and re-exported through the facade unless an explicit
  public module API is accepted.
