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
- 2026-05-10: Continued Milestone 5 by adding `selected_backend_key` as a
  diagnostics-ledger run-list facet kind and rendering diagnostics comparison
  counts from that backend facet. The facet groups the typed projection column
  and does not derive backend choice from runtime ids, selected device ids, or
  raw payload JSON. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_projects_inference_diagnostic_selected_facts`,
  `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding `selected_backend_key` as a
  diagnostics-ledger/workflow-service run-list query filter and wiring the
  scheduler Backend filter through typed frontend store and presenter state.
  The query and local filter path use typed `selected_backend_key` projection
  fields and do not infer backend choice from runtime ids, selected device ids,
  scheduler payload JSON, runtime settings, or backend config strings.
  Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-diagnostics-ledger run_list_projection_drains_lifecycle_events_incrementally`,
  `cargo test -p pantograph-workflow-service workflow_run_list_query_contract_snapshot`,
  `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts src/stores/schedulerRunListStore.test.ts`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by carrying backend-owned
  `selected_runtime_variant_id` from workflow technical-fit decisions into
  scheduler model lifecycle diagnostic payloads and post-preflight reservation
  release payloads. Early admission/reservation-created events stay
  variant-free until technical fit provides the selected variant; the slice
  does not infer variants from runtime ids, backend keys, device facts, or raw
  payload JSON. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
  `cargo test -p pantograph-diagnostics-ledger model_lifecycle_projects_canonical_error_link_without_counting_new_error`,
  and `git diff --check`. Discovered follow-up:
  `cargo test -p pantograph-workflow-service workflow_execution_session_run_records_snapshot_before_execution`
  still fails at its Library usage projection assertion with zero assets; this
  is recorded as a separate projection/test-fragility issue.
- 2026-05-10: Continued Milestone 5 by adding durable diagnostics run-list and
  run-detail projection fields for `selected_runtime_variant_id`, populated
  only from typed scheduler lifecycle payloads and exposed through
  workflow-service/frontend DTOs. The slice bumps projection versions and adds
  nullable schema-repair columns, without deriving variants from runtime ids,
  backend keys, devices, runtime settings, scheduler payload JSON, or backend
  config strings. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-diagnostics-ledger model_lifecycle_projects_canonical_error_link_without_counting_new_error`,
  `cargo test -p pantograph-diagnostics-ledger current_schema_repairs_all_drifted_projection_tables`,
  `cargo test -p pantograph-diagnostics-ledger current_schema_repairs_missing_run_error_projection_columns`,
  `cargo test -p pantograph-workflow-service workflow_run_list_query_contract_snapshot`,
  `cargo test -p pantograph-workflow-service workflow_run_detail_query_contract_snapshot`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by rendering selected runtime variant in
  the scheduler run-list runtime placement column and search path. The
  frontend presenter consumes the typed `selected_runtime_variant_id` field
  directly and does not infer variants from runtime ids, backend keys, device
  ids/classes, backend config strings, or scheduler payload JSON. Verification
  passed:
  `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding selected runtime variant as a
  backend-owned run-list facet and diagnostics comparison filter. Ledger facets,
  workflow-service contract fixtures, TypeScript DTOs, and Diagnostics page
  presenters now use `selected_runtime_variant_id` directly, without splitting
  runtime ids or parsing scheduler payload JSON. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-diagnostics-ledger model_lifecycle_projects_canonical_error_link_without_counting_new_error`,
  `cargo test -p pantograph-workflow-service workflow_run_list_query_contract_snapshot`,
  `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding
  `selected_runtime_variant_id` as a typed run-list query filter and Scheduler
  page filter. The ledger query, facets query, workflow-service request DTO,
  TypeScript request DTO, scheduler store, and presenter all use the typed
  variant field directly. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-diagnostics-ledger model_lifecycle_projects_canonical_error_link_without_counting_new_error`,
  `cargo test -p pantograph-workflow-service workflow_run_list_query_contract_snapshot`,
  `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts src/stores/schedulerRunListStore.test.ts`,
  `npm run typecheck`, and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding unavailable roadmap capability
  facts for vLLM CPU/CUDA and MLX Metal. The embedded runtime now reports these
  typed facts through workflow capabilities while keeping both providers
  unavailable and non-executable. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-embedded-runtime roadmap_runtime_capabilities_report_vllm_and_mlx_placeholders`,
  `cargo test -p pantograph-embedded-runtime runtime_capabilities`,
  `cargo test -p pantograph-embedded-runtime technical_fit`, and
  `git diff --check`. The first format check reported rustfmt wrapping;
  `cargo fmt --all` was run and the check passed.
- 2026-05-10: Continued Milestone 5 by adding embedded-runtime admission
  regression coverage for explicit vLLM and MLX roadmap backend overrides. The
  canonical technical-fit projection now has focused coverage proving these
  preferences produce unselected explicit-override decisions with typed
  unavailable diagnostics rather than fallback candidates. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p pantograph-embedded-runtime roadmap_backend_overrides_reject_without_fallback_selection`,
  `cargo test -p pantograph-embedded-runtime technical_fit`, and
  `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding canonical llama.cpp
  `--list-devices` inventory fact projection. Existing backend-local
  `DeviceInfo` parsing remains available, while the new projection emits
  validated CPU/CUDA device facts and typed diagnostics for unsupported
  backend-local selectors such as Vulkan. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p inference device::tests::parse_llamacpp_inventory_facts`,
  `cargo test -p inference device::tests`, and `git diff --check`. The first
  format check reported rustfmt wrapping; `cargo fmt --all` was run and the
  check passed.
- 2026-05-10: Continued Milestone 5 by adding the PyTorch device probe
  contract. `PyTorchDeviceProbeSnapshot` now projects host-observed CPU/CUDA
  and macOS MPS probe facts into canonical runtime variant readiness facts,
  leaving live probe execution and scheduler admission to later owners.
  Verification passed: `cargo fmt --all -- --check`,
  `cargo test -p inference --features backend-pytorch pytorch_device_probe`,
  `cargo test -p inference --features backend-pytorch test_capabilities`, and
  `git diff --check`. Initial plain test filters matched no PyTorch tests
  because the PyTorch backend is feature-gated; the commands were rerun with
  `--features backend-pytorch`.
- 2026-05-10: Continued Milestone 5 by adding a frontend backend-confirmed
  device submit guard. Device Configuration now writes device config only when
  the selected device is still present in backend-confirmed options, while
  embedding-only saves remain possible. User-visible copy no longer implies
  llama-server owns final auto/GPU choice. Verification passed:
  `node --experimental-strip-types --test src/components/deviceConfigPresenters.test.ts`,
  `npm run typecheck`,
  `rg -n "llama-server owns|let llama-server choose|Select your GPU|frontend-owned auto|CPU Only|Provide fallback options" src/components/DeviceConfig.svelte src/components/deviceConfigPresenters.ts src/components/deviceConfigPresenters.test.ts`,
  and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding a serde fixture for canonical
  llama.cpp device inventory facts. `LlamaCppDeviceInventoryFact` is now a
  public inference DTO with default-empty diagnostics and a JSON fixture that
  round-trips a CUDA projection. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p inference --test device_contracts llamacpp_device_inventory_fact_fixture_preserves_canonical_projection`,
  `cargo test -p inference --test device_contracts`,
  `cargo test -p inference device::tests::parse_llamacpp_inventory_facts`, and
  `git diff --check`. The first fixture run failed because diagnostics lacked
  a serde default; the DTO was fixed and the suite passed.
- 2026-05-10: Continued Milestone 5 by adding a llama.cpp `gpu_layers`
  guardrail test. The test keeps `gpu_layers` in
  `LlamaCppRuntimeSettings`/`DeviceConfig` while asserting canonical
  `InferenceDevicePolicy` does not expose backend-local `gpu_layers`,
  hybrid, offload, or split fields. Verification passed:
  `cargo fmt --all -- --check`,
  `cargo test -p inference gpu_layers_remain_llamacpp_runtime_setting_not_device_policy`,
  and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding serde fixtures for
  `DeviceResolutionRequest` and `BackendExecutionCandidate`. The fixtures
  preserve canonical device policy, runtime/backend/device ids, task/model
  evidence, resource estimates, and observed throughput without accepting raw
  backend-local device strings. Verification passed:
  `cargo test -p inference --test device_contracts`,
  `cargo fmt --all -- --check`, and `git diff --check`. The first format check
  reported rustfmt wrapping in the new test; `cargo fmt --all` was run and the
  check passed.
- 2026-05-10: Continued Milestone 5 by adding a serde fixture for
  `DeviceResolutionDecision`, the resolved device choice consumed by runtime
  load contracts. The fixture preserves the canonical explicit CUDA policy and
  selected `cuda:0` device id without accepting backend-local raw selectors.
  Verification passed: `cargo test -p inference --test device_contracts`,
  `cargo fmt --all -- --check`, and `git diff --check`. The first format check
  reported rustfmt import wrapping; `cargo fmt --all` was run and the check
  passed.
- 2026-05-10: Continued Milestone 5 by replacing inference lifecycle event
  `selected_device_id` raw strings with canonical `InferenceDeviceId` in the
  event DTO and gateway lifecycle plumbing. Lifecycle event deserialization now
  rejects legacy backend-local selectors such as `CUDA0`. Verification passed:
  `cargo test -p inference inference_request_lifecycle_event`,
  `cargo test -p inference test_lifecycle_events_carry_active_runtime_selected_device`,
  `cargo fmt --all -- --check`,
  `rg -n "selected_device_id: Option<String>|selected_device_id.as_deref|selected_device_id: Some\\(\\\"cuda:0" crates/inference/src/types.rs crates/inference/src/gateway.rs crates/inference/src/gateway_tests.rs`,
  and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding an explicit device candidate
  mismatch guard to `BackendExecutionDecision::try_from_selected_candidate`.
  Explicit CUDA policy can no longer construct a selected CPU decision through
  the canonical decision DTO constructor; mismatches return typed
  `DeviceContractError` variants. Verification passed:
  `cargo test -p inference device_contracts::tests::explicit_device_policy_rejects_mismatched_selected_candidate`,
  `cargo test -p inference device_contracts::tests::backend_execution_decision_requires_one_selected_candidate`,
  `cargo test -p inference device_contracts::tests`,
  `cargo fmt --all -- --check`, and `git diff --check`. The first format check
  reported rustfmt wrapping in the new error attribute; `cargo fmt --all` was
  run and the check passed.
- 2026-05-10: Continued Milestone 5 by typing `ServerModeInfo` and
  `RuntimeFactSnapshot` resolved-device fields as canonical
  `InferenceDeviceId` while preserving the serialized string shape consumed by
  frontend and host status readers. Embedded-runtime ledger projection now
  reads canonical ids instead of sanitizing path-shaped selected-device strings.
  Verification passed: `cargo test -p inference runtime_fact_snapshot`,
  `cargo test -p inference test_mode_info_runtime_facts_report_active_runtime_selected_device`,
  `cargo test -p pantograph-embedded-runtime host_runtime_mode_snapshot_copies_runtime_facts_from_mode_info`,
  `cargo test -p pantograph-embedded-runtime hosted_runtime_constructor_syncs_registry_and_derives_capabilities_from_mode_info`,
  `cargo test -p pantograph-embedded-runtime inference_lifecycle_event_adapter_builds_node_status_event_with_backend_context`,
  `cargo test -p pantograph-embedded-runtime inference_diagnostic_event_adapter_drops_path_shaped_runtime_metadata`,
  `cargo fmt --all -- --check`, and `git diff --check`. The first
  embedded-runtime compile exposed ledger consumers and tests still
  constructing raw selected-device strings; those were converted to canonical
  ids or removed where invalid ids are now rejected before ledger projection.
- 2026-05-10: Continued Milestone 5 with a documentation slice updating
  inference and embedded-runtime README invariants for selected/resolved device
  ownership. The READMEs now say lifecycle/status/ledger selected device facts
  come from canonical device DTOs and active runtime descriptors, not raw
  backend config strings or sanitized backend-local metadata. Verification
  passed:
  `rg -n "selected_device_id|InferenceDeviceId|raw backend config|backend-local selected-device" crates/inference/src/README.md crates/pantograph-embedded-runtime/src/README.md`,
  `rg -n "Update relevant module READMEs" docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
  and `git diff --check`.
- 2026-05-10: Continued Milestone 5 by adding a runtime-load serde fixture for
  `RuntimeLoadPhaseRecord`. The fixture locks the dependency-resolved phase
  shape with managed runtime readiness facts, a canonical
  `DeviceResolutionDecision`, and command facts without inferring readiness from
  command-line arguments or raw backend config strings. Verification passed:
  `cargo test -p inference --test runtime_load_contracts`,
  `cargo fmt --all -- --check`, and `git diff --check`. The first format check
  reported rustfmt assertion wrapping; `cargo fmt --all` was run and the check
  passed.
- 2026-05-11: Continued Milestone 5 by typing PyTorch Transformers worker load
  request `device` as `Option<InferenceDeviceId>`. The adapter maps omitted or
  `auto` backend intent to omitted worker device, while concrete worker load
  devices must be canonical ids and legacy selectors such as `CUDA0` are
  rejected before Python. Verification passed:
  `cargo test -p inference --features backend-pytorch test_pytorch_worker_load_envelope_decodes_fixture`,
  `cargo test -p inference --features backend-pytorch test_pytorch_worker_load_envelope_rejects_legacy_device_id`,
  `cargo test -p inference --features backend-pytorch test_pytorch_direct_load_envelope_rejects_legacy_device_id`,
  `cargo test -p inference --features backend-pytorch test_pytorch_load_envelope_maps_pumas_package_facts`,
  `cargo test -p inference --features backend-pytorch test_pytorch_direct_load_envelope_uses_transformers_contract`,
  `cargo test -p inference --features backend-pytorch test_pytorch_transformers_load_args_default_device_auto`,
  `cargo fmt --all -- --check`,
  `rg -n "payload\\.device\\.as_deref|device: Option<String>" crates/inference/src/backend/pytorch_worker_contract.rs crates/inference/src/backend/pytorch.rs crates/inference/src/backend/pytorch_tests.rs`
  reported no matches, and `git diff --check`. Verification deviations fixed
  during the slice: the first cargo command used two filters, so it was rerun
  as separate commands; the existing worker load fixture had a stale nested
  `source_contract_version: 1`, which was updated to `2` so
  `model_source.validate_for_backend_load()` passes.
- 2026-05-11: Continued Milestone 5 by reserving `auto` out of
  `InferenceDeviceId`. Automatic device selection must now be represented by
  `InferenceDevicePolicy::Auto` or by an omitted backend-worker device field,
  not by a concrete device id string. PyTorch worker load-envelope tests now
  reject explicit `"auto"` in `payload.device`. Verification passed:
  `cargo test -p inference device_contracts`,
  `cargo test -p inference --features backend-pytorch test_pytorch_worker_load_envelope_rejects_auto_device_field`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- 2026-05-11: Continued Milestone 5 by replacing the PyTorch audio
  transcription worker request's raw `device: "auto"` field with omitted
  canonical device intent. Rust now types the optional audio worker device as
  `Option<InferenceDeviceId>`, the audio fixture omits the field, and the
  Python worker contract maps omission to backend-local `auto` while rejecting
  explicit `"auto"` or legacy ids such as `CUDA0` when a device field is
  present. Verification passed:
  `cargo test -p inference --features backend-pytorch audio_transcription`,
  `cargo test -p inference --features backend-pytorch test_python_worker_contract_projects_task_profile_loader`,
  `cargo test -p inference --features backend-pytorch test_python_worker_contract_tolerates_additive_load_fields`,
  `cargo fmt --all -- --check`, and `git diff --check`. Verification
  deviation fixed during the slice: one attempted cargo command used two test
  filters and was rerun as separate commands.
- 2026-05-11: Continued Milestone 5 by typing PyTorch worker response device
  facts as `InferenceDeviceId` for loaded-model and live-KV responses. Worker
  responses that report selected device `"auto"` or legacy ids such as `CUDA0`
  now fail decode instead of becoming trusted selected runtime facts.
  Verification passed:
  `cargo test -p inference --features backend-pytorch worker_load_response`,
  `cargo test -p inference --features backend-pytorch save_kv_cache_response`,
  `cargo test -p inference --features backend-pytorch get_loaded_info_response`,
  `cargo test -p inference --features backend-pytorch restore_kv_cache_response`,
  `cargo fmt --all -- --check`, and `git diff --check`. Verification
  deviations fixed during the slice: the first load/save negative tests used
  shorthand canonical-code expectations; they were corrected to the existing
  worker error codes and the focused tests passed.
- 2026-05-11: Re-plan trigger reached before replacing the remaining raw
  device fields found by code search. The remaining `device: Option<String>` /
  `device: String` fields are shared gateway/startup config and backend-local
  llama.cpp runtime settings, not isolated worker DTOs. Directly typing them
  as `InferenceDeviceId` would be wrong because llama.cpp still requires
  adapter-local selectors such as `CUDA0`, while PyTorch worker boundaries now
  require canonical selected device ids or omitted auto intent. The next slice
  needs an explicit typed startup/device-intent design that separates
  scheduler-facing canonical policy from backend-local adapter selectors
  before implementation continues.
- 2026-05-11: Resolved the startup-device re-plan design with a narrow
  inference backend contract slice. `BackendStartupDeviceIntent` now separates
  scheduler-facing `InferenceDevicePolicy`, concrete canonical
  `InferenceDeviceId`, and backend-local llama.cpp `DeviceBackend` selectors.
  The slice does not rewire `BackendConfig.device`; it establishes the typed
  adapter-facing transition contract needed before shared startup fields are
  migrated. Verification passed:
  `cargo test -p inference startup_device`, `cargo fmt --all -- --check`, and
  `git diff --check`.
- 2026-05-11: Continued Milestone 5 by typing the effective
  `LlamaCppRuntimeSettings.device` field as backend-local `DeviceBackend`
  after validation. `BackendConfig.device` still accepts the shared raw
  startup string for now, but normalized llama.cpp runtime settings no longer
  carry a raw device string internally; projection to legacy `DeviceConfig`
  happens only at the sidecar DTO boundary. Verification passed:
  `cargo test -p inference llamacpp_runtime_settings`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- 2026-05-11: Continued Milestone 5 by typing legacy sidecar
  `DeviceConfig.device` as backend-local `DeviceBackend`. The serialized JSON
  shape remains the llama.cpp selector string at the sidecar DTO boundary, but
  invalid selectors and canonical ids such as `cuda:0` now fail serde/input
  decoding instead of becoming runtime state. Server and embedding sidecar
  command construction now projects typed selectors through `DeviceBackend`.
  Verification passed:
  `cargo test -p inference config::tests::device_config`,
  `cargo test -p inference active_runtime_descriptor`,
  `cargo test -p inference start_sidecar_inference_applies_runtime_settings_to_llama_server_args`,
  `cargo test -p inference llamacpp_runtime_settings`,
  `cargo test -p inference embedding_runtime::tests`,
  `cargo test -p inference test_mode_info_runtime_facts_report_active_runtime_selected_device`,
  `cargo test -p inference backend::llamacpp::tests`,
  `cargo fmt --all -- --check`, and `git diff --check`. Discovered issue fixed
  in-slice: model fingerprint hashing was using the display label for typed
  devices; it now hashes the stable backend selector id.
- 2026-05-11: Continued Milestone 5 by typing gateway startup request device
  intent with `BackendStartupDeviceIntent` for inference and embedding start
  requests. Gateway startup config construction now rejects llama.cpp-local
  selectors for PyTorch, canonical device ids for llama.cpp, unresolved
  explicit PyTorch policies without a concrete device id, external attachment
  device intents, and Candle embedding device intents instead of silently
  preserving or ignoring raw startup strings. `BackendConfig.device` remains
  the next shared migration target. Verification passed:
  `cargo test -p inference gateway::tests::start_config`,
  `cargo test -p inference startup_device`,
  `cargo test -p pantograph-embedded-runtime edit_session_execution`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- 2026-05-11: Continued Milestone 5 by migrating shared
  `BackendConfig.device` from `Option<String>` to
  `Option<BackendStartupDeviceIntent>`. llama.cpp runtime settings now accept
  backend-local selectors or explicit scheduler auto policy and reject
  canonical ids/unresolved explicit policies; PyTorch startup accepts canonical
  ids or auto policy and rejects llama.cpp-local selectors; node-engine
  llama.cpp settings parse workflow device values into typed backend-local
  selector intent before building backend config. Verification passed:
  `cargo test -p inference backend::tests`,
  `cargo test -p inference gateway::tests::start_config`,
  `cargo test -p inference backend::llamacpp::tests`,
  `cargo test -p node-engine --features inference-nodes backend_config_applies_llamacpp_runtime_settings`,
  `cargo test -p node-engine --features inference-nodes runtime_settings_match_compares_reload_required_performance_settings`,
  `cargo test -p node-engine --features inference-nodes gateway_match_rejects_different_runtime_settings`,
  `cargo test -p inference --features backend-pytorch test_pytorch_worker_load_envelope`,
  `cargo test -p pantograph-embedded-runtime edit_session_execution`,
  `cargo fmt --all -- --check`, and `git diff --check`. Verification
  deviation: an attempted node-engine command used two Cargo filters and
  failed before tests ran; the filters were rerun separately and passed.
- 2026-05-11: Continued Milestone 5 by typing the PyTorch test-only
  Transformers load-args helper device field as `Option<InferenceDeviceId>`.
  Omitted auto intent now remains omitted in the helper instead of becoming the
  raw string `"auto"`, matching the worker envelope contract. Verification
  passed:
  `cargo test -p inference --features backend-pytorch test_pytorch_transformers_load_args`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- 2026-05-11: Continued Milestone 5 by typing PyTorch direct/package model load
  device inputs as `Option<InferenceDeviceId>`. Node-engine PyTorch execution
  now parses workflow device input at the adapter boundary, treats omitted or
  explicit `auto` as `None`, and rejects legacy ids such as `CUDA0` before
  calling the inference backend. Verification passed:
  `cargo test -p inference --features backend-pytorch test_pytorch_direct_load_envelope`,
  `cargo test -p inference --features backend-pytorch test_pytorch_load_envelope`,
  `cargo test -p inference --features backend-pytorch test_can_reuse_loaded_model_requires_matching_request`,
  `cargo test -p node-engine --features pytorch-nodes pytorch_load_device_from_inputs`,
  `cargo fmt --all -- --check`, and `git diff --check`. Verification
  deviation: the first PyTorch test attempt used two Cargo filters and failed
  before tests ran; both filters were rerun separately and passed.
- 2026-05-11: Continued Milestone 5 by removing the legacy managed runtime
  root probe from `managed_install_dir`. Managed runtime command resolution,
  projection, install, and remove paths now resolve only under the canonical
  `app_data/third-party/runtimes` tree instead of accepting a retired
  `app_data/runtimes` directory when it happens to exist. Verification passed:
  `cargo test -p inference managed_runtime::paths`,
  `cargo fmt --all -- --check`, and `git diff --check`. Discovered issue fixed
  in-slice: the legacy path probe was a no-legacy violation. Remaining
  follow-up: shared allowed-root validation for runtime roots, executables,
  dynamic libraries, Pumas package paths, artifact paths, and worker-visible
  paths.
- 2026-05-11: Continued Milestone 5 by removing the Linux llama.cpp hidden CPU
  executable fallback for explicit CUDA device requests. `--device CUDA*` now
  requires `cuda/llama-server` and fails command resolution when that runtime
  variant is missing, instead of using the CPU executable. Verification passed:
  `cargo test -p inference managed_runtime::llama_cpp_platform::linux::tests`,
  `cargo fmt --all -- --check`, and `git diff --check`. Verification
  deviation: the first focused test run used an overly strict
  `LD_LIBRARY_PATH` assertion and the first format check found rustfmt-only
  wrapping; both were corrected and rerun successfully. Remaining follow-up:
  migrate managed-runtime command-resolution errors from `String` to typed
  runtime-variant diagnostics.
- 2026-05-11: Continued Milestone 5 by typing managed-runtime command
  resolution errors. `resolve_binary_command` now returns
  `ManagedRuntimeCommandResolutionError`, Linux missing-CUDA command resolution
  emits a typed `MissingRuntimeVariant` error containing the canonical
  `missing_runtime_variant` diagnostic for `llama_cpp.cuda`, and existing
  string-returning facades stringify that error only at their boundary.
  Verification passed:
  `cargo test -p inference managed_runtime::contracts::tests`,
  `cargo test -p inference managed_runtime::llama_cpp_platform::linux::tests`,
  `cargo test -p inference managed_runtime::operations`,
  `cargo fmt --all -- --check`, and `git diff --check`. Verification
  deviation: the first compile found one remaining state-resolution `String`
  error inside command resolution, which was converted into a typed `State`
  variant; rustfmt-only wrapping was also corrected.
