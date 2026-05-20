# Milestone 6: PyTorch/Diffusers Image Generation Execution Slice

**Goal:** Prove canonical image-generation inference can execute through
PyTorch/diffusers and produce a retained image artifact.

**Tasks:**

- [x] Add an image-generation execution planner that consumes
  `ImageGenerationRequest`, Pumas package facts, graph/runtime hints,
  dependency readiness, backend capabilities, and device policy, then returns
  one explicit execution plan or a bounded diagnostic.
- [x] Consume the device resolution decision from Milestone 5 instead of
  parsing raw device strings or auto-selecting devices inside image-generation
  planning.
- [x] Implement the planner in a focused inference module such as
  `backend/pytorch/image_generation` or `image_generation_planner` and keep
  `pytorch.rs` as a facade over load/generate calls.
- [x] Keep planner parsing/validation synchronous and side-effect free. Async
  shells may gather Pumas facts, dependency readiness, and device facts before
  calling the planner.
- [x] Add a planned image-generation gateway boundary before end-to-end gateway
  wiring. The gateway must carry reduced package facts and the scheduler-owned
  `BackendExecutionDecision` into `ImageGenerationExecutionPlan`; it must not
  dispatch image generation from `ImageGenerationRequest` alone or infer
  backend/runtime/device/package decisions from request fields.
- [x] Add a first-class run-scoped workflow execution plan before successful
  end-to-end image execution. The plan is produced by scheduler/admission,
  contains per-node reduced execution decisions, and is consumed by node
  execution without writing scheduler facts into graph inputs.
- [x] Project the workflow execution plan's image-generation node decision into
  inference's `BackendExecutionDecision` at the composition boundary before
  calling `generate_image_from_planning_input`.
- [x] Canonicalize selected Pumas model identity at the execution-plan
  admission/projection boundary before using it for scheduler history,
  lifecycle diagnostics, runtime readiness, or worker dispatch. A selected
  model value that is already a `pumas://models/...` ref must not be blindly
  prefixed again; malformed refs, local paths, or ambiguous model/artifact
  identities must fail with typed execution-plan diagnostics rather than being
  repaired by inference nodes.
- [x] Parse selected model identity once at the workflow execution-plan owner
  boundary into a validated workflow-owned model-ref type, then project that
  validated value into inference. Embedded-runtime projection may adapt the
  type shape, but it must not re-parse, re-prefix, repair, or reinterpret raw
  selected model strings.
- [x] If the model-ref normalization slice introduces a new source module,
  public constructor, or changed execution-plan boundary contract, update the
  owning README or add an ADR in the same commit. The documentation must state
  the parse-once invariant, accepted raw selected-fact forms, rejected local
  path/URI forms, append-only evolution rules, and scheduler-history identity
  semantics.
- [x] Add focused workflow-service execution-plan model-ref normalization tests
  covering raw model ids, already-prefixed Pumas refs, malformed/local-path
  refs, and admission matching through the public constructor/admission
  boundary rather than private helper details.
- [x] Add focused embedded-runtime projection coverage proving the validated
  workflow execution-plan model ref reaches `BackendExecutionDecision` without
  re-parsing, re-prefixing, repairing, or reinterpreting raw selected model
  strings. The slice also repaired the stale embedded-runtime
  `PortOptionsQuery.context` fixture required to run the projection test
  target.
- [x] Validate planned image execution identity consistency before the image
  planner returns an `ImageGenerationExecutionPlan`: the scheduler-selected
  model ref from `BackendExecutionDecision` must match the resolved Pumas
  package facts model ref used for worker dispatch. For image generation, a
  missing scheduler-selected model ref or a present mismatch must fail with a
  typed planner or execution-plan diagnostic rather than silently using
  package facts as an implicit model decision. The execution-plan contract may
  keep selected model refs optional only for task families where the
  scheduler/admission decision is explicitly not model-bound.
- [x] Tighten selected-decision fact typing at the workflow execution-plan
  boundary. Keep workflow-service independent from inference DTOs, but use
  workflow-owned validated constructors or newtypes for selected backend key,
  runtime id, runtime variant id, device id, and model ref so invalid selected
  facts fail before node-engine receives runtime context.
- [x] Verify backend runtime selection maps diffusion/image-generation package
  facts and graph hints to PyTorch execution, preserving `diffusers` as the
  dependency and package capability label.
- [x] Treat graph runtime requests as scheduler inputs, never execution
  shortcuts. An omitted runtime on an inference node means runtime selection is
  implicitly owned by scheduler/admission policy. An explicit graph runtime
  request is a hard scheduler requirement: the scheduler must select that
  runtime from validated executable candidates or emit a typed diagnostic that
  explains why it cannot be used. Explicit runtime requests must not bypass
  package/capability validation, memory-fit checks, or lifecycle diagnostics,
  and they must not become fallback runtime choices.
- [x] Add an optional `runtime` input to inference graph nodes for explicit
  scheduler requirements. The input is only graph intent; it must be projected
  into scheduler/admission request data and must never be passed directly from
  the graph or node executor into inference execution. The inference crate
  receives runtime selection exclusively through the scheduler-produced
  execution decision so there is one source of truth and one runtime-selection
  path.
- [x] Replace scattered `diffusers` backend-key mappings that affect execution
  with calls into the single normalization boundary defined in Milestone 0.
- [x] Keep package/dependency keys and execution backend keys represented by
  distinct function/type names. `diffusers` may remain factual package,
  dependency, or capability evidence, but must not become a graph-visible or
  scheduler-selected execution backend key through shared string helpers.
- [x] Audit existing dependency preflight, technical-fit, runtime capability,
  gateway, and workflow runtime preflight mappings so execution selection,
  runtime display, and dependency diagnostics do not each maintain conflicting
  `diffusers` rules.
  - 2026-05-17: workflow-service runtime requirement extraction no longer
    recursively scans arbitrary node JSON for `backend_key`, Pumas package
    hints, dependency bindings, or GGUF paths. It now reads only the
    graph-authored `llm-inference.runtime` input for canonical inference hard
    scheduler requirements and uses explicit node-family constants for
    dedicated `onnx-inference` and `audio-generation` nodes. Focused audit
    search found no remaining generic backend-key value scanner in
    workflow-service, embedded-runtime, inference, node-engine, or
    workflow-nodes.
- [x] Replace embedded-runtime technical-fit candidate construction with an
  `ExecutionEvidenceTechnicalFitAdapter` boundary. The adapter must consume
  inference-owned `ExecutionEvidenceReport` values plus workflow runtime
  capability/readiness context and produce `RuntimeTechnicalFitCandidate`
  values and typed technical-fit diagnostics. It must not preserve the old
  direct package-hint/backend compatibility loops as fallback behavior.
- [x] Add an explicit diagnostics mapping table for the
  `ExecutionEvidenceTechnicalFitAdapter`. Each inference-owned
  `ExecutionEvidenceDiagnostic` kind must map to one runtime-registry
  technical-fit diagnostic code with preserved attribution for task id,
  backend/runtime id, model/package facts, and explicit graph runtime
  requirements where available. Unsupported tasks, unavailable backends,
  missing runtime capabilities, missing required package evidence,
  compatibility rejection, explicit graph runtime mismatch, and no accepted
  candidate must remain distinguishable; the adapter must not flatten them into
  a generic no-fit message or use a broad catch-all diagnostic except for
  genuinely unknown future non-exhaustive codes.
- [x] Extend technical-fit diagnostics as an append-only contract before wiring
  the evidence adapter. Prefer adding typed runtime-registry diagnostic codes
  and structured attribution fields, then project them through embedded-runtime
  and workflow-service, over encoding evidence meaning in diagnostic messages
  or compatibility issue strings. The new contract should include the minimum
  fields needed for long-term scheduler history and diagnostics analysis:
  task id, selected or requested backend/runtime key, selected or requested
  runtime variant id where available, model id/ref, package/evidence key, and
  explicit graph runtime request. Keep the design small and append-only; do
  not introduce a parallel diagnostic envelope unless the existing
  `RuntimeTechnicalFitDeviceDiagnostic` shape cannot be evolved without
  breaking consumers.
- [x] Implement the diagnostic contract extension as a serial shared-contract
  slice before adapter wiring. Runtime-registry remains the source contract,
  embedded-runtime owns projection into workflow-service DTOs, and any exposed
  Tauri/UniFFI/Rustler/frontend mirrors or JSON fixtures must be updated in the
  same slice when they carry these diagnostics. Public diagnostic enums/DTOs
  that are expected to keep growing should use append-only serde-compatible
  evolution and `#[non_exhaustive]` where appropriate; projection code must
  match every known variant explicitly instead of collapsing new evidence
  diagnostics into message strings.
- [x] Keep the diagnostic mapping and attribution projection in focused modules
  instead of growing already broad technical-fit files. Add or update module
  README/ADR traceability for runtime-registry, embedded-runtime, and
  workflow-service ownership changes, including wire-format defaults,
  append-only evolution rules, and the no message-string parsing invariant.
- [x] Add contract and projection tests for the diagnostic extension before
  adapter wiring: runtime-registry serde/default/normalization tests,
  embedded-runtime runtime-to-workflow projection tests, workflow-service
  public DTO tests, and fixture or binding mirror tests for any exposed
  interop surfaces. Tests must cover every new diagnostic code and attribution
  field, unknown/omitted optional fields, and prove diagnostics survive
  projection without string matching.
- [x] Treat "no accepted execution-evidence candidate" as an explicit adapter
  outcome. Either add an inference-owned diagnostic code before mapping or
  synthesize one typed runtime-registry diagnostic in the adapter, but do not
  infer it only from an empty candidate list without a durable diagnostic code
  and attribution.
- [x] Retire scheduler-visible pseudo-Diffusers runtime/backend paths unless a
  real executable Diffusers backend is registered. `diffusers` may remain a
  display/dependency/capability label, but `python_runtime_capabilities`,
  runtime capability projection, and tests must not advertise `diffusers` as a
  selectable runtime/backend when the executable backend is PyTorch.
- [x] Update workflow capability extraction so package-owned
  `recommended_backend` fields from Pumas facts do not become hard required
  executable backends. Only the explicit graph-owned `runtime` input on an
  inference node may become a hard scheduler runtime requirement; package
  recommendations should flow through the inference-owned evidence boundary.
- [x] Keep node-engine dependency-context forwarding from becoming a backend
  decision path. Node-engine may carry model intent and host-installed planned
  inference decisions, but Pumas `recommended_backend` and dependency metadata
  must not be interpreted there as executable backend selection.
- [x] Add or update centralized normalization tests for an explicit graph
  `runtime = pytorch` request and Pumas Diffusers package hints. `diffusers`
  remains package/runtime capability evidence, not a graph-visible runtime
  request.
- [x] Add a general execution-evidence normalization boundary before replacing
  Diffusers mappings. This boundary belongs beside inference model/package
  contracts because it interprets Pumas/package facts, artifact kinds, task
  evidence, and backend hints; it must not live inside the image planner,
  PyTorch backend, node-engine, workflow-service, or the scheduler policy
  itself. Shared runtime-identity helpers may continue to normalize stable
  backend id spelling, but they must not decide that one package/runtime
  evidence label selects another executable backend.
- [x] Model execution evidence with typed roles that keep these concepts
  separate: executable backend candidate, dependency/package evidence,
  runtime capability evidence, graph preference/constraint, and display label.
  The same evidence model must be reusable for Transformers/PyTorch,
  GGUF/llama.cpp, ONNX Runtime, Candle embeddings, vLLM, MLX, and future
  runtimes. Diffusers image generation is only the first consumer.
- [x] Implement the Diffusers rule as data under that general boundary:
  package facts such as `artifact_kind = diffusers_bundle`, Diffusers package
  status, and Diffusers/backend hints can produce a PyTorch executable
  candidate only when PyTorch capability facts advertise the required
  Diffusers support. `diffusers` remains dependency/package/capability
  evidence and must not be emitted as a scheduler-selected executable backend
  key unless a real executable Diffusers backend is registered in the future.
- [x] Keep image/PyTorch-specific behavior inside the PyTorch/diffusers image
  bridge after scheduler selection. The image planner may require a
  scheduler-selected PyTorch backend and Diffusers package facts, but it must
  not own package-hint-to-backend candidate normalization, runtime ranking,
  warmed-runtime affinity, or historical scheduler policy.
  - 2026-05-17: inference now keeps family-specific PyTorch/Diffusers bridge
    behavior behind `image_generation_family_adapters`; scheduler-facing
    package-hint normalization remains in `execution_evidence`, and planner
    validation only consumes the scheduler-selected backend decision plus
    package facts.
- [x] Rename image-generation sampling-scheduler fields to
  `denoising_scheduler` across graph ports, node-engine request construction,
  inference planner DTOs, worker envelopes, Python worker inputs, diagnostics,
  and fixtures. Keep any compatibility handling explicit and temporary; do not
  let the overloaded `scheduler` name remain as the canonical graph/API field.
  Diffusers/Pumas package component facts may still use factual component-role
  names such as `scheduler` and paths such as `scheduler/scheduler_config.json`;
  the rename applies to Pantograph graph/API execution intent, not source
  package evidence.
- [x] Ensure `ImageGenerationRequest` is populated from canonical
  `llm-inference` inputs: prompt, negative prompt, width, height, steps,
  guidance scale, seed, optional `denoising_scheduler`, and image count.
- [x] Expose `denoising_scheduler` as a first-class optional image-generation
  input on canonical `llm-inference`. A connected `selection-input` may provide
  the value, but unset means the selected model/pipeline default by explicit
  policy rather than a fallback.
- [x] Extend backend port-option querying with an append-only typed context
  before implementing fact-aware denoising scheduler options. The context must
  carry stable references such as target node id, task kind, selected model
  ref, package-facts summary cursor, and optional backend/runtime constraint;
  it must not push full Pumas package facts, scheduler decisions, worker
  envelopes, graph node payloads, or local filesystem paths through frontend
  state.
- [x] Update every port-option interop surface in the same context slice:
  `node-engine` contracts/provider tests, Tauri `query_port_options`, UniFFI
  `workflow_graph_query_port_options`, Rustler `node_registry_query_port_options`,
  frontend TypeScript mirrors, and affected README/API notes. Do not add a
  context field in only one binding or rely on untyped JSON passthrough for
  cross-language behavior.
- [x] Expose registered backend port-option providers through backend-owned
  node definitions as append-only `options_provider` references. The reference
  identifies the provider query target only; it must not carry option rows,
  Pumas package facts, scheduler decisions, worker envelopes, local paths, or
  execution policy.
- [x] Model provider context references as validated typed values at Rust
  boundaries. Public constructors/projection helpers must return structured
  errors or bounded diagnostics, not `Result<T, String>` or string-matched
  control flow.
- [x] Model denoising scheduler option ids as validated typed values at Rust
  boundaries when the provider is added. Use stable primitive option ids for
  selected values and reserve display labels/descriptions for presentation
  only.
- [x] Add backend-owned port options for `llm-inference.denoising_scheduler`
  so graph editors can present valid denoising/sampling schedulers from
  model/package/runtime facts. The frontend must not hardcode the allowed
  denoising scheduler list.
  - 2026-05-17: `workflow-nodes` now registers a backend-owned
    `PortOptionsProvider` for `llm-inference.denoising_scheduler` behind the
    model-library feature. It resolves selected-model context to Pumas package
    facts, projects recognized Diffusers scheduler components to stable
    primitive option ids, marks explicit scheduler rows unavailable until the
    planner supports executable overrides, and returns typed metadata
    diagnostics when selected-model context or Diffusers evidence is missing.
- [x] Wire provider-backed `selection-input` behavior so fact-dependent options
  are displayed without silently writing executable defaults into graph data.
  Missing or stale selected values should render as unset/stale UI state and
  let the planner apply default policy or return typed diagnostics.
- [x] Keep provider-backed selection UI declarative, accessible, and
  event-driven. It must not introduce polling loops, optimistic backend-owned
  state updates, manual DOM mutation, or graph-canvas gesture conflicts. Async
  option loads must discard stale responses when model/runtime context changes
  before the previous query returns.
- [x] Add node-test coverage or the existing project-approved equivalent for
  provider-backed selection accessible-name, native keyboard selection, and
  graph gesture containment after real backend provider metadata is exposed
  through node definitions. The current slice keeps the control labelled,
  native-select based, and `nodrag`/`nopan`/`nowheel`, but does not add a
  browser-mounted component test platform.
  - 2026-05-17: added Node-tested selection-input control presenter coverage
    for provider-backed accessible names, native select keyboard behavior,
    empty-provider disabled state, and graph gesture containment classes. The
    Svelte component now consumes the tested control model without adding a
    DOM/browser component test platform.
- [x] Add context-keyed cache/invalidation for provider-backed option queries
  before reusing backend port options for model/runtime-dependent traits. The
  cache key must include node type, port id, provider context, and package-facts
  cursor/runtime facts; the current Pumas model-list cache is not sufficient
  for denoising scheduler choices.
- [x] Keep `PortOptionsProvider` generic for other selectable inference traits
  whose valid values are backend/model/runtime dependent. Promote a trait to a
  first-class port/provider only when it is user-facing, fact-dependent, and
  diagnostics/reproducibility relevant; keep long-tail model knobs in
  `expand-settings`.
- [x] Keep `expand-settings` out of the canonical denoising scheduler path.
  It may continue to expose long-tail model/runtime knobs, but first-class
  image traits with reproducibility or diagnostic impact must use typed ports,
  backend validation, stable option ids, and planner diagnostics instead of
  display-label-to-value normalization.
  - 2026-05-17: the denoising scheduler provider was added as a normal
    `PortOptionsProvider` without changing the generic provider trait or
    routing through `expand-settings`.
- [x] Implement planned PyTorch image generation using the existing Python
  worker diffusion load/generate path and typed worker request/response
  envelopes. Completed through `PyTorchBackend::generate_image_from_plan`;
  the raw `generate_image` path remains explicitly unsupported because image
  execution now requires a validated `ImageGenerationExecutionPlan`.
- [x] Change the Python worker image path to consume the validated Rust plan
  fields. It must not decide pipeline family, scheduler, custom-code trust, or
  device fallback on its own.
- [x] Ensure the Python worker applies the validated `denoising_scheduler`
  decision or rejects unsupported explicit scheduler changes before returning
  success. Worker metadata must not report a scheduler value that was accepted
  by Rust but ignored by the worker.
- [x] Version or explicitly shape-check the Python worker image-generation
  envelope so Rust rejects unknown, missing, or incompatible worker request and
  response fields before trusting them.
- [x] Ensure worker calls use the existing process lifecycle owner and do not
  introduce untracked `tokio::spawn`, unbounded queues, or blocking work while
  holding service locks.
- [x] Put PyTorch image-generation worker request/response translation in a
  focused helper or submodule so the backend facade remains readable and
  testable.
- [x] Add model-family adapters inside the PyTorch/diffusers bridge. Adapter
  selection must use package facts such as pipeline family/class and component
  layout, not model id or display-name string matching.
  - 2026-05-17: added an internal
    `image_generation_family_adapters` resolver. The current adapter supports
    Stable Diffusion from Pumas Diffusers family/component facts and does not
    inspect model ids or display names.
- [x] Implement family requirements as explicit table data or small typed
  requirement structs, not as scattered `match` arms mixed with worker calls.
- [x] Implement component-role extraction from Pumas facts before adapter
  selection. Extraction maps `ProcessorComponentFacts` and Transformers
  evidence into `ImageGenerationComponentRole` values and diagnostics.
  Completed through Pumas-provided typed Diffusers component facts:
  family adapter validation now reads `DiffusersComponentRole` values from
  `ResolvedModelPackageFacts`, reports exact role paths, and rejects missing
  or ambiguous required roles before worker dispatch.
- [x] Implement adapter selection in two stages: first resolve family/variant
  from Pumas facts, then validate that the resolved family requirements match
  the request and package components.
- [x] Add a missing-facts report that lists the exact absent or ambiguous facts
  needed from Pumas for a family to be supported.
  - 2026-05-17: adapter resolution first resolves one concrete family from
    `package_facts.diffusers.family_evidence`, then the resolved adapter
    validates required component roles. Adapter diagnostics preserve exact
    `package_facts.diffusers.family_evidence` and
    `package_facts.diffusers.components.<role>` paths before the planner maps
    them into public planner diagnostics.
- [x] Use Transformers task/config/generation names as the external naming
  reference where they fit Pantograph's Rust contracts.
  - 2026-05-17: reference review confirmed `model_type`, `architectures`,
    `auto_map`, `trust_remote_code`, and generation-config conventions remain
    the right Pumas-facing evidence names. Pantograph keeps those as input
    facts for Rust-owned planner contracts rather than exposing Transformers
    Python APIs through graph nodes or scheduler decisions.
- [x] Use ComfyUI and InvokeAI as reference implementations for diffusion
  family taxonomy and required-component validation, including SD/SDXL, FLUX,
  FLUX.2, qwen-image, lumina-image, glm-image, and z-image patterns.
  - 2026-05-17: reference review found ComfyUI family listings and detector
    evidence for SD/SDXL, FLUX, FLUX.2, Lumina Image, Qwen Image, and Z-Image;
    ComfyUI encoder/tokenizer selection evidence for Lumina, Qwen Image, FLUX,
    FLUX.2, and Z-Image; and InvokeAI taxonomy/variant notes for SD, SDXL,
    FLUX, FLUX.2 Klein variants, and Z-Image. GLM Image remains Pumas-owned
    family evidence until local executable references are stronger.
- [x] Keep reference-derived logic behind Pantograph-owned Rust planner types
  and Pumas package facts; do not mirror ComfyUI/InvokeAI graph/runtime
  architecture.
  - 2026-05-17: reference-derived findings are recorded as requirements for
    Pumas package facts, Pantograph family labels/variants, typed component
    roles, provider-backed scheduler option ids, and planner diagnostics only.
    Pantograph must not copy ComfyUI's node graph, state-dict scanner,
    sampler runtime, or InvokeAI's model manager/invocation/UI architecture.
