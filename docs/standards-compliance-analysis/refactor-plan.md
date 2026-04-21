# Plan: Pantograph Standards Compliance Refactor

## Objective
Bring Pantograph into practical compliance with the standards in
`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`
without changing product behavior unnecessarily. The plan prioritizes backend
ownership, runtime safety, source traceability, and quality gates before broader
file-size cleanup.

## Scope

### In Scope
- Standards compliance issues recorded in passes 01-05.
- Refactor sequencing for Rust workflow services, Tauri adapters, Svelte stores,
  host bindings, tooling, tests, and documentation.
- Additional discovered bugs or risks that should be tracked even when they are
  not only standards issues.

### Out of Scope
- Performing the implementation in this audit artifact.
- Reverting unrelated worktree asset changes.
- Replacing the product roadmap with a new feature roadmap.

## Inputs

### Problem
Pantograph has useful prior compliance work, but remaining debt is layered:
large files, frontend-owned workflow decisions, incomplete runtime lifecycle
ownership, missing traceability docs, red lint gates, incomplete CI, placeholder
execution nodes, and stale code all overlap.

### Constraints
- The backend remains the source of truth for workflow execution, scheduler,
  graph mutation, runtime readiness, and diagnostics projections.
- Public facades should be preserved during extraction unless a breaking API
  change is explicitly accepted.
- Existing generated/binding artifacts must remain reproducible.
- Unrelated dirty asset changes in the worktree are not part of this plan.

### Assumptions
- `pantograph-workflow-service` is the correct owner for workflow session/run
  semantics and graph mutation contracts.
- Tauri is an adapter/composition layer, not the owner of canonical workflow
  business logic.
- Svelte stores may own transient UI state but not durable workflow rules.
- The current C# binding path remains a supported binding surface.

### Dependencies
- Pass files in this directory.
- `docs/anti-pattern-remediation-tracker.md`
- `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-standards-compliance-refactor-handoff.md`
- Root `package.json`, `Cargo.toml`, `launcher.sh`, `.github/workflows/*`.

## Overlap Depth
Maximum overlap depth discovered after rereading the updated standards: 5.

The deepest finding cluster is workflow execution/diagnostics ownership:
1. File-size and source traceability issues.
2. Backend-owned data and layered architecture issues.
3. Runtime/concurrency and stale-event lifecycle issues.
4. Testing/tooling/CI contract enforcement issues.
5. Rust API/async/binding/release contract enforcement issues.

This plan was refined in 5 passes, matching that depth:
- Plan pass 1: grouped raw findings by subsystem.
- Plan pass 2: ordered groups by dependency and blast radius.
- Plan pass 3: merged overlapping workflow/runtime/doc/test work into layered milestones.
- Plan pass 4: checked each milestone against verification, lifecycle, and traceability standards.
- Plan pass 5: reconciled the April 21 standards updates, especially the new Rust-specific standards.

