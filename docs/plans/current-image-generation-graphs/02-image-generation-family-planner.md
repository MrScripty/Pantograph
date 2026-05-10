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
- ComfyUI's `supported_models.py` and `model_detection.py` show that diffusion
  support is family-specific, with distinct latent formats, text encoders,
  dtype support, memory factors, and sampling defaults.
- InvokeAI's `NEW_MODEL_INTEGRATION.md` and model loaders show that adding a
  family should touch taxonomy, config/fact validation, model loading,
  invocation/request shaping, sampling behavior, and metadata in a deliberate
  sequence.

## Concurrency And Lifecycle Review

- Image-generation planning is a synchronous domain operation. It must not
  spawn tasks or own long-lived resources.
- Pumas lookup, artifact reads/writes, and Python worker calls remain async or
  process-boundary operations owned by existing service/runtime owners.
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
