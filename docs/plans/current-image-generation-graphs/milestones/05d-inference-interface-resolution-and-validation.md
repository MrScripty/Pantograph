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
      environment service exists. Re-plan decision: use the graph-coordinator
      action-intent boundary for the active frontend slice. The
      `DependencyEnvironmentNode` emits only the user's resolve/check/install
      action for its node. The graph coordinator owns the current graph session
      id, graph revision, optional validation session id, and target node id,
      then constructs the typed `DependencyEnvironmentActionIntent`. Tauri only
      decodes/forwards that intent to workflow-service and encodes the response.
      Dependency-environment actions must consume the same backend
      descriptor/validation summary used by graph validation, submit gating,
      and scheduler admission. Frontend code must not build
      `DependencyPlanningRequest`, `DependencyEnvironmentRequest`, identity
      keys, platform context, artifact kind, scheduler intent, model facts,
      package facts, or local paths.
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
      Re-plan decision: keep `dependency-environment` as a graph-authored
      sidecar/control node associated with exactly one inference node, not as a
      consumer of inference result outputs and not as an ordinary node-engine
      execution step. The node owns user-visible dependency environment choices
      such as selected binding ids, manual override patches, action activity,
      status display, and optional persisted environment references. It does
      not own model paths, Pumas package facts, platform context, runtime
      policy, dependency request construction, or scheduler admission. Before
      deriving a canonical `DependencyEnvironmentRequest`, workflow-service
      must resolve a typed dependency action subject from the current graph:
      validate that the action target is a `dependency-environment` node,
      resolve exactly one associated inference node through canonical typed
      graph structure, join that inference node to the current descriptor
      validation record, and return typed diagnostics for zero, duplicate,
      stale, unavailable, invalid, or ambiguous subjects. The frontend and
      Tauri layers continue to send action intent only.
      Blast-radius review resolution: make the association representation
      explicit before implementing the resolver. Use a typed association/control
      port pair instead of adding an edge-kind system now: the
      `dependency-environment` descriptor exposes one association output and
      the canonical `llm-inference` bootstrap descriptor exposes one optional
      association input. Use shared constants for both handles:
      `dependency_environment_sidecar`, and add a first-class
      `DependencyEnvironmentSidecar`/`dependency_environment_sidecar` port value
      type across `node-engine`, `pantograph-node-contracts`, workflow-service
      graph DTOs, and frontend port typing. Do not model the association as
      `json`, `component`, `any`, or an untyped string convention. Workflow-
      service interprets only that typed edge as the sidecar association;
      node-engine must not materialize it into task input data, scheduler task
      graph projection must not treat it as a runtime input dependency, and
      scheduler admission must consume the resulting dependency readiness proof
      rather than raw graph inputs. The first resolver slice must reject
      missing association edges, duplicate associations, wrong target/source
      node types, wrong handles, stale validation sessions, and unavailable/
      invalid associated inference descriptors with dedicated typed
      diagnostics.
      Descriptor cleanup resolution: rewrite the graph-facing
      `dependency-environment` descriptor to remove model/task/backend/platform
      authority fields (`pumas_model_ref`, `model_id`, `model_type`,
      `task_type_primary`, `backend_key`, `platform_context`, and direct
      `dependency_requirements` input). Keep only sidecar-owned user choices
      and display state: selected binding ids, mode, manual override patches,
      backend-issued dependency status, backend-issued environment reference,
      and action/activity presentation state. If a backend-issued
      requirements/status snapshot is persisted for display/history, it must be
      treated as stale unless it matches the current graph revision,
      validation session, descriptor fingerprint, and dependency planning
      identity; workflow-service must never use it as request authority.
      Reclassify `dependency-environment` as a control/manual descriptor rather
      than a processing/batch node. Its `Task::run` must continue to fail closed
      until retired entirely from node-engine execution paths, and the
      workflow-service action endpoint remains the only active resolve/check/
      install path.
      Frontend cleanup resolution: retire `dependencyEnvironmentSources.ts`
      path-era subject inference and tests that prove successful
      `modelPath`/`model_path`, `backend_key`, package-fact, or
      `platform_context` dependency action flows. Frontend dependency UI may
      display backend validation/action state and maintain transient form
      inputs, but it must not synthesize dependency subjects, requirements, or
      platform context from graph edges. Selected binding ids and manual
      override patches are authored graph inputs; dependency status,
      requirements snapshots, environment references, and action activity are
      backend-issued display projections. Frontend code must not update those
      backend-owned fields optimistically or use them as action authority. If
      activity is kept in node data for UX continuity, it is presentation-only
      history and must never affect validation, scheduler admission, dependency
      planning, or runtime handoff.
      Scheduler/runtime boundary resolution: canonical dependency readiness
      flows through dependency planning preflight/readiness proof into scheduler
      dispatch and runtime handoff. Retire node-engine/embedded-runtime
      `environment_ref` input gates as dependency-admission authority once the
      scheduler proof path is wired. Runtime executors may receive the selected
      environment identity only from scheduler handoff, not from graph-authored
      dependency-environment node output data.
      Documentation/traceability resolution: each production slice touching
      these contracts must update the relevant source README in the same commit
      (`crates/workflow-nodes/src/processing/README.md`,
      `crates/pantograph-node-contracts/src/README.md` if the port value type
      is added there, `crates/pantograph-workflow-service/src/graph/README.md`,
      frontend node/service READMEs, and scheduler/runtime READMEs when the
      readiness-proof handoff is wired) or add an ADR explaining why a README
      update is insufficient.
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
      Re-plan update after the Stage 1 publisher slice: Stage 2 needs a
      workflow-service fact-source boundary before the event-driven lifecycle
      owner can start validation from graph edits. Decision: implement option 2
      now with option 3 discipline. Add a focused provider API, for example
      `InferenceInterfaceFactsProvider`, that accepts typed graph resolution
      inputs and returns per-node `InferenceInterfaceResolverFacts` from Pumas
      model/artifact readiness, inference capability facts, and runtime
      availability. The provider may be async, but graph-session locks must be
      released before it runs. The default provider fails closed with typed
      unavailable/not-implemented facts. Tests may inject facts directly, but
      Tauri and frontend callers must never provide raw Pumas facts, runtime
      facts, package summaries, load targets, paths, or capability blobs as
      validation authority. If this provider grows beyond simple fact lookup,
      promote it later into a dedicated workflow-service resolver service
      without changing the sync publisher or creating a second resolution path.
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
- [x] Make descriptor task kind authoritative for scheduler materialization.
      Graph-authored `task_kind` remains an optional resolver constraint only;
      scheduler task projection/materialization consumes the resolved descriptor
      task kind and fails closed when explicit graph constraints cannot be
      satisfied.
      Re-plan boundary discovered 2026-05-26: current
      `workflow_scheduler_task_graph` receives only the saved `WorkflowGraph`
      and still parses raw inference-node `node.data.task_kind` as execution
      authority. Implementing this item directly would either preserve the
      retired raw graph field as scheduler authority or fail every runtime
      inference task because descriptor-backed validation state is not yet an
      input to task projection. Required design decision: choose the scheduler
      projection boundary that supplies current descriptor validation state to
      task materialization before editing `task_graph.rs`.
      Options to resolve:
      1. Remove raw `task_kind` parsing immediately and make all inference
         scheduler tasks fail closed until admission/materialization is rebuilt.
         This is strict but blocks real inference-run testing.
      2. Add a descriptor-backed projection input to the task graph builder so
         scheduler materialization consumes current validation-state records:
         descriptor task kind, descriptor fingerprint, validated model ref, and
         validated runtime/device/trait constraints. This is the cleanest
         incremental code path because the existing task graph builder can stay
         deterministic and tests can inject validation state.
      3. Route all scheduler task graph creation through a graph-session or
         queue-admission service that owns validation-state lookup before
         calling task projection. This is the eventual production shape, but it
         is broader because queue admission, saved workflow submission, and
         scheduler materialization all need the same owner.
      Recommendation: implement option 2 first, shaped so option 3 can call it.
      Keep raw graph `task_kind`, runtime, device, and trait values as resolver
      inputs only; once descriptor-backed projection input exists, delete the
      raw execution-authority parsing path instead of keeping a compatibility
      branch.
      Decision recorded 2026-05-26: use option 2 for the next implementation
      slice and retain option 3 as the later queue-admission/session ownership
      boundary. The option 2 slice must add a deterministic descriptor-backed
      projection input to `workflow_scheduler_task_graph` or an adjacent
      builder function. That input is keyed by inference node id and carries the
      current validation record needed for task materialization: resolved
      descriptor task kind, descriptor fingerprint, validated path-free Pumas
      model ref, validated runtime/device constraints, validated trait
      settings, and typed blocking diagnostics when the descriptor is missing,
      stale, unavailable, or invalid. After this input exists, remove
      `required_task_type` and any scheduler-authority parsing of raw
      `node.data.task_kind`. Raw graph fields remain extractor/resolver inputs
      only.
