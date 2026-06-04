# Inference Interface Resolution And Validation

## Objective

Keep workflow graphs simple by using one generic inference node whose effective
typed inputs and outputs are resolved from the connected model. The same
backend-owned resolution contract must drive graph editor draft ports,
executable publish/admission validation, scheduler task materialization, and
pre-dispatch validation. Draft save remains graph persistence for continued
editing and must not create executable authority.

This is not an image-only system. It applies to image generation, text
generation, audio, multimodal models, diffusion LLMs, and future model
families as long as Pumas and inference capability facts can identify a
supported task interface.

## Problem

If graph editor port discovery, workflow validation, task materialization, and
runtime dispatch use separate logic, Pantograph can show users ports that save
executable publish rejects, publish workflows that execution cannot
materialize, or submit runtime tasks that runtime-host later rejects. The
scheduler and runtime-host still need fail-closed safeguards, but Pantograph
should catch invalid inference nodes as early as possible while the graph is
being edited without blocking draft graph persistence.

## Scope

In scope:

- shared descriptor, authored snapshot, drift, validation summary, and typed
  option/value contracts;
- workflow-service resolver, validation, materialization, queue-admission, and
  pre-dispatch revalidation boundaries;
- graph editor rendering, submit gating, drift display, and update-preview
  integration driven by backend descriptors;
- runtime-host request input alignment for scheduler-dispatched inference
  tasks;
- deletion or rewrite of retired inference-interface paths replaced by the
  descriptor system.

Out of scope:

- scheduler runtime/device ranking policy changes beyond consuming explicit
  graph-authored constraints and typed validation results;
- runtime-specific execution implementation for every model family;
- an advanced runtime support inspector that lists full scheduler candidate
  sets for every descriptor;
- compatibility shims that keep old model-path, static all-port, or
  `inference_settings` execution paths working as fallbacks.

## Ownership Boundaries

- **`pantograph-inference-interface-contracts`:** owns the shared DTO-only
  contract for resolved descriptors, authored snapshots, value categories,
  availability, diagnostics, drift reports, and typed validation summaries. It
  does not own the live graph validation event envelope or event-stream state,
  because graph/node routing, update proposals, graph revisions, and session
  staleness are workflow-service behavior. It must not import workflow-service,
  scheduler, runtime-host, Pumas lookup, inference execution, or frontend
  rendering policy. It may depend on stable, path-free model-reference DTOs only
  when that dependency does not pull in lifecycle, storage lookup, or runtime
  wiring behavior.
- **Pumas:** owns model identity, selected artifact identity, package facts,
  artifact readiness, storage kind, validation state, and local/external
  storage approval. Pumas does not own Pantograph support policy or scheduler
  runtime ranking. Pumas may expose logical artifact/component sizes and
  source-tagged package evidence, but it does not own exact loaded-memory,
  context-memory, residency, or admission estimates.
- **Inference crate:** owns canonical inference task/interface conventions,
  runtime-family capability facts, normalized trait names, supported and
  unavailable feature reporting, and runtime-specific execution translation
  after the scheduler selects a runtime.
- **Workflow-service:** owns the resolver application service that combines
  Pumas facts, inference capability facts, graph constraints, and runtime
  availability into a typed `InferenceInterfaceDescriptor`. It also owns draft
  graph validation, draft-save schema/canonicalization checks, executable
  publish validation, execution-time revalidation, and materialization of graph
  literals/defaults/upstream task results into typed runtime-host inputs. Draft
  save may persist invalid editable graphs and history, but executable publish,
  submit, and queue admission must fail closed unless the backend descriptor
  validation summary is current and executable. Workflow-service is the only
  owner for live validation event/session envelopes, graph/node event scope,
  stale-session filtering, and update-proposal transport.
- **Scheduler:** owns runtime/device/reservation/batch selection. It receives
  validated task intent and explicit user constraints, but it does not own
  prompt/image/mask/settings values, graph port discovery, or model-path
  resolution.
- **Graph editor:** renders backend-provided descriptors and diagnostics. It
  may submit draft graph state and optional user constraints for validation,
  but it must not hardcode model-family inference ports or infer backend
  support from filenames.
- **Node-engine:** validates and executes non-runtime tasks from materialized
  inputs. Runtime inference nodes are scheduler/runtime-host tasks, not
  node-engine planned-inference launches.
- **Runtime host:** receives dispatch-selected handoff plus materialized typed
  inputs. It must not pull graph state or workflow-service store state
  implicitly.

## Standards Guardrails

- Preserve package-role boundaries: `pantograph-inference-interface-contracts`
  is a contract crate only, workflow-service owns application orchestration,
  graph editor owns presentation, scheduler owns scheduling policy, and
  runtime-host owns dispatch execution.
- Parse and validate external/persisted/IPC payloads at the boundary into
  typed contracts. Internal code must consume validated values rather than
  repeatedly checking raw strings, raw JSON, or unbounded maps.
- Draft graph extraction is a validation boundary, not a best-effort helper.
  Strict model-ref binding and explicit constraint parsing must land before any
  synchronous or event-driven validation publisher consumes graph requests.
- Keep backend-owned data authoritative. The frontend may hold transient draft
  UI state, but current descriptor semantics, validation summaries, update
  proposals, and enqueue eligibility come from the backend.
- Prefer event-driven validation updates over frontend polling. If a short-lived
  polling bridge is unavoidable at a boundary, the owning module must document
  lifecycle, cancellation, cleanup, and why event delivery was not feasible.
- Do not hold graph/session locks across Pumas lookup, inference capability
  resolution, runtime availability checks, or other async/blocking work.
  Snapshot required state, release locks, resolve, then publish only if the
  validation session and graph revision are still current.
- Update source-directory READMEs or ADRs for every touched module boundary,
  especially new crates, workflow-service graph/validation modules,
  runtime-host contracts, and frontend validation/rendering modules.
- Public contract crates with feature flags must pass default,
  no-default-features, and all-features checks. Keep default features minimal
  and avoid expensive runtime dependencies in DTO-only crates.
- The first implementation pass must be a validated vertical slice before broad
  horizontal expansion: model ref input -> descriptor resolution -> authored
  port projection -> validation summary -> submit/admission gate.
- The first validation publication pass must return and store bounded
  node-scoped projection records, not only descriptor fingerprints. The graph
  editor needs descriptor/authored-port data for rendering, and dependency
  action derivation needs current node metadata without asking the frontend to
  carry Pumas facts, paths, or scheduler policy.
- Backend queue admission must consume the current validation summary before
  queue insertion or queue-placement diagnostics are recorded. Frontend gating
  is presentation only; backend admission remains the source of truth.
- API-breaking cleanup is expected for retired graph/runtime/interface paths.
  Do not keep aliases, compatibility shims, or alternate successful routes for
  `modelPath`, executable entry paths, unscoped validation events,
  `inference_settings`, `expand-settings`, static all-port inference metadata, or
  semantic `node.data.definition` inference overlays.
- Decomposition rule for the current validation-state work: do not add more
  state-machine or dependency-action logic to oversized modules such as
  `graph/session.rs` or the shared contract crate root. The implementation must
  introduce a focused workflow-service validation-state module with a small
  public facade and update the graph module README or ADR. If shared contract
  changes are required in `pantograph-inference-interface-contracts`, split
  contracts into focused modules or record and execute a decomposition slice in
  the same milestone rather than expanding `src/lib.rs` further. The current
  contract crate root is already large enough that new DTO families for
  validation publication, descriptor projections, or dependency actions need a
  focused module unless the change is only a few lines on an existing type.
- Apply parse-once semantics at every boundary. IPC/API payloads may enter as
  serialized DTOs, but workflow-service internals should consume validated
  action intents, graph revisions, validation session ids, and state records.
  Do not pass raw `String`, raw `u64`, or unbounded JSON maps through the
  validation-state owner when a validated newtype or enum exists.
