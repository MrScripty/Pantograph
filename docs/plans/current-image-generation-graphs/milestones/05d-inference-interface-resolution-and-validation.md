# Milestone 5d: Inference Interface Resolution And Validation

**Goal:** Add the canonical backend resolver/validator that turns a connected
Pumas model reference into the typed input/output interface for a generic
inference node. The graph editor, workflow validators, scheduler task
materialization, and pre-dispatch runtime checks must all consume this same
contract.

This milestone is inserted after Milestone 5c task-level orchestration and
before the remaining Milestone 5b production runtime-host wiring. Milestone 6
PyTorch/diffusers execution must consume this generic system rather than
defining an image-only inference-node interface.

**Tasks:**

- [x] Add the dedicated `pantograph-inference-interface-contracts` crate as a
      DTO-only shared contract owner. It must not import workflow-service,
      scheduler, runtime-host, Pumas lookup, frontend rendering policy, or
      worker execution code.
- [x] Add the new crate README and any affected source-directory README/ADR
      updates in the same slices that introduce or change public boundaries.
      The contract crate README must document purpose, dependencies, invariants,
      API consumer contract, structured producer contract, and revisit triggers.
- [x] Verify contract-crate dependency direction before broad implementation.
      It may depend on stable path-free model-reference DTOs only when doing so
      does not pull in Pumas lifecycle, lookup, storage, runtime wiring,
      workflow-service, scheduler, runtime-host, inference execution, or
      frontend policy.
- [x] Define resolver and validator request contracts separately. Resolver
      input is model-focused (`PumasModelRef`, optional task kind, optional
      runtime/device constraints). Validator input consumes a descriptor
      fingerprint plus draft node bindings, literals, and selected options.
- [x] Define the shared `InferenceInterfaceDescriptor` contract with typed
      input/output ports, task kind, grouped value categories, typed defaults,
      typed option sets, availability, runtime conditions, capability
      fingerprint, and bounded diagnostics. The contract must be versioned,
      path-free, `serde(deny_unknown_fields)`, and covered by serde fixtures.
- [x] Define the grouped value categories: scalar, artifact, reference, and
      constraint. Defaults must be typed small scalar/default-marker values
      unless a later artifact-reference default contract is explicitly added.
- [x] Define typed bounded option sets with per-option availability and
      diagnostics. Do not use `serde_json::Value`, metadata bags, or
      runtime-specific option blobs.
- [x] Define availability as coarse status plus additive typed reason codes,
      with bounded diagnostics attached to the descriptor, port, option, drift,
      or validation event that owns the availability decision. Public enums
      should be `#[non_exhaustive]` where future variants are expected, and UI
      behavior should key primarily from the coarse status.
- [x] Define the minimal authored interface snapshot persisted directly in
      inference node data. It must preserve graph shape and drift diagnostics
      only: descriptor fingerprint, authored port summaries, labels when
      needed, value types, requirements, default markers or small scalar
      defaults, and last known availability status/reasons. It must not contain
      Pumas package facts, executable paths, runtime load targets, worker
      settings, scheduler decisions, full runtime API schemas, or large media.
- [x] Replace inference-node use of `node.data.definition` as a semantic
      dynamic-port source. For inference nodes, `node.data.definition` may
      remain only as a generated projection from the authored snapshot/current
      descriptor during staged integration, and must not be accepted as an
      executable fallback when descriptor validation fails.
- [x] Retire the current static all-port `llm-inference` interface. Keep only
      bootstrap/control ports required before model resolution, such as
      `pumas_model_ref`, optional task kind, optional runtime/device
      constraints, and diagnostics. All task/model-specific prompt, image,
      mask, sampler, result, and runtime-condition ports must come from the
      descriptor/authored snapshot.
- [x] Remove the built-in composed `tool-loop` registration that depended on
      static all-port `llm-inference` ports. `tool-loop` remains a stable
      authoring descriptor and direct execution must fail closed until
      scheduler-owned agent-loop orchestration is implemented from descriptor
      backed inference turns.
- [x] Define authored-versus-current drift report contracts in
      `pantograph-inference-interface-contracts`. Drift reports must identify
      added/removed ports, type changes, requirement/default/option changes,
      availability changes, task/runtime-condition changes, severity, and
      blocking diagnostics.
- [x] Define workflow graph/service-owned typed graph patch operations and
      update proposal contracts for "update to current interface." The
      inference-interface contract crate may provide drift types, but graph
      patch operations must live with graph mutation ownership.
- [x] Define live draft validation session contracts with backend-issued
      validation session ids, graph fingerprint revisions, event sequence
      numbers, descriptor/drift/diagnostic/update-proposal events, and a
      backend-owned validation summary. Re-plan update: the live-validation
      contract must use the same `WorkflowGraphRevision` fingerprint as graph
      edit sessions and `DependencyEnvironmentActionIntent`; the older numeric
      `client_graph_revision` shape must be replaced, not translated through a
      compatibility map. Event sequences remain monotonic within a validation
      session.
      - Numeric revision replacement completed on 2026-05-26:
        `WorkflowGraphInferenceValidationSession` and
        `WorkflowGraphInferenceValidationEvent` now carry
        `WorkflowGraphRevision` directly. Focused tests prove mismatched event
        revisions fail validation and retired `client_graph_revision` payloads
        are rejected by the typed serde boundary instead of translated.
- [x] Define the graph validation summary contract with status, executable
      boolean, typed enqueue-disabled reasons, diagnostics count, and blocking
      diagnostics count. Frontend must not infer enqueue permission from raw
      diagnostics.
- [x] Add a workflow-service resolver boundary that combines Pumas model and
      selected-artifact facts, inference capability facts, runtime availability
      facts, and optional graph-authored constraints into one descriptor. It
      must return typed unavailable/not-implemented diagnostics when facts or
      runtime support are missing rather than guessing from names or paths.
- [x] Retire shared unscoped validation event/stream DTOs from
      `pantograph-inference-interface-contracts` while keeping shared
      descriptor, authored snapshot, drift, diagnostic, option, and validation
      summary DTOs there. `WorkflowGraphInferenceValidationEvent` is the only
      live validation event envelope and must remain graph/node scoped.
- [x] Replace `puma-lib` graph authoring with the model-ref-only intermediate
      slice before live-validation UX wiring. Saved graph/node data and
      successful graph semantics carry `pumas_model_ref` plus display-only
      identity only; executable paths, `entry_path`, package facts, runtime
      hints, load targets, and `inference_settings` must not be graph semantics.
      The active workflow `PumaLibNode.svelte` now follows this boundary by
      projecting selectable backend port options into `modelName`, `model_id`,
      and `pumas_model_ref` only; path-shaped options fail closed. The
      `hydrate_puma_lib_node` Tauri command now accepts only `model_id` and
      selector access for saved-node rehydration, so path-shaped hydration and
      dependency selection state are no longer successful Puma-Lib paths.