## Finding Map
| Finding | Primary Remediation Milestone |
| ------- | ----------------------------- |
| P01-F01 large files | M4 Decompose large surfaces |
| P01-F02 missing READMEs | M1 Restore traceability baseline |
| P01-F03 incomplete README sections | M1 Restore traceability baseline |
| P01-F04 nested generated Git repo | M1 Restore traceability baseline |
| P01-F05 incomplete previous compliance | All milestones |
| P02-F01 frontend workflow identity | M2 Backend-owned workflow contracts |
| P02-F02 frontend group mutations | M2 Backend-owned workflow contracts |
| P02-F03 overgrown composition root | M3 Runtime lifecycle and composition |
| P02-F04 duplicate frontend adapters | M2 Backend-owned workflow contracts |
| P02-F05 large binding facades | M4 Decompose large surfaces |
| P02-F06 placeholder tool execution | M2 Backend-owned workflow contracts |
| P03-F01 Vite all-interface bind | M0 Stabilize red/high-risk gates |
| P03-F02 production expects | M3 Runtime lifecycle and composition |
| P03-F03 untracked spawned tasks | M3 Runtime lifecycle and composition |
| P03-F04 weak PID records | M3 Runtime lifecycle and composition |
| P03-F05 path boundary drift | M2 Backend-owned workflow contracts |
| P03-F06 listener shutdown docs | M3 Runtime lifecycle and composition |
| P03-F07 critical DOM mutation | M0 Stabilize red/high-risk gates |
| P04-F01 missing general CI | M6 Tooling, CI, tests, release |
| P04-F02 red lint gates | M0 Stabilize red/high-risk gates |
| P04-F03 incomplete a11y enforcement | M6 Tooling, CI, tests, release |
| P04-F04 undocumented test strategy | M6 Tooling, CI, tests, release |
| P04-F05 dependency ownership drift | M5 Rust workspace hardening, M6 Tooling |
| P04-F06 missing toolchain pinning | M5 Rust workspace hardening, M6 Tooling |
| P04-F07 launcher missing `--test` | M6 Tooling, CI, tests, release |
| P04-F08 incomplete release workflow | M6 Tooling, CI, tests, release |
| P05-F01 missing Rust workspace lints | M5 Rust workspace hardening |
| P05-F02 incomplete Rust metadata/publish control | M5 Rust workspace hardening |
| P05-F03 missing canonical Rust verification | M5 Rust workspace hardening, M6 Tooling |
| P05-F04 Rust async spawn ownership audit | M3 Runtime lifecycle and composition |
| P05-F05 underdocumented Cargo feature contracts | M5 Rust workspace hardening |
| P05-F06 missing binding support tiers/artifact model | M5 Rust workspace hardening, M4 Decompose large surfaces |
| P05-F07 Rust platform `cfg` review | M5 Rust workspace hardening |
| P05-F08 unsafe policy not enforced | M5 Rust workspace hardening |

## Milestones

### M0: Stabilize Red and High-Risk Gates
Goal: Stop current known failures and remove the easiest security/runtime violations before deeper refactors.

Status:
- Complete: `src/components/nodes/workflow/ImageOutputNode.svelte` no longer appends/removes a temporary anchor for downloads.
- Complete: `src/components/runtime-manager/ManagedRuntimeSummaryGrid.svelte` no longer uses string-literal mustache spacing.
- Complete: Vite now defaults to loopback and documents explicit LAN opt-in.
- Complete: `src/generated/.git` was documented as intentional runtime
  undo/redo state before M1 resolved the source-root exception.

Tasks:
- [x] Fix `src/components/nodes/workflow/ImageOutputNode.svelte` so `npm run lint:critical` passes.
- [x] Fix `src/components/runtime-manager/ManagedRuntimeSummaryGrid.svelte` so `npm run lint:full` gets past the current Svelte lint failure.
- [x] Change Vite default host from `0.0.0.0` to `127.0.0.1`; add an explicit documented override if LAN exposure is needed.
- [x] Record the `src/generated/.git` decision: supported generated state with README and ignore rules, or remove it from the source root.

Verification:
- `npm run lint:critical`
- `npm run lint:full`
- `npm run typecheck`
- Manual Vite/Tauri dev launch with loopback host.

### M1: Restore Documentation and Traceability Baseline
Goal: Make source ownership and architectural intent navigable before moving large code.

Tasks:
- [x] Add missing READMEs identified in pass 01. Status: `crates/README.md` now
  documents the Rust workspace package-role boundary, and runtime identity plus
  registry crate roots now document their public contracts. `node-engine`,
  `workflow-nodes`, and `pantograph-workflow-service` crate roots now document
  the core workflow execution chain. Embedded runtime, frontend HTTP adapter,
  UniFFI, and Rustler crate roots now document host-facing runtime and binding
  contracts. The Tauri runtime-registry command helper boundary now documents
  its transport-only role. Workflow-service tests, examples, and private
  workflow helper modules now document their public-contract and decomposition
  roles. Inference Python workers, managed runtime platform adapters, and the
  reserved managed-binaries marker now document their runtime-artifact and
  worker-contract boundaries.
- [x] Resolve the `src/generated/` documentation exception by either moving
  generated runtime state outside `src/` or replacing the nested Git repository
  with a backend-owned structured history store that allows a tracked
  `src/generated/README.md`. Status: generated component Git metadata now
  lives in ignored `.pantograph/generated-components.git/`, while
  `src/generated/README.md`, `.gitignore`, and `.gitkeep` are trackable marker
  files for the Vite work tree.