- Live validation has an explicit lifecycle owner in workflow-service. No slice
  may add fire-and-forget validation tasks, unbounded event buffers, or frontend
  polling loops as the primary sync mechanism. Event-driven validation must use
  bounded queues or bounded state records, define overflow/backpressure behavior,
  cancel or supersede work on graph revision changes and session close, observe
  task errors/panics at the owner, and clean up subscriptions deterministically.
- 2026-06-03 live-validation task-owner sequence decision: implement the next
  event-driven validation step as a backend-only workflow-service task owner
  around the existing validation publisher before wiring graph-mutation triggers
  or frontend overlays. The owner must track spawned handles, cancel/supersede
  in-flight work, clean up on graph-session close/shutdown, observe completion,
  cancellation, errors, and panics, and publish only bounded typed lifecycle
  diagnostics. This keeps descriptor publication as one shared path and avoids
  complecting graph mutation policy, transport, and frontend display state.
  Tauri/frontend remain transport/presentation only and may not supply resolver
  facts, infer validation freshness, own cancellation policy, or gate submit
  independently of the backend summary.
- 2026-06-03 task-owner foundation implemented: workflow-service now has a
  backend-only validation task owner around the shared publisher with tracked
  handles, supersession cancellation, graph-session close cleanup, bounded
  terminal diagnostics, and deterministic backend tests for completion,
  supersession, and panic observation. The next slices remain trigger wiring and
  transport/frontend consumption; those slices must keep freshness, resolver
  facts, and submit authority backend-owned.
- 2026-06-03 backend explicit trigger implemented: `WorkflowService` can now
  start a backend-owned validation task from the existing typed graph-session
  and graph-revision request and return the backend-issued validation session
  id. Transport commands, frontend overlays, and graph-mutation auto-triggers
  remain separate slices and must not accept resolver facts or own freshness.
- 2026-06-03 task-start transport implemented: Tauri and the frontend command
  service now forward the backend explicit validation task trigger without
  adding validation policy. They carry only graph-session/revision intent and
  the returned validation session id; graph-mutation auto-triggering and editor
  overlay state remain separate slices.
- 2026-06-03 auto-trigger lifecycle re-plan decision: use Option 1. Before
  graph mutations auto-start validation tasks, add explicit shutdown/drain
  ownership to the workflow-service validation task owner. The shutdown path
  must be idempotent, stop accepting new validation work after shutdown starts,
  propagate cancellation to active validation tasks, await or abort tracked
  handles, record bounded terminal diagnostics for cancellation/panic/error,
  and expose a single backend lifecycle boundary that composition roots can
  call before dropping the service. Only after that is verified may semantic
  graph mutations call the task owner automatically. Tauri/frontend remain
  transport/presentation only and must not own task lifecycle, validation
  freshness, retry policy, resolver facts, or submit authority.
- 2026-06-03 task-owner shutdown slice implemented: workflow-service now has
  backend-owned validation task shutdown state. Shutdown stops accepting new
  validation work, cancels active lifecycle state with a typed shutdown reason,
  aborts/awaits tracked validation task handles, records bounded terminal
  diagnostics, and exposes `GraphSessionStore`/`WorkflowService` lifecycle
  methods. The follow-up composition-root slice wires those methods into
  service-owning shutdown paths before graph-mutation auto-triggering.
- 2026-06-03 composition-root shutdown wiring implemented: embedded-runtime
  shutdown and Tauri window shutdown now call the backend workflow-service
  validation task lifecycle boundary before runtime/gateway teardown. The next
  source slice may wire semantic graph mutations to auto-start backend-owned
  validation tasks; layout-only mutations and read-only candidate lookups must
  remain non-triggering.
- Keep the validation core synchronous wherever it only parses, validates,
  projects, or computes summaries. Async belongs at Pumas/inference/runtime fact
  lookup, event delivery, IPC, or persistence boundaries, and locks must be
  released before any `.await` or blocking work.
- Frontend validation stores must be keyed by backend-issued validation session
  id, graph fingerprint revision, event sequence, and node scope. Stale responses
  must be discarded without mutating current node state, editing must remain
  available while validation is pending, and submit/enqueue presentation must
  depend only on the latest backend validation summary.
- Validation summaries, projection records, events, diagnostics, and persisted
  authored snapshots must remain bounded and path-free. Do not include executable
  local paths, Pumas package fact blobs, runtime load targets, large media,
  secrets, environment variables, or scheduler decisions in graph/editor-facing
  state.
- New dependency or feature additions must follow dependency ownership rules:
  declare dependencies at the narrowest crate/package that executes them, keep
  DTO crates lean with minimal default features, and record tree/feature impact
  when adding nontrivial crates or frontend packages.

## Standards Compliance Review Notes