- [x] Add the workflow-service inference-interface fact-provider boundary before
      the event-driven validation lifecycle owner. The provider accepts typed
      request inputs from the strict extractor, resolves Pumas model/artifact
      readiness, inference capability facts, and runtime availability, and
      returns bounded `InferenceInterfaceResolverFacts` keyed by node. The
      initial provider may return typed unavailable/not-implemented facts until
      production Pumas/runtime adapters are wired, but it must not guess from
      names, model paths, package facts, frontend state, Tauri payloads, or
      runtime-host execution payloads. Keep the provider small and injectable
      for tests; promote it later to a dedicated resolver service only if
      lifecycle orchestration requires it.
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
      The dependency-environment target is a sidecar/control node. It must be
      resolved by workflow-service to exactly one associated inference node
      before request derivation. Do not connect it to inference result outputs
      or let it consume generated media/text data. Do not make the node-engine
      execute dependency actions. If the graph cannot prove the association
      from canonical typed structure, return typed diagnostics instead of
      falling back to path-shaped frontend state, package facts, or arbitrary
      edge guesses.
      Implementation sequence for the action-intent slice:
      1. Extend shared diagnostics with sidecar-specific codes for wrong target
         type, missing association, duplicate association, invalid association
         handle/type, stale associated inference descriptor, and unavailable or
         invalid associated inference descriptor. The first implementation
         must name these in the shared contract surface rather than returning
         stringly typed messages: `DependencySidecarTargetWrongType`,
         `DependencySidecarAssociationMissing`,
         `DependencySidecarAssociationDuplicate`,
         `DependencySidecarAssociationInvalid`,
         `DependencySidecarDescriptorStale`,
         `DependencySidecarDescriptorUnavailable`, and
         `DependencySidecarDescriptorInvalid`.
      2. Add the explicit typed association/control port pair to
         `dependency-environment` and `llm-inference` descriptors, including
         the `dependency_environment_sidecar` constants and first-class
         `DependencyEnvironmentSidecar`/`dependency_environment_sidecar` port
         value type in `crates/node-engine/src/types.rs`,
         `crates/pantograph-node-contracts/src/lib.rs`,
         `crates/pantograph-workflow-service/src/graph/types.rs`, frontend port
         typing, and workflow-node contract conversion code. Tests must prove
         the ports are association-only, exact typed compatibility rejects
         generic JSON/component/any shortcuts where possible, scheduler
         task-graph projection does not materialize the association as a
         runtime input binding, and retired model/path/backend/platform ports
         are absent from dependency-environment.
      3. Add a focused workflow-service dependency action subject resolver
         module that snapshots the graph under lock, releases the lock, proves
         the single associated inference node from the typed association edge,
         then joins to the current validation-state owner.
      4. Derive `DependencyEnvironmentRequest` only after subject resolution,
         current executable descriptor validation, and current dependency
         planning requirements identity are available. `Resolve` may create the
         current requirements identity; `Check` and `Install` must fail closed
         until it exists.
         Re-plan decision: implement this as a narrow backend-owned dependency
         requirements proof in workflow-service current validation state first,
         with the contract shaped so `pantograph-dependency-planning` can later
         become the producer without changing graph editor, Tauri, scheduler, or
         node-engine callers. The proof must be bounded, path-free, and keyed to
         the associated inference node: graph revision, validation session id,
         descriptor fingerprint, validated `PumasModelRef`, task kind, validated
         runtime/device/trait constraints, requirements id or fingerprint, proof
         status, and typed diagnostics. It must not store executable paths,
         Pumas package facts, runtime load targets, scheduler dispatch decisions,
         frontend display state, media payloads, or arbitrary JSON metadata.
         `Resolve` may create or refresh this proof through the backend
         dependency-planning boundary; `Check` and `Install` require an existing
         current proof and return typed missing/stale/invalid diagnostics instead
         of deriving partial requests.
      5. Remove frontend path-era dependency subject inference and embedded
         runtime/node-engine `environment_ref` gates as active dependency
         admission paths once scheduler readiness proof is wired.
      6. Add a cross-layer acceptance test before broadening the feature:
         create a draft graph with `puma-lib -> llm-inference` plus a
         `dependency-environment` sidecar association, publish/record current
         descriptor validation, send `DependencyEnvironmentActionIntent` for the
         dependency node, and assert the resolver reaches the associated
         inference subject or returns the expected typed missing-requirements
         diagnostic without frontend/Tauri model/path data. Adjacent negative
         tests must cover missing association, duplicate association, wrong
         handle/type, wrong target node type, stale validation session, and
         unavailable/invalid associated inference descriptor.
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
      - Sidecar diagnostic contract sub-slice completed on 2026-05-26:
        `pantograph-inference-interface-contracts` now exposes typed
        `DependencySidecar*` diagnostic codes for wrong target type, missing
        association, duplicate association, invalid association, stale
        descriptor, unavailable descriptor, and invalid descriptor. Contract
        tests prove the snake_case wire names round-trip through serde so later
        workflow-service resolvers can return typed diagnostics without message
        parsing, frontend path inference, or partial dependency requests.
        Verification passed: `cargo fmt --package
        pantograph-inference-interface-contracts`; `cargo test -p
        pantograph-inference-interface-contracts`.
      - Sidecar port value contract sub-slice completed on 2026-05-26:
        `dependency_environment_sidecar` now exists as an exact-only port value
        type across `node-engine`, `pantograph-node-contracts`,
        workflow-service graph DTOs, workflow-node contract projection, the app
        workflow TypeScript types, and the reusable `svelte-graph` package
        types. Compatibility tests prove it connects only to the same sidecar
        type and rejects `any`/`json` shortcuts, while projection tests prove
        Rust DTO conversions preserve the type. Remaining implementation work
        is adding the actual descriptor port pair and workflow-service subject
        resolver. Verification passed: `cargo fmt`; `cargo test -p
        node-engine test_port_data_type_compatibility`; `cargo test -p
        pantograph-node-contracts port_value_type_compatibility_matches_backend_rules`;
        `cargo test -p workflow-nodes
        projection_preserves_extended_engine_value_types`; `cargo test -p
        pantograph-workflow-service
        dependency_environment_sidecar_port_type_projects_through_contract_type`;
        `npm run typecheck`.
      - Descriptor port-pair sub-slice completed on 2026-05-26:
        `dependency-environment` is now a control/manual descriptor that
        exposes only authored sidecar choices plus the typed
        `dependency_environment_sidecar` association output. It no longer
        exposes graph-facing `pumas_model_ref`, model/task/backend/platform
        authority fields, `dependency_requirements`, `environment_ref`, or
        dependency status ports. Canonical `llm-inference` now exposes the
        matching optional `dependency_environment_sidecar` input while keeping
        execution fail-closed through the typed inference gateway. Verification
        passed: `cargo fmt`; `cargo test -p workflow-nodes
        test_descriptor_has_canonical_inference_contract_ports`; `cargo test -p
        workflow-nodes test_descriptor_has_required_ports`; `cargo test -p
        workflow-nodes contract_projection_preserves_port_directions_and_value_types`;
        `cargo test -p workflow-nodes`.
        Discovered follow-up: app and reusable graph mock backends still carry
        older dynamic `llm-inference` mock ports. They must be cleaned up in
        the frontend no-legacy/mock cleanup slice rather than treated as
        canonical descriptor facts.
      - Workflow-service sidecar subject resolver sub-slice completed on
        2026-05-26: `dependency_environment_subject.rs` now validates
        dependency-environment action targets through the exact
        `dependency_environment_sidecar` association edge before dependency
        request derivation. It accepts only a `dependency-environment` target
        associated to exactly one canonical `llm-inference` node, rejects
        missing/duplicate/wrong-handle/wrong-type associations with typed
        `DependencySidecar*` diagnostics, and joins dependency action state to
        the associated inference node's current validation record rather than
        the sidecar node id. Verification passed: `cargo fmt`; `cargo test -p
        pantograph-workflow-service dependency_environment_subject`; `cargo
        test -p pantograph-workflow-service
        dependency_environment_action_intent`; `cargo test -p
        pantograph-workflow-service
        action_intent_state_accepts_executable_summary_until_requirements_derivation`;
        `cargo test -p pantograph-workflow-service
        publish_inference_validation_session_records_current_summary`.
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
      - Frontend/transport boundary update on 2026-05-26: the next active
        dependency-environment implementation slice must replace
        `DependencyEnvironmentNode.svelte` direct calls to
        `run_dependency_environment_action` and any frontend
        `DependencyEnvironmentRequest` payload builders with a graph-coordinator
        callback/action API. The coordinator builds
        `DependencyEnvironmentActionIntent` from graph session state and forwards
        it through a transport-only Tauri command to workflow-service. The old
        `run_dependency_environment_action(DependencyEnvironmentRequest)` Tauri
        command and Tauri-local dependency validation/result construction are
        retirement targets, not alternate supported paths.
      - Frontend/transport boundary sub-slice completed on 2026-05-26: the
        active dependency-environment node now delegates resolve/check/install
        through a graph-coordinator context, `WorkflowGraph.svelte` supplies the
        coordinator, `workflowGraphBackendActions.ts` adds the active graph edit
        session and `WorkflowGraphRevision`, and Tauri registers only the
        transport-only `resolve_dependency_environment_action_intent` command
        for this path. The retired
        `run_dependency_environment_action(DependencyEnvironmentRequest)`
        command, Tauri-local not-implemented dependency result construction,
        and frontend path-shaped action payload builder were removed. Remaining
        work is descriptor-backed dependency request derivation after executable
        validation summaries carry current dependency requirements.
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
      - Sidecar subject join sub-slice completed on 2026-05-26:
        dependency-environment action freshness checks now consume a typed
        `DependencyEnvironmentActionSubjectResolution` from workflow-service
        graph structure. The validation-state owner uses the resolved inference
        node id when checking current node-scoped validation records, and returns
        typed sidecar descriptor diagnostics for missing, unavailable, or
        invalid associated descriptors. Remaining follow-up: derive the
        canonical `DependencyEnvironmentRequest` only after the current
        inference descriptor record carries a bounded dependency-requirements
        identity/proof; clean frontend mock descriptor data that still reflects
        retired static inference ports.
      - Scheduler task-graph sidecar exclusion sub-slice completed on
        2026-05-26: workflow-service task graph projection now treats
        `dependency_environment_sidecar` as a control-association edge, not as
        a scheduler materialized input binding or dependency task id. The
        projection imports the shared port id from `workflow-nodes` instead of
        adding a new string convention. Verification passed: `cargo fmt`;
        `cargo test -p pantograph-workflow-service
        sidecar_association_is_not_materialized_as_scheduler_input`; `cargo
        test -p pantograph-workflow-service task_binding_resolution`.
      - Dependency requirements proof re-plan decision recorded on 2026-05-26:
        continue with option 2 using option 3 discipline. The next code slice
        adds a focused workflow-service proof record and owner API that extends
        current validation-state node records just enough to derive dependency
        actions safely, while keeping the shape compatible with a later
        `pantograph-dependency-planning` producer. The slice must include tests
        proving `Resolve` can create or refresh the proof, `Check`/`Install`
        fail closed without a current proof, stale graph revision/session/
        descriptor mismatches block request derivation, and no path/package/
        frontend/runtime-load fields appear in the proof or request boundary.
      - Dependency requirements proof record sub-slice completed on 2026-05-26:
        `graph/inference_validation_state.rs` now stores a bounded
        `CurrentDependencyRequirementsProof` on the associated inference node
        record. The proof is keyed to graph revision, validation session,
        descriptor fingerprint, validated path-free `PumasModelRef`, task kind,
        runtime/device constraints, and dependency requirements id. Current
        proofs make dependency action intent resolution return `RequestReady`;
        missing, stale, unavailable, or invalid proofs return typed blocking
        diagnostics. Validation publication refresh clears existing proofs so
        descriptor/session changes cannot silently reuse stale requirements.
        The forward-facing proof owner API is temporarily marked
        `allow(dead_code)` until the dependency-planning producer slice calls
        it. Verification passed: `cargo fmt`; `cargo test -p
        pantograph-workflow-service inference_validation_state`.
      - Re-plan boundary discovered on 2026-05-26: the next implementation
        step needs a backend producer for the dependency requirements proof, but
        `pantograph-dependency-planning` currently exposes request, identity,
        environment, and preflight DTOs rather than a producer API that derives
        a requirements id/proof from a validated planning request. Implementing
        request derivation inside workflow-service would make workflow-service
        own dependency-planning policy; synthesizing ids locally would be a
        hidden fallback.
        Decision: use option 1. Add the producer API in
        `pantograph-dependency-planning` now, then have workflow-service call it
        to create/refresh proofs for `Resolve` and store the returned proof in
        current validation state. Workflow-service remains the graph/session
        owner and proof consumer; dependency-planning owns requirements identity,
        proof status, and dependency-planning diagnostics.
        Implementation slices:
        1. Add the dependency-planning producer contract in
           `pantograph-dependency-planning` with focused tests for stable,
           path-free requirements id derivation, selected model ref handling,
           platform key inclusion, selected binding ids, dependency override
           fingerprint inclusion, runtime/device constraint inclusion,
           dependency-planning-local trait intent inclusion, canonical
           hash-based requirements id derivation, and typed unavailable or
           invalid diagnostics. The producer is pure contract/domain logic
           unless supplied typed availability facts: it must not call Pumas,
           inspect files, select runtime/device policy, infer package readiness
           from ambient local state, or depend on scheduler DTOs such as
           `SchedulerTraitSetting`.
        2. Wire workflow-service `Resolve` to call the producer and update the
           current proof through the existing validation-state proof API; keep
           `Check` and `Install` fail-closed when no current proof exists.
           Workflow-service must snapshot the current graph, resolve the
           sidecar subject, extract and validate sidecar-authored selected
           binding ids and manual override patches, then release graph state
           before calling the producer. The proof stored in workflow-service
           must preserve dependency-planning diagnostics; conversion to
           `InferenceInterfaceDiagnostic` is a response-boundary mapping only.
        3. Derive canonical `DependencyEnvironmentRequest` only after the
           producer proof, current executable descriptor summary, sidecar
           authored choices, and dependency-planning identity agree.
        Standards gates for these slices:
        - Contract API: producer request/result/proof/status/diagnostic/error
          types are public `pantograph-dependency-planning` contract types,
          re-exported from `lib.rs`, validated through typed constructors or
          `TryFrom`, and marked `#[non_exhaustive]` where future runtimes,
          model families, or availability states may extend them. Do not add
          `Result<T, String>`, `anyhow`, raw metadata maps, or async-only APIs
          to pure producer logic.
        - Dependency ownership: if stable id hashing needs `blake3`, first add
          it to root `[workspace.dependencies]`, convert existing direct
          workspace-member `blake3` declarations to `{ workspace = true }`,
          and include `Cargo.toml`/`Cargo.lock` in the same atomic dependency
          hygiene sub-slice. Do not introduce a second hash crate or hide the
          dependency in a broader owner.
        - Parse-once boundary: workflow-service must deserialize and validate
          sidecar `selected_binding_ids` and manual override patches into
          dependency-planning typed values before the producer call. Raw JSON,
          frontend display state, package facts, and path-era aliases are not
          valid producer inputs.
        - Concurrency/freshness: workflow-service snapshots the graph under
          lock, releases graph state before producer/fact work, and stores the
          returned proof only after rechecking graph revision and validation
          session. A changed graph/session becomes a stale typed diagnostic,
          not a best-effort write.
        - Documentation: implementation slices that add or change producer
          modules update the relevant crate/module README with API consumer
          contract, structured producer contract, invariants, dependencies, and
          revisit triggers.
        - Verification: add dependency-planning unit/contract tests for stable
          path-free id derivation and diagnostics; workflow-service tests for
          `Resolve` proof refresh, `Check`/`Install` fail-closed behavior,
          stale graph/session rejection, dependency-planning diagnostic
          preservation and response-boundary mapping, and a cross-layer
          acceptance path from graph sidecar association to request readiness.
        No-fallback rule: do not synthesize temporary requirements ids in
        workflow-service, do not move dependency-planning policy into
        workflow-service, and do not carry executable paths, package facts,
        runtime load targets, frontend display state, media, or scheduler
        trait-setting/dispatch decisions in the proof. Do not concatenate raw
        model/user strings into requirements ids; use canonical typed input plus
        a stable hash that satisfies `DependencyRequirementsId`.
        Re-plan triggers: stop if the producer API requires Pumas-owned path
        resolution, runtime-host load targets, scheduler queue state, or a
        frontend/Tauri contract change before it can produce a typed path-free
        proof. Also stop if dependency availability must be discovered without
        typed availability facts, if scheduler DTOs would need to enter
        `pantograph-dependency-planning`, or if workflow-service would need to
        synthesize requirements ids/diagnostics locally.
      - Dependency-planning producer contract slice completed on 2026-05-26:
        `pantograph-dependency-planning` now exposes a pure synchronous
        dependency requirements proof producer. It derives path-free
        `DependencyRequirementsId` and `DependencyOverrideFingerprint` values
        from canonical typed planning identity, selected bindings, override
        patches, scheduler intent, platform key, and dependency-planning-local
        trait intents. Optional typed availability facts map to proof status
        and dependency-planning diagnostics without Pumas lookup, filesystem
        inspection, scheduler policy, runtime load targets, or frontend state.
        The slice added typed trait intent DTOs, public producer request/proof
        DTOs, validated wrappers, contract tests, README updates, and centralized
        existing `blake3` usage under root `[workspace.dependencies]`.
        Deviation: the existing planning request fixture still demonstrates a
        migration-era `selected_artifact_path`; producer tests explicitly clear
        it for path-free proof production and keep a rejection test proving the
        producer does not accept path-carrying identity. Verification passed:
        `cargo fmt`; `cargo test -p pantograph-dependency-planning`; `cargo
        check -p pantograph-dependency-planning -p node-engine`.
        Remaining follow-up: wire workflow-service `Resolve` to call this
        producer, parse sidecar JSON once into typed dependency-planning values,
        recheck graph revision/session before storing the proof, and keep
        `Check`/`Install` fail-closed until a current proof exists.
      - Workflow-service `Resolve` producer wiring slice completed on
        2026-05-26: dependency-environment action intent resolution now
        snapshots the target sidecar node under the graph lock, parses
        `selected_binding_ids` and `manual_overrides` once into
        dependency-planning typed values, releases graph state, and lets the
        current validation-state owner call the dependency-planning producer to
        create the proof for `Resolve`. `Check` and `Install` remain
        fail-closed until a current proof exists; after `Resolve` stores the
        proof, `Check` can become request-ready for the same graph revision and
        validation session. Malformed sidecar choices return typed
        `InvalidOption` diagnostics instead of transport errors or fallback
        defaults. The stored proof now preserves
        `DependencyPlanningDiagnostic` rows, keeping conversion to
        `InferenceInterfaceDiagnostic` at graph/action response boundaries.
        Verification passed: `cargo fmt`; `cargo test -p
        pantograph-workflow-service dependency_environment_action_intent`;
        `cargo test -p pantograph-workflow-service inference_validation_state`;
        `cargo check -p pantograph-workflow-service`. Existing warning:
        `set_active_run_execution_plan` remains unused and unrelated to this
        slice. Remaining follow-up: derive the canonical
        `DependencyEnvironmentRequest` only after the producer proof, current
        executable descriptor summary, sidecar-authored choices, and
        dependency-planning identity agree.
      - Workflow-service dependency-environment request derivation slice
        completed on 2026-05-26: `RequestReady` now requires workflow-service
        to derive and validate a canonical path-free
        `DependencyEnvironmentRequest` from the current executable descriptor
        state, dependency-planning producer proof, host platform context,
        sidecar-selected bindings, and sidecar manual override patches.
        `Resolve` creates or refreshes the current proof through the same
        derivation path; `Check` and `Install` compare the current sidecar
        choices against the stored proof and return typed stale diagnostics
        when they no longer agree. The request remains backend-internal and is
        not carried by frontend or Tauri action intents/results. Verification
        passed: `cargo fmt`; `cargo test -p pantograph-workflow-service
        inference_validation_state`. Remaining follow-up: connect the validated
        request to the canonical dependency-environment service/result boundary
        once that service owner exists, and keep frontend/Tauri as action-intent
        transport only.
      - Re-plan boundary discovered on 2026-05-26: the validated
        `DependencyEnvironmentRequest` now exists inside workflow-service, but
        there is no canonical workflow-service dependency-environment
        service/result owner to consume it. The remaining executable dependency
        paths still route through retired `ModelDependencyRequest`/
        `model_path` resolver contracts in node-engine, embedded-runtime, and
        Tauri model-dependency commands. Continuing implementation directly
        would either preserve the legacy hydration path or require inventing a
        new service boundary without a recorded owner contract. Required
        re-plan decision: define the backend owner for
        `DependencyEnvironmentRequest -> DependencyEnvironmentResult`, its
        relationship to Pumas/dependency planning, and the deletion sequence
        for the retired `ModelDependencyRequest` dependency-environment
        execution path.
      - Re-plan decision recorded on 2026-05-26: use option 2, a dedicated
        backend dependency-environment service crate, as the canonical owner of
        `ValidatedDependencyEnvironmentRequest -> DependencyEnvironmentResult`.
        Suggested crate boundary: `pantograph-dependency-environment-service`.
        Workflow-service remains the graph/session adapter: it derives and
        validates the request from current descriptor/dependency-planning state,
        calls the service, and projects the typed result back into
        dependency-environment action responses or backend-owned display state.
        Tauri remains transport-only, the graph editor still sends only action
        intents, and node-engine/embedded-runtime must not execute
        dependency-environment actions.
      - Service design constraints: the new service consumes only canonical
        dependency-planning DTOs and Pumas-approved dependency/environment APIs.
        It must not accept `ModelDependencyRequest`, `model_path`, local load
        paths, package-fact blobs, frontend-selected platform context, or graph
        display metadata. Its first implementation may return typed
        unavailable/not-implemented `DependencyEnvironmentResult` states for
        Pumas operations that are not yet exposed, but those states must be
        explicit diagnostics, not fallbacks to legacy resolvers. If Pumas lacks
        a required operation, record the missing Pumas contract before
        implementing an adapter.
      - Standards/blast-radius tightening recorded on 2026-05-26: the service
        crate must expose a narrow boundary that depends on
        `pantograph-dependency-planning` and provider traits only. It must not
        depend on workflow-service, node-engine, embedded-runtime, Tauri,
        frontend DTOs, or graph display state. Keep pure request/result
        validation and projection synchronous; use async only at concrete
        provider methods that perform Pumas calls, installs, file inspection,
        process spawning, or other I/O.
      - Result-contract tightening: add a validated result boundary before any
        production caller trusts service output. This may be a
        `ValidatedDependencyEnvironmentResult` wrapper or service-owned
        constructors, but it must enforce semantic invariants that the current
        row-shape validation does not fully guarantee: ready results carry the
        required environment identity/proof fields, operation/status
        combinations are explicit, selected binding identifiers remain unique,
        unavailable/not-implemented states carry diagnostics, and path-shaped
        values cannot enter the result.
      - Workflow-service lock rule: dependency-environment action handling must
        snapshot graph/session state under lock, derive the validated request
        from the snapshot, release locks before calling the service or any
        provider, then reacquire only to publish the result if the graph
        revision/session id still matches. Do not await Pumas, install, file,
        or service work while holding validation/session locks.
      - Legacy deletion gates: retire frontend request-building helpers such as
        dependency-environment source resolution that gather `modelPath`,
        backend keys, platform context, package facts, or graph-authored
        requirements for action requests. The frontend may display validation
        state and send action intents only. Remove dependency-environment
        execution through embedded-runtime/node-engine `ModelDependencyRequest`
        instead of wrapping it, and add source-search verification that no
        dependency-environment action path calls the Tauri
        model-dependency commands.
      - Scheduler/admission rule: scheduler readiness must consume a canonical
        dependency readiness/admission proof derived from validated dependency
        results. Do not pass graph-authored dependency-environment node outputs,
        frontend display state, or full dependency result payloads into
        scheduling policy.
      - Lifecycle ownership rule: dependency resolve/check/install lifecycle
        state, status caches, operation de-duplication, and install locks belong
        to the service or its concrete provider. Tauri may transport commands
        and events only; it must not own dependency policy or business state.
      - Composition-root rule: feature/domain modules may depend on the service
        facade and provider traits, but they must not create concrete Pumas,
        process, filesystem, runtime, or install-provider infrastructure ad hoc.
        Concrete providers are selected and wired at the backend composition
        boundary, with fake/null providers swapped there for tests.
      - Rust API standards gate: public service APIs must use typed request,
        result, status, operation, and diagnostic enums/newtypes instead of raw
        strings or path-shaped primitives. Fallible public APIs return
        structured error enums, not `Result<T, String>` or `anyhow`; library
        code must not use `unwrap`/`expect` in request, lifecycle, or provider
        paths. Public extensible enums/structs should be marked
        `#[non_exhaustive]`, validated values and builders should be
        `#[must_use]`, and constructors must remain private where validation is
        required.
      - Dependency standards gate: before adding any new crate dependency,
        inspect the existing Cargo tree, prefer `std` or existing workspace
        dependencies for small behavior, declare each dependency in the
        workspace member that directly uses it, and feature-gate or justify any
        heavy/platform-specific provider dependency. The service crate must not
        gain broad runtime/framework dependencies merely to expose the core
        contract.
      - Documentation/traceability gate: the new crate must include crate-level
        Rust docs and README/module README coverage for every new `src/`
        directory, including purpose, invariants, API consumer contract,
        lifecycle/error semantics, rejected alternatives, and revisit triggers.
        Any existing source directory touched by the implementation must have
        its README updated or an ADR/plan traceability entry explaining why the
        boundary changed.
      - Verification gate: every implementation slice must include focused
        public-API tests for validated request/result construction and typed
        diagnostics, plus at least one thin vertical-slice test that proves
        workflow-service derives a request, calls the fake/null service without
        holding graph/session locks, rejects stale graph revisions/session ids,
        and publishes only backend-owned display/status state. Tests that use
        caches, install locks, temp files, or process-global state must isolate
        durable resources per test or serialize with an explicit documented
        guard. Add source-search checks that retired path-shaped fields and
        commands are absent from the dependency-environment action path.
      - Background-work gate: the service crate must not create a global async
        runtime or spawn detached tasks. If later provider work requires
        background installs, polling, retries, or subprocess supervision, that
        work needs an explicit lifecycle owner with cancellation, tracked task
        handles, shutdown behavior, idempotency/overlap rules, and tracing at
        the lifecycle owner.
      - Implementation sequence for option 2:
        1. Add the dedicated crate with a small public API that accepts
           `ValidatedDependencyEnvironmentRequest` and returns a validated
           dependency-environment result for resolve/check/install. Define the
           validated result wrapper or service constructors, structured error
           types, crate docs, and README/module contract docs in this slice.
        2. Add a no-I/O/null implementation or fake provider for tests that
           proves request validation, result invariants, typed diagnostics, no
           path-shaped fields, no dependency on legacy resolver crates, and no
           unintended third-party dependency growth.
        3. Wire workflow-service dependency action handling to call the service
           from a lock-free snapshot after request derivation. The action
           result must remain an intent response; only backend-owned
           display/status state may receive the full dependency result, and
           publication must reject stale graph revisions/session ids.
        4. Replace or delete active dependency-environment execution through
           node-engine/embedded-runtime `ModelDependencyRequest`; do not keep a
           compatibility adapter.
        5. Remove Tauri model-dependency command usage from the
           dependency-environment action path once workflow-service uses the new
           service, and verify the frontend action path sends only descriptor
           action intents.
        6. After the service returns ready environment identity, feed the
           scheduler readiness/admission proof path from canonical dependency
           results rather than graph-authored dependency-environment outputs.
      - Dependency-environment service contract slice completed on 2026-05-26:
        added `pantograph-dependency-environment-service` as a no-I/O service
        facade over provider traits and added
        `ValidatedDependencyEnvironmentResult` to the shared
        dependency-planning contract. The slice accepts only
        `ValidatedDependencyEnvironmentRequest`, validates provider output
        before returning it, and includes a not-implemented provider that emits
        typed diagnostic results instead of falling back to
        `ModelDependencyRequest`.
      - Files touched by the slice: workspace `Cargo.toml`/`Cargo.lock`,
        `crates/pantograph-dependency-planning/src/environment.rs`,
        `crates/pantograph-dependency-planning/src/lib.rs`, and the new
        `crates/pantograph-dependency-environment-service/` crate with README,
        source README, public API tests, and crate docs.
      - No-fallback/no-legacy result: the new crate has no dependency on
        node-engine, embedded-runtime, Tauri, workflow-service, frontend DTOs,
        Pumas, filesystem/process/runtime infrastructure, or legacy resolver
        crates. Path-shaped and `ModelDependencyRequest` terms appear only in
        rejection guards, documentation, and tests.
      - Verification passed: `cargo tree -p
        pantograph-dependency-environment-service --depth 1`; `cargo test -p
        pantograph-dependency-environment-service`; `cargo test -p
        pantograph-dependency-planning dependency_environment`; `cargo fmt`;
        `git diff --check` for touched implementation files; targeted
        source-search for legacy path-shaped fields and commands.
      - Deviation: this slice does not yet wire workflow-service to the new
        service. That remains the next vertical slice so lock-free snapshotting,
        stale revision/session rejection, and backend-owned display projection
        can be tested at the workflow-service boundary.
      - Workflow-service dependency-environment service wiring slice completed
        on 2026-05-26: `GraphSessionStore` now owns an injectable canonical
        dependency-environment service facade, derives the validated
        dependency-environment request from current validation state, releases
        graph/session and validation-state locks, then calls the service and
        returns an action-intent result. Invalid provider output is mapped to a
        typed blocked diagnostic; valid service output keeps the action response
        as `RequestReady` while full dependency-environment result ownership
        remains backend-side.
      - Files touched by the slice:
        `crates/pantograph-workflow-service/Cargo.toml`,
        `crates/pantograph-workflow-service/src/graph/session.rs`,
        `crates/pantograph-workflow-service/src/graph/inference_validation_state.rs`,
        `crates/pantograph-workflow-service/src/graph/session_tests.rs`,
        `crates/pantograph-workflow-service/src/graph/README.md`, and the
        service crate provider trait to support shared injected providers.
      - No-fallback/no-legacy result: workflow-service calls the canonical
        service only after request derivation and does not call Tauri
        model-dependency commands, node-engine dependency execution, embedded
        runtime dependency execution, `ModelDependencyRequest`, model paths, or
        local load paths.
      - Verification passed: `cargo test -p pantograph-workflow-service
        dependency_environment_action_intent`; `cargo test -p
        pantograph-dependency-environment-service`; `cargo fmt`; `git diff
        --check` for touched implementation files; targeted source-search for
        retired path-shaped fields and dependency command names in the touched
        workflow-service action path. Existing unrelated warning remains:
        `set_active_run_execution_plan` is unused in workflow-service scheduler
        store.
      - Remaining follow-up: persist/project backend-owned dependency result
        display/status state if the graph editor needs to show more than action
        intent readiness, then delete or replace active dependency-environment
        execution through node-engine/embedded-runtime `ModelDependencyRequest`.
      - Embedded-runtime dependency-environment execution deletion slice
        completed on 2026-05-26: `TauriTaskExecutor` now rejects
        `dependency-environment` task execution with a typed execution error
        that names workflow-service dependency-environment service ownership,
        does not fall through to core execution, and does not call
        `ModelDependencyResolver`. The slice removed the old
        `execute_dependency_environment` action path, deleted manifest-emitting
        `environment_ref` helper code, removed dependency-environment
        `backend_key` request projection, and updated embedded-runtime task
        executor READMEs to make dependency preflight separate from
        dependency-environment actions.
      - Files touched by the slice:
        `crates/pantograph-embedded-runtime/src/task_executor.rs`,
        `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment.rs`,
        `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment/helpers.rs`,
        `crates/pantograph-embedded-runtime/src/task_executor_tests.rs`,
        `crates/pantograph-embedded-runtime/src/task_executor_tests/input_helpers.rs`,
        `crates/pantograph-embedded-runtime/src/task_executor/README.md`,
        `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment/README.md`,
        and this plan.
      - No-fallback/no-legacy result: embedded-runtime no longer executes
        dependency-environment resolve/check/install actions, emits no
        graph-authored `environment_ref` manifests, and does not treat
        dependency-environment `backend_key` input as runtime-selection
        authority. Python-backed dependency preflight remains in place only for
        the still-active runtime gate path and remains a separate later
        replacement/deletion target.
      - Verification passed: `cargo test -p pantograph-embedded-runtime
        dependency_environment_execution_is_retired_from_embedded_runtime`;
        `cargo test -p pantograph-embedded-runtime input_helpers`; `cargo test
        -p pantograph-embedded-runtime dependency_preflight`; `cargo fmt`; `git
        diff --check` for touched files; targeted source-search proving the old
        embedded-runtime dependency-environment execution, `environment_ref`
        manifest emission, and dependency-environment `backend_key` projection
        strings are absent from the touched task-executor path.
      - Remaining follow-up: replace or delete the broader
        `ModelDependencyRequest` dependency preflight/model-ref path used by
        Python-backed runtime nodes, then remove Tauri model-dependency command
        usage where it is no longer a backend-owned transport concern.
      - Tauri model-dependency command deletion slice completed on 2026-05-26:
        removed the direct Tauri command registrations and wrappers for
        `resolve_model_dependency_requirements`, `check_model_dependencies`,
        `install_model_dependencies`, `get_model_dependency_status`, and
        `audit_dependency_pin_compliance`, deleted the retired
        `workflow::model_dependency_commands` module, and updated the Tauri
        workflow README to require dependency-environment actions to cross
        Tauri only as workflow-service action intents.
      - Files touched by the slice: `src-tauri/src/app_setup.rs`,
        `src-tauri/src/workflow/commands.rs`, `src-tauri/src/workflow/mod.rs`,
        deleted `src-tauri/src/workflow/model_dependency_commands.rs`,
        `src-tauri/src/workflow/README.md`, and this plan.
      - No-fallback/no-legacy result: the graph editor/frontend path already
        invokes `resolve_dependency_environment_action_intent`, and Tauri no
        longer exposes direct `ModelDependencyRequest` dependency
        resolve/check/install/status commands as compatibility entrypoints.
        The embedded-runtime resolver object remains managed only because the
        still-active Python-backed dependency preflight path depends on it and
        is tracked as the next separate replacement/deletion slice.
      - Verification passed: `cargo fmt`; `git diff --check` for touched files;
        targeted source-search proving direct model-dependency Tauri commands,
        registrations, and the `model_dependency_commands` module are absent
        from `src-tauri/src/workflow`, `src-tauri/src/app_setup.rs`, and
        frontend dependency-environment action paths.
      - Verification blocker: `cargo check -p pantograph` currently fails on an
        existing unrelated app setup error,
        `Arc<WorkflowService>::set_media_conversion_executor` missing at
        `src-tauri/src/app_setup.rs:96`. This slice did not touch that
        composition-root call.
      - Remaining follow-up: replace or delete the broader
        embedded-runtime/node-engine `ModelDependencyRequest` dependency
        preflight/model-ref path, then remove the managed resolver object from
        Tauri app setup once no active runtime path depends on it.
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
      Fact-source tightening: Stage 2 must also reuse the workflow-service fact
      provider. Event-driven validation may trigger from graph edits or explicit
      validation requests, but those triggers pass identity/revision intent only;
      they do not carry resolver facts from frontend or Tauri.
