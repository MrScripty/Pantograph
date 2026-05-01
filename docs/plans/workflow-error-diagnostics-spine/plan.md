# Plan: Workflow Error Diagnostics Spine

## Objective

Make workflow-run errors durable, ordered, explicit diagnostics facts so every
error on the workflow run path is visible in the ledger, projected into the GUI,
and deep-linkable from the graph editor or other surfaces that show the error.

## Scope

### In Scope

- Add a first-class diagnostics ledger event for workflow errors with stable
  phase, component, severity, code, location, and related-event metadata.
- Record error events at workflow submission, queue/admission, preflight,
  dependency resolution, model load, runtime launch, node execution, artifact,
  projection, and transport boundaries where a run context is available.
- Preserve existing `run.terminal`, scheduler lifecycle, runtime snapshot, and
  node trace events while linking them to the canonical error event.
- Project error events into run detail, run list, scheduler timeline, node
  status, and run graph diagnostics without requiring frontend-local repair.
- Make fatal run-scoped error events sufficient to mark a run or node failed in
  projections when no later recovery event supersedes them, while keeping the
  scheduler/session store as the primary owner of live run state.
- Add domain failure handling so fatal workflow errors explicitly transition
  scheduler/session state out of `Running` before diagnostics projections are
  refreshed.
- Return structured error envelopes to the graph editor and other callers with
  `workflow_run_id`, `diagnostic_event_id`, phase, component, and node/runtime
  location when available.
- Add diagnostics GUI affordances for obvious error visibility, filtering,
  color treatment, focused event navigation, and graph/node highlighting.
- Harden diagnostics ledger appends so hostile error text, large stderr, and
  serialization edge cases produce bounded ledger events instead of alternate
  duplicate diagnostics files.
- Surface explicit `diagnostics_unavailable` command/projection state when the
  primary diagnostics ledger cannot accept an event.
- Update module READMEs and tests for changed contracts.

### Out of Scope

- Replacing the existing diagnostics ledger with a different event store.
- Redesigning scheduler admission policy, runtime selection policy, or model
  dependency resolution policy.
- Changing how llama.cpp, Ollama, PyTorch, or Puma-Lib load models except to
  wrap and report their errors with richer diagnostics context.
- Adding remote telemetry or sending diagnostics outside local app data.
- Adding duplicate local JSONL/event-log fallback storage for workflow-run
  diagnostics.
- Backfilling historical runs that already lost their original error detail.
- Building a broad analytics dashboard beyond error surfacing and trace
  navigation.

### Deferred Design Improvements

These concerns are acknowledged but should not block the diagnostics spine
implementation unless current code cannot preserve enough context for the new
recorder API.

- Typed llama.cpp sidecar startup errors for spawn failure, managed command
  resolution, health-check timeout, process termination, OOM, and ready-but-
  wrong-model states.
- Broader inference backend error taxonomy across llama.cpp, Ollama, PyTorch,
  Candle, external runtime connections, and future backends.
- Deeper node engine error taxonomy beyond the execution-boundary wrapper,
  provided user-authored nodes remain able to return ordinary errors without
  diagnostics boilerplate.
- Generated Rust-to-TypeScript diagnostics DTOs. This plan requires fixture or
  contract tests for drift, but type generation can be a follow-up.
- Stronger static dependency/import enforcement beyond initial module
  visibility, review rules, scripts, and tests.
- Extracting the generic diagnostics registry/recorder into a cross-app package
  after Pantograph proves the API shape.

Current implementation must still create clean seams for these follow-ups:
avoid string parsing when structured context is available, preserve original
errors until the recorder can classify them, and keep diagnostics policies in
typed registry/recorder modules rather than scattered call-site logic.

## Inputs

### Problem

Workflow errors can currently surface as graph-editor submit errors while the
run remains projected as `Running`, or they can appear only as an error string
inside an existing event. The ledger records some failure states, but there is
no single canonical error event that tells operators exactly what failed, where
it failed, when it happened in event order, and how to navigate from the UI
error to the run diagnostics that explain it.

The recent `run_f856e7d0-9c60-4597-b063-1dd8c19367f7` investigation showed a
representative failure mode: a runtime error containing control characters
prevented terminal diagnostics from being written, so the GUI reported a submit
failure while the durable run projection stayed `Running`. A targeted fix now
sanitizes terminal and scheduler lifecycle error strings, but the broader
system still needs first-class error traceability.

### Constraints

- Backend Rust remains the source of truth for workflow run identity, run
  status, scheduler/runtime state, node status, and durable diagnostics.
- The diagnostics ledger is not the workflow state machine. Fatal error
  handling must update scheduler/session state through workflow-service domain
  control paths, then record diagnostics facts.
- Frontend code must render backend projections declaratively and must not
  infer or repair business state locally.
- Error text and structured details must be sanitized and bounded before they
  enter the ledger or transport envelopes.
- Diagnostic event IDs and workflow run IDs must be generated by backend code
  and must be stable enough for frontend deep links.
- Ledger writes occur in async workflow paths; durable recording must avoid
  blocking the runtime, must not hold locks across unrelated work, and must not
  introduce unowned background tasks.
- Existing query and projection APIs must remain facade-compatible until new
  fields are available; additive DTO fields are preferred over breaking
  replacements.
- Touched code must follow
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.
- Existing unrelated dirty files must remain untouched unless explicitly
  assigned to this work.

### Assumptions

- A new `diagnostic.error_occurred` ledger event is acceptable as an additive
  persisted schema change.
- The existing diagnostics ledger event sequence remains the canonical ordering
  source for run diagnostics.
- Existing run terminal and node failed events should remain for compatibility
  but can reference the new canonical error event.
- The diagnostics ledger remains the only durable workflow-run diagnostics
  trace. This plan intentionally does not add JSONL fallback persistence.
- If the ledger is unavailable, callers receive a clear
  `diagnostics_unavailable` error/link state instead of a duplicate local trace.
- Deep links can be represented in the current frontend navigation state before
  a full route-based URL system exists.

### Dependencies

- `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `schema.rs`, `sqlite/event_sqlite.rs`, `repository.rs`, `lib.rs`, and tests:
  durable error event contract and persistence.
- `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `diagnostics_api.rs`, `session_queue_api.rs`, `session_runtime.rs`,
  `trace/`, and workflow tests: workflow-run error capture and projections.
- `crates/pantograph-embedded-runtime/src/embedded_workflow_host.rs`,
  `embedded_workflow_host_helpers.rs`, `task_executor.rs`,
  `model_dependency_descriptors.rs`, `runtime_registry*`, and node execution
  ledger adapters: runtime, model, and node execution error context.
- `src-tauri/src/workflow/headless_workflow_commands.rs`,
  `headless_diagnostics*.rs`, `diagnostics/`, and app setup wiring: transport
  envelopes, diagnostics-unavailable reporting, and GUI projection transport.
- `src/services/diagnostics/types.ts`,
  `src/services/workflow/workflowServiceErrors.ts`,
  `src/services/workflow/WorkflowCommandService.ts`, and related tests:
  frontend DTO and command error parsing.
- `src/components/workbench/DiagnosticsPage.svelte`,
  `diagnosticsPagePresenters.ts`, `RunGraphSnapshot.svelte`,
  `runGraphPresenters.ts`, and tests: user-visible error rendering and focused
  event navigation.
- `src/stores/workbenchStore.ts` and `src/stores/schedulerRunListStore.ts`:
  workbench page selection and run-list error badges.

### Affected Structured Contracts

- Diagnostics ledger payload enum gains `DiagnosticErrorOccurred`.
- Diagnostics error registry contract gains typed phase definitions, scope
  definitions, allowed source components, default severity/recoverability,
  causality policy, required context fields, and projection effect.
- Diagnostics SQLite schema gains any columns or projection tables required to
  query error events efficiently without parsing free-form payloads.
- Workflow error envelope gains optional diagnostics link fields:
  `workflow_run_id`, `diagnostic_event_id`, `node_id`, `runtime_id`,
  `model_id`, `phase`, `source_component`, `severity`, and
  `caused_by_event_id` when the producer knows the exact prior event that
  caused the surfaced error.
- Workflow error envelope and/or projection DTOs gain a typed
  `diagnostics_unavailable` indicator for cases where the primary diagnostics
  ledger cannot record a run error.
- Run detail/list projections gain error summary fields, latest fatal error,
  error counts, and focused event identifiers.
- Scheduler timeline projection gains error styling metadata and related event
  references.
- Secondary lifecycle/status events that are direct consequences of a canonical
  diagnostic error gain an optional `canonical_error_event_id` or equivalent
  typed link back to that error event.
- Frontend TypeScript mirrors gain the same additive fields and must preserve
  unknown future enum values safely where existing patterns support that.

### Affected Persisted Artifacts

- Current-version diagnostics ledger namespace/file and event payload shape.
- Breaking diagnostics ledger schema changes create a new ledger version instead
  of migrating old trace data into the new shape.
- Old diagnostics ledgers are retained or cleaned up by retention policy only;
  active code must not append new events to old ledger versions.
- Existing workflow JSON files are not migrated by this plan.

### Ownership and Lifecycle Notes

- Workflow-service owns run-scoped error event creation for scheduler, session,
  queue, terminal, artifact, and projection failures.
