# Execution Management

## Execution Notes

Update during implementation:

- 2026-05-09: Plan created from read-only investigation. No implementation
  changes have been made as part of this plan creation.
- 2026-05-09: Plan updated after codebase blast-radius review to add the
  contract gate, explicit `diffusers`-to-PyTorch execution normalization,
  saved-graph IO inspector mode, no silent retired-node rewriting, and
  single-body generated media retention.
- 2026-05-09: Plan iterated against local coding standards to add standards
  guardrails for typed Rust contracts, sync-core/async-shell boundaries,
  path/dimension validation, frontend ownership, no polling/optimistic updates,
  decomposition review, contract tests, and stricter worker write-set rules.
- 2026-05-09: Plan updated to make the no-legacy boundary explicit: old
  Pantograph graph shapes are not supported through migrations or compatibility
  shims. Transformers, ComfyUI, and InvokeAI were added as reference-only
  guidance for naming, generation semantics, diffusion family taxonomy, and
  family-specific validation.
- 2026-05-09: Researched Transformers, ComfyUI, InvokeAI, and Pantograph's
  existing Pumas package fact contracts. Added the concrete image-generation
  family planner design, minimum Pumas facts, table-driven family requirements,
  component-role extraction, sync adapter boundaries, and no-name-inference
  rule needed before implementation.
- 2026-05-09: Plan updated after codebase blast-radius review to require
  consolidation with existing runtime/preflight/gateway/node-engine paths,
  exact Pumas missing-facts diagnostics, source-level image body
  de-duplication, task/artifact-aware `diffusers` execution normalization,
  shared saved/run graph inspection projection, and validated-plan-only Python
  worker execution.
- 2026-05-09: Plan iterated against standards again to add concrete compliance
  gates: cross-boundary executable fixtures, test-first vertical slices,
  isolated test roots, public facade preservation, centralized typed constants,
  accessibility/focus checks, Python worker envelope validation, async
  lifecycle ownership checks, path/resource spot checks, and optional worker
  coordination ledger/report paths.
- 2026-05-09: Added device and runtime-variant planning as a first-class
  objective. The plan now requires backend-owned device policy, selected device
  facts, runtime variant readiness, llama.cpp multi-variant managed runtime
  support, and no fallback from explicit unavailable device requests.
- 2026-05-10: Device/runtime planning updated to make every executable backend
  sit behind an adapter boundary. Adapters expose task/model/device/runtime
  facts, feasibility diagnostics, estimates, and backend-specific translation;
  the scheduler owns backend/device ranking, RAM/VRAM placement, queue policy,
  explicit preference validation, and later learned throughput policy from
  ledger/artifact facts.
- 2026-05-10: Re-ran the plan against coding standards and tightened the
  implementation gates for composition-root lifecycle ownership, loopback-only
  local services, bounded queues/listeners, centralized path validation,
  checked resource arithmetic, Rust public API traits/errors, feature/dependency
  checks, interop envelope tests, frontend accessibility, and isolated
  durable-state tests.
- 2026-05-10: Tightened the no-fallback/no-legacy rule across the plan. Old
  backend, runtime, device, technical-fit, frontend, and graph execution paths
  are removal or replacement targets. Auto is a first-class scheduler policy,
  not a backup path, and canonical planning failures must return typed
  diagnostics rather than invoking fallback behavior.
- 2026-05-10: Re-iterated the split plan against the coding standards. Added
  README coverage for the multi-file plan directories, tightened the standards
  matrix for no-fallback/no-legacy and documentation traceability, and expanded
  release verification for worktree hygiene, cross-platform cfg isolation, and
  dependency ownership.
- 2026-05-10: Updated milestone ordering so Pumas is not treated as the final
  plan step. Pumas P0-P1 starts after Pantograph Milestone 0 to freeze the
  package-facts producer contract early. Pumas P2-P5 may run in parallel with
  Pantograph Milestones 1-5, but Pumas fact extraction, summaries, update
  cursors, selected-artifact semantics, and cache migration/backfill must be
  complete and pinned before Pantograph Milestone 6 begins real PyTorch/diffusers
  image execution.
- 2026-05-10: Committed the plan directory as the initial documentation slice.
  Milestone 0 then completed as a contract-gate documentation slice with write
  scope limited to this plan. The frozen decisions name the planned graph
  diagnostic DTO, shared graph inspection projection, task/artifact-aware
  `diffusers` to PyTorch execution normalization boundary, missing-facts
  diagnostic fields, wire-format rules, fixture names, isolated test roots,
  decomposition decisions, and concrete fallback/legacy paths found by code
  search.
- 2026-05-10: Completed Milestone 1 and the current graph-shape parts of
  Milestone 2 as one vertical slice. The canonical tracked Juggernaut workflow
  is `.pantograph/workflows/juggernaut-x-v10-sdxl.json`; duplicate ignored
  local Juggernaut/Tiny SD workflow files were removed from the workspace.
  Tracked image-generation saved workflows now use
  `puma-lib -> llm-inference -> image-output`, retain stable model ids only,
  and do not persist local Pumas paths or derived dependency snapshots. Current
  graph canonicalization no longer rewrites retired inference nodes into
  executable `llm-inference`; compatibility migration helpers were deleted and
  graph persistence tests now prove retired shapes remain available for stale
  diagnostics without migration records.
- 2026-05-10: Completed the remaining Milestone 2 Pumas selector/probe slice.
  Diffusion task labels from execution descriptors, package summaries, and
  selector rows now project to graph-facing `image_generation`, while factual
  package metadata such as `pipeline_tag: text-to-image`,
  `recommended_backend: diffusers`, and runtime engine hints remain available
  for downstream diagnostics and planning. This preserves the no-fallback rule:
  the change is deterministic task normalization for current graph intent, not
  a legacy direct-diffusion compatibility route.
- 2026-05-10: Started Milestone 3 with the backend graph diagnostic DTO and
  classifier foundation. `WorkflowGraphDiagnostic` is now owned by the graph
  service facade with typed code/severity/scope fields, bounded message/detail
  payloads, and contract-validation classification for retired node types,
  unknown node types, effective-definition failures, missing edge endpoints,
  missing handles, incompatible ports, capacity errors, and cycles. The older
  `Vec<String>` graph validator remains as a string projection of the
  structured diagnostics for existing binding consumers. Contract-validation
  tests were split into `contract_validation_tests.rs`, keeping the production
  validation module below the standards decomposition threshold after the DTO
  addition.
- 2026-05-10: Continued Milestone 3 with the shared backend graph inspection
  projection. `WorkflowGraphInspectionProjection` now returns the graph
  snapshot, selected-node facts, typed stale diagnostics, and optional run
  context. The service facade can inspect a saved workflow through the existing
  filesystem store without mutating the graph, rewriting retired nodes, or
  requiring workflow-run side effects.
