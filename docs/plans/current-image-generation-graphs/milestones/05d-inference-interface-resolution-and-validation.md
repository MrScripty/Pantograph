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
- [ ] Add the production inference facts provider and conservative resource
      estimator before completing the successful production inference session
      path. The provider must be injected through backend composition, consume
      Pumas static artifact facts plus runtime/device/task-shape and proven
      residency facts, and populate `InferenceInterfaceResolverFacts` with
      conservative `SchedulerEstimateHint` values or typed insufficient-facts
      diagnostics. Pumas supplies logical package evidence only; Pantograph
      owns loaded-memory/context estimation, scheduler admission, and later
      diagnostics-ledger refinement. Do not synthesize estimates from graph
      paths, selector summaries, UI state, `ModelRefV2`,
      `ModelDependencyRequest`, or Tauri-owned policy.
      2026-06-02 gate: this task must wait for the Pumas package artifact size
      facts handoff plan to be implemented and pinned:
      `../pumas-package-artifact-size-facts-handoff.md`. After that Pumas
      contract is available, the next Pantograph slice is to update the Pumas
      dependency and inference package-facts DTOs, project source-tagged logical
      artifact/file/component sizes through embedded-runtime, reject stale or
      missing size facts with typed diagnostics, and only then implement the
      conservative estimator/provider. Until that dependency exists, the valid
      behavior remains fail-closed `missing_resource_estimates`; zero-resource
      reservations or graph-path-derived estimates are forbidden.
      - 2026-06-02 Pumas v3 contract/projection slice completed: Pantograph
        pins the pushed untagged Pumas commit
        `f87c3da8276a914a54c6f4f36d617bef9d9f424e`, mirrors package-facts
        contract v3 logical artifact/file size DTOs in `inference`, projects
        logical-size facts through `PumasDispatchPackageFactsSource`, rejects
        missing logical-size facts with typed diagnostics in dispatch and
        runtime-host package-facts bridges, and keeps estimator/admission
        policy deferred to the next backend-owned provider slice.
      - Remaining work in this task: implement the production
        `InferenceInterfaceFactsProvider` and pure conservative estimator from
        Pumas logical-size evidence plus runtime/device/task-shape/proven
        residency facts, preserving typed insufficient-facts diagnostics.
      - 2026-06-02 conservative estimator primitive slice completed:
        `pantograph-embedded-runtime` now owns
        `inference_resource_estimator.rs`, a pure checked-arithmetic transform
        from Pumas logical package size facts into `InferenceResourceEstimate`
        values and bounded `SchedulerEstimateHint` RAM/VRAM hints. The slice
        only touched embedded-runtime module wiring/docs and this plan, kept
        production behavior fail-closed until provider composition, and did not
        derive estimates from graph paths, selector summaries, UI state,
        `ModelRefV2`, `ModelDependencyRequest`, or Tauri policy. Focused
        verification: `cargo fmt`; `cargo test -p pantograph-embedded-runtime
        inference_resource_estimator`; `cargo check -p
        pantograph-embedded-runtime`; `git diff --check`. Remaining follow-up:
        remove the staged module's narrow `#[allow(dead_code)]` when the
        production `InferenceInterfaceFactsProvider` consumes the estimator and
        combines it with runtime/device/task-shape/proven-residency facts.
      - 2026-06-02 production provider wiring slice completed:
        `pantograph-embedded-runtime` now owns
        `inference_interface_facts_provider.rs` and resource-backed hosted
        workflow-service composition installs it before wrapping the service in
        `Arc`. `pantograph-workflow-service` gained a graph-session provider
        composition API so inference-interface and dependency-environment
        providers can coexist without one overwriting the other. The provider
        projects canonical task ports from inference task evidence, filters
        runtime availability through runtime-registry backend keys matched to
        Pumas backend hints, and publishes conservative scheduler estimate
        hints from logical-size facts. No-fallback confirmation: missing
        package, task, runtime, or estimate facts publish only proven resolver
        facts and rely on canonical resolver/admission diagnostics; no graph
        paths, selector summaries, UI state, `ModelRefV2`,
        `ModelDependencyRequest`, or Tauri policy are consulted. Focused
        verification: `cargo fmt`; `cargo test -p pantograph-embedded-runtime
        inference_interface_facts_provider`; `cargo test -p
        pantograph-embedded-runtime inference_resource_estimator`; `cargo
        check -p pantograph-embedded-runtime`; `cargo check -p
        pantograph-workflow-service`; `git diff --check`. Remaining follow-up:
        feed richer device/task-shape/proven-residency facts into the provider
        and add end-to-end graph validation coverage that proves resolver facts
        reach enqueue/admission without blocking graph editing.
      - 2026-06-02 workflow-service provider composition coverage completed:
        `pantograph-workflow-service` now has a service-boundary graph-session
        test proving `WorkflowService::with_graph_session_fact_providers`
        preserves the inference-interface facts provider for current validation
        and the dependency-environment provider for action-intent resolution in
        the same edit session. The test keeps Tauri/frontend out of dependency
        policy and does not restore unavailable-provider fallback behavior.
        Focused verification: `cargo fmt`; `cargo test -p
        pantograph-workflow-service
        workflow_service_graph_session_fact_providers_preserve_inference_and_dependency_providers`;
        `cargo check -p pantograph-workflow-service`.
      - 2026-06-02 resource-backed hosted validation coverage completed:
        `pantograph-embedded-runtime` now has a composition test proving owner
        Pumas package facts and runtime-registry capability facts reach
        workflow-service current validation through the resource-backed hosted
        service. The test verifies executable descriptor projection, task
        ports, selected runtime constraint, and nonzero scheduler RAM/VRAM
        estimate hints from Pumas logical sizes. It does not add runtime
        execution, queue admission, graph-path-derived estimates, or Tauri/
        frontend policy. Focused verification: `cargo fmt`; `cargo test -p
        pantograph-embedded-runtime
        resource_backed_hosted_service_refreshes_validation_from_production_facts`;
        `cargo check -p pantograph-embedded-runtime`.
      - 2026-06-02 resource-backed executable snapshot publication coverage
        completed: `pantograph-embedded-runtime` now has a composition test
        proving resource-backed validation can resolve the dependency
        environment sidecar proof through the workflow-service API, publish
        the compact graph-session executable validation snapshot, and project
        queue-facing scheduler inference task records from that saved
        snapshot. This still stops before production execution-session
        dispatch.
      - 2026-06-03 dependency-readiness lifecycle re-plan selected: the
        earlier synchronous-provider first-run option is superseded. The
        implemented deferral path queues backend readiness work, records
        scheduler deferred state, and keeps the active run open when no fresh
        proof exists. The next slice must add an explicit backend-owned resume
        command for that active `session_id` plus `workflow_run_id`.
        Workflow-service must validate the run is active and
        readiness-pending, preserve scheduler task state as the backend source
        of truth, retry readiness admission only from canonical backend facts,
        and continue toward dispatch or expose typed pending/fail-closed
        diagnostics that Tauri/frontend can only display or forward through
        backend commands. This task still requires production facts to fail
        closed when missing, stale, or insufficient, and still forbids graph
        paths, selector summaries, UI state, Tauri policy, `ModelRefV2`,
        `ModelDependencyRequest`, reduced execution plans, synchronous probes,
        client reruns, or caller-supplied proofs. After the first complete
        production inference path is proven through that explicit backend
        resume path, add the composition-root-owned backend readiness
        worker/listener lifecycle with tracked tasks, cancellation/shutdown,
        freshness, timeout, retry, reservation-release, overlap-prevention,
        and observability behavior.
      - 2026-06-03 auto-resume lifecycle re-plan resolution: the next slice
        implements that worker/listener as an embedded-runtime-owned lifecycle
        handle. It reuses workflow-service scheduler state and resume
        candidates, the existing backend resume API, and the embedded backend
        host. Tauri/frontend stay display/forward-only and may not own the
        loop. A typed snapshot notification channel remains a later
        event-first improvement after the bounded polling lifecycle lands.
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
      Production inference facts provider resolution: the fail-closed
      `UnavailableInferenceInterfaceFactsProvider` is a guardrail, not the
      production source. The production source must be a backend-owned provider
      injected through composition that consumes Pumas static artifact facts,
      runtime/device/task-shape facts, and proven residency facts to populate
      `InferenceInterfaceResolverFacts.estimate_hints`. Pumas supplies logical
      file/component sizes and package evidence; Pantograph owns the
      conservative loaded-memory/context estimator, scheduler resource
      admission, and later diagnostics-ledger refinement. Missing or
      insufficient estimate inputs must produce typed diagnostics and leave
      runtime task projection blocked by `missing_resource_estimates`; do not
      synthesize estimates from graph paths, UI state, selector summaries,
      `ModelRefV2`, `ModelDependencyRequest`, or Tauri-owned policy.
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
      Implementation update 2026-05-28: the active frontend helper now detects
      only the typed `dependency_environment_sidecar` visual association and
      user-authored manual override inputs. It no longer projects upstream
      `modelPath`/`model_path`, model ids, task/backend keys, platform context,
      or dependency requirements into dependency action state, and the old
      upstream-requirements adoption helper/test was removed.
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
      Implementation update 2026-05-31: session-level scheduler runtime tests
      now use a persisted authored `inference_interface_snapshot` for the
      generic `llm-inference` node and connect `text-input.text` to the
      descriptor-authored `prompt` input. The dispatch test asserts the
      resulting scheduler/runtime-host request contains the materialized prompt
      input. This proves the current validation/materialization path can use
      persisted authored task ports without reintroducing static
      `llm-inference` prompt ports or `node.data.definition` fallback. The
      remaining production work is to generate and persist this authored
      snapshot from current validation state for saved workflows rather than
      hand-authoring it in tests.
- [ ] Replace `ModelDependencyRequest`/`ModelRefV2.model_path` dependency
      preflight with scheduler-owned readiness and runtime handoff. Re-plan
      decision on 2026-05-26: use option 3. Dependency readiness, runtime/device
      selection influence, selected environment identity, executable model-load
      target resolution, and runtime-host launch facts belong to the
      workflow-service/scheduler/runtime-host boundary, not to node-engine,
      embedded-runtime task execution, Tauri, frontend graph data, or Puma-Lib
      node metadata.
      Scope clarification: this is not a compatibility adapter around
      `ModelDependencyRequest`, `ModelRefV2`, `model_path`, `environment_ref`,
      or graph-authored package facts. It is a replacement path. Each legacy
      producer/consumer must either move to the canonical scheduler-owned
      handoff or be deleted when no active runtime path needs it.
      Required contract shape:
      1. Workflow-service validation/admission derives a path-free dependency
         readiness request from `pumas_model_ref`, descriptor facts, scheduler
         intent, selected binding ids, manual override patches, and current
         graph revision/session identity.
      2. Scheduler admission consumes a typed readiness proof and the current
         validation summary before it can mark an inference task schedulable.
         The proof must include stable dependency identity, action/result state,
         diagnostics, and selected environment identity, but no local paths or
         graph-authored environment refs.
      3. Runtime-host handoff consumes only scheduler-selected runtime/device,
         readiness proof identity, materialized upstream inputs, and
         backend-owned executable load-target facts. Load targets are resolved
         by the backend/runtime host from Pumas-approved references, never by
         node-engine or graph editor data.
      4. Node-engine continues to execute only non-inference tasks on this path.
         Inference nodes become scheduler tasks and must not call dependency
         preflight, `ModelDependencyResolver`, `resolve_model_ref`, or
         `build_model_ref_v2`.
      5. Tauri remains transport/composition only. It may manage concrete
         services until the active runtime path no longer needs them, but must
         not expose direct dependency commands, synthesize paths, or own
         dependency readiness policy.
      Staged implementation order:
      1. Add or reuse scheduler-owned DTOs for dependency readiness proof and
         runtime-host handoff inputs, with focused tests that reject
         `ModelDependencyRequest`, `ModelRefV2`, `model_path`, `environment_ref`,
         and local load-target fields at the boundary.
      2. Wire workflow-service queue/admission to require the backend
         validation summary plus dependency readiness proof before creating
         schedulable inference tasks. Keep graph editing and validation display
         separate from queue admission.
      3. Wire runtime-host dispatch to consume scheduler handoff facts directly
         and resolve executable load targets through the backend/Pumas-approved
         provider path.
      4. Replace embedded-runtime and node-engine Python-backed dependency
         preflight call sites with fail-closed diagnostics that point to the
         scheduler handoff when a runtime task reaches them without proof.
      5. Delete `ModelDependencyRequest`, `ModelDependencyResolver`,
         `ModelRefV2`, `build_model_ref_v2`, path-derived model-id repair, and
         tests/fixtures for successful path-shaped preflight once all active
         runtime execution paths use the scheduler handoff.
      Verification gate: every slice must run focused scheduler/workflow-service
      tests for proof freshness and stale graph revision rejection, focused
      runtime-host tests for path-free handoff consumption, and source-search
      checks proving no successful path converts the scheduler handoff back into
      `ModelDependencyRequest`, `ModelRefV2`, graph `modelPath`, or
      `model_path`.
      Standards compliance tightening:
      1. Contract placement and validation must follow executable-contract
         standards. Shared readiness proof and runtime handoff DTOs must live in
         an existing dedicated contract crate or a narrowly owned new contract
         crate, not inside Tauri, embedded-runtime implementation modules, or
         frontend code. Raw IPC/graph/persisted input must parse once into
         validated newtypes/enums with private unchecked constructors,
         `TryFrom`/`FromStr` boundary parsing, and specific error enums.
         Public library APIs must not expose `Result<T, String>`, raw
         `serde_json::Value`, path-shaped primitives, or unbranded `String`
         identifiers when those values cross crate or process boundaries.
      2. Scheduler/readiness policy must be a synchronous core with async
         shells only around I/O, runtime dispatch, Pumas/load-target lookup, or
         durable store operations. No implementation slice may create a global
         Tokio runtime, spawn detached tasks, hold graph/session/store locks
         across `.await`, or hide background polling/retry loops outside a
         named lifecycle owner with cancellation, tracked task handles,
         shutdown behavior, tracing, and idempotency rules.
      3. Composition and dependency ownership must stay narrow. Concrete Pumas
         load-target providers, runtime-host dispatchers, resource monitors,
         and test fakes are selected at composition roots or explicit lifecycle
         owners. Feature/domain crates depend on contracts/facades only. Any
         new dependency requires cargo-tree inspection, narrow owner
         declaration, lockfile update when applicable, and a note in the slice
         log explaining why `std` or existing workspace crates were
         insufficient.
      4. Multi-user/concurrent workflow safety is required in the contract, not
         left to callers. Readiness proofs and runtime handoffs must carry
         enough validated identity to reject stale or cross-session use:
         workflow/run/session id, graph revision, validation session id when
         applicable, scheduler task id, readiness proof id/version, selected
         runtime/device ids, and correlation id. Queue admission and runtime
         dispatch must be idempotent for retries and must reject proofs whose
         validation summary or dependency result no longer matches the current
         admitted task.
      5. Verification must include the thinnest cross-layer acceptance path
         before deleting legacy runtime success paths: graph/session validation
         produces a readiness proof, scheduler admission consumes it, runtime
         handoff is built without paths, and a fake runtime-host receives only
         scheduler-owned handoff facts. Unit tests alone, typecheck alone, or
         isolated DTO tests are insufficient for the first production slice
         crossing workflow-service, scheduler, and runtime-host ownership.
         Tests that touch durable stores, temp roots, global caches, or runtime
         registries must isolate those resources per test or serialize with a
         documented guard.
      6. Documentation/deletion gates are part of each source slice. Every
         touched `src/` directory must update its README or a linked ADR with
         purpose, invariants, API consumer contract, lifecycle/error semantics,
         rejected alternatives, and revisit triggers. Slices that replace a
         legacy path must delete or fail-close the superseded successful path in
         the same commit; do not leave deprecated branches, compatibility
         shims, or successful path-shaped fixtures for later cleanup unless the
         plan records an explicit unresolved re-plan boundary.
- [x] 2026-05-26 scheduler task-state readiness gate slice completed:
  - Smallest useful vertical slice: reuse the existing scheduler-owned
    readiness proof/runtime handoff contracts and remove the premature `Ready`
    initial state for runtime inference tasks that have a
    `SchedulableTaskIntent` but no admitted dependency readiness proof.
  - Allowed files touched:
    `crates/pantograph-scheduler/src/queue.rs`,
    `crates/pantograph-scheduler/src/README.md`,
    `crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs`,
    `crates/pantograph-workflow-service/src/scheduler/task_orchestrator_tests.rs`,
    `crates/pantograph-workflow-service/src/scheduler/README.md`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: no adapter back to
    `ModelDependencyRequest`, `ModelRefV2`, graph `modelPath`, `model_path`, or
    local load targets was introduced. Runtime inference with complete graph
    inputs now initializes as `WaitingDependencyReadiness`; connected-input
    runtime tasks still initialize as `AwaitingInputs`.
  - Verification passed: `cargo fmt`; `cargo test -p pantograph-scheduler`;
    `cargo test -p pantograph-workflow-service orchestrator_initializes --
    --nocapture`; `git diff --check`; and targeted source search of the touched
    scheduler/workflow-service files for retired path-shaped terms.
  - Remaining follow-up: add the production proof-producing workflow-service
    admission path that consumes backend validation summary plus dependency
    readiness proof and transitions runtime inference from
    `WaitingDependencyReadiness` to dispatch selection.
- [x] 2026-05-26 workflow-service runtime readiness admission bridge slice
      completed:
  - Smallest useful vertical slice: add the workflow-service scheduler
    orchestrator bridge that consumes a path-free `DependencyPreflightResult`,
    invokes `pantograph-scheduler` readiness admission policy, and persists the
    validated active-run task-state transition for runtime inference tasks that
    are waiting on dependency readiness.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/scheduler/task_orchestrator.rs`,
    `crates/pantograph-workflow-service/src/scheduler/task_orchestrator_tests.rs`,
    `crates/pantograph-workflow-service/src/scheduler/README.md`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the bridge accepts only
    scheduler/dependency-planning contracts. It does not call
    `ModelDependencyRequest`, build `ModelRefV2`, read graph `modelPath` or
    `model_path`, synthesize local load targets, or adapt the new proof path
    back into legacy dependency preflight.
  - Focused tests added: ready dependency proof transitions runtime inference
    from `WaitingDependencyReadiness` to `Ready`; missing proof defers with
    typed diagnostics; non-ready proof defers without a legacy bridge.
  - Verification passed: `cargo fmt`; `cargo test -p
    pantograph-workflow-service orchestrator_ -- --nocapture`; `git diff
    --check`; and targeted source search of touched workflow-service scheduler
    files for retired path-shaped terms.
  - Remaining follow-up: wire a production caller that derives the
    `DependencyPreflightResult` from backend validation/dependency readiness
    ownership, then build dispatch candidates from the ready task state and
    scheduler facts without introducing load-target paths into workflow-service
    graph data.
- [x] 2026-05-26 production readiness-proof caller re-plan boundary recorded:
  - Discovered issue: workflow-service now has a path-free bridge for consuming
    `DependencyPreflightResult`, but the active production run path does not yet
    have a canonical owner that produces that proof for each admitted runtime
    inference task. Current `WorkflowTechnicalFitDependencyReadinessFact`
    values and reduced execution-plan dependency-readiness projections are
    diagnostic/inspection facts; they do not carry the full
    `DependencyPlanningIdentityKey`, `dependency_requirements_id`,
    `DependencyEnvironmentRef`, graph/session/revision freshness, and
    scheduler task correlation required by scheduler readiness admission.
  - Why this blocks the next implementation slice: deriving a
    `DependencyPreflightResult` from reduced technical-fit facts or reduced
    `WorkflowExecutionPlanNodeDecision` would create a second authority path and
    violate the no-fallback/no-legacy rule. The proof must come from a
    backend-owned dependency-readiness provider/session boundary that consumes
    typed validation summary, Pumas model ref, selected binding ids, scheduler
    intent, and run identity.
  - Required re-plan: define the production proof producer and freshness
    contract before wiring runtime session execution. The design must state
    whether the proof is produced during executable validation publication,
    queue admission, or a scheduler-owned readiness lifecycle step, and how
    stale graph revisions, validation sessions, dependency actions, retries,
    and concurrent workflow runs are rejected or correlated.
  - No-fallback/no-legacy confirmation: do not convert
    `WorkflowTechnicalFitDependencyReadinessFact`, reduced execution-plan
    dependency readiness, graph node data, or Tauri/frontend payloads into a
    fake `DependencyPreflightResult`.
- [x] 2026-05-26 production readiness-proof caller design selected:
  - Decision: use option 3 as the authoritative execution design: a
    scheduler-owned dependency-readiness lifecycle after queue admission. Queue
    admission creates runtime inference tasks in `WaitingDependencyReadiness`
    and records the task correlation needed by the lifecycle; it does not treat
    editor validation, executable validation publication, technical-fit facts,
    or reduced execution-plan facts as execution proof.
  - Backend ownership: workflow-service owns the lifecycle that builds typed
    readiness requests, invokes the dependency-readiness provider, and applies
    the resulting proof to scheduler task state. Tauri, frontend graph editor,
    node-engine execution, runtime adapters, and runtime hosts must not own this
    proof-producing path.
  - Required request shape: introduce or extend a path-free
    `DependencyReadinessRequest` built from `PumasModelRef`, task kind or
    `DependencyTaskId`, selected binding ids, scheduler intent
    (runtime/device hard constraints, trait settings, and override patches),
    current validation snapshot/session identity, graph revision, workflow
    run/session/task ids, scheduler task id, correlation id, and environment
    identity. Do not include graph-authored local paths or load targets.
  - Required response handling: the lifecycle consumes
    `DependencyPreflightResult` and calls the existing
    `apply_runtime_dependency_readiness_admission` bridge. Ready proof moves the
    task to `Ready` and makes it eligible for dispatch selection. Deferred,
    retryable-failed, and terminal-failed proof states remain typed scheduler
    task states with diagnostics; they must not be converted into fallback
    runtime attempts.
  - Freshness and idempotency: reject stale proof when graph revision,
    validation session/snapshot, workflow run id, scheduler task id, model ref,
    task kind, selected bindings, scheduler intent, dependency requirements,
    dependency environment, or dependency-planning identity do not match the
    admitted task. The lifecycle must be idempotent for retries and restarts,
    and concurrent users/runs are isolated by run/session/task identity.
  - Projection separation: `WorkflowTechnicalFitDependencyReadinessFact` and
    `WorkflowExecutionPlanNodeDecision.dependency_readiness` remain
    diagnostic/inspection/UX preview projections only. They may explain why a
    graph is likely valid or invalid, but they cannot be synthesized into
    `DependencyPreflightResult` or used to bypass the readiness lifecycle.
  - Implementation sequence: first add the workflow-service readiness lifecycle
    contract and focused tests with stubbed dependency-readiness provider output;
    next build the request from admitted scheduler task graph plus validation
    summary identity; then persist or correlate the proof with active task
    state; then use ready scheduler tasks as the only source for runtime-host
    dispatch candidates.
  - Later objective: option 4 remains the graph-editor UX target for live
    preview/readiness display, but it is additive. Preview validation can run
    before queue admission without blocking editing; authoritative execution
    readiness proof is still produced only by this scheduler-owned lifecycle.
- [x] 2026-05-26 production readiness lifecycle standards iteration:
  - Standards reviewed:
    `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`,
    `CODING-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
    `CONCURRENCY-STANDARDS.md`, `TESTING-STANDARDS.md`,
    `DOCUMENTATION-STANDARDS.md`,
    `languages/rust/RUST-API-STANDARDS.md`, and
    `languages/rust/RUST-ASYNC-STANDARDS.md`.
  - Lifecycle tightening: the first implementation slice must introduce a named
    workflow-service readiness lifecycle owner with explicit start/stop,
    cancellation, retry, restart, and cleanup behavior. If the lifecycle
    spawns work, it must track task handles, propagate cancellation, await or
    abort on shutdown, log panics/cancellation at the owner, and prevent
    overlapping runs from applying stale proof. Do not create global Tokio
    runtimes, detached tasks, blocking async paths, or locks held across
    `.await`.
  - Contract placement tightening: `DependencyReadinessRequest`,
    readiness-proof ids, task/run/session/correlation ids, readiness policies,
    and proof-state enums must live in `pantograph-dependency-planning` or an
    existing dedicated shared contract crate when they cross crate boundaries.
    Workflow-service may own orchestration and lifecycle state, but it must not
    become the shared DTO owner. Raw graph/IPC/store values must parse once into
    validated newtypes/enums with specific error types; public APIs must not use
    raw strings, unbranded numeric revisions, `serde_json::Value`, path-shaped
    primitives, boolean mode flags, or `Result<T, String>` for this boundary.
  - Sync-core/async-shell tightening: readiness policy, freshness comparison,
    task-state transition selection, and proof validation should be synchronous
    pure code with focused unit tests. Async is limited to provider I/O,
    durable store access, runtime-host dispatch selection, and lifecycle
    orchestration, with cancellation and tracing spans carrying run/session/task
    correlation.
  - Verification tightening: before runtime-host dispatch is wired, add the
    thinnest cross-layer acceptance test that starts from an admitted runtime
    inference task, builds the typed readiness request, receives a provider
    `DependencyPreflightResult`, applies scheduler readiness admission, and
    exposes only `Ready` scheduler tasks as dispatch candidates. Add focused
    tests for stale graph revision, stale validation session, mismatched
    scheduler task id, mismatched model ref, duplicate/replayed proof,
    concurrent workflow runs, deferred proof, retryable failure, and terminal
    failure. Tests touching durable stores, temp roots, runtime registries, or
    dependency caches must isolate state per test or document a serialization
    guard.
  - Documentation/deletion gate: every source slice must update the touched
    source README or a linked ADR with the lifecycle owner, API consumer
    contract, error/retry/idempotency behavior, rejected alternatives, and
    revisit triggers. A slice that replaces a legacy readiness or preflight
    success path must delete or fail-close that old path in the same commit;
    do not leave compatibility shims, successful path-shaped fixtures, or
    adapter branches for later cleanup.
  - Blast-radius result: with the lifecycle, contract-placement, verification,
    and documentation gates above, option 3 stays standards-compliant and
    preserves the intended separation: graph editor and node-engine can display
    or validate preview facts, scheduler/workflow-service owns execution
    readiness, and runtime hosts receive only scheduler-approved handoff facts.