- [x] Update host-facing READMEs for `pantograph-uniffi`, `pantograph-rustler`,
  `pantograph-workflow-service`, generated components, and Tauri workflow
  command boundaries to include required sections. Status: workflow-service,
  UniFFI, and Rustler source READMEs now use the required decision,
  host-facing, and structured-producer sections. Frontend source-root generated
  component state and Tauri workflow command docs now use exact host-facing and
  structured-producer contract headings.
- [x] Mark structured producer directories, especially templates, saved workflows,
  generated components, schemas, and binding artifacts, with `Structured Producer Contract`.
  Status: `.pantograph` saved workflow and orchestration data directories now
  document their structured producer contracts and README marker ignore rules.
  UniFFI binding generator helpers now document generated binding artifact
  contracts.
- [x] Add a decision-traceability script adapted from the standards template and
  configure host-facing/structured-producer paths. Status: repo-local script,
  npm entrypoint, and Lefthook staged-file command are now in place.
- [x] Normalize the remaining template-generated README files that still contain
  banned placeholder language before enabling broad full-branch traceability as
  a hard gate. The repo-wide scan now finds no banned placeholder README
  language across `src`, `src-tauri`, `crates`, `packages`, or `scripts`.
  Status: workflow-nodes root/control/output/system/tool READMEs are normalized
  and now explicitly record the tool-loop/tool-executor placeholder risk.
  Node-engine orchestration and context-key helper READMEs are normalized.
  Svelte graph package edge, constants, context, and utility READMEs are
  normalized.
  Tauri root, agent/RAG/tools, helper binary, hotload sandbox, and LLM backend
  READMEs are normalized.
  App component edge, architecture-node, orchestration, and side-panel READMEs
  are normalized.
  Frontend config, feature entrypoint, node registry, and shared type READMEs
  are normalized.
  Frontend lib, design-system, hotload sandbox, agent/architecture service, and
  shared barrel READMEs are normalized.

Additional issue recorded during implementation:
- `crates/inference/src/managed_runtime/managed_binaries/` is an empty,
  unreferenced source-tree directory. It is now documented as a no-artifacts
  marker, but M3 managed-runtime cleanup should remove it unless a real
  source-owned fixture role is accepted.
- Resolved: the placeholder README sweep was broader than the original pass-01
  examples, but all 40 generated placeholder descriptions and placeholder
  import examples have now been replaced with directory-specific ownership
  text.

Verification:
- Run the new decision-traceability script against changed directories.
- Review for banned placeholder language.

### M2: Backend-Owned Workflow Contracts
Goal: Eliminate frontend and adapter ownership of canonical workflow behavior.

Tasks:
- [ ] Move execution-id claiming, stale-event filtering, run/session attribution,
  and diagnostics relevance into backend-owned trace/session projection APIs.
  Progress: app toolbar event handling now delegates execution-id claiming and
  stale-event filtering to the shared workflow execution event projector instead
  of maintaining its own duplicate gate. The shared projector now returns an
  explicit ownership projection consumed by `WorkflowService.ts` and workflow
  execution event reducers. Tauri workflow-event serialization now emits a
  backend-authored `ownership` projection, and the shared frontend projector
  prefers that payload when present. The remaining work is to replace the last
  consumer-local current-run comparison with a backend-owned trace/session
  relevance query.
- [x] Make `workflow_get_diagnostics_snapshot` provide the exact frontend-ready
  identity and relevance decisions needed by `diagnosticsStore.ts`.
  Status: diagnostics projections now carry backend-authored context containing
  requested snapshot filters, source execution id, relevant execution id, and
  relevance. `diagnosticsStore.ts` consumes that context instead of claiming or
  filtering diagnostics snapshot events with frontend-local execution helpers.
- [x] Convert group create/ungroup/update-port operations to return backend-owned
  graph mutation responses, then remove local graph reconstruction from
  `packages/svelte-graph/src/stores/createWorkflowStores.ts`.
  Status: `pantograph-workflow-service` now owns session-scoped node-group
  graph mutations through `graph/group_mutation.rs`; Tauri exposes them through
  graph mutation responses, and the Svelte store renders the returned backend
  graph instead of reconstructing collapsed group nodes and boundary edges.
