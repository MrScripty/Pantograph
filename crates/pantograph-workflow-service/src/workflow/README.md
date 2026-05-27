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
| `artifact_api.rs` | WorkflowService ArtifactStore facade methods for descriptor lookup, binary body reads, consume acknowledgement, policy updates, cleanup, and stats. |
| `attribution_api.rs` | Client/session/bucket facade methods plus workflow-version and presentation-revision resolution against the durable attribution store. |
| `artifact_contracts.rs` | ArtifactStore descriptor, lifecycle, policy, read, stream, consume, format-default, and conversion-attribution DTOs for binary-safe media payload handling. |
| `artifact_store.rs` | Backend ArtifactStore body ownership, private disk persistence, restart reconciliation, retention cleanup, and consume acknowledgement. |
| `contracts.rs` | Public workflow request/response/error DTO definitions re-exported by the parent facade. |
| `diagnostic_errors.rs` | Typed workflow error phase registry, scoped diagnostics recorder API, and durable error-event append helpers. |
| `execution_plan.rs` | Run-scoped workflow execution-plan DTOs produced by scheduler admission and consumed through embedded-runtime projection. |
| `graph_api.rs` | Graph edit-session, mutation, connection, persistence, and runtime snapshot facade methods. |
| `host.rs` | Host trait defaults and scheduler diagnostics provider contracts re-exported by the parent facade. |
| `identity.rs` | Validated workflow identity value object and grammar used by workflow submission and saved graph boundaries. |
| `io_contract.rs` | Workflow input/output surface derivation and host-response validation helpers. |
| `diagnostics_api.rs` | Diagnostics, scheduler timeline, scheduler estimate, run projection, I/O artifact, Library usage, trusted diagnostic event append, retention, and projection rebuild facade methods. |
| `media_capability_contracts.rs` | Backend-owned media format capability and managed redistributable category/status DTOs. |
| `non_runtime_task_adapter.rs` | Scheduler-task adapter conversion for executing one allowlisted non-runtime node-engine task from typed task templates and materialized scheduler task results. |
| `preflight_api.rs` | Workflow capability, I/O discovery, and preflight facade methods. |
| `execution_plan_model_ref.rs` | Parse-once selected Pumas model-ref value object for run execution plans, including raw model-id normalization and local-path/unsupported-URI rejection. |
| `execution_plan_selected_facts.rs` | Workflow-owned selected backend/runtime/device fact value objects for run execution plans. |
| `runtime_host_task_result_mapping.rs` | Focused mapping from validated runtime-host terminal responses into typed scheduler task results without exposing executable load targets. |
| `runtime_preflight.rs` | Runtime requirement matching, issue formatting, and preflight warning collection. |
| `session_execution_api.rs` | Workflow session creation and queued session run orchestration facade methods. |
| `session_io_artifacts.rs` | Retained workflow/session I/O artifact metadata and small text/JSON ArtifactStore materialization helpers for diagnostics-ledger projection. |
| `session_lifecycle_api.rs` | Workflow stale cleanup, stale cleanup worker, keep-alive, and close-session facade methods. |
| `session_queue_api.rs` | Workflow session status, queue inspection, scheduler snapshot, session-scoped queue controls, and first-pass GUI-admin queued-run cancel facade methods. |
| `session_runtime.rs` | Session runtime preflight cache checks, runtime-capability fingerprinting, runtime loaded-state invalidation, runtime loading, unload-candidate selection, and affinity refresh helpers. |
| `service_config.rs` | Workflow service construction, capacity-limit configuration, diagnostics-provider/media-conversion setup, and session-store guard helpers. |
| `task_binding_resolution.rs` | Dependency-to-input binding resolution from materialized scheduler task results into validated scheduler-admissible task intents. |
| `task_execution_classification.rs` | Single workflow-service boundary that maps immutable node type plus canonical node-contract facts into scheduler execution classes before orchestration or adapters choose a path. |
| `task_graph.rs` | Path-free workflow topology projection into run-scoped scheduler task graph DTOs, including dependency edges, canonical scheduler identifiers, execution class, optional schedulable task intents, concrete typed source-input and non-runtime task templates for the current allowlist, and typed projection diagnostics. |
| `task_graph_contracts.rs` | Public path-free scheduler task graph DTOs, execution-class enum, concrete typed source-input and non-runtime task-template enums, and projection diagnostic enums re-exported through the workflow facade. |
| `task_result_contracts.rs` | Public typed scheduler task-result DTOs used by task orchestration materialization before dependency binding resolution. |
| `task_state_read_model.rs` | Presentation-neutral scheduler task-state projection from immutable task graph facts plus validated task-state records for graph editor and run inspection consumers. |
| `tests/` | Behavior-focused workflow facade test modules split from the legacy monolithic test module. |
| `tests.rs` | Legacy workflow facade and scheduler/session behavior tests extracted from the root facade file. |
| `validation.rs` | Request, binding, output-target, and produced-output validation helpers shared by facade operations. |

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
session-runtime workflows, run-scoped execution-plan contracts, and the root
facade test module.

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
- Scheduler task-state read models project only path-free, presentation-neutral
  facts from immutable task graph definitions joined with canonical scheduler
  task-state records. They must not expose transition ids, state versions,
  runtime handoff, executable Pumas load targets, or worker launch details.