Standards reviewed for this milestone: plan, architecture, concurrency,
testing, documentation, security, interop, frontend, dependency, Rust API, Rust
async, Rust security, Rust interop, and Rust dependency standards from
`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.

The plan must preserve these compliance decisions during implementation:

- **Layering:** graph editor stays presentation-only, Tauri stays transport-only,
  workflow-service owns validation orchestration, the scheduler owns only
  runtime/device/resource policy, and runtime-host owns dispatch execution.
- **Validation:** deserialize and parse at IPC/API/persisted graph boundaries
  into typed DTOs/newtypes; after that, internal code consumes validated
  graph-session, revision, node, descriptor, and action types.
- **Async/concurrency:** validation session management owns cancellation,
  supersession, bounded queues/state, and task observation. Domain projection and
  validation logic stays sync-core; async-shell code is limited to fact lookup,
  transport, persistence, or event delivery.
- **Frontend:** event-driven updates replace polling where feasible, stale
  validation responses are ignored by session/revision/sequence, subscriptions
  are cleaned up on graph/session lifecycle changes, and validation does not
  block graph display or editing.
- **Docs:** every source module boundary created or materially changed by this
  milestone must update a source-directory README or ADR with purpose, API
  consumer contract, structured producer contract when applicable, invariants,
  alternatives rejected, and revisit triggers.
- **Tests:** each slice needs focused unit/integration tests plus the thinnest
  cross-layer acceptance path before broad horizontal expansion. Lifecycle tests
  are required for cancellation, stale-event rejection, bounded queue/state
  behavior, and frontend subscription cleanup.

## Resolved Design Decisions

- Use a dedicated shared crate, `pantograph-inference-interface-contracts`, for
  DTOs and validation contracts only.
- Split resolver and validator requests. Resolver input stays model-focused;
  validator input consumes the resolved descriptor/fingerprint plus draft graph
  node bindings, literals, and selected options.
- Use a normalized graph-facing descriptor with ports, defaults, typed option
  sets, availability, and diagnostics. Do not expose a full runtime-specific
  execution schema through graph-visible descriptors.
- Represent value types with grouped categories: scalar, artifact, reference,
  and constraint. Defaults are typed small scalar/default-marker values unless
  a later artifact-reference default contract is explicitly added.
- Represent options as typed bounded option sets with per-option availability
  and diagnostics. Do not use `serde_json::Value` option payloads.
- Represent availability as a small coarse status plus additive typed reasons
  and bounded diagnostics. UI behavior keys primarily from status; detailed
  messages come from reason codes and diagnostics.
- Persist a minimal authored interface snapshot directly in inference node
  data inside the saved workflow graph. The snapshot preserves editor shape
  and drift diagnostics only; current backend-resolved descriptors remain the
  only executable validation source.
- When authored and current interfaces drift, keep the authored shape visible,
  produce typed drift diagnostics, disable enqueue when blockers remain, and
  offer an explicit update path. Never silently rewrite graph shape.
- Backend returns typed graph patch proposals for interface updates; frontend
  previews and applies them only after user confirmation.
- Keep descriptor/drift contracts in `pantograph-inference-interface-contracts`.
  Keep graph patch/update proposal contracts in workflow graph/service
  contracts because they mutate persisted graph structure.
- Keep validation summary DTOs in `pantograph-inference-interface-contracts`
  because they are generic descriptor-validation results. Remove or retire
  shared unscoped validation event/stream DTOs from that crate; the live scoped
  event stream is workflow-service owned.
- Use typed graph patch operations at the API layer. The GUI renders those
  operations visually through graph nodes, ports, edges, and validation
  overlays.
- Use live validation sessions as the target UX. Graphs render immediately
  from saved authored snapshots while backend validation streams descriptor,
  drift, diagnostics, update-proposal, and graph-summary events.
- Live validation events are workflow-service DTOs with an explicit scope:
  graph-scoped for summary events and node-scoped for descriptor, drift,
  diagnostic, and update-proposal events. Do not use shared unscoped validation
  events as a transport or compatibility path.
- Correlate live validation with a backend-issued validation session id plus
  the same graph fingerprint revision used by graph edit sessions. Frontend
  must ignore stale events for old sessions or revisions. Event sequence
  numbers remain monotonic within each validation session; graph identity must
  not use a separate numeric revision or a translation map.
- Graph-level enqueue gating uses a backend-owned validation summary with a
  status enum, executable boolean, typed disabled reasons, and diagnostic
  counts. Frontend must not infer queue eligibility from raw diagnostics.
- Runtime/device constraints narrow interface validation but do not make the
  resolver choose the final runtime/device. Explicit invalid constraints block
  enqueue; typed alternatives are advisory repair suggestions only.
- Optional explicit constraints use strict boundary semantics: missing, null, or
  blank-string values are absent; wrong JSON types and nonblank unparsable
  strings are invalid diagnostics. Apply this to `task_kind`, `runtime`,
  `device`, and future typed trait inputs.
- Descriptor task kind is authoritative for scheduler materialization.
  Graph-authored `task_kind` is an optional resolver constraint only; if the
  explicit task kind cannot be satisfied, validation blocks before enqueue
  rather than letting scheduler projection reinterpret raw node data.
- Runtime/device alternatives are included only when needed to explain or
  repair an explicit invalid constraint. Full runtime support summaries are a
  future advanced inspector, not the first implementation.
- Existing graph `node.data.definition` dynamic-port overlays are not the
  canonical persisted inference interface after this milestone. For inference
  nodes, the authored interface snapshot is the persisted editor shape; any
  `definition` payload that remains during integration must be a generated
  projection from the authored snapshot/current descriptor and must not be
  treated as executable validation input.
- The existing static `llm-inference` all-port descriptor is retired as the
  graph-visible model interface. The generic inference node may keep only
  bootstrap/control ports needed before model resolution, such as
  `pumas_model_ref`, optional task kind, optional runtime/device constraints,
  and diagnostics. Task-specific prompt, image, mask, sampler, result, and
  runtime-condition ports come from the backend descriptor/authored snapshot.
- Existing `inference_settings` JSON canonicalization and `expand-settings`
  frontend-owned dynamic port sync are legacy inference-interface paths. They
  must be deleted or rewritten to consume backend descriptors as presentation
  plumbing only; they must not remain a successful alternate interface source.
- Retire `puma-lib` authoring in two validated stages. First, make `puma-lib`
  graph authoring model-ref-only: saved graph/node data and successful node
  semantics carry `pumas_model_ref` plus display-only identity, not executable
  paths, `entry_path`, package facts, runtime hints, load targets, or
  `inference_settings`. Then wire live validation so selecting or changing a
  Pumas model starts descriptor validation and overlays pending/stale/
  unavailable/invalid state in the editor.
- Model-ref binding extraction must be strict. Reject duplicate incoming
  `pumas_model_ref` bindings, verify the source handle and source node/type can
  provide a Pumas model ref, and report connected-versus-inline disagreement as
  a typed diagnostic instead of silently choosing a winner.
- Synchronous validation and later live validation share one workflow-service
  publisher core. The core snapshots the graph, releases the graph/session lock,
  resolves descriptor projections, records current graph/node validation state,
  and returns the same bounded projection records the event-driven path will
  publish. Lightweight events may carry fingerprints for routing, but a caller
  must be able to obtain descriptor/authored-port data from the validation
  response or current validation-state owner without resolving again in Tauri or
  frontend code.
- Existing `PortOptionsProvider`, selection-input, and option-cache plumbing may
  be reused only to render backend-projected option rows. Typed option identity,
  availability, diagnostics, and defaults remain owned by
  `pantograph-inference-interface-contracts`. If port-options plumbing is used
  for descriptor-backed rows, cache keys or invalidation must include the
  descriptor fingerprint or validation session/revision.
- Backend validation summary is the queue-admission authority. The frontend
  disables submit/enqueue from the latest backend summary, and workflow-service
  must run the same descriptor-backed readiness validation before queue
  admission so UI and backend cannot diverge. That check must happen before the
  run is inserted into the scheduler queue or queue-placement diagnostics are
  emitted.
- Runtime-host input contract alignment is required before production dispatch.
  Runtime-host execution requests must carry scheduler-selected handoff plus
  typed, path-free materialized inputs and artifact/result references assembled
  by workflow-service; graph paths, Pumas package facts, and scheduler-owned
  payload guesses remain out of the runtime-host request.

## Descriptor Contract

Planning names:

```rust
pub struct InferenceInterfaceDescriptor {
    pub contract_version: u32,
    pub model_ref: PumasModelRef,
    pub task_kind: InferenceTaskKind,
    pub descriptor_fingerprint: InferenceInterfaceFingerprint,
    pub inputs: Vec<InferencePortDescriptor>,
    pub outputs: Vec<InferencePortDescriptor>,
    pub availability: InferenceAvailability,
    pub diagnostics: Vec<InferenceInterfaceDiagnostic>,
}

pub struct InferencePortDescriptor {
    pub port_id: InferencePortId,
    pub label: String,
    pub value_type: InferenceValueType,
    pub direction: InferencePortDirection,
    pub requirement: InferencePortRequirement,
    pub default_value: Option<InferenceDefaultValue>,
    pub options: InferencePortOptions,
    pub availability: InferenceAvailability,
    pub conditions: Vec<InferencePortCondition>,
    pub diagnostics: Vec<InferenceInterfaceDiagnostic>,
}
```

Resolver request planning shape:

```rust
pub struct ResolveInferenceInterfaceRequest {
    pub model_ref: PumasModelRef,
    pub task_kind: Option<InferenceTaskKind>,
    pub requested_runtime: Option<InferenceRuntimeConstraint>,
    pub requested_device: Option<InferenceDeviceConstraint>,
}
```

Validator request planning shape:

```rust
pub struct ValidateInferenceNodeRequest {
    pub node_id: WorkflowNodeId,
    pub descriptor_fingerprint: CapabilityFingerprint,
    pub model_input: PumasModelRef,
    pub connected_inputs: Vec<InferenceNodeInputBinding>,
    pub literal_values: Vec<InferenceNodeLiteralValue>,
    pub selected_options: Vec<InferenceNodeSelectedOption>,
}
```

Implementation may choose shorter names, but the contract must remain typed,
versioned, bounded, path-free, and `serde(deny_unknown_fields)` at any
persisted or IPC boundary. Values with external or media payloads should use
typed artifact/result references where needed rather than embedding paths or
large blobs in graph JSON.

Value categories:

```rust
pub enum InferenceValueType {
    Scalar(InferenceScalarType),
    Artifact(InferenceArtifactType),
    Reference(InferenceReferenceType),
    Constraint(InferenceConstraintType),
}
```

Typed options:

```rust
pub enum InferencePortOptions {
    Any,
    None,
    Enum { values: Vec<InferenceOptionValue> },
    NumericRange {
        min: InferenceNumericValue,
        max: InferenceNumericValue,
        step: Option<InferenceNumericValue>,
        default: Option<InferenceNumericValue>,
    },
}
```

Availability:

```rust
pub struct InferenceAvailability {
    pub status: InferenceAvailabilityStatus,
    pub reasons: Vec<InferenceAvailabilityReason>,
}
```

Diagnostics remain bounded and typed, but they are attached to descriptors,
ports, options, drift reports, and validation events rather than embedded in
every availability value. This keeps the coarse availability value small enough
for UI state and cache keys while preserving detailed diagnostics at the
owning object. Public enums in this contract should be `#[non_exhaustive]`
when future variants are expected, and serde casing must be explicit.