- 2026-05-11: Continued Milestone 5 by updating technical-fit and
  scheduler/diagnostics fixture runtime variant ids from slash-shaped examples
  to canonical dot-shaped ids such as `llama_cpp.cuda` and `pytorch.cuda`.
  Verification passed:
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`,
  `cargo test -p pantograph-workflow-service --test contract workflow_technical_fit_cross_layer_fixture_deserializes`,
  `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts src/components/workbench/diagnosticsPagePresenters.test.ts src/stores/schedulerRunListStore.test.ts src/services/workflow/WorkflowService.commands.test.ts`,
  `bash -lc 'if rg -n "[a-z0-9_-]+/[a-z0-9_-]+/(cpu|cuda|metal)|runtime-a/cuda|runtime-b/metal|llama_cpp/|pytorch/" crates/pantograph-embedded-runtime crates/pantograph-workflow-service src -g "*.rs" -g "*.ts" -g "*.svelte" -g "*.json"; then exit 1; else exit 0; fi'`,
  and `git diff --check`. Verification deviation: initial Node and
  workflow-service contract runs caught two missed fixture/search values; those
  were corrected and rerun successfully.
- 2026-05-11: Continued Milestone 5 by adding typed runtime variant identity to
  managed-runtime catalog versions, projected version statuses, and persisted
  installed versions. Existing llama.cpp managed installs default to
  `llama_cpp.cpu`; variant-specific install jobs, retained artifacts, progress
  snapshots, selected variant state, and CUDA/Metal readiness remain follow-up
  slices. Verification passed:
  `cargo test -p inference managed_runtime::catalog`,
  `cargo test -p inference managed_runtime::operations`,
  `cargo test -p inference runtime_load`,
  `cargo test -p pantograph-embedded-runtime managed_runtime`,
  `cargo test -p pantograph-embedded-runtime runtime_capabilities`,
  `cargo fmt --all -- --check`, and `git diff --check`. Verification
  deviation: the first embedded-runtime compile caught a test-module import
  scope issue for `RuntimeVariantId`, and the first format check reported
  rustfmt-only wrapping; both were corrected and rerun.
- 2026-05-15: Continued Milestone 6 with the selected model-ref normalization
  slice. The smallest useful vertical slice was limited to workflow-service
  execution-plan ownership: `WorkflowExecutionPlanModelRef` now parses selected
  model identity once at the workflow-service boundary, canonicalizes raw
  Pumas model ids to `pumas://models/...`, preserves already-prefixed Pumas
  refs, rejects local paths and unsupported URI shapes, and maps invalid values
  to typed `WorkflowExecutionPlanError::InvalidSelectedModelRef` diagnostics.
  Admission compares capability model ids and selected technical-fit model ids
  through the same canonical value object, so raw and prefixed forms match
  without double-prefixing or downstream repair. The slice preserves the
  no-fallback/no-legacy rule by failing invalid selected identity before
  embedded-runtime projection, scheduler history, runtime readiness, or worker
  dispatch can consume it. Verification passed:
  `cargo test -p pantograph-workflow-service workflow_execution_plan --lib`,
  `cargo check -p pantograph-workflow-service`,
  and `cargo fmt -p pantograph-workflow-service -- --check`. Verification
  deviation: `cargo test -p pantograph-embedded-runtime workflow_execution_plan_projection --lib`
  could not reach projection coverage because an unrelated embedded-runtime
  fixture in `crates/pantograph-embedded-runtime/src/model_dependencies_tests.rs`
  constructs `PortOptionsQuery` without the newer required `context` field.
  That compile blocker is deferred to a separate test-fixture cleanup slice;
  the model-ref projection assertion remains an explicit follow-up.
- 2026-05-15: Continued Milestone 6 with the embedded-runtime selected
  model-ref projection coverage slice. The smallest useful vertical slice was
  limited to fixing the stale embedded-runtime `PortOptionsQuery` test fixture
  by passing an explicit absent context and adding a projection test proving a
  raw selected model id accepted by workflow-service reaches
  `BackendExecutionDecision` as the canonical `pumas://models/...` model ref.
  The projection remains an adapter-only copy of validated workflow-service
  identity; it does not re-parse, re-prefix, repair, or reinterpret raw model
  strings. Verification passed:
  `cargo fmt -p pantograph-embedded-runtime -- --check`,
  `cargo test -p pantograph-embedded-runtime workflow_execution_plan_projection --lib`,
  and
  `cargo test -p pantograph-embedded-runtime puma_lib_option_and_dependency_resolver_agree_on_primary_file_path --lib`.
- 2026-05-15: Continued Milestone 6 with the image-planner model identity
  consistency gate. The smallest useful vertical slice was limited to
  inference image planning and the tests that consume that public path. The
  planner now rejects image-generation planning when
  `BackendExecutionDecision.selected_model_ref` is missing or when the
  scheduler-selected model ref does not match the resolved package-facts model
  ref after deterministic `pumas://models/...` identity comparison. This keeps
  package facts from becoming an implicit scheduler decision and fails with
  typed planner diagnostics before worker dispatch. Gateway and PyTorch worker
  image test helpers now provide the selected model ref explicitly; the
  gateway planning success fixture was corrected to leave
  `denoising_scheduler` unset because explicit scheduler changes remain an
  unsupported option in the current planner slice. Verification passed:
  `cargo fmt -p inference -- --check`,
  `cargo test -p inference image_generation_planner --lib`,
  `cargo test -p inference test_generate_image_from_planning_input --lib`,
  and
  `cargo test -p inference --features backend-pytorch test_pytorch_worker_generate_image_request_maps_from_validated_plan --lib`.
  Verification deviation: the first gateway-focused run exposed the stale
  explicit denoising-scheduler fixture in success tests; the fixture was
  corrected and the focused gateway tests passed.
- 2026-05-15: Continued Milestone 6 with the workflow execution-plan selected
  fact typing slice. The smallest useful vertical slice was limited to
  workflow-service execution-plan DTOs, embedded-runtime projection tests, the
  workflow README, and this plan. `execution_plan_selected_facts.rs` now owns
  validated workflow-service newtypes for selected backend key, runtime id,
  runtime variant id, and concrete selected device id. Invalid selected fact
  shapes fail the workflow execution-plan constructor/deserializer with typed
  `InvalidSelectedDecisionFact` errors before node-engine, embedded-runtime
  projection, scheduler history, runtime readiness, or worker dispatch can
  consume them. Workflow-service remains independent from inference DTOs; the
  embedded-runtime projection still adapts validated workflow values into
  inference `BackendExecutionDecision`. Verification passed:
  `cargo fmt -p pantograph-workflow-service -p pantograph-embedded-runtime -- --check`,
  `cargo test -p pantograph-workflow-service workflow_execution_plan --lib`,
  and
  `cargo test -p pantograph-embedded-runtime workflow_execution_plan_projection --lib`.
- 2026-05-15: Added planning for the next Milestone 6 execution-evidence
  normalization block. The plan now separates shared runtime identity
  spelling from package-fact interpretation: `pantograph-runtime-identity`
  continues to normalize backend/runtime aliases, while an inference-owned
  evidence boundary will interpret package facts, artifact kinds, task
  evidence, backend hints, runtime capabilities, and optional graph
  constraints into typed execution evidence. Diffusers/PyTorch is planned as
  one row in that general system: Diffusers evidence can make PyTorch eligible
  when PyTorch capability facts support it, but `diffusers` remains a
  dependency/package/capability label and must not become a scheduler-selected
  executable backend key. The plan stages implementation into a contract slice,
  embedded-runtime technical-fit/dependency-preflight migration slice, and
  cross-boundary audit/diagnostics slice, with PyTorch image worker execution
  and image-family adapters explicitly out of scope.

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
- Execution-evidence normalization cannot be placed beside inference package
  contracts without making workflow-service depend on inference DTOs,
  node-engine depend on workflow-service, scheduler policy parse full package
  facts, or PyTorch/image-specific code own package-hint-to-backend mapping.
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
- Milestone 6 Option 3 contract foundation slice completed for workflow-service
  execution-plan DTOs, serde validation, bounded diagnostics, README
  traceability, and focused contract tests. Admission production, projection,
  node-engine consumption, diagnostics attachment, and durable recovery remain
  follow-up slices.
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
- Milestone 5 graph-visible full Pumas package-fact/source ports have been
  removed from canonical inference descriptors, frontend definitions, bundled
  templates, tracked current image-generation saved workflows, and node-engine
  dependency input repair.
- Milestone 5 embedded-runtime technical-fit candidate synthesis now joins
  Pumas package facts with runtime capability/runtime-variant/device facts
  before selection, and incomplete Pumas candidate fragments are no longer
  selectable.
- Milestone 6 workflow-service execution-plan admission now owns selected
  model-ref normalization through a validated model-ref type. Raw Pumas model
  ids and already-prefixed `pumas://models/...` refs produce one canonical
  scheduler-history identity, while local paths, unsupported URI schemes, and
  malformed refs fail closed with typed execution-plan diagnostics.
- Milestone 6 embedded-runtime projection coverage now proves the
  workflow-service selected model-ref value reaches
  `BackendExecutionDecision` in canonical `pumas://models/...` form without
  projection-side repair or double-prefixing.
- Milestone 6 image planning now requires the scheduler-selected model ref to
  agree with resolved package facts before returning an
  `ImageGenerationExecutionPlan`; missing or mismatched image model identity
  produces typed planner diagnostics and no worker plan.
- Milestone 6 workflow execution plans now validate selected backend key,
  runtime id, runtime variant id, selected device id, and selected model ref at
  the workflow-service owner boundary using workflow-owned value objects.
  Embedded-runtime projection no longer has to catch malformed selected
  backend/device facts as a normal execution path.

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
- The selected model-ref normalization slice initially left embedded-runtime
  projection verification blocked by a stale `PortOptionsQuery` fixture. The
  following embedded-runtime projection slice repaired that fixture and added
  the focused projection test without expanding execution behavior.

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
  execution paths with these contracts. The remaining work includes migrating
  managed runtime command variant selection, frontend device options, and
  node-engine backend routing.
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
- `cargo test -p inference gpu_layers_remain_llamacpp_runtime_setting_not_device_policy`
  passed for preserving llama.cpp `gpu_layers` as a backend-local runtime
  setting outside canonical cross-backend device policy.
- `cargo test -p inference --test device_contracts` passed for device/runtime
  serde fixtures, including device-resolution request and backend execution
  candidate contract coverage.
- `cargo test -p inference --test device_contracts` passed for the
  device-resolution decision fixture consumed by runtime-load contracts.
- `cargo test -p inference inference_request_lifecycle_event` and
  `cargo test -p inference test_lifecycle_events_carry_active_runtime_selected_device`
  passed for typed lifecycle selected-device ids.
- `cargo test -p inference device_contracts::tests` passed for explicit device
  candidate mismatch rejection in canonical backend execution decisions.
- `cargo test -p inference runtime_fact_snapshot`,
  `cargo test -p inference test_mode_info_runtime_facts_report_active_runtime_selected_device`,
  and focused embedded-runtime host/ledger tests passed for typed runtime fact
  resolved-device ids.
- README verification searches passed for canonical selected/resolved device id
  ownership notes in inference, embedded runtime, and the milestone checklist.
- `cargo test -p inference --test runtime_load_contracts` passed for the
  runtime-load phase serde fixture.
- Focused PyTorch worker load tests passed with `--features backend-pytorch`
  for typed worker load device ids, legacy-device rejection, direct-load
  construction, package projection, default auto omission, and the repaired
  load fixture.
- Device-contract parser tests now prove `auto` is a reserved scheduler policy
  keyword, not a concrete `InferenceDeviceId`; the PyTorch worker load contract
  also rejects explicit `"auto"` in `payload.device`.
- PyTorch audio transcription worker tests now prove the Rust envelope omits
  auto device intent, rejects legacy/explicit-auto device fields, and the
  Python worker contract applies the same rule before mapping omission to
  backend-local `auto`.
- PyTorch worker response tests now prove loaded-model and live-KV selected
  device facts decode as canonical `InferenceDeviceId` values and reject
  explicit auto or legacy ids.
- Startup-device intent tests prove scheduler policy, canonical selected
  device ids, and llama.cpp-local selectors remain distinct and are not inferred
  from one another during the shared startup-config migration.
- llama.cpp runtime-settings tests now prove effective device state is parsed
  into `DeviceBackend`, rejects canonical `cuda:0` as a llama.cpp selector, and
  projects back to the existing sidecar `DeviceConfig` only at the DTO boundary.
- Sidecar device config tests now prove `DeviceConfig.device` stores typed
  backend-local `DeviceBackend` internally, serde preserves the existing
  llama.cpp selector string shape, and invalid selectors or canonical ids fail
  before sidecar runtime state exists.
- Gateway start-config tests now prove startup requests carry typed
  `BackendStartupDeviceIntent`, reject wrong backend/device namespaces, and do
  not silently ignore explicit device intent for external runtime attachment or
  Candle embedding startup.
- Backend config tests now prove shared startup config stores typed
  `BackendStartupDeviceIntent`, with llama.cpp/PyTorch adapters validating
  their accepted namespace before startup and node-engine projecting workflow
  llama.cpp settings into backend-local selector intent.
- PyTorch helper tests now prove the test-only Transformers load args keep
  device as `Option<InferenceDeviceId>` and model auto policy by omission
  rather than a raw `"auto"` device string.
- PyTorch direct/package load tests now prove executable load APIs accept
  canonical `InferenceDeviceId` values or omitted auto intent, while node-engine
  rejects legacy PyTorch device strings before backend load.
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
- `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts`,
  `npm run typecheck`, and `git diff --check` passed after scheduler run-list
  runtime placement labels and search began using typed
  `selected_runtime_variant_id` without adding a new frontend test platform.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-diagnostics-ledger model_lifecycle_projects_canonical_error_link_without_counting_new_error`,
  `cargo test -p pantograph-workflow-service workflow_run_list_query_contract_snapshot`,
  `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
  `npm run typecheck`, and `git diff --check` passed after run-list facets and
  diagnostics comparison controls began exposing typed
  `selected_runtime_variant_id`.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-diagnostics-ledger model_lifecycle_projects_canonical_error_link_without_counting_new_error`,
  `cargo test -p pantograph-workflow-service workflow_run_list_query_contract_snapshot`,
  `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts src/stores/schedulerRunListStore.test.ts`,
  `npm run typecheck`, and `git diff --check` passed after Scheduler run-list
  queries began forwarding typed `selected_runtime_variant_id` filters.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-embedded-runtime roadmap_runtime_capabilities_report_vllm_and_mlx_placeholders`,
  `cargo test -p pantograph-embedded-runtime runtime_capabilities`,
  `cargo test -p pantograph-embedded-runtime technical_fit`, and
  `git diff --check` passed after vLLM CPU/CUDA and MLX Metal roadmap
  capability facts were exposed as unavailable typed diagnostics only. The
  first format check reported rustfmt wrapping; `cargo fmt --all` was run and
  the check passed.
- `cargo fmt --all -- --check`,
  `cargo test -p pantograph-embedded-runtime roadmap_backend_overrides_reject_without_fallback_selection`,
  `cargo test -p pantograph-embedded-runtime technical_fit`, and
  `git diff --check` passed after explicit vLLM and MLX roadmap backend
  overrides gained unselected-decision regression coverage.
- `cargo fmt --all -- --check`,
  `cargo test -p inference device::tests::parse_llamacpp_inventory_facts`,
  `cargo test -p inference device::tests`, and `git diff --check` passed after
  llama.cpp `--list-devices` output gained canonical inventory fact projection
  while preserving existing backend-local parsing. The first format check
  reported rustfmt wrapping; `cargo fmt --all` was run and the check passed.
- `cargo fmt --all -- --check`,
  `cargo test -p inference --features backend-pytorch pytorch_device_probe`,
  `cargo test -p inference --features backend-pytorch test_capabilities`, and
  `git diff --check` passed after PyTorch probe facts gained canonical
  CPU/CUDA/macOS-MPS runtime variant projection. Initial plain test filters
  matched no PyTorch tests because the backend is feature-gated.
- `node --experimental-strip-types --test src/components/deviceConfigPresenters.test.ts`,
  `npm run typecheck`,
  `rg -n "llama-server owns|let llama-server choose|Select your GPU|frontend-owned auto|CPU Only|Provide fallback options" src/components/DeviceConfig.svelte src/components/deviceConfigPresenters.ts src/components/deviceConfigPresenters.test.ts`,
  and `git diff --check` passed after the Device Configuration save path
  began rejecting stale non-backend-confirmed executable-device values.
- `cargo fmt --all -- --check`,
  `cargo test -p inference --test device_contracts llamacpp_device_inventory_fact_fixture_preserves_canonical_projection`,
  `cargo test -p inference --test device_contracts`,
  `cargo test -p inference device::tests::parse_llamacpp_inventory_facts`, and
  `git diff --check` passed after `LlamaCppDeviceInventoryFact` gained a
  public serde fixture. The first fixture run failed on missing diagnostics
  defaults; adding the serde default fixed the DTO and the test passed.
- `cargo test -p inference managed_runtime::operations`,
  `cargo test -p inference managed_runtime::state`,
  `cargo test -p pantograph-embedded-runtime managed_runtime`,
  `npm run typecheck`, `cargo fmt --all -- --check`, and `git diff --check`
  passed after managed runtime install jobs, retained artifacts, progress
  snapshots, and install history entries began carrying typed
  `RuntimeVariantId`. `npm run -w frontend check:types` was attempted first and
  failed because this repository has no `frontend` workspace; the root
  `npm run typecheck` script was used instead. The first inference compile
  caught a misplaced `runtime_variant_id` initialization in the capability
  path; it was moved to the download/install path and then tightened to use the
  selected `ManagedRuntimeDownloadSource` rather than definition-level
  inference.
- `cargo test -p inference managed_runtime::operations`,
  `cargo test -p inference managed_runtime::state`,
  `cargo test -p inference managed_runtime::neutral_contracts`,
  `cargo test -p inference runtime_load`,
  `cargo test -p pantograph-embedded-runtime managed_runtime`,
  `npm run typecheck`, `cargo fmt --all -- --check`, and `git diff --check`
  passed after managed runtime selection state began carrying
  selected/active/default runtime variant ids. The first focused inference run
  failed because two tests still constructed stale selected-version-only state;
  updating those fixtures to include the selected variant exercised the new
  no-version-only selection rule.
- `cargo test -p inference managed_runtime::operations::tests::catalog_projection_keeps_same_version_runtime_variants_distinct`
  passed after managed runtime catalog projection stopped collapsing same
  release versions with different runtime variant ids.
- `cargo test -p inference managed_runtime::catalog` and
  `cargo test -p inference managed_runtime::operations` passed after Linux and
  Windows llama.cpp managed-runtime catalog definitions began exposing CPU and
  CUDA variants for the same release archive. Metal and variant-specific
  installed-artifact readiness remain follow-ups.
- 2026-05-11 explicit llama.cpp runtime-variant command-resolution slice:
  smallest useful vertical slice was limited to managed runtime command target
  resolution, platform-specific llama.cpp command selection, focused command
  diagnostics, and the plan notes. Allowed write set:
  `crates/inference/src/managed_runtime/contracts.rs`,
  `definitions.rs`, `operations.rs`, `operations/state_transitions.rs`,
  `operations_tests.rs`, `llama_cpp_platform/*`, and this plan directory.
  The slice preserves the no-fallback/no-legacy rule by requiring a persisted
  selected runtime variant before command construction and by refusing to infer
  CUDA selection from raw llama.cpp `--device` arguments.
- `cargo test -p inference managed_runtime::operations`,
  `cargo test -p inference managed_runtime::contracts`,
  `cargo test -p inference managed_runtime::llama_cpp_platform::linux::tests`,
  `cargo fmt --all`, `cargo fmt --all -- --check`, and `git diff --check`
  passed after command resolution began selecting the explicit runtime variant.
  Follow-up: runtime/device inventory calls that need an unselected variant
  should get a variant-aware inventory command path instead of relying on raw
  device flags to choose executables.
- 2026-05-11 llama.cpp platform-boundary slice: smallest useful vertical slice
  was limited to archive server-name matching, runtime-variant install
  subdirectory mapping, focused extraction tests, and plan notes. Allowed write
  set: `crates/inference/src/managed_runtime/llama_cpp_platform/*` and this
  plan directory. The slice preserves the no-fallback/no-legacy rule by moving
  platform layout facts behind `LlamaPlatform` methods instead of adding
  alternate executable discovery.
- `cargo test -p inference managed_runtime::llama_cpp_platform` and
  `cargo test -p inference managed_runtime::operations` passed after shared
  llama.cpp extraction stopped hard-coding archive server filenames and CUDA
  subdirectory routing. The first compile failed because the new focused test
  was added as a second `tests` module; merging it into the existing module
  fixed the issue.
- 2026-05-11 managed-runtime variant-readiness slice: smallest useful vertical
  slice was limited to variant-aware install validation, installed-version
  upsert identity, focused readiness/state tests, and plan notes. Allowed write
  set: `crates/inference/src/managed_runtime/definitions.rs`,
  `llama_cpp_platform/*`, `operations.rs`, `operations/projection.rs`,
  `operations/state_transitions.rs`, `operations_tests.rs`,
  `neutral_contracts.rs`, and this plan directory. The slice preserves the
  no-fallback/no-legacy rule by requiring the requested runtime variant's
  files for readiness instead of reusing CPU readiness for CUDA.
- `cargo test -p inference managed_runtime::llama_cpp_platform`,
  `cargo test -p inference managed_runtime::operations`, and
  `cargo test -p inference managed_runtime::neutral_contracts` passed after
  validation became runtime-variant aware and same-release installed variants
  stopped overwriting each other. Discovered follow-up: manual selection still
  accepts only a version string, so same-version variant switching needs a
  variant-aware command/API in a later slice.
- 2026-05-11 managed-runtime variant-selection slice: smallest useful vertical
  slice was limited to selected/default version updates, embedded-runtime and
  Tauri command plumbing, frontend manager selection controls, focused backend
  selection tests, and plan notes. Allowed write set:
  `crates/inference/src/managed_runtime/operations.rs`,
  `operations/state_transitions.rs`, `operations_tests.rs`,
  `crates/pantograph-embedded-runtime/src/managed_runtime_manager.rs`,
  `src-tauri/src/llm/commands/binary.rs`,
  `src/services/managedRuntime/ManagedRuntimeService.ts`,
  `src/components/runtime-manager/ManagedRuntimeCard.svelte`,
  `src/components/runtime-manager/ManagedRuntimeCatalogPanel.svelte`, and this
  plan directory. The slice preserves the no-fallback/no-legacy rule by
  rejecting ambiguous same-version selections unless the caller supplies the
  selected `RuntimeVariantId`.
- `cargo test -p inference managed_runtime::operations`,
  `cargo test -p pantograph-embedded-runtime managed_runtime`, and
  `npm run typecheck` passed after selected/default runtime updates became
  variant-aware. Remaining follow-up: managed runtime install/download commands
  still accept only a version string and need variant-aware download-source
  selection before same-release CPU/CUDA catalog installs can close.
- 2026-05-11 managed-runtime variant-install slice: smallest useful vertical
  slice was limited to download-source selection, embedded-runtime and Tauri
  install command plumbing, frontend manager install actions, focused backend
  download-source tests, and plan notes. Allowed write set:
  `crates/inference/src/managed_runtime/operations.rs`,
  `operations/download.rs`, `operations_tests.rs`,
  `crates/pantograph-embedded-runtime/src/managed_runtime_manager.rs`,
  `src-tauri/src/llm/commands/binary.rs`,
  `src/services/managedRuntime/ManagedRuntimeService.ts`,
  `src/components/runtime-manager/ManagedRuntimeCard.svelte`,
  `src/components/runtime-manager/ManagedRuntimeCatalogPanel.svelte`, and this
  plan directory. The slice preserves the no-fallback/no-legacy rule by
  rejecting ambiguous same-version catalog install requests unless the caller
  supplies `RuntimeVariantId`.
- `cargo test -p inference managed_runtime::operations`,
  `cargo test -p pantograph-embedded-runtime managed_runtime`, and
  `npm run typecheck` passed after managed runtime install/download requests
  became variant-aware.
- 2026-05-11 macOS llama.cpp Metal variant slice: smallest useful vertical
  slice was limited to macOS managed-runtime variant exposure, Metal file
  validation, selected Metal command diagnostics, focused platform tests, and
  plan notes. Allowed write set:
  `crates/inference/src/managed_runtime/contracts.rs`,
  `crates/inference/src/managed_runtime/llama_cpp_platform/mod.rs`,
  `macos_arm64.rs`, `macos_x64.rs`, and this plan directory. The slice
  preserves the no-fallback/no-legacy rule by requiring
  `libggml-metal.dylib` for `llama_cpp.metal` and rejecting missing Metal
  runtime files with typed runtime-variant diagnostics instead of using CPU.
- `cargo test -p inference managed_runtime::llama_cpp_platform` and
  `cargo test -p inference managed_runtime::contracts` passed after macOS
  llama.cpp managed-runtime variants gained Metal modeling. The first focused
  test run passed but reported a Linux dead-code warning for the Metal constant;
  scoping the constant to macOS/test builds removed the warning.
- 2026-05-11 Candle explicit image-generation override rejection slice:
  smallest useful vertical slice was limited to runtime-registry explicit
  override diagnostics, embedded-runtime technical-fit projection coverage, and
  plan notes. Allowed write set:
  `crates/pantograph-runtime-registry/src/technical_fit.rs`,
  `crates/pantograph-embedded-runtime/src/technical_fit.rs`, and this plan
  directory. The slice preserves the no-fallback/no-legacy rule by keeping
  explicit Candle backend preference unselected for Diffusers image-generation
  package facts and projecting the backend compatibility issue as a typed
  `backend_incompatible` diagnostic instead of selecting Candle, inventing a
  candidate, or falling back to CPU/auto/another backend.
