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
   - 2026-05-18 scheduler pressure contract slice: runtime-registry
     `RuntimeTechnicalFitResourcePressure` now represents only current queue
     and loaded-runtime pressure. Candidate budget-pressure ranking is
     activated from typed candidate `peak_vram_bytes`/`peak_ram_bytes`
     estimates plus current pressure, so the old pressure-level peak MB
     estimate fields are no longer accepted as a second source of truth.
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
   - 2026-05-18: budget-pressure selection now consumes typed candidate
     `peak_vram_bytes`/`peak_ram_bytes` estimate presence for ranking
     activation. Full admission/reservation diagnostics remain in the next
     memory-policy slice.
   - 2026-05-18: runtime-registry technical-fit candidates now project typed
     `peak_ram_bytes`/`peak_vram_bytes` estimates into pure selector admission
     checks. The projection stays reduced to estimate records and runtime
     snapshot facts; it does not pass Pumas package facts or worker envelopes
     through scheduler-facing data.
5. [ ] Make scheduler admission consume memory estimates and current resource
   pressure before selection. Explicit runtime/device requirements must fail
   with diagnostics when they cannot fit; omitted requirements let the
   scheduler choose among valid candidates.
   - 2026-05-18 re-plan boundary: the next code slice cannot safely make
     admission consume typed estimates while `RuntimeReservationRequirements`,
     `RuntimeAdmissionBudget`, and admission failure payloads still use
     separate MB-shaped fields and runtime snapshots omit budget/claim facts.
     The accepted path is option 3: replace the reservation/admission contract
     with typed byte-valued estimate/claim facts, expose only reduced typed
     budget and active claim facts in runtime snapshots, then make the
     technical-fit selector reject over-budget candidates before selection.
     Do not bridge typed estimates into the old MB reservation fields, keep
     both contracts active, or call registry admission from the selector as an
     implicit side-effect.
   - Option 3 staged implementation:
     1. Replace runtime-registry reservation requirements and admission
        failures with typed byte-valued estimate/claim contracts and focused
        admission tests.
        - 2026-05-18: completed for runtime-registry. Admission budgets now
          carry typed resource budget rows, reservation requirements carry
          typed byte-valued resource claims, admission failures report byte
          fields, and reservation accounting/underflow errors use byte labels.
     2. Project embedded-runtime workflow requirements into that typed
        reservation contract with checked MiB-to-byte conversion until the
        upstream workflow requirement contract is replaced.
        - 2026-05-18: completed the embedded-runtime projection shim for the
          current workflow requirement contract. It converts the existing
          workflow MiB estimates into typed runtime-registry byte claims with
          checked arithmetic and returns `WorkflowServiceError::Internal` on
          conversion overflow rather than saturating or silently omitting the
          claim.
     3. Extend runtime snapshots with reduced typed admission budget and active
        claim facts so selection policy remains pure and testable.
        - 2026-05-18: completed for runtime-registry snapshots. Runtime
          snapshots now expose typed admission budget rows and per-active
          reservation byte claims without workflow, Pumas, or worker envelope
          details. Technical-fit selection can consume these immutable facts in
          the next slice without calling mutable registry admission.
     4. Filter/diagnose technical-fit candidates against typed peak memory
        estimates, current reservations, safety margins, and explicit
        runtime/device requirements.
        - 2026-05-18: completed in runtime-registry. The pure
          technical-fit selector rejects over-budget candidates before
          selection using snapshot admission budgets, active reservation byte
          claims, safety margins, and candidate peak RAM/VRAM estimates.
          Explicit overrides surface typed resource budget/accounting
          diagnostics instead of falling through to synthetic fallback
          diagnostics, and selector code does not call mutable registry
          admission or translate estimates back to MB fields.
     5. Replace the upstream workflow runtime-requirement MB fields with the
        shared typed estimate contract in a later serial contract slice.
        - 2026-05-18: completed the workflow-service/embedded-runtime contract
          replacement. `WorkflowRuntimeRequirements` now carries
          `resource_estimates` typed records and no longer exposes
          `estimated_*_mb` fields or `estimation_confidence` string control
          flow. Default capability memory estimates publish available
          `peak_ram_bytes`/`peak_vram_bytes` only when complete model-size
          facts exist; missing or partial facts publish typed
          `insufficient_facts` estimates. Embedded-runtime now projects typed
          workflow estimates into runtime-registry estimates and reservation
          byte claims directly, with no MiB conversion shim.
        - Verification: `cargo test -p pantograph-workflow-service
          capabilities::tests::memory_estimate --lib`, `cargo test -p
          pantograph-workflow-service
          workflow_technical_fit_resource_estimates_use_typed_states_without_legacy_mb_fields
          --lib`, `cargo test -p pantograph-workflow-service --test
          contract`, `cargo test -p pantograph-embedded-runtime host_helper
          --lib`, `cargo test -p pantograph-embedded-runtime
          runtime_requirements_resource_estimates --lib`, `cargo check -p
          pantograph-embedded-runtime`, `cargo fmt --all -- --check`, and
          `git diff --check`.
        - 2026-05-18 follow-up cleanup: resolved the stale
          backend-extraction fixture. `text-input` no longer carries incidental
          `backend_key` metadata in the default-capabilities fixture, and the
          test now asserts no hard backend requirement unless the canonical
          typed runtime input path supplies one. Verification: `cargo test -p
          pantograph-workflow-service workflow_capabilities --lib`,
          `cargo fmt --all -- --check`, and `git diff --check`.