- [x] Validate denoising scheduler, dimensions, negative prompt, guidance
  scale, image count, dtype, device policy, dependency environment, and
  required package components before calling the worker.
  - 2026-05-17 dtype validation slice: the image-generation planner now
    validates explicit Transformers `torch_dtype`/`dtype` package evidence
    against family-owned PyTorch/Diffusers dtype rules before producing a
    worker plan. Supported Stable Diffusion evidence accepts Transformers
    float32, float16, and bfloat16 spellings; unsupported explicit dtypes such
    as int8 produce typed `UnsupportedDtype` planner diagnostics. Missing
    dtype evidence remains non-blocking until Pumas provides it because dtype
    is not yet a graph/runtime override in this slice; no unsupported dtype is
    silently coerced or passed around as a fallback.
- [x] Validate option support per family. For example, guidance scale,
  negative prompt, image count, denoising scheduler, dtype, and dimensions
  must be accepted, ignored, or rejected by typed family rules before
  execution.
- [x] Ensure gateway-level image option diagnostics are reconciled with planner
  diagnostics so users see one authoritative option-support answer.
- [x] Validate component source ambiguity per family. Families such as Z-Image
  and FLUX.2 must reject ambiguous VAE/text-encoder sources instead of trying
  to assemble components heuristically.
- [ ] Use checked arithmetic for dimensions, image counts, estimated memory,
  and artifact size calculations. Reject overflow or unacceptable estimates
  through typed planner diagnostics.
  - 2026-05-17: stream artifact byte-range and implicit sequence progression
    now use checked arithmetic and fail closed before appending media chunks
    when overflow would occur. This removes silent clamping for retained
    artifact range metadata. Remaining scope for this checklist item:
    scheduler/runtime memory estimates and any other planner-side artifact
    size limits must be verified before marking complete.
  - 2026-05-17 memory policy planning: remaining memory work must be
    implemented as a cross-runtime scheduler/inference contract, not a
    PyTorch-image-only calculation. Inference owns request-local checked
    arithmetic and typed estimate diagnostics; Pumas owns package/component
    facts; backend/runtime providers expose readiness and estimate facts; the
    scheduler owns memory admission, reservations, retries, rescheduling,
    termination policy, and history-backed ranking. Estimate DTOs must
    distinguish `available`, `not_available`, `not_implemented`,
    `insufficient_facts`, `overflow`, `unsupported_family`, and
    `unsupported_runtime` instead of using sentinel values. History-backed
    timing and memory ranking remains gated until every valid runtime
    candidate for the same workflow/model/runtime key has at least five
    completed runs.
  - 2026-05-17 inference resource-estimate contract slice: added
    inference-owned resource estimate kinds, explicit estimate states, typed
    diagnostics, and constructor validation. The contract can represent
    available byte estimates and non-available states such as overflow,
    insufficient facts, unsupported family/runtime, not available, and not
    implemented without using `0`, `None`, saturation, or confidence strings
    as control-flow sentinels. Remaining scope: project the contract through
    runtime-registry/workflow technical-fit, move the existing output RGBA
    estimate into this shape, add side-effect-free family/runtime calculators,
    and wire scheduler admission/history.
  - 2026-05-17 output RGBA estimate migration slice: replaced
    `ImageGenerationExecutionPlan.estimated_output_rgba_bytes` with typed
    `resource_estimates`. Successful plans now carry an `output_rgba_bytes`
    `available` estimate when width/height/count are known and an
    `insufficient_facts` estimate when dimensions are omitted. Overflow still
    rejects planning with `ResourceEstimateOverflow` and also records the
    shared estimate diagnostic/state instead of returning a saturated or
    omitted numeric estimate.
  - 2026-05-17 technical-fit estimate replan decision: use replacement option
    3 for the next shared-contract boundary. Typed resource estimate records
    must replace runtime-registry/workflow technical-fit optional MB fields and
    `estimation_confidence` strings instead of being adapted into them or
    added beside them. The next implementation stages must remove old
    technical-fit estimate fields from each boundary they touch, update tests
    and fixtures in the same slice, and keep scheduler ranking unchanged until
    the typed candidate/admission slices are ready.
  - 2026-05-18 runtime-registry technical-fit estimate contract slice:
    runtime-registry candidates and decisions now use `resource_estimates`
    typed records with explicit `available`, `insufficient_facts`, `overflow`,
    unavailable, unimplemented, and unsupported states plus bounded
    diagnostics. The old registry `resource_estimate` optional MB-field shape
    was removed; workflow-service and embedded-runtime projection remain before
    the shared technical-fit estimate replacement is complete.
  - 2026-05-18 workflow/embedded technical-fit estimate projection slice:
    workflow-service mirrors the typed estimate record contract and
    embedded-runtime projects runtime-registry estimates into workflow
    decisions without optional MB-field compatibility. Runtime-requirement
    peak RAM/VRAM inputs now become byte-valued typed estimates through
    checked arithmetic, and overflow produces a typed estimate diagnostic.
    Candidate admission and history-backed scheduler ranking remain later
    memory-policy slices.
  - 2026-05-18 scheduler pressure contract slice: runtime-registry removed
    peak RAM/VRAM MB estimates from `RuntimeTechnicalFitResourcePressure`.
    Budget-pressure ranking activation now reads typed candidate
    `peak_vram_bytes`/`peak_ram_bytes` estimates and current loaded-runtime
    pressure, leaving memory admission/reservation failure diagnostics for the
    next memory-policy slice.
  - 2026-05-18 memory admission re-plan boundary: do not implement the next
    admission slice by translating typed estimates back into the old
    runtime-registry MB reservation fields. Runtime-registry admission,
    reservation requirements, admission budgets, failure payloads, runtime
    snapshots, and technical-fit selector inputs must move to one typed
    byte-valued estimate/claim path. The first code slice should replace the
    reservation/admission contract in runtime-registry; later slices can expose
    reduced budget/claim facts in snapshots and make pure technical-fit
    selection reject over-budget candidates before runtime selection.
  - 2026-05-18 typed admission/reservation contract slice: runtime-registry
    admission now uses typed byte-valued budget rows and reservation claims,
    and admission failures report bytes instead of MB fields. Embedded-runtime
    projects current workflow MiB estimates into typed runtime-registry byte
    claims with checked arithmetic until the upstream workflow requirement
    contract is replaced. Remaining memory-policy work: expose reduced budget
    and active claim facts in runtime snapshots, then make technical-fit
    selection reject over-budget candidates through typed diagnostics.
  - 2026-05-18 runtime snapshot admission-facts slice: runtime-registry
    snapshots now carry typed admission budget rows and per-active-reservation
    byte claims as reduced immutable scheduler facts. They do not include full
    workflow metadata, Pumas facts, or worker envelopes. Remaining
    memory-policy work: make technical-fit selection consume these facts and
    reject over-budget candidates before runtime selection.
  - 2026-05-18 technical-fit memory admission slice: runtime-registry's pure
    selector now consumes snapshot admission budgets, active reservation byte
    claims, and candidate `peak_ram_bytes`/`peak_vram_bytes` estimates before
    selection. Automatic selection rejects over-budget candidates instead of
    ranking them, explicit overrides surface typed `ResourceBudgetExceeded`,
    `ResourceAccountingOverflow`, or `ResourceBudgetUnderflow` diagnostics,
    and the selector does not call mutable admission/reservation APIs or
    translate typed estimates back into legacy MB fields. Verification:
    `cargo test -p pantograph-runtime-registry resource_ --lib`,
    `cargo test -p pantograph-runtime-registry --lib`,
    `cargo test -p pantograph-runtime-registry --test technical_fit_contract`,
    `cargo fmt --all -- --check`, and `git diff --check`. Remaining
    memory-policy follow-up: replace embedded-runtime's temporary workflow MiB
    projection with typed workflow resource-requirement contracts and then wire
    scheduler history/diagnostics-ledger policy over the same reduced facts.
  - 2026-05-18 workflow runtime-requirement contract replacement:
    workflow-service `WorkflowRuntimeRequirements` now carries typed
    `resource_estimates` records instead of legacy `estimated_*_mb` fields and
    `estimation_confidence`. Embedded-runtime projects those typed workflow
    estimates directly into runtime-registry technical-fit estimates and
    reservation byte claims, so no boundary translates estimates through MiB
    fields. Default capability estimation emits available peak RAM/VRAM byte
    estimates only from complete model-size facts and otherwise emits typed
    `insufficient_facts` estimates. Verification: `cargo test -p
    pantograph-workflow-service capabilities::tests::memory_estimate --lib`,
    `cargo test -p pantograph-workflow-service
    workflow_technical_fit_resource_estimates_use_typed_states_without_legacy_mb_fields
    --lib`, `cargo test -p pantograph-workflow-service --test contract`,
    `cargo test -p pantograph-embedded-runtime host_helper --lib`, `cargo test
    -p pantograph-embedded-runtime runtime_requirements_resource_estimates
    --lib`, `cargo check -p pantograph-embedded-runtime`, `cargo fmt --all --
    --check`, and `git diff --check`. Follow-up cleanup completed
    2026-05-18: the stale `text-input` fixture no longer carries incidental
    `backend_key` metadata, and `cargo test -p pantograph-workflow-service
    workflow_capabilities --lib` now verifies that no hard backend requirement
    is inferred without the canonical typed runtime input path.
  - 2026-05-18 diagnostics-ledger runtime-history slice:
    run-terminal payloads now carry optional typed `resource_observation`
    facts for observed peak RAM, observed peak VRAM, and explicit
    out-of-memory failure kind. Run-list/run-detail projections and
    runtime-selection history summaries persist those facts for later
    scheduler memory ranking without parsing terminal error strings. This
    slice does not turn on history-backed ranking or infer missing resource
    observations.
    Verification: `cargo test -p pantograph-diagnostics-ledger
    runtime_selection_history --lib`, `cargo test -p
    pantograph-diagnostics-ledger
    existing_v24_schema_adds_scheduler_learning_output_and_memory_projection_columns
    --lib`, `cargo check -p pantograph-workflow-service`, and `cargo test -p
    pantograph-diagnostics-ledger --lib`, `cargo test -p
    pantograph-workflow-service diagnostics --lib`, `cargo fmt --all --
    --check`, and `git diff --check`.
  - 2026-05-18 re-plan boundary: actual producer wiring must not infer
    terminal memory/OOM observations from error strings or artifact-store
    memory policy. The codebase needs a canonical typed
    execution-resource-observation contract at the inference/node execution
    boundary before runtime backends can populate observed peak RAM, observed
    peak VRAM, and explicit OOM facts for workflow terminal events.
  - 2026-05-18 resource-monitor design: resolve this boundary with an
    inference-owned, platform-neutral observation DTO plus resource-monitor
    modules gated by thin OS-specific `cfg()` files. Linux, macOS, Windows,
    unsupported-target, PyTorch worker, and managed-runtime producers must
    emit typed source/availability facts. Node-engine and embedded-runtime only
    forward/project those facts; workflow-service records them on terminal
    events; scheduler consumes reduced history later. Do not add scheduler OS
    probes, workflow terminal string matching, artifact-cache memory reuse, or
    image-only PyTorch ownership of the shared telemetry contract.
  - 2026-05-18 blast-radius review update: before wiring telemetry, extract a
    small inference lifecycle event builder/context so resource fields are not
    added through the existing repeated `gateway.rs` lifecycle constructor
    pattern. Use lifecycle events as the first transport rather than adding
    optional telemetry fields to every `InferenceExecutionResult` variant.
    Add PyTorch telemetry on the generic worker success/failure envelope, not
    image-only metadata. Prefer the existing `sysinfo` dependency for initial
    process RSS monitoring before direct `/proc`, Windows API, or macOS native
    wrappers. Confine or retire existing adapter-local OOM string detection by
    converting it immediately to typed memory-failure facts; never expose it as
    workflow terminal string matching. Extend runtime-registry candidate
    history with observed memory/OOM fields before scheduler history ranking
    consumes those persisted diagnostics-ledger facts.
  - 2026-05-18 standards iteration update: resource observation
    implementation slices must use correct-by-construction DTOs with typed
    bounded source/availability facts, deterministic de-duplication, and no
    raw process output or local paths in lifecycle diagnostics. Process-RSS
    sampling must have named interval/limit constants, a single lifecycle
    owner, tracked cancellation/`JoinHandle` cleanup, and focused tests for
    finish/cancel/drop behavior. New platform/API dependencies are forbidden
    unless the existing `sysinfo` path is proven insufficient and dependency
    ownership, transitive cost, feature gating, and no-default/all-features
    checks are recorded. Any slice touching existing threshold-crossing files
    such as `gateway.rs`, `server.rs`, `embedding_runtime.rs`,
    `backend/llamacpp_support.rs`, or runtime-registry technical-fit modules
    must extract focused helpers or record an explicit decomposition review;
    telemetry work must not add new `too_many_arguments` allowances.
  - 2026-05-18 post-review blast-radius update: diagnostic payload projection
    is a first-class slice before terminal payload wiring. Extend
    `InferenceExecutionDiagnosticObservedPayload` and the embedded-runtime
    persistability gate before attaching observations to
    `RunTerminalPayload.resource_observation`, so resource telemetry is not
    silently dropped from inference diagnostics. The lifecycle builder/context
    migration must cover direct constructors in gateway, node-engine,
    embedded-runtime, and tests before telemetry fields are added. Resource
    monitoring must use a tracked guard/cancellation owner and must not copy
    the current untracked process-spawner `tokio::spawn` pattern. Legacy OOM
    cleanup targets are explicit: `inference::server`,
    `inference::embedding_runtime`, and `backend::llamacpp_support`. Extract a
    Python worker response helper before adding telemetry to operation
    envelopes, and keep runtime-registry memory/OOM history projection ahead
    of scheduler ranking activation.
  - 2026-05-18 resource-observation DTO slice: `inference` now owns the shared
    `InferenceExecutionResourceObservation` contract and related typed metric,
    source, availability, and memory-failure enums. The contract validates and
    bounds producer facts without wiring image-generation execution,
    lifecycle events, terminal payloads, scheduler ranking, or legacy OOM
    parsing.
  - 2026-05-18 lifecycle builder slice: lifecycle event construction now goes
    through `InferenceRequestLifecycleEvent::builder` or
    `InferenceRequestLifecycleEventContext` in gateway, node-engine emitters,
    and affected tests. This prepares the event contract for future resource
    telemetry fields without adding another repeated field-list constructor.
  - 2026-05-18 diagnostic resource-observation projection slice:
    `InferenceRequestLifecycleEvent` now carries optional typed resource
    observations, and `InferenceExecutionDiagnosticObservedPayload` persists a
    bounded diagnostic summary with peak RAM, peak VRAM, memory failure,
    source, and availability facts. This is diagnostic projection only; run
    terminal payload wiring, producer telemetry, resource monitoring, and
    scheduler ranking remain separate slices.
  - 2026-05-19 telemetry-scope re-plan update: implement the option-4
    architecture boundary now while enabling option-3 behavior first. Add an
    inference-owned `InferenceExecutionTelemetryScope` and cloneable recorder
    as the single typed ingress for backend/resource observations. The gateway
    creates the scope at the backend execution boundary, passes only the
    minimal recorder or a minimal `BackendExecutionContext` containing that
    recorder to backend code, drains/merges observations into the existing
    terminal lifecycle event, and remains the only owner of lifecycle event
    emission. Backend code must not emit lifecycle diagnostics directly,
    return telemetry through task-output metadata, parse workflow/Pumas facts
    for telemetry, use thread-local/global observation state, or preserve a
    parallel result-wrapper compatibility path. The first implementation
    should record PyTorch worker success/failure observations that are already
    present on the worker envelope, then fold process-RSS monitoring into the
    same scope so observed CUDA/MPS/CPU/process metrics merge through one
    canonical path. Live observation streaming, scheduler real-time feedback,
    participant identity for parallel runtimes/devices, and any new
    `ResourceObserved`/observation event kind remain later explicit contract
    slices; do not add them until the ledger/UI/scheduler consumption contract
    is designed.
  - 2026-05-20 telemetry scope foundation slice: added the inference-owned
    `InferenceExecutionTelemetryScope` and cloneable
    `InferenceExecutionTelemetryRecorder` as the canonical in-memory
    backend-to-gateway observation collector. The scope drains one terminal
    merged resource observation and does not emit lifecycle events, stream live
    observations, carry Pumas/workflow facts, use global/thread-local state, or
    place telemetry in task outputs. Focused tests cover empty drains,
    terminal one-shot drain behavior, and merge semantics for RAM, VRAM, and
    memory-failure observations. Verification passed: `cargo test -p
    inference telemetry --lib`. Remaining follow-up: wire gateway process-RSS
    monitoring and PyTorch worker success/failure resource observations
    through this scope instead of direct per-path lifecycle attachment.
  - 2026-05-20 gateway process-RSS telemetry-scope slice: migrated the
    gateway's planned image, generic typed non-streaming, and streaming
    process-RSS lifecycle producers to start a gateway-owned telemetry scope,
    record the existing process-RSS observation into the scope, and drain the
    merged terminal observation into the existing lifecycle event. This keeps
    lifecycle ownership in `gateway.rs` while removing direct per-path monitor
    attachment as the canonical producer path. No backend trait, task output,
    scheduler policy, live observation stream, or worker envelope behavior
    changed. Verification passed: `cargo test -p inference
    test_generate_image_from_planning_input_with_lifecycle_records_planned_decision
    --lib`, `cargo test -p inference
    test_execute_typed_text_reports_generation_option_diagnostics --lib`,
    `cargo test -p inference
    test_chat_completion_stream_with_lifecycle_records_completion --lib`, and
    `cargo test -p inference
    test_stream_typed_text_with_lifecycle_records_terminal_chunk_usage --lib`.
    Remaining follow-up: pass the telemetry recorder through the backend
    execution boundary so PyTorch worker success/failure resource observations
    reach the same lifecycle terminal summary.
  - 2026-05-20 planned image backend-native telemetry slice: replaced the
    planned image backend trait boundary with a minimal
    `BackendExecutionContext` containing only the telemetry recorder. Gateway
    creates the context from its execution telemetry scope for planned image
    lifecycle calls, and the PyTorch/Diffusers bridge records worker
    success/failure `resource_observation` facts into that recorder before
    returning task output or backend errors. The worker observations remain
    diagnostics only: they are not written into image metadata, emitted as
    backend-owned lifecycle events, passed through Pumas/workflow facts, or
    routed through a compatibility result wrapper. Focused coverage proves a
    backend-native CUDA VRAM observation merges with process-RSS lifecycle
    telemetry on the planned image backend execution terminal event. The
    backend README now documents that `BackendExecutionContext` is diagnostic
    recorder context only, not lifecycle or scheduler ownership transfer.
    Verification passed: `cargo test -p inference
    test_generate_image_from_planning_input_with_lifecycle_records_planned_decision
    --lib`, `cargo test -p inference --features backend-pytorch
    pytorch_image_generation --lib`, `cargo check -p inference`, and
    `cargo check -p inference --features backend-pytorch`. Remaining
    follow-up: failure-path OOM typing and managed-runtime structured
    telemetry still need separate producer slices.
  - 2026-05-20 PyTorch image OOM resource producer slice: the Python
    PyTorch/Diffusers worker now converts adapter-local PyTorch/CUDA
    out-of-memory signals into typed
    `resource_observation.memory_failure_kind = out_of_memory` facts on
    worker error envelopes. This confines string/classification handling to
    the PyTorch worker adapter and immediately emits typed diagnostics; gateway
    and workflow terminal paths still consume only typed resource observations
    and do not parse terminal error text. Verification passed: `cargo test -p
    inference --features backend-pytorch
    test_python_worker_generate_image_from_envelope_reports_oom_failure --lib`.
    Remaining follow-up: managed-runtime structured telemetry remains a
    separate producer slice.
  - 2026-05-20 managed-runtime telemetry re-plan boundary: stop before
    implementing the remaining managed-runtime producer work. The existing
    plan does not yet define whether the next slice should observe managed
    child-process RSS, runtime-native structured telemetry, or both; it also
    does not name the lifecycle owner for each managed runtime request/sidecar
    boundary. The next plan update must define the target process/API source,
    the handoff into `InferenceExecutionTelemetryScope`, typed unavailable
    states for runtimes without telemetry APIs, and the distinction between
    `os_process_rss` and `managed_runtime_telemetry` sources. Do not implement
    this by parsing managed-runtime logs outside the runtime adapter or by
    adding scheduler-owned probes.
  - 2026-05-20 managed-runtime telemetry planning decision: use the two-source
    model. Managed child-process RSS is the first implementation slice and
    must be owned by the runtime lifecycle boundary that has the concrete
    `ProcessHandle::pid()`; it reports only host RAM as `os_process_rss` and
    merges through `InferenceExecutionTelemetryScope`. Runtime-native
    telemetry is a second provider contract that reports structured
    runtime/API metrics as `managed_runtime_telemetry` or typed unavailable
    states. Runtime-native telemetry must not be simulated from child-process
    RSS, and child-process RSS must not be mislabeled as runtime-native
    telemetry. Scheduler and diagnostics-ledger consume the projected typed
    observations only; they do not run probes or parse runtime logs.
- [x] Validate Pumas-provided paths and artifact entry paths against the
  approved Pumas/model roots before worker execution.
  - 2026-05-17: image-generation execution now accepts only validated
    root-relative Pumas artifact entry paths before worker-envelope
    construction and rejects local/absolute/traversing/URI-shaped artifact
    paths with planner diagnostics. Already root-validated resolved absolute
    paths remain a later Pumas root-inventory extension point, not an
    executable fallback.
  - 2026-05-17: image-generation execution now also requires a
    Pumas-resolved artifact load target for worker loading. The target carries
    Pumas-approved `local_load_path`, storage kind, validation state, artifact
    kind, and load-path kind. The planner rejects mismatched model refs,
    non-Diffusers artifact kinds, non-directory load targets, invalid
    validation states, and empty/control-character load paths before worker
    dispatch. Pantograph still does not join Pumas paths or synthesize local
    load targets.
- [x] Return a terminal planning/readiness diagnostic when validation fails.
  Do not try alternate backends, generic Diffusers loading, default schedulers,
  CPU fallback, or alternate dependency environments.
  Completed through the side-effect-free planner/gateway boundary: backend,
  model-ref, artifact path/load-target, task, package contract, family,
  component, option, resource-estimate, and dependency-readiness failures
  produce typed planner diagnostics and prevent worker dispatch.