- Workflow-service and scheduler/session state owners own live run-state
  transitions. Diagnostics recording must not be the trigger that mutates
  scheduler state.
- Embedded-runtime owns runtime, managed-binary, model dependency, and node
  execution error context before returning errors to workflow-service.
- Tauri command wrappers own transport-boundary error envelopes and explicit
  diagnostics-unavailable reporting if backend service construction or command
  dispatch fails before workflow-service has a run context.
- Frontend workbench owns only presentation state: selected diagnostics page,
  focused event ID, filters, and graph highlight state. It does not mutate
  backend diagnostics status.
- No new polling loop is planned. UI updates continue to use explicit
  projection refreshes and existing workbench state changes.
- No fallback diagnostics writer is planned. If implementation discovers a
  genuine need for a separate preservation sink, work stops and a new standards
  review is required before adding it.

### Public Facade Preservation

- Keep existing Tauri command names and workflow service query names stable.
- Add fields to existing DTOs instead of replacing response shapes.
- Keep `run.terminal`, scheduler model lifecycle, and node failed event
  payloads readable by existing consumers. New `diagnostic.error_occurred`
  events become the canonical detailed error source but not the only failure
  signal.
- Existing frontend callers that only read `message` from error envelopes must
  keep working.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Error recording itself fails and hides the original failure | High | Sanitize/truncate before validation, harden ledger append paths, surface `diagnostics_unavailable` when the primary ledger cannot record the event, and do not create duplicate JSON traces. |
| Duplicate error events make traces noisy | Medium | Define one owner per phase and link secondary failure events with `related_error_event_id` instead of re-emitting full errors. |
| Fatal error events are mistaken for the live scheduler state transition mechanism | High | Workflow-service domain failure handling must explicitly mark scheduler/session state failed; diagnostics events record the fact and projections provide a read-model safety net. |
| Fatal error events mark recoverable failures as failed in read models | Medium | Add `severity`, `recoverability`, and projection rules that only fatal run-scoped errors drive failed projection state. |
| Schema changes break existing projections | High | Non-breaking additive changes may stay within the current ledger version only when old readers/writers remain correct. Breaking ledger changes require a new ledger version/namespace and projection rebuild tests within that version, not mixed-version migration. |
| UI color-only error indicators fail accessibility | Medium | Pair color with icons/text, accessible labels, focus states, and keyboard navigation tests. |
| Runtime process output exceeds ledger limits | Medium | Bound technical detail length and place oversized detail behind payload refs only if the existing artifact policy supports it. |
| Cross-layer changes become too broad for review | Medium | Implement in small milestones with atomic commits and verification per slice. |

## Clarifying Questions

- None required before planning.
- Reason: The user requirement is specific enough to define backend-owned
  durable error events, GUI visibility, and diagnostics deep links.
- Revisit trigger: If the implementation must support a route system or
  external telemetry target that does not exist today.

## Definition of Done

- Every workflow-run error path covered by this plan records a sanitized,
  durable, ordered error event in the diagnostics ledger or reports
  `diagnostics_unavailable` explicitly when the primary ledger cannot accept
  the event.
- Error events identify what failed, where it failed, when it happened, and
  which run/node/runtime/model they relate to when that context exists.
- Fatal run-scoped error events cause run projections to show `Failed` without
  relying solely on terminal event emission.
- Diagnostics GUI clearly marks error and warning events, exposes an error
  filter/list, and focuses a specific event from a graph editor error.
- Graph editor and workflow command errors include clickable diagnostics links
  when a run/error event exists.
- Backend, frontend, and projection tests cover submit, model-load, node
  execution, projection, and transport failure paths.
- Touched README files document the new contracts and source-of-truth rules.
- Verification commands pass or any blocked verification is recorded with a
  blocker and follow-up.

## Milestones

### Milestone 1: Error Event Contract

**Goal:** Add the durable, typed error event contract without changing runtime
behavior.

**Tasks:**
- [x] Add `DiagnosticErrorOccurredPayload` with phase, component, severity,
  error code, message, technical message, cause chain, recoverability,
  location fields, optional related event IDs, and an optional
  `caused_by_event_id` that may only be set from direct producer knowledge.
- [x] Add `DiagnosticEventKind::DiagnosticErrorOccurred` and update
  `DiagnosticEventSourceComponent` support deliberately. The current source
  enum is closed to scheduler, workflow-service, runtime, node-execution,
  retention, Library, and local-observer components, so new component labels
  must either be added with DB serialization tests or represented as typed
  phase/location fields inside the error payload.
- [x] Update `validate_event_scope` and `validate_event_source` so run-scoped,
  node-scoped, runtime-scoped, and transport-boundary error events have
  explicit allowed source rules instead of bypassing existing ledger validation.
- [x] Add validation helpers that sanitize and bound error text before ledger
  validation rejects it.
- [x] Add SQLite serialization, deserialization, timeline summary/detail, and
  ledger schema/version identity support.
- [x] Export the contract through the diagnostics ledger facade and update
  `crates/pantograph-diagnostics-ledger/src/README.md`.

**Verification:**
- `cargo test -p pantograph-diagnostics-ledger diagnostic_error`
- `cargo test -p pantograph-diagnostics-ledger sqlite`
- Version-bound replay tests proving current-version events read correctly and
  incompatible old-version ledgers are not mixed into current projections.

**Status:** Complete on 2026-05-01. Implemented the typed
`diagnostic.error_occurred` ledger payload, source/scope validation, bounded
sanitization helper, SQLite JSON round-trip support, scheduler timeline
projection visibility, crate facade exports, README documentation, and ledger
tests. The existing `schema_version` column remains the version identity for
this additive event-kind contract; no physical ledger version bump was needed
because the stored table shape did not change.

Decomposition review: `event.rs` already owns the typed diagnostic event
payloads, validation, and source/scope rules, and `sqlite/event_sqlite.rs`
already owns scheduler timeline projection event selection. Extracting only the
new error payload and one timeline branch would create a parallel local pattern
without reducing the reviewed behavioral surface. Keep the addition local for
Milestone 1 and revisit a broader ledger module split only if later milestones
add more projection or payload families.

Verification completed:
- `cargo check -p pantograph-diagnostics-ledger`
- `cargo fmt -p pantograph-diagnostics-ledger`
- `cargo test -p pantograph-diagnostics-ledger`

### Milestone 2: Backend Error Recorder Facade

**Goal:** Provide one standards-compliant backend helper for emitting
run-scoped error diagnostics through the primary ledger.

**Tasks:**
- [x] Add a typed diagnostics error registry that defines supported phases,
  required scope kind, allowed source components, default severity,
  default recoverability, causality policy, and projection effect.
- [x] Add a workflow-service error recorder that accepts typed context and
  returns the appended diagnostic event ID when available.
- [x] Shape the recorder API so common call sites use phase-specific helpers or
  builders such as `model_load_failed(scope, &err)`,
  `runtime_launch_failed(scope, &err)`, `node_execution_failed(scope, &err)`,
  `projection_failed(scope, &err)`, and `transport_failed(scope, &err)`.
- [x] Add typed scope structs for run, node, runtime/model, scheduler,
  artifact, projection, and transport error contexts. Call sites pass scope
  values rather than arbitrary maps or free-form field sets.
- [x] Add an explicit `.caused_by(event_id)` builder step for translated errors
  that have direct knowledge of a prior canonical diagnostic event. The default
  path leaves `caused_by_event_id` unset.
- [x] Add explicit diagnostics-unavailable mapping for ledger append failure or
  command failure before service wiring is available.
- [x] Add error-context builders for workflow run, scheduler, runtime, node,
  model dependency, managed binary, artifact, projection, and transport phases.
- [x] Ensure helper logic is sync-core/async-shell: pure shaping is sync,
  storage calls remain at existing I/O boundaries.
- [x] Update workflow-service and Tauri diagnostics READMEs for ownership and
  diagnostics-unavailable behavior.

**Verification:**
- Unit tests for sanitization, truncation, event ID propagation, and
  diagnostics-unavailable envelope shaping.
- API ergonomics tests or compile-focused examples for model load, runtime
  launch, node execution, projection failure, and transport failure that prove
  call sites do not hand-build diagnostic payloads.
- Registry matrix tests for every phase covering required scope fields, allowed
  source components, default severity/recoverability, causality policy, and
  projection effect.
- Integration tests for ledger append failure surfacing
  `diagnostics_unavailable` without creating duplicate local diagnostics
  files.
- `cargo check -p pantograph-workflow-service`
- `cargo check --manifest-path src-tauri/Cargo.toml`

**Status:** Complete on 2026-05-01. Added the focused
`workflow/diagnostic_errors.rs` recorder module with a typed phase registry,
scope structs, phase-specific builder methods, direct-causality builder,
diagnostics-unavailable outcomes, workflow-service README notes, and Tauri
diagnostics ownership notes. Expanded the builder and registry set for run
snapshot, scheduler admission, runtime preflight, runtime model load, runtime
launch, model dependency, managed binary, node execution, output validation,
run timeout, artifact, projection, and transport failure phases before wiring
workflow catch sites.

Verification completed for this slice:
- `cargo check -p pantograph-workflow-service`
- `cargo test -p pantograph-workflow-service workflow_diagnostic_error`
- `cargo check --manifest-path src-tauri/Cargo.toml` passed with existing
  dead-code warnings in Tauri workflow modules.