- [ ] Replace the `puma-lib` dependency-requirements hydration boundary before
      removing graph-authored paths. The chosen design is to use the canonical
      path-free `pantograph-dependency-planning::DependencyPlanningRequest`
      contract, or evolve that contract if a required typed field is missing.
      Tauri command code must only decode/forward requests and encode responses;
      it must not construct dependency policy, resolve Pumas facts, synthesize
      `modelPath`, or adapt the canonical request back into
      `node_engine::ModelDependencyRequest`. The first backend slice replaces
      the old Tauri action command with
      `DependencyEnvironmentRequest -> DependencyEnvironmentResult` and returns
      typed `not_implemented` diagnostics until a canonical dependency
      environment service exists. Re-plan decision: use option 3 for the
      frontend/action boundary. Dependency-environment actions must consume the
      same backend descriptor/validation summary used by graph validation,
      submit gating, and scheduler admission. The frontend sends action intent
      only, such as workflow/session node id, graph revision, validation
      session id when available, and the requested resolve/check/install action.
      It must not build `DependencyPlanningRequest`,
      `DependencyEnvironmentRequest`, identity keys, platform context, artifact
      kind, scheduler intent, model facts, package facts, or local paths.
      Workflow-service builds the canonical dependency-environment request from
      the latest current descriptor validation summary and graph/node state,
      then forwards it through the Tauri boundary. If descriptor validation is
      missing, pending, stale, unavailable, unresolved, invalid, or disagrees
      with the current graph revision, the action returns typed diagnostics and
      does not call dependency execution.
      Re-plan update: before deriving canonical dependency-environment
      requests, replace numeric validation revision identity with graph
      fingerprint revision identity and store/read the latest current
      validation state by `graph_session_id + graph_revision` with optional
      `validation_session_id`. This is option 2 now with option 3 discipline:
      the initial implementation may be a narrow workflow-service state map,
      but its API must look like a dedicated validation-state owner so later
      submit gating and scheduler admission can consume the same source without
      rewriting callers.
      Standards iteration update: the next implementation slice must not add
      more validation-state logic to `graph/session.rs` because that module is
      already over the decomposition review threshold. Create a focused
      workflow-service validation-state module, expose a small owner API, move
      the dependency-action intent freshness checks behind that API, and update
      the graph source-directory README or ADR. The owner API must accept
      validated intent/revision/session types after boundary parsing; raw
      strings, numeric revisions, or JSON maps may not become internal
      freshness keys.
- [ ] Delete or rewrite `ModelDependencyRequest`/`model_path` dependency
      hydration call sites that are on the `puma-lib` -> inference path. Any
      remaining dependency-environment work must consume the canonical
      path-free dependency-planning contract directly, not a compatibility
      adapter around the retired request.
- [ ] After model-ref-only authoring is validated, wire live validation so model
      selection/change starts backend descriptor validation, renders authored
      ports immediately, overlays pending/stale/unavailable/invalid state, and
      gates submit/enqueue from the backend summary.
      Re-plan decision: implement this in two stages. Stage 1 is the
      synchronous workflow-service validation path: a backend-owned graph
      session method snapshots the draft graph under lock, releases the lock,
      resolves descriptor facts, builds typed validation events and summary,
      records the current summary through the validation-state owner, and
      returns the validation session to the caller. This is the smallest
      shippable path for proving model-ref -> descriptor -> authored ports ->
      validation summary -> submit/admission gate without giving Tauri or the
      frontend validation policy. Stage 2 is the event-driven validation path:
      graph mutations or explicit validation requests start backend validation
      without blocking graph editing; stale validation sessions are discarded by
      graph revision/session id; current events are published to the editor as
      descriptor, drift, diagnostic, update-proposal, and graph-summary events.
      Stage 1 must be shaped as the same internal publisher used by Stage 2 so
      event-driven validation can replace the transport without rewriting the
      validation core.
      Review tightening: Stage 1 is not complete if it only records an externally
      supplied summary or emits descriptor fingerprints. It must use a
      workflow-service publisher core that calls the existing descriptor
      projection path, returns bounded per-node projection records containing the
      current descriptor/authored-port data needed by the editor, and records
      graph plus node validation state through the validation-state owner. Events
      may stay lightweight, but descriptor rendering and dependency-action
      derivation must not require Tauri/frontend code to resolve descriptors,
      carry Pumas facts, or infer scheduler policy.
- [x] Add a strict model-ref binding resolver for inference request extraction.
      It must reject duplicate incoming `pumas_model_ref` edges, verify the
      source handle and source node/type can provide a Pumas model ref, and
      report connected-versus-inline disagreement as typed diagnostics instead
      of silently selecting one source. This is the first required code slice
      before any synchronous or event-driven validation publisher consumes graph
      requests, because the current extractor can silently collapse duplicate
      edges or accept the wrong source handle.
- [x] Tighten optional explicit constraint parsing. Missing, null, and blank
      strings are absent; wrong JSON types and nonblank unparsable strings are
      invalid diagnostics for `task_kind`, `runtime`, `device`, and later typed
      trait inputs. This belongs with the strict extraction slice so invalid
      graph-authored constraints cannot be downgraded to "scheduler decides" by
      accident.
- [ ] Make descriptor task kind authoritative for scheduler materialization.
      Graph-authored `task_kind` remains an optional resolver constraint only;
      scheduler task projection/materialization consumes the resolved descriptor
      task kind and fails closed when explicit graph constraints cannot be
      satisfied.
- [ ] Implement the first cross-layer acceptance slice before broad horizontal
      expansion, after the boundary cleanup slices pass: connected model ref
      input resolves a descriptor, projects authored visible ports, produces a
      backend validation summary, and gates frontend submit plus backend
      admission without invoking retired `inference_settings`,
      `expand-settings`, static all-port, or model-path paths.
- [ ] Ensure runtime/device constraints narrow interface validation without
      becoming scheduler decisions. Explicit invalid constraints block enqueue
      and may include typed advisory alternatives when they can be computed
      safely; alternatives must not become fallback execution choices.
- [ ] Wire graph editor draft validation to request and render the descriptor
      for unsaved graph state through the live validation session model. The
      editor renders saved authored ports immediately, overlays pending/stale/
      invalid validation state from backend events, disables submit/enqueue
      from the latest backend summary, and does not block editing while
      validation runs.