- [x] Update PyTorch capability facts so image generation is advertised only
  when the PyTorch/diffusers execution path is actually available.
- [x] Ensure PyTorch worker loading uses Pumas-resolved diffusers-directory
  package facts.
  - 2026-05-17: the PyTorch image worker envelope now carries
    `artifact_load_target` instead of `artifact_entry_path`; the Python worker
    validates the target and calls `load_diffusion_model` with
    `artifact_load_target.local_load_path` before generation. Node-engine
    planned image execution requires `resolved_model_artifact_load_target`
    alongside package facts, and `puma-lib` asks Pumas selector access for
    `resolve_model_artifact_load_target` when full package facts are available.
    2026-05-17 smoke-path update: the Tiny SD Turbo-style imported Diffusers
    fixture now carries a selected artifact id and resolves a Pumas-approved
    external-reference directory load target through Puma-Lib execution.
    Pantograph still does not synthesize a fallback target when Pumas reports
    a selected artifact as missing or ambiguous.
- [x] Ensure dependency/runtime readiness reports missing `diffusers`,
  `transformers`, `accelerate`, `torch`, or Pillow as explicit readiness
  diagnostics.
  Completed through inference-owned PyTorch/Diffusers package declarations,
  embedded-runtime package readiness provider/probe facts, scheduler candidate
  filtering diagnostics, selected-decision readiness proof, and the planner
  missing/unavailable dependency proof gate.
- [x] Retain final generated image output through ArtifactStore and IO
  projections. Completed 2026-05-17: embedded runtime node I/O projection now
  decodes valid base64 on the canonical `image` port into a retained
  `image/png` image artifact body and keeps non-image strings as text. This is
  output retention only; it does not synthesize worker inputs, package facts,
  load targets, runtime choices, or backend fallbacks.
- [x] Ensure `image` and `results` outputs do not persist duplicate full image
  base64 bodies after artifact conversion.
- [x] Change node-engine image-generation output shaping so large generated
  bodies are not duplicated in `image` and `results` before artifact
  conversion. `results` should contain descriptors/metadata or compact
  summaries once artifacts exist.
- [x] Add a small model smoke path using Tiny SD Turbo or another bounded
  fixture before attempting Juggernaut.
  - 2026-05-17: added a Puma-Lib Tiny SD Turbo-style imported Diffusers
    fixture smoke path. The fixture declares the selected `diffusers` artifact
    id, Pumas rebuilds/validates the external-reference model, Puma-Lib emits
    full package facts, and Puma-Lib also emits a
    `resolved_model_artifact_load_target` with `artifact_kind =
    diffusers_bundle`, `load_path_kind = directory`, `storage_kind =
    external_reference`, `validation_state = valid`, and the Pumas model ref's
    selected artifact id. This is a bounded fixture smoke path, not a
    Juggernaut run and not a synthesized Pantograph path.
- [x] Add model-family fixtures or table-driven tests for Pumas facts shaped
  like z-image turbo, qwen-image, lumina-image, glm-image, and FLUX.2 where
  available. These tests should validate routing/request construction and may
  use mocked generation rather than loading large models.
  - 2026-05-17: added table-driven planner coverage using mocked Pumas
    Diffusers family evidence for SDXL, FLUX.2, Qwen Image, Lumina Image,
    GLM Image, and Z-Image. The slice proves each currently unsupported family
    fails with typed `UnsupportedFamily` diagnostics and does not fall back to
    generic Diffusers or Stable Diffusion worker execution.

**Verification:**

- Node-engine unit tests cover image-generation request construction from
  canonical graph inputs.
- Runtime selection test proves diffusion package facts do not select Candle
  and `diffusers` resolves to PyTorch execution in this slice.
- Code search verifies no current execution selector treats `diffusers` as a
  separately registered backend key unless it is only exposing capability or
  dependency metadata.
- Mapping tests prove dependency preflight, technical fit, runtime preflight,
  and gateway execution all agree on the same task/artifact-aware execution
  backend decision while preserving `diffusers` display/dependency facts.
- Execution-plan identity tests prove selected model refs are normalized once,
  reject malformed/local-path values, and preserve already-prefixed Pumas refs
  without producing `pumas://models/pumas://models/...`.
- Execution-plan identity tests exercise the public validated constructor and
  projection boundary, proving raw selected strings are not repaired in
  embedded-runtime or node-engine after the owner-boundary parse.
- Planned image execution tests prove scheduler-selected model refs and
  resolved package-facts model refs agree before worker planning. Mismatches
  and missing selected model refs for image-generation decisions must produce
  typed diagnostics and no worker dispatch.
- Normalization tests prove package/dependency labels such as `diffusers` are
  not reused as scheduler-selected execution backend keys.
- Planner tests prove unsupported pipeline family, missing component facts,
  incompatible denoising scheduler, invalid dimensions, unsupported options,
  unsupported explicit Transformers dtype evidence, unavailable dependency
  environment, unavailable device, and unacceptable resource estimates fail
  with diagnostics and no fallback attempt.
- Planner tests cover component-role extraction from Pumas facts and reject
  missing or ambiguous family evidence.
- Planner tests cover generation-default merge order: request value,
  model-provided default, then family default only when explicitly allowed.
- Planner tests cover missing-facts diagnostics with exact field paths and
  expected evidence for insufficient Pumas facts.
- Family requirement tests cover accepted/rejected options for SD/SDXL, FLUX,
  FLUX.2, Qwen Image, Lumina Image, GLM Image, and Z-Image.
- Port option tests prove `llm-inference.denoising_scheduler` choices are
  produced by backend/model/runtime facts, use stable ids rather than display
  labels, and do not mutate graph data when the current value is absent or
  stale.
- Port option context tests prove fact-aware option queries require typed
  context, reject insufficient context with diagnostics, and do not transport
  full Pumas package facts, local paths, graph payloads, worker envelopes, or
  scheduler decisions through frontend state.
- Binding contract tests prove the port-option context serde shape is preserved
  across node-engine, Tauri, UniFFI, Rustler, and frontend TypeScript mirrors in
  the same slice.
- Selection-input tests prove provider-backed options render unset/stale values
  without auto-selecting a default or first option into graph data.
- Selection-input accessibility and interaction tests prove the provider-backed
  control has an accessible name, keyboard selection behavior, stale-value
  presentation, and does not let graph pan/drag gestures corrupt the embedded
  control interaction.
- Frontend async tests prove stale provider responses are discarded when the
  selected model, package-facts cursor, or runtime context changes before a
  prior option query resolves.
- Provider cache tests prove denoising scheduler options are keyed by node type,
  port id, selected model/package-facts cursor, and backend/runtime context so
  model changes cannot reuse stale option lists.
- 2026-05-17 denoising scheduler provider verification passed: `cargo fmt
  --manifest-path crates/workflow-nodes/Cargo.toml`, `cargo test -p
  workflow-nodes --features model-library inference_denoising_options --lib`,
  `cargo test -p workflow-nodes --features model-library
  builtin_contracts_preserve_registered_port_options_provider_refs --lib`,
  `cargo check -p workflow-nodes`, `cargo check -p workflow-nodes --features
  model-library`, and `cargo fmt --manifest-path
  crates/workflow-nodes/Cargo.toml -- --check`.
  - Discovered issue/follow-up: the current Pumas imported Diffusers fixture
    can resolve full package facts without `diffusers` evidence, so the
    provider returns a `missing_diffusers_evidence` metadata diagnostic instead
    of manufacturing selectable rows. Later Pumas/library work should ensure
    imported Diffusers bundles expose scheduler/component evidence through the
    full package-facts path and, for shared/local-client access, expose the
    same facts without requiring Pantograph to infer them from paths or
    summaries.
- Planner tests prove overflow-prone dimensions/counts/resource estimates are
  rejected without allocation or worker calls.
- Path validation tests prove worker execution rejects model/package paths
  outside allowed Pumas roots.
- PyTorch backend test covers diffusers load/generate envelope shaping.
- Python worker bridge test proves the worker receives a validated plan and
  does not enable custom-code trust or fallback behavior independently.
- Worker envelope contract test proves Rust and Python agree on the
  image-generation request/response shape, version marker,
  error fields, artifact descriptor fields, and unsupported-field behavior.
- Async/lifecycle test or review proves worker execution is owned by the
  existing backend lifecycle and does not add untracked tasks, unbounded queues,
  or blocking critical sections.
- Vertical smoke test generates or mock-generates one image and verifies a
  retained image artifact descriptor is produced.
- Table-driven model-family tests prove image-generation routing is not
  hardcoded to Tiny SD Turbo or Juggernaut.
- Reference-guidance notes identify which Transformers naming/config semantics
  and ComfyUI/InvokeAI family patterns informed the planner and adapters.
- Artifact test verifies generated image retention stores one media body and
  projects only descriptors/metadata instead of duplicate base64 payloads.

**Status:** In progress. Planner contract, Rust worker envelope,
planner-to-worker translation, Python image-envelope shape validation, planned
PyTorch image helper, and the planned gateway/backend boundary are implemented.

2026-05-12 boundary check:

- Smallest useful vertical slice considered: start the image-generation planner
  contract with a Pumas Diffusers fixture and typed missing/ambiguous-facts
  diagnostics.
- Allowed write set considered for that future slice:
  `crates/inference/**`, focused fixtures under
  `crates/inference/tests/fixtures/inference_package_facts/`, affected
  inference tests, and this plan directory.
- No-fallback/no-legacy confirmation: the slice cannot start from name-derived
  family inference, local bridge conversion, or the currently pinned Pumas
  crate because Milestone 6 must consume the pinned Pumas producer contract.
- Current blocker: root `Cargo.toml` pins `pumas-library` to tag `v0.6.0`, and
  the locked source at commit `6d038ff8` exposes
  `PACKAGE_FACTS_CONTRACT_VERSION = 1`. Pantograph inference fixtures and DTOs
  now expect package-facts contract version 2 for Diffusers/image-generation
  facts. Starting execution planning against the local fixture alone would
  bypass the Pumas producer-fact completion gate in `04-milestones.md`.
- Required re-plan before implementation: choose and pin the Pumas release/tag
  or commit that contains package-facts contract version 2 plus the P6
  cross-repo fixture guarantees, then decide whether Pantograph advances the
  workspace `pumas-library` dependency in a dedicated dependency slice before
  planner implementation begins.

2026-05-12 dependency-boundary slice:

- Smallest useful vertical slice: pin Pantograph's workspace `pumas-library`
  dependency to Pumas commit `281a45a5bc604975ebd0d5e71d12adaa5a228382`, the
  producer revision recorded by the Pumas P6 fixture handoff and containing
  `PACKAGE_FACTS_CONTRACT_VERSION = 2`.
- Allowed write set: root `Cargo.toml`, `Cargo.lock`,
  `crates/pantograph-embedded-runtime/**`,
  `crates/pantograph-frontend-http-adapter/src/lib.rs`, and this plan
  directory.
- No-fallback/no-legacy confirmation: Pantograph now pins a concrete Pumas
  producer revision instead of consuming only vendored local fixtures or adding
  a compatibility bridge from package-facts contract version 1 to version 2.
- Verification discovered three pre-existing compile gaps from prior contract
  changes while validating direct Pumas consumers: embedded-runtime test
  snapshots were missing the new runtime warmup timing fields,
  embedded-runtime did not map the new runtime-registry resource accounting
  errors, and the frontend HTTP adapter did not explicitly handle graph error
  details in workflow error envelopes. These were fixed in this slice with
  explicit fields and explicit match arms rather than wildcard fallbacks.
- Broader verification note: `cargo test -p pantograph-embedded-runtime
  runtime_registry` remains outside this dependency slice and currently fails
  in two session/runtime tests because auto technical-fit correctly reports
  ambiguous equal-ranked candidates. The planner/test-fixture slice must supply
  canonical backend/device intent instead of restoring implicit selection.
- Remaining follow-up: begin the first Milestone 6 planner contract slice,
  consuming the pinned Pumas Diffusers facts and returning typed diagnostics
  for missing or ambiguous image-family facts.

2026-05-12 planner contract slice:

- Smallest useful vertical slice: add a side-effect-free inference planner
  contract that consumes `ImageGenerationRequest`, pinned Pumas Diffusers
  package facts, and a scheduler-owned backend/runtime/device decision, then
  returns either one PyTorch/Diffusers execution plan or typed diagnostics.
- Allowed write set: `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`,
  `crates/inference/src/lib.rs`, `crates/inference/src/README.md`, and this
  plan directory.
- No-fallback/no-legacy confirmation: the planner rejects non-PyTorch backend
  decisions, missing Diffusers facts, ambiguous family evidence, unsupported
  families, missing Stable Diffusion component roles, and invalid numeric
  request options. It does not treat `diffusers` as an executable backend alias
  and does not infer family from model names.
- Verification passed: `cargo test -p inference image_generation_planner`,
  `cargo check -p inference`, `cargo fmt --all -- --check`, and
  `git diff --check`.
- Remaining follow-up: wire the planner into PyTorch image generation and add
  broader family/default/dependency/path diagnostics before worker execution.

2026-05-15 planner checked-resource-estimate verification slice:

- Smallest useful vertical slice: add direct planner coverage proving
  overflow-prone image dimensions and image counts fail closed through typed
  diagnostics without allocation or wrapped byte estimates.
- Allowed write set: `crates/inference/src/image_generation_planner_tests.rs`
  and this plan directory.
- No-fallback/no-legacy confirmation: this only verifies the existing checked
  arithmetic boundary. It does not relax planner validation, choose alternate
  backends, infer defaults, or call worker execution after a resource estimate
  failure.
- Verification passed: `cargo test -p inference
  planner_rejects_resource_estimate_overflow_without_allocation --lib`, `cargo
  check -p inference`, `cargo fmt -p inference`, and `git diff --check`.
- Remaining follow-up: path validation, family-specific option rules, and
  generation default merge-order coverage still need separate focused planner
  slices before real worker execution is treated as complete.

2026-05-15 planner missing-component diagnostic slice:

- Smallest useful vertical slice: prove missing Diffusers component-role
  diagnostics include the exact absent family requirement field path.
- Allowed write set: `crates/inference/src/image_generation_planner_tests.rs`
  and this plan directory.
- No-fallback/no-legacy confirmation: this only strengthens diagnostic
  coverage for the existing fail-closed planner path. It does not infer missing
  components, choose alternate packages, call worker execution, or relax family
  requirements.
- Verification passed: `cargo test -p inference
  planner_reports_exact_missing_component_role_path --lib`, `cargo check -p
  inference`, `cargo fmt -p inference`, and `git diff --check`.
- Remaining follow-up: Pumas/model root validation needs a root-bearing
  planning or backend execution contract; do not replace that with ad hoc
  string inspection inside the side-effect-free planner.

2026-05-15 planner unsupported-family guardrail slice:

- Smallest useful vertical slice: add focused coverage for a single valid but
  unsupported image-generation family so the planner emits
  `UnsupportedFamily` instead of attempting generic Diffusers loading.
- Allowed write set: `crates/inference/src/image_generation_planner_tests.rs`
  and this plan directory.
- No-fallback/no-legacy confirmation: Flux family evidence is treated as
  recognized but unsupported in this slice. The planner does not reinterpret it
  as Stable Diffusion, strip the family requirement, or dispatch to worker
  execution.
- Verification passed: `cargo test -p inference
  planner_rejects_unsupported_single_family_without_generic_diffusers_fallback
  --lib`, `cargo check -p inference`, `cargo fmt -p inference`, and `git diff
  --check`.
- Remaining follow-up: support for FLUX, FLUX.2, Qwen Image, Lumina Image, GLM
  Image, Z-Image, and SDXL requires explicit family requirement tables,
  option-support rules, component ambiguity diagnostics, and fixtures before
  any of those families can execute.

2026-05-15 planner guidance-scale numeric guardrail slice:

- Smallest useful vertical slice: reject non-finite `guidance_scale` values in
  the side-effect-free image-generation planner before any worker envelope is
  built.
- Allowed write set: `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`, and this plan
  directory.
- No-fallback/no-legacy confirmation: invalid numeric options now fail with the
  existing typed `InvalidNumericOption` diagnostic. The slice does not add
  request defaults, backend fallback, scheduler overrides, raw device parsing,
  or worker-side recovery for invalid values.
- Verification passed: `cargo test -p inference
  planner_rejects_non_finite_guidance_scale --lib`, `cargo check -p
  inference`, `cargo fmt -p inference`, and `git diff --check`.

2026-05-15 denoising scheduler option-id planner boundary slice:

- Smallest useful vertical slice: add a validated
  `DenoisingSchedulerOptionId` Rust contract, parse explicit image-generation
  denoising scheduler request values before producing an
  `ImageGenerationExecutionPlan`, and serialize the planned field as
  `denoising_scheduler` instead of carrying untyped display/class strings in
  the planner result.
- Allowed write set: `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract.rs`,
  `crates/inference/src/backend/pytorch_image_generation_tests.rs`,
  `crates/inference/src/gateway_tests.rs`, `crates/inference/src/lib.rs`,
  `crates/inference/src/README.md`, and this plan directory.
- No-fallback/no-legacy confirmation: invalid scheduler option ids now reject
  planning through a typed diagnostic. The slice does not invent scheduler
  choices, hardcode provider option rows, derive scheduler ids from Diffusers
  class names, pass Pumas facts through frontend state, choose a backend or
  runtime, or let worker execution recover from invalid planner input.
- Verification passed: `cargo test -p inference image_generation_planner
  --lib`, `cargo test -p inference --features backend-pytorch
  test_generate_image_envelope_from_plan_validates_worker_request --lib`,
  `cargo test -p inference --features backend-pytorch pytorch_worker_image
  --lib`, `cargo check -p inference`, `cargo fmt -p inference -- --check`,
  and `git diff --check`.
- Remaining follow-up: the actual `llm-inference.denoising_scheduler`
  provider still needs a factual option source. Current pinned Pumas facts
  expose the package scheduler component/class, but not the full runtime-valid
  replacement scheduler option set, so the provider must wait for a compact
  Pumas/runtime-backed option contract or explicitly return only facts it can
  prove without hardcoded choices.

2026-05-15 denoising scheduler request/worker rename slice:

- Smallest useful vertical slice: complete the canonical request-field rename
  for image-generation denoising scheduler intent across Rust inference
  request DTOs, node-engine request construction, planner diagnostics, PyTorch
  image worker envelopes, Python image worker inputs, output metadata, and
  worker fixtures.
- Allowed write set: `crates/inference/src/types.rs`,
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
- No-fallback/no-legacy confirmation: the old image-generation sampling field
  name is no longer accepted by Rust request DTOs or PyTorch image worker
  payloads, and node-engine continues to ignore graph/API `scheduler` as a
  compatibility alias. Factual Diffusers/Pumas component-role strings named
  `scheduler` remain factual package evidence, not executable sampling intent.
- Verification passed: `cargo test -p inference image_generation --lib`,
  `cargo test -p inference --features backend-pytorch pytorch_worker_image
  --lib`, `cargo test -p node-engine --features inference-nodes
  image_generation --lib`, `cargo check -p inference`, `cargo check -p
  node-engine --features inference-nodes`, `cargo fmt -p inference -p
  node-engine -- --check`, code search for old image-generation request/worker
  `scheduler` field consumers, and `git diff --check`.
- Remaining follow-up: family-specific option-support tables still need to
  classify guidance scale, negative prompt, image count, scheduler override,
  dtype, and dimensions as accepted, ignored, or rejected per family.

2026-05-15 denoising scheduler worker guardrail slice:

- Smallest useful vertical slice: make the PyTorch image worker reject
  explicit `denoising_scheduler` values until scheduler swapping is actually
  implemented, so worker success metadata cannot claim a scheduler value that
  the worker ignored.
- Allowed write set: `crates/inference/torch/worker.py`,
  `crates/inference/src/backend/pytorch_worker_image_contract_tests.rs`,
  `crates/inference/src/backend/pytorch_worker_image_python_tests.rs`,
  `crates/inference/src/backend/pytorch_image_generation_tests.rs`,
  `crates/inference/tests/fixtures/pytorch_worker_contract/generate_image_request.json`,
  `crates/inference/tests/fixtures/pytorch_worker_contract/generate_image_response.json`,
  affected inference READMEs, and this plan directory.
- No-fallback/no-legacy confirmation: the worker does not fall back to the
  model default while reporting the explicit value as accepted. It returns the
  existing typed worker invalid-request envelope until a later slice can apply
  validated scheduler changes for supported families/runtimes.
- Verification passed: `cargo test -p inference --features backend-pytorch
  pytorch_worker_image --lib`, `cargo test -p inference --features
  backend-pytorch test_image_generation_result_from_worker_response_maps_images
  --lib`, `cargo check -p inference`, `cargo fmt -p inference -- --check`, and
  `git diff --check`.
- Follow-up at this point: planner/family option-support rules still needed to
  reject unsupported explicit `denoising_scheduler` values before worker
  dispatch when the selected family/runtime cannot apply them. The next slice
  resolves this for the current Stable Diffusion planner path.

2026-05-15 denoising scheduler planner option-support slice:

- Smallest useful vertical slice: reject explicit `denoising_scheduler` values
  in the side-effect-free image-generation planner until family/runtime support
  can actually apply scheduler changes.
- Allowed write set: `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract_tests.rs`,
  `crates/inference/src/README.md`, and this plan directory.
- No-fallback/no-legacy confirmation: the planner still validates primitive
  option id shape, but valid explicit scheduler ids now fail with
  `UnsupportedOption` before worker dispatch instead of being sent to Python
  for backend-local recovery or silent default behavior.
- Verification passed: `cargo test -p inference image_generation_planner
  --lib`, `cargo test -p inference --features backend-pytorch
  pytorch_worker_image --lib`, `cargo check -p inference`, `cargo fmt -p
  inference -- --check`, and `git diff --check`.
- Remaining follow-up: broader family option-support tables still need to
  classify guidance scale, negative prompt, image count, dtype, dimensions, and
  future supported scheduler overrides per image family.

2026-05-15 image-generation family rules table slice:

- Smallest useful vertical slice: move Stable Diffusion image-generation
  required components and option-support policy out of the main planner and
  into a focused `image_generation_family_rules` module with table-owned typed
  rules.
- Allowed write set: `crates/inference/src/image_generation_family_rules.rs`,
  `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`,
  `crates/inference/src/lib.rs`, `crates/inference/src/README.md`, and this
  plan directory.
- No-fallback/no-legacy confirmation: unsupported image families still produce
  `UnsupportedFamily`, and unsupported request traits such as explicit
  `denoising_scheduler`, img2img/inpaint fields, and opaque `extra_options`
  still produce typed `UnsupportedOption` diagnostics before worker dispatch.
  The slice does not infer family from model names, add generic Diffusers
  loading, or hardcode scheduler option values.
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

2026-05-15 milestone status reconciliation slice:

- Smallest useful vertical slice: reconcile already-implemented Milestone 6
  execution-boundary checklist items with the current codebase so remaining
  work is not obscured by stale unchecked tasks.
- Allowed write set: this plan directory only.
- No-fallback/no-legacy confirmation: this slice does not change executable
  code, introduce compatibility shims, or relax any planner/worker diagnostics.
  The verified code paths still require planned image-generation context,
  scheduler-owned backend/device decisions, Pumas package facts, and typed
  worker envelopes before execution.
- Verified completed boundaries:
  `ImageGenerationExecutionPlan` carries `DeviceResolutionDecision`;
  `image_generation_planner` remains synchronous and side-effect free;
  `InferenceGateway::generate_image_from_planning_input` is the planned
  gateway boundary; workflow-service owns `WorkflowExecutionPlan`;
  embedded-runtime projects workflow node decisions to inference
  `BackendExecutionDecision`; node-engine consumes
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

2026-05-15 planner unsupported-option guardrail slice:

- Smallest useful vertical slice: make the canonical image-generation planner
  reject request fields that the current text-to-image execution plan cannot
  carry (`init_image`, `mask_image`, `strength`, and non-null
  `extra_options`) instead of silently dropping them before worker dispatch.
- Allowed write set: `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`, and this plan
  directory.
- No-fallback/no-legacy confirmation: unsupported img2img/inpaint and opaque
  option fields now fail with typed `UnsupportedOption` planner diagnostics.
  The slice does not add generic Diffusers fallback behavior, compatibility
  shims, worker fields, frontend behavior, generated files, lockfiles,
  persisted schemas, workflow fixtures, or alternate execution paths.
- Verification passed: `cargo test -p inference
  planner_rejects_unsupported_image_options_without_silent_ignore --lib`,
  `cargo check -p inference`, `cargo fmt -p inference`, and
  `git diff --check`.
- Remaining follow-up: future img2img/inpaint and family-specific opaque
  option support needs explicit family option tables plus worker contract
  fields before these request fields can become executable.
- Discovered issue resolved by the 2026-05-15 denoising scheduler worker
  guardrail slice: explicit `denoising_scheduler` values now fail the Python
  worker instead of being deleted before generation and reported as success.
  Do not broaden scheduler override support until a focused option-rule slice
  decides whether each family accepts, rejects, or explicitly reports ignored
  scheduler overrides, then updates the worker contract and diagnostics
  accordingly.
- Naming decision: replace the overloaded image-generation `scheduler` option
  with `denoising_scheduler`. This value refers only to the Diffusers
  denoising/sampling scheduler selected inside image generation. It must not
  influence Pantograph workflow scheduling, queue order, runtime placement,
  device selection, warm runtime reuse, or retry policy.
- Graph/option-provider decision: `denoising_scheduler` is a first-class
  optional `llm-inference` image input. The graph may wire a `selection-input`
  into it, and the valid choices should come from a backend-owned
  `PortOptionsProvider` for that target port. `selection-input` remains a
  generic value passthrough; the image-generation planner remains the authority
  that validates whether the selected value is executable for the selected
  model family, package facts, runtime, and worker contract.
- Default decision: an omitted `denoising_scheduler` means use the selected
  model/pipeline default by explicit planner policy. A provided value must be
  validated and applied by the worker end to end. If the worker cannot apply
  it, planning must reject the request rather than accepting and silently
  ignoring it.
- Selection/expand-settings boundary: use backend-owned port options for
  selectable traits that are important, user-facing, fact-dependent, and
  diagnostics/reproducibility relevant. Keep long-tail model/runtime knobs in
  schema-driven `expand-settings`. The same provider mechanism may later serve
  dtype, adapters, tokenizer/chat-template variants, pooling strategies, audio
  voices, or other selectable traits, but each promoted trait needs its own
  stable ids, option diagnostics, and planner validation.
- Frontend anti-pattern to avoid: the current `SelectionInputNode` can
  auto-write the first/default allowed value into graph data when a target
  connection changes or the current value is missing/stale. Future
  `denoising_scheduler` work must not rely on silent frontend mutation for
  executable defaults. Defaults belong to planner policy, and stale selections
  must surface as unset/invalid state or typed planner diagnostics.

2026-05-12 worker image-envelope contract slice:

- Smallest useful vertical slice: add the Rust-side PyTorch worker
  image-generation request/response envelope DTOs, operation tag, JSON
  fixtures, and focused validation tests before wiring any worker execution.
- Allowed write set: `crates/inference/src/backend/pytorch_worker_contract.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract_tests.rs`,
  `crates/inference/src/backend/pytorch.rs`,
  `crates/inference/src/backend/README.md`,
  `crates/inference/tests/fixtures/pytorch_worker_contract/`, and this plan
  directory.
- No-fallback/no-legacy confirmation: the image worker request requires the
  Rust planner's selected Diffusers family, component roles, pipeline class,
  artifact entry path, and validated device id. The DTO rejects unknown payload
  fields such as `trust_remote_code`, so the Python worker cannot receive
  unplanned custom-code or fallback controls through this envelope.
- Verification passed: `cargo test -p inference --features backend-pytorch
  pytorch_worker_generate_image`, `cargo check -p inference --features
  backend-pytorch`, `cargo check -p inference`, `cargo fmt --all -- --check`,
  and `git diff --check`.
- Discovered standards debt: `backend/pytorch.rs` and `backend/pytorch_tests.rs`
  are already over the decomposition threshold. This slice kept new image
  request/response DTOs and tests in focused files; later execution wiring
  should avoid adding more policy or test bulk to those oversized files.
- Remaining follow-up: translate `ImageGenerationExecutionPlan` into the worker
  envelope and add Python-side shape validation before actual generation.

2026-05-12 planner-to-worker translation slice:

- Smallest useful vertical slice: translate a validated
  `ImageGenerationExecutionPlan` into `PyTorchGenerateImageRequest` and prove
  the resulting worker envelope validates.
- Allowed write set:
  `crates/inference/src/backend/pytorch_worker_image_contract.rs`,
  `crates/inference/src/backend/pytorch_worker_image_contract_tests.rs`, and
  this plan directory.
- No-fallback/no-legacy confirmation: translation copies the planner-selected
  model ref, artifact path, family, component roles, pipeline class, prompt
  options, and selected device id directly. It does not reinterpret backend
  hints, parse raw device strings, choose defaults, or infer family from model
  names.
- Verification passed: `cargo test -p inference --features backend-pytorch
  pytorch_worker_generate_image_request_maps_from_validated_plan`, `cargo test
  -p inference --features backend-pytorch pytorch_worker_generate_image`,
  `cargo check -p inference --features backend-pytorch`, `cargo fmt --all --
  --check`, and `git diff --check`.
- Remaining follow-up: add Python worker-side image envelope shape validation,
  then wire PyTorch backend image generation through the validated plan and
  envelope.

2026-05-12 Python image-envelope validation slice:

- Smallest useful vertical slice: add torch-free Python worker validation for
  the already-planned image-generation worker envelope without invoking
  Diffusers generation or wiring the Rust backend call path.
- Allowed write set: `crates/inference/torch/worker_image_contract.py`,
  `crates/inference/torch/README.md`,
  `crates/inference/src/backend/pytorch_worker_image_contract_tests.rs`, and
  this plan directory.
- No-fallback/no-legacy confirmation: Python accepts only the Rust-planned
  `generate_image` envelope shape, rejects unknown payload fields such as
  `trust_remote_code`, requires Rust-selected canonical `device`, and projects
  generation kwargs without choosing family, scheduler, custom-code trust, or
  device fallback.
- Verification passed: `cargo test -p inference --features backend-pytorch
  python_worker_generate_image_contract`, `cargo test -p inference
  --features backend-pytorch pytorch_worker_generate_image`,
  `cargo check -p inference --features backend-pytorch`,
  `cargo fmt --all -- --check`, and `git diff --check`.
- Deviation/discovered standards issue: an initial implementation placed the
  image validation in `worker_contract.py`, which pushed that file above the
  decomposition target. The slice was corrected by moving image-specific
  validation into focused `worker_image_contract.py` and documenting the new
  helper in the torch README. The first `cargo fmt --all -- --check` reported
  formatting changes in the new Rust/PyO3 tests; ran `cargo fmt --all` and
  reran the check successfully.
- Remaining follow-up: wire `worker_image_contract.py` into `worker.py` and
  the Rust `PyTorchBackend::generate_image` path through the validated planner
  and worker envelope.

2026-05-12 Python image worker bridge slice:

- Smallest useful vertical slice: register `worker_image_contract.py` with the
  embedded PyTorch worker loader, import it from `worker.py`, and expose a
  `generate_image_from_envelope` entrypoint that validates the Rust-planned
  envelope before calling the existing loaded Diffusers pipeline helper.
- Allowed write set: `crates/inference/src/backend/pytorch_worker.rs`,
  `crates/inference/src/backend/pytorch_worker_image_python_tests.rs`,
  `crates/inference/src/backend/pytorch.rs`, `crates/inference/torch/worker.py`,
  and this plan directory.
- No-fallback/no-legacy confirmation: the new worker entrypoint delegates
  validation to the strict image envelope contract, returns typed worker error
  responses for invalid requests, and does not choose pipeline family,
  scheduler, custom-code trust, dependency environment, or device fallback in
  Python.
- Verification passed: `cargo test -p inference --features backend-pytorch
  python_worker_generate_image_from_envelope`, `cargo test -p inference
  --features backend-pytorch python_worker_generate_image_contract`,
  `cargo test -p inference --features backend-pytorch
  pytorch_worker_generate_image`, `cargo check -p inference --features
  backend-pytorch`, `cargo fmt --all -- --check`, and `git diff --check`.
- Discovered issue: the first bridge test run exposed a test stub scoping bug
  (`_Image` not visible inside the stub pipeline method). The test setup was
  corrected to use the same Python globals/locals dictionary and the bridge
  tests then passed. Standards debt: `crates/inference/torch/worker.py` is
  already above the decomposition threshold; this slice kept image validation
  and tests in focused files and only added the minimal public bridge function
  required by the existing Rust/PyO3 worker facade.
- Remaining follow-up: wire `PyTorchBackend::generate_image` through the
  validated planner and Python worker envelope.

2026-05-12 planned Rust image backend helper slice:

- Smallest useful vertical slice: add a focused PyTorch image-generation Rust
  helper that consumes `ImageGenerationExecutionPlan`, builds the validated
  worker envelope, invokes `generate_image_from_envelope`, and maps typed worker
  responses into `ImageGenerationResult`.
- Allowed write set: `crates/inference/src/backend/pytorch.rs`,
  `crates/inference/src/backend/pytorch_image_generation.rs`,
  `crates/inference/src/backend/pytorch_image_generation_tests.rs`, and this
  plan directory.
- No-fallback/no-legacy confirmation: the helper accepts only the validated
  Rust execution plan and never builds a request from raw graph backend hints,
  raw device strings, model names, or request-only defaults. The existing
  `InferenceBackend::generate_image(ImageGenerationRequest)` trait method
  remains unwired rather than bypassing the planner.
- Verification passed: `cargo test -p inference --features backend-pytorch
  pytorch_image_generation`, `cargo check -p inference --features
  backend-pytorch`, `cargo fmt --all -- --check`, and `git diff --check`.
- Verification deviation: the first `cargo fmt --all -- --check` reported
  formatting changes in the new module/test files; ran `cargo fmt --all` and
  reran the check successfully.
- Remaining follow-up/re-plan boundary: the public inference backend trait and
  gateway image-generation path currently carry only `ImageGenerationRequest`;
  they do not carry Pumas package facts or the scheduler-owned
  `BackendExecutionDecision` required by the no-fallback planner. Full
  end-to-end gateway wiring needs a planned-context boundary instead of
  reconstructing facts from request fields. The planned-context boundary must
  use the scheduler policy in `06-device-runtime-selection.md` to resolve
  omitted backend/runtime/device intent automatically, including
  readiness/history ranking and controlled exploration among valid candidates.
  It must also depend on the Milestone 5 graph-boundary cleanup and executable
  candidate synthesis slices: full Pumas facts, ledger summaries, candidate
  lists, and scheduler decisions must not be added to workflow graph nodes or
  worker envelopes. Those facts belong to the planning/scheduler boundary and
  must be reduced into the validated `ImageGenerationExecutionPlan` before
  PyTorch execution.

2026-05-14 planned gateway/backend boundary slice:

- Smallest useful vertical slice: add an explicit planned image-generation
  gateway/backend method and fail raw gateway image generation after request
  shape validation unless a validated `ImageGenerationExecutionPlan` is
  supplied.
- Allowed write set: `crates/inference/src/backend/mod.rs`,
  `crates/inference/src/backend/pytorch.rs`, `crates/inference/src/gateway.rs`,
  `crates/inference/src/gateway_tests.rs`, and this plan directory.
- No-fallback/no-legacy confirmation: raw `ImageGenerationRequest` dispatch no
  longer reaches any backend. The gateway still returns typed validation
  diagnostics for invalid dimensions/counts/resource estimates, but valid raw
  requests fail closed with a typed config error requiring
  `ImageGenerationExecutionPlan`. The PyTorch backend only receives planned
  image generation through the existing validated helper; no request-only
  backend/runtime/device/package inference, `diffusers` backend alias,
  scheduler fallback, or raw device parsing was added.
- Verification passed: `cargo test -p inference test_generate_image`, `cargo
  test -p inference --features backend-pytorch pytorch_image_generation`,
  `cargo check -p inference --features backend-pytorch`, `cargo fmt --package
  inference -- --check`, and `git diff --check`.
- Remaining follow-up: wire the workflow/inference execution path to gather
  Pumas package facts, dependency readiness, runtime capabilities, scheduler
  history/readiness decisions, and selected backend/runtime/device facts in an
  async shell, then reduce them into `ImageGenerationExecutionPlan` before
  calling the planned gateway method. Artifact retention and compact
  graph-output shaping remain separate later slices.

2026-05-14 gateway planning-input execution slice:

- Smallest useful vertical slice: add a gateway method that accepts the
  side-effect-free image-generation planning input, runs the Rust planner, and
  dispatches only the resulting `ImageGenerationExecutionPlan` to the active
  backend.
- Allowed write set: `crates/inference/src/gateway.rs`,
  `crates/inference/src/gateway_tests.rs`, and this plan directory.
- No-fallback/no-legacy confirmation: the gateway planning-input method either
  produces one planner-owned execution plan or returns typed
  `ImageGenerationPlannerDiagnostic` records through `GatewayError`. It does
  not infer Pumas facts, backend/runtime/device decisions, package family, or
  execution defaults from request fields or active backend state, and it does
  not dispatch raw image requests after planner rejection.
- Verification passed: `cargo test -p inference test_generate_image`, `cargo
  check -p inference`, `cargo fmt --package inference -- --check`, and `git
  diff --check`.
- Remaining follow-up: wire the workflow/inference async shell that gathers the
  existing request, Pumas facts, dependency readiness, candidate facts, history
  summaries, and scheduler decision before calling this gateway method.

2026-05-14 typed image execution boundary validation slice:

- Smallest useful vertical slice: update typed gateway tests that still
  expected request-only image generation to execute, proving typed image
  execution and typed lifecycle paths now fail closed at the planned execution
  boundary until a planning-input caller supplies Pumas facts and the scheduler
  decision.
- Allowed write set: `crates/inference/src/gateway_tests.rs` and this plan
  directory.
- No-fallback/no-legacy confirmation: no raw typed-image compatibility path was
  restored. `execute_typed` and `execute_typed_with_lifecycle` continue to
  reach the same typed config diagnostic requiring
  `ImageGenerationExecutionPlan`; lifecycle recording marks backend execution
  failed and does not emit postprocessing/result-projection success phases.
- Verification passed: `cargo test -p inference
  test_execute_typed_image_generation_requires_planned_context`, `cargo test
  -p inference planned_boundary`, `cargo test -p inference gateway::tests::`,
  `cargo fmt --package inference -- --check`, and `git diff --check`.
- Remaining follow-up: add the workflow/inference async shell that calls the
  planning-input gateway method for image generation, then update typed
  lifecycle coverage for the successful planned-image path.

2026-05-14 inference README planned-boundary documentation slice:

- Smallest useful vertical slice: update the inference module README so public
  gateway examples and API notes no longer describe raw `generate_image()` as
  an executable image-generation path.
- Allowed write set: `crates/inference/src/README.md` and this plan directory.
- No-fallback/no-legacy confirmation: documentation now points image-generation
  callers to `ImageGenerationPlanningInput` or `ImageGenerationExecutionPlan`
  and states that raw `generate_image()` validates request shape but does not
  dispatch to a backend.
- Verification passed: `git diff --check`.
- Remaining follow-up: the workflow/inference async shell still needs to build
  the planning input from request, Pumas facts, readiness, candidates, history,
  and scheduler decision facts.

2026-05-14 re-plan boundary before workflow execution-plan wiring:

- Boundary: the next successful image-generation execution slice must connect
  the scheduler-owned `WorkflowTechnicalFitDecision` to node-engine image
  execution so `generate_image_from_planning_input` receives a reduced
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

2026-05-15 execution-plan architecture decision:

- Decision: use Option 3 as the target architecture. The scheduler/admission
  path will produce a first-class per-run workflow execution plan containing
  per-node execution decisions. Node execution consumes that plan; it does not
  recompute scheduling policy, infer runtime choices, or persist scheduler
  facts in graph inputs.
- Reason: the scheduler algorithm is expected to change often. A run-level
  execution plan lets future ranking, exploration, readiness, residency,
  warmup, memory-fit, retry, and queue-policy changes stay in scheduler/plan
  production instead of leaking into node-engine, inference gateway, graph
  schemas, frontend state, or worker envelopes.
- No-fallback/no-legacy confirmation: the execution plan is not a compatibility
  shim for request-only image generation. If a runnable image-generation node
  lacks a selected per-node decision, execution fails with typed diagnostics
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

Staged Option 3 implementation plan:

1. Contract foundation slice (completed 2026-05-15):
   - Add a small workflow execution-plan DTO in a focused workflow-service
     module, not by growing the already broad workflow contracts module. The
     DTO is owned at the workflow/embedded-runtime boundary; node-engine must
     not import workflow-service execution-plan contracts because
     workflow-service already depends on node-engine.
   - The initial DTO should contain run id/workflow id, a schema version, and a
     map keyed by stable node id to reduced execution decisions.
   - The first per-node decision shape must include selected backend key,
     selected runtime id/variant id, selected device class/id, selected task id,
     selected model ref when available, and bounded diagnostics/trace ids.
   - Do not include full Pumas facts, worker envelopes, raw graph node payloads,
     local paths beyond existing approved model/package refs, or mutable
     scheduler internals.
   - Make the DTO append-only and correct-by-construction: use typed ids/enums
     where available, `#[non_exhaustive]` on public enums likely to grow,
     explicit schema/version fields, bounded diagnostic arrays, and `Result`
     returning constructors or projection helpers instead of public raw-field
     mutation for validated decisions.
   - Public fallible constructors, projection helpers, and validation paths
     must return specific typed errors or diagnostics, not `Result<T, String>`
     or ad hoc string matching.
   - Update the owning module README or add an ADR in the same slice, because
     this contract changes the workflow execution boundary and will be consumed
     across crates. If a new source directory/module is introduced, its README
     must include the relevant `API Consumer Contract` and
     `Structured Producer Contract` sections for execution-plan DTO semantics,
     versioning, default behavior, and persistence compatibility.
   - Verification: contract serde tests, append-only/default behavior tests,
     and a no-graph-input test proving scheduler decisions are not written into
     workflow node inputs.
   - Completed scope: added `workflow/execution_plan.rs` with schema-versioned
     `WorkflowExecutionPlan`, reduced per-node decisions, bounded diagnostics,
     policy trace ids, typed errors, private validated fields, and
     `serde(try_from)` validation for deserialization. Updated workflow-service
     public re-exports and README traceability. No scheduler admission,
     durable persistence, embedded-runtime projection, node-engine context, or
     inference execution behavior was changed in this slice.
   - Verification result: `cargo test -p pantograph-workflow-service
     workflow::tests::contracts` passed after fixing a missing test import for
     attribution id types. `cargo fmt -p pantograph-workflow-service` was run.

