# Image Generation Family Planner Design

## Planner Contracts

The graph surface stays simple: `puma-lib` resolves a model and
`llm-inference` requests `task_kind = image_generation`. Diffusion-family
complexity belongs inside the inference crate's synchronous planner and
PyTorch/diffusers bridge.

The planner design should use these Pantograph-owned Rust contracts:

| Contract | Responsibility |
| -------- | -------------- |
| `ImageGenerationFamily` | Stable enum for family-level routing: `StableDiffusion`, `StableDiffusionXl`, `Flux`, `Flux2`, `QwenImage`, `LuminaImage`, `GlmImage`, `ZImage`, and `Unsupported`. |
| `ImageGenerationFamilyVariant` | Optional typed refinement such as SDXL refiner, Flux schnell/dev, Flux2 Klein, Z-Image pixel-space, or future family variants. |
| `ImageGenerationComponentRole` | Stable role labels for package components: pipeline index, scheduler config, transformer, UNet, VAE, tokenizer, text encoder, secondary text encoder, image processor, processor, generation config, and weights. |
| `ImageGenerationFamilyRequirements` | Required/optional component roles, accepted artifact kinds, accepted pipeline classes, accepted dtypes, dimension constraints, option support, and default scheduler policy for one family. |
| `ImageGenerationFamilyAdapter` | Pure validation/build adapter that consumes `ResolvedModelPackageFacts`, `ImageGenerationRequest`, dependency/device facts, and backend capabilities, then returns one `PyTorchDiffusersImageGenerationPlan` or typed diagnostics. |
| `PyTorchDiffusersImageGenerationPlan` | Fully validated backend-local plan containing the Pumas model ref, resolved source, package root, family, variant, pipeline class, scheduler choice, dtype, device policy, generation args, dependency environment, and expected artifact projection. |
| `ImageGenerationPlanDiagnostic` | Bounded typed error for missing facts, unsupported family, unsupported component layout, unsupported option, incompatible scheduler, invalid dimensions, unavailable dependency environment, unavailable device, unacceptable resource estimate, custom code denied, and unsafe path. |

`ImageGenerationFamilyAdapter` is not an async trait. Pumas lookup, runtime
readiness, device discovery, and worker execution happen before or after the
planner in existing async shells. Family adapters must not inspect frontend
state, scheduler queues, session residency, warm worker state, or retained
artifact bodies.

Minimum Pumas facts needed for deterministic planning:

| Fact | Required Use |
| ---- | ------------ |
| `artifact.artifact_kind` and `artifact.entry_path` | Confirm `diffusers_bundle` and resolve the package root handed to the worker. |
| `artifact.selected_files` / `artifact.sibling_files` | Verify required files exist without scanning outside the approved package root. |
| `components[].kind`, `components[].status`, `components[].relative_path`, `components[].class_name` | Map package files to `ImageGenerationComponentRole` and validate required family components. |
| `transformers.config_model_type`, `transformers.architectures`, `transformers.auto_map`, `transformers.torch_dtype`, `transformers.generation_config_status` | Use Transformers-compatible naming/config evidence without relying on model display names. |
| `task.pipeline_tag`, `task.task_type_primary`, `task.input_modalities`, `task.output_modalities` | Confirm text-to-image/image-generation task semantics. |
| `generation_defaults.defaults` | Merge model defaults into request planning without treating them as user overrides. |
| `custom_code.requires_custom_code` | Reject unless a future explicit trust policy allows custom code. |
| `backend_hints.accepted` | Confirm `diffusers` package capability while still resolving execution to PyTorch. |

The Pumas-side producer work for these facts is tracked in
[Pumas Library Image Generation Facts](07-pumas-library-image-generation-facts.md).
Pantograph should consume those facts through package-facts DTOs and must not
replace missing Pumas facts with name-derived family guessing.

If any required fact is absent or ambiguous, the planner returns a
missing-facts diagnostic. It must not infer family from the saved workflow
name, display name, directory name, or file name.

Initial family requirements should be table-driven:

| Family | Reference Finding | Initial Pantograph Requirement |
| ------ | ----------------- | ------------------------------ |
| `StableDiffusion` / `StableDiffusionXl` | InvokeAI maps SD/SDXL variants to explicit diffusers pipeline classes; ComfyUI distinguishes SDXL latent format, CLIP targets, and sampling settings. | Require model index/config evidence, scheduler config, VAE, tokenizer/text encoder components, compatible SD/SDXL pipeline class, supported dimensions, and explicit variant when refiner/inpaint behavior appears. |
| `Flux` | ComfyUI treats FLUX as a flow/transformer family with Flux latent format, guidance-embed differences, T5/CLIP text handling, and bfloat16 preference; InvokeAI separates FLUX loaders from SD loaders. | Require transformer, VAE, tokenizer/text encoder roles, Flux-compatible pipeline class, dtype/device validation, guidance-scale support validation, and no SDXL pipeline fallback. |
| `Flux2` | ComfyUI and InvokeAI treat FLUX.2 as distinct from FLUX, with 32-channel VAE and Qwen/Mistral-style encoder variants. | Require Flux2 family evidence, transformer role, Flux2 VAE role, Qwen3 or supported encoder evidence, variant detection from facts, and Flux2-specific dtype/dimension validation. |
| `QwenImage` | ComfyUI exposes qwen-image as its own family using Qwen image tokenizer/text encoder patterns and Wan-style latent format. | Require QwenImage family evidence, transformer/vae/tokenizer/text-encoder roles, qwen-image pipeline class or config evidence, and option validation for guidance/negative prompt semantics. |
| `LuminaImage` | ComfyUI's Lumina2 family uses a Gemma-style text encoder and Flux-like latent format. | Require Lumina family evidence, compatible text encoder role, VAE, transformer, scheduler settings, and dtype/device support. |
| `GlmImage` | Treat as an explicitly named family because it is expected in Pumas, even if local references are less complete than FLUX/Z-Image. | Require Pumas-provided family/pipeline evidence before support is enabled; otherwise return unsupported-family rather than guessing from directory names. |
| `ZImage` | InvokeAI shows Z-Image needs transformer, VAE, and Qwen3 encoder sources; ComfyUI models Z-Image as distinct text encoder and latent behavior, including pixel-space variant. | Require Z-Image family evidence, transformer, VAE unless pixel-space variant is explicit, Qwen3 tokenizer/text encoder evidence, variant validation, and explicit rejection when component source is ambiguous. |

Reference repo findings used by this design:

- Transformers keeps generation semantics in a structured generation config and
  uses `model_type`, `architectures`, `auto_map`, and `trust_remote_code`
  evidence as model-definition signals. Pantograph should use equivalent facts
  from Pumas but keep Rust-owned planner contracts.
- ComfyUI's README lists SD1.x/SD2.x, SDXL/Turbo, FLUX, Lumina Image 2.0,
  Qwen Image, FLUX.2, and Z-Image as distinct image-model families rather than
  one generic Diffusers path. `comfy/model_detection.py` then identifies
  family-specific transformer evidence such as FLUX versus FLUX.2 keys, Lumina
  2 `cap_embedder`/`noise_refiner` keys, and Qwen Image `txt_norm` keys.
  Pantograph should use these as reference evidence for Pumas producer facts,
  not as runtime state-dict scanning inside Pantograph.
- ComfyUI's text-encoder loader shows that family support depends on concrete
  encoder/tokenizer pairings: Lumina uses Gemma-family encoders, Qwen Image
  uses Qwen Image encoder/tokenizer paths, FLUX uses T5/CLIP-style handling,
  FLUX.2 Klein uses Qwen3 or Mistral-style encoder variants, and Z-Image can
  use Qwen3 encoder/tokenizer facts. Pumas facts should expose those component
  roles explicitly so Pantograph can validate them without directory-name or
  model-id guessing.
- ComfyUI's sampler registry treats scheduler names as a bounded set such as
  `simple`, `sgm_uniform`, `karras`, `exponential`, `ddim_uniform`, `beta`,
  `normal`, `linear_quadratic`, and `kl_optimal`. Pantograph should keep
  denoising scheduler choices as validated primitive ids provided by backend
  port options, not display-label strings or graph-local defaults.
