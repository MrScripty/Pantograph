# Inputs And Contracts

## Inputs

### Problem

The Juggernaut X v10 SDXL graph currently renders as broken:

- The diffusion node has no body.
- Edges appear absent or unconnected.
- The Puma-Lib node reports that it cannot find the model.
- The graph still uses retired `diffusion-inference` structure.
- There are duplicate saved Juggernaut workflow files with the same display
  name.

The likely executable path is PyTorch with the Python `diffusers` library.
Pantograph has pieces of the typed image-generation contract and a PyTorch
worker diffusion path, but the current graph and tooling do not consistently
route diffusion models through that path.

Pantograph also lacks a coherent device-selection contract across inference
backends. The initial implementation should support CPU and CUDA on Linux and
Windows, plus Metal/MPS on macOS. ROCm/HIP, Vulkan, XPU/iGPU, OpenVINO, remote
hardware plugins, and hybrid/offload should remain future extensions. llama.cpp
in particular can require different runtime artifacts for the same release
version depending on the selected compute backend, so Pantograph needs a small
canonical device policy that resolves into backend-specific settings only
after backend readiness is known.

Backend choice must be implicit by default and scheduler-owned. The scheduler
needs typed facts from all inference backends so it can choose a backend,
runtime variant, and device placement that suits the model, task, available
RAM/VRAM, client intent, and host capabilities. The inference crate should
therefore expose backend adapter facts and execute the selected decision, while
the scheduler owns ranking, reservations, queue policy, and future learned
throughput decisions.

### Current Codebase Findings

- `diffusion-inference` is retired and intentionally absent from the current
  workflow node registry.
- Canonical image generation is represented by `llm-inference` with
  `task_kind = image_generation`.
- The Tiny SD Turbo template already uses the current graph shape and should be
  used as the local reference pattern.
- `src-tauri/src/bin/pumas_dependency_runtime_probe.rs` still maps diffusion
  model records to `diffusion-inference`, which conflicts with current
  standards.
- The saved Juggernaut workflow files contain stale Puma model facts. One file
  has only a display-style model name and empty path; the other has a stale raw
  model path outside the current Pumas repository root.
- The actual Pumas Juggernaut metadata exists under the current Pumas library
  root and identifies the model as a diffusers-directory diffusion model.
- Candle upstream has diffusion examples, but Pantograph's Candle backend
  remains staged and reports unavailable until executable model loading exists.
- `crates/inference/src/device.rs` already has a llama.cpp-oriented
  `DeviceBackend` enum for `Cpu`, `Cuda`, `Vulkan`, `Metal`, and `Auto`, plus
  llama.cpp `--list-devices` parsing. The first implementation should narrow
  this to CPU/CUDA on Linux/Windows and Metal on macOS while keeping unsupported
  variants staged for later.
- `crates/inference/src/managed_runtime/llama_cpp_platform/linux.rs` currently
  switches to a nested CUDA `llama-server` binary when `--device CUDA...` is
  requested. This is an implicit runtime-variant selection path and should be
  made explicit in managed runtime contracts.
- `ManagedBinaryId` currently has one `LlamaCpp` identity. That is not enough
  to represent multiple installed runtime variants for the same llama.cpp
  release unless the install-state model grows a variant dimension.
- `BackendConfig.device`, `LlamaCppRuntimeSettings.device`,
  `InferenceRequestLifecycleEvent.selected_device_id`,
  `ServerModeInfo.active_resolved_device`, and diagnostics ledger fields show
  that device facts already exist in several layers, but they are not yet
  governed by one validated device policy.
- PyTorch worker code currently accepts strings such as `auto`, `cpu`, `cuda`,
  `cuda:0`, and `mps`, and auto-selects `cuda > mps > cpu`. This old behavior
  must be removed or replaced by validated backend-owned resolution. Auto is a
  scheduler policy, not a backup path, and explicit device requests fail when
  invalid.
- Runtime/backend selection currently appears in several local forms and does
  not yet have a single scheduler-facing candidate shape. The plan should add
  typed candidates that let the scheduler compare llama.cpp, PyTorch, vLLM,
  Candle, and future MLX without inspecting raw backend command strings.

### Constraints

- Follow
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`.
- Standards reviewed for this plan: `PLAN-STANDARDS.md`,
  `CODING-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
  `TESTING-STANDARDS.md`, `FRONTEND-STANDARDS.md`,
  `DOCUMENTATION-STANDARDS.md`, `CONCURRENCY-STANDARDS.md`,
  `SECURITY-STANDARDS.md`, `INTEROP-STANDARDS.md`,
  `DEPENDENCY-STANDARDS.md`, `CROSS-PLATFORM-STANDARDS.md`,
  `TOOLING-STANDARDS.md`,
  `ACCESSIBILITY-STANDARDS.md`,
  `languages/rust/RUST-API-STANDARDS.md`,
  `languages/rust/RUST-ASYNC-STANDARDS.md`,
  `languages/rust/RUST-CROSS-PLATFORM-STANDARDS.md`,
  `languages/rust/RUST-INTEROP-STANDARDS.md`,
  `languages/rust/RUST-SECURITY-STANDARDS.md`,
  `languages/rust/RUST-DEPENDENCY-STANDARDS.md`, and
  `languages/rust/RUST-TOOLING-STANDARDS.md`.
