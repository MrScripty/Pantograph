# Objective And Scope

## Objective

Make image-generation workflows use Pantograph's current canonical inference
system instead of retired direct diffusion nodes. Fix the Juggernaut X v10 SDXL
workflow by replacing stale graph structure with current graph structure,
deleting the duplicate saved workflow, and exposing stale graph diagnostics so
invalid workflow graphs are visible and understandable in the IO inspector.

Add a validated PyTorch/diffusers image-generation execution slice for
diffusers-directory image models from Pumas. Tiny SD Turbo is only the bounded
smoke fixture and Juggernaut X v10 SDXL is only the first repaired saved
workflow. The implementation must support the same current graph and execution
contract for other Pumas diffusion/image-generation models such as z-image
turbo, qwen-image, lumina-image, glm-image, and FLUX.2 when their package facts
resolve to a supported PyTorch/diffusers execution path. Candle diffusion
remains future work until Pantograph has executable Candle model loading for
image generation.

Add a backend-owned device and runtime-variant selection contract before
execution work expands. Pantograph must let users choose a target device class
or concrete device for inference. The first supported targets are CPU and
NVIDIA CUDA on Linux/Windows, and Metal/MPS on macOS. The selected device
must resolve through backend-specific capability facts and runtime variants,
not through frontend-only settings or hidden backend fallback. llama.cpp is the
first backend that requires multiple managed runtime variants for one release
version because CPU, CUDA, and macOS Metal builds may require different
binaries or dynamic backend libraries.

Define every executable inference backend behind an adapter boundary:
llama.cpp, PyTorch/Transformers, vLLM, Candle, and future MLX. Adapters expose
task/model/device/runtime facts, feasibility diagnostics, resource estimates
where known, and backend-specific execution translation. The scheduler owns
backend selection, device placement, RAM/VRAM reservations, queue/resource
policy, explicit client preference validation, and later learned throughput
policy from ledger and artifact facts. The inference crate must execute the
scheduler-selected backend/runtime/device decision without becoming the
scheduler.

Before implementation, freeze the execution and inspection contracts that keep
the resulting code simple:

- `diffusers` is a model/package capability and dependency family under the
  PyTorch execution backend until Pantograph has a separately registered
  Diffusers backend facade.
- Retired graph nodes are stale graph facts, not silently rewritten current
  graphs.
- Current graph normalization and retired-node classification are separate
  code paths. Current app load/save/session paths may keep useful repairs for
  current graphs, but must not migrate retired node types.
- Pantograph does not provide legacy or backwards support for old graph shapes.
  Breaking changes should remove stale producers, tests, fixtures, and saved
  workflow shapes instead of preserving compatibility paths.
- Pantograph does not provide legacy or fallback support for old backend,
  runtime, device, technical-fit, or worker execution methods. Those methods
  are removed or replaced by canonical contracts when touched.
- IO inspector can inspect stale saved graphs without requiring a successful
  workflow run.
- Generated media has one retained artifact body. Other node outputs expose
  descriptors and metadata rather than duplicate base64 payloads.
- The image-generation path is model-family agnostic. Model-specific behavior
  comes from Pumas package facts, task options, dependency facts, scheduler
  metadata, and backend readiness, not hardcoded saved-workflow names.
- Image-generation execution is deterministic. Pantograph selects exactly one
  execution plan, validates it before execution, and reports a diagnostic if
  that plan is invalid. It must not silently fall back between backends,
  pipeline families, schedulers, devices, dependency environments, or alternate
  model interpretations.
- Device selection is deterministic. Pantograph records the requested device
  policy, the resolved backend runtime variant, and the selected device id. If
  the requested device cannot run the selected model/task/backend, execution is
  blocked with a diagnostic rather than falling back to CPU or another device.
- Backend selection is scheduler-owned. Workflows may express backend,
  runtime, device, latency, or throughput preferences, but those preferences
  are intent only. Invalid explicit preferences fail with diagnostics rather
  than being silently replaced.