6. [ ] Persist observed timing and memory/OOM facts. History-backed memory and
   timing ranking starts only after every valid runtime candidate for the same
   workflow/model/runtime key has at least five completed runs; before that,
   policy uses current facts and controlled exploration.
   - 2026-05-18 diagnostics-ledger history persistence slice:
     run-terminal events now accept typed `resource_observation` facts for
     peak RAM bytes, peak VRAM bytes, and explicit `out_of_memory` failures.
     Run-list/run-detail projections persist those fields as typed scheduler
     history facts, and runtime-selection history summaries now expose memory
     samples, typical ranges, and OOM counts beside duration/queue history.
     This does not activate history-backed scheduler ranking yet and does not
     infer OOM from free-form terminal error text.
   - Verification: `cargo test -p pantograph-diagnostics-ledger
     runtime_selection_history --lib`, `cargo test -p
     pantograph-diagnostics-ledger
     existing_v24_schema_adds_scheduler_learning_output_and_memory_projection_columns
     --lib`, `cargo check -p pantograph-workflow-service`, and `cargo test -p
     pantograph-diagnostics-ledger --lib`, `cargo test -p
     pantograph-workflow-service diagnostics --lib`, `cargo fmt --all --
     --check`, and `git diff --check`.
   - 2026-05-18 producer-wiring re-plan boundary: the next implementation
     cannot populate real `resource_observation` facts by parsing terminal
     error text or by repurposing artifact cache `max_memory_bytes` policy,
     because neither is typed runtime execution telemetry. Current code has
     duration facts and some OOM string detection in inference/runtime support,
     but no canonical byte-valued execution-resource observation contract that
     flows from inference runtime results through node-engine/embedded-runtime
     into workflow terminal events.
   - Re-plan options:
     1. Reject: infer OOM from existing free-form error messages at the
        workflow terminal boundary. This violates the no-incidental-metadata
        rule and would create a legacy string parser.
     2. Reject: treat artifact-store memory policy fields as observed runtime
        memory. Those fields are retention/cache policy limits, not measured
        inference execution facts.
     3. Preferred: add a canonical typed execution-resource observation
        contract at the inference/node execution boundary, then project it
        upward. Runtime backends may report `peak_ram_bytes`,
        `peak_vram_bytes`, and explicit `out_of_memory`; unavailable metrics
        remain absent or explicitly not available at the producer contract,
        not guessed later. Node-engine and embedded-runtime forward the typed
        observation, and workflow-service records it on `RunTerminalPayload`.
     4. Alternative: store memory facts only on inference diagnostic events
        and teach runtime-selection history to join inference diagnostics.
        This keeps terminal events smaller but fragments the scheduler-history
        source of truth and complicates exact workflow/task/model/runtime
        attribution.
   - 2026-05-18 clean design decision: implement option 3 as a typed
     replacement contract with platform-specific resource collectors isolated
     behind inference-owned monitor modules. Do not parse terminal error text,
     do not treat cache policy as observed runtime memory, and do not add OS
     or backend-specific measurement code to scheduler, workflow-service, or
     node-engine business logic.

### Execution Resource Observation Contract

Resource observation is runtime execution telemetry, not model-library facts,
artifact retention policy, or scheduler pressure state. The canonical contract
should be introduced at the inference execution boundary and projected upward
without changing ownership:

| Layer | Responsibility |
| ----- | -------------- |
| `inference` | Owns `InferenceExecutionResourceObservation`, memory metric kinds, memory failure kinds, observation-source ids, and platform/backend monitor implementations. |
| `node-engine` | Carries the typed observation on inference task results or lifecycle events without interpreting OS/runtime details. |
| `pantograph-embedded-runtime` | Projects node-engine/inference observations into workflow-service terminal payloads and diagnostics attribution. |
| `pantograph-workflow-service` | Records the already-typed observation on `RunTerminalPayload.resource_observation`; it does not derive memory facts from errors. |
| Scheduler/runtime-registry | Reads reduced history summaries later. It never imports platform monitor modules or calls OS probes directly. |

The inference contract should use typed fields rather than a generic metadata
map:

- `peak_ram_bytes: Option<u64>` for process or backend-reported host RAM
  high-water marks.
- `peak_vram_bytes: Option<u64>` for device memory high-water marks.
- `memory_failure_kind: Option<InferenceMemoryFailureKind>` with at least
  `out_of_memory`.
- `sources: Vec<InferenceResourceObservationSource>` so diagnostics can say
  whether a value came from a PyTorch CUDA counter, PyTorch MPS counter,
  managed runtime structured telemetry, or OS process RSS. Sources are bounded
  typed ids, not free-form labels.
- `availability: Vec<InferenceResourceObservationAvailability>` for metrics
  the runtime knows about but cannot report on the current platform, such as
  `not_available`, `not_implemented`, `runtime_not_installed`, or
  `unsupported_device`.

Unavailable metrics must not be converted to zero, omitted as silent success,
or backfilled from unrelated policy values. Absence is acceptable only when the
producer explicitly does not claim that metric; unavailable states are required
when a backend/runtime advertises the metric concept but cannot currently
produce it.

Codebase review refinement:

- Use `InferenceRequestLifecycleEvent` as the first canonical transport for
  execution resource observations. `InferenceExecutionResult` is task-output
  shaped and would require repetitive optional telemetry fields on every result
  variant. Lifecycle events already carry request, task, backend, runtime,
  model, device, phase, and timing attribution, so they are the lower-blast
  radius path for observed execution facts.