- [ ] Add the workflow-service live validation lifecycle owner before event
      delivery reaches the frontend. The owner must start, cancel, supersede, and
      clean up validation sessions; use bounded event/state buffers with explicit
      overflow/backpressure diagnostics; observe task errors and panics; cancel
      or supersede in-flight work when graph revisions change; and stop accepting
      validation work when a graph/session closes. Domain validation/projection
      remains sync-core; async is limited to fact lookup, persistence, transport,
      and event delivery boundaries.
      Dependency: implement the fact-provider boundary first so the lifecycle
      owner can call provider -> sync publisher -> current-state recorder without
      accepting raw facts from transport callers or duplicating descriptor
      resolution policy.
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
- [x] 2026-05-26 synchronous validation publisher core slice completed:
  - Smallest useful vertical slice: added
    `graph/inference_interface_publication.rs`, an edit-session publication API,
    and node-scoped current validation records so workflow-service can publish a
    validation session from the current draft graph without moving descriptor
    policy into Tauri or frontend code.
  - No-fallback/no-legacy confirmation: publication consumes only strict
    extracted `pumas_model_ref` requests and supplied resolver facts, then uses
    the existing resolver/projection boundary. It does not inspect model paths,
    Pumas load targets, package facts, `inference_settings`, `expand-settings`,
    static all-port metadata, or runtime-host execution payloads as alternate
    sources.
  - Standards result: graph-session locks are held only for canonicalization,
    graph snapshotting, and current revision calculation. Descriptor projection
    runs after the lock is released, current validation state remains owned by
    workflow-service, and source ownership is documented in the graph README.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    inference_interface_publication --lib`; `cargo test -p
    pantograph-workflow-service
    publish_inference_validation_session_records_current_summary --lib`;
    `cargo test -p pantograph-workflow-service inference_validation_state
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and `git diff --check`.
  - Discovered issue: the workflow-service check commands continue to report the
    pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: add the event-driven validation lifecycle owner that
    starts/cancels/supersedes validation work and reuses this synchronous
    publisher before frontend event delivery or queue admission consume live
    validation updates.