- Scheduler algorithms are intentionally changeable policy. Scheduler ranking,
  exploration, history weighting, retry/reschedule policy, and future learned
  placement logic must live behind stable, versioned scheduler input and
  decision contracts. Workflow graphs, inference nodes, runtime loading,
  Pumas fact ownership, diagnostics storage, frontend DTOs, and backend
  adapters must not need algorithm-specific changes when the scheduler policy
  is revised.
- Transformers ecosystem naming and generation-option conventions guide
  Pantograph's canonical task/model/request semantics until backend-specific
  adapter translation is required.

## Scope

### In Scope

- Delete the duplicate Juggernaut X v10 SDXL saved workflow and keep one
  current workflow definition.
- Update the retained Juggernaut graph to use `puma-lib` plus canonical
  `llm-inference` with `task_kind = image_generation`.
- Use stable Pumas model identity and resolved package facts instead of stale
  raw model paths.
- Keep the canonical image-generation graph and execution path reusable for
  additional diffusion/image models, including z-image turbo, qwen-image,
  lumina-image, glm-image, and FLUX.2.
- Add a deterministic image-generation execution planner that derives one
  PyTorch/diffusers execution plan from Pumas facts, graph inputs, backend
  capabilities, dependency readiness, and device policy.
- Add a deterministic device and runtime-variant planning layer used by
  llama.cpp, PyTorch/diffusers, vLLM, Candle, and future MLX integration.
- Add a common backend-adapter capability contract for llama.cpp,
  PyTorch/Transformers, vLLM, Candle, and future MLX so the scheduler can
  compare typed backend/runtime/device candidates without reading backend
  command strings.
- Add scheduler-facing candidate facts for backend id, task/model
  compatibility, runtime variant, device class/id, resource estimates where
  known, optional observed-throughput hints, and bounded rejection diagnostics.
- Record selected backend/runtime/device and output-size facts in lifecycle and
  ledger projections so future scheduler policy can learn from actual
  throughput without inspecting artifact bodies.
- Extend managed runtime planning so one backend release can expose multiple
  runtime variants, starting with llama.cpp variants for CPU, CUDA, and macOS
  Metal builds where available.
- Add backend-owned device capability inventory and readiness diagnostics for
  CPU and CUDA on Linux/Windows, and Metal/MPS on macOS.
- Add model-family adapter seams inside the PyTorch/diffusers bridge for
  pipeline families such as SD/SDXL, FLUX, qwen-image, lumina-image,
  glm-image, and z-image, selected by package facts rather than model names.
- Rename image-generation sampling-scheduler semantics away from the overloaded
  `scheduler` term. The canonical graph/API field is `denoising_scheduler`,
  meaning the Diffusers denoising/sampling scheduler, not Pantograph workflow
  scheduling. Omitted `denoising_scheduler` means the selected model/pipeline
  default is used by explicit policy; provided values must be validated and
  executed end to end.
- Use backend-owned port options for selectable inference traits whose valid
  values depend on model facts, package facts, runtime capabilities, or backend
  readiness. Denoising scheduler selection is the first image-generation use
  case, but the same mechanism may serve other selectable traits such as dtype,
  adapter selection, tokenizer/chat-template variants, pooling strategies, or
  audio voices when they are user-facing and fact-dependent.
- Keep generic long-tail model/runtime parameters behind schema-driven
  `expand-settings`; promote only important, frequently used,
  diagnostics-relevant options to first-class graph ports and backend-owned
  option providers.
- Use the local reference repos as implementation guidance without copying
  their architecture:
  - Transformers for task/config naming conventions and generation option
    semantics.
  - ComfyUI for explicit diffusion model-family detection, component
    requirements, and supported-model taxonomy.
  - InvokeAI for new-model-family integration boundaries, model loader
    separation, and family-specific validation examples such as FLUX, FLUX.2,
    SDXL, and Z-Image.
- Fix current graph/template/probe code that still emits or suggests
  `diffusion-inference`.
- Add backend-owned stale graph diagnostics for unknown node types, retired
  node types, missing node definitions, missing edge endpoints, missing edge
  handles, and unresolved Puma model references.