- 2026-05-10: Continued Milestone 3 by attaching backend stale diagnostics to
  edit-session graph snapshot responses. Session responses now carry
  `graph_diagnostics` from the same contract classifier, so graph editor
  transports can forward stale graph facts without frontend inference.
- 2026-05-10: Continued Milestone 3 by adding backend stale diagnostics to the
  historic run graph projection. Run inspection already composes that
  projection, so it can now expose stale graph facts from the immutable run
  snapshot without reading current graph files or rewriting retired nodes.
- 2026-05-10: Continued Milestone 3 by blocking stale executable graphs before
  session queue admission. The rejection now uses an invalid-request workflow
  error envelope with structured graph details containing the same bounded
  `WorkflowGraphDiagnostic` records, and the queue remains untouched.
- 2026-05-10: Started Milestone 4 by extending the IO inspector run graph
  presenter/component path to consume backend `graph_diagnostics`. Stale node
  and edge markers now come from backend facts in `WorkflowRunGraphProjection`,
  not frontend inference over node types or missing canvas edges.
- 2026-05-10: Continued Milestone 4 by exposing saved graph inspection through
  the Tauri/frontend workflow service boundary. `workflow_graph_inspect`
  returns the backend `WorkflowGraphInspectionProjection` directly, including
  selected-node diagnostics, without fabricating run ids or run-shaped
  compatibility data.
- 2026-05-10: Continued Milestone 4 by adding focused saved-graph inspection
  presenters. Saved workflow options require backend-listed workflow ids, and
  saved-graph display models reuse the existing graph canvas presenter with
  backend diagnostics while keeping run context and artifact controls disabled.
- 2026-05-10: Continued Milestone 4 by wiring IO Inspector no-run mode to
  saved graph inspection. The page lists saved workflows from the backend,
  inspects the selected saved workflow through `workflow_graph_inspect`, and
  renders `WorkflowGraphInspectionProjection` diagnostics in a read-only saved
  graph component without artifact controls or fabricated run metadata.
- 2026-05-10: Continued Milestone 4 by surfacing backend stale diagnostics for
  selected run graph nodes in the existing I/O details panel. Artifact metadata
  and reads remain projection-owned; the new stale section only renders
  `runGraph.graph_diagnostics` filtered by selected node id.
- 2026-05-10: Resolved the Milestone 4 component/accessibility test-harness
  decision by staying with the repository's existing Node test approach. Svelte
  DOM harness adoption remains out of scope for this plan pass; accessibility
  coverage is limited to pure presenter/keyboard helper tests and typechecked
  declarative Svelte wiring.
- 2026-05-10: Continued Milestone 4 by extracting Node-tested saved graph
  accessible-label and keyboard-selection helpers. The saved graph snapshot
  component now uses those helpers for SVG node button labels and Enter/Space
  activation without introducing a new frontend test platform.
- 2026-05-10: Continued Milestone 4 by adding a neutral graph-inspection canvas
  wrapper in `graphInspectionPresenters.ts`. Saved graph inspection now uses
  that shared presenter boundary instead of directly depending on the
  run-named canvas helper, while preserving the same backend diagnostic display
  model.
- 2026-05-10: Continued Milestone 4 by moving saved workflow path resolution
  and saved graph selected-node preservation into Node-tested presenter
  helpers. The IO Inspector still owns transient selected-node state, but the
  fallback rules are now covered without a DOM test harness.
- 2026-05-10: Started Milestone 5 with the device/runtime contract gate.
  `crates/inference/src/device_contracts/` now owns validated device ids,
  runtime variant ids, backend ids, canonical device policy DTOs, runtime
  capability facts, backend candidate facts, and selected execution decisions.
  The slice is additive and does not route legacy `DeviceConfig`,
  `DeviceBackend::from_id`, raw llama.cpp ordinals, or technical-fit fallback
  paths through the new contracts.
- 2026-05-10: Continued Milestone 5 by replacing the infallible backend-local
  llama.cpp device parser. `DeviceBackend::try_from_id` now returns typed
  parse errors for unknown selectors, missing ordinals, and malformed ordinals,
  and `parse_llamacpp_device_listing` filters malformed rows through the same
  parser instead of accepting prefix-only device ids.
- 2026-05-10: Continued Milestone 5 by validating llama.cpp runtime-start
  device settings. `BackendConfig::default` now carries explicit `auto` device
  intent, while `LlamaCppRuntimeSettings::try_from_backend_config` rejects
  missing, blank, unknown, or malformed raw device selectors before any sidecar
  command is constructed.
- 2026-05-10: Continued Milestone 5 by adding public device/runtime contract
  fixture tests under `crates/inference/tests/device_contracts.rs`. The
  fixtures pin runtime variant capability diagnostics, selected backend
  execution decisions, and invalid raw device id rejection through the public
  crate API before later Tauri/frontend/worker/persisted projections consume
  the DTOs.
- 2026-05-10: Continued Milestone 5 by adding backend-local llama.cpp
  selector projection into canonical device contract facts.
  `DeviceBackend::to_contract_device` maps resolved CPU, CUDA, and Metal
  selectors to `InferenceDeviceClass`/`InferenceDeviceId`, while unresolved
  auto mode and unsupported Vulkan return typed `DeviceBackendContractError`
  values instead of selected scheduler facts.
- 2026-05-10: Continued Milestone 5 by tightening runtime-load phase records.
  `RuntimeLoadPhaseRecord::dependency_resolved` now requires a
  `DeviceResolutionDecision`, so managed runtime command facts cannot be
  emitted without selected runtime variant, device class, and selected device
  id facts.
- 2026-05-10: Continued Milestone 5 by projecting selected device facts into
  active llama.cpp runtime descriptors. Ready sidecars now expose canonical
  selected device class/id only when the backend-local selector parses and
  projects successfully; unresolved `auto` and unsupported backend-local
  selectors omit selected facts, and malformed active device state produces no
  descriptor.
- 2026-05-10: Continued Milestone 5 with the lifecycle event selected-device
  class contract. `InferenceRequestLifecycleEvent` now has an optional typed
  `selected_device_class` field, and direct event constructors were updated
  without deriving class from raw backend config.
- 2026-05-10: Continued Milestone 5 by wiring gateway lifecycle producers to
  canonical active runtime device facts. Request lifecycle events now carry
  selected device class/id from `LlamaCppActiveRuntimeDescriptor` when present
  and do not report config-only raw device strings as selected devices.
- 2026-05-10: Continued Milestone 5 by extending inference diagnostic ledger
  payloads with `selected_device_class`. The embedded runtime adapter copies
  the canonical lifecycle event class into
  `InferenceExecutionDiagnosticObservedPayload` without deriving it from raw
  config or runtime setting strings.
