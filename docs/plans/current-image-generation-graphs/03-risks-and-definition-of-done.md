# Risks And Definition Of Done

## Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Graph submit remains blocked without visible reason | High | Add backend-owned stale graph diagnostics before or with submit blocking changes. |
| Diffusion routes to Candle because of stale backend hints | High | Gate Candle image generation as unavailable and prove diffusion uses PyTorch/diffusers. |
| Updating only the saved graph leaves generators/probes producing stale graphs | High | Fix templates/probe code and add tests that reject `diffusion-inference`. |
| Pumas model cannot resolve from saved graph data | Medium | Save stable `model_id` / `pumas_model_ref` and verify selected-model detail resolution. |
| Large generated image data is embedded in workflow JSON | High | Verify generated image output is retained as artifacts and only descriptors are saved. |
| Generated image payloads are duplicated across `image` and `results` outputs | High | Retain one media body and project descriptors/metadata for secondary views. |
| Stale diagnostics become frontend-inferred | Medium | Keep diagnostics in backend DTOs; frontend only renders facts. |
| PyTorch/diffusers dependencies are missing locally | Medium | Add readiness diagnostics with actionable dependency/environment facts. |
| Runtime hint naming drifts between `pytorch` and `diffusers` | Medium | Centralize normalization so `diffusers` resolves to PyTorch execution in this slice, and test both graph hints and Pumas hints. |
| Existing persistence keeps silently canonicalizing retired nodes | High | Replace current app-path rewrites with stale graph diagnostics and update tests that expect silent rewrites. |
| Retired graph behavior remains as hidden compatibility code | High | Delete or rewrite compatibility helpers/tests during touched slices; stale retired shapes become diagnostics only. |
| IO inspector stale graph support becomes a second graph model | Medium | Add a saved-graph inspection query that reuses the same backend graph projection and stale diagnostic DTOs as run inspection. |
| Diffusion support becomes hardcoded to Tiny SD Turbo or Juggernaut | High | Treat both as fixtures/examples and validate the generic Pumas package-facts path for multiple diffusion/image model families. |
| Reference repos are copied as architecture instead of used as guidance | Medium | Borrow naming, taxonomy, and validation ideas only; keep Pantograph's Pumas-owned model source, Rust contracts, artifact store, and backend planner boundaries. |
| Family adapters become unstructured special cases | High | Use `ImageGenerationFamilyRequirements` table data plus typed adapters so per-family behavior is explicit, testable, and additive. |
| Pumas facts do not expose enough family evidence | High | Return missing-facts diagnostics and record the needed Pumas fact field instead of inferring from paths or saved workflow names. |
| Missing Pumas fact requirements are too vague to act on | Medium | Missing-facts diagnostics must list exact absent or ambiguous fields such as pipeline family/class, component role, variant, dtype, scheduler, or custom-code evidence. |
| Broad runtime identity changes collapse useful Diffusers diagnostics into PyTorch | Medium | Add a narrow execution-selection normalization boundary while preserving Diffusers runtime/dependency labels for capability and diagnostic surfaces. |
| Generic gateway option diagnostics compete with family validation | Medium | Treat gateway option diagnostics as lifecycle metadata and make family adapters own accepted/rejected/ignored option semantics. |
| PyTorch backend file absorbs another large feature inline | Medium | Add focused worker bridge helpers or a submodule while keeping the public PyTorch backend facade stable. |
| Python worker dynamically discovers pipeline behavior after Rust validation | High | Worker calls must execute a validated `PyTorchDiffusersImageGenerationPlan`; do not rely on generic `DiffusionPipeline` behavior, implicit scheduler swaps, or `trust_remote_code=True` discovery. |
| Runtime silently falls back after image-generation planning fails | High | Plan and validate exactly one execution path; return a diagnostic if it is invalid. |
| Explicit device requests silently fall back to CPU or another accelerator | High | Add a device-resolution decision before backend execution and fail explicit unavailable devices with diagnostics. |
| Auto mode is implemented as old fallback behavior | High | Treat auto as a first-class scheduler policy that must select one valid candidate or fail with typed diagnostics. |
| Technical-fit fallback remains executable | High | Remove `ConservativeFallback`/override-fallback execution and replace it with typed rejection diagnostics. |
| Frontend invents fallback executable device options after backend discovery fails | High | Render backend discovery diagnostics/unavailable state and submit only backend-confirmed policy choices. |
| Backend adapters become hidden schedulers | High | Adapters expose feasibility, estimates, diagnostics, and translation only; scheduler owns ranking, reservations, queue/resource policy, and learned placement. |
| Scheduler has to inspect backend command strings | High | Add typed backend/runtime/device candidate facts before backend-specific command translation. |
| Explicit backend preference routes to an incompatible runtime | High | Validate workflow backend/runtime/device preferences against candidate facts and reject impossible requests with bounded diagnostics. |
| Device planning overgeneralizes backend-specific offload | Medium | Keep `gpu_layers`, CPU/GPU split, and hybrid/offload as backend adapter capabilities rather than cross-backend device policy. |
| Future learned scheduling lacks reliable facts | Medium | Record selected backend/runtime/device, execution duration, terminal status, resource estimate, and artifact output-size descriptors in lifecycle/ledger projections. |
| Backend startup adds unowned runtime/process work | High | Keep runtime creation, subprocess/server lifecycle, worker pools, and shutdown in a composition-root lifecycle owner with tracked tasks and cancellation. |
| Local backend server exposes unsafe transport | High | Require loopback binding, connection/request limits, readiness timeouts, and graceful shutdown for any touched local service. |
| Runtime paths or package paths escape trusted roots | High | Centralize allowed-root validation for executables, dynamic libraries, Pumas packages, artifacts, and worker-visible paths before filesystem or subprocess use. |
| Resource estimates or generated-media sizes overflow | High | Use checked arithmetic and typed diagnostics for dimensions, context/token limits, byte ranges, memory estimates, and output-size calculations. |
| Optional runtime dependencies bloat default builds or break feature modes | Medium | Document runtime feature contracts and run default, no-default-features, and all-features checks when touched. |
| llama.cpp runtime variants overwrite or hide each other | High | Add runtime variant identity under one managed binary source of truth so one release can expose CPU/CUDA and macOS Metal readiness independently. |
| Future device families leak into the initial implementation | Medium | Scope Milestone 5 to CPU/CUDA on Linux/Windows and Metal on macOS; keep ROCm/HIP, Vulkan, XPU/iGPU, OpenVINO, remote hardware plugins, and hybrid/offload as future extensions. |
| Frontend becomes the source of truth for available devices | High | Device options render backend-owned capability facts and submit only validated device policy intent. |
| Pumas facts are insufficient for a model family | High | Reject planning with an explicit missing-facts diagnostic instead of inferring from names or trying generic Diffusers behavior. |
| Workflow options are unsupported by the selected image family | Medium | Validate options during planning and fail with bounded option diagnostics instead of ignoring or substituting values. |
| Denoising scheduler is confused with Pantograph scheduling | Medium | Rename the image sampling option to `denoising_scheduler`; keep it as a generation parameter that never controls workflow queueing, runtime placement, device selection, or retry policy. |
| Frontend option selection silently changes executable behavior | High | Backend-owned port options may present selectable values, but omitted/stale values must remain unset or diagnostic-backed; planner-owned defaults must not be implemented as frontend first-option writes. |
| Port options become a second execution policy engine | Medium | Use `PortOptionsProvider` only to list selectable values. The image-generation planner must revalidate selected values against model/package/runtime facts, family rules, and worker support before execution. |
| New planner or diagnostic contracts become stringly typed | High | Parse raw inputs once into validated Rust types and expose structured DTOs/errors. |
| Async implementation blocks runtime or leaks tasks | High | Keep planning sync, isolate async I/O shells, avoid holding locks across awaits, and add lifecycle ownership for any new task. |
| Frontend saved-graph inspection drifts from backend facts | Medium | Backend owns graph/diagnostic facts; frontend renders declaratively with no optimistic backend-owned state. |
| Frontend device controls become inaccessible or stale-subscription prone | Medium | Use accessible names, keyboard interaction where interactive, resilient selectors, and deterministic subscription/poll cleanup tests. |
| Large touched files become less maintainable | Medium | Perform decomposition review and extract focused helpers/submodules when thresholds are crossed. |