## Persisted Authored Snapshot

Saved workflow graph files must store a minimal authored interface snapshot in
the generic inference node data. This keeps the graph self-contained enough to
reopen with the same visible port shape, preserve existing edges, and explain
interface drift after Pantograph, Pumas facts, runtime availability, or
inference support changes.

The snapshot may include:

- descriptor fingerprint and resolver contract version;
- authored input and output port summaries;
- port id, direction, value type, requirement, display label when needed,
  default marker or small scalar default when needed, and last known
  availability status/reason codes.

The snapshot must not include:

- Pumas package facts;
- executable model paths or runtime load targets;
- worker settings;
- scheduler decisions;
- full runtime-specific API schemas;
- large artifacts or media payloads.

Current backend-resolved descriptors remain the only executable validation
source. Authored snapshots preserve editor shape and diagnostics only.

For inference nodes, this authored snapshot supersedes the existing
`node.data.definition` dynamic-port overlay as the persisted interface source.
During staged integration, `node.data.definition` may exist only as a generated
view used by current graph rendering/validation code until those consumers are
updated. It must not be edited by the frontend as semantic inference state, and
it must not be accepted as an executable fallback when the authored snapshot or
current descriptor fails validation.

## Drift And Update UX

When a graph is opened or edited:

```text
load saved graph
  -> render authored ports immediately from persisted node data
  -> start live backend validation session
  -> resolve current descriptor
  -> compare authored snapshot with current descriptor
  -> stream typed drift diagnostics and validation summary
```

Drift report planning shape:

```rust
pub struct InferenceInterfaceDriftReport {
    pub node_id: WorkflowNodeId,
    pub authored_fingerprint: CapabilityFingerprint,
    pub current_fingerprint: CapabilityFingerprint,
    pub status: InferenceInterfaceDriftStatus,
    pub changes: Vec<InferenceInterfaceDriftChange>,
    pub blocking_diagnostics: Vec<InferenceInterfaceDiagnostic>,
}
```

Backend-owned graph patch proposals use typed graph operations in workflow
graph/service contracts. The frontend renders the proposal visually, shows
affected ports/edges/literals, and applies it only after user confirmation.
The frontend must not compute semantic graph mutation itself.

Live validation event planning shape. This DTO lives in workflow-service, not in
`pantograph-inference-interface-contracts`:

```rust
pub struct WorkflowGraphInferenceValidationEvent {
    pub session_id: DraftGraphValidationSessionId,
    pub graph_revision: DraftGraphRevision,
    pub sequence: ValidationEventSequence,
    pub scope: WorkflowGraphInferenceValidationEventScope,
    pub payload: WorkflowGraphInferenceValidationEventPayload,
}

pub enum WorkflowGraphInferenceValidationEventScope {
    Graph,
    Node { node_id: WorkflowNodeId },
}
```

Summary payloads must be graph-scoped. Descriptor, drift, diagnostic, and
update-proposal payloads must be node-scoped. The shared contract crate may own
the payload DTOs that are not graph-mutation-specific, but it must not expose an
alternate unscoped event stream.

Graph-level enqueue gating planning shape:

```rust
pub struct DraftGraphValidationSummary {
    pub status: DraftGraphValidationStatus,
    pub executable: bool,
    pub enqueue_disabled_reasons: Vec<ValidationDisabledReason>,
    pub diagnostics_count: u32,
    pub blocking_diagnostics_count: u32,
}
```

Validation is non-blocking for editing. It is authoritative for enqueue and
execution.

Validation sessions should reuse existing graph-session response and event
transport patterns where practical, but they must not hold graph/session locks
while resolving Pumas or inference facts. The workflow-service should snapshot
draft graph state under the relevant lock, release it, resolve asynchronously,
then publish events only when the validation session id and graph fingerprint
revision are still current. The workflow-service validation-session lifecycle
owner must cancel or supersede in-flight validation for replaced graph
revisions, stop accepting validation work when a graph/session closes, and
observe task errors before reporting final session state.

## Resolution Flow

```text
generic inference node receives or changes model reference
  -> graph editor submits draft node/model/constraint state
  -> workflow-service resolves Pumas model/artifact/package facts
  -> workflow-service queries inference/runtime capability facts
  -> resolver returns InferenceInterfaceDescriptor plus diagnostics
  -> graph editor renders typed inputs/outputs and unavailable reasons
  -> draft save persists editable graph/history without executable authority
  -> executable publish consumes the same descriptor validation contract
  -> execution revalidates descriptor fingerprint before task materialization
  -> workflow-service materializes typed runtime-host inputs after readiness
```

Missing required inputs fail draft validation and executable publish/admission
when known. They do not block Draft Save, because users must be able to persist
and reopen invalid in-progress graphs. Missing optional inputs are filled from
descriptor defaults at materialization time or omitted only when the
runtime-host input contract explicitly defines omission as the same canonical
default. If facts become stale between graph editing and execution, executable
publish/admission or execution revalidation fails with typed diagnostics
instead of falling back to previously rendered ports.

## Validation Uses

- **Draft graph validation:** validates unsaved graph state and returns
  node/port diagnostics for editor display.
- **Draft save persistence:** verifies persisted graph JSON shape,
  canonicalization, versioning, and authored-history integrity. It may persist
  invalid or non-executable inference graphs, and it must not create, mutate,
  infer, or cache executable scheduler authority.
- **Executable publish validation:** verifies required inputs, optional
  defaults, valid options, connected upstream output types, explicit
  runtime/device/trait constraints, current validation-session identity, graph
  revision, descriptor fingerprint, and validation summary before a workflow
  version can be marked executable or submitted.
- **Task graph projection:** stores only path-free model refs, typed
  constraints, resolved descriptor task kind, descriptor fingerprint, and
  bindings needed for scheduler task orchestration. Projection must consume the
  current descriptor-backed validation state rather than reinterpreting raw
  graph `task_kind`, `runtime`, `device`, or trait JSON as executable truth.
- **Input materialization:** combines connected upstream task results, graph
  literal values, and descriptor defaults into typed runtime-host execution
  inputs.
- **Pre-dispatch revalidation:** confirms the descriptor fingerprint and
  runtime-relevant availability facts before scheduler dispatch selection.
- **Queue admission:** consumes the same descriptor-backed validation summary
  used by the editor, and rejects non-executable graphs before enqueue rather
  than relying only on structural stale-graph diagnostics. Queue admission must
  run before queue insertion, queue-placement event recording, and scheduler task
  graph materialization.

## No-Fallback Requirements

- Do not create separate port-discovery and validation logic.
- Do not hardcode image-specific inference ports in the generic graph editor.
- Do not pass full Pumas package facts or executable paths through graph node
  data.
- Do not let the scheduler own prompt/image/mask/settings payloads.
- Do not encode runtime-specific APIs as untyped metadata bags.
- Do not preserve retired model-path or planned-inference launch paths as a
  successful fallback when descriptor resolution fails.