- [x] 2026-05-26 live validation fact-source boundary re-plan decision:
  - Decision: implement option 2 now with option 3 discipline. The next
    implementation slice should add a workflow-service fact-provider boundary
    that turns strict graph resolution inputs into per-node
    `InferenceInterfaceResolverFacts` for the existing synchronous publisher.
  - Why: the current publisher is deterministic and backend-owned, but it still
    requires facts supplied by the caller. Event-driven validation cannot start
    from graph edits until workflow-service owns Pumas model/artifact readiness,
    inference capability facts, and runtime availability lookup behind a typed
    provider.
  - No-fallback/no-legacy confirmation: frontend and Tauri must not pass raw
    Pumas facts, runtime facts, package summaries, executable load targets,
    paths, capability blobs, or scheduler decisions as validation authority.
    The default provider must fail closed with typed unavailable/not-implemented
    facts rather than guessing from names or legacy graph metadata.
  - Later objective: if the provider grows beyond simple fact lookup, promote it
    into a dedicated workflow-service resolver service that still reuses the
    sync publisher and does not create a parallel descriptor-resolution path.
- [x] 2026-05-26 workflow-service fact-provider boundary slice completed:
  - Smallest useful vertical slice: added `InferenceInterfaceFactsProvider` and
    the fail-closed `UnavailableInferenceInterfaceFactsProvider`, injected the
    provider into `GraphSessionStore`, and changed
    `publish_inference_validation_session` so transport/session callers request
    validation by graph/session identity while workflow-service supplies facts
    to the sync publisher.
  - No-fallback/no-legacy confirmation: the default provider returns typed
    missing-model-facts projections. The session publisher no longer accepts raw
    resolver facts from callers and still does not read model paths, package
    facts, executable load targets, `inference_settings`, `expand-settings`,
    runtime-host payloads, frontend state, or scheduler decisions as alternate
    descriptor sources.
  - Standards result: graph-session locks are held only for canonicalization,
    graph snapshotting, and revision calculation. Provider fact lookup runs
    after the lock is released, stays injectable for tests, and keeps Tauri and
    frontend as intent/transport callers rather than validation-policy owners.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session --lib`; `cargo test -p
    pantograph-workflow-service inference_interface_publication --lib`;
    `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and `git diff --check`.
  - Discovered issue: the workflow-service check commands continue to report the
    pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: implement the event-driven validation lifecycle owner
    on top of provider -> sync publisher -> current-state recorder before
    frontend event delivery or queue admission consumes live validation updates.