## Definition Of Done

- Only one Juggernaut X v10 SDXL saved workflow remains.
- The retained Juggernaut workflow uses canonical `llm-inference` image
  generation and no `diffusion-inference` nodes.
- The Juggernaut Puma-Lib node resolves the current Pumas model id.
- No current template, probe, or fixture emits `diffusion-inference` for
  diffusion models.
- Current load/save paths do not silently rewrite retired node types into
  current graph shapes.
- Current graph normalization is split from retired-node classification, and
  current app paths use only the current-normalization path plus stale
  diagnostics.
- No old graph-shape compatibility or migration path remains in current
  load/save/session execution paths.
- Backend graph validation reports stale node and stale edge facts.
- IO inspector can display stale graphs and clearly mark stale nodes and edge
  issues.
- Submit/admission failure for stale graphs includes visible, bounded reasons.
- A validated PyTorch/diffusers image-generation vertical slice reaches an
  image output artifact.
- Diffusers hints from saved graphs and Pumas facts resolve to PyTorch
  execution until a registered Diffusers backend exists.
- The image-generation path is proven generic for Pumas diffusers-directory
  image models rather than hardcoded to Tiny SD Turbo or Juggernaut.
- Users can request CPU or CUDA on Linux/Windows and Metal/MPS on macOS through
  backend-owned contracts, and execution records the selected runtime variant,
  device class, and selected device id.