- [x] 2026-05-26 workflow-service readiness lifecycle contract slice completed:
  - Smallest useful vertical slice: add a focused workflow-service scheduler
    readiness lifecycle owner that builds a validated, path-free
    `DependencyReadinessRequest` for one admitted runtime inference task, calls
    an injected dependency-readiness provider, and delegates the returned
    `DependencyPreflightResult` to the existing scheduler readiness-admission
    bridge.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/scheduler/readiness_lifecycle.rs`,
    `crates/pantograph-workflow-service/src/scheduler/readiness_lifecycle_tests.rs`,
    `crates/pantograph-workflow-service/src/scheduler/mod.rs`,
    `crates/pantograph-workflow-service/src/scheduler/README.md`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the lifecycle projects only
    `SchedulableTaskIntent`/Pumas model ref/scheduler intent into
    `DependencyReadinessRequest`. It does not call `ModelDependencyRequest`,
    build `ModelRefV2`, read graph `modelPath` or `model_path`, synthesize load
    targets, call Pumas load-target resolution, dispatch runtime hosts, or
    route dependency readiness through node-engine/Tauri/frontend payloads.
  - Focused tests added: ready provider proof builds the expected request and
    admits the task to `Ready`; missing provider proof defers the task through
    scheduler policy; mismatched provider proof becomes a typed scheduler
    policy failure without a legacy bridge.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`;
    `cargo test -p pantograph-workflow-service readiness_lifecycle --
    --nocapture`; `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo check -p pantograph-workflow-service`; `git diff --check`; and
    targeted source search of the new lifecycle source/tests for retired
    path-shaped terms. `cargo check` still reports the pre-existing
    `set_active_run_execution_plan` dead-code warning in scheduler store; this
    slice did not introduce it.
  - Remaining follow-up: wire a production provider/lifecycle caller from the
    active run loop, then add the full-path acceptance test that exposes only
    `Ready` scheduler tasks as runtime-host dispatch candidates. The current
    lifecycle is synchronous and owns no background tasks; any later async
    provider work must add explicit cancellation, tracked task handles, and
    shutdown behavior in this lifecycle module.
- [x] 2026-05-26 dependency-environment to preflight projection slice
      completed:
  - Smallest useful vertical slice: add the canonical pure contract projection
    from `ValidatedDependencyEnvironmentResult` to
    `ValidatedDependencyPreflightResult` in `pantograph-dependency-planning`.
    This gives the workflow-service readiness provider/caller a real path-free
    proof source without synthesizing scheduler proof from graph node data,
    technical-fit preview facts, reduced execution-plan projections, or
    frontend/Tauri display state.
  - Allowed files touched:
    `crates/pantograph-dependency-planning/src/preflight.rs`,
    `crates/pantograph-dependency-planning/src/lib.rs`,
    `crates/pantograph-dependency-planning/src/README.md`,
    `crates/pantograph-dependency-planning/tests/contract.rs`, this milestone,
    and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the projection preserves only validated
    path-free identity, readiness state, dependency requirements id,
    dependency environment ref, and typed diagnostics. It does not carry
    requirements/binding display rows, operation timing, `ModelDependencyRequest`,
    `ModelRefV2`, graph `modelPath`, `model_path`, local load targets, Pumas
    package facts, runtime host handoff data, or executable paths into
    scheduler admission.
  - Focused tests added: ready dependency-environment result projects to ready
    preflight proof with requirements/environment identity; unavailable
    dependency-environment result projects its readiness state and diagnostics
    without fabricating environment identity.
  - Verification passed: `cargo fmt -p pantograph-dependency-planning` and
    `cargo test -p pantograph-dependency-planning dependency_preflight_projection
    --test contract -- --nocapture`.
  - Remaining follow-up: implement the workflow-service readiness provider that
    calls the dependency-environment service, projects the validated result
    through this function, and feeds the scheduler-owned readiness lifecycle.
- [x] 2026-05-26 workflow-service dependency-environment readiness provider
      adapter slice completed:
  - Smallest useful vertical slice: implement
    `WorkflowDependencyReadinessProvider` for the canonical
    `DependencyEnvironmentService` facade. The adapter turns the lifecycle's
    validated `DependencyReadinessRequest` into a path-free
    `DependencyEnvironmentRequest` with `Resolve`, validates provider output,
    projects it through
    `dependency_preflight_result_from_environment_result`, and returns the
    resulting `DependencyPreflightResult` to scheduler readiness admission.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/scheduler/readiness_lifecycle.rs`,
    `crates/pantograph-workflow-service/src/scheduler/readiness_lifecycle_tests.rs`,
    `crates/pantograph-workflow-service/src/scheduler/README.md`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the adapter consumes only canonical
    dependency-planning and dependency-environment-service contracts. It does
    not call `ModelDependencyRequest`, build `ModelRefV2`, read graph
    `modelPath` or `model_path`, inspect package facts, synthesize load
    targets, use Tauri/frontend display state, or convert reduced technical-fit
    facts into proof. The default not-implemented provider produces typed
    terminal scheduler diagnostics instead of a fallback runtime attempt.
  - Focused test added: the readiness lifecycle uses
    `DependencyEnvironmentService<NotImplementedDependencyEnvironmentProvider>`
    as a concrete provider and terminal-fails the runtime task through
    scheduler policy.
  - Verification passed: `cargo fmt -p pantograph-workflow-service` and
    `cargo test -p pantograph-workflow-service readiness_lifecycle --
    --nocapture`. The test still reports the pre-existing
    `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: wire the active run loop to call this lifecycle for
    runtime inference tasks in `WaitingDependencyReadiness`, then expose only
    `Ready` scheduler tasks to runtime-host dispatch selection.
- [x] 2026-05-26 active run-loop readiness caller re-plan boundary recorded:
  - Discovered issue: the next production caller cannot be wired standards-
    compliantly yet because the current `DependencyReadinessRequest`,
    `DependencyPlanningIdentityKey`, and `DependencyPreflightResult` path does
    not carry enough execution freshness identity to prove that a dependency
    readiness proof belongs to the admitted active runtime task. The lifecycle
    currently correlates model ref, task kind, scheduler intent, workflow id,
    run id, node id, and scheduler task id through task intent/caller context,
    but graph revision, executable validation snapshot identity, validation
    session id, descriptor fingerprint, dependency sidecar choices, and proof
    replay/idempotency identity are not first-class fields in the shared
    readiness/preflight contract.
  - Why this blocks implementation: calling the lifecycle from the active run
    loop now would let queue execution advance from task intent plus provider
    output alone. That would bypass the already planned requirement that
    scheduler admission consume the current backend validation summary and
    reject stale graph revisions, stale validation sessions, mismatched
    descriptor fingerprints, mismatched dependency sidecar choices, replayed
    proof, and cross-run/cross-task proof reuse before runtime-host dispatch.
  - Required re-plan: extend or wrap the shared dependency-readiness/preflight
    contract with a path-free execution freshness envelope before wiring the
    active run-loop caller. The design must decide the owner and exact fields
    for graph revision, validation session/snapshot id, descriptor fingerprint,
    scheduler task id, proof id/version, correlation id, selected binding ids,
    dependency override fingerprint, and idempotent retry/replay behavior.
    Workflow-service may orchestrate the lifecycle, but the cross-crate DTOs
    must remain in `pantograph-dependency-planning` or another dedicated
    contract crate.
  - No-fallback/no-legacy confirmation: do not wire active execution by
    inferring freshness from graph node data, technical-fit preview facts,
    reduced execution-plan dependency readiness, Tauri/frontend state, or the
    absence of a path. Do not preserve the old runtime-dispatch-not-wired path
    as a compatibility fallback once the freshness envelope and runtime caller
    are implemented.
- [x] 2026-05-27 active run-loop readiness freshness design selected:
  - Decision: use option 2. Add a separate path-free execution freshness
    envelope in `pantograph-dependency-planning` and keep
    `DependencyPlanningIdentityKey` focused on stable dependency identity.
    The envelope wraps existing readiness/preflight contracts instead of
    widening dependency identity with run-specific scheduler state.
  - Contract ownership: `pantograph-dependency-planning` owns the shared DTOs,
    validation wrappers, serde fixtures, and replay/freshness diagnostics.
    Workflow-service owns lifecycle orchestration and construction from the
    active scheduler task graph plus current executable validation state.
    Scheduler admission owns the final proof match against the admitted runtime
    task before moving a task from `WaitingDependencyReadiness` to `Ready`.
    `pantograph-dependency-planning` must define its own validated envelope
    newtypes for graph revision, validation session/snapshot identity,
    workflow/run/task/node identity, descriptor fingerprint, readiness proof
    identity, and correlation identity. It must not import
    inference-interface or scheduler-owned types because
    `pantograph-inference-interface-contracts` already depends on
    `pantograph-dependency-planning`. Workflow-service owns the adapters from
    graph, validation, and scheduler types into those shared envelope newtypes.
  - Planned API shape: add DTOs equivalent to
    `DependencyReadinessExecutionContext`,
    `ValidatedDependencyReadinessExecutionContext`,
    `DependencyReadinessProofEnvelope`, and
    `ValidatedDependencyReadinessProofEnvelope`. They must carry path-free
    workflow id, workflow run id, scheduler task id, node id, graph revision,
    validation session id or executable validation snapshot id, descriptor
    fingerprint, dependency requirements id, selected binding ids, dependency
    override fingerprint, readiness proof id, readiness proof version, and
    correlation id. These contracts must not carry local paths, Pumas package
    facts, load targets, selected artifact paths, worker settings, frontend
    display state, or runtime-host payloads.
  - Proof payload boundary: provider calls may still use the typed
    `DependencyReadinessRequest` and `DependencyPlanningRequest` data needed to
    resolve or install dependencies, including raw override patches when a
    provider owns that operation. Scheduler admission and dispatch proof must
    carry only stable identity, the dependency requirements id, selected binding
    ids, dependency override fingerprint, readiness state, bounded diagnostics,
    and envelope freshness fields. Do not copy raw override values, Pumas load
    targets, or provider-private request payloads into scheduler proof.
  - Validation rules: envelope validation must prove that the
    `DependencyReadinessRequest`, resulting `DependencyPreflightResult`,
    admitted `SchedulableTaskIntent`, `WorkflowSchedulerTask` descriptor
    fingerprint, and active graph validation session/snapshot identity all
    agree. Stale graph revisions, stale validation sessions, descriptor drift,
    mismatched selected binding ids, stale dependency override fingerprints,
    replayed proof ids, cross-run proof reuse, and cross-task proof reuse must
    produce typed scheduler admission failures with no runtime dispatch.
  - Freshness source ownership: workflow-service must make the source of these
    fields concrete before wiring the active run loop. The queue-admission path
    must retain the executable validation snapshot identity, graph revision,
    validation session id, node descriptor fingerprint, selected binding ids,
    dependency requirements id, and dependency override fingerprint in either a
    task-graph freshness context or a narrow active-run freshness context owned
    by workflow-service. Callers must not rediscover these values from draft
    graph node data, frontend state, technical-fit preview facts, or ad hoc
    side tables.
  - Ready-state proof retention: the transition from
    `WaitingDependencyReadiness` to `Ready` must retain the scheduler-admitted
    readiness proof envelope with the active runtime task, or in a
    workflow-service owned active-run proof store keyed by the task/state
    identity. Dispatch selection must read that retained proof; it must not ask
    the provider again or reconstruct proof from current graph state.
  - Standards compliance constraints:
    - Plan standards: each implementation slice must declare the affected
      structured contracts, persisted artifacts, ownership/write set, risks,
      verification, and re-plan triggers before editing code. Dirty source,
      test, config, lockfile, generated, build-output, sqlite WAL/SHM, and
      workflow fixture files remain blockers unless explicitly part of that
      slice.
    - Architecture standards: contracts stay in
      `pantograph-dependency-planning`, workflow-service remains the lifecycle
      owner, scheduler owns admission/dispatch policy, and node-engine,
      graph-editor, Tauri, frontend, runtime-host, and provider crates must not
      own or reconstruct scheduler readiness proof.
    - Rust API standards: raw envelope DTOs are parsed once into validated
      newtypes through `TryFrom` or equivalent constructors. Internal APIs that
      rely on freshness must accept validated envelope/context types, not raw
      `String` ids or unchecked serde structs. Public enums/structs that are
      likely to grow must be `#[non_exhaustive]`, public fallible APIs must
      return typed errors, and production paths must not use `unwrap`,
      `expect`, or `Result<T, String>` for recoverable validation failures.
    - Executable-contract standards: every envelope carries an explicit
      contract version, serde fixtures cover ready/unavailable/stale/replay
      cases, unknown-field and malformed-id inputs fail at the boundary, and
      future additions are append-only unless a new re-plan records the
      breaking change and migration.
    - Concurrency standards: dependency-planning validation remains a sync core.
      Workflow-service may use async only around provider/store calls, must not
      hold locks across provider awaits or blocking work, and must make proof
      retention atomic with the scheduler state transition so concurrent users,
      retries, and overlapping workflow runs cannot expose stale dispatch
      candidates.
    - Dependency standards: do not add new third-party crates for the envelope
      unless the slice records the ownership boundary, transitive cost, feature
      impact, and why existing workspace dependencies or the standard library
      are insufficient.
    - Security/diagnostic standards: diagnostics must be typed and bounded in
      count and size, must include enough context for stale/mismatch/replay
      investigation, and must not include local paths, selected artifact paths,
      raw override values, provider-private payloads, or runtime-host payloads.
    - Documentation standards: update module README/API Consumer Contract or
      Structured Producer Contract sections for every source directory whose
      public contract, persisted task state, lifecycle invariant, or scheduler
      proof ownership changes.
  - Lifecycle staging:
    1. Add the dependency-planning envelope DTOs, validation wrappers, typed
       diagnostics, serde fixtures, negative unknown-field tests, and source
       search checks for forbidden path/load-target fields.
    2. Extend workflow-service scheduler task projection to expose the current
       freshness source fields through an owner API, or add a narrow active-run
       freshness map with the same owner API. Do not create an ad hoc side
       table read directly by callers.
    3. Update the readiness lifecycle to build an enveloped readiness request,
       invoke the dependency-readiness provider, wrap the preflight proof in an
       envelope, and call scheduler readiness admission with the validated
       envelope.
    4. Replace `SchedulerDependencyReadinessProof` and scheduler admission
       inputs with envelope-aware proof contracts. Keep the proof validation in
       scheduler admission/dispatch selection rather than duplicating it in the
       node engine, graph editor, or runtime host.
    5. Update scheduler admission/orchestrator tests to reject stale,
       mismatched, replayed, cross-run, and cross-task proofs, and to verify
       the admitted proof is retained when the task transitions to `Ready`.
    6. Wire the active run loop to the lifecycle and expose dispatch candidates
       only for `Ready` tasks whose readiness proof envelope validates.
    7. Update the scheduler README and workflow-service scheduler README text
       that currently describes persisting only task-state transitions so the
       documented invariant includes the retained admitted readiness proof.
    8. Delete or replace the old runtime-dispatch-not-wired terminal path in
       the same replacement sequence; it must not remain as a compatibility
       fallback once the active runtime caller is canonical.
  - Verification gates: run focused dependency-planning contract tests,
    workflow-service lifecycle tests, scheduler admission/orchestrator tests,
    replay/idempotency tests for duplicate proof ids and active-run recovery,
    concurrent-workflow tests for cross-run/cross-task proof isolation,
    store/fixture migration tests for any persisted active-run task-state
    changes, `git diff --check`, and targeted source searches for
    `ModelDependencyRequest`, `ModelRefV2`, `modelPath`, `model_path`,
    `local_load_path`, `load_target`, and `selected_artifact_path` in the
    changed execution path. Add a cross-layer acceptance test from admitted
    runtime inference task to envelope-validated readiness and ready-only
    runtime-host dispatch candidate exposure. Run the affected suites with
    normal parallelism so shared-state leakage is visible.
- [x] 2026-05-27 dependency readiness execution envelope contract slice:
  - Smallest useful vertical slice: add only the
    `pantograph-dependency-planning` contract surface for path-free readiness
    execution contexts, request envelopes, proof envelopes, validated wrappers,
    serde fixtures, and public integration tests. Allowed write set:
    `crates/pantograph-dependency-planning/src/execution.rs`,
    `crates/pantograph-dependency-planning/src/lib.rs`,
    `crates/pantograph-dependency-planning/src/request.rs`,
    dependency-planning README/test/fixture docs, dependency-planning
    readiness execution fixtures/tests, and this plan/status log.
  - No-fallback/no-legacy confirmation: this slice adds canonical typed
    contracts only. It does not wire the active run loop, does not preserve the
    old runtime-dispatch-not-wired path as a compatibility behavior, does not
    reconstruct proof from graph/frontend state, and does not carry local
    paths, Pumas load targets, raw provider request payloads, or runtime-host
    payloads in scheduler proof.
  - Implementation completed: added dependency-planning-owned envelope newtypes
    for workflow/run/task/node identity, graph revision, validation
    session/snapshot identity, descriptor fingerprint, proof id/version, and
    correlation id; added `DependencyReadinessExecutionContext`,
    `DependencyReadinessRequestEnvelope`,
    `DependencyReadinessProofEnvelope`, and validated wrappers; added request
    and ready proof envelope fixtures plus negative tests for unknown fields,
    missing validation identity, caller-context drift, requirements drift, zero
    proof versions, executable handoff fields, and raw request payload leakage.
  - Standards notes: validation remains sync-core and parse-once through typed
    constructors; public fallible APIs return `DependencyPlanningContractError`;
    no new dependencies were added; module/test/fixture READMEs were updated
    with API Consumer Contract and Structured Producer Contract implications.
  - Verification passed: `cargo fmt -p pantograph-dependency-planning`,
    `cargo test -p pantograph-dependency-planning --test
    readiness_execution_contract -- --nocapture`, `cargo test -p
    pantograph-dependency-planning -- --nocapture`, `cargo check -p
    pantograph-dependency-planning`, `git diff --check`, and a targeted source
    search over changed production dependency-planning files for
    `ModelDependencyRequest`, `ModelRefV2`, `modelPath`, `model_path`,
    `local_load_path`, `load_target`, and `selected_artifact_path`.
  - Remaining follow-up: the next slice must expose workflow-service owned
    freshness source fields from queue admission/active-run state without ad
    hoc side tables, then wrap readiness lifecycle requests/proofs with the new
    dependency-planning envelopes before scheduler admission is changed.