2. Admission production slice (completed 2026-05-15):
   - Build the initial execution plan immediately after runtime preflight and
     scheduler admission, using the existing `WorkflowTechnicalFitDecision`
     already computed before run start.
   - Treat `WorkflowExecutionSessionPreflightCache` as the current source of
     admission evidence. Do not re-query scheduler/runtime facts or create a
     second technical-fit source of truth during plan production.
   - The first implementation may derive per-node decisions from the current
     workflow-level technical-fit decision only when the selected model/task can
     be mapped to exactly one runnable inference node. Ambiguous model-to-node
     mapping, missing selected model/task facts, or multiple runtime/model
     needs represented by one workflow-level decision must fail with typed
     diagnostics rather than fabricating per-node decisions.
   - Store it as run-scoped execution context, not as saved workflow content.
     If persistence is needed for diagnostics or recovery, persist only the
     execution-plan record with explicit schema/version and source ids.
   - Keep execution-plan production in a synchronous core helper fed by
     already-gathered runtime preflight and scheduler facts. Add async only
     when a slice demonstrably needs I/O or an existing async runtime-state
     API; do not make admission/planning async by default. Do not introduce
     untracked background tasks, polling loops, unbounded queues, or locks held
     across `.await`.
   - If a later slice persists execution plans during admission, the durable
     write must be transactional or explicitly idempotent across cancellation
     points. Do not split admission, plan persistence, and active-run state into
     partially committed steps without a compensating diagnostic-backed state.
   - Verification: session admission tests prove the admitted run has an
     execution plan when technical-fit selected a candidate, and no plan is
     produced when technical-fit fails. Tests must isolate any durable run,
     scheduler, or sqlite state per test.
   - Completed scope: added `workflow/execution_plan_admission.rs` as the
     synchronous projection helper from cached workflow capabilities plus the
     selected `WorkflowTechnicalFitDecision` into a reduced
     `WorkflowExecutionPlan`. The helper returns no plan when technical fit
     has no selected candidate and fails closed with typed
     `WorkflowExecutionPlanError` variants for missing selected facts,
     ambiguous selected model records, ambiguous model-to-node mappings,
     unknown selected models, and ambiguous task facts. It maps only
     unambiguous model/task/node facts, requires selected runtime variant and
     selected device class, and records a Pumas model ref instead of full
     package facts.
   - Run-scoped storage: `WorkflowExecutionSessionPreflightCache` now carries
     workflow capability model summaries as admission evidence, and the active
     scheduler run can hold an optional execution plan. Session run admission
     builds and attaches the plan immediately after runtime preflight and
     before runtime load/lifecycle records. Plan build errors terminate the
     admitted run with a workflow capability violation diagnostic rather than
     continuing with request-only execution.
   - No-fallback/no-legacy confirmation: this slice did not add graph-input
     storage, durable execution-plan persistence, frontend DTOs, worker
     envelope fields, node-engine consumption, or request-only image execution.
     Missing/ambiguous canonical admission facts fail closed; they do not
     trigger backend/runtime/device inference from graph hints or active
     backend state.
   - Verification result: `cargo test -p pantograph-workflow-service
     workflow::tests::contracts::workflow_execution_plan_admission`,
     `cargo test -p pantograph-workflow-service
     scheduler::store::tests::active_run_records_run_scoped_execution_plan`,
     `cargo test -p pantograph-workflow-service
     workflow::tests::session_runtime_preflight`, `cargo test -p
     pantograph-workflow-service workflow::tests::session_execution`, `cargo
     check -p pantograph-workflow-service`, and `cargo fmt -p
     pantograph-workflow-service` passed.
   - Deviation/discovered issue: existing session mock technical-fit fixtures
     that represented selected candidates without selected runtime variant or
     selected device facts were updated to canonical complete decisions where
     execution-plan production is expected. This is not compatibility
     fallback; incomplete selected decisions now fail plan production.
   - Discovered follow-up: preflight cache invalidation still keys on graph
     fingerprint, runtime capability fingerprint, and override selection. The
     new cached capability model summaries assume those fingerprints cover the
     selected model/task/node evidence. If Pumas package/model facts can change
     independently of the graph and runtime capability fingerprints, a later
     slice must add a package-facts/update-cursor fingerprint before execution
     plans are reused for warm sessions.

3. Projection adapter slice (completed 2026-05-15):
   - Add a focused adapter that projects a workflow execution-plan node
     decision into inference's `BackendExecutionDecision`.
   - Keep this adapter at the composition boundary. Embedded-runtime owns the
     workflow-plan-to-node-runtime-context projection; inference planner remains
     side-effect free, and node-engine does not know workflow-service DTOs or
     scheduler ranking policy.
   - Use parse/validation boundaries rather than stringly typed pass-through:
     unknown backend ids, runtime variant ids, device ids/classes, missing task
     ids, and malformed selected model refs produce typed projection
     diagnostics.
   - Verification: adapter tests cover selected backend/runtime/device
     projection, missing fields, unknown device/runtime identifiers, and
     diagnostic propagation.
   - Completed scope: added
     `crates/pantograph-embedded-runtime/src/workflow_execution_plan_projection.rs`
     as the composition-boundary adapter from workflow execution-plan node
     decisions to inference `BackendExecutionDecision`. The adapter validates
     backend ids, runtime variant ids, device ids/classes, task ids, selected
     Pumas model refs, bounded diagnostics, and parseable technical-fit policy
     trace ids before constructing the inference decision.
   - No-fallback/no-legacy confirmation: the adapter copies selected facts
     from the workflow execution plan only. It does not inspect graph hints,
     active backend state, raw device strings, Pumas package facts, or request
     fields to choose a backend/runtime/device. Invalid projected ids fail with
     typed `WorkflowExecutionPlanProjectionError` values.
   - Verification result: `cargo test -p pantograph-embedded-runtime
     workflow_execution_plan_projection`, `cargo check -p
     pantograph-embedded-runtime`, and `cargo fmt -p
     pantograph-embedded-runtime` passed.
   - Deviation: the adapter module is intentionally marked `#[allow(dead_code)]`
     until the next node-engine consumption slice threads it into runtime
     context. This keeps the adapter crate-private instead of exposing a
     premature public API just to silence unused warnings.

4. Node-engine consumption slice:
   - Prerequisite active-plan query (completed 2026-05-15): added a
     workflow-service read-only active-run execution-plan query so
     embedded-runtime can fetch the plan for the currently executing session
     run without changing graph inputs, serializing the plan through saved
     workflow data, or exposing scheduler internals to node-engine. The query
     returns `None` for no active run or mismatched run id, and clones only the
     bounded `WorkflowExecutionPlan` DTO. Verification passed: `cargo test -p
     pantograph-workflow-service
     scheduler::store::tests::active_run_records_run_scoped_execution_plan`,
     `cargo check -p pantograph-workflow-service`, and `cargo fmt -p
     pantograph-workflow-service`.
   - Planned inference context contract (completed 2026-05-15): added
     node-engine-owned `PlannedInferenceDecisionContext` plus the
     `PLANNED_INFERENCE_DECISIONS` executor-extension key. The context stores
     reduced inference `BackendExecutionDecision` values by node id for one
     workflow run, validates non-empty run/node ids, and fails closed for stale
     run ids, missing node decisions, or selected-task mismatches. Node-engine
     still does not import workflow-service execution-plan DTOs or scheduler
     policy.
   - Verification result: `cargo test -p node-engine --features
     inference-nodes planned_inference`, `cargo check -p node-engine
     --features inference-nodes`, `cargo check -p node-engine`, and `cargo fmt
     -p node-engine` passed. The featureless `cargo test -p node-engine
     planned_inference` compiled and filtered out the feature-gated tests as
     expected.
   - Embedded-runtime context installation (completed 2026-05-15): added
     workflow-plan-to-node-engine context projection and installed the
     resulting `PlannedInferenceDecisionContext` during keep-alive session
     workflow execution. The warm-session executor now removes the planned
     inference extension before each run and then installs a freshly projected
     context only when the active workflow run has an execution plan. This
     prevents stale plan reuse while keeping workflow-service DTOs out of
     node-engine.
   - Verification result: `cargo test -p pantograph-embedded-runtime
     workflow_execution_plan_projection`, `cargo test -p node-engine
     --features inference-nodes extensions::tests::test_remove_clears_key`,
     `cargo check -p pantograph-embedded-runtime`, `cargo check -p
     node-engine --features inference-nodes`, and `cargo fmt -p node-engine -p
     pantograph-embedded-runtime` passed.
   - Thread the execution plan into node execution through a typed runtime
     context, likely `ExecutorExtensions`, without serializing it into graph
     inputs.
   - Node-engine consumes only a minimal inference-facing decision lookup keyed
     by node id/task id, projected by embedded-runtime from the workflow
     execution plan. It must not depend on workflow-service execution-plan DTOs.
   - Because session executors and `ExecutorExtensions` are reused across warm
     runs, every run must install a fresh run-scoped context that carries the
     current workflow run id, or explicitly clear/replace the old context before
     execution. Missing or mismatched run id must fail closed to prevent stale
     plan reuse.
   - `execute_image_generation_inference` reads the current node's per-node
     decision, combines it with the existing `ImageGenerationRequest` and
     Pumas `ResolvedModelPackageFacts`, and calls
     `generate_image_from_planning_input`.
   - Missing plan, missing node decision, missing package facts, or failed
     projection must terminate the workflow task with typed diagnostics.
   - `ExecutorExtensions` use must remain a typed runtime context only. Do not
     serialize execution-plan data into graph input maps, saved workflow JSON,
     frontend DTOs, or worker envelopes. Node-engine may compose the request,
     package facts, and reduced decision, but it must not own scheduler ranking
     or retry policy.
   - Verification: node-engine tests prove successful planned image execution
     and fail-closed behavior for absent/invalid execution-plan decisions. The
     first cross-layer acceptance test must be written before implementing the
     node-consumption slice, fail for the expected missing planned path, then
     pass after implementation. It must start from canonical
     `llm-inference` image inputs plus resolved package facts and assert the
     planned gateway call/output without depending on private scheduler
     internals.
   - Node-engine planned image consumption (completed 2026-05-15): canonical
     `llm-inference` image-generation execution now builds the existing image
     request, requires the `PLANNED_INFERENCE_DECISIONS` run-scoped context,
     validates the current workflow run id and node/task decision, requires
     `resolved_model_package_facts`, and invokes
     `generate_image_from_planning_input`. The raw typed image-generation
     gateway path is no longer used for this node path, preserving the
     no-fallback rule.
   - Embedded-runtime run-id alignment (completed 2026-05-15): keep-alive
     session execution now passes `workflow_run_id` to the core task executor,
     runtime extension execution id, and inference lifecycle ledger sink. This
     keeps planned-context stale-run validation and request-id correlation on
     the same run identifier while leaving session residency/checkpoint keys on
     the session id.
   - Verification result: `cargo test -p node-engine --features
     inference-nodes test_canonical_llm_image_generation`, `cargo test -p
     node-engine --features inference-nodes
     core_executor::tests::inference_tests`, `cargo test -p node-engine
     --features inference-nodes planned_inference`, `cargo check -p
     node-engine --features inference-nodes`, `cargo check -p node-engine`,
     `cargo check -p pantograph-embedded-runtime`, and `cargo fmt -p
     node-engine -p pantograph-embedded-runtime` passed.
   - Verification deviation: `cargo test -p pantograph-embedded-runtime
     scheduler_session_live_events_use_backend_workflow_run_id` failed before
     exercising this slice because technical-fit rejected the test's
     `candle.cpu` runtime with "Candle executable model loading is not
     implemented". Record this as a fixture/runtime readiness follow-up rather
     than weakening the planned image execution boundary.

5. Lifecycle/diagnostics slice:
   - Attach execution-plan identifiers and selected per-node decision facts to
     existing scheduler, runtime-load, inference lifecycle, and diagnostics
     ledger records without duplicating large payloads.
   - Diagnostics must carry bounded identifiers, selected backend/runtime/device
     facts, policy trace ids, and planner codes only. Do not persist full Pumas
     package facts, local filesystem paths, worker kwargs, image bytes, raw
     graph node payloads, or unbounded diagnostic vectors.
   - Verification: diagnostics tests prove selected backend/runtime/device
     facts come from the execution plan and that planner failures preserve
     diagnostic codes.
   - Planned image lifecycle/ledger diagnostics (completed 2026-05-15):
     `InferenceGateway::generate_image_from_planning_input_with_lifecycle`
     emits task-validation and backend-execution lifecycle facts for the
     planned image path using only bounded selected backend, runtime variant,
     device, model id, artifact kind, and planner diagnostic code facts.
     Planner rejections are projected into lifecycle compatibility issue
     summaries so ledger consumers can see typed planner codes without storing
     Pumas package facts, prompts, local paths, worker kwargs, or image bytes.
     Node-engine uses this lifecycle path only when the existing inference
     lifecycle sink extension is installed; otherwise it remains on the same
     non-lifecycle planned gateway path. The raw image-generation gateway path
     remains rejected and no graph/schema/worker/frontend/durable-plan contract
     changed.
   - Verification result: `cargo test -p inference
     generate_image_from_planning_input_with_lifecycle --lib`, `cargo test -p
     inference gateway::tests::test_generate_image`, `cargo test -p
     node-engine --features inference-nodes
     test_canonical_llm_image_generation_uses_planned_gateway_boundary`,
     `cargo test -p node-engine --features inference-nodes
     test_canonical_llm_image_generation`, `cargo test -p
     pantograph-embedded-runtime
     inference_diagnostic_event_adapter_persists_image_generation_bounded_lifecycle_summary`,
     `cargo test -p pantograph-embedded-runtime
     node_execution_ledger::tests::inference_diagnostic_event_adapter`, `cargo
     check -p inference`, `cargo check -p node-engine --features
     inference-nodes`, `cargo check -p pantograph-embedded-runtime`, and `cargo
     fmt -p inference -p node-engine -p pantograph-embedded-runtime` passed.
   - Scheduler admission/runtime-load execution-plan diagnostics (completed
     2026-05-15): diagnostics-ledger now has a bounded
     `SchedulerExecutionPlanSummary` contract with schema version, node
     decision count, and policy trace ids. Workflow-service produces that
     summary from the scheduler-owned active execution plan after admission and
     attaches it to scheduler run-admitted and model lifecycle load/unload
     records. Runtime/session cache events without run-plan context explicitly
     omit the field. The slice does not expose full execution plans, Pumas
     package facts, graph payloads, worker envelopes, local paths, or scheduler
     ranking internals to node-engine, inference, frontend DTOs, saved workflow
     files, or worker contracts.
   - Verification result: `cargo test -p pantograph-diagnostics-ledger
     scheduler_run_admitted_payload_round_trips_policy_trace_contract --lib`,
     `cargo test -p pantograph-diagnostics-ledger
     scheduler_run_admitted_rejects_invalid_execution_plan_summary --lib`,
     `cargo test -p pantograph-diagnostics-ledger
     scheduler_run_admitted_rejects_inconsistent_policy_trace_counts --lib`,
     `cargo test -p pantograph-diagnostics-ledger
     model_lifecycle_projects_canonical_error_link_without_counting_new_error
     --lib`,
     `cargo test -p pantograph-workflow-service
     workflow_execution_session_records_load_completed_only_with_runtime_proof
     --lib`, `cargo check -p pantograph-diagnostics-ledger`, `cargo check -p
     pantograph-workflow-service`, `cargo fmt -p pantograph-diagnostics-ledger
     -p pantograph-workflow-service`, and `git diff --check` passed.
   - Unsupported image option lifecycle diagnostic coverage (completed
     2026-05-15): planned image-generation gateway lifecycle tests now prove
     planner `UnsupportedOption` diagnostics are projected as
     `unsupported_option` compatibility issues with exact request field paths,
     while request image bodies remain absent from serialized lifecycle
     events. This is test-only coverage; it does not add fallback execution,
     graph schema fields, frontend behavior, worker fields, lockfiles, saved
     workflow fixtures, persisted schemas, or new diagnostics payload types.
   - Verification result: `cargo test -p inference
     test_generate_image_from_planning_input_with_lifecycle_records_unsupported_option_code --lib`,
     `cargo check -p inference`, `cargo fmt -p inference`, and
     `git diff --check` passed.
   - Remaining follow-up: recovery/retry policy still needs explicit planning
     before execution plans can become durable replay state. That belongs to
     the recovery and future expansion slice, not scheduler lifecycle
     diagnostic payloads.

6. Recovery and future expansion slice:
   - Define how execution plans participate in retry/recovery. A retry may
     reuse a still-valid plan or request a new scheduler plan, but the policy
     must be explicit and diagnostic-backed.
   - Durable persistence of execution plans remains deferred until replay,
     retry, and idempotency semantics are specified. The initial vertical
     slices should keep the plan in run-scoped execution context unless a later
     slice adds the durable semantics and tests below.
   - Add replay/recovery/idempotency tests before treating the execution-plan
     record as durable. Duplicate admission, cancellation during execution-plan
     production, and retry after runtime/resource failure must not produce
     conflicting selected decisions without a new explicit plan version/source
     id.
   - Keep later additions append-only: multi-node placement, memory
     reservations, exploration cohorts, warmed-runtime affinity, historical
     performance summaries, and artifact-retention decisions can extend the
     plan without changing node graph ergonomics.
   - Run-scoped plan lifecycle guardrail (completed 2026-05-15):
     scheduler-store tests now prove an active execution plan is cleared when
     the active run finishes and is not visible to either the finished run id or
     the next admitted run before a new plan is produced. This codifies the
     current policy: retries/re-admissions must get a fresh active-run plan
     unless a future durable replay slice explicitly adds idempotent persistence
     and diagnostic-backed reuse semantics. No production behavior, durable
     storage, scheduler ranking, node-engine context, inference gateway,
     frontend DTOs, saved workflow fixtures, worker contracts, lockfiles, or
     generated files changed.
   - Verification result: `cargo test -p pantograph-workflow-service
     finish_run_clears_run_scoped_execution_plan_before_next_admission --lib`,
     `cargo test -p pantograph-workflow-service
     active_run_records_run_scoped_execution_plan --lib`, `cargo check -p
     pantograph-workflow-service`, `cargo fmt -p pantograph-workflow-service`,
     and `git diff --check` passed.
   - Remaining follow-up: durable replay/retry semantics remain deferred and
     require a separate re-plan with transactional/idempotent persistence,
     duplicate-admission tests, cancellation tests, and diagnostic-backed
     retry/reuse policy before any execution-plan record becomes recoverable
     state.

2026-05-15 compact image-generation output slice:

- Smallest useful vertical slice: stop canonical image-generation node
  execution from duplicating generated image base64 in both graph-visible
  `image` and structured `results` outputs before workflow artifact
  conversion runs.
- Allowed write set: `crates/node-engine/src/core_executor/inference_nodes.rs`,
  `crates/node-engine/src/core_executor/inference_tests.rs`,
  `crates/node-engine/src/core_executor/README.md`,
  `crates/node-engine/src/README.md`, and this plan directory.
- No-fallback/no-legacy confirmation: this only changes output projection after
  successful planned image generation. It does not restore raw
  image-generation execution, infer backend/runtime/device decisions, change
  worker envelopes, bypass artifact conversion, or add compatibility shims.
- Result: the `image` port remains the single image body source for
  image-output templates and workflow artifact conversion, while `results`
  carries compact per-image summaries plus seed/backend metadata without
  `data_base64` bodies.
- Verification passed: `cargo test -p node-engine --features inference-nodes
  test_canonical_llm_image_generation_uses_planned_gateway_boundary`, `cargo
  check -p node-engine --features inference-nodes`, `cargo fmt -p
  node-engine`, and `git diff --check`.
- Remaining follow-up: workflow-service artifact conversion still needs a
  vertical retained-output test once an end-to-end image-generation workflow
  fixture exists, proving the `image` port is retained as one media body and
  `results` remains compact after conversion.

2026-05-15 execution-evidence normalization planning:

- Objective: replace scattered `diffusers` executable-backend decisions with a
  general execution-evidence normalization boundary that works for all model
  families and runtimes. The immediate Diffusers/PyTorch case must be one row
  in a broader evidence system, not a hidden image-generation or PyTorch-only
  shortcut.
- Architectural placement: create the evidence interpretation boundary beside
  inference model/package contracts, for example an inference-owned
  `execution_evidence` module or equivalently named submodule. That boundary
  can consume `ResolvedModelPackageFacts`, artifact kind, task evidence,
  backend hints, runtime capability facts, and optional graph constraints. It
  must not live inside `image_generation_planner`, `backend/pytorch`,
  node-engine, workflow-service, or the scheduler ranking algorithm.
- Shared identity scope: `pantograph-runtime-identity` remains the owner for
  spelling/alias normalization such as `torch -> pytorch` or
  `llamacpp -> llama_cpp`. It must not own package-fact interpretation such as
  "Diffusers package evidence can be executed by PyTorch"; that decision
  depends on inference package contracts and backend capability facts.
- General contract shape: the evidence boundary should return typed evidence
  records with explicit roles instead of a single overloaded string. Required
  roles are executable backend candidate, dependency/package evidence, runtime
  capability evidence, graph preference/constraint, and display label. The
  records must retain enough source detail for diagnostics without carrying
  full Pumas facts into scheduler state.
- General applicability: the first implementation must be shaped so later
  rows can cover Transformers/PyTorch, GGUF/llama.cpp, ONNX Runtime bundles,
  Candle embeddings, vLLM, MLX, Stable Audio, and remote runtimes without
  changing scheduler APIs. A new runtime should add capability/evidence rows,
  not fork image-generation logic.
- Diffusers rule: Diffusers package evidence such as `diffusers_bundle`,
  present Diffusers facts, `BackendHintLabel::Diffusers`, and
  `recommended_backend = diffusers` may produce a PyTorch executable backend
  candidate only when PyTorch runtime capability facts advertise Diffusers
  support for the requested task/model shape. The emitted executable backend
  key is `pytorch`; `diffusers` remains dependency/package/capability evidence.
  Existing pseudo-backend helpers that convert `BackendHintLabel::Diffusers`
  directly to `"diffusers"` for execution selection are implementation debt to
  remove, not compatibility behavior to preserve.
- PyTorch capability requirement: the evidence rule cannot infer PyTorch image
  eligibility from package facts alone. The PyTorch backend capability facts
  must explicitly advertise the supported image-generation task, Diffusers
  artifact/source support, and valid runtime variants before the evidence
  boundary can emit a PyTorch executable candidate for Diffusers package facts.
- Runtime-specific separation: PyTorch-specific image behavior starts only
  after scheduler selection has produced a PyTorch `BackendExecutionDecision`.
  The PyTorch/diffusers image bridge may validate pipeline family, components,
  denoising scheduler support, device policy, dependency readiness, and worker
  envelope fields, but it must not decide package-hint-to-backend mapping or
  choose among valid runtimes.
- Scheduler separation: the evidence boundary enumerates valid executable
  candidates and their supporting facts. Scheduler policy still owns ranking,
  warmed-runtime affinity, exploration before enough history exists,
  historical timing/failure weighting, memory-fit ranking, queue policy, and
  retry/reschedule decisions.