- [x] 2026-05-26 descriptor-task-kind scheduler projection re-plan boundary:
  - Discovered issue: `workflow_scheduler_task_graph` currently receives only
    `WorkflowGraph` and parses raw inference-node `node.data.task_kind` as the
    source for `SchedulableTaskIntent` materialization.
  - Why implementation stops: Milestone 5d now requires graph-authored
    `task_kind`, runtime, device, and trait fields to remain resolver
    constraints only. Scheduler materialization must consume the resolved
    descriptor task kind from current validation state. Changing
    `task_graph.rs` before that descriptor-backed input exists would either
    preserve retired graph-field execution authority or make all runtime
    inference tasks fail before the validation/admission boundary can supply the
    selected descriptor.
  - Re-plan result: options and recommendation are recorded in this checklist.
    The next implementation slice should add a deterministic descriptor-backed
    projection input to the scheduler task graph builder, then delete raw
    graph-field execution parsing instead of keeping a compatibility branch.
- [x] 2026-05-26 descriptor-task-kind scheduler projection decision:
  - Decision: implement option 2 next. Add the descriptor-backed projection
    input at the workflow-service task graph boundary before changing
    scheduler materialization semantics.
  - Slice boundary: allowed production files are the workflow-service task
    graph module/contracts plus directly affected workflow-service call sites
    that currently call `workflow_scheduler_task_graph`. Allowed tests are the
    focused task graph, task binding, external input materialization, and
    session execution tests needed to prove descriptor-backed projection and
    fail-closed diagnostics.
  - No-fallback/no-legacy confirmation: the implementation must delete
    scheduler-authority parsing of raw `node.data.task_kind` rather than
    preserving it as a fallback. Graph-authored `task_kind`, runtime, device,
    and trait values stay on the resolver/extractor side only.
  - Later objective: option 3 will move validation-state lookup and enqueue
    readiness ownership into queue admission or graph session orchestration.
    That owner must call the option 2 projection API instead of creating a
    second scheduler materialization path.