- [x] 2026-05-27 current validation proof freshness field slice:
  - Smallest useful vertical slice: keep the next workflow-service prerequisite
    inside the current inference validation-state owner by retaining selected
    binding ids and dependency override fingerprint on
    `CurrentDependencyRequirementsProof`. Allowed write set:
    `crates/pantograph-workflow-service/src/graph/inference_validation_state.rs`
    plus this plan/status log.
  - No-fallback/no-legacy confirmation: this slice does not infer dependency
    freshness from graph node data, frontend state, reduced execution-plan
    projections, or runtime-host payloads. It preserves the canonical
    dependency requirements proof as the owner of path-free dependency sidecar
    identity needed by later scheduler proof envelopes.
  - Implementation completed: `CurrentDependencyRequirementsProofRequest` and
    `CurrentDependencyRequirementsProof` now carry selected binding ids and
    dependency override fingerprint. Proofs recorded directly and proofs
    derived from the dependency requirements producer retain those fields
    instead of dropping them.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`, `cargo
    test -p pantograph-workflow-service inference_validation_state --lib --
    --nocapture`, `cargo check -p pantograph-workflow-service`, `git diff
    --check`, and targeted source search over the changed validation-state file
    for `ModelDependencyRequest`, `ModelRefV2`, `modelPath`, `model_path`,
    `local_load_path`, `load_target`, and `selected_artifact_path`. The search
    matches only existing path-sanitization/test assertions in this file.
    Existing warning noted: `set_active_run_execution_plan` remains a
    pre-existing unused store method.
  - Remaining follow-up: expose these proof freshness fields through
    workflow-service scheduler inference projections and executable validation
    snapshots so queue admission can build
    `DependencyReadinessExecutionContext` without re-reading draft graph state.
- [x] 2026-05-27 executable snapshot dependency proof promotion re-plan
      boundary recorded:
  - Discovered issue: queue admission builds scheduler task graphs from saved
    `WorkflowExecutableValidationSnapshotRecord`, but executable validation
    snapshots are currently built from `WorkflowGraphInferenceValidationPublication`
    only. The publication contains descriptor, authored-port, validation,
    runtime/device, and model-ref facts, but it does not carry the current
    dependency requirements proof, selected binding ids, or dependency override
    fingerprint retained by `CurrentInferenceValidationStateStore`.
  - Why implementation stops: wiring dependency readiness execution contexts
    from queue admission now would either produce incomplete envelopes or force
    queue admission to reach back into live draft graph/session state. That
    would violate the no-fallback/no-legacy rule and the source-of-truth
    boundary: saved workflow execution must use durable executable validation
    snapshot state, not frontend state, draft graph node data, or live
    validation-session side tables.
  - Required re-plan: choose the canonical promotion path that moves current
    dependency requirements proof freshness from the draft validation owner into
    the durable executable validation snapshot or an explicitly linked durable
    owner before queue admission consumes it.
  - Options to decide before the next code slice:
    1. Add `graph_session_id` to executable snapshot publish and have
       workflow-service read current validation-state dependency proofs during
       publish. This is the smallest bridge, but it couples publish to a live
       graph session and needs strict stale-session diagnostics.
    2. Replace caller-supplied executable publication with a backend-owned
       publish command on the graph session that snapshots the graph, validates
       descriptor state, dependency proof state, and durable workflow version in
       one lifecycle-owned operation. This is cleaner and easier to reason
       about long term, but larger than a narrow field-threading slice.
    3. Persist dependency readiness freshness as a separate durable artifact
       keyed by validation snapshot id and node id, then have queue admission
       load snapshot plus freshness artifact. This avoids live-session coupling
       but adds another durable artifact and join invariant.
  - Recommendation to evaluate: option 2 best matches long-term ownership and
    simplicity because workflow-service would own the entire executable publish
    lifecycle and reject stale dependency proof before snapshot persistence.
    Option 1 may be viable as a thinner intermediate only if it is explicitly
    replaced by option 2 and does not become a compatibility path.
- [x] 2026-05-27 executable snapshot dependency proof promotion decision:
  - Decision: use option 2. Replace caller-supplied executable validation
    snapshot publication with a workflow-service owned graph-session publish
    command. The command owns the full lifecycle: snapshot the draft graph under
    the graph-session lock, release the lock before provider/store work,
    resolve or consume the current inference validation publication, require a
    matching current dependency requirements proof for every executable
    runtime inference node, validate the durable workflow version/fingerprint,
    and persist the executable validation snapshot with dependency proof
    freshness included.
  - Ownership boundary: workflow-service is the only owner of this publish
    lifecycle. Tauri, frontend, node-engine, runtime-host, scheduler, Pumas
    providers, and callers must not assemble executable validation snapshots or
    dependency proof freshness themselves. The saved executable validation
    snapshot becomes the durable source for queue admission; queue admission
    must not reach into live draft graph state or current validation side
    tables.
  - Snapshot contract update: add path-free dependency proof freshness to
    executable validation snapshot node records. Each runtime inference node
    that is executable must persist dependency requirements id, selected
    binding ids, dependency override fingerprint, graph revision, validation
    session id, descriptor fingerprint, model ref, task kind, runtime/device
    constraints, and typed diagnostics needed to explain stale or unavailable
    proof. Do not persist local paths, Pumas package facts, load targets, raw
    dependency override patches, provider-private request payloads, frontend
    display state, or runtime-host payloads.
  - API shape: add a backend-owned graph-session publish request that takes the
    graph session id, workflow id, workflow semantic version, optional
    validation snapshot id, and validation session id or explicit "publish
    latest current executable validation" selector. The response remains a
    `ValidatedWorkflowExecutableValidationSnapshotRecord`. Existing lower-level
    store/read APIs may remain as persistence helpers, but callers should use
    the graph-session publish API for executable workflow submission
    preparation.
  - Replaced behavior: retire direct caller assembly of
    `WorkflowExecutableValidationSnapshotPublishRequest` for the production
    executable publish path once the graph-session publish command is wired.
    Do not keep a compatibility path that accepts stale caller-supplied
    publications for runtime-containing workflows.
  - Review tightening: executable topology fingerprints currently exclude node
    data and dependency selections by design, while executable validation
    snapshots are stored one per workflow version. The graph-session publish
    command must therefore treat changed dependency proof freshness for the
    same workflow version as a typed conflict unless the incoming compact
    snapshot is byte-for-byte identical. It must not overwrite the existing
    snapshot or silently reuse a semantic version when dependency requirements
    id, selected binding ids, override fingerprint, validation session, graph
    revision, or descriptor fingerprint differ.
  - Review tightening: add a workflow-service owned
    `ExecutableValidationSnapshotSource` read model, or equivalently named
    internal DTO, that joins the current inference validation publication with
    current dependency requirements proof before snapshot persistence. Queue
    admission must consume only the persisted snapshot fields; it must not join
    live validation side tables, draft graph node data, frontend state, Tauri
    state, or provider-private payloads after a workflow run is admitted.
  - Review tightening: queue admission must build the shared
    `DependencyReadinessExecutionContext` from saved snapshot freshness plus
    the active workflow run id and scheduler task id. Scheduler readiness must
    consume a validated request/proof envelope or an explicitly documented
    workflow-service wrapper around that envelope; it must not reconstruct
    selected binding ids or dependency override state from active task intent.
  - Review tightening: desktop submission needs a thin transport integration
    for the graph-session publish command. The graph editor/frontend may
    request publish for the active graph session before creating/running the
    execution session, but it must not assemble executable snapshots,
    dependency proof freshness, Pumas facts, runtime choices, or dependency
    policy. Tauri commands remain pass-through adapters only.
  - Maintainability tightening: do not add the new publish lifecycle as more
    bulk inside the existing oversized workflow-service files. Split the next
    code slices into focused modules for snapshot source/read-model construction,
    graph-session executable publish orchestration, executable snapshot
    contracts, and queue-admission proof projection.
  - Standards tightening: persisted and wire contract changes must be explicit
    executable-contract revisions. Bump executable validation snapshot schema
    version and scheduler task graph schema version when their persisted shapes
    change, keep serde `deny_unknown_fields`/default semantics intentional, add
    typed errors for old-schema/stale-proof/conflict cases, and reject retired
    schema shapes rather than preserving fallback readers.
  - Standards tightening: cross-language and transport contract changes must be
    updated together in the owning slice. Rust request/response DTOs, Tauri
    command wrappers, TypeScript workflow-service types, command-service
    methods, and any intentionally supported UniFFI binding surface must agree
    on field names, casing, optionality, and error mapping. If UniFFI is not
    part of this product surface for the slice, document it as intentionally
    out of scope rather than leaving a partial binding.
  - Standards tightening: keep a sync-core/async-shell shape. Graph-session
    publish may await provider/transport work, but snapshot-source construction,
    validation, conflict classification, and projection should be synchronous
    typed helpers. Locks must only protect graph/session snapshots and current
    validation reads; provider calls, attribution-store writes, and frontend
    transport work must run after locks are released.
  - Standards tightening: durable publish must be retry-safe. If cancellation
    or failure occurs after workflow-version resolution but before snapshot
    persistence, queue admission must fail closed with the missing-snapshot
    diagnostic and a later publish retry must either store an identical compact
    snapshot or return the typed changed-proof/version conflict.
  - Standards tightening: the first implementation slice must include a
    vertical acceptance path before broad horizontal expansion: graph-session
    validation publication plus dependency proof -> graph-session executable
    publish -> persisted snapshot lookup -> saved workflow run admission ->
    dependency readiness execution-context/envelope construction. Focused unit
    tests can cover each contract branch, but they do not replace this path.
  - Standards tightening: shared contracts, persisted DTOs, schema versions,
    frontend TypeScript types, Tauri command wrappers, generated/binding files,
    and plan files remain serial integration-owner work. Do not split these
    across parallel workers. Sub-agents, if used later, may only own
    non-overlapping implementation modules and must report required shared
    contract changes instead of editing them.
  - Implementation staging:
    1. Extend `CurrentInferenceValidationStateStore` with a read API that
       returns current executable node projection plus current dependency
       requirements proof for a graph session/revision/validation session,
       without exposing mutable side-table access to queue admission.
    2. Extend executable validation snapshot node records and scheduler
       inference projections with dependency proof freshness fields. Add
       schema version bumps, schema/fixture tests for persisted snapshots, and
       fail-closed diagnostics when an executable runtime inference node lacks
       current proof.
    3. Add the workflow-service graph-session publish command that snapshots
       the graph, validates current publication/proof freshness, resolves the
       workflow version, rejects changed proof freshness for an existing
       workflow version with a typed conflict, and persists the executable
       validation snapshot.
    4. Update queue admission to consume only the saved snapshot freshness
       fields when building `DependencyReadinessExecutionContext` and the
       scheduler-facing readiness request/proof envelope.
    5. Add frontend/Tauri submit integration that calls the graph-session
       publish command as a transport operation before running a saved workflow
       execution session. Update TypeScript command/service types in the same
       slice, keep validation summary and submit gating backend owned, and use
       event/subscription-backed state instead of adding UI polling.
    6. Remove or restrict the old caller-supplied executable publication path
       for runtime-containing workflows so it cannot bypass dependency proof
       promotion.
  - Verification gates: add focused tests for stale graph revision, stale
    validation session, missing dependency proof, mismatched dependency
    requirements id, mismatched selected binding ids, stale dependency override
    fingerprint, changed proof freshness under the same workflow semantic
    version, snapshot persistence round trip, compact snapshot path-field
    rejection/search, queue-admission freshness projection, scheduler envelope
    validation, publish retry after partial durable state, frontend/Tauri
    publish-before-run transport, TypeScript/Rust contract field alignment, and
    rejection of direct caller-supplied runtime snapshot publication after
    cutover. Run workflow-service validation-state, executable snapshot,
    session execution, scheduler task graph/readiness, dependency-planning
    envelope, frontend command-service/toolbar, and Tauri command tests plus
    `git diff --check`.
- [x] 2026-05-27 current executable validation snapshot source slice:
  - Smallest useful vertical slice: added the workflow-service validation-state
    read model that returns current executable node projection records joined
    with current dependency requirements proof. Touched files were limited to
    `crates/pantograph-workflow-service/src/graph/executable_validation_snapshot_source.rs`,
    `crates/pantograph-workflow-service/src/graph/inference_validation_state.rs`,
    `crates/pantograph-workflow-service/src/graph/mod.rs`, and this plan log.
  - No-fallback/no-legacy confirmation: queue admission and future snapshot
    publication now have a validation-state-owned source for proof freshness.
    The slice does not read draft graph node data, frontend/Tauri state,
    provider-private payloads, Pumas package facts, local paths, or runtime load
    targets.
  - Implementation completed: `CurrentInferenceValidationStateStore` now has a
    `current_executable_validation_snapshot_source` read API that returns owned
    projection-plus-proof records and fails closed for missing, stale,
    unavailable, invalid, incomplete, stale-session, missing-summary, and
    non-executable-summary cases. The current validation node record retains
    the original projection record so the later snapshot publisher does not
    have to reconstruct descriptor/authored-port data from reduced scheduler
    projections.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`, `cargo
    test -p pantograph-workflow-service inference_validation_state --lib --
    --nocapture`, `cargo check -p pantograph-workflow-service`, `git diff
    --check`, and targeted source search over the changed validation-state and
    snapshot-source files for `ModelDependencyRequest`, `ModelRefV2`,
    `modelPath`, `model_path`, `local_load_path`, `load_target`,
    `selected_artifact_path`, `package_facts`, and `runtime_load_target`.
    Search matches are existing path-sanitization/test assertions. Existing
    warning: `set_active_run_execution_plan` remains a pre-existing unused
    store method.
  - Follow-up completed by the next slice: the graph-session executable publish
    command now consumes this read model, so the temporary module-level
    `dead_code` allowance was removed.
- [x] 2026-05-27 graph-session executable snapshot proof promotion slice:
  - Smallest useful vertical slice: combined the required snapshot contract
    extension with graph-session-owned executable snapshot publication and
    fail-closed rejection of caller-supplied runtime publications. This was
    required because adding optional proof fields would preserve a legacy
    successful path, while making them required without a graph-session
    publisher would leave a broken intermediate state.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/workflow/executable_validation_snapshot.rs`,
    `crates/pantograph-workflow-service/src/workflow/attribution_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/task_graph.rs`,
    `crates/pantograph-workflow-service/src/graph/session_inference_validation_api.rs`,
    `crates/pantograph-workflow-service/src/graph/inference_validation_state.rs`,
    `crates/pantograph-workflow-service/src/graph/mod.rs`,
    workflow-service graph/workflow READMEs, focused workflow-service tests,
    and this plan/status log.
  - No-fallback/no-legacy confirmation: executable snapshots now require
    path-free dependency proof freshness for each runtime inference node.
    Caller-supplied runtime validation publications fail closed and cannot
    assemble executable snapshots from reduced projection data. Scheduler
    inference projections from current validation state also require current
    dependency requirements proof; missing/stale/unavailable/invalid proof is a
    typed failure rather than a best-effort projection.
  - Implementation completed: bumped
    `WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION` to 2; added
    required node fields for `dependency_requirements_id`,
    `selected_binding_ids`, and `dependency_override_fingerprint`; added
    source-based snapshot construction from the validation-state read model;
    added `WorkflowGraphSessionExecutableValidationSnapshotPublishRequest` and
    `WorkflowService::publish_graph_session_executable_validation_snapshot`;
    rejected changed dependency proof freshness when a snapshot already exists
    for the same workflow version; and restricted
    `publish_workflow_executable_validation_snapshot` so runtime-containing
    caller-supplied publications are no longer a successful path.
  - Focused tests updated/added: source-to-snapshot proof compaction,
    caller-supplied publication proof rejection, caller-supplied runtime
    service publish rejection, proof-aware scheduler projection, and existing
    snapshot round-trip/projection tests updated for schema 2 fields.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
    check -p pantograph-workflow-service`;
    `cargo test -p pantograph-workflow-service executable_validation_snapshot
    --lib -- --nocapture`; `cargo test -p pantograph-workflow-service
    inference_validation_state --lib -- --nocapture`; `cargo test -p
    pantograph-workflow-service task_graph --lib -- --nocapture`; and `cargo
    test -p pantograph-workflow-service task_binding_resolution --lib --
    --nocapture`; `cargo test -p pantograph-workflow-service workflow_version
    --lib -- --nocapture`; `git diff --check`; and targeted source search over
    changed workflow-service source/test files for `ModelDependencyRequest`,
    `ModelRefV2`, `modelPath`, `model_path`, `local_load_path`,
    `load_target`, `selected_artifact_path`, `package_facts`, and
    `runtime_load_target`. Search matches are existing path-sanitization/test
    assertions and `PumasModelRef.selected_artifact_path: None` fixture
    values. Existing warning: `set_active_run_execution_plan` remains a
    pre-existing unused store method.
  - 2026-05-27 follow-up slice completed: queue admission now carries saved
    snapshot freshness into scheduler task templates as a typed
    `dependency_readiness_source`; the readiness lifecycle builds and validates
    a shared `DependencyReadinessRequestEnvelope` from that source before
    dependency-environment provider calls and scheduler readiness admission.
  - 2026-05-27 follow-up slice completed: scheduler readiness admission,
    dispatch selection, dispatch decision, runtime handoff, workflow-service
    readiness lifecycle, and runtime-host request fixtures now consume the
    shared `DependencyReadinessProofEnvelope` directly. The older
    scheduler-local ready-proof wrapper was removed rather than preserved as a
    compatibility shim, and scheduler validation now checks proof execution
    context against workflow/run/node/task identity before admitting or
    dispatching runtime work.
  - 2026-05-27 follow-up slice completed: Frontend/Tauri transport now exposes
    the graph-session executable validation snapshot publisher as a typed
    Tauri command and TypeScript service method. The command forwards the
    `WorkflowGraphSessionExecutableValidationSnapshotPublishRequest` directly
    to workflow-service and returns the backend-owned
    `WorkflowExecutableValidationSnapshotRecord`; Tauri and TypeScript do not
    compute validation freshness, workflow version fingerprints, dependency
    proof freshness, or scheduler admission authority.
  - 2026-05-27 follow-up slice completed: saved-run submit now calls the
    graph-session executable validation snapshot publisher before each
    scheduler execution-session run attempt. The toolbar passes only the active
    graph session id, saved workflow id, explicit workflow semantic version,
    and null optional validation/snapshot ids so workflow-service resolves the
    current validation state or fails closed. Direct edit-session execution
    remains disabled. Verification passed with TypeScript typecheck, focused
    toolbar/command tests, and `git diff --check`.
  - 2026-06-02 follow-up slice completed: embedded-runtime resource-backed
    hosted composition now has focused coverage for publishing a compact
    graph-session executable validation snapshot from production facts. The
    test builds isolated Pumas package facts, registers a ready PyTorch runtime
    with backend keys, refreshes workflow-service current validation, resolves
    dependency-environment readiness through the public backend action-intent
    API, publishes the executable snapshot, and asserts saved snapshot plus
    scheduler projection records preserve runtime constraints, proof freshness
    ids, and nonzero RAM/VRAM estimator hints. The slice discovered and
    preserved the fail-closed proof prerequisite: publishing without a current
    dependency requirements proof returns `dependency requirements proof is
    missing for executable node infer`; the validated path records that proof
    through backend readiness resolution rather than a test-only shortcut.
    Verification passed with `cargo fmt`, `cargo test -p
    pantograph-embedded-runtime
    resource_backed_hosted_service_publishes_executable_snapshot_from_production_facts`,
    and `cargo check -p pantograph-embedded-runtime`.
  - 2026-05-27 re-plan decision: use option 3 as the next implementation path.
    Add a backend-owned current validation summary read model/API for
    `graph_session_id + graph_revision`, then wire the graph editor submit gate
    to that summary. The API must return typed states for current, pending,
    missing, stale, unavailable, invalid, and executable/not-executable
    validation; include the current `validation_session_id`, graph revision,
    summary diagnostics, and enough graph-level status for the toolbar to
    disable Submit without resolving descriptors or Pumas/runtime facts in
    Tauri/frontend code. This read model is the only submit-gate authority; the
    existing publish-before-run command remains the fail-closed admission guard.
  - 2026-05-27 investigation tightening: implement the option 3 read model as a
    cheap read of already-recorded workflow-service validation state, not as a
    descriptor/Pumas/runtime/dependency re-resolution path. Expected editor
    states such as missing, pending, stale, unavailable, invalid, and
    non-executable must be typed response states rather than transport errors.
    The response must include a backend-owned submit gate such as
    `submit_gate.allowed`, typed `reason_code`, and display message so the
    frontend does not duplicate validation or scheduler-admission policy. The
    response should also carry `graph_session_id`, `graph_revision`, current
    `validation_session_id`, the bounded validation summary, and bounded
    diagnostics. Tauri remains a thin transport wrapper, and TypeScript mirrors
    for this summary/gate must be strongly typed; do not reuse the persisted
    executable snapshot's opaque `validation_summary: unknown` transport field
    as the editor gate contract.
  - Submit race-control tightening: after the editor reads an allowed backend
    submit gate, saved-run Submit must pass that summary's
    `validation_session_id` into
    `publishGraphSessionExecutableValidationSnapshot` so the fail-closed
    publisher proves it is snapshotting the same validation generation the UI
    accepted. If the graph revision or validation session changed before
    publish, workflow-service must return typed stale/mismatch diagnostics
    rather than silently selecting a newer or older validation state.
  - Standards verification tightening for option 3: the implementation slice
    must include focused workflow-service tests for current, missing, pending,
    stale, unavailable, invalid, non-executable, validation-session mismatch,
    and executable summary/gate responses; Tauri command tests proving thin
    forwarding and typed error envelopes; TypeScript service tests proving
    strong DTO mirrors; toolbar/store tests proving Submit is disabled from the
    backend `submit_gate` without frontend policy duplication; and one thin
    cross-layer acceptance check that reads the backend summary, disables or
    enables Submit from it, and passes the accepted `validation_session_id` to
    the snapshot publisher. Update affected source READMEs in the same slice.
    Option 3 must not add polling, background validation workers, descriptor
    resolution, Pumas/runtime fact lookup, dependency proof generation, or
    scheduler admission policy to Tauri or frontend code. If a temporary
    frontend refresh helper is needed, it must be action/event-triggered, scoped
    to the graph session/revision owner, and cleaned up on graph/session change.
  - Future target, option 4 event-driven live validation lifecycle: after the
    option 3 read model is working, replace explicit refresh/query mechanics
    with workflow-service-owned event-driven validation. Graph mutations and
    explicit validation requests should start validation asynchronously without
    blocking graph editing; stale validation sessions are discarded by graph
    session id, graph revision, and validation session id; node-scoped events
    carry descriptor, drift, diagnostic, and update-proposal payloads; graph-
    scoped events carry summary status; the editor renders authored ports
    immediately, overlays pending/stale/unavailable/invalid backend state, and
    disables Submit from the latest backend summary. This future slice must
    reuse the option 3 summary DTO/state owner rather than creating a parallel
    validation truth path.
  - Option 4 preservation note: the eventual event-driven validation lifecycle
    must publish the same backend-owned summary/gate shape used by option 3,
    with graph session, graph revision, validation session, and monotonic event
    sequence identity. It may change transport mechanics from explicit query to
    event delivery, but it must not add a second submit-gate authority, a
    frontend polling policy loop, or frontend-owned validation interpretation.
    The workflow-service lifecycle owner must own any spawned validation tasks,
    tracked handles, cancellation tokens, bounded event buffers, supersession
    rules, shutdown/session-close cleanup, and panic/error observation. Graph
    editing must remain available while validation is pending, and tests must
    prove stale or superseded sessions cannot mutate current state.
  - 2026-05-27 follow-up slice completed: workflow-service now exposes a
    current graph validation summary read model for `graph_session_id +
    graph_revision`, Tauri exposes it as a thin command, TypeScript mirrors the
    typed summary/gate DTOs, and `WorkflowToolbar.svelte` disables Submit from
    the backend `submit_gate`. Saved-run Submit now passes the accepted
    `validation_session_id` into the executable snapshot publisher so
    validation-session races fail closed in workflow-service.
  - No-fallback/no-legacy confirmation: the summary API is a read of current
    workflow-service validation state. It does not resolve descriptors, Pumas
    model state, runtime facts, dependency proofs, or scheduler policy in
    Tauri/frontend code, and missing/pending/stale/unavailable/invalid states
    are typed gate responses rather than compatibility fallbacks.
  - Verification passed: `cargo fmt`; `cargo test -p
    pantograph-workflow-service current_validation_summary`; `npm run
    test:frontend -- WorkflowService.commands.test.ts
    workflowToolbarEvents.test.ts`; and `npm run typecheck`. Verification
    blocked for full Tauri app compile by the pre-existing
    `src-tauri/src/app_setup.rs:96 Arc<WorkflowService>::set_media_conversion_executor`
    missing-method error.
  - 2026-05-27 follow-up slice completed: workflow-service now owns current
    validation refresh for `graph_session_id + graph_revision`. The refresh
    path rejects stale requested revisions as typed summary/gate responses,
    generates the validation-session id in the backend, runs the existing
    descriptor publication core outside the graph lock, records current state,
    and returns the same summary DTO consumed by Submit. Tauri and TypeScript
    expose thin refresh commands, and the toolbar uses refresh instead of a
    passive read so a clean saved graph can obtain a backend validation session
    without frontend-owned validation identity.
  - 2026-05-27 follow-up slice completed: current validation refresh now
    returns a dedicated typed response containing the summary/gate plus bounded
    `InferenceInterfaceNodeProjectionRecord` values from the same backend
    descriptor-publication pass. This gives graph-editor follow-up work access
    to descriptor/authored-port projection data without asking Tauri or
    frontend code to resolve descriptors, read Pumas facts, infer scheduler
    policy, or reinterpret submit eligibility. The toolbar continues to consume
    only `response.summary.submit_gate` for Submit.
  - No-fallback/no-legacy confirmation: refresh requests do not accept
    caller-minted validation session ids, raw Pumas facts, runtime facts,
    dependency proofs, package summaries, load targets, local paths, or
    scheduler policy. The default fact-provider path still reports typed
    unavailable/not-implemented states rather than synthesizing support.
    Projection records are display/review data only and must not become a
    second validation or submit-gate authority.
  - Verification passed: `cargo fmt`; `cargo test -p
    pantograph-workflow-service current_validation_summary`; `npm run
    test:frontend -- WorkflowService.commands.test.ts
    workflowToolbarEvents.test.ts`; `npm run typecheck`; and `git diff
    --check`. Verification blocked for full Tauri app compile by the existing
    `src-tauri/src/app_setup.rs:96 Arc<WorkflowService>::set_media_conversion_executor`
    missing-method error.
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
      2026-05-27 follow-up slice in progress: frontend graph materialization
      now needs to match the backend effective-definition rule. Generic
      inference nodes must render authored ports from
      `node.data.inference_interface_snapshot` and ignore retired
      `node.data.definition` overlays, so saved authored snapshots can display
      without preserving frontend-owned dynamic inference-setting ports. This is
      display-only and must not make the frontend a descriptor resolver or
      submit-gate authority.
      Verification passed: `npm run test:frontend --
      definitionOverlay.test.ts`; `npm run typecheck`; and `git diff --check`.
      2026-05-27 follow-up slice in progress: graph store display overlays now
      need to recompute node definitions after runtime overlays merge, so
      validation projection snapshots can update visible inference ports without
      writing to the persisted graph or reusing retired `node.data.definition`.
      Verification passed: `npm run test:frontend --
      createWorkflowStores.test.ts definitionOverlay.test.ts`; `npm run
      typecheck`; and `git diff --check`.
      2026-05-27 follow-up slice in progress: toolbar validation refresh should
      consume backend `node_projections` by projecting authored snapshots into
      display-only runtime overlays. Submit state must still come only from the
      backend summary/gate.
      Verification passed: `npm run test:frontend --
      workflowValidationProjectionOverlays.test.ts workflowToolbarEvents.test.ts`;
      `npm run typecheck`; and `git diff --check`.
- [x] Ensure runtime/device constraints narrow interface validation without
      becoming scheduler decisions. Explicit invalid constraints block enqueue
      and may include typed advisory alternatives when they can be computed
      safely; alternatives must not become fallback execution choices.
      2026-05-27 implementation slice: tightened
      `resolve_inference_interface_from_facts` so runtime alternatives are
      computed only from runtimes that are both available and supported by the
      resolved capability, and device validation is scoped through the same
      eligible runtime set. When an explicit runtime is present, an explicit
      device must be available on that runtime; a device on a different runtime
      remains an invalid explicit device constraint rather than a fallback
      placement. The scheduler still receives only validated constraints from
      the validation projection path and does not choose advisory alternatives.
      Verification passed: `cargo fmt --manifest-path
      crates/pantograph-workflow-service/Cargo.toml`; `cargo test -p
      pantograph-workflow-service inference_interface_resolver`; `cargo check
      -p pantograph-workflow-service` (existing warning:
      `WorkflowExecutionSessionStore::set_active_run_execution_plan` is unused).