- Runtime registry/admission can present typed backend execution candidates
  with backend id, task/model compatibility, runtime variant, device facts,
  resource estimates where known, and bounded rejection diagnostics.
- Runtime registry technical-fit selection rejects candidates that exceed
  typed snapshot RAM/VRAM budgets or impossible active-claim arithmetic before
  selection, and reports typed diagnostics for the rejected runtime/candidate.
- Backend adapter and device/runtime implementation slices have a recorded
  standards gate covering crate role, public facade impact, lifecycle owner,
  persisted artifacts, feature/dependency impact, path/resource validation,
  frontend accessibility impact, and test isolation.
- The scheduler owns final backend/runtime/device candidate selection. Inference
  backend adapters do not rank candidates across backends or make queue,
  residency, fairness, or learned-throughput decisions.
- Explicit invalid workflow backend/runtime/device requests are rejected with
  visible diagnostics instead of fallback. Examples include diffusion through
  llama.cpp, MLX on Linux/Windows, and Candle image generation before executable
  Candle support exists.
- No executable fallback or legacy compatibility path remains for backend,
  runtime, device, pipeline-family, dependency-environment, or graph-shape
  selection. Old methods are deleted or replaced by canonical contracts.
- Auto device/backend selection is a canonical scheduler policy. It records the
  selected backend/runtime variant/device when successful and fails with typed
  diagnostics when no valid candidate exists.
- Runtime technical-fit cannot select `ConservativeFallback` or synthesized
  override-fallback candidates as executable decisions.
- llama.cpp managed runtime state can represent multiple runtime variants for
  the same release version without creating another binary-management system.
- Explicit unavailable devices block execution with typed diagnostics instead
  of falling back to CPU, another GPU, or another runtime variant.
- Image generation uses a deterministic execution plan and does not silently
  fall back when plan construction, dependency validation, option validation,
  device validation, or pipeline-family validation fails.
- Generated image retention stores one media body and exposes artifact
  descriptors or metadata for all other projections.
- Candle is not selected for image generation until executable Candle diffusion
  support exists.
- New or changed public/cross-layer contracts use structured DTOs, explicit
  serde casing, typed errors, and round-trip tests where applicable.
- New public or cross-crate Rust types expose appropriate `Debug`,
  `#[non_exhaustive]`, and `#[must_use]` semantics where the standards require
  them.
- Runtime roots, executable paths, dynamic-library paths, Pumas package paths,
  artifact paths, and worker-visible paths are validated against allowed roots
  before filesystem, subprocess, or worker use.
- Dimensions, token/context limits, batch sizes, byte ranges, memory estimates,
  and output-size calculations use checked arithmetic and fail with typed
  diagnostics at boundaries.
- Async/resource changes have explicit lifecycle ownership, cancellation or
  shutdown behavior, and no unbounded queues or polling loops.
- Any touched local backend service binds to loopback only, enforces bounded
  connection/request limits, reports readiness timeout failures, and shuts down
  through the lifecycle owner.
- Lifecycle and ledger projections include selected backend, selected runtime
  variant, selected device class/id, execution duration, terminal status, and
  retained artifact output-size facts needed by future scheduler learning.
- Frontend changes render backend-owned facts declaratively and do not introduce
  optimistic updates for graph, diagnostic, runtime, model, or artifact data.
- Frontend runtime/device controls and diagnostics actions have accessible
  names, keyboard interaction where interactive, resilient tests, and
  deterministic subscription or scoped-poll cleanup.
- Runtime feature/dependency changes preserve documented default,
  no-default-features, and all-features build behavior for affected public
  crates.
- Decomposition review is complete for every touched file or component that
  exceeds standards thresholds.
- Reference-repo review notes identify the Transformers naming/config
  conventions and ComfyUI/InvokeAI diffusion family patterns used by the
  implementation.
- Image-generation planner contracts define family taxonomy, component roles,
  family requirements, adapter responsibilities, execution plan shape, and
  missing-facts diagnostics before backend execution is implemented.
- Family requirements are table-driven for SD/SDXL, FLUX, FLUX.2, Qwen Image,
  Lumina Image, GLM Image, and Z-Image, with unsupported or under-specified
  families rejected explicitly.
- Missing-facts diagnostics identify the exact Pumas fields needed for a family
  instead of returning a generic unsupported-model message.
- Gateway image option diagnostics do not override or duplicate family planner
  option support decisions.
- Node-engine image-generation outputs do not carry duplicate base64 bodies in
  both `image` and `results` before artifact conversion.
- Touched READMEs describe the current graph and ownership boundaries.