## Risks And Mitigations

- **Risk:** The new descriptor system becomes a second dynamic-port system next
  to `node.data.definition` and `inference_settings`.
  **Mitigation:** Treat authored snapshots as the only persisted inference
  interface source, project them into old rendering paths only as a temporary
  generated view for current rendering/validation code, and delete/rewrite the
  old paths before the milestone is complete. The projection must not be an
  executable fallback or compatibility route.
- **Risk:** Frontend draft validation drifts from backend save/submit
  validation.
  **Mitigation:** The backend validation summary is the only enqueue authority;
  frontend UI state can display pending or stale validation, but cannot infer
  executability.
- **Risk:** Async resolver work introduces stale event races or lock
  contention.
  **Mitigation:** Use validation session ids, graph fingerprint revisions,
  event sequence numbers, lock-free resolution after snapshotting, and
  stale-event rejection tests.
- **Risk:** The contract crate accidentally imports runtime or lookup behavior.
  **Mitigation:** Keep the crate DTO-only, document dependencies in its README,
  and add dependency-direction review to the first implementation slice.
- **Risk:** Runtime-host input alignment couples graph-editor drift contracts
  to dispatch execution.
  **Mitigation:** Runtime-host owns a separate execution input contract that is
  informed by descriptor value categories but does not import graph patch,
  drift, or UI validation contracts.

## Staged Implementation

1. Add the descriptor DTOs and validation errors in the canonical backend
   contract owner with serde fixtures, dependency-direction review,
   source-directory README updates, and default/no-default/all-features checks.
2. Retire the shared unscoped validation event/stream DTOs. Keep shared
   descriptor, authored snapshot, drift, diagnostic, option, and validation
   summary DTOs in `pantograph-inference-interface-contracts`; keep the scoped
   live event/session envelope in workflow-service. Update README/serde fixtures
   and source-search verification in the same slice.
3. Replace `puma-lib` graph authoring with a model-ref-only intermediate slice.
   Saved graph/node semantics and successful node outputs carry
   `pumas_model_ref` and display identity only; executable paths, `entry_path`,
   package facts, runtime hints, load targets, and `inference_settings` are
   removed from graph semantics before live validation UX is added. Before this
   code slice removes graph-authored paths, replace the dependency-requirements
   hydration path that currently builds `ModelDependencyRequest` from
   `modelPath`. Use the existing canonical path-free
   `pantograph-dependency-planning::DependencyPlanningRequest` contract, or
   evolve it if a required typed field is missing. Tauri command handlers must
   only decode/forward requests and encode responses; workflow-service and
   dependency-planning boundaries own validation/request semantics. Do not adapt
   the canonical request back into `ModelDependencyRequest`, `ModelRefV2`,
   `modelPath`, or `model_path`.
3a. Replace validation-session numeric graph revision identity with the
   graph-session `WorkflowGraphRevision` fingerprint and introduce the first
   workflow-service current-validation-state owner keyed by
   `graph_session_id + graph_revision` with optional `validation_session_id`.
   This is the option-2 implementation path with option-3 discipline: start
   narrow enough to unblock dependency-environment actions, but expose a
   single owner-shaped API that submit gating and scheduler admission can later
   consume without adding parallel validation-summary caches or frontend-owned
   policy.
   Standards sequencing for this step: first create a focused validation-state
   module and move dependency-action intent freshness checks out of
   `graph/session.rs`; then replace the numeric revision field in live
   validation DTOs. Tests must prove stale graph revisions are rejected through
   the owner API, old numeric revision payloads no longer deserialize after the
   replacement, and the graph lock is not held across resolver work.
3b. Resolve dependency-environment actions through a graph-authored
   sidecar/control-node association. A dependency-environment action target is
   a dependency-environment node associated with exactly one inference node; it
   is not connected to inference result outputs, does not consume generated
   media/text data, and is not an ordinary node-engine execution step.
   Use a typed association/control port pair instead of introducing graph edge
   kinds in this milestone: the dependency-environment descriptor exposes one
   association output and the canonical `llm-inference` bootstrap descriptor
   exposes one optional association input with the shared
   `dependency_environment_sidecar` handle. Add a first-class
   `DependencyEnvironmentSidecar`/`dependency_environment_sidecar` port value
   type across node-engine, node contracts, workflow-service graph DTOs,
   workflow-node contract conversion, and frontend port typing. Do not use
   generic JSON, component, any, or raw string conventions for this
   association. Workflow-service must own a typed dependency action subject
   resolver that validates the target node type, proves the single associated
   inference node from that typed association edge, joins that inference node
   to current descriptor validation state, and returns typed diagnostics for
   missing, duplicate, wrong-handle/type, stale, unavailable, invalid, or
   ambiguous subjects. Frontend and Tauri code continue to send only graph
   action intent and must not infer subjects from `modelPath`/`model_path`,
   package facts, platform context, or arbitrary edge guesses.
3c. Clean up dependency-environment ownership before request derivation. Remove
   graph-facing dependency-environment ports that carry model/task/backend/
   platform authority (`pumas_model_ref`, `model_id`, `model_type`,
   `task_type_primary`, `backend_key`, `platform_context`, and direct
   `dependency_requirements` input). Keep only sidecar-owned user choices and
   display state: selected binding ids, mode, manual override patches,
   backend-issued dependency status, backend-issued environment reference, and
   action/activity presentation state. Backend-issued status, requirements,
   environment references, and activity are display/history projections only;
   frontend code must not write them optimistically or use them as request
   authority. Retire frontend path-era subject inference and tests that treat
   `modelPath`/`model_path`, package facts, backend keys, or platform context
   as successful dependency action inputs. Canonical dependency readiness flows
   through dependency planning preflight/readiness proof into scheduler dispatch
   and runtime handoff; sidecar association edges must not be projected into
   node-engine task input bindings, and once the scheduler proof path is wired,
   retire node-engine/embedded-runtime `environment_ref` input gates as
   dependency-admission authority.
3d. Add a backend-owned dependency requirements proof before deriving
   `DependencyEnvironmentRequest`. Updated 2026-05-26: the selected design is
   the dependency-planning producer option. `pantograph-dependency-planning`
   owns the producer API that derives the dependency requirements id/fingerprint,
   proof status, and typed diagnostics from a validated, path-free
   `DependencyPlanningRequest`. Workflow-service may store the current proof in
   live validation state and may call the producer for `Resolve`, but it must
   not synthesize requirements ids, derive dependency-planning policy, or build
   compatibility DTOs. The producer is a pure contract/domain producer unless
   the caller supplies typed dependency availability facts; it must not call
   Pumas, inspect files, select runtime/device policy, or infer package
   readiness from missing local state. The proof is bounded and path-free:
   associated inference node id, graph revision, validation session id,
   descriptor fingerprint, validated Pumas model ref, task kind, validated
   runtime/device constraints, dependency-planning-local trait intents,
   selected binding ids, dependency override fingerprint, platform key,
   dependency requirements id or fingerprint, proof status, and
   `DependencyPlanningDiagnostic` rows. The requirements id must be derived from
   canonical typed fields with a stable hash, not by concatenating user/model
   strings that may violate the validated identifier grammar. It must not
   contain executable paths, package facts, frontend display state, runtime load
   targets, media payloads, scheduler trait-setting DTOs, or scheduler dispatch
   decisions. Workflow-service must snapshot and validate sidecar-authored
   selected binding ids and manual override patches before releasing graph
   state and before calling the producer; it must store dependency-planning
   diagnostics in the proof and map them to `InferenceInterfaceDiagnostic` only
   at graph/action response boundaries. `Resolve` may create or refresh the
   current proof through the dependency-planning producer; `Check` and `Install`
   must require an existing current proof and fail closed with typed diagnostics
   when it is missing, stale, unavailable, or invalid. The graph editor and
   Tauri transport continue to send action intent only.
   Standards constraints for this producer slice: new producer DTOs, status
   enums, diagnostics, and errors must be public contract types re-exported
   from `pantograph-dependency-planning::lib`, use typed constructors or
   `TryFrom` validation at the boundary, return a crate-specific error enum,
   and mark extension-prone public enums/structs `#[non_exhaustive]`. Do not
   add `Result<T, String>`, `anyhow`, raw metadata maps, or async APIs for pure
   producer logic. Workflow-service must parse sidecar JSON ports once into
   validated dependency-planning types before producer calls and must never
   pass raw JSON through the producer boundary. If the requirements id stable
   hash uses `blake3`, first normalize `blake3` as a workspace dependency and
   convert direct member declarations to `{ workspace = true }` in the same
   dependency-hygiene sub-slice; no new hashing crate is allowed without a
   written dependency evaluation. Workflow-service must snapshot graph state
   under lock, release it before producer/fact work, then record the proof only
   after graph revision and validation session still match current state.