- Add a small inference lifecycle event builder/context before threading
  resource observations through `gateway.rs`. The current gateway lifecycle
  helpers already have repeated field-by-field constructors and several
  `too_many_arguments` allowances. Resource telemetry must be added once
  through a builder/context, not by adding another parameter to every lifecycle
  helper.
- Treat the Python worker boundary as a generic runtime producer. Add optional
  resource observation fields to the generic `PyTorchWorkerSuccess<T>` and
  `PyTorchWorkerFailure` envelopes, not only to image-generation metadata, so
  image generation, text generation, audio transcription, and later PyTorch
  task families can report the same facts.
- Keep `RunTerminalPayload.resource_observation` reduced to scheduler-history
  facts: peak RAM, peak VRAM, and memory failure kind. Preserve detailed
  source and availability facts in inference diagnostic events unless a later
  scheduler use case requires them on run-terminal events. This avoids
  widening the terminal ledger contract before the scheduler consumes those
  fields.
- Extend runtime-registry candidate history summaries with observed memory and
  OOM fields before enabling history-backed ranking. Diagnostics-ledger already
  computes memory/OOM history, but scheduler-facing candidate history currently
  exposes timing/queue fields only; memory ranking must not silently ignore the
  persisted memory facts.

Standards-compliance refinement:

- Standards reviewed for this slice: `PLAN-STANDARDS.md`,
  `CODING-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
  `TESTING-STANDARDS.md`, `DEPENDENCY-STANDARDS.md`,
  `CROSS-PLATFORM-STANDARDS.md`, `INTEROP-STANDARDS.md`,
  `DOCUMENTATION-STANDARDS.md`, and Rust-specific API, async,
  cross-platform, security, and dependency standards.
- Build resource observation DTOs as correct-by-construction contracts. Public
  or cross-crate enums/structs must use explicit serde casing, additive
  evolution, typed errors, and validation constructors where raw producer
  input crosses a crate, process, or language boundary. Avoid raw strings for
  metric kinds, source kinds, availability kinds, failure kinds, and runtime
  attribution.
- Bound diagnostic payloads. Source and availability collections must have a
  documented maximum, deterministic ordering, and de-duplication rules so a
  noisy runtime cannot create unbounded lifecycle events. Do not include raw
  process output, absolute local paths, environment values, or binary payloads
  in resource observations.
- Keep sampling overhead explicit. Any process-RSS sampler must use named
  interval and sample-count/timeout constants, refresh only the target process
  when the chosen API supports it, and report typed `not_available` or
  `not_implemented` instead of spinning or widening scope. Full-system refresh
  or expensive polling requires a written justification in the implementation
  slice.
- Own monitor lifecycle with a single execution owner. Sampling loops, if
  needed, must return a `#[must_use]` guard or equivalent handle, keep the
  `JoinHandle`/cancellation owner in the backend/runtime execution shell, and
  prove finish/cancel/drop cleanup in focused tests. No untracked
  `tokio::spawn`, no blocking work inside async request paths without
  `spawn_blocking`, and no locks held across awaits.
- Preserve dependency ownership. The first implementation uses the existing
  `sysinfo` dependency already owned by `inference`; new OS/API dependencies
  are forbidden unless `sysinfo` is proven insufficient, the dependency owner
  is the narrowest crate that executes it, the transitive cost is recorded, and
  any platform-specific or heavy dependency is feature-gated when appropriate.
- Treat existing threshold-crossing files as extraction risks. `gateway.rs`,
  `server.rs`, `embedding_runtime.rs`, `backend/llamacpp_support.rs`, and
  runtime-registry technical-fit modules already exceed the decomposition
  review threshold. Slices touching them must extract focused helpers or record
  why a narrow edit is safer; do not add new `too_many_arguments` allowances
  for telemetry wiring.
- Update module documentation when ownership changes. New `src/` directories,
  public resource-observation contracts, worker envelope changes, runtime
  monitor modules, or host-facing projections require README or ADR updates in
  the same slice with API consumer contract, structured producer contract,
  unsupported behavior, and revisit triggers.

Post-review implementation refinement:

- Treat diagnostic projection as a first-class slice, not incidental plumbing.
  `pantograph-embedded-runtime` currently persists
  `InferenceExecutionDiagnosticObservedPayload` only when its known bounded
  diagnostic fields are present. Adding resource observations to lifecycle
  events without extending that diagnostic payload and persistability gate
  would let telemetry exist in lifecycle status while silently skipping the
  diagnostic projection used by history and operator inspection. Add the
  diagnostic payload field, validation, truncation/bounding, and projection
  tests before terminal `RunTerminalPayload.resource_observation` wiring.
- Make the lifecycle builder/context a shared contract migration, not a
  `gateway.rs`-only helper. Node-engine, embedded-runtime, and tests also
  construct `InferenceRequestLifecycleEvent` directly. The builder/default
  constructor slice must migrate external constructors before telemetry fields
  become required or semantically meaningful, so the event contract remains
  easy to evolve and existing tests do not copy field lists.
- Keep resource monitoring independent from the current process-spawner task
  pattern. `StdProcessSpawner` uses untracked stdout/stderr/monitor
  `tokio::spawn` tasks today; resource sampling must not copy that style.
  Resource monitors need their own tracked guard/cancellation design and may
  expose future process-spawner cleanup as a separate follow-up rather than
  coupling this telemetry slice to a broad process lifecycle rewrite.