- Explicit-runtime ranking rule: an explicit graph `runtime` request narrows
  the scheduler candidate set to candidates that satisfy the requested runtime
  after package/capability validation. It must then use the canonical scheduler
  ranking path inside that constrained set. Do not add a separate
  override-only picker that selects by candidate id, insertion order, or
  ad-hoc priority; otherwise explicit runtime requests would bypass timing
  history, warmed-runtime state, memory-fit ranking, and future scheduler
  policy changes.
- Graph runtime-request semantics: graph `runtime` / executable backend
  inputs are scheduler inputs with typed intent, not execution decisions. If
  the inference node omits the runtime, scheduler/admission policy chooses
  among all validated executable candidates using memory fit, warmed-runtime
  state, exploration policy, historical timing/failure diagnostics, and queue
  policy. If the inference node explicitly requests a runtime, that request is
  a hard scheduler requirement: scheduler/admission must use that runtime only
  after normal candidate validation, or fail candidate selection with a typed
  diagnostic and scheduler-ledger evidence. Explicit runtime requests cannot
  create an executable candidate that package/capability facts do not support,
  cannot bypass scheduler policy, and cannot be reinterpreted from nested
  Pumas/package recommendations. If a graph explicitly asks for `diffusers`
  while no executable Diffusers backend exists, the result is a typed
  diagnostic with a hint to request PyTorch or omit the runtime so scheduler
  policy can choose.
- Single runtime-selection path: inference graph nodes need an optional
  `runtime` input so workflows can express a hard scheduler requirement when
  needed, but that graph value stops at scheduler/admission input projection.
  Node-engine request construction, image/text/audio inference request DTOs,
  worker envelopes, and inference gateway calls must not read graph runtime
  values as execution selections. They must receive only the
  scheduler-produced backend/runtime/device execution decision. This keeps the
  scheduler as the sole runtime selection authority and prevents parallel
  runtime-selection paths from graph data, dependency metadata, or node
  execution context.
- First implementation slice: add the typed evidence boundary plus focused
  tests for Diffusers package facts, PyTorch capability facts, and an explicit
  graph runtime request represented as typed evidence input
  (`runtime = pytorch`). The allowed initial write set should be limited to the
  new inference evidence module, PyTorch static capability facts and their
  focused tests if required to express Diffusers support, existing inference
  model/backend compatibility tests or fixtures, `crates/inference/src/README.md`
  or an ADR documenting the new evidence boundary, runtime-identity tests only
  if alias documentation needs clarification, and this plan. It should not edit
  workflow-service admission, embedded-runtime technical fit, workflow-node
  descriptors, node-engine, or gateway behavior until the boundary contract is
  pinned. The evidence boundary must be synchronous core logic unless a future
  slice identifies real I/O that belongs in an async shell.
- Second implementation slice: add the optional inference-node `runtime` input
  and scheduler-request projection. This slice should expose the graph input
  in the workflow-node descriptor, update workflow capability/runtime
  extraction to read only that explicit inference-node input as a hard
  scheduler requirement, project it into `override_selection` or the canonical
  scheduler request field, and add guard tests proving nested Pumas
  `recommended_backend`, dependency metadata, and node-engine forwarded
  context do not become runtime requirements. Allowed write set can include
  `crates/workflow-nodes/src/processing/inference.rs`, workflow-node
  descriptor tests, `crates/pantograph-workflow-service/src/capabilities.rs`,
  focused workflow-service capability/preflight tests, graph registry or
  persistence tests only if the new `runtime` port requires contract allowlist
  updates, owning module README or ADR updates required by the changed graph
  and workflow capability contracts, and this plan. Before implementation,
  add a failing vertical acceptance test through the real workflow graph
  capability/preflight and scheduler-request projection path proving:
  omitted `runtime` produces no hard override, explicit `runtime = pytorch`
  produces the scheduler requirement, and nested package/dependency
  `recommended_backend` does not. It must not add graph runtime values to
  node-engine inference request construction, inference DTOs, gateway calls,
  or worker envelopes.
- Third implementation slice: migrate embedded-runtime technical-fit
  candidate construction and dependency preflight to consume the boundary.
  Allowed write set can include
  `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
  `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment.rs`,
  `crates/pantograph-embedded-runtime/src/runtime_capabilities.rs`,
  `crates/pantograph-embedded-runtime/src/model_dependency_descriptors.rs`,
  `crates/pantograph-embedded-runtime/src/model_dependencies_tests.rs`, focused
  technical-fit/dependency tests, affected embedded-runtime README/ADR updates,
  and this plan. If explicit runtime requirements require runtime-registry
  selector changes, put those changes in a separate serial scheduler-policy
  slice or explicitly extend this slice with `pantograph-runtime-registry`
  ownership before implementation.
- Fourth implementation slice: migrate workflow/runtime preflight and gateway
  diagnostics that currently maintain their own `diffusers` backend logic.
  This includes workflow-service required-backend extraction for nested
  package/Pumas `recommended_backend`, node-engine dependency-context forwarding
  expectations, lifecycle diagnostics, and any remaining gateway diagnostic
  paths. Add a cross-boundary guard search proving execution selectors do not
  treat `diffusers` as a scheduler-selected backend key.
- Out of scope for these normalization slices: actual PyTorch image worker
  execution, model-family adapters, denoising scheduler provider rows,
  artifact retention, end-to-end smoke generation, durable scheduler replay,
  and new runtime ranking algorithms.
- Re-plan trigger: stop before implementation if the only practical placement
  for the evidence boundary would require workflow-service to depend on
  inference DTOs, node-engine to depend on workflow-service, scheduler policy
  to parse full package facts, or PyTorch-specific code to become the owner of
  package-hint-to-backend interpretation.

Standards compliance gates for every Option 3 slice:

- Worktree hygiene: inspect `git status` before each slice and do not start
  code work while unrelated tracked implementation files are dirty.
- Shared-contract ownership: workflow execution-plan DTOs, projection adapters,
  serde fixtures, generated DTOs, lockfiles, saved workflow fixtures, ADRs, and
  README updates are serial integration-owner work unless explicitly
  reassigned.
- Crate-boundary rule: workflow-service may own the run execution-plan
  contract, embedded-runtime adapts it, and node-engine consumes only
  node-engine/inference-facing runtime context. Do not add a node-engine
  dependency on workflow-service.
- Warm-session safety rule: reused executors must not retain stale execution
  decisions across workflow runs; run id validation or explicit context
  replacement/clearing is required before planned node execution.
- Typed-error rule: public cross-crate constructors and projection helpers must
  expose typed errors/diagnostics and avoid `Result<T, String>` or stringly
  branch behavior.
- Model-identity rule: selected model identity in execution plans must be
  canonical and typed before it is used for scheduler history, diagnostics,
  runtime readiness, or worker dispatch. Do not compose Pumas refs with string
  prefixing in multiple layers; normalize once at the owner boundary or reject
  the selected fact.
- Parse-once rule: once a selected model/backend/runtime/device fact crosses
  into a validated execution-plan type, downstream layers consume the typed
  value or fail projection. They must not re-validate by string matching,
  silently repair malformed data, or infer missing identity from graph inputs,
  package facts, active backend state, or worker metadata.
- Identity-consistency rule: planned image execution must not let package
  facts silently override a scheduler-selected model identity. If both the
  execution plan and package facts carry model refs, they must match before
  planning can produce a worker execution plan. For image generation, the
  execution plan must carry the selected model ref; absence is a planner or
  execution-plan diagnostic, not permission to use package facts as the
  selected identity.
- Graph-hint rule: graph-provided backend/runtime/device hints are admissible
  only as scheduler inputs. They are not execution decisions, are not required
  for ordinary inference graphs, and must produce typed diagnostics when they
  cannot be reconciled with canonical runtime/package facts.
- Graph runtime contract rule: new inference-node runtime intent must be a
  typed optional graph input projected into one scheduler-owned request field.
  It must not be preserved through legacy `backend_key` compatibility paths,
  dependency metadata forwarding, inference DTOs, gateway calls, or worker
  envelopes. Omitted runtime means no explicit scheduler override.
- Dependency/backend separation rule: package/dependency evidence and
  execution backend selection must use separate names/types. `diffusers`
  package facts can drive PyTorch eligibility, but they must not become an
  execution backend key or bypass scheduler-owned backend/runtime selection.
- Execution-evidence boundary rule: package-fact-to-executable-backend mapping
  must be centralized in a typed, inference-owned evidence boundary that is
  reusable across runtimes and model families. Runtime-specific planners and
  workers consume scheduler-selected decisions; they do not own candidate
  discovery or ranking.
- Documentation contract rule: a new execution-plan source module requires a
  README or ADR in the same slice. README content must document consumer
  expectations, serde shape, schema/version behavior, append-only evolution,
  and persistence/replay compatibility.
- Cancellation/idempotency rule: any durable execution-plan write introduced
  later must be transactional or idempotent across cancellation points and must
  have replay/retry tests before being treated as recoverable state.
- Public facade rule: preserve `InferenceGateway` and node-engine public task
  entrypoints while adding planned execution paths; do not restore raw
  `generate_image` or request-only typed image execution.
- Option-provider rule: backend-owned `PortOptionsProvider` implementations may
  list valid selectable values for graph ergonomics, but they must not become
  execution-policy owners. The planner must revalidate selected values against
  package facts, runtime facts, family rules, and worker support.
- Graph-default rule: frontend selection helpers must not silently make
  executable choices by writing first/default option values into graph data for
  fact-dependent inference traits. Omitted graph options must remain omitted
  until planner-owned defaults or typed diagnostics are applied.
- Security/path rule: any path-like model/package/artifact value crossing into
  execution-plan records must already be validated by the existing Pumas/model
  root validation boundary or rejected before worker execution.
- Verification rule: each slice needs focused unit/contract tests. The first
  graph-runtime projection slice needs a test-first vertical acceptance path
  through real workflow graph capability/preflight and scheduler-request
  projection. The first slice that changes node-engine or inference execution
  behavior needs a separate vertical acceptance path through the real planned
  node-engine/inference boundary.
- Scheduler-policy rule: explicit runtime requirements constrain valid
  scheduler candidates before canonical ranking; they must not create a
  parallel ranking algorithm or candidate-id based shortcut. Any required
  runtime-registry selector change is scheduler-policy work and needs its own
  focused tests plus README/ADR traceability when the public contract changes.

2026-05-16 execution-evidence boundary implementation slice:

- Smallest useful vertical slice: add the inference-owned
  `execution_evidence` module, expose its typed candidate/evidence/diagnostic
  contracts, and update PyTorch static capability facts so Diffusers
  image-generation package facts can produce a PyTorch executable candidate
  only through explicit backend capability support.
- Allowed write set: `crates/inference/src/execution_evidence.rs`,
  `crates/inference/src/execution_evidence_tests.rs`,
  `crates/inference/src/lib.rs`, `crates/inference/src/backend/pytorch.rs`,
  `crates/inference/src/backend/pytorch_tests.rs`,
  `crates/inference/src/README.md`, and this plan directory.
- No-fallback/no-legacy confirmation: `diffusers` remains package,
  dependency, display, and capability evidence. The evidence boundary does not
  alias an explicit graph `runtime = diffusers` request to PyTorch, does not
  rank candidates, does not read scheduler history, does not reserve memory,
  and does not pass graph runtime data into inference execution.
- Verification passed: `cargo test -p inference execution_evidence --lib`,
  `cargo test -p inference --features backend-pytorch test_capabilities --lib`,
  `cargo check -p inference`, `cargo fmt -p inference`,
  `cargo fmt -p inference -- --check`, and `git diff --check`.
- Remaining follow-up: migrate embedded-runtime technical-fit and dependency
  preflight to consume the new evidence boundary before removing scattered
  pseudo-Diffusers execution mappings from those layers.

2026-05-16 graph runtime input/projection implementation slice:

- Smallest useful vertical slice: expose `llm-inference.runtime` as the
  canonical optional graph-authored scheduler requirement, remove the
  graph-visible `backend_key` inference input, and project only that explicit
  runtime value through workflow capability extraction into scheduler/preflight
  request data.
- Allowed write set: `crates/workflow-nodes/src/processing/inference.rs`,
  `crates/workflow-nodes/src/contracts.rs`,
  `crates/workflow-nodes/src/README.md`,
  `crates/workflow-nodes/src/processing/README.md`,
  `crates/pantograph-workflow-service/src/capabilities.rs`,
  `crates/pantograph-workflow-service/src/workflow/tests/workflow_capabilities.rs`,
  `crates/pantograph-workflow-service/src/README.md`, and this plan
  directory.
- No-fallback/no-legacy confirmation: this slice does not preserve
  `backend_key` as a canonical inference-node runtime input and does not pass
  graph runtime values into node-engine execution, worker envelopes, gateway
  calls, or inference DTOs. Omitted `runtime` values produce no hard runtime
  requirement; Pumas `recommended_backend` and dependency metadata remain
  package/capability evidence, not scheduler requirements.
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
- Verification deviation: `cargo test -p workflow-nodes --lib` still fails in
  `contracts::tests::builtin_contracts_preserve_registered_port_options_provider_refs`
  because the test expects the `puma-lib.model_path` options provider without
  enabling the `model-library` feature that registers that provider. The same
  test passes with `--features model-library`; fixing the feature-gated test
  policy is outside this slice.
- Remaining follow-up: scheduler/runtime-registry selection still needs the
  next slice to enforce explicit runtime constraints against canonical
  candidates and typed diagnostics, and embedded-runtime technical-fit,
  dependency preflight, workflow/runtime preflight, and gateway diagnostics
  still need migration to consume the inference-owned evidence boundary.

2026-05-16 pseudo-Diffusers sidecar capability retirement slice:

- Smallest useful vertical slice: remove `diffusers` from the static Python
  sidecar runtime capability inventory while preserving the ability for a
  future real executable Diffusers backend to appear from actual backend
  capability facts.
- Allowed write set: `crates/pantograph-embedded-runtime/src/runtime_capabilities.rs`,
  `crates/pantograph-embedded-runtime/src/README.md`, and this plan directory.
- No-fallback/no-legacy confirmation: this slice does not alias Diffusers to
  PyTorch, does not add a compatibility runtime, and does not hide scheduler
  failure behind a fallback. Diffusers remains dependency/model-source evidence
  for PyTorch image generation unless a real backend registers `diffusers` as
  its executable backend key.
- Verification passed: `cargo test -p pantograph-embedded-runtime
  python_runtime_capabilities_report_python_backed_engines --lib`, `cargo test
  -p pantograph-embedded-runtime
  python_runtime_capabilities_keep_unavailable_reason --lib`, `cargo test -p
  pantograph-embedded-runtime
  host_runtime_capabilities_allow_real_diffusers_backend_registration --lib`,
  `cargo test -p pantograph-embedded-runtime runtime_capabilities --lib`,
  `cargo check -p pantograph-embedded-runtime`, and `cargo fmt -p
  pantograph-embedded-runtime -- --check`, and `git diff --check`.
- Remaining follow-up: technical-fit candidate construction still needs to
  consume the inference-owned evidence boundary so package facts, backend
  capability facts, explicit graph runtime requirements, and scheduler ranking
  share one normalization path.

2026-05-16 Diffusers package-hint execution mapping retirement slice:

- Smallest useful vertical slice: stop converting Pumas package
  `BackendHintLabel::Diffusers` into a scheduler-visible executable backend
  candidate in the runtime-capability-only technical-fit path.
- Allowed write set: `crates/pantograph-embedded-runtime/src/technical_fit.rs`
  and this plan directory.
- No-fallback/no-legacy confirmation: this slice fails closed when only
  package/runtime hint evidence says `diffusers`; it does not synthesize a
  PyTorch candidate, keep a pseudo-Diffusers candidate, or preserve any
  compatibility alias. Real executable backend candidates still come from the
  backend-capability checked path.
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

2026-05-16 Option 3 technical-fit adapter contract plan:

- Decision: implement Option 3 as a replacement adapter in
  `pantograph-embedded-runtime`, tentatively named
  `ExecutionEvidenceTechnicalFitAdapter`. The adapter consumes
  `inference::ExecutionEvidenceReport`, package facts, runtime capabilities,
  resource estimates, and the optional graph runtime requirement already
  projected through workflow capability data. It emits runtime-registry
  `RuntimeTechnicalFitCandidate` values and
  `RuntimeTechnicalFitDeviceDiagnostic` values.
- Ownership: inference remains the owner of model/package/backend evidence and
  compatibility normalization. Embedded-runtime owns only projection from
  accepted evidence candidates into runtime-registry candidate shape by joining
  with current `WorkflowRuntimeCapability` facts for runtime id, runtime
  variant id, device class, readiness, residency, warmup, and resource estimate.
  Runtime-registry remains the owner of scheduler ranking, history weighting,
  memory-fit policy, and final selection.
- Replacement rule: the adapter replaces
  `runtime_candidates_from_pumas_package_facts_with_backend_capabilities`,
  `runtime_candidates_from_pumas_package_facts_with_runtime_capabilities`, and
  package-hint-to-backend candidate construction for canonical inference
  technical-fit. The old candidate builders must be deleted or reduced to
  adapter-internal projection helpers in the same slice that wires the adapter;
  they must not remain as fallback paths.
- Diagnostic contract: evidence diagnostics must map to blocking
  technical-fit diagnostics when no valid candidate remains. Required mappings
  include unsupported task, backend unavailable, missing runtime capability,
  required package evidence unavailable, backend compatibility rejected, and
  graph runtime requirement unsatisfied. Diagnostic records must preserve the
  requested runtime key and executable backend key when present.
- Explicit runtime contract: omitted graph `runtime` means no
  `GraphRuntimeRequirement` is sent into the evidence boundary and scheduler
  policy may choose among validated candidates. Explicit graph `runtime` is
  parsed into `GraphRuntimeRequirement`; the adapter may only project evidence
  candidates matching that requirement. If none match, it emits the typed
  unsatisfied-runtime diagnostic and produces no selected candidate/fallback.
- Implementation stages:
  1. Add failing embedded-runtime tests for the adapter contract: Diffusers
     package facts plus PyTorch backend facts produce a PyTorch technical-fit
     candidate preserving `diffusers` as package evidence; explicit
     `runtime = diffusers` produces a typed diagnostic and no candidate unless
     a real Diffusers backend exists; omitted runtime lets multiple validated
     executable candidates proceed to scheduler ranking; evidence rejection
     does not create old package-hint fallback candidates.
  2. Add the adapter module/function and diagnostic projection helpers without
     wiring it into the public technical-fit request path. Keep it synchronous;
     all required facts are already in memory.
  3. Replace the current backend-package-facts candidate construction call site
     with the adapter, delete the old direct candidate construction path, and
     update README traceability. Do not keep a branch that runs the old builder
     when evidence returns no candidates.
  4. Run focused embedded-runtime technical-fit tests, runtime-capability tests
     that guard no pseudo-Diffusers exposure, `cargo check -p
     pantograph-embedded-runtime`, formatting, and `git diff --check`.
- No-fallback/no-legacy confirmation: this plan explicitly rejects keeping
  both systems. If the evidence boundary cannot produce a valid executable
  candidate, technical-fit must return typed diagnostics and the scheduler must
  fail candidate selection rather than using package hints, historical aliases,
  default runtime choices, node-engine context, or direct backend compatibility
  loops to recover old behavior.

2026-05-16 technical-fit diagnostic contract slice:

- Smallest useful vertical slice: extend the canonical technical-fit diagnostic
  DTOs with evidence-oriented codes and attribution fields before wiring the
  replacement evidence adapter. Allowed write set was runtime-registry
  technical-fit DTO/policy/tests, workflow-service technical-fit DTO and
  contract fixture, embedded-runtime projection/tests, the TypeScript workflow
  mirror, crate READMEs, and this plan directory.
- No-fallback/no-legacy confirmation: the slice only adds structured diagnostic
  capacity and projection. It does not add fallback candidates, legacy
  compatibility aliases, pseudo-Diffusers execution keys, selector recovery, or
  worker dispatch paths.
- Verification passed: `cargo test -p pantograph-runtime-registry
  technical_fit --lib`, `cargo test -p pantograph-workflow-service
  technical_fit --lib`, `cargo test -p pantograph-embedded-runtime
  technical_fit --lib`, `cargo test -p pantograph-workflow-service
  workflow_technical_fit_cross_layer_fixture_deserializes --test contract`,
  `cargo check -p pantograph-runtime-registry`, `cargo check -p
  pantograph-workflow-service`, `cargo check -p pantograph-embedded-runtime`,
  and `npm run typecheck`.
- Remaining follow-up: implement the focused
  `ExecutionEvidenceTechnicalFitAdapter` mapping/projection module before
  wiring evidence into technical-fit. The current slice deliberately did not
  add adapter policy or preserve the old builders as a fallback path.

2026-05-16 focused technical-fit diagnostics projection slice:

- Smallest useful vertical slice: extract embedded-runtime diagnostic code,
  severity, device-class, and attribution projection into
  `technical_fit_diagnostics.rs` before the evidence adapter mapping table is
  added. Allowed write set was embedded-runtime technical-fit files, the
  embedded-runtime README, and this plan directory.
- No-fallback/no-legacy confirmation: this is a behavior-preserving ownership
  refactor only. It does not add adapter wiring, fallback candidates, legacy
  compatibility aliases, pseudo-Diffusers runtime/backend keys, selector
  recovery, or worker dispatch paths.
- Verification passed: focused embedded-runtime technical-fit tests,
  `cargo check -p pantograph-embedded-runtime`, `cargo fmt -p
  pantograph-embedded-runtime -- --check`, and `git diff --check`.

2026-05-16 execution-evidence technical-fit adapter contract slice:

- Smallest useful vertical slice: add the internal
  `technical_fit_execution_evidence.rs` adapter boundary without public
  technical-fit wiring. The adapter consumes inference-owned
  `ExecutionEvidenceReport` values plus minimal task/model context and
  workflow runtime capability facts, then emits runtime-registry candidates
  and typed diagnostics.
- No-fallback/no-legacy confirmation: the adapter does not call old
  package-hint builders, does not alias `runtime = diffusers` to PyTorch, and
  emits an explicit `evidence_no_accepted_candidate` diagnostic when no
  validated candidate survives.
- Verification passed: `cargo test -p pantograph-embedded-runtime
  technical_fit_execution_evidence --lib`, `cargo test -p
  pantograph-embedded-runtime technical_fit --lib`, `cargo check -p
  pantograph-embedded-runtime`, `cargo fmt -p pantograph-embedded-runtime
  -- --check`, and `git diff --check`.
- Deviation/follow-up: the adapter module has a staged `dead_code` allowance
  because this slice stops before public wiring. The next wiring slice must
  remove that allowance, replace the current backend-package-facts candidate
  construction call site with the adapter, and delete or reduce the old direct
  candidate builders so they cannot remain as fallback behavior.

2026-05-16 execution-evidence technical-fit adapter wiring slice:

- Smallest useful vertical slice: replace the embedded-runtime
  backend-package-facts technical-fit candidate construction call site with the
  `ExecutionEvidenceTechnicalFitAdapter`, remove the staged `dead_code`
  allowance, and delete the old direct package-hint/backend-compatibility
  candidate builders.
- Allowed write set: `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
  `crates/pantograph-embedded-runtime/src/technical_fit_execution_evidence.rs`,
  `crates/pantograph-embedded-runtime/src/README.md`, and this plan directory.