- [x] 2026-05-26 descriptor-backed scheduler task projection slice completed:
  - Smallest useful vertical slice: added
    `WorkflowSchedulerInferenceTaskProjections` and the
    `workflow_scheduler_task_graph_with_inference_projections` builder so
    workflow-service task graph projection receives current descriptor-backed
    inference task records keyed by node id. Runtime inference tasks now build
    `SchedulableTaskIntent` from resolved descriptor task kind, descriptor
    fingerprint, validated path-free Pumas model ref, runtime/device
    constraints, trait settings, and estimate hints supplied by that input.
  - No-fallback/no-legacy confirmation: removed scheduler-authority parsing of
    raw inference-node `node.data.task_kind`, `runtime`, `device`,
    `pumas_model_ref`, and `denoising_scheduler` from `task_graph.rs`.
    Graph-authored values remain resolver/extractor inputs only. Missing,
    stale, unavailable, and invalid descriptor states now surface as typed
    scheduler projection diagnostics instead of falling back to graph data.
  - Binding behavior: descriptor-backed schedulable intent is authoritative,
    while connected upstream node results still gate readiness so inference
    nodes do not execute before graph inputs have materialized.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service task_graph --lib`;
    `cargo test -p pantograph-workflow-service task_binding_resolution --lib`;
    `cargo test -p pantograph-workflow-service task_orchestrator --lib`;
    `cargo test -p pantograph-workflow-service store_task_results --lib`;
    `cargo test -p pantograph-workflow-service external_input_materialization
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; and `cargo check -p
    pantograph-workflow-service --all-features`.
  - Discovered issue: `cargo test -p pantograph-workflow-service
    session_execution --lib` still fails in broader session orchestration tests
    around source-input materialization and runtime-session expectations. This
    was not expanded into the descriptor projection slice because the planned
    queue-admission/session owner is the next boundary that should connect
    current validation state to workflow submission.
  - Remaining follow-up: implement the queue-admission or graph-session owner
    that looks up current validation state and calls the descriptor-backed task
    projection API before scheduler placement. Until then, direct callers of
    `workflow_scheduler_task_graph` intentionally fail closed for runtime
    inference nodes that lack descriptor projections.
- [x] 2026-05-26 current-validation scheduler projection state slice:
  - Smallest useful vertical slice: expose the current inference-validation
    state owner as the source of `WorkflowSchedulerInferenceTaskProjections`,
    add a graph-session wrapper that computes the current canonical graph
    revision, and make focused tests assert executable summaries project ready
    scheduler intent while non-executable summaries fail closed.
  - Allowed write set: `inference_validation_state.rs`,
    `session_inference_validation_api.rs`, `session_tests.rs`,
    `workflow/task_graph.rs`, and Milestone 5d/status plan notes.
  - No-fallback/no-legacy confirmation: scheduler projection now requires a
    current executable validation summary and rejects missing, stale, and
    non-executable validation state. It does not synthesize runtime/device/task
    intent from raw graph fields, frontend state, Tauri state, model paths, or
    legacy package metadata.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    inference_validation_state --lib`; `cargo test -p
    pantograph-workflow-service publish_inference_validation_session --lib`;
    `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and `git diff --check`.
  - Discovered issue: workflow-service still reports the pre-existing
    `set_active_run_execution_plan` dead-code warning during `cargo check`.
    This is outside the projection-state slice and remains part of the broader
    Milestone 5c/5d session-execution cleanup.
  - Remaining follow-up: queue admission/session orchestration must call the
    graph-session projection API before scheduler placement and replace direct
    task graph construction for submitted inference workflows. The known
    broader `session_execution --lib` failures remain the validation-to-run
    submission boundary and were not expanded into this projection-state slice.
- [x] 2026-05-26 saved-workflow validation-state address re-plan boundary:
  - Discovered issue: execution-session queue admission currently loads saved
    workflow graphs through `WorkflowHost::workflow_graph(workflow_id)` and
    `WorkflowExecutionSessionRunRequest` carries no graph-session id,
    validation-session id, validation snapshot id, or saved executable graph
    fingerprint. The new current-validation-state owner is keyed by
    graph-edit-session identity plus `WorkflowGraphRevision`, so queue
    admission cannot use it for saved workflow runs without a canonical address.
  - Discovered legacy test issue: broad `cargo test -p
    pantograph-workflow-service session_execution --lib` still contains tests
    that expect the retired whole-run `WorkflowHost::run_workflow` runtime path
    or pass request inputs to non-source nodes. Preserving those expectations
    would reintroduce a legacy execution path instead of routing inference work
    through scheduler-owned task state, descriptor-backed validation, and the
    runtime-host handoff.
  - Why implementation stops: wiring queue admission directly to
    `workflow_scheduler_task_graph_with_inference_projections` now would require
    either guessing validation state from saved graph fields, accepting frontend
    validation data in the run request, or preserving the old whole-run runtime
    execution behavior. All three violate the no-fallback/no-legacy rule and
    make scheduler placement depend on non-canonical authority.
  - Re-plan options to decide:
    1. Run descriptor validation synchronously from the saved workflow graph
       during queue admission, then build scheduler projections from that
       transient result. This is simple to address but can block admission on
       fact lookup and duplicates the graph-editor validation lifecycle unless
       carefully factored through the same publisher core.
    2. Extend `WorkflowExecutionSessionRunRequest` with graph-session and
       validation-session identity. This works for draft/edit-session runs, but
       it does not give saved workflow submissions a stable validation address
       and would couple queue admission to editor state.
    3. Persist a compact executable validation snapshot at executable publish
       time, keyed under the existing workflow version identity plus graph
       fingerprint and descriptor contract version. Queue admission reads that
       saved snapshot, rejects stale/missing/non-executable state, and only then
       builds scheduler projections. This aligns saved-run admission with the
       graph-editor validation UX and preserves historical authored port shape
       without storing paths or Pumas facts.
    4. Temporarily keep the legacy whole-run host execution tests/path until
       runtime-host handoff is complete. This is rejected because it preserves
       retired behavior and masks missing descriptor validation authority.
  - Recommendation: choose option 3 for saved workflow submissions and reserve
    option 1 only as the implementation detail inside the executable publish
    validator that produces the persisted snapshot. The next implementation
    slice should add the saved validation snapshot contract/owner and then
    rewrite or delete legacy `session_execution` tests so they assert fail-
    closed scheduler-owned behavior instead of whole-run fallback execution.