- Name the legacy OOM cleanup targets explicitly: `inference::server`,
  `inference::embedding_runtime`, and `backend::llamacpp_support`. Each must
  either stop parsing log text because structured telemetry exists, or keep
  parsing as a narrow external-runtime adapter translation that immediately
  emits typed memory-failure facts and bounded diagnostics. Do not let these
  strings flow into workflow terminal events, scheduler policy, or runtime
  history as text-derived behavior.
- Extract Python worker response helpers before adding resource telemetry to
  operation responses. The Python worker currently hand-builds repeated JSON
  success/error envelopes across operations. Add a small worker-local response
  builder that preserves the current contract and attaches optional resource
  observation in one place, then update Rust/Python fixture tests. Avoid
  adding per-operation telemetry dictionaries that would drift across text,
  image, audio, KV, load, unload, and health paths.

### Resource Monitor Platform Design

Resource monitoring should follow the repository's cross-platform pattern:
shared contracts and factories in platform-neutral modules, with `cfg()` kept
inside thin platform modules. The rest of the crate calls a neutral API such as
`RuntimeResourceMonitor::start()` / `finish()` or
`observe_execution_resources(...)` and receives typed observations.

Planned module shape:

| Module | Gate | Responsibility |
| ------ | ---- | -------------- |
| `inference::resource_observation` | none | Public typed DTOs, validation, source/availability enums, and merge rules for backend plus OS observations. |
| `inference::resource_monitor` | none | Trait/factory for monitor lifecycle, no scheduler policy. |
| `inference::resource_monitor::process_rss` | none, implemented with existing dependency first | Shared process RSS high-water observation using the existing `sysinfo` dependency and `ProcessHandle::pid()` when sufficient. It reports host RAM only, never VRAM. |
| `inference::resource_monitor::linux` | `#[cfg(target_os = "linux")]` | Linux-specific process memory implementation only if `sysinfo` is insufficient; prefer avoiding direct `/proc` parsing until a concrete gap is proven. No GPU assumptions. |
| `inference::resource_monitor::macos` | `#[cfg(target_os = "macos")]` | macOS-specific process memory implementation only if the shared `sysinfo` path is insufficient; MPS memory still comes from PyTorch/MPS telemetry when available. |
| `inference::resource_monitor::windows` | `#[cfg(target_os = "windows")]` | Windows-specific process memory implementation only if the shared `sysinfo` path is insufficient and the dependency/unsafe boundary is accepted; otherwise returns typed `not_implemented`. |
| `inference::resource_monitor::unsupported` | fallback cfg for unsupported targets | Compiles the neutral contract and reports typed unsupported/not-implemented availability. |
| PyTorch worker telemetry | runtime/backend capability, not OS cfg only | CUDA/MPS/CPU counters returned in the worker envelope when PyTorch exposes them. CUDA is runtime/device availability, not a Linux-only assumption. |
| Managed runtime telemetry | runtime adapter owned | Structured binary/API telemetry when available. Free-form logs may be interpreted only inside the runtime adapter and must be emitted as typed facts or diagnostics before crossing crate boundaries. |

CUDA, MPS, Metal, ROCm, MLX, vLLM, llama.cpp, and future runtimes must be
modeled as runtime/backend capabilities layered over platform collectors. For
example, PyTorch CUDA telemetry can be available on Linux or Windows, while
PyTorch MPS is macOS-specific. The scheduler only sees normalized runtime
variant/device ids plus observed bytes and failure kind.

Collector lifecycle rules:

- Observation starts at the smallest execution boundary that owns the runtime
  call, such as `generate_image_from_plan`, text generation, embedding
  execution, or managed binary request execution.
- Monitors must have deterministic cleanup and must not spawn unbounded
  polling loops. If a sampling loop is needed for process RSS, it must be
  bounded, cancellable, owned by the backend/runtime execution shell, and
  covered by cleanup tests.
- Backend-native counters should be preferred over OS process RSS for device
  memory. OS RSS is a host RAM signal and must not be labeled as VRAM.
- Metrics from different sources may be merged only by typed metric kind and
  source precedence. Conflicting values should keep the maximum observed value
  for peak metrics and record both sources for diagnostics.
- OOM reporting is typed by the backend/runtime adapter. Existing string
  detection inside backend support must either be replaced by structured
  backend errors or confined to the adapter that owns the external process
  contract; workflow terminal code must not string-match it.
- Existing llama.cpp and embedding-runtime OOM string detection is a legacy
  adapter-local translation point. The replacement slice must either retire it
  in favor of structured runtime telemetry or explicitly confine it to the
  external-process adapter and convert it immediately to typed
  `InferenceMemoryFailureKind::OutOfMemory` facts. It must not leak as
  workflow terminal string matching or scheduler policy.

Staged implementation:

1. [x] Add the inference-owned execution-resource observation DTOs and
   validation tests. This slice is shared-contract only and should not wire
   scheduler ranking.
   - 2026-05-18: completed the shared inference contract slice.
     `inference::resource_observation` now owns typed execution resource
     telemetry DTOs for peak RAM bytes, peak VRAM bytes, memory failure kind,
     metric source attribution, and explicit metric unavailability. The
     constructor and serde decode path validate non-empty observations, reject
     zero-valued peak metrics, bound source/availability collections before
     de-duplication, deterministically order facts, and reject source
     attribution without a matching metric or availability fact. This slice
     does not wire lifecycle events, terminal payloads, scheduler ranking,
     worker telemetry, process sampling, or legacy OOM string translation.
   - Verification: `cargo test -p inference resource_observation --lib`,
     `cargo check -p inference`, `cargo fmt --all -- --check`, and
     `git diff --check`.