- No-fallback/no-legacy confirmation: canonical package-backed technical-fit
  now builds scheduler candidates from inference execution evidence only.
  Rejected evidence becomes typed diagnostic candidates, generic runtime
  capability candidates are not left eligible as a fallback for model-bound
  package evidence, and `diffusers` remains dependency/package evidence unless
  a real executable backend registers it.
- Verification passed: `cargo test -p pantograph-embedded-runtime
  technical_fit --lib`, `cargo check -p pantograph-embedded-runtime`, `cargo
  fmt -p pantograph-embedded-runtime -- --check`, and `git diff --check`.
- Remaining follow-up: continue the Milestone 6 audit of dependency preflight,
  runtime capability, gateway, workflow runtime preflight, and node-engine
  dependency-context paths so none of them maintain a conflicting Diffusers or
  package-hint backend-selection rule.

2026-05-16 node-engine dependency-context backend-selection cleanup slice:

- Smallest useful vertical slice: remove package-facts backend-hint and
  `recommended_backend` interpretation from node-engine dependency preflight
  and dependency-context forwarding while preserving package facts as typed
  model/task evidence for inference requests and dependency requests.
- Allowed write set: `crates/node-engine/src/core_executor.rs`,
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

2026-05-16 planned image-generation gateway test-contract cleanup slice:

- Smallest useful vertical slice: align the node-engine planned image-generation
  gateway success test with the current image planner contract by removing the
  explicit `denoising_scheduler = euler` input from the success path. Explicit
  denoising scheduler changes are still covered as planner diagnostics until
  family/runtime support is implemented.
- Allowed write set: `crates/node-engine/src/core_executor/inference_tests.rs`
  and this plan directory.
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

2026-05-16 inference compatibility package-vs-backend naming slice:

- Smallest useful vertical slice: rename the Diffusers image-generation
  compatibility test helper so it represents a PyTorch backend with Diffusers
  package support, and attribute compatibility checks to executable backend
  key `pytorch` while preserving `BackendHintLabel::Diffusers` as package
  evidence.
- Allowed write set: `crates/inference/src/backend/compatibility.rs` and this
  plan directory.
- No-fallback/no-legacy confirmation: this slice does not introduce a
  selectable Diffusers backend or alias Diffusers to PyTorch. It clarifies that
  PyTorch is the executable backend and Diffusers is package/source evidence.
- Verification passed: `cargo test -p inference
  diffusers_bundle_model_index_satisfies_image_generation_preprocessing --lib`
  and `cargo test -p inference backend::compatibility --lib`.
- Remaining follow-up: continue the broader package/dependency-key audit across
  runtime display, dependency diagnostics, and scheduler-selected backend facts.

2026-05-16 gateway image option-diagnostic reconciliation slice:

- Smallest useful vertical slice: align gateway-level image option diagnostics
  with the current planned image-generation planner contract so known
  unsupported image traits are not reported as honored or mapped before the
  planned boundary rejects execution.
- Allowed write set: `crates/inference/src/gateway.rs`,
  `crates/inference/src/gateway_tests.rs`, and this plan directory.
- No-fallback/no-legacy confirmation: this slice only changes diagnostic
  projection for image-generation option support. It does not restore raw
  image-generation execution, add request-only backend dispatch, introduce
  compatibility aliases, accept explicit denoising scheduler changes, or route
  opaque `extra_options` around planner validation.
- Result: gateway option diagnostics now report explicit
  `image.denoising_scheduler`, `image.init_image`, `image.mask_image`,
  `image.strength`, and image-scoped opaque `extra_options` as unsupported
  until those traits are modeled in family/runtime rules and worker contracts.
  Supported current text-to-image fields such as width, seed, prompt-related
  options, and image count continue to report honored when they are present.
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
- Remaining follow-up: planned lifecycle compatibility issues and option
  diagnostics now agree for currently unsupported image traits, but future
  support for denoising scheduler overrides, img2img/inpaint, or opaque
  options still requires typed family option rules, provider rows where
  user-facing, and worker envelope fields before those values can execute.

2026-05-16 planner component ambiguity guardrail slice:

- Smallest useful vertical slice: make the side-effect-free
  image-generation planner reject multiple present Pumas/Diffusers component
  sources for any required role in the selected supported family.
- Allowed write set: `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`, and this plan
  directory.
- No-fallback/no-legacy confirmation: ambiguous component facts now produce
  the typed `ambiguous_component_role` planner diagnostic. The planner does
  not pick a source by order, model id, display name, path shape, or Diffusers
  generic loading behavior, and it does not attempt alternate backends or
  worker dispatch after ambiguity.
- Result: Stable Diffusion, the only currently executable image family in this
  planner slice, fails closed when a required role such as `vae` has multiple
  present sources. Future executable families with multi-encoder or
  role-specific component layouts must add explicit family requirement rows and
  ambiguity fixtures before they execute.
- Verification passed: `cargo test -p inference
  planner_rejects_ambiguous_component_role_sources_without_heuristic_selection
  --lib`, `cargo test -p inference image_generation_planner --lib`, `cargo
  check -p inference`, `cargo fmt -p inference -- --check`, and `git diff
  --check`.
- Remaining follow-up: FLUX, FLUX.2, Qwen Image, Lumina Image, GLM Image,
  Z-Image, and SDXL are still unsupported families; enabling them requires
  explicit component-role requirement tables and ambiguity tests for their
  package shapes.

2026-05-16 planner selected-task guardrail slice:

- Smallest useful vertical slice: require the scheduler-owned
  `BackendExecutionDecision` consumed by image planning to select
  `image_generation` explicitly before producing an image execution plan.
- Allowed write set: `crates/inference/src/image_generation_planner.rs`,
  `crates/inference/src/image_generation_planner_tests.rs`, and this plan
  directory.
- No-fallback/no-legacy confirmation: a mismatched or missing selected task now
  fails with the typed `selected_task_mismatch` planner diagnostic. The
  planner does not repair the scheduler decision from the request task, package
  task evidence, active backend state, or graph hints, and it does not dispatch
  to worker execution after a mismatch.
- Verification passed: `cargo test -p inference
  planner_rejects_scheduler_decision_for_non_image_task --lib`, `cargo test
  -p inference image_generation_planner --lib`, `cargo check -p inference`,
  `cargo fmt -p inference -- --check`, and `git diff --check`.
- Remaining follow-up: this closes the selected-task consistency gap only.
  Dependency-environment readiness and path-root validation still need their
  own slices before real worker execution is considered complete.

2026-05-16 milestone checklist reconciliation slice:

- Smallest useful vertical slice: reconcile stale Milestone 6 checklist rows
  for already-implemented planner and compact image-output behavior so the
  remaining unchecked work reflects unresolved implementation rather than old
  bookkeeping.
- Allowed write set: this plan directory only.
- No-fallback/no-legacy confirmation: this slice changes only plan status. It
  does not change executable code, restore raw image generation, add fallback
  backend/runtime selection, or weaken planner/node-engine diagnostics.
- Verified completed boundaries: `plan_image_generation_execution` consumes
  `ImageGenerationRequest`, Pumas package facts, and scheduler-owned
  backend/device decisions to return either one `ImageGenerationExecutionPlan`
  or typed diagnostics; node-engine planned image execution keeps the full
  image body on the `image` output and projects compact image summaries through
  `results` without duplicate `data_base64` bodies.
- Verification passed: `cargo test -p inference image_generation_planner
  --lib` passed in the immediately preceding planner slices, and this
  reconciliation reran `cargo test -p node-engine --features
  inference-nodes,pytorch-nodes
  test_canonical_llm_image_generation_uses_planned_gateway_boundary --lib`.
- Remaining follow-up: artifact-store retention still needs an end-to-end
  retained-output workflow fixture. The top-level planner checklist remains a
  contract umbrella; dependency readiness and full backend capability
  projection still have their own unchecked rows.

2026-05-16 runtime evidence checklist reconciliation slice:

- Smallest useful vertical slice: reconcile completed runtime-selection
  checklist rows after verifying the inference execution-evidence boundary and
  embedded-runtime technical-fit adapter now enforce the planned graph runtime
  semantics.
- Allowed write set: this plan directory only.
- No-fallback/no-legacy confirmation: this slice changes only plan status. The
  verified code path still treats graph runtime as scheduler/admission input,
  not node-engine or worker execution input; `diffusers` remains package and
  dependency evidence and is not aliased to PyTorch for an explicit graph
  runtime request.
- Verified completed boundaries: Diffusers image-generation package facts
  produce a PyTorch executable candidate only when PyTorch backend capability
  facts advertise Diffusers support; explicit `runtime = pytorch` filters to
  the validated PyTorch candidate; explicit `runtime = diffusers` produces
  typed diagnostics and no PyTorch alias; omitted runtime leaves validated
  candidates for scheduler ranking.
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

2026-05-16 re-plan boundary after runtime evidence reconciliation:

- Boundary: do not begin the next code slice until the remaining Milestone 6
  audit/family/worker/artifact work is re-scoped. The remaining unchecked rows
  are no longer isolated planner guardrails; they cross runtime identity,
  workflow capability extraction, diagnostics fixtures, backend-owned
  port-option facts, PyTorch worker readiness, path-root validation,
  artifact-store retention, and family adapter ownership.
- Audit findings from the post-slice search:
  `pantograph-runtime-identity` still reserves `diffusers` as a canonical
  runtime/backend spelling and display label for a potential real backend;
  workflow capability extraction now uses only `llm-inference.runtime` as a
  hard inference runtime requirement, but still scans generic `backend_key`
  values for non-`llm-inference` node families and GGUF evidence; embedded
  runtime diagnostics/metrics fixtures still mention observed `diffusers`
  runtime ids; package/diffusers labels still appear correctly as evidence in
  inference compatibility and execution-evidence tests.
- Planning needed before code:
  decide whether `diffusers` stays reserved in runtime identity only for a
  future real executable backend or is removed from display/alias helpers until
  such a backend exists; decide whether generic non-inference `backend_key`
  extraction is still canonical for ONNX/audio/etc. or should be replaced by
  family-specific runtime inputs; decide whether diagnostics/metrics fixtures
  mentioning observed `diffusers` are stale pseudo-runtime fixtures or future
  real-backend fixtures; define the compact Pumas/runtime fact source for
  `llm-inference.denoising_scheduler` provider rows; define the approved
  Pumas/model root contract for artifact path validation; define the
  dependency-readiness owner for `diffusers`, `transformers`, `accelerate`,
  `torch`, and Pillow before worker dispatch.
- No-fallback/no-legacy confirmation: do not satisfy these rows by hardcoding
  a denoising scheduler list, preserving pseudo-Diffusers runtime candidates,
  restoring recursive inference `backend_key` selection, accepting local paths
  without a root contract, or allowing the PyTorch worker to discover missing
  dependencies as the first readiness signal.
- Verification before this boundary: `cargo test -p inference
  execution_evidence --lib`, `cargo test -p pantograph-embedded-runtime
  technical_fit_execution_evidence --lib`, `cargo test -p
  pantograph-embedded-runtime technical_fit --lib`, and targeted code search
  for `diffusers`, `backend_key`, `recommended_backend`, and backend hints.

2026-05-16 re-plan decisions for inference traits, runtime identity, and
readiness:

- Transformers/Diffusers naming decision: Pantograph uses the local
  Transformers reference checkout at
  `/media/jeremy/OrangeCream/Linux Software/repos/reference/frameworks-libraries/transformers`
  as the naming and convention reference for model/task traits, package
  component names, and what facts are needed to use Transformers- and
  Diffusers-family models. Diffusers must not be treated as image-generation
  only; it can describe image, text, audio, or future diffusion-model
  conventions. Pantograph-owned Rust contracts still translate those
  conventions into typed scheduler/runtime/device decisions before execution.
- Diffusers identity decision: keep `diffusers` as a user-facing
  source/package/capability/future-runtime label, but it is scheduler-selectable
  only when a real executable runtime registers installed and available
  capability facts for `diffusers`. Until then, `diffusers` may appear as
  unavailable, not implemented, not installed, unsupported platform, missing
  dependency, disabled by policy, or missing model/runtime facts, and those
  states must be typed facts rather than hidden strings or fallback behavior.
- Typed graph input decision: replace generic recursive `backend_key`
  discovery with explicit typed runtime/trait inputs for each node family as
  those families are brought onto the canonical scheduler path. `runtime`
  remains graph-authored intent and is projected into scheduler/admission
  requirements; future explicit traits such as `device`, `denoising_scheduler`,
  `dtype`, adapter selection, tokenizer/chat-template variants, attention
  backend, pooling strategy, audio voice, or other model/runtime-dependent
  traits must follow the same typed-input pattern. Node composition should stay
  low-boilerplate with hints/options, while explicit settings remain possible
  and reproducible.
- Availability contract decision: inference can expose traits or runtimes that
  are intentionally present in the crate but not yet usable. These must be
  represented as typed capability facts with precise availability, not removed
  or selected. The scheduler must treat unavailable facts as non-selectable,
  and graph/port-option providers may show them disabled with the reason.
  Required availability states include at least available, not installed, not
  implemented, unsupported platform, missing dependency, disabled by policy,
  missing model facts, requires runtime capability, and requires model
  capability.
- Diagnostics contract decision: runtime and capability diagnostics must be
  explicit about the runtime/candidate/capability they describe. Capability
  facts are the factual source of truth: they say which runtime, package,
  model, or trait exists and whether it is available, not installed, not
  implemented, unsupported, missing dependencies, disabled by policy, or
  blocked by missing model/runtime facts. Scheduler/admission diagnostics are
  the decision trail: they say which concrete candidate runtime/backend/variant
  was selected or rejected for a workflow and why. Provider-facing facts for
  graph editors use the same source facts but must identify the trait id,
  runtime/model scope, availability state, and disabled-display reason. Do not
  make provider availability messages, scheduler diagnostics, or lifecycle
  events the source facts themselves, and do not require message-string parsing
  to recover the runtime/candidate/trait that a diagnostic describes.
- Pumas root/path decision: "roots" means approved filesystem/storage base
  locations for Pumas-managed model artifacts, such as the Pumas
  `shared-resources/models` tree. Worker execution must receive a typed Pumas
  model/artifact reference, a validated root-relative artifact path, or a
  resolved path that has already been checked against approved Pumas/model
  roots. Arbitrary graph/user/local paths, path traversal, and unapproved
  temporary/download paths must fail with typed diagnostics before worker
  dispatch.
- Dependency readiness decision: missing `diffusers`, `transformers`,
  `accelerate`, `torch`, Pillow, or other runtime package dependencies must be
  reported by a readiness owner before worker dispatch. The PyTorch worker must
  not be the first component to discover these dependencies are missing. The
  next planning slice must choose whether the readiness owner is
  embedded-runtime, inference backend capability facts, managed runtime, or a
  PyTorch bridge preflight shell, and must define the reduced typed facts
  passed to scheduler/planner/gateway.
- Codebase review findings after these decisions:
  - `inference::execution_evidence` is the correct reusable boundary for
    separating executable backend candidates from package/dependency evidence,
    graph runtime constraints, runtime capability facts, and display labels. Do
    not move that normalization into image planning, node-engine,
    workflow-service, or scheduler ranking.
  - `pantograph-embedded-runtime::task_executor::dependency_environment` still
    contains legacy dependency-preflight behavior that can derive backend keys
    from package hints, legacy dependency requirements, or node-type defaults.
    That path must be replaced for canonical inference execution rather than
    expanded; package hints may feed execution evidence, but dependency
    readiness must not become a second scheduler/runtime-selection path.
  - `node_engine::PortOption` and TypeScript mirrors currently lack first-class
    disabled/unavailable fields. Do not encode availability in metadata or
    label strings. The graph editor needs typed disabled options so unavailable
    runtime traits can be visible, greyed out, and non-selectable with a
    precise reason.
  - `selection-input` provider context is structurally good because it carries
    stable refs and summary cursors instead of full Pumas facts, but backend or
    runtime context must come from scheduler/runtime context or runtime overlay,
    not persisted structural graph data. Avoid writing scheduler decisions into
    graph nodes to satisfy provider queries.
  - `ImageGenerationExecutionPlan` and the PyTorch image worker contract still
    carry `artifact_entry_path` as a raw string. Introduce a validated
    Pumas/root-relative artifact path contract before dispatch, and reject
    arbitrary local paths, traversal, and unapproved storage roots at the owning
    Rust boundary.
  - `pantograph-runtime-identity` may keep `diffusers` as reserved spelling,
    but display/selectability must come from runtime capability facts. A display
    label must not imply an executable Diffusers sidecar when only PyTorch is
    registered as the executable backend.