- [ ] Wire graph editor draft validation to request and render the descriptor
      for unsaved graph state through the live validation session model. The
      editor renders saved authored ports immediately, overlays pending/stale/
      invalid validation state from backend events, disables submit/enqueue
      from the latest backend summary, and does not block editing while
      validation runs.
      2026-05-27 implementation slice: frontend validation refresh eligibility
      is now keyed only by workflow graph type, graph session id, and current
      graph revision. Dirty saved-workflow state no longer suppresses
      validation refresh, so unsaved graph edits can request backend descriptor
      projections from the current edit session and render display-only
      validation overlays while editing continues. Dirty state remains a submit
      gate through `workflowSubmitDisabledReason`; validation availability is
      not treated as permission to enqueue unsaved workflow revisions.
      Verification passed: `npm run test:frontend --
      workflowToolbarEvents.test.ts workflowValidationProjectionOverlays.test.ts`;
      `npm run typecheck`.
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
- [x] 2026-05-28 dependency-environment frontend source cleanup slice
  completed:
  - Smallest useful vertical slice: retired frontend path-era upstream subject
    inference from `dependencyEnvironmentSources.ts` and removed automatic
    adoption of upstream dependency requirements into dependency-environment
    node data.
  - Files touched by the slice:
    `src/components/nodes/workflow/dependencyEnvironmentSources.ts`,
    `src/components/nodes/workflow/DependencyEnvironmentNode.svelte`,
    `src/components/nodes/workflow/DependencyEnvironmentStatusPanel.svelte`,
    `src/components/nodes/workflow/dependencyEnvironmentNodeState.ts`,
    `src/components/nodes/workflow/dependencyEnvironmentState.test.ts`,
    `src/components/README.md`, this milestone, and
    `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: dependency-environment UI now detects
    only the typed `dependency_environment_sidecar` visual association and
    user-authored manual override inputs. It does not read upstream
    `modelPath`/`model_path`, model ids, task/backend keys, platform context,
    package-shaped dependency requirements, or arbitrary graph data as action
    authority.
  - Verification passed: `node --experimental-strip-types --test
    src/components/nodes/workflow/dependencyEnvironmentState.test.ts`; `npm run
    typecheck`; targeted source-search over the touched frontend dependency
    environment files; and `git diff --check`.
  - Remaining follow-up: dependency requirements/status/environment references
    are still display fields in node data for backend-issued projections. A
    later backend-event slice must refresh those fields from workflow-service
    validation/action results and keep them stale unless they match current
    graph revision, validation session, descriptor fingerprint, and dependency
    planning identity.
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
      Re-plan decision recorded on 2026-05-28: implement option 2 followed by
      option 3. First extract the current refresh implementation into a
      workflow-service validation publisher core that owns the reusable
      `graph snapshot -> fact provider -> descriptor publication -> current
      validation state` path. The existing explicit refresh command must call
      this publisher instead of keeping inline validation logic. After that
      publisher is tested, add the live validation lifecycle owner around the
      same publisher. Do not add a second validation resolver, do not move
      descriptor resolution into Tauri/frontend, and do not let graph mutation
      APIs become validation policy owners.
      Option 2 implementation requirements:
      1. Add a workflow-service publisher API that accepts graph session id,
         requested graph revision, and a graph snapshot or snapshot request.
         It must compute/confirm the current graph revision, release
         graph-session locks before fact resolution, call the existing
         inference-interface fact provider, publish descriptor projections,
         record current validation state, and return the same summary plus
         bounded node projections used by the toolbar refresh path.
      2. Route every existing validation publication entrypoint through the
         same publisher core. `refresh_current_validation_summary` must call
         it, and `publish_inference_validation_session` must either call it or
         be retired if caller-supplied validation session ids are no longer
         canonical. Do not keep parallel graph snapshot, fact lookup,
         publication, or current-state recording implementations.
      3. Re-check graph/session freshness after fact resolution and before
         recording current validation state, returning stale typed diagnostics
         instead of writing publication results when the graph revision or
         session has changed while facts were being resolved.
      4. Keep the publisher synchronous-domain/async-boundary split explicit:
         extraction, publication, aggregation, and state recording stay in
         workflow-service domain modules; async is limited to fact-provider,
         lock, persistence, or transport boundaries.
      5. Add an explicit bounded node-projection policy. If publication would
         exceed the bound, return a typed validation diagnostic and do not
         serialize or emit an unbounded projection list. Keep this separate from
         the existing validation event cap.
      6. Add the publisher as a focused workflow-service module owned by the
         graph validation boundary, then have `GraphSessionStore` compose it.
         Do not add more inline lifecycle/publisher behavior to
         `GraphSessionStore`.
      7. Add tests proving lock-free fact resolution, stale revision rejection
         before and after fact lookup, bounded projection records, current-state
         publication, no raw resolver facts from transport callers, no parallel
         publication path, and parity with the existing refresh response shape.
      Option 3 lifecycle-owner requirements:
      1. Add a workflow-service lifecycle owner keyed by graph session id and
         graph revision that starts, cancels, supersedes, and cleans up
         validation sessions. It must publish pending/stale/invalid/executable
         state without blocking graph display or editing.
      2. Use bounded event/state buffers with typed overflow/backpressure
         diagnostics, observe task errors and panics, cancel or supersede
         in-flight work when graph revisions change, and reject writes for stale
         graph revisions or validation session ids.
      3. Stop accepting validation work when a graph session closes and ensure
         close cleanup cannot leave stale events able to update current
         validation state.
      4. Add validation-state cleanup APIs and call them from graph-session
         close and stale-session cleanup. Cleanup must cancel in-flight
         validation work, drop buffered validation state for the closed session,
         and prevent late provider results from recording current state.
      5. Keep the lifecycle owner as a dedicated workflow-service module
         composed by `GraphSessionStore`; it owns cancellation, supersession,
         backpressure, session-close cleanup, and publication scheduling, while
         the publisher owns one validation publication attempt.
      6. Expose lifecycle events through a dedicated graph-validation
         transport-only Tauri event channel/subscription after backend tests
         pass. Do not reuse execution-run `WorkflowEvent` channels as the
         validation lifecycle transport, and do not couple validation event
         delivery to graph mutation responses.
      7. Tauri may carry events and commands only; it must not own freshness,
         descriptor resolution, enqueue policy, dependency request derivation,
         cancellation policy, validation buffers, or graph mutation policy.
      8. The graph editor consumes validation events as display/status overlays:
         authored ports render immediately, backend projections update
         display-only runtime overlays, submit/enqueue state comes from the
         latest backend validation summary, and editing remains available while
         validation is pending.
      Standards-compliance gates for the option 2 -> option 3 sequence:
      1. Public workflow-service APIs must use typed request/response/error
         structs, for example `WorkflowGraphValidationPublishRequest`,
         `WorkflowGraphValidationPublishResponse`, and
         `WorkflowGraphValidationSnapshot`, or equivalent existing canonical
         DTOs. Raw graph-session, node, revision, and validation-session strings
         must be parsed at the graph API/Tauri boundary before internal use; do
         not expose `Result<T, String>`, `anyhow`, untyped maps, or raw JSON as
         the publisher/lifecycle module contract.
      2. Add or reuse explicit typed diagnostic codes before implementation uses
         them. Overflow, backpressure, stale revision, superseded validation,
         provider failure, task panic, task cancellation, closed session, and
         projection-bound failures must not be encoded only in message text.
         Diagnostic context must stay bounded and omit raw Pumas facts, local
         paths, and unbounded node projection payloads.
      3. The dedicated graph-validation transport event must have typed identity
         fields for graph session id, graph revision, validation session id,
         event sequence, event kind, bounded payload, and diagnostic summary.
         Frontend consumers must discard stale events by identity and sequence;
         Tauri must only forward typed transport DTOs and must not translate
         resolver facts or validation policy.
      4. Tests must use deterministic fakes, barriers, or channels rather than
         sleeps to prove no graph-session lock is held across fact-provider
         await points, provider results are rejected after close/supersession,
         stale writes cannot update current validation state, bounded buffers
         emit typed diagnostics, and task cancellation/panic paths are observed
         at the lifecycle owner.
      5. New source modules/directories must update the relevant README or linked
         ADR in the same implementation slice. Required documentation includes
         the API consumer contract, lifecycle ownership, cancellation/cleanup
         behavior, event ordering, projection bounds, diagnostics, and rejected
         alternatives. Candidate ownership points are the workflow-service graph
         README, workflow-service crate README, Tauri workflow README, frontend
         workflow service/graph-editor READMEs, and this milestone file.
      6. Do not add new dependencies for lifecycle/event delivery unless an
         implementation slice records why existing Tokio/std primitives are not
         sufficient. Runtime creation remains at the composition root; spawned
         validation work must have tracked handles, cancellation, shutdown, and
         panic/error observation.
      7. Each slice must include source-search verification proving retired
         validation publication paths, reused execution `WorkflowEvent`
         transports, raw transport facts, compatibility aliases, and fallback
         validation resolvers are absent from the touched blast radius.
      Re-plan triggers: stop if implementing the publisher requires frontend or
      Tauri-supplied resolver facts, if graph mutation APIs must own validation
      policy, if lifecycle cancellation cannot be modeled without a workflow-
      service owner, if event delivery would require unbounded buffers or
      detached tasks without shutdown, or if queue admission would need to
      consume anything other than current backend validation state. Also stop
      if current validation state cannot be cleaned up on graph-session close,
      if stale provider results cannot be rejected after fact lookup, or if a
      dedicated graph-validation transport would require duplicating validation
      resolver logic outside workflow-service.
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
      - 2026-05-28 lifecycle event read-model sub-slice completed:
        `inference_validation_lifecycle.rs` now exposes a typed lifecycle event
        snapshot read model through `GraphSessionStore` for later transport
        wiring. The snapshot carries only graph-session identity, graph
        revision, validation-session identity, monotonic sequence, event kind,
        typed cancellation/rejection reason, and bounded dropped-event count.
        It does not expose resolver facts, Pumas/package facts, paths,
        scheduler decisions, frontend state, runtime-host payloads, or
        unbounded projection data.
      - Verification passed: `cargo fmt -p pantograph-workflow-service` and
        `cargo test -p pantograph-workflow-service
        inference_validation_lifecycle --lib`; and `cargo test -p
        pantograph-workflow-service
        validation_lifecycle_event_snapshot_reports_backend_refresh_events
        --lib`; `cargo check -p pantograph-workflow-service`; touched-source
        added-line search for retired path/proof/transport terms; and `git diff
        --check`. Focused coverage now includes serialization of the typed event
        snapshot shape and the graph-session read API. `cargo check` still
        reports the pre-existing `set_active_run_execution_plan` dead-code
        warning.
      - Remaining follow-up: expose this read model through a dedicated
        graph-validation transport/event API after backend lifecycle tests prove
        close cleanup, supersession, stale-result rejection, and bounded
        delivery semantics. Do not reuse execution-run `WorkflowEvent`
        channels or add frontend/Tauri validation policy.
      - 2026-05-28 lifecycle snapshot transport read sub-slice completed:
        workflow-service now exposes the lifecycle event snapshot through a
        service-level graph API, Tauri registers a thin
        `graph_validation_lifecycle_event_snapshot` command, and the frontend
        workflow command service mirrors the backend DTO shape with focused IPC
        forwarding coverage. The command is recovery/read-model plumbing for
        future event-driven graph-editor validation; it is not submit
        authority, does not carry resolver/Pumas/runtime facts, and does not
        reuse execution-run `WorkflowEvent` channels.
      - Verification passed: `cargo fmt -p pantograph-workflow-service`;
        `cargo test -p pantograph-workflow-service
        validation_lifecycle_event_snapshot_reports_backend_refresh_events
        --lib`; `cargo check -p pantograph-workflow-service`; `node
        --experimental-strip-types --test
        src/services/workflow/WorkflowService.commands.test.ts`; `npm run
        typecheck`; and `git diff --check`. `cargo check --manifest-path
        src-tauri/Cargo.toml` is blocked by the pre-existing unrelated
        `set_media_conversion_executor` missing-method error in
        `src-tauri/src/app_setup.rs:96`.
      - Remaining follow-up: add the dedicated graph-validation event emission
        and frontend subscription service on top of this read command, with
        bounded event identity/sequence handling and no frontend validation
        policy.
      - 2026-05-28 lifecycle event sink sub-slice completed:
        workflow-service now exposes a typed
        `WorkflowGraphValidationLifecycleEventSink` and configurable lifecycle
        sink boundary. The lifecycle owner records each event in its bounded
        backend log first, then publishes the cloned typed event to the sink for
        later transport delivery. The sink carries only graph-session identity,
        graph revision, validation-session identity, event sequence, and typed
        lifecycle kind; it does not carry resolver/Pumas/runtime facts, submit
        authority, frontend state, or execution-run events.
      - Verification passed: `cargo fmt -p pantograph-workflow-service`;
        `cargo test -p pantograph-workflow-service
        inference_validation_lifecycle --lib`; `cargo check -p
        pantograph-workflow-service`; added-line source search over the touched
        workflow-service files for retired path fields, raw JSON, execution
        event transports, `Result<T, String>`, and spawned tasks; and `git diff
        --check`. `cargo check` still reports the pre-existing
        `set_active_run_execution_plan` dead-code warning.
      - Remaining follow-up: implement a Tauri lifecycle-event bridge that
        configures this sink at composition time, emits a dedicated
        graph-validation event name, and leaves validation policy in
        workflow-service.
      - 2026-05-28 Tauri lifecycle-event bridge sub-slice completed:
        `app_setup.rs` now configures a transport-only Tauri adapter for the
        workflow-service validation lifecycle event sink. The adapter emits
        dedicated `workflow://graph-validation/lifecycle-event` payloads
        unchanged and logs emit failures; it does not derive validation
        freshness, resolver facts, submit gates, or event ordering.
      - Verification passed: `cargo fmt --manifest-path src-tauri/Cargo.toml`;
        `cargo check -p pantograph-workflow-service`; added-line source search
        over the touched Tauri bridge files for retired path fields, raw JSON,
        execution event transports, `Result<T, String>`, and spawned tasks; and
        `git diff --check`. `cargo check --manifest-path src-tauri/Cargo.toml`
        remains blocked by the pre-existing unrelated
        `set_media_conversion_executor` missing-method error in
        `src-tauri/src/app_setup.rs:96`.
      - Remaining follow-up: add the frontend subscription service for the
        dedicated graph-validation lifecycle event and wire editor-side
        identity/sequence filtering against the backend snapshot read command.
      - 2026-05-28 frontend lifecycle-event subscription slice completed:
        added `WorkflowGraphValidationLifecycleSubscriptionService.ts` for the
        dedicated graph-validation lifecycle event. The helper filters by
        active graph session, drops stale sequence numbers, reports handler
        errors, and leaves initial snapshot/recovery reads to
        `graphValidationLifecycleEventSnapshot`.
      - Discovered and fixed contract issue: the previous TypeScript lifecycle
        snapshot mirror used `dropped_events` and string event kinds, while the
        Rust DTO serializes `dropped_event_count` and internally tagged
        `kind` objects. The frontend types, mock response, and command tests now
        match the Rust serialization contract.
      - Verification passed: `node --experimental-strip-types --test
        src/services/workflow/WorkflowGraphValidationLifecycleSubscriptionService.test.ts
        src/services/workflow/WorkflowService.commands.test.ts`; `npm run
        typecheck`; added-line source search over touched frontend workflow
        service files for retired path fields, raw JSON, execution event
        transports, `Result<T, String>`, and `any`; and `git diff --check`.
      - Remaining follow-up: wire the graph editor to consume the subscription
        and snapshot read model without blocking editing while validation runs.
      - 2026-05-28 toolbar lifecycle-event consumption slice completed:
        `WorkflowToolbar.svelte` now subscribes to dedicated graph-validation
        lifecycle events and refreshes the backend current-validation summary
        only when the event graph-session id and graph revision match the active
        workflow draft key. Editing remains non-blocking, event timing is not
        submit authority, and the toolbar still disables submit from the
        backend-owned submit gate.
      - Verification passed: `node --experimental-strip-types --test
        src/components/workflowToolbarEvents.test.ts
        src/services/workflow/WorkflowGraphValidationLifecycleSubscriptionService.test.ts`;
        `npm run typecheck`; added-line source search over the touched toolbar
        files for retired path fields, raw JSON, `Result<T, String>`, `any`,
        and timer-based polling; and `git diff --check`.
      - Remaining follow-up: render node/port-level drift and pending
        validation overlays from the backend descriptor projection rather than
        only toolbar submit state.
      - 2026-05-28 inference-node validation summary overlay slice completed:
        `workflowValidationProjectionOverlays` now carries the backend
        per-node validation summary beside the authored interface snapshot, and
        `WorkflowToolbar.svelte` clears both runtime overlay keys whenever the
        active graph revision key changes. `LLMInferenceNode.svelte` renders a
        compact display-only status from that backend summary, while submit
        gating, scheduler admission, and runtime execution continue to consume
        backend validation authority directly.
      - No-fallback/no-legacy result: the frontend does not compute descriptor
        validity, infer enqueue permission, inspect Pumas facts, inspect model
        paths, or persist validation summary overlays into saved graph data.
        The overlay is a current-revision presentation fact only.
      - Verification passed: `node --experimental-strip-types --test
        src/components/workflowValidationProjectionOverlays.test.ts
        src/components/nodes/workflow/inferenceValidationDisplay.test.ts`;
        `npm run typecheck`.
      - Remaining follow-up: add port/edge-level drift badges and update-preview
        rendering from backend descriptor/drift records. Do not derive those
        facts in the graph package or make them submit authority.
      - 2026-05-28 authored snapshot drift foundation slice completed:
        graph resolution input extraction now preserves a valid saved
        `node.data.inference_interface_snapshot` as the authored inference
        shape and rejects malformed snapshots with typed
        `invalid_authored_snapshot` diagnostics. Validation publication passes
        that authored snapshot into descriptor projection; if its descriptor
        fingerprint differs from the current resolved descriptor, the backend
        summary is blocked with `drift_requires_review` instead of silently
        replacing authored ports.
      - No-fallback/no-legacy result: absent authored snapshots still bootstrap
        from the current descriptor for first-time model resolution, but present
        stale snapshots are not treated as executable. No graph paths,
        `node.data.definition`, frontend overlays, scheduler facts, or runtime
        payloads become drift authority.
      - Verification passed: `cargo fmt -p pantograph-workflow-service`;
        `cargo test -p pantograph-workflow-service inference_interface_request
        --lib`; `cargo test -p pantograph-workflow-service
        inference_interface_projection --lib`; `cargo check -p
        pantograph-workflow-service` with the pre-existing
        `set_active_run_execution_plan` dead-code warning.
      - Remaining follow-up: produce detailed backend drift reports and typed
        update proposals from the same authored/current comparison, then expose
        them to the graph editor as display/update-preview data.
      - 2026-05-28 backend drift report projection slice completed:
        workflow-service now builds typed `InferenceInterfaceDriftReport`
        values from authored snapshot versus current descriptor comparison,
        attaches the report to node projections and connection surfaces, emits
        the existing node-scoped `DriftReported` validation event, and mirrors
        the optional drift report in frontend workflow DTO/overlay types.
      - No-fallback/no-legacy result: drift reports are backend-authored facts
        only. Frontend overlay code forwards them for display and clears them
        with the graph-revision overlay keys; it does not compare ports,
        produce graph patches, infer submit authority, or inspect Pumas/model
        paths.
      - Verification passed: `cargo fmt -p pantograph-workflow-service`;
        `cargo test -p pantograph-workflow-service
        inference_interface_projection --lib`; `cargo test -p
        pantograph-workflow-service inference_interface_publication --lib`;
        `cargo check -p pantograph-workflow-service` with the pre-existing
        `set_active_run_execution_plan` dead-code warning; `node
        --experimental-strip-types --test
        src/components/workflowValidationProjectionOverlays.test.ts`; `npm run
        typecheck`.
      - Remaining follow-up: generate typed update proposals from the drift
        report and render graph-editor badges/preview UI without introducing a
        frontend-owned patch authority.
      - 2026-05-28 backend update-proposal projection slice completed:
        drifted validation projections now include a backend-authored
        `InferenceInterfaceUpdateProposal` with a
        `ReplaceAuthoredSnapshot` operation for the current descriptor. The
        publication session emits the existing node-scoped `UpdateProposal`
        event, and frontend workflow DTO/runtime-overlay mirrors carry the
        proposal as optional display/preview data.
      - No-fallback/no-legacy result: proposals are produced only from
        workflow-service authored/current projection state and are not applied
        by this slice. Frontend code does not synthesize patches, compare ports,
        mutate graph data, infer submit permission, or use proposal data as
        scheduler/runtime authority.
      - Verification passed: `cargo fmt -p pantograph-workflow-service`;
        `cargo test -p pantograph-workflow-service
        inference_interface_publication --lib`; `cargo check -p
        pantograph-workflow-service` with the pre-existing
        `set_active_run_execution_plan` dead-code warning; `node
        --experimental-strip-types --test
        src/components/workflowValidationProjectionOverlays.test.ts`; `npm run
        typecheck`.
      - Remaining follow-up: render graph-editor badges/update-preview controls
        and wire proposal application through backend graph-patch APIs after
        the UX surface is implemented.
      - 2026-05-28 inference-node drift badge slice completed:
        `LLMInferenceNode.svelte` now renders a compact display-only
        drift/update badge when backend runtime overlays include a drift report
        or update proposal. The presenter prefers backend proposal operation
        counts and falls back to backend drift-change counts.
      - No-fallback/no-legacy result: the node component does not compare ports,
        author graph patches, apply proposal operations, infer submit
        authority, or route drift/proposal data into scheduler/runtime state.
      - Verification passed: `node --experimental-strip-types --test
        src/components/nodes/workflow/inferenceValidationDisplay.test.ts`;
        `npm run typecheck`.
      - Remaining follow-up: implement the full editor-side update-preview
        panel/control and backend-owned proposal application path.
      - 2026-05-28 lifecycle event sink graph-session coverage slice
        completed: added a focused graph-session test proving
        `refresh_current_validation_summary` publishes the same typed pending
        and publication-accepted lifecycle events to the configured sink that
        are retained in the backend lifecycle snapshot.
      - Verification passed: `cargo fmt -p pantograph-workflow-service`;
        `cargo test -p pantograph-workflow-service
        validation_lifecycle_event_sink_receives_backend_refresh_events
        --lib`; `cargo test -p pantograph-workflow-service
        inference_validation_lifecycle --lib`; `cargo check -p
        pantograph-workflow-service`; added-line source search for retired path
        fields, raw JSON, execution event transports, `Result<T, String>`, and
        spawned tasks; and `git diff --check`.
- [x] 2026-05-28 graph-editor update-preview apply re-plan boundary:
  - Discovered issue: the milestone now has backend-authored drift reports,
    update proposals, validation events, and node-level display badges, but no
    backend graph-patch application API exists for applying an
    `InferenceInterfaceUpdateProposal` to the active graph session. The
    existing contract types validate proposal shape, but there is no
    workflow-service command/session method that verifies proposal identity,
    graph revision, validation session, current node state, affected edges, and
    user confirmation before mutating the graph.
  - Why implementation must stop here: building the graph-editor apply button
    now would require either direct frontend mutation of
    `node.data.inference_interface_snapshot` or a new Tauri/frontend command
    whose backend authority, freshness checks, and failure diagnostics are not
    designed. Either path would violate the no-fallback/no-legacy rule and move
    graph patch authority out of workflow-service.
  - Required re-plan: define the backend-owned proposal application boundary
    before implementing the full update-preview/apply UX. The design must cover
    request/response DTOs, graph-session id and graph revision checks,
    validation-session/proposal id checks, revalidation of current descriptor
    fingerprint, affected edge/literal handling, confirmation for destructive
    operations, typed diagnostics for stale/missing/rejected proposals, Tauri as
    transport only, and frontend behavior that displays a preview without
    blocking editing or deriving patch operations locally.
  - No-fallback/no-legacy confirmation: until that boundary is planned and
    implemented, the frontend may display backend drift/proposal facts only. It
    must not apply proposal operations itself, rewrite snapshots directly,
    remove edges, clear literals, or treat proposal presence as submit
    authority.
  - Re-plan decision: implement option 2 first, with option 4 as the target
    UX. The next production slice must add a minimal backend-owned proposal
    application API that supports only non-destructive
    `ReplaceAuthoredSnapshot` operations. The request must carry typed graph
    session id, current graph revision, validation session id, node id,
    proposal id, current descriptor fingerprint, and explicit user
    confirmation. Workflow-service must reject missing, stale, mismatched,
    destructive, multi-operation, edge-removal, literal-clearing, wrong-node, or
    wrong-descriptor proposals with typed diagnostics. Successful application
    mutates only the authored inference snapshot for the target node through
    graph-session mutation ownership, cancels/supersedes stale validation for
    the changed graph revision, and returns the updated graph/revision through
    existing graph mutation response patterns.
  - Implementation order for option 2:
    1. Add workflow-service request/response DTOs and focused validation tests
       for `ReplaceAuthoredSnapshot` only.
    2. Add the graph-session apply method that revalidates current validation
       state/proposal identity before mutating the graph.
    3. Add a thin Tauri command that only decodes/forwards/encodes the typed
       request/response.
    4. Add frontend service plumbing and a minimal Apply action that calls the
       backend API using backend proposal ids and never constructs patch
       operations locally.
    5. Keep destructive `RemoveInvalidEdge` and `ClearInvalidLiteral` proposal
       application disabled with typed diagnostics until the full option 4
       preview/confirmation UX is implemented.
  - Target option 4: after the minimal non-destructive apply path is validated,
    add a full backend preview/apply workflow that returns a dedicated preview
    model for all operation kinds, lets the graph editor display diffs while
    editing remains unblocked, requires explicit confirmation for destructive
    operations, and applies proposals only through workflow-service graph-patch
    APIs.