### Current Error Catch-Site Fit Review

**Status:** Reviewed against the current codebase before implementation.

The existing workflow error handling can move into the proposed diagnostics
design, but several paths currently flatten typed failure context before the
diagnostics layer can classify it. Implementation must preserve typed context
until the recorder has produced the canonical `diagnostic.error_occurred`
event.

| Current location | Current behavior | Planned phase/scope | Fit and required adjustment |
| ---------------- | ---------------- | ------------------- | --------------------------- |
| `workflow/session_execution_api.rs` submission validation and session lookup | Returns `WorkflowServiceError` before a run ID exists. | Transport or scheduler/session scope, not run scope. | Fits if these remain non-run command errors. Do not invent a workflow-run event before `workflow_run_id` exists. Envelope may carry no diagnostics event ID. |
| `workflow/session_execution_api.rs` queued snapshot creation | Creates `workflow_run_id`, then may fail while building attribution/run snapshot before enqueue. | `run_snapshot` with `RunErrorScope`. | Fits well. Record a fatal run-scoped diagnostic before returning and link the command envelope to it. |
| `workflow/session_execution_api.rs` scheduler estimate, queue placement, admission, reservation, and terminal event writes | Ledger append failures currently become `WorkflowServiceError` and can cancel or fail the request. | `diagnostics_unavailable` boundary plus existing scheduler lifecycle events. | Needs adjustment. A failure to write diagnostics must not replace the original workflow failure with an unrelated ledger failure. Surface typed `diagnostics_unavailable` while preserving the original command/runtime error. |
| `workflow/session_execution_api.rs` preflight failure after admission | Finishes run, writes `run.terminal`, releases reservation, and returns the preflight error. | `runtime_preflight_failed` with `SchedulerErrorScope` or `RuntimeModelErrorScope`. | Fits. Record canonical diagnostic first, then terminal/release events should link with `canonical_error_event_id`. |
| `workflow/session_execution_api.rs` runtime load failure after admission | Writes scheduler model lifecycle `LoadFailed`, finishes run, terminal event, release reservation. | `model_load_failed` or `runtime_launch_failed` with `RuntimeModelErrorScope`. | Fits, but `LoadDependencyResolved` must not be displayed as model-loaded. Only true backend readiness/model-match proof can drive loaded wording. |
| `workflow/workflow_run_api.rs` timeout branch | Cancels the run handle and returns `RuntimeTimeout`. | `run_timeout` with `RunErrorScope`. | Fits. Recorder should emit fatal run-scoped diagnostic before terminal failure. |
| `workflow/workflow_run_api.rs` output validation and output-target failures | Returns `OutputNotProduced`, `CapabilityViolation`, or `Internal` after host execution. | `output_validation_failed` or `artifact_conversion_failed` with run/node scope where available. | Fits. Output-target failures can use requested node/port context; zero-output internal failures are run-scoped unless node context is available. |
| `embedded_workflow_host_helpers.rs` runtime readiness, Puma-Lib model path, backend switch, gateway start, model-match check | Converts Puma-Lib, managed runtime, gateway, and model mismatch failures into `RuntimeNotReady(String)`. | `model_dependency_failed`, `runtime_launch_failed`, `managed_binary_not_ready`, `model_load_failed`. | Needs typed preservation. Add structured embedded-runtime error/context or recorder helper inputs before converting to `WorkflowServiceError`. |
| `embedded_workflow_host_helpers.rs` Puma-Lib execution descriptor lookup warning | Logs warning and falls back to model path when possible. | Recoverable `model_dependency_warning` if fallback succeeds; fatal `model_dependency_failed` if no model path remains. | Fits only if warning diagnostics are supported by the registry. Do not turn recoverable fallback warnings into run-fatal errors. |
| `embedded_workflow_host.rs` `executor.demand` failure | Converts `WaitingForInput` to `InvalidRequest`; all other node engine errors become `Internal(String)`. | `node_execution_failed` with `NodeErrorScope`. | Needs better node context. The catch site should use node/task identifiers from `NodeEngineError` or node diagnostics recorder events before flattening. If only the demanded output node is known, record that uncertainty explicitly. |
| `inference/src/gateway.rs` backend start failure | Stores lifecycle `last_error` and returns `GatewayError::Backend(error)`. | `runtime_launch_failed` or `model_load_failed` with runtime/model scope. | Fits. This is a good source for runtime lifecycle timing and should feed technical detail without being treated as scheduler state. |
| `inference/src/server.rs` llama.cpp sidecar startup | Returns `String` errors for spawn, health timeout, OOM, process error, termination, and readiness failure. | `llamacpp_sidecar_start_failed` under runtime/model scope. | Fits, but string-only errors reduce classification quality. Prefer a typed llama.cpp sidecar start error or structured context before converting to display text. |
| `inference/src/managed_binaries.rs` managed runtime status/command resolution | Returns structured `ManagedBinaryFacadeError`. | `managed_binary_not_ready` or `managed_binary_command_resolution_failed`. | Fits well. Preserve key, selected version, install root, missing files, readiness state, and unavailable reason in the diagnostic scope/details. |
| `workflow/diagnostics_api.rs` projection drain/query/rebuild failures | Converts ledger/projection failures to `WorkflowServiceError`. | `projection_failed` with `ProjectionErrorScope`. | Fits as diagnostics-system errors, not workflow-run causality unless a specific run projection request has a run ID. Must not mutate scheduler state. |
| `src-tauri/src/workflow/headless_workflow_commands.rs` command wrappers | Convert backend errors to JSON strings through `to_envelope_json`; `build_runtime(...)` failures are plain `String`. | Transport failure or envelope link propagation. | Needs envelope expansion. Backend `WorkflowServiceError` must carry diagnostics link fields; pre-service construction failures need explicit transport-scope handling or `diagnostics_unavailable`. |
| `src/services/workflow/workflowServiceErrors.ts` frontend normalization | Parses code/message/details and treats non-envelope failures as `transport_error`. | Frontend diagnostics link consumption. | Fits after DTO expansion. Parser must expose typed diagnostics link fields and keep non-envelope failures transport-scoped. |

**Design Fit Conclusion:** The new diagnostics design can cover the current
catch sites without changing its core architecture. The implementation blocker
is not the event model; it is preserving typed context long enough for the
recorder to classify failures before existing code converts them to strings or
generic `WorkflowServiceError` variants.

### Milestone 3: Workflow Path Capture

**Goal:** Emit canonical error events at every workflow run boundary where
errors can be returned or swallowed.

**Tasks:**
- [x] Wrap workflow submission/session creation errors that have run context.
- [x] Wrap scheduler queue/admission/preflight/runtime-load failures.
  Runtime-load and runtime-preflight failure capture are implemented; scheduler
  queue and admission diagnostics handoff failures are wrapped when a run ID
  exists.
- [x] Wrap model dependency resolution and Puma-Lib descriptor lookup failures.
  Embedded-runtime now preserves producer-known model dependency phase hints
  for Puma-Lib model lookup, Puma-Lib list, model directory read, and GGUF
  discovery failures before workflow-service records the canonical event.
  Puma-Lib execution descriptor fallback warnings are intentionally not emitted
  as fatal diagnostics because they are recoverable fallback logs.
- [x] Wrap managed runtime command resolution and process spawn/startup
  failures for llama.cpp, Ollama, and PyTorch. Runtime admission now selects
  `managed_binary`, `runtime_launch`, `model_dependency`, or
  `runtime_model_load` from a typed `WorkflowRuntimeDiagnosticPhaseHint`
  instead of parsing error text. Embedded llama.cpp model startup now marks
  gateway switch/start failures as `runtime_launch`; Tauri managed runtime
  command-resolution failures are tagged at the inference `ProcessSpawner`
  boundary and mapped to managed-binary backend failures before workflow-service
  records them. PyTorch currently enters through the Python sidecar node
  execution boundary, so its process launch failures are captured as node
  execution diagnostics rather than managed-binary runtime-load diagnostics.
- [x] Wrap node execution failures and attach node IDs/types and output port
  context where available.
- [x] Capture node execution diagnostics at the node execution/injection
  boundary, not inside user-authored node code. User nodes may return ordinary
  errors or optional typed node errors; the host wrapper owns workflow/run,
  node, attempt, injected capability, runtime/model, and port context.
- [x] Wrap artifact read/write/conversion and diagnostics projection failures.
  Workflow output artifactization failures, attribution-backed artifact-store
  read/write/stream failures, and diagnostics projection drain/query/rebuild
  failures are wrapped.
- [x] Add a workflow-service domain failure path that marks scheduler/session
  state failed when a fatal run-scoped workflow error occurs. This path owns
  live state transition before diagnostics projections refresh.
- [ ] Link `run.terminal`, scheduler lifecycle `*_failed`, node failed, and
  runtime snapshot error events to the canonical error event with a typed
  `canonical_error_event_id` or equivalent link. `run.terminal` now carries the
  canonical diagnostic error event ID when the returned workflow error has a
  diagnostics link. Scheduler model lifecycle load failures now carry the same
  canonical diagnostic error event ID. Node failed and runtime snapshot payload
  links remain.

**Verification:**
- Workflow-service tests for submit, queue/admission, preflight, model-load,
  and terminal failure phases.