- [x] Collapse duplicate Tauri wire normalizers into one executable contract
  module consumed by both `WorkflowService.ts` and `TauriWorkflowBackend.ts`.
  Status: `src/lib/tauriConnectionIntentWire.ts` owns connection-intent
  serialization/normalization and `src/lib/tauriConnectionIntentWire.test.ts`
  covers the camelCase/snake_case Tauri payload conversions used by both
  consumers.
- [x] Decide whether `tool-loop` and `tool-executor` are disabled/experimental
  or real. Remove successful placeholder behavior either way. Status:
  descriptors remain registered for saved workflow compatibility, while
  `tool-executor` and tool-call continuation in `tool-loop` now fail explicitly
  until backend-owned tool execution contracts exist.
- [x] Consolidate active workflow persistence/path validation around the service
  store and delete or archive superseded Tauri-local paths.
  Status: `FileSystemWorkflowGraphStore` now owns the external workflow path
  validation tests, and the unused Tauri-local `workflow_persistence_commands.rs`
  module has been removed so command wiring uses the service-backed store only.

Verification:
- Rust contract tests for workflow service diagnostics, group mutations, and
  event relevance.
- Frontend tests proving diagnostics store only applies backend projections.
- Cross-layer test: backend mutation response -> Tauri invoke -> Svelte graph state.
- `cargo test -p pantograph-workflow-service`
- `npm run test:frontend`

### M3: Runtime Lifecycle and Composition
Goal: Make startup, shutdown, process, and background tasks explicitly owned.

Tasks:
- [ ] Extract `src-tauri/src/main.rs` into a small composition facade and focused
  setup/shutdown modules.
  Progress: window-close shutdown now lives in `src-tauri/src/app_lifecycle.rs`,
  giving gateway shutdown, stale session worker shutdown, loaded runtime
  invalidation, and runtime-registry sync a focused lifecycle owner outside the
  command registration root. Startup now flows through `run_app()` so fatal
  composition errors are explicit rather than hidden in `main()`.
- [x] Replace production `expect(...)` calls in startup/setup/shutdown with typed
  errors, logged context, or documented invariant-only assertions.
  Status: project-root resolution, Tauri app-data resolution, workflow-session
  cleanup worker startup, workflow runtime capacity application, and the final
  Tauri run result now return logged startup/setup errors instead of panicking.
- [x] Introduce a task supervisor or owned service handles for extension init,
  process stdout/stderr readers, process monitors, health monitors, and cleanup workers.
  Status: `src-tauri/src/app_tasks.rs` now owns a Tauri-managed startup task
  registry, and the executor-extension initialization task is tracked and
  aborted during window shutdown before runtime workers/processes are stopped.
  Tauri managed-runtime process handles now own and abort their stdout reader,
  stderr reader, and process-monitor tasks when stopped. `HealthMonitor` now
  owns and aborts its polling loop through its service API, and app shutdown
  stops health monitoring before workflow/runtime teardown.
- [x] Route automatic recovery spawned from health-monitor failure handling
  through an owned recovery task handle or supervisor.
  Added during implementation: `RecoveryManager` now tracks the automatic
  recovery task launched from health-monitor failure handling, ignores
  duplicate launches while a task is still active, and exposes a shutdown hook
  used by the window lifecycle path.
- [x] Replace bare PID files with structured records that include pid, start time,
  version/mode, and owner identity where needed.
  Status: Tauri-managed runtime launches now write JSON PID records with
  schema version, pid, start time, owner, owner version, runtime mode, and
  executable path. Inference stale-sidecar cleanup still accepts legacy
  plain-PID files while preferring the structured record shape.
- [x] Document listener bind address, max connections, timeout/heartbeat strategy,
  and graceful shutdown for product listener paths.
  Status: inference and Tauri runtime docs now state that managed runtime
  listeners are loopback-bound by default, readiness/health probes are bounded
  by startup and request timeouts, max-connection policy is delegated to the
  managed runtime until a backend contract exists, and graceful shutdown runs
  through gateway/process lifecycle owners.

Verification:
- Targeted Rust tests for startup error paths where feasible.
- Shutdown tests or smoke checks proving spawned tasks stop.
- PID stale/reuse tests behind platform abstractions.
- `cargo check`
- `cargo test` for affected crates.