- [x] 2026-05-28 option-2 apply contract DTO slice:
  - Smallest useful vertical slice: add workflow-service request/response DTOs
    and focused validation tests for applying a backend-authored inference
    interface update proposal only when it is a single non-destructive
    `ReplaceAuthoredSnapshot` operation.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/graph/inference_interface_patch.rs`,
    `crates/pantograph-workflow-service/src/graph/mod.rs`,
    `crates/pantograph-workflow-service/src/lib.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this Milestone 5d
    file, and `05-execution-management.md`.
  - No-fallback/no-legacy result: the new request parses graph session id,
    graph revision, validation session id, node id, proposal id, descriptor
    fingerprint, and explicit confirmation into typed boundary DTOs. Contract
    validation rejects unconfirmed, mismatched, destructive, multi-operation,
    edge-removal, and literal-clearing proposal shapes instead of applying or
    interpreting them locally. No graph mutation, Tauri command, frontend apply
    logic, compatibility shim, or stale proposal fallback was added.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
    test -p pantograph-workflow-service inference_interface_patch --lib`;
    `cargo check -p pantograph-workflow-service`; `git diff --check`.
  - Deviation/discovered issue: `cargo check -p pantograph-workflow-service`
    still reports the pre-existing dead-code warning for
    `set_active_run_execution_plan`; it is outside this DTO contract slice.
  - Remaining follow-up: implement the graph-session apply method that re-reads
    current validation state, proves proposal freshness/identity against the
    active graph revision and validation session, mutates only the authored
    snapshot, and cancels/supersedes stale validation for the changed graph.
- [x] 2026-05-28 option-2 backend apply method slice:
  - Smallest useful vertical slice: add the workflow-service graph-session
    apply method for the first supported inference-interface update proposal
    shape and expose it through the workflow graph facade for later
    transport-only Tauri wiring.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/graph/inference_validation_state.rs`,
    `crates/pantograph-workflow-service/src/graph/session_inference_validation_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`,
    `crates/pantograph-workflow-service/src/workflow/graph_api.rs`,
    `crates/pantograph-workflow-service/src/README.md`, this Milestone 5d
    file, and `05-execution-management.md`.
  - No-fallback/no-legacy result: the apply path re-reads current validation
    proposal state by typed graph session, graph revision, validation session,
    node id, proposal id, and descriptor fingerprint; then it rechecks the
    live graph revision under the session lock before mutating. It applies only
    `node.data.inference_interface_snapshot`, returns the canonical session
    graph mutation response, and cancels active validation through the existing
    lifecycle owner. It does not remove edges, clear literals, apply frontend
    patches, preserve stale proposals, or introduce Tauri business logic.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
    test -p pantograph-workflow-service
    apply_inference_interface_update_proposal --lib`; `cargo check -p
    pantograph-workflow-service`; `git diff --check`.
  - Deviation/discovered issue: crate check still reports the pre-existing
    `set_active_run_execution_plan` dead-code warning; it is unrelated to this
    backend apply slice.
  - Remaining follow-up: add a thin Tauri command that decodes the typed apply
    request, delegates to `WorkflowService`, and encodes the backend graph
    mutation response without owning proposal validation or graph patch logic.
- [x] 2026-05-28 option-2 Tauri apply command slice:
  - Smallest useful vertical slice: expose the backend-owned inference
    interface proposal apply API through a Tauri command and register it for
    frontend invocation.
  - Allowed files touched:
    `src-tauri/src/workflow/workflow_edit_session.rs`,
    `src-tauri/src/workflow/workflow_execution_commands.rs`,
    `src-tauri/src/workflow/workflow_execution_tauri_commands.rs`,
    `src-tauri/src/app_setup.rs`, `src-tauri/src/workflow/README.md`, this
    Milestone 5d file, and `05-execution-management.md`.
  - No-fallback/no-legacy result: Tauri only decodes the typed
    `InferenceInterfaceApplyProposalRequest`, forwards it to
    `WorkflowService::workflow_graph_apply_inference_interface_update_proposal`,
    and returns the backend graph mutation response. It does not validate
    proposal identity, construct patch operations, rewrite node data directly,
    remove edges, clear literals, compare ports, infer submit authority, or add
    a compatibility path for stale proposals.
  - Verification passed: `cargo fmt --manifest-path src-tauri/Cargo.toml`;
    `git diff --check`.
  - Verification deviation: `cargo check --manifest-path src-tauri/Cargo.toml`
    remains blocked by the pre-existing unrelated missing
    `set_media_conversion_executor` method on `Arc<WorkflowService>` in
    `src-tauri/src/app_setup.rs:96`. Before that blocker, Cargo checked
    `pantograph-workflow-service` and emitted only the known
    `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: add frontend service plumbing and a minimal Apply
    action that calls this command with backend proposal ids and explicit
    confirmation, without constructing proposal patch operations locally.
- [x] 2026-05-28 option-2 frontend apply action slice:
  - Smallest useful vertical slice: add frontend service plumbing, graph-level
    coordination, and a minimal inference-node Apply action for backend-owned
    non-destructive `ReplaceAuthoredSnapshot` proposals.
  - Allowed files touched: `src/services/workflow/types.ts`,
    `src/services/workflow/WorkflowGraphMutationService.ts`,
    `src/services/workflow/WorkflowService.commands.test.ts`,
    `src/services/workflow/README.md`,
    `src/components/inferenceInterfaceUpdateContext.ts`,
    `src/components/workflowGraphBackendActions.ts`,
    `src/components/WorkflowGraph.svelte`, `src/components/README.md`,
    `src/components/nodes/workflow/LLMInferenceNode.svelte`,
    `src/components/nodes/workflow/inferenceValidationDisplay.ts`,
    `src/components/nodes/workflow/inferenceValidationDisplay.test.ts`,
    `src/components/nodes/workflow/README.md`, this Milestone 5d file, and
    `05-execution-management.md`.
  - No-fallback/no-legacy result: the frontend forwards backend proposal ids,
    graph session/revision identity, backend validation-session identity,
    descriptor fingerprint, and explicit confirmation to the backend apply
    command. It does not construct graph patch operations, compare ports,
    rewrite authored snapshots directly, remove edges, clear literals, infer
    submit authority, or apply stale proposals. Unsupported destructive or
    multi-operation proposal application remains disabled for the future
    option-4 preview/confirmation workflow.
  - Verification passed: `node --experimental-strip-types --test
    src/components/nodes/workflow/inferenceValidationDisplay.test.ts
    src/services/workflow/WorkflowService.commands.test.ts`; `npm run
    typecheck`; `git diff --check`.
  - Remaining follow-up: build the option-4 full update-preview/confirmation UX
    for destructive operations and richer diffs, still applying through
    workflow-service graph-patch APIs only.
- [x] 2026-05-28 staged node-local update preview slice:
  - Smallest useful vertical slice: render a compact inference-node update
    preview from backend drift messages and backend proposal operation counts.
  - Allowed files touched:
    `src/components/nodes/workflow/LLMInferenceNode.svelte`,
    `src/components/nodes/workflow/inferenceValidationDisplay.ts`,
    `src/components/nodes/workflow/inferenceValidationDisplay.test.ts`,
    `src/components/nodes/workflow/README.md`, this Milestone 5d file, and
    `05-execution-management.md`.
  - No-fallback/no-legacy result: the preview consumes backend-authored drift
    report rows and backend-authored proposal operation counts only. It does
    not compare ports, construct graph patch operations, rewrite graph data,
    remove edges, clear literals, infer submit authority, or enable destructive
    proposal application.
  - Verification passed: `node --experimental-strip-types --test
    src/components/nodes/workflow/inferenceValidationDisplay.test.ts`; `npm
    run typecheck`; `git diff --check`.
  - Remaining follow-up: add graph-level edge/port visual overlays and the
    richer option-4 preview/confirmation workflow for destructive operations.
- [x] Wire staged graph editor drift presentation and update preview.
      Option-2 scope: the editor shows authored-current diffs visually on
      nodes/ports/edges, keeps invalid edges visible, displays
      backend-authored proposal data, and calls the backend apply API only for
      supported `ReplaceAuthoredSnapshot` proposals. Unsupported destructive
      operations remain visible as typed diagnostics and disabled actions until
      the option-4 preview/confirmation workflow exists. The editor must never
      construct patch operations locally, rewrite snapshots directly, remove
      edges, clear literals, compare ports for authority, or treat proposal
      presence as submit permission.
  - 2026-05-28: Completed the staged graph-level drift overlay slice.
  - Smallest useful vertical slice: project backend-authored affected-edge
    facts into transient graph edge data and node-handle classes so the editor
    can visibly mark affected edges and ports without changing the saved graph.
  - Allowed files touched: `src/components/workflowInferenceDriftEdgeOverlays.ts`,
    `src/components/workflowInferenceDriftEdgeOverlays.test.ts`,
    `src/components/WorkflowGraph.svelte`,
    `src/components/edges/ReconnectableEdge.svelte`,
    `src/components/nodes/BaseNode.svelte`, `src/components/README.md`, this
    Milestone 5d file, and `05-execution-management.md`.
  - No-fallback/no-legacy result: the overlay consumes only backend-authored
    `affected_edges` and `remove_invalid_edge` operation facts. It keeps
    invalid edges visible, does not compare ports, does not construct patch
    operations, does not mutate saved edge/node data, does not remove edges,
    does not clear literals, and does not affect submit permission.
  - Verification passed: `node --experimental-strip-types --test
    src/components/workflowInferenceDriftEdgeOverlays.test.ts
    src/components/nodes/workflow/inferenceValidationDisplay.test.ts`; `npm run
    typecheck`.
  - Remaining follow-up: option-4 full preview/confirmation UX must still add a
    richer destructive-operation review flow before destructive proposals can
    be applied.
- [x] Wire `PortOptionsProvider`, selection-input, and option-cache consumers
      only as descriptor-backed presentation plumbing where reused. Typed
      option identity, defaults, availability, and diagnostics remain owned by
      `pantograph-inference-interface-contracts`, and cache keys/invalidation
      must include descriptor fingerprint or validation session/revision.
  - 2026-05-28: Completed the descriptor-aware selection-input option query
    slice.
  - Smallest useful vertical slice: carry backend validation descriptor
    fingerprint through `SelectionInputNode` provider queries and the shared
    port-options cache key while preserving backend-owned option authority.
  - Allowed files touched:
    `src/services/workflow/types.ts`,
    `src/services/workflow/portOptionsCache.test.ts`,
    `src/services/workflow/README.md`,
    `src/components/nodes/workflow/selectionInputProviderOptions.ts`,
    `src/components/nodes/workflow/selectionInputProviderOptions.test.ts`,
    `src/components/nodes/workflow/README.md`, this Milestone 5d file, and
    `05-execution-management.md`.
  - No-fallback/no-legacy result: selection-input and option-cache code pass
    stable descriptor/model/runtime context references to backend-owned option
    providers only. They do not compute valid options, defaults, availability,
    diagnostics, inference settings, runtime support, or submit authority
    locally.
  - Verification passed: `node --experimental-strip-types --test
    src/components/nodes/workflow/selectionInputProviderOptions.test.ts
    src/services/workflow/portOptionsCache.test.ts`; `npm run typecheck`.
- [x] 2026-05-29 Draft Save / Executable Publish boundary re-plan decision:
  - Re-plan trigger: the previous "workflow save validation" item conflated
    draft graph persistence with executable publish/submit authority. That
    would either block users from saving invalid in-progress graphs or make
    save imply scheduler readiness, both of which violate the graph editor UX
    and backend-owned execution authority boundaries.
  - Decision: use an explicit Draft Save and Executable Publish boundary.
    Draft Save persists editable graph state, authored port history, and
    versioned graph data for later editing. It may save invalid or
    non-executable inference graphs and must not create, mutate, infer, or
    cache executable validation snapshots, scheduler projections, queue
    admission state, or submit authority.
  - Executable Publish / Submit / Queue Admission rule: required inputs,
    optional defaults, valid options, connected upstream output types, explicit
    runtime/device/trait constraints, validation-session identity, semantic
    graph revision, descriptor fingerprint, and backend validation summary must
    be current and executable before a workflow version can be published as
    executable, submitted, inserted into the queue, recorded as queue-placed, or
    materialized into a scheduler task graph.
  - Frontend rule: the graph editor may display and edit invalid graphs while
    validation is pending, stale, unavailable, or blocked. Submit/enqueue
    remains disabled from backend-owned validation/publish state; frontend
    code must not synthesize executable authority from saved graph JSON,
    proposal presence, descriptors, options, or diagnostics.
  - No-fallback/no-legacy confirmation: do not preserve any alternate
    successful route through draft save, raw graph inference fields, current
    frontend state, Tauri payloads, legacy `inference_settings`,
    `expand-settings`, static port tables, or whole-run runtime execution.
    Invalid executable state returns typed diagnostics; invalid draft state can
    still be persisted as editable graph history.
- [x] Wire Draft Save persistence to remain non-executable. Draft Save may
      validate persisted graph shape, schema version, authored snapshot/history
      integrity, and canonical graph serialization only. Tests must prove an
      invalid inference graph can be saved for continued editing without
      producing executable validation snapshots, scheduler projections, queue
      admission state, or submit permission.
  - 2026-05-29: Completed the Draft Save non-executable persistence slice.
  - Smallest useful vertical slice: add workflow-service graph persistence
    regression coverage proving an invalid generic inference draft can be saved
    for later editing through the save API without emitting executable
    validation snapshots, scheduler projections, queue-admission state, or
    submit-gate authority.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/graph/persistence_tests.rs`, this
    Milestone 5d file, and `05-execution-management.md`.
  - No-fallback/no-legacy result: Draft Save remains graph persistence only.
    The test does not add a compatibility branch, validation bypass,
    executable snapshot synthesis, scheduler projection fallback, frontend
    submit authority, or queue-admission side effect.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    draft_save_persists_invalid_inference_graph_without_executable_authority
    --lib`; `cargo test -p pantograph-workflow-service persistence --lib`;
    `cargo check -p pantograph-workflow-service`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` still emits
    the pre-existing `set_active_run_execution_plan` dead-code warning outside
    this slice.
- [x] Wire Executable Publish validation to consume the same descriptor
      contract. Required inputs, optional defaults, valid options, connected
      upstream output types, explicit runtime/device/trait constraints,
      validation session identity, semantic graph revision, descriptor
      fingerprint, and backend validation summary must be validated before a
      workflow version can be published, submitted, or admitted as executable.
  - 2026-05-29: Completed the Executable Publish validation regression slice.
  - Smallest useful vertical slice: add workflow-service coverage for the
    public graph-session executable publish API proving a current but
    non-executable inference validation summary fails closed and cannot produce
    a durable executable validation snapshot.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/workflow/tests/workflow_version.rs`,
    this Milestone 5d file, and `05-execution-management.md`.
  - No-fallback/no-legacy result: Executable Publish consumes backend-owned
    graph-session validation state, validation-session identity, semantic graph
    revision, descriptor summary, and submit gate state. It does not publish
    from draft save, frontend state, raw graph fields, runtime defaults,
    scheduler projections, Tauri payloads, or the retired caller-supplied
    runtime publication path.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_graph_session_executable_validation_snapshot_rejects_non_executable_summary
    --lib`; `cargo test -p pantograph-workflow-service
    workflow_executable_validation_snapshot --lib`; `cargo test -p
    pantograph-workflow-service executable_validation_snapshot --lib`; `cargo
    check -p pantograph-workflow-service`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` still emits
    the pre-existing `set_active_run_execution_plan` dead-code warning outside
    this slice.
- [x] Wire workflow submit and scheduler queue admission to consume the
      backend validation summary. The frontend submit button and backend queue
      admission must both fail closed while inference validation is pending,
      stale, unavailable, unresolved, or blocked; raw diagnostics must not be
      the enqueue authority. Backend admission must run before queue insertion,
      queue-placement diagnostic event recording, and scheduler task graph
      materialization so non-executable inference graphs never become queued
      runs that are later canceled.
  - 2026-05-29: Completed the submit/queue-admission fail-closed regression
    slice.
  - Smallest useful vertical slice: strengthen the runtime-inference saved-run
    admission regression so a runtime inference graph without a saved
    executable validation snapshot fails closed with the canonical snapshot
    diagnostic, leaves the session queue empty, and does not invoke runtime
    load or retired whole-run host execution.
  - Allowed files touched:
    `crates/pantograph-workflow-service/src/workflow/attribution_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
    this Milestone 5d file, and `05-execution-management.md`.
  - No-fallback/no-legacy result: saved-run queue admission now exposes the
    missing executable snapshot as the typed backend gate instead of a generic
    attribution not-found error. Runtime inference admission remains blocked
    before queue insertion, queue-placement event recording, scheduler state
    mutation, runtime load, or whole-run host execution when executable
    validation authority is missing.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    workflow_execution_session_runtime_run_fails_closed_before_legacy_launch
    --lib`; `cargo test -p pantograph-workflow-service
    workflow_executable_validation_snapshot --lib`; `cargo check -p
    pantograph-workflow-service`.
  - Verification blocked: `cargo test -p pantograph-workflow-service
    session_execution --lib` still fails in legacy/runtime session cases that
    expect whole-run runtime behavior, non-source input handling, or
    runtime-ready errors before scheduler task/input materialization. These are
    the existing broader session-execution cleanup targets; this slice did not
    add a compatibility path to keep them green.
  - Discovered issue: `cargo check -p pantograph-workflow-service` still emits
    the pre-existing `set_active_run_execution_plan` dead-code warning outside
    this slice.
- [x] Confirm scheduler task graph projection consumes descriptor-backed
      validation state and stores path-free model refs, task kind, typed
      constraints, descriptor fingerprint, and bindings only. Projection must
      consume the current descriptor-backed validation state for resolved task
      kind, descriptor fingerprint, and validated constraints; raw graph
      `task_kind`, runtime, device, or trait JSON are resolver inputs only and
      must not be parsed again as execution authority.
  - 2026-05-29 decision: selected option 1. Descriptor-backed scheduler
    projection is complete in Milestone 5d; final runtime-host input
    materialization is split out to the runtime-dispatch milestone because it
    requires scheduler input readiness plus a dispatch-selected runtime-host
    handoff. This preserves the current typed fail-closed behavior until that
    handoff exists.
  - 2026-05-29 re-plan boundary: descriptor-backed scheduler projection is
    already implemented and covered by task-graph/session projection tests, but
    final runtime-host input materialization cannot be completed inside this
    checklist item while runtime scheduler dispatch still intentionally fails
    closed with `runtime scheduler dispatch is not wired`. Continuing directly
    would require either pretending materialization happened before a
    dispatch-selected runtime-host handoff exists, reviving the retired
    whole-run runtime path, or parsing raw graph inference fields as execution
    authority. All three violate the no-fallback/no-legacy rule.
  - Required re-plan decision before implementation continues:
    1. Split this item into two milestones: mark descriptor-backed scheduler
       projection complete here, then move runtime-host input materialization
       to the runtime-dispatch milestone that wires selected scheduler handoff
       into runtime-host execution. This is the recommended path because it
       preserves current fail-closed behavior and keeps ownership clear.
    2. Implement a materialization-only pure domain API now that accepts
       scheduler-selected handoff, upstream task results, graph literals, and
       descriptor defaults, but leave runtime dispatch unwired. This can be a
       valid intermediate slice if the API is fully typed and has no runtime
       execution side effects, but it expands the current milestone beyond the
       queue/admission gate.
    3. Wire runtime-host dispatch now and complete materialization end to end
       in this milestone. This may be the target architecture, but it widens
       the blast radius into scheduler runtime selection, runtime-host
       contracts, task result progression, resource reservations, and adapter
       tests.
    4. Re-enable or preserve whole-run runtime execution until materialization
       exists. This is rejected because it preserves retired legacy behavior
       and masks the missing scheduler-owned runtime-host handoff.
  - No-fallback/no-legacy gate: implementation must not continue by deriving
    runtime-host inputs from raw graph `task_kind`, runtime/device strings,
    `inference_settings`, model paths, current frontend state, Tauri payloads,
    or the old whole-run host execution path. Until the runtime-dispatch slice
    wires scheduler-owned materialization and host handoff, runtime inference
    dispatch remains fail-closed with typed diagnostics.
- [x] Move final runtime-host input materialization, runtime-host input
      contract alignment, and pre-dispatch descriptor revalidation to the
      runtime-dispatch milestone. Those tasks require scheduler input readiness
      and dispatch-selected runtime-host handoff ownership, so Milestone 5d must
      not implement them locally or provide compatibility paths.
  - 2026-05-29 decision: runtime-host execution requests must eventually
    include scheduler-selected handoff plus typed, path-free materialized inputs
    and artifact/result references, but the owning implementation slice is the
    runtime-dispatch milestone. Pre-dispatch descriptor revalidation must run
    before scheduler dispatch selection there; if Pumas facts, selected artifact
    state, runtime capability, or descriptor fingerprint changed since
    executable publish/admission validation, the task fails with typed
    diagnostics rather than falling back to stale ports.
- [ ] Delete or rewrite retired inference-node port discovery and validation
      surfaces replaced by this resolver. Do not keep compatibility aliases,
      legacy model-path support checks, planned-inference validation branches,
      or image-only graph-editor port tables as successful paths.
      - 2026-06-03 connection-intent retired expand-settings fixture slice
        completed: removed the positive `expand-settings` dynamic
        `inference_settings` fixture and tests from
        `crates/pantograph-workflow-service/src/graph/connection_intent.rs`.
        Descriptor-backed inference connection-surface tests remain the
        successful inference connection path.
      - No-fallback/no-legacy gate: the slice deletes the successful
        `expand-settings.temperature` candidate/commit proof instead of
        adapting it to a hidden settings edge or static inference port. The
        connection-intent boundary now has no `expand-settings` or
        `inference_settings` source hits.
      - Verification passed: `cargo fmt -p pantograph-workflow-service --
        --check`; `cargo test -p pantograph-workflow-service connection_
        --lib`; `cargo check -p pantograph-workflow-service`; targeted
        `connection_intent.rs` search for `expand-settings`,
        `inference_settings`, and dynamic expand wording.
- [x] Delete or rewrite retired `inference_settings` JSON canonicalization and
      `expand-settings` frontend-owned dynamic port sync. If any of that UI
      remains, it must consume backend descriptors as presentation plumbing and
      cannot remain an alternate inference-interface source.
      - 2026-06-03 implementation slice completed: removed the dead
        `syncInferencePorts` compatibility no-op from
        `packages/svelte-graph/src/stores/createWorkflowStores.ts`, removed
        the app-level `src/stores/workflowStore.ts` re-export, deleted the
        private `InferenceParamSchema` type used only by that no-op, and
        updated the stores README to describe backend-authored
        `inference_interface_snapshot` display projection instead of
        frontend-owned dynamic settings-port synchronization.
      - No-fallback/no-legacy gate: no replacement sync entry point,
        compatibility no-op, settings schema type, or frontend-owned dynamic
        port builder remains in the touched store boundary.
      - Verification passed: `node --experimental-strip-types --test
        packages/svelte-graph/src/stores/createWorkflowStores.test.ts
        packages/svelte-graph/src/stores/createSessionStores.test.ts
        src/stores/workbenchStore.test.ts`; `npm run typecheck`; targeted
        store search for `syncInferencePorts`, `InferenceParamSchema`,
        `inferenceSettingsPorts`, and retired settings-source wording; `git
        diff --check`.
      - 2026-06-03 workflow-service canonicalization deletion slice completed:
        removed the retired dynamic inference-settings canonicalizer module
        from `crates/pantograph-workflow-service/src/graph`, deleted its
        `node.data.inference_settings` parser, definition-overlay synthesis,
        `expand-settings` passthrough builder, and dynamic parameter-edge
        creation path, and updated graph README ownership notes.
      - No-fallback/no-legacy gate: graph canonicalization now keeps only the
        unrelated `llm-inference.stream -> text-output.stream` edge repair and
        no longer treats saved graph settings fields or `expand-settings` edges
        as successful inference-interface sources. The focused regression test
        asserts retired settings edges do not synthesize descriptor ports or
        parameter edges.
      - Verification passed: `cargo fmt -p pantograph-workflow-service --
        --check`; `cargo test -p pantograph-workflow-service
        canonicalize_workflow_graph --lib`; `cargo check -p
        pantograph-workflow-service`; targeted workflow-service source search
        for the deleted canonicalizer APIs; targeted touched-file search
        classifying remaining retired settings hits as negative regression
        fixture text only; `git diff --check`.