- 2026-05-10: Continued Milestone 5 by projecting selected device class into
  diagnostics run-list/run-detail read models. The SQLite schema, projection
  DTOs, workflow diagnostics query request, and contract fixtures now carry
  `selected_device_class` from typed inference diagnostic payloads without
  inferring it from raw device ids.
- 2026-05-10: Continued Milestone 5 by blocking workflow admission on
  fallback-shaped technical-fit decisions. `ConservativeFallback`,
  `MissingCandidateData`, and `MissingRuntimeState` decisions now produce
  blocking `WorkflowRuntimeIssue` diagnostics before run/session admission,
  even when the decision includes a selected runtime id.
- 2026-05-10: Continued Milestone 5 by removing runtime-registry
  technical-fit fallback selection. Unmatched overrides and incomplete
  candidate/runtime state now produce unselected typed diagnostic decisions
  instead of synthetic override candidates or conservative selected runtimes.
- 2026-05-10: Continued Milestone 5 by retiring fallback-named technical-fit
  DTO values. Runtime-registry, embedded-runtime projection, and
  workflow-service tests now use automatic/explicit decisions plus typed
  missing-candidate or missing-runtime-state reasons instead of
  `ConservativeFallback`, `OverrideFallback`, or `conservative_fallback`
  serialized values.
- 2026-05-10: Continued Milestone 5 by retiring workflow-service
  `runtime_hint` capability extraction as an executable backend requirement
  source. The current Juggernaut image workflow now records `backend_key:
  pytorch` on the image-generation node, and legacy `runtime_hint` values are
  ignored by capability extraction instead of being canonicalized into backend
  requirements.
- 2026-05-10: Continued Milestone 5 by stopping runtime-setting diagnostics
  from promoting raw `device` settings into selected device ids. Runtime
  settings still retain bounded sanitized metadata, but selected device
  class/id fields now remain absent unless canonical lifecycle/device-decision
  facts provide them.
- 2026-05-10: Continued Milestone 5 by removing the embedded host helper's
  llama.cpp raw-auto model start path. When a requested llama.cpp model is not
  already active, session load now fails closed with a runtime diagnostic until
  the canonical runtime/device decision path supplies a selected runtime and
  device.
- 2026-05-10: Continued Milestone 5 by removing workflow-service graph helper
  special handling for legacy `runtime_hint`. Edge insert prioritization and
  KV-cache memory-impact backend-change detection now use current backend/model
  fields instead of treating `runtime_hint` as a runtime/backend signal.
- 2026-05-10: Continued Milestone 5 by removing embedded-runtime dependency
  preflight backend preference from legacy `runtime_hint`. Backend preference
  now comes from `backend_key` or package/requirements facts until typed
  workflow backend intent is wired.
- 2026-05-10: Continued Milestone 5 by removing `runtime_hint` from embedded
  host llama.cpp model-path detection. The helper now recognizes current
  backend/package facts only when deciding whether an inference node targets
  llama.cpp.
- 2026-05-10: Continued Milestone 5 by removing `runtime_hint` from
  embedded-runtime embedding workflow llama.cpp detection. Non-embedding
  llama.cpp inference detection now uses current backend/package facts.
- 2026-05-10: Continued Milestone 5 by removing `runtime_hint` from node-engine
  dependency-preflight backend preference selection. Node-engine preflight now
  uses `backend_key`, package facts, or inferred task/model facts, and retired
  inference node guidance points to `backend_key`.
- 2026-05-10: Continued Milestone 5 by retiring
  `InferenceExecutionRequest.runtime_hint` from the public inference DTO and
  node-engine request builders. Typed inference execution requests no longer
  carry backend/runtime preference strings; scheduler-facing contracts remain
  the owner for backend/runtime/device decisions. Broad node-engine verification
  also exposed and fixed a package-facts text-generation route that could still
  enter the old llama.cpp execution branch from backend hints; prompt-bearing
  package-facts text requests now use the typed gateway path.
- 2026-05-10: Continued Milestone 5 by removing graph-visible
  `runtime_hint` from the canonical `llm-inference` descriptor, frontend mock
  definitions, built-in templates, and the tracked Tiny SD saved workflow.
  These current graph producers now use `backend_key`; no descriptor alias or
  saved-workflow compatibility shim was added.
- 2026-05-10: Continued Milestone 5 by updating workflow-service current graph
  canonicalization/session fixtures to use `backend_key`. Capability tests that
  prove legacy runtime hints are ignored remain as explicit negative coverage.
- 2026-05-10: Continued Milestone 5 by validating llama.cpp sidecar
  `DeviceConfig.device` selectors before inference, embedding, or reranking
  startup stops an existing runtime or spawns `llama-server`. Malformed
  selectors now fail locally instead of being passed through or hidden from
  selected-device facts.
- 2026-05-10: Continued Milestone 5 by applying the same backend-local device
  selector validation to the dedicated llama.cpp embedding sidecar startup
  path before it builds command arguments or calls the process spawner.
- 2026-05-10: Continued Milestone 5 by removing the remaining fallback-named
  runtime-registry technical-fit candidate source. Runtime capability
  candidates now serialize as `runtime_capability_facts`, and the retired
  `runtime_capability_fallback` value is rejected rather than accepted through
  an alias.
- 2026-05-10: Continued Milestone 5 by removing frontend synthetic device
  options after backend discovery failure. `DeviceConfig.svelte` now renders
  only backend-confirmed device options and keeps the selector unavailable when
  discovery returns no usable facts.
- 2026-05-10: Continued Milestone 5 by removing gateway mode-info projection
  from raw `BackendConfig.device`. Runtime fact snapshots now carry
  `active_resolved_device` only when the active llama.cpp runtime descriptor
  exposes a canonical selected device id.