- Scheduler task-state read models may expose path-free execution category,
  non-runtime task kind, and scheduler state diagnostics so graph editor,
  run inspection, and diagnostics views can explain blocked or failed tasks
  without reading execution internals.
- Scheduler task-state queries are dedicated workflow-service read endpoints.
  They do not extend session queue items or scheduler snapshots with task
  internals, keeping queue admission facts separate from task progress facts.
- Scheduler task binding resolution consumes only materialized typed task
  results and path-free task intent templates. Missing, unavailable, invalid,
  or wrong-type upstream values must produce typed binding diagnostics instead
  of reading graph-local paths, reduced execution plans, node-engine demand
  state, runtime handoff, or Pumas load targets.
- Scheduler task graph projection excludes graph control-association edges such
  as `dependency_environment_sidecar` from runtime input bindings and
  dependency task ids. Those edges are interpreted by workflow-service
  validation/action owners, not by scheduler materialization.
- Scheduler task execution class is projected by
  `task_execution_classification.rs` from immutable node type plus canonical
  node-contract facts. Runtime inference, request-provided source inputs,
  non-runtime node-engine execution, Pumas materialization, and unsupported
  tasks must enter orchestration through this single boundary rather than
  through scattered node-type checks.
- Scheduler source-input task templates are concrete, typed task-definition
  facts for request-provided workflow inputs. Projection records the canonical
  output port shape, such as `text-input.text` or `boolean-input.value`, but it
  must not read graph-local frontend data for source values. Source-input tasks
  remain `AwaitingInputs` until the scheduler/session runner materializes the
  matching request payload into typed task results.
- Scheduler non-runtime task templates are concrete, typed task-definition
  facts for node-engine-executed tasks in the current allowlist only. Projection
  may build output-compatible templates such as `text-output`, but request
  source values belong to source-input templates. The scheduler-task entrypoint
  and node-engine adapter must consume only non-runtime templates plus
  materialized task results. User-authored or broader node support needs an
  explicit typed template variant or the later generic typed port-value
  contract; do not pass arbitrary JSON or frontend node data as an execution
  template.
- The non-runtime task adapter converts typed workflow-service templates and
  materialized task results into node-engine `single_task` requests, then
  converts node-engine outputs back into `WorkflowSchedulerTaskResult` values.
  It must reject runtime inference before calling node-engine and must not
  read graph data, call output demand, or execute Pumas/model-provider nodes.
- Runtime-host task-result mapping consumes only validated terminal
  runtime-host responses. It maps typed, path-free runtime-host outputs into
  `WorkflowSchedulerTaskResult` values and fails closed for accepted
  non-terminal responses or unsupported future runtime-host variants.