2. [x] Extract a small inference lifecycle event builder/context and update
   lifecycle tests before adding telemetry fields. This prevents
   resource-observation wiring from expanding the existing `gateway.rs`
   repeated constructor and `too_many_arguments` pattern and migrates
   node-engine/embedded-runtime test constructors to the same event-building
   boundary.
   - 2026-05-18: completed the lifecycle event builder/context migration.
     `InferenceRequestLifecycleEvent::builder` and
     `InferenceRequestLifecycleEventContext` now centralize event
     construction. Direct lifecycle event struct literals were removed from
     gateway, node-engine lifecycle emitters, inference contract tests, and
     embedded-runtime lifecycle tests; only the builder's internal
     initialization remains. The slice does not add resource telemetry fields
     or alter lifecycle semantics.
   - Discovered issue resolved: an embedded-runtime technical-fit unit-test
     fixture still constructed `RuntimeSelectionHistorySummary` without the
     newer memory/OOM fields, which blocked embedded-runtime lifecycle test
     compilation. The fixture now provides explicit zero/none memory history
     facts. The workflow lifecycle sink tests also queried node-status
     projections without applying the projection refresh requested by
     diagnostic append; the fixtures now refresh the node-status projection
     explicitly before querying.
   - Verification: `cargo test -p inference lifecycle --lib`, `cargo test -p
     inference --test model_contracts
     public_inference_contract_json_keys_avoid_scheduler_policy_language`,
     `cargo test -p node-engine inference_lifecycle --lib`, `cargo test -p
     pantograph-embedded-runtime inference_lifecycle --lib`, `cargo test -p
     pantograph-embedded-runtime
     runtime_selection_history_summaries_project_exact_candidate_keys --lib`,
     `cargo check -p pantograph-embedded-runtime`, `cargo fmt --all --
     --check`, and `git diff --check`.
3. [x] Extend `InferenceExecutionDiagnosticObservedPayload` and
   embedded-runtime diagnostic projection with bounded resource-observation
   fields and persistability-gate coverage. This slice proves lifecycle-event
   resource observations cannot be silently dropped before terminal payload
   projection is implemented.
   - 2026-05-18: completed diagnostic resource-observation projection. The
     diagnostics-ledger inference diagnostic payload now carries a bounded
     `resource_observation` summary with peak RAM bytes, peak VRAM bytes,
     typed memory failure kind, source summaries, and availability summaries.
     Embedded-runtime maps the inference-owned typed observation into this
     diagnostic payload and treats resource observation as a persistability
     condition, including task-validation events. This slice intentionally
     does not project resource observations into run-terminal payloads or
     scheduler history.
   - Sequencing deviation: the lifecycle event needed the optional
     `resource_observation` field in this slice to prove diagnostic
     persistability. This also completes staged item 5's shared event-contract
     part before the resource monitor slice, without wiring any producers.
   - No-fallback/no-legacy confirmation: resource-observation mapping uses
     explicit enum cases and exhaustive compiler checks. New metric/source or
     unavailable-state variants must update the projection intentionally; they
     are not silently mapped to generic fallback strings.
   - Verification: `cargo test -p inference lifecycle --lib`, `cargo test -p
     pantograph-diagnostics-ledger
     diagnostic_event_ledger_validates_inference_execution_diagnostic_scope_and_bounds
     --lib`, `cargo test -p pantograph-embedded-runtime
     inference_diagnostic_event_adapter_persists_resource_observation_without_other_diagnostics
     --lib`, `cargo test -p pantograph-embedded-runtime inference_diagnostic
     --lib`, `cargo check -p pantograph-embedded-runtime`, `cargo fmt --all
     -- --check`, and `git diff --check`.
4. [x] Add `resource_monitor` factory/modules with a `sysinfo` process-RSS
   first implementation, Linux/macOS/Windows/unsupported gates for proven
   platform gaps, and tests proving the neutral API compiles without
   scattering `cfg()` through business logic.
   - 2026-05-18: completed the first resource-monitor implementation slice.
     `inference::resource_monitor` now owns a platform-neutral
     `RuntimeResourceMonitor` contract, default factory, `#[must_use]`
     monitor guard, typed lifecycle errors, and a process-RSS sampler backed by
     the existing `sysinfo` dependency. Platform selection is isolated in the
     resource-monitor platform module with Linux/macOS/Windows files selecting
     the shared `sysinfo` process-RSS implementation and unsupported targets
     returning typed `unsupported_platform` availability facts.
   - No-fallback/no-legacy confirmation: unobservable process RSS is reported
     as an explicit `PeakRamBytes` availability fact sourced from
     `OsProcessRss`; it is not converted to zero, inferred as VRAM, or used as
     scheduler policy. The monitor reports host RAM only and keeps CUDA/MPS or
     managed-runtime device telemetry for later backend-owned producer slices.
   - Lifecycle/concurrency confirmation: the process-RSS sampler is owned by a
     `#[must_use]` guard, uses named constants for thread name and interval,
     refreshes only the target PID, stops through an atomic cancellation flag,
     joins on `finish`, and performs synchronous cleanup in `Drop`. This slice
     does not copy the existing untracked `tokio::spawn` process-spawner
     pattern and does not wire monitors into async execution paths.
   - Verification: `cargo test -p inference resource_monitor --lib`, `cargo
     test -p inference process_rss --lib`, `cargo check -p inference`, `cargo
     check -p inference --no-default-features`, `cargo check -p inference
     --all-features`, `cargo fmt --all`, `cargo fmt --all -- --check`, and
     `git diff --cached --check`.