- [x] Delete or rewrite retired `puma-lib` path/readiness/inference-settings
      authoring surfaces after the model-ref-only slice. This includes
      `modelPath`/`model_path`/`entry_path` graph data, option metadata,
      node-engine `puma-lib` output semantics, frontend mocks, templates, and
      tests that still treat paths or `inference_settings` as successful
      inference-interface inputs.
      - 2026-06-03 search-gate reconciliation completed: targeted source
        review found no successful `puma-lib` graph-authoring output for
        `modelPath`, `model_path`, `entry_path`, or `inference_settings`.
        `workflow-nodes` descriptor tests assert those ports are absent,
        Tauri hydration tests assert selected node data drops retired metadata,
        frontend mock/template/component tests are negative guards, and
        remaining `entry_path` strings in `puma-lib` are Pumas selector fixture
        inputs or non-authoring `display_*` evidence metadata.
      - No-fallback/no-legacy gate: `display_entry_path`,
        `display_indexed_path`, `entry_path_state`, `artifact_state`, and
        `selector_row_executable` remain selector-row evidence for user
        selection/diagnostics only. They are not graph data, dependency action
        inputs, inference descriptor inputs, scheduler handoff fields, or
        runtime load-target authority, and they must not be forwarded into
        `llm-inference` or dependency-environment requests.
      - Verification passed: targeted source search over `puma-lib` Rust,
        Tauri `puma_lib_commands`, frontend puma node state, app/package mocks,
        and starter template tests for `modelPath`, `model_path`,
        `entry_path`, and `inference_settings`; classification recorded here
        instead of deleting display-only evidence.
- [x] Decide the first implementation slice for existing `PortOptionsProvider`,
      selection-input, and option-cache reuse versus a dedicated descriptor
      option renderer. The decision must preserve typed descriptor ownership
      and avoid duplicating backend semantics.
      - 2026-06-03 decision: keep `PortOptionsProvider` and
        `pumaModelOptionsCache` as the Pumas model-selection surface for
        `puma-lib.pumas_model_ref` only. Do not use that provider/cache as the
        inference descriptor option renderer, and do not add provider-backed
        denoising/runtime/device option fallbacks.
      - First implementation slice when descriptor option rendering is needed:
        render typed `InferencePortOptions` from the current backend-authored
        `inference_interface_snapshot`/validation summary at the graph-node
        presentation boundary. If the descriptor snapshot is missing, stale, or
        unavailable, render typed diagnostics/disabled controls from validation
        state rather than querying Pumas or synthesizing option rows in the
        frontend.
      - Standards alignment: this keeps Pumas package selection, descriptor
        validation, frontend presentation, and option caching as separate
        concerns. It avoids complecting model-library selector rows with
        inference-interface semantics and preserves one backend owner for
        model-specific option meaning.
      - Verification passed: source review of `PortOptionsProvider`,
        `pumaModelOptionsCache`, `selectionInputProviderOptions`, descriptor
        snapshot overlays, and inference-interface contract option DTOs.
- [x] Add a contract-crate decomposition gate before adding more shared DTO
      families. `pantograph-inference-interface-contracts/src/lib.rs` is already
      large enough that validation publication records, node projection DTOs, or
      additional dependency-action contracts must move into focused modules
      unless the change is a small addition to an existing type. Update the crate
      README and dependency-direction checks in the same slice.
  - 2026-05-29 implementation slice: moved shared validation primitives,
    contract-version checks, bounded validation helpers, validated identifier
    newtypes, and the crate error type from the crate root into private
    `validation.rs`; kept the public API curated through `lib.rs` re-exports;
    updated the crate README with module ownership. This preserves existing
    wire DTO behavior and adds no new dependencies or fallback paths.
  - Verification passed: `cargo fmt -p
    pantograph-inference-interface-contracts -- --check`; `cargo test -p
    pantograph-inference-interface-contracts`; `cargo check -p
    pantograph-inference-interface-contracts`; `cargo check -p
    pantograph-inference-interface-contracts --all-features`; `cargo check -p
    pantograph-inference-interface-contracts --no-default-features`.
- [ ] Keep the legacy deletion searches as implementation gates, not cleanup
      notes. The first acceptance slice must prove it does not invoke
      `inference_settings`, `expand-settings`, static all-port inference
      metadata, frontend `modelPath` dependency actions, `ModelDependencyRequest`,
      `ModelRefV2`, or model-path-derived node-engine inference paths. Remaining
      occurrences must be classified as unrelated non-workflow configuration,
      test fixtures being rewritten, or deletion targets before the milestone can
      claim no-legacy completion.
  - 2026-06-03 frontend direct-sidecar exposure slice completed: removed the
    unused `LLMService.startSidecar(modelPath, mmprojPath)` frontend API and
    the stale architecture graph command node/edges for `start_sidecar_llm`.
    The active frontend/config surface no longer exposes a camelCase
    model-path direct sidecar start path.
  - No-fallback/no-legacy gate: the slice deletes the unused path-shaped
    frontend entry point and does not translate it to another command,
    synthesize model refs, or add a compatibility shim. Existing backend-owned
    configured start modes remain the frontend runtime lifecycle path.
  - Verification passed: `npm run typecheck`; targeted frontend/config search
    for `startSidecar`, `start_sidecar_llm`, `modelPath`, and `mmprojPath`;
    `git diff --check`.
  - Discovered follow-up: `src-tauri/src/llm/commands/server.rs` still defines
    and `src-tauri/src/app_setup.rs` still registers the path-shaped
    `start_sidecar_llm` command. It is no longer referenced by frontend source
    after this slice, but removing it is a separate Tauri/backend command slice
    that must verify no command registration or direct path-shaped runtime
    startup remains.
  - 2026-06-03 Tauri direct-sidecar command removal slice completed: removed
    the unreferenced `start_sidecar_llm` Tauri command, removed its
    `app_setup.rs` registration, and deleted the now-dead explicit llama.cpp
    path request helper/test from `src-tauri/src/llm/startup.rs`.
  - No-fallback/no-legacy gate: the slice deletes the direct model-path runtime
    startup command and helper rather than adapting them to `ModelRefV2`,
    graph paths, or a compatibility command. Configured inference/embedding
    startup remains available through the existing config/gateway path.
  - Verification passed: `cargo fmt --manifest-path src-tauri/Cargo.toml --
    --check`; `cargo test --manifest-path src-tauri/Cargo.toml llm::startup`;
    `cargo check --manifest-path src-tauri/Cargo.toml`; targeted source search
    for `start_sidecar_llm` and `build_explicit_llamacpp_inference_request`;
    targeted frontend/config search for `modelPath`, `startSidecar`, and
    `mmprojPath`; `git diff --check`.
  - Verification caveat: Tauri Rust checks still emit pre-existing dead-code
    warnings in workflow diagnostics/event/runtime modules. They are not caused
    by this command deletion and remain separate cleanup work.
  - 2026-05-29 re-plan boundary: the search gate found multiple distinct
    retired surfaces with different owners, so a single deletion slice would
    cross too much code:
    - Graph-visible deletion targets: `workflow-nodes` previously registered
      `expand-settings` and exposed `model-provider` `model_path`/
      `inference_settings` ports; both have now been removed in separate
      validated slices. Legacy processing nodes `audio-generation`,
      `onnx-inference`, and `depth-estimation` have now been removed as
      graph-visible descriptors/components in a separate validated slice. The
      C# native smoke workflow has now been rewritten to the canonical
      path-free fixture shape.
    - Already-modernized but still visible as review context: `puma-lib`
      descriptor output is model-ref-only, and existing tests assert it does not
      expose `model_path` or `inference_settings`, but its option provider still
      contains package-entry-path and display metadata that must be classified
      before no-legacy completion.
      - 2026-05-29 implementation slice completed: removed the selector option
        metadata `inference_settings` stub and its fallback helper/test from
        `puma-lib`. Selector option values remain typed `pumas_model_ref`
        payloads; option metadata may still carry display/debug paths,
        readiness state, storage/validation state, package summaries, and the
        selector cursor, but those fields are documented as non-executable
        evidence only and are not graph-authoring authority.
      - No-fallback/no-legacy gate: no replacement settings source or
        compatibility metadata key was added. Backend inference-interface
        descriptors remain the only owner for model-specific inference option
        ports/settings.
      - Verification passed: `cargo fmt -p workflow-nodes -- --check`; `cargo
        test -p workflow-nodes puma_lib --lib --features model-library`;
        `cargo check -p workflow-nodes --features model-library`; `cargo check
        -p workflow-nodes --no-default-features`; targeted source search for
        `resolve_inference_settings_fallback` and `inference_settings` in
        `puma_lib.rs`/input README.
      - 2026-05-29 implementation slice completed: removed
        `inference_settings` from `PumasSelectedModelDetail`, stopped owner and
        local-client selected detail from calling
        `get_inference_settings_batch`, and removed embedded-runtime
        `puma-lib` projection of saved or selected-detail inference settings.
        Selected detail now hydrates model reference, descriptor, and package
        summary evidence only.
      - No-fallback/no-legacy gate: no saved-node `inference_settings` input,
        Pumas batch setting, or embedded-runtime output remains a successful
        `puma-lib` settings path in the touched owner files. Backend
        inference-interface descriptors remain the sole settings/default owner.
      - Verification passed: `cargo fmt -p workflow-nodes -p
        pantograph-embedded-runtime -- --check`; `cargo test -p
        workflow-nodes selected_model_detail --lib --features model-library`;
        `cargo test -p pantograph-embedded-runtime puma_lib_execution --lib`;
        `cargo check -p workflow-nodes --features model-library`; `cargo check
        -p pantograph-embedded-runtime`; targeted source search over the
        touched setup/embedded-runtime files for the removed selected-detail
        settings path.
      - Verification caveat: `cargo check -p pantograph` now reaches past this
        API change. At the time of this slice it still failed in
        `src-tauri/src/app_setup.rs` because `WorkflowService` did not expose
        `set_media_conversion_executor`; the stale injection has since been
        removed. The remaining media-conversion cleanup is to delete the
        unused Tauri managed media conversion adapter and keep any future
        conversion feature behind a backend-owned service boundary rather than
        desktop-owned business logic.
    - Not Milestone 5d deletion targets: `crates/inference` backend and worker
      `model_path`/`entry_path` fields are runtime/load-target internals owned
      by runtime-host dispatch and Pumas artifact resolution, not graph
      authoring. They must not be deleted as part of frontend/node registry
      cleanup.
    - 2026-05-31 update: the Tauri-side Pumas dependency runtime probe was
      retired in Milestone 5b because it still used `ModelDependencyRequest`
      and placed dependency/runtime business logic in the Tauri crate.
  - Options before implementation continues:
    1. Delete `expand-settings` first as a narrow registry slice, update the
       registry inventory count/tests, and remove its frontend-owned dynamic
       port source. This is the recommended first deletion because it is a
       direct alternate inference-interface source.
       - 2026-05-29 implementation slice completed: removed the
         `workflow-nodes` `expand-settings` descriptor/registration and updated
         registry inventory tests; removed the frontend
         `ExpandSettingsNode.svelte` component, display helper, display tests,
         node-type registrations, and retired `syncExpandPorts`/
         `autoConnectExpandToInference` no-op store exports. Remaining
         `expand-settings` source-search hits are documentation and the registry
         assertion that the node is retired.
       - Verification passed: `cargo fmt -p workflow-nodes -- --check`; `cargo
         test -p workflow-nodes test_inventory_collects_all_builtins --lib`;
         `cargo test -p workflow-nodes --lib`; `cargo check -p
         workflow-nodes`; `cargo check -p workflow-nodes --all-features`;
         `cargo check -p workflow-nodes --no-default-features`; `npm run
         typecheck`; `node --experimental-strip-types
         packages/svelte-graph/src/stores/definitionOverlay.test.ts`; targeted
         source search for `expand-settings`, `ExpandSettings`,
         `expandSettings`, `syncExpandPorts`, and
         `autoConnectExpandToInference`.
    2. Rewrite or retire `model-provider` model-path and
       `inference_settings` outputs as a separate input-node slice. This needs a
       decision about whether `model-provider` remains a non-Pumas selector or
       is fully superseded by `puma-lib` plus backend descriptors.
       - 2026-05-29 implementation slice completed: kept `model-provider` as
         a non-Pumas display selector and removed graph-visible executable
         authority from its Rust descriptor. The descriptor now emits only
         `model_name`, display-only `model_info`, and optional
         `search_results`; `ModelInfo` no longer contains `path` or
         `inference_settings` and rejects those legacy fields at the typed
         serde boundary. Frontend mocks/components already matched the narrowed
         `model-provider` shape, so no frontend code change was required.
       - No-fallback/no-legacy gate: no compatibility output or shim was added.
         `model-provider` is not an inference-interface source; backend
         inference-interface descriptors remain the owner for model-specific
         inference ports and options. Targeted source search found no
         `model-provider` `model_path` or `inference_settings` descriptor
         output; the remaining active `inference_settings` mock hit belongs to
         the separately planned `llm-inference` static-port cleanup.
       - Verification passed: `cargo fmt -p workflow-nodes -- --check`;
         `cargo test -p workflow-nodes model_provider --lib`; `cargo test -p
         workflow-nodes --lib`; `cargo check -p workflow-nodes`; `cargo check
         -p workflow-nodes --all-features`; `cargo check -p workflow-nodes
         --no-default-features`; targeted source search for
         `model-provider`/`ModelProviderTask` with `model_path` and
         `inference_settings`.
       - 2026-06-03 implementation slice completed: aligned the reusable
         package `MockWorkflowBackend` with the canonical app mock by removing
         graph-visible `backend_key`, `resolved_model_source`,
         `resolved_model_package_facts`, and `inference_settings` inputs from
         canonical `llm-inference`; added scheduler-vocabulary `runtime` and
         `device` mock inputs for UI selection only; wired the focused
         deletion-gate test into `npm run test:frontend`; and documented that
         mock runtime/device inputs are not backend availability or dependency
         readiness proof.
       - No-fallback/no-legacy gate: no compatibility alias, resolved package
         facts port, model-source port, or settings port was retained in the
         package mock. Backend inference-interface descriptors and scheduler
         decisions remain the owners for executable ports, runtime choice, and
         readiness.
       - Verification passed: `node --experimental-strip-types --test
         packages/svelte-graph/src/backends/MockWorkflowBackend.test.ts`; `npm
         run test:frontend`; `npm run typecheck`; targeted package mock search
         for retired port IDs; `git diff --check`.
       - Verification caveat: raw `node --experimental-strip-types --test`
         cannot import `MockWorkflowBackend.ts` directly because the production
         package source correctly uses `.js` import specifiers for emitted ESM
         paths. The focused test is therefore a source-contract deletion gate,
         while `npm run typecheck` verifies the changed module through the
         TypeScript project.
       - 2026-06-03 documentation hygiene slice completed: rewrote the package
         backend README guardrail to avoid exact retired settings-port
         identifiers while still documenting that mock `llm-inference` must not
         expose backend policy, resolved model facts, or retired settings
         payload ports as graph-visible authority.
       - Verification passed: targeted README source search for exact retired
         settings identifiers and `git diff --check`.
       - 2026-06-03 implementation slice completed: removed the remaining
         app-level `llm-inference.inference_settings` mock input from
         `src/services/workflow/mocks.ts` and tightened
         `WorkflowService.commands.test.ts` so the canonical app mock rejects
         that retired settings port alongside `backend_key`, `runtime_hint`,
         and resolved package/source fact ports.
       - No-fallback/no-legacy gate: the app mock now matches the canonical
         descriptor-owned settings rule. No mock-side settings alias,
         compatibility port, package-fact port, or frontend-owned settings
         source was added.
       - Verification passed: `node --experimental-strip-types --test
         src/services/workflow/WorkflowService.commands.test.ts`; `npm run
         typecheck`; targeted app mock search for retired port IDs; `git diff
         --check`.
    3. Retire legacy graph-visible processing inference nodes with path-shaped
       ports (`audio-generation`, `onnx-inference`, `depth-estimation`) in a
       separate registry slice, or explicitly reclassify any retained node as a
       non-generic typed node with no model-path interface ownership.
       - 2026-05-29 implementation slice completed: removed the Rust
         `audio-generation`, `onnx-inference`, and `depth-estimation`
         processing descriptors, inventory registrations, exports, and
         descriptor tests from `workflow-nodes`; removed their specialized
         frontend node components and registrations; and updated source
         READMEs to keep those task families behind backend-resolved generic
         inference descriptors and scheduler/runtime dispatch.
       - No-fallback/no-legacy gate: no compatibility descriptors,
         path-shaped ports, `environment_ref` inputs, `inference_settings`
         inputs, or specialized graph-visible components were retained.
         Registry tests now assert the three node types are retired. Remaining
         `depth_estimation` references are backend task-kind contracts and
         runtime capability tests, not authorable graph node registrations.
         Runtime/backend execution paths that still mention old node-type
         strings are classified as runtime-dispatch legacy-removal work and
         must not be treated as graph-authoring success paths.
       - Verification passed: `cargo fmt -p workflow-nodes -- --check`; `npm
         run typecheck`; `cargo test -p workflow-nodes
         test_inventory_collects_all_builtins --lib`; `node
         --experimental-strip-types
         src/components/nodes/workflow/primitiveInputMetadata.test.ts`;
         `cargo check -p workflow-nodes`; `cargo test -p workflow-nodes
         --lib`; `cargo check -p workflow-nodes --all-features`; `cargo check
         -p workflow-nodes --no-default-features`; targeted source search for
         retired graph-visible node/component/task names.
    4. Update the C# native smoke fixture to the canonical
       `puma-lib -> llm-inference -> image-output` shape without `modelPath` as
       a fixture slice after the graph node deletions stabilize.
       - 2026-05-29 implementation slice completed: rewrote the opt-in C#
         native diffusion smoke fixture to author
         `puma-lib -> llm-inference -> image-output` with
         `pumas_model_ref` and no graph-authored `modelPath`, `model_path`,
         `inference_settings`, or direct `diffusion-inference` node. The
         smoke script now requires `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID`,
         accepts optional `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID`, and
         rejects retired model-path environment variables instead of adapting
         them into graph data.
       - No-fallback/no-legacy gate: the fixture is path-free and descriptor
         aligned. The only remaining path-environment references in the touched
         script are explicit fail-closed rejection messages for retired
         variables.
       - Verification passed: `bash -n
         scripts/check-uniffi-csharp-diffusion-smoke.sh`; C# compile-only
         check against the generated UniFFI binding; targeted source search
         over the touched fixture/script/docs for retired path, settings, and
         direct diffusion node terms; `git diff --check` for the touched files.
       - Discovered issue: full `./scripts/check-uniffi-csharp-smoke.sh`
         compiled the Rust/C# harness but failed during the existing text
         workflow runtime path with `saved executable validation snapshot was
         not found for workflow version`. That is a validation-snapshot
         lifecycle issue outside this fixture rewrite and remains a follow-up
         before using the full smoke as a runtime acceptance gate.
    5. Remove retired `inference_settings` edges from bundled starter workflow
       templates and keep template documentation aligned with the
       model-ref-only handoff.
       - 2026-05-29 implementation slice completed: removed
         `puma-lib.inference_settings -> llm-inference.inference_settings`
         edges from the tiny-SD text-to-image and GGUF reranker starter
         templates, and updated the template README so starters carry
         `pumas_model_ref` only. Model-specific inference options remain
         backend descriptor-owned.
       - No-fallback/no-legacy gate: no replacement template edge or hidden
         graph field was added. Retired direct inference-node names remain only
         in README rejection/invariant text.
       - Verification passed: JSON parse check for both templates; `npm run
         typecheck`; targeted source search over `src/templates/workflows` for
         retired settings/path/backend/direct-inference terms; `git diff
         --check` for the touched template files.
    6. Remove retired `inference_settings` and explicit backend requirements
       from tracked current image workflow fixtures and their smoke validator.
       - 2026-05-29 implementation slice completed: removed the
         `puma-lib.inference_settings -> llm-inference.inference_settings`
         edge from the tracked Juggernaut workflow fixture, updated the
         current-image workflow smoke to reject explicit `backend_key` instead
         of requiring it, and documented that saved examples carry only
         `pumas_model_ref` from `puma-lib`.
       - No-fallback/no-legacy gate: the smoke validator now enforces
         scheduler-owned runtime selection and model-ref-only inference
         authoring. Retired direct inference-node and backend-field terms
         remain only in rejection/invariant checks.
       - Verification passed: `node
         scripts/check-current-image-workflow-smoke.mjs`; JSON parse check for
         `.pantograph/workflows/juggernaut-x-v10-sdxl.json`; targeted source
         search over the touched fixture/script/README; `git diff --check` for
         touched files.
    7. Defer runtime/backend `model_path`/`entry_path` findings to the
       runtime-dispatch milestone; do not delete them from inference backends in
       this milestone.
  - No-fallback/no-legacy gate: none of these options may preserve
    `inference_settings`, `expand-settings`, `modelPath`, `ModelDependencyRequest`,
    `ModelRefV2`, or model-path-derived node-engine inference paths as
    successful workflow graph execution or inference-interface sources.

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
- [x] 2026-05-28 graph-session validation cleanup slice completed:
  - Smallest useful vertical slice: added an explicit workflow-service cleanup
    boundary for current inference-validation state and wired graph edit-session
    close to clear validation records for the closed graph session.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/inference_validation_state.rs`,
    `crates/pantograph-workflow-service/src/graph/session.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and the execution log.
  - No-fallback/no-legacy confirmation: close cleanup does not preserve any
    alternate validation cache or transport-owned summary. The current
    validation owner remains keyed by typed graph session and graph revision,
    and session close removes only the closed session's backend-owned state.
  - Standards result: cleanup is owned by workflow-service, uses typed
    `WorkflowGraphSessionId` values internally, adds deterministic focused
    tests, and documents the lifecycle invariant in the graph README. No new
    dependencies, frontend transport, Tauri policy, or legacy validation path
    were added.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    close_session_clears_current_validation_state --lib`; `cargo test -p
    pantograph-workflow-service
    clear_graph_session_removes_only_matching_validation_state --lib`;
    `cargo test -p pantograph-workflow-service inference_validation_state
    --lib`; `cargo test -p pantograph-workflow-service
    current_validation_summary --lib`; and `cargo check -p
    pantograph-workflow-service`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: add the event-driven lifecycle owner that cancels or
    supersedes in-flight validation work and uses this cleanup boundary for
    buffered events and late provider-result rejection.
- [x] 2026-05-28 validation publication post-provider freshness slice
  completed:
  - Smallest useful vertical slice: re-check graph-session freshness after
    inference fact-provider await points and before recording current
    validation state, so graph changes during fact lookup return stale summary
    diagnostics instead of publishing outdated descriptor projections.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_inference_validation_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and the execution log.
  - No-fallback/no-legacy confirmation: stale provider results are rejected
    through the existing backend-owned validation summary path. The slice does
    not add frontend/Tauri resolver facts, alternate publication paths, path
    metadata, or compatibility fallbacks.
  - Standards result: the graph lock is reacquired only to canonicalize and
    compute the current graph revision after fact lookup; publication and state
    recording still happen only when the originally snapshotted revision is
    current. The new race test uses `tokio::sync::Notify` instead of sleeps.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    refresh_current_validation_summary_rejects_revision_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session --lib`; `cargo test -p
    pantograph-workflow-service current_validation_summary --lib`; `cargo test
    -p pantograph-workflow-service inference_validation_state --lib`; `cargo
    check -p pantograph-workflow-service`; and `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: the event-driven lifecycle owner can now build on a
    publisher that rejects stale post-provider results, but it still needs task
    ownership, cancellation/supersession, bounded event state, and transport
    event delivery slices.
- [x] 2026-05-28 validation lifecycle owner identity slice completed:
  - Smallest useful vertical slice: added a workflow-service
    `inference_validation_lifecycle` owner for active validation-session
    identity, supersession, session-close rejection, and publication freshness
    checks, then composed it into `GraphSessionStore`.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/inference_validation_lifecycle.rs`,
    `crates/pantograph-workflow-service/src/graph/session.rs`,
    `crates/pantograph-workflow-service/src/graph/session_inference_validation_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`,
    `crates/pantograph-workflow-service/src/graph/mod.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and the execution log.
  - No-fallback/no-legacy confirmation: validation publications are accepted
    only for the active backend-owned graph session, graph revision, and
    validation-session id. Superseded validation results return the current
    backend summary without recording stale node projections; no Tauri/frontend
    validation state, raw facts, or compatibility resolver path was added.
  - Standards result: lifecycle policy is isolated in a dedicated
    workflow-service module, uses typed ids/revisions/session ids, keeps async
    at lock boundaries only, adds deterministic supersession/close tests, and
    updates the graph README. No spawned tasks or new dependencies were added
    in this slice.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    inference_validation_lifecycle --lib`; `cargo test -p
    pantograph-workflow-service
    refresh_current_validation_summary_rejects_superseded_validation_session
    --lib`; `cargo test -p pantograph-workflow-service
    current_validation_summary --lib`; `cargo test -p
    pantograph-workflow-service publish_inference_validation_session --lib`;
    `cargo test -p pantograph-workflow-service inference_validation_state
    --lib`; `cargo check -p pantograph-workflow-service`; and `git diff
    --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: add task ownership/cancellation and bounded event
    buffers around this lifecycle owner before exposing dedicated
    graph-validation transport events.
- [x] 2026-05-28 validation lifecycle bounded event state slice completed:
  - Smallest useful vertical slice: extended the workflow-service validation
    lifecycle owner with a bounded per-graph-session lifecycle event log,
    monotonic event sequences, supersession/pending/accepted event kinds,
    dropped-event accounting, and close cleanup that removes buffered lifecycle
    events for the closed graph session.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/inference_validation_lifecycle.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and the execution log.
  - No-fallback/no-legacy confirmation: lifecycle event state is owned only by
    workflow-service and keyed by typed graph session, graph revision, and
    validation-session ids. No frontend/Tauri event transport, raw Pumas facts,
    graph mutation validation policy, unbounded event stream, or alternate
    validation resolver was added.
  - Standards result: event retention is bounded, deterministic tests use only
    direct owner calls, no sleeps or new dependencies were added, and the graph
    README now documents lifecycle event ordering and bounded retention.
    Transport-visible overflow/backpressure diagnostics remain a follow-up for
    the dedicated graph-validation transport slice.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    inference_validation_lifecycle --lib`; `cargo test -p
    pantograph-workflow-service
    refresh_current_validation_summary_rejects_superseded_validation_session
    --lib`; `cargo test -p pantograph-workflow-service
    current_validation_summary --lib`; `cargo test -p
    pantograph-workflow-service publish_inference_validation_session --lib`;
    `cargo check -p pantograph-workflow-service`; source-search verification
    for retired workflow events, model paths, raw JSON, `anyhow`, and
    `Result<T, String>` in the lifecycle owner; and `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: add task ownership/cancellation with observed
    completion, panic, and cancellation paths; expose bounded lifecycle events
    through a dedicated graph-validation transport after backend lifecycle tests
    pass.
- [x] 2026-05-28 validation lifecycle publication event slice completed:
  - Smallest useful vertical slice: changed lifecycle publication acceptance to
    decide active-session freshness while holding the active-session read lock,
    release that lock, and then record either an accepted or typed rejected
    publication event in the bounded lifecycle event log.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/inference_validation_lifecycle.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and the execution log.
  - No-fallback/no-legacy confirmation: rejected publications remain typed
    lifecycle outcomes for the active workflow-service owner. No transport
    facts, frontend policy, raw path state, or alternate validation resolver was
    added.
  - Standards result: lifecycle event retention no longer holds the active
    lifecycle read guard while awaiting event-buffer writes. Superseded
    publications now have a typed `PublicationRejected` lifecycle event rather
    than message-only or missing state.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    inference_validation_lifecycle --lib`; `cargo test -p
    pantograph-workflow-service
    refresh_current_validation_summary_rejects_superseded_validation_session
    --lib`; `cargo test -p pantograph-workflow-service
    current_validation_summary --lib`; `cargo test -p
    pantograph-workflow-service publish_inference_validation_session --lib`;
    `cargo check -p pantograph-workflow-service`; source-search verification
    for retired workflow events, model paths, raw JSON, `anyhow`, and
    `Result<T, String>` in the lifecycle owner; and `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: add task ownership/cancellation with observed
    completion, panic, and cancellation paths; add transport-visible
    overflow/backpressure diagnostics in the dedicated graph-validation
    transport slice.
- [x] 2026-05-28 validation publisher attempt extraction slice completed:
  - Smallest useful vertical slice: added a dedicated workflow-service
    `inference_validation_publisher` module for one validation publication
    attempt after graph snapshotting, then routed both
    `refresh_current_validation_summary` and
    `publish_inference_validation_session` through it.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/inference_validation_publisher.rs`,
    `crates/pantograph-workflow-service/src/graph/session_inference_validation_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`,
    `crates/pantograph-workflow-service/src/graph/mod.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and the execution log.
  - No-fallback/no-legacy confirmation: fact lookup, lifecycle freshness
    checks, descriptor publication, and current-state recording now have one
    workflow-service path. No frontend/Tauri resolver facts, raw paths, graph
    mutation validation policy, or alternate publication branch was added.
  - Standards result: the publisher accepts typed graph session, graph revision,
    validation-session id, and graph snapshot input, keeps graph lock ownership
    outside the fact-provider await point, records current state only after
    freshness checks, and adds a deterministic explicit-publication stale
    revision test using barriers rather than sleeps.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session --lib`; `cargo test -p
    pantograph-workflow-service refresh_current_validation_summary --lib`;
    `cargo test -p pantograph-workflow-service current_validation_summary
    --lib`; `cargo test -p pantograph-workflow-service
    inference_validation_lifecycle --lib`; `cargo check -p
    pantograph-workflow-service`; source-search verification that fact lookup,
    descriptor publication, and current-state recording appear only in the
    publisher module rather than `session_inference_validation_api.rs`;
    source-search verification for retired workflow events, model paths, raw
    JSON, `anyhow`, and `Result<T, String>` in the publisher/session validation
    API; and `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: move graph snapshot request ownership fully into the
    publisher API, add the explicit bounded node-projection policy, and then
    add task ownership/cancellation around the publisher before transport
    delivery.
- [x] 2026-05-28 validation node projection bound slice completed:
  - Smallest useful vertical slice: added an explicit
    `MAX_VALIDATION_NODE_PROJECTIONS` policy to synchronous validation
    publication and a typed `TooManyNodeProjections` publication error before
    node projection records are allocated or emitted.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/inference_interface_publication.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and the execution log.
  - No-fallback/no-legacy confirmation: oversized projection output fails at
    the workflow-service publisher boundary instead of falling back to truncated
    serialization, frontend filtering, transport-side caps, or alternate
    validation paths.
  - Standards result: the bound is enforced by a typed Rust error and a focused
    unit test that constructs more inference requests than the maximum without
    sleeps, external services, or raw JSON transport contracts.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    inference_interface_publication --lib`; `cargo test -p
    pantograph-workflow-service publish_inference_validation_session --lib`;
    `cargo test -p pantograph-workflow-service
    refresh_current_validation_summary --lib`; `cargo test -p
    pantograph-workflow-service current_validation_summary --lib`; `cargo
    check -p pantograph-workflow-service`; source-search verification for the
    projection bound and typed error; and `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: project this typed publication error into
    transport-visible validation diagnostics when the dedicated live validation
    transport is added.
- [x] 2026-05-28 validation lifecycle cancellation signal slice completed:
  - Smallest useful vertical slice: added owner-issued cancellation receivers to
    active validation lifecycle records. Starting a replacement validation
    session signals cancellation to the superseded session, and closing a graph
    edit session signals cancellation before dropping active lifecycle state.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/inference_validation_lifecycle.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and the execution log.
  - No-fallback/no-legacy confirmation: cancellation is owned by the
    workflow-service lifecycle owner and keyed by typed graph session, graph
    revision, and validation-session ids. No detached tasks, transport-owned
    cancellation, frontend policy, or alternate validation resolver was added.
  - Standards result: cancellation uses Tokio watch channels already available
    in the crate, adds deterministic supersession/close tests without sleeps,
    and preserves the sync-core/async-shell boundary by signaling cancellation
    before future task ownership is introduced.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    inference_validation_lifecycle --lib`; `cargo test -p
    pantograph-workflow-service
    refresh_current_validation_summary_rejects_superseded_validation_session
    --lib`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session --lib`; `cargo test -p
    pantograph-workflow-service current_validation_summary --lib`; `cargo
    check -p pantograph-workflow-service`; source-search verification for
    retired workflow events, model paths, raw JSON, `anyhow`,
    `Result<T, String>`, and spawned task calls in the lifecycle owner; and
    `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: use these cancellation receivers from tracked spawned
    validation tasks and observe task completion, cancellation, and panic paths
    at the lifecycle owner.