- Backend service/domain crates are the source of truth for graph validity,
  stale graph facts, model resolution facts, runtime readiness, and execution
  contracts.
- Frontend renders backend facts and owns only presentation state such as
  selected graph node, panel sizing, hover/focus, and layout.
- Tauri remains an app-shell and IPC adapter. It may transport stale graph
  diagnostics and inference requests, but must not define their semantics.
- No backwards compatibility is required for older Pantograph workflow graph
  shapes, raw device/runtime shapes, backend hints, or execution-selection
  methods. Old methods are removed or replaced by canonical contracts rather
  than preserved as fallback or compatibility branches.
- Retired direct inference nodes must remain unregistered built-ins.
- Pumas remains the canonical model source. Pantograph queries Pumas facts and
  owns final workflow graph structure and runtime selection policy.
- The scheduler owns backend/device/runtime ranking, resource placement, queue
  policy, explicit preference validation, and future learned throughput policy.
  Inference backend adapters own feasibility facts, diagnostics, resource
  estimates where known, and backend-specific execution translation.
- Large media artifacts must be retained through ArtifactStore and IO
  projections, not embedded redundantly in saved workflow JSON.
- The implementation must preserve layered ownership: frontend presentation,
  Tauri IPC adapter, backend workflow/inference domain contracts, and Python
  worker infrastructure stay separate.
- Shared contracts must be defined and frozen before parallel or broad
  implementation begins. Contract changes are handled serially by the
  integration owner.
- Runtime planning, validation, and DTO shaping should use correct-by-
  construction Rust types and structured errors rather than stringly typed
  internal control flow.
- Planning failure is terminal for the requested run. Missing model facts,
  invalid backend/device/runtime requests, unsupported image families, failed
  dependency validation, and unavailable runtime variants must produce typed
  diagnostics instead of falling back to old execution paths.
- New public or cross-crate Rust contracts must expose appropriate `Debug`,
  explicit serde casing, `#[non_exhaustive]` where additive extension is likely,
  `#[must_use]` for decisions/builders/results that should not be ignored, and
  structured error enums for fallible operations.
- Raw graph JSON, Pumas package facts, model paths, dimensions, schedulers,
  device selections, runtime-variant selections, and dependency-environment
  identifiers must be parsed and validated once at the backend boundary before
  trusted internal use.
- Runtime roots, executable paths, dynamic-library paths, Pumas package paths,
  artifact paths, and worker-visible paths must be validated against allowed
  roots through shared validation before filesystem, subprocess, or worker use.
- Image dimensions, context lengths, token limits, batch sizes, byte ranges,
  memory estimates, and output-size calculations must use checked arithmetic
  and typed diagnostics before allocation, slicing, or scheduling decisions.
- Async code should keep planning, validation, and transformation synchronous
  where possible. Async shells own Pumas I/O, artifact I/O, worker calls, and
  IPC boundaries.
- Blocking filesystem, process, or Python-worker operations must not run while
  holding async locks or inside unbounded async critical sections.
- Runtime creation, subprocess/server lifecycle, worker pools, local listeners,
  and background tasks must be owned by a composition root or explicit
  lifecycle manager with bounded queues, cancellation, panic/error reporting,
  and shutdown behavior.
- Local backend services must bind to loopback only and define bounded
  connection/request limits, startup/readiness timeouts, and graceful shutdown.
- Runtime feature flags and optional dependencies must remain documented and
  verified in default, no-default-features, and all-features modes when touched.
- Frontend changes must remain declarative, event/subscription-driven where
  feasible, and must not introduce polling loops or optimistic updates for
  backend-owned graph, diagnostic, model, runtime, or artifact facts.
- Frontend runtime/device controls and diagnostic actions must use accessible
  names, keyboard interaction where interactive, resilient tests, and
  deterministic subscription or scoped-poll cleanup.
- Files or UI components that cross the standards decomposition thresholds
  require an explicit split or a short documented reason why the current shape
  remains safe.
- Changes must proceed in validated thin vertical slices with atomic commits
  during implementation.

### Assumptions

- The retained Juggernaut workflow should target the current Pumas model id
  `diffusion/rundiffusion/juggernaut-x-v10`.