- [ ] Add the descriptor-backed dependency-environment action intent slice.
      Define a minimal action-intent DTO that carries only graph/session
      identity, graph revision/validation session identity, target node id, and
      action. Keep it in the shared inference/workflow contract surface used by
      the graph editor and workflow-service, not in Tauri business logic. The
      backend must derive the canonical `DependencyEnvironmentRequest` from the
      validation summary: `pumas_model_ref` from the strict model-ref binding
      resolver; task id/type and expected artifact kind from the resolved
      descriptor; runtime/device constraints from validated explicit graph
      constraints; selected binding ids and manual overrides from the
      dependency-environment node data; platform context from host/dependency
      planning policy; and dependency requirements/environment ids from the
      current validated dependency planning result. `Check` and `Install`
      actions must fail closed with missing-requirements diagnostics until a
      current requirements id exists.
      - Contract sub-slice completed on 2026-05-26:
        `pantograph-inference-interface-contracts` now owns
        `DependencyEnvironmentActionIntent`,
        `ValidatedDependencyEnvironmentActionIntent`, and
        `WorkflowGraphSessionId`. The intent reuses the canonical
        `DependencyEnvironmentAction` enum and carries only graph session id,
        graph revision, optional validation session id, target
        node id, and action. Tests prove it rejects retired `run` actions and
        unknown legacy/backend-owned fields such as paths, model refs, and
        platform context. Remaining implementation work is the workflow-service
        builder that validates the current descriptor summary and derives
        `DependencyEnvironmentRequest` without moving policy into Tauri or the
        frontend.
      - Workflow-service fail-closed builder sub-slice completed on
        2026-05-26: `GraphSessionStore` and `WorkflowService` now accept
        `DependencyEnvironmentActionIntent`, validate it against the current
        graph edit session, reject stale graph revisions with typed
        `GraphRevisionMismatch` diagnostics, reject missing target nodes with
        typed `TargetNodeMissing` diagnostics, and return typed
        `ValidationSummaryMissing` diagnostics instead of building a partial
        `DependencyEnvironmentRequest` when descriptor validation state is not
        available. Remaining implementation work is storing/currently resolving
        descriptor validation summaries and deriving the canonical
        `DependencyEnvironmentRequest` only when those summaries are executable
        and current.
- [ ] Add the current inference-validation state owner before dependency action
      derivation. The owner must be workflow-service code, not Tauri/frontend
      code, and must be keyed by validated `graph_session_id +
      WorkflowGraphRevision` plus optional validation session id. It stores the
      latest descriptor-backed validation summary and enough node-scoped
      descriptor metadata to let later slices derive dependency-environment
      requests without passing Pumas facts or paths through the graph editor.
      It must publish/update state only after confirming the graph revision is
      still current, must not hold graph-session locks across Pumas/inference
      resolution, and must return typed diagnostics for missing, pending,
      stale, unavailable, invalid, or mismatched validation.
      Review tightening: the current owner shape must grow beyond
      `validation_session_id + DraftGraphValidationSummary` before dependency
      derivation or admission can be considered wired. Store bounded
      node-scoped records keyed by `WorkflowNodeId`, including at minimum
      descriptor fingerprint, resolved descriptor task kind, descriptor
      availability/summary status, validated Pumas model ref identity, and
      validated explicit runtime/device/trait constraints. Full Pumas package
      facts, executable paths, runtime load targets, media payloads, and frontend
      rendering state remain out of the owner.
      - Initial owner/freshness sub-slice completed on 2026-05-26:
        `graph/inference_validation_state.rs` now owns dependency-environment
        action freshness checks keyed by validated graph session id and
        `WorkflowGraphRevision`. `GraphSessionStore` parses the boundary input,
        canonicalizes the graph only long enough to compute the current
        revision and target-node existence, releases the graph lock, then
        delegates to the owner. The slice preserves the no-fallback rule:
        stale revisions, missing target nodes, and missing current summaries
        return typed blocked diagnostics and never derive a partial
        `DependencyEnvironmentRequest`. Remaining work is replacing numeric
        live-validation revision identity, publishing executable validation
        summaries into this owner, and deriving dependency-environment requests
        only from current executable summary state.
      - Current-summary recording sub-slice completed on 2026-05-26:
        `GraphSessionStore::record_inference_validation_session` now records a
        live validation session only after validating the session and proving
        its `WorkflowGraphRevision` still matches the current graph session.
        The validation-state owner stores the current validation-session id and
        summary for the graph session/revision, rejects stale explicit
        validation-session ids, blocks non-executable summaries with typed
        diagnostics, and still stops at `DependencyRequirementsMissing` until
        dependency requirements are available.
- [ ] Reuse existing graph-session/event transport patterns for live validation
      only when they preserve backend ownership and event-driven UI updates.
      Workflow-service must snapshot draft graph state under lock, release the
      lock before Pumas/inference fact resolution, and publish events only for
      the current validation session id and graph revision.
      The event-driven implementation is required for graph-editor UX. It must
      never block displaying or editing the graph while validation runs. The
      graph editor renders saved/authored ports immediately, overlays backend
      validation state as it arrives, disables submit/enqueue from the latest
      backend summary, and keeps editing available even while validation is
      pending. Tauri remains a transport layer and must not own validation
      freshness, descriptor resolution, enqueue policy, or dependency request
      derivation.
      Review tightening: implement the synchronous publisher in workflow-service
      first and expose it through transport only after backend tests prove lock
      release before descriptor resolution, stale revision/session rejection,
      bounded projection records, and current-state publication. Stage 2 may add
      event delivery/cancellation, but it must reuse this publisher core rather
      than adding a second validation path.
- [ ] Add the workflow-service live validation lifecycle owner before event
      delivery reaches the frontend. The owner must start, cancel, supersede, and
      clean up validation sessions; use bounded event/state buffers with explicit
      overflow/backpressure diagnostics; observe task errors and panics; cancel
      or supersede in-flight work when graph revisions change; and stop accepting
      validation work when a graph/session closes. Domain validation/projection
      remains sync-core; async is limited to fact lookup, persistence, transport,
      and event delivery boundaries.
- [ ] Wire graph editor drift presentation and update preview. The editor must
      show authored-current diffs visually on nodes/ports/edges, keep invalid
      edges visible, preview backend-proposed typed patch operations, and apply
      them only after user confirmation.
- [ ] Wire `PortOptionsProvider`, selection-input, and option-cache consumers
      only as descriptor-backed presentation plumbing where reused. Typed
      option identity, defaults, availability, and diagnostics remain owned by
      `pantograph-inference-interface-contracts`, and cache keys/invalidation
      must include descriptor fingerprint or validation session/revision.
- [ ] Wire workflow save validation to consume the same descriptor contract.
      Required inputs, optional defaults, valid options, connected upstream
      output types, and explicit runtime/device constraints must be validated
      before a workflow can be saved or submitted as executable.
- [ ] Wire workflow submit and scheduler queue admission to consume the
      backend validation summary. The frontend submit button and backend queue
      admission must both fail closed while inference validation is pending,
      stale, unavailable, unresolved, or blocked; raw diagnostics must not be
      the enqueue authority. Backend admission must run before queue insertion,
      queue-placement diagnostic event recording, and scheduler task graph
      materialization so non-executable inference graphs never become queued
      runs that are later canceled.