- [x] 2026-05-28 validation lifecycle typed cancellation reason slice completed:
  - Smallest useful vertical slice: changed lifecycle cancellation receivers
    from boolean cancellation to typed cancellation reasons for supersession and
    graph-session close.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/inference_validation_lifecycle.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and the execution log.
  - No-fallback/no-legacy confirmation: cancellation reason ownership remains
    inside workflow-service and uses explicit typed lifecycle state instead of
    inferring cancellation source from incidental metadata, strings, transport
    state, or frontend policy.
  - Standards result: cancellation causes are typed, deterministic lifecycle
    tests assert the exact reason, and no new dependency, spawned task, or
    transport event path was added.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    inference_validation_lifecycle --lib`; `cargo test -p
    pantograph-workflow-service
    refresh_current_validation_summary_rejects_superseded_validation_session
    --lib`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session --lib`; `cargo test -p
    pantograph-workflow-service current_validation_summary --lib`; `cargo
    check -p pantograph-workflow-service`; source-search verification for
    retired workflow events, model paths, raw JSON, `anyhow`,
    `Result<T, String>`, and spawned task calls in the lifecycle owner; and
    `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: have the validation publisher consume typed
    cancellation reasons before accepting provider results, then use them from
    tracked spawned validation tasks.
- [x] 2026-05-28 validation publisher cancellation consumption slice completed:
  - Smallest useful vertical slice: have the shared validation publisher
    consume the lifecycle cancellation receiver after fact lookup and stop
    before publication acceptance or current-state recording when a validation
    session has been superseded or its graph session was closed.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/inference_validation_lifecycle.rs`,
    `crates/pantograph-workflow-service/src/graph/inference_validation_publisher.rs`,
    `crates/pantograph-workflow-service/src/graph/session_inference_validation_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and the execution log.
  - No-fallback/no-legacy confirmation: stale provider results are rejected by
    the workflow-service lifecycle owner and shared publisher instead of a
    transport retry, frontend filter, legacy validation path, or inferred graph
    metadata. Cancellation remains a typed lifecycle outcome.
  - Standards result: the cancellation outcome is typed, displayable for the
    existing service error envelope, and checked before publication acceptance
    so async fact lookup cannot publish a superseded graph shape.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    inference_validation_lifecycle --lib`; `cargo test -p
    pantograph-workflow-service
    publish_inference_validation_session_rejects_superseded_validation_session
    --lib`; `cargo test -p pantograph-workflow-service
    refresh_current_validation_summary_rejects_superseded_validation_session
    --lib`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session --lib`; `cargo test -p
    pantograph-workflow-service current_validation_summary --lib`; `cargo
    check -p pantograph-workflow-service`; source-search verification for
    retired workflow events, model paths, raw JSON, `anyhow`,
    `Result<T, String>`, and spawned task calls in the lifecycle/publisher
    path; and `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: use the same cancellation receiver from tracked
    spawned validation tasks and observe task completion, cancellation, and
    panic paths at the lifecycle owner.
- [x] 2026-05-28 graph revision validation cancellation slice completed:
  - Smallest useful vertical slice: add an explicit graph-revision-changed
    cancellation reason to the validation lifecycle owner and wire the
    `remove_edge` mutation path to cancel the active validation session after
    a successful graph mutation.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/inference_validation_lifecycle.rs`,
    `crates/pantograph-workflow-service/src/graph/inference_validation_publisher.rs`,
    `crates/pantograph-workflow-service/src/graph/session.rs`,
    `crates/pantograph-workflow-service/src/graph/session_inference_validation_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and the execution log.
  - No-fallback/no-legacy confirmation: graph mutation cancellation is owned by
    workflow-service lifecycle state and uses a typed cancellation reason
    instead of treating stale provider output as a successful path, adding a
    frontend filter, or preserving a second validation route.
  - Standards result: the lifecycle owner records a bounded typed cancellation
    event, the publisher carries the current graph revision in cancelled
    outcomes so refresh responses remain stale-aware, and the mutation path
    cancels outside the graph session lock.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    inference_validation_lifecycle --lib`; `cargo test -p
    pantograph-workflow-service
    publish_inference_validation_session_rejects_graph_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service
    refresh_current_validation_summary_rejects_revision_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session --lib`; `cargo test -p
    pantograph-workflow-service refresh_current_validation_summary --lib`;
    `cargo test -p pantograph-workflow-service remove_edges --lib`; `cargo
    check -p pantograph-workflow-service`; source-search verification for
    retired workflow events, model paths, raw JSON, `anyhow`,
    `Result<T, String>`, and spawned task calls in the lifecycle/publisher
    path; and `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: wire `cancel_active_validation_after_graph_mutation`
    through the rest of the graph-revision-changing edit operations, then use
    tracked spawned validation tasks and observe task completion,
    cancellation, and panic paths at the lifecycle owner.
- [x] 2026-05-28 node-data validation cancellation slice completed:
  - Smallest useful vertical slice: wire node-data graph mutations through the
    workflow-service validation cancellation helper after graph state is
    snapshotted and the graph session lock is released.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_node_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and the execution log.
  - No-fallback/no-legacy confirmation: inference-node data changes cancel
    active validation through the canonical lifecycle owner instead of letting
    stale provider results publish, relying on frontend invalidation, or
    preserving a parallel stale-output validation path.
  - Standards result: the production mutation path does not hold the graph
    session lock while notifying the lifecycle owner, uses the existing typed
    graph-revision-changed cancellation reason, and adds a focused race test.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_node_data_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service
    update_node_data_merges_patch_into_existing_data --lib`; `cargo test -p
    pantograph-workflow-service publish_inference_validation_session --lib`;
    `cargo check -p pantograph-workflow-service`; production source-search
    verification for retired workflow events, model paths, raw JSON, `anyhow`,
    `Result<T, String>`, and spawned task calls in `session_node_api.rs`; and
    `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` continues to
    report the pre-existing dead-code warning for
    `WorkflowExecutionSessionStore::set_active_run_execution_plan`.
  - Remaining follow-up: wire the lifecycle cancellation helper through the
    remaining graph-revision-changing node, edge, connection, grouping, undo,
    and redo mutations.
- [x] 2026-05-28 validation graph revision re-plan boundary recorded:
  - Discovered issue: `WorkflowGraph::compute_fingerprint()` currently derives
    graph revision from node ids/types and edge endpoints/handles only. It does
    not include `node.data`, even though inference validation resolver inputs
    can come from graph-authored node data such as model refs, selected
    artifact refs, explicit runtime/device constraints, descriptor snapshots,
    authored port snapshots, literals, defaults, and dependency-environment
    sidecar choices.
  - Why implementation stops: continuing to wire graph-revision-change
    cancellation through more edit operations would harden a stale freshness
    contract. The validation lifecycle, current validation state, submit gate,
    scheduler admission, and saved authored snapshot drift checks all depend on
    a revision/fingerprint that must change when validation-relevant graph data
    changes. Otherwise node-data edits can retain the same validation revision
    and make stale summaries appear current.
  - Required re-plan decision: define the canonical validation freshness key.
    It must either be the graph revision updated to include a stable,
    validation-relevant projection of graph node data, or a separate typed
    validation-input fingerprint stored beside graph revision in validation
    state, events, submit gates, scheduler projections, and saved graph
    authored snapshots.
  - Selected design: use option 3. Redefine `WorkflowGraphRevision` as the
    canonical semantic graph revision, then make inference-validation freshness
    depend on that revision plus validation-owned descriptor/model/runtime
    fingerprints. The graph revision is the workflow-service-owned identity for
    graph semantics; descriptor fingerprints remain owned by the inference
    interface resolver and prove that backend-resolved ports, defaults,
    availability, and constraints still match the authored graph.
  - Standards alignment:
    - Simplicity: one graph semantic revision remains the freshness authority
      for graph state, while descriptor/model/runtime fingerprints remain
      separate because they are backend resolver facts rather than graph
      document semantics.
    - Executable/structured contract discipline: persisted saved graphs,
      validation state, lifecycle events, submit gates, scheduler projections,
      and drift diagnostics must agree on the same revision semantics instead
      of duplicating field interpretation.
    - Stale async response safety: fact lookup, dependency actions, and
      scheduler admission must re-check the current semantic graph revision and
      descriptor fingerprints before accepting results.
    - No compatibility shim: do not keep mutation-side cancellation as the
      freshness mechanism, do not add timestamps/request counters as authority,
      and do not let frontend/Tauri cache-busting decide validation freshness.
  - Canonical semantic revision scope:
    - Include node identity, node type, connection topology, group/input/output
      semantics, and graph-authored node data that can affect validation,
      execution, scheduler materialization, dependency-environment actions, or
      saved authored inference snapshots.
    - Exclude layout-only presentation fields such as node position from
      validation freshness. If layout needs independent dirty tracking later,
      add a separate layout/document revision instead of overloading
      `WorkflowGraphRevision`.
    - Node-data projection must be centralized in workflow-service graph code.
      Unknown graph-authored data on inference/execution-relevant nodes must
      fail conservative by participating in semantic revision until the node
      contract explicitly marks it presentation-only. Known presentation-only
      fields may be excluded only through a typed, documented projection rule.
    - The projection must use deterministic serialization/canonical ordering;
      do not hash raw JSON text, map iteration order, incidental frontend field
      order, or transport-specific formatting.
  - Required implementation slices before resuming graph-mutation cancellation:
    1. Add focused tests that currently expose the bug: validation/execution
       node-data changes must change `WorkflowGraphRevision`, while position-
       only changes must not change validation freshness.
    2. Introduce a centralized semantic graph revision projection/helper in
       workflow-service graph code and update `WorkflowGraph::compute_fingerprint()`
       to use a versioned semantic projection.
    3. Audit current revision consumers: edit-session responses, connection
       stale checks, derived graph cache, current validation state, validation
       lifecycle events, dependency-environment intents, submit gate,
       scheduler projections, executable validation snapshots, and saved graph
       drift diagnostics.
    4. Update graph README/API consumer contract text so callers understand
       that graph revision is semantic, not visual layout identity.
    5. Resume mutation cancellation wiring only after the revision semantics
       are deterministic and tested.
  - No-fallback/no-legacy confirmation: do not compensate with incidental
    cancellation calls, frontend cache busting, timestamps, or transport-side
    invalidation. The freshness key must be canonical, deterministic, and owned
    by workflow-service/shared contracts as appropriate.
  - Files inspected: `crates/pantograph-workflow-service/src/graph/types.rs`,
    `crates/pantograph-workflow-service/src/graph/session_node_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session.rs`,
    `crates/pantograph-workflow-service/src/graph/inference_validation_state.rs`,
    `crates/pantograph-workflow-service/src/graph/inference_validation_publisher.rs`,
    and this milestone.
  - Remaining follow-up: implement the selected semantic graph revision design
    before wiring cancellation into the remaining graph edit operations or
    closing the live validation lifecycle owner item.
- [x] 2026-05-28 validation freshness option 3 standards review completed:
  - Decision recorded: option 3 is selected as the standards-compliant design.
    `WorkflowGraphRevision` becomes the canonical semantic graph revision, and
    inference-validation freshness is that revision plus descriptor/model/
    runtime fingerprints owned by validation/resolver contracts.
  - Standards result: the plan keeps graph semantics, resolver facts, async
    lifecycle freshness, scheduler admission, and UI/layout state separated by
    explicit ownership boundaries. It rejects incidental invalidation,
    timestamps, frontend cache busting, or transport request ids as validation
    authority.
  - Blast-radius notes: implementation must audit graph fingerprinting,
    derived graph cache invalidation, edit-session revision responses,
    connection stale checks, validation state, lifecycle events,
    dependency-environment action intents, submit gate, scheduler projections,
    executable validation snapshots, saved authored snapshots, graph README,
    and drift diagnostics.
  - Test requirements: add focused Rust tests for semantic node-data revision
    changes and layout-only non-changes before replacing the fingerprint
    projection; then add stale validation/admission tests that prove old
    validation summaries cannot remain current after semantic node-data edits.
- [x] 2026-05-28 semantic graph revision projection slice completed:
  - Smallest useful vertical slice: replace graph fingerprint node rows with a
    centralized versioned semantic projection that includes deterministic
    node-data content while excluding layout-only position fields.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/types.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: validation freshness now derives from
    canonical graph revision semantics instead of incidental mutation-side
    cancellation, frontend cache busting, timestamps, or transport request ids.
  - Standards result: node-data hashing is deterministic and canonical; object
    key order does not affect graph revision; layout-only position edits do not
    invalidate validation freshness; and the API consumer/producer contract
    documents semantic revision behavior.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service graph_fingerprint
    --lib`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_node_data_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service
    update_node_position_updates_session_graph --lib`; `cargo test -p
    pantograph-workflow-service current_validation_summary --lib`; `cargo test
    -p pantograph-workflow-service publish_inference_validation_session --lib`;
    `cargo test -p pantograph-workflow-service
    port_definition_round_trip_preserves_inference_payloads --lib`; `cargo
    check -p pantograph-workflow-service`; production source-search
    verification in `types.rs` for path fields, `anyhow`, `Result<T, String>`,
    and spawned task calls; and `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` still
    reports the pre-existing `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: audit remaining revision consumers and add stale
    validation/admission tests proving old summaries cannot remain current
    after semantic node-data edits; then resume remaining mutation cancellation
    wiring.
- [x] 2026-05-28 current validation semantic revision regression slice
      completed:
  - Smallest useful vertical slice: add session-level regression coverage that
    publishes an executable validation summary, mutates inference-node semantic
    data, and proves the old validation summary is stale while the new revision
    is missing until revalidated. The same slice proves layout-only position
    edits preserve the current executable summary.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the test asserts validation freshness
    comes from canonical `WorkflowGraphRevision` semantics and submit-gate
    state, not mutation-side cache busting, frontend invalidation, timestamps,
    or transport request ids.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    current_validation_summary --lib`; `cargo test -p
    pantograph-workflow-service graph_fingerprint --lib`; `cargo check -p
    pantograph-workflow-service`; and `git diff --check`.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: continue the revision-consumer audit through
    scheduler admission, executable validation snapshots, saved graph drift
    diagnostics, and dependency-environment action intents before resuming the
    remaining mutation cancellation wiring.
- [x] 2026-05-28 scheduler projection semantic revision regression slice
      completed:
  - Smallest useful vertical slice: add graph-session coverage proving the
    scheduler inference task projection boundary refuses to materialize tasks
    after inference-node semantic data changes make the previously published
    validation summary stale.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: scheduler projection now has focused
    regression coverage for the canonical validation-state lookup keyed by the
    current semantic graph revision. The test does not add a compatibility
    branch, graph-data fallback, timestamp, or frontend invalidation path.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    scheduler_inference_task_projections --lib`; `cargo test -p
    pantograph-workflow-service
    publish_inference_validation_session_records_current_summary --lib`;
    `cargo check -p pantograph-workflow-service`; and `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` still
    reports the pre-existing `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: continue the revision-consumer audit with executable
    validation snapshot sources, saved graph drift diagnostics, and
    dependency-environment action intents.
- [x] 2026-05-28 executable snapshot semantic revision regression slice
      completed:
  - Smallest useful vertical slice: add graph-session coverage proving the
    executable validation snapshot source refuses to produce an executable
    snapshot after inference-node semantic data changes stale the previously
    published validation summary.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: executable snapshot sourcing is
    validated through the canonical current semantic graph revision and
    validation-state lookup. The slice adds no compatibility snapshot path,
    graph-data fallback, timestamp, or frontend invalidation path.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    executable_validation_snapshot_source_rejects_semantic_node_data_stale_summary
    --lib`; `cargo test -p pantograph-workflow-service
    scheduler_inference_task_projections --lib`; `cargo check -p
    pantograph-workflow-service`; and `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` still
    reports the pre-existing `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: continue the revision-consumer audit with saved graph
    drift diagnostics and dependency-environment action intents.
- [x] 2026-05-28 dependency-environment semantic revision regression slice
      completed:
  - Smallest useful vertical slice: add graph-session coverage proving
    dependency-environment action intents fail closed after inference-node
    semantic data changes stale the previously published validation summary.
    The test covers both the old requested graph revision and the new current
    graph revision without a matching validation summary.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: dependency-environment actions remain
    keyed by the current semantic graph revision and validation-state summary;
    the slice adds no compatibility dependency action path, graph-data fallback,
    timestamp, or frontend invalidation path.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    dependency_environment_action_intent_rejects_semantic_node_data_stale_summary
    --lib`; `cargo test -p pantograph-workflow-service
    dependency_environment_action_intent --lib`; `cargo check -p
    pantograph-workflow-service`; and `git diff --check`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` still
    reports the pre-existing `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: continue the revision-consumer audit with saved graph
    drift diagnostics, then resume remaining mutation cancellation wiring.
- [x] 2026-05-28 remove-edges validation cancellation slice completed:
  - Smallest useful vertical slice: wire `remove_edges` through the existing
    workflow-service validation lifecycle cancellation helper after successful
    topology mutation, and add publication-time coverage proving pending
    validation output is rejected when a multi-edge removal changes the graph.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: multi-edge topology edits now use the
    canonical validation lifecycle owner and typed cancellation path, with no
    frontend invalidation, timestamp, transport request id, or alternate
    publication fallback.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_remove_edges_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service remove_edges --lib`;
    `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_graph_changed_during_fact_lookup
    --lib`; `cargo check -p pantograph-workflow-service`; source-search
    verification for touched production code; and `git diff --check`.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire the remaining graph-structure mutation APIs
    (`add_edge`, `add_node`, `remove_node`, `delete_selection`, group
    mutations, undo/redo, and connection insert/commit operations) through the
    same lifecycle helper where they change semantic graph state.
- [x] 2026-05-28 add-node validation cancellation slice completed:
  - Smallest useful vertical slice: wire `add_node` through the existing
    workflow-service validation lifecycle cancellation helper after successful
    semantic graph mutation, and add publication-time coverage proving pending
    validation output is rejected when node addition changes the graph.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_node_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: node addition now uses the canonical
    validation lifecycle owner and typed cancellation path, with no frontend
    invalidation, timestamp, transport request id, or alternate publication
    fallback.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_add_node_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_node_data_changed_during_fact_lookup
    --lib`; `cargo check -p pantograph-workflow-service`; source-search
    verification for touched production code; and `git diff --check`.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire `remove_node`, `delete_selection`, `add_edge`,
    group mutations, undo/redo, and connection insert/commit operations through
    the same lifecycle helper where they change semantic graph state.
