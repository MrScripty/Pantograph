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
  complete and pinned before Pantograph Milestone 5a consumes production model
  facts for scheduler dispatch, before Milestone 5b resolves runtime-host load
  targets, and before Milestone 6 begins real PyTorch/diffusers image
  execution.
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
- 2026-05-18: Designed the execution-resource observation replacement path.
  Resource telemetry is now planned as inference-owned typed DTOs plus
  platform-specific monitor modules gated behind thin `cfg()` files. Linux,
  macOS, Windows, unsupported-target, PyTorch worker, and managed-runtime
  producers must report normalized observations or typed unavailable states;
  scheduler, workflow-service, and node-engine must not parse terminal error
  text, read artifact cache policy as measured memory, or import OS-specific
  monitor code.
- 2026-05-18: Resource observation blast-radius review refined the plan.
  Telemetry must flow first through `InferenceRequestLifecycleEvent`, with a
  small inference lifecycle event builder/context added before resource fields
  are wired into `gateway.rs`. PyTorch worker telemetry belongs on the generic
  success/failure envelope, process RSS monitoring should use existing
  `sysinfo` plus `ProcessHandle::pid()` before direct platform APIs, legacy
  OOM string detection must be retired or confined to typed adapter-local
  translation, and runtime-registry candidate history must expose observed
  memory/OOM facts before scheduler history ranking consumes them.
- 2026-05-19: Resource observation re-plan direction updated after codebase
  design review. The next telemetry work should introduce an inference-owned
  execution telemetry scope/recorder at the gateway/backend boundary and use it
  as the canonical backend-to-lifecycle observation path. Initial behavior is
  terminal summary only: PyTorch worker observations and process-RSS facts are
  recorded into the scope, then the gateway drains and merges them into the
  existing terminal lifecycle event. Live observation streaming, participant
  identity for parallel runtimes/devices, and scheduler real-time feedback
  remain later explicit contract slices.
- 2026-05-20: Started the telemetry-scope implementation with a foundation
  slice in `inference`. The new `InferenceExecutionTelemetryScope` and
  cloneable recorder collect typed resource observations for terminal summary
  drain only. This slice intentionally does not wire gateway/backends yet, add
  live observation events, pass Pumas/workflow facts through telemetry, or
  preserve a parallel result-wrapper compatibility path.
- 2026-05-20: Continued telemetry-scope implementation by migrating gateway
  process-RSS lifecycle producers to record through the gateway-owned telemetry
  scope before terminal lifecycle emission. Planned image execution, generic
  typed non-streaming execution, and streaming lifecycle wrappers now use the
  same collector path for process-RSS observations; backend-native worker
  telemetry remains the next boundary slice.
- 2026-05-20: Continued telemetry-scope implementation by replacing the
  planned image backend trait boundary with a minimal
  `BackendExecutionContext` that carries the telemetry recorder. The
  PyTorch/Diffusers bridge now records worker success/failure resource
  observations into the gateway-owned scope, so backend-native CUDA/MPS facts
  and process-RSS facts merge through one terminal lifecycle path.
- 2026-05-20: Continued producer telemetry with PyTorch image OOM typing. The
  Python worker now converts adapter-local PyTorch/CUDA out-of-memory signals
  into typed `memory_failure_kind = out_of_memory` resource observations on
  worker error envelopes; terminal workflow code still consumes only typed
  observations and does not parse error text.
- 2026-05-20: Re-plan boundary reached for managed-runtime structured
  telemetry. The current plan names the producer source but does not yet
  define whether the next implementation should monitor managed child-process
  RSS, consume runtime-native telemetry APIs, or do both. The plan now requires
  a precise boundary before implementation: lifecycle owner, target PID/API
  source, typed unavailable states, and separation between `os_process_rss`
  and `managed_runtime_telemetry`.
- 2026-05-20: Managed-runtime telemetry planning decision recorded. Use the
  two-source model: managed child-process RSS is the first implementation
  slice and remains `os_process_rss`; runtime-native structured metrics are a
  later provider contract and remain `managed_runtime_telemetry`. Both merge
  through `InferenceExecutionTelemetryScope`, while scheduler and
  diagnostics-ledger consume only projected typed observations.
- 2026-05-20: Managed child-process RSS implementation slice completed for
  llama.cpp sidecars. Gateway execution telemetry now asks the active backend
  for a ready runtime process id, monitors that child with the existing
  `RuntimeResourceMonitor` when present, and otherwise monitors the Pantograph
  process for in-process backends. The slice intentionally did not add
  runtime-native telemetry, scheduler-side probing, log parsing, or simulated
  managed-runtime metrics. Focused verification passed:
  `cargo test -p inference active_process_id_reports_ready_sidecar_child_pid_only --lib`
  and
  `cargo test -p inference test_chat_completion_stream_with_lifecycle_monitors_active_runtime_process --lib`.
- 2026-05-20: Runtime-native telemetry provider contract slice completed.
  Backends can now expose a `RuntimeNativeTelemetryProvider` that finishes into
  a typed resource observation or no observation. Gateway terminal telemetry
  merges the provider output through `InferenceExecutionTelemetryScope`.
  llama.cpp sidecars currently report `managed_runtime_telemetry`
  availability as `missing_runtime_capability` for peak RAM/VRAM, while
  concrete metrics remain a later adapter-specific implementation.
  Verification passed:
  `cargo test -p inference test_chat_completion_stream_with_lifecycle_merges_runtime_native_telemetry --lib`
  and `cargo check -p inference`.
- 2026-05-20: Re-plan boundary found before legacy OOM string handling. The
  remaining cleanup spans sidecar readiness loops, backend startup errors,
  scheduler-facing diagnostics, and typed memory failure telemetry. Plan the
  replacement as a typed adapter-local startup/runtime failure contract before
  editing `inference::server`, `inference::embedding_runtime`, or
  `backend::llamacpp_support`; do not preserve fallback string parsing outside
  that boundary.
- 2026-05-20: Legacy OOM handling planning decision recorded. Use a shared
  llama.cpp sidecar event classifier now and keep the full sidecar startup
  state machine as the later endpoint. The classifier is the only allowed
  boundary for bounded llama.cpp output inspection and must immediately
  translate OOM into typed startup/runtime failure facts that can map to
  `InferenceMemoryFailureKind::OutOfMemory`; scheduler, gateway, diagnostics
  ledger, and generic process code must not parse logs or keep string fallback
  paths.
- 2026-05-20: Shared llama.cpp sidecar event classifier implementation
  completed. `llamacpp_sidecar_events` is now the only sidecar output
  classifier for main llama.cpp and dedicated embedding readiness loops.
  `backend::llamacpp_support` maps typed startup failures to `BackendError`
  without scanning strings, and OOM maps through
  `LlamaCppSidecarStartupError::OutOfMemory` /
  `InferenceMemoryFailureKind::OutOfMemory`. Verification passed:
  `cargo test -p inference llamacpp_sidecar_events --lib`,
  `cargo test -p inference start_sidecar_inference_cleans_process_and_pid_file_on_start_error --lib`,
  `cargo test -p inference embedding_runtime_wait_for_ready_uses_shared_oom_classifier --lib`,
  `cargo test -p inference map_sidecar_start_error --lib`, and
  `cargo check -p inference`.
- 2026-05-20: Continued memory-policy history work by projecting the already
  persisted diagnostics-ledger peak RAM, peak VRAM, and out-of-memory counts
  into runtime-registry candidate history summaries. This is a typed
  evidence-contract slice only: pure scheduler policy still does not query the
  ledger directly, memory/OOM weighting remains inactive, missing history is
  not broadened or fabricated, and no terminal/log text parsing is introduced.
  Verification passed:
  `cargo test -p pantograph-runtime-registry candidate_history_summary_preserves_memory_and_oom_evidence --lib`,
  `cargo test -p pantograph-runtime-registry technical_fit --lib`,
  `cargo test -p pantograph-embedded-runtime runtime_selection_history_summaries_project_exact_candidate_keys --lib`,
  `cargo test -p pantograph-embedded-runtime technical_fit --lib`,
  `cargo check -p pantograph-embedded-runtime`, and
  `cargo fmt --package pantograph-runtime-registry --package pantograph-embedded-runtime -- --check`.
- 2026-05-20: Closed Milestone 6's checked-arithmetic/resource-estimate
  blocker as complete. The current execution path now has typed checked
  resource estimates, typed admission/budget rejection, ledger memory/OOM
  persistence, producer telemetry, and exact-key candidate-history projection.
  Memory/OOM history weighting remains a later scheduler-policy objective,
  not a compatibility fallback and not a current image-generation execution
  blocker.
- 2026-05-20: Completed the Candle future-capability guardrail. Candle
  capability tests now explicitly prove image generation is not advertised,
  Candle runtime variants remain unavailable with typed diagnostics, and the
  backend README documents that upstream Candle diffusion examples do not make
  Candle selectable until Pantograph has executable Candle diffusion loading,
  typed Pumas component support, runtime readiness facts, and technical-fit
  tests. Existing embedded-runtime technical-fit coverage proves an explicit
  Candle image-generation request fails through structured diagnostics without
  fallback selection. Verification passed:
  `cargo test -p inference --features backend-candle test_capabilities --lib`
  and
  `cargo test -p pantograph-embedded-runtime candle_image_generation_override_rejects_backend_incompatibility_without_selection --lib`.
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
- 2026-05-17: Completed the retained image output artifact slice. Smallest
  useful vertical slice: embedded-runtime node I/O projection now treats a
  valid base64 string on the canonical `image` port as the generated image
  payload, decodes it into a retained `image/png` artifact body, and projects
  diagnostics metadata as an image artifact with PNG format metadata. Non-image
  string ports remain text artifacts, and stream artifactization reuses the
  same base64 decoder. Allowed write set was
  `crates/pantograph-embedded-runtime/src/media_base64.rs`,
  `crates/pantograph-embedded-runtime/src/node_io_artifacts.rs`,
  `crates/pantograph-embedded-runtime/src/task_executor/stream_artifacts.rs`,
  `crates/pantograph-embedded-runtime/src/lib.rs`, focused tests, and plan
  docs. This preserves the no-fallback/no-legacy rule because the slice is
  limited to post-execution output retention and does not infer or synthesize
  package facts, artifact load targets, runtime choices, worker inputs,
  backend decisions, saved workflow compatibility, or alternate execution
  behavior. Verification passed: `cargo test -p pantograph-embedded-runtime
  media_base64`, `cargo test -p pantograph-embedded-runtime
  node_io_artifacts`, `cargo test -p pantograph-embedded-runtime
  recorder_stream`, `cargo check -p pantograph-embedded-runtime`, `cargo fmt
  --manifest-path crates/pantograph-embedded-runtime/Cargo.toml`, and `git
  diff --check`. Remaining follow-up: add the true image-generation workflow
  fixture/smoke test once the Pumas selected-artifact fixture can resolve a
  load target without Pantograph fallback synthesis.
- 2026-05-17: Completed the stream artifact checked-arithmetic slice.
  Smallest useful vertical slice: replace remaining `saturating_add` range and
  implicit sequence math in embedded-runtime stream artifactization with
  checked arithmetic. Overflow now fails the media artifactization path before
  appending a chunk and returns the existing redacted failed media event rather
  than clamping byte ranges or reusing a saturated sequence value. Allowed
  write set was
  `crates/pantograph-embedded-runtime/src/task_executor/stream_artifacts.rs`
  and plan docs. This preserves the no-fallback/no-legacy rule because it only
  changes invalid artifact metadata progression from silent saturation to
  explicit failure; it does not alter scheduler policy, backend/runtime
  selection, worker envelopes, model facts, saved workflow fixtures, lockfiles,
  generated files, or media retention ownership. Verification passed: `cargo
  test -p pantograph-embedded-runtime stream_artifact_progress`, `cargo test
  -p pantograph-embedded-runtime
  stream_artifactizer_redacts_media_chunk_when_byte_range_overflows`, `cargo
  test -p pantograph-embedded-runtime recorder_stream`, `cargo check -p
  pantograph-embedded-runtime`, `cargo fmt --manifest-path
  crates/pantograph-embedded-runtime/Cargo.toml`, and `git diff --check`.
  Remaining follow-up: the broader checked-arithmetic milestone still needs a
  scheduler/runtime memory-estimate review before it can be marked complete.
- 2026-05-17: Completed the Milestone 6 checklist reconciliation slice after
  the load-target, dependency-readiness, planned PyTorch worker, and artifact
  retention implementation slices. Marked the canonical planned PyTorch image
  path complete through `PyTorchBackend::generate_image_from_plan` while
  explicitly keeping raw `generate_image` unsupported so unplanned image
  execution cannot bypass scheduler/Pumas/load-target proof. Marked typed
  component-role validation complete through Pumas Diffusers component facts
  and family-adapter diagnostics. Marked terminal planner/readiness
  diagnostics complete because planner/gateway validation failures now reject
  execution with typed diagnostics instead of trying alternate backends,
  default schedulers, CPU fallback, generic Diffusers loading, or alternate
  dependency environments. Marked dependency/runtime readiness complete through
  inference-owned PyTorch/Diffusers package declarations, embedded-runtime
  readiness provider/probe facts, scheduler selected-decision proof, and the
  planner missing/unavailable readiness gate. This is a plan-only
  reconciliation; no source, test, config, lockfile, generated file, workflow
  fixture, scheduler policy, runtime behavior, worker envelope, or saved
  workflow behavior changed. Verification re-run for the implemented behavior:
  `cargo test -p inference pytorch_image_generation --features
  backend-pytorch`, `cargo test -p inference generate_image_from_planning_input
  --features backend-pytorch`, `cargo test -p inference
  image_generation_family_adapters --features backend-pytorch`, and `git diff
  --check`. Remaining Milestone 6 follow-ups stay open for memory-estimate
  checked arithmetic review, bounded model smoke fixture, and additional
  family-shaped Pumas fact fixtures.
- 2026-05-17: Completed the unsupported image-family fact-shape coverage
  slice. Added table-driven planner tests that mock Pumas Diffusers family
  evidence for SDXL, FLUX.2, Qwen Image, Lumina Image, GLM Image, and Z-Image
  without loading large models. Each unsupported family now proves the planner
  returns typed `UnsupportedFamily` diagnostics at
  `package_facts.diffusers.family_evidence` and does not fall back to generic
  Diffusers loading, Stable Diffusion assumptions, alternate backends, worker
  execution, model-id/display-name matching, or fixture-specific path logic.
  Allowed write set was
  `crates/inference/src/image_generation_planner_tests.rs` and plan docs.
  Verification passed: `cargo test -p inference
  planner_rejects_unsupported_image_family_fact_shapes_without_generic_fallback
  --features backend-pytorch`, `cargo test -p inference
  planner_rejects_unsupported_single_family_without_generic_diffusers_fallback
  --features backend-pytorch`, `cargo check -p inference --features
  backend-pytorch`, `cargo fmt --manifest-path crates/inference/Cargo.toml`,
  and `git diff --check`. Remaining follow-up: executable support for those
  families still requires explicit family rules, component requirements,
  option-support tables, and runtime worker support in later slices.
- 2026-05-17: Completed the image-family reference documentation slice.
  Inspected local Transformers, ComfyUI, and InvokeAI reference repositories
  for naming and taxonomy evidence only. Findings recorded in
  `02-image-generation-family-planner.md`: Transformers remains the naming
  reference for `model_type`, `architectures`, `auto_map`,
  `trust_remote_code`, and generation-config facts; ComfyUI confirms SD/SDXL,
  FLUX, FLUX.2, Lumina Image, Qwen Image, and Z-Image as distinct families
  with family-specific detector and encoder/tokenizer evidence; InvokeAI
  confirms taxonomy/variant conventions for SD, SDXL, FLUX, FLUX.2 Klein, and
  Z-Image and reinforces that new families require separate taxonomy,
  validation, loading, denoise, scheduler, metadata, and test concerns. This
  documentation slice does not change source, tests, worker envelopes,
  scheduler policy, Pumas DTOs, generated files, lockfiles, saved workflow
  fixtures, or runtime behavior. It preserves the no-fallback/no-legacy rule by
  recording reference-derived knowledge only as requirements for Pumas package
  facts, Pantograph-owned Rust planner labels/variants/component roles,
  provider-backed scheduler option ids, and planner diagnostics. Verification
  performed: targeted `sed`/`rg` reads over
  `/media/jeremy/OrangeCream/Linux Software/repos/reference/frameworks-libraries/transformers`,
  `/media/jeremy/OrangeCream/Linux Software/repos/reference/ai-systems/ComfyUI`,
  `/media/jeremy/OrangeCream/Linux Software/repos/reference/ai-systems/InvokeAI`,
  plus `git diff --check`. Remaining follow-up: executable support for any new
  family still requires explicit Pantograph family rules and Pumas facts in
  later implementation slices.
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
  to inference `BackendExecutionDecision`; node-engine consumes only the
  planned inference host service; PyTorch image worker translation lives in
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
- 2026-05-16: Clarified graph runtime request semantics in Milestone 6 after
  scheduler-boundary review. An omitted inference-node runtime is not an
  implicit default and does not request PyTorch, Diffusers, or any other
  backend; it leaves runtime selection entirely to scheduler/admission policy.
  An explicit graph runtime request is a hard scheduler requirement that must
  be validated against executable candidates, package/capability facts,
  memory-fit policy, and lifecycle diagnostics. The scheduler must either use
  the requested runtime or fail candidate selection with a typed diagnostic and
  ledger evidence; package metadata such as Pumas `recommended_backend` must
  not be promoted into that explicit runtime request.
- No-fallback/no-legacy confirmation: this clarification keeps graph runtime
  intent as scheduler input only. It does not add runtime fallback behavior,
  package-fact overrides, node-engine runtime selection, or execution shortcuts
  outside scheduler/admission policy.
- 2026-05-16: Further clarified the single runtime-selection path. Inference
  nodes require an optional `runtime` input for workflows that need to express
  an explicit scheduler requirement. When omitted, scheduler/admission policy
  chooses the runtime. When present, the value is projected only into
  scheduler/admission input and must not be forwarded directly into inference
  request DTOs, node-engine execution context, gateway calls, or worker
  envelopes as the selected runtime. The inference crate receives runtime
  selection exclusively from the scheduler-produced execution decision.
- No-fallback/no-legacy confirmation: this update does not add a compatibility
  path where graph data or dependency metadata can select runtime execution
  outside scheduler/admission. It makes scheduler output the only source of
  truth for runtime selection.
- 2026-05-16: Updated the Milestone 6 implementation staging after a
  blast-radius review found stale graph `backend_key = pytorch` language and
  an unclear write set for the new inference-node runtime input. The plan now
  uses explicit graph `runtime = pytorch` request terminology, requires
  workflow capability extraction to read only that graph-owned runtime input as
  a hard scheduler requirement, and adds a dedicated graph runtime
  input/projection slice before embedded-runtime technical-fit migration.
- No-fallback/no-legacy confirmation: this update rejects preserving
  `backend_key` as a graph-visible runtime-selection compatibility path.
  Pumas `recommended_backend`, dependency metadata, node-engine forwarded
  context, inference DTOs, gateway calls, and worker envelopes must still not
  carry selected runtime values except through scheduler-produced execution
  decisions.
- 2026-05-16: Completed a standards pass over the latest graph runtime
  projection and execution-evidence staging against the Coding Standards. The
  plan now explicitly requires README/ADR traceability for new evidence,
  graph-runtime, workflow capability, embedded-runtime, and scheduler-policy
  contract changes; keeps the evidence boundary synchronous unless real I/O is
  introduced; adds a test-first vertical acceptance path for graph runtime
  projection; and requires explicit runtime requests to constrain canonical
  scheduler ranking instead of using a separate candidate-id override picker.
- No-fallback/no-legacy confirmation: these standards refinements do not add
  compatibility shims. They require typed graph runtime intent, one
  scheduler-owned projection path, canonical scheduler ranking inside explicit
  runtime constraints, and typed diagnostics when package/capability/runtime
  facts cannot produce a valid selected decision.
- 2026-05-16: Completed the first Milestone 6 execution-evidence
  implementation slice. `crates/inference/src/execution_evidence.rs` now owns
  synchronous package/backend/runtime/graph evidence normalization with typed
  records, candidate evidence, graph runtime requirements, and bounded
  diagnostics. PyTorch static capabilities now explicitly advertise
  image-generation Diffusers bundle support and runtime variants before the
  evidence boundary can emit a PyTorch executable candidate.
- No-fallback/no-legacy confirmation: the slice does not treat `diffusers` as
  an executable backend alias. An explicit graph `runtime = diffusers` request
  produces no PyTorch candidate and returns a typed unsatisfied-runtime
  diagnostic unless a real executable Diffusers backend exists in the backend
  list. The evidence boundary does not rank candidates or pass graph runtime
  data to inference execution.
- Verification passed: `cargo test -p inference execution_evidence --lib`,
  `cargo test -p inference --features backend-pytorch test_capabilities --lib`,
  `cargo check -p inference`, `cargo fmt -p inference`,
  `cargo fmt -p inference -- --check`, and `git diff --check`.
- Remaining follow-up: embedded-runtime technical-fit, dependency preflight,
  workflow/runtime preflight, and gateway diagnostics still need migration to
  consume this evidence boundary before pseudo-Diffusers execution mappings can
  be removed from those layers.
- 2026-05-16: Completed the graph runtime input/projection implementation
  slice. Canonical `llm-inference` now exposes optional `runtime` graph intent
  instead of a graph-visible `backend_key` inference input, workflow-node
  contract payload metadata follows that port, and workflow capability
  extraction reads only explicit inference-node `runtime` values as hard
  scheduler requirements. Pumas `recommended_backend`, dependency metadata, and
  stale inference-node `backend_key` fields no longer become required
  executable backends for canonical inference nodes. A saved-workflow
  capability test now proves the real workflow capability path projects
  explicit `runtime = PyTorch` as `pytorch` while ignoring package metadata and
  inference-node backend metadata without a runtime input.
- No-fallback/no-legacy confirmation: this slice rejects preserving
  `backend_key` as a canonical inference-node runtime-selection compatibility
  path. It does not send graph runtime values directly into node-engine
  execution, worker envelopes, gateway calls, or inference DTOs; scheduler
  output remains the only selected-runtime source for execution.
- Verification passed: `cargo test -p workflow-nodes
  test_descriptor_has_canonical_inference_contract_ports --lib`, `cargo test
  -p pantograph-workflow-service capabilities --lib`, `cargo test -p
  pantograph-workflow-service
  default_capabilities_project_inference_runtime_as_scheduler_requirement
  --lib`, `cargo test -p workflow-nodes --features model-library
  builtin_contracts_preserve_registered_port_options_provider_refs --lib`,
  `cargo check -p workflow-nodes`, `cargo check -p
  pantograph-workflow-service`, `cargo fmt -p workflow-nodes -p
  pantograph-workflow-service -- --check`, and `git diff --check`.
- Discovered issue: `cargo test -p workflow-nodes --lib` still fails because
  `builtin_contracts_preserve_registered_port_options_provider_refs` expects
  the `puma-lib.model_path` options provider when the `model-library` feature
  is not enabled. The provider-specific test passes with `--features
  model-library`; deciding whether to gate, split, or re-scope that full-suite
  test is deferred outside this slice.
- Remaining follow-up: scheduler/runtime-registry selection still needs to
  enforce explicit runtime constraints against canonical executable candidates
  and typed diagnostics. Embedded-runtime technical-fit, dependency preflight,
  workflow/runtime preflight, and gateway diagnostics still need migration to
  consume the inference-owned execution-evidence boundary.
- 2026-05-16: Completed the pseudo-Diffusers sidecar capability retirement
  slice. Embedded-runtime Python sidecar runtime capabilities now advertise
  PyTorch, ONNX Runtime, and Stable Audio only; `diffusers` no longer appears
  as a selectable Python-sidecar runtime/backend for PyTorch image-generation
  execution. Runtime capability projection still allows a future real
  executable Diffusers backend to appear from actual backend capability facts.
- No-fallback/no-legacy confirmation: this slice removes the pseudo runtime
  instead of aliasing it to PyTorch. Diffusers remains dependency/model-source
  evidence until a real backend registers it as an executable backend key.
- Verification passed: `cargo test -p pantograph-embedded-runtime
  python_runtime_capabilities_report_python_backed_engines --lib`, `cargo test
  -p pantograph-embedded-runtime
  python_runtime_capabilities_keep_unavailable_reason --lib`, `cargo test -p
  pantograph-embedded-runtime
  host_runtime_capabilities_allow_real_diffusers_backend_registration --lib`,
  `cargo test -p pantograph-embedded-runtime runtime_capabilities --lib`,
  `cargo check -p pantograph-embedded-runtime`, and `cargo fmt -p
  pantograph-embedded-runtime -- --check`, and `git diff --check`.
- Remaining follow-up: embedded-runtime technical-fit candidate construction
  still needs to consume the inference-owned execution-evidence boundary.
- 2026-05-16: Completed the Diffusers package-hint execution mapping
  retirement slice. Embedded-runtime technical-fit no longer converts Pumas
  `BackendHintLabel::Diffusers` into a scheduler-visible executable backend
  candidate in the runtime-capability-only path. Without backend capability
  facts, Diffusers package hints remain evidence and produce no executable
  candidate.
- No-fallback/no-legacy confirmation: this slice fails closed instead of
  preserving a pseudo-Diffusers candidate or aliasing Diffusers to PyTorch.
  Real executable candidates still require the backend-capability checked path.
- Verification passed: `cargo test -p pantograph-embedded-runtime
  pumas_package_facts_runtime_capability_path_does_not_emit_diffusers_backend_candidate
  --lib`, `cargo test -p pantograph-embedded-runtime
  pumas_package_facts_candidates_use_backend_compatibility_reports --lib`,
  `cargo test -p pantograph-embedded-runtime
  candle_image_generation_override_rejects_backend_incompatibility_without_selection
  --lib`, `cargo test -p pantograph-embedded-runtime technical_fit --lib`,
  `cargo check -p pantograph-embedded-runtime`, and `cargo fmt -p
  pantograph-embedded-runtime -- --check`, and `git diff --check`.
- Remaining follow-up: the backend-capability checked technical-fit path still
  needs to consume the inference-owned execution-evidence report so accepted
  PyTorch/Diffusers candidates and typed diagnostics share the same boundary as
  inference execution evidence.
- 2026-05-16: Accepted Option 3 for the technical-fit replacement contract.
  The next implementation work must add an embedded-runtime
  `ExecutionEvidenceTechnicalFitAdapter` that consumes inference-owned
  `ExecutionEvidenceReport` values and workflow runtime capability facts, then
  emits runtime-registry `RuntimeTechnicalFitCandidate` values plus typed
  technical-fit diagnostics. Inference owns package/backend compatibility
  evidence; embedded-runtime owns runtime-capability projection; runtime
  registry owns scheduler ranking and final selection.
- Replacement/no-fallback rule: this adapter is not an additional path beside
  existing package-hint/backend compatibility candidate construction. The
  adapter must replace the old technical-fit builders for canonical inference
  technical-fit, and the old direct builders must be deleted or reduced to
  adapter-internal projection helpers when the adapter is wired. If evidence
  produces no valid executable candidate, technical-fit returns typed
  diagnostics and scheduler candidate selection fails; it must not recover by
  using package hints, pseudo-Diffusers aliases, node-engine context, default
  runtime choices, or the old compatibility loop.
- Planned adapter stages: first add failing adapter tests for PyTorch/Diffusers
  evidence, explicit `runtime = diffusers` rejection, omitted runtime
  scheduler freedom, and no fallback candidates; second add the synchronous
  adapter and diagnostic projection helpers without public wiring; third wire
  technical-fit to the adapter and remove the old builders; fourth run focused
  embedded-runtime technical-fit/runtime-capability tests, `cargo check -p
  pantograph-embedded-runtime`, formatting, and `git diff --check`.
- 2026-05-16: Added the diagnostics mapping-table requirement for the
  technical-fit evidence adapter. The adapter must explicitly translate each
  inference-owned `ExecutionEvidenceDiagnostic` kind into one scheduler-facing
  runtime-registry technical-fit diagnostic while preserving attribution for
  task, backend/runtime, model/package facts, and explicit graph runtime
  requirements. Unsupported task, backend unavailable, missing runtime
  capability, missing required package evidence, backend compatibility
  rejection, graph runtime requirement mismatch, and no accepted executable
  candidate must remain distinguishable so scheduler failure history can be
  used for future runtime policy, memory-fit, retry, and diagnostics analysis.
- No-fallback/no-legacy confirmation: this mapping table is a contract for
  diagnostic projection, not a recovery path. If evidence cannot produce a
  valid executable candidate, technical-fit must fail candidate selection with
  the mapped typed diagnostics instead of falling back to package hints,
  pseudo-Diffusers aliases, node-engine context, default runtime choices, or
  the old compatibility loop.
- 2026-05-16: Accepted the append-only technical-fit diagnostic contract option
  for the evidence adapter. The implementation should extend
  runtime-registry technical-fit diagnostics with typed evidence-oriented codes
  and structured attribution fields, then project those fields through
  embedded-runtime and workflow-service. This is preferred over reusing vague
  existing codes or hiding evidence meaning in message strings because the
  scheduler will use this history for future runtime policy, memory-fit,
  retry, and failure analysis. The contract should remain small, append-only,
  and easy to reason about: task id, backend/runtime keys, runtime variant id
  where available, model id/ref, package/evidence key, and explicit graph
  runtime request are the expected attribution fields.
- Maintainability decision: do not introduce a parallel diagnostic envelope
  unless the existing technical-fit diagnostic DTO cannot be safely evolved.
  A single structured diagnostic contract keeps runtime-registry,
  embedded-runtime, workflow-service, diagnostics ledger, and future scheduler
  policy aligned without duplicating projection logic.
- No-fallback/no-legacy confirmation: rejected evidence must produce typed
  diagnostics and no selectable fallback candidate. The adapter must not leave
  generic runtime-capability candidates eligible for model-bound canonical
  inference when package/backend execution evidence failed.
- 2026-05-16: Completed a standards iteration over the append-only diagnostic
  contract plan. The implementation must land the runtime-registry diagnostic
  DTO/code extension as a serial shared-contract slice before adapter wiring,
  then update embedded-runtime and workflow-service projections plus any
  exposed Tauri, UniFFI, Rustler, frontend, JSON fixture, or diagnostics-ledger
  mirrors in the same slice when they carry technical-fit diagnostics.
  Public enums/DTOs expected to grow should use serde-compatible append-only
  evolution and `#[non_exhaustive]` where appropriate, with explicit projection
  matches for every known variant.
- Standards guardrails: keep mapping/projection helpers in focused modules
  instead of growing oversized technical-fit files; update README/ADR
  traceability for runtime-registry, embedded-runtime, and workflow-service
  ownership changes; add serde/default/normalization tests, runtime-to-workflow
  projection tests, public DTO tests, and binding/fixture round-trip tests for
  any exposed interop surface. The tests must prove every new code and
  attribution field survives projection without message-string parsing.
- 2026-05-16 technical-fit diagnostic contract slice: smallest useful vertical
  slice was the shared append-only diagnostic DTO extension needed before the
  evidence adapter wiring. Allowed write set:
  `crates/pantograph-runtime-registry/src/technical_fit.rs`,
  `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`,
  `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
  `crates/pantograph-workflow-service/src/technical_fit.rs`,
  `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
  `src/services/workflow/types.ts`,
  `crates/pantograph-workflow-service/tests/fixtures/technical_fit_contract.json`,
  the three touched crate READMEs, and this plan directory.
- No-fallback/no-legacy confirmation: this slice only adds typed
  evidence-oriented diagnostic codes and structured attribution fields to the
  canonical technical-fit contract. It does not add selector fallback behavior,
  pseudo-Diffusers aliases, legacy compatibility shims, or worker dispatch
  paths. Unknown future runtime-registry diagnostic codes remain fail-closed in
  projection as `no_valid_candidate` until explicitly mapped.
- Verification passed: `cargo test -p pantograph-runtime-registry
  technical_fit --lib`, `cargo test -p pantograph-workflow-service
  technical_fit --lib`, `cargo test -p pantograph-embedded-runtime
  technical_fit --lib`, `cargo test -p pantograph-workflow-service
  workflow_technical_fit_cross_layer_fixture_deserializes --test contract`,
  `cargo check -p pantograph-runtime-registry`, `cargo check -p
  pantograph-workflow-service`, `cargo check -p pantograph-embedded-runtime`,
  and `npm run typecheck`.
- 2026-05-16 focused technical-fit diagnostics projection slice: smallest
  useful vertical slice was to extract embedded-runtime diagnostic code,
  severity, device-class, and attribution projection into
  `technical_fit_diagnostics.rs` before the evidence adapter mapping table
  grows. Allowed write set: `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
  `crates/pantograph-embedded-runtime/src/technical_fit_diagnostics.rs`,
  `crates/pantograph-embedded-runtime/src/README.md`, and this plan directory.
- No-fallback/no-legacy confirmation: this is a behavior-preserving ownership
  refactor. It does not add adapter wiring, legacy candidate builders,
  pseudo-Diffusers aliases, scheduler recovery paths, or worker dispatch.
- Verification passed: focused embedded-runtime technical-fit tests,
  `cargo check -p pantograph-embedded-runtime`, `cargo fmt -p
  pantograph-embedded-runtime -- --check`, and `git diff --check`.
- Remaining follow-up: implement the
  `ExecutionEvidenceTechnicalFitAdapter` mapping table in a separate focused
  module and then delete or reduce the old package-facts candidate builders
  when the adapter is wired.
- 2026-05-16 execution-evidence technical-fit adapter contract slice:
  smallest useful vertical slice was to add the internal
  `technical_fit_execution_evidence.rs` adapter without public wiring. The
  adapter consumes inference-owned `ExecutionEvidenceReport` values plus
  minimal task/model context and workflow runtime capability facts, then
  emits runtime-registry candidates and typed diagnostics. Allowed write set:
  `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
  `crates/pantograph-embedded-runtime/src/technical_fit_execution_evidence.rs`,
  `crates/pantograph-embedded-runtime/src/README.md`, and this plan directory.
- No-fallback/no-legacy confirmation: the adapter maps validated executable
  evidence to scheduler candidates and maps rejected evidence to typed
  diagnostics. It does not call the old package-hint builders, does not alias
  `runtime = diffusers` to PyTorch, and synthesizes an explicit
  `evidence_no_accepted_candidate` diagnostic when no candidate survives.
- Verification passed: `cargo test -p pantograph-embedded-runtime
  technical_fit_execution_evidence --lib`, `cargo test -p
  pantograph-embedded-runtime technical_fit --lib`, `cargo check -p
  pantograph-embedded-runtime`, `cargo fmt -p pantograph-embedded-runtime
  -- --check`, and `git diff --check`.
- Deviation/follow-up: the new module has a narrow staged `dead_code`
  allowance because this slice intentionally stops before public technical-fit
  wiring. Remove that allowance in the wiring slice, replace the current
  backend-package-facts candidate construction call site with the adapter, and
  delete or reduce the old direct candidate builders so they cannot remain as
  fallback behavior.
- 2026-05-16 execution-evidence technical-fit adapter wiring slice: smallest
  useful vertical slice was to replace the embedded-runtime
  backend-package-facts candidate construction call site with the
  `ExecutionEvidenceTechnicalFitAdapter`, remove the staged `dead_code`
  allowance, and delete the old direct package-hint/backend-compatibility
  candidate builders. Allowed write set:
  `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
  `crates/pantograph-embedded-runtime/src/technical_fit_execution_evidence.rs`,
  `crates/pantograph-embedded-runtime/src/README.md`, and this plan directory.
- No-fallback/no-legacy confirmation: canonical package-backed technical-fit
  now builds scheduler candidates from inference execution evidence only.
  Rejected evidence is represented as typed diagnostic candidates, generic
  runtime capability candidates are not left eligible as a fallback for
  model-bound package evidence, and `diffusers` remains dependency/package
  evidence unless a real executable backend registers it.
- Verification passed: `cargo test -p pantograph-embedded-runtime
  technical_fit --lib`, `cargo check -p pantograph-embedded-runtime`, `cargo
  fmt -p pantograph-embedded-runtime -- --check`, and `git diff --check`.
- Remaining follow-up: continue the Milestone 6 audit of dependency preflight,
  runtime capability, gateway, workflow runtime preflight, and node-engine
  dependency-context paths so none of them maintain a conflicting Diffusers or
  package-hint backend-selection rule.
- 2026-05-16 node-engine dependency-context backend-selection cleanup slice:
  smallest useful vertical slice was to remove package-facts backend-hint and
  `recommended_backend` interpretation from node-engine dependency preflight
  and dependency-context forwarding while preserving package facts as typed
  model/task evidence for inference requests and dependency requests. Allowed
  write set: `crates/node-engine/src/core_executor.rs`,
  `crates/node-engine/src/core_executor/dependency_preflight.rs`,
  `crates/node-engine/src/core_executor/inference_tests.rs`,
  `crates/node-engine/src/engine/dependency_inputs.rs`,
  `crates/node-engine/src/README.md`,
  `crates/node-engine/src/core_executor/README.md`, and this plan directory.
- No-fallback/no-legacy confirmation: node-engine no longer uses Pumas package
  `backend_hints` or `recommended_backend` to choose dependency preflight,
  task-validation diagnostics, or implicit graph context. Explicit graph
  `backend_key` remains the only backend signal accepted by this node-engine
  path until scheduler-owned execution decisions replace it.
- Verification passed: `cargo test -p node-engine --features
  inference-nodes,pytorch-nodes dependency_preflight --lib`, `cargo test -p
  node-engine --features inference-nodes,pytorch-nodes
  build_model_dependency_request --lib`, and `cargo test -p node-engine
  resolve_dependency_inputs --lib`.
- Broader verification discovered issue: `cargo test -p node-engine --features
  inference-nodes,pytorch-nodes --lib` currently fails
  `test_canonical_llm_image_generation_uses_planned_gateway_boundary` because
  that test still sends explicit `denoising_scheduler = euler` while the
  current image planner reports `unsupported_option` for explicit denoising
  scheduler changes. This is not caused by package-hint backend selection and
  should be handled in the planned denoising-scheduler option-support/gateway
  diagnostics reconciliation work rather than by restoring node-engine backend
  selection.
- Remaining follow-up: complete the broader audit of dependency preflight,
  runtime capability, gateway, workflow runtime preflight, and display
  diagnostics so explicit scheduler execution decisions become the only
  selected runtime/backend source end to end.
- 2026-05-16 planned image-generation gateway test-contract cleanup slice:
  smallest useful vertical slice was to align the node-engine planned
  image-generation gateway success test with the current image planner contract
  by removing the explicit `denoising_scheduler = euler` input from the success
  path. Explicit denoising scheduler changes remain covered as planner
  diagnostics until family/runtime support is implemented. Allowed write set:
  `crates/node-engine/src/core_executor/inference_tests.rs` and this plan
  directory.
- No-fallback/no-legacy confirmation: this slice does not make the planner
  accept unsupported scheduler changes, does not add a default scheduler, and
  does not bypass planner diagnostics. It keeps successful planned execution on
  currently supported canonical inputs only.
- Verification passed: `cargo test -p node-engine --features
  inference-nodes,pytorch-nodes test_canonical_llm_image_generation_uses_planned_gateway_boundary
  --lib` and `cargo test -p node-engine --features inference-nodes,pytorch-nodes
  --lib`.
- Remaining follow-up: implement backend-owned denoising scheduler port options
  and reconcile gateway-level image option diagnostics with planner diagnostics
  before accepting explicit scheduler changes in successful planned execution.
- 2026-05-16 inference compatibility package-vs-backend naming slice:
  smallest useful vertical slice was to rename the Diffusers image-generation
  compatibility test helper so it represents a PyTorch backend with Diffusers
  package support, and attribute compatibility checks to executable backend
  key `pytorch` while preserving `BackendHintLabel::Diffusers` as package
  evidence. Allowed write set:
  `crates/inference/src/backend/compatibility.rs` and this plan directory.
- No-fallback/no-legacy confirmation: this slice does not introduce a
  selectable Diffusers backend or alias Diffusers to PyTorch. It clarifies that
  PyTorch is the executable backend and Diffusers is package/source evidence.
- Verification passed: `cargo test -p inference
  diffusers_bundle_model_index_satisfies_image_generation_preprocessing --lib`
  and `cargo test -p inference backend::compatibility --lib`.
- Remaining follow-up: continue the broader package/dependency-key audit across
  runtime display, dependency diagnostics, and scheduler-selected backend facts.
- 2026-05-16 gateway image option-diagnostic reconciliation slice: smallest
  useful vertical slice was to align gateway-level image option diagnostics
  with the current planned image-generation planner contract. Allowed write
  set: `crates/inference/src/gateway.rs`,
  `crates/inference/src/gateway_tests.rs`, and this plan directory.
- No-fallback/no-legacy confirmation: known unsupported image-generation
  traits are now reported as unsupported diagnostics instead of honored/mapped
  gateway options. The slice does not restore raw image execution, accept
  explicit denoising scheduler changes, route opaque image `extra_options`
  around planner validation, add compatibility aliases, or change worker
  envelopes.
- Verification passed: `cargo test -p inference
  test_execute_typed_with_lifecycle_records_planned_boundary_failure --lib`,
  `cargo test -p inference
  test_generate_image_from_planning_input_with_lifecycle_records_unsupported_option_code --lib`,
  `cargo test -p inference gateway::tests --lib`, `cargo check -p
  inference`, `cargo fmt -p inference -- --check`, and `git diff --check`.
- Verification deviation: an attempted combined Cargo test command with two
  positional test filters failed because Cargo accepts only one test-name
  filter before `--`; the verification was rerun with the gateway module
  filter and passed.
- Remaining follow-up: support for denoising scheduler overrides,
  img2img/inpaint fields, or image-specific opaque options still needs typed
  family/runtime rules, provider rows where user-facing, and worker contract
  fields before those values can execute.
- 2026-05-16 planner component ambiguity guardrail slice: smallest useful
  vertical slice was to reject multiple present Pumas/Diffusers component
  sources for any required role in the selected supported image family.
  Allowed write set: `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`, and this plan
  directory.
- No-fallback/no-legacy confirmation: ambiguous required component facts now
  fail with the typed `ambiguous_component_role` planner diagnostic. The
  planner does not choose a component by order, string shape, model/display
  name, or generic Diffusers behavior, and no alternate backend/runtime or
  worker dispatch is attempted after ambiguity.
- Verification passed: `cargo test -p inference
  planner_rejects_ambiguous_component_role_sources_without_heuristic_selection
  --lib`, `cargo test -p inference image_generation_planner --lib`, `cargo
  check -p inference`, `cargo fmt -p inference -- --check`, and `git diff
  --check`.
- Remaining follow-up: FLUX, FLUX.2, Qwen Image, Lumina Image, GLM Image,
  Z-Image, and SDXL remain unsupported until explicit family component
  requirement rows and ambiguity fixtures are added for their package shapes.
- 2026-05-16 planner selected-task guardrail slice: smallest useful vertical
  slice was to require the scheduler-owned `BackendExecutionDecision` consumed
  by image planning to select `image_generation` explicitly before producing
  an image execution plan. Allowed write set:
  `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`, and this plan
  directory.
- No-fallback/no-legacy confirmation: mismatched or missing selected task facts
  now fail with typed `selected_task_mismatch` diagnostics. The planner does
  not repair scheduler decisions from request fields, package task evidence,
  active backend state, graph hints, or worker behavior.
- Verification passed: `cargo test -p inference
  planner_rejects_scheduler_decision_for_non_image_task --lib`, `cargo test
  -p inference image_generation_planner --lib`, `cargo check -p inference`,
  `cargo fmt -p inference -- --check`, and `git diff --check`.
- Remaining follow-up: dependency-environment readiness and path-root
  validation still need focused slices before real worker execution is
  considered complete.
- 2026-05-16 milestone checklist reconciliation slice: smallest useful
  vertical slice was to mark stale Milestone 6 checklist rows complete only
  where current code and tests already prove the behavior: the
  side-effect-free image-generation planner boundary and compact node-engine
  image-output shaping.
- Allowed write set: this plan directory only.
- No-fallback/no-legacy confirmation: this slice changes plan status only. It
  does not change executable code, restore raw image generation, add fallback
  backend/runtime selection, or weaken planner/node-engine diagnostics.
- Verification passed: `cargo test -p inference image_generation_planner
  --lib` passed in the immediately preceding planner slices, and this
  reconciliation reran `cargo test -p node-engine --features
  inference-nodes,pytorch-nodes
  test_canonical_llm_image_generation_uses_planned_gateway_boundary --lib`.
- Remaining follow-up: artifact-store retention still needs an end-to-end
  retained-output workflow fixture, and dependency readiness/full backend
  capability projection remain unresolved checklist rows.
- 2026-05-16 runtime evidence checklist reconciliation slice: smallest useful
  vertical slice was to mark runtime-selection checklist rows complete after
  verifying the inference execution-evidence boundary and embedded-runtime
  technical-fit adapter enforce graph runtime semantics. Allowed write set:
  this plan directory only.
- No-fallback/no-legacy confirmation: this slice changes plan status only. The
  verified code path still treats graph runtime as scheduler/admission input,
  not node-engine or worker execution input; `diffusers` remains package and
  dependency evidence and is not aliased to PyTorch for explicit graph runtime
  requests.
- Verification passed: `cargo test -p inference execution_evidence --lib`,
  `cargo test -p pantograph-embedded-runtime technical_fit_execution_evidence
  --lib`, and `cargo test -p pantograph-embedded-runtime technical_fit --lib`.
- Verification deviation: the first two Cargo tests were launched in parallel
  and one waited on Cargo's package/build lock. Both completed successfully,
  and later Cargo verification in this session was run serially.
- Remaining follow-up: the broader package/dependency-key audit remains open
  for dependency preflight, runtime display, gateway diagnostics, workflow
  runtime preflight, and any other path that may still carry conflicting
  `diffusers` display or dependency rules.
- 2026-05-16 re-plan boundary after runtime evidence reconciliation:
  remaining Milestone 6 rows now cross runtime identity, workflow capability
  extraction, diagnostics fixtures, backend-owned port-option facts, PyTorch
  worker readiness, path-root validation, artifact-store retention, and family
  adapter ownership. Do not begin the next code slice until this work is
  re-scoped.
- Audit findings: `pantograph-runtime-identity` still reserves `diffusers` as
  a canonical spelling/display label for a potential real backend; workflow
  capability extraction now uses only `llm-inference.runtime` as a hard
  inference runtime requirement but still scans generic `backend_key` values
  for non-`llm-inference` node families and GGUF evidence; embedded-runtime
  diagnostics/metrics fixtures still mention observed `diffusers` runtime ids;
  package/diffusers labels still appear correctly as evidence in inference
  compatibility and execution-evidence tests.
- Planning needed: decide the reserved-runtime-identity policy for
  `diffusers`, the future of generic non-inference `backend_key` extraction,
  whether diagnostics/metrics `diffusers` runtime fixtures are stale or future
  real-backend fixtures, the compact Pumas/runtime source for
  `llm-inference.denoising_scheduler` provider rows, the approved Pumas/model
  root contract for path validation, and the dependency-readiness owner for
  `diffusers`, `transformers`, `accelerate`, `torch`, and Pillow.
- No-fallback/no-legacy confirmation: do not hardcode denoising scheduler
  lists, preserve pseudo-Diffusers runtime candidates, restore recursive
  inference `backend_key` selection, accept local paths without a root
  contract, or let the PyTorch worker be the first dependency-readiness signal.
- Verification before this boundary: `cargo test -p inference
  execution_evidence --lib`, `cargo test -p pantograph-embedded-runtime
  technical_fit_execution_evidence --lib`, `cargo test -p
  pantograph-embedded-runtime technical_fit --lib`, and targeted code search
  for `diffusers`, `backend_key`, `recommended_backend`, and backend hints.
- 2026-05-16 re-plan decisions for inference traits, runtime identity, and
  readiness:
  - Use the local Transformers checkout at
    `/media/jeremy/OrangeCream/Linux Software/repos/reference/frameworks-libraries/transformers`
    as the naming/convention reference for model/task traits, package
    component names, and facts needed to use Transformers- and
    Diffusers-family models. Diffusers is not image-generation-only; it can
    describe image, text, audio, or future diffusion-model conventions.
  - Keep `diffusers` as a user-facing source/package/capability/future-runtime
    label, but make it scheduler-selectable only when a real executable
    runtime registers installed and available `diffusers` capability facts.
    Until then, represent it with typed unavailable/not implemented/not
    installed/etc. facts that the scheduler cannot select and graph editors can
    show disabled.
  - Replace generic recursive `backend_key` discovery with explicit typed
    runtime/trait inputs for each node family as those families move onto the
    canonical scheduler path. `runtime` and future traits such as `device`,
    `denoising_scheduler`, `dtype`, adapters, tokenizer/chat-template
    variants, attention backend, pooling strategy, and audio voice remain
    graph intent, then scheduler/admission reduces them into execution
    decisions.
  - Model unavailable-but-known runtimes/features explicitly with typed states:
    available, not installed, not implemented, unsupported platform, missing
    dependency, disabled by policy, missing model facts, requires runtime
    capability, and requires model capability. The scheduler treats these as
    non-selectable; providers may display them as disabled with reasons.
  - Runtime and capability diagnostics must be runtime/candidate/trait scoped.
    Scheduler-facing diagnostics identify the candidate runtime/backend and
    non-selectable reason; provider-facing capability facts identify the trait
    id, runtime/model scope, availability state, and disabled-display reason.
  - Pumas/model "roots" are approved storage bases such as the Pumas
    `shared-resources/models` tree. Worker execution must receive a typed
    Pumas model/artifact ref, a validated root-relative artifact path, or a
    resolved path that has already been checked against approved roots.
    Arbitrary graph/user/local paths, traversal, and unapproved temp/download
    paths fail before worker dispatch.
  - Missing `diffusers`, `transformers`, `accelerate`, `torch`, Pillow, or
    other runtime package dependencies must be reported by a readiness owner
    before worker dispatch. The next planning slice must choose whether that
    owner is embedded-runtime, inference backend capability facts, managed
    runtime, or a PyTorch bridge preflight shell.
- No-fallback/no-legacy confirmation: these decisions do not allow
  pseudo-Diffusers runtime candidates, hardcoded frontend scheduler lists,
  recursive inference `backend_key` selection, raw local-path execution,
  worker-side dependency discovery as readiness policy, or direct graph-to-
  inference runtime selection.
- 2026-05-16 diagnostics and standards tightening slice:
  - Smallest useful vertical slice: update Milestone 6 re-plan guidance after
    codebase review and standards pass so implementation cannot satisfy the
    remaining rows with metadata-only availability, parallel contract drift,
    hidden dependency-readiness paths, raw artifact paths, or unscoped
    diagnostics.
  - Allowed write set: this plan directory only.
  - No-fallback/no-legacy confirmation: this slice changes planning guidance
    only. It keeps capability facts as factual source data, scheduler/admission
    diagnostics as the decision trail, provider-facing disabled state as a
    projection of the same source facts, and rejects message-string parsing or
    lifecycle events as runtime/trait source facts.
  - Standards review: checked the update against the Coding Standards for
    correct-by-construction boundary values, append-only public contracts,
    single-owner path validation, dependency ownership, existing diagnostic
    channels, binding/mirror updates, README/ADR traceability, and verification
    proportional to blast radius.
  - Verification: docs-only diff review; no code tests were run.
  - Remaining follow-up: implement the ordered Milestone 6 slices for typed
    availability, disabled port options, dependency readiness ownership,
    validated Pumas artifact/root paths, reserved `diffusers` identity
    reconciliation, and typed runtime/trait input replacement for remaining
    generic `backend_key` discovery.
- 2026-05-16 shared capability availability contract slice:
  - Smallest useful vertical slice: add the shared inference-owned DTO and
    validation contract for runtime, runtime-variant, runtime-trait, package,
    dependency, and model-capability availability facts before any scheduler or
    provider projection uses those facts.
  - Allowed write set: `crates/inference/src/capability_availability.rs`,
    `crates/inference/src/lib.rs`, `crates/inference/src/README.md`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: the slice adds typed unavailable
    states and validated fact values only. It does not rank candidates, select
    runtimes, synthesize a fallback backend, create a pseudo-Diffusers runtime,
    add a diagnostic side channel, or encode disabled provider state in labels
    or metadata.
  - Verification passed: `cargo fmt --manifest-path crates/inference/Cargo.toml`,
    `cargo fmt --manifest-path crates/inference/Cargo.toml -- --check`,
    `cargo test -p inference capability_availability --lib`, and
    `git diff --check`.
  - Remaining follow-up: project these facts through existing scheduler
    diagnostics and port-option DTOs in later slices, then replace dependency
    readiness and provider disabled-state paths without creating parallel
    contracts.
- 2026-05-16 port-option disabled/unavailable contract slice:
  - Smallest useful vertical slice: extend the existing backend-owned
    port-options channel with append-only disabled/unavailable fields and carry
    them through the selection-input frontend normalization/rendering path.
  - Allowed write set: `crates/node-engine/src/port_options.rs`,
    `crates/node-engine/src/lib.rs`, `crates/node-engine/src/README.md`,
    existing `PortOption` literal call sites in `crates/workflow-nodes` and
    `src-tauri/src/workflow/puma_lib_commands.rs`, TypeScript workflow and
    svelte-graph port-option mirrors, selection-input state/provider/component
    files, focused selection-input tests, and this plan directory.
  - No-fallback/no-legacy confirmation: this slice does not add scheduler
    policy, hardcoded denoising scheduler lists, pseudo-runtime choices, or
    metadata/label-based disabled state. Disabled availability remains typed
    port-option data for later projection from capability facts.
  - Verification passed: `cargo fmt --manifest-path crates/node-engine/Cargo.toml`,
    `cargo fmt --manifest-path crates/node-engine/Cargo.toml -- --check`,
    `cargo test -p node-engine port_options --lib`, `cargo test -p
    workflow-nodes puma_lib --lib`, `node --experimental-strip-types --test
    src/components/nodes/workflow/selectionInputProviderOptions.test.ts
    src/components/nodes/workflow/selectionInputState.test.ts`, `npm run
    typecheck`, and `git diff --check`.
  - Verification deviation/discovered issue: the first verification attempted
    `cargo test --manifest-path src-tauri/Cargo.toml puma_lib_commands --lib`,
    but the Tauri package has no library target. The follow-up `cargo check
    --manifest-path src-tauri/Cargo.toml` reached the Tauri crate and failed on
    pre-existing unrelated errors in `src-tauri/src/llm/startup.rs`
    (`BackendConfig.device` now expects `BackendStartupDeviceIntent`, not
    `String`) and `src-tauri/src/workflow/diagnostics/types.rs`
    (`RuntimeLifecycleSnapshot` initializer missing timing fields). These are
    recorded as out-of-scope follow-ups and were not fixed in this slice.
  - Remaining follow-up: wire actual availability facts into provider rows in
    the backend-owned options providers, then add scheduler/admission tests
    proving unavailable rows are non-selectable without hiding the reason in
    metadata.
- 2026-05-16 dependency-readiness legacy-removal re-plan slice:
  - Smallest useful vertical slice: update Milestone 6 with the concrete
    replacement plan for canonical inference dependency readiness after the
    legacy boundary was identified.
  - Allowed write set: this plan directory only.
  - Decision: remove dependency-environment backend-key/default/hint fallback
    behavior from canonical inference rather than preserving it. Inference
    declares typed runtime/package requirements; embedded-runtime resolves
    local installed/readiness facts for now; scheduler/admission consumes
    reduced readiness facts; inference planner/gateway refuses non-ready
    scheduler decisions; workers receive only already-approved execution
    envelopes. A future managed-runtime resolver can replace the local
    embedded-runtime resolver without changing graph, scheduler, inference, or
    worker contracts.
  - Legacy removal map recorded: remove backend selection from
    `infer_backend_key`, package hints, `recommended_backend`,
    `runtime_engine_hints`, dependency requirements, and node-type defaults;
    remove `diffusers` as an executable backend mapping unless a real
    executable runtime registers ready facts; remove local Python fallback
    allowances for canonical inference; remove worker-side package discovery
    as the first readiness signal.
  - Staged implementation recorded: dependency-readiness DTO/projection,
    inference-owned PyTorch/Diffusers package requirement declarations,
    embedded-runtime readiness resolution, scheduler/admission filtering,
    planner/gateway rejection of non-ready decisions, legacy fallback removal,
    and retirement/restriction of `dependency-environment` from canonical
    inference.
  - No-fallback/no-legacy confirmation: the updated plan does not allow
    compatibility shims, pseudo-Diffusers executable runtimes, direct graph-to-
    inference runtime selection, dependency-environment fallback backend
    selection, or worker-first dependency readiness discovery.
  - Verification: docs-only diff review and `git diff --check`; no code tests
    were run.
  - Remaining follow-up: implement the staged replacement in thin slices,
    starting with the dependency-readiness DTO/projection and tests.
- 2026-05-16 dependency-readiness scheduler-proof tightening slice:
  - Smallest useful vertical slice: update Milestone 6 after codebase impact
    review so dependency readiness cannot be implemented as late diagnostics,
    lifecycle-only facts, worker-first package discovery, or a reuse of
    device diagnostics/runtime-requirement booleans.
  - Allowed write set: this plan directory only.
  - Decision: dependency readiness must attach to scheduler-facing candidate
    data and the selected execution decision as typed readiness proof. The
    scheduler/admission path consumes that proof before ranking/selection, and
    the inference planner/gateway validates the selected proof before worker
    dispatch without rerunning local package probes or choosing a runtime.
  - Standards review: this keeps the scheduler as the single runtime decision
    owner, preserves `inference::capability_availability` as factual source
    data, avoids overloading primitive ids with combined runtime/model/package
    scope, prevents `supports_runtime_requirements` and device diagnostics
    from becoming hidden dependency-readiness channels, and keeps Python
    sidecar availability separate from PyTorch/Diffusers package readiness.
  - No-fallback/no-legacy confirmation: the updated plan rejects
    diagnostic-only readiness, worker-first missing-package discovery,
    dependency-environment fallback selection, pseudo-Diffusers executable
    runtimes, and Python-executable-only readiness for canonical image
    generation.
  - Verification: docs-only diff review and `git diff --check`; no code tests
    were run.
  - Remaining follow-up: implement the staged dependency-readiness replacement
    by first adding the DTO/projection and then attaching readiness proof to
    scheduler candidates and selected decisions before filtering and planner
    rejection slices.
- 2026-05-16 gitignore sqlite sidecar guard-hygiene slice:
  - Smallest useful vertical slice: unblock guarded implementation slices by
    ignoring local sqlite sidecar files generated by Pantograph diagnostics
    stores while leaving dismissed proposal markdown untouched.
  - Allowed write set: `.gitignore` and this plan directory only.
  - No-fallback/no-legacy confirmation: this slice changes repository hygiene
    only. It does not alter graph, backend, runtime, device, technical-fit,
    frontend, worker execution, dependency readiness, or scheduler behavior.
  - Verification: `git status --short`, `git diff --check`, and ignored-file
    status review after the `.pantograph/*.sqlite-*` rule; no code tests were
    run because no source behavior changed.
  - Remaining follow-up: continue with the next Milestone 6 implementation
    slice after confirming only approved/dismissed untracked files remain.
- 2026-05-16 dependency-readiness DTO/projection slice:
  - Smallest useful vertical slice: add the inference-owned
    dependency-readiness DTO/projection and focused contract tests before
    scheduler filtering or embedded-runtime resolution consumes readiness
    facts.
  - Allowed write set: `crates/inference/src/capability_availability.rs`,
    `crates/inference/src/lib.rs`, `crates/inference/src/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log.
  - No-fallback/no-legacy confirmation: the slice adds typed scheduler proof
    contracts only. It does not rank candidates, select runtimes, probe local
    Python packages, infer backend keys, synthesize Diffusers as an executable
    runtime, add worker discovery, or preserve dependency-environment fallback
    behavior.
  - Implementation notes: added `DependencyReadinessFact`,
    `DependencyReadinessSubjectKind`, and `DependencyReadinessResolverOwner`
    to `inference::capability_availability`, with runtime/backend id,
    optional runtime variant, optional task, optional model family,
    package/dependency id, availability state, resolver owner, reason code,
    bounded reason text, and projection back to `CapabilityAvailabilityFact`.
  - Verification passed: `cargo fmt --manifest-path crates/inference/Cargo.toml`,
    `cargo fmt --manifest-path crates/inference/Cargo.toml -- --check`,
    `cargo test -p inference capability_availability --lib`, and
    `git diff --check`.
  - Remaining follow-up: add inference-owned PyTorch/Diffusers package
    requirement declarations as factual data, then resolve those declarations
    into readiness facts in embedded-runtime without adding scheduler policy.
- 2026-05-16 PyTorch/Diffusers package-requirement declaration slice:
  - Smallest useful vertical slice: declare the PyTorch/Diffusers
    image-generation package requirements inside inference as typed factual
    contract data before embedded-runtime resolves local readiness.
  - Allowed write set: `crates/inference/src/dependency_requirements.rs`,
    `crates/inference/src/lib.rs`, `crates/inference/src/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log.
  - No-fallback/no-legacy confirmation: the slice declares required packages
    only. It does not inspect installed packages, infer backend keys from Pumas
    hints, choose runtimes, filter candidates, synthesize a Diffusers
    executable runtime, dispatch workers, or preserve dependency-environment
    fallback behavior.
  - Implementation notes: added `DependencyRequirementDeclaration` and
    `DependencyRequirementNecessity` plus
    `pytorch_diffusers_image_generation_package_requirements()`, which declares
    required `diffusers`, `transformers`, `accelerate`, `torch`, and `pillow`
    packages scoped to `pytorch` and `image_generation`. The declaration can
    project an externally resolved state into `DependencyReadinessFact`.
  - Verification passed: `cargo fmt --manifest-path crates/inference/Cargo.toml`,
    `cargo fmt --manifest-path crates/inference/Cargo.toml -- --check`,
    `cargo test -p inference dependency_requirements --lib`, and
    `git diff --check`.
  - Remaining follow-up: implement embedded-runtime readiness resolution from
    these declarations into typed readiness facts and existing diagnostics.
- 2026-05-16 embedded-runtime dependency-readiness resolver slice:
  - Smallest useful vertical slice: add a pure embedded-runtime resolver that
    maps inference-owned dependency declarations plus host-observed Python
    package state into typed dependency-readiness facts.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/dependency_readiness.rs`,
    `crates/pantograph-embedded-runtime/src/lib.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log.
  - No-fallback/no-legacy confirmation: the slice resolves supplied facts
    only. It does not probe Python, install packages, infer backend keys,
    select runtimes, rank candidates, filter scheduler candidates, synthesize
    Diffusers as an executable runtime, dispatch workers, or preserve
    dependency-environment fallback behavior.
  - Implementation notes: added `PythonPackageReadinessSnapshot` and
    `resolve_python_package_readiness()`. The resolver emits
    `DependencyReadinessFact` values for available packages, missing packages,
    unavailable Python runtime state, and unsupported non-package declarations
    using typed availability states, stable reason codes, bounded reason text,
    and `EmbeddedRuntime` resolver ownership.
  - Verification passed: `cargo fmt --manifest-path
    crates/pantograph-embedded-runtime/Cargo.toml`,
    `cargo fmt --manifest-path crates/pantograph-embedded-runtime/Cargo.toml
    -- --check`, `cargo test -p pantograph-embedded-runtime
    dependency_readiness --lib`, and `git diff --check`. Initial test run
    failed on a test-helper iterator signature mismatch; the helper was fixed
    and the same focused test passed.
  - Remaining follow-up: attach dependency-readiness facts to
    runtime-registry/admission candidates and selected execution decisions as
    typed proof before scheduler filtering consumes them.
- 2026-05-16 runtime-registry dependency-readiness proof-carriage slice:
  - Smallest useful vertical slice: add typed dependency-readiness proof fields
    to runtime-registry technical-fit candidates and selected decisions without
    changing candidate eligibility or ranking yet.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`,
    affected embedded-runtime technical-fit candidate/decision constructors,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log.
  - No-fallback/no-legacy confirmation: readiness proof is now explicit
    scheduler-facing candidate data and selected-decision data. This slice
    does not treat readiness as `supports_runtime_requirements`, device
    diagnostics, lifecycle diagnostics, Python worker discovery, backend-key
    inference, or dependency-environment fallback behavior.
  - Implementation notes: added
    `RuntimeTechnicalFitDependencyReadinessFact` plus typed subject-kind,
    state, and resolver-owner enums to runtime-registry. Candidate
    normalization trims/canonicalizes proof scope, the selector copies proof
    from the selected candidate into `RuntimeTechnicalFitDecision`, and
    embedded-runtime constructors now initialize the new field explicitly.
  - Verification passed: `cargo test -p pantograph-runtime-registry
    technical_fit --lib` and `cargo test -p pantograph-embedded-runtime
    technical_fit --lib`.
  - Remaining follow-up: project real embedded-runtime
    `DependencyReadinessFact` values into runtime-registry candidates, carry
    selected proof through workflow/admission execution plans into inference,
    then make scheduler filtering consume readiness proof.
- 2026-05-16 execution-evidence dependency-readiness projection slice:
  - Smallest useful vertical slice: let the embedded-runtime
    execution-evidence adapter project supplied inference dependency-readiness
    facts onto matching runtime-registry candidates, without changing
    scheduler ranking or filtering.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/technical_fit_execution_evidence.rs`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`,
    `crates/pantograph-runtime-registry/src/lib.rs`, the Milestone 6 plan, and
    this execution log.
  - No-fallback/no-legacy confirmation: the adapter only projects supplied
    typed readiness facts. It does not infer dependencies from Pumas package
    hints, inspect local Python packages, rank candidates, filter candidates,
    synthesize a Diffusers executable backend, or call the legacy dependency
    environment.
  - Implementation notes: `ExecutionEvidenceTechnicalFitAdapterInput` now
    accepts `dependency_readiness_facts`. The adapter matches readiness facts
    by executable backend key, optional runtime variant, and task id, then
    emits runtime-registry readiness proof carrying dependency id, state,
    resolver owner, model-family scope, reason code, and reason text.
  - Verification passed: `cargo fmt --manifest-path
    crates/pantograph-runtime-registry/Cargo.toml -- --check`, `cargo fmt
    --manifest-path crates/pantograph-embedded-runtime/Cargo.toml -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit --lib`, and
    `cargo test -p pantograph-embedded-runtime technical_fit_execution_evidence
    --lib`.
  - Remaining follow-up: production technical-fit construction still passes an
    empty readiness fact slice. A later slice must supply host-resolved Python
    package readiness snapshots, carry selected proof through
    workflow/admission execution plans into inference, and then make
    scheduler filtering consume readiness proof.
- 2026-05-16 technical-fit request dependency-readiness input slice:
  - Smallest useful vertical slice: make backend-package-fact technical-fit
    request construction accept explicit dependency-readiness facts and pass
    them into the execution-evidence adapter.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log.
  - No-fallback/no-legacy confirmation: callers now have an explicit proof
    input for dependency readiness. The builder does not infer readiness from
    package hints, runtime display strings, Python sidecar presence, legacy
    dependency-environment checks, or worker package imports.
  - Implementation notes:
    `build_runtime_technical_fit_request_with_backend_package_facts` now
    accepts a `&[inference::DependencyReadinessFact]` slice and passes it
    through to the adapter. Existing production construction still supplies an
    explicit empty slice until the host package snapshot source is wired.
  - Verification passed: `cargo test -p pantograph-embedded-runtime
    technical_fit_request_projects_dependency_readiness_into_pumas_candidates
    --lib`.
  - Remaining follow-up: wire a host-resolved Python package snapshot source so
    production technical-fit calls pass real PyTorch/Diffusers package
    readiness facts, then carry selected proof through workflow/admission
    execution plans into inference.
- 2026-05-16 workflow execution-plan dependency-readiness propagation slice:
  - Smallest useful vertical slice: carry selected dependency-readiness proof
    from workflow technical-fit decisions through run execution-plan admission
    and into inference `BackendExecutionDecision` at the embedded-runtime
    composition boundary.
  - Allowed write set: `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/workflow/execution_plan.rs`,
    `crates/pantograph-workflow-service/src/workflow/execution_plan_admission.rs`,
    `crates/pantograph-workflow-service/src/README.md`, focused
    workflow-service tests, `crates/inference/src/device_contracts/planning.rs`,
    `crates/inference/src/README.md`, affected inference/node-engine test constructors,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `crates/pantograph-embedded-runtime/src/workflow_execution_plan_projection.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`, focused
    embedded-runtime tests, and this plan directory.
  - No-fallback/no-legacy confirmation: the slice only transports typed
    scheduler/admission proof. It does not infer readiness from graph inputs,
    package hints, diagnostics messages, worker imports, runtime display
    strings, or legacy dependency-environment checks.
  - Implementation notes: workflow-service now owns a dependency-readiness
    proof DTO on `WorkflowTechnicalFitDecision` and
    `WorkflowExecutionPlanNodeDecision`; admission copies selected proof into
    run-scoped node decisions. Inference `BackendExecutionDecision` now retains
    `DependencyReadinessFact` values, and embedded-runtime projection validates
    proof ids while converting workflow DTOs into inference contracts.
  - Verification passed: `cargo test -p pantograph-workflow-service
    workflow_execution_plan_admission_carries_dependency_readiness_proof
    --lib`; `cargo test -p pantograph-embedded-runtime
    workflow_node_decision_projects_dependency_readiness_proof --lib`; `cargo
    test -p inference image_generation_planner --lib`; `cargo test -p
    node-engine planned_inference --lib`; `cargo test -p
    pantograph-workflow-service workflow_execution_plan --lib`; `cargo test -p
    pantograph-embedded-runtime workflow_execution_plan_projection --lib`;
    `cargo test -p pantograph-workflow-service technical_fit --lib`; `cargo
    test -p pantograph-embedded-runtime technical_fit --lib`; `cargo test -p
    pantograph-embedded-runtime
    technical_fit_request_projects_dependency_readiness_into_pumas_candidates
    --lib`; cargo fmt checks for workflow-service, embedded-runtime,
    inference, and node-engine; `git diff --check`.
  - Remaining follow-up: production `workflow_technical_fit_decision` still
    needs host-resolved Python package readiness facts before scheduler
    filtering can reject unavailable PyTorch/Diffusers candidates without
    failing every candidate that currently has intentionally empty proof.
- 2026-05-16 runtime-registry dependency-readiness filtering slice:
  - Smallest useful vertical slice: make runtime technical-fit selection
    consume explicit unavailable dependency-readiness proof on candidates
    before ranking or explicit override selection.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: candidates with unavailable
    dependency-readiness proof are now ineligible and produce typed
    `evidence_required_package_unavailable` diagnostics. The selector does
    not recover by selecting another backend for an explicit override, parsing
    diagnostic messages, consulting worker imports, or treating package hints
    as readiness facts.
  - Implementation notes: automatic selection filters candidates whose
    dependency-readiness facts are not all ready. No-valid-candidate and
    explicit-override diagnostics now prefer dependency-readiness diagnostics
    over generic incompatibility when the rejected candidate carries negative
    proof. Empty proof remains non-blocking until production host package
    snapshots are wired.
  - Verification passed: `cargo test -p pantograph-runtime-registry
    dependency_readiness --lib`; `cargo test -p pantograph-runtime-registry
    technical_fit --lib`; `cargo fmt --manifest-path
    crates/pantograph-runtime-registry/Cargo.toml -- --check`; `git diff
    --check`.
  - Remaining follow-up: wire host-resolved Python package readiness facts
    into production technical-fit requests, then add the image
    planner/gateway missing-proof rejection before worker dispatch.
- 2026-05-16 host package-readiness source re-plan boundary:
  - Boundary found after the runtime-registry readiness filtering slice:
    production `workflow_technical_fit_decision` still passes an explicit
    empty dependency-readiness slice, and there is no current
    standards-compliant host package inventory source to replace it.
  - Existing Python/dependency-environment paths are execution-time,
    node-input-shaped, and legacy/fallback-oriented. Using them as scheduler
    readiness proof would pass Pumas facts or runtime package state through
    the wrong owner boundary and would make worker/dependency preflight a
    hidden scheduler input.
  - What needs planning: define the host-owned package-readiness source that
    observes Python package availability for scheduler admission, including
    ownership (`embedded-runtime` host adapter vs inference declaration owner),
    probe timing/caching, timeout/error behavior, package-name normalization,
    environment selection, diagnostics attribution, and how unavailable or
    unimplemented probes are represented without falling back to worker
    discovery.
  - Implementation must remain staged: first add the host package-readiness
    provider contract and focused tests, then wire production
    technical-fit calls to pass real readiness facts, then enable
    planner/gateway rejection for missing selected readiness proof before
    worker dispatch.
- 2026-05-16 package-readiness provider planning decision:
  - Decision: use the runtime-scoped package-readiness provider option now.
    Embedded-runtime owns the host/provider contract; inference continues to
    own dependency declarations; runtime-registry and scheduler admission only
    consume typed readiness facts.
  - Contract direction: provider input is keyed by executable backend/runtime
    identity plus optional runtime variant/environment selector and dependency
    declarations. Provider output is typed `DependencyReadinessFact` values
    with bounded provider diagnostics for unavailable Python, missing packages,
    unimplemented probes, unsupported platforms, invalid package ids, or
    timeout/probe failures.
  - No-fallback/no-legacy confirmation: do not adapt execution-time
    dependency-environment preflight, graph node inputs, worker imports,
    runtime display strings, or package hints into scheduler readiness proof.
  - Later objective: promote managed environment inventory to a first-class
    scheduler evidence model covering runtime + environment + package
    inventory for venv, Conda, managed, and remote runtimes. The provider
    contract should leave this path open without requiring scheduler policy
    call-site churn.
  - Next implementation slices: add the provider trait/DTO and focused
    contract tests; implement the initial PyTorch/default-Python provider;
    wire production `workflow_technical_fit_decision` to pass provider facts;
    then add image planner/gateway rejection for selected decisions that lack
    ready dependency proof.
- 2026-05-17 package-readiness provider codebase review update:
  - Smallest useful vertical slice: update the provider plan after reviewing
    the current proof path, legacy dependency paths, Python runtime adapter,
    inference planner/gateway, runtime-registry filtering, and standards blast
    radius.
  - Allowed write set: this plan directory only.
  - Findings accepted: the provider direction is standards-compliant only if
    it remains a focused embedded-runtime host-observation module and does not
    grow `technical_fit.rs`, reuse `dependency-environment`, parse diagnostics
    strings, or infer package readiness from imports, Pumas package hints,
    display labels, graph inputs, worker failures, or Python sidecar presence.
  - Contract tightening: provider input must separate executable backend key,
    scheduler runtime id, runtime variant id, and package-readiness
    environment selector. Provider output remains typed
    `DependencyReadinessFact` values plus typed bounded provider diagnostics
    for unavailable Python, missing packages, unsupported dependency kinds,
    invalid package ids, unimplemented probes, unsupported platforms, timeouts,
    and probe process failures.
  - Probe policy: the first PyTorch/default-Python implementation must use a
    fixed no-shell Python probe, bounded timeout, bounded output capture,
    request-local dedupe/cache keyed by backend/runtime/variant/environment
    and dependency set, and no locks held across awaits. It must check
    inference-owned dependency ids as package/distribution ids or fail
    typed/closed when a dependency cannot be safely probed.
  - Standards tightening: implement provider DTOs with validated domain types
    (`BackendId`, `RuntimeVariantId`, `CapabilityAvailabilityId`, and a typed
    environment selector) instead of raw internal strings. Provider failures
    must be typed error/diagnostic enums, not `Result<T, String>` or `anyhow`.
    Keep request normalization, dedupe-key construction, and fact projection
    synchronous; only the subprocess probe boundary should be async. The
    provider must not create a Tokio runtime, spawn untracked tasks, or hold
    locks while awaiting Python. Subprocess execution must use explicit args,
    `kill_on_drop`, timeout handling, bounded stdout/stderr capture, and typed
    process-status diagnostics.
  - Test isolation requirement: focused provider tests must use fake probe
    runners for contract behavior and must isolate or serialize environment
    variables, temp paths, cache state, and subprocess-related global state so
    the suite does not depend on the developer machine's Python environment.
  - Codebase blast-radius update: add the provider as a new focused
    embedded-runtime module, likely `package_readiness_provider.rs`, and keep
    `dependency_readiness.rs` as pure declaration/snapshot projection.
    `technical_fit.rs` may call a small helper to collect provider facts, but
    must not own provider DTOs, probe orchestration, subprocess behavior, or
    dedupe/cache state. The provider should wrap existing Python executable
    resolution into typed diagnostics instead of leaking `Result<T, String>`
    from `python_runtime.rs`. It should build readiness once per
    backend/runtime/variant/environment/dependency set and project those facts
    onto candidates, not probe per candidate. The fake probe runner and
    provider contract tests must land before the real Python runner.
  - Regression guardrails: the provider must not import or call
    `task_executor::dependency_environment`, worker package imports, or graph
    input fallback data. Projection tests must prove backend key, scheduler
    runtime id, runtime variant, and environment selector mapping is
    intentional so the existing `runtime_id`/`backend_key` ambiguity does not
    spread into the provider API.
  - No-fallback/no-legacy confirmation: this remains a replacement path. The
    legacy dependency-environment backend-key/default/hint selection behavior
    still must be removed from canonical inference rather than kept as a
    compatibility shim.
  - Required focused tests added to the plan: provider available-package,
    missing-package, unavailable-Python, unsupported-kind, invalid/unprobeable
    package id, timeout/probe-failure, request-local dedupe, and production
    technical-fit propagation tests.
  - Verification: docs-only plan review and targeted codebase inspection; no
    code tests were run.
  - Remaining follow-up: implement the provider trait/DTO and focused contract
    tests as the next validated slice, then wire production technical-fit to
    pass real provider facts.
- 2026-05-17 package-readiness provider contract implementation slice:
  - Smallest useful vertical slice: add the embedded-runtime
    package-readiness provider trait/DTO contract and fake probe-runner tests
    without production technical-fit wiring.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/package_readiness_provider.rs`,
    `crates/pantograph-embedded-runtime/src/package_readiness_provider_tests.rs`,
    `crates/pantograph-embedded-runtime/src/lib.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`, and this plan
    directory. The untracked root proposal markdown file was ignored per user
    instruction.
  - No-fallback/no-legacy confirmation: the provider consumes
    inference-owned dependency declarations and fake probe outcomes only. It
    does not call `task_executor::dependency_environment`, inspect graph node
    inputs, infer readiness from package hints/display labels, run worker
    imports, select or rank runtimes, create runtimes, or dispatch workers.
  - Implementation notes: added `PackageReadinessProviderRequest`,
    `PackageReadinessEnvironmentSelector`, typed probe requests/outcomes,
    provider diagnostics, and `PackageReadinessProvider<R>`. Request-local
    dedupe is keyed by executable backend key, scheduler runtime id, runtime
    variant, environment selector, and sorted package dependency ids.
    Non-package declarations are projected through the pure resolver without
    triggering an empty Python probe.
  - Focused tests added: fake-runner coverage for available package facts,
    missing package diagnostics, unavailable Python diagnostics, unsupported
    dependency kind without probe fallback, invalid package id diagnostics,
    timeout/probe failure projection, typed scope projection, and request-local
    dedupe for reordered declarations.
  - Standards notes: kept the production provider module under the 500-line
    target by moving focused tests to an adjacent test module and updated the
    embedded-runtime source README for ownership traceability.
  - Verification passed: `cargo fmt --manifest-path
    crates/pantograph-embedded-runtime/Cargo.toml`; `cargo fmt
    --manifest-path crates/pantograph-embedded-runtime/Cargo.toml --
    --check`; `cargo test -p pantograph-embedded-runtime
    package_readiness_provider --lib`; `cargo check -p
    pantograph-embedded-runtime`.
  - Remaining follow-up: implement the real no-shell Python probe runner with
    typed process diagnostics, then wire production
    `workflow_technical_fit_decision` to collect provider facts and pass them
    into existing technical-fit request construction.
- 2026-05-17 no-shell Python package-readiness probe runner slice:
  - Smallest useful vertical slice: add the real default-host Python package
    probe runner behind the package-readiness provider trait without wiring it
    into production technical-fit requests.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/python_package_readiness_probe.rs`,
    `crates/pantograph-embedded-runtime/src/lib.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`, and this plan
    directory. The untracked root proposal markdown file was ignored per user
    instruction.
  - No-fallback/no-legacy confirmation: the runner uses inference-owned
    package ids and the typed provider request only. It does not call
    `task_executor::dependency_environment`, read graph node inputs, infer
    package readiness from Pumas hints/display labels, import worker modules,
    select or rank runtimes, create runtimes, or dispatch workers.
  - Implementation notes: added `ProcessPythonPackageReadinessProbeRunner`
    with explicit `python -I -c <script> <package ids>` args, no shell,
    `kill_on_drop`, bounded timeout, bounded stdout/stderr capture, typed
    process diagnostics, package-id safety validation before launch, JSON
    parsing into `PythonPackageReadinessSnapshot`, and explicit
    `ProbeNotImplemented` for managed Python environments that the current
    host inventory cannot resolve yet.
  - Focused tests added: no-shell command shaping, valid JSON parsing,
    invalid JSON diagnostics, explicit Python environment rejection without
    process launch, and invalid package-id rejection without process launch.
    The tests do not require local Python packages to be installed.
  - Verification passed: `cargo fmt --manifest-path
    crates/pantograph-embedded-runtime/Cargo.toml`; `cargo fmt
    --manifest-path crates/pantograph-embedded-runtime/Cargo.toml --
    --check`; `cargo test -p pantograph-embedded-runtime
    python_package_readiness_probe --lib`; `cargo test -p
    pantograph-embedded-runtime package_readiness_provider --lib`; `cargo
    check -p pantograph-embedded-runtime`.
  - Remaining follow-up: wire `workflow_technical_fit_decision` to collect
    package-readiness provider facts through this runner, then enable the
    selected-decision missing-proof gate before worker dispatch.
- 2026-05-17 production technical-fit package-readiness collection slice:
  - Smallest useful vertical slice: wire production
    `workflow_technical_fit_decision` to collect package-readiness provider
    facts and pass them into existing technical-fit request construction
    without enabling the later planner/gateway missing-proof rejection gate.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `crates/pantograph-embedded-runtime/src/technical_fit_package_readiness.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`, and this plan
    directory. The untracked root proposal markdown file was ignored per user
    instruction.
  - No-fallback/no-legacy confirmation: the collection helper consumes
    inference execution evidence and package-readiness provider output only.
    It does not call `task_executor::dependency_environment`, inspect graph
    node inputs as package readiness, infer readiness from Pumas hints/display
    labels, import worker modules, select or rank runtimes, create runtimes,
    or dispatch workers.
  - Implementation notes: added a focused `technical_fit_package_readiness`
    module. It normalizes package facts through inference execution evidence,
    creates provider requests only for validated PyTorch image-generation
    candidates, dedupes identical backend/runtime/environment/dependency
    requests before provider resolution, and returns dependency-readiness facts
    to the existing execution-evidence adapter. Production collection is
    skipped when required Pumas package facts are already missing so
    technical-fit fails on that typed model-facts diagnostic without launching
    package probes.
  - Focused tests added: a fake provider snapshot proves
    PyTorch/Diffusers package facts produce ready dependency-readiness facts
    through the new technical-fit collection helper without launching Python.
  - Verification passed: `cargo fmt --manifest-path
    crates/pantograph-embedded-runtime/Cargo.toml -- --check`; `cargo test
    -p pantograph-embedded-runtime technical_fit_package_readiness --lib`;
    `cargo test -p pantograph-embedded-runtime technical_fit --lib`; `cargo
    check -p pantograph-embedded-runtime`.
  - Remaining follow-up: enable the image planner/gateway selected-decision
    missing-proof gate before worker dispatch, then remove legacy
    dependency-environment backend-key/fallback selection from canonical
    inference execution.
- 2026-05-17 image planner dependency-readiness proof gate slice:
  - Smallest useful vertical slice: make side-effect-free image-generation
    planning reject scheduler-selected backend decisions that do not carry
    ready PyTorch/Diffusers dependency-readiness proof before worker dispatch.
  - Allowed write set: `crates/inference/src/image_generation_planner.rs`,
    `crates/inference/src/image_generation_planner_tests.rs`,
    `crates/inference/src/gateway_tests.rs`,
    `crates/inference/src/README.md`, and this plan directory. The untracked
    root proposal markdown file was ignored per user instruction.
  - No-fallback/no-legacy confirmation: the planner validates
    scheduler-carried dependency-readiness proof only. It does not probe
    packages, read graph node inputs, infer readiness from Pumas hints/display
    labels, import worker modules, select or rank runtimes, or let the PyTorch
    worker become the first missing-package detector.
  - Implementation notes: added typed planner diagnostics for missing
    dependency-readiness proof and unavailable dependency-readiness proof.
    The proof gate checks the inference-owned PyTorch/Diffusers image
    dependency declarations against `BackendExecutionDecision.dependency_readiness`
    before building `ImageGenerationExecutionPlan`.
  - Focused tests added/updated: planner acceptance and gateway planning
    fixtures now carry ready proof; new planner tests reject empty proof and
    not-installed proof before worker dispatch.
  - Verification passed: `cargo fmt --manifest-path crates/inference/Cargo.toml`;
    `cargo test -p inference image_generation_planner --lib`; `cargo test -p
    inference generate_image_from_planning_input --lib`; `cargo check -p
    inference`; `cargo check -p pantograph-embedded-runtime`.
  - Remaining follow-up: remove the legacy dependency-environment
    backend-key/fallback selection paths from canonical inference execution.
- 2026-05-17 embedded-runtime dependency preflight fail-closed slice:
  - Smallest useful vertical slice: remove legacy backend selection and local
    Python fallback execution from embedded-runtime dependency preflight while
    keeping the explicit `dependency-environment` node as diagnostic/tooling
    workflow support.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/task_executor.rs`,
    `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment.rs`,
    `crates/pantograph-embedded-runtime/src/task_executor/python_execution.rs`,
    focused task-executor tests, embedded-runtime task-executor README notes,
    `crates/pantograph-embedded-runtime/src/README.md`, and this plan
    directory. The untracked root proposal markdown file was ignored per user
    instruction.
  - No-fallback/no-legacy confirmation: canonical Python-backed execution no
    longer selects backend keys from explicit `backend_key` inputs, Pumas
    package backend hints, dependency-requirement backend keys, node-type
    defaults, or local Python fallback allowances. Missing dependency
    bindings and missing runtime packages now block before Python adapter
    dispatch. The explicit `dependency-environment` node may still pass an
    authored backend key for diagnostic/tooling workflows, but that path does
    not dispatch canonical inference execution.
  - Implementation notes: request construction now treats Pumas package facts
    as model/task evidence only, strips legacy backend-key fields before
    Python adapter dispatch, derives Python runtime lifecycle identity from
    the resolved model ref or node family, and removes the fallback branch
    that resolved a model ref after non-ready dependency status.
  - Focused tests added/updated: renamed the dependency fallback tests to
    fail-closed tests; added coverage that canonical preflight ignores
    explicit backend keys; updated package/requirements tests to assert no
    backend selection; added runtime recorder coverage proving legacy
    backend-key inputs are stripped and model-ref engine owns runtime
    identity.
  - Verification passed: `cargo fmt --manifest-path
    crates/pantograph-embedded-runtime/Cargo.toml`; `cargo test -p
    pantograph-embedded-runtime input_helpers --lib`; `cargo test -p
    pantograph-embedded-runtime dependency_fail_closed --lib`; `cargo test -p
    pantograph-embedded-runtime recorder_stream --lib`; `cargo test -p
    pantograph-embedded-runtime task_executor --lib`; `cargo test -p
    pantograph-embedded-runtime technical_fit_package_readiness --lib`;
    `cargo fmt --manifest-path crates/pantograph-embedded-runtime/Cargo.toml
    -- --check`; `cargo check -p pantograph-embedded-runtime`.
  - Verification deviation: an intermediate parallel run of focused Cargo
    tests waited on Cargo package/build locks; the relevant broader
    `task_executor` test suite was rerun serially and passed.
  - Remaining follow-up: continue Milestone 6 with validated Pumas
    artifact/root path proof, reserved `diffusers` runtime identity fixture
    cleanup, and remaining explicit typed runtime/trait input replacement for
    node families not yet on scheduler-owned execution.
- 2026-05-17 root-relative Pumas artifact path slice:
  - Smallest useful vertical slice: add a validated root-relative Pumas
    artifact entry path type at the inference planner/worker boundary so image
    generation rejects unsafe artifact paths before worker-envelope
    construction.
  - Allowed write set: `crates/inference/src/model_contracts.rs`,
    `crates/inference/src/image_generation_planner.rs`,
    `crates/inference/src/backend/pytorch_worker_image_contract.rs`,
    focused inference planner/PyTorch image contract/gateway tests,
    `crates/inference/src/lib.rs`, `crates/inference/src/README.md`,
    the single node-engine planned-image test fixture that consumes
    `ImageGenerationExecutionPlan`, and this plan directory. The untracked
    root proposal markdown file was ignored per user instruction.
  - No-fallback/no-legacy confirmation: the planner accepts only validated
    root-relative Pumas artifact entry paths for image execution plans and
    rejects absolute local paths, traversal, URI-shaped paths, control
    characters, empty values, and overlong paths with typed diagnostics. It
    does not pass raw graph/user paths to the worker, infer roots in the
    worker, or add a resolved-path fallback.
  - Implementation notes: added `PumasArtifactEntryPath` as a serde string
    value object; changed `ImageGenerationExecutionPlan` and
    `PyTorchGenerateImageRequest` to carry the validated type; kept the JSON
    wire shape unchanged; and updated the planned node-engine image test to
    consume the typed path.
  - Focused tests added/updated: planner success serialization now proves the
    artifact path remains a primitive string; new planner tests reject
    absolute and traversing artifact entry paths; PyTorch worker request tests
    map from the validated plan; gateway and node-engine planned-image tests
    use root-relative artifact paths and ready dependency proof.
  - Verification passed: `cargo fmt --manifest-path crates/inference/Cargo.toml`;
    `cargo test -p inference image_generation_planner --lib`; `cargo test -p
    inference --features backend-pytorch pytorch_worker_image_contract --lib`;
    `cargo test -p inference --features backend-pytorch
    pytorch_image_generation --lib`; `cargo fmt --manifest-path
    crates/inference/Cargo.toml -- --check`; `cargo test -p node-engine
    --features inference-nodes,pytorch-nodes
    test_canonical_llm_image_generation_uses_planned_gateway_boundary --lib`;
    `cargo check -p node-engine --features inference-nodes,pytorch-nodes`;
    `cargo check -p inference --features backend-pytorch`; `cargo test -p
    inference generate_image_from_planning_input --lib`; `cargo test -p
    inference gateway::tests --lib`.
  - Verification deviation: initial PyTorch image test filters were run
    without the `backend-pytorch` feature and selected zero tests; they were
    rerun with the feature enabled. The first
    `pytorch_worker_image_contract` run exposed a stale empty
    dependency-readiness fixture and was fixed by carrying ready scheduler
    proof. A node-engine planned-image test also needed the same ready proof
    fixture update. Intermediate parallel Cargo commands waited on package and
    build locks.
  - Remaining follow-up: reconcile reserved `diffusers` runtime identity and
    diagnostics fixtures, then continue replacing remaining generic recursive
    `backend_key` discovery with explicit typed runtime/trait inputs per
    family.
- 2026-05-17 reserved Diffusers runtime identity cleanup slice:
  - Smallest useful vertical slice: keep `diffusers` as a reserved canonical
    runtime spelling without presenting it as an implemented Python sidecar,
    and remove bare `diffusers` from diagnostics/metrics fixtures that model
    observed executable runtime ids.
  - Allowed write set: `crates/pantograph-runtime-identity/src/lib.rs`,
    `crates/pantograph-runtime-identity/src/README.md`,
    `crates/pantograph-embedded-runtime/src/workflow_runtime_tests/diagnostics_snapshot.rs`,
    `crates/pantograph-embedded-runtime/src/workflow_runtime_tests/metrics.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`, and this plan
    directory. The untracked root proposal markdown file was ignored per user
    instruction.
  - No-fallback/no-legacy confirmation: this keeps `diffusers` reserved for a
    future real executable runtime but does not advertise a selectable
    Diffusers sidecar, create a pseudo-runtime candidate, alias Diffusers
    package evidence to PyTorch, or change scheduler selection. Current
    PyTorch image execution remains `pytorch` with `pytorch.diffusers` used
    only as runtime-variant context.
  - Implementation notes: changed the runtime identity display label from
    `Diffusers (Python sidecar)` to `Diffusers (reserved runtime)`, documented
    that reserved identities do not imply installation or implementation, and
    replaced stale bare `diffusers` observed-runtime fixture values with
    `pytorch.diffusers` or another implemented runtime id depending on the
    test purpose.
  - Verification passed: `cargo test -p pantograph-runtime-identity`; `cargo
    test -p pantograph-embedded-runtime diagnostics_snapshot --lib`; `cargo
    test -p pantograph-embedded-runtime
    trace_runtime_metrics_with_observed_runtime_ids --lib`; `cargo check -p
    pantograph-runtime-identity`; `cargo check -p
    pantograph-embedded-runtime`; `cargo fmt --manifest-path
    crates/pantograph-runtime-identity/Cargo.toml -- --check`.
  - Verification deviation: parallel Cargo verification waited on package and
    build locks; all focused commands completed successfully.
  - Remaining follow-up: continue replacing remaining generic recursive
    `backend_key` discovery with explicit typed runtime/trait inputs per
    family, then resume the PyTorch worker/family adapter implementation
    rows.
- 2026-05-17 workflow-service runtime requirement extraction slice:
  - Smallest useful vertical slice: remove workflow-service's generic
    recursive `backend_key` discovery from scheduler runtime-requirement
    extraction and replace it with node-family explicit runtime requirements.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/capabilities.rs`,
    `crates/pantograph-workflow-service/src/README.md`, and this plan
    directory. The untracked root proposal markdown file was ignored per user
    instruction.
  - No-fallback/no-legacy confirmation: package facts, Pumas hints,
    dependency bindings, dependency-environment tooling inputs, GGUF paths, and
    arbitrary nested node metadata no longer become hard scheduler backend
    requirements. Canonical inference uses only the graph-authored optional
    `llm-inference.runtime` input as a scheduler requirement; dedicated
    `onnx-inference` and `audio-generation` node families map from their node
    type rather than mutable `backend_key` payload strings.
  - Implementation notes: replaced `extract_backend_keys_from_value` with
    `extract_node_family_runtime_requirement`, removed GGUF evidence routing
    from workflow-service backend extraction, and documented that runtime
    requirement extraction must not scan arbitrary node JSON.
  - Focused tests added/updated: capability tests now prove Pumas package
    backend evidence, nested unknown-node backend keys, and
    dependency-environment backend keys are ignored for scheduler runtime
    requirements, while `llm-inference.runtime`, `onnx-inference`, and
    `audio-generation` still produce explicit required backend values.
  - Verification passed: `cargo fmt --manifest-path
    crates/pantograph-workflow-service/Cargo.toml`; `cargo test -p
    pantograph-workflow-service extract_required_backends --lib`; `cargo
    check -p pantograph-workflow-service`; `cargo fmt --manifest-path
    crates/pantograph-workflow-service/Cargo.toml -- --check`.
  - Audit result: focused search found no remaining generic backend-key value
    scanner in workflow-service, embedded-runtime, inference, node-engine, or
    workflow-nodes. Remaining `backend_key` fields are DTO/event/runtime
    identity fields, explicit dependency-environment tooling data, Pumas
    package facts, tests, or backend-local diagnostics rather than scheduler
    recursive discovery.
  - Remaining follow-up: resume Milestone 6 PyTorch/Diffusers bridge work with
    the image/PyTorch-specific behavior kept after scheduler selection, then
    continue family adapter and worker artifact retention rows.
- 2026-05-17 image-generation family adapter slice:
  - Smallest useful vertical slice: add an internal PyTorch/Diffusers
    family-adapter resolver and route the image-generation planner through it
    without changing scheduler selection, worker execution, or package-fact
    production.
  - Allowed write set: `crates/inference/src/image_generation_family_adapters.rs`,
    `crates/inference/src/image_generation_planner.rs`,
    `crates/inference/src/lib.rs`, `crates/inference/src/README.md`, focused
    inference tests, and this plan directory. The untracked root proposal
    markdown file was ignored per user instruction.
  - No-fallback/no-legacy confirmation: adapter resolution consumes only
    Pumas Diffusers family/component facts after the scheduler-selected
    backend decision reaches the planner. It does not normalize package hints
    into executable candidates, rank runtimes, inspect model ids or display
    names, pick warmed runtimes, call the worker, or try alternate families
    after a validation error.
  - Implementation notes: added `image_generation_family_adapters` with a
    two-stage resolver: first resolve exactly one supported family from
    `package_facts.diffusers.family_evidence`, then validate required
    component roles for that adapter. The planner maps the adapter's internal
    exact missing/ambiguous-facts diagnostics into the existing public planner
    diagnostic contract.
  - Focused tests added/updated: adapter tests cover Stable Diffusion
    resolution and exact missing VAE component reporting; existing planner
    tests continue to cover ambiguous family evidence, unsupported families,
    exact missing component role paths, ambiguous component sources, option
    rejection, and no Diffusers backend aliasing.
  - Verification passed: `cargo fmt --manifest-path crates/inference/Cargo.toml`;
    `cargo test -p inference image_generation_family_adapters --lib`; `cargo
    test -p inference image_generation_planner --lib`; `cargo check -p
    inference`; `cargo fmt --manifest-path crates/inference/Cargo.toml --
    --check`.
  - Verification deviation: the two focused Cargo test commands were started
    in parallel and one waited on Cargo package/build locks; both completed
    successfully.
  - Remaining follow-up: continue component-role extraction from richer Pumas
    processor/Transformers facts when available, then wire PyTorch worker
    loading to the Pumas-resolved Diffusers directory and retained artifact
    output.
- 2026-05-17 provider-backed selection control coverage slice:
  - Smallest useful vertical slice: complete the approved Node-test coverage
    for provider-backed `selection-input` control accessibility and graph
    gesture containment without introducing a new browser-mounted component
    test platform.
  - Allowed write set:
    `src/components/nodes/workflow/selectionInputState.ts`,
    `src/components/nodes/workflow/selectionInputState.test.ts`,
    `src/components/nodes/workflow/SelectionInputNode.svelte`, and this plan
    directory. The modified and untracked root proposal markdown files were
    ignored because they are unrelated to this slice.
  - No-fallback/no-legacy confirmation: provider-backed selectable values
    still come only from backend port-option results. The component does not
    synthesize executable defaults, hardcode denoising scheduler options,
    infer package facts in frontend state, or add compatibility behavior for
    stale values.
  - Implementation notes: extracted a tested selection-input control model
    that owns the accessible name, graph-gesture containment class, native
    select keyboard contract, and empty-provider disabled state. The Svelte
    component now consumes that model for its label, `nodrag`/`nopan`/`nowheel`
    class, and disabled state.
  - Focused tests added/updated: `selectionInputState.test.ts` now covers
    provider-backed accessible names, blank-label fallback, native select
    keyboard behavior, empty provider disabled state, and graph gesture
    containment classes.
  - Verification passed: `node --experimental-strip-types --test
    src/components/nodes/workflow/selectionInputState.test.ts`; `npm run
    typecheck`.
  - Verification deviation: none.
  - Remaining follow-up: pin Pantograph to the accepted Pumas artifact
    load-target resolver commit, then wire PyTorch worker loading to the
    Pumas-resolved Diffusers directory and continue retained artifact output.
- 2026-05-17 Pumas artifact load-target dependency pin slice:
  - Smallest useful vertical slice: move the workspace `pumas-library`
    dependency pin from the package-facts handoff commit to the accepted Pumas
    artifact load-target resolver commit required by the PyTorch/Diffusers
    worker-loading plan.
  - Allowed write set: root `Cargo.toml`, `Cargo.lock`, and this plan
    directory. The modified and untracked root proposal markdown files were
    ignored because they are unrelated to this slice.
  - No-fallback/no-legacy confirmation: Pantograph now depends on Pumas-owned
    artifact load-target resolution instead of joining Pumas paths locally,
    asking the Python worker to resolve model-library state, or preserving a
    local compatibility bridge around older model-level descriptor APIs.
  - Implementation notes: updated the root workspace Pumas dependency to
    `8444b50df28c3e2bd8db58fb3645fa4dd8664b27` and regenerated `Cargo.lock`
    with the exact requested revision. The dependency source remains the
    canonical GitHub repository. Lockfile generation used a temporary Cargo Git
    CLI `insteadOf` override against the local clean Pumas checkout because the
    requested commit was present locally but the configured remote branch still
    advertised `f63ef180` during verification.
  - Focused tests added/updated: no new behavior tests were needed for the
    dependency pin itself; verification compiled the Pantograph crates that
    consume Pumas model-library and embedded-runtime contracts.
  - Verification passed: `cargo check -p pantograph-embedded-runtime`; `cargo
    check -p workflow-nodes --features model-library`; `git diff --check`.
  - Verification deviation: a direct `cargo update -p pumas-library
    --manifest-path Cargo.toml` could not fetch the requested revision from the
    configured GitHub URL in this environment, so the lockfile was generated
    through Cargo's Git CLI path with a one-command local checkout override.
  - Remaining follow-up: wire PyTorch worker loading to the Pumas-resolved
    Diffusers directory and retained artifact output.
- 2026-05-17 Pumas artifact load-target worker-loading slice:
  - Smallest useful vertical slice: carry the Pumas artifact load-target
    contract from `puma-lib`/node-engine into the image-generation planner and
    PyTorch worker envelope, then make the Python worker load the resolved
    Diffusers directory before generation.
  - Allowed write set: inference model/planner/worker contracts, Python image
    worker envelope handling, focused inference fixtures/tests,
    node-engine planned image input/context forwarding/tests,
    workflow-nodes Pumas selector access, embedded-runtime `puma-lib` Pumas
    resolver wiring/tests, and this plan directory. The modified/untracked
    root proposal markdown files were ignored because they are unrelated to
    this slice.
  - No-fallback/no-legacy confirmation: Pantograph now requires a
    Pumas-resolved load target for planned image execution and passes only
    that approved target into the PyTorch worker. It does not join Pumas paths,
    ask the Python worker to resolve model-library state, infer roots from
    graph inputs, or synthesize a target when Pumas reports the selected
    artifact is not ready.
  - Implementation notes: added inference-owned `PumasArtifactLoadTarget` and
    `PumasArtifactLoadPathKind` mirrors for the Pumas response target, extended
    `ImageGenerationPlanningInput` and `ImageGenerationExecutionPlan` with the
    target, and added planner diagnostics for invalid target model ref,
    artifact kind, path kind, validation state, and local load path. The
    PyTorch worker envelope now carries `artifact_load_target`; the Python
    worker validates it and calls `load_diffusion_model(local_load_path, ...)`
    before generation. Node-engine planned image execution requires
    `resolved_model_artifact_load_target`, and `puma-lib` uses
    `PumasSelectorAccess::resolve_model_artifact_load_target` when full package
    facts are available.
  - Focused tests added/updated: inference planner, gateway, PyTorch worker
    Rust/Python contract tests, PyTorch worker fixture, model-contract serde
    coverage for the Pumas target wire shape, node-engine planned image
    success/fail-closed tests, dependency-context forwarding tests, and
    `puma-lib` selector tests. The `puma-lib` fixture test now explicitly
    records that the current imported-bundle fixture lacks an indexed
    selected-artifact row and therefore must not produce a synthesized target.
  - Verification passed: `cargo fmt --manifest-path crates/inference/Cargo.toml`;
    `cargo fmt --manifest-path crates/node-engine/Cargo.toml`; `cargo fmt
    --manifest-path crates/pantograph-embedded-runtime/Cargo.toml`; `cargo fmt
    --manifest-path crates/workflow-nodes/Cargo.toml`; `cargo test -p
    inference image_generation_planner --features backend-pytorch`; `cargo
    test -p inference pytorch_worker_image --features backend-pytorch`; `cargo
    test -p inference generate_image_from_planning_input --features
    backend-pytorch`; `cargo test -p inference
    pumas_artifact_load_target_decodes_existing_pumas_wire_shape --features
    backend-pytorch`; `cargo test -p node-engine
    test_canonical_llm_image_generation --features inference-nodes,pytorch-nodes`;
    `cargo test -p node-engine dependency_inputs --features
    inference-nodes,pytorch-nodes`; `cargo test -p pantograph-embedded-runtime
    puma_lib`; `cargo check -p inference --features backend-pytorch`; `cargo
    check -p node-engine --features inference-nodes,pytorch-nodes`; `cargo
    check -p pantograph-embedded-runtime`.
  - Verification deviation: several Cargo commands were started in parallel
    and waited on Cargo package/build locks; they completed successfully after
    Cargo serialized the build.
  - Discovered issue/follow-up: the then-current Pantograph/Pumas imported
    Diffusers test fixture could hydrate package facts but Pumas
    `resolve_model_artifact_load_target` reported `ArtifactMissing` because
    the indexed selected-artifact row did not contain the selected artifact
    identity. Pantograph behavior remained fail-closed with no synthesized
    target. This fixture gap was resolved by the later 2026-05-17 small-model
    smoke path slice; retained artifact output remains handled by the earlier
    artifact-retention slice.
- 2026-05-17 memory policy planning slice:
  - Smallest useful vertical slice: plan the remaining checked-memory and
    artifact-size work as a cross-runtime scheduler/inference contract before
    implementing more image-generation memory behavior.
  - Allowed write set:
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/06-device-runtime-selection.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. The modified and untracked root proposal markdown
    files were ignored because they are unrelated to this slice.
  - No-fallback/no-legacy confirmation: the plan keeps memory-fit decisions in
    scheduler policy and rejects overflow, missing required facts, unsupported
    runtime/family cases, and unavailable estimates through typed diagnostics
    or explicit estimate states. It does not add PyTorch-only fallback math,
    worker-side runtime choice, sentinel `0` estimates, or silent saturation.
  - Planning notes: Pumas owns package/component/storage/validation facts;
    inference owns request-local checked arithmetic and typed estimate
    diagnostics; backend/runtime providers expose reduced readiness and
    resource-estimate facts; scheduler owns admission, reservations, retry,
    rescheduling, termination after retry exhaustion, and history-backed
    ranking. Estimate states must distinguish `available`, `not_available`,
    `not_implemented`, `insufficient_facts`, `overflow`,
    `unsupported_family`, and `unsupported_runtime`. Timing and memory history
    may influence ranking only after every valid runtime candidate for the same
    workflow/model/runtime key has at least five completed runs.
  - Reference note: InvokeAI's VAE working-memory estimator is useful evidence
    for the kinds of image-family facts Pantograph will need, but Pantograph
    must keep the estimator and scheduler policy in its own typed contracts
    rather than copying InvokeAI invocation or model-manager architecture.
  - Focused tests added/updated: none; this is a documentation/planning slice.
  - Verification passed: `git diff --check`.
  - Verification deviation: no runtime tests were run because no source,
    fixture, config, lockfile, or generated file changed.
  - Remaining follow-up: implement the staged resource-estimate DTO,
    calculator, candidate-projection, scheduler-admission, and observed
    memory/timing ledger slices before marking the Milestone 6 checked-memory
    checklist item complete.
- 2026-05-17 inference resource-estimate contract slice:
  - Smallest useful vertical slice: add the inference-owned resource-estimate
    contract foundation before moving image output-size or runtime memory
    estimates onto it.
  - Allowed write set: `crates/inference/src/resource_estimates.rs`,
    `crates/inference/src/lib.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. The unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: the new contract uses explicit
    estimate states and typed diagnostics for overflow, insufficient facts,
    unavailable, unimplemented, and unsupported estimates. It does not encode
    missing estimates as `0`, use `None` as a scheduler decision state, add
    PyTorch-only memory heuristics, or alter scheduler ranking.
  - Focused tests added: inference serde/constructor tests prove available
    estimates carry byte values, non-available estimates serialize without
    sentinel values, and non-available construction rejects the available
    state.
  - Verification passed: `cargo test -p inference resource_estimate --lib`.
  - Remaining follow-up: add runtime-registry/workflow technical-fit
    projection for the same typed estimate states before closing the first
    staged memory-estimate contract item, then move existing output RGBA
    estimate arithmetic into this shape.
- 2026-05-17 output RGBA estimate migration slice:
  - Smallest useful vertical slice: replace the image execution plan's
    optional `estimated_output_rgba_bytes` field with the new typed
    `resource_estimates` contract for the existing output-size estimate only.
  - Allowed write set: `crates/inference/src/resource_estimates.rs`,
    `crates/inference/src/lib.rs`,
    `crates/inference/src/image_generation_planner.rs`,
    `crates/inference/src/image_generation_planner_tests.rs`,
    `crates/inference/src/gateway_tests.rs`,
    `crates/inference/src/backend/pytorch_image_generation_tests.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log.
  - No-fallback/no-legacy confirmation: the old optional estimate field is
    removed rather than mirrored. Known output estimates use
    `available(value_bytes)`, missing width/height uses an explicit
    `insufficient_facts` state, and overflow produces typed planner and
    resource-estimate diagnostics instead of saturation, `0`, or silent
    omission.
  - Focused tests updated: image planner tests assert available and
    insufficient-facts estimate states, overflow remains diagnostic-backed,
    and gateway/PyTorch plan fixtures use the typed estimate contract.
  - Verification passed: `cargo test -p inference image_generation --lib` and
    `cargo test -p inference resource_estimate --lib`.
  - Remaining follow-up: project typed estimate states through
    runtime-registry/workflow technical-fit and add family/runtime calculators
    for estimates beyond output RGBA bytes.
- 2026-05-17 technical-fit resource-estimate replan slice:
  - Decision: use option 3, replacing the old technical-fit estimate contract
    with typed resource estimate records across runtime-registry,
    workflow-service, and embedded-runtime.
  - Rejected options:
    - Minimal adapter into old optional MB fields and confidence strings,
      because it would preserve the legacy contract and lose typed state
      details such as overflow versus insufficient facts.
    - Appending typed estimates beside old fields, because it would create two
      sources of truth and fragment scheduler/admission reasoning.
    - Waiting for richer Pumas/runtime facts before contract replacement,
      because the typed contract can already represent `insufficient_facts`,
      `not_available`, and `not_implemented` while later facts mature.
  - Planned replacement stages:
    1. Runtime-registry contract slice: replace
       `RuntimeTechnicalFitResourceEstimate` optional MB fields with typed
       estimate records/states/diagnostics and update registry serde,
       normalization, technical-fit, and selection tests. Do not change
       ranking policy in this slice.
    2. Workflow-service mirror slice: replace
       `WorkflowTechnicalFitResourceEstimate` and runtime-requirement estimate
       confidence strings with the same typed record shape, update public DTO
       tests/fixtures, and remove legacy estimate field construction.
    3. Embedded-runtime projection slice: project typed workflow/runtime
       estimates without converting them through optional MB/confidence fields,
       update adapter tests, and remove legacy projection helpers.
    4. Candidate/admission slice: project reduced typed estimates into
       scheduler-facing candidate facts, then make admission consume typed
       `peak_vram_bytes`/`peak_ram_bytes` estimates and current pressure with
       typed diagnostics.
    5. History slice: persist observed timing and memory/OOM facts and keep
       history-backed ranking gated until every valid runtime candidate for the
       same workflow/model/runtime key has at least five completed runs.
  - No-fallback/no-legacy confirmation: the replacement must remove old
    technical-fit estimate fields from each touched boundary instead of
    preserving compatibility shims. Missing or unavailable estimates must use
    typed states, not `None`, `0`, saturation, or confidence-string control
    flow.
  - Verification passed: `git diff --check`.
- 2026-05-18 runtime-registry technical-fit estimate contract slice:
  - Smallest useful vertical slice: replace the runtime-registry
    technical-fit candidate/decision estimate contract with typed estimate
    records while leaving workflow-service and embedded-runtime mirrors for the
    next serial projection slices.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/lib.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: the registry no longer exposes the old
    singular `resource_estimate` optional MB-field shape on technical-fit
    candidates or decisions. It uses typed `resource_estimates` records with
    explicit states and diagnostics and does not adapt those records into old
    fields or preserve both shapes.
  - Focused tests updated: runtime-registry technical-fit tests prove typed
    estimate state serde does not emit legacy MB fields, request normalization
    preserves typed estimate records, and selected decisions copy typed
    estimates from candidates.
  - Verification passed: `cargo test -p pantograph-runtime-registry
    technical_fit --lib`.
  - Remaining follow-up: update workflow-service and embedded-runtime
    technical-fit mirrors/projections to the same typed estimate records before
    running broader cross-crate checks.
- 2026-05-18 workflow/embedded technical-fit estimate projection slice:
  - Smallest useful vertical slice: replace downstream workflow-service and
    embedded-runtime technical-fit projection with the runtime-registry typed
    `resource_estimates` contract while leaving scheduler admission/ranking for
    the later memory-policy slices.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/lib.rs`,
    affected workflow-service technical-fit/preflight tests,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `crates/pantograph-embedded-runtime/src/technical_fit_execution_evidence.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: workflow-service and embedded-runtime
    no longer construct or project the old singular technical-fit
    `resource_estimate` optional MB-field shape. Embedded-runtime converts
    runtime-requirement peak RAM/VRAM inputs into typed byte estimates with
    checked arithmetic, and overflow is represented as a typed diagnostic
    rather than saturation, omission, `0`, or confidence-string control flow.
  - Focused tests added/updated: workflow-service serde coverage proves typed
    estimate states serialize without legacy MB fields; embedded-runtime
    technical-fit tests prove runtime-registry estimates project into workflow
    decisions, execution-evidence candidates retain typed estimate vectors, and
    MiB-to-byte overflow emits a typed estimate diagnostic.
  - Verification passed: `cargo test -p pantograph-runtime-registry
    technical_fit --lib`; `cargo test -p pantograph-workflow-service
    technical_fit --lib`; `cargo test -p pantograph-embedded-runtime
    technical_fit --lib`; `cargo fmt --all -- --check`; `git diff --check`.
  - Verification deviation: the first embedded-runtime focused test run
    exposed stale single-estimate expectations after the contract replacement;
    the expectations were updated to assert the new multi-estimate
    `peak_vram_bytes`/`peak_ram_bytes` behavior and the focused suite then
    passed.
  - Remaining follow-up: update candidate admission to consume typed
    `peak_vram_bytes`/`peak_ram_bytes` estimates and current pressure, then add
    timing/memory/OOM history capture for the later scheduler ranking policy.
- 2026-05-17 small-model Pumas load-target smoke path slice:
  - Smallest useful vertical slice: update the embedded-runtime Puma-Lib Tiny
    SD Turbo-style imported Diffusers fixture so it includes a selected
    `diffusers` artifact id and proves Puma-Lib emits a Pumas-approved
    artifact load target before any Juggernaut attempt.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/task_executor_tests.rs`,
    `crates/pantograph-embedded-runtime/src/task_executor_tests/puma_lib.rs`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. The modified and untracked root proposal markdown
    files were ignored because they are unrelated to this slice.
  - No-fallback/no-legacy confirmation: the fixture now lets Pumas own the
    selected artifact identity and load-target resolution. Pantograph still
    does not join Pumas paths, synthesize missing load targets, ask workers to
    resolve library state, or continue when Pumas reports an artifact as
    missing, ambiguous, stale, or invalid.
  - Implementation notes: added `selected_artifact_id = diffusers` to the
    imported Diffusers fixture metadata and tightened the Puma-Lib execution
    test to require `resolved_model_artifact_load_target` with
    `diffusers_bundle`, directory load path, `external_reference` storage,
    `valid` validation state, and the selected artifact id in the returned
    Pumas model ref.
  - Focused tests added/updated:
    `puma_lib_execution_rebinds_stale_model_path_from_model_id` now covers the
    small-model fixture load-target path.
  - Verification passed: `cargo test -p pantograph-embedded-runtime puma_lib
    --lib`.
  - Remaining follow-up: the broad checked-memory item remains open for the
    staged resource-estimate contract/admission work; broader final release
    validation still needs the Milestone 8 test/build sweep.
- 2026-05-18 scheduler pressure typed-estimate ranking slice:
  - Smallest useful vertical slice: remove the remaining legacy peak RAM/VRAM
    MB fields from runtime-registry resource pressure and make existing
    budget-pressure ranking activate from typed candidate memory estimates.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/tests/fixtures/technical_fit_contract.json`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: resource pressure no longer carries
    legacy estimate fields. The scheduler uses typed candidate
    `peak_vram_bytes`/`peak_ram_bytes` records and current loaded-runtime
    pressure for budget-pressure ranking activation, with memory admission and
    reservation diagnostics deferred to the next memory-policy slice.
  - Focused tests/fixtures updated: runtime-registry technical-fit tests cover
    budget-pressure ranking with typed candidate estimates, the registry
    contract fixture omits legacy pressure estimate fields, and embedded
    projection tests assert current-pressure-only request projection.
  - Verification passed: `cargo test -p pantograph-runtime-registry
    technical_fit --lib`; `cargo test -p pantograph-runtime-registry --test
    technical_fit_contract`; `cargo test -p pantograph-embedded-runtime
    technical_fit --lib`.
  - Remaining follow-up: implement the memory-policy admission/reservation
    slice so explicit runtime/device requirements fail with typed diagnostics
    when typed estimates and current pressure cannot fit.
- 2026-05-18 memory admission re-plan boundary:
  - Finding: the next memory-policy code slice crosses a contract boundary.
    Runtime-registry admission and reservation still use MB-shaped
    `RuntimeReservationRequirements`/`RuntimeAdmissionBudget` fields, while
    technical-fit candidates now use typed byte-valued estimates. Runtime
    snapshots also do not expose reduced admission budget or active claim facts,
    so a pure selector cannot reject over-budget candidates yet.
  - Options considered:
    1. Translate typed estimates back into existing MB reservation fields.
       Rejected because it preserves the legacy contract and creates a second
       source of truth.
    2. Add typed fields beside the MB fields. Rejected because it fragments
       admission reasoning and creates ambiguous precedence rules.
    3. Replace admission/reservation with typed byte-valued estimate and claim
       facts, expose reduced budget/claim facts in snapshots, then make
       technical-fit selection consume those facts. Accepted because it keeps
       scheduler policy pure, keeps runtime selection easy to change, and
       removes the legacy MB path instead of shimming it.
  - No-fallback/no-legacy confirmation: the next code slices must remove the
    old reservation/admission MB contract from each touched boundary. They must
    not keep old fields as compatibility aliases or call mutable registry
    admission from the pure technical-fit selector.
  - Verification passed: `git diff --check`.
- 2026-05-18 typed admission/reservation contract slice:
  - Smallest useful vertical slice: replace runtime-registry
    admission/reservation MB contracts with typed byte-valued resource budget
    rows and reservation claims, plus the minimal embedded-runtime projection
    needed to compile callers against the new contract.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/admission.rs`,
    `crates/pantograph-runtime-registry/src/lib.rs`,
    `crates/pantograph-runtime-registry/src/lib_tests.rs`,
    `crates/pantograph-runtime-registry/src/lib_tests/admission.rs`,
    `crates/pantograph-runtime-registry/src/lib_tests/lifecycle.rs`,
    `crates/pantograph-runtime-registry/src/lib_tests/reservations.rs`,
    `crates/pantograph-embedded-runtime/src/embedded_workflow_host.rs`,
    `crates/pantograph-embedded-runtime/src/embedded_workflow_host_helpers.rs`,
    `crates/pantograph-embedded-runtime/src/lib_tests/host_helper_tests.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: the touched runtime-registry boundary
    no longer accepts MB-shaped admission budgets, reservation requirements,
    reservation claims, or admission failure payloads. Embedded-runtime
    converts the still-upstream workflow MiB requirement fields into typed byte
    claims with checked arithmetic; overflow fails with a typed workflow
    service error instead of saturation, omission, or a fallback claim.
  - Focused tests updated: runtime-registry admission/reservation tests assert
    byte-valued budget math, admission failures, accounting overflow, and
    budget underflow; embedded host helper tests assert checked projection into
    typed runtime-registry claims and updated error mapping.
  - Verification passed: `cargo test -p pantograph-runtime-registry admission
    --lib`; `cargo test -p pantograph-embedded-runtime host_helper --lib`;
    `cargo test -p pantograph-runtime-registry --lib`; `cargo fmt --all --
    --check`; `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` run
    reported formatting changes only; `cargo fmt --all` was applied before
    final verification.
  - Remaining follow-up: expose reduced typed admission budget and active claim
    facts in runtime snapshots so the pure technical-fit selector can reject
    over-budget candidates before selection.
- 2026-05-18 runtime snapshot admission-facts slice:
  - Smallest useful vertical slice: expose reduced typed admission budget and
    active reservation claim facts on runtime-registry snapshots without
    changing technical-fit selection behavior.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/snapshot.rs`,
    `crates/pantograph-runtime-registry/src/registry_queries.rs`,
    `crates/pantograph-runtime-registry/src/lib.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/lib_tests/admission.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: snapshots expose only typed byte
    budget rows and typed active reservation claims. They do not revive MB
    fields, pass full workflow/Pumas facts through runtime selection, or call
    mutable admission during technical-fit ranking.
  - Focused tests added/updated: runtime-registry snapshot coverage proves
    typed admission budget and per-active-reservation byte claims are present;
    technical-fit test snapshot builders were updated for the appended snapshot
    fields.
  - Verification passed: `cargo test -p pantograph-runtime-registry
    runtime_snapshot_exposes_reduced_admission_budget_and_active_claims
    --lib`; `cargo test -p pantograph-runtime-registry --lib`; `cargo test -p
    pantograph-runtime-registry --test technical_fit_contract`; `cargo fmt
    --all -- --check`.
  - Remaining follow-up: make technical-fit candidate eligibility consume the
    typed snapshot budget/claim facts and emit typed diagnostics for
    over-budget candidates before selection.
- 2026-05-18 diagnostics-ledger runtime-history resource observation slice:
  - Smallest useful vertical slice: add typed terminal resource observations
    to the diagnostics ledger and include those facts in runtime-selection
    history summaries, without changing scheduler ranking behavior.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/lib.rs`,
    `crates/pantograph-diagnostics-ledger/src/runtime_selection_history.rs`,
    `crates/pantograph-diagnostics-ledger/src/schema.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite/runtime_selection_history_sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-diagnostics-ledger/README.md`,
    `crates/pantograph-diagnostics-ledger/src/README.md`,
    terminal-event construction call sites in workflow-service,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: OOM history is explicit typed
    `RunMemoryFailureKind::OutOfMemory`; it is not inferred from terminal
    error strings or incidental metadata. Memory observations are byte-valued
    fields in the terminal payload and projections, not MB compatibility
    fields.
  - Focused tests added/updated: runtime-selection history now proves peak
    RAM/VRAM typical ranges and OOM counts are summarized for the exact
    workflow/task/model/runtime/device key; schema migration coverage proves
    the projection columns are added for existing v24 ledgers.
  - Verification passed: `cargo test -p pantograph-diagnostics-ledger
    runtime_selection_history --lib`; `cargo test -p
    pantograph-diagnostics-ledger
    existing_v24_schema_adds_scheduler_learning_output_and_memory_projection_columns
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo test -p
    pantograph-diagnostics-ledger --lib`; `cargo test -p
    pantograph-workflow-service diagnostics --lib`; `cargo fmt --all --
    --check`; `git diff --check`.
  - Remaining follow-up: producers still need to populate real terminal
    resource observations from runtime execution and memory/OOM telemetry
    before scheduler history ranking can consume them.
- 2026-05-18 producer telemetry re-plan boundary:
  - Investigation result: the current codebase can persist terminal
    `resource_observation` facts, but it does not yet have a typed
    inference/node-execution telemetry contract that carries observed peak
    RAM, observed peak VRAM, or OOM state to workflow terminal recording.
    Existing OOM detection in inference support is string-based, and
    artifact-store `max_memory_bytes` is a cache/retention policy limit rather
    than measured runtime memory.
  - Rejected options: do not parse terminal error strings for OOM, and do not
    map artifact policy limits into observed runtime memory facts.
  - Preferred re-plan option: define a canonical typed
    execution-resource-observation contract at the inference/node execution
    boundary, wire runtime backends to populate it when available, forward it
    through node-engine and embedded-runtime, and only then attach it to
    `RunTerminalPayload`. This preserves one scheduler-history source of
    truth and avoids incidental metadata.
  - Alternative retained for discussion: carry memory facts only on inference
    diagnostic events and make history queries join them. This is less clean
    because it fragments attribution across event families.
  - Stop condition: implementation should pause here until the typed producer
    contract is accepted; filling terminal memory/OOM history without that
    contract would violate the no-fallback/no-legacy rule.
- 2026-05-18 resource-observation blast-radius plan refinement:
  - Smallest useful vertical slice: update the existing producer-telemetry
    re-plan with codebase review constraints before implementation starts.
    This is documentation-only and does not change contracts or runtime
    behavior.
  - Allowed write set:
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log.
  - No-fallback/no-legacy confirmation: the refined plan does not preserve
    old terminal error parsing, artifact-cache memory reuse, image-only
    telemetry, or scheduler-side OS probing. Legacy OOM string checks must be
    removed or converted immediately to typed adapter-local memory-failure
    facts.
  - Implementation guidance added: extract an inference lifecycle event
    builder/context before adding resource observation fields; use lifecycle
    events as the first telemetry transport; put PyTorch resource facts on the
    generic worker success/failure envelopes; use `sysinfo` process RSS first
    and reserve OS-specific modules for proven gaps; keep detailed
    source/availability facts in inference diagnostics unless scheduler later
    needs them on terminal events; extend runtime-registry candidate history
    with memory/OOM fields before ranking uses diagnostics-ledger memory
    history.
  - Verification for this documentation slice: `git diff --check`.
- 2026-05-18 resource-observation standards iteration:
  - Smallest useful vertical slice: iterate the accepted resource-observation
    plan against the coding standards and current codebase blast radius before
    starting implementation. This is documentation-only and does not change
    contracts or runtime behavior.
  - Allowed write set:
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - Standards reviewed: plan execution/worktree hygiene, Rust API
    correct-by-construction contracts, sync-core/async-shell boundaries,
    platform module isolation, dependency ownership, bounded diagnostics,
    interop envelope validation, lifecycle ownership, testing isolation, and
    documentation traceability.
  - Codebase blast-radius findings: `gateway.rs`, `server.rs`,
    `embedding_runtime.rs`, `backend/llamacpp_support.rs`, and
    runtime-registry technical-fit modules are threshold-crossing files that
    require decomposition review before telemetry edits; `ProcessHandle::pid`
    and the existing `sysinfo` dependency support a dependency-minimizing
    process-RSS slice; current OOM string detection exists only as legacy
    adapter-local translation points and must not become scheduler or workflow
    policy.
  - Plan updates made: resource observation DTOs must be typed, bounded,
    de-duplicated, deterministically ordered, and free of raw process output or
    local paths; process sampling must have named interval/limit constants,
    target-process refresh where possible, tracked cancellation/`JoinHandle`
    ownership, and finish/cancel/drop cleanup tests; new platform dependencies
    require proof that `sysinfo` is insufficient plus dependency-tree and
    feature-contract verification; new source directories and host-facing
    contracts require README or ADR updates in the same slice.
  - No-fallback/no-legacy confirmation: these refinements reject compatibility
    shims, terminal string parsing, unbounded diagnostic metadata, scheduler OS
    probes, and image-only resource telemetry. They require replacement with
    canonical typed resource-observation contracts and adapter-local typed
    translation where external runtimes lack structured telemetry.
  - Verification for this documentation slice: `git diff --check`.
- 2026-05-18 resource-observation post-review blast-radius update:
  - Smallest useful vertical slice: fold the latest codebase-review findings
    into the accepted resource-observation plan before implementation starts.
    This is documentation-only and does not change contracts or runtime
    behavior.
  - Allowed write set:
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - Codebase blast-radius findings: embedded-runtime diagnostic projection
    currently only persists `InferenceExecutionDiagnosticObservedPayload` when
    known bounded fields are present, so resource telemetry needs a diagnostic
    payload and persistability-gate slice before terminal payload wiring;
    `InferenceRequestLifecycleEvent` is constructed outside `gateway.rs` in
    node-engine, embedded-runtime, and tests, so the builder/context migration
    must be shared; the existing process spawner owns untracked
    stdout/stderr/monitor `tokio::spawn` tasks and must not be copied by the
    resource sampler; legacy OOM parser cleanup targets are
    `inference::server`, `inference::embedding_runtime`, and
    `backend::llamacpp_support`; the Python worker repeats JSON response
    envelope construction and needs a local helper before resource telemetry
    is added; runtime-registry history projection still needs memory/OOM facts
    before scheduler ranking can consume persisted diagnostics-ledger memory
    history.
  - Plan updates made: the staged resource-observation design now makes
    diagnostic projection an explicit slice before terminal
    `RunTerminalPayload.resource_observation`, broadens lifecycle builder
    migration across gateway/node-engine/embedded-runtime/tests, records the
    tracked monitor lifecycle requirement as separate from current process
    spawner cleanup, names the legacy OOM cleanup targets, requires Python
    worker response helper extraction before telemetry, and keeps
    runtime-registry memory/OOM history projection before ranking activation.
  - No-fallback/no-legacy confirmation: these refinements do not preserve old
    terminal string parsing, incidental metadata, image-only telemetry, or
    fragmented scheduler history. Legacy string checks may remain only as
    adapter-local external-runtime translation that immediately emits typed
    memory-failure facts and bounded diagnostics.
  - Verification for this documentation slice: `git diff --check`.
- 2026-05-18 inference resource-observation DTO slice:
  - Smallest useful vertical slice: add the shared inference-owned execution
    resource observation contract and focused validation/serde tests without
    wiring producers, lifecycle events, terminal payloads, or scheduler
    ranking.
  - Allowed write set: `crates/inference/src/resource_observation.rs`,
    `crates/inference/src/lib.rs`, `crates/inference/README.md`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: the new contract is typed producer
    telemetry only. It does not parse terminal error strings, reuse artifact
    cache memory policy, infer missing metrics as zero, add image-only
    telemetry, or activate scheduler ranking. Unavailable runtime metrics are
    explicit typed availability facts.
  - Implementation completed: added `InferenceExecutionResourceObservation`
    plus typed metric, source, availability, and memory-failure enums. The
    constructor and serde decode path validate non-empty observations, reject
    zero-valued peak metrics, enforce bounded source/availability collections
    before de-duplication, order facts deterministically, and reject source
    attribution when there is no matching metric value or availability fact.
    The contract is re-exported from `inference` and documented in the module
    README as a producer contract rather than scheduler policy.
  - Focused tests added: resource-observation round-trip tests, decode
    validation, zero-value rejection, bounded collection rejection,
    deterministic de-duplication/ordering, source-attribution validation,
    unavailable metric encoding, and merge max-peak behavior.
  - Verification passed: `cargo test -p inference resource_observation
    --lib`, `cargo check -p inference`, `cargo fmt --all -- --check`, and
    `git diff --check`.
  - Remaining follow-up: the next slice is the lifecycle event builder/context
    migration before telemetry fields are added to lifecycle events.
- 2026-05-18 lifecycle event builder/context slice:
  - Smallest useful vertical slice: add the shared lifecycle event
    builder/context and migrate current direct lifecycle event constructors
    before resource telemetry fields are added.
  - Allowed write set: `crates/inference/src/types.rs`,
    `crates/inference/src/lib.rs`, `crates/inference/src/gateway.rs`,
    `crates/inference/tests/model_contracts.rs`, `crates/inference/README.md`,
    `crates/node-engine/src/core_executor.rs`,
    `crates/node-engine/src/core_executor/dependency_preflight.rs`,
    `crates/pantograph-embedded-runtime/src/node_execution_ledger_tests.rs`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: this replaces direct field-list
    lifecycle event construction with a canonical builder/context. It does not
    add resource telemetry fields, parse OOM strings, synthesize terminal
    memory observations, or preserve a parallel legacy constructor.
  - Implementation completed: added `InferenceRequestLifecycleEvent::builder`,
    `InferenceRequestLifecycleEventBuilder`, and
    `InferenceRequestLifecycleEventContext`; re-exported the new types from
    `inference`; migrated gateway lifecycle emitters, node-engine lifecycle
    emitters, inference public-contract tests, and embedded-runtime lifecycle
    fixtures to the builder.
  - Discovered issue resolved: embedded-runtime lifecycle verification was
    blocked by a stale technical-fit test fixture that did not populate the
    diagnostics-ledger memory/OOM history fields added by an earlier slice.
    The fixture now uses explicit zero/none memory facts. The workflow
    lifecycle sink tests also depended on querying node-status projections
    without applying the diagnostic projection refresh that append requests;
    they now refresh node-status projection explicitly before querying.
  - Focused tests updated: lifecycle serde/contract tests, model-contract JSON
    key test, embedded-runtime lifecycle adapter and workflow sink tests, and
    runtime-selection history summary fixture coverage.
  - Verification passed: `cargo test -p inference lifecycle --lib`, `cargo
    test -p inference --test model_contracts
    public_inference_contract_json_keys_avoid_scheduler_policy_language`,
    `cargo test -p node-engine inference_lifecycle --lib`, `cargo test -p
    pantograph-embedded-runtime inference_lifecycle --lib`, `cargo test -p
    pantograph-embedded-runtime
    runtime_selection_history_summaries_project_exact_candidate_keys --lib`,
    `cargo check -p pantograph-embedded-runtime`, `cargo fmt --all --
    --check`, and `git diff --check`.
  - Remaining follow-up: the next resource-observation slice is extending
    `InferenceExecutionDiagnosticObservedPayload` and embedded-runtime
    diagnostic projection/persistability gates before terminal payload wiring.
- 2026-05-18 diagnostic resource-observation projection slice:
  - Smallest useful vertical slice: add resource observation to the shared
    lifecycle event and diagnostic payload, then prove embedded-runtime
    persists resource observations as inference diagnostics before terminal
    payload wiring.
  - Allowed write set: `crates/inference/src/types.rs`,
    `crates/inference/src/resource_observation.rs`, `crates/inference/README.md`,
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/lib.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-embedded-runtime/src/node_execution_ledger.rs`,
    `crates/pantograph-embedded-runtime/src/node_execution_ledger_tests.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    `docs/plans/current-image-generation-graphs/milestones/06-pytorch-diffusers-image-generation-execution-slice.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - Sequencing note: the lifecycle event needed an optional
    `resource_observation` field in this slice to prove the diagnostic
    persistability gate. This completes the shared event-contract portion of
    staged item 5 before the resource-monitor slice, without adding producers
    or terminal payload projection.
  - No-fallback/no-legacy confirmation: resource observations are not inferred
    from terminal error strings, cache policy, or scheduler metadata. The
    projection maps explicit typed enum cases and keeps the inference resource
    observation enums exhaustive so future metric/source/state additions force
    deliberate projection updates instead of falling through to generic
    strings.
  - Implementation completed: added optional resource observation to
    `InferenceRequestLifecycleEvent` and its builder; added bounded
    diagnostics-ledger resource-observation summaries to
    `InferenceExecutionDiagnosticObservedPayload`; mapped inference resource
    observations through embedded-runtime diagnostic projection; and updated
    diagnostics validation for source bounds.
  - Focused tests added/updated: lifecycle serde coverage for resource
    observation, diagnostics-ledger payload-bound validation, and
    embedded-runtime projection coverage proving resource observations alone
    keep lifecycle diagnostics persistable.
  - Verification passed: `cargo test -p inference lifecycle --lib`, `cargo
    test -p pantograph-diagnostics-ledger
    diagnostic_event_ledger_validates_inference_execution_diagnostic_scope_and_bounds
    --lib`, `cargo test -p pantograph-embedded-runtime
    inference_diagnostic_event_adapter_persists_resource_observation_without_other_diagnostics
    --lib`, `cargo test -p pantograph-embedded-runtime inference_diagnostic
    --lib`, `cargo check -p pantograph-embedded-runtime`, `cargo fmt --all
    -- --check`, and `git diff --check`.
  - Remaining follow-up: the next slice is the resource-monitor
    factory/modules with the `sysinfo` process-RSS first implementation.
- 2026-05-18 diagnostics-ledger run resource rollup slice:
  - Smallest useful vertical slice: add a diagnostics-ledger query that rolls
    persisted typed inference resource observations for one workflow run into
    the existing compact `RunResourceObservation` DTO, without changing
    workflow-service terminal emission yet.
  - Allowed write set: `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/lib.rs`,
    `crates/pantograph-diagnostics-ledger/src/repository.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite/run_resource_observation_sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: the rollup reads only typed
    `InferenceExecutionDiagnosticObservedPayload.resource_observation` values.
    It does not derive memory from error text, scheduler state, runtime names,
    or unavailable metric diagnostics. Availability-only observations remain
    detailed inference diagnostics and are not converted into fake terminal
    bytes.
  - Implementation completed: added `RunResourceObservationRollupQuery`,
    exposed `DiagnosticsLedgerRepository::run_resource_observation_rollup`,
    implemented the SQLite query over persisted inference diagnostic events,
    and added tests for max RAM/max VRAM/OOM rollup, missing observations, and
    availability-only observations.
  - Verification passed: `cargo test -p pantograph-diagnostics-ledger
    run_resource_observation_rollup --lib`, `cargo check -p
    pantograph-diagnostics-ledger`, and `cargo fmt --all -- --check`.
  - Remaining follow-up: workflow-service still needs to call the rollup while
    appending its single owned `RunTerminal` event.
- 2026-05-18 workflow-service terminal resource observation slice:
  - Smallest useful vertical slice: wire workflow-service terminal event
    emission to read the diagnostics-ledger run resource rollup and store it in
    `RunTerminalPayload.resource_observation`.
  - Allowed write set: `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: workflow-service remains the only
    producer of terminal run-completion events. It does not inspect inference
    diagnostics or compute resource facts itself; diagnostics-ledger owns the
    rollup, and embedded-runtime remains limited to detailed inference
    diagnostic persistence.
  - Implementation completed: `record_run_terminal_event_if_configured` now
    calls `run_resource_observation_rollup` before appending `RunTerminal`.
    The focused test proves the terminal payload includes peak RAM, peak VRAM,
    and OOM from the typed rollup.
  - Verification passed: `cargo test -p pantograph-workflow-service
    run_terminal_event_includes_diagnostics_ledger_resource_rollup --lib`,
    `cargo check -p pantograph-workflow-service`, and `cargo fmt --all --
    --check`.
  - Remaining follow-up: item 8 is complete; the next plan item is backend
    producer telemetry.
- 2026-05-18 PyTorch image CUDA resource producer slice:
  - Smallest useful vertical slice: add a PyTorch worker producer for planned
    image-generation CUDA peak VRAM telemetry and verify it through the
    existing Python worker image harness.
  - Allowed write set: `crates/inference/torch/worker.py`,
    `crates/inference/src/backend/pytorch_worker_image_python_tests.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: the worker uses PyTorch CUDA memory
    APIs only and does not infer memory from error strings, scheduler metadata,
    or runtime names. CPU/process RSS remains owned by the shared monitor path,
    not this PyTorch CUDA producer.
  - Implementation completed: image generation resets CUDA peak memory stats
    before the planned load/generate operation and attaches a typed
    `peak_vram_bytes` observation with `pytorch_cuda` source facts to the
    worker response when CUDA reports a positive peak allocation.
  - Verification passed: `cargo test -p inference --features backend-pytorch
    test_python_worker_generate_image_from_envelope_reports_cuda_peak_vram
    --lib`, `cargo test -p inference --features backend-pytorch
    pytorch_worker_image_python --lib`, `cargo check -p inference --features
    backend-pytorch`, and `cargo fmt --all -- --check`.
  - Remaining follow-up: item 9 still needs MPS availability/metric behavior,
    shared process RSS producer wiring, managed runtime structured telemetry,
    and failure-path OOM typing.
- 2026-05-18 PyTorch image MPS resource availability slice:
  - Smallest useful vertical slice: verify the existing PyTorch image worker
    MPS producer emits typed metric availability when MPS is available but no
    canonical peak VRAM counter exists.
  - Allowed write set: `crates/inference/src/backend/pytorch_worker_image_python_tests.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: MPS does not report fake
    `peak_vram_bytes`, zero-byte values, or scheduler-derived estimates. The
    producer emits typed `not_implemented` availability for the MPS source.
  - Implementation completed: added Python worker image harness coverage for
    an MPS-planned image request and asserted a `pytorch_mps` availability
    fact with `not_implemented` state and no peak VRAM value.
  - Verification passed: `cargo test -p inference --features backend-pytorch
    test_python_worker_generate_image_from_envelope_reports_mps_metric_unimplemented
    --lib`.
  - Remaining follow-up: item 9 still needs shared process RSS producer
    wiring, managed runtime structured telemetry, real MPS metric support if a
    canonical PyTorch counter becomes available, and failure-path OOM typing.
- 2026-05-18 planned image process-RSS lifecycle producer slice:
  - Smallest useful vertical slice: wire the existing neutral
    `resource_monitor` process-RSS producer into planned image-generation
    backend execution lifecycle events without changing task result payloads.
  - Allowed write set: `crates/inference/src/gateway.rs`,
    `crates/inference/src/gateway_tests.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: the producer uses the canonical
    `RuntimeResourceMonitor` API and records only typed host RAM observations
    sourced as `os_process_rss`. It does not infer VRAM, write telemetry into
    image outputs, or add platform-specific probes outside the monitor module.
  - Implementation completed: planned image-generation lifecycle execution
    now starts the default runtime resource monitor immediately before the
    backend execution call, finishes it immediately after the call, and
    attaches the observation to the backend execution completed/failed event.
    Task-validation and cleanup events remain free of resource observations.
  - Verification passed: `cargo test -p inference
    test_generate_image_from_planning_input_with_lifecycle_records_planned_decision
    --lib` and `cargo test -p inference image_generation --lib`.
  - Remaining follow-up: item 9 still needs shared process-RSS lifecycle
    wiring for the other runtime execution families, a lifecycle path for
    backend-native worker observations, managed runtime structured telemetry,
    real MPS metrics if a canonical PyTorch counter becomes available, and
    failure-path OOM typing.
- 2026-05-18 generic typed process-RSS lifecycle producer slice:
  - Smallest useful vertical slice: extend the process-RSS producer from the
    planned image lifecycle path to the generic typed non-streaming backend
    execution lifecycle path without changing task outputs.
  - Allowed write set: `crates/inference/src/gateway.rs`,
    `crates/inference/src/gateway_tests.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: the slice reuses the canonical
    `RuntimeResourceMonitor` API, records only typed `os_process_rss` host RAM
    observations on backend execution terminal lifecycle events, and does not
    add scheduler policy, task-output metadata, or string-derived memory facts.
  - Implementation completed: `execute_typed_with_lifecycle` starts the
    monitor immediately before `execute_typed_validated`, finishes it
    immediately afterward, and records the observation through the typed
    backend execution result event. Validation, preprocessing, postprocessing,
    result projection, cleanup, and streaming paths remain unchanged.
  - Verification passed: `cargo test -p inference
    test_execute_typed_text_reports_generation_option_diagnostics --lib` and
    `cargo test -p inference lifecycle --lib`.
  - Remaining follow-up: item 9 still needs a lifecycle path for
    backend-native worker observations, streaming/runtime process-RSS
    monitoring in a dedicated slice, managed runtime structured telemetry,
    real MPS metrics if a canonical PyTorch counter becomes available, and
    failure-path OOM typing.
- 2026-05-18 streaming process-RSS lifecycle producer slice:
  - Smallest useful vertical slice: extend the existing stream lifecycle
    wrapper to own the canonical process-RSS monitor for stream completion,
    stream failure, and dropped-stream cancellation.
  - Allowed write set: `crates/inference/src/gateway.rs`,
    `crates/inference/src/gateway_tests.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    and this execution log. Unrelated root proposal markdown files remain
    ignored.
  - No-fallback/no-legacy confirmation: the stream producer records only typed
    `os_process_rss` host RAM observations on terminal lifecycle events. It
    does not copy telemetry into stream chunks, generated text, task outputs,
    or scheduler policy.
  - Implementation completed: `LifecycleStream` now starts the default
    monitor when the lifecycle stream wrapper is created, finishes it exactly
    once in the stream terminal path, and records the observation on completed,
    failed, or cancelled backend execution events before cleanup.
  - Verification passed: `cargo test -p inference
    test_chat_completion_stream_with_lifecycle_records_completion --lib`,
    `cargo test -p inference
    test_stream_typed_text_with_lifecycle_records_terminal_chunk_usage --lib`,
    and `cargo test -p inference lifecycle --lib`.
  - Remaining follow-up: item 9 still needs a lifecycle path for
    backend-native worker observations, managed-runtime process-boundary
    resource monitoring where the target process is not the Pantograph
    process, managed runtime structured telemetry, real MPS metrics if a
    canonical PyTorch counter becomes available, and failure-path OOM typing.
- 2026-05-20 release validation and Tauri startup contract slice:
  - Smallest useful vertical slice: complete Milestone 8 automated release
    validation and fix only the stale Tauri release/test contract sites that
    blocked the release gate.
  - Allowed write set: `src-tauri/src/llm/startup.rs`,
    `src-tauri/src/llm/commands/server.rs`,
    `src-tauri/src/llm/commands/rag.rs`,
    `src-tauri/src/workflow/workflow_execution_runtime.rs`,
    stale Tauri test fixtures using `RuntimeLifecycleSnapshot`,
    `WorkflowRuntimeRequirements`, or managed-runtime job/selection DTOs,
    `docs/plans/current-image-generation-graphs/milestones/08-release-build-and-user-validation.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: the startup path now translates
    app-configured llama.cpp selectors through
    `BackendStartupDeviceIntent::llama_cpp_selector` and returns an error for
    invalid selectors such as canonical scheduler ids instead of coercing or
    guessing. The scheduler-selected canonical-device path remains separate
    from backend-local llama.cpp startup selectors.
  - Implementation completed: Tauri startup request builders now return
    `Result`, command call sites propagate invalid startup device diagnostics,
    edit-session embedding runtime startup propagates the same typed error,
    and diagnostics lifecycle snapshot projection initializes newly added
    timing fields through the canonical `RuntimeLifecycleSnapshot::default`.
    Tauri unit fixtures were updated to the current runtime lifecycle,
    workflow requirement, and managed-runtime variant DTO shapes.
  - Discovered issue fixed in-slice: the first release build failed because
    `src-tauri/src/llm/startup.rs` still passed raw device strings into the
    typed backend startup contract, and
    `src-tauri/src/workflow/diagnostics/types.rs` still constructed
    `RuntimeLifecycleSnapshot` without the warmup timing fields. A focused
    Tauri test compile then exposed stale test-only DTO fixtures, which were
    updated rather than preserving legacy fields.
  - Verification passed: `cargo test -p pantograph startup`,
    `cargo check -p pantograph`, `cargo fmt --package pantograph -- --check`,
    `bash launcher.sh --build-release`, and
    `bash launcher.sh --release-smoke`.
  - Earlier Milestone 8 verification already passed before this slice:
    `cargo test -p pantograph-diagnostics-ledger
    runtime_selection_history --lib`, `npm run typecheck`,
    `npm run test:frontend`, `npm run build`,
    `cargo check -p inference --no-default-features`, and
    `cargo check -p inference --all-features`.
  - Standards and boundary notes: no new dependencies or lockfile changes were
    introduced; no frontend code changed in this slice; no new production
    `unwrap()`/`expect()` paths were added; no platform-specific `cfg` or path
    handling was added; no generated, build-output, sqlite WAL/SHM, or workflow
    fixture files were dirtied. Release validation still reports pre-existing
    Tauri dead-code warnings and a Tauri identifier warning because
    `com.pantograph.app` ends with `.app`.
  - Remaining follow-up: `bash launcher.sh --release-smoke` verifies the
    release artifact and managed-runtime contracts, but it explicitly reports
    that Pantograph does not yet expose a headless desktop release-smoke
    entrypoint. Manual desktop validation still needs to confirm Juggernaut
    graph visibility, Pumas model resolution, stale diagnostic behavior, and
    image output artifact retention.
- 2026-05-20 headless current image release-smoke slice:
  - Smallest useful vertical slice: extend the existing release smoke so it
    validates the current image workflow graph path through bounded headless
    contracts instead of requiring a desktop GUI session.
  - Allowed write set: `scripts/check-runtime-redistributables-smoke.sh`,
    `scripts/check-current-image-workflow-smoke.mjs`, `scripts/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/08-release-build-and-user-validation.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: the smoke validates canonical
    `puma-lib -> llm-inference -> image-output` graph shape for both the
    tracked Juggernaut workflow and the bundled current image template. It
    rejects retired executable inference node types and does not add any
    compatibility shim, runtime fallback, or frontend inference path.
  - Implementation completed: added a Node smoke validator for the tracked
    Juggernaut graph and bundled current image template, then extended
    `bash launcher.sh --release-smoke` to run that validator plus focused
    Rust checks for node inventory visibility, Pumas model resolution, stale
    graph diagnostics, and image artifact retention.
  - Verification passed: `node scripts/check-current-image-workflow-smoke.mjs`,
    `cargo test -p workflow-nodes test_inventory_collects_all_builtins --lib`,
    `cargo test -p pantograph-embedded-runtime
    puma_lib_execution_rebinds_stale_model_path_from_selector_access_without_pumas_api
    --lib`, `cargo test -p pantograph-workflow-service
    inspection_projection_returns_stable_stale_graph_diagnostics --lib`,
    `cargo test -p pantograph-workflow-service
    workflow_io_artifact_query_reads_refreshed_projection --lib`,
    `bash launcher.sh --release-smoke`, and `git diff --check`.
  - Discovered issue deferred: the previous release smoke had an exact
    embedded-runtime preflight test filter that executed zero tests. Running
    the intended tests directly showed
    `workflow_preflight_blocks_selected_runtime_failed_after_restart` and
    `workflow_preflight_blocks_interrupted_runtime_job_after_restart` now use a
    stale helper that writes incidental `backend_key` metadata onto a
    `text-input` node, while canonical runtime requirements come only from
    typed inference `runtime` input. A quick helper canonicalization proved the
    required-backend assertion, but the tests still need a dedicated slice to
    update their expected readiness/preflight path without reintroducing legacy
    metadata behavior.
    Resolved by the 2026-05-20 canonical runtime-preflight ordering slice
    below.
  - Standards and boundary notes: no dependencies, lockfiles, workflow
    fixtures, generated files, sqlite WAL/SHM files, or production runtime
    paths changed. The new smoke script is an internal developer validation
    tool documented in `scripts/README.md`; it reads checked-in workflow JSON
    only and exits with typed, bounded diagnostic messages.
  - Remaining follow-up: a real desktop/model execution validation remains a
    manual/user-environment activity. The automated plan gate now has
    deterministic release-smoke coverage for the graph and contract boundaries
    named in Milestone 8.
- 2026-05-20 runtime-selection memory/OOM history ranking slice:
  - Smallest useful vertical slice: make the existing history-backed
    runtime-selection policy consume the typed memory/OOM fields already
    projected through diagnostics-ledger and embedded-runtime candidate history
    summaries.
  - Allowed write set: `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `docs/plans/current-image-generation-graphs/02-image-generation-family-planner.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: the policy uses only typed
    `RuntimeTechnicalFitCandidateHistorySummary` fields after the existing
    all-eligible-candidates threshold is met. It does not parse diagnostics
    strings, inspect task outputs, infer OOM from errors, or activate partial
    history when any eligible candidate lacks the required sample count.
  - Implementation completed: history-backed candidate ordering now compares
    terminal failure rate, OOM rate, average/median duration, queue wait, peak
    VRAM, and peak RAM in that order. Tests cover preferring a slower runtime
    with lower OOM rate and using peak VRAM as a deterministic tie-breaker.
  - Verification passed: `cargo test -p pantograph-runtime-registry history
    --lib`, `cargo check -p pantograph-runtime-registry`, and `cargo fmt
    --package pantograph-runtime-registry -- --check`.
  - Standards and boundary notes: no public DTO shape, dependency, lockfile,
    persisted artifact, generated file, sqlite WAL/SHM, frontend, or runtime
    process lifecycle changed. The scheduler policy remains isolated in
    `pantograph-runtime-registry`, so later algorithm changes stay local to
    the policy module and its tests.
  - Remaining follow-up: the policy can later evolve to resource-pressure-aware
    weighting and richer multi-device/multi-runtime observations, but the
    current no-fallback history gate is now active for the typed timing,
    memory, and OOM facts available today.
- 2026-05-20 canonical runtime-preflight ordering slice:
  - Smallest useful vertical slice: replace the stale embedded-runtime
    required-backend test helper with canonical `llm-inference.data.runtime`
    input and make workflow preflight ignore irrelevant technical-fit blockers
    unless a workflow/model/explicit selection actually requires runtime
    readiness.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/lib_tests.rs`,
    `crates/pantograph-embedded-runtime/src/lib_tests/runtime_preflight_tests.rs`,
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/workflow_preflight.rs`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: the test helper no longer writes
    incidental `backend_key` metadata onto `text-input` nodes. Runtime
    requirements are derived from the typed inference `runtime` input, and
    unavailable explicit/required runtimes still fail with typed
    `WorkflowRuntimeIssue` diagnostics instead of selecting another runtime or
    coercing the request.
  - Implementation completed: `rewrite_test_workflow_required_backend` now
    builds `text-input -> llm-inference -> text-output` with `runtime` on the
    inference node. Workflow-service runtime preflight now first checks whether
    runtime readiness should be enforced, then prefers concrete runtime
    capability readiness diagnostics before falling back to technical-fit
    blocking diagnostics. Non-inference workflows with no runtime requirement
    keep their technical-fit decision for visibility but do not block on an
    irrelevant candidate diagnostic.
  - Verification passed: `cargo test -p pantograph-workflow-service
    workflow_preflight --lib`, `cargo test -p pantograph-embedded-runtime
    runtime_preflight --lib`, `cargo check -p pantograph-workflow-service`,
    `cargo check -p pantograph-embedded-runtime`, `cargo fmt --package
    pantograph-workflow-service --package pantograph-embedded-runtime --
    --check`, and `git diff --check`.
  - Discovered issue fixed in-slice: the preflight service previously
    evaluated technical-fit blocking diagnostics before checking whether
    runtime readiness was required. That made a text-only workflow fail because
    an unrelated Candle candidate reported unavailable executable loading, and
    it obscured concrete managed-runtime readiness reasons such as validation
    failure or interrupted-job reconciliation.
  - Remaining follow-up: none for this slice. Broader technical-fit and device
    milestone items remain tracked in their milestone sections.
- 2026-05-20 Pumas P6 fixture-alignment closeout slice:
  - Smallest useful vertical slice: close the remaining cross-repo
    fixture-alignment checklist rows by verifying Pantograph's planner and
    public model-contract tests consume the checked-in Pumas Diffusers fixture
    without local patching, name guessing, or Pantograph-specific producer
    fields.
  - Allowed write set:
    `docs/plans/current-image-generation-graphs/07-pumas-library-image-generation-facts.md`
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: this slice added no compatibility
    shim, source-derived family guess, or backend fallback. The existing
    planner test proves missing/ambiguous Pumas facts reject with typed
    diagnostics, and the existing public contract test proves the fixture
    remains factual rather than consumer-policy-shaped.
  - Implementation completed: marked the remaining P6 checklist rows complete
    and recorded the current `pumas-library` pin
    `8444b50df28c3e2bd8db58fb3645fa4dd8664b27` as the verified fixture
    alignment boundary.
  - Verification passed: `cargo test -p inference image_generation_planner
    --lib`, `cargo test -p inference --test model_contracts
    pumas_image_generation_fixture_decodes_with_structured_diffusers_facts`,
    and `git diff --check`.
  - Verification deviation: the two Cargo commands briefly waited on package
    and build locks, then completed successfully.
  - Remaining follow-up: none for P6.
- 2026-05-20 stale graph Pumas model-ref diagnostic slice:
  - Smallest useful vertical slice: close the remaining Milestone 3 backend
    stale-graph diagnostic gap for graph-authored Pumas model references by
    validating only structural model-ref shape during graph contract
    inspection.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/graph/diagnostics.rs`,
    `crates/pantograph-workflow-service/src/graph/contract_validation.rs`,
    `crates/pantograph-workflow-service/src/graph/contract_validation_tests.rs`,
    `docs/plans/current-image-generation-graphs/milestones/03-backend-stale-graph-diagnostics.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: graph validation does not call Pumas,
    infer model availability, repair local paths, or translate invalid model
    refs into fallback model ids. It emits a typed
    `InvalidPumasModelReference` stale graph diagnostic when graph data already
    contains malformed/local/unsupported Pumas model-ref payloads.
  - Implementation completed: added the diagnostic code, top-level
    `pumas_model_ref`/`model_ref` payload validation for known graph nodes,
    and focused tests for a local-path Pumas ref, invalid object shape, and a
    valid Pumas ref that stays non-stale.
  - Verification passed: `cargo test -p pantograph-workflow-service
    graph::contract_validation --lib`, `cargo check -p
    pantograph-workflow-service`, `cargo fmt --package
    pantograph-workflow-service -- --check`, and `git diff --check`.
  - Remaining follow-up: none for Milestone 3. Live Pumas model availability
    remains owned by Pumas resolution and execution planning diagnostics.
- 2026-05-20 saved-graph focus preservation slice:
  - Smallest useful vertical slice: complete the remaining Milestone 4
    keyboard-focus row for saved-graph stale details using the existing Node
    presenter test strategy.
  - Allowed write set:
    `src/components/workbench/graphInspectionPresenters.ts`,
    `src/components/workbench/graphInspectionPresenters.test.ts`,
    `src/components/workbench/SavedGraphInspectionSnapshot.svelte`,
    `docs/plans/current-image-generation-graphs/milestones/04-io-inspector-stale-graph-presentation.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: the slice does not add polling,
    subscriptions, a new test platform, manual graph mutation, or
    frontend-fabricated stale diagnostics. It keeps backend-owned graph facts
    and frontend-only transient focus/selection state separate.
  - Implementation completed: added stable presenter-owned saved-graph node
    focus ids, returns focus to the selected graph node after the stale-details
    panel updates, and records that no push/subscription helper is required
    because this milestone pass introduced no push/subscription path.
  - Verification passed: `node --experimental-strip-types --test
    src/components/workbench/graphInspectionPresenters.test.ts`, `npm run
    typecheck`, and `git diff --check`.
  - Remaining follow-up: none for Milestone 4. Browser-mounted focus
    regression tests remain deferred until the repository adopts a DOM-capable
    frontend test strategy.
- 2026-05-20 Milestone 5 path-derived Pumas model inference removal slice:
  - Smallest useful vertical slice: remove workflow-service capability
    extraction of model ids from Pumas-looking `model_path`, `entry_path`, and
    `selected_artifact_path` values while preserving explicit
    `model_id`/`pumas_model_ref.model_id` discovery.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/capabilities.rs`,
    `crates/pantograph-workflow-service/src/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: the slice deletes legacy path-derived
    model inference and adds no compatibility alias, graph migration, Pumas path
    join, backend fallback, or scheduler bypass. Pumas remains the authority for
    path-to-model interpretation and artifact load-target resolution.
  - Implementation completed: removed the path parser helpers, updated
    capability tests to prove path-shaped fields are ignored without explicit
    model identity, added the positive `pumas_model_ref.model_id` test, and
    documented the workflow-service invariant.
  - Verification passed: `cargo fmt --all -- --check`, `cargo test -p
    pantograph-workflow-service --lib capabilities`, `cargo check -p
    pantograph-workflow-service`, and `git diff --check -- crates/pantograph-workflow-service/src/capabilities.rs crates/pantograph-workflow-service/src/README.md`.
  - Verification deviation: `cargo test -p pantograph-workflow-service
    capabilities` failed before exercising this slice because unrelated
    integration-test initializers in
    `crates/pantograph-workflow-service/tests/contract.rs` are missing
    `memory_failure_kind`, `observed_peak_ram_bytes`, and
    `observed_peak_vram_bytes`. The focused library test and crate check passed.
  - Remaining follow-up: non-image tracked workflow examples and formal
    workflow schema migration ownership remain open Milestone 5 workflow/fixture
    closure work.
- 2026-05-20 Milestone 5 workflow diagnostics contract memory-field compile
  unblock slice:
  - Smallest useful vertical slice: fix the two workflow-service contract
    snapshots that construct diagnostics-ledger run projection records without
    the current observed memory fields.
  - Allowed write set:
    `crates/pantograph-workflow-service/tests/contract.rs`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: this test-only slice adds explicit
    nullable memory observation fields to the contract snapshots. It does not
    add defaults, compatibility shims, projection migrations, runtime behavior,
    or scheduler fallback.
  - Implementation completed: added `observed_peak_ram_bytes`,
    `observed_peak_vram_bytes`, and `memory_failure_kind` to the run-list and
    run-detail projection record initializers plus their expected JSON
    snapshots.
  - Verification passed: `cargo fmt --all -- --check`, `cargo test -p
    pantograph-workflow-service workflow_run_`, `cargo test -p
    pantograph-workflow-service capabilities`, `cargo check -p
    pantograph-workflow-service`, and `git diff --check --
    crates/pantograph-workflow-service/tests/contract.rs`.
  - Verification deviation: the first focused command attempted two Cargo test
    filters at once and failed before tests ran. The valid broader
    `workflow_run_` filter was rerun successfully.
  - Remaining follow-up: none for this compile blocker.
- 2026-05-20 Milestone 5 graph edge-insert retired model fact priority removal
  slice:
  - Smallest useful vertical slice: stop edge-insert helper priority from
    preferring retired `resolved_model_source` and
    `resolved_model_package_facts` ports as model-reference targets.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/graph/connection_insert.rs`,
    `crates/pantograph-workflow-service/src/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: this removes retired handle names from
    graph helper ordering and adds no alias, migration, package-fact port
    recreation, scheduler bypass, or compatibility route. Canonical priority
    remains limited to explicit model identity ports.
  - Implementation completed: removed the retired ports from
    `edge_insert_input_priority`, added a private unit test for canonical and
    retired port ordering, and documented that graph helpers must not treat
    retired package/source fields as current model signals.
  - Verification passed: `cargo fmt --all -- --check`, `cargo test -p
    pantograph-workflow-service graph::connection_intent`, `cargo check -p
    pantograph-workflow-service`, and `git diff --check`.
  - Verification deviation: the first format check failed and `cargo fmt
    --all` was run; verification was rerun successfully.
  - Remaining follow-up: KV-cache memory-impact still tracks
    `resolved_model_source` as a graph change signal and needs a separate
    focused semantic slice.
- 2026-05-20 Milestone 5 KV-cache memory-impact retired source signal removal
  slice:
  - Smallest useful vertical slice: stop KV-cache memory-impact classification
    from treating `resolved_model_source` changes on `llm-inference` nodes as
    model identity changes.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/graph/memory_impact.rs`,
    `crates/pantograph-workflow-service/src/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: this removes one retired graph signal
    from memory-impact semantics and adds no alias, package-fact inference,
    compatibility migration, scheduler bypass, or runtime fallback.
  - Implementation completed: removed `resolved_model_source` from the
    model-identity tracked fields and added a focused test proving that changes
    to the retired field fall through to tokenizer/config refresh behavior.
  - Verification passed: `cargo fmt --all -- --check`, `cargo test -p
    pantograph-workflow-service graph::memory_impact`, `cargo check -p
    pantograph-workflow-service`, and `git diff --check`.
  - Verification deviation: the first format check failed and `cargo fmt
    --all` was run; verification was rerun successfully.
  - Remaining follow-up: `model_path` still participates in memory-impact
    model-change detection and needs a separate schema/legacy saved graph
    ownership decision before removal or scoping.
- 2026-05-20 Milestone 5 legacy-removal closeout planning slice:
  - Smallest useful vertical slice: update the remaining Milestone 5 closeout
    plan so legacy graph/device/runtime/model-path behavior is planned as
    removal or typed stale diagnostics, not compatibility.
  - Allowed write set:
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: the plan now forbids runtime planning
    migrations for old graph shapes, classifies raw local model paths as
    non-canonical model identity, requires runtime/device graph values to flow
    only as typed scheduler intent, and requires tests mentioning retired
    fields to be negative coverage or removed.
  - Implementation completed: added the Milestone 5 legacy removal contract,
    expanded the contract inventory with explicit retired-field classifications,
    and tightened workflow/fixture closure around `model_path`, non-image
    examples, and stale-shape fixtures.
  - Verification passed: documentation-only review with `git diff --check --
    docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md
    docs/plans/current-image-generation-graphs/05-execution-management.md`.
  - Remaining follow-up: implement the next code slice from the updated
    closeout order, starting with contract inventory or the narrowest remaining
    `model_path` removal/rejection boundary.
- 2026-05-20 Milestone 5 post-plan codebase investigation update:
  - Smallest useful vertical slice: record the read-only investigation of the
    legacy-removal closeout plan's actual blast radius across `workflow-nodes`,
    workflow-service graph semantics, node-engine typed inference builders,
    tracked workflow fixtures, frontend/Tauri device config, runtime recovery,
    embedding restart, and managed-runtime projection.
  - Allowed write set:
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: this planning update does not preserve
    old behavior. It records `puma-lib` path production, node-engine
    retired-fact intake, non-image workflow fixture drift, and
    fallback-shaped runtime/device paths as constraints to remove, replace,
    or scope before related checklist rows can close.
  - Investigation completed: verified that bundled templates and tracked image
    examples are canonical, while tracked non-image workflow examples still
    carry retired direct inference nodes and `model_path`/`backend_key`
    wiring; verified that `puma-lib` still registers model options on
    `model_path`; verified that node-engine still accepts retired model fact
    inputs; verified that `DeviceConfig` remains generic-looking at the
    frontend/Tauri boundary.
  - Verification passed: documentation-only review with `git diff --check --
    docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md
    docs/plans/current-image-generation-graphs/05-execution-management.md`.
  - Remaining follow-up: implement the `puma-lib` contract replacement first,
    then remove node-engine retired fact intake and reconcile non-image tracked
    workflows as canonical examples or stale diagnostic fixtures.
- 2026-05-20 Milestone 5 standards-iteration update:
  - Smallest useful vertical slice: re-iterate the Milestone 5 closeout plan
    against the repository standards for planning, architecture, testing,
    concurrency, frontend ownership, interop, security, Rust API/async/path
    safety, and cross-platform behavior.
  - Allowed write set:
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`
    and this execution log. Root proposal markdown files remained ignored.
  - Standards result: the existing gates already covered typed boundaries,
    sync-core/async-shell design, lifecycle ownership, allowed-root validation,
    checked arithmetic, frontend backend-owned state, Node frontend tests,
    interop fixtures, README traceability, and platform-module isolation.
    The missing sequencing detail was that `puma-lib` path-producer replacement
    was recorded as a finding but not promoted into the closeout order.
  - Plan update completed: added a dedicated Graph Model Selection Contract
    Replacement closeout step before raw device removal, with allowed write
    areas, no-legacy constraints, typed Pumas selector option requirements, and
    acceptance tests proving `model_path`/`backend_key` are not executable
    graph outputs.
  - No-fallback/no-legacy confirmation: the update does not preserve old graph
    behavior. It makes the path-producing `puma-lib` contract the next serial
    replacement boundary and prevents later scheduler/node-engine slices from
    treating raw paths or backend keys as accepted compatibility aliases.
- 2026-05-20 Milestone 5 `puma-lib` contract replacement slice:
  - Smallest useful vertical slice: replace the `puma-lib` workflow-node
    descriptor and model selector options contract so graph selection uses
    `pumas_model_ref` rather than executable `model_path` or graph-visible
    `backend_key` values.
  - Allowed write set: `crates/workflow-nodes/src/input/puma_lib.rs`,
    `crates/workflow-nodes/src/input/README.md`,
    `crates/workflow-nodes/src/contracts.rs`, this milestone plan, and this
    execution log. Node-engine execution, scheduler policy, saved workflows,
    generated DTOs, lockfiles, and root proposal markdown files were not
    touched.
  - No-fallback/no-legacy confirmation: descriptor outputs removed
    `model_path` and `backend_key`; the provider is registered on
    `pumas_model_ref`; option values are typed Pumas model-reference payloads;
    non-ready selector rows become typed disabled options; display paths remain
    display metadata only.
  - Implementation completed: updated workflow-node descriptor metadata,
    provider registration, option value projection, disabled/unavailable state
    mapping, contract tests, and input-node README contract text.
  - Verification passed: `cargo fmt --all -- --check`, `cargo test -p
    workflow-nodes puma_lib --features model-library`, and `cargo test -p
    workflow-nodes builtin_contracts_preserve_registered_port_options_provider_refs
    --features model-library`.
  - Deviations: real Pumas selector snapshots may provide selected artifact
    fields outside the nested `PumasModelRef`; this slice intentionally did not
    copy those parallel fields into the option value, keeping `PumasModelRef`
    authoritative.
  - Remaining follow-up: remove node-engine retired model/fact intake and
    reconcile non-image tracked workflow examples in their own slices.
- 2026-05-20 Milestone 5 node-engine dependency-input retired context cleanup
  slice:
  - Smallest useful vertical slice: stop node-engine dependency-input assembly
    from implicitly propagating executable model paths, Pumas load targets, and
    resolved dependency facts as canonical model-reference context, and reject
    path-shaped values targeting `pumas_model_ref`.
  - Allowed write set:
    `crates/node-engine/src/engine/dependency_inputs.rs`,
    `crates/node-engine/src/engine/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: this removes a legacy dependency-input
    repair path and adds no compatibility alias, fallback resolver, Pumas path
    join, package-fact revival, or scheduler bypass. `pumas_model_ref` now
    requires explicit object-shaped model-reference intent in this assembly
    path.
  - Implementation completed: reduced implicit model context to
    `pumas_model_ref`, model/task facts, selected binding ids, and bounded
    platform context; blocked non-object direct values for `pumas_model_ref`;
    updated focused unit tests and the node-engine engine README.
  - Verification passed: `cargo fmt -p node-engine`, `cargo test -p
    node-engine dependency_inputs --lib`, and `cargo check -p node-engine`.
  - Remaining follow-up: direct explicit `model_path` graph edges still exist
    for non-canonical/legacy consumers and must be removed, rejected, or scoped
    in the later `ModelDependencyRequest`/`ModelRefV2` contract replacement
    slice rather than treated as accepted compatibility.
- 2026-05-20 Milestone 5 embedded-runtime `puma-lib` execution path-output
  removal slice:
  - Smallest useful vertical slice: align embedded-runtime `puma-lib` host
    execution with the new graph-facing `pumas_model_ref` contract by removing
    stale path rebinding, path-derived model-id inference, top-level
    `model_path`, graph-visible `backend_key`, hidden package facts, and hidden
    artifact load-target outputs from the task executor.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/task_executor/puma_lib.rs`,
    `crates/pantograph-embedded-runtime/src/task_executor_tests/puma_lib.rs`,
    `crates/pantograph-embedded-runtime/src/model_dependencies_tests.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: the executor now emits official
    Pumas model references and bounded selector facts only. The slice removed
    path repair, path-shaped model-ref alias creation, embedded package-fact
    revival, and artifact-load-target shortcuts instead of preserving them as
    compatibility behavior.
  - Implementation completed: `execute_puma_lib` now reads
    `pumas_model_ref`/`model_id`, hydrates selector detail without producing
    executable paths, leaves selected-artifact path authority inside official
    Pumas model refs, and forwards only existing bounded planning metadata.
    Focused execution tests and the model-library options/resolver test now
    assert the retired outputs stay absent.
  - Verification passed: `cargo fmt -p pantograph-embedded-runtime`, `cargo
    fmt -p pantograph-embedded-runtime -- --check`, `cargo test -p
    pantograph-embedded-runtime puma_lib`, and `cargo check -p
    pantograph-embedded-runtime`.
  - Verification deviation: the initial command `cargo test -p
    pantograph-embedded-runtime puma_lib --features model-library` failed
    because `model-library` is a `workflow-nodes` feature. The corrected
    command passed.
  - Remaining follow-up: replace the explicit
    `ModelDependencyRequest.model_path` dependency-preflight contract,
    dependency cache/correlation fields, and remaining direct explicit
    `model_path` graph edges with the planned typed Pumas model-reference
    request/result contract.
- 2026-05-20 Milestone 5 node-engine canonical preflight Pumas identity gate
  slice:
  - Smallest useful vertical slice: make canonical node-engine dependency
    preflight build host resolver requests from explicit `pumas_model_ref` or
    `model_id` instead of graph-facing `model_path`, and fail closed when
    neither Pumas identity is present.
  - Allowed write set:
    `crates/node-engine/src/core_executor/dependency_preflight.rs`,
    `crates/node-engine/src/core_executor/inference_tests.rs`,
    `crates/node-engine/src/README.md`,
    `crates/node-engine/src/core_executor/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: this slice removes successful
    path-only canonical preflight identity and package-fact model-id fallback.
    It does not add a compatibility builder, path alias, package-fact identity
    fallback, directory scan, or scheduler bypass.
  - Implementation completed: `build_model_dependency_request` now reads
    explicit Pumas identity, leaves `model_path` empty, and canonical preflight
    blocks with a typed execution error when Pumas identity is absent. Failure
    payloads report `model_id` instead of stale local paths. Focused tests and
    README traceability were updated.
  - Verification passed: `cargo fmt -p node-engine`, `cargo fmt -p
    node-engine -- --check`, `cargo test -p node-engine dependency_preflight
    --features inference-nodes,pytorch-nodes --lib`, `cargo test -p
    node-engine build_model_dependency_request --features
    inference-nodes,pytorch-nodes --lib`, and `cargo check -p node-engine
    --features inference-nodes,pytorch-nodes`.
  - Remaining follow-up: replace the exported path-shaped
    `ModelDependencyRequest`/`ModelRefV2` contracts, embedded-runtime
    dependency-preflight construction, dependency cache/activity correlation,
    and any direct explicit `model_path` graph edges with typed Pumas
    model-reference request/result contracts.
- 2026-05-20 Milestone 5 embedded-runtime dependency-preflight Pumas identity
  gate slice:
  - Smallest useful vertical slice: align embedded-runtime Python dependency
    preflight and explicit `dependency-environment` execution with the
    node-engine gate by building resolver requests from explicit
    `pumas_model_ref` or `model_id`, leaving `model_path` empty, and failing
    closed when Pumas identity is missing.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment.rs`,
    `crates/pantograph-embedded-runtime/src/task_executor_tests/dependency_preflight.rs`,
    `crates/pantograph-embedded-runtime/src/task_executor_tests/dependency_fail_closed.rs`,
    `crates/pantograph-embedded-runtime/src/task_executor_tests/input_helpers.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: this slice removes successful
    embedded-runtime path-only preflight identity and package-fact or
    dependency-requirement model-id fallback. It does not add a compatibility
    builder, path alias, package-fact identity fallback, directory scan, or
    scheduler bypass.
  - Implementation completed: `build_model_dependency_request` now reads
    explicit Pumas identity, leaves `model_path` empty, and both
    `dependency-environment` and Python-node dependency preflight block with a
    typed execution error when Pumas identity is absent. Failure payloads report
    `model_id` instead of stale local paths. Focused tests and README
    traceability were updated.
  - Verification passed: `cargo fmt -p pantograph-embedded-runtime`, `cargo
    fmt -p pantograph-embedded-runtime -- --check`, `cargo test -p
    pantograph-embedded-runtime dependency_preflight --lib`, `cargo test -p
    pantograph-embedded-runtime dependency_fail_closed --lib`, `cargo test -p
    pantograph-embedded-runtime build_model_dependency_request --lib`, and
    `cargo check -p pantograph-embedded-runtime`.
  - Remaining follow-up: replace the exported path-shaped
    `ModelDependencyRequest`/`ModelRefV2` contracts, lower-level
    descriptor/cache/activity correlation, and any direct explicit
    `model_path` graph edges with typed Pumas model-reference request/result
    contracts.
- 2026-05-20 Milestone 5 dependency-environment descriptor Pumas identity
  contract slice:
  - Smallest useful vertical slice: align the graph-visible
    `dependency-environment` descriptor with canonical preflight by replacing
    the required `model_path` input with required object-shaped
    `pumas_model_ref`.
  - Allowed write set:
    `crates/workflow-nodes/src/processing/dependency_environment.rs`,
    `crates/workflow-nodes/src/processing/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - No-fallback/no-legacy confirmation: this slice removes graph-facing
    dependency-environment `model_path` identity and does not add a path alias,
    compatibility shim, Pumas path join, package-fact identity fallback, or
    scheduler bypass.
  - Implementation completed: the descriptor now requires `pumas_model_ref`,
    omits `model_path`, and the processing-node README records that Pumas
    owns model-reference to artifact load-target resolution.
  - Verification passed: `cargo fmt -p workflow-nodes`, `cargo fmt -p
    workflow-nodes -- --check`, `cargo test -p workflow-nodes
    dependency_environment --lib`, and `cargo check -p workflow-nodes`.
  - Remaining follow-up: replace the exported path-shaped
    `ModelDependencyRequest`/`ModelRefV2` contracts, non-canonical processing
    descriptor model-path inputs, lower-level descriptor/cache/activity
    correlation, and any direct explicit `model_path` graph edges with typed
    Pumas model-reference request/result contracts.
- 2026-05-20 Milestone 5 dependency-planning contract ownership re-plan:
  - Decision: use option 3, a neutral shared dependency-planning contract owner,
    before replacing `ModelDependencyRequest.model_path` and
    `ModelRefV2.model_path`.
  - Rationale: node-engine should forward validated graph intent, not own Pumas
    artifact semantics. Inference contracts are useful references, but the
    dependency-planning boundary spans graph execution, host/Pumas resolution,
    scheduler intent, dependency readiness, and worker handoff, so it must not
    become image/PyTorch/inference-feature-specific.
  - Plan update completed: Milestone 5 now requires a contract-owner gate first,
    then staged node-engine adapter, host resolver, graph model-ref successor,
    Pumas load-target, cache/activity identity, preflight caller, and legacy DTO
    removal slices. The shared contract must provide typed request/result/
    diagnostic DTOs, serde fixtures, README traceability, and validated
    parsing from raw graph JSON without introducing parallel artifact authority
    or executable path identity.
  - No-fallback/no-legacy confirmation: this planning update does not preserve
    path-shaped compatibility. It makes the remaining `model_path` fields
    explicit removal targets and keeps executable local paths scoped to
    selected backend/worker handoff after Pumas-approved planning.
  - Verification passed: docs-only update; `git diff --check`.
- 2026-05-20 Milestone 5 dependency-planning blast-radius review:
  - Finding: the plan needed tighter implementation boundaries after inspecting
    the affected code. `ModelDependencyRequest`/`ModelRefV2`,
    embedded-runtime descriptor/cache/activity code, frontend
    dependency-environment action/source/activity matching, and existing
    inference-local Pumas DTO mirrors are all touched by the replacement.
  - Plan update completed: Milestone 5 now rejects a third independent Pumas
    model-ref/artifact DTO family, requires one canonical Pantograph location
    for Pumas-facing mirrors or re-exports, adds frontend
    dependency-environment contract alignment as its own staged slice, and
    requires removal of embedded-runtime path-to-Pumas fallback resolution.
  - No-fallback/no-legacy confirmation: path-shaped UI action payloads,
    dependency activity correlation, descriptor cache keys, and path-to-Pumas
    resolver behavior are explicit replacement/removal targets. Remaining
    `model_path` uses must be classified before deletion as either
    graph/dependency identity, which is removed, or selected backend/worker load
    target handoff, which may exist only after scheduler/Pumas planning.
  - Verification passed: docs-only update; `git diff --check`.
- 2026-05-20 Milestone 5 dependency-planning standards iteration:
  - Standards reviewed:
    `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`,
    `ARCHITECTURE-PATTERNS.md`, `CODING-STANDARDS.md`,
    `DOCUMENTATION-STANDARDS.md`, `TESTING-STANDARDS.md`,
    `FRONTEND-STANDARDS.md`, `INTEROP-STANDARDS.md`,
    `languages/rust/RUST-API-STANDARDS.md`, and
    `languages/rust/RUST-TOOLING-STANDARDS.md`.
  - Finding: the broad standards gates were directionally correct, but the new
    dependency-planning replacement needed explicit crate/module shape,
    feature-check, executable-fixture, frontend replacement, and decomposition
    constraints before implementation.
  - Plan update completed: Milestone 5 now requires the neutral contract owner
    to be a contract/domain-only crate or module with workspace lints,
    crate-level docs, README traceability, curated re-exports, typed error
    modules, and serde fixtures. It also records that
    `DependencyEnvironmentNode.svelte` is already over the UI decomposition
    threshold and `model_dependencies.rs` is close to the general file
    threshold, so implementation must extract or reuse focused helpers instead
    of adding responsibilities there.
  - No-fallback/no-legacy confirmation: these standards constraints do not add
    compatibility behavior. They make the replacement stricter by requiring
    parse-once typed boundaries, executable contract fixtures, typed diagnostics,
    backend-owned frontend state, and no path-shaped UI/action/activity aliases.
  - Verification passed: docs-only update; `git diff --check`.
- 2026-05-20 Milestone 5 dependency-planning contract-owner gate:
  - Smallest useful vertical slice: add a neutral shared
    `pantograph-dependency-planning` contract crate before migrating
    node-engine, embedded-runtime, or frontend consumers.
  - Allowed write set:
    root `Cargo.toml`, `Cargo.lock`, `crates/pantograph-dependency-planning/`,
    `crates/inference/Cargo.toml`, `crates/inference/src/model_contracts.rs`,
    `crates/inference/README.md`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - Implementation completed: the new crate owns typed dependency-planning
    requests, scheduler intent, caller context, selected binding ids, result
    states, typed diagnostics, and Pumas-facing model/load-target mirror types.
    Inference now re-exports those Pumas mirror types from the shared contract
    owner instead of defining a parallel local copy.
  - No-fallback/no-legacy confirmation: no runtime behavior, resolver path,
    graph alias, frontend compatibility payload, worker fallback, or scheduler
    bypass was added. Local load paths appear only in ready load-target result
    handoff DTOs.
  - Standards evidence: the crate is contract/domain-only, uses workspace
    lints, has crate-level docs and README coverage for the crate, source,
    tests, and fixtures, and adds executable serde fixtures. The crate has no
    Pumas client, filesystem, subprocess, frontend, Tauri, Python, worker, or
    scheduler policy dependencies.
  - Verification passed:
    `cargo fmt -p pantograph-dependency-planning -p inference`,
    `cargo test -p pantograph-dependency-planning`,
    `cargo check -p inference`,
    `cargo test -p inference --test model_contracts pumas_artifact_load_target_decodes_existing_pumas_wire_shape`,
    `cargo fmt -p pantograph-dependency-planning -p inference -- --check`,
    `cargo check -p pantograph-dependency-planning --all-features`,
    `cargo check -p pantograph-dependency-planning --no-default-features`,
    `cargo check -p node-engine --features inference-nodes,pytorch-nodes`,
    `cargo check -p inference --no-default-features`,
    and `cargo check -p inference --all-features`.
  - Remaining follow-up: migrate node-engine request construction, host
    resolver traits, embedded-runtime cache/activity identity, and frontend
    dependency-environment actions to the shared contract; then remove the old
    path-shaped DTO fields instead of adapting them.
- 2026-05-20 Milestone 5 dependency override patch shared-contract ownership:
  - Smallest useful vertical slice: move dependency override patch DTOs from
    node-engine-local ownership into `pantograph-dependency-planning`.
  - Allowed write set: `Cargo.lock`, `crates/node-engine/Cargo.toml`,
    `crates/node-engine/src/model_dependencies.rs`,
    `crates/pantograph-dependency-planning/`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - Implementation completed: `DependencyOverrideScope`,
    `DependencyOverrideFieldsV1`, and `DependencyOverridePatchV1` now live in
    the shared contract crate and are re-exported from node-engine for current
    callers. `DependencyPlanningRequest` now carries
    `dependency_override_patches` directly.
  - No-fallback/no-legacy confirmation: node-engine's duplicate override DTOs
    were removed rather than adapted. No runtime behavior, path resolver,
    frontend compatibility payload, worker fallback, scheduler bypass, or second
    override schema was added.
  - Standards evidence: override patches remain typed contract data with
    validation and executable fixture coverage. The slice did not touch
    scheduler policy, Pumas lookup, worker execution, frontend controls,
    generated files, saved workflows, or platform-specific code.
  - Verification passed:
    `cargo fmt -p pantograph-dependency-planning -p node-engine`,
    `cargo test -p pantograph-dependency-planning`,
    `cargo test -p node-engine model_dependencies --lib`,
    `cargo check -p node-engine --features inference-nodes,pytorch-nodes`,
    `cargo fmt -p pantograph-dependency-planning -p node-engine -- --check`,
    `cargo check -p pantograph-dependency-planning --all-features`,
    and `cargo check -p pantograph-dependency-planning --no-default-features`.
  - Remaining follow-up: migrate node-engine and embedded-runtime from
    `ModelDependencyRequest` to the shared request/result contracts, then remove
    path-shaped dependency identity fields.
- 2026-05-20 Milestone 5 dependency-planning platform/source context contract:
  - Smallest useful vertical slice: close the shared-contract gap found before
    node-engine request migration by adding typed platform context and
    diagnostic-only source node type to `pantograph-dependency-planning`.
  - Allowed write set: `crates/pantograph-dependency-planning/`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - Implementation completed: `DependencyPlanningPlatformContext` now carries a
    validated `DependencyPlatformKey`, can derive the stable key from OS/arch
    facts, and `DependencyPlanningCallerContext` can carry
    `source_node_type` as a validated diagnostic field. The request fixture now
    exercises both fields.
  - No-fallback/no-legacy confirmation: this does not preserve the old
    `platform_context: serde_json::Value` or executable `node_type` routing.
    Platform data is a typed planning/correlation fact, and source node type is
    caller context only.
  - Verification passed:
    `cargo fmt -p pantograph-dependency-planning`,
    `cargo test -p pantograph-dependency-planning`,
    `cargo check -p pantograph-dependency-planning --all-features`,
    `cargo check -p pantograph-dependency-planning --no-default-features`,
    `cargo fmt -p pantograph-dependency-planning -- --check`,
    `cargo check -p node-engine --features inference-nodes,pytorch-nodes`,
    and `git diff --check`.
  - Remaining follow-up: migrate node-engine request construction to decode
    graph input into the shared request, then remove the old path-shaped
    request fields and raw platform JSON from internal dependency APIs.
- 2026-05-20 Milestone 5 node-engine request migration re-plan boundary:
  - Investigation result: production node-engine preflight cannot be migrated
    directly to `DependencyPlanningRequest` while the resolver still returns
    `ModelRefV2`, because `ModelRefV2` requires `model_path`.
  - Rejected implementation paths: converting the shared request back into
    `ModelDependencyRequest`, returning `ModelRefV2` with a Pumas-approved
    local load path, or keeping `model_path` blank and repairing it in
    `build_model_ref_v2`. Each option preserves or reintroduces the legacy
    path-shaped node-engine contract.
  - Required plan adjustment: introduce the path-free preflight/model-reference
    successor before changing production preflight callers, then switch the
    resolver trait to consume `DependencyPlanningRequest` and return the
    path-free output. Pumas-approved local load targets must remain host/planner
    handoff facts for backend/worker execution, not node-engine graph identity.
  - Verification passed: docs-only boundary update; `git diff --check`.
- 2026-05-20 Milestone 5 path-free preflight successor planning:
  - Decision: use the clean option 4 re-plan. Introduce the path-free
    preflight/model-reference successor before migrating production node-engine
    preflight to `DependencyPlanningRequest`.
  - Plan update completed: Milestone 5 staging now places the successor gate
    before the node-engine adapter gate. The successor must carry Pumas model
    identity, task facts, expected artifact kind when known, selected binding
    ids, optional dependency requirements id, scheduler/runtime/device facts
    when selected, and typed diagnostics without `model_path`, local load
    paths, `entry_path`, or `selected_artifact_path` as executable identity.
  - Rejected paths remain rejected: no adapter shim back into
    `ModelDependencyRequest`, no fake/blank repaired `model_path`, no Pumas
    load target in graph/node-engine model identity, and no broad one-slice
    replacement of request, resolver, frontend, cache, and worker handoff.
  - Remaining follow-up: implement the successor contract in
    `pantograph-dependency-planning` with fixtures/tests, then migrate
    node-engine preflight and host resolver boundaries in separate validated
    slices.
  - Verification passed: docs-only update; `git diff --check`.
- 2026-05-21 Milestone 5 path-free preflight request/result contract:
  - Smallest useful vertical slice: add the contract-only path-free preflight
    request/result contracts and shared identity/correlation key before
    production node-engine preflight migration.
  - Allowed write set: `crates/pantograph-dependency-planning/`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - Implementation completed: `DependencyPlanningIdentityKey` now carries the
    shared Pumas-ref/task/scheduler-intent/platform/binding identity used by
    preflight, cache, activity, and frontend correlation without local load
    paths. `DependencyPreflightRequest` carries the identity key, matching
    `DependencyPlanningRequest`, dependency requirements identity, and
    environment identity. `DependencyPreflightResult` carries path-free
    readiness proof plus bounded diagnostics as the graph/node-engine
    preflight successor to `ModelRefV2`. The retired
    `DependencyPreflightModelRef` export and fixture were removed. The
    preflight DTOs deny unknown path/load-target/package-fact fields, and
    validation rejects `selected_artifact_path` in path-free identity.
  - No-fallback/no-legacy confirmation: the slice did not migrate production
    execution, convert the successor back to `ModelRefV2`, add a load target to
    node-engine identity, or preserve any successful `model_path` contract.
  - Standards evidence: contract shapes remain in the contract/domain crate,
    public DTOs are re-exported through `lib.rs`, serde fixtures assert wire
    shape, and README/fixture documentation records the separation between
    path-free identity and host/planner load-target results.
  - Verification passed:
    `cargo fmt -p pantograph-dependency-planning`,
    `cargo test -p pantograph-dependency-planning`,
    `cargo fmt -p pantograph-dependency-planning -- --check`,
    `cargo check -p pantograph-dependency-planning`,
    `cargo check -p pantograph-dependency-planning --all-features`,
    `cargo check -p pantograph-dependency-planning --no-default-features`,
    `git diff --check`, and
    `rg -n "DependencyPreflightModelRef|dependency_preflight_model_ref" crates/pantograph-dependency-planning`
    confirmed the retired preflight model-ref contract is absent from the
    crate.
  - Deviation/discovered issue: no implementation deviation was required. The
    plan language was corrected in the same slice so current milestone
    instructions target `DependencyPreflightRequest`/`DependencyPreflightResult`
    instead of the retired `DependencyPreflightModelRef`.
  - Remaining follow-up: migrate node-engine request construction and host
    resolver traits to consume `DependencyPreflightRequest` and return
    `DependencyPreflightResult`, then move embedded-runtime cache/activity and
    frontend dependency-environment matching to `DependencyPlanningIdentityKey`.
- 2026-05-21 Milestone 5 node-engine adapter re-plan boundary:
  - Investigation result: the next production migration cannot be a standalone
    node-engine `DependencyPlanningRequest` builder. A builder that is not used
    by production preflight is dead migration code, and using it only to
    reconstruct the old `ModelDependencyRequest` would preserve the retired
    path-shaped resolver contract.
  - Required adjustment: the next code slice must first add the typed
    dependency-environment resolve/check/install contracts, then move canonical
    preflight and dependency-environment consumers together onto typed
    contracts. Node-engine should build the shared typed preflight request, the
    host resolver should consume it, and the preflight result should be
    `DependencyPreflightResult` rather than `ModelRefV2`.
  - Ownership note: dependency-environment check/install must not keep
    `ModelDependencyRequest.model_path` as canonical inference dependency
    identity after preflight moves. The old resolver contract is deletion work
    once the typed replacement is in place, not a compatibility tier.
  - Verification passed: uncommitted node-engine exploration was removed before
    commit; source diff returned clean. Plan-only boundary update will be
    committed separately from implementation work.
- 2026-05-21 Milestone 5 dependency-environment contract re-plan decision:
  - Decision: use option 3 for the node-engine adapter boundary. Define the
    full typed dependency-environment contract before production migration, then
    migrate canonical inference preflight, resolver operations, and
    dependency-environment check/install callers off `ModelDependencyRequest`
    together.
  - Plan update completed: Milestone 5 staging now adds a
    dependency-environment typed contract gate before the node-engine adapter
    gate. The new gate requires typed resolve/check/install request and result
    DTOs keyed by `DependencyPlanningIdentityKey` and shared planning facts,
    with no `model_path`, `modelPath`, local load path, `entry_path`,
    `selected_artifact_path`, raw platform JSON, raw mode strings, or
    path-shaped Pumas aliases as request identity.
  - Rejected paths remain rejected: no unused production request-builder slice,
    no adapter from `DependencyPlanningRequest` back into
    `ModelDependencyRequest`, no `ModelRefV2` repair path, and no temporary
    canonical branch that keeps path-shaped dependency identity alive.
  - Next implementation slice: add the dependency-environment typed contract in
    `pantograph-dependency-planning` with serde fixtures and validation tests
    for resolve, check, install, unavailable, invalid, and path-field rejection
    states. Production resolver migration follows only after that replacement
    contract exists.
  - Verification passed for this docs-only update: `git diff --check`.
- 2026-05-21 Milestone 5 standards iteration for option 3:
  - Standards reviewed:
    `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`,
    `ARCHITECTURE-PATTERNS.md`, `INTEROP-STANDARDS.md`,
    `TESTING-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`,
    `languages/rust/RUST-API-STANDARDS.md`, and
    `languages/rust/RUST-TOOLING-STANDARDS.md`.
  - Blast-radius findings: the option 3 plan touches the contract/domain crate,
    node-engine resolver trait and preflight callers, embedded-runtime
    dependency check/install/cache/activity paths, frontend dependency-
    environment action/status DTOs, mock workflow backends, saved workflow
    fixtures, and module/fixture READMEs. Current code still contains
    path-shaped success paths, raw JSON/string mode parsing, and test stubs
    around `ModelDependencyRequest`, so the plan must require replacement and
    deletion rather than adapters.
  - Plan update completed: Milestone 5 now requires domain-only contract
    ownership, public re-exports, crate and fixture README updates, typed
    request action/readiness/install/validation/failure states, validated ids,
    parse-once boundary validation, serde fixtures, public API tests, feature-
    mode Rust checks, durable test-state isolation, frontend Node-test and
    typecheck verification, and deletion of old resolver methods without
    default trait adapters or compatibility aliases.
  - No-fallback/no-legacy confirmation: this standards pass keeps option 3 as
    a replacement path. It does not approve `ModelDependencyRequest` as an
    alternate branch, `ModelRefV2` repair, `modelPath` frontend aliases, raw
    platform JSON, or path-shaped cache/activity identity.
  - Verification passed for this docs-only standards update: `git diff
    --check`.
- 2026-05-21 Milestone 5 dependency-environment typed contract:
  - Smallest useful vertical slice: add the contract-only
    dependency-environment resolve/check/install DTOs before any production
    resolver migration.
  - Allowed write set: `crates/pantograph-dependency-planning/`,
    `docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and this execution log. Root proposal markdown files remained ignored.
  - Implementation completed: `DependencyEnvironmentRequest` and
    `DependencyEnvironmentResult` now provide typed dependency-environment
    resolve/check/install contracts. The request carries
    `DependencyPlanningIdentityKey` plus `DependencyPlanningRequest` and
    validates their shared model ref, task, artifact kind, platform, selected
    bindings, and runtime/device facts. The result carries typed readiness,
    install, validation, failure, environment-ref, and diagnostic facts.
  - Tests/fixtures added: public serde fixtures cover resolve, check, install,
    ready, unavailable, and invalid states. Contract tests cover fixture
    decoding/validation, check/install requirements ids, path-shaped request
    field rejection, unknown raw mode rejection, mismatched identity rejection,
    malformed environment ids, and typed diagnostics.
  - No-fallback/no-legacy confirmation: production node-engine,
    embedded-runtime, frontend, Pumas lookup, package-manager, scheduler, and
    host adapter behavior were not changed. This slice adds no conversion from
    the typed dependency-environment contract back into
    `ModelDependencyRequest` and no path-shaped compatibility alias.
  - Standards evidence: the contract stays in the domain crate, public types
    are re-exported through `lib.rs`, README and fixture README traceability was
    updated, raw JSON is validated before becoming trusted internal input, and
    environment refs use validated newtypes rather than raw path or mode
    strings.
  - Verification passed:
    `cargo fmt -p pantograph-dependency-planning`,
    `cargo test -p pantograph-dependency-planning`,
    `cargo fmt -p pantograph-dependency-planning -- --check`,
    `cargo check -p pantograph-dependency-planning`,
    `cargo check -p pantograph-dependency-planning --all-features`,
    `cargo check -p pantograph-dependency-planning --no-default-features`, and
    `git diff --check`.
  - Remaining follow-up: migrate the node-engine resolver boundary,
    dependency-environment check/install consumers, cache/activity identity, and
    frontend dependency-environment DTOs to these typed contracts, then remove
    `ModelDependencyRequest` and path-shaped resolver/test surfaces.
- 2026-05-21 Milestone 5 resolver-boundary re-plan trigger:
  - Investigation result: replacing `ModelDependencyResolver` after the
    dependency-environment contract slice is not yet standards-compliant. The
    new `DependencyEnvironmentResult` covers action, readiness, install,
    validation, failure, diagnostics, and environment refs, but it does not own
    the resolved requirements and per-binding check/install status payloads that
    current node-engine, embedded-runtime, activity, and frontend consumers
    require.
  - Legacy risk: keeping `ModelDependencyRequirements`,
    `ModelDependencyStatus`, or `ModelDependencyInstallResult` on the new
    resolver boundary would leave node-engine-owned legacy DTOs as the
    canonical dependency-environment result contract. Mapping new typed results
    back into those old types would be a compatibility shim.
  - Required re-plan: decide the shared dependency-planning contract for
    dependency requirements, selected bindings, per-binding status rows,
    check/install timestamps, code/message diagnostics, and environment refs
    before replacing `ModelDependencyResolver`.
  - Stop condition: implementation of the resolver boundary is paused until the
    result payload ownership is planned. Continuing without that decision would
    violate the no-fallback/no-legacy rule.
- 2026-05-21 Milestone 5 dependency result re-plan decision:
  - Decision: use option 2 with option 3 discipline. Expand the shared
    dependency-environment contract enough to replace the current
    node-engine-owned requirements/status/install DTOs, but keep the design
    lifecycle-scoped to dependency-environment resolve/check/install rather than
    designing the entire future dependency-management subsystem now.
  - Required shared payload ownership: `pantograph-dependency-planning` must own
    typed dependency requirements, requirement bindings, validation errors,
    selected binding ids, per-binding check/install status rows, status
    code/message diagnostics, operation timestamps, environment refs, and typed
    readiness/install/validation/failure states before the resolver boundary is
    replaced.
  - Option 3 discipline: the new payload types must avoid Python-only,
    package-manager-only, image-only, or runtime-specific assumptions.
    Requirement definitions, binding/profile facts, operation status rows,
    environment refs, diagnostics, and future runtime-managed binary or
    device/toolchain readiness facts must remain separable so a later full
    dependency-domain model can split them without another compatibility layer.
  - Rejected paths: do not keep `ModelDependencyRequirements`,
    `ModelDependencyStatus`, or `ModelDependencyInstallResult` as canonical
    resolver outputs, and do not map new typed environment results back into
    those node-engine DTOs.
  - Next implementation slice: add the shared dependency result payloads with
    serde fixtures and validation tests for ready, missing, invalid,
    unavailable, install failed, and no-binding states. Resolver migration
    resumes only after those shared payloads can replace the old DTOs directly.
  - Verification passed for this docs-only plan update: `git diff --check`.
- 2026-05-21 Milestone 5 dependency result standards iteration:
  - Standards reviewed:
    `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`,
    `ARCHITECTURE-PATTERNS.md`, `INTEROP-STANDARDS.md`,
    `TESTING-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`,
    `languages/rust/RUST-API-STANDARDS.md`, and
    `languages/rust/RUST-TOOLING-STANDARDS.md`.
  - Blast-radius finding: the old node-engine result DTOs contain trusted
    cross-layer raw strings for requirement kinds, binding/profile/environment
    ids, environment kind, status code/message, validation paths, and operation
    timestamps. Implementing option 2 without tightening those fields would
    preserve stringly typed protocol state and make future non-Python
    dependency classes harder to add cleanly.
  - Plan update completed: the next payload slice now requires validated
    newtypes or typed enums for trusted ids/states/codes/timestamps, bounded
    typed identifiers where extension is necessary, Python/package-manager
    facts isolated behind optional detail structs, `snake_case` serde wire
    shape, unknown-field rejection, path-field rejection, typed validation
    errors, public `DependencyPlanningContractError` validation APIs, README
    and fixture README updates, and focused fixtures/tests for malformed ids,
    no-binding states, duplicate binding selection, invalid timestamps,
    failed installs, unavailable requirements, invalid validation errors, and
    Python-detail containment.
  - No-fallback/no-legacy confirmation: this standards pass keeps the option 2
    contract as a replacement path. It does not approve keeping
    `ModelDependencyRequirements`, `ModelDependencyStatus`, or
    `ModelDependencyInstallResult` as canonical resolver outputs, mapping typed
    results back into those DTOs, or using node-engine path-shaped dependency
    identity as an alternate branch.
  - Verification passed for this docs-only standards update: `git diff
    --check`.
- 2026-05-21 Milestone 5 shared dependency result payload slice:
  - Slice: expand the shared `pantograph-dependency-planning`
    dependency-environment result contract before replacing the
    node-engine-owned resolver result DTOs.
  - Allowed write set used:
    `crates/pantograph-dependency-planning/src/environment.rs`,
    `crates/pantograph-dependency-planning/src/environment/`,
    `crates/pantograph-dependency-planning/src/lib.rs`,
    `crates/pantograph-dependency-planning/README.md`,
    `crates/pantograph-dependency-planning/src/README.md`,
    `crates/pantograph-dependency-planning/tests/contract.rs`,
    `crates/pantograph-dependency-planning/tests/fixtures/`, and this plan.
  - Implementation completed: dependency-environment results now carry shared
    typed requirement rows, binding rows, selected binding ids, per-binding
    status rows, operation timestamps, validation-error rows, environment refs,
    and diagnostics. New type coverage includes requirement kind, environment
    kind, binding status, operation state, validation code, profile id,
    requirement name, validation field path, and non-zero operation timestamp.
    Python/package-manager facts are scoped to `PythonRequirementDetails` and
    `PythonBindingDetails`. Decomposition review split payload rows into
    `src/environment/payload.rs`, scalar contracts into
    `src/environment/scalar.rs`, and action/state enums into
    `src/environment/state.rs` with a directory README, keeping the parent
    `environment.rs` focused on the request/result envelope.
  - Focused tests/fixtures added or updated: ready result rows, unavailable
    requirements, invalid install failure row, no-binding unavailable result,
    unknown-field rejection, duplicate selected binding ids, invalid
    timestamps, path-shaped diagnostic field paths, and Python-detail
    containment.
  - Standards evidence: new boundary rows use snake_case serde and deny
    unknown fields; selected binding order is preserved while duplicate
    selected binding ids are rejected; operation timestamps are non-zero and
    ordered; diagnostic and validation field paths reject filesystem-shaped
    paths; public validation returns `DependencyPlanningContractError`; crate
    README, source README, environment child-module README, and fixture README
    document the new contract.
  - No-fallback/no-legacy confirmation: this slice did not migrate
    `ModelDependencyResolver`, did not adapt the new shared result contract back
    into `ModelDependencyRequirements`, `ModelDependencyStatus`, or
    `ModelDependencyInstallResult`, and did not touch node-engine,
    embedded-runtime, frontend, workers, scheduler policy, Pumas lookup,
    generated DTOs, lockfiles, or saved workflow behavior.
  - Verification passed: `cargo fmt -p pantograph-dependency-planning`,
    `cargo test -p pantograph-dependency-planning`,
    `cargo fmt -p pantograph-dependency-planning -- --check`,
    `cargo check -p pantograph-dependency-planning`,
    `cargo check -p pantograph-dependency-planning --all-features`, and
    `cargo check -p pantograph-dependency-planning --no-default-features`.
    Final whitespace verification passed: `git diff --check`.
  - Remaining follow-up: replace the node-engine resolver boundary with these
    shared result payloads and remove the old node-engine-owned
    `ModelDependencyRequirements`, `ModelDependencyStatus`, and
    `ModelDependencyInstallResult` contracts rather than mapping into them.
- 2026-05-21 Milestone 5 resolver boundary re-plan trigger:
  - Discovery: the next implementation slice cannot safely replace only
    `ModelDependencyRequirements`, `ModelDependencyStatus`, and
    `ModelDependencyInstallResult`. `ModelDependencyResolver` also owns
    `resolve_model_ref`, which returns `ModelRefV2`; `ModelRefV2` still
    requires `model_path` and is consumed by node-engine and embedded-runtime
    preflight paths as a path-shaped model identity/handoff contract.
  - Why this stops implementation: editing the resolver boundary now without a
    canonical model-ref/preflight replacement would either keep
    `ModelDependencyRequest.model_path` and `ModelRefV2.model_path` as legacy
    compatibility fields, or invent a new handoff contract during a broad code
    migration. Both violate the no-fallback/no-legacy rule and the plan’s
    serial contract ownership requirement.
  - Re-plan needed: decide the canonical replacement for model-ref hydration
    before node-engine/embedded-runtime resolver migration. Options to evaluate
    include a separate shared preflight result in `pantograph-dependency-planning`,
    folding model-ref hydration into dependency-environment results, or
    removing model-ref hydration from the dependency resolver boundary and
    letting scheduler/host planning facts own the handoff.
  - Stop condition: no node-engine, embedded-runtime, frontend, worker, or
    resolver implementation changes should start until that contract decision is
    made and recorded.
  - Verification passed for this docs-only re-plan note: `git diff --check`.
- 2026-05-21 Milestone 5 resolver boundary re-plan decision:
  - Updated decision: use option 3 followed by option 2. First define the
    canonical scheduler/host execution handoff that replaces model-ref
    hydration; then replace the resolver boundary and delete the old
    path-shaped contracts. This supersedes the earlier "option 2 with option 3
    discipline" staging for the next migration step.
  - Execution handoff boundary: the post-preflight handoff is produced only
    after scheduler/host planning and is the only contract allowed to carry
    executable Pumas-approved load targets, selected backend/runtime/device
    decisions, dependency-environment launch facts, and worker-local execution
    facts. Node-engine preflight, dependency-environment status, graph/cache
    identity, and frontend activity correlation remain path-free.
  - Owner-selection result: codebase impact review inspected existing
    scheduler, runtime-registry, embedded-runtime, worker, and
    dependency-planning boundaries. The owner anchor is the existing
    `pantograph-workflow-service::WorkflowExecutionPlan`, with executable
    launch material produced by an embedded-runtime/host subordinate projection
    after Pumas load-target approval. `WorkflowExecutionPlan` remains
    path-free and must not grow local paths, full package facts, graph inputs,
    worker envelopes, or launch-process details. The subordinate handoff is the
    only allowed source of worker-local Pumas load targets; graph value maps,
    node-engine preflight, dependency-environment results, frontend activity
    identity, and cache keys remain path-free.
  - Rust API-shape decision: node-engine calls a host service rather than
    receiving executable handoff data. Add a planned-inference execution host
    extension such as `PLANNED_INFERENCE_EXECUTION_HOST` storing
    `Arc<dyn PlannedInferenceExecutionHost>`. The first method is image
    generation:
    `generate_image(PlannedImageGenerationRequest)`, where the request carries
    only `workflow_run_id`, `node_id`, `request_id`, and
    `inference::ImageGenerationRequest`. Embedded-runtime implements the trait,
    resolves the active `WorkflowExecutionPlan` node decision, obtains the
    Pumas-approved load target through Pumas, builds an internal
    `PlannedImageGenerationLaunchHandoff`, and calls a gateway launch API such
    as `InferenceGateway::generate_image_from_launch_handoff(...)`. The gateway
    method may wrap the existing canonical planning-input methods, but
    executable package/load-target facts must enter backend execution only
    through the host-built handoff.
  - No additional architecture re-plan is required for this boundary. Future
    implementation slices still need normal thin-slice planning with allowed
    write sets, focused tests, and verification, but the owner, API shape,
    rejected fields, lifecycle, and diagnostic direction are now recorded.
  - 2026-05-22 standards iteration: the Stage 1 API shape remains
    standards-compliant only if the first code slice treats
    `PlannedInferenceExecutionHost` as an async outer I/O boundary, keeps pure
    validation/projection synchronous, gives public request/error/handoff types
    private validated constructors or `TryFrom` conversions, preserves bounded
    workflow_run_id/node_id/request_id diagnostics, and performs a
    decomposition review before adding logic to the already-large
    `node-engine` image-generation executor/test files.
  - Resolver replacement after the handoff is planned:
    dependency-environment resolve/check/install consume
    `DependencyEnvironmentRequest` and return
    `DependencyEnvironmentResult`; canonical preflight consumes
    `DependencyPreflightRequest` and returns `DependencyPreflightResult`; no
    resolver method returns `ModelRefV2` or executable load-target facts to
    node-engine.
  - No-fallback/no-legacy confirmation: the migration must remove
    `resolve_model_ref`, `ModelDependencyRequest.model_path`,
    `ModelRefV2.model_path`, path-derived model-id repair, and path-shaped
    cache/activity identity when their canonical replacements land. Do not add
    adapters that map dependency-environment results, preflight results, or
    execution handoff facts back into `ModelDependencyRequirements`,
    `ModelDependencyStatus`, `ModelDependencyInstallResult`,
    `ModelDependencyRequest`, or `ModelRefV2`.
  - Next implementation slice: implement only the node-engine planned
    execution host boundary and focused node-engine tests. The allowed write
    set is `crates/node-engine/src/planned_inference.rs`,
    `crates/node-engine/src/extensions.rs`, `crates/node-engine/src/lib.rs`
    only if re-exports are required,
    `crates/node-engine/src/core_executor/inference_nodes.rs`, optional
    extracted node-engine image-generation child modules/tests created by the
    decomposition review, and matching node-engine inference tests. Do not
    implement embedded-runtime Pumas load-target resolution, gateway/backend
    worker execution, resolver migration, frontend/generated/saved workflow
    changes, or lockfile edits in that first code slice.
  - Verification passed for this docs-only design update: `git diff --check`.
- 2026-05-22 Milestone 5 node-engine planned execution host boundary:
  - Smallest useful vertical slice: add the node-engine host-service boundary
    for planned image generation and remove image generation's direct
    graph-carried package/load-target execution bridge.
  - Allowed write set: `crates/node-engine/src/planned_inference.rs`,
    `crates/node-engine/src/extensions.rs`,
    `crates/node-engine/src/core_executor/inference_nodes.rs`,
    `crates/node-engine/src/core_executor/inference_tests.rs`, and Milestone 5
    plan notes. `crates/node-engine/src/lib.rs` was not needed. Unrelated root
    proposal markdown files remained ignored.
  - Implementation completed: added `PlannedInferenceExecutionHost`,
    `PlannedImageGenerationRequest`, and bounded host execution errors; added
    the `PLANNED_INFERENCE_EXECUTION_HOST` extension key; routed canonical
    image generation through the host with workflow_run_id/node_id/request_id
    correlation; stopped parsing image-generation
    `resolved_model_package_facts` and `resolved_model_artifact_load_target`;
    removed the old image-generation planned gateway test backend; and added
    focused host-boundary tests.
  - No-fallback/no-legacy confirmation: image generation now fails closed when
    the planned execution host is missing. Node-engine no longer uses
    graph-carried package facts, artifact load targets, model paths, or gateway
    planning as a fallback for image-generation execution.
  - Decomposition note: `inference_nodes.rs` and `inference_tests.rs` remain
    over the preferred line-count threshold, but this slice kept those files to
    minimal routing/test edits and put new public API logic in
    `planned_inference.rs`. Further image-generation behavior must be
    extracted before adding substantial logic.
  - Verification passed:
    `cargo test -p node-engine --features inference-nodes canonical_llm_image_generation`,
    `cargo test -p node-engine --features inference-nodes build_image_generation_execution_request`,
    `cargo test -p node-engine --features inference-nodes`,
    `cargo check -p node-engine`,
    `cargo check -p node-engine --all-features`,
    `cargo check -p node-engine --no-default-features`,
    `cargo fmt -p node-engine -- --check`, and `git diff --check`.
  - Follow-up status: the inference gateway launch-handoff API and
    embedded-runtime planned inference host were completed in subsequent
    2026-05-22 slices, and the retired node-engine planned-decision extension
    path was removed.
- 2026-05-22 Milestone 5 inference gateway launch handoff:
  - Smallest useful vertical slice: add the inference-owned
    image-generation worker-launch handoff and gateway methods that consume it,
    without changing node-engine, embedded-runtime, frontend, generated files,
    saved workflows, lockfiles, or Pumas resolver code.
  - Allowed write set: `crates/inference/src/image_generation_planner.rs`,
    `crates/inference/src/gateway.rs`, `crates/inference/src/lib.rs`,
    `crates/inference/src/gateway_tests.rs`, and Milestone 5 plan notes.
  - Implementation completed: added `PlannedImageGenerationLaunchHandoff` with
    private fields and constructor validation for image-generation task
    decisions and selected-model/package-facts consistency; exported the
    handoff and error type; added gateway methods for launch-handoff execution
    with and without lifecycle events; and added focused tests for forwarding,
    lifecycle event shape, task mismatch, and selected model mismatch.
  - No-fallback/no-legacy confirmation: the handoff is only
    host-to-gateway worker-launch material. It is not serialized into graph
    values, preflight results, dependency-environment results, frontend
    activity identity, or cache keys, and it does not revive graph-carried
    package facts or artifact load targets.
  - Verification passed:
    `cargo test -p inference generate_image_from_launch_handoff`,
    `cargo test -p inference planned_image_generation_launch_handoff`,
    `cargo test -p inference gateway::tests::`,
    `cargo check -p inference`,
    `cargo check -p inference --all-features`,
    `cargo check -p inference --no-default-features`,
    `cargo fmt -p inference -- --check`, and `git diff --check`.
  - Remaining follow-up: implement embedded-runtime
    `PlannedInferenceExecutionHost` to build this handoff from the active
    `WorkflowExecutionPlan` and Pumas typed load-target resolver, then remove
    the old planned-decision extension path when it is no longer consumed.
- 2026-05-22 Milestone 5 embedded-runtime planned inference host:
  - Smallest useful vertical slice: install an embedded-runtime
    `PlannedInferenceExecutionHost` that builds the image-generation launch
    handoff from the active scheduler execution plan and Pumas' typed artifact
    load-target resolver, then remove the retired node-engine decision-context
    extension path.
  - Allowed write set: `crates/pantograph-embedded-runtime/src/planned_inference_host.rs`,
    `crates/pantograph-embedded-runtime/src/planned_inference_host_tests.rs`,
    `crates/pantograph-embedded-runtime/src/lib.rs`,
    `crates/pantograph-embedded-runtime/src/workflow_execution_session_execution.rs`,
    `crates/pantograph-embedded-runtime/src/workflow_execution_plan_projection.rs`,
    `crates/pantograph-embedded-runtime/src/workflow_execution_plan_projection_tests.rs`,
    `crates/node-engine/src/planned_inference.rs`,
    `crates/node-engine/src/extensions.rs`, and Milestone 5 plan notes.
    Unrelated root proposal markdown files remained ignored.
  - Implementation completed: session workflow execution now installs
    `EmbeddedPlannedInferenceExecutionHost` under
    `PLANNED_INFERENCE_EXECUTION_HOST`; the host reads the active
    `WorkflowExecutionPlan`, projects the current node decision to
    `BackendExecutionDecision`, resolves Pumas package facts and a ready
    Pumas-approved artifact load target, builds
    `PlannedImageGenerationLaunchHandoff`, and calls the inference gateway
    launch-handoff API with lifecycle recording when the session sink exists.
    The retired `PlannedInferenceDecisionContext`,
    `PLANNED_INFERENCE_DECISIONS` extension key, and plan-to-context
    projection were removed.
  - No-fallback/no-legacy confirmation: node-engine no longer receives
    scheduler decision maps, package facts, artifact load targets, model paths,
    or executable handoff data. If active plan lookup, node decision
    projection, selected model ref, Pumas facts, Pumas load-target readiness,
    handoff validation, or gateway execution fails, image generation fails
    through typed planned-execution errors instead of falling back to graph
    fields or path joins.
  - Decomposition note: new host behavior lives in
    `planned_inference_host.rs` with tests split into
    `planned_inference_host_tests.rs`. The existing session executor remains
    over the preferred line-count threshold, so this slice kept its change to
    the host-installation call and did not add Pumas or gateway policy there.
  - Verification passed:
    `cargo test -p pantograph-embedded-runtime planned_inference_host`,
    `cargo test -p pantograph-embedded-runtime workflow_execution_plan_projection`,
    `cargo test -p node-engine --features inference-nodes canonical_llm_image_generation`,
    `cargo test -p node-engine --features inference-nodes build_image_generation_execution_request`,
    `cargo test -p node-engine --features inference-nodes planned_inference`,
    `cargo check -p pantograph-embedded-runtime`,
    `cargo check -p pantograph-embedded-runtime --all-features`,
    `cargo check -p pantograph-embedded-runtime --no-default-features`,
    `cargo check -p node-engine --features inference-nodes`,
    `cargo check -p node-engine --all-features`,
    `cargo check -p node-engine --no-default-features`,
    `cargo fmt -p pantograph-embedded-runtime -p node-engine -- --check`,
    and `git diff --check`.
  - Remaining follow-up: continue the resolver/preflight migration by replacing
    old node-engine and embedded-runtime dependency resolver/preflight
    contracts with the shared dependency-planning request/result contracts,
    then remove the legacy request-building helpers instead of adapting them.
  - Discovered follow-up: `WorkflowExecutionPlan.selected_model_ref` is still a
    validated string model reference, not the full typed Pumas model-ref
    contract with selected-artifact fields. The new host therefore relies on
    Pumas' resolver to fail closed for ambiguous selected artifacts. Resolve
    this in the typed preflight/execution-plan contract replacement by carrying
    the canonical Pumas model ref as a typed identity, not by reintroducing
    local paths or graph-carried artifact load targets.
- 2026-05-21 Milestone 5 path-free preflight standards iteration:
  - Standards reviewed:
    `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CODING-STANDARDS.md`,
    `ARCHITECTURE-PATTERNS.md`, `INTEROP-STANDARDS.md`,
    `TESTING-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`,
    `languages/rust/RUST-API-STANDARDS.md`, and
    `languages/rust/RUST-TOOLING-STANDARDS.md`.
  - Existing-code finding: `pantograph-dependency-planning` already owns a
    small path-free `preflight` module and the shared
    `DependencyPlanningIdentityKey`. The next slice should extend that owner
    into the request/result contract gate instead of creating a second
    preflight contract in node-engine, embedded-runtime, scheduler, or frontend
    code.
  - Blast-radius finding: the earlier preflight split decision is
    standards-compliant only if implementation keeps preflight narrow. Without
    explicit guardrails, the preflight contract slice could accidentally pass
    full dependency-environment status or install payloads through
    graph/node-engine identity, let scheduler intent become a scheduler
    decision, or reintroduce local paths through fields such as
    `selected_artifact_path`, `local_load_path`, Python executable paths, or
    package source paths.
  - Plan update completed: Milestone 5 required the preflight contract gate to
    extend `pantograph-dependency-planning::preflight`, reuse existing
    validated contracts, reject path-shaped and load-target/package-fact
    fields, keep scheduler/runtime/device values as intent or hard
    requirements only, split the preflight module if it approaches
    decomposition limits, update READMEs and fixtures, and run the full
    dependency-planning contract verification set.
  - No-fallback/no-legacy confirmation: this standards pass does not authorize
    compatibility shims. The later migration must remove `ModelRefV2`,
    `ModelDependencyRequest.model_path`, path-derived model-id repair, and old
    node-engine-owned dependency result DTOs when their canonical replacements
    land.
  - Verification passed: `git diff --check -- docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md docs/plans/current-image-generation-graphs/05-execution-management.md`.
- 2026-05-21 Milestone 5 codebase blast-radius iteration:
  - Existing-code findings: shared preflight identity previously used
    `selected_runtime_id`/`selected_device_id` names before scheduler
    selection; the preflight contract implementation now resolves that finding
    with scheduler intent/requirement naming. Node-engine and embedded-runtime
    still independently build
    `ModelDependencyRequest`; embedded-runtime dependency environment emits
    worker-local manifest and Python executable fields; workflow-service still
    preserves path-only Puma-Lib state; image generation still consumes
    graph/input-carried package facts and artifact load targets as an
    intermediate execution bridge.
  - Plan update completed: Milestone 5 now requires intent/requirement naming
    before broader migration, one shared projection path into preflight and
    dependency-environment contracts, a strict split between environment
    identity/readiness and worker-local launch facts, workflow-service
    removal/rejection/stale diagnostics for path-only Puma-Lib state, and a
    later execution-plan handoff that removes graph/input-carried executable
    package/load-target facts before Milestone 5 closes.
  - No-fallback/no-legacy confirmation: these updates do not authorize
    compatibility modes. Retired request builders, path-only saved workflow
    behavior, `ModelRefV2`, and graph-carried load-target bridges must be
    removed or replaced by canonical contracts as their migration slices land.
  - Verification passed: `git diff --check -- docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md docs/plans/current-image-generation-graphs/05-execution-management.md`.
- 2026-05-21 Milestone 5 preflight contract blast-radius follow-up:
  - Existing-code findings: the preflight contract-only slice needed a
    slightly wider dependency-planning write scope than `preflight.rs` alone
    because
    `DependencyPlanningIdentityKey` is consumed by dependency-environment
    request validation and fixtures. Unknown-field rejection is top-level for
    several contracts but not guaranteed for nested preflight protocol structs.
    Later migration targets in node-engine and embedded-runtime already exceed
    the standards file-size threshold, and the current preflight path performs
    repeated resolve/check/model-ref hydration work through legacy DTOs.
  - Plan update completed: Milestone 5 permitted the preflight contract gate to
    update dependency-planning sibling modules, fixtures, and tests when needed
    for the identity terminology correction; required nested unknown-field
    rejection for the complete preflight protocol; requires decomposition before
    adding migration behavior to the large node-engine and embedded-runtime
    files; requires embedded-runtime/node-engine to import shared DTOs directly
    from `pantograph-dependency-planning`; and requires later migration to avoid
    carrying forward the repeated resolve -> check -> resolve model-ref
    sequence.
  - No-fallback/no-legacy confirmation: this update still keeps the next slice
    contract-only and does not authorize node-engine, embedded-runtime,
    frontend, worker, scheduler, generated DTO, lockfile, or saved-workflow
    migration. Retired DTOs and path-shaped behavior must be removed or
    replaced when their migration slices land, not adapted through aliases.
  - Verification passed: `git diff --check -- docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md docs/plans/current-image-generation-graphs/05-execution-management.md`.
- 2026-05-21 Milestone 5 node-engine preflight decomposition:
  - Smallest useful vertical slice: split node-engine preflight graph-input
    projection helpers out of the over-threshold preflight module before
    resolver/preflight behavioral migration.
  - Allowed write set: `crates/node-engine/src/core_executor/dependency_preflight.rs`,
    `crates/node-engine/src/core_executor/dependency_preflight/`, and Milestone
    5 plan notes. Root proposal markdown files remained ignored.
  - Implementation completed: moved graph-input projection, backend/task
    inference helpers, package-fact reading, and legacy `ModelRefV2` assembly
    helpers into
    `crates/node-engine/src/core_executor/dependency_preflight/input_projection.rs`.
    Added a directory README that marks the child module as a temporary
    projection owner and explicitly keeps dependency-planning contracts, Pumas
    artifact lookup, scheduler policy, and worker load-target handoff outside
    the module.
  - No-fallback/no-legacy confirmation: this slice was structural
    decomposition only. It did not add compatibility adapters, did not adapt
    `DependencyPreflightRequest` or `DependencyPreflightResult` back into
    `ModelDependencyRequest`/`ModelRefV2`, and did not preserve path-shaped
    preflight as a new alternate branch.
  - Standards evidence: `dependency_preflight.rs` now owns lifecycle
    enforcement, resolver calls, path repair/rejection, and compatibility
    lifecycle events; `input_projection.rs` owns projection helpers only. Line
    counts after the split are `dependency_preflight.rs` 462 and
    `dependency_preflight/input_projection.rs` 383.
  - Verification passed:
    `cargo fmt -p node-engine`,
    `cargo test -p node-engine`,
    `cargo fmt -p node-engine -- --check`,
    `cargo check -p node-engine`,
    `cargo check -p node-engine --all-features`,
    `cargo check -p node-engine --no-default-features`, and
    `git diff --check`.
  - Remaining follow-up: replace the old node-engine and embedded-runtime
    resolver/preflight contracts with
    `DependencyPreflightRequest`/`DependencyPreflightResult` and
    `DependencyEnvironmentRequest`/`DependencyEnvironmentResult`, then remove
    the legacy projection helpers rather than wrapping them.
- 2026-05-21 Milestone 5 embedded-runtime resolver decomposition:
  - Smallest useful vertical slice: split embedded-runtime resolver operations
    out of the over-threshold resolver module before resolver contract
    migration.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/model_dependencies.rs`,
    `crates/pantograph-embedded-runtime/src/model_dependency_operations.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`, the node-engine cfg
    import surfaced by all-features verification, and Milestone 5 plan notes.
    Root proposal markdown files remained ignored.
  - Implementation completed: moved the async resolve/check/install/model-ref
    operation flow into `model_dependency_operations.rs`. The root resolver
    module now owns shared resolver state, helper boundaries, child-module
    composition, and trait wiring. The embedded-runtime README now documents
    the operations module.
  - No-fallback/no-legacy confirmation: this slice was structural
    decomposition only. It did not add typed-contract adapters, did not map
    `DependencyEnvironmentResult` or `DependencyPreflightResult` back into old
    node-engine DTOs, and did not change resolver behavior.
  - Standards evidence: line counts after the split are
    `model_dependencies.rs` 278 and `model_dependency_operations.rs` 371. The
    remaining resolver contract migration can now change operation behavior
    without adding new policy to an over-threshold file.
  - Verification passed:
    `cargo fmt -p node-engine -p pantograph-embedded-runtime`,
    `cargo test -p pantograph-embedded-runtime model_dependencies`,
    `cargo fmt -p node-engine -p pantograph-embedded-runtime -- --check`,
    `cargo check -p pantograph-embedded-runtime`,
    `cargo check -p pantograph-embedded-runtime --all-features`,
    `cargo check -p pantograph-embedded-runtime --no-default-features`, and
    `git diff --check`.
  - Remaining follow-up: decompose
    `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment.rs`
    before migrating dependency-environment execution to the shared typed
    contracts.
- 2026-05-21 Milestone 5 dependency-environment executor decomposition:
  - Smallest useful vertical slice: split embedded-runtime
    dependency-environment executor helpers out of the over-threshold task
    executor module before dependency-environment contract migration.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment.rs`,
    `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment/`,
    `crates/pantograph-embedded-runtime/src/task_executor/README.md`, and
    Milestone 5 plan notes. Root proposal markdown files remained ignored.
  - Implementation completed: moved input projection, dependency mode parsing,
    environment-ref manifest emission, stable key helpers, and legacy
    dependency-environment request building into
    `task_executor/dependency_environment/helpers.rs`. Added a directory README
    documenting that the child module is not the shared contract owner and must
    be removed or replaced during typed-contract migration rather than wrapped
    as compatibility behavior.
  - No-fallback/no-legacy confirmation: this slice was structural
    decomposition only. It did not change successful dependency-environment
    behavior, did not add typed-contract adapters, and did not map shared
    dependency-planning DTOs back into old node-engine DTOs.
  - Standards evidence: line counts after the split are
    `task_executor/dependency_environment.rs` 244 and
    `task_executor/dependency_environment/helpers.rs` 415.
  - Verification passed:
    `cargo fmt -p pantograph-embedded-runtime`,
    `cargo test -p pantograph-embedded-runtime task_executor::tests::input_helpers`,
    `cargo test -p pantograph-embedded-runtime task_executor::tests::dependency_preflight`,
    `cargo fmt -p pantograph-embedded-runtime -- --check`,
    `cargo check -p pantograph-embedded-runtime`,
    `cargo check -p pantograph-embedded-runtime --all-features`,
    `cargo check -p pantograph-embedded-runtime --no-default-features`, and
    `git diff --check`.
  - Discovered issue: `cargo test -p pantograph-embedded-runtime task_executor`
    still fails in recorder-stream cases that execute Python-backed onnx/audio
    nodes without `pumas_model_ref`/`model_id`, triggering the current
    fail-closed dependency-preflight guard. The helper split did not introduce
    that behavior; keep it as a typed-preflight migration follow-up instead of
    repairing it through compatibility behavior.
  - Remaining follow-up: migrate dependency-environment execution to
    `DependencyEnvironmentRequest`/`DependencyEnvironmentResult` and remove the
    legacy request-building helpers rather than preserving them as wrappers.
- 2026-05-22 Milestone 5 embedded-runtime dependency descriptor path-fallback
  removal:
  - Smallest useful vertical slice: remove the embedded-runtime dependency
    descriptor branch that repaired path-only dependency requests into Pumas
    model identities before the larger resolver-boundary replacement.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/model_dependency_descriptors.rs`,
    `crates/pantograph-embedded-runtime/src/model_dependencies.rs`,
    `crates/pantograph-embedded-runtime/src/model_dependency_operations.rs`,
    `crates/pantograph-embedded-runtime/src/model_dependencies_tests.rs`, and
    Milestone 5 plan notes. Root proposal markdown files remained ignored.
  - Implementation completed: descriptor lookup no longer calls
    `PumasApi::resolve_pumas_model_ref` for path-only requests, no longer
    derives `model_id` from `ModelDependencyRequest.model_path`, and no longer
    uses `model_path` as the dependency status cache key. Cache lookup/inserts
    now require `model_id`; path-only requests remain unresolved.
  - No-fallback/no-legacy confirmation: this slice removes a legacy
    path-resolution branch rather than wrapping it. It does not convert shared
    dependency-planning contracts back into `ModelDependencyRequest` or
    `ModelRefV2`, and it does not make executable paths graph, cache,
    activity, or node-engine dependency identity.
  - Verification passed:
    `cargo fmt -p pantograph-embedded-runtime`,
    `cargo test -p pantograph-embedded-runtime model_dependencies`,
    `cargo check -p pantograph-embedded-runtime`,
    `cargo check -p pantograph-embedded-runtime --all-features`,
    `cargo check -p pantograph-embedded-runtime --no-default-features`,
    `cargo fmt -p pantograph-embedded-runtime -- --check`, and
    `git diff --check`.
  - Remaining follow-up: complete the resolver-boundary migration to
    `DependencyPreflightRequest`/`DependencyPreflightResult` and
    `DependencyEnvironmentRequest`/`DependencyEnvironmentResult`, then remove
    the old `ModelDependencyRequest`/`ModelRefV2` request-building and
    path-shaped dependency-environment activity/frontend surfaces.
- 2026-05-22 Milestone 5 node-engine typed dependency-planning request
  projection:
  - Smallest useful vertical slice: add the node-engine graph-input projection
    into the shared `DependencyPlanningRequest` contract and use it as a
    fail-closed preflight gate before replacing the resolver trait.
  - Allowed write set:
    `crates/node-engine/src/core_executor/dependency_preflight.rs`,
    `crates/node-engine/src/core_executor/dependency_preflight/`,
    `crates/node-engine/src/core_executor/inference_tests.rs`, and Milestone 5
    plan notes. Root proposal markdown files remained ignored.
  - Implementation completed: added
    `dependency_preflight/planning_projection.rs`, which requires
    `pumas_model_ref`, projects typed scheduler runtime/device intent,
    platform key, selected binding ids, task identity, and caller context, and
    rejects `pumas_model_ref.selected_artifact_path` as path-shaped dependency
    identity. Dependency preflight now validates this shared request before
    calling the remaining legacy resolver.
  - No-fallback/no-legacy confirmation: this slice does not convert the shared
    request back into `ModelDependencyRequest`, does not repair
    `ModelRefV2.model_path`, and does not accept `model_id` or `model_path` as
    substitutes for `pumas_model_ref`.
  - Standards evidence: after splitting the new projection owner,
    `dependency_preflight.rs` is 467 lines,
    `dependency_preflight/input_projection.rs` is 384 lines, and
    `dependency_preflight/planning_projection.rs` is 145 lines. The directory
    README documents that the typed projection must not become a compatibility
    bridge back to old DTOs.
  - Verification passed:
    `cargo fmt -p node-engine`,
    `cargo test -p node-engine --features inference-nodes build_dependency_planning_request`,
    `cargo test -p node-engine --features inference-nodes dependency_preflight`,
    `cargo check -p node-engine --features inference-nodes`,
    `cargo check -p node-engine --all-features`,
    `cargo check -p node-engine --no-default-features`,
    `cargo fmt -p node-engine -- --check`, and `git diff --check`.
  - Remaining follow-up: replace `ModelDependencyResolver` and the
    embedded-runtime implementation with
    `DependencyPreflightRequest`/`DependencyPreflightResult` and
    `DependencyEnvironmentRequest`/`DependencyEnvironmentResult`, then delete
    the old `ModelDependencyRequest`, `ModelRefV2`, and legacy request builders.
- 2026-05-22 Milestone 5 dependency preflight ownership re-plan:
  - Boundary discovered: production dependency preflight cannot migrate
    directly from the current typed request projection into
    `DependencyPreflightRequest` because the typed contract requires
    `dependency_requirements_id` and `environment_ref`, while the only live
    producer still goes through `ModelDependencyResolver`,
    `ModelDependencyRequest`, and `ModelRefV2.model_path`.
  - Decision: use a host-owned dependency readiness service as the next
    replacement step, with scheduler-owned dependency planning recorded as the
    later objective. The service is named `DependencyReadinessHost` to keep it
    distinct from the node-engine preflight phase and the proof-bearing
    `DependencyPreflightResult`. Node-engine builds and validates path-free
    dependency intent, calls the host service, and receives
    `DependencyPreflightResult` plus typed diagnostics only. The host service
    owns current-run dependency readiness proof by resolving/checking/installing
    dependency environments through the shared typed environment contracts. It
    must not return executable load targets, local paths, package facts, Python
    executable facts, or worker launch material through node-engine preflight
    identity.
  - Contract correction required before host wiring: do not use
    `DependencyPreflightRequest` as the host-service input because it already
    requires `dependency_requirements_id` and `environment_ref`. Add a shared
    `DependencyReadinessRequest` in `pantograph-dependency-planning`; it carries
    `DependencyPlanningIdentityKey`, `DependencyPlanningRequest`, and any typed
    readiness action/policy fields needed by the host, but not the readiness
    proof facts that the host produces. Also add one canonical identity-key
    constructor such as `DependencyPlanningIdentityKey::from_planning_request`
    so node-engine and host code do not duplicate field-by-field key assembly.
    Readiness policy must be an enum/newtype such as `DependencyReadinessPolicy`
    or an exact semantic reuse of `DependencyEnvironmentAction`, not a raw mode
    string or boolean. The request needs a validated wrapper that rejects
    unknown fields, path-shaped identity fields, executable handoff fields,
    identity/planning request mismatch, duplicate selected binding ids,
    unsupported contract versions, and missing or invalid required policy.
    Readiness input validation must not require `dependency_requirements_id` or
    `environment_ref`.
  - Rejected options: making explicit dependency-environment graph nodes the
    only canonical readiness path was rejected because it adds boilerplate and
    weakens automatic multi-model execution; moving immediately to full
    scheduler-owned dependency planning was deferred as the long-term target
    because it expands this slice into scheduler policy and resource/history
    planning; adapter conversions back to `ModelDependencyRequest` or
    `ModelRefV2` were rejected as legacy preservation.
  - No-fallback/no-legacy confirmation: the planned host service replaces the
    old resolver boundary instead of wrapping it. The migration sequence must
    delete `ModelDependencyResolver`, `ModelDependencyRequest`, `ModelRefV2`,
    `resolve_model_ref`, path-derived model-id repair, and path-shaped
    cache/activity/frontend dependency-environment contracts as their typed
    replacements land.
  - Next implementation slice: add only the shared
    `DependencyReadinessRequest` contract and canonical identity-key
    constructor in `pantograph-dependency-planning`, with serde fixtures,
    validation tests, and README traceability. Prefer a new focused
    `readiness.rs` module and `readiness_contract.rs` integration test if the
    readiness input contract would grow `preflight.rs` or the already-large
    `tests/contract.rs` further. Do not wire node-engine production behavior,
    implement embedded-runtime Pumas/package-manager I/O, migrate frontend DTOs,
    change scheduler policy, add lockfile changes, or preserve old resolver
    conversions in that slice.
  - Following vertical slice: wire the node-engine `DependencyReadinessHost`
    boundary and the minimal embedded-runtime host registration/implementation
    needed for a real producer-to-consumer path. That slice must use typed
    errors/diagnostics rather than `Result<T, String>`, fail closed when the
    host is missing, and avoid leaving the new service as dead migration code
    or keeping the old resolver as a successful alternate branch. It needs at
    least one full-path acceptance test from graph dependency-planning input to
    node-engine readiness request to embedded-runtime host
    `DependencyPreflightResult`, plus focused missing-host and host-returned
    unavailable diagnostics. Any durable dependency-environment state in tests
    must use per-test roots rather than shared sqlite/cache paths.
  - Cleanup targets for the resolver deletion sequence explicitly include
    node-engine public re-exports, `core_executor/dependency_preflight`
    legacy input projection, direct audio/llamacpp/pytorch `ModelRefV2`
    consumers, embedded-runtime runtime extension snapshots and lifecycle
    registration, dependency-environment executor helpers, dependency activity
    events, model-dependency tests/stubs, frontend dependency-environment
    DTO/action/activity matcher files, and saved or mock workflow fixtures that
    still carry successful `modelPath`/`model_path` dependency identity.
  - Standards iteration result: the host-owned readiness re-plan remains
    standards-compliant only with the typed-policy, parse-once validation,
    serde fixture, decomposition, and cross-layer acceptance gates above. If a
    slice cannot meet those gates, stop and re-plan rather than adding a
    path-shaped adapter or compatibility branch.
- 2026-05-22 Milestone 5 dependency readiness contract slice completed:
  - Slice/write set: added the shared host input contract in
    `crates/pantograph-dependency-planning/src/readiness.rs`, public re-exports
    in `lib.rs`, the canonical
    `DependencyPlanningIdentityKey::from_planning_request` constructor and
    shared field-rejection helpers in `preflight.rs`, README updates, the
    `dependency_readiness_request.json` fixture, and focused
    `tests/readiness_contract.rs` coverage. This was serial integration-owner
    work because it changes a shared Rust contract and plan files.
  - No-fallback/no-legacy result: the slice did not wire node-engine or
    embedded-runtime production behavior, did not call Pumas/package-manager
    I/O, did not migrate frontend DTOs, and did not add adapters back to
    `ModelDependencyRequest`, `ModelRefV2`, or path-shaped `model_path`
    identity.
  - Contract behavior: readiness input is path-free and proof-free. It carries
    `DependencyPlanningIdentityKey`, `DependencyPlanningRequest`, and typed
    `DependencyReadinessPolicy`; validation rejects unknown fields,
    path-shaped fields, executable handoff fields, mismatched identity,
    duplicate selected binding ids, unsupported contract versions, and missing
    policy, while not requiring `dependency_requirements_id` or
    `environment_ref`.
  - Standards evidence: `readiness.rs` keeps the new responsibility out of
    `preflight.rs`; file-size review after the slice is `preflight.rs` 418
    lines, `readiness.rs` 100 lines, `tests/contract.rs` 794 lines unchanged,
    and `tests/readiness_contract.rs` 199 lines. Source/test/fixture READMEs
    were updated in the same slice.
  - Verification passed:
    `cargo fmt -p pantograph-dependency-planning`,
    `cargo test -p pantograph-dependency-planning`,
    `cargo check -p pantograph-dependency-planning`,
    `cargo check -p pantograph-dependency-planning --all-features`,
    `cargo check -p pantograph-dependency-planning --no-default-features`,
    `cargo fmt -p pantograph-dependency-planning -- --check`, and
    `git diff --check`. Initial test compile exposed one local type inference
    issue and one `#[must_use]` test warning; both were fixed before the final
    verification pass.
  - Remaining follow-up: the next implementation slice is the real vertical
    host path: node-engine `DependencyReadinessHost` boundary plus minimal
    embedded-runtime host registration/implementation and full-path acceptance
    test, while deleting migrated old resolver helpers instead of preserving
    them as successful fallback branches.
- 2026-05-22 scheduler-owned dynamic task dispatch re-plan:
  - Boundary corrected: the scheduler target is not a single static
    whole-workflow plan. Pantograph has concurrent users and workflows may
    pause between DAG tasks while other workflow tasks run, batch, or wait for
    resources. Ready workflow nodes should become schedulable task units owned
    by a durable scheduler queue.
  - Decision: add `08-scheduler-owned-dynamic-task-dispatch.md` and inserted
    Milestone 5a. The design abstracts technical complexity away from the graph
    editor and node-engine by keeping them on typed intent, capability hints,
    task state, and diagnostics. Scheduler policy owns queueing, batching,
    fairness, resource admission, runtime/device selection, dependency
    readiness action, retry/defer/fail decisions, and dispatch timing.
  - Concern separation: graph editor composes intent and renders backend-owned
    capability/status facts; node-engine validates graph semantics and submits
    path-free task intent; capability service answers what is possible; the
    scheduler decides what runs now and where; dependency readiness prepares or
    checks environments under scheduler policy; runtime host consumes a
    short-lived dispatch plan; diagnostics/history records typed outcomes.
  - Milestone impact: Milestone 5 remains the enabling contract/runtime/device
    replacement work. Milestone 5a owns the dynamic scheduler architecture:
    capability hints, schedulable task intent, scheduler queue state, dispatch
    decisions, resource/residency manager, batching policy, dependency
    readiness integration, runtime host handoff, and legacy resolver/path
    deletion.
  - No-fallback/no-legacy confirmation: the re-plan does not approve
    `ModelDependencyResolver`, `ModelDependencyRequest`, `ModelRefV2`,
    `model_path`, or frontend `modelPath` dependency actions as successful
    alternate paths. They remain deletion/replacement targets under the new
    scheduler-owned dispatch milestone.
  - Documentation updates: updated `plan.md`, `README.md`, `04-milestones.md`,
    `milestones/README.md`, Milestone 5 notes, and Milestone 6 gating notes;
    added `08-scheduler-owned-dynamic-task-dispatch.md` and
    `milestones/05a-scheduler-owned-dynamic-task-dispatch.md`.
- 2026-05-22 Milestone 5a standards iteration:
  - Reviewed the scheduler-owned dynamic task dispatch plan against
    `PLAN-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
    `CONCURRENCY-STANDARDS.md`, `TESTING-STANDARDS.md`,
    `DOCUMENTATION-STANDARDS.md`, `SECURITY-STANDARDS.md`,
    `CROSS-PLATFORM-STANDARDS.md`, `RUST-API-STANDARDS.md`,
    `RUST-ASYNC-STANDARDS.md`, and `RUST-CROSS-PLATFORM-STANDARDS.md`.
  - Required plan updates: the first Milestone 5a slice must establish one
    scheduler-owned implementation boundary; raw graph/IPC/saved-workflow/
    queue payloads must parse once into validated Rust contracts; core
    ranking/admission policy remains synchronous unless I/O requires an async
    shell; long-running queue, readiness, observation, and dispatch workers
    need a single lifecycle owner with bounded queues, cancellation, shutdown,
    panic handling, and reservation cleanup.
  - Cross-platform/resource update: resource observation must use a
    platform-neutral observer trait with thin Linux, Windows, and macOS modules,
    typed unavailable/error diagnostics, deterministic fake tests first, and
    target compile checks before production collectors become required.
  - Verification update: Milestone 5a now requires boundary validation,
    replay/idempotency, duplicate-dispatch prevention, reservation release,
    lifecycle shutdown, cross-layer vertical-slice, multi-workflow batching, and
    feature-matrix checks for touched public crates.
  - No-fallback confirmation: implementation may remove retired systems
    entirely. It must not preserve scheduler policy in frontend/Tauri/
    node-engine/runtime adapters, scatter platform collection through business
    logic, or keep `ModelDependencyResolver`, `ModelDependencyRequest`,
    `ModelRefV2`, `model_path`, or frontend `modelPath` as successful
    alternate paths.
- 2026-05-22 Milestone 5a boundary crate slice completed:
  - Smallest useful vertical slice: establish a dedicated scheduler-owned crate
    before adding queue, policy, resource admission, batching, or dispatch
    behavior.
  - Allowed write set: root `Cargo.toml`, `Cargo.lock`, `crates/README.md`,
    `crates/pantograph-scheduler/`, Milestone 5a plan notes, and this execution
    log.
  - Implementation: added `pantograph-scheduler` to workspace members and
    default members, added crate README and crate-level docs, and added typed
    scheduler ownership-boundary enums plus tests proving graph/editor,
    node-engine, frontend/Tauri adapters, runtime adapters, runtime host,
    dependency readiness service, capability service, and diagnostics ledger do
    not own scheduler capabilities.
  - No-fallback/no-legacy confirmation: the slice does not touch legacy
    execution paths and does not preserve `ModelDependencyResolver`,
    `ModelDependencyRequest`, `ModelRefV2`, graph-visible `model_path`, or
    frontend `modelPath` as successful alternate paths.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`, and
    `cargo fmt -p pantograph-scheduler -- --check`.
  - Remaining follow-up: continue Milestone 5a with path-free schedulable task
    intent contracts in `pantograph-scheduler` and focused boundary validation
    tests.
- 2026-05-22 Milestone 5a schedulable task intent contract slice completed:
  - Smallest useful vertical slice: add a validated, path-free task intent DTO
    for one ready workflow DAG node without adding queue execution behavior.
  - Allowed write set: `Cargo.lock`, `crates/pantograph-scheduler/`, Milestone
    5a plan notes, and this execution log.
  - Implementation: split the scheduler crate into `ownership`, `intent`, and
    `error` modules; added `SchedulableTaskIntent`,
    `ValidatedSchedulableTaskIntent`, validated workflow/run/node/task/fairness
    ids, optional hard runtime/device constraints, typed trait setting values,
    bounded estimate hints, and serde fixture tests.
  - No-fallback/no-legacy confirmation: top-level `model_path` is rejected at
    the serde boundary; the scheduler intent contract carries Pumas model refs
    and typed intent only. It does not expose executable Pumas load targets,
    local load paths, `ModelDependencyRequest`, `ModelRefV2`, frontend
    `modelPath`, or worker launch facts.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`, and
    `cargo fmt -p pantograph-scheduler -- --check`.
  - Remaining follow-up: continue Milestone 5a with backend-owned capability
    hints for graph editor and option-provider consumers.
- 2026-05-22 Milestone 5a capability hint contract slice completed:
  - Smallest useful vertical slice: add backend-owned capability hint contracts
    and validation in `pantograph-scheduler` without wiring frontend or option
    provider consumers.
  - Allowed write set: `crates/pantograph-scheduler/`, Milestone 5a plan notes,
    and this execution log.
  - Implementation: added `SchedulerCapabilityHintSnapshot`,
    `ValidatedSchedulerCapabilityHintSnapshot`, availability states, runtime
    hints, device hints, trait option hints, option values, diagnostic
    severities, and typed diagnostic codes with fixture-backed serde tests.
  - No-fallback/no-legacy confirmation: capability hints expose possibilities
    and diagnostics only. Tests reject final `selected_runtime_id` and
    `local_load_path` fields so the graph/editor side cannot receive final
    scheduler decisions, executable Pumas load targets, local paths,
    `ModelDependencyRequest`, `ModelRefV2`, graph `model_path`, frontend
    `modelPath`, reservations, batching groups, or worker launch facts.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`, and
    `cargo fmt -p pantograph-scheduler -- --check`.
  - Remaining follow-up: continue Milestone 5a by replacing node-engine
    dependency preflight output with typed readiness proof rather than
    `ModelRefV2`.
- 2026-05-22 Milestone 5a re-plan trigger before node-engine preflight
  replacement:
  - Finding: existing node-engine and embedded-runtime dependency preflight
    functions still return `Option<ModelRefV2>` and current runtime input
    assembly inserts that value into Python runtime inputs. Directly replacing
    that output with `DependencyPreflightResult` before scheduler dispatch and
    runtime host handoff exist would either break the execution path or require
    a compatibility conversion back into `ModelRefV2`.
  - No-fallback/no-legacy impact: a conversion from typed readiness proof back
    into `ModelRefV2` would preserve the retired successful path and is not
    allowed.
  - Required re-plan: insert an ordering seam before node-engine replacement.
    The next implementation slice should create scheduler-owned readiness
    admission/handoff contracts that runtime hosts can consume without
    `ModelRefV2`; after that seam exists, replace node-engine preflight output
    and delete the legacy resolver/model-ref production path instead of
    adapting it.
- 2026-05-22 Milestone 5a re-plan decision:
  - Decision: use Option 3. Insert scheduler-owned readiness admission and a
    non-legacy runtime handoff seam before replacing node-engine dependency
    preflight output.
  - Smallest next useful vertical slice: define readiness admission contracts
    from validated `SchedulableTaskIntent` to typed ready/defer/fail results,
    with dependency readiness proof when ready and no executable Pumas load
    targets, local paths, `ModelDependencyRequest`, or `ModelRefV2`.
  - Follow-on slice: define the runtime handoff seam runtime/execution hosts
    can consume without converting readiness proof back to `ModelRefV2`.
  - No-fallback/no-legacy confirmation: this reordering is not a compatibility
    bridge. The legacy preflight producer remains a deletion target after the
    host-facing non-legacy input exists.
  - Forward boundary audit through Milestone 8: likely future re-plan
    checkpoints are scheduler queue persistence ownership, resource observer
    and reservation ownership, scheduler-owned dependency readiness policy,
    runtime-host-only Pumas load-target resolution, batching/fairness across
    concurrent workflow runs, capability hint projection without final
    scheduler decisions, and deterministic Milestone 8 release validation
    assumptions. Each checkpoint must replace stale systems or stop for a
    focused re-plan; none may preserve retired resolver/path behavior.
- 2026-05-22 Milestone 5a readiness admission contract slice completed:
  - Smallest useful vertical slice: add scheduler-owned readiness admission
    request/decision contracts and validation in `pantograph-scheduler`,
    without wiring node-engine or runtime host execution.
  - Allowed write set: `crates/pantograph-scheduler/`, Milestone 5a plan
    notes, and this execution log.
  - Implementation notes: added `SchedulerReadinessAdmissionRequest`,
    `SchedulerReadinessAdmissionDecision`,
    `SchedulerDependencyReadinessProof`, typed readiness admission state,
    action, diagnostic severity/code, validated wrappers, public exports,
    README coverage, and a JSON fixture-backed public contract test suite.
  - No-fallback/no-legacy confirmation: ready admission requires matching
    path-free `DependencyPreflightResult` proof; deferred and terminal failed
    states require typed diagnostics and cannot carry ready proof. The new
    contract does not expose executable Pumas load targets, local paths,
    `ModelDependencyRequest`, `ModelRefV2`, graph `model_path`, frontend
    `modelPath`, selected runtime/device dispatch decisions, reservations,
    batching groups, or worker launch facts.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`, and
    `cargo fmt -p pantograph-scheduler -- --check`.
  - Remaining follow-up: continue Milestone 5a with the non-legacy runtime
    handoff seam so runtime/execution hosts can consume readiness admission
    without converting back to `ModelRefV2`.
- 2026-05-22 Milestone 5a runtime handoff seam contract slice completed:
  - Smallest useful vertical slice: add path-free `SchedulerRuntimeHandoff`
    contracts and validation in `pantograph-scheduler`, without wiring
    node-engine or runtime host execution.
  - Allowed write set: `crates/pantograph-scheduler/`, Milestone 5a plan
    notes, and this execution log.
  - Implementation notes: added `SchedulerRuntimeHandoff`,
    `SchedulerRuntimeHandoffState`, `SchedulerRuntimeHandoffSelection`,
    `ValidatedSchedulerRuntimeHandoff`, public exports, README coverage, and a
    JSON fixture-backed public contract test suite.
  - No-fallback/no-legacy confirmation: the handoff carries correlation ids,
    validated task intent, scheduler-owned readiness proof, matching
    dependency environment ref, and optional scheduler dispatch selection only.
    It rejects path/load-target fields, validates top-level correlation against
    task intent, validates environment refs against readiness proof, requires
    dispatch selection only in dispatch-selected state, and enforces explicit
    runtime/device hard requirements when dispatch selection is present. It
    does not expose executable Pumas load targets, local paths,
    `ModelDependencyRequest`, `ModelRefV2`, graph `model_path`, frontend
    `modelPath`, reservations, batching groups, or worker launch facts.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`, and
    `cargo fmt -p pantograph-scheduler -- --check`.
  - Remaining follow-up: replace node-engine dependency preflight output with
    typed readiness proof after this non-legacy host handoff seam, then delete
    the legacy `ModelRefV2` preflight production path without a bridge.
- 2026-05-22 Milestone 5a re-plan trigger before node-engine preflight output
  replacement:
  - Finding: the next source replacement is broader than the preflight return
    type. `enforce_dependency_preflight` still returns `Option<ModelRefV2>`,
    but PyTorch, llama.cpp, and audio execution also read `model_path` inputs
    for successful model loading and emit `ModelRefV2` outputs through
    `build_model_ref_v2`. Embedded-runtime dependency preflight still resolves
    `ModelRefV2` as well.
  - No-fallback/no-legacy impact: changing only the preflight output to
    `DependencyPreflightResult` would not remove the successful
    `model_path`/`ModelRefV2` execution path. Converting
    `SchedulerRuntimeHandoff` or readiness proof back into `ModelRefV2` would
    be a compatibility bridge and is not allowed.
  - Required re-plan: choose the source replacement sequence for runtime-host
    load-target resolution and node-engine legacy removal before editing
    node-engine execution. Viable options are: replace the runtime host path so
    host dispatch consumes scheduler handoff and resolves Pumas-approved load
    targets at the runtime boundary, then remove `ModelRefV2`/`model_path`
    successful execution paths; temporarily fail closed for affected runtime
    nodes until host dispatch is wired; or split a new milestone if the host
    replacement is too broad for Milestone 5a. Do not implement a
    `SchedulerRuntimeHandoff` to `ModelRefV2` adapter.
- 2026-05-22 Milestone 5b runtime-host legacy-removal re-plan decision:
  - Decision: use Option 3 planning structure with Option 1 implementation
    direction. Split the source replacement work into
    `milestones/05b-runtime-host-handoff-legacy-removal.md` and
    `09-runtime-host-handoff-legacy-removal.md`.
  - Scope change: Milestone 5a keeps scheduler-owned contracts and dynamic
    dispatch work. Milestone 5b owns runtime-host execution request/response,
    Pumas load-target resolution at the host boundary, PyTorch/llama.cpp/audio
    migration off graph `model_path`, node-engine preflight replacement, and
    deletion of `ModelDependencyResolver`, `ModelDependencyRequest`,
    `ModelRefV2`, `build_model_ref_v2`, frontend `modelPath` actions, and
    path-shaped success fixtures.
  - No-fallback/no-legacy confirmation: fail-closed behavior is allowed only
    when canonical scheduler handoff is missing, and must emit typed
    diagnostics. No scheduler-handoff-to-`ModelRefV2` adapter, path repair, or
    alternate successful legacy branch is permitted.
  - Remaining follow-up: before the next source slice, choose the smallest
    Milestone 5a or 5b item that can be completed without editing outside its
    allowed write set. If continuing legacy removal, start Milestone 5b with
    the host execution input contract before touching runtime loading code.
- 2026-05-22 Milestone 5a durable scheduler queue state slice completed:
  - Smallest useful vertical slice: add a pure durable queue-state contract and
    idempotent transition replay helper in `pantograph-scheduler`, without
    introducing a worker, persistence adapter, runtime dispatch, or lifecycle
    owner.
  - Allowed write set: `crates/pantograph-scheduler/`, Milestone 5a plan
    notes, and this execution log.
  - Implementation notes: added `SchedulerQueueTaskState`,
    `SchedulerQueueTaskRecord`, `SchedulerQueueTransition`,
    `SchedulerQueueTransitionId`, validated wrappers,
    `SchedulerQueueTransitionApplyResult`, and
    `apply_scheduler_queue_transition`, plus README coverage and a JSON
    fixture-backed public contract test suite.
  - No-fallback/no-legacy confirmation: queue records and transitions carry
    task correlation, path-free task intent, queue state, state version, and
    transition id only. They reject path/load-target fields and do not expose
    executable Pumas load targets, local paths, `ModelDependencyRequest`,
    `ModelRefV2`, graph `model_path`, frontend `modelPath`, reservations,
    batching groups, worker launch facts, or runtime host execution data.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`,
    `cargo fmt -p pantograph-scheduler -- --check`, and `git diff --check`.
  - Remaining follow-up: add typed scheduler task lifecycle diagnostics so
    graph editor and run inspection can explain waiting, deferred, unavailable,
    failed, and completed task states without frontend inference.
- 2026-05-22 Milestone 5a typed scheduler task lifecycle diagnostics slice
  completed:
  - Smallest useful vertical slice: add backend-owned lifecycle diagnostic
    snapshot contracts in `pantograph-scheduler` for graph/run inspection,
    without adding frontend projection, persistence, workers, resource
    observation, batching, or runtime dispatch.
  - Allowed write set: `crates/pantograph-scheduler/`, Milestone 5a plan
    notes, and this execution log.
  - Implementation notes: added `SchedulerTaskLifecycleDiagnosticSnapshot`,
    `SchedulerTaskLifecycleDiagnostic`,
    `SchedulerTaskLifecycleDiagnosticCode`,
    `SchedulerTaskLifecycleDiagnosticSeverity`,
    `ValidatedSchedulerTaskLifecycleDiagnosticSnapshot`, public exports,
    README coverage, and a JSON fixture-backed public contract test suite.
  - No-fallback/no-legacy confirmation: lifecycle snapshots carry task
    correlation, queue state, and typed state-compatible diagnostics only.
    They reject path/runtime-host fields and do not expose executable Pumas
    load targets, local paths, `ModelDependencyRequest`, `ModelRefV2`, graph
    `model_path`, frontend `modelPath`, reservations, batching groups, worker
    launch facts, or scheduler policy internals.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`,
    `cargo fmt -p pantograph-scheduler -- --check`, and `git diff --check`.
  - Remaining follow-up: add one scheduler lifecycle owner for long-running
    queue workers, dependency readiness actions, resource observation loops,
    and runtime host dispatch.
- 2026-05-22 Milestone 5a scheduler lifecycle supervision contract slice
  completed:
  - Smallest useful vertical slice: add the scheduler-owned lifecycle
    supervision contract in `pantograph-scheduler`, without spawning workers,
    adding async runtime code, wiring persistence, or implementing resource,
    dependency, retry, reservation, or runtime-host loops.
  - Allowed write set: `crates/pantograph-scheduler/`, Milestone 5a plan
    notes, and this execution log.
  - Implementation notes: added `SchedulerLifecycleOwnerSnapshot`,
    `SchedulerLifecycleComponentSnapshot`, component/state/cancellation/panic
    enums, queue-bound contract, lifecycle diagnostics, validated wrapper,
    public exports, README coverage, and a JSON fixture-backed public contract
    test suite.
  - No-fallback/no-legacy confirmation: lifecycle supervision records one
    owner for queue worker, dependency-readiness action, resource-observation
    loop, runtime-host dispatch, retry loop, and reservation cleanup
    components. It requires bounded queues where work can accumulate, validates
    cancellation, shutdown/stop, panic, failure, and diagnostic state, and
    rejects path/runtime internals. It does not expose executable Pumas load
    targets, local paths, `ModelDependencyRequest`, `ModelRefV2`, graph
    `model_path`, frontend `modelPath`, scheduler dispatch decisions, batching
    groups, worker launch facts, or runtime host execution data.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`,
    `cargo fmt -p pantograph-scheduler -- --check`, and `git diff --check`.
  - Remaining follow-up: define the scheduler dispatch decision contract
    without leaking executable load targets or graph/runtime legacy identity.
- 2026-05-22 Milestone 5a scheduler dispatch decision contract slice
  completed:
  - Smallest useful vertical slice: add scheduler-selected dispatch decision
    contracts in `pantograph-scheduler`, without wiring runtime host
    execution, resolving Pumas load targets, implementing resource reservation
    storage, or adding batching policy.
  - Allowed write set: `crates/pantograph-scheduler/`, Milestone 5a plan
    notes, and this execution log.
  - Implementation notes: added `SchedulerDispatchDecision`,
    `SchedulerRuntimeVariantId`, `SchedulerBatchingGroupId`,
    `SchedulerReservationLeaseId`, `SchedulerDispatchDiagnostic`,
    dispatch diagnostic enums, validated wrapper, public exports, README
    coverage, and a JSON fixture-backed public contract test suite.
  - No-fallback/no-legacy confirmation: dispatch decisions carry correlation
    ids, path-free task intent, selected runtime/runtime variant, selected
    device set, selected Pumas model/artifact identity, dependency readiness
    proof, environment ref, optional batching group, reservation lease id,
    runtime trait projection, and typed diagnostics only. They validate hard
    runtime/device requirements, selected model consistency, dependency
    environment proof, duplicate devices, and bounded diagnostics while
    rejecting executable load targets, local paths, `ModelDependencyRequest`,
    `ModelRefV2`, graph `model_path`, frontend `modelPath`, worker launch
    facts, and runtime host internals.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`,
    `cargo fmt -p pantograph-scheduler -- --check`, and `git diff --check`.
  - Remaining follow-up: add the resource/residency manager abstraction for
    admission-time snapshots, reservations, residency, runtime readiness, and
    typed impossible-fit diagnostics.
- 2026-05-22 Milestone 5a resource/residency manager contract slice completed:
  - Smallest useful vertical slice: add the platform-neutral
    resource/residency snapshot and observer trait contracts in
    `pantograph-scheduler`, without implementing OS collectors, dispatch
    policy, reservation storage, batching policy, or runtime host execution.
  - Allowed write set: `crates/pantograph-scheduler/`, Milestone 5a plan
    notes, and this execution log.
  - Implementation notes: added `SchedulerResourceResidencySnapshot`,
    `SchedulerResourceObserver`, device-resource snapshots, active
    reservation records, runtime-readiness records, model-residency records,
    load/warmup estimates, batching memory impact, fit assessments, typed
    resource diagnostics, validated wrapper, public exports, README coverage,
    and a JSON fixture-backed public contract test suite.
  - No-fallback/no-legacy confirmation: the snapshot carries scheduler-owned
    resource and residency facts only. It validates checked byte arithmetic,
    duplicate device-resource observations, duplicate reservation leases,
    unavailable runtime/residency diagnostics, and impossible-fit diagnostics
    while rejecting executable load targets, local paths,
    `ModelDependencyRequest`, `ModelRefV2`, graph `model_path`, frontend
    `modelPath`, worker launch facts, and runtime host internals.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`,
    `cargo fmt -p pantograph-scheduler -- --check`, `git diff --check`, and
    file-size standards check for the new resource modules.
  - Remaining follow-up: move dependency readiness policy into scheduler
    admission/dispatch so node-engine cannot perform dependency resolver
    discovery as an execution fallback.
- 2026-05-22 Milestone 5a scheduler dependency-readiness policy slice
  completed:
  - Smallest useful vertical slice: add a pure scheduler readiness policy
    function in `pantograph-scheduler` that maps a validated admission request
    plus optional host preflight result into check, install-missing, defer,
    retry, fail, or admit decisions, without wiring node-engine execution or
    dependency service calls.
  - Allowed write set: `crates/pantograph-scheduler/`, Milestone 5a plan
    notes, and this execution log.
  - Implementation notes: added `plan_scheduler_readiness_admission`, retryable
    readiness admission state/action variants, validation for retryable
    decisions, public exports, README coverage, and focused policy tests for
    no preflight, ready proof, missing dependency install/defer policy,
    retryable failed readiness, terminal unavailable readiness, and mismatched
    proof diagnostics.
  - No-fallback/no-legacy confirmation: scheduler policy now owns the readiness
    action decision and returns typed admission diagnostics for missing,
    failed, unavailable, not-implemented, unsupported, or mismatched preflight
    states. The slice does not call `ModelDependencyResolver`, build
    `ModelDependencyRequest`, produce `ModelRefV2`, expose graph `model_path`
    or frontend `modelPath`, resolve executable Pumas load targets, or add a
    node-engine fallback branch.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`,
    `cargo fmt -p pantograph-scheduler -- --check`, `git diff --check`, and
    file-size standards check for modified scheduler readiness files.
  - Remaining follow-up: add batching policy surface for compatible tasks
    across workflow runs.
- 2026-05-22 Milestone 5a scheduler batching policy surface slice completed:
  - Smallest useful vertical slice: add scheduler-owned batching candidate and
    policy decision contracts in `pantograph-scheduler`, without implementing
    runtime execution, an optimizer, queue workers, or host handoff wiring.
  - Allowed write set: `crates/pantograph-scheduler/`, Milestone 5a plan
    notes, and this execution log.
  - Implementation notes: added `SchedulerBatchPolicyDecision`,
    `SchedulerBatchCandidate`, `SchedulerBatchMemoryImpact`, batch policy
    state, diagnostics, validated wrapper, public exports, README coverage, and
    a JSON fixture-backed public contract test suite for compatible
    cross-workflow-run batches.
  - No-fallback/no-legacy confirmation: batching decisions carry task
    correlation, path-free task intent, task family, selected runtime, selected
    device set, selected Pumas model ref, residency state, input shape
    signature, latency, memory impact, batch sizing, and typed diagnostics
    only. They validate compatibility across workflow runs, checked memory
    totals, duplicate candidates, rejected diagnostics, and batch size bounds
    while rejecting local paths, executable load targets,
    `ModelDependencyRequest`, `ModelRefV2`, graph `model_path`, frontend
    `modelPath`, worker launch facts, and runtime host internals.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`,
    `cargo fmt -p pantograph-scheduler -- --check`, `git diff --check`, and
    file-size standards check for new scheduler batching files.
  - Remaining follow-up: wire runtime/execution host handoff through dispatch
    decisions.
- 2026-05-22 Milestone 5a runtime handoff dispatch-decision slice completed:
  - Smallest useful vertical slice: replace the parallel lightweight runtime
    handoff selection shape with the canonical `SchedulerDispatchDecision` in
    `SchedulerRuntimeHandoff`, without wiring runtime execution or resolving
    Pumas load targets.
  - Allowed write set: `crates/pantograph-scheduler/`, Milestone 5a plan
    notes, and this execution log.
  - Implementation notes: removed `SchedulerRuntimeHandoffSelection`,
    changed runtime handoff to carry optional `SchedulerDispatchDecision`,
    validated dispatch decision/task intent/readiness proof/environment
    correlation, updated public exports, README coverage, and runtime handoff
    tests for readiness-only, dispatch-selected, and mismatch cases.
  - No-fallback/no-legacy confirmation: runtime host handoff now carries
    dispatch-selected runtime/device/model/reservation/batch facts only through
    the scheduler dispatch decision. It validates handoff, task intent,
    readiness proof, and environment correlation while rejecting executable
    load targets, local paths, `ModelDependencyRequest`, `ModelRefV2`, graph
    `model_path`, frontend `modelPath`, worker launch facts, and the removed
    parallel dispatch-selection DTO.
  - Verification passed: `cargo fmt -p pantograph-scheduler`,
    `cargo test -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler`,
    `cargo check -p pantograph-scheduler --all-features`,
    `cargo check -p pantograph-scheduler --no-default-features`,
    `cargo fmt -p pantograph-scheduler -- --check`, `git diff --check`, and
    file-size standards check for modified scheduler handoff files.
  - Remaining follow-up: update documentation coverage and deletion tracking
    for any public contract or source directory gaps left in Milestone 5a.
- 2026-05-22 Milestone 5a scheduler documentation coverage slice completed:
  - Smallest useful vertical slice: add standards-compliant source and test
    README coverage for `pantograph-scheduler` after all Milestone 5a public
    contracts existed, without changing scheduler behavior.
  - Allowed write set: `crates/pantograph-scheduler/README.md`,
    `crates/pantograph-scheduler/src/README.md`,
    `crates/pantograph-scheduler/tests/README.md`,
    `crates/pantograph-scheduler/tests/fixtures/README.md`, Milestone 5a plan
    notes, and this execution log.
  - Implementation notes: documented the source-module contract families,
    host-facing API expectations, structured fixture semantics, lifecycle
    ownership boundaries, platform-neutral resource observer boundary, and
    no-legacy invariants for scheduler tests.
  - No-fallback/no-legacy confirmation: documentation records that scheduler
    source/tests reject executable load targets, local paths,
    `ModelDependencyRequest`, `ModelRefV2`, graph `model_path`, frontend
    `modelPath`, worker launch facts, and compatibility DTOs instead of
    preserving them.
  - Verification passed: `git diff --check`; README section review against
    documentation standards; source/test directory coverage review for
    `crates/pantograph-scheduler/src/` and
    `crates/pantograph-scheduler/tests/` plus structured fixture coverage
    review for `crates/pantograph-scheduler/tests/fixtures/`.
  - Remaining follow-up: decide the final Milestone 5a retired-path deletion
    boundary before editing non-scheduler source files.
- 2026-05-22 Milestone 5a/5b retired-path boundary planning slice completed:
  - Smallest useful vertical slice: update the plans to close Milestone 5a's
    deletion item as a re-plan decision, not as implementation of deletion.
  - Allowed write set: Milestone 5a plan, Milestone 5b plan, runtime-host
    replacement plan, and this execution log.
  - Decision: use Option 2. Milestone 5a is scheduler-contract complete;
    Milestone 5b owns actual removal of `ModelDependencyResolver`,
    `ModelDependencyRequest`, `ModelRefV2`, graph `model_path`, frontend
    `modelPath`, path repair helpers, and path-shaped success fixtures as a
    hard gate before real image-generation execution can depend on runtime
    loading.
  - Sequencing requirement: Milestone 5b must start with the runtime-host
    execution request/response contract, then host-owned Pumas load-target
    resolution, then runtime migrations and deletion.
  - No-fallback/no-legacy confirmation: the plan rejects
    scheduler-handoff-to-`ModelRefV2` adapters, path repair, compatibility
    branches, and deleting legacy before a canonical replacement path exists.
  - Verification: documentation-only plan review and `git diff --check`.
- 2026-05-22 Milestone 5b runtime-host request/response contract slice
  completed:
  - Smallest useful vertical slice: add the embedded-runtime host-facing
    execution request/response DTOs, validated wrappers, typed diagnostics,
    and JSON fixtures without resolving Pumas load targets or launching
    runtimes.
  - Allowed write set: `crates/pantograph-embedded-runtime/`, Milestone 5b
    plan notes, and this execution log.
  - Implementation notes: added `RuntimeHostExecutionRequest`,
    `RuntimeHostExecutionResponse`, typed response state and diagnostics,
    validated wrappers, public exports, fixture-backed tests, and README
    coverage for the new host boundary.
  - No-fallback/no-legacy confirmation: the request consumes a
    dispatch-selected `SchedulerRuntimeHandoff` and rejects readiness-only
    handoff; request/response contracts expose no executable load target,
    local path, `ModelDependencyRequest`, `ModelRefV2`, graph `model_path`,
    frontend `modelPath`, path repair, reservation/batching internals, or
    worker launch details.
  - Verification passed: `cargo fmt -p pantograph-embedded-runtime`,
    `cargo test -p pantograph-embedded-runtime runtime_host_execution`,
    `cargo check -p pantograph-embedded-runtime`,
    `cargo check -p pantograph-embedded-runtime --all-features`,
    `cargo check -p pantograph-embedded-runtime --no-default-features`,
    `cargo fmt -p pantograph-embedded-runtime -- --check`,
    `git diff --check`, README coverage review, and source/test fixture
    directory coverage review, and file-size standards check for new
    runtime-host source/test files.
  - Remaining follow-up: add host-owned Pumas load-target resolution service.
- 2026-05-22 Milestone 5b host-owned Pumas load-target resolution slice
  completed:
  - Smallest useful vertical slice: add the embedded-runtime host-only load
    target resolver module that builds Pumas requests from validated
    runtime-host execution requests and maps ready/unavailable Pumas responses
    into host-owned results without wiring runtime execution.
  - Allowed write set: `crates/pantograph-embedded-runtime/`, Milestone 5b
    plan notes, and this execution log.
  - Implementation notes: added request projection from
    `ValidatedRuntimeHostExecutionRequest` to Pumas
    `ResolveModelArtifactLoadTargetRequest`, host-only ready target mapping,
    typed unavailable diagnostics, and focused tests for selected model refs,
    ready responses, and unavailable responses.
  - No-fallback/no-legacy confirmation: the resolver uses scheduler-selected
    Pumas model/artifact identity and Pumas typed resolver states only; it does
    not accept graph `model_path`, frontend `modelPath`,
    `ModelDependencyRequest`, `ModelRefV2`, path repair, package-fact scraping,
    or alternate successful resolver branches.
  - Verification passed: `cargo fmt -p pantograph-embedded-runtime`,
    `cargo test -p pantograph-embedded-runtime runtime_host_load_target`,
    `cargo test -p pantograph-embedded-runtime runtime_host_execution`, crate
    check matrix, fmt check, diff checks, README coverage review, and file-size
    standards check.
  - Remaining follow-up before PyTorch migration: wire scheduler dispatch to
    call runtime-host execution directly from the actual dispatch-selected
    scheduler handoff.
- 2026-05-23 Milestone 5b scheduler-to-runtime-host re-plan slice completed:
  - Smallest useful vertical slice: update the plan to replace the prior
    reduced-plan launch direction with direct scheduler-to-runtime-host
    dispatch before touching runtime execution code.
  - Allowed write set: `04-milestones.md`,
    `08-scheduler-owned-dynamic-task-dispatch.md`,
    `09-runtime-host-handoff-legacy-removal.md`, the Milestone 5b plan, and
    this execution log.
  - Earlier decision: use option 3. Scheduler dispatch is the only successful
    caller of runtime-host execution, and it must build
    `RuntimeHostExecutionRequest` from the actual dispatch-selected
    `SchedulerRuntimeHandoff`. This ordering is superseded by the later option
    4 task-level scheduler orchestration re-plan below.
  - Boundary clarification: `WorkflowExecutionPlanNodeDecision` remains a
    reduced inspection/diagnostics projection. It must not be used to
    synthesize scheduler handoff, launch inference, or feed a backend-decision
    compatibility bridge.
  - No-fallback/no-legacy confirmation: retire node-engine planned-inference
    launch ownership after direct scheduler dispatch is wired; do not keep
    `PlannedInferenceExecutionHost`, `EmbeddedPlannedInferenceExecutionHost`,
    `ModelDependencyResolver`, `ModelRefV2`, or `model_path` fixtures as
    alternate successful execution branches.
  - Verification: documentation-only plan review and `git diff --check`.
- 2026-05-23 Milestone 5b scheduler-owned runtime-host execution dispatch
  port slice completed:
  - Smallest useful vertical slice: add the embedded-runtime scheduler
    dispatch port and dispatcher without wiring production execution yet.
  - Allowed write set: `crates/pantograph-embedded-runtime/`, Milestone 5b
    plan notes, and this execution log.
  - Implementation notes: added `RuntimeHostExecutionPort`,
    `SchedulerRuntimeHostDispatcher`, typed port/dispatch errors, request
    validation before port invocation, response correlation validation, public
    crate exports, README coverage, and focused tests.
  - No-fallback/no-legacy confirmation: the dispatcher can only build
    `RuntimeHostExecutionRequest` from a scheduler handoff. It exposes no
    API from `WorkflowExecutionPlan`, `WorkflowExecutionPlanNodeDecision`,
    `BackendExecutionDecision`, graph input, `ModelRefV2`, or `model_path`,
    rejects readiness-only handoff before the port is called, and does not
    wire or preserve planned-inference launch behavior.
  - Verification passed: `cargo fmt -p pantograph-embedded-runtime`,
    `cargo test -p pantograph-embedded-runtime runtime_host_dispatch`,
    `cargo check -p pantograph-embedded-runtime`,
    `cargo check -p pantograph-embedded-runtime --all-features`,
    `cargo check -p pantograph-embedded-runtime --no-default-features`,
    `cargo fmt -p pantograph-embedded-runtime -- --check`,
    `git diff --check`, README coverage review, and file-size standards
    check.
  - Remaining follow-up: wire scheduler dispatch to call the runtime-host
    execution port from the actual dispatch-selected scheduler handoff.
- 2026-05-23 option 4 task-level scheduler orchestration re-plan slice:
  - Smallest useful vertical slice: update the plan to make option 4 the
    target architecture before production runtime-host wiring continues.
  - Allowed write set: this plan directory only. No source, test, config,
    lockfile, generated, build-output, sqlite, or workflow fixture files are
    part of this slice.
  - Decision: add `10-task-level-scheduler-orchestration.md` and Milestone 5c
    as the required bridge between Milestone 5a scheduler contracts and the
    remaining Milestone 5b runtime-host legacy removal work.
  - Design effect: scheduler task state becomes the progress driver for
    workflow execution. Node-engine output demand and reduced
    `WorkflowExecutionPlanNodeDecision` remain non-authoritative for runtime
    launch. Runtime-host execution must receive actual dispatch-selected
    `SchedulerRuntimeHandoff` from task state.
  - No-fallback/no-legacy confirmation: the plan forbids handoff synthesis
    from reduced execution plans, preserving whole-workflow output demand as a
    runtime launch path, `ModelRefV2` bridges, graph `model_path`, frontend
    `modelPath`, and planned-inference launch as successful branches.
  - Verification passed: documentation standards review against plan,
    architecture, and concurrency standards; `git diff --check -- docs/plans/current-image-generation-graphs`.
- 2026-05-23 Milestone 5c path-free scheduler task graph projection slice
  completed:
  - Smallest useful vertical slice: add run-scoped task graph projection from
    validated workflow topology without changing execution behavior.
  - Allowed write set: `crates/pantograph-workflow-service/Cargo.toml`,
    `Cargo.lock`, workflow-service facade/README/test files,
    `workflow/task_graph.rs`, the Milestone 5c plan, the task-orchestration
    design note, and this execution log.
  - Implementation notes: workflow-service now depends on the canonical
    scheduler/dependency-planning contracts and exposes
    `workflow_scheduler_task_graph`. The projection emits one task per
    topology node, preserves dependency/input bindings, parses scheduler-owned
    workflow/run/node/task ids, creates `SchedulableTaskIntent` only for
    canonical inference nodes with valid `pumas_model_ref` and explicit
    `task_kind`, and carries typed projection diagnostics otherwise.
  - No-fallback/no-legacy confirmation: legacy `model_ref`, `model_path`,
    `ModelRefV2`, execution plans, backend projections, and Pumas executable
    load targets are not scheduler identity sources in this slice.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`,
    `cargo test -p pantograph-workflow-service workflow::tests::task_graph`,
    `cargo check -p pantograph-workflow-service`,
    `cargo check -p pantograph-workflow-service --all-features`, and
    `cargo check -p pantograph-workflow-service --no-default-features`.
  - Discovered issue: scheduler trait settings cannot currently represent
    floating-point generation options such as guidance scale. Future trait
    projection work must extend the typed scheduler trait value contract rather
    than silently dropping or stringifying floats.
- 2026-05-23 Milestone 5c active-run scheduler task-state bridge slice
  completed:
  - Smallest useful vertical slice: add workflow-service active-run storage
    and transition APIs for scheduler task queue records using the canonical
    `pantograph-scheduler` queue contracts.
  - Allowed write set: workflow-service scheduler store/readme/test files,
    the Milestone 5c plan, and this execution log.
  - Implementation notes: active runs now hold scheduler task records keyed by
    task id. Store APIs validate records, apply idempotent scheduler queue
    transitions, preserve replayed transition results, and reject records or
    transitions whose workflow run id does not match the active run.
  - No-fallback/no-legacy confirmation: workflow-service did not define a
    second state machine, did not reinterpret task state, and did not create
    runtime handoff, node-engine execution, or Pumas load-target identity from
    these records.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`,
    `cargo test -p pantograph-workflow-service scheduler::store::tests`, and
    `cargo check -p pantograph-workflow-service`,
    `cargo check -p pantograph-workflow-service --all-features`,
    `cargo check -p pantograph-workflow-service --no-default-features`,
    `cargo fmt -p pantograph-workflow-service -- --check`, and
    `git diff --check`.
  - Deviation/follow-up: the active-run task-state APIs are staged for the
    Milestone 5c orchestrator and currently carry scoped `#[allow(dead_code)]`
    markers. The next orchestrator/storage slice must consume or remove those
    markers instead of leaving a long-term unused API.
- 2026-05-23 Milestone 5c scheduler queue transition coverage slice
  completed:
  - Smallest useful vertical slice: prove the canonical scheduler task-state
    transition contract covers every state named by Milestone 5c before
    workflow-service durable replay and orchestrator consumption expand.
  - Allowed write set: `crates/pantograph-scheduler/tests/queue_state.rs`,
    `crates/pantograph-scheduler/tests/README.md`, the Milestone 5c plan,
    the task-orchestration design note, and this execution log.
  - Implementation notes: queue-state integration tests now cover the full
    `SchedulerQueueTaskState` matrix, initial pending creation, terminal-state
    closure, stale expected-state rejection, and idempotent replay through the
    public scheduler API.
  - No-fallback/no-legacy confirmation: this slice keeps the state machine in
    `pantograph-scheduler`, adds no node-engine or workflow-service execution
    branch, and does not synthesize runtime handoff, model paths, or Pumas load
    targets from task state.
  - Verification passed: `cargo test -p pantograph-scheduler --test
    queue_state`, `cargo check -p pantograph-scheduler`, `cargo check -p
    pantograph-scheduler --all-features`, `cargo check -p pantograph-scheduler
    --no-default-features`, `cargo fmt -p pantograph-scheduler -- --check`,
    and `git diff --check`.
- 2026-05-23 Milestone 5c scheduler task-state read-model slice completed:
  - Smallest useful vertical slice: add a path-free presentation read-model
    projection from durable scheduler queue records before route wiring or
    diagnostics-ledger joins.
  - Allowed write set: workflow-service workflow facade/module/test/readme
    files, the Milestone 5c plan, the task-orchestration design note, and this
    execution log.
  - Implementation notes: `workflow_scheduler_task_state_read_models`
    validates `SchedulerQueueTaskRecord` inputs and returns sorted
    presentation facts with workflow/run/node/task correlation, task type,
    model id, queue state, optional requested runtime/device constraints, and
    typed trait settings.
  - No-fallback/no-legacy confirmation: the read model does not expose raw
    task intent, transition ids, state versions, runtime handoff, executable
    Pumas load targets, worker launch details, `ModelRefV2`, `model_path`, or
    `local_load_path`, and it does not execute or dispatch tasks.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`,
    `cargo test -p pantograph-workflow-service
    workflow::tests::task_state_read_model`, `cargo check -p
    pantograph-workflow-service`, `cargo check -p pantograph-workflow-service
    --all-features`, and `cargo check -p pantograph-workflow-service
    --no-default-features`.
- 2026-05-23 Milestone 5c active-run task-state query slice completed:
  - Smallest useful vertical slice: expose the path-free scheduler task-state
    read models through a dedicated workflow-service query for active runs
    without changing queue item or scheduler snapshot DTOs.
  - Allowed write set: workflow-service task-state read-model module/tests,
    workflow facade exports, workflow README, the Milestone 5c plan, the
    task-orchestration design note, and this execution log.
  - Implementation notes:
    `workflow_get_scheduler_task_state_read_models` validates session/run ids,
    reads canonical active-run scheduler queue records from the session store,
    and projects them through `workflow_scheduler_task_state_read_models`.
  - No-fallback/no-legacy confirmation: the query returns presentation facts
    only. It does not expose runtime handoff, executable Pumas load targets,
    transition ids, state versions, raw task intent, worker launch details, or
    session-level admission internals.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`,
    `cargo test -p pantograph-workflow-service
    workflow::tests::task_state_read_model`, `cargo check -p
    pantograph-workflow-service`, `cargo check -p pantograph-workflow-service
    --all-features`, `cargo check -p pantograph-workflow-service
    --no-default-features`, and `cargo fmt -p pantograph-workflow-service
    -- --check`.
- 2026-05-23 Re-plan boundary reached before the scheduler task orchestrator:
  - The next implementation step needs a materialized task-result contract
    before code can safely replace whole-workflow output-node demand.
  - Reason: inference task intent may depend on upstream non-runtime graph
    tasks producing `PumasModelRef`, scalar settings, media refs, or
    diagnostics after run admission. Building scheduler intent directly from
    incomplete graph inputs would either fail valid composed workflows or
    preserve a legacy node-engine fallback path.
  - Required planning: define task result DTOs, dependency binding resolution,
    missing-input diagnostics, persistence/replay ownership, and the exact
    point where materialized values become a `SchedulableTaskIntent` for
    scheduler admission.
  - No-fallback/no-legacy confirmation: do not implement the orchestrator by
    demanding workflow output nodes, synthesizing runtime handoff from reduced
    execution plans, or treating graph-local model paths as materialized
    scheduler identity.
- 2026-05-23 Task materialization re-plan direction selected:
  - Decision: implement option 2 now with option 3 discipline. Add a typed
    `WorkflowSchedulerTaskResult` contract plus active-run result storage and
    dependency-to-input resolution before building the orchestrator.
  - Contract requirements: stable schema version, workflow/run/node/task
    correlation, typed result status, typed output variants, bounded
    diagnostics, invalid/unavailable states, and no executable paths or worker
    launch details.
  - Active-run storage is a staged implementation boundary only. The DTO and
    validation must be shaped so durable diagnostics-ledger persistence and
    replay can replace active-run storage later without changing graph editor,
    node-engine, scheduler, or runtime-host semantics.
  - Resolution rule: a runtime inference task becomes scheduler-admissible only
    after required upstream materialized values validate into canonical typed
    inputs such as `PumasModelRef`, scalar settings, media/artifact refs, and
    diagnostics. Missing, wrong-type, unavailable, invalid, or ambiguous
    materialized values must produce typed diagnostics and scheduler task
    state, not fallback execution.
  - Follow-up: durable event-sourced task-result replay remains the later
    option 3 objective after the orchestrator can operate against the typed
    materialization contract.
- 2026-05-23 Standards iteration for task-result materialization planning:
  - Reviewed the plan against the coding, plan, Rust API, testing,
    documentation, concurrency, and architecture standards.
  - Plan guardrails now require focused workflow-service modules, typed and
    validated public DTOs, explicit materialized value variants instead of
    incidental metadata maps, staged active-run storage that stays
    replay-ready, clear workflow-service/scheduler/node-engine/runtime-host
    ownership, no locks held across await points, bounded async orchestration,
    README updates, and focused contract/storage/binding/vertical tests.
  - Shared contracts, generated DTOs, saved workflow fixtures, lockfiles,
    README files, and plan files remain serial integration-owner work; any
    sub-agent work must have non-overlapping adapter or test write sets.
- 2026-05-23 Milestone 5c graph-visible scheduler constraint alignment slice
  completed:
  - Smallest useful vertical slice: align optional scheduler runtime/device
    constraints across the canonical inference node descriptor,
    workflow-service task graph projection, backend port-option query context,
    frontend selection-input provider context/cache, and UniFFI smoke fixture.
  - Allowed write set: `workflow-nodes` inference descriptor/tests,
    `node-engine` port-option context/tests, workflow-service task graph test,
    focused frontend selection-input/cache types/tests, frontend mock node
    definition, UniFFI runtime test fixture, Milestone 5c plan, task-level
    orchestration plan, and this execution log.
  - Implementation notes: added the optional `device` port beside `runtime` on
    `llm-inference`; task graph projection now has test coverage proving
    `device` becomes `requested_device_id`; port-option context now uses
    `requestedRuntimeId` and `requestedDeviceId` instead of `backendId` and
    `runtimeVariantId`.
  - No-fallback/no-legacy confirmation: this slice only carries typed graph
    constraints into scheduler-facing planning/display contracts. It does not
    expose executable Pumas load targets, revive `backend_key` as graph-visible
    scheduler policy, synthesize runtime handoff, or route runtime inference
    through node-engine output demand.
  - Verification passed: `cargo fmt -p workflow-nodes -p node-engine -p
    pantograph-workflow-service -p pantograph-uniffi`, `cargo test -p
    workflow-nodes processing::inference --lib`, `cargo test -p
    pantograph-workflow-service workflow::tests::task_graph --lib`,
    `cargo test -p node-engine port_options --lib`, `node
    --experimental-strip-types --test
    src/components/nodes/workflow/selectionInputProviderOptions.test.ts
    src/services/workflow/portOptionsCache.test.ts
    src/services/workflow/WorkflowService.commands.test.ts`, `npm run
    typecheck`, `cargo check -p node-engine -p workflow-nodes -p
    pantograph-workflow-service -p pantograph-uniffi`, `cargo check -p
    node-engine -p workflow-nodes -p pantograph-workflow-service -p
    pantograph-uniffi --all-features`, `cargo check -p node-engine -p
    workflow-nodes -p pantograph-workflow-service -p pantograph-uniffi
    --no-default-features`, and `git diff --check`.
  - Remaining follow-up: typed task-result materialization and active-run
    result storage remain the next Milestone 5c implementation item.
- 2026-05-23 Milestone 5c typed task-result materialization slice completed:
  - Smallest useful vertical slice: add the workflow-service task-result
    materialization contract and staged active-run result storage before
    dependency binding resolution or orchestrator dispatch.
  - Allowed write set: focused workflow-service task-result contract and
    tests, narrow workflow-service exports/test registration, scheduler-store
    active-run result storage module and initialization, scheduler/workflow
    READMEs, Milestone 5c plan, task-level orchestration plan, and this
    execution log.
  - Implementation notes: added a versioned `WorkflowSchedulerTaskResult`
    DTO with typed output values for `PumasModelRef`, strings, booleans,
    signed/unsigned integers, media artifact refs, diagnostic-only outputs,
    closed result status, bounded diagnostics, and terminal metadata. Added
    focused staged active-run result storage APIs that validate result schema
    and active workflow-run correlation.
  - No-fallback/no-legacy confirmation: task results carry scheduler-owned
    typed facts only. They do not carry local model paths, executable Pumas
    load targets, runtime handoff, worker launch details, node-engine
    internals, `ModelRefV2`, frontend `modelPath`, or reduced-plan launch
    projections.
  - Verification passed: `cargo test -p pantograph-workflow-service
    workflow::tests::task_result_contracts --lib`, `cargo test -p
    pantograph-workflow-service active_run_scheduler_task_results --lib`,
    `cargo fmt -p pantograph-workflow-service -- --check`, `cargo check -p
    pantograph-workflow-service`, `cargo check -p pantograph-workflow-service
    --all-features`, `cargo check -p pantograph-workflow-service
    --no-default-features`, and `git diff --check`.
  - Verification deviation fixed: the first focused compile exposed a
    non-existent `QueueItemNotRunning` error variant in the new staged store
    module; the implementation now uses the existing `QueueItemNotFound`
    contract for no-active-run and wrong-active-run cases.
  - Remaining follow-up: dependency-to-input binding resolution from
    materialized task results is the next Milestone 5c item before the
    orchestrator can admit downstream runtime tasks.
- 2026-05-23 Milestone 5c dependency-to-input binding resolution slice
  completed:
  - Smallest useful vertical slice: add path-free task intent templates and a
    focused workflow-service resolver that turns materialized upstream task
    outputs into scheduler-admissible intents.
  - Allowed write set: workflow-service task graph contracts/projection/tests,
    new focused binding-resolution module and tests, workflow exports/test
    registration, workflow README, Milestone 5c plan, task-level orchestration
    plan, and this execution log.
  - Implementation notes: inference tasks now retain a
    `WorkflowSchedulerTaskIntentTemplate` when task type, scheduler
    runtime/device constraints, and trait settings are valid but
    `pumas_model_ref` is expected from an upstream binding. The resolver
    consumes `WorkflowSchedulerTaskResult` values and returns ready, blocked,
    unavailable, or invalid typed outcomes.
  - No-fallback/no-legacy confirmation: the resolver only accepts typed
    materialized `PumasModelRef` outputs. It does not read graph-local
    `model_path`, `ModelRefV2`, frontend `modelPath`, reduced execution plans,
    runtime handoff, Pumas load targets, or node-engine output-demand state.
  - Verification passed: `cargo test -p pantograph-workflow-service
    workflow::tests::task_binding_resolution --lib`, `cargo test -p
    pantograph-workflow-service workflow::tests::task_graph --lib`, `cargo
    fmt -p pantograph-workflow-service -- --check`, `cargo check -p
    pantograph-workflow-service`, `cargo check -p pantograph-workflow-service
    --all-features`, `cargo check -p pantograph-workflow-service
    --no-default-features`, and `git diff --check`.
  - Verification deviation fixed: the first binding resolver fixture used an
    unregistered synthetic model-selector node and failed topology validation;
    the test now uses registered `puma-lib` behavior.
  - Remaining follow-up: the scheduler task orchestrator application shell is
    the next Milestone 5c item and must consume the resolver's ready/blocked
    outcomes without reviving whole-workflow execution.
- 2026-05-23 Milestone 5c runtime-host contract-crate re-plan decision:
  - Re-plan boundary: the scheduler task orchestrator belongs in
    `pantograph-workflow-service`, but the current runtime-host execution
    request/response, validation wrappers, execution port, dispatcher, and
    typed dispatch errors live in `pantograph-embedded-runtime`. Since
    embedded-runtime already depends on workflow-service, importing those
    contracts into workflow-service would create a crate dependency cycle.
  - Decision: use option 1. Move runtime-host execution contracts and the
    dispatcher into a lower-level shared contract crate before implementing
    the orchestrator. The shared crate should depend only on lower-level
    contracts such as `pantograph-scheduler`, `serde`, `async-trait`, and
    `thiserror`; it must not depend on workflow-service, embedded-runtime,
    node-engine, Pumas Library, or inference runtime crates.
  - Ownership after the move: workflow-service orchestrates against the shared
    runtime-host port; embedded-runtime implements that port and owns
    runtime-specific Pumas load-target resolution; scheduler owns
    `SchedulerRuntimeHandoff`; node-engine remains limited to non-runtime task
    execution from materialized inputs.
  - Rejected alternatives: adding runtime execution methods to `WorkflowHost`,
    moving the orchestrator into embedded-runtime, or mirroring runtime-host
    DTOs in workflow-service.
  - No-fallback/no-legacy confirmation: remove the embedded-runtime-owned
    contract definitions after the shared-crate move rather than preserving
    parallel DTOs, aliases, compatibility shims, or reduced-plan launch
    branches. Runtime inference still launches only from an actual
    dispatch-selected `SchedulerRuntimeHandoff`.
  - Next implementation slice: create/move the shared runtime-host contract
    crate and update embedded-runtime to implement/re-export through the new
    owner only after the old definitions are removed.
- 2026-05-23 Standards iteration for runtime-host contract-crate re-plan:
  - Standards reviewed: coding standards for file decomposition, layered
    ownership, single owner for stateful flows, and documentation; plan
    standards for re-plan triggers, worktree hygiene, serial ownership of
    shared contracts, and verification; architecture patterns for monorepo
    package roles and executable boundary contracts; dependency standards for
    narrow crate dependency ownership; Rust API standards for typed validated
    public contracts; Rust async standards for sync-core/async-shell and task
    lifecycle; and testing standards for vertical slice and executable
    contract coverage.
  - Plan updates made: the shared runtime-host crate is explicitly constrained
    to DTOs, validated wrappers, typed errors, the async port trait, and
    synchronous validation/correlation helpers. It must not own scheduler
    policy, workflow orchestration, runtime loading, Pumas load-target
    resolution, node-engine execution, concrete I/O, spawned tasks, or Tokio
    runtime lifecycle.
  - Required implementation guardrails added: crate-level docs, source
    README, public `lib.rs` re-exports, `TryFrom` validated wrappers,
    `#[must_use]`/`#[non_exhaustive]` public API discipline, workspace
    dependency ownership checks, no new third-party dependency without
    standards justification, and executable JSON fixtures for request/response
    validation.
  - Required replacement guardrails added: remove or convert the existing
    embedded-runtime-owned runtime-host DTO/port/dispatcher definitions to use
    the shared owner. Do not preserve aliases, mirrored DTOs, compatibility
    modules, or alternate successful runtime launch paths.
  - Required verification added: focused shared-crate contract tests,
    embedded-runtime runtime-host dispatch/load-target tests,
    workflow-service compile checks proving the dependency cycle is gone,
    default/all-features/no-default-features checks for touched crates, and
    `git diff --check`.
- 2026-05-23 Milestone 5c runtime-host shared contract crate slice:
  - Smallest useful vertical slice: move runtime-host execution DTOs,
    validation wrappers, diagnostics, execution port, dispatcher, and typed
    errors into the new `pantograph-runtime-host-contracts` shared boundary
    crate; update embedded-runtime load-target resolution to import the shared
    validated request; delete the embedded-runtime-owned DTO/dispatcher
    modules, tests, and fixtures.
  - No-fallback/no-legacy confirmation: no aliases, mirrored DTOs,
    compatibility modules, or alternate runtime launch paths were retained.
    Runtime-host Pumas load-target resolution remains host-only and consumes
    only scheduler-selected model refs from the validated shared request.
  - Verification passed: `cargo test -p pantograph-runtime-host-contracts`,
    `cargo test -p pantograph-embedded-runtime runtime_host_load_target --lib`,
    `cargo check -p pantograph-runtime-host-contracts`, `cargo check -p
    pantograph-embedded-runtime`, `cargo check -p
    pantograph-workflow-service`, default/all-features/no-default-features
    checks for the shared contract crate, embedded-runtime, and
    workflow-service, `cargo fmt -p pantograph-runtime-host-contracts -p
    pantograph-embedded-runtime -- --check`, `git diff --check`, targeted
    deletion `rg` checks, and file-size review for the new/touched
    runtime-host files.
  - Deviation recorded: workflow-service does not yet depend on the shared
    port because the orchestrator slice has not introduced a real use. The
    consumer dependency will be added with orchestrator wiring to preserve
    dependency ownership standards and avoid unused dependencies.
- 2026-05-23 Milestone 5c workflow-service orchestrator runtime-host
  boundary slice:
  - Smallest useful vertical slice: add the focused workflow-service
    `scheduler/task_orchestrator.rs` async shell and tests proving it dispatches
    only scheduler-owned `SchedulerRuntimeHandoff` values through the shared
    runtime-host dispatcher.
  - No-fallback/no-legacy confirmation: the shell does not import
    embedded-runtime, resolve model paths, synthesize handoff from reduced
    execution plans, or call node-engine runtime inference paths.
  - Verification passed: `cargo test -p pantograph-workflow-service
    scheduler::task_orchestrator --lib`, `cargo fmt -p
    pantograph-workflow-service -- --check`, default/all-features/no-default
    workflow-service checks, `git diff --check`, and file-size review for the
    new orchestrator files.
  - Remaining follow-up: the orchestrator remains staged until production
    session execution wires dependency readiness, task-state transitions,
    ledger writes, bounded queues, cancellation, retry/defer, panic handling,
    and runtime-host dispatch lifecycle.
- 2026-05-23 Milestone 5c task-definition/task-state re-plan update:
  - Smallest useful vertical slice: update planning only after codebase review
    showed the current scheduler queue record requires complete
    `SchedulableTaskIntent` even for tasks that are validly waiting on
    upstream materialized inputs.
  - Allowed write set: Milestone 5c plan, task-level scheduler orchestration
    design note, and this execution log. No source, test, config, lockfile,
    generated, build-output, sqlite, or workflow fixture files are part of
    this planning slice.
  - Decision: replace the current intent-required `SchedulerQueueTaskRecord`
    and `SchedulerQueueTransition` with phase-aware scheduler task-state
    records and transitions. `WorkflowSchedulerTaskGraph` remains the
    immutable task definition owner in workflow-service; the scheduler crate
    owns mutable lifecycle state. `SchedulableTaskIntent` stays strict and is
    present only on state variants that are actually schedulable.
  - Rejected alternatives: lazy-create scheduler records only after
    materialization, because blocked tasks disappear from scheduler state;
    add `Option<SchedulableTaskIntent>` to the current record, because that
    makes invalid state combinations representable; or move workflow graph
    bindings/templates into scheduler, because that couples scheduler policy
    to graph composition.
  - No-fallback/no-legacy confirmation: do not create dummy Pumas refs,
    placeholder task intents, synthetic task types, reduced execution-plan
    handoffs, node-engine output-demand fallback, or compatibility queue
    shims. Old queue record contracts are replacement/removal targets.
  - Verification passed for docs-only slice: `git diff --check` against the
    three touched plan files. Per user instruction, this update is not
    committed yet.
- 2026-05-23 Standards iteration for task-state replacement planning:
  - Standards reviewed: plan standards for worktree hygiene, re-plan triggers,
    verification, and serial ownership; coding standards for decomposition,
    backend-owned state, and single-owner state machines; architecture
    patterns for package roles and executable boundary contracts; Rust API
    standards for correct-by-construction public APIs; Rust async standards
    for sync-core/async-shell and lifecycle ownership; dependency standards
    for narrow dependency ownership; testing standards for vertical slices,
    recovery, idempotency, and durable resource isolation; frontend standards
    for backend-owned display state; and documentation standards for README
    traceability.
  - Plan updates made: added task-state replacement standards gates requiring
    focused scheduler modules/tests, state-specific typed payloads instead of
    optional intents, stable schema/contract versions, validated wrappers,
    typed errors, bounded diagnostics, synchronous scheduler policy, no new
    dependency without a recorded standards decision, direct removal of old
    queue contracts, fixture/artifact regeneration or typed rejection, README
    updates, vertical pre-intent-to-dispatch coverage, replay/idempotency
    coverage, durable test-state isolation, and targeted deletion checks.
  - No-fallback/no-legacy confirmation: the standards pass did not approve any
    compatibility shim, alias, silent migration, best-effort parser, dummy
    intent, node-engine runtime fallback, or reduced-plan handoff synthesis.
    Old queue record contracts remain replacement/removal targets.
  - Verification passed for this docs-only standards pass: `git diff --check`
    against the three touched plan files. No source implementation changed,
    and no commit was created.
- 2026-05-23 Milestone 5c phase-aware scheduler task-state replacement slice
  completed:
  - Smallest useful vertical slice: replace the intent-required scheduler
    queue record/transition contract with phase-aware durable task-state
    records and transitions, then update active-run storage and path-free read
    models to consume the new state shape.
  - Allowed write set: `pantograph-scheduler` queue/task-state contract,
    lifecycle import/tests, fixtures, README files;
    `pantograph-workflow-service` active-run scheduler task-state
    storage/read-model tests; and current plan notes.
  - No-fallback confirmation: removed old scheduler queue record/transition
    source symbols, fixtures, exports, and tests instead of adding aliases or
    compatibility shims. Pre-intent states do not synthesize
    `SchedulableTaskIntent`; invalid/unavailable/terminal states carry typed
    diagnostics.
  - Implementation notes: added `SchedulerTaskState` variants for
    awaiting-inputs, input-unavailable, invalid, ready,
    waiting-dependency-readiness, waiting-resources, waiting-batch, running,
    paused-deferred, retryable-failed, terminal-failed, and completed. Durable
    records now carry state-specific payloads, state version, and transition
    id; schedulable states carry the strict path-free task intent, while
    pre-intent and terminal diagnostic states do not.
  - Focused tests/fixtures: replaced the old queue transition fixture with
    `task_state_transition_ready.json`; added scheduler transition coverage
    for pre-intent states, diagnostics requirements, idempotent replay,
    duplicate transition ids, terminal closure, stale previous state
    rejection, and path-shaped field rejection. Workflow-service read models
    now verify pre-intent records expose optional task/model/runtime/device
    fields without internal scheduler payloads.
  - Verification passed: `cargo test -p pantograph-scheduler --test
    queue_state`; `cargo test -p pantograph-scheduler --test task_lifecycle`;
    `cargo test -p pantograph-workflow-service scheduler::store::tests
    --lib`; `cargo test -p pantograph-workflow-service
    workflow::tests::task_state_read_model --lib`; `cargo check -p
    pantograph-scheduler`; `cargo check -p pantograph-scheduler
    --all-features`; `cargo check -p pantograph-scheduler
    --no-default-features`; `cargo check -p pantograph-workflow-service`;
    `cargo check -p pantograph-workflow-service --all-features`; and `cargo
    check -p pantograph-workflow-service --no-default-features`.
  - Remaining follow-up: full graph-editor/run-inspection task-state views
    still need immutable task-definition joins, waiting reasons, timing,
    attempts, and ledger-backed diagnostics from the orchestrator slices.
- 2026-05-23 Milestone 5c joined task-state read-model slice completed:
  - Smallest useful vertical slice: replace records-only scheduler task-state
    read-model projection with a backend-owned join over immutable
    `WorkflowSchedulerTaskGraph` facts and mutable `SchedulerTaskStateRecord`
    values.
  - Allowed write set: workflow-service active-run scheduler task-state
    storage/read-model modules and tests, workflow-service README/export notes,
    and current plan notes.
  - No-fallback confirmation: the read-model query now obtains task
    definitions from active-run scheduler task graph state and lifecycle facts
    from scheduler task-state records. It does not infer graph facts in the
    frontend, expose executable paths, synthesize task intent for pre-intent
    states, or keep the old records-only projection as a compatibility path.
  - Implementation notes: active-run storage now keeps
    `WorkflowSchedulerTaskGraph` with task-state records; read models expose
    node type, dependency task ids, input bindings, projection diagnostics,
    and optional task/model/runtime/device facts; mismatched graph/state joins
    fail closed.
  - Verification passed: `cargo test -p pantograph-workflow-service
    scheduler::store::tests --lib`; `cargo test -p
    pantograph-workflow-service workflow::tests::task_state_read_model --lib`;
    `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --all-features`; and `cargo check -p
    pantograph-workflow-service --no-default-features`.
  - Remaining follow-up: graph-editor/run-inspection read models still need
    waiting reasons, timing, attempts, and ledger-backed diagnostics from the
    orchestrator lifecycle slices.
- 2026-05-23 Milestone 5c orchestrator initialization slice completed:
  - Smallest useful vertical slice: add the workflow-service orchestrator
    method that turns immutable `WorkflowSchedulerTaskGraph` tasks into
    initial `SchedulerTaskStateRecord` values before dependency readiness or
    runtime dispatch begins.
  - Allowed write set: workflow-service task orchestrator module/tests and
    current plan notes.
  - No-fallback confirmation: initialization does not call node-engine output
    demand, synthesize model paths, create dummy Pumas refs, or launch runtime
    inference. Valid schedulable tasks start as `Ready`, unresolved/template
    tasks start as `AwaitingInputs`, and projection-diagnostic tasks start as
    `Invalid` with typed scheduler diagnostics.
  - Verification passed: `cargo test -p pantograph-workflow-service
    scheduler::task_orchestrator --lib`; `cargo check -p
    pantograph-workflow-service`; `cargo check -p pantograph-workflow-service
    --all-features`; and `cargo check -p pantograph-workflow-service
    --no-default-features`.
  - Remaining follow-up: production store initialization, dependency readiness
    calls, runtime-host dispatch lifecycle, ledger writes, bounded queues,
    cancellation, retry/defer, and panic handling remain open.
- 2026-05-23 Milestone 5c orchestrator active-run persistence slice
  completed:
  - Smallest useful vertical slice: let the workflow-service orchestrator
    derive initial task-state records from `WorkflowSchedulerTaskGraph` and
    persist the task graph plus records together on the active run.
  - Allowed write set: workflow-service task orchestrator module/tests and
    current plan notes.
  - No-fallback confirmation: the method uses the canonical active-run
    scheduler task-state store. It does not create a parallel queue, call
    node-engine output demand, launch runtime inference, or keep a records-only
    compatibility path.
  - Verification passed: `cargo test -p pantograph-workflow-service
    scheduler::task_orchestrator --lib`; `cargo check -p
    pantograph-workflow-service`; `cargo check -p pantograph-workflow-service
    --all-features`; and `cargo check -p pantograph-workflow-service
    --no-default-features`.
  - Remaining follow-up: production session execution must call this
    initialization after task graph extraction, then advance state through
    dependency readiness, dispatch, result materialization, ledger writes,
    retry/defer, and cancellation policy.
- 2026-05-23 Milestone 5c production orchestrator ownership re-plan update:
  - Smallest useful vertical slice: update planning only, after the
    implementation slices exposed that production session execution needs a
    single owner for the orchestrator and runtime-host dispatcher before
    replacing whole-workflow node-engine output demand.
  - Allowed write set: current image-generation plan files only. Existing
    unrelated proposal Markdown changes remain ignored.
  - Decision: combine service-owned orchestrator injection with a dedicated
    scheduler-task execution entrypoint. `WorkflowService` must own or be
    configured with `WorkflowSchedulerTaskOrchestrator` and
    `SchedulerRuntimeHostDispatcher`; `run_workflow_execution_session` should
    delegate to a focused scheduler-task execution path after queue admission
    and task graph extraction.
  - Rejected alternatives: construct the orchestrator locally inside
    `run_workflow_execution_session`, continue scheduler-managed inference via
    whole-workflow node-engine output demand, or keep old and new execution
    paths behind compatibility branches.
  - No-fallback/no-legacy confirmation: the plan requires direct replacement
    of the old scheduler-managed inference launch path. The graph editor and
    node engine remain path-free consumers of backend facts; runtime inference
    dispatch must flow through scheduler handoff and the shared runtime-host
    execution port.
  - Verification passed for this docs-only standards pass:
    `git diff --check -- docs/plans/current-image-generation-graphs/05-execution-management.md docs/plans/current-image-generation-graphs/10-task-level-scheduler-orchestration.md docs/plans/current-image-generation-graphs/milestones/05c-task-level-scheduler-orchestration.md`.
    Per user instruction, this plan update is not committed yet.
- 2026-05-23 Milestone 5c production orchestrator standards iteration:
  - Scope: docs-only standards pass over the current production-orchestrator
    cutover plan. Existing unrelated proposal Markdown changes remain ignored.
  - Standards reviewed:
    `PLAN-STANDARDS.md`, `CODING-STANDARDS.md`,
    `ARCHITECTURE-PATTERNS.md`, `CONCURRENCY-STANDARDS.md`,
    `TESTING-STANDARDS.md`, `DEPENDENCY-STANDARDS.md`,
    `DOCUMENTATION-STANDARDS.md`, `languages/rust/RUST-API-STANDARDS.md`,
    and `languages/rust/RUST-ASYNC-STANDARDS.md`.
  - Findings: the selected service-owned orchestrator direction is
    standards-aligned, but the plan needed explicit production cutover gates
    for composition-root wiring, narrow session-execution delegation,
    sync-core/async-shell separation, bounded lock/transaction scopes,
    tracked task lifecycle, typed error/diagnostic boundaries, dependency
    ownership, README updates, dead-code allowance cleanup, legacy launch-path
    deletion, and vertical production-entrypoint verification.
  - Plan updates: added production cutover standards gates to the task-level
    orchestration plan and mirrored the critical guardrails in the Milestone
    5c standards section.
  - No-fallback/no-legacy confirmation: this standards pass does not approve
    globals, local orchestrator construction, lazy singleton dispatchers,
    compatibility branches, old/new execution feature flags, untyped
    metadata, string parsing, or retained scheduler-managed inference launch
    paths.
  - Verification planned for this docs-only slice: `git diff --check`. Per
    user instruction, this plan update is not committed yet.
- 2026-05-23 Milestone 5c production task-state initialization slice
  completed:
  - Smallest useful vertical slice: wire production
    `run_workflow_execution_session` to initialize scheduler-owned active-run
    task state from the immutable `WorkflowSchedulerTaskGraph` after queue
    admission and before the current whole-run execution path continues.
  - Allowed write set: workflow-service scheduler orchestrator export,
    workflow facade/configuration/session execution modules, focused session
    execution fixtures/tests, workflow-service README, and current Milestone
    5c plan notes. Existing unrelated Pumas proposal Markdown changes remain
    ignored.
  - No-fallback confirmation: the slice installs a service-owned orchestrator
    and a typed-unavailable default runtime-host execution port. It does not
    synthesize runtime handoff from reduced execution plans, expose Pumas
    paths, fabricate task intents for pre-intent states, dispatch runtime
    inference through node-engine, or add a compatibility branch. The old
    whole-run output-demand path remains only as the next explicit removal
    target, not as a new fallback.
  - Implementation notes: `WorkflowService` now owns
    `WorkflowSchedulerTaskOrchestrator`, can be configured with the shared
    `RuntimeHostExecutionPort`, fetches/validates the workflow graph outside
    the session-store lock, initializes active-run task graph/state through
    the orchestrator, and fails admitted runs closed if initialization fails.
    Test fixtures now provide workflow graph/runtime facts for session hosts
    that the production path exercises.
  - Verification passed: `cargo test -p pantograph-workflow-service
    workflow::tests::session_execution::workflow_execution_session_initializes_scheduler_task_state_before_run_execution
    --lib`; `cargo test -p pantograph-workflow-service
    scheduler::task_orchestrator --lib`; `cargo test -p
    pantograph-workflow-service workflow::tests::session_execution --lib`;
    `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --all-features`; `cargo check -p
    pantograph-workflow-service --no-default-features`; and `cargo fmt -p
    pantograph-workflow-service`.
  - Deviation/discovered issue: broader session verification exposed a stale
    scheduler-estimate assertion expecting MB-formatted memory text while the
    current production formatter emits exact byte text. The assertion was
    updated to match current behavior; no production estimate formatter was
    changed in this slice.
  - Remaining follow-up: replace whole-run node-engine output demand with the
    dedicated scheduler-task execution path, wire dependency readiness,
    runtime-host dispatch lifecycle, task result progression, ledger writes,
    bounded workers, cancellation, retry/defer, panic handling, and then
    delete superseded launch paths and remaining staged dead-code allowances.
- 2026-05-23 Milestone 5c non-runtime adapter first re-plan update:
  - Scope: docs-only update after the production task-state initialization
    slice exposed the next cutover boundary. Existing unrelated Pumas proposal
    Markdown changes remain ignored, and this update is not committed per user
    instruction.
  - Decision: use option 2 next. Add the dedicated scheduler-task execution
    entrypoint plus a narrow non-runtime node-engine single-task adapter
    before wiring runtime inference dispatch. The slice must execute only
    explicitly non-runtime task kinds from materialized scheduler-owned inputs
    and persist typed `WorkflowSchedulerTaskResult` values.
  - No-fallback/no-legacy confirmation: the selected next slice must not wrap
    `workflow_run_internal`, must not call node-engine output demand from the
    new entrypoint, must not use `PlannedInferenceExecutionHost`, and must not
    route runtime inference through the non-runtime adapter. Runtime inference
    tasks stay blocked/deferred/failed with typed scheduler diagnostics until
    actual dispatch-selected `SchedulerRuntimeHandoff` execution is wired.
  - Rejected alternatives: a minimal wrapper around existing whole-run
    execution because it preserves the legacy successful path; runtime
    dispatch first because it depends on the task entrypoint and typed
    materialization path; full cutover in one slice because it is too broad
    for validated thin-slice implementation.
  - Verification planned for this docs-only slice: `git diff --check --
    docs/plans/current-image-generation-graphs/05-execution-management.md
    docs/plans/current-image-generation-graphs/10-task-level-scheduler-orchestration.md
    docs/plans/current-image-generation-graphs/milestones/05c-task-level-scheduler-orchestration.md`.
- 2026-05-23 Milestone 5c non-runtime adapter standards iteration:
  - Scope: docs-only standards pass over the planned scheduler-task
    entrypoint and non-runtime node-engine adapter slice. Existing unrelated
    Pumas proposal Markdown changes remain ignored.
  - Standards reviewed:
    `PLAN-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
    `CONCURRENCY-STANDARDS.md`, `TESTING-STANDARDS.md`,
    `DEPENDENCY-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`,
    `languages/rust/RUST-API-STANDARDS.md`, and
    `languages/rust/RUST-ASYNC-STANDARDS.md`.
  - Codebase findings: node-engine already exposes async single-task
    execution through `TaskExecutor`/`CoreTaskExecutor`, but that surface can
    execute inference nodes when given inference task kinds or planned
    inference extensions. The workflow-service adapter plan therefore needs
    an explicit positive non-runtime allowlist and typed conversion boundary
    before it may call node-engine.
  - Plan updates: added standards gates requiring focused workflow-service
    modules, no broad growth in session execution or workflow-run internals,
    positive non-runtime task-kind allowlisting, explicit typed
    `WorkflowSchedulerTaskResult` to/from node-engine value conversion,
    bounded lock scopes around awaits, typed diagnostics for every rejection
    and conversion failure, no dependency/lockfile changes, README updates,
    normal-parallel test execution, feature checks, and targeted deletion
    searches.
  - No-fallback/no-legacy confirmation: implementation must not pass raw
    serde blobs as a compatibility format, must not accept path-like fields as
    successful values, must not call `workflow_run_internal`,
    `DemandEngine::demand`, output-node demand, or
    `PlannedInferenceExecutionHost`, and must reject runtime inference task
    kinds before node-engine can execute them.
  - Verification planned for this docs-only standards pass: `git diff --check
    -- docs/plans/current-image-generation-graphs/05-execution-management.md
    docs/plans/current-image-generation-graphs/10-task-level-scheduler-orchestration.md
    docs/plans/current-image-generation-graphs/milestones/05c-task-level-scheduler-orchestration.md`.
- 2026-05-23 Milestone 5c non-runtime adapter codebase investigation update:
  - Scope: docs-only update after reviewing the scheduler state contract,
    workflow-service task-result contract, node-engine single-task executor,
    workflow-node `puma-lib` descriptor, workflow-service graph registry, and
    graph persistence blast radius. Existing unrelated Pumas proposal Markdown
    changes remain ignored.
  - Findings: `WorkflowSchedulerTaskResultValue` does not support arbitrary
    JSON, finite floating-point numbers, vectors, or generic object payloads,
    so "non-runtime" is too broad as an adapter category. The adapter must
    start with an explicit typed-output-compatible allowlist.
  - Findings: the current phase-aware scheduler task-state contract still
    requires `SchedulableTaskIntent` on ready/running/completed executable
    states. That cannot represent a completed pure non-runtime node without a
    fake runtime intent, so the plan now requires a
    `SchedulerTaskExecutionIntent`-style payload before real non-runtime task
    execution.
  - Findings: node-engine core `puma-lib` still emits `model_path`, while the
    canonical workflow-node descriptor exposes `pumas_model_ref`. The plan now
    excludes `puma-lib` from the generic non-runtime adapter and treats the
    core path-emitting implementation plus stale `puma-lib.model_path`
    registry/persistence behavior as deletion or typed-diagnostic replacement
    targets.
  - Plan updates: added the non-runtime executable-state gap as an open
    Milestone 5c task, narrowed the initial adapter allowlist to typed
    `text-input`, `text-output`, and `boolean-input` behavior, required
    node-type authority from immutable `WorkflowSchedulerTaskGraph`, excluded
    arbitrary-JSON/floating/vector/file/media/model/Pumas nodes until their
    explicit contracts exist, and added verification for no fake runtime
    intent, stale `puma-lib.model_path` cleanup, and no call to node-engine
    core `execute_puma_lib`.
  - Verification planned for this docs-only update: `git diff --check --
    docs/plans/current-image-generation-graphs/05-execution-management.md
    docs/plans/current-image-generation-graphs/10-task-level-scheduler-orchestration.md
    docs/plans/current-image-generation-graphs/milestones/05c-task-level-scheduler-orchestration.md`.
- 2026-05-23 Milestone 5c non-runtime executable-state contract slice
  completed:
  - Smallest useful vertical slice: replace executable scheduler task-state
    payloads with a typed execution-intent enum so non-runtime scheduler tasks
    can become ready, run, and complete without fabricating a
    `SchedulableTaskIntent`.
  - Allowed write set: `crates/pantograph-scheduler/src/queue.rs`,
    scheduler public exports and source README, scheduler task-state fixture
    and tests, narrow workflow-service scheduler/read-model test helpers, the
    orchestrator initial-ready-state construction site, and current plan docs.
    Existing unrelated Pumas proposal Markdown changes remain ignored.
  - No-fallback/no-legacy confirmation: runtime execution state still carries
    `SchedulableTaskIntent` only through the runtime variant, while
    non-runtime execution state carries `SchedulerNonRuntimeTaskIntent`. No
    dummy model refs, synthetic runtime task types, path fields, Pumas load
    targets, or compatibility aliases were added. Existing scheduler policy
    consumers that call `task_intent()` receive only runtime intents.
  - Implementation notes: added `SchedulerTaskExecutionIntent`,
    `SchedulerNonRuntimeTaskIntent`, and `SchedulerNonRuntimeTaskKind`; moved
    ready/waiting/running/deferred/retryable/completed task-state payloads to
    `execution_intent`; updated validation to check runtime and non-runtime
    correlation separately; updated the runtime ready-state fixture; and
    updated workflow-service constructors/tests to wrap runtime intents
    explicitly.
  - Verification passed: `cargo fmt -p pantograph-scheduler -p
    pantograph-workflow-service`; `cargo test -p pantograph-scheduler --test
    queue_state`; `cargo test -p pantograph-workflow-service
    scheduler::store::tests --lib`; `cargo test -p
    pantograph-workflow-service workflow::tests::task_state_read_model --lib`;
    `cargo test -p pantograph-workflow-service scheduler::task_orchestrator
    --lib`; `cargo check -p pantograph-scheduler`; `cargo check -p
    pantograph-scheduler --all-features`; `cargo check -p
    pantograph-scheduler --no-default-features`; `cargo check -p
    pantograph-workflow-service`; `cargo check -p pantograph-workflow-service
    --all-features`; `cargo check -p pantograph-workflow-service
    --no-default-features`; targeted search proving no
    `SchedulerTaskState::{Ready,...,Completed} { task_intent }` construction
    remains in scheduler/workflow-service Rust sources; and `git diff --check`.
  - Remaining follow-up: remove stale `puma-lib.model_path` compatibility
    surfaces, then add the dedicated scheduler-task execution entrypoint and
    narrow typed non-runtime node-engine adapter.
- 2026-05-23 Milestone 5c stale `puma-lib.model_path` cleanup slice
  completed:
  - Smallest useful vertical slice: remove the successful path-only
    `puma-lib` model identity branch before scheduler-task execution can
    consume graph-local model paths.
  - Allowed write set: workflow-service graph persistence sanitizer/tests,
    graph registry options-provider regression, graph README, and current
    plan docs. Existing unrelated Pumas proposal Markdown changes remain
    ignored.
  - No-fallback/no-legacy confirmation: current `puma-lib` persistence now
    strips `modelPath` and `model_path` regardless of whether a canonical
    model id is present. The graph registry test asserts `pumas_model_ref`
    as the canonical options-provider boundary. Retired non-`puma-lib`
    stale-diagnostic tests were left intact because they do not provide a
    successful `puma-lib` execution identity branch.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`;
    focused registry and persistence tests for `pumas_model_ref` and
    path-stripping behavior; `cargo test -p pantograph-workflow-service
    graph::persistence_tests --lib`; `cargo test -p
    pantograph-workflow-service graph::registry::tests --lib`; `cargo check
    -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --all-features`; `cargo check -p
    pantograph-workflow-service --no-default-features`; targeted stale
    `puma-lib.model_path` successful-branch search; and `git diff --check`.
  - Remaining follow-up: add the dedicated scheduler-task execution
    entrypoint and narrow typed non-runtime node-engine adapter.
- 2026-05-23 Milestone 5c scheduler task-state read-model diagnostics slice
  completed:
  - Smallest useful vertical slice: extend the existing workflow-service
    scheduler task-state read model with path-free execution category and
    scheduler state diagnostics for graph editor, run inspection, and
    diagnostics consumers.
  - Allowed write set: workflow-service task-state read-model contract/tests,
    workflow facade exports, workflow source README, and current plan docs.
    Existing unrelated Pumas proposal Markdown changes remain ignored.
  - No-fallback/no-legacy confirmation: the read model still hides transition
    ids, state versions, runtime handoff, executable Pumas load targets,
    local model paths, and worker launch facts. Non-runtime executable states
    expose only the non-runtime task kind; pre-intent states do not fabricate
    model/runtime/device facts.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
    test -p pantograph-workflow-service workflow::tests::task_state_read_model
    --lib`; `cargo test -p pantograph-workflow-service
    workflow::tests::session_execution::workflow_execution_session_initializes_scheduler_task_state_before_run_execution
    --lib`.
  - Remaining follow-up: add typed scheduler timing and attempt facts with the
    retry/defer/ledger lifecycle slice instead of inferring them from state
    version, then add the dedicated scheduler-task execution entrypoint and
    narrow typed non-runtime node-engine adapter.
- 2026-05-23 Milestone 5c node-engine single-task API replan update:
  - Scope: docs-only replan for the boundary discovered before implementing
    the non-runtime adapter. Existing unrelated Pumas proposal Markdown
    changes remain ignored.
  - Decision: use option 1. Node-engine will expose a narrow single-task API
    that owns `graph_flow::Context` creation and empty `ExecutorExtensions`
    setup. Workflow-service will consume that API only behind its
    scheduler-task adapter and positive non-runtime allowlist.
  - Planned node-engine write set: one focused single-task API module and
    tests, `crates/node-engine/src/lib.rs` exports, and
    `crates/node-engine/src/README.md` documentation. Node-engine inference,
    planned-inference, `DemandEngine`, workflow-session, registry, and
    `puma-lib` implementation files remain out of scope unless a new replan
    is recorded.
  - No-fallback/no-legacy confirmation: workflow-service must not gain a
    direct `graph-flow` dependency, must not re-export or construct
    graph-flow internals, must not duplicate node behavior, and must reject
    runtime inference, `puma-lib`, `model-provider`, file I/O, arbitrary JSON,
    unknown task kinds, and unsupported non-runtime tasks before constructing
    the node-engine request.
  - Rejected options: re-exporting `graph_flow::Context`, adding `graph-flow`
    to workflow-service, duplicating allowed node behavior in workflow-service,
    wrapping `workflow_run_internal`, dispatching runtime tasks first, or
    doing the full execution cutover in one slice.
  - Verification planned: focused node-engine single-task API tests, focused
    workflow-service non-runtime adapter tests, default/all-features/no-default
    feature checks for touched crates, `git diff --check`, README updates, and
    targeted searches proving no calls to `workflow_run_internal`,
    `DemandEngine::demand`, output-node demand, `PlannedInferenceExecutionHost`,
    node-engine workflow sessions, or node-engine core `execute_puma_lib`.
- 2026-05-23 Milestone 5c standards tightening after blast-radius review:
  - Add a focused workflow-service task-classification boundary before the
    non-runtime adapter executes tasks. The classifier maps immutable task graph
    facts and canonical node-contract facts into runtime-inference,
    non-runtime node-engine, Pumas-materialization, or unsupported classes.
    This prevents scattered `llm-inference` string checks from becoming the
    long-term architecture and gives future model families/runtimes one
    extension point.
  - Generalize materialized-input readiness beyond `pumas_model_ref`. Runtime
    inference must not become executable until every required connected
    upstream input is materialized and validated. If a required port needs a
    value type not represented by `WorkflowSchedulerTaskResultValue`, the slice
    must stop and plan the typed value contract instead of passing raw JSON or
    falling back to node-engine demand.
  - Make non-runtime readiness explicit. Supported no-dependency non-runtime
    tasks may become `Ready(NonRuntime)` only from validated typed task
    templates; dependent non-runtime tasks remain `AwaitingInputs` until the
    typed resolver validates their upstream values; unsupported or excluded
    nodes produce typed diagnostics and must not defer indefinitely.
  - Tighten node-engine single-task authority. The new API must inject the
    explicit node type from immutable task-definition facts and fail closed if
    core node resolution would disagree, so task-id suffix inference cannot
    become execution authority.
- 2026-05-23 Milestone 5c node-engine single-task API slice completed:
  - Slice scope: `crates/node-engine/src/single_task.rs`,
    `crates/node-engine/src/lib.rs`, `crates/node-engine/src/README.md`, and
    this plan. No workflow-service, scheduler, inference, planned-inference,
    `DemandEngine`, workflow-session, registry, `puma-lib`, manifest, lockfile,
    generated, sqlite, or workflow fixture files were edited.
  - Implemented `NodeEngineSingleTaskRequest`, `NodeEngineSingleTaskResponse`,
    `NodeEngineSingleTaskError`, and `execute_core_task_once`. The API owns
    local graph-flow context plus empty executor extensions, injects explicit
    node type into `_data`, preserves caller `_data` fields, rejects malformed
    `_data`, and prevents task-id suffix inference from becoming execution
    authority.
  - No-fallback/no-legacy confirmation: this API is execution mechanics only,
    not a scheduler allowlist or runtime path. It does not call
    `workflow_run_internal`, `DemandEngine`, output-node demand, workflow
    sessions, planned-inference host extensions, runtime-host dispatch, or
    `execute_puma_lib`. Runtime inference and `puma-lib` remain rejected by the
    future workflow-service adapter/classifier before node-engine is called.
  - Verification: `cargo fmt -p node-engine`; `cargo test -p node-engine
    single_task -- --nocapture`; `cargo check -p node-engine`; `cargo check -p
    node-engine --no-default-features`; `cargo check -p node-engine
    --all-features`; `git diff --check -- crates/node-engine/src/single_task.rs
    crates/node-engine/src/lib.rs crates/node-engine/src/README.md`; targeted
    search over the new module for `workflow_run_internal`,
    `DemandEngine::demand`, `DemandEngine`, `PlannedInferenceExecutionHost`,
    `execute_puma_lib`, and `WorkflowExecutionSession`.
  - Remaining follow-up: add the workflow-service task-classification boundary,
    generalized materialized-input readiness, scheduler-task execution
    entrypoint, non-runtime adapter conversion, and runtime-task fail-closed
    diagnostics before any scheduler-owned non-runtime workflow task executes.
- 2026-05-23 Milestone 5c workflow-service task-classification slice
  completed:
  - Smallest useful vertical slice: add the workflow-service classification
    boundary required before scheduler-task entrypoint or non-runtime adapter
    execution. This slice does not execute tasks or alter runtime dispatch.
  - Allowed write set: workflow-service task classification module,
    task-graph DTO/projection/tests, narrow workflow facade exports and test
    helpers, workflow README, and current plan docs. Existing unrelated Pumas
    proposal Markdown changes remain ignored.
  - Implementation notes: added schema-versioned
    `WorkflowSchedulerTaskExecutionClass` to `WorkflowSchedulerTask`; added
    `task_execution_classification.rs` as the only workflow-service mapping
    from immutable node type plus canonical node-contract facts into
    `RuntimeInference`, `NonRuntimeNodeEngine`, `PumasMaterialization`, or
    `Unsupported`; moved the existing inference-intent branch to consume that
    class; and covered `llm-inference`, `puma-lib`, first-stage scalar
    node-engine nodes, excluded nodes, unknown nodes, and mismatched
    contracts.
  - No-fallback/no-legacy confirmation: the slice removes the task-graph
    projection's scattered `llm-inference` special case and does not add any
    compatibility path, raw node-data execution, graph-local path, Pumas load
    target exposure, node-engine output demand, planned-inference host call,
    runtime-host dispatch, or scheduler policy bypass.
  - Verification: `cargo fmt -p pantograph-workflow-service`; `cargo test -p
    pantograph-workflow-service workflow::tests::task_graph --lib`; `cargo
    test -p pantograph-workflow-service task_execution_classification --lib`;
    `cargo test -p pantograph-workflow-service scheduler::task_orchestrator
    --lib`; `cargo test -p pantograph-workflow-service
    scheduler::store::tests --lib`; `cargo test -p
    pantograph-workflow-service workflow::tests::task_state_read_model --lib`;
    `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; targeted search over the
    new classifier/projection modules for forbidden execution paths; and
    `git diff --check`.
  - Remaining follow-up: generalized materialized-input readiness,
    scheduler-task execution entrypoint, non-runtime adapter conversion, and
    runtime-task fail-closed diagnostics before any scheduler-owned
    non-runtime workflow task executes.
- 2026-05-23 Milestone 5c generalized runtime input-readiness slice
  completed:
  - Smallest useful vertical slice: extend workflow-service binding
    resolution so runtime inference intent materialization waits for every
    connected upstream task result, not only the `pumas_model_ref` binding.
  - Allowed write set: workflow-service task binding-resolution module/tests
    and current plan docs. Existing unrelated Pumas proposal Markdown changes
    remain ignored.
  - Implementation notes: added a reusable materialized-output lookup for
    task input bindings, validates upstream task-result contracts and terminal
    status before intent materialization, maps missing results to blocked
    readiness, unavailable upstreams to unavailable readiness, and invalid
    upstream results to invalid readiness. The existing `pumas_model_ref`
    specialization now consumes that shared readiness check before inserting
    the model ref into `SchedulableTaskIntent`.
  - No-fallback/no-legacy confirmation: the slice does not pass raw graph
    data, does not infer values from node-engine demand state, does not read
    graph-local paths or Pumas load targets, and does not make runtime
    inference executable until connected upstream outputs are materialized.
  - Verification: `cargo fmt -p pantograph-workflow-service`; `cargo test -p
    pantograph-workflow-service workflow::tests::task_binding_resolution
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; targeted search over the
    resolver for forbidden execution paths and graph-local model path usage
    returned only the existing negative test fixture/assertion; and `git diff
    --check`.
  - Remaining follow-up: explicit per-port value contracts for non-Pumas
    inputs as new task result value variants are added, scheduler-task
    execution entrypoint, non-runtime adapter conversion, and runtime-task
    fail-closed diagnostics.
- 2026-05-23 Milestone 5c execution-class initial-state slice completed:
  - Smallest useful vertical slice: make scheduler task-state initialization
    consume `WorkflowSchedulerTaskExecutionClass` before adding the
    scheduler-task execution entrypoint. This prevents unsupported classes
    from silently waiting forever and lets allowlisted source non-runtime
    nodes become ready without runtime intents.
  - Allowed write set: workflow-service scheduler task orchestrator/tests and
    current plan docs. Existing unrelated Pumas proposal Markdown changes
    remain ignored.
  - Implementation notes: `initial_task_state_records` now delegates to a
    focused state classifier. Runtime inference with a validated
    `SchedulableTaskIntent` becomes `Ready(Runtime)`. First-stage
    no-dependency `NonRuntimeNodeEngine` tasks become `Ready(NonRuntime)`
    with `SchedulerNonRuntimeTaskIntent`; dependent non-runtime tasks remain
    `AwaitingInputs`; `PumasMaterialization` remains `AwaitingInputs` with a
    dedicated materialization diagnostic; and `Unsupported` becomes
    `Invalid` with a typed diagnostic.
  - No-fallback/no-legacy confirmation: the slice does not execute
    non-runtime tasks, does not call node-engine, does not fabricate runtime
    `SchedulableTaskIntent` values, does not send Pumas materialization
    through the generic node-engine adapter, and does not preserve indefinite
    successful waiting for unsupported task classes.
  - Verification: `cargo fmt -p pantograph-workflow-service`; `cargo test -p
    pantograph-workflow-service scheduler::task_orchestrator --lib`; `cargo
    check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; targeted search over the
    orchestrator slice found only existing runtime-host dispatch/store/session
    test symbols and no node-engine demand, Pumas path, or planned-inference
    execution path; and `git diff --check`.
  - Remaining follow-up: scheduler-task execution entrypoint, non-runtime
    adapter conversion, and runtime-task fail-closed diagnostics.
- 2026-05-24 Milestone 5c non-runtime task-template replan update:
  - Scope: docs-only replan for the boundary discovered before workflow-service
    can call the node-engine single-task adapter. Existing unrelated Pumas
    proposal Markdown changes remain ignored.
  - Decision: use immediate option 2 now. Add a schema-versioned typed
    non-runtime task-template field to `WorkflowSchedulerTaskGraph` with
    concrete variants only for the first-stage allowlist:
    `TextInput { value: String }`, `BooleanInput { value: bool }`, and a
    no-static-data `TextOutput` template that consumes upstream materialized
    text. The task-graph projection is the only layer allowed to read graph
    node data for these templates.
  - Option 3 remains the later target. The long-term replacement is a generic
    typed port-value execution template derived from canonical node contracts
    for user-authored nodes and future runtime/model families. It must replace
    the concrete interim shape rather than creating a second successful
    execution path, and it must not pass raw JSON or incidental metadata.
  - No-fallback/no-legacy confirmation: the scheduler-task entrypoint and
    non-runtime adapter must consume only immutable typed templates plus
    materialized `WorkflowSchedulerTaskResult` values. They must not read raw
    graph/editor node data, `_data`, arbitrary `serde_json`, graph-local model
    paths, Pumas paths/load targets, frontend-owned scheduler facts,
    `workflow_run_internal`, output-node demand, or
    `PlannedInferenceExecutionHost`.
  - Verification planned for the next implementation slice: typed template
    projection tests for `text-input`, `boolean-input`, and `text-output`;
    negative tests for missing, malformed, unsupported, arbitrary-JSON, and
    user-authored node data; focused non-runtime adapter conversion tests;
    runtime-task rejection tests; default/all-features/no-default workflow
    service checks; README updates; targeted forbidden-path searches; and
    `git diff --check`.
- 2026-05-24 Milestone 5c typed non-runtime task-template slice completed:
  - Smallest useful vertical slice: add the immediate option 2 task-template
    contract to the immutable scheduler task graph before workflow-service can
    call the node-engine single-task adapter.
  - Allowed write set: workflow-service task graph contracts/projection/tests,
    narrow workflow facade exports, orchestrator initial-state invariant/tests,
    workflow README, Milestone 5c plan notes, and this execution log. Existing
    unrelated Pumas proposal Markdown changes remain ignored.
  - Implementation notes: bumped `WorkflowSchedulerTaskGraph` schema version
    to 3; added `WorkflowSchedulerNonRuntimeTaskTemplate` with concrete
    `TextInput`, `BooleanInput`, and `TextOutput` variants; projected
    canonical `text-input.text`, canonical `boolean-input.value`, and
    `text-output` only when an upstream `text` binding exists; added typed
    diagnostics for missing, invalid, or unsupported non-runtime templates;
    and made orchestrator initialization reject non-runtime tasks without a
    validated template.
  - No-fallback/no-legacy confirmation: the scheduler-task entrypoint and
    future adapter still have no raw graph/editor data path. Stale
    `text-input.value`, string booleans, text-output without upstream text,
    arbitrary JSON, graph-local paths, Pumas load targets, frontend-owned
    scheduler facts, output-node demand, and planned-inference host paths do
    not become successful execution inputs.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`;
    `cargo test -p pantograph-workflow-service workflow::tests::task_graph
    --lib`; `cargo test -p pantograph-workflow-service
    scheduler::task_orchestrator --lib`; `cargo test -p
    pantograph-workflow-service workflow::tests::task_state_read_model --lib`;
    `cargo test -p pantograph-workflow-service scheduler::store::tests --lib`;
    `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and targeted forbidden-path
    search over the touched projection/orchestrator modules.
  - Remaining follow-up: add the scheduler-task execution entrypoint,
    non-runtime adapter conversion from typed templates/materialized results
    into node-engine single-task requests, runtime-task fail-closed
    diagnostics, and then runtime-host dispatch cutover.
- 2026-05-24 Milestone 5c non-runtime adapter conversion slice completed:
  - Smallest useful vertical slice: add workflow-service conversion/execution
    for one allowlisted non-runtime scheduler task without yet wiring active-run
    state transitions or persistence.
  - Allowed write set: one focused workflow-service adapter module and tests,
    workflow module registration, workflow README, Milestone 5c plan notes,
    and this execution log. Existing unrelated Pumas proposal Markdown changes
    remain ignored.
  - Implementation notes: added `non_runtime_task_adapter.rs` with
    `execute_non_runtime_scheduler_task`, typed adapter errors, conversion from
    `WorkflowSchedulerNonRuntimeTaskTemplate` plus materialized task results
    into node-engine `NodeEngineSingleTaskRequest`, and conversion from raw
    node-engine outputs back into validated `WorkflowSchedulerTaskResult`
    values.
  - No-fallback/no-legacy confirmation: runtime inference tasks are rejected
    before node-engine execution; the adapter does not read graph/editor node
    data, does not pass Pumas paths/load targets, does not call output-node
    demand, does not use `PlannedInferenceExecutionHost`, and does not execute
    `puma-lib` or `model-provider`.
  - Deviation/follow-up: the adapter module carries a temporary module-scoped
    `dead_code` allowance because the scheduler-task execution entrypoint is
    the next slice and has not yet called the adapter. Remove this allowance in
    the entrypoint slice; do not let it persist beyond wiring.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`;
    `cargo test -p pantograph-workflow-service
    workflow::non_runtime_task_adapter --lib`; `cargo test -p
    pantograph-workflow-service workflow::tests::task_graph --lib`; `cargo
    test -p pantograph-workflow-service scheduler::task_orchestrator --lib`;
    `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and targeted forbidden-path
    search over the adapter.
  - Remaining follow-up: add the scheduler-task execution entrypoint that
    transitions ready non-runtime tasks through running/completed states,
    persists the returned task result, removes the temporary adapter
    `dead_code` allowance, and emits runtime-task fail-closed diagnostics.
- 2026-05-24 Milestone 5c scheduler-task completion consistency plan update:
  - Smallest useful vertical slice: documentation-only replan for the
    scheduler-task entrypoint boundary before implementation begins.
  - Allowed write set: Milestone 5c orchestration plan, Milestone 5c checklist
    file, and this execution log. Existing unrelated Pumas proposal Markdown
    changes remain ignored.
  - Decision: use the immediate option 2 active-run store completion operation
    before wiring the entrypoint. The store operation must record a successful
    `WorkflowSchedulerTaskResult` and the corresponding completed task-state
    transition together under one active-run store lock, after the entrypoint
    awaits the non-runtime adapter outside that lock.
  - No-fallback/no-legacy confirmation: the next implementation must not add a
    split successful "persist result" then "complete task" path, a split
    "complete task" then "persist result" path, a compatibility shim around
    old workflow output demand, or an untyped metadata flag to infer result
    ownership. Stale state, mismatched run/task/node correlation, duplicate
    successful completion, and adapter failure before a valid result must emit
    typed diagnostics and fail closed.
  - Later objective: option 3 execution lease/transaction commands with
    attempt tokens remain planned for retries, duplicate dispatch prevention,
    cancellation, worker pools, and durable replay ownership; they are not
    required for the immediate non-runtime entrypoint slice.
  - Planned verification: `git diff --check` for this documentation slice; in
    the implementation slice, active-run store tests proving atomic
    result-plus-completed transition and focused entrypoint tests proving no
    completed-without-result or result-without-completed state.
- 2026-05-24 Milestone 5c active-run atomic task completion slice completed:
  - Smallest useful vertical slice: add the store-owned atomic success
    boundary required before scheduler-task entrypoint wiring.
  - Allowed write set: `crates/pantograph-workflow-service/src/scheduler/store_task_results.rs`,
    Milestone 5c plan notes, and this execution log. Existing unrelated Pumas
    proposal Markdown changes remain ignored.
  - Implementation notes: added `complete_active_run_scheduler_task`, which
    validates the active run id, completed result status, completed transition,
    expected running state, current running task-state record, duplicate result
    absence, and workflow/run/node/task correlation before storing the task
    result and completed task-state record together.
  - No-fallback/no-legacy confirmation: the slice does not add a split
    successful result-store path for entrypoint use, does not revive output
    demand or planned-inference host behavior, and fails closed for stale state,
    wrong node correlation, duplicate success, and non-completed result status.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
    test -p pantograph-workflow-service active_run_complete_scheduler_task
    --lib`; `cargo test -p pantograph-workflow-service scheduler::store
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; and `cargo check -p
    pantograph-workflow-service --all-features`.
  - Remaining follow-up: wire the scheduler-task execution entrypoint to
    transition ready tasks to running, await the non-runtime adapter outside
    store locks, commit successful completion through
    `complete_active_run_scheduler_task`, remove the adapter `dead_code`
    allowance, and emit runtime-task fail-closed diagnostics.
- 2026-05-24 Milestone 5c ready non-runtime scheduler-task entrypoint slice
  completed:
  - Smallest useful vertical slice: execute one active-run scheduler task that
    is already ready and classified as non-runtime node-engine work, then
    persist its success through the atomic completion store method.
  - Allowed write set: `scheduler/task_orchestrator.rs`,
    `scheduler/task_orchestrator_tests.rs`, workflow non-runtime adapter
    visibility, workflow-service README, Milestone 5c plan notes, and this
    execution log. Existing unrelated Pumas proposal Markdown changes remain
    ignored.
  - Implementation notes: added
    `WorkflowSchedulerTaskOrchestrator::execute_ready_non_runtime_task`. The
    entrypoint reads cloned active-run task graph/state, validates ready
    non-runtime intent, transitions ready to running, reads materialized task
    results, awaits the non-runtime adapter outside store mutation calls,
    commits success through `complete_active_run_scheduler_task`, and moves
    adapter failures to terminal failed.
  - No-fallback/no-legacy confirmation: runtime inference tasks are rejected
    before node-engine execution; the entrypoint does not call output-node
    demand, `workflow_run_internal`, `PlannedInferenceExecutionHost`, or
    `execute_puma_lib`; and the non-runtime adapter module-level `dead_code`
    allowance was removed.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
    test -p pantograph-workflow-service
    orchestrator_executes_ready_non_runtime_task --lib`; `cargo test -p
    pantograph-workflow-service scheduler::task_orchestrator --lib`; `cargo
    test -p pantograph-workflow-service workflow::non_runtime_task_adapter
    --lib`; `cargo test -p pantograph-workflow-service scheduler::store
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and targeted forbidden-path
    search over the entrypoint and adapter.
  - Remaining follow-up: connect session execution to the scheduler-task
    entrypoint loop, advance dependent tasks when materialized inputs become
    ready, wire runtime inference dispatch through the runtime-host handoff
    port, and remove the old output-demand launch path rather than preserving
    it as a compatibility branch.
- 2026-05-24 Milestone 5c dependent non-runtime readiness advancement slice
  completed:
  - Smallest useful vertical slice: advance one active-run dependent
    non-runtime scheduler task from `AwaitingInputs` after its scheduler-owned
    materialized inputs validate.
  - Allowed write set: `scheduler/task_orchestrator.rs`,
    `scheduler/task_orchestrator_tests.rs`, workflow-service README,
    Milestone 5c plan notes, task-level orchestration plan notes, and this
    execution log. Existing unrelated Pumas proposal Markdown changes remain
    ignored.
  - Implementation notes: added
    `WorkflowSchedulerTaskOrchestrator::advance_awaiting_non_runtime_task_inputs`.
    The method reads active-run task graph/state, rejects non-non-runtime
    tasks, requires current `AwaitingInputs`, validates typed materialized
    scheduler task results, leaves missing upstream input blocked, advances
    valid text bindings to `Ready(NonRuntime)`, and maps unavailable or invalid
    upstream values to typed scheduler task-state diagnostics.
  - No-fallback/no-legacy confirmation: the slice does not call node-engine
    output demand, `workflow_run_internal`, `PlannedInferenceExecutionHost`, or
    `execute_puma_lib`; it consumes active-run task graph bindings and stored
    `WorkflowSchedulerTaskResult` values only.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
    test -p pantograph-workflow-service
    orchestrator_advances_dependent_non_runtime_task --lib`; `cargo test -p
    pantograph-workflow-service scheduler::task_orchestrator --lib`; `cargo
    test -p pantograph-workflow-service workflow::non_runtime_task_adapter
    --lib`; `cargo test -p pantograph-workflow-service scheduler::store
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and targeted forbidden-path
    search over the orchestrator and adapter.
  - Remaining follow-up: connect session execution to the scheduler-task
    entrypoint/readiness loop, wire runtime inference dispatch through the
    runtime-host handoff port, add cancellation/retry/defer idempotency, and
    remove the old output-demand launch path rather than preserving it as a
    compatibility branch.
- 2026-05-24 Milestone 5c store-lock-safe non-runtime entrypoint split
  completed:
  - Smallest useful vertical slice: replace the single async non-runtime
    execution helper that required `&mut WorkflowExecutionSessionStore` across
    an await with production-safe start/execute/complete/fail operations.
  - Allowed write set: `scheduler/task_orchestrator.rs`,
    `scheduler/task_orchestrator_tests.rs`, workflow-service README,
    Milestone 5c plan notes, task-level orchestration plan notes, and this
    execution log. Existing unrelated Pumas proposal Markdown changes remain
    ignored.
  - Implementation notes: added `StartedNonRuntimeTaskExecution` and split
    ready non-runtime task progression into `start_ready_non_runtime_task`,
    `execute_started_non_runtime_task`,
    `complete_started_non_runtime_task`, and
    `fail_started_non_runtime_task`. Start/complete/fail are synchronous store
    mutation boundaries; execute awaits only the non-runtime adapter and holds
    no store reference.
  - No-fallback/no-legacy confirmation: the previous async helper was removed
    rather than preserved as a compatibility path; runtime inference still
    fails closed before node-engine; and the split continues to avoid
    output-node demand, `workflow_run_internal`, `PlannedInferenceExecutionHost`,
    and `execute_puma_lib`.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
    test -p pantograph-workflow-service
    orchestrator_executes_ready_non_runtime_task --lib`; `cargo test -p
    pantograph-workflow-service scheduler::task_orchestrator --lib`; `cargo
    test -p pantograph-workflow-service workflow::non_runtime_task_adapter
    --lib`; `cargo test -p pantograph-workflow-service scheduler::store
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and targeted forbidden-path
    search over the orchestrator and adapter.
  - Remaining follow-up: wire `run_workflow_execution_session` to consume the
    split scheduler-task loop, remove the scoped staging `dead_code`
    allowances, wire runtime inference dispatch through the runtime-host
    handoff port, add cancellation/retry/defer idempotency, and remove the old
    output-demand launch path rather than preserving it as a compatibility
    branch.
- 2026-05-24 Milestone 5c session cutover replan boundary recorded:
  - Discovery: after the store-lock-safe non-runtime split, the next production
    implementation point is `run_workflow_execution_session`. That path still
    performs runtime preflight, execution-plan production, reservation events,
    and runtime load before calling the legacy whole-run node-engine execution
    path.
  - Boundary: inserting the scheduler-task loop without an explicit sequence
    would either preserve legacy output-demand fallback for runtime-containing
    workflows or implicitly bypass runtime admission/load for non-runtime-only
    workflows. Both choices are scheduler policy, not incidental wiring.
  - Required planning before code: decide how to read the active-run task
    graph class summary before runtime load; decide whether non-runtime-only
    runs complete through scheduler task results without runtime admission/load;
    and decide whether runtime-containing runs should fail closed with typed
    "runtime dispatch not wired" diagnostics until runtime-host dispatch lands
    or whether runtime dispatch must be implemented before session cutover.
  - No-fallback/no-legacy confirmation: the plan now forbids keeping
    `workflow_run_internal` output demand as a compatibility branch for tasks
    already handled by the scheduler loop.
- 2026-05-24 Milestone 5c source-input scheduler contract slice completed:
  - Smallest useful vertical slice: replace graph-data-backed source
    `text-input`/`boolean-input` non-runtime execution with explicit
    source-input scheduler task projection and materialization contracts.
  - Allowed write set: workflow-service task graph contracts, classification,
    projection, external-input materialization, non-runtime adapter,
    task-orchestrator, task-run summary, task-state read model, focused tests,
    workflow-service README, Milestone 5c/task-level orchestration plan notes,
    and this execution log. Existing unrelated Pumas proposal Markdown changes
    remain ignored.
  - Implementation notes: task graph schema version 4 adds
    `WorkflowSchedulerTaskExecutionClass::SourceInput` and
    `WorkflowSchedulerSourceInputTemplate`; `WorkflowSchedulerNonRuntimeTaskTemplate`
    no longer contains `TextInput` or `BooleanInput`; source-input tasks
    initialize as `AwaitingInputs`; external input materialization now consumes
    typed source-input templates and emits typed scheduler task results; run
    summaries and read models distinguish source inputs from node-engine work.
  - No-fallback/no-legacy confirmation: projection does not read request
    values from graph node data; source inputs do not execute through the
    non-runtime adapter; and the slice does not call output-node demand,
    `workflow_run_internal`, runtime dispatch, or Pumas path resolution.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
    test -p pantograph-workflow-service workflow::tests::task_graph --lib`;
    `cargo test -p pantograph-workflow-service
    workflow::external_input_materialization --lib`; `cargo test -p
    pantograph-workflow-service workflow::non_runtime_task_adapter --lib`;
    `cargo test -p pantograph-workflow-service workflow::task_run_summary
    --lib`; `cargo test -p pantograph-workflow-service
    workflow::tests::task_state_read_model --lib`; `cargo test -p
    pantograph-workflow-service scheduler::task_orchestrator --lib`; `cargo
    test -p pantograph-workflow-service scheduler::store --lib`; `cargo test
    -p pantograph-workflow-service workflow::task_result_output_projection
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; and `cargo check -p
    pantograph-workflow-service --all-features`.
  - Discovered issue resolved in-slice: focused test compilation found stale
    `create_session` call sites missing the canonical attribution argument in
    touched test fixtures; those call sites were updated to the current
    six-argument API. Remaining follow-up: wire the session runner to a
    store-owned atomic source-input materialization operation and remove the
    staged `external_input_materialization` dead-code allowance when consumed.
- 2026-05-24 Milestone 5c source-input materialization store slice completed:
  - Smallest useful vertical slice: add source-input materialization to the
    shared scheduler task-state contract and workflow-service active-run
    result/state store boundary.
  - Allowed write set: `pantograph-scheduler` queue contracts/tests/README,
    workflow-service scheduler store task-result module/tests/README,
    task-state read-model exhaustiveness, Milestone 5c/task-level orchestration
    plan notes, and this execution log. Existing unrelated Pumas proposal
    Markdown changes remain ignored.
  - Implementation notes: added `SchedulerSourceInputTaskIntent` and
    `SchedulerSourceInputTaskKind`; allowed `AwaitingInputs -> Completed` for
    source-input materialization with source-input intent; added
    `materialize_active_run_source_input_task` to validate source-input task
    class/template, completed task-result correlation, current awaiting-inputs
    state, and the source-input completion transition before atomically storing
    the result and completed task-state record.
  - No-fallback/no-legacy confirmation: the slice does not fake node-engine
    `Running`, does not execute source inputs through runtime/non-runtime
    adapters, does not mutate graph data, and does not call output demand or
    `workflow_run_internal`.
  - Verification passed: `cargo fmt -p pantograph-scheduler -p
    pantograph-workflow-service -- --check`; `cargo test -p
    pantograph-scheduler --test queue_state`; `cargo test -p
    pantograph-workflow-service scheduler::store::store_task_results --lib`;
    `cargo test -p pantograph-workflow-service scheduler::store --lib`; `cargo
    test -p pantograph-workflow-service workflow::tests::task_state_read_model
    --lib`; `cargo check -p pantograph-scheduler`; `cargo check -p
    pantograph-scheduler --no-default-features`; `cargo check -p
    pantograph-scheduler --all-features`; `cargo check -p
    pantograph-workflow-service`; `cargo check -p pantograph-workflow-service
    --no-default-features`; `cargo check -p pantograph-workflow-service
    --all-features`; and `git diff --check`.
  - Remaining follow-up: the dedicated session runner must consume
    `materialize_external_workflow_inputs`, call the new store materialization
    operation for each source-input result, advance dependent task readiness,
    and remove staged dead-code allowances once the helpers are wired.
- 2026-05-24 Milestone 5c orchestrator source-input materialization slice
  completed:
  - Smallest useful vertical slice: add the orchestrator-owned source-input
    materialization call that consumes the external-input converter and atomic
    active-run source-input store boundary.
  - Allowed write set: workflow-service scheduler orchestrator/tests/README,
    workflow-service facade exports, Milestone 5c/task-level orchestration
    plan notes, and this execution log. Existing unrelated Pumas proposal
    Markdown changes remain ignored.
  - Implementation notes: added
    `WorkflowSchedulerTaskOrchestrator::materialize_external_inputs_for_active_run`
    to read active-run scheduler task graph/state, convert request
    `WorkflowPortBinding` inputs to typed source-input task results, build
    source-input task-state materialization transitions, and commit through
    `materialize_active_run_source_input_task`.
  - No-fallback/no-legacy confirmation: source inputs still bypass
    node-engine, runtime dispatch, output demand, and `workflow_run_internal`;
    graph node data is not mutated and source values are not stored outside
    the atomic source-input result/state operation.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service
    scheduler::task_orchestrator --lib`; `cargo test -p
    pantograph-workflow-service workflow::external_input_materialization --lib`;
    `cargo test -p pantograph-workflow-service
    scheduler::store::store_task_results --lib`; `cargo check -p
    pantograph-workflow-service`; `cargo check -p pantograph-workflow-service
    --no-default-features`; `cargo check -p pantograph-workflow-service
    --all-features`; and `git diff --check`.
  - Remaining follow-up: wire the dedicated session runner to call the new
    orchestrator method, advance dependent readiness, execute ready
    non-runtime tasks, project requested outputs from scheduler task results,
    and remove remaining orchestrator staging `dead_code` allowances.
- 2026-05-24 Milestone 5c non-runtime-only session runner cutover slice
  completed:
  - Smallest useful vertical slice: route non-runtime-only session runs through
    scheduler task progression after queue admission while bypassing runtime
    admission/preflight/load and the legacy whole-run host execution path.
  - Allowed write set: workflow-service session execution, scheduler
    orchestrator re-export/cfg cleanup, focused session execution tests,
    scheduler README, Milestone 5c/task-level orchestration plan notes, and
    this execution log. Existing unrelated Pumas proposal Markdown changes
    remain ignored.
  - Implementation notes: session execution now precomputes scheduler task
    graph and initial task-state records before runtime admission, summarizes
    the run class, skips runtime admission for non-runtime-only graphs,
    materializes request source inputs through the orchestrator, advances
    dependent non-runtime readiness, executes ready non-runtime tasks through
    the single-task adapter, projects requested outputs from scheduler task
    results, and finishes the run without `workflow_run_internal`.
  - No-fallback/no-legacy confirmation: the non-runtime-only branch does not
    call runtime admission, runtime preflight/load, runtime-host dispatch,
    output demand, or the legacy whole-run host path.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service
    workflow::tests::session_execution::workflow_execution_session_lifecycle_create_run_close
    --lib`; `cargo test -p pantograph-workflow-service
    scheduler::task_orchestrator --lib`; `cargo test -p
    pantograph-workflow-service workflow::task_result_output_projection --lib`;
    `cargo check -p pantograph-workflow-service`; and `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and `git diff --check`.
  - Discovered issue: `cargo test -p pantograph-workflow-service
    workflow::tests::session_execution --lib` still includes legacy
    whole-run-host/runtime expectations over non-runtime text graphs. Later
    cleanup must convert runtime-behavior tests to runtime-task graphs or
    assert scheduler diagnostics for non-runtime-only runs.
  - Remaining follow-up: runtime-containing session runs still need the
    dispatch-selected runtime-host handoff cutover, and remaining orchestrator
    staging `dead_code` allowances should be removed when that branch consumes
    the runtime handoff APIs.
- 2026-05-24 Milestone 5c runtime-containing fail-closed session slice
  completed:
  - Smallest useful vertical slice: remove the successful legacy
    runtime-containing session branch until actual dispatch-selected
    runtime-host handoff is wired.
  - Allowed write set: workflow-service session execution, scheduler
    orchestrator and focused tests, scheduler README, Milestone 5c/task-level
    orchestration plan notes, and this execution log. Existing unrelated Pumas
    proposal Markdown changes remain ignored.
  - Implementation notes: session execution now skips runtime admission,
    runtime preflight/load, and `workflow_run_internal` for runs with runtime
    inference scheduler tasks. The orchestrator applies scheduler-validated
    terminal failed transitions to active runtime tasks with typed
    `SchedulerPolicyError` diagnostics, and the workflow API returns a
    capability violation explaining that runtime dispatch must go through a
    dispatch-selected scheduler runtime-host handoff.
  - No-fallback/no-legacy confirmation: runtime-containing runs no longer use
    whole-run host execution, node-engine output demand, runtime load, or
    reduced-plan launch as a compatibility path while runtime handoff dispatch
    is incomplete.
  - Verification passed: `cargo fmt -p pantograph-workflow-service`; `cargo
    test -p pantograph-workflow-service
    scheduler::task_orchestrator::tests::orchestrator_marks_runtime_tasks_terminal_when_dispatch_is_not_wired
    --lib`; `cargo test -p pantograph-workflow-service
    workflow::tests::session_execution::workflow_execution_session_runtime_run_fails_closed_before_legacy_launch
    --lib`; and `cargo test -p pantograph-workflow-service
    workflow::task_run_summary --lib`.
  - Remaining follow-up: wire actual runtime inference tasks through
    dispatch-selected `SchedulerRuntimeHandoff` values and handle
    Pumas-materialization-only/unsupported task-class terminal behavior in the
    scheduler-task session runner.
- 2026-05-24 Milestone 5c unhandled scheduler task-class fail-closed slice
  completed:
  - Smallest useful vertical slice: remove the remaining successful
    session-runner fallback for Pumas-materialization-only or unsupported task
    classes.
  - Allowed write set: workflow-service session execution, scheduler
    orchestrator and focused tests, Milestone 5c/task-level orchestration plan
    notes, and this execution log. Existing unrelated Pumas proposal Markdown
    changes remain ignored.
  - Implementation notes: session execution now starts the scheduler-task run
    and terminal-fails unhandled non-completed task classes through the
    orchestrator instead of entering runtime admission/preflight/load or
    `workflow_run_internal`. The orchestrator uses scheduler-validated
    terminal transitions with `SchedulerPolicyError` diagnostics for task
    classes without a typed execution path.
  - No-fallback/no-legacy confirmation: the old whole-run session branch is no
    longer a successful path for scheduler-task runs. This slice does not add
    compatibility shims or suppress the dead-code warnings exposed by making
    the old branch unreachable.
  - Verification passed with expected dead-code warnings from newly retired
    legacy surfaces: `cargo fmt -p pantograph-workflow-service`; `cargo test
    -p pantograph-workflow-service
    scheduler::task_orchestrator::tests::orchestrator_marks_unhandled_task_classes_terminal_failed
    --lib`; `cargo test -p pantograph-workflow-service
    scheduler::task_orchestrator --lib`; `cargo test -p
    pantograph-workflow-service
    workflow::tests::session_execution::workflow_execution_session_runtime_run_fails_closed_before_legacy_launch
    --lib`; `cargo check -p pantograph-workflow-service`; and `git diff
    --check`.
  - Discovered issue: making the old session runner unreachable exposes
    retired runtime-load/session-admission helpers, execution-plan admission,
    runtime-reservation diagnostic helpers, queue runtime-admission fields,
    `workflow_run_internal`, and media artifactization helpers as compiler
    dead-code warnings. Next slice must remove or reconnect these through the
    canonical scheduler/runtime-host paths rather than adding allow attributes.
- 2026-05-24 Milestone 5c legacy-surface cleanup replan recorded:
  - Decision: use option 2 before continuing runtime-dispatch implementation.
    The next slice must classify newly exposed dead/legacy surfaces by owner
    and action, then delete, reattach through dispatch-selected
    scheduler/runtime-host ownership, or convert to scheduler task-result/output
    ownership.
  - No-fallback/no-legacy confirmation: `workflow_run_internal`, output-node
    demand, reduced execution-plan launch/admission, old runtime
    admission/preflight/load, graph-local model-path identity, and old
    whole-run artifactization are not compatibility branches. Dead-code
    warnings from those surfaces are cleanup blockers, not warnings to
    suppress.
  - Initial classification recorded in Milestone 5c and the task-level
    orchestration plan. Existing unrelated Pumas proposal Markdown changes
    remain ignored.
  - Verification for this documentation slice: run `git diff --check` and
    inspect the plan diff. Code verification belongs to the following cleanup
    implementation slice.
- 2026-05-24 Milestone 5c reduced execution-plan admission cleanup slice
  completed:
  - Smallest useful vertical slice: remove the reduced execution-plan
    admission helper exposed as retired dead code by the scheduler-task
    session-runner cutover.
  - Allowed write set: workflow-service facade module list/export,
    workflow-service `workflow/README.md`, workflow-service contract tests,
    deletion of `workflow/execution_plan_admission.rs`, Milestone 5c/task-level
    orchestration plan notes, and this execution log. Existing unrelated Pumas
    proposal Markdown changes remain ignored.
  - No-fallback/no-legacy confirmation: technical-fit admission can no longer
    synthesize a reduced executable run plan through
    `build_workflow_execution_plan_from_admission`. Runtime execution remains
    limited to scheduler task graph/state plus dispatch-selected runtime-host
    handoff; this slice does not add shims or replacement fallback behavior.
  - Verification passed: `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service workflow::tests::contracts
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; targeted source search for
    `build_workflow_execution_plan_from_admission`,
    `execution_plan_admission`, and `workflow_execution_plan_admission`; and
    `git diff --check`.
  - Remaining follow-up: remove or canonically reattach the old queue
    runtime-admission fields/helpers, retired runtime-load/session-admission
    diagnostics helpers, `session_runtime_load_lifecycle`,
    `workflow_run_internal`, and the old media artifactization conversion
    boundary.
- 2026-05-24 Milestone 5c unused queue helper cleanup slice completed:
  - Smallest useful vertical slice: delete unused queue prediction/update
    helpers that were exposed as dead code by the scheduler-task session
    runner cutover.
  - Allowed write set: workflow-service scheduler store queue module,
    Milestone 5c/task-level orchestration plan notes, and this execution log.
    Existing unrelated Pumas proposal Markdown changes remain ignored.
  - No-fallback/no-legacy confirmation: actual run admission remains owned by
    `begin_queued_run` and scheduler policy. The removed helpers no longer
    offer a side path for predicted-admission polling or queue decision
    mutation.
  - Verification passed: targeted source search for
    `queued_run_is_admission_candidate` and
    `set_queue_decision_reason_if_present`; `cargo fmt -p
    pantograph-workflow-service -- --check`; `cargo test -p
    pantograph-workflow-service scheduler::store --lib`; `cargo check -p
    pantograph-workflow-service`; `cargo check -p pantograph-workflow-service
    --no-default-features`; `cargo check -p pantograph-workflow-service
    --all-features`; and `git diff --check`.
  - Remaining follow-up: continue the cleanup gate with stale
    queued/dequeued/preflight fields, retired runtime-load/session-admission
    diagnostics helpers, `session_runtime_load_lifecycle`,
    `workflow_run_internal`, and the old media artifactization conversion
    boundary.
- 2026-05-24 Milestone 5c scheduler session timeout reattachment slice
  completed:
  - Smallest useful vertical slice: reattach queued-run `timeout_ms` to the
    canonical scheduler-task session runner for non-runtime scheduler-task
    execution.
  - Allowed write set: workflow-service session execution API, focused
    workflow-service session execution test, Milestone 5c/task-level
    orchestration plan notes, and this execution log. Existing unrelated Pumas
    proposal Markdown changes remain ignored.
  - No-fallback/no-legacy confirmation: the timeout is enforced around
    scheduler-task execution and returns typed `RuntimeTimeout`; the slice does
    not route through `workflow_run_internal`, runtime admission/load, or
    node-engine output demand.
  - Verification passed: `cargo fmt -p pantograph-workflow-service --
    --check`; `cargo test -p pantograph-workflow-service
    workflow::tests::session_execution::workflow_execution_session_timeout_applies_to_scheduler_task_runner
    --lib`; `cargo test -p pantograph-workflow-service
    workflow::tests::session_execution::workflow_execution_session_lifecycle_create_run_close
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and `git diff --check`.
  - Remaining follow-up: runtime dispatch timeouts, cancellation, attempt
    timing, and ledger-backed duration history belong to the later
    scheduler/runtime-host lifecycle slice. Continue cleanup of stale
    queued/dequeued/preflight fields, retired runtime-load/session-admission
    diagnostics helpers, `session_runtime_load_lifecycle`,
    `workflow_run_internal`, and the old media artifactization conversion
    boundary.
- 2026-05-24 Milestone 5c preflight cache cleanup slice completed:
  - Smallest useful vertical slice: remove stale capability-model and
    technical-fit-decision payloads from the session preflight cache.
  - Allowed write set: workflow-service scheduler store, workflow-service
    session runtime preflight cache builder, focused preflight cache tests,
    Milestone 5c/task-level orchestration plan notes, and this execution log.
    Existing unrelated Pumas proposal Markdown changes remain ignored.
  - No-fallback/no-legacy confirmation: preflight cache tests now call the
    preflight boundary directly instead of relying on successful whole-run
    session execution. The cache remains a readiness/invalidation cache and
    does not preserve old runtime admission or launch data.
  - Verification passed: targeted source search for stale preflight cache
    fields; `cargo fmt -p pantograph-workflow-service -- --check`; `cargo test
    -p pantograph-workflow-service workflow::tests::session_runtime_preflight
    --lib`; `cargo test -p pantograph-workflow-service scheduler::store
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and `git diff --check`.
  - Remaining follow-up: continue cleanup of stale dequeued/finish-state
    fields, reduced execution-plan active-run storage, retired
    runtime-load/session-admission diagnostics helpers,
    `session_runtime_load_lifecycle`, `workflow_run_internal`, and the old
    media artifactization conversion boundary.
- 2026-05-24 Milestone 5c stale queue payload cleanup slice completed:
  - Smallest useful vertical slice: remove stale dequeued-run and finish-state
    payload fields that no active scheduler policy or read model consumes.
  - Allowed write set: workflow-service scheduler store/queue files,
    existing focused scheduler/session tests, Milestone 5c/task-level
    orchestration plan notes, and this execution log. Existing unrelated Pumas
    proposal Markdown changes remain ignored.
  - No-fallback/no-legacy confirmation: `WorkflowExecutionSessionDequeuedRun`
    no longer copies `required_backends` or `required_models`, and
    `WorkflowExecutionSessionRunFinishState` no longer returns a redundant
    `workflow_id`. Session affinity and preflight requirements remain on
    canonical session/preflight state and admission/placement projections;
    finish state now reports only the active runner's `unload_runtime`
    decision.
  - Verification passed: targeted source inspection of the dequeued and finish
    constructors; `cargo fmt -p pantograph-workflow-service -- --check`;
    `cargo test -p pantograph-workflow-service scheduler::store --lib`;
    `cargo test -p pantograph-workflow-service
    workflow::tests::session_execution::workflow_execution_session_lifecycle_create_run_close
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and `git diff --check`.
  - Remaining follow-up: reduced execution-plan active-run storage, retired
    runtime-load/session-admission diagnostics helpers,
    `session_runtime_load_lifecycle`, `workflow_run_internal`, and the old
    media artifactization conversion boundary.
- 2026-05-24 Milestone 5c retired runtime-load lifecycle cleanup slice
  completed:
  - Smallest useful vertical slice: delete the unused
    `session_runtime_load_lifecycle` module and its private model-lifecycle
    event request/helper entry point.
  - Allowed write set: workflow-service module declarations,
    `session_execution_api.rs`, workflow-service README ownership docs,
    Milestone 5c/task-level orchestration plan notes, and this execution log.
    Existing unrelated Pumas proposal Markdown changes remain ignored.
  - No-fallback/no-legacy confirmation: the old runtime-load lifecycle helper
    is not preserved as a compatibility diagnostic path. Runtime load,
    dispatch, retry, cancellation, and attempt timing diagnostics must be
    reintroduced through the later scheduler/runtime-host lifecycle owner with
    task/runtime handoff correlation.
  - Verification passed: targeted source search for deleted lifecycle symbols;
    `cargo fmt -p pantograph-workflow-service -- --check`; `cargo test -p
    pantograph-workflow-service
    workflow::tests::session_execution::workflow_execution_session_lifecycle_create_run_close
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and `git diff --check`.
  - Discovered issue: `cargo test -p pantograph-workflow-service
    workflow::tests::session_capacity --lib` still fails because that broader
    suite expects legacy runtime/session capacity behavior while current
    scheduler-task session execution blocks source-input runs or fails closed.
    Keep those tests as a later conversion/deletion target for the dedicated
    scheduler/runtime-host lifecycle and source-input materialization slices.
  - Remaining follow-up: reduced execution-plan active-run storage, remaining
    retired runtime-load/session-admission diagnostics helpers,
    `workflow_run_internal`, and the old media artifactization conversion
    boundary.
- 2026-05-24 Milestone 5c retired session-admission diagnostics cleanup slice
  completed:
  - Smallest useful vertical slice: delete unused scheduler delay, admitted,
    reservation, runtime-load error, technical-fit trace mapping, retry
    timestamp, and queued graph-settings helper surfaces from
    `session_execution_api.rs`.
  - Allowed write set: `session_execution_api.rs`, Milestone 5c/task-level
    orchestration plan notes, and this execution log. Existing unrelated Pumas
    proposal Markdown changes remain ignored.
  - No-fallback/no-legacy confirmation: old session-admission diagnostics are
    not kept as inert helper code. Scheduler delay, admission, reservation,
    runtime-load failure, and retry timing diagnostics must be reintroduced
    through the scheduler-task lifecycle owner with task ids, attempt ids, and
    runtime handoff correlation.
  - Verification passed: targeted source search for deleted helper symbols;
    `cargo fmt -p pantograph-workflow-service -- --check`; `cargo test -p
    pantograph-workflow-service workflow::session_execution_api::tests --lib`;
    `cargo test -p pantograph-workflow-service
    workflow::tests::session_execution::workflow_execution_session_lifecycle_create_run_close
    --lib`; `cargo check -p pantograph-workflow-service`; `cargo check -p
    pantograph-workflow-service --no-default-features`; `cargo check -p
    pantograph-workflow-service --all-features`; and `git diff --check`.
  - Remaining follow-up: reduced execution-plan active-run storage,
    `workflow_run_internal`, and the old media artifactization conversion
    boundary.
- 2026-05-24 Milestone 5c old whole-run execution cleanup slice completed:
  - Smallest useful vertical slice: remove the private `workflow_run_internal`
    whole-run host execution path, its test-only caller module, the old
    `artifact_output_conversion` module, host-injected media conversion
    configuration, and the now-unused workflow-service
    `pantograph-media-conversion` dependency.
  - Allowed write set: workflow-service crate manifest and lockfile,
    workflow-service module declarations/config/tests/fixtures/READMEs,
    deleted whole-run/artifactization modules and tests, Milestone 5c/task-
    level orchestration plan notes, and this execution log. Existing unrelated
    Pumas proposal Markdown changes remain ignored.
  - No-fallback/no-legacy confirmation: the old node-engine whole-run host
    launch and whole-run artifactization path are removed rather than kept as
    private compatibility helpers. Future artifact/media output handling must
    be scheduler-task result materialization and runtime-host output
    projection work, not `workflow_run_internal` output conversion.
  - Verification passed: targeted source/doc search for old whole-run and
    media-conversion symbols; `cargo fmt -p pantograph-workflow-service
    -- --check`; `cargo test -p pantograph-workflow-service
    workflow::tests::session_execution::workflow_execution_session_lifecycle_create_run_close
    --lib`; `cargo test -p pantograph-workflow-service
    workflow::tests::task_result_contracts --lib`; `cargo check -p
    pantograph-workflow-service`; `cargo check -p pantograph-workflow-service
    --no-default-features`; `cargo check -p pantograph-workflow-service
    --all-features`; and `git diff --check`.
  - Remaining follow-up/replan boundary: reduced execution-plan active-run
    storage is still present and read by the embedded-runtime planned
    inference host. Removing it requires replacing the cross-crate
    planned-inference bridge with scheduler-selected runtime handoff
    dispatch, not a workflow-service-only deletion.

### Traceability Links

- Module README updated: N/A for Milestone 0 because no production module
  ownership changed.
- ADR added/updated: N/A unless implementation changes runtime backend
  ownership boundaries.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: pending.