### M4: Decompose Large Surfaces with Facade Preservation
Goal: Reduce large files without breaking public APIs.

Tasks:
- Split `crates/pantograph-workflow-service/src/workflow.rs` by session API,
  graph persistence/editing, diagnostics/trace, scheduler queue, runtime
  capabilities, and facade exports.
  Progress: public workflow request/response/error DTO definitions now live in
  `crates/pantograph-workflow-service/src/workflow/contracts.rs` and are
  re-exported by `workflow.rs`, preserving existing public imports while
  reducing the facade. Workflow I/O surface derivation and host-response
  validation now live in `workflow/io_contract.rs`, keeping the facade as the
  caller while isolating bindable input/output schema handling. Host trait
  defaults and scheduler diagnostics provider contracts now live in
  `workflow/host.rs` and are re-exported by the facade. Request, binding,
  output-target, and produced-output validation helpers now live in
  `workflow/validation.rs`, with `validate_workflow_id` re-exported internally
  for technical-fit request shaping. Session runtime preflight cache
  fingerprinting now lives in `workflow/session_runtime.rs` with the cache
  lookup and refresh logic that consumes it. Session runtime loaded-state
  invalidation now lives in `workflow/session_runtime.rs` with the load/unload
  state transitions. Graph edit-session, mutation, connection, persistence,
  and runtime snapshot facade methods now live in `workflow/graph_api.rs`.
  Workflow capability, I/O discovery, and preflight facade methods now live in
  `workflow/preflight_api.rs`. Generic workflow run facade and internal
  session-run handoff now live in `workflow/workflow_run_api.rs`. Service
  construction, capacity-limit configuration, diagnostics-provider setup, and
  session-store guard helpers now live in `workflow/service_config.rs`.
  Session creation and queued session run facade methods now live in
  `workflow/session_execution_api.rs`. Session status, queue inspection,
  scheduler snapshot, cancellation, and reprioritization facade methods now
  live in `workflow/session_queue_api.rs`. Stale cleanup, stale cleanup worker,
  keep-alive, and close-session facade methods now live in
  `workflow/session_lifecycle_api.rs`. The root workflow facade test module now
  lives in `workflow/tests.rs`, reducing `workflow.rs` to the production facade
  shell; the extracted test module still needs a later split by behavior area.
- Split `crates/pantograph-embedded-runtime/src/lib.rs` into runtime host,
  workflow sessions, registry lifecycle, diagnostics projection, model deps,
  and test modules.
  Progress: workflow scheduler diagnostics provider projection now lives in
  `crates/pantograph-embedded-runtime/src/workflow_scheduler_diagnostics.rs`,
  and workflow-facing runtime-registry/warmup coordination error mapping now
  lives in `crates/pantograph-embedded-runtime/src/runtime_registry_errors.rs`.
  Shared runtime extension snapshots and executor extension injection now live
  in `crates/pantograph-embedded-runtime/src/runtime_extensions.rs`. The root
  embedded-runtime test module now lives in
  `crates/pantograph-embedded-runtime/src/lib_tests.rs`; the extracted module
  still needs a later split by behavior area. Embedded-runtime configuration
  and initialization error contracts now live in
  `crates/pantograph-embedded-runtime/src/runtime_config.rs`. Inference-gateway
  runtime-registry controller trait implementations now live in
  `crates/pantograph-embedded-runtime/src/runtime_registry_controller.rs`.
  Embedded-runtime constructors, host projection, registry injection, accessors,
  and shutdown sequencing now live in
  `crates/pantograph-embedded-runtime/src/embedded_runtime_lifecycle.rs`.
  Embedded workflow host helper logic for runtime reservations, retention
  hints, workflow I/O binding, and data-graph output shaping now lives in
  `crates/pantograph-embedded-runtime/src/embedded_workflow_host_helpers.rs`.
  Public embedded-runtime workflow, session, queue, inspection, and keep-alive
  facade methods now live in
  `crates/pantograph-embedded-runtime/src/embedded_workflow_service_api.rs`.
  Public embedded-runtime graph persistence, edit-session, mutation,
  connection, and insert-preview facade methods now live in
  `crates/pantograph-embedded-runtime/src/embedded_workflow_graph_api.rs`.
  Embedded-runtime data-graph execution now lives in
  `crates/pantograph-embedded-runtime/src/embedded_data_graph_execution.rs`.
  Embedded-runtime edit-session graph execution now lives in
  `crates/pantograph-embedded-runtime/src/embedded_edit_session_execution.rs`.
  The embedded `WorkflowHost` implementation now lives in
  `crates/pantograph-embedded-runtime/src/embedded_workflow_host.rs`.