- [x] 2026-05-28 remove-node validation cancellation slice completed:
  - Smallest useful vertical slice: wire `remove_node` through the existing
    workflow-service validation lifecycle cancellation helper after successful
    semantic graph mutation, and add publication-time coverage proving pending
    validation output is rejected when node removal changes the graph.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_node_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: node removal now uses the canonical
    validation lifecycle owner and typed cancellation path, with no frontend
    invalidation, timestamp, transport request id, or alternate publication
    fallback.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_remove_node_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service
    remove_node_prunes_attached_edges --lib`; `cargo check -p
    pantograph-workflow-service`; source-search verification for touched
    production code; and `git diff --check`.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire `delete_selection`, `add_edge`, group mutations,
    undo/redo, and connection insert/commit operations through the same
    lifecycle helper where they change semantic graph state.
- [x] 2026-05-28 delete-selection validation cancellation slice completed:
  - Smallest useful vertical slice: wire `delete_selection` through the
    existing workflow-service validation lifecycle cancellation helper after
    successful semantic graph mutation, while preserving the empty-selection
    no-op response path without cancellation.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_node_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: mixed node/edge selection deletion now
    uses the canonical validation lifecycle owner and typed cancellation path,
    with no frontend invalidation, timestamp, transport request id, or
    alternate publication fallback.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_delete_selection_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service
    delete_selection_removes_mixed_selection_with_one_undo_snapshot --lib`;
    `cargo check -p pantograph-workflow-service`; source-search verification
    for touched production code; and `git diff --check`.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire `add_edge`, group mutations, undo/redo, and
    connection insert/commit operations through the same lifecycle helper where
    they change semantic graph state.
- [x] 2026-05-28 add-edge validation cancellation slice completed:
  - Smallest useful vertical slice: wire `add_edge` through the existing
    workflow-service validation lifecycle cancellation helper after successful
    topology mutation, and add publication-time coverage proving pending
    validation output is rejected when edge addition changes the graph.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: edge addition now uses the canonical
    validation lifecycle owner and typed cancellation path, with no frontend
    invalidation, timestamp, transport request id, or alternate publication
    fallback.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_add_edge_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_graph_changed_during_fact_lookup
    --lib`; `cargo check -p pantograph-workflow-service`; source-search
    verification for touched production code; and `git diff --check`.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire group mutations, undo/redo, and connection
    insert/commit operations through the same lifecycle helper where they
    change semantic graph state.
- [x] 2026-05-28 create-group validation cancellation slice completed:
  - Smallest useful vertical slice: wire `create_group` through the existing
    workflow-service validation lifecycle cancellation helper after successful
    semantic graph mutation, and add publication-time coverage proving pending
    validation output is rejected when group creation changes the graph.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: group creation now uses the canonical
    validation lifecycle owner and typed cancellation path, with no frontend
    invalidation, timestamp, transport request id, or alternate publication
    fallback.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_create_group_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service group --lib`; `cargo
    check -p pantograph-workflow-service`; source-search verification for
    touched production code; and `git diff --check`.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire `ungroup`, `update_group_ports`, undo/redo, and
    connection insert/commit operations through the same lifecycle helper where
    they change semantic graph state.
- [x] 2026-05-28 ungroup validation cancellation slice completed:
  - Smallest useful vertical slice: wire `ungroup` through the existing
    workflow-service validation lifecycle cancellation helper after successful
    semantic graph mutation, and add publication-time coverage proving pending
    validation output is rejected when ungrouping changes the graph.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: ungrouping now uses the canonical
    validation lifecycle owner and typed cancellation path, with no frontend
    invalidation, timestamp, transport request id, or alternate publication
    fallback.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_ungroup_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service group --lib`; `cargo
    check -p pantograph-workflow-service`; source-search verification for
    touched production code; and `git diff --check`.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire `update_group_ports`, undo/redo, and connection
    insert/commit operations through the same lifecycle helper where they
    change semantic graph state.
- [x] 2026-05-28 update-group-ports validation cancellation slice completed:
  - Smallest useful vertical slice: wire `update_group_ports` through the
    existing workflow-service validation lifecycle cancellation helper after
    successful group-port semantic mutation, and add publication-time coverage
    proving pending validation output is rejected when group port mapping
    changes the graph.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: group port edits now use the canonical
    validation lifecycle owner and typed cancellation path, with no frontend
    invalidation, timestamp, transport request id, or alternate publication
    fallback.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_update_group_ports_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service group --lib`; `cargo
    check -p pantograph-workflow-service`; source-search verification for
    touched production code; and `git diff --check`.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire undo/redo and connection insert/commit
    operations through the same lifecycle helper where they change semantic
    graph state.
- [x] 2026-05-28 undo validation cancellation slice completed:
  - Smallest useful vertical slice: wire successful `undo` operations through
    the existing workflow-service validation lifecycle cancellation helper, and
    add publication-time coverage proving pending validation output is rejected
    when undo restores a previous semantic graph state.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: undo now uses the canonical validation
    lifecycle owner and typed cancellation path after a successful state
    restore, with no frontend invalidation, timestamp, transport request id, or
    alternate publication fallback. Failed no-op undo remains a typed error
    before cancellation.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_undo_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service undo --lib`; `cargo
    check -p pantograph-workflow-service`; source-search verification for
    touched production code; and `git diff --check`.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire redo and connection insert/commit operations
    through the same lifecycle helper where they change semantic graph state.
- [x] 2026-05-28 redo validation cancellation slice completed:
  - Smallest useful vertical slice: wire successful `redo` operations through
    the existing workflow-service validation lifecycle cancellation helper, and
    add publication-time coverage proving pending validation output is rejected
    when redo reapplies a semantic graph change.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: redo now uses the canonical validation
    lifecycle owner and typed cancellation path after a successful state
    restore, with no frontend invalidation, timestamp, transport request id, or
    alternate publication fallback. Failed no-op redo remains a typed error
    before cancellation.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_redo_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service redo --lib`; `cargo
    check -p pantograph-workflow-service`; source-search verification for
    touched production code; and `git diff --check`.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire connection insert/commit operations through the
    same lifecycle helper where they change semantic graph state.
- [x] 2026-05-28 connect validation cancellation slice completed:
  - Smallest useful vertical slice: wire accepted `connect` commits through
    the existing workflow-service validation lifecycle cancellation helper, and
    add publication-time coverage proving pending validation output is rejected
    when a committed connection changes the semantic graph.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_connection_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: accepted connection commits now use
    the canonical validation lifecycle owner and typed cancellation path after
    the graph mutation, with no frontend invalidation, timestamp, transport
    request id, or alternate publication fallback. Rejected connection commits
    continue to return typed rejection responses before cancellation.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_connect_changed_during_fact_lookup
    --lib`; `cargo check -p pantograph-workflow-service`; source-search
    verification for touched production code; and `git diff --check`.
  - Verification attempted and failed: `cargo test -p
    pantograph-workflow-service connect --lib` still fails pre-existing
    connection-intent coverage around LLM prompt/device port selection and
    stream-to-output canonicalization
    (`connection_candidates_return_existing_nodes_and_insertable_types`,
    `preview_node_insert_on_edge_returns_valid_bridge_for_llm`,
    `insert_node_on_edge_returns_two_replacement_edges`, and
    `connect_canonicalizes_llm_stream_drop_to_text_output_response_edge`). The
    cancellation slice does not change connection selection policy, but the
    failure must be resolved or explicitly re-planned before relying on broad
    connection-intent verification for insert-on-edge slices.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire `insert_node_and_connect` and
    `insert_node_on_edge` through the same lifecycle helper where they change
    semantic graph state, after accounting for the current connection-intent
    test failures.
- [x] 2026-05-28 insert-node-and-connect validation cancellation slice
  completed:
  - Smallest useful vertical slice: wire accepted `insert_node_and_connect`
    commits through the existing workflow-service validation lifecycle
    cancellation helper, and add publication-time coverage proving pending
    validation output is rejected when node insertion plus connection changes
    the semantic graph.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_connection_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: accepted insert-and-connect commits
    now use the canonical validation lifecycle owner and typed cancellation
    path after the graph mutation, with no frontend invalidation, timestamp,
    transport request id, or alternate publication fallback. Rejected
    insert-and-connect attempts continue to return typed rejection responses
    before cancellation.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_insert_node_and_connect_changed_during_fact_lookup
    --lib`; `cargo test -p pantograph-workflow-service
    insert_node_and_connect --lib`; `cargo check -p
    pantograph-workflow-service`; source-search verification for touched
    production code; and `git diff --check`.
  - Deviation corrected inside the slice: the first verification attempt found
    a missing test import and rustfmt wrapping difference; both were fixed
    before rerunning verification successfully.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: wire `insert_node_on_edge` through the same lifecycle
    helper where it changes semantic graph state, after resolving or
    re-planning the current connection-intent test failures that affect
    insert-on-edge verification.
- [x] 2026-05-28 insert-node-on-edge validation cancellation slice completed:
  - Smallest useful vertical slice: wire accepted `insert_node_on_edge`
    commits through the existing workflow-service validation lifecycle
    cancellation helper, and add publication-time coverage proving pending
    validation output is rejected when edge insertion changes the semantic
    graph.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/session_connection_api.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: accepted insert-on-edge commits now
    use the canonical validation lifecycle owner and typed cancellation path
    after the graph mutation, with no frontend invalidation, timestamp,
    transport request id, or alternate publication fallback. Rejected
    insert-on-edge attempts continue to return typed rejection responses before
    cancellation.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    publish_inference_validation_session_rejects_insert_node_on_edge_changed_during_fact_lookup
    --lib`; `cargo check -p pantograph-workflow-service`; source-search
    verification for touched production code; and `git diff --check`.
  - Verification attempted and failed: `cargo test -p
    pantograph-workflow-service insert_node_on_edge --lib` still fails the
    pre-existing `insert_node_on_edge_returns_two_replacement_edges` test
    because connection-intent bridge selection chooses static `device` instead
    of retired `prompt` for `llm-inference`. The cancellation slice used a
    canonical `merge` insertion path to avoid preserving the retired
    `llm-inference` static prompt/response behavior.
  - Discovered issue: source-search verification in `session_tests.rs` still
    reports existing test fixtures for `model_path` and existing async test
    `tokio::spawn` calls; this slice added no production path usage or
    untracked task ownership. `cargo check -p pantograph-workflow-service`
    still reports the pre-existing `set_active_run_execution_plan` dead-code
    warning.
  - Remaining follow-up: explicitly re-plan the connection-intent LLM
    insert-on-edge tests so they either consume descriptor-backed inference
    interface snapshots or are deleted as retired static-port coverage.
- [x] 2026-05-28 connection-intent static `llm-inference` test re-plan
  decision:
  - Discovered issue: broad connection verification still contains tests that
    expect retired static `llm-inference` task ports (`prompt`, `response`, and
    `stream`). The canonical graph-visible `llm-inference` descriptor has
    already been shrunk to bootstrap/control ports, so keeping these tests green
    by restoring static task ports would preserve legacy behavior and violate
    the no-fallback/no-legacy rule.
  - Standards result: use option 3. First remove or rewrite the retired
    static-port connection coverage while preserving generic connection/insert
    coverage through canonical static nodes such as `text-input`,
    `text-output`, and `merge`. Then add descriptor-backed inference
    connection intent as a separate planned slice where `llm-inference` ports
    come from resolved validation/interface snapshots and missing or stale
    snapshots fail closed with typed diagnostics.
  - Standards mapping:
    - Coding standards: delete retired compatibility coverage instead of
      keeping backwards-compatibility shims.
    - Architecture standards: keep descriptor/interface snapshots as the
      structured producer-consumer contract for inference task ports; do not let
      `connection_intent` reinterpret stale static node metadata as inference
      authority.
    - Rust API standards: parse and validate descriptor-backed port shapes at
      the boundary into typed values before passing them inward; do not add raw
      stringly port guesses for `llm-inference`.
    - Testing standards: replace the failing tests with boundary invariant
      coverage proving forbidden retired task ports cannot cross the static
      registry boundary, generic connection helpers still work, and
      `llm-inference` connection/insert requests fail closed or report typed
      diagnostics until descriptor-backed ports are present.
    - Frontend standards: when descriptor-backed connection intent is later
      exposed to the graph editor, the backend remains the source of truth and
      validation/port refresh must be event-driven rather than local optimistic
      port invention.
  - Next implementation slice: update `connection_intent` tests and any
    directly affected assertions so retired static `llm-inference` port
    expectations are removed or rewritten as fail-closed invariants, while
    retaining canonical non-inference connection coverage.
  - Deferred canonical slice: design and implement descriptor-backed inference
    connection intent after the test cleanup. That slice must consume current
    validation/interface snapshots, expose typed diagnostics for missing/stale
    snapshots, and include cross-layer coverage from descriptor resolution to
    graph-editor connection candidates before broadening runtime support.
- [x] 2026-05-28 connection-intent static `llm-inference` cleanup slice
  completed:
  - Smallest useful vertical slice: remove retired static `llm-inference`
    prompt/response/stream expectations from connection-intent verification,
    block static `llm-inference` constraint inputs such as `device` from graph
    connection authority, and preserve insert/connect coverage through
    canonical non-inference nodes.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/connection_intent.rs`,
    `crates/pantograph-workflow-service/src/graph/connection_insert.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, this
    milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: static `llm-inference` no longer
    becomes insertable from text via the `device` constraint, retired
    stream-to-response canonicalization coverage now fails closed, and no
    compatibility branch restores static prompt/response/stream ports.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service connect --lib`;
    `cargo test -p pantograph-workflow-service insert_node_on_edge --lib`;
    `cargo check -p pantograph-workflow-service`; source-search verification
    for touched production code; and `git diff --check`.
  - Discovered issue: source-search verification still reports the pre-existing
    `connection_insert` prompt-priority helper, which is not currently reachable
    through static `llm-inference` ports after this slice but should be revisited
    when descriptor-backed inference connection intent is designed. The same
    search still reports existing test fixtures for `model_path`, descriptor
    port ids, retained fail-closed `stream` assertions, and async test
    `tokio::spawn` calls. `cargo check -p pantograph-workflow-service` still
    reports the pre-existing `set_active_run_execution_plan` dead-code warning.
  - Remaining follow-up: design descriptor-backed inference connection intent
    as its own canonical slice, consuming current validation/interface snapshots
    and surfacing typed diagnostics for missing or stale snapshots instead of
    reintroducing static inference task ports.
- [x] 2026-05-28 descriptor-backed inference connection contract-first design
  decision:
  - Decision: use option 3. The next descriptor-backed inference connection
    implementation must start by extending the shared
    `pantograph-inference-interface-contracts` crate with an executable
    connection-surface contract instead of adding a workflow-service-only DTO.
    The contract is needed because the same resolved inference-node interface is
    consumed by workflow-service connection intent, graph-editor presentation,
    saved authored graph drift handling, and queue-admission validation.
  - Contract ownership: `pantograph-inference-interface-contracts` owns only
    path-free DTOs, validated ids, bounded diagnostics, status enums, descriptor
    fingerprint identity, authored/current drift shape, and fixture contract
    tests. It must not own Pumas lookup, graph mutation, live validation event
    streams, scheduler placement, runtime-host execution, frontend rendering, or
    Tauri/app wiring.
  - Required contract shape: add a versioned connection-surface DTO keyed by
    graph revision, optional validation session id, target inference node id,
    descriptor fingerprint, current status, descriptor-backed input/output port
    descriptors, draft validation summary, optional drift report, and bounded
    diagnostics. Missing, pending, stale, unavailable, blocked, or drift-blocked
    descriptor state must be represented explicitly and must fail closed for
    connection commit and workflow queue submission.
  - Cross-layer flow: workflow-service publishes the contract from the current
    validation/interface projection and consumes it when resolving connection
    candidates, insert-on-edge previews, and connection commits. The graph editor
    renders backend-published ports and diagnostics without inventing inference
    ports locally. Scheduler admission consumes executable validation authority
    derived from the same descriptor source, not the editor connection surface
    and not raw `node.data`.
  - Saved graph behavior: saved graph files may retain the minimum authored
    inference interface snapshot needed to preserve historical graph shape and
    explain drift after Pantograph/model/runtime changes. They must not store
    Pumas package facts, executable paths, scheduler decisions, runtime load
    targets, media payloads, or frontend presentation state. Reopening a graph
    compares the authored snapshot with the current descriptor and surfaces typed
    drift diagnostics while keeping graph editing available.
  - No-fallback/no-legacy confirmation: do not restore static
    `llm-inference` task ports such as `prompt`, `response`, `stream`, `device`,
    or denoising/sampler ports. Do not parse raw model paths, Pumas metadata
    bags, runtime blobs, frontend caches, or `node.data.definition` as inference
    port authority. If the descriptor-backed surface cannot be resolved, return
    typed diagnostics and keep submit/queue actions blocked.
  - Standards result:
    - Coding standards: the contract separates authored graph shape, current
      validation state, editor rendering, scheduler execution, runtime policy,
      persistence, and diagnostics so maintainers can reason about each concern
      independently.
    - Architecture standards: this is an executable producer-consumer contract in
      a dedicated contracts crate because multiple independently evolving
      consumers trust the same persisted and cross-layer shape.
    - Rust API standards: use validated newtypes/enums, `serde(deny_unknown_fields)`,
      versioned DTOs, `TryFrom` validated wrappers, bounded collections, and
      structured diagnostics rather than stringly port guesses.
    - Documentation standards: update the contract crate README/API consumer
      contract and structured producer contract when the DTO is added, including
      compatibility, drift, and regeneration rules for saved artifacts.
    - Frontend standards: keep validation and port refresh event-driven from
      backend state; graph editing remains responsive while validation runs, but
      submit/queue remains disabled when the backend contract is not executable.
    - Testing standards: start with contract fixtures and boundary invariant
      tests proving retired static ports cannot return, missing/stale descriptor
      state fails closed, and a resolved descriptor produces graph-editor
      connection candidates through the real workflow-service boundary.
  - Thin-slice sequence:
    1. Extend `pantograph-inference-interface-contracts` with the connection
       surface DTO, validation wrappers, README contract text, and JSON fixtures.
    2. Add workflow-service publication/adapter code that builds the surface from
       existing validation projection records and rejects stale/missing surfaces
       with typed diagnostics.
    3. Wire connection candidates, insert-on-edge preview, and connection commit
       to consume the surface for `llm-inference` dynamic task ports while
       retaining static bootstrap/control inputs only.
    4. Expose the same backend-owned surface to the graph editor through the
       existing event/session path; frontend renders ports, drift, and
       diagnostics but does not compute inference capability.
    5. Verify queue-admission gating uses executable validation authority derived
       from the same descriptor source and cannot submit stale or unresolved
       inference nodes.
  - Re-plan triggers: stop before implementation if the connection-surface DTO
    starts carrying Pumas paths/facts, runtime-host payloads, scheduler placement
    decisions, frontend layout/presentation state, unbounded diagnostics, or live
    validation event transport. Stop if implementation would require a
    compatibility shim for retired static task ports or a second inference-port
    authority outside descriptor-backed validation.
- [x] 2026-05-28 descriptor-backed inference connection contract slice
  completed:
  - Smallest useful vertical slice: added the shared inference connection surface
    contract to `pantograph-inference-interface-contracts` without wiring
    workflow-service, frontend, scheduler, runtime-host, or Tauri consumers.
  - Files touched by the slice:
    `crates/pantograph-inference-interface-contracts/src/lib.rs`,
    `crates/pantograph-inference-interface-contracts/README.md`,
    `crates/pantograph-inference-interface-contracts/tests/contract.rs`,
    `crates/pantograph-inference-interface-contracts/tests/fixtures/README.md`,
    `crates/pantograph-inference-interface-contracts/tests/fixtures/connection_surface_image_generation_current.json`,
    `crates/pantograph-inference-interface-contracts/tests/fixtures/connection_surface_image_generation_drift_blocked.json`,
    this milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the contract is path-free and rejects
    unknown legacy/backend-owned fields through `serde(deny_unknown_fields)`.
    Current surfaces require a descriptor fingerprint. Non-current surfaces fail
    closed and must not be executable. Drift-blocked surfaces require a drift
    report and diagnostics. No static `llm-inference` task ports were restored.
  - Verification passed: `cargo fmt -p pantograph-inference-interface-contracts`;
    `cargo test -p pantograph-inference-interface-contracts`; and
    `cargo check -p pantograph-inference-interface-contracts`.
  - Remaining follow-up: add the workflow-service publication/adapter slice that
    builds this connection surface from existing validation projection records
    and rejects stale/missing surfaces with typed diagnostics before wiring
    connection candidates, insert-on-edge preview, or commits.
- [x] 2026-05-28 descriptor-backed inference connection publication adapter
  slice completed:
  - Smallest useful vertical slice: added workflow-service publication behavior
    that converts existing validation projection records into the shared
    `InferenceConnectionSurface` contract without changing graph connection
    candidates, insert-on-edge preview, commits, frontend presentation, or queue
    admission.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/inference_interface_publication.rs`,
    this milestone, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the adapter consumes descriptor-backed
    projection records only. It does not parse raw `node.data`, Pumas paths,
    frontend state, runtime-host payloads, or scheduler decisions as port
    authority. Non-current surfaces synthesize typed diagnostics when descriptor
    diagnostics are absent and remain non-executable.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`;
    `cargo test -p pantograph-workflow-service inference_interface_publication
    --lib`; and `cargo check -p pantograph-workflow-service`.
  - Discovered issue: `cargo check -p pantograph-workflow-service` still reports
    the pre-existing `set_active_run_execution_plan` dead-code warning in
    `scheduler/store.rs`; this slice added no new warnings.
  - Remaining follow-up: wire connection candidates and connection commits to
    consume the published connection surface for `llm-inference` dynamic task
    ports while retaining static bootstrap/control inputs only.
- [x] 2026-05-28 descriptor-backed inference connection workflow-service
  consumption slice completed:
  - Smallest useful vertical slice: added a current validation-state read model
    for `InferenceConnectionSurface` records and wired edit-session connection
    candidates/direct commits to consume those surfaces for `llm-inference`
    dynamic task ports.
  - Files touched by the slice:
    `crates/pantograph-workflow-service/src/graph/connection_intent.rs`,
    `crates/pantograph-workflow-service/src/graph/connection_insert.rs`,
    `crates/pantograph-workflow-service/src/graph/session_connection_api.rs`,
    `crates/pantograph-workflow-service/src/graph/inference_validation_state.rs`,
    `crates/pantograph-workflow-service/src/graph/inference_interface_publication.rs`,
    `crates/pantograph-workflow-service/src/graph/mod.rs`,
    `crates/pantograph-workflow-service/src/graph/README.md`, this milestone,
    and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: live connection authority for
    model/task-specific `llm-inference` ports now comes from current
    descriptor-backed validation surfaces. Static `llm-inference` still exposes
    only bootstrap/control connection inputs, and missing/stale/non-current
    surfaces do not fall back to authored snapshots, `node.data.definition`,
    Pumas paths, frontend state, scheduler decisions, or runtime-host payloads.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`;
    `cargo test -p pantograph-workflow-service connection_intent --lib`;
    `cargo test -p pantograph-workflow-service current_connection_surfaces
    --lib`; `cargo test -p pantograph-workflow-service connect --lib`; and
    `cargo check -p pantograph-workflow-service`.
  - Verification note: `cargo check -p pantograph-workflow-service` still
    reports the pre-existing `set_active_run_execution_plan` dead-code warning
    in `scheduler/store.rs`; this slice added no new warnings.
  - Remaining follow-up: insert-on-edge preview still uses static insertion
    definitions because a newly inserted inference node has no model-bound
    descriptor surface yet. The next descriptor-backed connection slice should
    expose backend-owned surfaces to the graph editor/event path and decide
    whether bridge preview should support already-resolved inference nodes only
    or wait for the later event-driven editor port refresh.
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
    `WorkflowService::set_media_conversion_executor`. That blocker has since
    been fixed by removing the stale injection; before relying on app-crate
    tests for future slices, complete the separate cleanup that deletes the
    now-unused Tauri managed media conversion adapter.
  - Remaining follow-up: replace the Tauri command input/API and frontend call
    sites so selected-model hydration receives `pumas_model_ref` only, then
    implement the canonical path-free dependency-environment hydration service
    boundary and remove the legacy `ModelDependencyRequest` dependency path.