4. Tighten request extraction before it feeds live validation. This is a hard
   prerequisite for the synchronous validation publisher and the later
   event-driven path: use one
   workflow-service model-ref binding resolver, reject duplicate incoming
   bindings, validate source handle/type, report connected-versus-inline
   disagreement, and treat wrong-type or unparsable explicit constraints as
   invalid diagnostics rather than absent values.
5. Add the synchronous workflow-service validation publisher core. It snapshots
   the draft graph under lock, releases the lock, extracts strict model-ref and
   constraint requests, resolves descriptor projections, builds scoped
   validation events and graph summary, stores current graph/node validation
   records in the validation-state owner, and returns a validation session plus
   bounded per-node projection records. The core must use
   `resolve_inference_interface_projection` rather than duplicating descriptor,
   authored-snapshot, or summary rules. Tauri and frontend code are not touched
   in this slice except by tests or generated bindings that prove no business
   policy moved there.
   - Implemented 2026-05-26 in
     `graph/inference_interface_publication.rs`,
     `graph/session_inference_validation_api.rs`, and
     `graph/inference_validation_state.rs`. The implementation records
     node-scoped projection state for later dependency and scheduler admission
     work while keeping graph editing non-blocking.
5a. Add the workflow-service inference-interface fact-provider boundary before
    event-driven validation starts from graph edits. This is option 2 with
    option 3 discipline: define a focused provider API that accepts typed graph
    resolution inputs and returns per-node `InferenceInterfaceResolverFacts`
    from Pumas model/artifact state, inference capability facts, and runtime
    availability. The default implementation must return typed
    unavailable/not-implemented facts instead of guessing. Tests may inject
    facts directly, but frontend and Tauri callers must not pass raw Pumas
    facts, runtime facts, package summaries, load targets, paths, or capability
    blobs as validation authority.
    - Implemented 2026-05-26 in `graph/inference_interface_facts.rs` and the
      edit-session publisher API. `GraphSessionStore` now owns the injectable
      provider and the public session publisher no longer accepts caller-supplied
      resolver facts.
5b. Add the event-driven validation lifecycle owner before wiring UI event
    delivery. It owns validation session startup, cancellation, supersession,
    bounded event/state buffers, overflow/backpressure diagnostics, task
    observation, and cleanup on graph/session close. It reuses the synchronous
    publisher core plus the workflow-service fact-provider boundary and changes
    only scheduling/transport behavior.
6. Add the thinnest cross-layer acceptance slice proving model ref input can
   resolve a descriptor, project authored ports, produce a validation summary,
   and gate submit/admission without invoking legacy inference settings.
7. Add a generated projection from authored/current descriptors into the
   existing effective node definition path only where needed for staged
   rendering and graph validation. For inference nodes, this projection must
   be derived from the authored snapshot or current descriptor, not from
   frontend-authored `node.data.definition`.
8. Promote the fact-provider boundary into a dedicated resolver service only
   when the implementation needs broader orchestration than a provider can keep
   simple. That later service may own request extraction, Pumas/runtime fact
   lookup, descriptor publication, validation state recording, and lifecycle
   hooks, but it must still reuse the sync publisher core and must not create a
   second descriptor-resolution path.
9. Wire graph editor draft validation to render descriptor ports and disabled
   reasons without frontend-inferred family logic.
10. Wire the Draft Save versus Executable Publish boundary explicitly. Draft
   Save consumes only persisted-graph schema/canonicalization and authored
   history checks and may save invalid editable graphs. Executable Publish
   consumes the descriptor validation contract and rejects missing, stale,
   invalid, unavailable, unresolved, blocked, or contract-incompatible
   inference interfaces before any submit/queue authority is created.
11. Wire scheduler task projection/materialization and runtime-host input
    assembly to consume current descriptor-backed validation state: resolved
    descriptor task kind, descriptor defaults, typed constraints, descriptor
    fingerprint, and typed upstream values. Raw graph `task_kind`, runtime,
    device, and trait fields remain resolver inputs only and must not be parsed
    again as execution authority during scheduler projection.
    - Re-plan boundary recorded 2026-05-26: current task graph projection has
      only `WorkflowGraph` as input and still parses raw `node.data.task_kind`.
      Add a descriptor-backed task projection input before deleting that raw
      execution-authority path so the scheduler can consume current validation
      state without preserving legacy graph semantics.
    - Decision recorded 2026-05-26: implement the descriptor-backed projection
      input first. The input must carry per-inference-node current validation
      records into scheduler task graph construction and must make missing,
      stale, unavailable, or invalid descriptors typed blocking diagnostics.
      Queue admission/session orchestration remains the later owner of
      validation-state lookup and will call this projection API once ready.
    - Implementation update 2026-05-26: workflow-service now has a
      descriptor-backed scheduler task projection API and no longer parses raw
      inference-node graph fields as scheduler authority. Remaining work is the
      queue-admission/session owner that supplies current validation records to
      that API before scheduler placement.
    - Implementation update 2026-05-26: current inference-validation state now
      exposes descriptor-backed scheduler projections through a graph-session
      wrapper that checks the current canonical graph revision. Missing, stale,
      or non-executable validation state fails closed before scheduler
      placement. Remaining work is wiring queue admission/session orchestration
      to call this wrapper for submitted inference workflows.
    - Re-plan boundary recorded 2026-05-26: saved workflow submissions do not
      have a canonical validation-state address yet. Queue admission loads
      saved graphs by workflow id, while current validation state is graph-edit
      session scoped. Add a saved executable validation snapshot at executable
      publish time, then have queue admission consume that snapshot before
      scheduler projection. Do not keep whole-run host execution or raw graph
      parsing as compatibility behavior.
    - Decision recorded 2026-05-26: implement the saved executable validation
      snapshot as the authority for saved workflow submissions. The snapshot is
      produced by a service-level executable publish boundary, keyed under the
      existing attribution `WorkflowVersionId`, and consumed by queue admission
      before queue insertion, queue-placement event recording, or scheduler task
      graph materialization. Draft `save_workflow` remains graph persistence and
      does not create execution authority. The snapshot stores bounded typed
      descriptor projection authority derived from backend validation
      publication records; graph-editor authored shape remains in the saved
      graph, and Pumas facts/paths, frontend state, Tauri payloads, raw
      inference-node fields, lossy current-state caches, and whole-run
      runtime-host execution are not valid fallback sources. Missing snapshot
      storage, missing/stale/non-executable snapshots, and contract mismatches
      fail closed with typed diagnostics.
    - Standards iteration recorded 2026-05-26: implementation must treat the
      saved executable validation snapshot as an executable boundary contract
      with validated typed DTOs, sync-core/async-shell factoring,
      transactional or idempotent publish persistence, no frontend-owned
      executable state, no Tauri business logic, cross-binding serde round-trip
      checks, isolated sqlite/temp workflow roots in tests, and a failing-first
      vertical acceptance path from executable publish through queue admission.