- [ ] Update scheduler task graph projection and materialization so generic
      inference tasks store path-free model refs, task kind, typed constraints,
      descriptor fingerprint, and bindings only. Workflow-service must
      materialize final runtime-host inputs from upstream task results, graph
      literal values, and descriptor defaults after scheduler input readiness.
      Projection must consume the current descriptor-backed validation state for
      resolved task kind, descriptor fingerprint, and validated constraints; raw
      graph `task_kind`, runtime, device, or trait JSON are resolver inputs only
      and must not be parsed again as execution authority.
- [ ] Align the runtime-host input contract with descriptor materialization.
      Runtime-host execution requests must include scheduler-selected handoff
      plus typed, path-free materialized inputs and artifact/result references;
      they must not receive graph paths, Pumas package facts, or scheduler-owned
      payload guesses.
- [ ] Add pre-dispatch descriptor revalidation before scheduler dispatch
      selection. If Pumas facts, selected artifact state, runtime capability,
      or descriptor fingerprint changed since draft/save validation, fail the
      task with typed diagnostics rather than falling back to stale ports.
- [ ] Delete or rewrite retired inference-node port discovery and validation
      surfaces replaced by this resolver. Do not keep compatibility aliases,
      legacy model-path support checks, planned-inference validation branches,
      or image-only graph-editor port tables as successful paths.
- [ ] Delete or rewrite retired `inference_settings` JSON canonicalization and
      `expand-settings` frontend-owned dynamic port sync. If any of that UI
      remains, it must consume backend descriptors as presentation plumbing and
      cannot remain an alternate inference-interface source.
- [ ] Delete or rewrite retired `puma-lib` path/readiness/inference-settings
      authoring surfaces after the model-ref-only slice. This includes
      `modelPath`/`model_path`/`entry_path` graph data, option metadata,
      node-engine `puma-lib` output semantics, frontend mocks, templates, and
      tests that still treat paths or `inference_settings` as successful
      inference-interface inputs.
- [ ] Decide the first implementation slice for existing `PortOptionsProvider`,
      selection-input, and option-cache reuse versus a dedicated descriptor
      option renderer. The decision must preserve typed descriptor ownership
      and avoid duplicating backend semantics.
- [ ] Add a contract-crate decomposition gate before adding more shared DTO
      families. `pantograph-inference-interface-contracts/src/lib.rs` is already
      large enough that validation publication records, node projection DTOs, or
      additional dependency-action contracts must move into focused modules
      unless the change is a small addition to an existing type. Update the crate
      README and dependency-direction checks in the same slice.
- [ ] Keep the legacy deletion searches as implementation gates, not cleanup
      notes. The first acceptance slice must prove it does not invoke
      `inference_settings`, `expand-settings`, static all-port inference
      metadata, frontend `modelPath` dependency actions, `ModelDependencyRequest`,
      `ModelRefV2`, or model-path-derived node-engine inference paths. Remaining
      occurrences must be classified as unrelated non-workflow configuration,
      test fixtures being rewritten, or deletion targets before the milestone can
      claim no-legacy completion.

**Verification:**

- Descriptor serde fixture round trips for ports, defaults, options,
  availability, runtime conditions, and diagnostics.
- Contract-crate dependency-direction check proving DTO-only ownership and no
  imports of workflow-service, scheduler, runtime-host, Pumas lookup/runtime
  wiring, inference execution, or frontend policy.
- Contract-crate README/affected README or ADR review proving public boundary,
  structured producer, and API consumer contracts are documented.
- `cargo check` coverage for affected public crates in default,
  no-default-features, and all-features modes.
- Authored snapshot fixture round trips proving saved graph shape can be
  preserved without Pumas package facts, executable paths, runtime load
  targets, scheduler decisions, or large media payloads.
- Drift report fixture and unit tests for added/removed ports, type changes,
  requirement changes, default changes, option changes, availability changes,
  blocking drift, and non-blocking drift.
- Typed graph patch proposal tests proving backend proposals update authored
  snapshots, report affected edges/literals, require confirmation for
  destructive changes, and reject unsafe reconciliation.
- Live validation session tests proving stale session/revision events cannot
  update current graph validation state.
- Contract-crate deletion/source-search tests proving shared unscoped validation
  event/stream DTOs are not exported or used as transport, while shared
  validation summary serde fixtures still round trip.
- Validation summary tests proving enqueue is disabled while validation is
  pending, stale, unavailable, unresolved, or blocked, and enabled only when
  backend says `executable = true`.
- Resolver unit tests for ready, unavailable, unsupported, unimplemented,
  missing selected artifact, stale fingerprint, and ambiguous capability
  states.
- Runtime/device constraint tests proving explicit invalid constraints block,
  alternatives are advisory only, omitted constraints leave scheduler as the
  final runtime/device decision owner, and descriptors do not expose full
  scheduler candidate lists in the common path.
- Strict request-extraction tests proving duplicate incoming `pumas_model_ref`
  bindings, invalid source handle/type, connected-versus-inline disagreement,
  wrong-type explicit constraints, and nonblank unparsable constraints emit
  typed diagnostics instead of silent selection or absence.
- Model-ref-only `puma-lib` tests proving saved graph/node data, node metadata,
  Tauri option hydration, frontend mocks/templates, and node-engine successful
  semantics no longer expose executable paths, `entry_path`, package facts,
  runtime hints, load targets, or `inference_settings` as inference-interface
  inputs.
- First vertical-slice acceptance test proving model ref input resolves a
  descriptor, projects authored ports, emits validation summary, and gates
  frontend submit plus backend admission from that summary.
- Effective-definition tests proving inference nodes derive visible ports from
  authored/current descriptors, not semantic `node.data.definition` fallbacks.
- Backend draft-validation tests proving graph editor-visible ports come from
  the descriptor and include typed disabled reasons.
- Live validation transport tests proving graph locks are not held while Pumas
  or inference fact resolution runs, and stale session/revision events are
  ignored.
- Validation lifecycle tests proving event-driven sessions cancel or supersede
  in-flight work on graph revision changes, clean up on graph/session close,
  enforce bounded event/state capacity, emit typed overflow/backpressure
  diagnostics when applicable, and observe task errors or panics at the owner.
- Workflow save-validation tests proving invalid ports, missing required
  inputs, wrong upstream output types, and unsupported options fail closed.
- Submit/admission tests proving frontend submit state and backend queue
  admission use the backend validation summary as authority.
- Frontend validation-store tests proving session/revision/sequence/node-scoped
  stale events cannot mutate current node state, graph display and editing remain
  available while validation is pending, and event subscriptions or temporary
  timers are cleaned up on graph/session changes.