- Scheduler/session tests proving fatal workflow errors remove runs from
  `Running` state through domain control flow, not by reacting to ledger appends.
- Embedded-runtime tests for runtime launch/model dependency/node execution
  context.
- Node execution tests proving plain user-node errors still produce scoped
  `node_execution_failed` diagnostics without node authors importing ledger or
  recorder APIs.
- Tauri command tests for transport envelope diagnostics fields.
- Regression that control characters and large runtime stderr still produce
  error events and failed projections.

**Status:** In progress as of 2026-05-01. Runtime-load failures after
scheduler admission now record a canonical `diagnostic.error_occurred` event
with runtime/model scope before the existing failed terminal and reservation
release events are emitted. Added a failing runtime-load workflow-service test
that proves the canonical error is sanitized, visible in the ledger, and the
run projection reaches `Failed`. Host execution errors, workflow run timeouts,
requested-output validation failures, invalid host output bindings, and output
artifactization failures now route through the recorder from
`workflow_run_internal`. Runtime preflight failures after admission now record a
`runtime_preflight_failed` diagnostic before terminal/reservation release
handling and carry the diagnostics link on the returned error. Failed
`run.terminal` payloads now preserve the canonical diagnostic error event ID
from linked workflow errors. Runtime-loading errors can now carry a typed
diagnostic phase hint so workflow-service records model dependency, managed
binary, runtime launch, or model-load failures under the registered phase that
the producer selected. Queued run snapshot failures and scheduler
queue/admission diagnostics handoff failures now record canonical run-scoped
or scheduler-scoped diagnostics before returning the service error.
Attribution-backed artifact-store write, open, read, append, stream-read, and
finalize failures now record canonical artifact diagnostics and return linked
workflow errors when the artifact request or descriptor carries workflow run
context.
Runtime-preflight and runtime-load fatal paths now use an explicit
workflow-service helper to finish the admitted run in the session store before
terminal/reservation diagnostics are emitted. The runtime-load regression
asserts the session leaves `Running`, returns to `IdleUnloaded`, and advances
the run count.
Scheduler model lifecycle failure payloads now expose
`canonical_error_event_id`, and runtime-load failure handling records the
canonical error before the failed lifecycle event so the lifecycle event can
link back to the authoritative diagnostic error.

Verification completed for this slice:
- `cargo check -p pantograph-workflow-service`
- `cargo test -p pantograph-workflow-service workflow_execution_session`
- `cargo test -p pantograph-workflow-service workflow_run_returns_output_not_produced_when_target_missing`
- `cargo test -p pantograph-workflow-service workflow_execution_session_runtime_load_failure_records_canonical_error`
- `cargo check -p pantograph-workflow-service`
- `cargo check -p pantograph-embedded-runtime`
- `cargo test -p pantograph-workflow-service workflow_service_error_preserves_runtime_diagnostic_phase_hint`
- `cargo test -p pantograph-workflow-service workflow_execution_session_runtime_load_failure_uses_phase_hint`
- `cargo test -p inference map_sidecar_start_error_preserves_managed_binary_failures`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test -p pantograph-workflow-service workflow_execution_session_run_snapshot_failure_records_canonical_error`
- `cargo test -p pantograph-workflow-service workflow_artifact_api_records_write_failure_with_run_context`
- `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_appends_error_events_and_projects_timeline`

### Milestone 4: Projection And Query Semantics

**Goal:** Make error events queryable and ensure fatal events drive run/node
failure projections.

**Tasks:**
- [x] Extend run list/detail projections with latest error summary, error
  counts, fatal error ID, and warning count.
- [x] Extend scheduler timeline projection with error severity, phase, and
  related event references.
- [x] Extend run graph/node projections so node-scoped fatal errors mark the
  node failed and expose the error event ID.
- [x] Add projection replay rules for old streams that only have terminal or
  node failed events.
  Fatal `diagnostic.error_occurred` events now drive run list/detail status to
  `Failed` when no later terminal event supersedes them. Legacy terminal and
  node failed replay rules are covered by focused projection tests.
- [x] Add a typed projection operation wrapper for drains, queries, and rebuilds
  so projection code can record `projection_failed` diagnostics without
  hand-built payloads or scheduler-state side effects.
- [x] Add query filters for errors and warnings without frontend-side event
  parsing.

**Verification:**
- Ledger replay tests from mixed old/new event streams.
- Workflow-service diagnostics query tests for run list, run detail, timeline,
  and graph projections.
- Recovery/idempotency tests proving duplicate projection replay does not
  duplicate error counts.

**Status:** In progress as of 2026-05-01. Added the first fatal-error
projection rule: run list/detail projections now treat fatal canonical error
events as failed run facts and preserve the sanitized error message in run
detail when no terminal event is present. Run list/detail projections now carry
latest error ID, severity, phase, code, message, fatal error ID, error count,
and warning count. Scheduler timeline rows now carry error severity, phase, and
related event IDs. Run list queries can filter by latest error severity and
phase. Node-status projections now treat node-scoped fatal error events as
failed node facts and expose the error event ID, severity, phase, and code.
Projection drain/query/rebuild failures now route through the workflow
diagnostic error recorder with projection name and operation context. Projection
diagnostic errors are diagnostics-only and may be global when the failed
projection operation does not have a specific workflow run. Remaining Milestone
4 work is complete. Legacy terminal failure replay remains covered by run-list
projection tests, and legacy node failed status replay now has dedicated
coverage proving old streams still mark nodes failed without canonical error
IDs.

Verification completed for this slice:
- `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_projects_fatal_error_as_failed_run`
- `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_appends_error_events_and_projects_timeline`
- `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger`
- `cargo test -p pantograph-workflow-service workflow_run_list_query`
- `cargo test -p pantograph-workflow-service --test contract run_projection_cross_layer_fixture_deserializes`
- `cargo test -p pantograph-workflow-service --test contract workflow_run_detail_query_contract_snapshot`
- `cargo test -p pantograph-workflow-service --test contract workflow_scheduler_timeline_query_contract_snapshot`
- `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_projects_node_fatal_error_as_failed_node`
- `cargo test -p pantograph-workflow-service --test contract workflow_node_status_query_contract_snapshot`
- `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_validates_error_scope_source_and_text`
- `cargo test -p pantograph-workflow-service workflow_diagnostic_error_recorder_appends_global_projection_error`
- `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_replays_legacy_node_failed_status`

### Milestone 5: Frontend Diagnostics Visibility

**Goal:** Make errors obvious in diagnostics without duplicating backend state.

**Tasks:**
- [x] Extend `src/services/diagnostics/types.ts` with additive error event and
  projection fields.
- [x] Update diagnostics presenters to classify severity, phase, component,
  and related event links from backend data.
- [x] Add an Errors filter/list and visual treatment for error and warning
  timeline events.
- [x] Update run list badges and run detail header to show fatal/latest error
  summaries.
- [x] Add graph/node badges and focused highlighting for node-scoped error
  event IDs.
- [x] Use semantic buttons/links with accessible names for filters, expanders,
  focused event navigation, and graph jump controls.

**Verification:**
- `npm run typecheck`
- `npm run build`
- Presenter unit tests for severity classification and deep-link focus.
- Component tests for accessible error filters, keyboard navigation, and
  non-color-only status indicators.

**Status:** In progress as of 2026-05-01. Added TypeScript mirrors for the
diagnostic error projection fields, severity/phase presenters, run-detail error
summary panels, node-scoped diagnostics rows, and non-color-only timeline
treatments. Focused frontend presenter and projection service tests pass along
with `npm run typecheck`. Added an explicit scheduler timeline Errors filter
and a compact error list driven by projected severity/phase fields. Remaining
work is the remaining interactive deep-link/focus controls covered by Milestone
6 and a full frontend build pass after those controls land. Diagnostics error
list actions now focus the selected canonical event through workbench
diagnostics state instead of only filtering the timeline, and the Svelte
accessibility gate passes.

### Milestone 6: Clickable Error Deep Links

**Goal:** Let graph editor and command errors navigate directly to the
diagnostics event that explains the failure.

**Tasks:**
- [x] Extend workflow command error parsing to preserve diagnostics link fields.
- [x] Update graph editor submit/save/run error surfaces to render clickable
  diagnostics actions when `workflow_run_id` and `diagnostic_event_id` exist.
- [x] Add workbench navigation state for selecting Diagnostics, loading the
  target run, focusing the event, and highlighting the related node if present.
- [x] Preserve plain text error messages for errors without diagnostics
  context.
- [x] Add tests that stale async error responses do not navigate away from a
  newer active workflow/run context.

**Verification:**
- Frontend command service tests for error envelope parsing.
- Workbench tests for diagnostics navigation/focus state.
- Manual smoke: submit a failing workflow, click the graph editor error, land
  on the focused diagnostics event.

**Status:** In progress as of 2026-05-01. Added optional diagnostics link
fields to workflow error envelopes, attached recorded diagnostic outcomes to
runtime-load and workflow-run failure returns, preserved link fields in frontend
error normalization, and added workbench diagnostics focus state. The workflow
submit error surface now renders a semantic Diagnostics action when a linked run
is available and the diagnostics page highlights the focused event/node. The
submit diagnostics action is now guarded by the workflow ID captured when the
submit started, so a stale async submit failure cannot navigate diagnostics
after the graph context changes. Focused Rust envelope tests, diagnostics
recorder tests, frontend error parsing tests, workbench store tests, the stale
toolbar regression test, and `npm run typecheck` pass. The graph editor submit
surface now renders the clickable action for linked errors; save surfaces do
not carry workflow-run diagnostics links because they fail before a run exists,
and they continue to preserve plain text errors.

### Milestone 7: End-To-End Verification And Documentation

**Goal:** Prove the full path works and leave durable contract documentation.

**Tasks:**
- [ ] Add a GUI or integration smoke path that triggers a controlled workflow
  failure and verifies run status, error event, timeline styling, and deep link.
- [x] Update module READMEs for diagnostics ledger, workflow diagnostics,
  workflow services, and workbench diagnostics as needed.
- [x] Add or update an ADR if the new error event spine changes architecture
  beyond existing README contracts.
- [ ] Run full backend/frontend verification for touched packages.
- [ ] Record completion summary, deviations, verification, and follow-ups in
  this plan.

**Verification:**
- `cargo test -p pantograph-diagnostics-ledger`
- `cargo test -p pantograph-workflow-service`
- `cargo test -p pantograph-embedded-runtime`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run typecheck`
- `npm run build`
- Any existing frontend test command that covers workbench diagnostics.