5. [x] Extend `InferenceRequestLifecycleEvent` to carry
   `InferenceExecutionResourceObservation` for all task families. Image
   generation is one consumer, not the contract owner.
   - 2026-05-18: completed as part of the diagnostic projection slice. The
     field is optional on the shared lifecycle event and available to every
     task family; producer wiring remains staged separately.
6. [x] Extract Python worker response helpers, then extend the generic
   PyTorch worker success/failure envelope with optional resource observation
   and update Python worker shape checks before adding task-specific
   producers.
   - 2026-05-18: completed the response-helper extraction sub-slice. The
     Python worker now builds success and error responses through
     `worker_contract.py` helpers, including a single optional
     `resource_observation` attachment point for the later typed envelope
     extension. `worker.py` no longer hand-builds per-operation JSON
     success/error response dictionaries across init, shutdown, load,
     generate text, image generation, audio transcription, unload, stream
     setup, and KV operations. This sub-slice intentionally does not add Rust
     `PyTorchWorkerSuccess`/`PyTorchWorkerFailure.resource_observation` fields
     or backend telemetry producers.
   - Discovered issue resolved: the generic PyTorch worker module test harness
     did not register a stub `worker_image_contract` module even though
     `worker.py` imports it at module load. The harness now provides that stub
     so generic worker response tests cover the real module import boundary.
   - No-fallback/no-legacy confirmation: this is a contract-construction
     extraction only. It preserves existing response shapes and error kinds,
     does not infer or synthesize resource telemetry, and leaves typed
     telemetry decoding/projection for the next Rust envelope sub-slice.
   - Verification: `cargo test -p inference --features backend-pytorch
     test_python_worker_response_helpers --lib`, `cargo test -p inference
     --features backend-pytorch
     test_python_worker_shutdown_from_envelope_returns_structured_success
     --lib`, `cargo test -p inference --features backend-pytorch
     test_python_worker_generate_image_from_envelope_returns_worker_response
     --lib`, `cargo test -p inference --features backend-pytorch
     pytorch_worker_generate_text_success_response_returns_text --lib`, and
     `cargo test -p inference --features backend-pytorch pytorch_worker
     --lib`, `cargo check -p inference --features backend-pytorch`, `cargo
     fmt --all -- --check`, and `git diff --check` for the allowed write set.
   - 2026-05-18: completed the generic Rust envelope sub-slice.
     `PyTorchWorkerSuccess<T>` and `PyTorchWorkerFailure` now carry optional
     `InferenceExecutionResourceObservation` fields with serde defaults, so
     existing worker responses remain valid while future PyTorch producers can
     attach typed peak-memory, unavailable-metric, or OOM facts without adding
     task-specific telemetry dictionaries.
   - No-fallback/no-legacy confirmation: the Rust envelope accepts only the
     inference-owned typed resource observation contract. Invalid observation
     payloads fail serde construction through the existing validated DTO
     instead of being accepted as incidental metadata or raw Python dicts.
     Backend failure constructors use `None` until a real producer reports
     observations; they do not synthesize memory facts from error strings.
   - Verification: `cargo test -p inference --features backend-pytorch
     test_pytorch_worker_success_response_decodes_resource_observation --lib`,
     `cargo test -p inference --features backend-pytorch
     test_pytorch_worker_error_response_decodes_resource_observation --lib`,
     `cargo test -p inference --features backend-pytorch pytorch_worker
     --lib`, `cargo check -p inference --features backend-pytorch`, `cargo
     fmt --all -- --check`, and `git diff --check` for the allowed write set.
7. [x] Extend node-engine inference task result/event plumbing to forward the
   observation without interpreting metric sources.
   - 2026-05-18 re-plan boundary: this item is ambiguous after the lifecycle
     event contract decision. The implemented canonical transport for resource
     observations is `InferenceRequestLifecycleEvent`, and node-engine already
     forwards the host-owned lifecycle sink into `InferenceGateway` for typed
     canonical inference paths without interpreting lifecycle payloads. Adding
     resource fields to `InferenceExecutionResult` or node outputs here would
     reintroduce a second telemetry path and conflict with the plan's
     lifecycle-first/no-fallback rule.
   - Clean options to resolve the boundary:
     1. Treat item 7 as already satisfied for node-engine because the canonical
        forwarding path is the lifecycle sink and keep the next implementation
        slice in inference gateway/backend producers.
     2. Rename item 7 to a focused verification slice that adds a node-engine
        test proving canonical inference execution passes lifecycle events
        with arbitrary event payloads through without interpretation, without
        adding result DTO fields.
     3. Re-plan to make `InferenceExecutionResult` carry resource observations
        as a second transport, then update node-engine outputs and gateway
        projection. This is rejected by the current plan because it widens
        every result variant and duplicates lifecycle telemetry ownership.
   - Recommended resolution: option 1 or option 2. Do not implement option 3
     unless the lifecycle-event transport decision is explicitly reversed.
   - 2026-05-18: completed option 2 as a verification-only slice.
     `test_canonical_llm_text_keeps_resource_observation_on_lifecycle_events`
     now runs canonical node-engine text inference with a test-only lifecycle
     sink that attaches a typed `InferenceExecutionResourceObservation` to the
     backend-execution completion event. The assertions prove node-engine
     forwards lifecycle events with typed resource observations intact while
     keeping task output JSON free of `resource_observation` fields.
   - No-fallback/no-legacy confirmation: this slice added no production
     fallback path, no result DTO telemetry field, no node-output telemetry
     field, and no metric-source interpretation in node-engine. The canonical
     transport remains `InferenceRequestLifecycleEvent`.
   - Verification: `cargo test -p node-engine --features inference-nodes
     test_canonical_llm_text_keeps_resource_observation_on_lifecycle_events
     --lib`, `cargo check -p node-engine --features inference-nodes`, and
     `cargo fmt --all -- --check`.