- The first real diffusion backend should be PyTorch/diffusers, but the design
  should cover a class of diffusers-directory image-generation models rather
  than a single saved workflow.
- Pumas package facts should contain or expose enough information to identify
  the image-generation package kind, pipeline family/class, component layout,
  dependency requirements, scheduler metadata, supported options, and device or
  precision constraints. If those facts are insufficient, Pantograph should
  report an explicit planning diagnostic rather than guessing.
- Device choice should be modeled as a user policy plus backend resolution
  result. The workflow may request a device class or concrete device, but the
  backend adapters and runtime registry report whether that request maps to an
  installed runtime variant and available host device, and scheduler admission
  owns the final accept/reject decision.
- Workflows may also express backend/runtime preference and latency or
  throughput intent. Those are scheduler inputs, not guaranteed execution
  commands. Invalid explicit preferences should fail with diagnostics rather
  than fallback.
- Transformers ecosystem conventions should guide canonical task, modality,
  processor/tokenizer, generation config, dtype/precision, and model-source
  naming until a backend adapter translates into llama.cpp, PyTorch, vLLM,
  Candle, or future MLX-specific settings.
- Tiny SD Turbo should be used as a fast smoke fixture; Juggernaut should be
  used as the first larger repaired saved workflow. Other diffusion/image
  families should work when Pumas facts, dependencies, and backend support are
  present.
- Current graph authoring uses `backend_key = "pytorch"` for executable
  diffusion/image-generation intent. `diffusers` remains a dependency and
  capability label until Pantograph owns a separately registered Diffusers
  backend; it is not a graph-visible backend preference.
- Stale graph diagnostics are factual validation results, not workflow-run
  diagnostic events unless they occur during submission, admission, or
  execution.
- Stale graphs should remain inspectable in the IO inspector through a
  saved-graph inspection mode even when they cannot be submitted.
- Graph editor and IO inspector should use the same backend-owned stale graph
  facts where possible.

### Dependencies

- `crates/workflow-nodes`
- `crates/pantograph-workflow-service`
- `crates/node-engine`
- `crates/inference`
- `crates/pantograph-embedded-runtime`
- `src-tauri/src/workflow`
- `src-tauri/src/bin/pumas_dependency_runtime_probe.rs`
- `src/templates/workflows`
- `.pantograph/workflows`
- `src/components/workbench/IoInspectorPage.svelte`
- `src/components/nodes/workflow/PumaLibNode.svelte`
- `packages/svelte-graph`
- Pumas Library 0.6.0 model snapshot and selected-model detail APIs
- Reference-only implementation guidance:
  `/media/jeremy/OrangeCream/Linux Software/repos/reference/frameworks-libraries/transformers/`,
  `/media/jeremy/OrangeCream/Linux Software/repos/reference/ai-systems/ComfyUI/`,
  and
  `/media/jeremy/OrangeCream/Linux Software/repos/reference/ai-systems/InvokeAI/`.

### Affected Contracts

- Backend stale graph diagnostic DTO.
- Backend workflow graph validation/read-model facade.
- Backend saved-graph inspection query for IO inspector stale graph display.
- Shared graph inspection projection used by run inspection and saved-graph
  inspection without forcing saved graphs into run-only DTO names.
- Tauri command or event payload carrying stale graph facts.
- Frontend stale graph display model.
- Canonical `llm-inference` image-generation graph contract.
- Deterministic image-generation execution plan DTO/internal contract.
- Image-generation model-family adapter contract inside the PyTorch/diffusers
  execution bridge.
- Reference-derived diffusion family taxonomy and option-validation notes used
  by the PyTorch/diffusers planner. These notes must remain Pantograph-owned
  internal contracts, not copied ComfyUI/InvokeAI runtime contracts.
- Image-generation family planner contracts:
  `ImageGenerationFamily`, `ImageGenerationFamilyVariant`,
  `ImageGenerationComponentRole`, `ImageGenerationFamilyAdapter`,
  `ImageGenerationFamilyRequirements`, `ImageGenerationPlanDiagnostic`, and
  `PyTorchDiffusersImageGenerationPlan`.
- PyTorch/diffusers runtime readiness and capability facts.
- Backend device inventory, device policy, runtime variant, and selected
  device readiness contracts.
- Backend execution candidate and selected execution decision contracts for the
  scheduler-facing runtime registry/admission boundary.
- Scheduler-learning fact fields in lifecycle and diagnostics projections:
  selected backend, selected runtime variant, selected device class/id,
  resource estimate when known, execution duration, terminal status, and
  retained artifact output-size descriptors.
- Managed runtime variant catalog/status DTOs for backends such as llama.cpp
  that can expose multiple installable artifacts for one release version.
- PyTorch backend image-generation request/response bridge to the Python
  worker.
- Pumas model identity usage in saved workflow graph data.