- Workflow diagnostics projection tests cover Library usage warm projection
  catching-up state so service callers preserve backend projection freshness
  instead of inferring it from raw ledger rows.
- Workflow Library asset access audit writes must enter through the diagnostics
  API helper, use diagnostics-ledger typed operation/cache-status enums, and
  remain optional when diagnostics storage is not configured.
- Trusted runtime adapters that already hold typed
  `DiagnosticEventAppendRequest` values must append them through the workflow
  diagnostics API helper so durable writes stay optional and ledger ownership
  remains in workflow-service.
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
- Durable diagnostics ledger writes for scheduler estimates, queue controls,
  admission, delay, and model lifecycle events occur outside session-store
  locks. Scheduler/session mutations should return or copy the immutable facts
  needed for event payloads before dropping the store guard.
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
- Artifact descriptor format metadata carries typed real-conversion status and
  per-conversion dependency lease attribution when conversion occurs. Pass-
  through artifactization leaves those fields empty; it must not synthesize
  lease ids from ambient active dependency snapshots.
- ArtifactStore body cleanup may remove payload files, read handles, and
  access modes, but descriptor format metadata remains queryable so conversion
  status, command identity, and dependency lease attribution survive retention
  or delete-on-consume body deletion.
- Workflow-service no longer owns host-injected media conversion execution.
  Scheduler-task output materialization must provide any future artifact
  conversion status, command identity, converted bytes, and dependency lease
  attribution through a dedicated typed boundary.
- Workflow-service does not acquire managed media leases, resolve executable
  paths, or spawn converter processes. Those remain host-owned concerns behind
  the injected executor boundary.
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
- Workflow run error diagnostics enter through the typed recorder in
  `diagnostic_errors.rs`. Call sites must use registered phase helpers and
  typed scope structs so ledger error events carry the required run, node,
  runtime/model, projection, or transport context consistently.
- Workflow execution-plan contracts stay run-scoped and scheduler-owned.
  `execution_plan.rs` records only reduced per-node selected backend, runtime,
  device, task, optional model ref, bounded diagnostics, and policy trace ids.
  It must not store graph inputs, raw node payloads, full Pumas facts, worker
  envelopes, image bytes, local filesystem paths, or scheduler internals.
- Workflow scheduler task graph projection is path-free. `task_graph.rs` may
  carry canonical `pumas_model_ref` values, graph input bindings, optional hard
  runtime/device constraints, and scheduler trait settings, but it must not
  read or preserve legacy `model_path`/`model_ref` identity or executable Pumas
  load targets.
- Workflow scheduler task results are typed materialization records, not
  execution launch records. `task_result_contracts.rs` may carry path-free
  model refs, scalar values, media/artifact refs, status, bounded diagnostics,
  and terminal metadata, but it must not carry local model paths, Pumas load
  targets, worker launch details, runtime handoff, or raw node-engine
  internals.
- Workflow execution-plan DTOs are correct-by-construction: public builders and
  serde deserialization validate schema version, attribution ids, node ids,
  selected task/device facts, bounded diagnostic vectors, and policy trace ids.
  Invalid or ambiguous execution-plan state must become typed errors or
  diagnostics before embedded-runtime projects it into node execution context.
- Workflow errors that have a recorded diagnostic event should return
  `WorkflowServiceError::with_diagnostics(...)` so Tauri envelopes and frontend
  pages can link directly to the run diagnostic. The wrapper must preserve the
  original error code, message, and details.
- Runtime-loading producers that know the specific failure family should attach
  `WorkflowRuntimeDiagnosticPhaseHint` before returning the service error.
  Session runtime admission uses that hint to choose the canonical
  `runtime_model_load`, `runtime_launch`, `model_dependency`, or
  `managed_binary` recorder phase without parsing error text.
- Failed terminal events should copy the diagnostic event id into
  `RunTerminalPayload::canonical_error_event_id` when the error provides one.
  The terminal event records state transition; `diagnostic.error_occurred`
  remains the detailed error fact.