12. Add queue-admission readiness validation that consumes the backend validation
    summary and fails closed before enqueue, queue-placement event recording, or
    scheduler task graph materialization when inference descriptors are pending,
    stale, unavailable, unresolved, or blocked.
13. Add pre-dispatch descriptor fingerprint revalidation before runtime-host
   dispatch selection.
14. Delete or rewrite any remaining retired inference-node port tables,
    model-path-derived support checks, or planned-inference validation branches
    replaced by this resolver.
15. Delete or rewrite `inference_settings` JSON canonicalization and
    `expand-settings` dynamic inference-interface paths once the descriptor
    projection owns graph-visible inference ports.

## Implementation Status

- 2026-05-25: Completed staged implementation step 1 as the first validated
  vertical slice. Added `pantograph-inference-interface-contracts` as a
  DTO-only crate, registered it in the workspace, documented its dependency
  and ownership boundaries, and added serde fixtures/tests for ready
  descriptors, authored snapshots, blocking drift reports, and backend-owned
  validation summaries. The crate uses existing path-free
  `pantograph-dependency-planning::PumasModelRef`, `RuntimeIntentId`, and
  `DeviceIntentId` contracts rather than creating parallel Pumas or scheduler
  identifiers.
- Verification passed: `cargo fmt -p
  pantograph-inference-interface-contracts`; `cargo test -p
  pantograph-inference-interface-contracts`; `cargo check -p
  pantograph-inference-interface-contracts`; `cargo check -p
  pantograph-inference-interface-contracts --no-default-features`; `cargo
  check -p pantograph-inference-interface-contracts --all-features`; and
  targeted source search proving the new crate does not import
  workflow-service, scheduler, runtime-host contracts, Pumas library/runtime
  lookup, inference execution, or frontend policy.
- Remaining next slices: first retire shared unscoped validation event/stream
  DTOs and update contract-crate verification; then replace `puma-lib` graph
  authoring with model-ref-only semantics; then tighten request extraction for
  strict model-ref binding and explicit constraint diagnostics. After those
  boundary slices pass, implement the thinnest cross-layer resolver/validation
  acceptance path: model ref input resolves a descriptor, projects authored
  ports, produces a backend validation summary, and gates submit/admission
  without invoking retired `inference_settings`, `expand-settings`, static
  all-port descriptors, or model-path paths.
- 2026-05-25: Next-slice investigation found a re-plan boundary before
  retiring the static all-port `llm-inference` descriptor. The same static
  ports are currently consumed by `workflow-nodes` composed `tool-loop`
  contracts and inference payload metadata projection tests. Removing the
  static task/model ports now is the correct no-legacy direction, but doing so
  without redesigning that internal composition would invalidate the tool-loop
  contract mappings for `prompt`, `tools`, `response`, and `tool_calls`.
  Keeping the static ports while adding authored snapshot projection would
  preserve the retired fallback path. The next implementation slice must
  decide and execute a clean transition for internal composed-node use before
  static `llm-inference` task ports are deleted.
- 2026-05-25: Re-plan decision selected. Agent/tool loops remain a required
  workflow capability, but they must become scheduler-owned agent loop
  primitives rather than composed wrappers over the retired static
  `llm-inference` port shape. The graph-visible `tool-loop` node should expose
  stable loop-level inputs and outputs such as model reference, prompt,
  system prompt, tools, max turns, optional runtime/device constraints,
  response, turns, tool calls, and diagnostics. Workflow-service/scheduler
  then expands each loop turn into typed inference and tool tasks only when
  the previous turn requires it, stores tool results as scheduler task
  results, supports early termination, and interleaves other users' work
  between turns. This avoids forcing users to draw fixed-length chains or
  clients to resubmit workflow runs as a loop workaround.
- Implementation staging: first remove `tool-loop`'s dependency on static
  `llm-inference` composed-contract mappings and make any not-yet-executable
  path fail closed with typed diagnostics. Then add the scheduler-owned
  agent-loop task graph expansion after descriptor-backed generic inference
  can materialize one real inference turn.
- 2026-05-25: Completed staged implementation step 2 as a validated vertical
  slice. Removed the shared unscoped `DraftGraphValidationEvent`,
  `DraftGraphValidationEventPayload`, and `DraftGraphValidationStreamState`
  DTOs from `pantograph-inference-interface-contracts`; kept shared descriptor,
  authored snapshot, drift, diagnostic, option, and validation summary DTOs
  intact; and documented that live validation event/session envelopes are
  workflow-service owned. No compatibility alias, shim, or alternate unscoped
  transport remains in the shared crate.
- Verification passed: `cargo fmt -p pantograph-inference-interface-contracts
  -- --check`; `cargo test -p pantograph-inference-interface-contracts`;
  `cargo check -p pantograph-inference-interface-contracts`; `cargo check -p
  pantograph-inference-interface-contracts --no-default-features`; `cargo
  check -p pantograph-inference-interface-contracts --all-features`; `cargo
  check -p pantograph-workflow-service`; `cargo check -p
  pantograph-workflow-service --no-default-features`; `cargo check -p
  pantograph-workflow-service --all-features`; targeted source search in
  `crates/` for retired unscoped validation event/stream DTO names; and `git
  diff --check`. At the time, workflow-service checks still reported the known
  `set_active_run_execution_plan` dead-code warning; that active-plan storage
  bridge was removed in the later 2026-05-31 Milestone 5c/5b cleanup slice.
- Remaining next slices: replace `puma-lib` graph authoring with model-ref-only
  semantics, then tighten request extraction for strict model-ref binding and
  explicit constraint diagnostics. After those boundary slices pass, implement
  the thinnest cross-layer resolver/validation acceptance path.
- 2026-05-25: Re-plan boundary found before the `puma-lib` model-ref-only code
  slice. `src-tauri/src/workflow/puma_lib_commands.rs` still hydrates selected
  puma-lib nodes by requiring a `modelPath` and using it to build
  `node_engine::ModelDependencyRequest`; the node-engine dependency request and
  related commands still require `model_path`. Removing `modelPath`,
  `entry_path`, package facts, and `inference_settings` from graph data now
  would either break dependency hydration or require a hidden synthesized path,
  which violates the no-fallback/no-legacy rule. The next step is to re-plan a
  model-ref-only dependency hydration contract before editing production
  puma-lib authoring.
- 2026-05-25: Re-plan decision selected. Replace the puma-lib dependency
  hydration boundary with the existing canonical path-free
  `pantograph-dependency-planning::DependencyPlanningRequest`, evolving that
  contract only if a required typed field is missing. Workflow-service owns
  graph/node validation and request assembly semantics; dependency-planning owns
  the DTO; host/runtime integration owns Pumas-approved load-target lookup.
  Tauri remains a transport adapter and must not contain dependency policy,
  Pumas fact resolution, scheduler/runtime selection, or hidden path synthesis.
  No adapter may translate the canonical request back into
  `node_engine::ModelDependencyRequest`, `ModelRefV2`, `modelPath`, or
  `model_path`.
- 2026-05-25: Implemented the first `puma-lib` hydration cleanup slice in the
  Tauri command module. Successful selected-model hydration now writes only
  model-ref/display node data (`modelName`, `model_id`, `pumas_model_ref`, and
  sanitized `selected_binding_ids`) and no longer emits graph-authored
  `modelPath`, executable entry paths, package facts, runtime hints,
  dependency bindings, dependency requirements, load-target facts, or
  `inference_settings`. `resolve_requirements=true` fails closed until the
  canonical path-free dependency-environment service boundary replaces the
  retired request. Verification passed: `cargo fmt`, `git diff --check`, and
  targeted source search. Verification blocked: `cargo test -p pantograph
  puma_lib_commands` was stopped at the time by the unrelated
  `WorkflowService::set_media_conversion_executor` compile error in
  `src-tauri/src/app_setup.rs`.