**Status:** In progress as of 2026-05-01. Added
`docs/adr/ADR-016-workflow-error-diagnostics-spine.md` to freeze the canonical
error event, secondary-event link, diagnostics-unavailable, no JSON fallback,
and scheduler/session state ownership decisions.

## Execution Notes

- 2026-04-30: Plan created after investigating workflow runs that surfaced GUI
  submit errors while durable projections remained `Running`.

## Standards Verification Passes

### Pass 1: Plan Structure

**Status:** Complete.

**Checks:**
- Required sections from `PLAN-STANDARDS.md` are present.
- Milestones are ordered by dependency.
- Verification is listed per milestone.
- Re-plan triggers and completion criteria are explicit.

**Findings:**
- The plan includes Objective, Scope, Inputs, Constraints, Assumptions,
  Dependencies, Risks, Definition of Done, Milestones, Verification, Re-Plan
  Triggers, Recommendations, Completion Summary, and Traceability Links.
- Milestones are dependency ordered: durable event contract, backend recorder,
  workflow capture, projections, frontend rendering, deep links, then final
  verification/documentation.
- Each milestone includes verification commands or concrete test categories.
- Completion criteria require durable ordered events, failed projections,
  obvious GUI diagnostics, deep links, tests, and README updates.
- No blocking open question remains for planning.

### Pass 2: Codebase Ownership

**Status:** Complete with one plan correction applied.

**Checks:**
- Each milestone maps to real Pantograph modules.
- Write sets avoid overlapping ownership where possible.
- Public facade preservation is additive and compatibility-aware.
- Existing diagnostics/read-model boundaries remain backend-owned.

**Findings:**
- Diagnostics ledger ownership maps to real files:
  `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `schema.rs`, `sqlite/event_sqlite.rs`, `repository.rs`, and `lib.rs`.
- Workflow capture and projection ownership maps to real workflow-service
  modules: `session_execution_api.rs`, `session_queue_api.rs`,
  `session_runtime.rs`, `diagnostics_api.rs`, and `trace/`.
- Runtime/model context ownership maps to existing embedded-runtime modules:
  `embedded_workflow_host.rs`, `embedded_workflow_host_helpers.rs`,
  `task_executor.rs`, model dependency descriptors, and runtime registry
  adapters.
- Tauri transport and diagnostics ownership maps to
  `src-tauri/src/workflow/headless_workflow_commands.rs` and
  `src-tauri/src/workflow/diagnostics/`.
- Frontend rendering ownership maps to existing typed diagnostics and
  workbench boundaries: `src/services/diagnostics/types.ts`,
  `src/services/workflow/workflowServiceErrors.ts`,
  `src/services/workflow/WorkflowCommandService.ts`,
  `src/components/workbench/DiagnosticsPage.svelte`,
  `diagnosticsPagePresenters.ts`, `RunGraphSnapshot.svelte`, and
  `runGraphPresenters.ts`.
- Correction applied: the plan now explicitly accounts for the closed
  `DiagnosticEventSourceComponent` enum and existing `validate_event_source`
  rules. Implementation must not introduce unvalidated free-form source
  components.
- Existing README files exist for diagnostics ledger, Tauri diagnostics,
  frontend diagnostics services, and workbench components; the plan requires
  targeted updates to those existing docs.

### Pass 3: Standards Compliance

**Status:** Complete.

**Checks:**
- Rust work follows sync-core/async-shell and task lifecycle rules.
- Frontend work follows backend-owned data, declarative rendering, and
  accessibility rules.
- Tests cover unit, integration, replay/recovery, and cross-layer acceptance
  where needed.
- Documentation and README updates are included for changed contracts.

**Findings:**
- Reviewed standards inputs:
  `PLAN-STANDARDS.md`, `CODING-STANDARDS.md`, `TESTING-STANDARDS.md`,
  `FRONTEND-STANDARDS.md`, `ACCESSIBILITY-STANDARDS.md`,
  `DOCUMENTATION-STANDARDS.md`, `CONCURRENCY-STANDARDS.md`,
  `languages/rust/RUST-STANDARDS.md`,
  `languages/rust/RUST-ASYNC-STANDARDS.md`, `TOOLING-STANDARDS.md`, and
  `templates/PLAN-TEMPLATE.md`.
- Rust implementation guidance is standards-aligned: pure error shaping and
  sanitization remain synchronous; ledger I/O stays at existing async command
  or workflow-service boundaries; the plan intentionally avoids adding a
  fallback writer or unowned background task.
- Frontend guidance is standards-aligned: workbench pages consume backend
  projection DTOs, presenters own formatting/classification, and components do
  not parse raw ledger payloads or repair run state locally.
- Accessibility guidance is standards-aligned: error visibility requires
  icon/text labels, accessible names, keyboard focus/navigation, and tests; the
  plan does not rely on color-only diagnostics.
- Testing guidance is standards-aligned: milestone verification covers unit,
  SQLite serialization/versioning, projection replay, idempotency/recovery,
  workflow-service integration, embedded-runtime context, Tauri command
  envelopes, presenter tests, component accessibility tests, and an end-to-end
  smoke path.
- Documentation guidance is standards-aligned: changed contracts require
  README updates at the ledger, workflow diagnostics, Tauri diagnostics,
  frontend diagnostics service, and workbench component boundaries, with an ADR
  only if implementation changes architecture beyond existing README
  contracts.

### Pass 4: Risk-Resolution Standards Revalidation

**Status:** Complete with additional implementation gates added.

**Checks:**
- Updated risk-resolution strategy still follows `PLAN-STANDARDS.md`.
- Required implementation order satisfies backend-owned data, concurrency,
  Rust async, frontend, accessibility, documentation, and testing standards.
- Codebase blast radius was rechecked for decomposition-review triggers and
  existing local documentation.
- Worktree hygiene requirements were rechecked before implementation begins.

**Findings:**
- The backend-first sequence remains standards-compliant: ledger contract,
  projection precedence, recorder proof, typed transport envelope, frontend DTO
  drift coverage, workbench focus state, and phased capture keep durable truth
  backend-owned before UI navigation consumes it.
- The testing strategy remains standards-compliant because it requires native
  Rust contract tests, SQLite versioning/replay tests, frontend
  parser/presenter tests, accessibility tests, stale-response tests, and a
  cross-layer acceptance path for the produced error event through GUI-visible
  diagnostics.
- Concurrency and Rust async guidance remains standards-compliant because
  payload shaping happens outside the ledger lock, the lock covers append only,
  no unowned background task is permitted, and stale frontend async responses
  require request timestamp or nonce checks.
- Documentation layout is standards-compliant: the plan lives under
  `docs/plans/<slug>/`, and the affected source directories already have
  README files that must be updated when contracts change.
- Decomposition review is required before implementation edits the largest
  touch points. Current line counts exceed standards thresholds:
  `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `src/components/workbench/DiagnosticsPage.svelte`, and
  `src/services/diagnostics/types.ts`.
- Worktree hygiene is currently a blocker for code implementation unless the
  user explicitly allows the unrelated dirty implementation files to remain in
  place. Planning-only Markdown edits remain allowed.

### Pass 5: Primary-Ledger-Only Standards Revalidation

**Status:** Complete.

**Checks:**
- Removing JSONL fallback preserves the backend-owned diagnostics source of
  truth required by `CODING-STANDARDS.md`.
- Diagnostics-unavailable behavior remains explicit enough for
  `PLAN-STANDARDS.md` risk mitigation and completion criteria.
- The updated plan still covers replay/recovery, cross-layer acceptance, and
  error-path verification required by `TESTING-STANDARDS.md`.
- The change does not introduce new background tasks, polling loops, or
  blocking file writes that would violate concurrency or Rust async standards.

**Findings:**
- Removing fallback JSONL improves standards compliance by eliminating a
  second durable diagnostics source and avoiding split ownership between
  SQLite projections and duplicate local files.
- The primary-ledger-only strategy remains standards-compliant because ledger
  hardening, deterministic truncation, serialization tests, and explicit
  `diagnostics_unavailable` envelopes replace hidden fallback writes.