- Ordered implementation slices required before worker-ready image generation:
  - [x] Add a shared typed availability contract for runtime/trait/package
    capability facts. Required states include available, not installed, not
    implemented, unsupported platform, missing dependency, disabled by policy,
    missing model facts, requires runtime capability, and requires model
    capability. Project it to scheduler diagnostics and port-option rows
    through existing DTOs instead of adding a parallel diagnostic envelope.
    - 2026-05-16 slice: added `inference::capability_availability` as a pure
      DTO/validation boundary with typed subject kinds, the required
      availability states, validated primitive ids, bounded single-line
      reasons, serde round-trip coverage, and README ownership notes. No
      scheduler ranking, UI projection, or fallback runtime path was added.
  - [x] Extend `PortOption` and all Rust/TypeScript/interop mirrors with
    append-only disabled/unavailable fields and focused node tests. Provider
    rows must keep primitive option ids separate from presentation labels and
    must not hide disabled state in metadata.
    - 2026-05-16 slice: added append-only `disabled`,
      `unavailableState`, `unavailableReasonCode`, and
      `unavailableReason` fields to the Rust `node_engine::PortOption`
      contract, mirrored them in TypeScript workflow and svelte-graph
      contracts, and propagated them through selection-input provider
      normalization and rendered disabled option rows. Focused tests verify
      serde defaults, typed disabled-state serialization, metadata separation,
      stale-response handling, and selected disabled rows remaining visible.
  - [x] Replace canonical inference dependency readiness with a single owner:
    inference declares runtime/package dependency requirements; embedded-runtime
    or managed-runtime resolves installed/readiness facts; scheduler/admission
    consumes reduced readiness facts; inference planner/gateway refuses
    non-ready decisions; the PyTorch worker only receives already-approved
    execution envelopes. Remove canonical inference reliance on
    `dependency_environment` backend-hint/default backend selection.
    - 2026-05-16 re-plan decision: remove the legacy behavior rather than
      preserving it behind compatibility shims. The replacement owner split is:
      inference declares typed runtime/package requirements for each task,
      model family, and runtime bridge; embedded-runtime resolves local
      installed/readiness facts for now; scheduler/admission consumes the
      reduced typed readiness facts when selecting candidates; inference
      planner/gateway rejects non-ready scheduler decisions; workers only
      receive already-approved execution envelopes. A later managed-runtime
      migration may move the local resolver behind managed-runtime, but graph,
      scheduler, inference, and worker contracts must not change when that
      ownership moves.
    - Legacy removal map for this row:
      - Remove canonical inference reliance on
        `task_executor::dependency_environment::infer_backend_key`.
      - Remove backend selection from Pumas package hints,
        `recommended_backend`, `runtime_engine_hints`, dependency
        requirements, or node-type defaults in dependency preflight.
      - Remove `BackendHintLabel::Diffusers => "diffusers"` as an executable
        backend selection path; `diffusers` remains package/source/future
        runtime evidence until a real executable runtime registers available
        facts.
      - Remove local Python fallback allowances that let canonical inference
        execute when dependency bindings or runtime packages are missing.
      - Remove worker-side package discovery as the first readiness signal for
        `diffusers`, `transformers`, `accelerate`, `torch`, Pillow, or future
        runtime packages.
    - Replacement contract shape:
      - Add a typed dependency-readiness fact/projection using the shared
        availability states and validated ids. Facts identify runtime/backend,
        runtime variant when applicable, package/dependency id, task/family
        scope when applicable, availability state, stable reason code, bounded
        reason text, and resolver owner.
      - Dependency-readiness facts must attach to scheduler-facing candidate
        data and the final selected execution decision as typed readiness
        proof. They must not be represented only as late diagnostics,
        lifecycle events, worker errors, `supports_runtime_requirements`, or
        device diagnostics.
      - Keep `CapabilityAvailabilityFact` as the shared factual availability
        primitive, but add an explicit dependency-readiness projection or
        wrapper when scheduler candidates need additional scope such as
        runtime/backend id, runtime variant id, task id, model family,
        package/dependency id, resolver owner, reason code, and bounded reason
        text. Do not overload one primitive id string with combined model,
        runtime, package, and task scope.
      - Inference declares package requirements for PyTorch/Diffusers image
        execution (`diffusers`, `transformers`, `accelerate`, `torch`, Pillow)
        without probing the local Python environment.
      - Embedded-runtime resolves those declarations into readiness facts using
        installed/runtime state and existing diagnostic channels. Python
        executable presence only proves the Python sidecar shell exists; it
        does not prove PyTorch/Diffusers package readiness. The resolver does
        not rank candidates or select runtimes.
      - Scheduler/admission treats unavailable dependency facts as
        non-selectable candidate evidence. If an explicit graph runtime was
        requested, the scheduler either selects that runtime with ready facts
        or fails closed with diagnostics. If runtime is implicit, the scheduler
        may choose among only ready candidates.
      - Inference planner/gateway validates that the scheduler-selected runtime
        carries ready dependency facts before building a worker envelope. The
        planner/gateway may validate the selected proof, but must not rerun
        local package probes or make an independent runtime selection.
      - If `dependency-environment` remains useful for explicit diagnostics or
        tooling workflows, split or restrict that path so canonical inference
        code cannot import it for backend selection, dependency readiness, or
        fallback execution.
    - Staged implementation plan:
      1. [x] Add the dependency-readiness DTO/projection and focused serde/default
         tests without changing selection behavior.
         - 2026-05-16 slice: added
           `inference::DependencyReadinessFact` plus typed subject-kind and
           resolver-owner enums. The contract scopes package/dependency
           readiness to runtime/backend id, optional runtime variant, optional
           task, optional model family, availability state, resolver owner,
           reason code, and bounded reason text. Focused tests cover serde
           shape, optional-scope defaults, scoped-id validation, and projection
           back to `CapabilityAvailabilityFact`. No scheduler filtering,
           runtime selection, environment probing, or worker behavior changed.
      2. [x] Add inference-owned PyTorch/Diffusers image package requirement
         declarations and tests proving they are factual declarations, not
         local probes or scheduler policy.
         - 2026-05-16 slice: added
           `inference::dependency_requirements` with typed
           `DependencyRequirementDeclaration` and
           `DependencyRequirementNecessity` contracts. The PyTorch/Diffusers
           image-generation declaration lists required `diffusers`,
           `transformers`, `accelerate`, `torch`, and `pillow` packages scoped
           to `pytorch` and `image_generation`, and can project externally
           resolved states into `DependencyReadinessFact`. Tests prove the
           declarations are required package facts with validated ids and no
           local probes, scheduler ranking, candidate filtering, or worker
           dispatch.
      3. [x] Add embedded-runtime readiness resolution from those declarations into
         typed facts and existing diagnostics.
         - 2026-05-16 slice: added
           `pantograph-embedded-runtime::dependency_readiness` as a pure
           resolver from inference-owned dependency declarations plus a
           host-observed Python package snapshot into
           `DependencyReadinessFact` values. The resolver reports available
           packages, missing packages, unavailable Python runtime state, and
           unsupported declaration kinds with typed availability states, stable
           reason codes, and bounded reason text. It does not probe Python,
           install packages, rank candidates, select runtimes, or attach facts
           to scheduler candidates yet.
      4. Attach dependency-readiness facts to runtime-registry/admission
         candidates and selected execution decisions as typed proof. Add tests
         proving unavailable readiness cannot be hidden behind
         `supports_runtime_requirements`, device diagnostics, lifecycle
         diagnostics, or worker errors.
         - 2026-05-16 contract-carriage slice: added a reduced
           runtime-registry `RuntimeTechnicalFitDependencyReadinessFact`
           contract and attached it to `RuntimeTechnicalFitCandidate` and
           `RuntimeTechnicalFitDecision`. The selector copies the selected
           candidate's dependency-readiness proof into the decision while
           leaving ranking/filtering behavior unchanged for the next slice.
           Tests prove normalization preserves scoped dependency proof and that
           selected decisions carry the proof rather than relying on
           `supports_runtime_requirements` or device diagnostics. This is a
           partial completion of stage 4 because workflow/admission execution
           plans still need to carry the selected proof into inference.
         - 2026-05-16 adapter projection slice: extended the
           embedded-runtime execution-evidence adapter input with supplied
           `inference::DependencyReadinessFact` values and projected matching
           facts onto runtime-registry candidates as reduced
           `RuntimeTechnicalFitDependencyReadinessFact` proof. The adapter
           matches facts by executable backend key, optional runtime variant,
           and task id, preserving dependency id, state, resolver owner,
           model-family scope, reason code, and reason text. This still leaves
           production host package snapshots and workflow/admission execution
           plan propagation for later slices; no scheduler filtering changed.
         - 2026-05-16 request-builder proof input slice: extended
           embedded-runtime technical-fit request construction so
           `build_runtime_technical_fit_request_with_backend_package_facts`
           receives explicit dependency-readiness facts and passes them to the
           execution-evidence adapter. Focused coverage proves the
           backend-checked Pumas candidate path carries PyTorch/Diffusers
           readiness proof when the caller supplies it. Production
           `workflow_technical_fit_decision` still passes an empty fact slice
           until the host package snapshot source is wired; scheduler
           filtering remains unchanged.
         - 2026-05-16 workflow execution-plan proof propagation slice:
           extended workflow-service technical-fit and run execution-plan DTOs
           with workflow-owned dependency-readiness proof, copied selected
           proof during admission, and projected it at the embedded-runtime
           composition boundary into inference `BackendExecutionDecision`
           values. Focused tests prove runtime-registry selected proof reaches
           workflow technical-fit decisions, workflow execution plans, and
           inference-owned dependency-readiness facts without using graph
           inputs, diagnostics strings, worker discovery, or hidden fallback
           channels. Stage 4 proof carriage is complete; production host
           package snapshots still need to supply real facts before scheduler
           filtering can safely become blocking.
      5. Wire scheduler/admission candidate filtering to consume readiness
         proof and fail candidate selection while emitting ledger diagnostics
         when required dependencies are unavailable.
         - 2026-05-16 runtime-registry readiness filtering slice: runtime
           technical-fit selection now treats explicit unavailable
           dependency-readiness proof on a candidate as ineligible. Automatic
           selection skips unavailable candidates, no-valid-candidate outcomes
           surface `evidence_required_package_unavailable` diagnostics with
           runtime/backend/task/dependency attribution, and explicit overrides
           report the same typed dependency diagnostic instead of falling back
           to a generic backend-incompatible result. Empty proof remains
           non-blocking until production host package snapshots are wired and
           the planner/gateway missing-proof gate can be enabled without
           rejecting every current production candidate.
         - 2026-05-16 package-readiness provider planning decision: use a
           runtime-scoped package-readiness provider contract owned by
           embedded-runtime as the production source for scheduler/admission
           dependency proof. The provider is keyed by executable backend/runtime
           identity and optional runtime variant/environment selector, consumes
           inference-owned dependency declarations, and returns typed
           `DependencyReadinessFact` values plus bounded provider diagnostics.
           It must not reuse execution-time dependency-environment preflight,
           graph node inputs, worker imports, or package-hint strings as
           scheduler proof. The first implementation may resolve the default
           PyTorch/Python environment only, but the contract must keep
           environment identity explicit enough for a later managed
           environment inventory objective. That later objective will model
           runtime + environment + package inventory as first-class scheduler
           evidence across venv/Conda/managed/remote runtimes without changing
           scheduler policy call sites.
         - 2026-05-17 provider contract review update: implement the provider
           in a focused embedded-runtime module rather than growing
           `technical_fit.rs`. The input contract must name executable backend
           key, scheduler runtime id, runtime variant id, and package-readiness
           environment selector separately so `runtime_id`/`backend_key`
           ambiguity cannot become part of the API. Provider diagnostics must
           be typed enums/DTOs, not strings, and must cover unavailable Python,
           missing packages, unsupported dependency kinds, invalid package ids,
           unimplemented probes, unsupported platforms, timeouts, and probe
           process failures. The first provider may use only the default host
           Python/PyTorch environment, but it must still carry an explicit
           environment selector so the later managed-environment inventory can
           replace the source without changing scheduler policy call sites.
           Python probing must use a fixed no-shell command/script, bounded
           timeout, bounded output capture, request-local dedupe/cache keyed by
           backend/runtime/variant/environment/dependency set, and no locks
           held across awaits. Package checks must use inference-owned
           dependency ids as package/distribution ids or fail typed/closed when
           a package cannot be safely probed; do not infer readiness from
           imports, display labels, package hints, or Pumas advisory facts.
         - 2026-05-17 standards compliance tightening: provider DTOs must use
           validated domain types at the boundary (`BackendId`,
           `RuntimeVariantId`, `CapabilityAvailabilityId`, and a typed
           environment selector) rather than raw `String` fields for internal
           APIs. Fallible provider operations must return a specific typed
           error/diagnostic enum, not `Result<T, String>` or `anyhow`, and
           public provider return values should be `#[must_use]` where
           ignoring readiness would be a correctness bug. Keep normalization,
           request shaping, dedupe-key construction, and fact projection
           synchronous and side-effect-free; only the outer Python process
           probe may be async. The provider must not create a runtime, spawn
           untracked background tasks, or hold any sync/async lock while
           awaiting a probe. Any subprocess probe must use `tokio::process`
           with explicit args, `kill_on_drop`, timeout handling, bounded
           stdout/stderr capture, and typed process-status diagnostics. Tests
           that touch environment variables, temp paths, caches, or process
           probes must isolate or serialize that state; unit tests should use a
           fake probe runner so the contract suite does not depend on the
           developer machine's installed Python packages.
         - 2026-05-17 codebase blast-radius review update: implement the
           provider as a new focused embedded-runtime module, likely
           `package_readiness_provider.rs`, while keeping
           `dependency_readiness.rs` as the pure declaration/snapshot-to-fact
           projection module. `technical_fit.rs` may call a small helper to
           collect provider facts and pass them into existing technical-fit
           request construction, but must not absorb provider DTOs, probe
           orchestration, subprocess handling, or cache/dedupe state. The
           provider should wrap existing Python executable resolution behind
           typed provider diagnostics instead of leaking `Result<T, String>`
           from `python_runtime.rs`. Readiness should be built once per
           backend/runtime/variant/environment/dependency set and then
           projected onto matching candidates; do not probe per candidate.
           Add the fake probe runner and provider contract tests before the
           real Python runner. Treat any direct provider dependency on
           `task_executor::dependency_environment`, worker package imports, or
           graph input fallback data as a regression. Because the current
           execution-evidence adapter matches readiness using executable
           backend key, provider projection tests must explicitly prove the
           backend-key/runtime-id/environment fields are mapped intentionally
           and do not spread the existing `runtime_id`/`backend_key` ambiguity.
        - 2026-05-17 provider trait/DTO slice: added
          `pantograph-embedded-runtime::package_readiness_provider` as the
          focused runtime-scoped provider contract plus fake-runner contract
          tests. The provider request separates executable backend key,
          scheduler runtime id, optional runtime variant id, typed package
          environment selector, and inference-owned dependency declarations.
          Provider output is typed dependency-readiness facts plus bounded
          provider diagnostics for missing packages, unavailable Python,
          unsupported dependency kinds, invalid package ids, timeouts, and
          probe failures. Request-local dedupe is keyed by backend/runtime/
          variant/environment/dependency set, non-package declarations do not
          trigger empty Python probes, and the provider does not import
          `task_executor::dependency_environment`, inspect graph inputs, call
          worker package imports, select runtimes, rank candidates, or dispatch
          workers. This slice intentionally stops before the real no-shell
          Python probe runner and before production technical-fit wiring.
        - 2026-05-17 no-shell Python probe runner slice: added
          `pantograph-embedded-runtime::python_package_readiness_probe` as the
          real process-backed default-host Python package probe runner behind
          the provider trait. The runner uses explicit `tokio::process`
          command args, `kill_on_drop`, a bounded timeout, bounded stdout/stderr
          capture, typed `PythonUnavailable`, `InvalidPackageId`,
          `ProbeNotImplemented`, `ProbeTimedOut`, and `ProbeProcessFailed`
          diagnostics, and validates package ids before process launch. It
          rejects explicit managed Python environments as not implemented until
          managed environment inventory exists. The runner does not call
          dependency-environment preflight, graph inputs, worker imports,
          runtime ranking, candidate selection, or worker dispatch, and it is
          still not wired into production technical-fit calls in this slice.
        - 2026-05-17 production technical-fit readiness collection slice:
          added `technical_fit_package_readiness.rs` and wired
          `workflow_technical_fit_decision` to collect package-readiness facts
          through `PackageReadinessProvider<ProcessPythonPackageReadinessProbeRunner>`
          before building runtime-registry technical-fit requests. The helper
          normalizes package facts through inference execution evidence,
          creates provider requests only for validated PyTorch image-generation
          candidates, dedupes identical backend/runtime/environment/dependency
          requests before provider resolution, and passes returned readiness
          facts into the existing execution-evidence adapter. The slice does
          not enable the image planner/gateway missing-proof gate and does not
          call dependency-environment preflight, worker imports, graph inputs,
          runtime ranking, or worker dispatch.
      6. [x] Make image planner/gateway reject selected decisions that lack ready
         dependency proof before worker dispatch.
         - 2026-05-17 image planner missing-proof gate slice: extended the
           side-effect-free image-generation planner to require ready
           scheduler-carried PyTorch/Diffusers dependency-readiness proof for
           `diffusers`, `transformers`, `accelerate`, `torch`, and `pillow`
           before producing an execution plan. Missing proof now returns typed
           `missing_dependency_readiness_proof` diagnostics, unavailable proof
           returns typed `dependency_readiness_unavailable` diagnostics, and
           gateway planning tests now use ready scheduler proof rather than an
           empty proof vector. The gate does not probe packages, inspect graph
           inputs, select runtimes, or fall back to worker-side discovery.
      7. [x] Remove the legacy dependency-environment backend-key and fallback
         selection paths from canonical inference execution.
         - 2026-05-17 embedded-runtime dependency preflight fail-closed slice:
           removed canonical backend selection from dependency preflight
           request construction for `backend_key` inputs, Pumas package
           backend hints, dependency-requirement backend keys, and node-type
           defaults. Package facts remain model/task evidence only, and
           dependency-requirement data may still supply non-selection
           diagnostic/tooling context. Canonical Python-backed execution now
           strips legacy backend-key fields before adapter dispatch and records
           runtime identity from the resolved model ref or node family.
      8. [x] Retire `dependency-environment` from canonical inference execution or
         restrict it to explicit diagnostic/tooling workflows that cannot
         influence runtime selection.
         - 2026-05-17 embedded-runtime dependency preflight fail-closed slice:
           kept the `dependency-environment` node as an explicit
           diagnostic/tooling workflow step that can emit dependency
           requirements, status, and environment refs, but prevented its
           backend-key paths from influencing canonical Python-backed
           inference. Missing dependency bindings or missing runtime packages
           now block before adapter execution instead of resolving a local
           Python fallback model ref.
    - Required tests:
      - Package hints, `recommended_backend`, runtime hints, dependency
        requirements, and node-type defaults do not select executable
        backends for canonical inference.
      - `diffusers` package evidence does not become a scheduler-selectable
        executable runtime unless a real executable runtime registers ready
        facts.
      - Missing `torch`, `diffusers`, `transformers`, `accelerate`, or Pillow
        blocks before worker dispatch and records runtime-scoped diagnostics.
        - 2026-05-17 status: image-generation planner tests now cover missing
          scheduler dependency-readiness proof and unavailable dependency
          proof before worker dispatch.
      - Scheduler candidate and selected decision fixtures carry explicit
        dependency-readiness proof for selected PyTorch/Diffusers execution;
        diagnostic-only readiness is rejected as insufficient.
      - Python sidecar availability does not mark PyTorch/Diffusers image
        execution ready unless the required package facts are available.
      - Explicit graph runtime requirements fail closed when that runtime has
        unavailable dependency facts.
      - Implicit runtime selection skips unavailable candidates and only ranks
        candidates with ready dependency facts.
      - Worker tests prove missing packages are not first discovered inside the
        PyTorch worker for canonical image generation.
      - Package-readiness provider contract tests cover available packages,
        missing package, unavailable Python, unsupported dependency kind,
        invalid/unprobeable package id, timeout/probe failure, request-local
        dedupe, and production technical-fit propagation of provider facts.
        - 2026-05-17 status: provider contract tests now cover available
          packages, missing package, unavailable Python, unsupported dependency
          kind, invalid package id, timeout/probe failure, and request-local
          dedupe. Production technical-fit collection now has focused helper
          coverage proving PyTorch/Diffusers readiness facts are collected from
          a provider and supplied to the technical-fit path; the selected
          decision missing-proof gate remains for a later slice.
        - 2026-05-17 status: task-executor tests now prove canonical
          dependency preflight ignores explicit backend-key inputs, package
          backend hints, dependency-requirement backend keys, and node-type
          defaults, and that no-bindings or missing-runtime-package statuses
          block before Python adapter dispatch.
    - No-fallback/no-legacy confirmation: do not keep dependency-environment
      backend selection as a fallback, do not alias Diffusers package evidence
      to PyTorch or a pseudo-Diffusers executable runtime, and do not allow a
      worker to be the first component to discover missing canonical runtime
      packages.
  - [x] Introduce a validated Pumas artifact/root path type or DTO at the
    workflow execution-plan/admission projection boundary and carry only that
    proof, a root-relative artifact path, or an already root-validated resolved
    path into inference and worker envelopes.
    - 2026-05-17 root-relative artifact path slice: added
      `inference::PumasArtifactEntryPath` as a validated root-relative,
      non-traversing artifact entry path. Image planning now rejects empty,
      absolute/local, traversing, URI-shaped, control-character, and overlong
      artifact entry paths with typed planner diagnostics before producing an
      execution plan. The PyTorch image worker request carries the same
      validated type while preserving the JSON string wire shape. This covers
      the root-relative artifact path option; a later Pumas/model root
      inventory can add already root-validated resolved-path proof without
      changing worker-envelope call sites.
  - [x] Reconcile reserved `diffusers` runtime identity and diagnostics
    fixtures: preserve the canonical spelling only as package/source/future
    runtime identity, remove or mark misleading sidecar display strings until a
    real executable runtime registers, and ensure diagnostics/metrics fixtures
    distinguish package evidence from observed executable runtimes.
    - 2026-05-17 reserved Diffusers identity cleanup slice: kept
      `diffusers` as a canonical reserved runtime spelling, changed its
      display label so it no longer claims a Python sidecar implementation,
      and removed bare `diffusers` from embedded-runtime diagnostics/metrics
      fixtures that represented observed executable runtime ids. PyTorch image
      execution continues to use `pytorch` as the executable runtime and
      `pytorch.diffusers` only as runtime-variant context.
  - [x] Replace remaining generic recursive `backend_key` discovery for each
    node family as that family moves onto canonical scheduler-owned inference
    execution. Do not create a new broad scanner; add explicit typed runtime or
    trait inputs per family and fail closed with diagnostics when the typed
    contract is absent or invalid.
    - 2026-05-17 workflow-service slice: removed the broad recursive
      backend-key scanner from `capabilities.rs`; `puma-lib` package facts,
      dependency-environment tooling inputs, unknown nodes, and nested metadata
      no longer become scheduler required-backend values. Remaining
      graph-visible runtime selection for canonical inference is the typed
      optional `runtime` input that scheduler/admission interprets.
- Standards-compliance requirements for the ordered slices:
  - Shared contracts are serial integration-owner work. Availability facts,
    `PortOption` disabled state, dependency-readiness facts, validated
    artifact/root DTOs, runtime identity display semantics, public diagnostic
    enums, Tauri/UniFFI/Rustler mirrors, TypeScript mirrors, fixtures, README
    notes, and ADRs must not be split across parallel write sets unless an
    explicit integration owner is recorded first.
  - Boundary values must be correct by construction. New runtime ids, trait ids,
    availability states, dependency ids, provider-context ids, and
    Pumas/root-relative artifact paths must use typed constructors or
    `TryFrom`/`FromStr` validation with specific error enums. Internal code must
    accept validated values rather than repeatedly re-validating raw `String`,
    `PathBuf`, or JSON fields.
  - Public wire contracts must evolve append-only. Public enums/DTOs expected
    to grow should use serde-compatible defaults and `#[non_exhaustive]` where
    appropriate; projection code must match known variants explicitly and must
    not collapse new states into display strings, metadata blobs, or generic
    "unknown" diagnostics unless the unknown state is truly from a future
    producer.
  - Path safety must have one owner. Pumas/model root validation belongs in a
    focused Rust boundary module or DTO constructor; handlers, planners, and
    worker-envelope builders must not duplicate inline path traversal/root
    checks. Diagnostics may include stable model/artifact ids and bounded
    field paths, but must not leak arbitrary local paths when a stable Pumas id
    is available.
  - Dependency ownership must match execution boundaries. Runtime-package
    readiness checks for `diffusers`, `transformers`, `accelerate`, `torch`,
    Pillow, and future runtime packages must be declared by the owner that
    executes or manages them. Do not add broad workspace dependencies or depend
    on incidental Python/package-manager availability. Any new third-party
    dependency must be justified against the dependency standards before it is
    added.
  - Diagnostics must use existing owned channels. Scheduler/admission failures,
    provider unavailable options, dependency readiness failures, and path-root
    rejection must carry bounded structured fields through the existing
    runtime-registry, workflow-service, node-engine, and lifecycle diagnostic
    paths. Do not add a second diagnostic system or require message-string
    parsing for control flow.
  - Verification must follow the blast radius of each slice. Contract slices
    require Rust serde/default/round-trip tests plus affected Tauri, UniFFI,
    Rustler, TypeScript mirror, and fixture tests. Provider UI slices require
    the project-approved Node tests for disabled/unavailable options,
    accessible names, keyboard behavior, stale-response discard, and graph
    gesture containment. Scheduler/readiness/path slices require success and
    failure tests proving unavailable candidates are non-selectable, explicit
    graph runtime requirements fail closed, invalid roots/path traversal reject
    before worker dispatch, and no worker-side dependency discovery is the first
    readiness signal.
  - Documentation must move with public contract changes. Any new source
    module, shared DTO, public constructor, diagnostic enum, provider context
    field, or execution-plan boundary must update the owning README or add an
    ADR in the same slice, documenting ownership, parse-once invariants,
    append-only evolution, diagnostics semantics, and migration/removal of old
    behavior.
- No-fallback/no-legacy confirmation: these decisions do not allow
  pseudo-Diffusers runtime candidates, hardcoded frontend scheduler lists,
  recursive inference `backend_key` selection, raw local-path execution,
  worker-side dependency discovery as readiness policy, or direct graph-to-
  inference runtime selection.