- Materialization tests proving defaults are applied only from the descriptor,
  connected upstream task results are type-checked, and runtime-host inputs
  are path-free typed values or typed artifact/result references.
- Scheduler projection/materialization tests proving resolved descriptor task
  kind is authoritative, graph-authored `task_kind` only narrows resolver
  selection, and unsatisfied explicit task constraints block before enqueue.
- Runtime-host contract tests proving execution requests carry typed
  materialized inputs with scheduler handoff and reject graph paths, package
  facts, and untyped payload bags.
- Pre-dispatch tests proving stale descriptor fingerprints and changed
  capability facts fail before scheduler dispatch.
- Search/deletion checks for retired dynamic inference port tables,
  `modelPath`/`model_path` support paths, planned-inference validation branches,
  raw package-fact graph payloads, and untyped inference metadata bags.
- Search/deletion checks for retired `inference_settings` JSON interface paths,
  `expand-settings` inference-interface ownership, static all-port
  `llm-inference` task/model ports, and inference-node `node.data.definition`
  semantic fallbacks.
- `cargo fmt`, focused package tests, affected crate `cargo check` in default,
  no-default-features, and all-features modes, plus frontend type/test coverage
  for touched draft-validation UI.
- Dependency ownership and feature checks for every new crate/package or feature
  added by this milestone, proving dependencies are declared by the owning
  crate/package, DTO crates keep minimal default features, and frontend workspace
  commands do not rely on unrelated root-only dependencies.

**Status:**

- [ ] Pending implementation. Added as a Milestone 5 family prerequisite after
      the runtime-host input payload replan exposed that production runtime
      dispatch needs a canonical model-specific inference interface before
      task inputs can be materialized safely.
- [ ] Design decisions recorded through live validation summary and
      runtime/device alternative diagnostics. Codebase investigation resolved
      the main integration direction: authored snapshots replace inference
      `node.data.definition` semantics, static all-port inference descriptors
      are retired, JSON `inference_settings`/`expand-settings` paths are
      removal or rewrite targets, validation summary gates submit/enqueue,
      workflow-service owns the live scoped validation event stream, `puma-lib`
      moves through model-ref-only authoring before live validation UX, and
      runtime-host input contracts must be aligned before production dispatch.
      Remaining decisions are the concrete option-renderer reuse path,
      resolver module/API shape, and graph patch apply ownership for drafts
      versus saved workflows.
- [x] 2026-05-25 contract-crate slice completed:
  - Smallest useful vertical slice: added the DTO-only
    `pantograph-inference-interface-contracts` crate, workspace registration,
    crate/source/test README coverage, versioned typed descriptors, resolver
    and validator request contracts, authored snapshots, drift reports,
    validation summaries, bounded availability/reason/diagnostic enums, and
    serde fixtures.
  - No-fallback/no-legacy confirmation: the slice adds only the canonical
    contract owner. It does not route `node.data.definition`,
    `inference_settings`, `expand-settings`, static all-port descriptors,
    model paths, Pumas package facts, runtime load targets, scheduler
    decisions, or worker execution through the new contracts.
  - Dependency-direction result: the new crate depends only on
    `pantograph-dependency-planning`, `serde`, and `thiserror`; the Pumas model
    reference and runtime/device intent ids are reused from the existing
    path-free dependency-planning contract instead of creating parallel DTOs.
  - Verification passed: `cargo fmt -p
    pantograph-inference-interface-contracts`; `cargo test -p
    pantograph-inference-interface-contracts`; `cargo check -p
    pantograph-inference-interface-contracts`; `cargo check -p
    pantograph-inference-interface-contracts --no-default-features`; `cargo
    check -p pantograph-inference-interface-contracts --all-features`; and
    targeted source search for forbidden imports, model-path fields, retired
    inference settings, and untyped metadata bags in the new crate.
  - Remaining follow-up: implement the first cross-layer descriptor resolution
    and validation acceptance slice before broad frontend, scheduler,
    runtime-host, or legacy-deletion work.
- [x] 2026-05-25 tool-loop composed-contract dependency removal slice
      completed:
  - Smallest useful vertical slice: removed
    `workflow_nodes::builtin_composed_node_contracts`, deleted the built-in
    `tool-loop` internal graph mapping over `llm-inference` and
    `tool-executor`, updated workflow-nodes README/control docs, and updated
    ADR-009 so the recorded architecture no longer claims a static
    `tool-loop` composed registration exists.
  - No-fallback/no-legacy confirmation: this removes the hidden static
    all-port inference fallback before the generic inference descriptor is
    shrunk. `tool-loop` is still authorable, but direct runtime execution fails
    closed with a scheduler-owned agent-loop diagnostic until the canonical
    scheduler orchestration path exists.
  - Verification passed: `cargo fmt -p workflow-nodes -- --check`; `cargo
    test -p workflow-nodes --lib contracts`; `cargo test -p workflow-nodes
    --features model-library --lib contracts`; `cargo test -p workflow-nodes
    --lib tool_loop`; `cargo check -p workflow-nodes`; `cargo check -p
    workflow-nodes --no-default-features`; `cargo check -p workflow-nodes
    --all-features`; retired composed-contract source search; and `git diff
    --check`.
  - Remaining follow-up: implement descriptor/authored-snapshot projection and
    then shrink graph-visible `llm-inference` to bootstrap/control ports. Later
    scheduler-owned agent-loop expansion must build on descriptor-backed
    materialized inference turns rather than restoring a composed static
    inference contract.
- [x] 2026-05-25 inference `node.data.definition` fallback rejection slice
      completed:
  - Smallest useful vertical slice: updated workflow-service effective
    definition resolution so `llm-inference` nodes reject
    `node.data.definition` as an executable dynamic-port source, while
    non-inference dynamic overlays remain supported. Contract validation now
    emits a blocking `invalid_dynamic_definition` diagnostic for inference
    nodes carrying legacy dynamic definitions.
  - No-fallback/no-legacy confirmation: model-specific inference ports must
    come from authored inference interface snapshots and backend descriptor
    validation. The graph validator no longer accepts inference-specific
    prompt/option/result ports from arbitrary saved JSON dynamic definitions.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    graph::effective_definition --lib`; `cargo test -p
    pantograph-workflow-service
    contract_diagnostics_reject_inference_dynamic_definition_fallbacks --lib`;
    `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; targeted graph source search;
    and `git diff --check`. The check commands still report only the known
    `set_active_run_execution_plan` dead-code warning from the active-plan
    runtime handoff removal boundary.
  - Remaining follow-up: project authored inference interface snapshots into
    effective inference ports, then retire task/model-specific static
    `llm-inference` ports without reusing `node.data.definition` as the
    fallback source.