- The plan now treats diagnostics ledger append failure as a visible internal
  diagnostics failure rather than silently writing a parallel trace.
- The affected structured contracts now include a typed
  `diagnostics_unavailable` indicator so frontend components do not infer this
  state locally.
- Verification remains standards-compliant because tests must prove hostile
  error text and large runtime stderr still append valid ledger events, while
  true ledger unavailability produces a typed unavailable response and no
  duplicate JSONL files.

### Pass 6: Causality Link Standards Revalidation

**Status:** Complete.

**Checks:**
- Optional causality fields do not weaken the ledger's existing chronological
  ordering contract.
- Causality and canonical-error links remain typed structured contracts rather
  than frontend-derived or string-parsed relationships.
- The plan prevents inferred cause links from timestamps, `event_seq`, or
  shared `workflow_run_id` alone.
- Tests are required for both linked direct-propagation paths and unlinked
  chronology-only neighboring events.

**Findings:**
- The affected code currently has no causality/link field in
  `DiagnosticEventPayload`, `DiagnosticEventAppendRequest`, or
  `WorkflowErrorEnvelope`, so implementation must add this as an explicit typed
  contract with validation and serialization tests.
- The standard-compliant design is conservative: `workflow_run_id` and
  `event_seq` remain the trace scope/order, while `caused_by_event_id` is only
  for producer-known direct propagation.
- `canonical_error_event_id` on secondary lifecycle/status events is the safer
  first implementation target than a broad causality graph because it solves
  diagnostics navigation without asking producers to infer complex causes.
- Frontend code must render these fields from backend DTOs and must not derive
  them by scanning timeline order.

### Pass 7: Registry Usability Standards Revalidation

**Status:** Complete.

**Checks:**
- Registry-driven phase/scope definitions preserve backend-owned diagnostics
  policy and avoid frontend or call-site inference.
- The registry remains typed and reviewable instead of stringly dynamic.
- Reuse across apps is enabled without weakening Pantograph workflow contracts.
- Tests are required to keep registered phase definitions complete and
  consistent.

**Findings:**
- A typed static registry is standards-compliant because it centralizes
  diagnostics policy in backend code while keeping call sites simple and
  service-independent.
- Scope structs reduce repetitive error-handler code and make invalid phase/id
  combinations harder to express.
- Separating generic recorder mechanics from Pantograph workflow phase
  registration supports reuse in another app without moving policy into
  frontend code or arbitrary JSON configuration.
- Registry matrix tests are required so adding/removing phases cannot silently
  skip source validation, required IDs, default severity/recoverability,
  projection effect, or causality policy.
- Direct diagnostic-error payload construction outside the recorder would be a
  standards violation because it bypasses the registered source, scope,
  sanitization, projection, envelope, and causality rules.

### Pass 8: Scheduler State Ownership Revalidation

**Status:** Complete.

**Checks:**
- Fatal diagnostics do not make the ledger append layer the workflow state
  machine.
- Live scheduler/session state remains owned by workflow-service domain control
  paths.
- Projection failure rules remain read-model recovery/safety behavior.
- Any push/broadcast behavior originates from workflow-domain failure handling,
  not diagnostics logging.

**Findings:**
- The standards-compliant ownership model is domain event first, diagnostics
  trace second: runtime/model/node failure handling must explicitly mark the run
  failed in scheduler/session state before or alongside diagnostics recording.
- A ledger append must not mutate scheduler state or publish control events;
  otherwise logging would become hidden workflow control flow.
- A typed `WorkflowRunFailed` domain event is acceptable if a push is needed,
  provided it is emitted by workflow-service failure handling and consumed by
  scheduler/session owners and diagnostics writers as separate subscribers.
- Projection rules may still derive failed GUI state from fatal error events
  when terminal/status events are missing, but only as read-model recovery so
  the GUI does not show indefinite `Running`.
- Verification must cover both live scheduler/session state leaving `Running`
  and projection replay recovering failed display state from durable events.

## Anti-Pattern And Blast-Radius Review

### Search Scope

- Reviewed planned touch points against diagnostics ledger event contracts,
  SQLite projection drains, workflow-service runtime admission, workflow error
  envelopes, Tauri command wrappers, frontend diagnostics DTOs, presenters, and
  workbench navigation state.
- Search focused on anti-patterns that could produce standards violations:
  duplicate ownership, frontend state repair, free-form event sources, status
  projection drift, blocking async paths, stale hand-mirrored DTOs, vague model
  lifecycle semantics, and second-source fallback storage.

### Findings To Address Before Or During Implementation

| Finding | Risk | Required Guardrail |
| ------- | ---- | ------------------ |
| Projection drains use finite event-kind allowlists. `diagnostic_projection_events_after`, `node_status_events_after`, and timeline projection code must each opt in to any new error event. | Appending a new error event can succeed while run list/detail/timeline/node projections never see it, leaving the GUI stuck on `Running`. | Milestone 4 must update every affected drain/query path and add replay tests proving the same error event updates timeline, run detail, run list, and node status when scoped. |
| Run list/detail projections currently set `status = excluded.status` for every accepted status event. | If fatal error events become status-driving without precedence rules, later nonterminal events can accidentally revive failed runs. | Add explicit terminal/fatal precedence in projection code: fatal run-scoped errors and terminal failed/cancelled/completed states must not be overwritten by later non-recovery events. |
| Existing model lifecycle semantics distinguish `LoadDependencyResolved`, `LoadStarted`, and `LoadCompleted`, but runtime admission currently records dependency resolution after `ensure_session_runtime_loaded`. | UI or diagnostics can imply a 20GB model was loaded when only runtime/dependency admission completed. | Do not map dependency resolution to “model loaded”. Only emit or display `LoadCompleted` after the runtime/backend proves the model is resident or ready for inference. Add tests for llama.cpp/Puma-Lib failure paths. |
| `DiagnosticEventSourceComponent` is closed and source validation is per event kind. | Adding source labels such as graph-editor, managed-binary, or Tauri transport as free-form strings would fail validation or weaken ledger consistency. | Keep source components typed and validated. Put narrower locations in payload fields or add enum variants with DB round-trip tests. |
| Frontend `DiagnosticEventKind` is hand mirrored and already lags backend event kinds such as scheduler queue-control/admission/reservation events. | Adding `diagnostic_error_occurred` can compile only if casts hide drift, or fail frontend type checks unexpectedly. | Update TypeScript unions and projection fixtures with backend event-kind coverage; prefer generated DTOs later. Add tests that timeline presenter accepts every backend-projected kind. |
| Tauri commands flatten `WorkflowServiceError` through `Result<T, String>` and `to_envelope_json`. | Diagnostics links can be lost if added outside the serialized envelope or if frontend parsing treats them as unknown details only. | Add link fields to the structured Rust envelope, preserve them in JSON serialization, and update frontend normalization to expose typed diagnostics link fields without requiring component JSON parsing. |
| Workbench navigation state currently tracks selected page and active run only. | Clickable error navigation needs focused event and optional node focus; bolting it onto components risks stale async responses or hidden local state. | Add typed transient navigation/focus state to `workbenchStore.ts`, with stale-response guards in pages that refresh diagnostics asynchronously. |
| Fallback JSONL can become a second diagnostics store. | Operators may see one error in fallback and different state in SQLite projections. | Do not implement JSONL fallback. Harden the primary ledger path and surface `diagnostics_unavailable` if it cannot record. |
| Ledger append is synchronous behind a mutex in workflow-service paths. | A broad recorder that formats large cause chains while holding the ledger lock can increase contention or deadlock if it calls back into workflow-service. | Shape/sanitize error context before taking the ledger lock; hold the lock only for append; never call workflow-service or runtime code from inside the locked section. |
| Event payload size is capped at 8 KiB. | Runtime stderr, llama.cpp process output, or cause chains can still make error recording fail if not bounded before append. | Apply deterministic truncation before `DiagnosticEventAppendRequest::validate`; store only bounded summaries unless an existing payload-ref policy explicitly supports larger artifacts. |
| Existing projection schemas use `CREATE TABLE IF NOT EXISTS`; new columns on existing user databases need explicit version behavior. | Adding latest-error columns without a version boundary can leave installed databases missing columns. | For non-breaking projection-only changes, use explicit projection version bumps and rebuild tests. For breaking ledger/schema changes, create a new ledger version/namespace and do not mix old and new trace data. Do not rely on `CREATE TABLE IF NOT EXISTS` to evolve existing tables. |
| Tauri diagnostics overlays and durable ledger projections both expose run/debug state. | Adding error state to both layers can recreate duplicate ownership. | Keep canonical run error truth in the diagnostics ledger/projections. Tauri diagnostics may transport or overlay UI-only data but must not synthesize canonical failure state. |
| Causality links can become guessed from chronology. | Incorrect `caused_by_event_id` links would make the trace less trustworthy than ordered events alone. | Set `caused_by_event_id` only when the same producer path caught or translated a known prior failure event. Never infer cause from timestamps, `event_seq`, or shared `workflow_run_id` alone. |

### Blast Radius