- Failed node status events may carry
  `NodeExecutionStatusPayload::canonical_error_event_id` when node execution
  already knows the causal diagnostic event. Consumers should prefer that link
  for node-focused navigation while keeping `error_event_id` for direct
  node-scoped fatal diagnostic projections.
- Runtime/model load, unload, warmup, and scheduler trace timing must flow
  through `pantograph-timing-contracts` before local duration arithmetic is
  trusted. Attempt records carry explicit identity and attribution, and
  impossible timestamp state becomes typed diagnostics instead of saturated
  duration values.

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

**External:** inherited parent crate dependencies.

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
  files for historic run views. The projection also carries backend-owned
  `graph_diagnostics` derived from the reconstructed snapshot so run inspection
  exposes stale facts without frontend inference.
- Session run snapshot creation validates the current graph before queue
  admission. Blocking stale graph diagnostics return an invalid-request error
  envelope with structured graph details and do not enqueue or execute the run.
- Local Network status: `workflow_local_network_status_query` reports
  local-only system and scheduler-load facts through a provider abstraction.
  Scheduler-load facts include active and queued workflow run ids for local
  selected-run placement display. They include typed scheduler model-cache
  posture for not-required/unknown state, but they do not claim concrete
  model/cache residency.
  Run-placement records include the owning workflow execution session, run
  state, runtime-loaded posture, model-cache posture, and required
  backend/model facts so the GUI can show selected-run requirements without
  querying raw scheduler internals.
  Future peer records must extend the peer DTOs instead of changing local-node
  semantics.
- Retention policy updates: `workflow_retention_policy_update` changes the
  global standard diagnostics retention policy and records a typed
  `retention.policy_changed` audit event with `gui_admin` actor scope.
  Retention policy query responses include first-pass typed setting groups
  derived from the active standard policy for final outputs, workflow inputs,
  intermediate node I/O, failed-run data, size/storage limits, media behavior,
  compression behavior, and cleanup trigger.
- Retention cleanup: `workflow_retention_cleanup_apply` records typed
  `retention.artifact_state_changed` audit events with `gui_admin` actor scope
  before expired payload references disappear from the projection.
- I/O artifact queries expose typed retention state from the diagnostics
  projection. Callers must treat `retention_state` as authoritative instead of
  deriving payload state from `payload_ref`; retention cleanup updates arrive
  through typed ledger events and are materialized into the current artifact
  projection row. Query responses also include retention-state summary counts
  from the same materialized projection.
- Run inspection responses expose backend-owned `resolved_node_io` rows that
  distinguish produced outputs, graph-derived inputs, explicit input facts, and
  workflow-boundary facts. UI callers may decide layout and grouping, but they
  must not reconstruct durable node I/O semantics from graph edges and raw
  artifact rows when the resolved read model is available. Each row carries a
  `provenance_kind` so callers can label graph edges, workflow boundaries,
  explicit inputs, and future exception classes without parsing role strings.
- Workflow-session output aliases share payload identity across `node_output`
  and `workflow_output` facts. Artifact body storage is keyed by the payload
  artifact id; fact ids remain ledger/projection identities and must not be
  used as read targets.
- I/O artifact projection rows expose producer and consumer node/port endpoint
  fields separately from the event node id so workflow input/output metadata
  and future node-to-node I/O can share one query contract.
- Workflow-session I/O artifact events resolve event node type from immutable
  run-snapshot graph settings. They must not read current graph files after a
  run completes.
- I/O artifact query requests expose producer and consumer node filters. The
  workflow facade forwards those filters to diagnostics-ledger projection and
  retention-summary queries instead of filtering response pages locally.
- Run-list query responses include comparison facets from backend
  `run_list_projection` rows for workflow version, status, scheduler policy,
  retention policy, selected runtime, selected device, and selected network
  node.
- Run-list and run-detail projection records expose node-derived selected
  backend, model, and task rollups. The scheduler-owned selected runtime,
  device, and network-node fields keep their existing lifecycle semantics.