- Split `crates/node-engine/src/core_executor.rs` by execution phases, blocking
  adapters, demand preparation, output handling, and tests.
- Split `src/components/WorkflowGraph.svelte` and
  `packages/svelte-graph/src/components/WorkflowGraph.svelte` into graph canvas,
  connection drag, horseshoe insert, edge insert, selection, keyboard, and
  container-border subcomponents/helpers.
- Split `DependencyEnvironmentNode.svelte` into data parsing, activity log,
  mode controls, override editor, status panels, and command controls.
- Split `pantograph-uniffi` and `pantograph-rustler` facades by exported surface
  family while preserving public names.

Verification:
- File-size scan shows extracted files below review thresholds or documented exceptions.
- Existing public imports/exports remain compatible.
- Affected Rust and frontend tests pass after each slice.

### M5: Rust Workspace, Binding, and Package Contract Hardening
Goal: Adopt the Rust-specific standards now present under `languages/rust/`
without blocking unrelated frontend work.

Tasks:
- Add root `[workspace.lints.rust]` and `[workspace.lints.clippy]`, including
  `unsafe_code = "deny"` by default, then opt member crates into workspace lints.
- Decide the warning ratchet for existing Rust warnings before turning clippy
  into a hard `-D warnings` gate.
- Normalize Rust crate metadata: workspace `version`, `rust-version`,
  `repository`, shared package inheritance, and explicit `publish = false`
  for app, binding-wrapper, internal, and workspace-only crates.
- Document Cargo feature contracts for `inference`, `node-engine`,
  `pantograph-embedded-runtime`, `pantograph-uniffi`, `pantograph-rustler`,
  `workflow-nodes`, and `src-tauri`.
- Keep or justify current default features; move expensive optional behavior
  behind explicit features where consumers should not always pay the cost.
- Classify binding exports as `supported`, `experimental`, or `internal-only`;
  document product-native artifact names and version matching.
- Add or document host-visible binding `version()` behavior.
- Review non-test inline Rust platform `cfg` blocks against the thin-platform-module exception rule.
- Preserve the current no-unsafe state with workspace lint policy; define the
  exception checklist before any future unsafe-owning crate is introduced.
- Add Criterion benchmark policy for Rust performance claims and hot-path changes.

Verification:
- `cargo fmt --all -- --check`
- `cargo check --workspace --all-features`
- `cargo check --workspace --no-default-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` or a documented ratchet.
- Targeted binding native tests and host-language smoke paths for supported surfaces.
- Target checks for required platforms in CI where practical.

### M6: Tooling, CI, Tests, Dependencies, and Release
Goal: Make compliance enforceable instead of only documented.

Tasks:
- Add a general CI workflow with separate jobs for critical lint, typecheck,
  frontend tests, Rust fmt/clippy/check/test/doc-test, dependency audit, and
  summary aggregation.
- Ensure every CI job explicitly bootstraps the package manager or toolchain it
  invokes instead of relying on runner defaults.
- Add `lint:no-new`, `format:check`, and decision-traceability commands; decide
  which are immediately blocking and which begin as ratcheted audits.
- Add `launcher.sh --test` as the canonical local test entrypoint.
- Define the GUI `--release-smoke` CI strategy, including display server,
  sandbox/GPU/shared-memory constraints, and bounded startup behavior.
- Document the repo's hybrid test placement and acceptance strategy.
- Normalize repeated Rust dependency versions to workspace inheritance.
- Document or correct package-local dependency ownership for `packages/svelte-graph`.
- Add toolchain pinning files after confirming intended Rust, Node, and Python versions.
- Add release hardening: artifact naming policy, SBOM generation, release CI
  outline, and changelog automation decision.
- Add Svelte-specific a11y lint/test coverage for interactive generic elements,
  icon-only buttons, embedded controls, and keyboard behavior.