- InvokeAI's model-integration guide names `StableDiffusion1`,
  `StableDiffusion2`, `StableDiffusionXL`, `Flux`, `Flux2`, `SD3`, and
  `ZImage` as taxonomy values, and records Flux2 variants such as Klein4B,
  Klein9B, and Klein9BBase. It also calls out Qwen3 encoder implementations
  for FLUX.2 Klein and Z-Image and CLIP embeddings for SDXL/SD3. These are
  useful conventions for Pumas facts and Pantograph family variants, but they
  must not make Pantograph mirror InvokeAI's model manager, invocation graph,
  metadata, or UI architecture.
- InvokeAI's new-model guide shows that a new diffusion family requires
  taxonomy, config/fact validation, component loading, denoise invocation
  shaping, scheduler handling, metadata, and tests as separate concerns.
  Pantograph should preserve the same separation of concerns through Pumas
  facts, scheduler decisions, planner rules, worker envelopes, and artifact
  retention rather than a monolithic family loader.

## Concurrency And Lifecycle Review

- Image-generation planning is a synchronous domain operation. It must not
  spawn tasks or own long-lived resources.
- Pumas lookup, artifact reads/writes, and Python worker calls may be async or
  process-boundary operations only when those owners actually need I/O or
  awaitable runtime state. Do not make planner/admission functions async by
  default; pass reduced facts into synchronous planning cores.
- No new background task, timer, polling loop, or worker process lifecycle is
  introduced by this plan without adding an explicit owner, cancellation path,
  shutdown path, and deterministic cleanup tests.
- IO inspector saved-graph mode should use existing backend query/event
  patterns. It must not add a page-local polling loop to discover stale graph
  changes.
- Artifact conversion must not hold service locks across worker execution or
  long-running conversion work.
- If implementation reveals a need for new queues or event buffers, they must
  be bounded, own overflow behavior, and emit diagnostics when rejecting or
  dropping work.

## Memory Estimate Policy

Memory-fit planning is a scheduler concern that applies to all inference
families and runtimes, not only PyTorch image generation. The image-generation
planner may validate request-local numeric bounds, but it must not become the
owner of queue admission, runtime ranking, model residency, retries, or learned
placement.

The planned ownership boundary is:

- Pumas owns model-library facts such as artifact kind, selected artifact,
  component roles, package readiness, dtype/config evidence, storage kind, and
  validation state. Pumas may expose component sizes or model-residency facts
  when they are available, including `not_available` or `not_implemented`
  states. Pantograph must not synthesize missing Pumas facts from paths,
  display names, or Python worker discovery.
- The inference crate owns task/family request validation. It performs checked
  arithmetic for dimensions, image count, output byte estimates, and
  family-specific option constraints, then emits typed planner diagnostics for
  overflow, impossible values, unsupported families, or unavailable estimate
  inputs.
- Backend adapters and runtime-registry providers expose typed candidate facts:
  runtime readiness, device inventory, package/dependency readiness,
  backend/runtime capability, resource estimates when known, and bounded
  diagnostics. They do not rank candidates or decide whether a workflow should
  wait, retry, or terminate.
- The scheduler owns memory admission, candidate ranking, reservation policy,
  retry/reschedule/termination policy, and history weighting. It is the only
  component that may decide a candidate is runnable now based on current memory
  pressure, model residency, queue state, timing history, and OOM/failure
  history.
- Workers report observed load duration, warmup duration, execution duration,
  terminal status, output size, and observed memory/OOM facts back to lifecycle
  and diagnostics projections. Workers must not choose alternate runtimes or
  continue after scheduler-required memory proofs are missing.

Memory estimates should use typed states instead of sentinel values:

| State | Meaning |
| ----- | ------- |
| `available` | The estimate was computed from trusted facts with checked arithmetic. |
| `not_available` | The runtime or host cannot currently provide the fact, such as a missing managed binary or unprobed device. |
| `not_implemented` | The inference crate knows the family/runtime concept but does not yet implement that estimate. |
| `insufficient_facts` | Required Pumas, runtime, or request facts are missing or ambiguous. |
| `overflow` | Checked arithmetic failed while computing the estimate. |
| `unsupported_family` / `unsupported_runtime` | The family or runtime is intentionally non-executable for this estimate. |

Initial estimate kinds should be explicit and additive:

- `output_rgba_bytes` for conservative generated-image output sizing.
- `vae_working_memory_bytes` for image-family encode/decode pressure.
- `model_residency_bytes` for model/component residency when facts are known.
- `runtime_overhead_bytes` for runtime-specific fixed overhead when known.
- `peak_vram_bytes` and `peak_ram_bytes` for scheduler admission inputs.