- 2026-05-10: Continued Milestone 5 by adding runtime-variant capability facts
  to the backend capability contract and projecting them through workflow
  service, embedded runtime, and TypeScript workflow mirrors. llama.cpp and
  PyTorch now publish CPU plus typed unavailable accelerator variant facts, and
  Candle publishes unavailable CPU/CUDA/macOS Metal placeholders while keeping
  image generation unavailable. vLLM and MLX capability providers remain a
  follow-up because they are not registered backends yet. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p inference backend::capability_tests`,
  `cargo test -p inference --all-features test_capabilities`,
  `cargo check -p inference --no-default-features`,
  `cargo test -p pantograph-embedded-runtime runtime_capabilities`,
  `cargo test -p pantograph-workflow-service --test contract workflow_capabilities_contract_snapshot`,
  `npm run typecheck`, and `git diff --check`. The obsolete
  `npm run -w frontend check:types` command failed because the root no longer
  has a `frontend` workspace; root `npm run typecheck` is the current
  equivalent and passed.
- 2026-05-10: Continued Milestone 5 with a documentation-only hardware/offload
  reservation slice. `06-device-runtime-selection.md` now states that
  ROCm/HIP, Vulkan, XPU/iGPU, OpenVINO, hybrid/offload, remote hardware
  plugins, and MLX are reserved contract space only until typed provider,
  probe, and admission facts exist. Verification passed:
  `rg -n "Future Support Reservation Notes|ROCm/HIP|Remote hardware plugins|MLX" docs/plans/current-image-generation-graphs/06-device-runtime-selection.md`
  and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding typed device policy intent to
  workflow-service and runtime-registry technical-fit requests and projecting
  it through embedded-runtime. The slice carries `auto` or explicit
  CPU/CUDA/Metal/MPS intent without changing selector ranking or translating
  backend-local device strings. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `npm run typecheck`, and `git diff --check`. Follow-up remains: technical-fit
  candidates and decisions still need runtime variant, selected device, and
  device diagnostics before explicit unavailable devices can be rejected by
  scheduler admission.
- 2026-05-10: Continued Milestone 5 by extending technical-fit candidates and
  selected decisions with runtime variant id, typed selected device class/id,
  resource estimate, optional observed-throughput hint, and bounded device
  diagnostics. Runtime-registry now copies those candidate facts into selected
  decisions, embedded-runtime projects backend runtime-variant capability
  diagnostics into candidates and decisions, and workflow-service/TypeScript
  mirrors carry the fields without inferring executable device choices.
  Verification passed: `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by enforcing explicit device policy inside
  runtime-registry technical-fit selection. Explicit CPU/CUDA/Metal/MPS
  requests now filter candidates by typed device class/id and return an
  unselected `ExplicitDeviceUnavailable` diagnostic when no candidate matches
  instead of choosing CPU, auto, or another candidate. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`, and
  `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding runtime id and runtime variant id
  to technical-fit override intent. Workflow-service normalizes the new intent,
  embedded-runtime projects it into runtime-registry selector input, and
  runtime-registry matches it against candidate facts with typed explicit
  runtime/variant override reasons instead of synthesizing fallback candidates.
  Verification passed: `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `npm run typecheck`, and `git diff --check`. The first chained verification
  attempt hit unrelated Pumas temporary SQLite readonly failures in two
  embedded-runtime technical-fit tests; rerunning that suite immediately
  passed.
- 2026-05-10: Continued Milestone 5 by making runtime-registry explicit
  overrides respect canonical candidate eligibility. A backend/runtime/model
  match with unsupported task/model facts now returns an unselected explicit
  override decision instead of bypassing compatibility as an executable
  selection. Verification passed: `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`, and
  `git diff --check`.
- 2026-05-10: Continued Milestone 5 by blocking workflow-service admission on
  error-severity technical-fit device diagnostics. Explicit unavailable device
  decisions now become blocking `WorkflowRuntimeIssue` diagnostics even when
  they do not also carry missing-candidate or missing-runtime-state reasons.
  Verification passed: `cargo fmt --all -- --check`,
  `cargo test -p pantograph-workflow-service technical_fit`, and
  `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding a cross-layer workflow
  technical-fit fixture shared by Rust serde coverage and the existing Node
  frontend test harness. The fixture pins typed runtime variant, explicit
  device policy, selected device facts, estimates, throughput hints, and
  bounded diagnostics without adding a new test platform. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-workflow-service --test contract workflow_technical_fit_cross_layer_fixture_deserializes`,
  `node --experimental-strip-types --test src/services/workflow/WorkflowService.commands.test.ts`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by making runtime-registry auto mode fail
  closed on equally ranked candidates. The selector now returns an unselected
  `ambiguous_auto_resolution` diagnostic instead of using deterministic
  candidate-id ordering as executable selection. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`, and
  `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding a runtime-registry
  `no_valid_candidate` error diagnostic for automatic technical-fit decisions
  with no eligible candidate. Explicit device misses still use the more
  specific `explicit_device_unavailable` diagnostic. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`, and
  `git diff --check`.
- 2026-05-10: Continued Milestone 5 by retiring the
  `deterministic_tie_break` technical-fit reason from runtime-registry,
  embedded-runtime projection, workflow-service, and the TypeScript workflow
  mirror. Auto ambiguity now has one canonical representation:
  `ambiguous_auto_resolution` diagnostics. Verification passed:
  `rg -n "DeterministicTieBreak|deterministic_tie_break" crates/pantograph-runtime-registry crates/pantograph-workflow-service crates/pantograph-embedded-runtime src/services/workflow`,
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding a runtime-registry
  technical-fit serde fixture and integration test. The fixture round-trips the
  public request/decision DTOs and verifies selector output with typed runtime
  variant/device facts. `serde_json` was added as a crate-local dev-dependency,
  so `Cargo.lock` changed as part of this test slice. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry --test technical_fit_contract`,
  `cargo test -p pantograph-runtime-registry technical_fit`, and
  `git diff --check`.
- 2026-05-10: Continued Milestone 5 by preserving bounded diagnostics from the
  best matching but ineligible explicit technical-fit override candidate.
  llama.cpp-for-diffusion style explicit backend requests now return the
  candidate's `backend_incompatible` diagnostic on the unselected decision
  instead of only reason codes. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`, and
  `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding synthetic bounded diagnostics
  for explicit override intent that has no matching technical-fit candidate.
  Runtime variant misses now return `missing_runtime_variant` on the unselected
  decision instead of reason codes alone. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`, and
  `git diff --check`.
- 2026-05-10: Continued Milestone 5 by surfacing backend-projected selected
  device class in frontend diagnostics DTOs and fact rows. The presenter
  renders `selected_device_class` directly and does not derive class from
  selected device ids, raw backend config, runtime settings, or diagnostic
  payload JSON. Verification passed:
  `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by surfacing backend-projected selected
  device class in the scheduler run-list placement column. The scheduler
  presenter combines typed `selected_device_class` and `selected_device_id`
  fields for display and search without parsing scheduler payload JSON or
  deriving class from device id strings. Verification passed:
  `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding a frontend scheduler Device Class
  filter that forwards the backend-supported `selected_device_class` run-list
  query field. The store and presenter use the typed projection field for
  option/filter state and do not derive class from selected device id strings
  or scheduler payload JSON. Verification passed:
  `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts`,
  `node --experimental-strip-types --test src/stores/schedulerRunListStore.test.ts`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding `selected_device_class` as a
  diagnostics-ledger run-list facet kind and rendering diagnostics comparison
  counts from that backend facet. The facet groups the typed projection column
  and does not derive class from selected device ids or raw payload JSON.
  Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_projects_inference_diagnostic_selected_facts`,
  `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding a diagnostics comparison filter
  for selected device class. The UI and presenter use typed
  `selected_device_class` projection fields for option/filter state and do not
  parse selected device ids or scheduler payload JSON. Verification passed:
  `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding a diagnostics comparison filter
  for selected backend. The UI and presenter use typed `selected_backend_key`
  projection fields and do not infer backend choice from runtime ids, selected
  device ids, or scheduler payload JSON. Verification passed:
  `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
  `npm run typecheck`, and `git diff --check`. A selected-runtime-variant
  follow-up remains because current active llama.cpp runtime descriptors do not
  yet expose a backend-owned runtime variant id.