- `cargo test -p pantograph-embedded-runtime candle_image_generation_override_rejects_backend_incompatibility_without_selection`,
  `cargo test -p pantograph-runtime-registry technical_fit`, and
  `cargo test -p pantograph-embedded-runtime technical_fit` passed. Remaining
  follow-up: vLLM unsupported model/task artifact preference rejection still
  needs canonical backend-fact coverage before the broad impossible-preference
  checklist item closes.
- 2026-05-11 vLLM explicit image-generation override rejection slice: smallest
  useful vertical slice was limited to embedded-runtime technical-fit coverage
  for a Diffusers image-generation package, vLLM backend compatibility facts,
  and plan notes. Allowed write set:
  `crates/pantograph-embedded-runtime/src/technical_fit.rs` and this plan
  directory. The slice preserves the no-fallback/no-legacy rule by keeping
  explicit vLLM backend preference unselected for unsupported image-generation
  package facts and projecting `backend_incompatible` diagnostics instead of
  selecting vLLM, roadmap facts, CPU/auto, or another backend.
- `cargo test -p pantograph-embedded-runtime vllm_image_generation_override_rejects_unsupported_package_without_selection`
  and `cargo test -p pantograph-embedded-runtime technical_fit` passed. This
  closes the broad impossible-preference checklist item with existing llama.cpp
  diffusion, MLX roadmap/platform, and Candle image-generation coverage.
- 2026-05-11 runtime-load contract reconciliation slice: smallest useful
  vertical slice was limited to marking the already-implemented runtime-load
  resolved-device contract complete after rechecking
  `RuntimeLoadPhaseRecord::dependency_resolved`, the public runtime-load JSON
  fixture, and existing plan notes. Allowed write set: this plan directory.
  The no-fallback/no-legacy rule is preserved because the contract requires a
  `DeviceResolutionDecision`; no load-readiness DTO can be constructed from
  command arguments or raw backend config strings alone.
- `cargo test -p inference --test runtime_load_contracts`,
  `cargo test -p inference runtime_load`, and `git diff --check` passed for
  the reconciliation slice. Remaining runtime-load integration work is tracked
  by later lifecycle/admission checklist items, not this contract item.
- 2026-05-11 lifecycle selected runtime-variant projection slice: smallest
  useful vertical slice was limited to adding `selected_runtime_variant_id` to
  inference lifecycle events, diagnostics-ledger inference diagnostic payloads,
  embedded-runtime ledger projection, run-list/run-detail inference diagnostic
  projection, focused tests, and plan notes. Allowed write set:
  `crates/inference/src/types.rs`, `crates/inference/src/gateway.rs`,
  `crates/inference/tests/model_contracts.rs`,
  `crates/node-engine/src/core_executor.rs`,
  `crates/node-engine/src/core_executor/dependency_preflight.rs`,
  `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
  `crates/pantograph-diagnostics-ledger/src/tests.rs`,
  `crates/pantograph-embedded-runtime/src/node_execution_ledger.rs`,
  `crates/pantograph-embedded-runtime/src/node_execution_ledger_tests.rs`, and
  this plan directory.
- The slice preserves the no-fallback/no-legacy rule by carrying runtime
  variant only when explicitly supplied by the lifecycle fact producer. It does
  not infer runtime variants from runtime id, backend key, command arguments,
  or raw device strings. Verification passed:
  `cargo test -p inference inference_request_lifecycle_event`,
  `cargo test -p inference --test model_contracts public_inference_contract_json_keys_avoid_scheduler_policy_language`,
  `cargo test -p pantograph-diagnostics-ledger inference_diagnostic`,
  `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_appends_inference_execution_diagnostic_summary`,
  `cargo test -p pantograph-diagnostics-ledger`,
  and
  `cargo test -p pantograph-embedded-runtime inference_diagnostic_event_adapter_builds_option_support_summary`.
  Verification deviation: the first diagnostics-ledger command incorrectly
  passed two Cargo test filters; it was rerun with single filters. Broader
  `cargo test -p pantograph-embedded-runtime node_execution_ledger` still fails
  in pre-existing workflow-sink retained-artifact/status tests with zero
  recorded rows; the focused inference diagnostic adapter test passed and the
  broader failure is recorded as a discovered follow-up outside this slice.
- 2026-05-11 scheduler-learning output descriptor facts slice: smallest useful
  vertical slice was limited to diagnostics-ledger run-list/run-detail
  projection schema, projection DTOs, descriptor-only output artifact rollups,
  workflow-service contract fixtures, module README ownership notes, and this
  plan directory. Allowed write set:
  `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `crates/pantograph-diagnostics-ledger/src/schema.rs`,
  `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
  `crates/pantograph-diagnostics-ledger/src/tests.rs`,
  `crates/pantograph-diagnostics-ledger/src/README.md`,
  `crates/pantograph-workflow-service/tests/contract.rs`,
  `crates/pantograph-workflow-service/tests/fixtures/run_projection_contract.json`,
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule by only promoting
  canonical `io.artifact_observed` descriptor facts into run projections. It
  does not inspect retained artifact bodies, infer backend/device choices from
  raw runtime strings, add a learned scheduler, or preserve old selection
  behavior. Existing run projections already carried model id, task kind,
  selected backend, selected runtime variant, selected device class/id,
  estimated duration when known, execution duration, and terminal status; this
  slice added output descriptor count and total reported byte size with checked
  arithmetic before SQLite updates.
- Verification passed:
  `cargo test -p pantograph-diagnostics-ledger run_projections_record_scheduler_learning_output_descriptor_measures`,
  `cargo test -p pantograph-diagnostics-ledger existing_v24_schema_adds_scheduler_learning_output_projection_columns`,
  `cargo test -p pantograph-diagnostics-ledger`,
  `cargo test -p pantograph-workflow-service workflow_run_list_query_contract_snapshot`,
  `cargo test -p pantograph-workflow-service workflow_run_detail_query_contract_snapshot`,
  `cargo test -p pantograph-workflow-service --test contract`,
  `cargo fmt --all -- --check`, and `git diff --check`.
  Verification deviation: one initial workflow-service command incorrectly
  passed two Cargo test filters in a single invocation; the two filters were
  rerun separately and then the full contract test binary passed. Discovered
  issue fixed within the slice: run projections previously filtered out
  `io.artifact_observed`, so descriptor facts could not reach run-list or
  run-detail projections.
- 2026-05-11 backend capability contract reconciliation slice: smallest useful
  vertical slice was limited to validating existing capability-contract code
  and marking the checklist item complete. Allowed write set: this plan
  directory. The no-fallback/no-legacy rule is preserved because the existing
  contract reports adapter-owned capability facts and unavailable roadmap facts
  without ranking candidates, selecting alternate backends, or translating raw
  device strings outside backend/runtime boundaries.
- Verification passed:
  `cargo test -p inference test_capabilities`,
  `cargo test -p inference backend_capability_facts_preserve_runtime_variant_facts`,
  `cargo test -p inference capabilities`,
  `cargo test -p pantograph-embedded-runtime roadmap_runtime_capabilities_report_vllm_and_mlx_placeholders`,
  and `cargo test -p pantograph-embedded-runtime runtime_capabilities`.
  Deviation: this was a plan reconciliation slice because previous Milestone 5
  slices had already added `BackendCapabilityFacts.runtime_variants`,
  llama.cpp/PyTorch/Candle adapter facts, and vLLM/MLX roadmap facts.
- 2026-05-11 shared allowed-root command path validation slice: smallest
  useful vertical slice was limited to extracting the existing node-engine path
  validator into `pantograph-path-security`, routing node-engine/workflow
  callers through it, and validating managed-runtime selected install roots,
  executable paths, and working directories before command handoff. Allowed
  write set: workspace Cargo manifests/lockfile,
  `crates/pantograph-path-security/`, `crates/node-engine/src/path_validation.rs`,
  managed-runtime command/path tests, and Milestone 5 plan notes.
- The slice preserves the no-fallback/no-legacy rule because escaped selected
  install roots now fail with typed path diagnostics instead of being treated
  as trusted executable state. Verification passed:
  `cargo test -p pantograph-path-security`,
  `cargo test -p node-engine path_validation`,
  `cargo test -p inference resolve_binary_command`,
  `cargo test -p inference managed_runtime_snapshot`,
  `cargo test -p pantograph-workflow-service persistence`,
  `cargo test -p workflow-nodes storage`,
  `cargo test -p inference managed_runtime::operations`,
  `cargo fmt --all -- --check`, and `git diff --check`. Deviations: initial
  parallel Cargo test commands serialized on Cargo locks; an initial projection
  compile needed `app_data_dir` threaded into installed-version projection.
- 2026-05-11 managed runtime dynamic-library path validation slice: smallest
  useful vertical slice was limited to removing inherited host library-search
  tails from managed llama.cpp command env overrides and validating owned
  dynamic-library path values before handoff. The slice preserves the
  no-fallback/no-legacy rule because unvalidated `LD_LIBRARY_PATH`,
  `DYLD_LIBRARY_PATH`, and Windows `PATH` tails are no longer retained as
  alternate runtime library search locations. Verification passed:
  `cargo test -p inference managed_runtime::paths`,
  `cargo test -p inference managed_runtime::llama_cpp_platform::linux::tests`,
  `cargo test -p inference resolve_binary_command`, and
  `cargo test -p inference runtime_sidecar_command_projection_preserves_resolved_command_facts`.
  Deviation: the broader neutral-contract check exposed a stale fixture that
  still used the retired `app_data/runtimes` root; the fixture was corrected in
  a focused follow-up commit.
- 2026-05-11 managed runtime pid-file path validation slice: smallest useful
  vertical slice was limited to validating extracted managed-runtime
  `--pid-file` paths against `app_data_dir`, resolving relative paths under
  that root, and rejecting escaped absolute pid-file paths with typed path
  diagnostics. Allowed write set: managed-runtime command contracts,
  operations, neutral projection tests, and Milestone 5 plan notes. The slice
  preserves the no-fallback/no-legacy rule because it does not synthesize an
  alternate pid-file location when validation fails.
- Verification passed:
  `cargo test -p inference resolve_binary_command`,
  `cargo test -p inference runtime_sidecar_command_projection_preserves_resolved_command_facts`,
  `cargo fmt --all -- --check`, and `git diff --check`. Remaining follow-up:
  Pumas package paths, artifact paths, and worker-visible paths still need
  shared allowed-root validation before filesystem or subprocess access.
- 2026-05-11 artifact-store checked byte accounting slice: smallest useful
  vertical slice was limited to memory-cache byte counters, streaming chunk
  byte-length updates, the artifact-store typed error enum, API error
  projection, focused unit tests, and Milestone 5 plan notes. Allowed write
  set:
  `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`,
  `crates/pantograph-workflow-service/src/workflow/artifact_store/cache.rs`,
  `crates/pantograph-workflow-service/src/workflow/artifact_store/stream.rs`,
  `crates/pantograph-workflow-service/src/workflow/artifact_api.rs`, and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because artifact byte
  accounting overflow is rejected or skipped before mutating counters instead
  of preserving saturating totals or clamping stream byte length as retained
  artifact state. Verification passed:
  `cargo test -p pantograph-workflow-service memory_cache_capacity_check_rejects_overflow`,
  `cargo test -p pantograph-workflow-service stream_chunk_rejects_byte_length_overflow`,
  `cargo test -p pantograph-workflow-service --test artifact_store`,
  and `cargo test -p pantograph-workflow-service workflow::artifact_store`.
  Deviations: the first focused compile exposed a missing API projection arm
  for `ArtifactAccountingOverflow`, which now maps to
  `WorkflowServiceError::InvalidRequest`; parallel focused Cargo tests
  serialized on Cargo locks before passing. Remaining numeric-boundary
  follow-up: image dimensions, context/token/batch limits, memory estimates,
  disk budget summation, artifact stats summation, byte-range projections, and
  worker/runtime request fields.
- 2026-05-11 artifact-store disk-budget checked summation slice: smallest
  useful vertical slice was limited to replacing the artifact-store
  disk-budget `sum::<u64>().saturating_add(...)` projection with checked
  accumulation across retained artifacts, pending streams, and the replacement
  body size. Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/artifact_store.rs` and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because disk projection
  overflow now returns typed `ArtifactStoreError::ArtifactAccountingOverflow {
  field: "disk_usage_bytes" }` before any new artifact body, descriptor,
  manifest, or memory-cache state is written; it does not clamp or saturate
  projected disk usage. Verification passed:
  `cargo test -p pantograph-workflow-service disk_limit_projection_rejects_total_byte_overflow`,
  `cargo test -p pantograph-workflow-service workflow::artifact_store`, and
  `cargo test -p pantograph-workflow-service --test artifact_store`.
  Deviations: `cargo fmt --all -- --check` found rustfmt-only wrapping before
  verification; `cargo fmt --all` was applied and the tests above passed after
  formatting. Parallel Cargo tests serialized on package/build locks before
  passing. Discovered issue/deferred follow-up: `artifact_store.rs` is already
  over the 500-line coding-standards decomposition-review trigger; splitting
  was deferred because the slice stayed within disk-accounting behavior and
  the existing colocated unit-test pattern. Remaining numeric-boundary
  follow-up: image dimensions, context/token/batch limits, memory estimates,
  artifact stats summation, byte-range projections, and worker/runtime request
  fields.
- 2026-05-11 artifact-store stats checked summation slice: smallest useful
  vertical slice was limited to making `ArtifactStore::stats()` fallible and
  replacing unchecked retained-body byte, streaming-body byte, and per-state
  counter additions with checked arithmetic. Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`,
  `crates/pantograph-workflow-service/src/workflow/artifact_api.rs`,
  `crates/pantograph-workflow-service/tests/artifact_store.rs`,
  `crates/pantograph-workflow-service/tests/artifact_store_policy.rs`, and
  this plan directory.
- The slice preserves the no-fallback/no-legacy rule because stats overflow
  now returns typed `ArtifactStoreError::ArtifactAccountingOverflow` instead
  of wrapping, saturating, or returning partial stats. The workflow service
  stats facade projects the typed store error through its existing `Result`
  boundary. Verification passed:
  `cargo test -p pantograph-workflow-service stats_rejects_retained_body_byte_overflow`,
  `cargo test -p pantograph-workflow-service workflow::artifact_store`,
  `cargo test -p pantograph-workflow-service --test artifact_store`,
  `cargo test -p pantograph-workflow-service --test artifact_store_policy`,
  `cargo test -p pantograph-embedded-runtime workflow_artifact_store_stats`,
  and `cargo test -p pantograph-uniffi workflow_artifact_store_stats`.
  Deviations/discovered issues: the embedded-runtime and UniFFI focused filters
  matched zero tests but compiled their public stats facades successfully. The
  UniFFI compile surfaced pre-existing unused imports in
  `crates/pantograph-embedded-runtime/src/technical_fit.rs`
  (`WorkflowBackendCapabilityFacts` and
  `WorkflowRuntimeVariantCapability`); cleanup is deferred because it is
  unrelated to artifact stats accounting. `artifact_store.rs` remains over the
  500-line coding-standards decomposition-review trigger; splitting remains
  deferred to avoid broadening this slice. Remaining numeric-boundary
  follow-up: image dimensions, context/token/batch limits, memory estimates,
  byte-range projections, and worker/runtime request fields.
- 2026-05-11 llama.cpp context-size fail-closed validation slice: smallest
  useful vertical slice was limited to including `BackendConfig.context_size`
  in `LlamaCppRuntimeSettings::try_from_backend_config` positive-value
  validation so `Some(0)` cannot become an effective llama-server `-c 0`
  setting. Allowed write set: `crates/inference/src/backend/mod.rs` and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because invalid explicit
  context size now returns `BackendError::Config` through the existing typed
  backend startup boundary instead of replacing zero with the default context
  size or preserving the previous executable zero-value path. Verification
  passed:
  `cargo test -p inference llamacpp_runtime_settings_reject_zero_sized_performance_knobs`,
  `cargo test -p inference llamacpp_runtime_settings`, and
  `cargo fmt --all -- --check`. Deviation: the two focused Cargo tests were
  started in parallel and serialized on Cargo package/build locks before
  passing. Remaining numeric-boundary follow-up: image dimensions,
  context/token/batch limits outside this llama.cpp startup normalization
  boundary, memory estimates, byte-range projections, and worker/runtime
  request fields.
- 2026-05-11 diagnostics projection rebuild batch-size validation slice:
  smallest useful vertical slice was limited to removing the `.max(1)`
  fallback from `workflow_projection_rebuild` and rejecting explicit
  `batch_size: Some(0)` through the existing workflow-service invalid-request
  boundary. Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/diagnostics.rs`, and
  this plan directory.
- The slice preserves the no-fallback/no-legacy rule because explicit zero no
  longer becomes batch size one. `None` still selects the canonical default
  because it is an absent option, not an invalid explicit numeric request.
  Verification passed:
  `cargo test -p pantograph-workflow-service workflow_projection_rebuild_validates_bounds`,
  `cargo test -p pantograph-workflow-service workflow_diagnostics_projection_refresh_validates_request`,
  and `cargo fmt --all -- --check`. Deviation: the two focused Cargo tests
  were started in parallel and serialized on Cargo package/build locks before
  passing. Remaining numeric-boundary follow-up: image dimensions,
  context/token/batch limits outside this projection rebuild validation
  boundary, memory estimates, byte-range projections, and worker/runtime
  request fields.
- 2026-05-11 image generation zero-dimension validation slice: smallest useful
  vertical slice was limited to validating typed image generation width and
  height at the inference gateway before backend dispatch. Allowed write set:
  `crates/inference/src/gateway.rs`, `crates/inference/src/gateway_tests.rs`,
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because explicit zero
  dimensions now fail with `BackendError::Config`; the gateway does not replace
  them with backend defaults, clamp to one, or pass zero through to backend
  implementations. `None` remains an absent option owned by the selected
  backend. Verification passed:
  `cargo test -p inference test_generate_image_rejects_zero_dimensions`,
  `cargo test -p inference test_execute_typed_forwards_image_generation_to_active_backend`,
  and `cargo fmt --all -- --check`. Deviation: the two focused Cargo tests
  were started in parallel and serialized on Cargo package/build locks before
  passing. Remaining numeric-boundary follow-up: image request limits beyond
  zero dimensions, context/token/batch limits, memory estimates, byte-range
  projections, and worker/runtime request fields.
- 2026-05-11 image generation positive count validation slice: smallest useful
  vertical slice was limited to extending typed image generation gateway
  validation to reject explicit zero `num_inference_steps` and
  `num_images_per_prompt` before backend dispatch. Allowed write set:
  `crates/inference/src/gateway.rs`, `crates/inference/src/gateway_tests.rs`,
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because explicit zero
  image count/request values now fail with `BackendError::Config`; the gateway
  does not replace them with backend defaults, clamp to one, or pass zero
  through to backend implementations. `None` remains an absent option owned by
  the selected backend. Verification passed:
  `cargo test -p inference test_generate_image_rejects_zero_positive_count_options`,
  `cargo test -p inference test_generate_image_rejects_zero_dimensions`, and
  `cargo fmt --all -- --check`. Deviations: the first
  `cargo fmt --all -- --check` found rustfmt-only wrapping; `cargo fmt --all`
  was applied and verification was rerun successfully. The two focused Cargo
  tests were started in parallel and serialized on Cargo package/build locks
  before passing. Remaining numeric-boundary follow-up: image request limits
  beyond positive-count validation, context/token/batch limits, memory
  estimates, byte-range projections, and worker/runtime request fields.
- 2026-05-11 retention cleanup zero-limit validation slice: smallest useful
  vertical slice was limited to removing the `.max(1)` fallback from
  `workflow_retention_cleanup_apply` and rejecting explicit `limit: Some(0)`
  through the existing workflow-service invalid-request boundary. Allowed
  write set:
  `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/diagnostics.rs`, and
  this plan directory.
- The slice preserves the no-fallback/no-legacy rule because explicit zero no
  longer becomes cleanup limit one. `None` still selects the canonical default
  because it is an absent option, not an invalid explicit numeric request.
  Verification passed:
  `cargo test -p pantograph-workflow-service workflow_retention_cleanup_rejects_zero_limit`,
  `cargo test -p pantograph-workflow-service workflow_retention_cleanup_expires_artifacts_through_projection`,
  and `cargo fmt --all -- --check`. Deviation: the two focused Cargo tests
  were started in parallel and serialized on Cargo package/build locks before
  passing. Remaining numeric-boundary follow-up: image request limits,
  context/token/batch limits outside this retention-cleanup validation
  boundary, memory estimates, byte-range projections, and worker/runtime
  request fields.
- 2026-05-11 diagnostics query zero-limit validation slice: smallest useful
  vertical slice was limited to removing diagnostics query DTO `.max(1)`
  fallbacks and rejecting explicit zero `page_size`/`limit` values through the
  existing workflow-service invalid-request boundary. Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/diagnostics.rs`, and
  this plan directory.
- The slice preserves the no-fallback/no-legacy rule because explicit zero no
  longer becomes page or result limit one. `None` still selects the canonical
  default because it is an absent option, not an invalid explicit numeric
  request. Verification passed:
  `cargo test -p pantograph-workflow-service workflow_diagnostics_usage_query_validates_ids_and_bounds`,
  `cargo test -p pantograph-workflow-service workflow_scheduler_timeline_query_validates_bounds`,
  `cargo test -p pantograph-workflow-service workflow_run_list_query_validates_bounds`,
  `cargo test -p pantograph-workflow-service workflow_io_artifact_query_validates_bounds`,
  `cargo test -p pantograph-workflow-service workflow_node_status_query_rejects_zero_limit`,
  and
  `cargo test -p pantograph-workflow-service workflow_library_usage_query_validates_bounds`,
  `cargo fmt --all -- --check`, and `git diff --check`.
  Deviations: the first `cargo fmt --all -- --check` found rustfmt-only
  wrapping in the touched tests; `cargo fmt --all` was applied and final format
  verification passed before commit. Remaining numeric-boundary follow-up:
  image request limits, context/token/batch limits outside this diagnostics
  query validation boundary, memory estimates, byte-range projections, and
  worker/runtime request fields.
- 2026-05-11 loaded-runtime capacity limit validation slice: smallest useful
  vertical slice was limited to replacing the
  `set_loaded_runtime_capacity_limit` min/max clamp with explicit validation
  for zero and above-session-limit values. Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/service_config.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/session_capacity_limits.rs`,
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because explicit invalid
  capacity limits no longer become one or `max_sessions`; they return
  `WorkflowServiceError::InvalidRequest` and leave the last valid limit
  unchanged. `None` remains the canonical reset to the service session limit.
  Verification passed:
  `cargo test -p pantograph-workflow-service loaded_runtime_capacity_limit_validates_session_bounds`
  and `cargo test -p pantograph-workflow-service session_capacity_limits`,
  `cargo fmt --all -- --check`, and `git diff --check`.
  Remaining numeric-boundary follow-up: image request limits,
  context/token/batch limits outside this capacity setter validation boundary,
  memory estimates, byte-range projections, and worker/runtime request fields.
- 2026-05-11 runtime-registry reserved resource accounting slice: smallest
  useful vertical slice was limited to replacing raw `sum()` aggregation for
  runtime admission reserved RAM/VRAM claims with checked addition and a typed
  registry accounting error. Allowed write set:
  `crates/pantograph-runtime-registry/src/lib.rs`,
  `crates/pantograph-runtime-registry/src/lib_tests/admission.rs`, and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because
  reserved-resource overflow no longer depends on debug panic or release
  wrapping. Valid arithmetic still produces existing insufficient RAM/VRAM
  admission diagnostics. Verification passed:
  `cargo test -p pantograph-runtime-registry reserved_resource_accounting_overflow_returns_typed_error`
  and `cargo test -p pantograph-runtime-registry admission`,
  `cargo fmt --all -- --check`, and `git diff --check`. Deviation: the first
  `cargo fmt --all -- --check` found rustfmt-only wrapping in the touched
  runtime-registry files; `cargo fmt --all` was applied and focused tests plus
  final format verification were rerun successfully. Remaining
  numeric-boundary follow-up: image request limits, context/token/batch limits
  outside this reservation accounting boundary, memory estimates, byte-range
  projections, and worker/runtime request fields.
- 2026-05-11 workflow capability memory estimate accounting slice: smallest
  useful vertical slice was limited to replacing saturating model-size megabyte
  rounding and raw peak-memory summation in `estimate_memory_requirements` with
  checked arithmetic and typed workflow-service errors. Allowed write set:
  `crates/pantograph-workflow-service/src/capabilities.rs`,
  `crates/pantograph-workflow-service/src/workflow/host.rs`, and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because invalid model
  metadata size arithmetic no longer saturates into a plausible memory
  estimate; it fails through `WorkflowServiceError::InvalidRequest`. Absent
  model sizes still produce the canonical unknown estimate. Verification
  passed: `cargo test -p pantograph-workflow-service memory_estimate` and
  `cargo test -p pantograph-workflow-service workflow_capabilities`,
  `cargo fmt --all -- --check`, and `git diff --check`. Deviation: the first
  `cargo fmt --all -- --check` found rustfmt-only wrapping in the touched
  capability tests; `cargo fmt --all` was applied and focused tests plus final
  format verification were rerun successfully.
  Remaining numeric-boundary follow-up: image request limits,
  context/token/batch limits outside this capability estimation boundary,
  byte-range projections, and worker/runtime request fields.