- [x] 2026-05-25 authored inference snapshot effective-port projection slice
      completed:
  - Smallest useful vertical slice: added workflow-service projection from
    typed `node.data.inference_interface_snapshot` authored snapshots into
    effective `llm-inference` ports. The projection ignores legacy
    `node.data.definition` when a valid authored snapshot is present, rejects
    invalid snapshots with typed effective-definition errors, and rejects
    unknown future snapshot value/requirement variants until explicitly mapped.
  - No-fallback/no-legacy confirmation: authored snapshots now have a canonical
    backend path into graph-effective ports. The slice does not re-enable
    `node.data.definition`, Pumas package facts, runtime load targets, or
    model paths as inference interface sources.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    graph::effective_definition --lib`; `cargo test -p
    pantograph-workflow-service
    contract_diagnostics_reject_inference_dynamic_definition_fallbacks --lib`;
    `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and `git diff --check`.
    The workflow-service check commands still report only the known
    `set_active_run_execution_plan` dead-code warning from the active-plan
    runtime handoff removal boundary.
  - Remaining follow-up: shrink graph-visible `llm-inference` static metadata
    to bootstrap/control ports and wire the resolver/validator so connected
    Pumas model refs produce current descriptors and validation summaries.
- [x] 2026-05-25 static `llm-inference` bootstrap descriptor slice completed:
  - Smallest useful vertical slice: shrank the built-in `llm-inference`
    descriptor to pre-resolution bootstrap/control ports only:
    `task_kind`, `runtime`, `device`, `pumas_model_ref`, and `diagnostics`.
    Removed static prompt/text/query/document/audio/tool/cache/options/sampler/
    result/image/usage/model-ref ports from workflow-node metadata, updated
    contract projection tests, and updated workflow-service registry tests so
    task/model-specific ports are descriptor/authored-snapshot owned.
  - No-fallback/no-legacy confirmation: the slice deleted the retired
    `llm-inference.denoising_scheduler` `PortOptionsProvider` and its tests
    instead of keeping a queryable static option source for a removed port.
    Denoising scheduler choices must now arrive as typed descriptor-backed
    option sets from the inference-interface resolver.
  - Verification passed: `cargo fmt -p workflow-nodes -- --check`; `cargo
    test -p workflow-nodes --lib inference`; `cargo test -p workflow-nodes
    --lib contracts`; `cargo test -p workflow-nodes --features model-library
    --lib contracts`; `cargo test -p pantograph-workflow-service
    graph::registry --lib`; `cargo check -p workflow-nodes`; `cargo check -p
    workflow-nodes --no-default-features`; `cargo check -p workflow-nodes
    --all-features`; `cargo check -p pantograph-workflow-service`; `cargo
    check -p pantograph-workflow-service --no-default-features`; `cargo check
    -p pantograph-workflow-service --all-features`; and targeted source search
    for retired static inference ports/provider symbols. Workflow-service
    checks still report only the known `set_active_run_execution_plan`
    dead-code warning from the active-plan runtime handoff removal boundary.
  - Standards/decomposition note: `crates/workflow-nodes/src/contracts.rs`
    remains over the preferred file-size threshold even after this slice
    removed a large static-port mapping. It predates the slice and should be
    split into production projection plus focused test modules before the next
    broad contract-projection edit, but it was reduced rather than expanded
    here to keep this slice atomic.
  - Remaining follow-up: implement the workflow-service resolver boundary so
    connected Pumas model refs produce current descriptors, validation
    summaries, typed option sets, and authored snapshot updates without
    restoring static all-port metadata, `inference_settings`, or
    `expand-settings` as interface sources.
- [x] 2026-05-25 inference-interface graph patch DTO slice completed:
  - Smallest useful vertical slice: added workflow-service-owned
    `InferenceInterfaceUpdateProposal` and typed graph patch operation DTOs for
    replacing authored inference snapshots, removing invalid edges, and
    clearing invalid literals. The DTOs reference shared drift reports and
    authored snapshots but keep graph mutation operations in workflow-service.
  - No-fallback/no-legacy confirmation: this slice adds proposal contracts
    only. It does not apply patches, mutate saved graphs, restore
    `node.data.definition`, or accept `inference_settings`/`expand-settings` as
    inference-interface sources. Destructive operations must mark the proposal
    destructive and require explicit confirmation.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service inference_interface_patch
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; targeted source search for
    forbidden fallback terms in the new DTO module; and `git diff --check`.
    Workflow-service checks still report only the known
    `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: wire live validation proposal events and apply/update
    endpoints after the resolver can produce current descriptors and drift
    reports.
- [x] 2026-05-25 live inference-validation session contract slice completed:
  - Smallest useful vertical slice: added workflow-service live validation
    session/event DTOs with backend-issued validation session ids, numeric
    client graph revisions, strictly increasing event sequences, descriptor,
    drift, diagnostic, update-proposal, and summary payloads. This numeric
    revision field is superseded by the 2026-05-26 current-validation-state
    re-plan and must be replaced with graph fingerprint revision identity
    before dependency-environment request derivation is implemented.
  - No-fallback/no-legacy confirmation: the slice defines the event contract
    only. It does not add transport, resolver, graph mutation, or frontend
    fallback behavior, and update-proposal payloads stay workflow-service-owned
    because they carry graph patch operations.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service
    inference_interface_validation --lib`; `cargo check -p
    pantograph-workflow-service`; `cargo check -p pantograph-workflow-service
    --no-default-features`; `cargo check -p pantograph-workflow-service
    --all-features`; and `git diff --check`. Workflow-service checks still
    report only the known `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: wire session event transport after descriptor
    resolution can produce current descriptors, drift reports, update
    proposals, and backend validation summaries.
- [x] 2026-05-25 workflow-service inference resolver boundary slice completed:
  - Smallest useful vertical slice: added a synchronous facts-in resolver
    boundary in workflow-service that combines path-free Pumas model state,
    selected-artifact state, inference capability facts, runtime availability,
    and optional graph-authored runtime/device constraints into an
    `InferenceInterfaceDescriptor`.
  - No-fallback/no-legacy confirmation: the resolver does not inspect model
    paths, Pumas package fact blobs, runtime-host payloads, static all-port
    metadata, `inference_settings`, or `expand-settings`. Descriptor ports are
    copied only from explicit capability facts, and missing/invalid facts return
    typed unavailable diagnostics instead of guessed ports.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service inference_interface_resolver
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; targeted source search for
    forbidden fallback/path/package/runtime-host terms in the resolver; and
    `git diff --check`. Workflow-service checks still report only the known
    `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: add lookup adapters that feed this resolver from Pumas
    load-target/readiness APIs, runtime capability registries, and runtime
    availability facts, then wire the first connected-model-ref acceptance
    slice.