Reference boundary: InvokeAI's VAE working-memory utility estimates
family-specific VAE pressure from dimensions, operation, element size, tile
size, latent scale factor, and family constants. Pantograph may use that as a
reference for the type of facts needed, but the implementation must remain a
Pantograph-owned typed estimator fed by Pumas/runtime facts rather than copying
InvokeAI invocation or model-manager architecture.

Staged implementation plan:

1. [ ] Replace the old technical-fit resource estimate contract with typed
   inference/runtime resource-estimate DTOs and diagnostics using the states
   above. This is a shared contract slice; it must not change scheduler
   ranking.
   - 2026-05-17: inference now exposes `InferenceResourceEstimate`,
     `InferenceResourceEstimateState`, estimate kinds, and typed estimate
     diagnostics. The contract represents unavailable/overflow/unsupported
     estimates as states with diagnostics rather than numeric sentinel values.
     The follow-up output RGBA migration tightened non-available construction
     so callers cannot build an unavailable estimate with the available state.
     Runtime-registry projection remains before this stage is complete;
     scheduler ranking remains later staged work.
   - 2026-05-17 replan decision: use replacement option 3. Runtime-registry,
     workflow-service, and embedded-runtime technical-fit contracts must move
     to typed estimate records and remove the old singular
     `resource_estimate` shape with optional MB fields and
     `estimation_confidence` string control flow. Do not bridge the new
     contract into old fields as a compatibility path and do not keep both
     shapes as competing sources of truth.
   - 2026-05-18 runtime-registry contract slice: runtime-registry technical-fit
     candidates and decisions now carry `resource_estimates` as typed estimate
     records with explicit states and diagnostics. The old singular
     `resource_estimate` optional MB-field contract was removed from this
     boundary. Workflow-service and embedded-runtime mirrors remain the next
     serial projection slices before this shared-contract stage is complete.
   - 2026-05-18 workflow/embedded projection slice: workflow-service now
     mirrors typed technical-fit estimate records and embedded-runtime projects
     runtime-registry estimates without reintroducing optional MB fields.
     Runtime-requirement peak RAM/VRAM estimates are converted to byte-valued
     typed records with checked arithmetic; overflow emits a typed estimate
     diagnostic instead of being saturated, omitted, or converted through a
     confidence string. Scheduler admission and history-backed ranking remain
     later staged work.
2. [x] Move existing output-size checked arithmetic into the shared estimate shape
   and add tests proving overflow is diagnostic-backed.
   - 2026-05-17: `ImageGenerationExecutionPlan` now carries typed
     `resource_estimates` instead of the legacy
     `estimated_output_rgba_bytes: Option<u64>` field. Output RGBA estimates
     use `available` with byte values when dimensions/count are known,
     `insufficient_facts` when width or height is omitted, and `overflow` with
     a planner rejection diagnostic when checked arithmetic fails.
3. [ ] Add side-effect-free family/runtime calculators for estimates that can be
   computed from already-available facts. Unknown or unimplemented estimates
   must be explicit states, not `0`, silent omission, or saturated values.
4. [ ] Project reduced estimate facts into scheduler-facing
   `BackendExecutionCandidate` data. Do not pass full Pumas package facts or
   worker envelopes through the scheduler.
5. [ ] Make scheduler admission consume memory estimates and current resource
   pressure before selection. Explicit runtime/device requirements must fail
   with diagnostics when they cannot fit; omitted requirements let the
   scheduler choose among valid candidates.
6. [ ] Persist observed timing and memory/OOM facts. History-backed memory and
   timing ranking starts only after every valid runtime candidate for the same
   workflow/model/runtime key has at least five completed runs; before that,
   policy uses current facts and controlled exploration.

## Standards Guardrails

- No production `unwrap()` or `expect()` in new request, lifecycle, worker,
  planner, artifact, or IPC paths. Use typed errors with context.
- No `Result<T, String>` for public or cross-crate APIs introduced by this
  plan.
- Public or cross-layer enums/DTOs should use explicit serde casing and be
  tested with round trips against frontend TypeScript types where applicable.
- Cross-language and cross-process envelopes, including Tauri IPC DTOs and
  Python worker messages, must be versioned or shape-checked at their boundary
  and updated on both sides in the same implementation slice.