## Commit Cadence Notes

- Commit the plan as its own documentation slice.
- During implementation, inspect `git status` before starting each slice.
- Do not begin implementation with dirty source, test, config, lockfile,
  generated, or build artifacts unless the user explicitly accepts those
  changes for the slice. Markdown plan files may be dirty only while the plan is
  being updated.
- Commit after each logical slice is complete and verified.
- Keep saved workflow cleanup, stale diagnostics contracts, frontend rendering,
  and PyTorch/diffusers execution as separate commits unless a vertical slice
  requires a small cross-layer commit.
- Follow commit format/history cleanup rules from `COMMIT-STANDARDS.md`.

## Optional Worker Assignment

Use only if implementation begins from a clean integration commit and slices
can be assigned without overlapping write sets. Shared contracts, saved
workflow files, generated TypeScript/Rust DTOs, lockfiles, READMEs, ADRs, and
this plan are serial integration-owner work unless explicitly reassigned in a
worker wave.

If workers are used, create
`docs/plans/current-image-generation-graphs/coordination-ledger.md` before the
wave starts. Worker reports go under
`docs/plans/current-image-generation-graphs/worker-reports/` and must list
changed paths, tests run, standards concerns, and any discovered out-of-scope
issues without editing shared contracts directly.

| Owner/Agent | Scope | Primary Write Set | Allowed Adjacent Write Set | Forbidden/Shared Files | Output Contract | Handoff Checkpoint |
| ----------- | ----- | ----------------- | -------------------------- | ---------------------- | --------------- | ------------------ |
| Integration Owner | Milestone 0 contracts, saved workflow cleanup, generated DTO alignment, README/ADR updates, worker integration | `docs/plans/current-image-generation-graphs/`, `.pantograph/workflows/`, shared DTO modules, generated type outputs, module READMEs | Tests that directly validate the serial contract changes | None; this owner coordinates shared files | Atomic commits with standards spot-check notes | Before any worker wave and after each worker integration |
| Worker A | Backend stale graph diagnostic DTOs and validation after contracts are frozen | `crates/pantograph-workflow-service/src/graph/`, related graph tests | Narrow workflow-service tests that consume graph diagnostics | Saved workflow files, generated frontend types, inference backend files, lockfiles | Tests plus report listing changed files and any standards concerns | After Milestone 3 tests pass in worker workspace |
| Worker B | Frontend IO inspector stale graph rendering after backend DTO shape is frozen | `src/components/workbench/`, `src/services/workflow/` presenter/type consumers, related frontend tests | `packages/svelte-graph/` only if presenter extraction requires it and integration owner approves | Backend DTO definitions, saved workflow files, lockfiles, generated files unless assigned | Tests plus report listing changed files, selectors used, and lifecycle cleanup checks | After backend DTO contract is frozen and generated/handwritten TS types are available |
| Worker C | Device policy, backend adapter candidate facts, and runtime variant contracts after Milestone 0 is frozen | `crates/inference/src/device.rs`, backend adapter contract modules, managed runtime contracts/tests, runtime registry technical-fit tests | Frontend device selector consumers only if generated DTO shape is frozen | Saved workflow files, PyTorch worker image execution, graph diagnostics, lockfiles | Tests plus report listing variant/device DTOs, adapter candidate facts, no-fallback checks, scheduler/inference ownership boundaries, standards gate results, path/resource validation, lifecycle owner, feature/dependency impact, and affected runtime state paths | Before PyTorch/diffusers execution planning consumes backend/device decisions |
| Worker D | PyTorch/diffusers execution planner and backend bridge after execution and device contracts are frozen | `crates/inference/src/`, `crates/node-engine/src/core_executor/`, `crates/inference/torch/`, related tests | `crates/pantograph-embedded-runtime/src/` only for execution-normalization consumers approved by integration owner | Saved workflow files, frontend components, shared graph diagnostics, lockfiles, device contract files unless assigned | Tests plus report listing changed files, worker bridge shape, no-fallback checks, and decomposition decisions | After canonical graph, execution planner, and device-resolution contracts are confirmed |

Worker rules:

- Each worker uses an isolated worktree or temporary clone from the same clean
  integration commit.
- Workers may read broadly but must not edit outside their primary or approved
  adjacent write set.
- If required changes fall outside the assigned write set, workers record them
  in the report instead of editing them.
- Integration owner reviews reports, verifies write sets, integrates one worker
  at a time, resolves conflicts in a separate integration commit, runs the
  wave's verification, and updates this plan before starting another wave.
- Worker reports are stored under the plan's `worker-reports/` directory if
  workers are used.

## Re-Plan Triggers

- Pumas 0.6.0 selected-model detail cannot resolve the Juggernaut model id.
- Existing PyTorch worker diffusion support cannot load Pumas diffusers
  directory package facts without a contract change.
- Runtime readiness cannot distinguish PyTorch base availability from
  diffusers dependency availability.
- Runtime readiness cannot represent multiple runtime variants for one
  managed backend release without duplicating binary-management ownership.
- Backend probes cannot provide enough facts to distinguish explicit device
  unavailability from auto-selection behavior.
- Backend adapter candidate facts force scheduler ranking, queue policy, or
  learned placement decisions into the inference crate.
- Scheduler admission needs to inspect raw backend command strings to choose a
  backend/runtime/device.
- A backend adapter requires a global runtime, untracked task, unbounded queue,
  unbounded listener, non-loopback local service, or process lifecycle outside a
  composition-root owner.
- Runtime roots, executable paths, dynamic-library paths, Pumas package paths,
  artifacts, or worker-visible paths cannot be validated through shared
  allowed-root handling.
- Resource estimate, dimensions, token/context, byte-range, or output-size
  calculations cannot be expressed with checked arithmetic and typed failures.
- Runtime feature/dependency changes cannot pass default, no-default-features,
  and all-features checks for affected public crates.
- IO inspector cannot consume graph diagnostics without creating a duplicate
  graph read model.
- Tests reveal that saved workflow JSON still embeds large generated image
  bodies after graph execution.
- Candle is already selected by an existing backend policy for diffusion
  package facts.