8. [x] Extend embedded-runtime projection into inference diagnostics and
   `RunTerminalPayload.resource_observation`, including mapping tests for
   peak RAM, peak VRAM, explicit OOM, source/availability diagnostics, and
   unavailable metrics that should not be persisted as fake terminal values.
   - Definition: this item is about completed-run resource history, not live
     scheduler capacity sampling. Live capacity monitoring answers "how much
     memory is available now"; `RunTerminalPayload.resource_observation`
     answers "what did this completed workflow run actually use, and did it
     hit OOM?" Scheduler policy needs both, but this milestone supplies the
     historical memory/OOM facts used to learn model/runtime behavior over
     repeated runs.
   - `RunTerminal` is the single final diagnostics-ledger event for a workflow
     run. It records completed/failed/cancelled status, duration, terminal
     error identity when present, and the compact run-level resource summary
     used by run-list and runtime-selection history projection.
   - Ownership rule: detailed per-inference observations originate on
     inference lifecycle events, embedded-runtime persists those details as
     inference diagnostics, diagnostics-ledger summarizes persisted inference
     observations into one run-level terminal resource observation, and
     workflow-service writes that summary when it emits the single owned
     `RunTerminal` event. No crate other than workflow-service should emit or
     mutate terminal run completion events.
   - 2026-05-18 re-plan boundary: the inference-diagnostic half of this item
     is already implemented in `pantograph-embedded-runtime`.
     `node_execution_ledger.rs` maps lifecycle
     `InferenceExecutionResourceObservation` values into
     `InferenceExecutionDiagnosticObservedPayload.resource_observation`, and
     `node_execution_ledger_tests.rs` covers peak RAM, peak VRAM, OOM,
     source summaries, and availability summaries.
   - The remaining requirement is `RunTerminalPayload.resource_observation`.
     That terminal event is emitted by
     `pantograph-workflow-service::record_run_terminal_event_if_configured`,
     while `pantograph-diagnostics-ledger` runtime-selection history currently
     reads observed memory/OOM fields from `RunTerminal` payloads only. An
     embedded-runtime-only implementation would either miss terminal/history
     projection or duplicate terminal ownership.
   - Clean options:
     1. Keep resource observations only on inference diagnostic events and
        leave `RunTerminalPayload.resource_observation` empty. This is rejected
        because scheduler history and run-list projection would not receive
        terminal memory/OOM facts.
     2. Preferred: add a diagnostics-ledger run resource rollup query over
        persisted `InferenceExecutionDiagnosticObserved` events, returning a
        compact `RunResourceObservation` with max peak RAM, max peak VRAM, and
        OOM when any inference event reports it. Workflow-service should call
        that query while appending the single owned `RunTerminal` event.
        Availability-only observations remain inference diagnostics and must
        not be converted into fake terminal byte values.
     3. Have embedded-runtime emit or mutate `RunTerminal` directly. This is
        rejected because it creates a second terminal-event owner and risks
        duplicate terminal facts.
     4. Teach run-list/history projection to read inference diagnostic events
        directly instead of terminal payloads. This is a later projection
        redesign candidate, but it does not satisfy the explicit terminal DTO
        contract in this milestone and spreads resource-summary policy across
        projection code.
   - Recommended staged replacement:
     1. [x] Add diagnostics-ledger rollup API and tests for max RAM/max VRAM/OOM,
        no observations, and availability-only observations.
     2. [x] Update workflow-service terminal event recording to include the rollup
        result and add focused tests proving a single `RunTerminal` owns the
        run-level observation.
     3. [x] Mark the embedded-runtime diagnostic projection sub-slice complete in
        this item without adding duplicate terminal behavior there.
   - 2026-05-18: completed the diagnostics-ledger rollup sub-slice.
     `RunResourceObservationRollupQuery` and
     `DiagnosticsLedgerRepository::run_resource_observation_rollup` summarize
     persisted `InferenceExecutionDiagnosticObserved` events for one workflow
     run into the existing compact `RunResourceObservation` terminal DTO. The
     rollup uses max peak RAM, max peak VRAM, and OOM if any inference
     diagnostic reports it.
   - No-fallback/no-legacy confirmation: the rollup reads only typed
     inference diagnostic resource observations. It does not parse terminal
     error strings, scheduler metadata, runtime names, or unavailable metric
     diagnostics as memory usage. Availability-only observations remain
     detailed inference diagnostics and produce no fake terminal byte values.
   - Verification: `cargo test -p pantograph-diagnostics-ledger
     run_resource_observation_rollup --lib`, `cargo check -p
     pantograph-diagnostics-ledger`, and `cargo fmt --all -- --check`.
   - 2026-05-18: completed the workflow-service terminal wiring sub-slice.
     `record_run_terminal_event_if_configured` now reads the diagnostics-ledger
     rollup immediately before appending the workflow-service-owned
     `RunTerminal` event and stores that compact run-level observation in
     `RunTerminalPayload.resource_observation`.
   - No-fallback/no-legacy confirmation: workflow-service does not compute
     resource facts itself and does not inspect inference diagnostics directly.
     It delegates rollup policy to diagnostics-ledger, preserves single
     terminal-event ownership, and does not introduce a second terminal
     producer in embedded-runtime.
   - Verification: `cargo test -p pantograph-workflow-service
     run_terminal_event_includes_diagnostics_ledger_resource_rollup --lib`,
     `cargo check -p pantograph-workflow-service`, and `cargo fmt --all --
     --check`.