- 2026-05-31 update: the stale workflow-service media conversion injection was
  removed from Tauri app setup, so that compile blocker is resolved. The
  remaining media-conversion follow-up is to delete the unused Tauri managed
  media conversion adapter rather than keep desktop-owned business logic. Any
  future conversion feature must be introduced as a backend-owned service
  boundary and remain separate from inference-interface resolution,
  scheduler/runtime dispatch, and graph-authored model inputs.

## Open Design Decisions

- Decide whether existing `PortOptionsProvider`, selection-input, and option
  cache remain as descriptor-backed presentation plumbing or are replaced by a
  dedicated descriptor option renderer. They cannot own inference semantics.
- Decide the resolver service module/API shape inside workflow-service after
  inspecting current graph validation and options-provider boundaries.
- Decide the inference capability fact API used by the resolver without
  coupling it to scheduler policy.
- Decide the concrete graph patch operation owner and API shape for unsaved
  draft graphs versus persisted workflow files.
- Decide the live validation transport over the current app shell after
  inspecting existing frontend/Tauri subscription patterns. The event envelope
  ownership and graph/node scope are no longer open; only the delivery mechanism,
  concrete bounded-capacity/backpressure thresholds, and frontend store
  integration remain to choose. Cleanup, cancellation, supersession, and task
  observation are workflow-service lifecycle-owner requirements.
- Align runtime-host execution input through the Milestone 5b canonical
  `DependencyReadinessProofEnvelope` and materialized runtime-input handoff. The
  runtime-host request must consume descriptor-backed materialized values and
  readiness proof facts, but must not import graph-editor drift/patch
  contracts, saved authored snapshots as execution authority, frontend
  validation state, or reduced workflow execution-plan projections.

## Verification Strategy

- Contract round-trip fixtures for descriptors, ports, defaults, options,
  availability, and diagnostics.
- Dependency-direction checks for the new contract crate proving it does not
  import workflow-service, scheduler, runtime-host, Pumas lookup/runtime wiring,
  inference execution, or frontend policy.
- Default, no-default-features, and all-features checks for affected public
  Rust crates, especially any new contract crate.
- Source-directory README or ADR updates for new/touched crate and module
  boundaries.
- First vertical-slice acceptance test proving model ref -> descriptor ->
  authored port projection -> validation summary -> submit/admission gate.
- Validation publisher tests proving the synchronous core snapshots the graph,
  releases locks before Pumas/inference/runtime fact lookup, records bounded
  graph/node validation state, rejects stale graph revisions and validation
  session ids, and returns typed diagnostics instead of partial state.
- Event-driven lifecycle tests proving validation sessions are cancelled or
  superseded on graph revision changes, cleaned up on session close, bounded
  event/state buffers apply the documented overflow/backpressure behavior, and
  task errors or panics are observed at the workflow-service owner.
- Contract-crate deletion tests/source search proving shared unscoped
  validation event/stream DTOs are not exported or used as transport, while
  shared validation summary serde fixtures still round trip.
- Model-ref-only `puma-lib` tests proving saved graph/node data, node metadata,
  Tauri option hydration, frontend mocks/templates, and node-engine successful
  semantics no longer expose executable paths, `entry_path`, package facts,
  runtime hints, load targets, or `inference_settings` as inference-interface
  inputs.
- Strict request-extraction tests for duplicate incoming `pumas_model_ref`
  bindings, invalid source handle/type, connected-versus-inline disagreement,
  missing/null/blank optional constraints, wrong-type explicit constraints, and
  nonblank unparsable constraint values.
- Scheduler projection/materialization tests proving resolved descriptor task
  kind is authoritative, graph-authored `task_kind` only narrows resolver
  selection, and unsatisfied explicit task constraints block before enqueue.
- Resolver tests for ready, unavailable, unsupported, unimplemented,
  ambiguous, stale, and missing selected-artifact states.
- Draft graph validation tests proving graph editor-visible ports come from
  backend descriptors.
- Save validation tests proving invalid, stale, or unavailable ports cannot be
  persisted as successful workflow intent.
- Task materialization tests proving required inputs block, optional defaults
  materialize, and connected upstream values are type-checked.
- Runtime dispatch tests proving runtime-host receives typed materialized
  inputs plus scheduler-selected handoff, not graph paths or scheduler-owned
  payloads.
- Search/deletion checks for retired port tables, `modelPath`/`model_path`,
  planned-inference launches, raw package-fact graph payloads, and untyped
  inference metadata bags.
- Search/deletion checks for retired `inference_settings` JSON interface
  paths, `expand-settings` descriptor ownership, and inference-node
  `node.data.definition` semantic fallbacks.
- Frontend submit-gating tests proving the toolbar and queue submission use the
  backend validation summary instead of inferring eligibility from local raw
  diagnostics.
- Frontend validation-store tests proving stale validation events/responses are
  ignored by session id, graph fingerprint revision, sequence, and node scope;
  graph display/editing remains available while validation is pending; and event
  subscriptions or any temporary timers are cleaned up on graph/session changes.
- Dependency ownership checks for any new crate/package or feature added by this
  milestone, including workspace dependency placement, minimal default features,
  and package-local command verification for touched frontend workspaces.

## Completion Criteria

- The generic inference node can display descriptor-resolved ports for a
  connected Pumas model without frontend family-specific inference logic.
- Saved workflow graph data contains only the minimal authored interface
  snapshot plus user-authored values/edges; it does not persist package facts,
  local paths, scheduler decisions, runtime load targets, or worker settings.
- Draft validation, executable publish, queue admission, materialization, and
  pre-dispatch revalidation all consume the same descriptor contract. Draft
  save persists editable graph/history and is not executable validation.
- Scheduler receives validated task intent and constraints, not prompt/image/
  mask/settings ownership or model-path resolution responsibility.
- Runtime-host receives scheduler-selected handoff plus typed materialized
  inputs, not graph state, package facts, local paths, or untyped payload bags.
- Retired static all-port, `inference_settings`, `expand-settings`, model-path,
  planned-inference, and `node.data.definition` semantic inference paths have
  been removed or rewritten with no successful fallback behavior.
- `puma-lib` graph authoring is model-ref-only, and shared unscoped
  validation event/stream DTOs are removed or retired so workflow-service is the
  only scoped validation event owner.
- Affected READMEs/ADRs, tests, fixture round trips, feature checks, frontend
  validation tests, and deletion searches pass.

## Re-Plan Triggers

- Descriptor resolution needs executable paths or full package facts in graph
  node data.
- Frontend draft validation cannot render dynamic ports without hardcoding
  model-family logic.
- Save validation and execution materialization require different contracts.
- Scheduler selection requires ownership of inference payload values.
- Runtime-specific features cannot be represented as typed optional traits,
  unavailable diagnostics, or explicitly scoped runtime conditions.
- Staged integration requires preserving `inference_settings`, `expand-settings`,
  or `node.data.definition` as successful semantic fallbacks for inference
  execution.
- Runtime-host dispatch cannot be expressed with typed materialized inputs plus
  scheduler handoff without passing graph state, Pumas package facts, or local
  paths through the runtime-host request.
- The new contract crate requires non-DTO runtime dependencies, expensive
  default features, or dependency direction that violates package-role
  boundaries.
- The first vertical slice cannot prove descriptor-driven rendering and
  enqueue gating without preserving retired inference-interface behavior.
- Static `llm-inference` task ports cannot be deleted because composed-node
  contracts or payload metadata still depend on them and no descriptor-backed
  replacement has been selected.