- Planner and diagnostic enums should be non-exhaustive or otherwise designed
  for additive extension where future model families are expected.
- Constants for backend keys, task kinds, dependency labels, family names,
  component roles, scheduler names, diagnostic codes, and artifact field names
  must be centralized in typed Rust enums/newtypes or shared constants before
  they cross crate or language boundaries.
- New frontend tests should use resilient accessible selectors and include
  keyboard/selection coverage when stale-node markers or graph controls become
  interactive.
- Interactive graph and IO inspector controls must use semantic controls where
  possible, accessible names, focus-visible states, and keyboard activation for
  stale-node selection, node details, artifact actions, and navigation to
  settings.
- Existing large modules touched by the plan must get decomposition review
  before more code is added, especially `pytorch.rs`,
  `artifact_output_conversion.rs`, `IoInspectorPage.svelte`, and graph
  canonicalization modules.
- Module README updates are required when implementation changes ownership,
  structured producer contracts, host-facing API contracts, runtime boundaries,
  or directory contents.
- New source directories under crates, frontend services, graph presenters, or
  worker bridge modules require README updates that state ownership,
  dependencies, public contracts, and unsupported behavior.

## Standards Compliance Matrix

| Standard Area | Required Plan Constraint |
| ------------- | ------------------------ |
| Plan standards | Each implementation slice starts with a narrow acceptance test or fixture, records verification in this plan, and commits only after the slice is validated. |
| Architecture patterns | Backend crates own graph validity, stale diagnostics, model/package facts, runtime readiness, and execution planning. Frontend and Tauri remain adapters/presentation. |
| Rust API standards | Parse raw inputs once into validated Rust types; public/cross-crate APIs expose structured errors and enums/newtypes instead of raw strings. |
| Rust async/concurrency | Planning and graph transformation stay synchronous; async shells own Pumas I/O, artifact I/O, IPC, and worker calls with no locks held across awaits or blocking work. |
| Rust/security standards | Model paths, artifact roots, dimensions, counts, resource estimates, and byte ranges are validated with checked arithmetic and allowed-root checks before worker or artifact access. |
| Runtime adapter standards | Backend adapters expose facts and execution translation only; composition-root lifecycle owners manage subprocesses, local services, worker pools, bounded queues, readiness timeouts, and shutdown. |
| Dependency/cross-platform standards | Runtime/device support remains capability-driven and platform-isolated; optional CUDA, Metal/MPS, vLLM, Candle, MLX, and Python dependencies preserve documented feature contracts and affected feature-mode checks. |
| Interop standards | Tauri IPC, frontend DTOs, and Python worker envelopes are updated together and covered by serde/envelope tests. Worker process lifecycle remains owned by the existing backend lifecycle manager. |
| Runtime/device standards | Device policy, runtime variant, selected device id, and device diagnostics are backend-owned typed facts. Explicit device requests fail when unavailable; auto records the selected runtime variant and device. |
| No-fallback/no-legacy standards | Old graph, backend, runtime, device, technical-fit, and worker execution methods are removed or replaced by canonical contracts. Planning failure returns typed diagnostics and never invokes fallback behavior. |
| Frontend/accessibility | IO inspector graph selection and artifact actions are declarative, keyboard accessible, named for assistive technology, and cleaned up on unmount if subscriptions are added. |
| Testing standards | Cross-layer behavior gets vertical-slice tests, durable/global state uses isolated temp roots, and saved/run graph projection replay is verified. |
| Documentation standards | Touched module READMEs and this plan directory document ownership, structured producer contracts, host-facing contracts, runtime boundaries, unsupported old graph shapes, and README coverage for multi-file documentation directories. |
| File-size/decomposition | Existing threshold-crossing files get extraction tasks or an explicit reason before edits; new code lands in focused modules rather than expanding large facades. |

## Affected Persisted Artifacts

- `.pantograph/workflows/Juggernaut X v10 SDXL.json`
- `.pantograph/workflows/juggernaut-x-v10-sdxl.json`
- Built-in workflow templates under `src/templates/workflows`
- Workflow metadata/listing records derived from saved workflow files
- Run snapshots that include stale graph diagnostics after implementation
- Retained IO artifacts for generated images