- Fixing the graph requires changing public inference request/result contracts.
- Standards spot checks reveal new production `unwrap()`/`expect()` use,
  stringly public planner state, unvalidated paths/dimensions, unowned async
  tasks, unbounded queues, frontend polling loops, or large-file threshold
  crossings without decomposition review.
- A worker needs to edit outside its assigned write set or a shared contract
  changes after worker implementation begins.

## Recommendations

- Prefer `backend_key = "pytorch"` for the first saved Juggernaut workflow and
  other executable diffusion image-generation graphs. Keep `diffusers` as
  package/runtime capability evidence until there is a separately registered
  Diffusers backend.
- Keep stale graph diagnostics separate from workflow-run diagnostics unless
  a stale graph reaches submission, admission, or execution. This avoids
  polluting the run ledger with editor/load validation facts.
- Use Tiny SD Turbo as the first executable image-generation vertical slice,
  then validate Juggernaut after the small-model path proves the backend,
  artifact, and UI contracts.

## Completion Summary

### Completed

- Initial plan documentation committed.
- Milestone 0 contract gate completed. Production behavior is unchanged; the
  slice freezes contracts and identifies the implementation/test fixtures for
  later vertical slices.
- Milestone 1 completed.
- Milestone 2 completed for tracked workflow/template cleanup, canonicalization
  split, no-rewrite persistence behavior, Pumas diffusion selector/probe task
  projection, and documentation.
- Milestone 3 partially completed for the backend diagnostic DTO, bounded
  payload contract, structured graph contract-validation classifier, and saved
  graph/edit-session/run inspection projection, plus submit/admission blocking
  details.
- Milestone 4 started for run-inspection graph stale markers backed by
  backend `graph_diagnostics`.
- Milestone 4 saved-graph inspection transport is available through
  `workflow_graph_inspect`; IO inspector rendering remains a follow-up slice.
- Milestone 4 saved-graph presenter helpers are available for IO inspector
  rendering without deriving paths from workflow display names or inventing run
  context.
- Milestone 4 saved-graph inspection mode now renders in IO Inspector when no
  active run is selected.
- Milestone 4 selected run nodes now show backend stale-node diagnostic details
  in the I/O details panel.
- Milestone 4 continues under the existing Node test strategy; no Vitest,
  Playwright, jsdom, or Svelte Testing Library dependency changes are part of
  this plan pass.
- Milestone 4 now has a neutral graph-inspection presenter wrapper for saved
  graph display; larger run component/module renaming is deferred.
- Milestone 4 saved-graph path and selected-node fallback rules are covered in
  the presenter layer under the existing Node test strategy.
- Milestone 5 device/runtime contract gate is in place in
  `crates/inference/src/device_contracts/`, with strict parser rejection for
  invalid identifiers and selected-candidate errors for zero or multiple
  candidates.
- Milestone 5 no longer has the `DeviceBackend::from_id` unknown-to-auto or
  malformed-ordinal-to-zero path. Backend-local llama.cpp selector parsing is
  typed and fallible.
- Milestone 5 now rejects invalid llama.cpp runtime-start device selectors
  before sidecar startup. Default backend config represents explicit `auto`
  intent instead of relying on absent raw device state.
- Milestone 5 has Rust public-contract fixtures for runtime variant capability
  and backend execution decision shapes. Cross-boundary fixtures for frontend,
  diagnostics-ledger, Python worker, and persisted state remain pending until
  those boundaries are touched.
- Milestone 5 has a typed llama.cpp adapter projection for resolved device
  selectors. It still needs scheduler/runtime consumers to stop passing
  `DeviceConfig` as raw selected state.
- Milestone 5 runtime-load dependency resolution now consumes a typed resolved
  device decision. Later slices still need managed runtime variant state and
  active-runtime/lifecycle projection to carry the same selected facts.
- Milestone 5 active llama.cpp runtime descriptors now carry selected device
  facts when backend-local state is resolved. Lifecycle events, diagnostics
  ledger projection, run inspection facts, and scheduler admission still need
  to consume resolved backend/runtime/device decisions end to end.
- Milestone 5 lifecycle event DTOs now include the selected device class field.
  Gateway population, diagnostics-ledger persistence/projection, and run
  inspection still need follow-up slices to consume canonical resolved device
  facts rather than raw config strings.
- Milestone 5 gateway lifecycle events now emit selected device class/id from
  canonical active runtime facts. Diagnostics-ledger persistence/projection,
  run inspection facts, scheduler admission, and managed runtime variant
  readiness still need follow-up slices.
- Milestone 5 inference diagnostic ledger payloads now retain selected device
  class.
- Milestone 5 diagnostics run-list/run-detail projections, workflow
  diagnostics query DTOs, and run-inspection records now expose selected device
  class directly. UI presentation still needs to decide where to display the
  field, and scheduler admission/managed runtime readiness still need
  end-to-end selected backend/runtime/device replacement.
- Milestone 5 workflow-service admission now blocks fallback-shaped
  technical-fit decisions instead of treating them as warning-only selected
  runtime facts. Runtime-registry and embedded-runtime producer-side fallback
  synthesis still needs replacement.
- Milestone 5 runtime-registry technical-fit selection no longer emits
  executable fallback decisions, and fallback-named technical-fit DTO variants
  have been removed from runtime-registry/workflow-service contracts.

### Deviations

- Milestone 0 did not add executable tests because it is a pre-implementation
  contract-freeze slice. It names the first failing acceptance test or fixture
  for every implementation milestone before source changes begin.
- Milestone 2 landed in two commits: the first handled graph-shape cleanup and
  no-rewrite canonicalization, and the second handled Pumas selector/probe
  diffusion task projection.
- `crates/workflow-nodes/src/input/puma_lib.rs` remains above the standards
  decomposition-review threshold at 1885 lines. Extraction is deferred because
  this slice needed a narrow graph-task projection change and a module split
  would have expanded the write set beyond the current milestone.
- `crates/pantograph-workflow-service/src/workflow/contracts.rs` and
  `src/services/workflow/types.ts` remain above the standards decomposition
  threshold. This slice touched them only for serial DTO alignment; splitting
  shared Rust/TypeScript contract modules is deferred to a dedicated
  contract-decomposition slice.
- `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/fixtures/execution_hosts.rs`,
  and `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`
  remain above the standards decomposition threshold. The submit/admission
  slice kept its edits local to the existing queue-admission boundary; broader
  workflow-session decomposition is deferred.
- `src/components/workbench/RunGraphSnapshot.svelte` and
  `src/components/workbench/runGraphPresenters.ts` remain above the frontend
  and generic decomposition thresholds. This slice only threaded backend stale
  facts through the existing presenter/component path; saved/run graph
  presenter extraction is deferred to the next Milestone 4 shared-inspection
  slice.