- Scheduler estimate queries expose the selected run's hot run-detail
  projection estimate fields, including typed scheduler model-cache posture,
  without making frontend callers parse full run detail or raw ledger payload
  rows.
- Run-detail query responses include matching node-status projection rows and
  node projection state so selected node task, backend, runtime, and model
  context remains available without parsing raw ledger payload rows.
- Queue cancel, reprioritize, and push-front commands emit typed scheduler
  queue-control events when diagnostics are configured. Accepted and denied
  outcomes must be recorded after the scheduler store makes the authority
  decision. Session-scoped commands emit `client_session` actor scope with the
  requested and effective session ids. The GUI-admin queued-run cancel,
  priority override, and push-front boundaries resolve the owning session
  through the scheduler store and emit `gui_admin` actor scope with the
  effective session id.
- Accepted reprioritize and push-front queue commands emit a fresh scheduler
  estimate for the updated queued run after the scheduler store has applied the
  mutation. Queue cancellation does not emit an estimate for the removed run.
- Workflow-session execution emits typed scheduler delay events for runtime
  admission waits when diagnostics ledger storage is configured. The event is
  recorded outside scheduler-store locks and is projected into run status,
  scheduler reason, and timeline rows by the diagnostics ledger.
- Workflow-session admission emits selected runtime and reserved model facts
  from the dequeued scheduler state into `scheduler.run_admitted` events. It
  must not reread current graph files or runtime internals to populate those
  audit fields.
- Workflow execution plans use schema version
  `WORKFLOW_EXECUTION_PLAN_SCHEMA_VERSION` and a stable `node_decisions` map
  keyed by node id. The serialized shape is append-only; consumers must ignore
  later additive fields they do not need, but producers must continue to supply
  validated workflow/run ids and explicit selected node-decision facts.
- Workflow execution-plan selected model refs are parsed once at the
  workflow-service owner boundary. Raw Pumas model ids serialize as canonical
  `pumas://models/...` refs, already-prefixed refs are preserved, and local
  paths or unsupported URI schemes are rejected before embedded-runtime
  projection, scheduler history, runtime readiness, or worker dispatch can
  trust the value.
- Workflow execution plans validate selected backend keys, runtime ids,
  runtime variant ids, and concrete selected device ids as workflow-owned
  values before node-engine or embedded-runtime projection can consume them.
  Invalid backend/runtime/device shapes fail at the execution-plan owner
  boundary as typed selected-decision fact errors instead of being repaired or
  rediscovered by adapters.
- Workflow-run error handling records canonical diagnostic errors where
  possible, but secondary diagnostic append failures must not replace the
  original workflow execution, timeout, output-validation, artifact-conversion,
  or terminal error. In those cases the returned error carries a
  `diagnostics_unavailable` link with no diagnostic event id.
- Workflow-session admission emits a local runtime-slot reservation-created
  event after the scheduler store admits a queued run, and emits the matching
  reservation-released event after `finish_run` clears the active scheduler
  state. These events use immutable snapshot/runtime requirement facts for
  selected runtime and reserved model fields.
- Workflow-session execution emits typed scheduler model/cache state on
  estimate and model lifecycle events when diagnostics ledger storage is
  configured. Submission estimates start as `unknown`; required-model load,
  unload, and failure transitions map to explicit scheduler cache states.
- Workflow-session scheduler estimates use immutable run-snapshot runtime
  requirements for first-pass confidence, required backend/model/extension
  reasons, estimated peak RAM/VRAM context, and typed candidate runtime ids
  from immutable runtime capabilities. They also emit typed blocking
  conditions for runtime admission, queue backlog, missing compatible runtime,
  and unknown model cache state where applicable. Future scheduler metadata may
  refine those estimates, but submission estimates must not read mutable graph
  files after the snapshot is created.
- Workflow capability preflight must not invent a required backend from the
  app's currently selected runtime when the graph lacks backend facts. Missing
  backend requirements stay empty until graph contracts, Pumas facts, or node
  data provide an explicit backend.
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