- Reuse existing workflow graph validation and effective-definition logic when
  producing stale graph diagnostics.
- Show stale graph facts in the IO inspector graph view and mark stale nodes
  explicitly.
- Add an IO inspector saved-graph inspection mode for stale graphs that cannot
  be submitted and therefore do not have a workflow run snapshot.
- Keep submit/admission blocked for stale executable graphs with a visible
  backend-owned reason.
- Validate image generation through the existing typed
  `InferenceExecutionRequest` / `ImageGenerationRequest` contract.
- Validate requested device policy and runtime variant through the same typed
  execution planning path before model loading or worker execution.
- Confirm PyTorch/diffusers is the first executable backend for diffusion
  models.
- Confirm backend/device support facts for PyTorch, llama.cpp, vLLM, Candle,
  and MLX roadmap status using backend probes and primary documentation rather
  than assumptions.
- Normalize `diffusers` graph and Pumas hints to PyTorch execution for this
  slice while preserving `diffusers` as the dependency/capability label.
- Implement the Rust PyTorch backend image-generation bridge to the existing
  Python worker diffusion load/generate commands.
- Factor the PyTorch image-generation bridge so `pytorch.rs` remains a facade
  over focused worker request/response helpers instead of absorbing another
  large inline implementation.
- Add the image-family planner as a focused inference module and integrate it
  into existing gateway/backend/node-engine paths instead of layering another
  independent routing system over them.
- Centralize image-generation execution backend normalization in one
  task/artifact-aware function so `diffusers` package/runtime hints resolve to
  PyTorch execution only for the appropriate image-generation path.
- Verify generated image outputs are retained once and projected as artifact
  descriptors everywhere large media would otherwise be duplicated.
- Remove duplicate image bodies at the node-engine/result projection boundary
  before artifact conversion, not only inside artifact conversion.
- Update touched READMEs when ownership boundaries or graph/template standards
  change.

### Out Of Scope

- Backwards compatibility with retired Pantograph workflow graph shapes.
- Generic saved-workflow migration for old `diffusion-inference` graphs.
- Silent load/save rewriting of retired node types into current graph shapes.
- Mixing current graph normalization with legacy graph migration in a single
  current app-path canonicalization function.
- Preserving old Pantograph graph shapes through migration helpers, hidden
  compatibility shims, or load/save rewrites.
- Reintroducing `diffusion-inference` as a graph-visible node.
- Creating a separate registered Diffusers backend facade in this plan.
- Making Tauri the source of truth for stale graph, model resolution, or
  inference execution semantics.
- Implementing Candle diffusion execution in this plan.
- Silent fallback between execution backends, model families, pipeline classes,
  schedulers, devices, runtime variants, dependency environments, or alternate
  model interpretations.
- Preserving raw-device defaults, executable technical-fit fallback, synthetic
  frontend device options, or worker-side generic auto-selection as backup
  paths.
- Implementing image-to-image, inpainting, ControlNet, LoRA composition, or
  multi-image generation beyond preserving extension points in the request
  contract.
- Hardcoding model-specific routing for Tiny SD Turbo, Juggernaut, z-image,
  qwen-image, lumina-image, glm-image, FLUX.2, or any other diffusion family.
- Changing Pumas into a Pantograph-specific scheduler or backend-policy
  provider.
- Letting frontend, Tauri, or saved workflow JSON be the source of truth for
  available devices or runtime-variant readiness.
- Implementing every backend/device combination in one slice. The plan should
  define the contract first, then enable backend-specific devices only when
  probes and tests prove support.
- Implementing ROCm/HIP, Vulkan, XPU/iGPU, OpenVINO, remote hardware plugins,
  or cross-backend hybrid/offload in this slice.
- Implementing the learned scheduler optimizer, full queue fairness policy, or
  residency manager in this plan. This plan records the facts those systems
  need and keeps policy ownership in the scheduler.
- Letting inference backend adapters rank candidates across backends or choose
  between workflows. Adapters provide feasibility, estimates, diagnostics, and
  execution translation only.