- `src-tauri/src/workflow/commands.rs`, `src-tauri/src/workflow/README.md`,
  `src/services/workflow/WorkflowService.ts`, `src/services/workflow/types.ts`,
  and `src/services/workflow/README.md` remain above the decomposition-review
  threshold. The saved-graph inspection transport slice added only a thin
  command/service boundary and a focused test file; broader command and DTO
  module extraction is deferred.
- `src/components/workbench/README.md` remains above the decomposition-review
  threshold. The saved-graph presenter slice added a focused module entry only;
  broader workbench documentation splitting is deferred.
- `src/components/workbench/IoInspectorPage.svelte` remains above the frontend
  decomposition threshold. The saved-graph UI slice kept the existing run
  inspector behavior intact and introduced a focused
  `SavedGraphInspectionSnapshot.svelte`; a future split should extract the run
  artifact-details panel and saved-graph mode orchestration.

### Follow-Ups

- Pumas P0-P1 should start after Milestone 0 because the Pantograph expected
  package-facts contract is now frozen.
- Continue Milestone 4 by adding component-level accessibility coverage for
  saved graph selection/focus and selected stale run-node details.
- Continue Milestone 4 by adding any remaining Node-testable accessibility
  helpers and by deciding whether the run/saved graph display naming cleanup is
  small enough for this milestone or should be deferred to a dedicated
  component decomposition slice.
- Defer broader `RunGraphSnapshot.svelte` and `runGraphPresenters.ts` renaming
  to a dedicated decomposition slice unless a later Milestone 4 bug requires
  touching those files again.
- Remaining focus-preservation coverage is limited by the no-new-harness
  decision. Add more Node-tested state/event helpers only where behavior can be
  verified without pretending to test browser focus.
- Continue Milestone 5 by replacing remaining legacy raw device-string
  execution paths with these contracts. The remaining work includes replacing
  `DeviceConfig` as the sidecar DTO, managed runtime command variant selection,
  frontend device options, technical-fit fallbacks, and node-engine backend
  routing.
- Continue Milestone 5 by wiring selected device class into frontend
  run-inspection presentation only from typed backend projection records, not
  by parsing raw diagnostic payload JSON.
- Continue Milestone 5 by replacing remaining legacy raw device-string
  execution paths and frontend synthetic device choices. Technical-fit fallback
  selection is no longer executable or contract-shaped in the canonical Rust
  path.
- Continue Milestone 5 by comparing technical-fit device policy intent against
  candidate runtime/device facts and returning typed blocking diagnostics for
  unavailable explicit devices before backend load.
- Continue Milestone 5 by rejecting explicit backend/runtime preferences that
  are task/model/platform incompatible, such as llama.cpp for diffusion image
  generation or MLX on Linux/Windows.

### Verification Summary

- `git status --short` inspected before the slice. The user approved ignoring
  untracked SQLite WAL/SHM files and the unrelated proposal markdown.
- Code search verified the relevant existing targets for
  `ConservativeFallback`, override fallback candidates, raw `runtime_hint`,
  retired `diffusion-inference` producers/templates/tests, raw device auto
  behavior, frontend synthetic fallback paths, and artifact single-body
  retention paths.
- No build or unit tests were run for Milestone 0 because the slice changed
  only plan documentation and recorded future acceptance tests.
- `cargo test -p pantograph-workflow-service graph::` passed for the graph
  canonicalization, persistence, registry, and session graph surface touched by
  the slice.
- `node --experimental-strip-types --test src/services/workflow/templateService.test.ts`
  passed for bundled templates and tracked saved image-generation workflow
  fixtures.
- `rg -n "diffusion-inference" crates src packages .pantograph/workflows -g '!target'`
  reports guardrail/test/doc references only after tracked workflow cleanup.
- `cargo test -p workflow-nodes --features model-library puma_lib` passed for
  Pumas selector/probe diffusion task projection, including a live selector
  options path for an imported diffusers bundle.
- A follow-up `rg -n "diffusion-inference" crates src packages .pantograph/workflows -g '!target'`
  reports only `.pantograph/workflows/README.md`; no tracked saved workflow,
  bundled template, or executable producer emits the retired node.
- `cargo test -p pantograph-workflow-service graph::` passed for the Milestone
  3 DTO/classifier foundation, including serde round-trip, bounded diagnostic
  details, retired and unknown node classification, and missing edge
  endpoint/handle diagnostics.
- `cargo test -p pantograph-workflow-service graph::inspection` and
  `cargo test -p pantograph-workflow-service workflow_graph_inspection` passed
  for the saved graph inspection projection and service facade replay path.
- `cargo test -p pantograph-workflow-service graph::session_contract` passed
  for edit-session graph snapshot stale diagnostic projection.
- `cargo test -p pantograph-workflow-service run_graph`,
  `cargo test -p pantograph-workflow-service workflow_run_inspection_query_returns_factual_run_snapshot_parts`,
  and `npm run typecheck` passed for historic run graph stale diagnostics and
  run-inspection transport type coverage.
- `cargo test -p pantograph-workflow-service workflow_service_stale_graph_envelope_includes_structured_details`,
  `cargo test -p pantograph-workflow-service workflow_execution_session_run_rejects_stale_graph_before_queue_admission`,
  and `cargo test -p pantograph-workflow-service graph::diagnostics` passed for
  structured submit/admission stale graph blocking.
- `node --experimental-strip-types --test src/components/workbench/runGraphPresenters.test.ts`
  and `npm run typecheck` passed for backend-owned stale graph markers in the
  IO inspector run graph presenter/component path.
- `node --experimental-strip-types --test src/services/workflow/WorkflowService.graphInspection.test.ts`,
  `npm run typecheck`, `cargo fmt --check`,
  `cargo test -p pantograph-workflow-service workflow_graph_inspection`, and
  `cargo test -p pantograph workflow_graph` passed for the saved-graph
  inspection Tauri/frontend service boundary. The first attempted Rust command
  used a non-existent package name (`pantograph-tauri`) and was rerun with the
  actual package name (`pantograph`).
- `node --experimental-strip-types --test src/components/workbench/graphInspectionPresenters.test.ts`
  and `npm run typecheck` passed for saved-graph inspection option/display
  presenters. The first focused presenter test run failed because the new
  module used an extensionless ESM import; adding `.ts` to the local presenter
  import fixed the issue.
- `node --experimental-strip-types --test src/components/workbench/graphInspectionPresenters.test.ts src/components/workbench/runGraphPresenters.test.ts`,
  `npm run typecheck`, and `git diff --check` passed for IO Inspector saved
  graph mode and the read-only saved graph snapshot component.
- `node --experimental-strip-types --test src/components/workbench/runGraphPresenters.test.ts`,
  `npm run typecheck`, and `git diff --check` passed after rendering selected
  run-node stale diagnostics in the I/O details panel from backend
  `graph_diagnostics`.