- [x] 2026-05-25 workflow-service inference projection slice completed:
  - Smallest useful vertical slice: added workflow-service projection from a
    resolved `InferenceInterfaceDescriptor` into the minimal authored inference
    snapshot persisted in node data plus the backend validation summary that
    frontend submit state and backend admission must consume.
  - No-fallback/no-legacy confirmation: the projection consumes only validated
    descriptors from the canonical resolver path. It does not read
    `node.data.definition`, `inference_settings`, `expand-settings`, static
    all-port metadata, Pumas package facts, model paths, runtime-host payloads,
    or scheduler decisions. Explicit invalid runtime/device constraints become
    blocking summary reasons instead of alternate execution choices.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service inference_interface_projection
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; and `cargo check -p
    pantograph-workflow-service --all-features`. Workflow-service checks still
    report only the known `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: wire graph/model-ref lookup adapters into the resolver
    and projection helper so a connected `puma-lib` output can produce current
    descriptors, authored visible ports, live validation events, and submit/
    queue-admission gating from the backend summary.
- [x] 2026-05-25 workflow-service inference request extraction slice
      completed:
  - Smallest useful vertical slice: added draft-graph extraction of
    `ResolveInferenceInterfaceRequest` values for generic inference nodes from
    a connected `puma-lib.pumas_model_ref` source or the node's own typed
    `pumas_model_ref` value, plus optional explicit `task_kind`, `runtime`, and
    `device` constraints.
  - No-fallback/no-legacy confirmation: request extraction accepts only the
    canonical path-free `pumas_model_ref` contract. It does not read
    `model_ref`, `model_path`, Pumas package summaries, executable load
    targets, `inference_settings`, `expand-settings`, static all-port metadata,
    runtime-host payloads, or scheduler decisions as alternate request sources.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service inference_interface_request
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; and `cargo check -p
    pantograph-workflow-service --all-features`. Workflow-service checks still
    report only the known `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: feed extracted requests into the resolver/projection
    pipeline with Pumas/runtime capability facts and emit live validation events
    plus backend validation summaries for the graph editor and queue admission.
- [x] 2026-05-26 strict inference request extraction slice completed:
  - Smallest useful vertical slice: tightened
    `graph/inference_interface_request.rs` so generic inference nodes accept
    only one incoming `pumas_model_ref` binding, require that binding to come
    from the `pumas_model_ref` handle on a `puma-lib` node, reject inline versus
    connected model-ref disagreement, and treat wrong-type optional
    `task_kind`/`runtime`/`device` values as invalid diagnostics.
  - No-fallback/no-legacy confirmation: the extractor still accepts only the
    canonical path-free `pumas_model_ref` contract. It does not read
    `modelPath`, `model_path`, Pumas package facts, executable load targets,
    `inference_settings`, `expand-settings`, static all-port metadata,
    runtime-host payloads, or scheduler decisions as alternate request sources.
  - Standards result: the slice keeps request extraction in the existing
    workflow-service graph module, updates the graph README invariant, and
    preserves parse-once boundary semantics with typed diagnostics instead of
    silent selection.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service inference_interface_request
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and `git diff --check`.
  - Discovered issue: the workflow-service check commands continue to report the
    pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: feed strict extracted requests into the synchronous
    validation publisher so current descriptor projections and validation
    summaries can be recorded without reintroducing frontend/Tauri policy.
- [x] 2026-05-25 live validation event node-identity re-plan boundary:
  - Discovered issue: the current live validation event payloads can carry
    descriptor fingerprints, drift reports, diagnostics, update proposals, and
    summaries, but they do not identify the inference node that owns a
    descriptor/proposal event. That is not sufficient for graphs with multiple
    inference nodes or multi-model workflows because the editor and backend
    admission path could not unambiguously apply descriptor resolution,
    authored-snapshot updates, drift previews, or per-node diagnostics.
  - Why implementation stops: wiring extracted graph requests into live
    validation events without node identity would either require a single-node
    assumption or implicit event ordering. Both would violate the canonical
    multi-inference design and create a hidden compatibility rule that later
    scheduler batching/distribution work would have to undo.
  - Re-plan options to decide:
    1. Add `node_id` to every `WorkflowGraphInferenceValidationEvent` envelope.
       This is simple, but summary-only events would carry redundant node
       identity or need a sentinel, which weakens the event model.
    2. Add `node_id` only to node-scoped event payload variants such as
       descriptor, drift, diagnostic, and update proposal, while keeping graph
       summary events graph-scoped. This is more explicit but spreads node
       identity across payload types.
    3. Introduce a typed event scope enum on the event envelope, for example
       graph-scoped versus node-scoped `{ node_id }`, and require descriptor,
       drift, diagnostic, and update-proposal payloads to be node-scoped while
       summary remains graph-scoped. This keeps one routing field, supports
       multi-node validation, and avoids sentinel node ids.
  - Recommendation: choose option 3 before the next implementation slice. Then
    wire the graph request extraction, resolver, projection, and validation
    session into a single event-producing service boundary using explicit
    graph/node event scope.