Verification:
- New CI workflow can run locally equivalent commands.
- `./launcher.sh --test`
- `npm run lint:critical`
- `npm run lint:full` or ratcheted equivalent.
- `npm run typecheck`
- `npm run test:frontend`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` or ratcheted equivalent.
- `cargo test --workspace`
- `cargo test --workspace --doc`
- `cargo check --workspace --all-features`
- `cargo check --workspace --no-default-features`
- Targeted cargo tests and binding smoke scripts.

### M7: Dead Code, Warning, and Backlog Cleanup
Goal: Remove stale implementations after ownership moves settle.

Tasks:
- Classify all `cargo check` warnings as remove, use, gate behind feature, or
  document as disabled/experimental.
- Delete unused Tauri-local workflow types, validators, and connection-intent
  helpers superseded by `pantograph-workflow-service`.
- Close or update `docs/anti-pattern-remediation-tracker.md` Phase 5 for
  process-node policy controls.
- Add issue/backlog entries for non-compliance problems intentionally deferred.

Verification:
- `cargo check` warning count is reduced to zero or a documented baseline.
- No duplicate active implementation remains for workflow business logic.

## Risks and Mitigations
| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Backend contract refactor breaks frontend graph editing | High | Add cross-layer mutation tests before removing frontend local reconstruction. |
| Large-file decomposition changes public APIs accidentally | High | Preserve facades first; move implementation behind re-exports. |
| CI becomes red for old debt and blocks all work | Medium | Fix red critical gates first; use ratchets only where debt remains. |
| Rust workspace lints reveal broad warning debt | Medium | Land lint policy with explicit baseline, then ratchet toward `-D warnings`. |
| Feature-contract checks expose optional-dependency coupling | Medium | Keep all-features/no-default-features checks early and split heavy features only after ownership is clear. |
| Binding exports drift during facade split | High | Run native tests, UniFFI metadata check, C# smoke, and Rustler mode checks after each binding slice. |
| Task supervision changes shutdown timing | Medium | Add explicit cancellation and bounded shutdown tests. |

## Re-Plan Triggers
- A backend-owned contract cannot represent current frontend behavior without a public API change.
- Binding smoke tests reveal that exported method names or generated artifacts must change.
- Generated-component history cannot be preserved by the external
  `.pantograph/generated-components.git/` store.
- A general CI workflow exposes a blocker not represented in passes 01-05.
- Rust clippy, doc-test, cross-target, or binding-host checks expose blockers
  not visible in `cargo check`.

## Completion Criteria
- All pass findings are resolved, explicitly deferred with an owner, or reclassified as acceptable with rationale.
- Critical and full local gates are green or ratcheted according to documented policy.
- Workflow execution/session/diagnostics ownership is backend-owned.
- Runtime tasks and process lifecycle have explicit owners and shutdown paths.
- Source directories have meaningful READMEs and traceability enforcement.
- Public bindings have documented support tiers and matching host/native verification.
- Rust workspace lints, crate metadata, feature contracts, unsafe policy, and
  required Rust verification are enforceable or explicitly ratcheted.

## Additional Issue Register
These were discovered during the audit and should remain tracked even if not
fully resolved by standards compliance:
- Resolved: `tool-loop` and `tool-executor` no longer produce successful
  placeholder tool outputs; they fail until backend-owned tool execution
  contracts exist.
- Many Rust dead-code warnings suggest stale workflow and server-discovery paths.
- `pantograph-rustler` currently emits `non_local_definitions` warnings from
  `rustler::resource!`; resolve, update Rustler, or document a temporary lint exception.
- The repo currently has no repo-owned Rust `unsafe` blocks, but also lacks the
  workspace lint policy that would preserve that state.
- Resolved: generated-component history metadata moved out of
  `src/generated/.git` into ignored `.pantograph/generated-components.git/`.
- CI currently verifies important binding/runtime separation paths but does not protect the main frontend and workspace quality gates.
- `crates/pantograph-workflow-service/src/workflow.rs` test fixture
  `MockWorkflowHost` stores `runtime_capabilities: vec![ready_runtime_capability()]`
  but does not override `WorkflowHost::runtime_capabilities`, so
  `workflow_session_lifecycle_create_run_close` fails its create-session
  runtime-capability assertion before close-session behavior is exercised.