- `rg -n "@testing-library/svelte|vitest|jsdom|svelte.*test|playwright" package.json package-lock.json src packages -g '!node_modules'`
  found no existing Svelte component DOM test harness. The remaining
  component/accessibility verification was not implemented to avoid introducing
  dependency and lockfile changes without an explicit test-harness plan.
- `node --experimental-strip-types --test src/components/workbench/graphInspectionPresenters.test.ts`,
  `npm run typecheck`, and `git diff --check` passed after extracting
  Node-tested saved graph accessible-label and keyboard-selection helpers.
- `node --experimental-strip-types --test src/components/workbench/graphInspectionPresenters.test.ts`,
  `npm run typecheck`, and `git diff --check` passed after adding the neutral
  graph-inspection canvas wrapper over the existing graph canvas presenter.
- `node --experimental-strip-types --test src/components/workbench/graphInspectionPresenters.test.ts`,
  `npm run typecheck`, and `git diff --check` passed after moving saved graph
  path selection and selected-node preservation helpers into the presenter
  layer.
- `cargo fmt --all -- --check`, `cargo test -p inference device_contracts`,
  and `git diff --check` passed for the Milestone 5 device/runtime contract
  gate.
- `cargo fmt --all -- --check`, `cargo test -p inference device::tests`,
  `cargo test -p inference device_contracts`, and `git diff --check` passed
  after replacing the legacy llama.cpp `DeviceBackend::from_id` fallback parser
  with typed errors.
- `cargo fmt --all -- --check`, `cargo test -p inference backend::tests`,
  `cargo test -p inference device::tests`,
  `cargo test -p inference lifecycle_events_do_not_report_auto`, and
  `git diff --check` passed after adding llama.cpp runtime-start device
  validation.
- `cargo fmt --all -- --check`,
  `cargo test -p inference --test device_contracts`, and `git diff --check`
  passed after adding public device/runtime contract fixtures.
- `cargo fmt --all -- --check`, `cargo test -p inference device::tests`,
  `cargo test -p inference --test device_contracts`, and `git diff --check`
  passed after adding llama.cpp selector-to-contract projection.
- `cargo fmt --all -- --check`, `cargo test -p inference runtime_load`,
  `cargo test -p inference --test device_contracts`, and `git diff --check`
  passed after making runtime-load dependency resolution carry a typed
  `DeviceResolutionDecision`.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_projects_inference_diagnostic_selected_facts`,
  `cargo test -p pantograph-diagnostics-ledger existing_v19_schema_adds_scheduler_resource_projection_columns`,
  `cargo test -p pantograph-workflow-service workflow_run_`, and
  `git diff --check` passed after projecting selected device class into
  diagnostics run-list/run-detail records and workflow diagnostics contracts.
  The first workflow-service attempt used two Cargo test filters and failed
  before tests ran; it was rerun with the single `workflow_run_` filter.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-workflow-service technical_fit_preflight_blocks_fallback_selected_backend`,
  `cargo test -p pantograph-workflow-service workflow_preflight`,
  `cargo test -p pantograph-workflow-service workflow_run_honors_blocking_backend_technical_fit_decision`,
  `cargo test -p pantograph-workflow-service session_runtime_preflight`, and
  `git diff --check` passed after workflow-service admission began blocking
  fallback-shaped technical-fit decisions. The first two focused tests failed
  before the early-return gap was fixed; the corrected slice rejects
  fallback/incomplete-state decisions before readiness enforcement can be
  skipped.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`, and
  `git diff --check` passed after runtime-registry stopped synthesizing
  override fallback candidates and stopped selecting conservative fallback
  runtime candidates.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit_preflight_blocks_missing_candidate_selected_backend`,
  `cargo test -p pantograph-workflow-service workflow_preflight`,
  `cargo test -p pantograph-workflow-service workflow_run_honors_blocking_backend_technical_fit_decision`,
  `cargo test -p pantograph-workflow-service session_runtime_preflight`, and
  `git diff --check` passed after fallback-named technical-fit DTO variants
  were removed. The first workflow-service verification attempt used multiple
  Cargo filters and failed before tests ran; the tests were rerun with valid
  single-filter commands.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry runtime_capability_source_kind`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `rg -n "RuntimeCapabilityFallback|runtime_capability_fallback" crates/pantograph-runtime-registry crates/pantograph-embedded-runtime crates/pantograph-workflow-service -g '!target'`,
  and `git diff --check` passed after the runtime capability candidate source
  kind was renamed to `runtime_capability_facts`. The search intentionally
  reports only the negative regression test string for the retired wire value.
  The first focused test attempt failed because it introduced a `serde_json`
  dependency assumption in a crate that only depends on `serde`; the test was
  rewritten to use `serde`'s typed string deserializer.
- `node --experimental-strip-types --test src/components/deviceConfigPresenters.test.ts`,
  `npm run typecheck`,
  `rg -n "Provide fallback options|CPU Only|let llama-server choose|buildBackendConfirmedDeviceOptions|deviceLoadError" src/components/DeviceConfig.svelte src/components/deviceConfigPresenters.ts src/components/deviceConfigPresenters.test.ts`,
  and `git diff --check` passed after frontend device options stopped
  synthesizing CPU-only or auto choices when backend discovery fails.
- `cargo fmt --all -- --check`,
  `cargo test -p inference test_mode_info_runtime_facts`,
  `cargo test -p inference test_lifecycle_events_do_not_report_config_only_device_as_selected`,
  and `git diff --check` passed after gateway mode-info runtime facts stopped
  deriving selected devices from raw backend config strings. The first format
  check failed on a wrapping-only rustfmt diff; `cargo fmt --all` was run and
  the check passed afterward.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `npm run typecheck`, and `git diff --check` passed after technical-fit
  candidates and selected decisions began carrying runtime variant, typed
  device, resource-estimate, observed-throughput hint, and device diagnostic
  facts without inferring missing executable choices.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`, and
  `git diff --check` passed after runtime-registry technical-fit selection
  began rejecting unavailable explicit device policies from typed candidate
  device facts.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `npm run typecheck`, and `git diff --check` passed after technical-fit
  overrides began carrying runtime id and runtime variant id intent. The first
  chained embedded-runtime run hit unrelated Pumas temporary SQLite readonly
  failures, and the immediate rerun passed.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-runtime-registry technical_fit`, and
  `git diff --check` passed after explicit overrides started reusing canonical
  candidate eligibility instead of bypassing incompatible task/model facts.

### Traceability Links

- Module README updated: N/A for Milestone 0 because no production module
  ownership changed.
- ADR added/updated: N/A unless implementation changes runtime backend
  ownership boundaries.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: pending.