- [x] 2026-05-25 live validation event scope slice completed:
  - Decision implemented: option 3. `WorkflowGraphInferenceValidationEvent`
    now carries a typed graph/node scope, descriptor/drift/diagnostic/update
    proposal payloads must be node-scoped, and summary payloads must be
    graph-scoped.
  - No-fallback/no-legacy confirmation: event routing no longer depends on
    single-inference assumptions, implicit event ordering, sentinel node ids, or
    payload-specific ad hoc node fields. This preserves explicit multi-node
    routing for multi-model workflows and future scheduler batching.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`;
    `cargo test -p pantograph-workflow-service inference_interface_validation
    --lib`.
  - Remaining follow-up: wire graph request extraction, resolver, projection,
    and scoped validation events into a single validation service boundary.
- [x] 2026-05-25 Milestone 5d codebase review decisions recorded:
  - Decision: workflow-service owns the live scoped validation event/session
    envelope. The shared inference-interface contract crate keeps descriptor,
    authored snapshot, drift, diagnostic, option, and validation summary DTOs
    only; shared unscoped validation event/stream DTOs are retirement targets.
  - Decision: `puma-lib` authoring moves in two stages. First implement a
    model-ref-only intermediate slice that removes executable paths, load
    targets, package facts, runtime hints, and `inference_settings` from graph
    semantics while preserving `pumas_model_ref` and display identity. Then wire
    live validation as the editor UX over that canonical boundary.
  - Decision: request extraction needs strict model-ref binding diagnostics for
    duplicate incoming bindings, invalid source handle/type, and
    connected-versus-inline disagreement. Optional explicit `task_kind`,
    `runtime`, `device`, and future trait inputs must treat missing/null/blank
    values as absent and wrong-type or unparsable values as invalid.
  - Decision: resolved descriptors own scheduler task kind for inference
    materialization. Graph-authored `task_kind` is only a hard resolver
    constraint when present; failed constraints block validation/enqueue before
    scheduler projection.
  - No-fallback/no-legacy confirmation: the next slices must remove or rewrite
    stale `puma-lib` path/readiness outputs, `inference_settings`,
    `expand-settings`, and shared unscoped validation events instead of leaving
    them as alternate successful inference-interface paths.
- [x] 2026-05-25 Milestone 5d standards iteration completed:
  - Standards reviewed: plan sequencing/worktree hygiene, backend-owned data,
    single owner for stateful flows, typed Rust API boundaries, serde wire-format
    alignment, vertical-slice verification, persisted dynamic artifact
    validation, frontend event-driven synchronization, and no compatibility
    code retention.
  - Plan updates: staged implementation now requires unscoped event DTO
    retirement, model-ref-only `puma-lib` authoring, and strict request
    extraction before the cross-layer acceptance slice. Verification now covers
    deletion/source searches, model-ref-only graph artifacts, strict binding and
    optional-constraint diagnostics, and descriptor-owned scheduler task kind.
  - No-fallback/no-legacy confirmation: staged projections are allowed only as
    generated views for current rendering/validation code; they are not
    compatibility routes or executable fallback sources.
- [x] 2026-05-25 shared unscoped validation event retirement slice completed:
  - Smallest useful vertical slice: removed `DraftGraphValidationEvent`,
    `DraftGraphValidationEventPayload`, and `DraftGraphValidationStreamState`
    from `pantograph-inference-interface-contracts` while keeping shared
    descriptor, authored snapshot, drift, diagnostic, option, and validation
    summary DTOs intact.
  - Allowed files touched: `crates/pantograph-inference-interface-contracts/src/lib.rs`,
    `crates/pantograph-inference-interface-contracts/README.md`, this milestone,
    `11-inference-interface-resolution-and-validation.md`, and
    `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: workflow-service remains the only live
    scoped validation event/session envelope owner; no alias, shim, or alternate
    unscoped transport was kept in the shared crate.
  - Verification passed: `cargo fmt -p pantograph-inference-interface-contracts
    -- --check`; `cargo test -p pantograph-inference-interface-contracts`;
    `cargo check -p pantograph-inference-interface-contracts`; `cargo check -p
    pantograph-inference-interface-contracts --no-default-features`; `cargo
    check -p pantograph-inference-interface-contracts --all-features`; `cargo
    check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; targeted source search in
    `crates/` for retired unscoped validation event/stream DTO names; and `git
    diff --check`. Workflow-service checks still report only the known
    `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: replace `puma-lib` graph authoring with the
    model-ref-only intermediate slice before wiring live validation UX.
- [x] 2026-05-25 `puma-lib` model-ref-only implementation re-plan boundary:
  - Discovered issue: graph-visible `puma-lib` can be made model-ref-only only
    if the Tauri hydration path stops requiring `modelPath` for dependency
    requirements. `src-tauri/src/workflow/puma_lib_commands.rs` currently
    builds `ModelDependencyRequest` from `node_data["modelPath"]`, while
    `node_engine::ModelDependencyRequest` and related commands still require a
    `model_path` string.
  - Why this blocks implementation: removing `modelPath`, `entry_path`,
    package facts, and `inference_settings` from puma-lib graph data without
    replacing dependency hydration would either break selected-model hydration
    or force a hidden synthesized path, which would violate the no-fallback and
    model-ref-only rules.
  - Required re-plan: decide the model-ref-only dependency hydration contract
    before editing production puma-lib authoring. The preferred direction should
    keep graph/node data path-free and move any Pumas-approved load target or
    dependency fact lookup behind workflow-service/scheduler-owned validation
    and dependency-planning boundaries.
- [x] 2026-05-25 `puma-lib` dependency hydration design decision:
  - Decision: use the existing canonical
    `pantograph-dependency-planning::DependencyPlanningRequest` as the
    path-free replacement contract for selected-model dependency hydration.
    Evolve that contract only if a typed field is missing; do not add a
    parallel puma-lib-specific request shape.
  - Ownership: workflow-service owns graph/node validation and request assembly
    semantics; dependency-planning owns DTO shape; host/runtime integration owns
    any Pumas-approved load target lookup. Tauri command handlers are transport
    adapters only and may not contain dependency policy, Pumas fact resolution,
    scheduler/runtime selection, or path synthesis business logic.
  - No-fallback/no-legacy confirmation: do not bridge the canonical request back
    into `node_engine::ModelDependencyRequest`, `ModelRefV2`, `modelPath`, or
    `model_path`. Those are replacement targets for the puma-lib/inference
    path, not compatibility branches.
  - Implementation staging: first route puma-lib hydration through the
    canonical dependency-planning request and service boundary; then remove
    graph-authored `modelPath`, `entry_path`, package facts, runtime hints, load
    targets, and `inference_settings`; then update frontend mocks/templates and
    node-engine tests to prove only `pumas_model_ref` plus display identity
    remain graph-facing.
- [x] 2026-05-25 `puma-lib` Tauri hydration graph-data cleanup slice:
  - Smallest useful vertical slice: update
    `src-tauri/src/workflow/puma_lib_commands.rs` so selected-model hydration
    returns graph node data with only `modelName`, `model_id`,
    `pumas_model_ref`, and sanitized `selected_binding_ids`. The slice removed
    successful graph-data emission of `modelPath`, `entry_path`, package facts,
    runtime hints, dependency bindings, load-target facts, dependency
    requirements, and `inference_settings`.
  - No-fallback/no-legacy confirmation: `resolve_requirements=true` now fails
    closed with a typed-boundary message instead of adapting the selected model
    back into `node_engine::ModelDependencyRequest` or synthesizing a hidden
    path. The Tauri command still accepts the legacy `model_path` lookup input
    only as an unresolved command API cleanup target; it is no longer emitted
    into hydrated graph node data.
  - Focused tests updated: puma-lib hydration tests now assert option values
    and hydrated node data are keyed by `pumas_model_ref`, and assert retired
    path/settings fields are absent from successful hydration outputs.
  - Verification passed: `cargo fmt`; `git diff --check`; targeted source
    search of `src-tauri/src/workflow/puma_lib_commands.rs` confirmed no
    successful hydrated node output path remains for `modelPath`,
    `entry_path`, `dependency_requirements`, or `inference_settings`.
  - Verification blocked: `cargo test -p pantograph puma_lib_commands` did not
    reach this module because the `pantograph` test binary currently fails to
    compile in `src-tauri/src/app_setup.rs` with missing
    `WorkflowService::set_media_conversion_executor`. Record this as a
    discovered unrelated compile blocker before relying on app-crate tests for
    future slices.
  - Remaining follow-up: replace the Tauri command input/API and frontend call
    sites so selected-model hydration receives `pumas_model_ref` only, then
    implement the canonical path-free dependency-environment hydration service
    boundary and remove the legacy `ModelDependencyRequest` dependency path.