- 2026-05-11 inference embedding usage token accounting slice: smallest useful
  vertical slice was limited to replacing embedding usage `saturating_add` plus
  `u32::MAX` clamping with checked token aggregation and typed gateway failure
  when the total cannot fit the public `InferenceUsage` fields. Allowed write
  set: `crates/inference/src/gateway.rs`,
  `crates/inference/src/gateway_tests.rs`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because embedding usage
  overflow no longer produces a plausible capped token count; typed execution
  and lifecycle embedding paths fail with `BackendError::Config`. Verification
  passed: `cargo test -p inference embedding_usage` and
  `cargo test -p inference embedding`, `cargo fmt --all -- --check`, and
  `git diff --check`. Deviation: the first `cargo fmt --all -- --check` found
  rustfmt-only wrapping in the touched gateway tests; `cargo fmt --all` was
  applied and focused tests plus final format verification were rerun
  successfully. Remaining numeric-boundary follow-up: image request limits,
  context/batch limits outside this embedding usage projection boundary,
  byte-range projections, and worker/runtime request fields.
- 2026-05-11 runtime-registry admission budget underflow validation slice:
  smallest useful vertical slice was limited to replacing runtime admission
  available-budget saturating subtraction with checked subtraction across total
  budget, safety margin, and reserved resource claims. Allowed write set:
  `crates/pantograph-runtime-registry/src/lib.rs`,
  `crates/pantograph-runtime-registry/src/lib_tests/admission.rs`, and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because impossible budget
  arithmetic no longer becomes zero available resource; it returns typed
  `RuntimeRegistryError::ResourceBudgetUnderflow`. Valid exhausted budgets
  still produce existing insufficient RAM/VRAM admission diagnostics.
  Verification passed:
  `cargo test -p pantograph-runtime-registry available_budget_underflow_returns_typed_error`
  and `cargo test -p pantograph-runtime-registry admission`,
  `cargo fmt --all -- --check`, and `git diff --check`. Remaining
  numeric-boundary follow-up: image request limits, context/batch limits
  outside this admission budget projection boundary, byte-range projections,
  and worker/runtime request fields.
- 2026-05-11 workflow capability zero-size memory estimate validation slice:
  smallest useful vertical slice was limited to rejecting explicit zero
  `size_bytes` in model metadata before projecting model memory estimates.
  Allowed write set:
  `crates/pantograph-workflow-service/src/capabilities.rs` and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because zero-byte model
  metadata no longer becomes a fabricated 1 MB estimate; it fails with
  `WorkflowServiceError::InvalidRequest`. Missing model metadata still produces
  the canonical unknown estimate. Verification passed:
  `cargo test -p pantograph-workflow-service memory_estimate`,
  `cargo fmt --all -- --check`, and `git diff --check`. Remaining
  numeric-boundary follow-up: image request limits, context/batch limits
  outside this memory estimate validation boundary, byte-range projections, and
  worker/runtime request fields.
- 2026-05-11 artifact retention cleanup TTL arithmetic slice: smallest useful
  vertical slice was limited to replacing retention cleanup TTL
  second-to-millisecond saturation and cutoff saturation with checked
  arithmetic. Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/artifact_store.rs` and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because impossible
  retention TTL arithmetic no longer projects a saturated cleanup cutoff; it
  fails with `ArtifactStoreError::ArtifactAccountingOverflow`. Verification
  passed:
  `cargo test -p pantograph-workflow-service retention_cleanup_rejects_ttl_millisecond_overflow`
  and `cargo test -p pantograph-workflow-service artifact_store`,
  `cargo fmt --all -- --check`, and `git diff --check`. Remaining
  numeric-boundary follow-up: image request limits, context/batch limits
  outside this retention cleanup boundary, byte-range projections, and
  worker/runtime request fields.
- 2026-05-12 stale-graph diagnostic summary count validation slice: smallest
  useful vertical slice was limited to replacing presentation-only
  `saturating_sub` in stale-graph summary formatting with checked arithmetic
  and a typed internal error for impossible formatter state. Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/validation.rs` and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because impossible
  diagnostic formatter counts no longer become `0 more`; the formatter fails
  with `WorkflowServiceError::Internal`. Normal stale-graph validation still
  returns the existing structured stale-graph diagnostics. Verification passed:
  `cargo test -p pantograph-workflow-service stale_graph_remaining_count_rejects_formatter_underflow`
  and `cargo test -p pantograph-workflow-service stale_graph`,
  `cargo fmt --all -- --check`, and `git diff --check`. Remaining
  numeric-boundary follow-up: duration/timing diagnostics, scheduler timestamp
  addition, runtime technical-fit rank overflow, cache counter drift, broader
  image request limits, context/batch limits, byte-range projections, and
  worker/runtime request fields.
- 2026-05-12 scheduler runtime-admission retry timestamp validation slice:
  smallest useful vertical slice was limited to replacing the session
  runtime-admission retry `now_ms + WORKFLOW_SESSION_QUEUE_POLL_MS` saturation
  with checked arithmetic and a typed workflow-service error. Allowed write
  set:
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because impossible retry
  timestamp arithmetic no longer schedules a saturated retry instant; it fails
  with `WorkflowServiceError::Internal`. Normal runtime-admission delay events
  still carry the scheduler retry timestamp. Verification passed:
  `cargo test -p pantograph-workflow-service scheduler_delay_until_rejects_timestamp_overflow`
  and
  `cargo test -p pantograph-workflow-service workflow_execution_session_run_waits_for_runtime_admission`,
  `cargo fmt --all -- --check`, and `git diff --check`. Deviation: the first
  `cargo fmt --all -- --check` found rustfmt-only wrapping in the touched
  scheduler timestamp call; `cargo fmt --all` was applied and focused tests
  plus final format verification were rerun successfully. Remaining
  numeric-boundary follow-up: duration/timing diagnostics, runtime
  technical-fit rank overflow, cache counter drift, broader image request
  limits, context/batch limits, byte-range projections, and worker/runtime
  request fields.
- 2026-05-12 artifact memory-cache removal counter validation slice: smallest
  useful vertical slice was limited to replacing memory-cache removal
  `saturating_sub` with checked byte-counter subtraction and returning the
  existing artifact accounting overflow error on counter drift. Allowed write
  set:
  `crates/pantograph-workflow-service/src/workflow/artifact_store/cache.rs`,
  `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`, and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because cache
  byte-counter underflow no longer silently resets the counter to zero; removal
  fails with `ArtifactStoreError::ArtifactAccountingOverflow` and leaves the
  cached body intact for diagnosis. Verification passed:
  `cargo test -p pantograph-workflow-service memory_cache_remove_rejects_counter_underflow`
  and `cargo test -p pantograph-workflow-service artifact_store`,
  `cargo fmt --all -- --check`, and `git diff --check`. Remaining
  numeric-boundary follow-up: duration/timing diagnostics, runtime
  technical-fit rank overflow, broader image request limits, context/batch
  limits, byte-range projections, and worker/runtime request fields.
- 2026-05-12 runtime technical-fit headroom rank validation slice: smallest
  useful vertical slice was limited to rejecting automatic queue/budget-pressure
  candidate selection when an eligible runtime snapshot reports more active
  reservations than the selector can rank exactly. Allowed write set:
  `crates/pantograph-runtime-registry/src/technical_fit.rs`,
  `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
  `crates/pantograph-runtime-registry/src/README.md`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because impossible
  reservation headroom data no longer gets capped into the lowest rank or
  hidden by selecting another candidate; it returns an unselected automatic
  decision with an error diagnostic for upstream diagnostics-ledger
  projection. Verification passed:
  `cargo test -p pantograph-runtime-registry selector_rejects_unrankable_headroom_under_queue_pressure`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo fmt --all -- --check`, and `git diff --check`. Deviation: the first
  final `cargo fmt --all -- --check` found rustfmt-only wrapping in the touched
  selector/test code; `cargo fmt --all` was applied and focused tests plus
  final format verification were rerun successfully. Remaining
  numeric-boundary follow-up: duration/timing diagnostics, broader image
  request limits, context/batch limits, byte-range projections, and
  worker/runtime request fields.
- 2026-05-12 workflow startup-repair diagnostics arithmetic slice: smallest
  useful vertical slice was limited to replacing startup-repair run-duration
  `saturating_sub` and repaired-count `saturating_add` with checked helpers.
  Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/diagnostics.rs`, and
  this plan directory.
- The slice preserves the no-fallback/no-legacy rule because impossible repair
  timing or count state no longer becomes a successful repair with zero
  duration or a capped count; the service returns
  `WorkflowServiceError::Internal` so corrupt projection state is visible.
  Verification passed:
  `cargo test -p pantograph-workflow-service startup_repair`,
  `cargo test -p pantograph-workflow-service diagnostics`,
  `cargo fmt --all -- --check`, and `git diff --check`. Remaining
  numeric-boundary follow-up: model/runtime load and unload duration
  diagnostics need the broader timing identity/history policy before they can
  be changed safely; broader image request limits, context/batch limits,
  byte-range projections, and worker/runtime request fields remain open.
- 2026-05-12: Re-plan trigger reached before replacing the remaining
  model/runtime load, unload, warmup, and scheduler trace duration saturation.
  Code search found timing math in `crates/inference/src/gateway.rs`,
  `crates/inference/src/embedding_runtime.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_runtime.rs`, and
  `crates/pantograph-workflow-service/src/trace/`. The policy direction now
  requires timing identity, durable diagnostics, historical baseline behavior,
  and scheduler retry/termination semantics. Replacing each
  `saturating_sub` locally would prevent underflow, but it would not provide
  the load/run identity, ledger history, or retry policy needed to decide when
  unusually slow loads fail, retry, or terminate. Required re-plan: define a
  canonical timing measurement contract for runtime/model load attempts,
  unload attempts, warmup attempts, and scheduler trace spans, including
  attempt ids, workflow/run/session/runtime/model/device attribution, checked
  timestamp math, diagnostics-ledger payloads, baseline/deviation policy, and
  scheduler failure/retry ownership.
- 2026-05-12: Re-plan decision resolved. Continue with the contract-first
  timing diagnostics path before enforcing learned timing policy. The next
  implementation slice should define canonical timing attempt identities and
  diagnostic payloads for runtime/model load, unload, warmup, and scheduler
  trace spans, then migrate checked duration math onto those owned contracts in
  thin vertical slices. Full baseline/deviation enforcement, scheduler
  reschedule policy, retry exhaustion, and terminal workflow failure semantics
  remain required follow-up work after the timing history exists.
- 2026-05-13 scheduler timing/history policy decision: timing and memory-fit
  metrics are mandatory scheduler inputs because the scheduler's main job is to
  find the least time-intensive valid execution path without overflowing system
  memory. History-backed runtime ranking must not begin until every valid
  runtime for the same workflow identity has at least five completed runs.
  Before that threshold, automatic selection relies solely on current facts and
  distributes runs across valid runtimes through recorded controlled
  exploration. After the threshold, ranking may weigh load duration, warmup
  duration, execution duration, memory pressure, OOM/failure history, and
  already-resident runtime state. This resolves the minimum-history policy
  portion of the retry/ranking re-plan boundary; implementation still needs
  durable timing summaries, ranking tests, retry exhaustion, and terminal
  failure semantics.
- 2026-05-12 workflow timing attempt contract slice: smallest useful vertical
  slice was limited to adding a workflow-service timing attempt contract and
  focused serde/validation tests. Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/timing_contracts.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/timing_contracts.rs`,
  `crates/pantograph-workflow-service/src/workflow.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests.rs`,
  `crates/pantograph-workflow-service/src/workflow/README.md`, and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because it creates the
  canonical attempt identity and typed diagnostic target needed to replace
  saturated load/unload/warmup/trace timing producers; it does not treat the
  current saturated producers as valid legacy behavior. Verification passed:
  `cargo test -p pantograph-workflow-service workflow_timing`,
  `cargo test -p pantograph-workflow-service contracts`,
  `cargo fmt --all -- --check`, and `git diff --check`. Deviation: the first
  `cargo fmt --all -- --check` found one rustfmt-only line wrap in the new
  contract module; `cargo fmt --all` was applied and focused tests plus final
  format verification were rerun successfully. Remaining follow-up: migrate
  runtime load/unload, inference warmup, embedding warmup, and scheduler trace
  producers onto timing attempt records and checked duration math. Full
  baseline/deviation enforcement and scheduler retry/termination policy remain
  the later required policy-completion path.
- 2026-05-12 workflow runtime-load timing attempt producer slice: smallest
  useful vertical slice was limited to workflow-session runtime-load lifecycle
  diagnostics. Load requested, dependency-resolved, completed, and failed
  scheduler model lifecycle events now carry one shared `timing_attempt_` id,
  and runtime-load duration uses checked timing contract arithmetic instead of
  `saturating_sub`. Allowed write set:
  `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `crates/pantograph-diagnostics-ledger/src/tests.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_runtime_load_lifecycle.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_runtime.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because runtime-load
  duration no longer becomes an anonymous saturated value in workflow-session
  execution; impossible timing state returns a typed workflow-service internal
  error through the timing contract. Verification passed:
  `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
  `cargo test -p pantograph-workflow-service session_execution`,
  `cargo test -p pantograph-diagnostics-ledger model_lifecycle`,
  `cargo fmt --all -- --check`, and `git diff --check`. Discovered issue fixed
  during verification: `workflow_execution_session_run_records_snapshot_before_execution`
  queried Library usage without explicitly refreshing the Library usage
  projection, so it could observe zero projected assets when the run emitted
  more events than the small projection batch covered. Remaining follow-up:
  migrate runtime unload, capacity-rebalance unload, inference gateway warmup,
  embedding warmup, and scheduler trace producers onto timing attempt ids and
  checked duration math. Full baseline/deviation enforcement and scheduler
  retry/termination policy remain the later required policy-completion path.
- 2026-05-12 workflow runtime-unload timing attempt producer slice: smallest
  useful vertical slice was limited to keep-alive-disabled workflow-session
  runtime unload. Unload scheduled, started, completed, and failed lifecycle
  events now carry one shared `timing_attempt_` id, and runtime-unload duration
  uses checked timing contract arithmetic instead of `saturating_sub`. Allowed
  write set:
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because
  keep-alive-disabled unload duration no longer becomes an anonymous saturated
  value; impossible timing state returns a typed workflow-service internal
  error through the timing contract. Verification passed:
  `cargo test -p pantograph-workflow-service workflow_execution_session_run_records_snapshot_before_execution`,
  `cargo test -p pantograph-workflow-service session_execution`,
  `cargo fmt --all -- --check`, and `git diff --check`. Remaining follow-up:
  migrate capacity-rebalance unload, inference gateway warmup, embedding
  warmup, and scheduler trace producers onto timing attempt ids and checked
  duration math. Full baseline/deviation enforcement and scheduler
  retry/termination policy remain the later required policy-completion path.
- 2026-05-12 capacity-rebalance unload timing attempt producer slice:
  smallest useful vertical slice was limited to workflow-session
  capacity-rebalance runtime unload. Scheduled, started, completed, and failed
  rebalance lifecycle events now carry one shared `timing_attempt_` id, and
  rebalance unload duration uses checked timing contract arithmetic instead of
  `saturating_sub`. Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/session_runtime.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/session_capacity.rs`,
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because capacity-rebalance
  unload duration no longer becomes an anonymous saturated value; impossible
  timing state returns a typed workflow-service internal error through the
  timing contract. Verification passed:
  `cargo test -p pantograph-workflow-service session_capacity`,
  `cargo fmt --all -- --check`, and `git diff --check`. Remaining follow-up:
  migrate inference gateway warmup, embedding warmup, and scheduler trace
  producers onto timing attempt ids and checked duration math. Full
  baseline/deviation enforcement and scheduler retry/termination policy remain
  the later required policy-completion path.
- 2026-05-12 scheduler trace-span timing attempt producer slice: smallest
  useful vertical slice was limited to workflow-service trace run, node, and
  scheduler queue-wait duration producers. Allowed write set:
  `crates/pantograph-workflow-service/src/trace/scheduler.rs`,
  `crates/pantograph-workflow-service/src/trace/state.rs`,
  `crates/pantograph-workflow-service/src/trace/store.rs`,
  `crates/pantograph-workflow-service/src/trace/types.rs`,
  `crates/pantograph-workflow-service/src/trace/README.md`,
  `crates/pantograph-workflow-service/src/trace/tests.rs`,
  `crates/pantograph-workflow-service/src/trace/tests/lifecycle.rs`,
  `crates/pantograph-workflow-service/src/trace/tests/scheduler_runtime.rs`,
  `crates/pantograph-workflow-service/tests/contract.rs`, and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because trace run, node,
  and queue-wait duration producers no longer use saturated subtraction.
  Timestamp underflow now omits the duration, carries the affected
  `timing_attempt_` id, and emits a typed `timestamp_underflow` timing
  diagnostic instead of normalizing the impossible timestamp state.
  Verification passed:
  `cargo test -p pantograph-workflow-service workflow_trace_store_emits_timing_diagnostic`,
  `cargo test -p pantograph-workflow-service trace::tests`, and
  `cargo test -p pantograph-workflow-service workflow_trace_contract_snapshot`,
  `cargo fmt --all -- --check`, `git diff --check`, and
  `rg -n "saturating_sub" crates/pantograph-workflow-service/src/trace -g '*.rs' -g '!**/tests/**' -g '!**/tests.rs'`
  returned no production trace matches.
  Verification deviation: one attempted focused test command used multiple
  Cargo test filters and failed at argument parsing before running tests; it
  was rerun with a valid shared filter. Remaining follow-up: migrate inference
  gateway warmup and embedding warmup producers onto timing attempt ids and
  checked duration math. Full baseline/deviation enforcement and scheduler
  retry/termination policy remain the later required policy-completion path.
- 2026-05-12 re-plan trigger: remaining inference warmup timing producers
  need shared timing-contract ownership outside workflow-service. Code
  inspection found the remaining production warmup `saturating_sub` sites in
  `crates/inference/src/gateway.rs` and
  `crates/inference/src/embedding_runtime.rs`; `inference` cannot depend on
  `pantograph-workflow-service` without reversing crate layering, and copying
  timing attempt id/diagnostic structs into `inference` would create a second
  non-canonical contract. Required re-plan: choose the shared owner for timing
  attempt ids, checked duration semantics, and timing diagnostics before
  migrating inference warmup producers. Viable options are a dedicated shared
  timing-contract crate, or moving the contract into an existing shared
  foundation crate such as `pantograph-runtime-attribution` with expanded
  ownership. Rejected options are adding a workflow-service dependency to
  `inference` or duplicating the contract locally. Recommendation: use a
  dedicated shared timing-contract crate unless crate-count constraints are
  more important than a crisp ownership boundary.