- [x] 2026-05-26 saved executable validation snapshot decision:
  - Decision: implement option 3. Saved workflow submissions must use a compact
    executable validation snapshot produced by the service-level executable
    publish boundary and consumed by execution-session queue admission. Queue
    admission must not depend on an open graph-edit session, frontend-provided
    validation state, raw graph inference fields, Pumas paths/facts, or the
    retired whole-run runtime execution path.
  - Snapshot identity: do not introduce a parallel executable-version identity.
    Persist executable validation snapshots under the existing attribution
    `WorkflowVersionId`, with workflow id, semantic version, canonical
    executable graph fingerprint, descriptor-contract version, and validation
    snapshot id recorded as typed snapshot fields. The snapshot must record the
    graph revision it validated and enough node-scoped descriptor projection
    records to build `WorkflowSchedulerInferenceTaskProjections`. Do not store
    runtime load paths, Pumas package facts, frontend presentation state, media
    payloads, or scheduler placement decisions in the snapshot.
  - Snapshot contents: store only bounded, typed admission authority:
    validation summary, validation session/snapshot id, descriptor fingerprint,
    resolved descriptor task kind, validated `PumasModelRef`, explicit
    runtime/device/trait constraints, descriptor defaults needed for later
    materialization, availability status, and blocking diagnostics. Optional
    authored-port shape remains in the saved graph as authoring history; the
    validation snapshot stores execution authority, not graph-editor display
    history. The saved executable snapshot must be derived from the backend
    validation publication/projection records, not from the lossy current-state
    scheduler projection cache alone; current-state records may be used as a
    live-edit cache but are not a persisted authority unless they first round
    trip through the compact snapshot DTO.
  - Save/publish rule: keep draft file save and executable publish as separate
    service-level operations. Draft save remains graph persistence and may
    preserve the editable graph/history without producing execution authority.
    Executable publish is the async workflow-service boundary that runs or
    reuses the validation publisher core, resolves the existing workflow version,
    and persists the executable validation snapshot. If descriptor facts are
    missing, stale, unavailable, unresolved, invalid, not implemented, or if the
    executable snapshot/version store is unavailable, the workflow may remain a
    draft but must not be marked executable or admitted to the queue.
  - Queue admission rule: before queue insertion, queue-placement diagnostic
    event recording, or scheduler task graph materialization, admission must run
    a pre-admission preparation step that loads the saved graph, resolves the
    submitted workflow version, loads the saved executable validation snapshot
    for that version/fingerprint, converts it to scheduler projections, and
    builds the scheduler task graph. Admission fails closed when the snapshot is
    missing, stale, non-executable, contract-incompatible, mismatched, or the
    required snapshot store is not configured. Only this snapshot may provide
    inference scheduler projections for saved workflow runs.
  - Legacy cleanup rule: tests and code that assert successful whole-run
    `WorkflowHost::run_workflow` execution for runtime/inference workflows must
    be rewritten to assert scheduler-owned/runtime-host handoff behavior or
    removed when the system they cover is retired. Request inputs must target
    source-input tasks; tests that pass values to non-source nodes as successful
    runtime inputs are legacy and must not be kept green through compatibility
    shims. The cleanup blast radius includes workflow-service tests plus
    embedded-runtime, HTTP, Rustler, UniFFI, and Tauri command contract tests
    that currently encode retired whole-run runtime behavior. Tauri remains a
    transport boundary and must not own validation, snapshot, or scheduler
    business logic.
  - Thin-slice order:
    1. Add the saved executable validation snapshot contract/owner keyed by
       existing `WorkflowVersionId`, plus focused tests for identity, bounded
       contents, snapshot-store-unavailable diagnostics, and fail-closed lookup.
    2. Add the service-level executable publish boundary while keeping draft
       `save_workflow` as graph persistence; create snapshots only from the
       backend validation publication/projection path and return typed publish
       status/diagnostics for the editor.
    3. Wire execution-session queue admission to consume snapshots before
       scheduler projection and to reject missing/stale/non-executable
       snapshots before queue insertion, queue-placement event recording, or
       scheduler task graph materialization.
    4. Rewrite/delete legacy `session_execution` and adapter contract tests
       around whole-run runtime fallback and non-source request inputs, then add
       scheduler-owned admission/runtime-host tests for the replacement
       behavior.
    5. Add pre-dispatch descriptor fingerprint revalidation against the saved
       snapshot before runtime-host dispatch selection.