9. [ ] Add backend producers incrementally: PyTorch CUDA/MPS worker telemetry,
   shared process RSS where supported, and managed runtime structured
   telemetry. Each producer slice must include focused tests/fixtures for its
   source and availability states.
   - 2026-05-18: completed the first PyTorch image-generation producer
     sub-slice. The Python image worker now resets CUDA peak memory stats
     before the planned load/generate operation and attaches typed
     `peak_vram_bytes` with `pytorch_cuda` source facts to successful worker
     responses when CUDA reports a positive peak allocation.
   - No-fallback/no-legacy confirmation: the producer reads PyTorch CUDA's
     typed memory API only. It does not parse error strings, infer memory from
     scheduler/device names, or synthesize zero-byte terminal facts. CPU remains
     unreported here because process RSS is owned by the separate shared
     monitor producer.
   - Verification: `cargo test -p inference --features backend-pytorch
     test_python_worker_generate_image_from_envelope_reports_cuda_peak_vram
     --lib`, `cargo test -p inference --features backend-pytorch
     pytorch_worker_image_python --lib`, `cargo check -p inference --features
     backend-pytorch`, and `cargo fmt --all -- --check`.
   - 2026-05-18: completed the PyTorch image-generation MPS availability
     sub-slice. The worker emits a typed `peak_vram_bytes` availability fact
     with `not_implemented` state and `pytorch_mps` source when MPS is
     available but PyTorch exposes no canonical peak VRAM counter.
   - No-fallback/no-legacy confirmation: MPS telemetry does not synthesize
     zero-byte observations or infer VRAM from scheduler/device metadata. It
     reports a typed unavailable metric until a canonical runtime counter is
     available.
   - Verification: `cargo test -p inference --features backend-pytorch
     test_python_worker_generate_image_from_envelope_reports_mps_metric_unimplemented
     --lib`.
   - 2026-05-18: completed the planned image-generation process-RSS producer
     sub-slice. The gateway now starts the neutral runtime resource monitor
     for the current process at the planned image backend execution boundary
     and attaches the finished observation only to the backend execution
     terminal lifecycle event.
   - No-fallback/no-legacy confirmation: process RSS remains host RAM only,
     sourced as `os_process_rss`, and is not written into task output metadata
     or relabeled as VRAM. Monitor startup/finish errors are logged as
     telemetry producer failures rather than replaced with synthetic byte
     values.
   - Verification: `cargo test -p inference
     test_generate_image_from_planning_input_with_lifecycle_records_planned_decision
     --lib` and `cargo test -p inference image_generation --lib`.
   - 2026-05-18: completed the generic typed non-streaming process-RSS
     lifecycle producer sub-slice. `execute_typed_with_lifecycle` now starts
     the same neutral monitor around backend execution for text, embedding,
     rerank, image, and audio typed requests, then attaches the observation to
     the backend execution terminal event.
   - No-fallback/no-legacy confirmation: typed task outputs still do not carry
     process RSS telemetry, and stream lifecycle behavior remains unchanged
     until it gets a dedicated bounded monitor lifecycle slice.
   - Verification: `cargo test -p inference
     test_execute_typed_text_reports_generation_option_diagnostics --lib` and
     `cargo test -p inference lifecycle --lib`.
   - Remaining producer follow-ups: surface backend-native worker observations
     through lifecycle without task-output coupling, add process-RSS lifecycle
     coverage to streaming/runtime paths in a dedicated slice, managed runtime
     structured telemetry, real MPS metric support if PyTorch exposes a
     canonical counter, and failure-path OOM typing without broad legacy string
     parsing.
10. [ ] Remove or explicitly confine legacy OOM string parsing in
    `inference::server`, `inference::embedding_runtime`, and
    `backend::llamacpp_support` behind typed adapter-local memory failure
    translation.
11. [ ] Extend runtime-registry candidate history DTOs and embedded-runtime
   history projection with observed memory and OOM fields already computed by
   diagnostics-ledger.
12. [ ] Activate scheduler history weighting only after observations are
    available in runtime-selection history and the existing five-completed-run
    threshold per valid runtime candidate is enforced.

Verification for this staged design should include:

- `cargo test -p inference resource_observation --lib`
- `cargo test -p inference resource_monitor --lib`
- `cargo test -p inference process_rss --lib`
- `cargo test -p inference lifecycle --lib`
- `cargo test -p inference pytorch_worker --lib`
- `cargo test -p node-engine inference_resource_observation --lib`
- `cargo test -p pantograph-embedded-runtime resource_observation --lib`
- `cargo test -p pantograph-runtime-registry history --lib`
- `cargo test -p pantograph-workflow-service diagnostics --lib`
- `cargo fmt --all -- --check`
- `git diff --check`

Cross-target compile checks should be added to CI or local release gates when
the OS modules are implemented:

- `cargo check --workspace --target x86_64-unknown-linux-gnu`
- `cargo check --workspace --target x86_64-pc-windows-msvc`
- `cargo check --workspace --target aarch64-apple-darwin`

Dependency and feature verification is required when implementation changes
Cargo manifests or feature contracts:

- `cargo tree -p inference -i sysinfo`
- `cargo tree -p inference --depth 1`
- `cargo check -p inference --no-default-features`
- `cargo check -p inference --all-features`

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