- **Persistence:** `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `schema.rs`, `repository.rs`, `sqlite/event_sqlite.rs`, and ledger tests.
- **Workflow orchestration:** workflow-service session execution, queue,
  runtime admission, diagnostics query, trace, terminal-event, and projection
  tests.
- **Runtime and managed binaries:** embedded-runtime registry/session helpers,
  `crates/inference` managed-runtime status/resolution, llama.cpp/Ollama/PyTorch
  launch and model dependency failure paths.
- **Transport:** Tauri headless workflow command wrappers and diagnostics
  transport DTOs that currently serialize backend errors as JSON strings.
- **Frontend contracts:** `src/services/diagnostics/types.ts`,
  `src/services/workflow/workflowServiceErrors.ts`,
  command/projection service tests, diagnostics presenters, graph presenters,
  workbench store, Diagnostics/Graph page components, and accessibility tests.
- **Documentation:** diagnostics ledger README, workflow-service workflow and
  trace READMEs, Tauri diagnostics README, frontend diagnostics service README,
  and workbench README.

### Implementation Priority Adjustments

- Treat projection precedence and model-load semantics as Milestone 1-4
  blockers, not frontend polish.
- Before adding GUI deep links, make the backend envelope carry typed
  diagnostics link fields end to end.
- Before adding broad capture points, add a narrow recorder test that proves a
  control-character runtime error produces a ledger error event, a failed run
  projection, and a timeline row.
- Add a DTO drift test or fixture coverage for every backend-projected
  `DiagnosticEventKind` before adding the new error kind.

## Risk Resolution Strategy

### Required Implementation Order

0. Resolve or explicitly allow the current unrelated dirty implementation
   files before code implementation begins. Planning/documentation edits may
   continue, but source, test, config, build, and generated-file changes must
   have clear ownership before Milestone 1 starts.
1. Add the durable ledger error event contract with validation,
   serialization/deserialization, DB round-trip, source-validation, and payload
   bound tests.
2. Add projection support and status precedence before broad error capture.
   A narrow replay test must prove one fatal `diagnostic.error_occurred` event
   updates scheduler timeline, run detail latest-error fields, run-list failed
   status, and node status when node-scoped.
3. Add the typed diagnostics error registry and scope model before broad
   recorder usage. The initial registry should be a static backend-owned table
   for Pantograph workflow phases, not a runtime string map.
4. Add one narrow backend recorder path for the known control-character runtime
   failure mode. This path must prove scheduler/session state exits `Running`
   through workflow-service domain handling, sanitized error text is recorded
   as a durable error event, projections show failed, and the timeline row is
   visible before additional capture points are added.
5. Extend `WorkflowErrorEnvelope` with typed diagnostics link fields and update
   frontend parsing. GUI components must receive typed fields instead of
   parsing raw JSON or `details`.
6. Update frontend diagnostics DTOs and add drift coverage for every
   backend-projected `DiagnosticEventKind`.
7. Add workbench diagnostics focus state and graph-editor navigation actions.
   UI deep links may be implemented only after backend envelopes carry stable
   `workflow_run_id` and `diagnostic_event_id` fields.
8. Add the node execution boundary wrapper and projection operation wrapper
   before broad node/projection capture. These wrappers must make the correct
   diagnostics path the default for injected node execution and projection
   drain/query/rebuild calls.
9. Broaden capture points phase by phase across submission, queue/admission,
   model dependency, runtime launch, node execution, artifact, projection, and
   transport boundaries.

### Diagnostics Error Registry And Scope Model

- Centralize phase definitions in a backend-owned diagnostics error registry.
  The registry defines each phase's typed phase ID, required scope kind,
  allowed `DiagnosticEventSourceComponent` values, default severity, default
  recoverability, causality policy, projection effect, and required context
  fields.
- Keep the first implementation statically registered in Rust so it remains
  compile-time reviewable and testable. Do not use arbitrary string phases,
  JSON schema blobs, or unvalidated runtime maps for core workflow diagnostics.
- Keep the recorder reusable by separating generic registry/recorder mechanics
  from Pantograph workflow phase registration. A future app can provide its own
  typed static phase table without rewriting sanitization, validation, ledger
  append, envelope-link, or diagnostics-unavailable logic.
- Define typed scope structs such as `RunErrorScope`, `NodeErrorScope`,
  `RuntimeModelErrorScope`, `SchedulerErrorScope`, `ArtifactErrorScope`,
  `ProjectionErrorScope`, and `TransportErrorScope`. Registry phase
  definitions choose one required scope kind; call sites should not pass raw
  field maps.
- Expose phase-specific convenience APIs or builders from registry definitions,
  but keep policy in the registry. Call sites should normally choose a phase,
  provide the typed scope, and pass the original error; severity/recoverability
  should use registry defaults unless the caller has a concrete reason to
  override.
- The ergonomic target is that the correct recorder path is shorter than manual
  event construction. Representative call sites should look like
  `diagnostics.model_load_failed(scope, &err).await?` or
  `diagnostics.run_failed(scope, &err).caused_by(error_event_id).await?`.
  Call sites should supply only what the compiler and registry cannot know:
  typed scope, original error, and an optional known causal diagnostic event ID.
- Ban direct construction of `DiagnosticEventPayload::DiagnosticErrorOccurred`
  outside the recorder module and tests. This can start as a review rule and
  later become a repository script or lint check.
- Add a registry matrix test that fails if a registered phase lacks required
  scope fields, source validation, default severity/recoverability, causality
  policy, projection effect, or documentation.

### Domain Failure Handling And Scheduler State

- Diagnostics recording is not the workflow state machine and must not be the
  mechanism that tells the scheduler a run failed.
- Fatal workflow errors must flow through a workflow-service domain failure
  path that owns the live state transition: mark scheduler/session state
  failed, release or cancel relevant reservations/queue entries, then record
  the canonical diagnostic error and linked lifecycle/status facts.
- If a push/broadcast is needed, emit it from domain failure handling as a
  typed workflow-domain event such as `WorkflowRunFailed`, carrying
  `workflow_run_id`, optional `canonical_error_event_id`, phase, and
  `failed_at_ms`. The diagnostics ledger append layer must not publish control
  events that mutate scheduler state.
- Diagnostics projections may use fatal error events to recover/read failed
  state for GUI projections when terminal/status events are missing, but that
  is a read-model safety net only.
- Add tests that prove a fatal runtime/model/node error removes the run from
  active `Running` scheduler/session state even if projection refresh is
  delayed, and separate replay tests that projections still recover failed
  display state from durable fatal error events.

### Projection Status Precedence

- Introduce a single backend helper for run projection state transitions before
  error events can drive read-model status.
- Terminal statuses `completed`, `failed`, and `cancelled` must not be
  overwritten by later nonterminal events.
- Fatal run-scoped diagnostic errors set projected run status to `failed`.
- Fatal node-scoped diagnostic errors set the node projection status to
  `failed` without requiring a separate node failed event.
- Later nonterminal scheduler, runtime, or node events must not revive failed
  runs or nodes.
- Any future recovery semantics require a dedicated recovery event and a
  separate plan update; recovery must not be inferred from ordinary lifecycle
  events.

### Node Execution Boundary Diagnostics

- Node authors must not be required to import diagnostics ledger, recorder, or
  workflow-run context APIs. User-authored nodes can continue returning ordinary
  errors.
- The execution/injection boundary owns automatic context: `workflow_run_id`,
  execution/session ID, `node_id`, `node_type`, node version when known,
  attempt/cancellation state, demanded output context, injected runtime/model
  capability context, and available input/output port context.
- Optional typed node errors may improve classification, such as invalid input,
  missing model, external tool failed, cancelled, or capability unavailable.
  Plain errors still produce useful `node_execution_failed` diagnostics through
  boundary-owned context.
- The node diagnostics wrapper records the canonical node-scoped diagnostic
  before existing code converts node engine failures into `WorkflowServiceError`
  variants.
- If the exact failing internal node/task is unknown and only the demanded
  output node is known, record that uncertainty explicitly rather than claiming
  false precision.

### Projection Failure Wrapper

- Projection drains, queries, and rebuilds should run through a typed wrapper
  such as `ProjectionKind::RunDetail`, `ProjectionKind::RunList`,
  `ProjectionKind::SchedulerTimeline`, `ProjectionKind::NodeStatus`, and
  `ProjectionKind::LibraryUsage`.
- Projection scopes should distinguish global projection work from scoped
  requests, such as run detail, node status, or artifact projection context.
- The wrapper records `projection_failed` diagnostics with projection name,
  version, operation, batch size/query context when known, and the original
  ledger/projection error.
- The wrapper must preserve and return the original error. It must not turn
  projection failures into workflow execution failures unless the caller is
  explicitly handling a workflow-run operation that already owns a run context.
- Projection failure handling may update projection-owned status such as
  failed/rebuild-needed, but must not import or call scheduler/session mutation
  APIs. Add tests or static review checks for this boundary.
- After the wrapper exists, workflow-facing diagnostics APIs should avoid raw
  `map_err(WorkflowServiceError::from)` around projection drains, queries, and
  rebuilds unless the call site is explicitly exempted in module documentation.

### Conservative Causality And Canonical Error Links

- Chronology is already represented by `event_seq` and `occurred_at_ms`.
  Causality fields must not duplicate chronology or encode guesses.
- `caused_by_event_id` is optional and may be set only when the producer has
  direct mechanical knowledge that a prior event caused the current surfaced
  error. Valid examples include a runtime error event being caught by
  workflow-service and translated into a `run.terminal` failure, or a node
  execution error event being translated into `node.execution_status = failed`.
- Do not set `caused_by_event_id` when the only evidence is that another event
  happened earlier, shares the same `workflow_run_id`, or has a nearby
  timestamp.
- When a lifecycle/status event is a direct consequence of the canonical
  diagnostic error event, prefer a narrow `canonical_error_event_id` link on the
  secondary event over a broad causality graph. This keeps terminal/status
  events navigable without pretending to model every causal relationship.
- If multiple possible causes exist and the producer cannot identify the
  decisive one, leave `caused_by_event_id` unset and rely on the ordered trace.
- Add tests that prove causality links are present for direct propagation paths
  and absent for chronology-only neighboring events.

### Model Lifecycle Semantics

- `LoadDependencyResolved` means required runtime/model dependencies were
  resolved for admission. It must not be displayed as “model loaded”.
- `LoadCompleted` may be emitted or displayed only after the backend runtime
  proves the model is resident or inference-ready.
- If llama.cpp, Ollama, PyTorch, or Puma-Lib cannot provide a real readiness
  signal, the workflow path must stop at dependency/resolution state and avoid
  claiming load completion.
- Add tests that reproduce llama.cpp/Puma-Lib model-load failures and assert no
  false `LoadCompleted` event is emitted.

### Error Source And Location Modeling

- Keep `DiagnosticEventSourceComponent` typed and allowlisted.
- Use existing source components where accurate:
  `WorkflowService`, `Scheduler`, `Runtime`, `NodeExecution`, `Library`,
  `Retention`, and `LocalObserver`.
- Add new source enum variants only when a producer is a true durable owner,
  and include DB serialization/deserialization and source-validation tests in
  the same commit.
- Represent narrower locations in the error payload with typed fields such as
  `phase`, `operation`, `subsystem`, `binary_id`, `command_id`, `ui_surface`,
  `node_id`, `runtime_id`, and `model_id` instead of free-form source labels.

### Transport And Frontend Contract Handling

- Add optional diagnostics link fields directly to `WorkflowErrorEnvelope`:
  `workflow_run_id`, `diagnostic_event_id`, `node_id`, `runtime_id`,
  `model_id`, `phase`, `source_component`, and `severity`.
- Keep the existing `message` and `details` behavior compatible for callers
  that do not read diagnostics links.
- Update `workflowServiceErrors.ts` so `WorkflowServiceError` exposes typed
  diagnostics link fields and keeps the original backend envelope.
- Graph editor, Diagnostics page, and Graph page components must use typed
  error/link fields from services or presenters. They must not parse backend
  envelope JSON directly.
- Add frontend fixture coverage that fails when backend-projected diagnostic
  event kinds are missing from TypeScript unions.

### Workbench Focus State

- Extend `workbenchStore.ts` with transient diagnostics focus state owned at
  the workbench level, not inside individual page components.
- Focus state should include selected page, active run, focused diagnostic
  event ID, optional focused node ID, and a request timestamp or nonce for
  stale-response protection.
- Diagnostics and Graph pages consume focus state declaratively and clear or
  acknowledge focus through store helpers.
- Async refresh paths must ignore stale focus/navigation responses when a newer
  active run or focus request has replaced them.

### Diagnostics Unavailable Boundary

- The diagnostics ledger is the only durable workflow-run diagnostics trace.
- The implementation must not add JSONL fallback files, duplicate event logs,
  or a second diagnostics query source.
- The implementation must not add a separate `error_recorded` event. A
  `diagnostic.error_occurred` ledger row is already the durable record of the
  error. Recording success is represented by the returned diagnostic event ID;
  recording failure is represented by `diagnostics_unavailable`.
- SQLite append failure should preserve the original command/runtime error in
  the returned envelope while also reporting that diagnostics recording failed.
- Use a typed `diagnostics_unavailable` state or envelope field so GUI surfaces
  can explain that trace persistence failed without pretending a durable event
  exists.
- A future secondary preservation sink, import path, or recovery viewer
  requires its own standards-compliant plan and architecture review.

### Locking, Payload Bounds, And Ledger Versioning

- Build, sanitize, and truncate error payloads before taking the diagnostics
  ledger mutex.
- The ledger lock must cover append only. Recorder code must not call back into
  workflow-service, runtime, frontend transport, or projection refresh logic
  while holding it.
- Keep error payloads under the existing 8 KiB ledger cap with deterministic
  truncation and tests for large stderr/cause-chain inputs.
- Diagnostics trace data is short-retention operational data, so breaking
  ledger schema changes must not in-place migrate old ledgers into the new
  shape.
- Each active diagnostics ledger version must have an explicit schema/version
  identity. Breaking changes create a new ledger namespace or physical ledger
  file such as `workflow-diagnostics-v2.sqlite`.
- Active runtime code writes only to the current ledger version. Old ledgers are
  read-only archived diagnostics, or ignored and cleaned by retention.
- Projections may be rebuilt within one ledger version, but projection rebuilds
  must not translate incompatible old-version trace data into a new-version
  ledger.
- Non-breaking additive changes are allowed inside the current ledger version
  only when old readers and writers remain correct. Existing user databases must
  not rely on `CREATE TABLE IF NOT EXISTS` to gain required columns.
- If archived old-version diagnostics are exposed in the GUI, they must be
  clearly labeled read-only and versioned.

### Decomposition Gates

- Before adding new ledger payload types, review whether
  `crates/pantograph-diagnostics-ledger/src/event.rs` should extract the
  diagnostic error payload, source/scope validation, or projection DTOs into
  smaller modules. If not extracting, record why the local addition is safer
  than a split in the milestone notes.
- Before adding projection logic, review whether
  `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs` should move
  error projection helpers, status precedence, or event-kind drain queries into
  focused files under `sqlite/`.
- Before adding workflow capture, avoid expanding
  `session_execution_api.rs` with broad recorder logic. Put reusable error
  registry, scope, and recorder code in a focused workflow diagnostics module
  and keep session execution limited to orchestration calls.
- Before adding Diagnostics page UI, keep classification and row shaping in
  presenters and consider extracting focused Svelte subcomponents if the page
  grows further.
- Before expanding `src/services/diagnostics/types.ts`, group new error DTOs
  clearly and consider a follow-up DTO generation plan if hand-mirrored unions
  continue to drift.

## Commit Cadence Notes

- Commit after each logical slice is implemented and verified.
- Keep ledger contract, backend recorder, workflow capture, projections,
  frontend rendering, deep links, and documentation updates in separate
  reviewable commits unless a small adjacent test belongs with the same slice.
- Follow commit format/history cleanup rules from `COMMIT-STANDARDS.md`.

## Optional Subagent Assignment

No parallel workers are planned for the first implementation pass.

If implementation is later parallelized, use one wave at a time:

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| Backend ledger owner | Diagnostics ledger schema/event contract | Committed ledger event, versioning policy, tests, README updates | Milestone 1 complete |
| Backend workflow owner | Workflow-service/embedded-runtime capture and projections | Error recorder, capture points, projection tests | Milestones 2-4 complete after ledger owner lands |
| Frontend owner | Diagnostics UI, command error links, accessible navigation | Typed DTOs, presenters, UI tests, workbench deep links | Milestones 5-6 complete after backend DTOs stabilize |

Worker write sets must be made explicit before spawning or assigning parallel
work. Shared DTOs, ledger versioning, and global frontend types must have one
owner per wave.

## Re-Plan Triggers

- A proposed ledger schema change is breaking but cannot use a new ledger
  version/namespace.
- A workflow error occurs before any workflow/run context can be established and
  cannot be linked to diagnostics by existing command context.
- Existing frontend navigation cannot focus a diagnostics event without a
  broader routing change.
- Error recording introduces blocking work or unowned background tasks in async
  workflow paths.
- Projection replay cannot derive failed status deterministically from error
  events.
- Standards verification finds a required README, ADR, test category, or
  ownership split missing from the plan.
- Code implementation is requested while unrelated dirty implementation files
  remain unresolved and not explicitly allowed.
- Decomposition review finds that adding the new error diagnostics work to an
  already oversized file would make review or testing materially worse.

## Recommendations

- Prefer one canonical error event plus links from existing lifecycle events
  over adding separate incompatible error fields to every event type.
- Prefer a typed static phase/scope registry over hardcoded ad hoc call-site
  handlers or arbitrary string configuration. This keeps the system practical
  to use while preserving validation and reviewability.
- Keep the diagnostics ledger as the only durable run-error trace; do not add
  JSON fallback files or alternate local event stores.
- Implement GUI color changes with icon/text labels and focused event controls
  at the same time to avoid inaccessible error-only color cues.

## Completion Summary

### Completed

- Plan created.
- Standards verification passes completed against the listed standards and
  current diagnostics/workbench codebase ownership.

### Deviations

- None.

### Follow-Ups

- None yet.

### Verification Summary

- Plan structure, codebase ownership, and standards compliance passes completed
  on 2026-04-30.
- Codebase verification corrected the Milestone 1 ledger contract work to
  account for the closed `DiagnosticEventSourceComponent` enum and
  `validate_event_source` rules.

### Traceability Links

- Module README updated: N/A for plan creation.
- ADR added/updated: N/A for plan creation; revisit during Milestone 7.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A until
  implementation PR.