- 2026-05-12 shared timing-contract crate slice: smallest useful vertical
  slice was to create `pantograph-timing-contracts` as the canonical shared
  owner for timing attempt ids, checked duration semantics, attribution DTOs,
  and timing diagnostics, then migrate workflow-service runtime-load/unload
  and trace producers to consume that crate directly. Allowed write set: root
  `Cargo.toml`, `Cargo.lock`, `crates/pantograph-timing-contracts/**`,
  `crates/pantograph-workflow-service/Cargo.toml`,
  `crates/pantograph-workflow-service/src/workflow.rs`,
  `crates/pantograph-workflow-service/src/workflow/README.md`,
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_runtime.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests.rs`,
  `crates/pantograph-workflow-service/src/trace/scheduler.rs`,
  `crates/pantograph-workflow-service/src/trace/state.rs`,
  `crates/pantograph-workflow-service/src/trace/store.rs`,
  `crates/pantograph-workflow-service/src/trace/types.rs`,
  `crates/pantograph-workflow-service/src/trace/tests/lifecycle.rs`,
  `crates/pantograph-workflow-service/src/trace/tests/scheduler_runtime.rs`,
  removed workflow-local timing contract files, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because the
  workflow-local timing contract was removed rather than kept as a
  compatibility shim. Workflow-service imports the shared canonical crate
  directly, so inference can migrate without duplicating timing DTOs or
  depending on workflow orchestration. Verification passed:
  `cargo test -p pantograph-timing-contracts`,
  `cargo test -p pantograph-workflow-service workflow_trace_store_emits_timing_diagnostic`,
  `cargo test -p pantograph-workflow-service trace::tests`,
  `cargo test -p pantograph-workflow-service session_capacity`,
  `cargo test -p pantograph-workflow-service session_execution`, and
  `cargo fmt --all -- --check`, `cargo check -p inference`, and
  `git diff --check`. Verification deviation/discovered issue: the
  first `cargo test -p pantograph-timing-contracts` compile exposed that the
  new crate's serde contract tests used `serde_json` without declaring it as a
  dev-dependency. Added `serde_json.workspace = true` under
  `[dev-dependencies]` and reran successfully. Remaining follow-up: migrate
  inference gateway warmup and embedding warmup producers onto
  `pantograph-timing-contracts` timing attempt ids and checked duration math.
  Full baseline/deviation enforcement and scheduler retry/termination policy
  remain the later required policy-completion path.
- 2026-05-12 inference warmup timing attempt producer slice: smallest useful
  vertical slice was to migrate inference gateway warmup and dedicated
  embedding runtime warmup producers onto `pantograph-timing-contracts` timing
  attempt ids and checked duration math. Allowed write set: `Cargo.lock`,
  `crates/inference/Cargo.toml`, `crates/inference/src/types.rs`,
  `crates/inference/src/gateway.rs`,
  `crates/inference/src/embedding_runtime.rs`,
  `crates/inference/src/gateway_tests.rs`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because inference warmup
  duration producers no longer use saturated subtraction. Warmup attempts now
  create `timing_attempt_` ids, successful warmups use checked duration math,
  and impossible timestamp order records typed `runtime_warmup`
  `timestamp_underflow` diagnostics instead of synthesizing a duration.
  Verification passed: `cargo test -p inference runtime_lifecycle_snapshot`,
  `cargo test -p inference test_runtime_lifecycle_snapshot`,
  `cargo check -p inference`, `cargo fmt --all -- --check`,
  `git diff --check`, and
  `rg -n "saturating_sub" crates/inference/src/gateway.rs crates/inference/src/embedding_runtime.rs`
  returned no matches. Verification deviation/discovered issue: the first
  focused inference compile exposed that the gateway helper used
  `Option<InferenceStartRequest>` for saved previous backend config even
  though the field stores `Option<BackendConfig>`. Corrected the helper
  signature and reran focused tests successfully. Remaining follow-up: full
  baseline/deviation enforcement, scheduler reschedule policy, retry
  exhaustion, and terminal workflow failure semantics remain the later
  required policy-completion path.
- 2026-05-12 re-plan trigger reached before Milestone 6 implementation. Status
  was inspected first and only the approved unrelated files were dirty:
  `.pantograph/workflow-diagnostics.sqlite-shm`,
  `.pantograph/workflow-diagnostics.sqlite-wal`, and
  `PROPOSAL-pumas-library-fast-model-snapshot.md`. Smallest useful next slice
  considered was the initial image-generation planner contract consuming
  Pumas Diffusers facts, but the plan's Pumas gate is not satisfied:
  Pantograph root `Cargo.toml` pins `pumas-library` to tag `v0.6.0`, the
  locked source at commit `6d038ff8` exposes
  `PACKAGE_FACTS_CONTRACT_VERSION = 1`, and Pantograph inference DTOs/fixtures
  now expect package-facts contract version 2. No implementation slice was
  started because using the local fixture alone would bypass the pinned Pumas
  producer contract and create a non-canonical bridge. Required re-plan: select
  the Pumas release/tag or commit that contains contract version 2 and the P6
  cross-repo fixture guarantees, then decide whether the next Pantograph slice
  is a dedicated dependency pin/update slice or the planner contract slice
  after that dependency boundary is resolved. Verification for this boundary:
  `rg -n "pumas-library|v0\\.6\\.0|6d038ff|contract 2|contract-version" docs/plans/current-image-generation-graphs Cargo.toml Cargo.lock crates -g '!target'`,
  `rg -n "MODEL_PACKAGE_FACTS_CONTRACT_VERSION|package_facts_contract_version|diffusers_sd_text_to_image_package_facts" crates/inference crates/pantograph-embedded-runtime crates/workflow-nodes -g '!target'`,
  and `rg -n "MODEL_PACKAGE_FACTS_CONTRACT_VERSION|PACKAGE_FACTS_CONTRACT_VERSION" ~/.cargo/git/checkouts -g '*.rs'`.
- 2026-05-12 dependency-boundary slice: smallest useful vertical slice was to
  pin Pantograph's workspace `pumas-library` dependency to Pumas commit
  `281a45a5bc604975ebd0d5e71d12adaa5a228382`, the contract-version-2 producer
  revision recorded by the Pumas P6 fixture handoff, then verify direct
  Pantograph consumers of the Pumas contract. Allowed write set: root
  `Cargo.toml`, `Cargo.lock`, `crates/pantograph-embedded-runtime/**`,
  `crates/pantograph-frontend-http-adapter/src/lib.rs`, and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because Pantograph now
  consumes a pinned Pumas producer revision with package-facts contract version
  2 instead of using a local fixture-only bridge or mapping old contract
  version 1 facts forward. Verification passed:
  `cargo check -p workflow-nodes --features model-library`,
  `cargo check -p pantograph-embedded-runtime`,
  `cargo test -p pantograph-embedded-runtime runtime_registry_resource_accounting_errors_map_to_internal`,
  `cargo test -p inference --test model_contracts package_fact`,
  `cargo test -p inference --test model_contracts pumas_image_generation_fixture_decodes_with_structured_diffusers_facts`,
  `cargo test -p pantograph-embedded-runtime pumas_package_facts`,
  `cargo check -p pantograph-uniffi`,
  `cargo check -p pantograph_rustler`,
  `cargo check -p pantograph-frontend-http-adapter`,
  `cargo test -p pantograph-frontend-http-adapter map_workflow_error_envelope_ignores_graph_details_for_scheduler_busy`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Verification deviations/discovered issues: the first `cargo update -p
  pumas-library` failed under the sandbox because GitHub DNS/network access was
  unavailable, then succeeded with approved network escalation. An attempted
  `cargo check -p pantograph-rustler` used the directory-style package name and
  failed before checking code; reran as `cargo check -p pantograph_rustler`.
  Direct consumer verification also exposed pre-existing compile gaps from
  earlier contract changes: embedded-runtime snapshots and tests were missing
  the new runtime warmup timing fields, embedded-runtime did not map the new
  runtime-registry resource accounting errors, and the frontend HTTP adapter
  did not explicitly ignore graph error details when reconstructing scheduler
  errors. These were fixed with explicit field propagation and explicit match
  arms, not wildcard fallbacks. A broader non-slice verification filter,
  `cargo test -p pantograph-embedded-runtime runtime_registry`, still fails in
  two session/runtime tests because current no-fallback technical-fit auto
  selection reports `ambiguous_auto_resolution` for equal-ranked candidates:
  `keep_alive_disable_reclaim_flips_scheduler_runtime_registry_diagnostics_to_start_runtime`
  and
  `execute_edit_session_graph_restore_keeps_scheduler_runtime_registry_diagnostics_ready`.
  The fix is deferred to the Milestone 6 planner/test-fixture slice because it
  needs explicit canonical workflow backend/device intent rather than
  reintroducing implicit auto selection or fixture-only backend hints.
- Remaining follow-up: begin Milestone 6 planner implementation with a focused
  contract slice that consumes pinned Pumas Diffusers facts and returns typed
  diagnostics for missing or ambiguous image-family evidence.
- 2026-05-12 planner contract slice: smallest useful vertical slice was to add
  the first side-effect-free inference planner contract for image generation.
  Allowed write set: `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`,
  `crates/inference/src/lib.rs`, `crates/inference/src/README.md`, and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because planning requires
  the already-selected PyTorch backend/runtime/device decision and current
  Pumas Diffusers facts. Missing Diffusers evidence, non-PyTorch backend
  decisions, ambiguous family evidence, unsupported families, missing Stable
  Diffusion component roles, invalid numeric options, and resource-estimate
  overflow return typed diagnostics instead of trying alternate backends,
  aliases, family-name inference, default devices, or generic Diffusers
  loading. Verification passed: `cargo test -p inference
  image_generation_planner`, `cargo check -p inference`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Remaining follow-up: wire the planner into PyTorch image generation, add
  dependency/path/default/family diagnostics, and then feed the planner into
  worker envelope construction.
- 2026-05-12 worker image-envelope contract slice: smallest useful vertical
  slice was to add Rust-side PyTorch worker image-generation request/response
  DTOs, the `generate_image` worker operation tag, JSON fixtures, and focused
  validation tests without invoking Python generation. Allowed write set:
  `crates/inference/src/backend/pytorch_worker_contract.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract_tests.rs`,
  `crates/inference/src/backend/pytorch.rs`,
  `crates/inference/src/backend/README.md`,
  `crates/inference/tests/fixtures/pytorch_worker_contract/`, and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because the worker image
  envelope carries planner-selected family/component/device facts and rejects
  unknown request fields such as `trust_remote_code`; it does not expose
  generic Diffusers loading, custom-code trust, backend aliases, or device
  fallback controls to Python. Verification passed: `cargo test -p inference
  --features backend-pytorch pytorch_worker_generate_image`,
  `cargo check -p inference --features backend-pytorch`,
  `cargo check -p inference`, `cargo fmt --all -- --check`, and
  `git diff --check`.
- Discovered standards debt: `crates/inference/src/backend/pytorch.rs` and
  `crates/inference/src/backend/pytorch_tests.rs` are already over the
  decomposition threshold. The slice kept new image DTOs and tests in focused
  files; later execution wiring should avoid adding more policy or test bulk to
  those files.
- Remaining follow-up: translate `ImageGenerationExecutionPlan` into the worker
  envelope and add Python-side shape validation before generation.
- 2026-05-12 planner-to-worker translation slice: smallest useful vertical
  slice was to translate `ImageGenerationExecutionPlan` into
  `PyTorchGenerateImageRequest` and prove the resulting worker envelope still
  validates. Allowed write set:
  `crates/inference/src/backend/pytorch_worker_image_contract.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract_tests.rs`, and
  this plan directory.
- The slice preserves the no-fallback/no-legacy rule because translation copies
  the planner-selected model ref, artifact path, family, component roles,
  pipeline class, prompt options, and selected device id directly. It does not
  reinterpret backend hints, parse raw device strings, choose defaults, or infer
  family from model names. Verification passed: `cargo test -p inference
  --features backend-pytorch
  pytorch_worker_generate_image_request_maps_from_validated_plan`, `cargo test
  -p inference --features backend-pytorch pytorch_worker_generate_image`,
  `cargo check -p inference --features backend-pytorch`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Remaining follow-up: add Python worker-side image envelope shape validation,
  then wire PyTorch backend image generation through the validated plan and
  envelope.
- 2026-05-12 Python image-envelope validation slice: smallest useful vertical
  slice was to add torch-free Python validation/projection for the
  Rust-planned `generate_image` worker envelope without loading Diffusers or
  wiring backend execution. Allowed write set:
  `crates/inference/torch/worker_image_contract.py`,
  `crates/inference/torch/README.md`,
  `crates/inference/src/backend/pytorch_worker_image_contract_tests.rs`, and
  this plan directory.
- The slice preserves the no-fallback/no-legacy rule because Python now rejects
  unknown image payload fields, including `trust_remote_code`, requires a
  Rust-selected canonical device id, and only projects the already-planned
  generation kwargs. It does not choose pipeline family, scheduler,
  custom-code trust, or device fallback. Verification passed:
  `cargo test -p inference --features backend-pytorch
  python_worker_generate_image_contract` and `cargo test -p inference
  --features backend-pytorch pytorch_worker_generate_image`,
  `cargo check -p inference --features backend-pytorch`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Verification deviation/discovered issue: the first implementation put the
  image helper in `worker_contract.py`, pushing it past the decomposition
  target. The slice was corrected by moving image-specific validation into
  `worker_image_contract.py` and updating the torch README. The first
  `cargo fmt --all -- --check` reported formatting changes in the new
  Rust/PyO3 tests; ran `cargo fmt --all` and reran successfully. Remaining
  follow-up: import the helper from `worker.py` and wire
  `PyTorchBackend::generate_image` through the validated planner and envelope.
- 2026-05-12 Python image worker bridge slice: smallest useful vertical slice
  was to register `worker_image_contract.py` in the embedded Python worker
  loader, import it from `worker.py`, and expose `generate_image_from_envelope`
  so Python validates the Rust-planned image worker envelope before calling the
  existing loaded Diffusers pipeline helper. Allowed write set:
  `crates/inference/src/backend/pytorch_worker.rs`,
  `crates/inference/src/backend/pytorch_worker_image_python_tests.rs`,
  `crates/inference/src/backend/pytorch.rs`, `crates/inference/torch/worker.py`,
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because the worker
  entrypoint uses the strict image envelope contract and returns typed worker
  error responses for invalid requests. It does not choose pipeline family,
  scheduler, custom-code trust, dependency environment, or device fallback in
  Python. Verification passed: `cargo test -p inference --features
  backend-pytorch python_worker_generate_image_from_envelope`, `cargo test -p
  inference --features backend-pytorch python_worker_generate_image_contract`,
  `cargo test -p inference --features backend-pytorch
  pytorch_worker_generate_image`, `cargo check -p inference --features
  backend-pytorch`, `cargo fmt --all -- --check`, and `git diff --check`.
- Verification deviation/discovered issue: the first bridge test run exposed a
  stub scoping bug (`_Image` was not visible inside the stub pipeline method),
  which was corrected by using one Python globals/locals dictionary. Standards
  debt: `crates/inference/torch/worker.py` is already above the decomposition
  threshold; this slice kept validation and tests in focused files and only
  added the minimal public bridge function required by the existing Rust/PyO3
  facade. Remaining follow-up: wire `PyTorchBackend::generate_image` through
  the validated planner and Python worker envelope.
- 2026-05-12 planned Rust image backend helper slice: smallest useful vertical
  slice was to add a focused PyTorch image-generation Rust helper that consumes
  `ImageGenerationExecutionPlan`, builds the validated worker envelope, invokes
  `generate_image_from_envelope`, and maps typed worker responses into
  `ImageGenerationResult`. Allowed write set:
  `crates/inference/src/backend/pytorch.rs`,
  `crates/inference/src/backend/pytorch_image_generation.rs`,
  `crates/inference/src/backend/pytorch_image_generation_tests.rs`, and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because it accepts only
  the validated Rust execution plan and does not build an image request from
  raw graph backend hints, raw device strings, model names, or request-only
  defaults. The existing `InferenceBackend::generate_image(ImageGenerationRequest)`
  trait method remains unwired rather than bypassing the planner. Verification
  passed: `cargo test -p inference --features backend-pytorch
  pytorch_image_generation`, `cargo check -p inference --features
  backend-pytorch`, `cargo fmt --all -- --check`, and `git diff --check`.
- Verification deviation: the first `cargo fmt --all -- --check` reported
  formatting changes in the new module/test files; ran `cargo fmt --all` and
  reran successfully.
- Remaining follow-up/re-plan boundary: the public inference backend trait and
  gateway image-generation path currently carry only `ImageGenerationRequest`;
  they do not carry Pumas package facts or the scheduler-owned
  `BackendExecutionDecision` required by the no-fallback planner. Full
  end-to-end gateway wiring needs a planned-context boundary instead of
  reconstructing facts from request fields.
- 2026-05-12 planning update after scheduler-policy review: automatic
  backend/runtime/device selection must be treated as a scheduler policy, not
  as an ambiguity failure whenever more than one valid candidate exists.
  Workflow graphs remain intent-first and may optionally constrain backend,
  runtime variant, or device, but they should not be required to specify local
  runtime topology. The scheduler must hard-filter invalid candidates, rank
  valid candidates using readiness/residency, resource pressure, workflow
  priority, and diagnostics-ledger history, and may use controlled seeded
  exploration when history is insufficient. Auto fails only when no valid
  candidate remains, an explicit workflow constraint is incompatible, or the
  policy cannot legally select one candidate. This supersedes the earlier
  temporary `ambiguous_auto_resolution` direction for equal-ranked valid
  candidates; the required implementation follow-up is to replace ambiguity as
  a terminal auto result with recorded ranking/exploration policy while keeping
  typed diagnostics for genuine no-decision cases.
- 2026-05-12 standards iteration for scheduler-policy update: reviewed the
  new automatic-selection direction against architecture, testing,
  concurrency, Rust async/API, and security standards. Added guardrails that
  split implementation into contract, pure-policy, ledger-summary, scheduler
  integration, and presentation slices; keep workflow graph nodes free of
  Pumas facts, ledger summaries, candidate lists, and selected decisions;
  require ranking to be synchronous and lock-free over validated candidate
  facts; require controlled exploration to record policy version, seed basis,
  eligible candidates, selected candidate, and reason; and require isolated
  ledger tests plus Rust/TypeScript fixture coverage for append-only decision
  fields. This standards pass confirms the intended blast radius is scheduler
  contracts/policy, diagnostics-ledger summaries, scheduler integration, and
  backend-owned projections only.
- 2026-05-12 codebase impact review for scheduler-policy update: investigated
  the touched code paths and recorded the agreed re-plan boundary before any
  source implementation. Current graph contracts/templates still expose
  resolved Pumas package facts as graph-visible inference edges, while the
  updated architecture requires inference nodes to stay intent-first. The next
  implementation sequence is therefore contract fields, graph-boundary cleanup,
  executable candidate synthesis, pure scheduler policy, bounded ledger
  summaries, scheduler integration, and planned image gateway wiring.
- Discovered design debts now recorded in the plan: runtime-registry still has
  temporary equal-priority `ambiguous_auto_resolution` behavior; embedded
  runtime currently has Pumas-derived candidate fragments without complete
  runtime/device facts; Pumas lookup failures can degrade into empty facts and
  must become typed candidate diagnostics; `BackendExecutionDecision` needs
  append-only policy evidence fields before behavior changes; and the image
  gateway still has a request-only dispatch path that must be replaced by a
  planned execution context. Decision: defer source edits until these slices
  are taken in order so the no-fallback/no-legacy rule is preserved.
- 2026-05-12 scheduler policy trace contract slice: added append-only policy
  trace DTOs to backend execution, runtime technical-fit, workflow
  technical-fit, embedded-runtime projection, and TypeScript workflow mirrors.
  Added `automatic_ranking` and `controlled_exploration` reason codes plus
  fixture coverage for policy version, candidate-set summary, ranking reason,
  optional exploration reason, and seed basis. This was contract-only: selector
  behavior, candidate synthesis, ledger reads, and image gateway dispatch are
  unchanged.
- Verification passed: `cargo test -p inference --test device_contracts`,
  `cargo test -p pantograph-runtime-registry --test technical_fit_contract`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-workflow-service --test contract workflow_technical_fit_cross_layer_fixture_deserializes`,
  `cargo test -p pantograph-workflow-service technical_fit`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `cargo check -p inference --features backend-pytorch`,
  `cargo test -p inference --features backend-pytorch image_generation_planner`,
  `cargo check -p pantograph-runtime-registry -p pantograph-workflow-service -p pantograph-embedded-runtime -p inference`,
  `node --experimental-strip-types --test src/services/workflow/WorkflowService.commands.test.ts`,
  `npm run typecheck`, `cargo fmt --all -- --check`, and
  `git diff --check`.
- Verification deviations fixed: added the missing workflow-service list
  normalization helper, re-exported the new trace DTOs from public crates,
  removed JSON `null` fields that serde omits, and updated remaining
  workflow-service test literals with explicit empty trace fields.
- 2026-05-12 graph-boundary package-facts cleanup slice: smallest useful
  vertical slice was to remove graph-visible full Pumas package fact/source
  ports from canonical inference descriptors, frontend definitions, built-in
  templates, tracked current image-generation saved workflows, and node-engine
  dependency input repair. Allowed write set:
  `.pantograph/workflows/juggernaut-x-v10-sdxl.json`,
  `.pantograph/workflows/tiny-sd-turbo-diffusion.json`,
  `crates/workflow-nodes/src/processing/inference.rs`,
  `crates/workflow-nodes/src/input/puma_lib.rs`,
  `crates/workflow-nodes/src/contracts.rs`,
  `crates/node-engine/src/engine/dependency_inputs.rs`,
  `src/services/workflow/mocks.ts`,
  `src/services/workflow/WorkflowService.commands.test.ts`,
  `src/services/workflow/templateService.test.ts`,
  `src/templates/workflows/gguf-reranker-workflow.json`,
  `src/templates/workflows/tiny-sd-turbo-text-to-image.json`, and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because workflow graphs
  now route only intent-level `pumas_model_ref` into canonical inference.
  Retired package-facts/source target handles are ignored during dependency
  input preparation instead of merged forward as compatibility context, and no
  descriptor alias or old port shim remains. Runtime planners and direct
  executor package-facts parsing are unchanged for the planned candidate
  synthesis and planned-context gateway slices.
- Verification passed:
  `cargo test -p workflow-nodes test_descriptor_has_canonical_inference_contract_ports`,
  `cargo test -p workflow-nodes test_descriptor_has_correct_ports`,
  `cargo test -p workflow-nodes llm_inference_contract_exposes_inference_task_payload_metadata`,
  `cargo test -p node-engine dependency_inputs`,
  `node --experimental-strip-types --test src/services/workflow/WorkflowService.commands.test.ts`,
  `node --experimental-strip-types --test src/services/workflow/templateService.test.ts`,
  `cargo check -p workflow-nodes -p node-engine`,
  `npm run typecheck`, `cargo fmt --all -- --check`, and
  `git diff --check`.
- Verification deviations/discovered issues: the first node-engine dependency
  input run exposed a stale positive test that still expected package facts to
  merge through a `pumas_model_ref` edge; the test was corrected to prove
  intent-only model reference flow. The first template-service test rerun
  exposed resolved package-facts edges in the tracked current Juggernaut and
  Tiny SD saved workflow fixtures; those were removed as part of this slice.
  Code search also found an ignored/untracked local
  `.pantograph/workflows/Maid-Qwen3.6-27b-heritic.json` containing old
  resolved-model port metadata; it was not committed because it is outside the
  tracked workflow fixture set and remains local stale data.
- 2026-05-12 embedded-runtime executable candidate synthesis slice: smallest
  useful vertical slice was to join Pumas package facts with runtime
  capability facts during embedded-runtime technical-fit request construction.
  Allowed write set:
  `crates/pantograph-embedded-runtime/src/technical_fit.rs` and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because Pumas-derived
  candidates without matching runtime capability/runtime-variant facts are now
  non-selectable and emit typed `missing_runtime_variant` diagnostics instead
  of becoming executable decisions with empty runtime/device fields. When
  backend compatibility already rejects the model/task, that more specific
  backend diagnostic remains the rejection reason. Selector ranking, ledger
  summaries, and image gateway dispatch are unchanged.
- Verification passed:
  `cargo test -p pantograph-embedded-runtime pumas_package_facts_candidates`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo check -p pantograph-embedded-runtime -p pantograph-runtime-registry`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Verification deviations fixed: the first focused compile exposed moved-value
  issues in the new candidate builders; the first focused test run showed that
  base runtime capability candidates are still included alongside Pumas
  candidates, so counts were updated to assert the synthesized Pumas candidate
  directly. The broader technical-fit run also caught diagnostic-priority
  regressions for explicit Candle/vLLM image-generation overrides; missing
  runtime capability diagnostics are now suppressed when backend compatibility
  already supplies the blocking reason.
- 2026-05-12 runtime-registry controlled exploration policy slice: smallest
  useful vertical slice was to replace the temporary equal-priority automatic
  ambiguity failure with recorded automatic policy evidence and deterministic
  controlled exploration in the pure runtime-registry selector. Allowed write
  set: `crates/pantograph-runtime-registry/src/technical_fit.rs`,
  `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`, and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because automatic
  selection still hard-filters invalid candidates and returns typed diagnostics
  for no valid candidate, explicit incompatibility, or unrankable policy state.
  Equal-ranked valid candidates are selected by recorded scheduler policy,
  carrying `automatic_ranking` and `controlled_exploration` reasons plus
  policy version, candidate-set summary, ranking reason, and seed basis. No
  workflow graph facts, Pumas facts, ledger state, runtime lifecycle behavior,
  image gateway dispatch, frontend behavior, generated files, lockfiles, or
  workflow fixtures changed.
- Verification passed:
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo check -p pantograph-runtime-registry`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Verification deviation fixed: the first `cargo fmt --all -- --check`
  reported local rustfmt changes in the selector helper block; ran
  `cargo fmt --all` and reran formatting successfully.
- Remaining follow-up: ledger-history ranking inputs, scheduler retry/
  termination integration, diagnostics-ledger policy projection, and planned
  image gateway dispatch remain later slices. The public
  `ambiguous_auto_resolution` wire value remains append-only DTO history, but
  equal-ranked valid automatic selection no longer produces it.
- 2026-05-12 re-plan trigger reached before diagnostics-ledger policy
  projection. Investigation found that `SchedulerRunAdmittedPayload` is emitted
  immediately after queue admission, before `ensure_session_runtime_preflight`
  obtains the canonical `WorkflowTechnicalFitDecision`. Attaching
  `selection_policy_trace` to that event would either require moving admission
  event timing, duplicating a later synthetic admission event, or adding a new
  scheduler technical-fit decision event/projection. Required re-plan: choose
  the durable ledger event boundary for scheduler policy evidence so selected
  candidate, ranking/exploration reason, candidate summary, and future
  ledger-history inputs are recorded once without leaking policy facts into
  workflow graphs or runtime lifecycle events.
- 2026-05-12 scheduler admission policy-ledger boundary slice: option 1 was
  selected. Smallest useful vertical slice was to move
  `scheduler.run_admitted`, reservation-created, and run-started diagnostic
  ledger events until after runtime technical-fit preflight on the successful
  queue-admission path, and to append technical-fit policy evidence to
  `SchedulerRunAdmittedPayload`. Allowed write set:
  `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `crates/pantograph-diagnostics-ledger/src/lib.rs`,
  `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
  `crates/pantograph-diagnostics-ledger/src/tests.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because it only changes
  when already-canonical scheduler decisions are recorded. It does not change
  candidate synthesis, technical-fit selection, queue admission selection,
  workflow graph facts, Pumas fact visibility, runtime lifecycle selection,
  frontend behavior, generated files, lockfiles, workers, or workflow
  fixtures. The admission payload now records selected runtime variant,
  selected backend key, and bounded technical-fit policy trace facts after the
  `WorkflowTechnicalFitDecision` exists.
- Verification passed:
  `cargo test -p pantograph-diagnostics-ledger scheduler_timeline`,
  `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
  `cargo test -p pantograph-workflow-service session_execution`,
  `cargo check -p pantograph-diagnostics-ledger -p pantograph-workflow-service`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Verification deviations fixed: the first workflow-service compile exposed
  missing public re-exports for the new diagnostics-ledger policy trace DTOs
  and missing focused test imports. The first broad `session_execution` run
  exposed stale assertions that expected the admission event to keep using the
  old required-backend placeholder runtime and expected reservation-created to
  precede admission; those were updated to the option 1 event order and
  technical-fit selected runtime facts.
- Remaining follow-up: diagnostics-ledger run-list/run-detail projections still
  persist selected backend/runtime variant primarily through existing
  reservation, lifecycle, and node-status paths. A later projection slice can
  promote admission policy trace summaries into queryable columns or compact
  read-model fields if UI/history ranking requires that; ledger-history ranking
  inputs and retry/termination policy remain later scheduler slices.
- 2026-05-12 diagnostics-ledger admission selected-facts projection slice:
  smallest useful vertical slice was to project the selected backend key and
  selected runtime variant id from the now-canonical `scheduler.run_admitted`
  payload into existing run-list and run-detail selected-fact columns. Allowed
  write set: `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
  `crates/pantograph-diagnostics-ledger/src/tests.rs`, and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because it only projects
  selected facts already recorded on the scheduler admission event. It does not
  add schema columns, infer runtime/backend facts from graph fields, change
  scheduler selection, read ledger history for ranking, touch workflow-service
  admission behavior, or change frontend/worker/runtime paths.
- Verification passed:
  `cargo test -p pantograph-diagnostics-ledger run_projections_capture_scheduler_admission_selected_policy_facts`,
  `cargo test -p pantograph-diagnostics-ledger scheduler_timeline`,
  `cargo check -p pantograph-diagnostics-ledger`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Verification deviation fixed: the first focused projection test exposed that
  the helper patch had been inserted at the wrong projection boundary and was
  updating the run-detail table during run-list projection. The helper was
  split into explicit run-list and run-detail functions and the focused test
  was rerun successfully.
- Remaining follow-up: technical-fit policy trace itself remains payload-level
  evidence. A later read-model slice can add compact policy-summary columns or
  history aggregation once ledger-history ranking inputs are designed.
- 2026-05-12 diagnostics-ledger admission policy-trace serde contract slice:
  smallest useful vertical slice was to pin the scheduler admission policy
  trace wire shape with an inline serde contract test in the existing
  diagnostics-ledger test module. Allowed write set:
  `crates/pantograph-diagnostics-ledger/src/tests.rs` and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because it adds only
  contract coverage for already-recorded canonical scheduler admission facts.
  It does not change selector policy, add compatibility aliases, infer facts
  from graph-visible Pumas fields, alter schema/projections, touch frontend or
  worker DTOs, change lockfiles, or update workflow fixtures.
- Verification passed:
  `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted_payload_round_trips_policy_trace_contract`,
  `cargo test -p pantograph-diagnostics-ledger scheduler_timeline`,
  `cargo check -p pantograph-diagnostics-ledger`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Remaining follow-up: ledger-history ranking inputs, retry/termination
  policy, and optional compact read-model policy summaries remain later
  scheduler slices.
- 2026-05-12 diagnostics-ledger policy-trace count validation slice: smallest
  useful vertical slice was to make scheduler admission policy trace candidate
  summaries fail closed when their count fields are impossible or when the
  eligible-candidate id list does not match the eligible count. Allowed write
  set: `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `crates/pantograph-diagnostics-ledger/src/tests.rs`, and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because malformed
  scheduler policy evidence now returns typed diagnostics-ledger validation
  errors instead of being accepted, saturated, normalized, or silently trimmed.
  It does not change selector policy, schema/projections, runtime execution,
  workflow graph facts, frontend behavior, workers, lockfiles, or workflow
  fixtures.
- Verification passed:
  `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted_rejects_inconsistent_policy_trace_counts`,
  `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted_payload_round_trips_policy_trace_contract`,
  `cargo check -p pantograph-diagnostics-ledger`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Remaining follow-up: runtime-registry still owns the canonical selection
  policy and already uses checked candidate counts before producing these
  payloads. Ledger-history ranking inputs, retry/termination policy, and
  optional compact read-model policy summaries remain later scheduler slices.
- 2026-05-12 inference gateway image output estimate overflow slice: smallest
  useful vertical slice was to reject image-generation requests whose width,
  height, and image count overflow the conservative RGBA output byte estimate
  before dispatching to the active backend. Allowed write set:
  `crates/inference/src/gateway.rs`,
  `crates/inference/src/gateway_tests.rs`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because impossible image
  output sizes now fail with `BackendError::Config` at the gateway boundary
  instead of reaching backend execution or being saturated/clamped. It does not
  add semantic caps, alter planner/runtime selection, change worker contracts,
  touch frontend behavior, generated files, lockfiles, or workflow fixtures.
- Verification passed:
  `cargo test -p inference test_generate_image_rejects_output_byte_estimate_overflow`,
  `cargo test -p inference test_generate_image_rejects_zero`,
  `cargo check -p inference`, `cargo fmt --all -- --check`, and
  `git diff --check`.
- Remaining follow-up: broader semantic image request limits, context/batch
  limits, byte-range projections, and worker/runtime request fields remain
  open numeric-boundary work.
- 2026-05-14 runtime-selection policy boundary extraction slice: smallest
  useful vertical slice was to move the existing automatic technical-fit
  filtering, ranking, controlled exploration, and automatic no-decision
  diagnostic assembly behind an in-crate pure `runtime_selection_policy`
  module while preserving the public `select_runtime_technical_fit` facade and
  explicit override behavior. Allowed write set:
  `crates/pantograph-runtime-registry/src/lib.rs`,
  `crates/pantograph-runtime-registry/src/technical_fit.rs`,
  `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`, and
  this plan directory.
- The slice preserves the no-fallback/no-legacy rule because it only delegates
  the existing automatic selector behavior. It does not add history ranking,
  candidate caps, Pumas fact fallback, compatibility aliases, workflow
  admission scheduler changes, diagnostics-ledger schema/DTO changes,
  TypeScript mirrors, generated files, lockfiles, or workflow fixtures.
- Verification passed:
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-runtime-registry`, and `cargo fmt --package
  pantograph-runtime-registry`.
- Verification deviation fixed: the first focused compile showed explicit
  override matching still needed the shared eligibility predicate. The
  predicate now lives in the runtime-selection policy module and is exposed
  only crate-internally for the technical-fit facade.
- Remaining follow-up: add internal validated runtime-selection input/output
  types behind the existing serde facade before changing candidate synthesis,
  cross-layer DTOs, scheduler history summaries, or the five-run threshold
  ranking algorithm.
- 2026-05-14 validated internal runtime-selection decision boundary slice:
  smallest useful vertical slice was to add internal
  `RuntimeSelectionDecisionInput` and `RuntimeSelectionDecision` wrappers
  behind the existing `RuntimeTechnicalFitRequest`/`RuntimeTechnicalFitDecision`
  serde facade, with a focused guard that rejects unnormalized requests before
  policy execution. Allowed write set:
  `crates/pantograph-runtime-registry/src/technical_fit.rs`,
  `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`,
  `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`, and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because public wire DTOs,
  selector facade behavior, explicit override behavior, automatic ranking,
  candidate synthesis, diagnostics-ledger contracts, TypeScript mirrors,
  generated files, lockfiles, and workflow fixtures were not changed. Invalid
  internal policy input now has a typed diagnostic path instead of unchecked
  policy execution.
- Verification passed:
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-runtime-registry`, and `cargo fmt --package
  pantograph-runtime-registry`.
- Remaining follow-up: candidate synthesis still needs required Pumas fact
  diagnostics and documented candidate-cap overflow diagnostics before
  cross-layer trace/admission/history work.
- 2026-05-14 generic runtime-capability variant candidate expansion slice:
  smallest useful vertical slice was to change the embedded-runtime generic
  runtime-capability technical-fit projection to emit one candidate per runtime
  variant instead of collapsing to the first available or first variant before
  runtime-selection policy can compare choices. Allowed write set:
  `crates/pantograph-embedded-runtime/src/technical_fit.rs` and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because it removes a
  pre-policy variant collapse. Unavailable variants remain visible as
  non-selectable diagnostic candidates; no Pumas fallback, compatibility shim,
  public DTO, generated file, lockfile, workflow fixture, frontend, or ledger
  contract changed.
- Verification passed:
  `cargo test -p pantograph-embedded-runtime runtime_request_projection_emits_all_runtime_variant_candidates`,
  `cargo test -p pantograph-embedded-runtime technical_fit`, and `cargo fmt
  --package pantograph-embedded-runtime`; `git diff --check`.
- Verification deviation fixed: the first focused compile showed candidate
  readiness was checked after moving variant diagnostics into the candidate.
  Readiness is now computed before candidate construction, and the dead
  first-variant helper was removed with the collapse behavior it encoded.
- Remaining follow-up: candidate synthesis still needs typed diagnostics for
  required Pumas fact absence and documented candidate-cap overflow before
  append-only trace/admission/history work.
- 2026-05-14 missing required Pumas package-fact diagnostics slice: smallest
  useful vertical slice was to add a typed `missing_model_package_facts`
  technical-fit diagnostic, synthesize non-selectable candidates for required
  models whose Pumas package facts were unavailable, and have automatic
  no-valid decisions return scoped candidate diagnostics when present. Allowed
  write set: `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
  `crates/pantograph-runtime-registry/src/technical_fit.rs`,
  `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`,
  `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
  `crates/pantograph-workflow-service/src/technical_fit.rs`,
  `src/services/workflow/types.ts`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because host
  technical-fit planning now fails candidate selection when required model
  package facts are absent instead of selecting from generic runtime capability
  facts. The slice adds a canonical diagnostic and TypeScript mirror value; it
  does not add compatibility aliases, legacy graph behavior, generated files,
  lockfile changes, workflow fixtures, diagnostics-ledger schema changes, or
  runtime loading.
- Verification passed:
  `cargo test -p pantograph-runtime-registry selector_surfaces_scoped_candidate_diagnostics_when_no_candidate_is_valid`,
  `cargo test -p pantograph-embedded-runtime missing_required_package_facts_block_capability_only_selection`,
  `cargo test -p pantograph-runtime-registry technical_fit`, `cargo test -p
  pantograph-embedded-runtime technical_fit`, `cargo test -p
  pantograph-workflow-service technical_fit`, `npm run typecheck`, and `cargo
  fmt --package pantograph-embedded-runtime --package pantograph-runtime-registry
  --package pantograph-workflow-service`; `git diff --check`.
- Remaining follow-up: candidate synthesis still needs documented
  candidate-cap overflow diagnostics before append-only trace/admission/history
  work.
- 2026-05-14 candidate-synthesis cap overflow diagnostics slice: smallest
  useful vertical slice was to enforce a documented embedded-runtime
  technical-fit candidate cap before policy invocation, synthesize a
  non-selectable `candidate_set_overflow` diagnostic candidate when the cap is
  exceeded, and mirror the new diagnostic code through workflow-service and
  frontend workflow types. Allowed write set:
  `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
  `crates/pantograph-runtime-registry/src/technical_fit.rs`,
  `crates/pantograph-workflow-service/src/technical_fit.rs`,
  `src/services/workflow/types.ts`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because oversized
  candidate sets now fail candidate selection with a typed diagnostic before
  policy ranking. The slice does not truncate candidates, select a generic
  fallback runtime, add compatibility aliases, change generated files, alter
  lockfiles, update workflow fixtures, or touch runtime loading.
- Verification passed:
  `cargo test -p pantograph-embedded-runtime runtime_request_projection_rejects_candidate_set_overflow`,
  `cargo test -p pantograph-runtime-registry selector_surfaces_scoped_candidate_diagnostics_when_no_candidate_is_valid`,
  `cargo test -p pantograph-embedded-runtime technical_fit`, `cargo test -p
  pantograph-runtime-registry technical_fit`, `cargo test -p
  pantograph-workflow-service technical_fit`, `npm run typecheck`, and `cargo
  fmt --package pantograph-embedded-runtime --package pantograph-runtime-registry
  --package pantograph-workflow-service`; `git diff --check`.
- Remaining follow-up: executable candidate facts still need append-only
  trace/admission fields and diagnostics-ledger summary propagation.
- 2026-05-14 scheduler selected device id propagation slice: smallest useful
  vertical slice was to carry `selected_device_id` from the workflow
  technical-fit decision into the workflow-session scheduler reservation
  context and copy it into existing scheduler admission and reservation-changed
  diagnostics payload fields. Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because selected device id
  is copied only from canonical technical-fit decision facts. It does not infer
  devices from raw backend config, runtime strings, graph hints, legacy device
  options, or active backend state, and it does not change ledger schemas,
  TypeScript mirrors, generated files, lockfiles, or workflow fixtures.
- Verification passed:
  `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
  `cargo test -p pantograph-workflow-service session_execution`, and `cargo fmt
  --package pantograph-workflow-service`; `git diff --check`.
- Remaining follow-up: selected device class and append-only typed
  runtime-selection trace fields still need a cross-layer DTO slice before
  diagnostics-ledger runtime-selection history work.
- 2026-05-14 typed runtime-selection trace foundation slice: smallest useful
  vertical slice was to add append-only typed policy trace fields for
  `policy_phase`, `decision_code`, and `history_threshold_state`, project them
  from runtime-registry through embedded-runtime and workflow-service, persist
  them in diagnostics-ledger scheduler admission payload JSON, and mirror them
  in frontend workflow types. Allowed write set:
  `crates/pantograph-runtime-registry/src/lib.rs`,
  `crates/pantograph-runtime-registry/src/technical_fit.rs`,
  `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`,
  `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
  `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
  `crates/pantograph-workflow-service/src/lib.rs`,
  `crates/pantograph-workflow-service/src/technical_fit.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
  `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `crates/pantograph-diagnostics-ledger/src/lib.rs`,
  `crates/pantograph-diagnostics-ledger/src/tests.rs`,
  `src/services/workflow/types.ts`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because it only adds
  trace metadata for canonical runtime-selection decisions. It does not query
  ledger history, alter ranking, infer runtime/device choices from legacy
  state, change generated files, alter lockfiles, update workflow fixtures, or
  add compatibility shims.
- Verification passed:
  `cargo test -p pantograph-runtime-registry technical_fit_decision_normalizes_selected_identifiers`,
  `cargo test -p pantograph-embedded-runtime workflow_decision_projection_preserves_reason_codes`,
  `cargo test -p pantograph-workflow-service workflow_technical_fit_decision_normalizes_selected_backend`,
  `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted_payload_round_trips_policy_trace_contract`,
  `cargo test -p pantograph-runtime-registry technical_fit`, `cargo test -p
  pantograph-embedded-runtime technical_fit`, `cargo test -p
  pantograph-workflow-service technical_fit`, `cargo test -p
  pantograph-diagnostics-ledger scheduler_run_admitted`, `cargo test -p
  pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
  `npm run typecheck`, and `cargo fmt --package pantograph-runtime-registry
  --package pantograph-embedded-runtime --package pantograph-workflow-service
  --package pantograph-diagnostics-ledger`; `git diff --check`.
- Remaining follow-up: diagnostics-ledger runtime-selection history summaries
  remain before the five-run threshold ranking algorithm.
- 2026-05-14 scheduler selected device class propagation slice: smallest
  useful vertical slice was to add optional `selected_device_class` to
  scheduler admission and reservation-changed diagnostics payloads and copy it
  from the canonical workflow technical-fit decision through the existing
  workflow-session reservation context. Allowed write set:
  `crates/pantograph-diagnostics-ledger/src/event.rs`,
  `crates/pantograph-diagnostics-ledger/src/tests.rs`,
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
  and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because selected device
  class is copied only from canonical technical-fit decision facts and
  serialized as scheduler diagnostic attribution. It does not infer device
  class from raw device strings, backend config, runtime ids, graph hints,
  active backend state, or legacy frontend options.
- Verification passed:
  `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
  `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted_payload_round_trips_policy_trace_contract`,
  `cargo test -p pantograph-workflow-service session_execution`, `cargo test
  -p pantograph-diagnostics-ledger scheduler_run_admitted`, and `cargo fmt
  --package pantograph-workflow-service --package pantograph-diagnostics-ledger`;
  `git diff --check`.
- Remaining follow-up: diagnostics-ledger runtime-selection history summaries
  remain before the five-run threshold ranking algorithm.
- 2026-05-14 async-shell runtime-selection history gathering slice: smallest
  useful vertical slice was to expose a workflow-service runtime-selection
  history query wrapper and gather exact-key candidate history summaries in
  embedded-runtime after candidate synthesis, before invoking the pure
  runtime-selection policy. Allowed write set:
  `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
  `crates/pantograph-embedded-runtime/src/technical_fit.rs`, and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because the pure policy
  still has no diagnostics-ledger dependency, every queried history key matches
  workflow/task/model/backend/runtime-variant/device fields exactly, and
  configured-ledger query failures propagate as workflow-service errors.
  Candidates without all exact key fields remain facts-only; no broad-history
  substitution, compatibility alias, legacy runtime path, generated file,
  lockfile, frontend change, or workflow fixture change was added.
- Verification passed:
  `cargo test -p pantograph-embedded-runtime runtime_selection_history_summaries_project_exact_candidate_keys`,
  `cargo test -p pantograph-embedded-runtime technical_fit`,
  `cargo test -p pantograph-workflow-service technical_fit`,
  `cargo test -p pantograph-runtime-registry technical_fit`,
  `cargo test -p pantograph-diagnostics-ledger runtime_selection_history`,
  `cargo fmt --package pantograph-embedded-runtime --package
  pantograph-workflow-service -- --check`, and `git diff --check`.
- Remaining follow-up: keep history-backed ranking limited to currently
  populated status, execution-duration, and queue-wait evidence until
  canonical load, warmup, and memory producers exist.
- 2026-05-14 planned image-generation gateway/backend boundary slice: smallest
  useful vertical slice was to add an explicit planned image-generation
  gateway/backend method and stop raw `ImageGenerationRequest` gateway dispatch
  from reaching a backend. Allowed write set:
  `crates/inference/src/backend/mod.rs`,
  `crates/inference/src/backend/pytorch.rs`, `crates/inference/src/gateway.rs`,
  `crates/inference/src/gateway_tests.rs`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because a valid raw image
  request now fails closed with a typed config error requiring
  `ImageGenerationExecutionPlan`, while invalid raw requests still return typed
  request-validation diagnostics. The PyTorch backend only receives image
  generation through the planned helper, and the slice adds no request-only
  backend/runtime/device/package inference, `diffusers` backend alias,
  scheduler fallback, generated file, lockfile, frontend change, or workflow
  fixture change.
- Verification passed: `cargo test -p inference test_generate_image`, `cargo
  test -p inference --features backend-pytorch pytorch_image_generation`,
  `cargo check -p inference --features backend-pytorch`, `cargo fmt --package
  inference -- --check`, and `git diff --check`.
- Remaining follow-up: wire the workflow/inference execution async shell to
  gather Pumas facts, readiness, candidate, history, and scheduler decision
  facts, reduce them into `ImageGenerationExecutionPlan`, then call the planned
  gateway/backend path before artifact retention and compact output shaping are
  implemented.
- 2026-05-14 gateway planning-input execution slice: smallest useful vertical
  slice was to add a gateway method that accepts the side-effect-free
  image-generation planning input, invokes the Rust planner, and dispatches
  only the resulting `ImageGenerationExecutionPlan` to the active backend.
  Allowed write set: `crates/inference/src/gateway.rs`,
  `crates/inference/src/gateway_tests.rs`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because rejected planning
  returns typed `ImageGenerationPlannerDiagnostic` records through
  `GatewayError` and never dispatches raw image requests. The method does not
  infer Pumas facts, backend/runtime/device decisions, package family, or
  execution defaults from request fields or active backend state.
- Verification passed: `cargo test -p inference test_generate_image`, `cargo
  check -p inference`, `cargo fmt --package inference -- --check`, and `git
  diff --check`.
- Remaining follow-up: workflow/inference execution still needs the async shell
  that gathers request, Pumas facts, readiness, executable candidates, history
  summaries, and the scheduler decision before calling the planning-input
  gateway method.
- 2026-05-14 typed image execution boundary validation slice: smallest useful
  vertical slice was to update typed gateway tests that still expected
  request-only image generation to execute, proving `execute_typed` and
  `execute_typed_with_lifecycle` now fail closed at the planned execution
  boundary until a planning-input caller supplies Pumas facts and the scheduler
  decision. Allowed write set: `crates/inference/src/gateway_tests.rs` and
  this plan directory.
- The slice preserves the no-fallback/no-legacy rule because no raw typed-image
  compatibility path was restored. The lifecycle path records backend execution
  failure with the `ImageGenerationExecutionPlan` diagnostic and does not emit
  postprocessing or result-projection success phases.
- Verification passed: `cargo test -p inference
  test_execute_typed_image_generation_requires_planned_context`, `cargo test
  -p inference planned_boundary`, `cargo test -p inference gateway::tests::`,
  `cargo fmt --package inference -- --check`, and `git diff --check`.
- Remaining follow-up: workflow/inference execution still needs a successful
  planned-image path that calls `generate_image_from_planning_input` and then
  updates typed lifecycle coverage for the planned success case.
- 2026-05-14 inference README planned-boundary documentation slice: smallest
  useful vertical slice was to update the inference module README so public
  gateway examples and API notes no longer describe raw `generate_image()` as
  an executable image-generation path. Allowed write set:
  `crates/inference/src/README.md` and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because documentation now
  directs image-generation callers to `ImageGenerationPlanningInput` or
  `ImageGenerationExecutionPlan` and states that raw `generate_image()`
  validates request shape but does not dispatch to a backend.
- Verification passed: `git diff --check`.
- Remaining follow-up: workflow/inference execution still needs to build the
  planning input from request, Pumas facts, readiness, executable candidates,
  history summaries, and the scheduler decision.
- 2026-05-14 re-plan boundary before workflow execution-plan wiring: the next
  successful image-generation execution slice must connect the scheduler-owned
  `WorkflowTechnicalFitDecision` to node-engine image execution so
  `generate_image_from_planning_input` receives a reduced
  `BackendExecutionDecision`. Current node-engine image execution has the
  request and optional Pumas facts, but it does not own the scheduler decision.
- Planning needed: choose the ownership boundary for projecting
  `WorkflowTechnicalFitDecision` into inference's `BackendExecutionDecision`
  without storing scheduler facts in workflow graph nodes, pushing Pumas facts
  through worker envelopes, or fragmenting runtime-selection policy across
  node-engine and embedded-runtime.
- Standards constraint: this must be a narrow execution-plan integration owned
  at the workflow/embedded-runtime composition boundary. Planning/admission
  helpers stay synchronous unless the slice actually performs I/O or awaits
  already-owned runtime state. The inference planner and gateway stay side
  effect free below that boundary; node-engine must not invent
  backend/runtime/device decisions from request fields, active backend state,
  or graph hints.
- 2026-05-15 execution-plan architecture decision: Option 3 is now the target
  architecture. Scheduler/admission will produce a first-class per-run workflow
  execution plan containing per-node execution decisions. Node execution
  consumes that plan and must not recompute scheduling policy, infer runtime
  choices, or persist scheduler facts in graph inputs.
- Rationale: scheduler ranking, exploration, readiness, residency, warmup,
  memory-fit, retry, and queue policy are expected to change often. A run-level
  execution plan keeps those changes in scheduler/plan production instead of
  leaking policy into node-engine, inference gateway, graph schemas, frontend
  state, or worker envelopes.
- Staged implementation:
  1. Contract foundation: add a small workflow execution-plan DTO with
     schema/version, run/workflow identity, and per-node reduced execution
     decisions. The initial decision includes selected backend key, runtime
     id/variant id, device class/id, task id, selected model ref when available,
     and bounded diagnostics/trace ids. It must not include full Pumas facts,
     worker envelopes, raw graph node payloads, or mutable scheduler internals.
  2. Admission production: build the initial execution plan immediately after
     runtime preflight and scheduler admission from the existing
     `WorkflowTechnicalFitDecision`; store it as run-scoped execution context,
     not saved workflow content.
  3. Projection adapter: add a focused adapter from workflow execution-plan
     node decision to inference `BackendExecutionDecision`, with typed failures
     for missing/invalid selected identifiers.
  4. Node-engine consumption: thread the execution plan into node execution via
     typed runtime context such as `ExecutorExtensions`; image generation reads
     the current node decision, combines it with request and Pumas facts, and
     calls `generate_image_from_planning_input`.
  5. Lifecycle/diagnostics: attach execution-plan identifiers and selected
     decision facts to scheduler, runtime-load, inference lifecycle, and ledger
     records without duplicating large payloads.
  6. Recovery/future expansion: define whether retries reuse a still-valid plan
     or request a new scheduler plan, then extend append-only for multi-node
     placement, memory reservations, exploration cohorts, warmed-runtime
     affinity, historical summaries, and artifact-retention decisions.
- No-fallback/no-legacy confirmation: a runnable image-generation node without
  a selected per-node execution-plan decision must fail with typed diagnostics
  instead of using active backend state, raw graph hints, request model strings,
  implicit `diffusers` aliases, or CPU/runtime fallback.
- Async boundary decision: do not add async to Option 3 planning unless it is
  needed for actual I/O, existing async runtime-state APIs, or durable writes
  that cannot be performed synchronously at the owner boundary. The core
  scheduler/admission projection and inference planner remain synchronous
  deterministic functions over already-gathered facts. If a future slice needs
  Pumas, dependency, runtime-registry, ledger-history, or artifact-store I/O,
  that slice must keep awaits at the owning service boundary and pass reduced
  typed facts into the synchronous planning core.
- 2026-05-15 standards pass over Option 3 execution-plan update:
  reviewed `PLAN-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
  `TESTING-STANDARDS.md`, `RUST-API-STANDARDS.md`, `RUST-ASYNC-STANDARDS.md`,
  `SECURITY-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`, and
  `INTEROP-STANDARDS.md` against the staged execution-plan proposal.
- Findings and required plan constraints:
  1. Contract foundation must be append-only and correct-by-construction:
     schema/version fields, typed ids/enums, bounded diagnostic arrays,
     `#[non_exhaustive]` where future extension is likely, and `Result`
     returning constructors/projection helpers for validated decisions.
  2. Admission production must use a synchronous core helper fed by
     already-gathered facts. Async is allowed only at actual I/O or existing
     async runtime-state boundaries, and those awaits must not hold locks or
     introduce untracked `tokio::spawn`, polling loops, or unbounded queues.
  3. Projection from workflow execution-plan node decision to inference
     `BackendExecutionDecision` must parse/validate selected backend,
     runtime-variant, device, task, and model-ref fields and return typed
     diagnostics for missing or malformed facts.
  4. `ExecutorExtensions` may carry a typed runtime context only; execution
     plan data must not be serialized into graph input maps, saved workflow
     JSON, frontend DTOs, or worker envelopes.
  5. Diagnostics and ledger records must carry bounded identifiers, selected
     backend/runtime/device facts, policy trace ids, and planner codes only.
     They must not persist full Pumas facts, local filesystem paths, raw graph
     payloads, worker kwargs, image bytes, or unbounded diagnostics.
  6. Recovery/future expansion requires replay, duplicate-admission,
     cancellation, and retry/idempotency tests before an execution-plan record
     is treated as durable.
  7. README or ADR traceability is required in the same slice that introduces
     the workflow execution-plan contract or changes the workflow execution
     ownership boundary.
  8. The first cross-layer implementation slice needs a vertical acceptance
     test through the real node-engine/inference boundary, plus focused
     contract/adapter tests for risky branches.
- Standards conclusion: Option 3 remains compliant if implemented with these
  gates. Without them, the likely violations would be mutable/stringly typed
  cross-crate contracts, scheduler policy leaking into node-engine, unbounded
  diagnostics, undocumented boundary changes, and insufficient cross-layer
  acceptance coverage.
- 2026-05-15 blast-radius pass over Option 3 execution-plan update:
  reviewed workflow-service/session preflight, technical-fit DTOs,
  embedded-runtime session execution, node-engine executor extensions,
  inference gateway/planner, diagnostics ledger hooks, and crate dependency
  direction.
- Findings and required plan refinements:
  1. Workflow-service can own the run execution-plan DTO, but node-engine must
     not import workflow-service execution-plan contracts because
     workflow-service already depends on node-engine. Embedded-runtime must own
     the projection from workflow execution plan into a node-engine/inference
     runtime context.
  2. `WorkflowExecutionSessionPreflightCache` already carries
     `WorkflowTechnicalFitDecision`, required backends, and required models.
     Execution-plan production must derive from that admission evidence instead
     of re-querying scheduler/runtime facts or creating a second source of
     technical-fit truth.
  3. The current `WorkflowTechnicalFitDecision` is workflow-level. The first
     per-node execution-plan implementation may derive node decisions from it
     only when selected model/task facts map unambiguously to one runnable
     inference node. Ambiguous model-to-node mapping, missing selected
     model/task facts, or multiple runtime/model needs represented by one
     workflow-level decision must fail with typed diagnostics.
  4. Warm session executors reuse `ExecutorExtensions`. Every run must install
     a fresh run-scoped execution-plan context carrying the current
     workflow-run id, or explicitly clear/replace the previous context before
     execution. Missing or mismatched run id must fail closed so stale execution
     decisions cannot be reused across runs.
  5. Node-engine should consume only a minimal inference-facing lookup keyed by
     node id/task id and carrying reduced `BackendExecutionDecision` data. It
     may compose request, resolved package facts, and reduced decision, but it
     must not own scheduler ranking, workflow-service DTOs, retry policy, or
     full Pumas package propagation.
  6. Durable execution-plan persistence remains deferred until replay, retry,
     and idempotency semantics are specified and tested. Initial vertical
     slices should use run-scoped execution context unless a later slice adds
     durable recovery behavior.
- Blast-radius conclusion: Option 3 remains the most maintainable path if
  implemented with these refinements. They keep scheduler policy changeable,
  preserve simple graph ergonomics, avoid dependency inversion, prevent
  stale-plan reuse in warm sessions, and keep Pumas facts localized to existing
  model-resolution boundaries.
- 2026-05-15 standards iteration over blast-radius refinements:
  re-checked the updated Option 3 constraints against plan, architecture, Rust
  API, Rust async, concurrency, testing, security, documentation, and interop
  standards.
- Additional required gates found during the standards iteration:
  1. Public cross-crate constructors, projection helpers, and validation paths
     for execution-plan contracts must return specific typed errors or bounded
     typed diagnostics. Do not introduce `Result<T, String>` public APIs or
     string-matched control flow at the workflow-plan/inference projection
     boundary.
  2. A new execution-plan source module must include README or ADR traceability
     in the same slice. If a README is used, it must document the API consumer
     contract and structured producer contract: serde shape, schema/version
     behavior, append-only evolution, default semantics, bounded diagnostics,
     and persistence/replay compatibility.
  3. The first node-engine/inference vertical acceptance test must be written
     before the node-consumption implementation, fail for the expected missing
     planned execution path, then pass after the slice. Focused unit tests still
     cover constructors, projection failures, and stale run-context rejection.
  4. If durable execution-plan persistence is introduced in a later slice, the
     admission/plan write/active-run transition must be transactional or
     explicitly idempotent across cancellation points. Recovery support requires
     replay, duplicate-admission, cancellation, and retry tests before the
     record is treated as authoritative state.
  5. Interop and frontend blast radius stays intentionally closed for the first
     slices: no generated frontend DTOs, saved workflow fixture mutations,
     worker envelopes, or IPC payloads may carry execution-plan decisions. If a
     later slice exposes the plan across a process or language boundary, both
     sides and their serialization fixtures must change in the same slice.
- Standards iteration conclusion: no objective change is required. The plan is
  standards-compliant if implementation follows the added gates; otherwise the
  likely violations would be untyped public errors, undocumented structured
  contracts, acceptance tests added too late, cancellable partial persistence,
  or accidental interop/schema blast radius.
- 2026-05-15: Completed Milestone 6 Option 3 contract foundation slice. Added
  workflow-service `WorkflowExecutionPlan` DTOs in a focused
  `workflow/execution_plan.rs` module with schema versioning, validated
  workflow/run ids, per-node reduced selected backend/runtime/device/task facts,
  optional selected model ref, bounded diagnostics, policy trace ids, typed
  constructor/deserialization errors, and private fields. Updated
  workflow-service facade exports plus source/module/test READMEs. No scheduler
  admission, durable persistence, embedded-runtime projection, node-engine
  context, frontend DTO, worker envelope, saved workflow fixture, or inference
  execution behavior changed in this slice, preserving the no-fallback rule.
  Verification passed: `cargo test -p pantograph-workflow-service
  workflow::tests::contracts`; `cargo fmt -p pantograph-workflow-service`.
  Verification deviation fixed: the first focused test run exposed missing
  `WorkflowId`/`WorkflowRunId` imports in the new contract tests.
- 2026-05-15: Completed Milestone 6 Option 3 admission production slice. Added
  a sync workflow execution-plan admission helper that consumes cached
  technical-fit decisions plus workflow capability model summaries, attaches
  the resulting reduced plan to the active run context, and fails admitted runs
  with typed workflow capability diagnostics when selected candidate facts are
  missing or ambiguous. No saved workflow graph, durable plan persistence,
  frontend DTO, worker envelope, or node-engine execution behavior changed in
  this slice. Verification passed: focused admission contract tests, scheduler
  active-run plan storage test, session runtime preflight suite, session
  execution suite, `cargo check -p pantograph-workflow-service`, and
  `cargo fmt -p pantograph-workflow-service`. Follow-up recorded: if Pumas
  model/package facts can change without graph/runtime fingerprint changes,
  preflight cache invalidation needs a package-facts/update-cursor component
  before warm-session plan reuse becomes authoritative.
- 2026-05-15: Completed Milestone 6 Option 3 projection adapter slice. Added
  an embedded-runtime composition-boundary adapter that projects a workflow
  execution-plan node decision into inference `BackendExecutionDecision` with
  typed validation for backend ids, runtime variant ids, device ids/classes,
  task ids, selected model refs, diagnostics, and technical-fit policy trace
  ids. No node-engine runtime context or execution behavior was threaded in
  this slice. Verification passed: `cargo test -p
  pantograph-embedded-runtime workflow_execution_plan_projection`, `cargo
  check -p pantograph-embedded-runtime`, and `cargo fmt -p
  pantograph-embedded-runtime`. Deviation recorded: the crate-private adapter
  module is temporarily marked `#[allow(dead_code)]` until the node-engine
  consumption slice uses it.
- 2026-05-15: Completed the Node-engine consumption prerequisite query slice.
  Workflow-service now exposes a read-only active-run execution-plan lookup so
  embedded-runtime can retrieve the bounded plan for the current
  workflow-run/session pair without passing scheduler facts through graph
  inputs, saved workflow JSON, frontend DTOs, or worker envelopes. Verification
  passed: focused scheduler store test, `cargo check -p
  pantograph-workflow-service`, and `cargo fmt -p
  pantograph-workflow-service`.
- 2026-05-15: Completed the Node-engine planned inference context contract
  slice. Added node-engine-owned `PlannedInferenceDecisionContext` and the
  `PLANNED_INFERENCE_DECISIONS` executor-extension key so hosts can install
  reduced inference decisions by workflow run id and node id without importing
  workflow-service DTOs. The context fails closed for stale run ids, missing
  node decisions, and task mismatches. Verification passed: `cargo test -p
  node-engine --features inference-nodes planned_inference`, `cargo check -p
  node-engine --features inference-nodes`, `cargo check -p node-engine`, and
  `cargo fmt -p node-engine`.
- 2026-05-15: Completed the embedded-runtime planned context installation
  slice. Runtime session execution now fetches the active workflow execution
  plan, projects it into node-engine's planned inference context, clears any
  stale planned context from the reused executor, and installs the fresh
  context for the current workflow run only. Verification passed:
  embedded-runtime projection tests, node-engine extension removal test,
  `cargo check -p pantograph-embedded-runtime`, `cargo check -p node-engine
  --features inference-nodes`, and formatting for both crates.
- 2026-05-15: Completed the Node-engine planned image-generation consumption
  slice. Canonical `llm-inference` image-generation execution now requires a
  run-scoped planned inference decision for the current workflow run/node,
  requires resolved Pumas package facts, and calls
  `InferenceGateway::generate_image_from_planning_input`; missing planned
  context, stale run context, missing package facts, planner rejection, or
  backend failure terminates the node instead of using raw typed
  image-generation execution. Embedded-runtime session execution now passes the
  workflow run id into the core executor, runtime extension execution id, and
  inference lifecycle ledger sink so planned-context stale-run validation and
  request-id correlation use the same run id. No graph inputs, saved workflow
  fixtures, frontend DTOs, worker envelopes, lockfiles, or durable execution
  plan persistence changed. Verification passed: `cargo test -p node-engine
  --features inference-nodes test_canonical_llm_image_generation`, `cargo test
  -p node-engine --features inference-nodes
  core_executor::tests::inference_tests`, `cargo test -p node-engine
  --features inference-nodes planned_inference`, `cargo check -p node-engine
  --features inference-nodes`, `cargo check -p node-engine`, `cargo check -p
  pantograph-embedded-runtime`, and `cargo fmt -p node-engine -p
  pantograph-embedded-runtime`. Verification deviation: `cargo test -p
  pantograph-embedded-runtime
  scheduler_session_live_events_use_backend_workflow_run_id` failed before
  reaching the run-id handoff behavior because technical-fit rejects
  `candle.cpu` with "Candle executable model loading is not implemented"; this
  remains a pre-existing session fixture/runtime readiness issue to fix outside
  this slice. Remaining follow-up: add lifecycle/ledger diagnostics for
  selected execution-plan identifiers and planner failures without persisting
  full package facts or worker payloads.
- 2026-05-15: Completed the planned image-generation lifecycle/ledger
  diagnostics slice. `InferenceGateway` now exposes
  `generate_image_from_planning_input_with_lifecycle`, which emits bounded
  planned image task-validation and backend-execution lifecycle facts using
  the scheduler-selected backend id, runtime variant id, device class/id,
  stable model id, and artifact kind. Planner rejection maps typed
  image-generation planner diagnostic codes into lifecycle compatibility issue
  summaries, while preserving the no-fallback rule and keeping prompts,
  package fact payloads, local paths, worker kwargs, and image bytes out of
  events. Node-engine uses this planned lifecycle path when an inference
  lifecycle sink is installed; otherwise it continues to call the non-lifecycle
  planned gateway method. The existing embedded-runtime ledger adapter now has
  coverage proving selected planned runtime/device facts persist into bounded
  inference diagnostic payloads. No graph inputs, saved workflow fixtures,
  frontend DTOs, worker envelopes, lockfiles, scheduler ranking, runtime-load
  behavior, or durable execution-plan persistence changed. Verification
  passed: `cargo test -p inference generate_image_from_planning_input_with_lifecycle
  --lib`, `cargo test -p inference gateway::tests::test_generate_image`, `cargo
  test -p node-engine --features inference-nodes
  test_canonical_llm_image_generation_uses_planned_gateway_boundary`, `cargo
  test -p node-engine --features inference-nodes
  test_canonical_llm_image_generation`, `cargo test -p
  pantograph-embedded-runtime
  inference_diagnostic_event_adapter_persists_image_generation_bounded_lifecycle_summary`,
  `cargo test -p pantograph-embedded-runtime
  node_execution_ledger::tests::inference_diagnostic_event_adapter`, `cargo
  check -p inference`, `cargo check -p node-engine --features
  inference-nodes`, `cargo check -p pantograph-embedded-runtime`, and `cargo
  fmt -p inference -p node-engine -p pantograph-embedded-runtime`. Remaining
  follow-up: scheduler admission/runtime-load diagnostics still need selected
  execution-plan identifiers and policy trace ids without exposing full plans
  or package facts.
- 2026-05-15: Completed the scheduler admission/runtime-load execution-plan
  diagnostics slice. Diagnostics-ledger now owns a bounded
  `SchedulerExecutionPlanSummary` payload contract containing only schema
  version, node decision count, and policy trace ids. Workflow-service derives
  that summary from the scheduler-produced active execution plan after
  admission and attaches it to scheduler run-admitted plus runtime model
  lifecycle load/unload events. Session runtime cache events that do not have
  run-scoped plan context explicitly omit the field. This preserves the
  no-fallback/no-legacy rule: diagnostics are projected from the canonical
  plan, not reconstructed from graph inputs, legacy backend mappings, Pumas
  package payloads, node-engine state, or inference gateway policy. No frontend
  DTOs, saved workflow fixtures, worker envelopes, lockfiles, generated files,
  scheduler ranking logic, durable plan persistence, node-engine execution, or
  inference behavior changed. Verification passed: `cargo test -p
  pantograph-diagnostics-ledger
  scheduler_run_admitted_payload_round_trips_policy_trace_contract --lib`,
  `cargo test -p pantograph-diagnostics-ledger
  scheduler_run_admitted_rejects_invalid_execution_plan_summary --lib`, `cargo
  test -p pantograph-diagnostics-ledger
  scheduler_run_admitted_rejects_inconsistent_policy_trace_counts --lib`,
  `cargo test -p pantograph-diagnostics-ledger
  model_lifecycle_projects_canonical_error_link_without_counting_new_error
  --lib`, `cargo test -p pantograph-workflow-service
  workflow_execution_session_records_load_completed_only_with_runtime_proof
  --lib`, `cargo check -p pantograph-diagnostics-ledger`, `cargo check -p
  pantograph-workflow-service`, `cargo fmt -p pantograph-diagnostics-ledger -p
  pantograph-workflow-service`, and `git diff --check`. Remaining follow-up:
  recovery/retry policy still needs explicit planning before execution plans
  can become durable replay state.
- 2026-05-15: Completed the run-scoped execution-plan lifecycle guardrail
  slice. Added scheduler-store coverage proving `finish_run` clears the active
  execution plan and that neither the finished workflow run id nor the next
  admitted run can observe the prior plan before a new scheduler plan is
  produced. This locks in the current recovery boundary: execution plans are
  active-run scoped and retries/re-admissions must produce a fresh plan unless
  a future durable replay slice explicitly adds transactional/idempotent
  persistence, duplicate-admission handling, cancellation coverage, and
  diagnostic-backed reuse policy. No production behavior, durable storage,
  scheduler ranking, node-engine context, inference gateway, frontend DTOs,
  saved workflow fixtures, worker contracts, lockfiles, or generated files
  changed. Verification passed: `cargo test -p pantograph-workflow-service
  finish_run_clears_run_scoped_execution_plan_before_next_admission --lib`,
  `cargo test -p pantograph-workflow-service
  active_run_records_run_scoped_execution_plan --lib`, `cargo check -p
  pantograph-workflow-service`, `cargo fmt -p pantograph-workflow-service`,
  and `git diff --check`. Remaining follow-up: durable replay/retry semantics
  remain deferred behind a separate re-plan.
- 2026-05-15: Completed the compact image-generation output slice.
  Node-engine canonical image-generation execution now keeps the graph-visible
  `image` port as the single generated image body source and publishes compact
  per-image summaries in `results` without `data_base64`. This removes
  pre-conversion duplication of generated image bytes while preserving the
  planned gateway boundary and leaving workflow artifact conversion as the
  owner of retained media-body storage. No raw image-generation execution,
  backend/runtime/device inference, scheduler policy, worker envelope,
  frontend DTO, saved workflow fixture, lockfile, generated file, durable
  storage, or artifact conversion behavior changed. Verification passed:
  `cargo test -p node-engine --features inference-nodes
  test_canonical_llm_image_generation_uses_planned_gateway_boundary`, `cargo
  check -p node-engine --features inference-nodes`, `cargo fmt -p
  node-engine`, and `git diff --check`. Remaining follow-up: add an
  end-to-end retained image workflow-output test once the image-generation
  workflow fixture exists.
- 2026-05-15: Completed the planner checked-resource-estimate verification
  slice. Added focused inference planner coverage proving overflow-prone width,
  height, and image-count combinations return
  `ResourceEstimateOverflow` without allocation, worker execution, alternate
  backend selection, default inference, or wrapped byte estimates. Verification
  passed: `cargo test -p inference
  planner_rejects_resource_estimate_overflow_without_allocation --lib`, `cargo
  check -p inference`, `cargo fmt -p inference`, and `git diff --check`.
  Remaining follow-up: path validation, family-specific option rules, and
  generation default merge-order coverage still need separate planner slices
  before real worker execution is treated as complete.
- 2026-05-15: Completed the planner missing-component diagnostic slice. Added
  focused inference planner coverage proving a missing required Diffusers
  component role reports the exact absent field path
  (`package_facts.diffusers.components.vae`) while preserving the fail-closed
  planner path. No planner behavior, fallback behavior, worker execution,
  scheduler policy, Pumas facts, frontend DTOs, saved workflow fixtures,
  lockfiles, generated files, or backend contracts changed. Verification
  passed: `cargo test -p inference
  planner_reports_exact_missing_component_role_path --lib`, `cargo check -p
  inference`, `cargo fmt -p inference`, and `git diff --check`. Remaining
  follow-up: Pumas/model root validation needs a root-bearing planning or
  backend execution contract rather than ad hoc string inspection inside the
  side-effect-free planner.
- 2026-05-15: Completed the planner unsupported-family guardrail slice. Added
  focused coverage proving Flux family evidence is recognized as a single valid
  but unsupported family and returns `UnsupportedFamily` instead of falling
  back to generic Diffusers loading, Stable Diffusion assumptions, or worker
  execution. Verification passed: `cargo test -p inference
  planner_rejects_unsupported_single_family_without_generic_diffusers_fallback
  --lib`, `cargo check -p inference`, `cargo fmt -p inference`, and `git diff
  --check`. Remaining follow-up: executable support for FLUX, FLUX.2, Qwen
  Image, Lumina Image, GLM Image, Z-Image, and SDXL requires explicit family
  requirement tables, option-support rules, component ambiguity diagnostics,
  and fixtures.
- 2026-05-15: Completed the planner guidance-scale numeric guardrail slice.
  The image-generation planner now rejects non-finite `guidance_scale` values
  with the existing typed `InvalidNumericOption` diagnostic before any worker
  envelope is built. This preserves the no-fallback boundary by failing the
  plan instead of relying on Python worker behavior, backend defaults, alternate
  runtime selection, or request rewriting. Verification passed: `cargo test -p
  inference planner_rejects_non_finite_guidance_scale --lib`, `cargo check -p
  inference`, `cargo fmt -p inference`, and `git diff --check`. Remaining
  follow-up: family-specific option-support tables still need to classify
  guidance scale, negative prompt, image count, denoising scheduler, dtype, and
  dimensions as accepted, ignored, or rejected per family.
- 2026-05-15 plan update after selection/port-option blast-radius review:
  inspected `PortOptionsProvider`, `selection-input`, `expand-settings`,
  graph canonicalization, Pumas model-option caching, node-engine image request
  construction, inference image DTOs, and the Python worker image path. The
  review found that backend port options are the correct owner for
  fact-dependent denoising scheduler choices, but the current query DTO only
  carries search/pagination and cannot express selected model/package/runtime
  context. Milestone 6 now requires an append-only typed port-option context
  before implementing `llm-inference.denoising_scheduler` options.
- Plan refinements from that review: provider-backed `selection-input` must
  display unset/stale choices without auto-writing defaults into graph data;
  provider option caches must be keyed by node type, port id, selected
  model/package-facts cursor, and backend/runtime context; `expand-settings`
  remains the long-tail knob surface and must not become the canonical path for
  reproducibility-relevant image traits; the `scheduler` rename applies to
  Pantograph graph/API execution intent while factual Diffusers/Pumas component
  roles and paths may continue to use `scheduler`; and the Python worker must
  apply or reject a validated `denoising_scheduler` value rather than reporting
  metadata for a value it ignored.
- Standards conclusion for the plan update: the added gates preserve backend-
  owned data, layered dependency direction, interop DTO synchronization,
  test-first vertical slices, and no-fallback/no-legacy behavior. They also
  prevent Pumas package facts, scheduler decisions, local paths, graph payloads,
  or worker envelopes from leaking through frontend state while keeping future
  selectable inference traits generic and maintainable.
- 2026-05-15 standards iteration over the denoising option plan update:
  re-checked the updated Milestone 6 gates against `PLAN-STANDARDS.md`,
  `ARCHITECTURE-PATTERNS.md`, `INTEROP-STANDARDS.md`,
  `FRONTEND-STANDARDS.md`, `TESTING-STANDARDS.md`,
  `CONCURRENCY-STANDARDS.md`, and Rust API standards. Additional compliance
  gaps were found and added to the milestone before implementation begins.
- Added standards gates from that iteration: the port-option context slice must
  update node-engine, Tauri, UniFFI, Rustler, frontend TypeScript mirrors, and
  README/API notes together; provider context and denoising scheduler option
  ids must be validated typed Rust values with structured errors/diagnostics;
  provider-backed selection UI must stay declarative, accessible, event-driven,
  and protected against stale async responses; and tests must cover binding
  contract preservation, accessibility/keyboard behavior, graph gesture
  interaction, stale response discard, and context-keyed cache invalidation.
- Standards conclusion for this iteration: with the added gates, implementing
  the plan should remain standards-compliant across architecture, interop,
  frontend ownership, testing, concurrency, and Rust API boundaries. Without
  these gates, likely violations would be cross-binding DTO drift, stringly
  typed option ids, stale frontend async writes, inaccessible embedded selects,
  and accidental frontend ownership of backend-derived executable choices.
- 2026-05-15: Completed the port-option context contract foundation slice.
  Smallest useful vertical slice: add append-only `PortOptionsQuery` context
  with validated stable reference ids, thread it through Tauri
  `query_port_options`, preserve UniFFI/Rustler JSON query parsing, mirror the
  context DTO in frontend TypeScript, and document that provider context must
  not carry full Pumas facts, local paths, scheduler decisions, graph payloads,
  or worker envelopes. Allowed write set:
  `crates/node-engine/src/port_options.rs`, `crates/node-engine/src/lib.rs`,
  `crates/node-engine/src/README.md`, `src-tauri/src/workflow/commands.rs`,
  `src-tauri/src/workflow/workflow_port_query_commands.rs`,
  `crates/pantograph-uniffi/**README.md`,
  `crates/pantograph-uniffi/src/runtime_tests.rs`,
  `crates/pantograph-rustler/src/README.md`,
  `src/services/workflow/types.ts`,
  `src/services/workflow/pumaModelOptionsCache.ts`,
  `src/services/workflow/README.md`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because it only adds
  validated query-context transport for future fact-aware providers. It does
  not add denoising scheduler options, infer runtime/model facts, select a
  backend, mutate graph defaults, change execution behavior, add fallback
  scheduler values, pass Pumas package facts through frontend state, or alter
  worker envelopes.
- Verification passed: `cargo test -p node-engine port_options`, `cargo test
  -p pantograph-uniffi
  direct_runtime_puma_lib_options_use_selector_access_from_pumas_api`, `cargo
  check -p pantograph_rustler`, `npm run typecheck`, `node
  --experimental-strip-types --test
  src/services/workflow/pumaModelOptionsCache.test.ts`, `cargo fmt -p
  node-engine -p pantograph-uniffi -p pantograph_rustler -p pantograph --
  --check`, and `git diff --check`.
- Verification deviation/discovered issue: `cargo check -p pantograph` still
  fails in pre-existing, unrelated Tauri code before this slice's command
  signature can be fully validated. The errors are the old `String` device
  startup values where `BackendStartupDeviceIntent` is now required in
  `src-tauri/src/llm/startup.rs`, plus missing `timing_diagnostics` and
  `warmup_timing_attempt_id` fields in
  `src-tauri/src/workflow/diagnostics/types.rs`. These are outside the
  port-option context write set and should be fixed in a separate compile-
  unblocking slice before relying on full Tauri crate verification.
- Remaining follow-up: implement the provider-backed `selection-input`
  behavior, context-keyed frontend option cache, and
  `llm-inference.denoising_scheduler` provider in later slices. Those slices
  must keep stable primitive option ids, discard stale async responses, and
  avoid writing planner defaults into graph data.
- 2026-05-15: Completed the generic provider-backed port-options cache slice.
  Smallest useful vertical slice: add a frontend service cache keyed by node
  type, port id, search/pagination request, and provider context so future
  model/runtime-dependent options cannot reuse stale rows across selected
  model, package-facts cursor, backend, or runtime changes. Allowed write set:
  `src/services/workflow/portOptionsCache.ts`,
  `src/services/workflow/portOptionsCache.test.ts`,
  `src/services/workflow/README.md`, `package.json`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because it adds only
  frontend query-result caching for backend-owned port options. It does not
  hardcode denoising scheduler values, synthesize options, write graph
  defaults, inspect Pumas facts, infer backend/runtime selection, or change
  node execution.
- Verification passed: `node --experimental-strip-types --test
  src/services/workflow/portOptionsCache.test.ts`, `npm run typecheck`, `npm
  run test:frontend`, and `git diff --check`.
- Remaining follow-up: provider-backed `selection-input` behavior still needs
  stale async response handling at the component/use-site boundary. The cache
  keys prevent stale context reuse, but UI owners must still discard late
  responses when the selected model/runtime context changes during a request.
- 2026-05-15: Completed the provider-backed selection-input display guardrail
  slice. Smallest useful vertical slice: add shared selection-input state
  helpers and update `SelectionInputNode.svelte` so existing static
  `allowed_values` ports keep their default-adoption behavior, while future
  provider-backed ports render unset or stale values without writing a default
  or first option into graph data. Allowed write set:
  `src/components/nodes/workflow/SelectionInputNode.svelte`,
  `src/components/nodes/workflow/selectionInputState.ts`,
  `src/components/nodes/workflow/selectionInputState.test.ts`,
  `src/components/nodes/workflow/README.md`,
  `src/services/workflow/types.ts`, `package.json`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because provider-backed
  selection inputs now remain display-only when no current valid value exists.
  It does not synthesize denoising scheduler defaults, mutate executable graph
  data for provider-backed ports, hardcode scheduler choices, infer runtime
  facts, or call backend providers.
- Verification passed: `node --experimental-strip-types --test
  src/components/nodes/workflow/selectionInputState.test.ts`, `npm run
  typecheck`, `npm run test:frontend`, and `git diff --check`.
- Remaining follow-up: provider-backed option loading still needs a concrete UI
  integration with backend `query_port_options` plus stale async response
  discard at the request owner. This slice only prevents silent graph mutation
  once a port definition marks an input as provider-backed.
- 2026-05-15: Completed the provider-backed selection-input async loading
  slice. Smallest useful vertical slice: wire `SelectionInputNode.svelte` to
  load provider-backed options through the generic `portOptionsCache`, build
  bounded backend query context from stable target-node references, and discard
  late async responses when model/runtime context changes or the effect is
  cleaned up. Allowed write set:
  `src/components/nodes/workflow/SelectionInputNode.svelte`,
  `src/components/nodes/workflow/selectionInputProviderOptions.ts`,
  `src/components/nodes/workflow/selectionInputProviderOptions.test.ts`,
  `src/components/nodes/workflow/README.md`, `package.json`, and this plan
  directory.
- The slice preserves the no-fallback/no-legacy rule because the frontend only
  requests backend-owned options and maps returned primitive option values into
  the select. It does not hardcode denoising scheduler choices, synthesize
  defaults, inspect full Pumas package facts, pass local paths through provider
  context, infer runtime/backend decisions, or write loaded provider options
  into graph data.
- Verification passed: `node --experimental-strip-types --test
  src/components/nodes/workflow/selectionInputProviderOptions.test.ts
  src/components/nodes/workflow/selectionInputState.test.ts`, `npm run
  typecheck`, `npm run test:frontend`, and `git diff --check`.
- Remaining follow-up: real `llm-inference.denoising_scheduler` provider
  metadata still needs to be exposed through backend node definitions. Once
  that exists, add the project-approved equivalent of mounted interaction
  coverage for accessible-name, native keyboard selection, and graph gesture
  containment without introducing a new browser test platform.
- 2026-05-15: Completed the canonical denoising scheduler graph-input slice.
  Smallest useful vertical slice: expose optional
  `llm-inference.denoising_scheduler` in workflow-node descriptors, project it
  as an image-generation options payload in canonical node contracts, and make
  node-engine image request construction read only `denoising_scheduler` /
  `denoisingScheduler` for graph/API sampling intent. Allowed write set:
  `crates/workflow-nodes/src/processing/inference.rs`,
  `crates/workflow-nodes/src/contracts.rs`,
  `crates/workflow-nodes/src/README.md`,
  `crates/node-engine/src/core_executor/inference_nodes.rs`,
  `crates/node-engine/src/core_executor/inference_tests.rs`,
  `crates/node-engine/src/core_executor/README.md`,
  `crates/node-engine/src/README.md`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because graph/API
  `scheduler` is no longer read as an image-generation sampling alias. Existing
  inference internals may still carry a field named `scheduler` until the later
  planner/worker DTO rename slice, but new graph request construction uses the
  canonical `denoising_scheduler` input only.
- Discovered issue fixed in-slice: node-engine was forwarding an empty image
  `extra_options` object, which the planner correctly treated as an explicit
  unsupported option. Empty image extra options now serialize as `null`; real
  extra options still fail closed through planner diagnostics.
- Verification passed: `cargo test -p workflow-nodes
  test_descriptor_has_canonical_inference_contract_ports --lib`, `cargo test
  -p workflow-nodes
  llm_inference_contract_exposes_inference_task_payload_metadata --lib`,
  `cargo test -p node-engine --features inference-nodes
  test_build_image_generation_execution_request_preserves_canonical_inputs
  --lib`, `cargo test -p node-engine --features inference-nodes
  test_build_image_generation_execution_request_ignores_noncanonical_scheduler_input
  --lib`, `cargo test -p node-engine --features inference-nodes
  test_canonical_llm_image_generation_uses_planned_gateway_boundary --lib`,
  `cargo check -p workflow-nodes`, `cargo check -p node-engine --features
  inference-nodes`, `cargo fmt -p workflow-nodes -p node-engine -- --check`,
  and `git diff --check`.
- Remaining follow-up: the larger rename task still needs inference planner
  DTOs, worker envelopes, Python worker inputs, diagnostics, metadata, and
  fixtures moved from internal `scheduler` names to `denoising_scheduler` where
  those names are Pantograph execution intent rather than factual Pumas
  component roles.
- 2026-05-15: Completed the backend node-definition option-provider metadata
  slice. Smallest useful vertical slice: project registered
  `PortOptionsProvider` ownership through `pantograph-node-contracts`,
  workflow-service graph definitions, and TypeScript mock definitions as
  append-only `options_provider` references. Allowed write set:
  `crates/pantograph-node-contracts/src/lib.rs`,
  `crates/pantograph-node-contracts/src/README.md`,
  `crates/workflow-nodes/src/contracts.rs`,
  `crates/workflow-nodes/src/README.md`,
  `crates/pantograph-workflow-service/src/graph/types.rs`,
  `crates/pantograph-workflow-service/src/graph/registry.rs`,
  `crates/pantograph-workflow-service/src/graph/canonicalization_inference.rs`,
  `src/services/workflow/mocks.ts`,
  `src/services/workflow/WorkflowService.commands.test.ts`,
  `src/services/workflow/README.md`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because provider metadata
  is derived from backend-registered provider ownership and only identifies the
  query target. It does not embed selectable values, hardcode denoising
  scheduler choices, pass Pumas package facts/local paths through frontend
  state, choose runtime/backend placement, synthesize defaults, or add legacy
  aliases.
- Verification passed: `cargo test -p workflow-nodes
  builtin_contracts_preserve_registered_port_options_provider_refs --features
  model-library --lib`, `cargo test -p pantograph-workflow-service
  node_definition_preserves_registered_port_options_provider_refs --lib`,
  `cargo test -p pantograph-workflow-service
  port_definition_round_trip_preserves_inference_payloads --lib`, `cargo test
  -p pantograph-workflow-service canonicalization --lib`, `cargo test -p
  workflow-nodes llm_inference_contract_exposes_inference_task_payload_metadata
  --features model-library --lib`, and
  `node --experimental-strip-types --test
  src/services/workflow/WorkflowService.commands.test.ts
  src/components/nodes/workflow/selectionInputState.test.ts
  src/components/nodes/workflow/selectionInputProviderOptions.test.ts`.
  Broader checks passed: `cargo check -p pantograph-node-contracts`, `cargo
  check -p workflow-nodes --features model-library`, `cargo check -p
  pantograph-workflow-service`, `npm run typecheck`, `npm run test:frontend`,
  `cargo fmt -p pantograph-node-contracts -p workflow-nodes -p
  pantograph-workflow-service -- --check`, and `git diff --check`.
- Remaining follow-up: add the actual
  `llm-inference.denoising_scheduler` backend provider once the option id
  validation and package/runtime fact source are wired. This slice only makes
  provider references visible to graph clients.
- 2026-05-15: Completed the denoising scheduler option-id planner boundary
  slice. Smallest useful vertical slice: add a validated
  `DenoisingSchedulerOptionId` Rust contract, parse explicit
  image-generation scheduler request values before producing an
  `ImageGenerationExecutionPlan`, serialize the planned field as
  `denoising_scheduler`, and project the typed id into the current PyTorch
  worker envelope boundary. Allowed write set:
  `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract.rs`,
  `crates/inference/src/backend/pytorch_image_generation_tests.rs`,
  `crates/inference/src/gateway_tests.rs`, `crates/inference/src/lib.rs`,
  `crates/inference/src/README.md`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because invalid explicit
  scheduler option ids reject planning through typed diagnostics. It does not
  invent or hardcode scheduler option rows, treat Diffusers class names as
  executable option ids, add provider defaults, select a backend/runtime,
  transport full Pumas facts through frontend state, or let worker execution
  recover from invalid planner input.
- Verification passed: `cargo test -p inference image_generation_planner
  --lib`, `cargo test -p inference --features backend-pytorch
  test_generate_image_envelope_from_plan_validates_worker_request --lib`,
  `cargo test -p inference --features backend-pytorch pytorch_worker_image
  --lib`, `cargo check -p inference`, `cargo fmt -p inference -- --check`,
  and `git diff --check`.
- Remaining follow-up: the real `llm-inference.denoising_scheduler` provider
  still needs a factual option source. Current pinned Pumas facts expose the
  package scheduler component/class, but not the full set of runtime-valid
  replacement scheduler option ids; implementing a plural valid-options
  provider without that contract would violate the no-hardcoded-option rule.
- 2026-05-15: Completed the denoising scheduler request/worker rename slice.
  Smallest useful vertical slice: complete the canonical request-field rename
  for image-generation denoising scheduler intent across Rust inference
  request DTOs, node-engine request construction, planner diagnostics, PyTorch
  image worker envelopes, Python image worker inputs, output metadata, and
  worker fixtures. Allowed write set: `crates/inference/src/types.rs`,
  `crates/inference/src/gateway.rs`, `crates/inference/src/gateway_tests.rs`,
  `crates/inference/src/backend/mod.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract_tests.rs`,
  `crates/inference/src/backend/pytorch_worker_image_python_tests.rs`,
  `crates/inference/src/backend/pytorch_image_generation_tests.rs`,
  `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`,
  `crates/inference/torch/worker.py`,
  `crates/inference/torch/worker_image_contract.py`,
  `crates/inference/tests/fixtures/pytorch_worker_contract/generate_image_request.json`,
  `crates/inference/tests/fixtures/pytorch_worker_contract/generate_image_response.json`,
  `crates/node-engine/src/core_executor/inference_nodes.rs`,
  `crates/node-engine/src/core_executor/inference_tests.rs`, affected
  READMEs, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because the old
  image-generation sampling field name is no longer accepted by Rust request
  DTOs or PyTorch image worker payloads, and node-engine still ignores
  graph/API `scheduler` as a compatibility alias. Factual Diffusers/Pumas
  component roles named `scheduler` remain package evidence, not executable
  sampling intent.
- Verification passed: `cargo test -p inference image_generation --lib`,
  `cargo test -p inference --features backend-pytorch pytorch_worker_image
  --lib`, `cargo test -p node-engine --features inference-nodes
  image_generation --lib`, `cargo check -p inference`, `cargo check -p
  node-engine --features inference-nodes`, `cargo fmt -p inference -p
  node-engine -- --check`, code search for old image-generation request/worker
  `scheduler` field consumers, and `git diff --check`.
- Follow-up at this point: the Python worker still needed to apply or reject
  explicit `denoising_scheduler` values instead of ignoring them. The next
  slice resolves this by rejecting unsupported explicit scheduler changes.
- 2026-05-15: Completed the denoising scheduler worker guardrail slice.
  Smallest useful vertical slice: make the PyTorch image worker reject
  explicit `denoising_scheduler` values until scheduler swapping is actually
  implemented, so worker success metadata cannot claim a scheduler value that
  the worker ignored. Allowed write set: `crates/inference/torch/worker.py`,
  `crates/inference/src/backend/pytorch_worker_image_contract_tests.rs`,
  `crates/inference/src/backend/pytorch_worker_image_python_tests.rs`,
  `crates/inference/src/backend/pytorch_image_generation_tests.rs`,
  `crates/inference/tests/fixtures/pytorch_worker_contract/generate_image_request.json`,
  `crates/inference/tests/fixtures/pytorch_worker_contract/generate_image_response.json`,
  affected inference READMEs, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because the worker does
  not fall back to the model default while reporting the explicit value as
  accepted. It returns the existing typed worker invalid-request envelope until
  a later slice can apply validated scheduler changes for supported
  families/runtimes.
- Verification passed: `cargo test -p inference --features backend-pytorch
  pytorch_worker_image --lib`, `cargo test -p inference --features
  backend-pytorch test_image_generation_result_from_worker_response_maps_images
  --lib`, `cargo check -p inference`, `cargo fmt -p inference -- --check`, and
  `git diff --check`.
- Follow-up at this point: planner/family option-support rules still needed to
  reject unsupported explicit `denoising_scheduler` values before worker
  dispatch when the selected family/runtime cannot apply them. The next slice
  resolves this for the current Stable Diffusion planner path.
- 2026-05-15: Completed the denoising scheduler planner option-support slice.
  Smallest useful vertical slice: reject explicit `denoising_scheduler` values
  in the side-effect-free image-generation planner until family/runtime support
  can actually apply scheduler changes. Allowed write set:
  `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract_tests.rs`,
  `crates/inference/src/README.md`, and this plan directory.
- The slice preserves the no-fallback/no-legacy rule because the planner still
  validates primitive option id shape, but valid explicit scheduler ids now
  fail with `UnsupportedOption` before worker dispatch instead of being sent to
  Python for backend-local recovery or silent default behavior.
- Verification passed: `cargo test -p inference image_generation_planner
  --lib`, `cargo test -p inference --features backend-pytorch
  pytorch_worker_image --lib`, `cargo check -p inference`, `cargo fmt -p
  inference -- --check`, and `git diff --check`.
- Remaining follow-up: broader family option-support tables still need to
  classify guidance scale, negative prompt, image count, dtype, dimensions, and
  future supported scheduler overrides per image family.
- 2026-05-15: Completed the image-generation family rules table slice.
  Smallest useful vertical slice: move Stable Diffusion image-generation
  required components and option-support policy out of the main planner and
  into focused table-owned Rust rules. Allowed write set:
  `crates/inference/src/image_generation_family_rules.rs`,
  `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`,
  `crates/inference/src/lib.rs`, `crates/inference/src/README.md`, and this
  plan directory.
- The slice preserves the no-fallback/no-legacy rule because unsupported image
  families still produce `UnsupportedFamily`, and unsupported request traits
  such as explicit `denoising_scheduler`, img2img/inpaint fields, and opaque
  `extra_options` still produce typed `UnsupportedOption` diagnostics before
  worker dispatch. The slice does not infer family from model names, add
  generic Diffusers loading, or hardcode denoising scheduler option values.
- Verification passed: `cargo test -p inference image_generation_planner
  --lib` and `cargo test -p inference image_generation_family_rules --lib`.
- Deviations/discovered issues: the main planner remains above the 500-line
  decomposition review trigger after this extraction. It is smaller and has
  less family policy mixed into it, but later slices should continue extracting
  focused request-default, diagnostics, and resource-estimate helpers when they
  touch those areas.
- Remaining follow-up: future SDXL, FLUX, FLUX.2, Qwen Image, Lumina Image,
  GLM Image, Z-Image, and dtype-specific rules still need explicit table rows
  and fixtures before those families become executable.
- 2026-05-15: Completed a milestone status reconciliation slice. Smallest
  useful vertical slice: reconcile already-implemented Milestone 6
  execution-boundary checklist items with the current codebase so remaining
  work is not obscured by stale unchecked tasks. Allowed write set: this plan
  directory only.
- The slice preserves the no-fallback/no-legacy rule because it changes only
  plan status, and the verified code paths still require planned
  image-generation context, scheduler-owned backend/device decisions, Pumas
  package facts, and typed worker envelopes before execution. It does not add
  compatibility shims or relax planner/worker diagnostics.
- Verified completed boundaries: `ImageGenerationExecutionPlan` carries
  `DeviceResolutionDecision`; `image_generation_planner` remains synchronous
  and side-effect free; `InferenceGateway::generate_image_from_planning_input`
  is the planned gateway boundary; workflow-service owns
  `WorkflowExecutionPlan`; embedded-runtime projects workflow node decisions
  to inference `BackendExecutionDecision`; node-engine consumes
  `PlannedInferenceDecisionContext`; PyTorch image worker translation lives in
  focused Rust/Python helper modules with contract-version and unknown-field
  checks.
- Verification passed before this update: `cargo test -p inference
  image_generation_planner --lib`, `cargo test -p inference
  image_generation_family_rules --lib`, `cargo check -p inference`, `cargo fmt
  -p inference -- --check`, and `git diff --check`.
- Remaining follow-up: the raw `InferenceGateway::generate_image` path still
  intentionally rejects unplanned image generation. Do not mark raw
  `PyTorchBackend::generate_image` complete unless the task is explicitly
  reworded to the planned `generate_image_from_plan` boundary or a future
  slice can provide package/runtime/device facts without fallback behavior.
- 2026-05-15: Completed a plan/codebase blast-radius review after the
  synchronous Option 3 execution-plan updates. Reviewed workflow-service
  execution-plan admission, embedded-runtime plan projection and session
  context installation, node-engine planned inference context and
  image-generation node execution, inference image-generation planner/family
  rules, runtime-registry selection policy boundaries, and dependency
  preflight mappings.
- Findings recorded in Milestone 6: selected Pumas model refs currently need a
  canonical owner-boundary normalization slice before the value is used for
  scheduler history or lifecycle diagnostics. The current admission helper can
  double-prefix already-prefixed `pumas://models/...` values if left as raw
  string composition. This must become a typed diagnostic, not a silent repair
  in node-engine or the worker.
- Additional refinement recorded in Milestone 6: selected backend/runtime/
  device/model facts should move toward workflow-owned validated constructors
  or newtypes while preserving the existing crate dependency direction. This
  keeps workflow-service independent from inference DTOs but prevents invalid
  selected facts from drifting through embedded-runtime into node-engine.
- Additional refinement recorded in Milestone 6: package/dependency evidence
  and execution backend keys need explicit type/name separation. `diffusers`
  remains factual package, dependency, and capability evidence for PyTorch
  eligibility, but it must not become a graph-visible or scheduler-selected
  execution backend key through shared string helpers.
- Maintainability findings: `node-engine` inference node execution,
  runtime-registry technical-fit, inference image-generation planner, and the
  workflow execution-plan DTO module are at or above the standards
  decomposition-review threshold. Later implementation slices must keep new
  scheduler policy, request shaping, model identity parsing, dependency
  normalization, and diagnostics helpers in focused modules instead of growing
  these broad files.
- No-fallback/no-legacy confirmation: these plan updates do not add runtime
  fallback, request-only image execution, graph-written scheduler facts,
  worker-envelope scheduler policy, or frontend execution decisions. They
  tighten the prerequisites for future slices so invalid model/backend/runtime
  facts fail as typed diagnostics before execution.
- Verification for this plan-only slice: read-only codebase inspection,
  `git status --short`, plan diff review, and `git diff --check`.
- 2026-05-15: Completed a standards iteration over the new execution-plan
  identity gates against the plan, architecture, Rust API, Rust async,
  concurrency, testing, documentation, interop, security, and generic coding
  standards.
- Standards refinement recorded in Milestone 6: selected model identity must
  be parsed once at the workflow execution-plan owner boundary into a
  validated workflow-owned model-ref type. Embedded-runtime may project that
  typed value into inference, but it must not re-prefix, repair, or reinterpret
  raw selected model strings. This aligns the plan with Rust
  correct-by-construction and security parse-once requirements.
- Standards refinement recorded in Milestone 6: any model-ref normalization
  module, public constructor, or execution-plan contract change must update
  the owning README or add an ADR in the same commit, including accepted raw
  selected-fact forms, rejected local path/URI forms, append-only evolution,
  and scheduler-history identity semantics.
- Standards refinement recorded in Milestone 6: focused model-ref tests must
  start as a failing public-boundary test and then pass after implementation.
  The test should cover raw ids, already-prefixed Pumas refs, malformed/local
  path values, and projection without coupling to private helper names.
- Standards refinement recorded in Milestone 6: graph runtime/backend/device
  hints are optional scheduler inputs only. They are not required graph fields,
  not execution decisions, and not fallback choices; scheduler/admission policy
  still owns candidate validation, warmed-runtime affinity, historical
  diagnostics, exploration, and memory-fit ranking.
- Standards conclusion: the plan remains standards-compliant after these
  refinements. The main risks to avoid during implementation are duplicate
  string parsing across crates, silent identity repair in projection or
  node-engine, undocumented contract changes, graph hints bypassing scheduler
  policy, and new logic added to files already past the decomposition-review
  threshold.
- Verification for this standards pass: `git status --short`, standards
  review, plan diff review, and `git diff --check`.
- 2026-05-15: Completed a codebase blast-radius review of the standards
  identity refinements. Reviewed workflow-service execution-plan DTO and
  admission projection, embedded-runtime workflow-plan projection,
  node-engine planned image execution, inference image-generation planning,
  runtime-registry selection policy, workflow capability extraction, and
  dependency preflight/package-hint normalization.
- Finding recorded in Milestone 6: planned image execution must verify that
  the scheduler-selected model ref in `BackendExecutionDecision` matches the
  resolved Pumas package facts model ref before returning an
  `ImageGenerationExecutionPlan`. The current planner builds the worker plan
  from package facts, so a mismatch would otherwise be hidden rather than
  reported as a scheduler/package identity diagnostic.
- Additional blast-radius finding: dependency preflight paths still convert
  `BackendHintLabel::Diffusers` into generic backend-key strings for
  dependency/package evidence. Future normalization work must split
  package/dependency labels from execution backend keys before those helpers
  can influence scheduler execution decisions.
- Additional blast-radius finding: workflow capability extraction already
  ignores legacy `runtime_hint`, but it recursively treats `backend_key` and
  `recommended_backend` as required backend evidence. Future graph-hint work
  must distinguish explicit graph-owned preferences from nested package/Pumas
  evidence so package facts do not become hard execution decisions.
- Additional blast-radius finding: runtime capability projection currently has
  enough shape for a scheduler-visible `diffusers` runtime/backend even though
  the executable image-generation backend is PyTorch. Future normalization work
  must retire pseudo-Diffusers runtime/backend selection paths unless a real
  executable Diffusers backend is registered; `diffusers` may remain only as
  display, dependency, package, or capability evidence in the PyTorch case.
- Additional blast-radius finding: PyTorch capability facts are the required
  gate for Diffusers image-generation eligibility. The evidence boundary must
  not infer PyTorch execution from Pumas Diffusers facts alone; PyTorch must
  explicitly advertise image-generation task/source/runtime support before the
  boundary emits a PyTorch executable candidate.
- Additional blast-radius finding: node-engine dependency-context forwarding
  should not become a backend selection path. It may carry model intent and
  host-installed planned inference decisions, but Pumas `recommended_backend`
  and dependency metadata must not be interpreted there as scheduler decisions.
- Maintainability finding: the affected files remain above the decomposition
  review threshold, including node-engine inference execution, dependency
  preflight, runtime-registry technical-fit/policy, inference planner, and the
  workflow execution-plan DTO. New identity and backend-normalization logic
  should land in focused modules with README/ADR updates when boundaries
  change.
- No-fallback/no-legacy confirmation: the new plan gate rejects mismatched
  scheduler/package model identity through typed diagnostics instead of
  repairing the model ref, using package facts as an implicit override, or
  falling back to request-only image execution.
- Verification for this plan-only update: `git status --short`, targeted
  codebase blast-radius inspection, plan diff review, and `git diff --check`.
- 2026-05-15: Completed a standards iteration over the planned image identity
  match gate against plan, Rust API, security, testing, documentation, async,
  architecture, and decomposition standards.
- Standards refinement recorded in Milestone 6: image-generation execution
  must fail when the scheduler-selected model ref is missing, not only when it
  differs from package facts. This prevents package facts from acting as an
  implicit model-selection fallback while preserving optional selected model
  refs only for task families whose scheduler/admission decision is explicitly
  not model-bound.
- Standards refinement recorded in Milestone 6: planned image identity tests
  must cover both missing selected model refs and selected/package mismatch,
  and both cases must return typed diagnostics with no worker dispatch.
- Standards conclusion: the latest identity-match plan remains compliant if
  implemented as a synchronous, typed validation step in the image planning or
  execution-plan projection boundary, with no graph-input repair, no package
  facts override, no node-engine scheduler policy, and no new logic added to
  oversized modules without a focused extraction.
- Verification for this standards pass: `git status --short`, standards
  review, plan diff review, and `git diff --check`.
- 2026-05-15: Updated the Milestone 6 execution-evidence normalization plan
  after checking whether the current text already answered the latest
  blast-radius findings. Existing plan text already covered the general rule:
  Diffusers remains package/dependency/capability evidence, PyTorch execution
  requires capability support, graph hints are scheduler inputs only, and the
  evidence boundary belongs in inference. The update adds the missing concrete
  implementation consequences: retire scheduler-visible pseudo-Diffusers
  runtime/backend paths unless a real executable Diffusers backend is
  registered, require explicit PyTorch image/Diffusers capability facts before
  emitting a PyTorch candidate, prevent workflow-service nested
  `recommended_backend` extraction from becoming hard runtime requirements, and
  keep node-engine dependency-context forwarding out of backend selection.
- No-fallback/no-legacy confirmation: these plan changes do not preserve
  `diffusers` as an executable alias for compatibility. They require typed
  diagnostics or no candidate when package facts, runtime capabilities, and
  graph constraints cannot produce a canonical executable backend decision.
- Verification for this plan-only update: `git status --short`, targeted plan
  duplicate-check, codebase blast-radius findings review, and `git diff
  --check`.

### Traceability Links

- Module README updated: N/A for Milestone 0 because no production module
  ownership changed.
- ADR added/updated: N/A unless implementation changes runtime backend
  ownership boundaries.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: pending.