- [x] 2026-05-26 saved executable validation snapshot standards iteration:
  - Standards reviewed:
    `PLAN-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
    `TESTING-STANDARDS.md`, `INTEROP-STANDARDS.md`,
    `FRONTEND-STANDARDS.md`, `CONCURRENCY-STANDARDS.md`,
    `languages/rust/RUST-API-STANDARDS.md`,
    `languages/rust/RUST-ASYNC-STANDARDS.md`,
    `languages/rust/RUST-INTEROP-STANDARDS.md`, and
    `DOCUMENTATION-STANDARDS.md`.
  - Executable-contract gate: the snapshot request/record/diagnostic DTOs must
    be executable boundary contracts with explicit serde casing/tagging,
    `deny_unknown_fields` where persisted compatibility permits it, typed
    workflow/version/fingerprint/status identifiers, `TryFrom`/validated
    constructors for raw boundary input, and non-stringly diagnostic enums.
    Internal APIs must accept validated snapshot/version types, not raw
    `String`, unvalidated JSON, or loosely typed maps.
  - Sync-core/async-shell gate: snapshot validation, compacting publication
    records, freshness checks, and projection into
    `WorkflowSchedulerInferenceTaskProjections` must be synchronous pure/domain
    functions. Async is allowed only at the executable publish shell and queue
    admission shell for fact-provider, persistence, or host I/O. The
    implementation must not hold graph/session/store locks across `.await`,
    must not run blocking filesystem/sqlite work directly inside async request
    paths unless it remains behind an existing synchronous boundary or is
    isolated with an explicit blocking/transaction owner, and must re-plan if a
    multi-step durable write cannot be made transactional or idempotent.
  - Persistence/concurrency gate: resolving `WorkflowVersionId`, writing the
    executable validation snapshot, and exposing executable status must be one
    coherent publish transaction or an idempotent durable state machine with
    typed recovery diagnostics. Queue admission must build an immutable
    pre-admission result before any enqueue, queue-placement event, or scheduler
    state mutation. If current APIs force observable queue side effects before
    snapshot validation, implementation must stop and re-plan instead of adding
    compensation fallback.
  - Interop/binding gate: any public wire shape for executable publish,
    snapshot lookup, queue-admission diagnostics, or editor submit gating must
    update Rust producers and TypeScript/Tauri, HTTP, UniFFI, and Rustler
    consumers in the same logical slice when they cross those boundaries.
    Serde fixture tests must prove Rust serialization/deserialization matches
    host-language expectations, including enum casing, omitted defaults,
    blocking diagnostics, and snapshot-store-unavailable errors.
  - Frontend ownership gate: the graph editor may display draft, pending,
    executable, stale, or blocked status and disable submit from backend
    validation/publish state, but it must not optimistically mark a workflow
    executable, synthesize scheduler projections, or persist execution
    authority. Validation refresh must remain event-driven or request-driven and
    must not add broad polling loops.
  - Testing gate: the first implementation slice must include a failing-first
    vertical acceptance test that exercises publish -> saved snapshot -> saved
    workflow queue admission -> scheduler task graph/projection or typed
    rejection. Unit tests must cover DTO validation, snapshot compaction,
    lookup mismatch, store-unavailable, and projection conversion. Integration
    tests must isolate sqlite/temp workflow roots per test or explicitly
    serialize the affected suite, and broad adapter tests must verify no retired
    whole-run runtime fallback remains.
  - Documentation gate: the slice that adds the snapshot owner must update the
    affected source README or add an ADR covering API consumer contract,
    structured producer contract, lifecycle/transaction rules, error semantics,
    compatibility/replay behavior for saved workflows, and revisit triggers.
- [x] 2026-05-26 saved executable validation snapshot contract/owner slice
  completed:
  - Smallest useful vertical slice: added the workflow-service executable
    validation snapshot DTOs, validated snapshot id, validated record wrapper,
    in-memory fail-closed snapshot store, lookup request, typed diagnostics,
    bounded content validation, and conversion into
    `WorkflowSchedulerInferenceTaskProjections`.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/workflow/executable_validation_snapshot.rs`,
    `crates/pantograph-workflow-service/src/workflow.rs`,
    `crates/pantograph-workflow-service/src/lib.rs`,
    `crates/pantograph-workflow-service/src/README.md`, this milestone file,
    and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the snapshot contract stores only
    typed executable authority keyed by `WorkflowVersionId`; it does not store
    local model paths, Pumas package facts, frontend presentation state, media
    payloads, Tauri state, queue placement, scheduler policy decisions, or
    legacy whole-run runtime inputs. Lookup fails closed when the store is
    unavailable, the snapshot is missing, the descriptor contract version is
    incompatible, or the workflow fingerprint mismatches.
  - Focused tests added for validating/projecting an executable snapshot,
    rejecting non-executable summaries, rejecting oversized node contents,
    store-unavailable lookup, missing-snapshot lookup, and fingerprint mismatch.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo fmt
    -p pantograph-workflow-service -- --check`; `cargo test -p
    pantograph-workflow-service executable_validation_snapshot --lib`; `cargo
    check -p pantograph-workflow-service`; and `git diff --check`.
  - Verification note: `cargo check -p pantograph-workflow-service` still emits
    the pre-existing `set_active_run_execution_plan` dead-code warning outside
    this slice.
  - Deviation recorded: the broader publish -> saved snapshot -> queue
    admission acceptance path is not executable until the next two planned
    slices add executable publish and queue-admission consumption. This slice
    covers the replacement contract/owner and fail-closed lookup core only; it
    does not add a compatibility shim or runnable alternate path.
  - Remaining follow-up: wire the service-level executable publish boundary to
    produce this snapshot, then require queue admission to consume the saved
    snapshot before queue insertion or scheduler task graph materialization.
- [x] 2026-05-26 validation-publication snapshot compaction slice completed:
  - Smallest useful vertical slice: added the synchronous pure-domain compactor
    from `WorkflowGraphInferenceValidationPublication` plus
    `WorkflowVersionRecord` into `WorkflowExecutableValidationSnapshotRecord`.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/workflow/executable_validation_snapshot.rs`,
    this milestone file, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: compaction consumes only backend
    validation-publication records and attribution-owned workflow version
    identity. It does not inspect graph JSON, frontend state, Tauri payloads,
    raw model paths, Pumas package facts, or scheduler placement state.
  - Focused test added for compaction preserving workflow version id,
    execution fingerprint, graph revision, validation summary, descriptor
    fingerprint, model ref, task kind, and explicit runtime/device constraints
    while leaving trait settings and estimate hints empty until a typed
    publisher source owns them.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo fmt
    -p pantograph-workflow-service -- --check`; `cargo test -p
    pantograph-workflow-service executable_validation_snapshot --lib`; `cargo
    check -p pantograph-workflow-service`; and `git diff --check`.
  - Verification note: `cargo check -p pantograph-workflow-service` still emits
    the pre-existing `set_active_run_execution_plan` dead-code warning outside
    this slice.
  - Remaining follow-up: persist the compacted snapshot behind a service-level
    executable publish operation. Durable storage ownership remains the next
    boundary to resolve before queue admission can consume saved snapshots.
- [x] 2026-05-26 executable validation snapshot persistence re-plan boundary:
  - Discovered issue: the service can now build a typed executable validation
    snapshot, but no durable store currently owns saved executable-validation
    snapshots keyed by `WorkflowVersionId`. `pantograph-runtime-attribution`
    owns workflow-version records and sqlite schema, while
    `pantograph-workflow-service` owns the typed snapshot contract and scheduler
    projection conversion. Adding publish persistence without deciding this
    boundary would either create a second workflow-version persistence source or
    force a dependency cycle from attribution into workflow-service contracts.
  - Required design decision before the next implementation slice:
    1. Add a workflow-service-local snapshot persistence subsystem keyed by
       `WorkflowVersionId`. This is fastest but weakens the plan's attribution
       boundary and makes replay/recovery reason across two durable stores.
    2. Extend `pantograph-runtime-attribution` with an opaque executable
       validation snapshot table keyed by `WorkflowVersionId`, storing compact
       JSON plus typed metadata such as schema version, descriptor-contract
       version, execution fingerprint, graph revision, validation session id,
       and validation snapshot id. Workflow-service remains the typed DTO owner
       and serializes/deserializes at the boundary. This keeps workflow-version
       durability in attribution without a crate dependency cycle.
    3. Move the executable validation snapshot DTOs into a lower shared
       contract crate so attribution can store typed records directly. This is
       clean long-term if multiple crates need to construct the snapshot, but
       it widens the immediate blast radius and risks moving scheduler-facing
       projection concerns out of workflow-service.
  - Decision for the next slice: use option 2. Attribution owns durable opaque
    snapshot storage and idempotent lookup by `WorkflowVersionId`;
    workflow-service owns typed snapshot construction, validation, serde, and
    projection. Do not use a workflow-service sidecar store as a fallback, and
    do not make attribution depend on workflow-service.
  - Storage contract for implementation: add attribution repository request and
    record DTOs that carry `workflow_version_id`, `workflow_id`,
    `workflow_execution_fingerprint`, snapshot schema version,
    descriptor-contract version, graph revision, validation session id,
    validation snapshot id, compact snapshot JSON, and creation timestamp.
    Attribution validates identity/fingerprint consistency against the existing
    workflow version row and stores the compact JSON opaquely; workflow-service
    remains responsible for deserializing, validating, and projecting the
    snapshot before scheduler use.
  - Idempotency/transaction rule: storing the same workflow version and
    identical snapshot metadata/JSON is a successful reuse. A different
    snapshot for the same `WorkflowVersionId` is a typed conflict unless a
    later migration explicitly introduces supersession. The write must happen
    in the same attribution transaction as the workflow-version lookup/check so
    publish cannot expose a version without coherent executable snapshot
    authority.
  - Lookup rule: lookup by `WorkflowVersionId` returns either the opaque
    snapshot record or a typed not-found/unavailable/metadata-mismatch error.
    Workflow-service converts those errors into publish/admission diagnostics;
    it must not reconstruct a snapshot from graph fields, current validation
    cache, frontend state, or runtime defaults when lookup fails.
  - No-fallback/no-legacy gate: queue admission must remain blocked until the
    durable snapshot store exists and can fail closed for missing,
    stale/mismatched, contract-incompatible, or store-unavailable snapshots.
- [x] 2026-05-26 attribution executable snapshot storage slice completed:
  - Smallest useful vertical slice: added attribution-owned opaque executable
    validation snapshot storage keyed by `WorkflowVersionId`, including public
    request/record DTOs, repository methods, sqlite schema version 8,
    idempotent insert/reuse, lookup by workflow version, and typed conflict or
    not-found errors.
  - Allowed files touched:
    `crates/pantograph-runtime-attribution/src/error.rs`,
    `crates/pantograph-runtime-attribution/src/lib.rs`,
    `crates/pantograph-runtime-attribution/src/records.rs`,
    `crates/pantograph-runtime-attribution/src/repository.rs`,
    `crates/pantograph-runtime-attribution/src/schema.rs`,
    `crates/pantograph-runtime-attribution/src/sqlite.rs`,
    `crates/pantograph-runtime-attribution/src/tests.rs`,
    `crates/pantograph-runtime-attribution/src/README.md`, this milestone file,
    and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: attribution stores compact snapshot
    JSON opaquely and validates only workflow-version identity, execution
    fingerprint, bounded metadata, and JSON shape. It does not parse
    workflow-service snapshot internals, depend on workflow-service, reconstruct
    snapshots from graph fields, read frontend/Tauri/runtime state, or provide a
    sidecar fallback store.
  - Focused tests added for idempotent identical snapshot reuse, successful
    lookup, conflicting snapshot rejection, workflow-version fingerprint
    mismatch rejection, and fail-closed missing snapshot lookup.
  - Verification passed: `cargo fmt -p pantograph-runtime-attribution -- --check`;
    `cargo test -p pantograph-runtime-attribution executable_validation_snapshot`;
    `cargo test -p pantograph-runtime-attribution`; `cargo check -p
    pantograph-runtime-attribution`; and `git diff --check`.
  - Remaining follow-up: wire workflow-service executable publish to serialize
    its typed compact snapshot into this attribution boundary, then wire queue
    admission to require attribution snapshot lookup before enqueue or scheduler
    graph materialization.
- [x] 2026-05-26 workflow-service executable snapshot attribution bridge slice
  completed:
  - Smallest useful vertical slice: added workflow-service conversion and
    facade methods that serialize a validated typed executable snapshot into
    attribution-owned opaque storage and read it back through attribution
    lookup with workflow-version, fingerprint, descriptor-contract, graph
    revision, validation session, and validation snapshot metadata checks.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/workflow/executable_validation_snapshot.rs`,
    `crates/pantograph-workflow-service/src/workflow/attribution_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/workflow_version.rs`,
    `crates/pantograph-workflow-service/src/README.md`, this milestone file,
    and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: workflow-service does not use the
    in-memory snapshot helper as a production fallback, does not reconstruct
    saved executable authority from graph fields, current validation cache,
    frontend state, Tauri payloads, runtime defaults, Pumas paths/facts, or
    scheduler placement, and rejects missing or stale attribution snapshots
    before scheduler projection.
  - Focused tests cover typed snapshot round-trip through the real ephemeral
    attribution store, missing attribution snapshot fail-closed behavior, and
    stale execution-fingerprint rejection.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service
    workflow_executable_validation_snapshot --lib`; `cargo test -p
    pantograph-workflow-service executable_validation_snapshot --lib`; and
    `cargo check -p pantograph-workflow-service`.
  - Verification note: `cargo check -p pantograph-workflow-service` still emits
    the pre-existing `set_active_run_execution_plan` dead-code warning outside
    this slice.
  - Remaining follow-up: executable publish still needs to call this bridge at
    the publish boundary, and queue admission still needs to require successful
    attribution snapshot lookup before queue insertion or scheduler graph
    materialization.
- [x] 2026-05-26 workflow-service executable publish snapshot slice completed:
  - Smallest useful vertical slice: added a typed executable publish request to
    workflow-service that validates the backend validation publication graph
    revision against the graph being published, resolves the immutable
    `WorkflowVersionId`, compacts the publication into a typed executable
    validation snapshot, and persists it through attribution-owned opaque
    storage.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/workflow/executable_validation_snapshot.rs`,
    `crates/pantograph-workflow-service/src/workflow/attribution_api.rs`,
    `crates/pantograph-workflow-service/src/workflow.rs`,
    `crates/pantograph-workflow-service/src/lib.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/workflow_version.rs`,
    `crates/pantograph-workflow-service/src/README.md`, this milestone file,
    and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: executable publish is separate from
    draft graph save and refuses stale validation publications. It does not
    derive executable authority from graph fields, frontend state, Tauri
    payloads, current validation caches, runtime defaults, Pumas paths/facts, or
    scheduler placement, and it persists through the attribution bridge only.
  - Focused tests cover successful publish-to-attribution persistence and stale
    validation-publication graph revision rejection.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service
    workflow_executable_validation_snapshot --lib`; `cargo test -p
    pantograph-workflow-service executable_validation_snapshot --lib`; and
    `cargo check -p pantograph-workflow-service`.
  - Verification note: `cargo check -p pantograph-workflow-service` still emits
    the pre-existing `set_active_run_execution_plan` dead-code warning outside
    this slice.
  - Remaining follow-up: queue admission must require successful attribution
    snapshot lookup before queue insertion, queue-placement event recording, or
    scheduler task graph materialization.
- [x] 2026-05-26 queue-admission executable snapshot consumption slice
  completed:
  - Smallest useful vertical slice: moved scheduler task graph preparation
    ahead of queue insertion and queue-placement event recording, and changed
    saved-run scheduler graph preparation to consume the saved executable
    validation snapshot when attribution has a run snapshot. Runtime inference
    graphs without saved snapshot authority now fail closed before enqueue.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
    `crates/pantograph-workflow-service/src/README.md`, this milestone file,
    and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: runtime inference queue admission no
    longer materializes scheduler intent from raw graph fields, whole-run host
    launch, current validation caches, frontend/Tauri payloads, runtime
    defaults, Pumas paths/facts, or queue-side mutation after enqueue. The
    saved executable snapshot is the only source for inference scheduler
    projections during saved-run admission.
  - Focused test coverage updated for runtime inference admission failing
    closed before legacy whole-run runtime launch when no saved executable
    snapshot exists.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service
    workflow_execution_session_runtime_run_fails_closed_before_legacy_launch
    --lib`; `cargo test -p pantograph-workflow-service
    executable_validation_snapshot --lib`; and `cargo check -p
    pantograph-workflow-service`.
  - Verification note: `cargo check -p pantograph-workflow-service` still emits
    the pre-existing `set_active_run_execution_plan` dead-code warning outside
    this slice.
  - Remaining follow-up: add an end-to-end publish -> saved snapshot -> queue
    admission -> scheduler task graph acceptance test once the test host can
    publish the same validation publication used by saved-run admission without
    duplicating large inference descriptor fixtures.
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
