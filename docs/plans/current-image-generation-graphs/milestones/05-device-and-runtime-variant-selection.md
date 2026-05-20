# Milestone 5: Device And Runtime Variant Selection

**Goal:** Add the backend-owned device policy and runtime-variant contract
needed before expanding executable inference backends.

**Non-negotiable rule:** this milestone removes or replaces old device,
backend, runtime, and technical-fit execution paths. It must not preserve
fallback or legacy compatibility behavior. If the canonical scheduler-facing
contracts cannot produce one valid execution decision, execution fails with a
typed diagnostic and the canonical design is fixed.

**Tasks:**

- [x] Define `InferenceDevicePolicy`, `InferenceDeviceClass`,
  `InferenceDeviceId`, `RuntimeVariantId`, `RuntimeVariantCapability`,
  `DeviceResolutionRequest`, `DeviceResolutionDecision`, and
  `DeviceResolutionDiagnostic`.
- [x] Define `BackendExecutionCandidate` and `BackendExecutionDecision` as the
  scheduler-facing facts and selected execution choice. Candidate facts must
  include backend id, task/model compatibility, runtime variant, device class,
  concrete device id when known, static resource estimates when available,
  optional observed-throughput hints, and bounded diagnostics.
- [x] Land the device contract gate first, with strict parser/unit tests, before
  modifying managed runtime, registry, backend startup, or frontend behavior.
- [x] Record the standards and blast-radius gate before code changes begin for
  each touched crate/module: crate role, public facade impact, runtime
  lifecycle owner, persisted artifacts, path/resource validation needs,
  feature/dependency impact, frontend accessibility impact, and test isolation
  strategy.
- [x] Add structured error enums and `TryFrom`/`FromStr` parsing for validated
  device contracts. Public fallible APIs must not expose `Result<T, String>` for
  contract validation.
- [x] Ensure new public or cross-crate types derive or implement useful
  `Debug`, use explicit serde casing, use `#[non_exhaustive]` where additive
  extension is likely, and use `#[must_use]` for validated decisions/builders
  that should not be ignored.
- [ ] Add serde fixture tests for every device/runtime DTO crossing Rust crate,
  Tauri/frontend, diagnostics-ledger, Python worker, or persisted-state
  boundaries.
- [ ] Replace raw device strings crossing crate boundaries with validated
  policy/device/variant types where this plan touches execution planning.
- [ ] Inventory and remove or replace old executable selection paths before
  enabling new behavior: raw `DeviceConfig` execution, `DeviceBackend::from_id`
  `unknown -> auto`, malformed device ordinal `-> 0`, frontend fallback device
  options, executable technical-fit conservative fallback, override-fallback
  candidate synthesis, gateway active-backend inference, and node-engine
  backend routing that chooses independently of the scheduler decision.
- [x] Remove or replace graph-visible full Pumas package-fact wiring from
  inference templates/contracts before scheduler integration. Inference graph
  nodes may carry model reference, task, task options, and optional explicit
  backend/runtime/device intent; package facts must be resolved at the
  host/planning boundary and reduced into candidate facts or execution plans.
  - 2026-05-12: `puma-lib` and `llm-inference` descriptors, contract payload
    metadata, frontend mocks, bundled workflow templates, tracked current
    image-generation saved workflows, and node-engine dependency input repair
    now route only `pumas_model_ref` into canonical inference. Retired
    `resolved_model_source`, `resolved_model_package_facts`, and
    `model_package_facts` target handles are rejected/ignored at dependency
    input preparation instead of being merged as executable graph context.
    Package facts remain a host/planning boundary concern for candidate
    synthesis and image execution planning, not graph-visible inference ports.
- [ ] Add executable candidate synthesis before automatic policy selection.
  Candidate synthesis must join Pumas model/package facts, backend
  compatibility, runtime variant capability, device facts, resource estimates,
  readiness state, and typed diagnostics into selectable
  `BackendExecutionCandidate` values. Incomplete candidate fragments must fail
  candidate selection with ledger diagnostics instead of becoming fallback
  execution decisions.
  - 2026-05-12 partial: embedded-runtime technical-fit candidate synthesis now
    enriches Pumas-derived candidates with matching runtime capability facts,
    runtime variant id, device class, readiness/warmup state, resource
    estimate, and backend compatibility reports. Pumas candidates without
    matching runtime capability facts are non-selectable and carry typed
    `missing_runtime_variant` diagnostics unless backend compatibility already
    supplies a more specific blocking diagnostic. Remaining follow-up: promote
    these candidate facts into the scheduler policy trace/ledger summary path.
  - 2026-05-12 partial: runtime-registry automatic technical-fit selection now
    records policy version, candidate-set summary, ranking reason, seed basis,
    and selected candidate reasons. Equal-priority eligible candidates are
    selected through deterministic controlled exploration instead of terminal
    `ambiguous_auto_resolution`. Genuine no-candidate, invalid-candidate, and
    unrankable-policy states still fail with typed diagnostics.
  - 2026-05-14 partial: embedded-runtime generic runtime-capability candidate
    synthesis now emits one candidate per reported runtime variant through the
    shared variant fact expansion helper. Unavailable variants remain visible
    to policy as rejected diagnostic candidates instead of being hidden by the
    former first-available/first-variant collapse. Remaining follow-up:
    required Pumas fact absence and documented candidate-cap overflow still
    need typed candidate-synthesis diagnostics before trace/admission/history
    slices.
  - 2026-05-14 partial: missing required Pumas package facts now synthesize
    non-selectable `missing_model_package_facts` candidates, and automatic
    no-valid technical-fit decisions surface scoped candidate diagnostics
    instead of replacing them with a generic no-valid-candidate message. Host
    technical-fit planning no longer allows capability-only selection when a
    required model cannot be resolved to package facts. Remaining follow-up:
    documented candidate-cap overflow still needs a typed
    candidate-synthesis diagnostic before trace/admission/history slices.
  - 2026-05-14 partial: embedded-runtime candidate synthesis now enforces the
    documented cap of 512 technical-fit candidates before policy invocation.
    Oversized candidate sets are replaced by one non-selectable
    `candidate_set_overflow` diagnostic candidate so policy receives a bounded
    request and fails selection with the exact cause instead of truncating or
    saturating counts. Remaining follow-up: promote executable candidate facts
    into the scheduler policy trace and diagnostics-ledger summary path.
- [x] Add a common backend-adapter capability contract for llama.cpp, PyTorch,
  vLLM, Candle, and future MLX. The contract reports facts and performs
  backend-specific translation; it must not rank candidates across backends or
  own cross-workflow scheduling policy.
  - 2026-05-11 reconciliation: the existing `BackendCapabilityFacts`
    contract carries canonical task, model-source, feature, and
    `RuntimeVariantCapability` facts for executable inference adapters.
    llama.cpp, PyTorch, and Candle expose their facts through adapter
    `static_capabilities`; embedded-runtime roadmap capability facts expose
    vLLM CPU/CUDA and future MLX Metal as unavailable/non-executable facts.
    Ranking remains in scheduler/runtime-registry decisions, not in adapters
    or roadmap fact providers.
- [ ] Keep backend adapter startup/load functions lifecycle-owned by the
  embedded runtime or another explicit composition-root owner. Adapters must
  not create global Tokio runtimes, untracked tasks, unbounded worker queues, or
  self-owned long-lived subprocesses.
- [ ] Move current llama.cpp-specific device parsing/probing behind a
  llama.cpp adapter boundary. Do not preserve `unknown -> auto` or malformed
  ordinal `-> 0` behavior.
- [ ] Keep backend-specific device-string translation inside backend adapters.
  Planning, admission, runtime registry, diagnostics, and frontend contracts
  must not pass llama.cpp/PyTorch/vLLM raw device strings as trusted internal
  state.
- [x] Add a runtime-variant dimension to managed runtime catalog/status state
  without reintroducing duplicate binary-management systems. Keep
  `ManagedBinaryId::LlamaCpp` as the binary-management identity and nest
  `RuntimeVariantId` readiness under it.
  - 2026-05-11 partial: managed-runtime catalog versions, projected version
    statuses, and persisted installed versions now carry typed
    `RuntimeVariantId` values. Current llama.cpp managed installs default to
    `llama_cpp.cpu`; CUDA/Metal variant-specific installs and readiness remain
    pending.
  - 2026-05-11 partial: catalog/status projection now joins catalog entries
    to installed versions by `(version, runtime_variant_id)` rather than
    version only, so same-release CPU/CUDA entries stay distinct.
  - 2026-05-11: install validation, projected executable readiness, and
    command resolution now all receive the selected `RuntimeVariantId`.
    Persisted installed versions are upserted by `(version, runtime_variant_id)`
    so one managed binary can retain same-release CPU and CUDA entries without
    a second binary-management system.
- [x] Include runtime variant id on managed install jobs, retained job
  artifacts, progress snapshots, and install history entries. One active
  managed-binary install job at a time is acceptable initially if the job
  clearly identifies its target variant.
  - 2026-05-11: `ManagedRuntimeDownloadSource`, active job status, retained
    job artifacts, projected artifact status, progress DTOs, and install
    history entries now carry typed `RuntimeVariantId`. Selection rejects ready
    persisted versions that do not have a canonical runtime variant id instead
    of inferring one.
- [x] Update selected version, selected variant, active job, retained artifact,
  and readiness through one durable state-transition path. Do not split related
  state across independent locks or cancellation points.
  - 2026-05-11: managed runtime selection state now persists
    selected/active/default runtime variant ids next to the selected versions.
    Install, remove, remove-version, and selection update transitions mutate
    version and variant together, snapshot readiness/status matching uses
    `(version, runtime_variant_id)`, and strict install-dir resolution rejects
    missing selected version+variant pairs instead of resolving by version only.
- [x] Model llama.cpp CPU and CUDA builds on Linux/Windows as runtime variants
  for the same release version where artifacts or local installs expose those
  variants.
  - 2026-05-11 partial: Linux and Windows llama.cpp platform definitions now
    declare CPU and CUDA catalog variants for the same release archive, while
    macOS remains CPU-only until Metal readiness is modeled. Install readiness
    still needs variant-specific artifact validation before this item can close.
  - 2026-05-11 partial: Linux CUDA readiness now requires
    `cuda/llama-server`, `cuda/libllama.so`, and `cuda/libggml.so`; Windows
    accepts CPU/CUDA variant ids against its current root executable/DLL layout.
    Same-release CPU/CUDA installs no longer overwrite each other in persisted
    state. Remaining follow-up: public manual selection still accepts only a
    version string, so same-version variant selection needs a variant-aware
    command/API before this item can close.
  - 2026-05-11 partial: manual selected/default version updates now require an
    explicit `RuntimeVariantId` when a version has multiple installed variants,
    and the Tauri/frontend manager selection controls submit version+variant
    pairs. Remaining follow-up: install/download requests still accept only a
    version string, so same-release catalog install actions need variant-aware
    download-source selection before this item can close.
  - 2026-05-11: managed runtime install/download commands now accept
    version+variant intent, reject ambiguous same-version catalog requests
    without a `RuntimeVariantId`, and carry the selected variant through the
    frontend, Tauri, embedded-runtime manager, and backend download-source
    resolver. This closes Linux/Windows CPU/CUDA modeling for the current
    managed llama.cpp archive layout.
- [x] Model llama.cpp Metal builds on macOS only when a Metal-capable runtime
  is available.
  - 2026-05-11: macOS arm64/x64 managed llama.cpp platform definitions now
    advertise `llama_cpp.metal` alongside CPU, while Linux/Windows still expose
    only their CPU/CUDA variants. Metal validation and command resolution
    require `libggml-metal.dylib`; missing Metal runtime files return the
    canonical missing runtime-variant diagnostic instead of falling back to CPU.
- [x] Make llama.cpp command resolution select an explicit runtime variant
  before constructing `llama-server` arguments.
  - 2026-05-11: `resolve_binary_command` now resolves the persisted selected
    version and runtime variant as one install target before platform command
    construction. Linux CUDA command selection depends on selected
    `llama_cpp.cuda`, not raw `--device` arguments; missing selected variant
    files return `MissingRuntimeVariant` diagnostics. Linux/macOS/Windows
    platform resolvers reject unsupported variant ids instead of falling back.
- [x] Keep platform-specific executable names, archive names, dynamic-library
  paths, environment variables, and probes inside platform modules or narrow
  platform traits.
  - 2026-05-11: llama.cpp archive server filenames and runtime-variant
    extraction subdirectory mapping now live behind the `LlamaPlatform` trait.
    Linux/Windows own the CUDA archive subdirectory mapping; macOS keeps the
    default no-variant-subdirectory behavior. Shared extraction no longer
    hard-codes platform server filenames or CUDA layout decisions.
- [ ] Validate runtime roots, executable paths, dynamic-library paths, Pumas
  package paths, artifact paths, and worker-visible paths through shared
  allowed-root validation before filesystem or subprocess access.
  - 2026-05-11 partial: removed the legacy managed runtime root probe so
    `managed_install_dir` resolves only under the canonical
    `third-party/runtimes` tree. Shared allowed-root validation for executable,
    dynamic-library, Pumas package, artifact, and worker-visible paths remains
    pending.
  - 2026-05-11 partial: added the shared `pantograph-path-security`
    allowed-root validator and routed node-engine file IO/workflow persistence
    callers plus managed-runtime command resolution through it. Managed-runtime
    command resolution now validates selected install roots against the
    canonical managed runtime root before installation checks, and validates
    resolved executable and working-directory paths before command handoff.
    Dynamic-library environment path entries, pid files, Pumas package paths,
    artifact paths, and worker-visible paths remain pending.
  - 2026-05-11 partial: managed-runtime command construction now emits only
    owned library-search path values for `LD_LIBRARY_PATH`,
    `DYLD_LIBRARY_PATH`, and Windows `PATH`, and command handoff validates
    those dynamic-library path values through the shared allowed-root
    validator. Pid files, Pumas package paths, artifact paths, and
    worker-visible paths remain pending.
  - 2026-05-11 partial: managed-runtime command handoff now validates
    extracted `--pid-file` paths against `app_data_dir`, resolving relative
    pid-file arguments under that root and rejecting absolute paths outside it
    with typed path diagnostics. Pumas package paths, artifact paths, and
    worker-visible paths remain pending.
- [ ] Use checked arithmetic and typed diagnostics for image dimensions, context
  lengths, token limits, batch sizes, memory estimates, output-size
  calculations, and byte ranges that cross IPC, persisted, worker, or runtime
  boundaries.
  - 2026-05-11 partial: artifact-store memory-cache accounting and streaming
    chunk byte-length updates now use checked arithmetic. Cache capacity
    overflow skips insertion without mutating counters, and stream byte-length
    overflow returns typed `ArtifactStoreError::ArtifactAccountingOverflow`
    projected through the workflow artifact API.
  - 2026-05-11 partial: artifact-store disk-budget projection now uses checked
    summation for retained bodies, pending streams, and replacement body bytes.
    Overflow returns typed `ArtifactStoreError::ArtifactAccountingOverflow`
    instead of preserving the previous saturating total. Remaining numeric
    boundaries include image dimensions, context/token/batch limits, memory
    estimates, artifact stats summation, byte-range projections, and
    worker/runtime request fields.
  - 2026-05-11 partial: artifact-store stats projection now returns `Result`
    and uses checked arithmetic for retained body bytes, streaming body bytes,
    and per-state counters. Overflow returns typed
    `ArtifactStoreError::ArtifactAccountingOverflow` through the workflow
    service API. Remaining numeric boundaries include image dimensions,
    context/token/batch limits, memory estimates, byte-range projections, and
    worker/runtime request fields.
  - 2026-05-11 partial: llama.cpp runtime startup now rejects
    `context_size: Some(0)` with the existing typed backend config diagnostic
    before projecting an effective runtime setting. Batch and micro-batch zero
    validation already used the same fail-closed path. Remaining numeric
    boundaries include image dimensions, broader context/token/batch limits,
    memory estimates, byte-range projections, and worker/runtime request
    fields.
  - 2026-05-11 partial: diagnostics projection rebuild now rejects explicit
    `batch_size: Some(0)` with `WorkflowServiceError::InvalidRequest` instead
    of normalizing it to one. `None` remains the intentional defaulted request
    shape. Remaining numeric boundaries include image dimensions, broader
    context/token/batch limits, memory estimates, byte-range projections, and
    worker/runtime request fields.
  - 2026-05-11 partial: inference gateway image generation now rejects
    explicit zero width or height with `BackendError::Config` before backend
    dispatch. Absent dimensions remain backend-owned/defaulted options.
    Remaining numeric boundaries include broader image request limits,
    context/token/batch limits, memory estimates, byte-range projections, and
    worker/runtime request fields.
  - 2026-05-11 partial: inference gateway image generation now also rejects
    explicit zero `num_inference_steps` and `num_images_per_prompt` with
    `BackendError::Config` before backend dispatch. Remaining numeric
    boundaries include broader image request limits, context/token/batch
    limits, memory estimates, byte-range projections, and worker/runtime
    request fields.
  - 2026-05-11 partial: workflow retention cleanup now rejects explicit
    `limit: Some(0)` with `WorkflowServiceError::InvalidRequest` instead of
    normalizing it to one. `None` remains the canonical defaulted request
    shape. Remaining numeric boundaries include broader image request limits,
    context/token/batch limits, memory estimates, byte-range projections, and
    worker/runtime request fields.
  - 2026-05-11 partial: diagnostics query DTO conversion now rejects explicit
    zero `page_size`/`limit` values with `WorkflowServiceError::InvalidRequest`
    instead of normalizing them to one across usage, scheduler timeline, run
    list, IO artifact, node status, and library usage queries. `None` remains
    the canonical defaulted request shape. Remaining numeric boundaries include
    broader image request limits, context/token/batch limits, memory estimates,
    byte-range projections, and worker/runtime request fields.
  - 2026-05-11 partial: loaded-runtime capacity limit updates now reject
    explicit zero or above-session-limit values with
    `WorkflowServiceError::InvalidRequest` instead of clamping them to one or
    `max_sessions`. `None` remains the canonical reset to the service session
    limit. Remaining numeric boundaries include broader image request limits,
    context/token/batch limits, memory estimates, byte-range projections, and
    worker/runtime request fields.
  - 2026-05-11 partial: runtime-registry admission now uses checked summation
    for reserved RAM/VRAM accounting and returns typed
    `RuntimeRegistryError::ResourceAccountingOverflow` if existing reservation
    claims exceed `u64` capacity instead of relying on raw `sum()` overflow
    behavior. Remaining numeric boundaries include broader image request
    limits, context/token/batch limits, memory estimates, byte-range
    projections, and worker/runtime request fields.
  - 2026-05-11 partial: workflow capability memory estimation now uses checked
    model-size rounding and checked total summation, returning
    `WorkflowServiceError::InvalidRequest` for overflow instead of saturating
    byte sizes or relying on raw `sum()` behavior. Remaining numeric
    boundaries include broader image request limits, context/token/batch
    limits, byte-range projections, and worker/runtime request fields.
  - 2026-05-11 partial: inference embedding usage aggregation now uses checked
    token summation and rejects totals that exceed the public `InferenceUsage`
    `u32` contract with `BackendError::Config` instead of clamping to
    `u32::MAX`. Remaining numeric boundaries include broader image request
    limits, context/batch limits, byte-range projections, and worker/runtime
    request fields.
  - 2026-05-11 partial: runtime-registry admission budget projection now uses
    checked subtraction for total budget, safety margin, and existing
    reservations, returning typed `RuntimeRegistryError::ResourceBudgetUnderflow`
    instead of saturating impossible budgets to zero available resource.
    Remaining numeric boundaries include broader image request limits,
    context/batch limits, byte-range projections, and worker/runtime request
    fields.
  - 2026-05-11 partial: workflow capability memory estimation now rejects
    explicit zero `size_bytes` model metadata with
    `WorkflowServiceError::InvalidRequest` instead of manufacturing a 1 MB
    estimate. Remaining numeric boundaries include broader image request
    limits, context/batch limits, byte-range projections, and worker/runtime
    request fields.
  - 2026-05-11 partial: artifact retention cleanup now uses checked TTL
    second-to-millisecond projection and checked cutoff subtraction, returning
    `ArtifactStoreError::ArtifactAccountingOverflow` instead of saturating an
    overflowing cleanup horizon. Remaining numeric boundaries include broader
    image request limits, context/batch limits, byte-range projections, and
    worker/runtime request fields.
  - 2026-05-12 decision: stale-graph diagnostic summary counts are diagnostic
    contract data, not cosmetic fallback text. If the formatter ever attempts
    to show more diagnostic reasons than exist, it must return a typed internal
    error instead of saturating the remaining count to zero.
  - 2026-05-12 partial: stale-graph diagnostic summary formatting now uses
    checked arithmetic for the displayed remaining diagnostic count and returns
    `WorkflowServiceError::Internal` on impossible formatter state. Remaining
    numeric boundaries include duration/timing diagnostics, scheduler timestamp
    addition, runtime technical-fit rank overflow, cache counter drift, broader
    image request limits, context/batch limits, byte-range projections, and
    worker/runtime request fields.
  - 2026-05-12 partial: workflow session runtime-admission retry timestamps now
    use checked scheduler timestamp addition and return
    `WorkflowServiceError::Internal` if `now_ms + poll_ms` overflows instead
    of scheduling a saturated retry timestamp. Remaining numeric boundaries
    include duration/timing diagnostics, runtime technical-fit rank overflow,
    cache counter drift, broader image request limits, context/batch limits,
    byte-range projections, and worker/runtime request fields.
  - 2026-05-12 partial: artifact memory-cache removal now checks byte-counter
    subtraction and returns `ArtifactStoreError::ArtifactAccountingOverflow`
    if a cached body is larger than the recorded cache byte total instead of
    saturating the counter to zero. Remaining numeric boundaries include
    duration/timing diagnostics, runtime technical-fit rank overflow, broader
    image request limits, context/batch limits, byte-range projections, and
    worker/runtime request fields.
  - 2026-05-12 decision: runtime technical-fit pressure ranking must not cap
    active reservation counts. If reservation headroom cannot be ranked within
    the selector contract, candidate selection fails and emits an error
    diagnostic for upstream diagnostics-ledger projection.
  - 2026-05-12 partial: runtime-registry technical-fit auto selection now
    rejects queue/budget-pressure headroom ranking when an eligible
    candidate's active reservation count exceeds the rankable range. The
    selector returns an unselected `no_valid_candidate` error diagnostic
    instead of capping the count before comparison. Remaining numeric
    boundaries include duration/timing diagnostics, broader image request
    limits, context/batch limits, byte-range projections, and worker/runtime
    request fields.
  - 2026-05-12 partial: workflow startup-repair diagnostics now use checked
    duration subtraction and checked repaired-run counting. Future
    `started_at_ms` values, timestamp overflow, or repair-count overflow return
    `WorkflowServiceError::Internal` instead of saturating duration to zero or
    the repaired count to `usize::MAX`. Remaining numeric boundaries include
    model/runtime load and unload duration diagnostics, broader image request
    limits, context/batch limits, byte-range projections, and worker/runtime
    request fields.
  - 2026-05-12 partial: inference gateway image generation now checks the
    conservative RGBA output byte estimate from width, height, and image count
    before backend dispatch. Overflow returns `BackendError::Config` instead
    of reaching a backend or worker with an impossible request size. Remaining
    numeric boundaries include model/runtime load and unload duration
    diagnostics, broader semantic image request limits, context/batch limits,
    byte-range projections, and worker/runtime request fields.
- [ ] If a touched backend starts or modifies a local service, require loopback
  binding, connection/request limits, readiness/startup/shutdown timeouts, and
  lifecycle-owned shutdown.
- [x] Remove hidden llama.cpp CPU fallback when CUDA is requested but CUDA
  runtime files are missing. Return a typed device/runtime-variant diagnostic
  instead.
  - 2026-05-11: Linux llama.cpp command resolution now rejects explicit
    `CUDA*` device requests when `cuda/llama-server` is missing instead of
    using the CPU executable. The resolver returns
    `ManagedRuntimeCommandResolutionError::MissingRuntimeVariant` carrying the
    canonical `missing_runtime_variant` diagnostic for `llama_cpp.cuda`.
- [x] Add backend device inventory facts for llama.cpp `--list-devices` and
  preserve existing parsing while moving it behind the canonical device
  contracts.
- [x] Add PyTorch device probe contract for `cpu` and `cuda` on Linux/Windows,
  plus `mps` on macOS.
- [x] Add vLLM device capability placeholder facts for CPU and CUDA only. Do
  not implement vLLM execution in this slice.
- [x] Add Candle capability placeholder facts for CPU, CUDA, and macOS Metal.
  Do not expose Candle image generation until executable Candle support exists.
- [x] Add future MLX capability facts as macOS-only roadmap facts. MLX must be
  rejected on Linux/Windows if explicitly requested.
- [x] Add future-support notes for ROCm/HIP, Vulkan, XPU/iGPU, OpenVINO,
  hybrid/offload, remote hardware plugins, and MLX without implementing them.
- [x] Update runtime registry technical-fit input/output so candidates can
  carry backend id, task/model support, runtime variant, device class,
  selected device id, resource estimates where known, optional observed
  throughput hints, and device diagnostics.
- [x] Replace `ConservativeFallback`, override-fallback candidate synthesis,
  and any fallback-named executable technical-fit selection with typed
  rejection diagnostics. Missing candidate/runtime state may be reported as
  advisory diagnostics, but it must not select an executable backend.
- [x] Add device policy intent to workflow/runtime technical-fit requests so
  scheduler admission can reject unavailable explicit devices before backend
  load.
- [x] Add optional workflow backend/runtime preference intent to technical-fit
  requests. The scheduler may honor it only when the selected backend can
  execute the model/task on the requested platform/device.
- [x] Reject impossible explicit backend/runtime preferences with bounded
  diagnostics. Examples: llama.cpp for diffusion image generation, MLX on
  Linux/Windows, Candle image generation before executable Candle support, or
  vLLM for unsupported model/task artifacts.
  - 2026-05-11 partial: explicit Candle backend overrides for Diffusers
    image-generation package facts now remain unselected and project the
    candidate compatibility issue as a typed `backend_incompatible`
    diagnostic. No Candle executable candidate, CPU/auto fallback, or alternate
    backend selection is synthesized.
  - 2026-05-11: explicit vLLM backend overrides for Diffusers image-generation
    package facts now reject through the same backend compatibility projection
    with a typed `backend_incompatible` diagnostic. This closes the listed
    impossible-preference examples together with the existing llama.cpp
    diffusion, MLX roadmap/platform, and Candle image-generation coverage.
- [x] Update runtime-load contracts so load readiness consumes a resolved
  device decision rather than inferring from command-line arguments or raw
  backend config strings.
  - 2026-05-11 reconciliation: existing runtime-load contract slices already
    require `RuntimeLoadPhaseRecord::dependency_resolved` to consume
    `DeviceResolutionDecision`, and the runtime-load JSON fixture proves the
    selected runtime variant, device class, and device id are serialized as
    resolved decision facts rather than inferred from command arguments.
- [x] Update inference lifecycle events, diagnostics ledger projection, and run
  inspection facts to include selected backend id, selected runtime variant,
  device class, and selected device id.
  - 2026-05-11: inference lifecycle events and diagnostics-ledger inference
    diagnostic payloads now carry optional `selected_runtime_variant_id`
    alongside existing backend/device facts. Run-list and run-detail projection
    consume the explicit payload field and expose/filter/facet it without
    deriving a runtime variant from runtime id, backend key, command arguments,
    or raw device strings.
- [x] Add scheduler-learning fact fields without implementing learned
  scheduling policy: model id, task kind, selected backend, selected runtime
  variant, selected device class/id, resource estimate when known, execution
  duration, terminal status, and artifact descriptor output-size measures.
- [x] Keep scheduler-learning facts descriptor-level. Do not require scheduler
  learning to inspect retained artifact bodies.
  - 2026-05-11: diagnostics-ledger run-list and run-detail projections now
    expose the selected model/task/backend/runtime/device facts, existing
    estimated duration resource facts, terminal status/duration, and
    descriptor-level output artifact measures. Output facts are limited to
    output artifact descriptor count and total reported byte size from
    `io.artifact_observed`; no retained artifact body inspection or learned
    scheduler policy was added.
- [ ] Add lifecycle ownership for device probes, install jobs, progress streams,
  and refresh events. Each background task must have a tracked owner,
  cancellation path, shutdown behavior, and panic/error reporting.
- [ ] Ensure explicit device requests fail when unavailable. Auto mode may
  select a device, but must record the selected runtime variant and selected
  device.
- [ ] Ensure auto mode is a first-class policy, not a fallback. If auto cannot
  resolve exactly one valid backend/runtime/device decision, fail with typed
  diagnostics instead of reusing raw-device defaults or old backend behavior.
- [x] Keep existing llama.cpp `gpu_layers` as a llama.cpp runtime setting, but
  do not expose a cross-backend hybrid/offload policy in this milestone.
- [ ] Treat hybrid placement, CPU/GPU split, and offload as backend-specific
  adapter capabilities. The scheduler can choose only from typed candidate
  facts and must not synthesize backend-specific flags from generic device
  policy.
- [ ] Update frontend model/runtime/device selectors to render backend-owned
  capability facts and submit only validated device policy intent.
- [ ] Verify new or changed frontend runtime/device controls with accessible
  selectors, accessible names, focus-visible behavior, keyboard interaction
  where interactive, and deterministic subscription or scoped-poll cleanup.
- [x] Replace frontend copy and submit paths that imply `llama-server` owns
  final auto device choice. Frontend may render backend facts and submit user
  intent only.
- [x] Remove optimistic frontend executable-device state. Frontend may keep
  transient form intent, but displayed runtime/device readiness must come from
  backend-confirmed snapshots.
- [x] Remove frontend fallback device options such as synthetic CPU-only lists
  after backend device discovery failure. Discovery failures render backend
  diagnostics/unavailable state and cannot create executable choices.
- [x] Replace or scope polling-heavy frontend refresh paths. Any remaining poll
  must have deterministic teardown and tests.
- [ ] Update canonical workflows and fixtures to the new device policy/runtime
  variant shape. Do not add legacy compatibility shims for old raw-device
  workflow shapes.
  - 2026-05-11 partial: updated technical-fit Rust/TypeScript contract
    fixtures and presenter tests from slash-shaped runtime variant ids to the
    canonical dot-shaped `RuntimeVariantId` examples such as
    `llama_cpp.cuda` and `pytorch.cuda`. Saved workflow files remain pending.
  - 2026-05-20 partial: bundled image-generation/reranker workflow templates
    and tracked image-generation saved workflow examples no longer persist
    graph-visible `backend_key` runtime-selection fields on canonical
    `llm-inference` nodes. Template and saved-workflow tests now require
    scheduler-owned runtime selection for these examples while preserving
    `pumas_model_ref`, `task_kind`, and image output wiring. Remaining
    follow-up: non-image tracked workflow examples and any future canonical
    workflow schema migration still need an explicit ownership decision before
    this broad checklist item can close.
- [ ] If runtime feature flags or optional dependencies change, document the
  feature contract and run affected public crates through default,
  no-default-features, and all-features checks.
- [x] Update relevant module READMEs for runtime variant ownership and device
  policy boundaries.

## Remaining Implementation Detail Plan

This section is the closeout map for the remaining unchecked Milestone 5
items. It does not create fallback or compatibility work. Each slice either
removes a retired path, replaces it with the canonical scheduler-owned
contract, or records a typed diagnostic when the canonical contract cannot make
a valid decision.

### Legacy Removal Contract

Milestone 5 closeout treats legacy behavior as removal work, not compatibility
work. A replaced graph, backend, runtime, device, technical-fit, worker, or
fixture contract must not remain reachable as an alternate execution path.

- Old graph shapes are not migrated into executable requests during runtime
  planning. Tracked examples must be rewritten to canonical fields. User-local
  or persisted stale shapes may be inspected only to produce stale/invalid graph
  diagnostics; they must not be silently accepted, repaired, or routed.
- Raw local model paths are not model identity for canonical inference. Pumas
  owns model identity, artifact selection, artifact load targets, storage kind,
  and validation state. Pantograph may carry `pumas_model_ref` and
  scheduler-facing intent, then consume Pumas-approved load targets at the
  host/planning boundary.
- Runtime and device values from workflow graphs are scheduler requirements or
  preferences only when expressed through typed canonical inputs. The inference
  crate receives the runtime/device decision exclusively from the scheduler
  path; it must not separately resolve graph hints, fallback runtime strings, or
  backend-local defaults.
- Remaining tests that mention retired fields must be negative coverage proving
  ignore/reject/stale-diagnostic behavior, or must be removed when the canonical
  replacement lands. Tests must not preserve old behavior by expecting successful
  execution through legacy fields.
- If a slice discovers that removing a retired path requires a broader schema or
  ownership decision, stop at a re-plan boundary. Do not add a transitional shim.

### Closeout Order

1. **Checklist Reconciliation And Contract Inventory**
   - Purpose: separate already-implemented checklist rows from true remaining
     code work before touching more execution paths.
   - Allowed write set: this milestone file and narrowly scoped test-only
     fixture index files if a missing fixture is discovered and can be added
     without changing runtime behavior.
   - Required inventory:
     `InferenceDevicePolicy`, `InferenceDeviceId`, `RuntimeVariantId`,
     `RuntimeVariantCapability`, `DeviceResolutionRequest`,
     `DeviceResolutionDecision`, `BackendExecutionCandidate`,
     `BackendExecutionDecision`, runtime-load phase records,
     diagnostics-ledger scheduler/run payloads, TypeScript mirrors, Python
     worker load envelopes, managed-runtime persisted state, Tauri command
     DTOs, and saved workflow/template JSON.
   - Required legacy-removal inventory: every remaining use of raw
     `model_path`, `backend_key`, `runtime_hint`, raw device strings,
     backend-local device ids, resolved Pumas package facts, capability-only
     technical-fit selection, and sidecar `DeviceConfig` execution must be
     classified as `removed`, `backend-adapter-local`, `negative diagnostic
     coverage`, or `re-plan required`. No item may be classified as
     `compatibility`.
   - Acceptance: every boundary is listed as `covered`, `missing fixture`,
     `not a boundary`, or `deferred with owner`. Missing fixtures become the
     next smallest slice; broad rows are not marked complete from prose alone.
     Retired fields have an explicit removal or rejection owner before any
     execution-admission slice begins.

2. **Graph Model Selection Contract Replacement**
   - Purpose: replace the remaining `puma-lib` model-file-path producer
     contract with a graph-facing Pumas model-reference selector before
     node-engine, workflow fixture, or scheduler-admission slices consume more
     graph model data.
   - Allowed primary write areas by slice:
     `crates/workflow-nodes/src/input/puma_lib.rs`,
     `crates/workflow-nodes/src/input/README.md`,
     `crates/workflow-nodes/src/contracts.rs` only if the descriptor contract
     must change, frontend selection-input/provider option helpers and Node
     tests only if the option value shape changes, and focused workflow-node
     tests. Do not touch node-engine execution, scheduler policy, saved
     workflows, generated DTOs, lockfiles, or Pumas proposal files in this
     slice.
   - Required direction: `pumas_model_ref` is the executable graph-facing
     model selection value. Pumas selector metadata may include display labels,
     readiness, package summary, storage kind, validation state, and stale
     diagnostics, but raw executable paths and `backend_key` must not be
     emitted as current execution outputs or option values. Any path-shaped
     data that remains must be explicitly named display/debug/stale diagnostic
     evidence and must not feed execution, memory identity, package-fact
     lookup, or scheduler candidate synthesis.
   - Required option-provider behavior: `PortOptionsProvider` results for
     `puma-lib` must advertise typed Pumas reference values and typed
     availability/readiness facts. They must not use executable entry paths as
     option values, infer runtime/backend choices, or hide missing/stale Pumas
     state behind an empty successful option list.
   - Acceptance: workflow-node tests prove `puma-lib` descriptors/options no
     longer expose `model_path` or `backend_key` as executable outputs, prove
     ready selector rows produce `pumas_model_ref` values, and prove missing,
     stale, invalid, unavailable, or not-implemented Pumas states become typed
     unavailable options or diagnostics. Documentation must describe the
     replacement contract and remove the old `model_path` facade language.
   - 2026-05-20 implementation: completed the workflow-node contract slice.
     `puma-lib` descriptors now expose `pumas_model_ref` as the selectable
     graph value and no longer expose `model_path` or graph-visible
     `backend_key` execution outputs. The registered `PortOptionsProvider`
     moved from `model_path` to `pumas_model_ref`, option values now carry the
     typed Pumas model-reference payload instead of executable entry paths, and
     selector paths are projected only as `display_*` metadata. Non-ready
     selector rows are disabled with typed port-option unavailable states and
     stable Pumas selector reason codes.
   - No-fallback/no-legacy confirmation: this slice did not add a compatibility
     alias for `model_path` or `backend_key`, did not synthesize missing
     selected artifact fields from parallel selector metadata, and did not
     preserve executable path option values. Pumas remains authoritative for
     model references and later artifact load-target resolution.
   - Verification: `cargo fmt --all -- --check`, `cargo test -p
     workflow-nodes puma_lib --features model-library`, and `cargo test -p
     workflow-nodes builtin_contracts_preserve_registered_port_options_provider_refs
     --features model-library`.
   - Remaining follow-up: node-engine still has retired model/fact intake and
     non-image workflow examples still carry retired direct inference shapes;
     those remain separate closeout slices and were intentionally not touched
     by this workflow-node contract slice.

3. **Node-Engine Dependency Planning Contract Replacement**
   - Purpose: remove path-shaped dependency resolution from node-engine. The
     graph and node-engine boundary carries `pumas_model_ref`, task kind, task
     options, and optional typed scheduler intent only. Node-engine must not
     know, infer, repair, or forward the exact Pumas artifact filesystem path
     as model identity or dependency identity.
   - Allowed primary write areas by slice:
     `crates/node-engine/src/core_executor/dependency_preflight.rs`,
     `crates/node-engine/src/core_executor.rs`,
     `crates/node-engine/src/engine/dependency_inputs.rs`,
     `crates/node-engine/src/model_dependencies.rs` or equivalent dependency
     contract owner, host/planner adapter code that currently implements
     `ModelDependencyResolver`,
     `crates/pantograph-embedded-runtime/src/model_dependency_descriptors.rs`,
     `crates/pantograph-embedded-runtime/src/model_dependency_activity.rs`,
     `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment.rs`,
     `crates/pantograph-embedded-runtime/src/task_executor/puma_lib.rs`, and
     focused tests/READMEs. Do not touch scheduler ranking policy, generated
     DTOs, lockfiles, saved workflows, frontend controls, or Pumas proposal
     files in the same slice unless the compiler proves a contract boundary
     must move together.
   - Required direction: replace `ModelDependencyRequest.model_path` as the
     dependency-preflight source of truth with a typed request keyed by
     `PumasModelRef`, canonical task id/task type, expected artifact kind when
     known, optional backend/runtime/device intent, and bounded caller context.
     The host/planner-side resolver asks Pumas for the approved artifact load
     target and derives runtime-specific dependency facts there. Node-engine
     receives typed dependency/preflight results and diagnostics, not Pumas
     paths. Replace `ModelRefV2` or split it so graph/node-engine model
     identity no longer requires `model_path`; executable paths may exist only
     in selected backend/worker handoff contracts after planning. Before
     implementation, pick the contract owner deliberately: keep the contract in
     `node-engine` only if it remains the curated public facade for both
     producer and consumer, or move it to a narrower shared contract module if
     the same DTO is trusted across multiple crates/processes. Do not hide a
     cross-boundary schema inside an adapter implementation module.
   - Required ownership split: Pumas owns model-library lookup, artifact
     selection, storage kind, validation state, and local load targets.
     Scheduler/planner owns runtime/device candidate selection. Node-engine owns
     graph task execution orchestration and typed intent forwarding only.
     Backend adapters/workers may receive a local load path only through an
     already-selected execution/preflight plan at the runtime handoff.
   - Required removal/replacement: no `model_path` repair in node-engine, no
     `resolved_model_source` revival, no path-shaped `pumas_model_ref` aliases,
     no directory scanning, no Pumas path joining, no model id derivation from
     paths, and no fallback to raw path preflight when Pumas cannot resolve a
     valid target. Missing/stale/invalid/unavailable Pumas state returns typed
     dependency planning diagnostics. Remove Puma-Lib execution-time stale-path
     rebinding and path-shaped outputs (`model_path`, `selected_artifact_path`,
     `entry_path`) from canonical inference identity. Remove dependency-input
     context propagation that can reintroduce executable paths or load targets
     as graph-level dependency identity. Replace dependency cache keys and
     activity correlation fields with stable Pumas identity plus selected
     artifact/runtime/task facts; activity may include redacted runtime paths
     only as backend/worker diagnostics after handoff. Replace public
     `Result<_, String>` resolver errors with typed error/diagnostic enums and
     preserve lower-level causes when adapting Pumas, dependency, or worker
     failures.
   - Staging:
     1. Introduce the typed dependency-planning request/diagnostic/result
        contract around `PumasModelRef`, task facts, optional expected artifact
        kind, optional scheduler/runtime/device intent, selected binding ids,
        and bounded caller context. Keep node-engine request construction
        synchronous; the resolver trait may remain async because host/planner
        implementations perform real Pumas/dependency I/O. Decode and validate
        raw JSON graph/input payloads once at the node-engine or host boundary,
        then pass validated domain types inward; do not pass unvalidated
        `serde_json::Value`, raw `String` mode/kind values, or filesystem
        paths through internal planning APIs when a domain type can encode the
        invariant.
     2. Replace `ModelRefV2` or introduce a successor graph model-reference
        contract that does not contain `model_path`. Update validation and
        output tests so graph identity is explicit Pumas identity, not a
        filesystem path.
     3. Move Pumas artifact-load-target resolution into the host/planner
        resolver implementation and return typed unavailable diagnostics for
        missing, stale, invalid, not-installed, or unsupported artifacts. Reuse
        the existing `PumasSelectorAccess::resolve_model_artifact_load_target`
        boundary rather than adding another Pumas lookup path.
     4. Replace dependency descriptor cache keys, install locks, environment
        manifest identity, and dependency activity events so they key/correlate
        by Pumas model ref, selected artifact identity/kind, backend/runtime
        intent, platform context, task kind, and selected dependency bindings,
        never by local load path.
     5. Update Puma-Lib execution and dependency-input assembly so canonical
        inference graphs propagate `pumas_model_ref`, task facts, selected
        binding ids, and scheduler intent only. Remove stale path rebinding and
        path-shaped `pumas_model_ref` aliases instead of keeping compatibility
        shims.
     6. Update node-engine preflight callers/tests to use the typed
        model-reference request and remove all successful `model_path` preflight
        cases from canonical `llm-inference`.
     7. Remove the old path-shaped `ModelDependencyRequest` fields or confine
        any remaining executable paths to backend/worker-local plan handoff
        types that are not graph/node-engine dependency identity.
   - Acceptance: node-engine dependency-preflight tests prove a graph with only
     `pumas_model_ref` and task intent can request dependency planning without
     local path knowledge; stale or unresolved Pumas state fails with typed
     diagnostics; legacy `model_path`, `resolved_model_source`,
     `selected_artifact_path`, and `entry_path` cannot produce successful
     dependency-preflight execution; `ModelRefV2` or its successor does not
     require or emit `model_path` as graph identity. Host/planner tests prove
     Pumas-approved load targets are resolved outside node-engine and are passed
     to runtime or worker code only after scheduler/planner selection.
     Embedded-runtime tests prove dependency cache/activity/environment identity
     remains stable across path changes for the same Pumas-selected artifact and
     changes when model ref, selected artifact, backend/runtime intent, platform,
     task kind, or selected bindings change. At least one cross-layer acceptance
     test must exercise the real producer-to-consumer path from graph
     `pumas_model_ref` input through node-engine dependency planning,
     host/planner Pumas load-target resolution, and selected runtime/worker
     handoff, asserting the worker receives a Pumas-approved local load path
     while node-engine graph identity never contains or derives from that path.
     Negative acceptance must prove missing/stale/invalid/unavailable Pumas
     state returns the typed dependency-planning diagnostic through the existing
     owned diagnostic channel without adding a parallel diagnostics system.

4. **Raw Device Boundary Removal**
   - Purpose: eliminate remaining cross-crate or cross-process raw device
     strings as trusted scheduler/runtime state.
   - Allowed primary write areas by slice:
     `crates/inference/src/config.rs`, `crates/inference/src/backend/`,
     `crates/inference/src/server.rs`,
     `crates/node-engine/src/core_executor/`,
     `crates/pantograph-embedded-runtime/src/`, and the matching tests and
     READMEs. Do not mix these areas in one slice unless the compiler requires
     a single contract change.
   - Required direction: public planning/admission contracts use
     `InferenceDevicePolicy`, `InferenceDeviceClass`, `InferenceDeviceId`, and
     `RuntimeVariantId`. Backend-local strings such as llama.cpp selectors,
     PyTorch `"cuda:0"`, Python worker `"auto"`, or sidecar `DeviceConfig`
     values may exist only inside the adapter/worker translation boundary.
   - Required removals/replacements: no `unknown -> auto`, no malformed ordinal
     `-> 0`, no frontend-generated executable device choices, no
     gateway-active-backend inference, and no node-engine backend routing that
     independently chooses a runtime after the scheduler decision exists.
   - Acceptance: tests prove explicit device intent is either admitted with the
     selected runtime variant/device facts or rejected with a typed diagnostic;
     `auto` is policy intent only and never a concrete selected device id.

5. **Candidate Synthesis And Ledger Projection Closure**
   - Purpose: finish the policy input path so automatic selection sees complete
     candidate facts and diagnostics-ledger history can explain the decision.
   - Allowed primary write areas:
     `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
     `crates/pantograph-runtime-registry/src/technical_fit.rs`,
     `crates/pantograph-diagnostics-ledger/src/`, workflow-service projection
     DTOs, TypeScript mirror types, and focused fixtures.
   - Required candidate facts: selected/candidate backend id, task/model
     compatibility, Pumas model/package fact status, runtime variant id,
     runtime availability/readiness, device class/id facts, resource estimate,
     throughput/history hints when threshold-eligible, bounded candidate
     diagnostics, and policy trace ids.
   - Required failure behavior: missing Pumas facts, stale Pumas facts,
     unavailable runtime variants, oversized candidate sets, impossible
     explicit runtime/device requests, and unrankable candidate state produce
     non-selectable candidates or no-decision diagnostics. They must not
     select a capability-only or fallback backend.
   - Acceptance: scheduler trace and ledger summaries can reconstruct why a
     candidate was selected or rejected without inspecting graph internals,
     frontend state, Pumas filesystem paths, or display strings.

6. **Lifecycle Ownership Hardening**
   - Purpose: make every touched long-running operation explicitly owned,
     cancellable, bounded, and observable.
   - Allowed primary write areas:
     managed-runtime install/progress modules, runtime-registry lifecycle
     helpers, backend startup/load helpers, device probes, Python worker
     process ownership, Tauri command adapters, and matching tests.
   - Required ownership fields for each background operation: owner/composition
     root, start API, stop/cancel API, shutdown behavior, timeout policy,
     overlap/restart policy, error/panic reporting, and projection/refresh
     notification path.
   - Required local-service constraints: loopback-only binding, explicit
     readiness timeout, startup timeout, shutdown timeout, bounded
     connection/request policy, and lifecycle-owned process termination.
   - Acceptance: tests cover cancellation or teardown for each new/changed
     task, and no adapter creates global Tokio runtimes, untracked tasks,
     unbounded queues, or self-owned long-lived subprocesses.

7. **Allowed-Root And Worker-Visible Path Closure**
   - Purpose: ensure every executable, dynamic library, Pumas artifact, and
     worker-visible path is approved by the owning boundary before filesystem
     or subprocess use.
   - Allowed primary write areas:
     `crates/pantograph-path-security`, managed-runtime command handoff,
     Pumas artifact load-target consumption, node-engine file IO,
     workflow-service artifact access, Python worker request construction, and
     focused tests.
   - Required source of truth: Pumas-owned artifact load targets are trusted as
     Pumas-approved local paths with typed storage/validation state, but
     Pantograph still validates whether a path is allowed to be handed to the
     selected runtime/worker. Pantograph must not join Pumas paths or infer
     artifact files from model directories.
   - Acceptance: tests prove runtime roots, executables, dynamic-library paths,
     pid files, Pumas artifact paths, artifact-store paths, and worker-visible
     paths cannot escape their approved roots and fail with typed diagnostics.

8. **Checked Numeric Boundary Closure**
   - Purpose: remove remaining saturation, clamping, defaulting, or raw
     arithmetic from public/runtime/worker boundaries where invalid values
     should fail.
   - Allowed primary write areas by slice: image request normalization,
     context/token/batch limit handling, byte-range projections, runtime/worker
     request DTO construction, memory/resource estimates, and diagnostics
     projection code.
   - Required policy: explicit zero, overflow, underflow, or unrepresentable
     totals fail with typed diagnostics. Absence may still mean a documented
     default only when the receiving canonical contract owns that default.
   - Acceptance: focused tests cover each changed arithmetic boundary and
     demonstrate no impossible value reaches a backend, worker, persisted
     ledger row, or frontend contract.

9. **Frontend Runtime/Device Contract Closure**
   - Purpose: make the UI a renderer of backend-owned capability facts and a
     submitter of typed intent only.
   - Allowed primary write areas: `src/components/DeviceConfig.svelte`,
     device/runtime presenter helpers and tests, workflow runtime/device
     command bindings, TypeScript workflow DTO mirrors, and accessibility
     tests that can run under the repository's current Node test approach.
   - Required UI behavior: no synthetic fallback choices, no optimistic
     executable device state, no frontend runtime ranking, no hidden
     `llama-server` final-choice language, accessible selectors/names,
     keyboard-safe controls, focus-visible behavior where controls are
     interactive, and deterministic cleanup for subscriptions or scoped polls.
   - Acceptance: frontend tests prove selected values come from backend facts
     or transient form intent, submit paths reject stale/unconfirmed choices,
     and labels explain scheduler/backend ownership without implying frontend
     execution authority.

10. **Workflow And Fixture Shape Closure**
   - Purpose: remove retired graph-visible execution hints from tracked
     workflows/templates and freeze the canonical examples used by tests.
   - Allowed primary write areas: tracked `.pantograph/workflows/` examples,
     tracked `.pantograph/orchestrations/` examples,
     `src/templates/workflows/`, template/saved-workflow tests, and this plan.
   - Required graph shape: canonical inference examples carry
     `pumas_model_ref`, task kind, task options, generation/options inputs, and
     optional canonical runtime/device intent only. They must not carry
     `backend_key`, `runtime_hint`, resolved Pumas package facts, raw local
     model paths, generated output bodies, or scheduler decisions.
   - Required `model_path` resolution: remove `model_path` from canonical
     inference identity and memory-impact calculations once replacement
     `pumas_model_ref` coverage exists for the affected workflow/test shape. If
     old saved graphs still contain `model_path`, classify that data as stale
     graph input for diagnostics only; do not use it for execution, memory
     identity, package-fact lookup, or scheduler candidate synthesis.
   - Required non-image fixture decision: non-image tracked examples must either
     be updated to the same canonical scheduler-owned runtime/device/model
     contract or explicitly scoped out of Milestone 5 with an owner and a later
     removal/replacement milestone. They must not keep raw runtime/device/model
     fields as working examples of current behavior.
   - Acceptance: tests distinguish intentionally tracked examples from ignored
     user-local files and assert that examples either use canonical inference
     contracts or are explicitly out of scope for Milestone 5. Any retained
     stale-shape fixture is named as stale diagnostic coverage and cannot be
     used by successful execution tests.

### Standards Compliance Gates

The closeout slices above were iterated against the repository standards in
`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`
on 2026-05-20. Implementation must keep these gates true; if a slice cannot
meet them, stop and re-plan instead of adding compatibility or local bypasses.

- **Plan/worktree hygiene:** each slice must start from a clean implementation
  worktree except explicitly approved plan/proposal markdown. Shared contracts,
  generated DTOs, lockfiles, saved workflow files, persisted schemas, workflow
  fixtures, and global config files remain serial integration-owner work and
  must not be split across parallel workers.
- **Layering and crate roles:** contracts and validated DTOs stay in the
  contract-owning crates. `inference` remains runtime/backend-facing and must
  not depend on workflow-service, diagnostics-ledger, frontend, Tauri, graph
  internals, or scheduler policy crates. Frontend/Tauri/node-engine adapters
  may pass typed user intent and consume backend-confirmed facts, but they must
  not choose an executable backend/runtime/device after the scheduler decision.
- **Typed boundaries:** raw strings, paths, booleans, numeric values, and
  display labels entering IPC, persisted, worker, runtime, or cross-crate
  boundaries must parse once into validated types with typed diagnostics.
  Public fallible APIs must use structured error enums rather than
  `Result<T, String>`, and public DTOs should derive useful `Debug`, use
  explicit serde casing, and use `#[non_exhaustive]` where future extension is
  expected.
- **Rust API and module shape:** planning, parsing, normalization, policy, and
  projection helpers should remain synchronous unless they perform real
  concurrent I/O. New modules must pass the decomposition review when they
  approach the repository thresholds: files over 500 lines, UI components over
  250 lines, modules with more than roughly seven public functions, or modules
  with more than three distinct responsibilities.
- **Concurrency and lifecycle:** any task, stream, local service, subprocess,
  install job, probe, watcher, or refresh loop introduced or touched by a slice
  must have one lifecycle owner, tracked handles, cancellation/shutdown
  behavior, timeout policy, bounded queues/requests where applicable, and
  panic/error reporting at the owner. No global Tokio runtimes, discarded
  `tokio::spawn` handles, unbounded queues, or blocking work in async request
  and lifecycle paths are allowed.
- **Path and resource security:** filesystem, subprocess, dynamic-library,
  Pumas artifact, artifact-store, pid-file, and worker-visible paths must go
  through the shared allowed-root validation boundary before use. Pumas owns
  model library state and artifact load targets; Pantograph may validate
  whether an approved target can be handed to a selected runtime/worker, but it
  must not join Pumas paths or infer artifact files from model directories.
  Local services must bind to loopback and define connection/request limits.
- **Checked arithmetic and resource accounting:** dimensions, token/context
  limits, batch sizes, byte ranges, cache counters, timing/duration values,
  memory estimates, resource observations, and scheduler/admission budgets that
  cross public/runtime/worker/persisted boundaries must use checked arithmetic.
  Overflow, underflow, impossible totals, or unrepresentable values fail with
  typed diagnostics; absence may default only at the canonical owner of that
  default.
- **Frontend ownership and accessibility:** UI code renders backend-owned
  capability/readiness facts and keeps only transient form intent locally.
  Runtime/device controls must not synthesize executable choices, optimistically
  display backend-owned state, or rank/select runtimes. Interactive controls
  need semantic elements or complete ARIA/keyboard handling, accessible names,
  focus-visible behavior, and tests using resilient accessible selectors.
  Polling must be avoided when event/subscription updates are feasible; any
  remaining poll must be scoped and have deterministic teardown tests.
- **Interop and fixture coverage:** every DTO crossing Rust crate, Tauri/TS,
  diagnostics-ledger, Python worker, persisted-state, or saved-workflow
  boundaries needs native-side contract tests and the matching host/consumer
  fixture or smoke path for the slice that changes it. Serde tag/casing/field
  shape must be asserted rather than inferred from TypeScript or Rust compile
  checks alone.
- **Documentation and traceability:** when a slice changes a public contract,
  feature flag, module role, structured producer, host-facing API, persisted
  artifact, or ownership boundary, update the relevant README/plan entry in
  the same commit. New or reorganized source directories need meaningful
  README coverage under the documentation standards; placeholder prose is not
  acceptable.

### Per-Slice Standards Evidence

Before any remaining closeout item is marked complete, the implementation note
for that slice must record the standards evidence below. This is not a
compatibility step and must not preserve retired behavior; it is the audit
trail that proves the replacement path stayed inside the agreed blast radius.

- **Write set and ownership:** exact files or directories touched, forbidden
  shared artifacts left untouched, and whether the slice was serial-only
  because it changed contracts, generated DTOs, persisted state, lockfiles,
  workflow fixtures, or plan files.
- **Layer and crate-role check:** why the changed code belongs in its crate or
  frontend module, and confirmation that dependency direction still follows the
  established architecture. In particular, `inference` must remain free of
  workflow-service, diagnostics-ledger, frontend, Tauri, graph-internal, and
  scheduler-policy dependencies.
- **Contract and fixture impact:** every Rust, TypeScript, Tauri, Python
  worker, diagnostics-ledger, persisted-state, and saved-workflow boundary
  affected by the slice, plus the fixture or contract test that proves serde
  casing, tags, validated ids, and diagnostic variants.
- **Lifecycle and concurrency owner:** for every task, subprocess, stream,
  probe, watcher, install job, local service, or refresh loop touched, record
  the owner, tracked handle shape, cancellation path, shutdown timeout, overlap
  policy, bounded queue/request policy, and panic/error reporting path.
- **Path, process, and resource security:** the allowed-root validation point,
  loopback/request-limit policy for local services, and checked arithmetic for
  dimensions, byte ranges, cache counters, timings, resource observations,
  memory estimates, and scheduler/admission budgets.
- **Cross-platform and feature matrix:** when a slice touches platform-specific
  runtime, monitor, subprocess, filesystem, dynamic-library, or feature-flag
  behavior, record the platform abstraction used, any `cfg()` containment, and
  the default/no-default/all-features or target checks that apply. Unsupported
  or unavailable platform/runtime features must surface typed unavailable
  diagnostics, not implicit fallback behavior.
- **Frontend accessibility and state ownership:** when UI is touched, record
  the accessible selectors/names, focus and keyboard behavior, cleanup of
  subscriptions or scoped polls, and confirmation that backend-owned
  runtime/device readiness is rendered from backend facts only.
- **Decomposition review:** if a changed file crosses the standards thresholds
  for size or responsibility, either decompose it in the slice or record the
  concrete follow-up owner before closing the related checklist row.
- **Verification:** exact commands run, including focused tests, formatting,
  diff checks, fixture checks, and any skipped verification with the reason and
  owner.

Acceptance: broad Milestone 5 checklist rows cannot close from prose-only
confidence. The matching implementation note must include the evidence above
and must point to the tests or diagnostics that prove the canonical path
replaced the retired behavior.

### Codebase Investigation Findings

The closeout plan was rechecked against the current codebase on 2026-05-20.
These findings are implementation constraints for the remaining slices. They
must be resolved directly or recorded as a deferred owner-specific follow-up
before the matching checklist row is closed.

- **Embedded-runtime technical-fit decomposition:** before expanding candidate
  synthesis, ledger projection, or scheduler history behavior, decompose
  `crates/pantograph-embedded-runtime/src/technical_fit.rs` into focused
  modules. The current file mixes host async lookup, Pumas fact resolution,
  dependency readiness probing, candidate synthesis, history lookup, DTO
  projection, and tests. The decomposition target is a thin orchestration
  facade with separate modules for Pumas/package-fact resolution, candidate
  synthesis, history summaries, workflow/runtime DTO projection, and tests.
- **No lossy projection catch-alls:** runtime-to-workflow projection mappings
  must not use `_ =>` arms that silently coerce unknown dependency readiness,
  resolver owner, device, runtime, or diagnostic states into a generic known
  state. Add explicit mappings when contracts add variants; otherwise emit a
  typed internal/projection diagnostic so contract drift is visible.
- **Typed Pumas unavailable states:** Pumas package-fact and artifact-load-target
  lookup must not collapse Owner, LocalClient, ReadOnly, missing selector,
  stale facts, decode failure, or resolver failure into an empty package-facts
  vector plus a log line. Each state needs a typed non-selectable candidate or
  scheduler diagnostic with Pumas access mode, model id, and resolver status.
  When artifact load-target resolution is available, technical-fit and image
  planning should consume that Pumas-owned contract rather than model-level
  descriptor APIs.
- **Explicit runtime observation facts:** runtime-registry observation must not
  infer executable runtime identity from `backend_key`, display backend name,
  or `"unknown"`. A ready runtime observation needs an explicit runtime id and
  runtime variant/source facts from the lifecycle snapshot or selected
  scheduler decision. Missing runtime facts should produce typed observation
  diagnostics instead of alias-derived runtime records.
- **Global device config replacement:** the current frontend/Tauri
  `DeviceConfig` path still looks like generic device selection while the Rust
  type is a llama.cpp adapter-local selector plus `gpu_layers`. Remaining UI
  work must replace or strictly scope this path so scheduler-owned
  runtime/device intent is separate from llama.cpp runtime settings. New
  frontend controls should submit typed intent, and llama.cpp-specific controls
  such as `gpu_layers` should stay clearly adapter-local.
- **Owned process-spawner tasks:** `StdProcessSpawner` currently starts stdout,
  stderr, and process-monitor tasks through untracked `tokio::spawn` calls.
  Lifecycle hardening must replace this with an owned sidecar process handle
  that tracks task handles, drains or cancels them on shutdown, reports task
  panics/errors at the owner, and preserves bounded event delivery.
- **Remove path-derived Pumas model inference:** workflow capability extraction
  still derives model ids from path-shaped fields such as `model_path`,
  `entry_path`, and `selected_artifact_path`. This is legacy graph inspection
  and should be removed when the workflow/fixture closure slice runs. The
  canonical source is explicit `pumas_model_ref`; Pumas owns path-to-model
  interpretation.
  - 2026-05-20 update: workflow-service capability extraction now ignores
    path-shaped model fields and uses only explicit model identity fields such
    as `model_id` and `pumas_model_ref.model_id`. Pumas path-to-model
    interpretation remains outside Pantograph capability extraction.
  - 2026-05-20 update: graph edge-insert input priority now treats
    `resolved_model_source` and `resolved_model_package_facts` as ordinary
    optional JSON ports instead of preferred model-reference ports.
  - 2026-05-20 update: KV-cache memory-impact no longer treats
    `resolved_model_source` changes as model identity changes. Canonical model
    identity remains explicit `pumas_model_ref`, `model_id`, or currently
    scoped model fields until the broader workflow schema migration removes
    remaining legacy model-path graph data.
- **Replace `puma-lib` path producer semantics:** `workflow-nodes` still
  describes `puma-lib` as a model-file-path producer, exposes `model_path` and
  `backend_key`, registers model-library options on the `model_path` port, and
  returns executable entry paths as option values when selector rows are ready.
  This contradicts the Milestone 5 legacy-removal contract. The replacement
  direction is to make `pumas_model_ref` the canonical graph-facing selection
  value, keep Pumas selector/readiness metadata display-only, and remove
  `backend_key`/raw path output as current execution signals. Any remaining
  path-shaped selector metadata must be named display/debug or stale-diagnostic
  evidence and must not feed execution, memory identity, package-fact lookup, or
  scheduler candidate synthesis.
- **Update stale `workflow-nodes` documentation with the contract change:** the
  input-node README still says `puma-lib` preserves a graph-facing
  `model_path` facade and emits `backend_key`. The same implementation slice
  that replaces the descriptor/options contract must update that README so
  documentation does not keep a second design alive.
- **Remove node-engine legacy model/fact intake:** node-engine typed inference
  request builders still accept `resolved_model_source`,
  `resolved_model_package_facts`, and `model_package_facts` graph inputs as
  compatibility sources, and image/text/rerank builders still derive request
  model names or GGUF paths from `model_path`, `selected_artifact_path`, or
  `entry_path`. The canonical node-engine boundary should accept
  graph-authored `pumas_model_ref` plus reduced host/planning facts through one
  current internal handoff. It should not parse retired graph fields, scan model
  directories, or use Pumas artifact paths as display model identity after the
  image planner has a Pumas artifact load target.
  - 2026-05-20 update: node-engine typed inference request builders now promote
    only explicit `pumas_model_ref`/`model_ref` graph inputs into request model
    identity. `resolved_model_source` no longer supplies model identity for
    text requests, and resolved package facts remain forwarded only as
    host/planning compatibility facts instead of being promoted into request
    `model_ref`. Image, audio, and rerank request builders no longer derive
    model names from `model_path`, `selected_artifact_path`, or `entry_path`;
    canonical tests now wire `pumas_model_ref` when model identity is required.
  - No-fallback/no-legacy confirmation: this slice did not add compatibility
    aliases for retired Pumas source fields, did not synthesize graph model
    identity from package facts, and did not keep artifact paths as successful
    image/rerank/audio model names. Pumas artifact load targets still enter the
    image planner through the existing internal planned-execution handoff; that
    path remains separate from graph model selection.
  - Standards/blast-radius evidence: write set was limited to
    `crates/node-engine/src/core_executor/inference_nodes.rs`,
    `crates/node-engine/src/core_executor/inference_tests.rs`,
    `crates/node-engine/src/core_executor/README.md`,
    `crates/node-engine/src/README.md`, and this plan. Crate roles remain
    unchanged: node-engine builds typed runtime requests and forwards
    host/planning facts but does not own Pumas path resolution, scheduler
    runtime/device decisions, frontend state, persisted schema, generated DTOs,
    lockfiles, or worker process ownership. No background task, local service,
    filesystem path access, feature flag, or frontend behavior changed.
  - Verification: `cargo fmt --all -- --check`, focused node-engine builder
    tests for text, embedding, rerank, image, and audio transcription,
    `cargo test -p node-engine --features inference-nodes
    core_executor::tests::inference_tests -- --nocapture`, `cargo check -p
    node-engine --features inference-nodes`, and `git diff --check`.
  - 2026-05-20 update: dependency preflight no longer accepts
    `resolved_model_source` as a model-path, companion-artifact, artifact-kind,
    `ModelRefV2`, or model-dependency identity source. Dependency input repair
    now rejects `resolved_model_source` with an explicit retired-input
    diagnostic, and it no longer reads `model_path`, `selected_artifact_path`,
    `entry_path`, `mmproj_path`, or legacy companion paths from
    `pumas_model_ref`.
  - No-fallback/no-legacy confirmation: this follow-up did not translate old
    Pumas source DTOs into dependency requests and did not preserve path-shaped
    `pumas_model_ref` fields as hidden executable path aliases. Existing
    explicit `model_path` dependency-preflight input remains only because the
    current `ModelDependencyRequest`/resolver contract still requires an
    executable path; replacing that with Pumas artifact load targets is a
    broader resolver API slice, not a compatibility shim added here.
  - Standards/blast-radius evidence: write set for the dependency-preflight
    cleanup was limited to
    `crates/node-engine/src/core_executor/dependency_preflight.rs`,
    `crates/node-engine/src/core_executor/inference_tests.rs`, and this plan.
    Node-engine remains the dependency preflight caller; Pumas path ownership,
    scheduler runtime/device decisions, resolver API shape, Python-worker
    process execution, generated DTOs, lockfiles, saved workflows, frontend
    state, background tasks, and persisted schemas were not changed.
  - Verification: `cargo fmt --all -- --check`, `cargo test -p node-engine
    --features inference-nodes core_executor::tests::inference_tests --
    --nocapture`, `cargo check -p node-engine --features inference-nodes`,
    `cargo test -p node-engine --features pytorch-nodes
    test_resolved_artifact_kind_uses_package_facts -- --nocapture`, `cargo
    test -p node-engine --features pytorch-nodes
    test_dependency_preflight_maps_explicit_hf_transformers_request --
    --nocapture`, and `git diff --check`.
  - Remaining follow-up: replace the explicit `model_path` dependency resolver
    contract with the Node-Engine Dependency Planning Contract Replacement
    above. Node-engine should request dependency planning with `pumas_model_ref`
    and task facts only; Pumas artifact load-target resolution belongs in the
    host/planner resolver implementation, and executable paths should appear
    only at the selected runtime/worker handoff.
- **Tracked saved workflow closure is broader than image examples:** bundled
  templates and tracked image examples are already canonical, but tracked
  non-image workflow files such as Whisper STT and KittenTTS still use retired
  direct inference nodes, `model_path`, graph-visible `backend_key`, and
  dependency-environment path wiring. These files must either be converted to
  canonical scheduler-owned model/runtime/device contracts or explicitly moved
  into stale diagnostic fixture coverage. They must not remain tracked as
  successful examples of current execution behavior.
- **Preserve the clean image planner boundary:** the current image-generation
  planner shape is the model to preserve: side-effect-free input, Pumas package
  facts, Pumas artifact load target, and a scheduler-owned
  `BackendExecutionDecision` produce one execution plan or typed diagnostics.
  Future image-family, scheduler, or worker extensions should extend this
  boundary rather than passing full Pumas facts through graph nodes or letting
  the worker/runtime select execution state.
- **Resource monitor staging:** terminal process RSS observation is acceptable
  as the current stage, but it must remain explicitly labeled as terminal
  observation only. Real-time resource observation for parallel runtimes and
  multi-device workflows remains a later objective and must not be simulated by
  storing incidental resource snapshots in scheduler history.
- **Runtime/device fallback-shaped paths need explicit scoping or removal:**
  Tauri recovery currently hides device-listing failures by passing an empty
  device list into embedding restart, embedding GPU-parallel mode uses
  llama.cpp-local `DeviceBackend::Auto`, and managed-runtime projection has a
  best-effort install-directory fallback. These paths may remain only when they
  are explicitly projection/local-adapter behavior and cannot influence
  scheduler admission or executable runtime/device selection. If they do
  influence execution, replace them with typed diagnostics.

### Re-plan Boundaries

Stop and re-plan before implementation when any remaining slice would require:
- keeping a legacy raw-device, backend, runtime, or graph field as an accepted
  compatibility alias;
- allowing a backend, frontend component, node-engine executor, or worker to
  choose a runtime after the scheduler decision exists;
- making `inference` depend on workflow-service, diagnostics-ledger, frontend,
  Tauri, or graph internals to obtain scheduler facts;
- letting Pantograph infer Pumas artifact paths instead of consuming Pumas-owned
  load targets;
- adding a new polling/background task without a named owner, cancellation
  path, timeout, and teardown test;
- changing runtime-selection algorithm semantics without keeping the policy in
  the scheduler/runtime-registry boundary; or
- changing feature flags, generated bindings, lockfiles, persisted schemas, or
  saved workflow fixtures without updating the matching verification and
  documentation in the same slice.

**Implementation Notes:**

- 2026-05-10 slice: device/runtime contract gate only.
  - Smallest useful vertical slice: add validated DTOs and parser/serde tests
    before modifying backend startup, managed runtime state, runtime registry
    selection, frontend controls, Python workers, saved workflows, generated
    bindings, or lockfiles.
  - Allowed write set:
    `crates/inference/src/device_contracts/`,
    `crates/inference/src/lib.rs`, `crates/inference/src/README.md`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: the slice introduces typed parser
    rejection diagnostics for invalid device ids, runtime variant ids, backend
    ids, and empty scheduler candidate sets. It does not adapt legacy raw
    `DeviceConfig`, `DeviceBackend::from_id`, malformed llama.cpp ordinals, or
    old technical-fit fallback paths into the new contracts.
  - Standards/blast-radius gate for `crates/inference`: crate role remains the
    inference-facing backend contract/facade; public facade impact is additive
    re-exports only; runtime lifecycle owner is unchanged; persisted artifacts
    are not read or written in this slice; no filesystem/path/resource access
    is introduced; feature/dependency impact is none; frontend accessibility
    impact is none; test isolation uses crate-local unit tests plus stable
    serde JSON assertions with no external services or generated fixtures.
  - Discovered issue for follow-up: existing `crates/inference/src/device.rs`
    contained legacy `DeviceBackend::from_id` behavior where unknown ids became
    `Auto` and malformed ordinals became `0`. This first slice recorded and
    isolated the issue instead of preserving it through the new contracts; the
    next parser slice removed that specific behavior.
  - Implemented as decomposed modules under
    `crates/inference/src/device_contracts/` after a decomposition review
    showed the first single-file pass would exceed the repository's file-size
    target.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference device_contracts`, and `git diff --check`.
- 2026-05-10 slice: llama.cpp device parser fallback removal.
  - Smallest useful vertical slice: replace `DeviceBackend::from_id` with a
    fallible backend-local parser and reuse that parser while reading
    `llama.cpp --list-devices` output.
  - Allowed write set: `crates/inference/src/device.rs`,
    `crates/inference/src/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: unknown llama.cpp selectors now return
    `DeviceBackendParseError::Unknown`; missing/malformed ordinals return typed
    parse errors; malformed inventory rows are skipped instead of becoming
    executable device facts; no compatibility `from_id` shim remains.
  - Standards/blast-radius gate for `device.rs`: crate role remains
    backend-local llama.cpp inventory/selector translation; public facade impact
    is a typed `DeviceBackendParseError` re-export and removal of the
    infallible parser; runtime lifecycle owner is unchanged; persisted
    artifacts are not touched; no new filesystem/path/resource access,
    features, dependencies, frontend behavior, generated bindings, or lockfiles
    are introduced; test isolation uses existing crate-local unit tests only.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference device::tests`,
    `cargo test -p inference device_contracts`, and `git diff --check`.
- 2026-05-10 slice: llama.cpp runtime-start device validation.
  - Smallest useful vertical slice: make `BackendConfig::default` carry
    explicit `auto` device intent and make
    `LlamaCppRuntimeSettings::try_from_backend_config` reject missing, blank,
    unknown, or malformed llama.cpp device selectors before sidecar startup.
  - Allowed write set: `crates/inference/src/backend/mod.rs`,
    `crates/inference/src/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: blank or absent raw device strings no
    longer normalize to `auto`; invalid raw selectors fail with typed
    `BackendError::Config` details from the fallible device parser; no backend
    startup path was added that infers CPU, auto, or ordinal zero.
  - Standards/blast-radius gate for `backend/mod.rs`: crate role remains the
    backend facade/settings boundary; public facade impact is limited to manual
    `BackendConfig::default` semantics; runtime lifecycle owner is unchanged;
    persisted artifacts, path validation, frontend controls, generated files,
    feature flags, dependencies, and lockfiles are untouched; test isolation
    uses existing crate-local backend/device/gateway unit tests.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference backend::tests`,
    `cargo test -p inference device::tests`,
    `cargo test -p inference lifecycle_events_do_not_report_auto`, and
    `git diff --check`.
- 2026-05-10 slice: public device-contract fixture gate.
  - Smallest useful vertical slice: add stable JSON fixtures and integration
    tests for runtime variant capability diagnostics and selected backend
    execution decisions through the public `inference` crate API.
  - Allowed write set: `crates/inference/tests/device_contracts.rs`,
    `crates/inference/tests/fixtures/device_contracts/`,
    `crates/inference/tests/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: fixtures use canonical lowercase
    backend/device/runtime ids and include an invalid raw `CUDA0` regression
    check that must fail deserialization instead of being accepted as legacy
    llama.cpp state.
  - Standards/blast-radius gate for integration tests: crate role remains
    public contract verification; public facade, runtime lifecycle, persisted
    runtime state, path validation, frontend behavior, generated files,
    feature flags, dependencies, and lockfiles are untouched; test isolation is
    deterministic fixture decoding only.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference --test device_contracts`, and `git diff --check`.
  - Remaining fixture coverage: Tauri/frontend, diagnostics-ledger, Python
    worker, and persisted-state fixtures are still deferred until those
    boundaries start consuming the device/runtime DTOs.
- 2026-05-10 slice: llama.cpp selector-to-contract projection.
  - Smallest useful vertical slice: add a backend-local projection from
    resolved `DeviceBackend` values into canonical `InferenceDeviceClass` and
    `InferenceDeviceId` facts.
  - Allowed write set: `crates/inference/src/device.rs`,
    `crates/inference/src/lib.rs`, `crates/inference/src/README.md`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: backend-local `auto` returns a typed
    `DeviceBackendContractError::AutoRequiresResolution`, and unsupported
    Vulkan selectors return typed rejection instead of being converted into
    scheduler-selected facts. Raw llama.cpp strings remain adapter-local.
  - Standards/blast-radius gate for `device.rs`: crate role remains
    backend-local parsing/projection; public facade impact is additive error
    re-export and projection method; runtime lifecycle, persisted artifacts,
    path validation, frontend behavior, generated files, feature flags,
    dependencies, and lockfiles are untouched; test isolation uses existing
    crate-local unit tests plus public fixture coverage.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference device::tests`,
    `cargo test -p inference --test device_contracts`, and `git diff --check`.
- 2026-05-10 slice: runtime-load resolved-device contract.
  - Smallest useful vertical slice: make dependency-resolved runtime-load phase
    records require a `DeviceResolutionDecision` alongside managed runtime and
    command facts.
  - Allowed write set: `crates/inference/src/runtime_load.rs`,
    `crates/inference/src/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: dependency resolution can no longer
    emit a runtime-load phase record without the selected runtime variant,
    device class, and selected device id decision. The constructor signature
    requires the typed decision instead of deriving from raw command arguments
    or backend device strings.
  - Standards/blast-radius gate for `runtime_load.rs`: crate role remains pure
    runtime-load contract/projection code; public facade impact is a stricter
    constructor/DTO shape; runtime lifecycle owner is unchanged; persisted
    artifacts, path validation, frontend behavior, generated files, feature
    flags, dependencies, and lockfiles are untouched; test isolation uses
    existing crate-local unit tests and public device-contract fixture tests.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference runtime_load`,
    `cargo test -p inference --test device_contracts`, and `git diff --check`.
- 2026-05-10 slice: active llama.cpp runtime selected-device facts.
  - Smallest useful vertical slice: project validated backend-local device
    state into active llama.cpp runtime descriptors without changing scheduler,
    managed runtime variant state, lifecycle events, frontend controls, worker
    execution, generated bindings, or lockfiles.
  - Allowed write set: `crates/inference/src/runtime_load.rs`,
    `crates/inference/src/server.rs`, `crates/inference/src/server_tests.rs`,
    `crates/inference/src/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: active descriptors emit selected
    device class/id only after `DeviceBackend::try_from_id` and
    `DeviceBackend::to_contract_device` succeed. Invalid raw active device
    state fails closed with no descriptor, while unresolved `auto` and
    unsupported backend-local selectors such as Vulkan omit selected facts
    instead of synthesizing scheduler decisions.
  - Standards/blast-radius gate for `runtime_load.rs` and `server.rs`: crate
    role remains pure runtime-load DTOs plus llama.cpp sidecar lifecycle;
    public facade impact is additive optional fields on the active runtime
    descriptor; runtime lifecycle owner is unchanged; persisted artifacts,
    path validation, frontend behavior, generated files, feature flags,
    dependencies, and lockfiles are untouched; test isolation uses existing
    crate-local sidecar/device unit tests.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference active_runtime_descriptor`,
    `cargo test -p inference device::tests`, and `git diff --check`.
  - Remaining follow-up: gateway lifecycle events, diagnostics ledger
    projection, run inspection facts, scheduler admission, and managed runtime
    variant state still need to consume resolved backend/runtime/device
    decisions instead of raw active backend config.
- 2026-05-10 slice: lifecycle event selected-device class contract.
  - Smallest useful vertical slice: add an optional typed
    `selected_device_class` field to `InferenceRequestLifecycleEvent` and
    update direct event constructors without changing gateway selection logic,
    ledger schema, run inspection, frontend bindings, generated outputs, or
    lockfiles.
  - Allowed write set: `crates/inference/src/types.rs`,
    `crates/inference/src/gateway.rs`, `crates/inference/src/README.md`,
    `crates/inference/tests/model_contracts.rs`,
    `crates/pantograph-embedded-runtime/src/node_execution_ledger_tests.rs`,
    `crates/node-engine/src/core_executor.rs`,
    `crates/node-engine/src/core_executor/dependency_preflight.rs`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: the slice adds a canonical typed field
    and leaves existing producers as `None` unless a test event already
    carries explicit canonical CUDA facts. It does not infer device class from
    raw `BackendConfig.device`, backend-local llama.cpp selectors, or legacy
    event strings.
  - Standards/blast-radius gate for the public lifecycle DTO: crate role
    remains request lifecycle fact transport; public facade impact is additive
    serde-defaulted field only; runtime lifecycle owner is unchanged;
    persisted ledger schema/projection, frontend behavior, generated files,
    feature flags, dependencies, and lockfiles are untouched; test isolation
    uses focused inference contract tests plus the embedded-runtime adapter
    compile/test path.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference inference_request_lifecycle_event_serde_uses_stable_contract`,
    `cargo test -p inference --test model_contracts public_inference_contract_json_keys_avoid_scheduler_policy_language`,
    `cargo test -p pantograph-embedded-runtime inference_lifecycle_event_adapter_builds_node_status_event_with_backend_context`,
    and `git diff --check`.
  - Discovered follow-up: diagnostics-ledger persistence and run inspection do
    not yet store or project `selected_device_class`; this is deferred to a
    ledger-specific slice after gateway producers emit canonical facts.
- 2026-05-10 slice: gateway lifecycle selected-device producer.
  - Smallest useful vertical slice: route request lifecycle event selected
    device class/id through gateway lifecycle context from active llama.cpp
    runtime descriptors, without changing diagnostics-ledger persistence, run
    inspection, scheduler admission, managed runtime variant state, frontend
    bindings, generated outputs, or lockfiles.
  - Allowed write set: `crates/inference/src/gateway.rs`,
    `crates/inference/src/gateway_tests.rs`,
    `crates/inference/src/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: lifecycle events now use
    `active_llamacpp_runtime_descriptor()` selected facts only. A
    config-only raw `BackendConfig.device` value, including explicit
    `"cuda:0"`, is not reported as a selected device unless the active runtime
    descriptor carries canonical class/id facts.
  - Standards/blast-radius gate for `gateway.rs`: crate role remains the
    inference lifecycle facade; public facade shape is unchanged from the prior
    DTO slice; runtime lifecycle owner is unchanged; persisted ledger schema,
    path validation, frontend behavior, generated files, feature flags,
    dependencies, and lockfiles are untouched; test isolation uses crate-local
    gateway lifecycle tests with a mock active llama.cpp descriptor.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference gateway::tests`,
    `cargo test -p inference active_runtime_descriptor`, and
    `git diff --check`.
  - Remaining follow-up: diagnostics-ledger persistence/projection and run
    inspection still do not retain or display selected device class; scheduler
    admission still needs resolved backend/runtime/device decisions end to end.
- 2026-05-10 slice: inference diagnostic selected-device class payload.
  - Smallest useful vertical slice: add `selected_device_class` to
    `InferenceExecutionDiagnosticObservedPayload` and map it from canonical
    inference lifecycle events in the embedded-runtime ledger adapter, without
    changing SQLite projection columns, run-inspection DTOs, scheduler
    admission, frontend bindings, generated outputs, or lockfiles.
  - Allowed write set: `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-diagnostics-ledger/src/README.md`,
    `crates/pantograph-embedded-runtime/src/node_execution_ledger.rs`,
    `crates/pantograph-embedded-runtime/src/node_execution_ledger_tests.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: the adapter copies
    `InferenceRequestLifecycleEvent.selected_device_class` only when that typed
    canonical fact exists. KV-cache and runtime-setting diagnostic payloads
    leave the class unset instead of inferring it from raw backend settings or
    device strings.
  - Standards/blast-radius gate for diagnostics payloads: crate role remains
    durable diagnostics contract/persistence; public facade impact is additive
    serde-defaulted payload field only; runtime lifecycle owner is unchanged;
    SQLite projection schema, path validation, frontend behavior, generated
    files, feature flags, dependencies, and lockfiles are untouched; test
    isolation uses focused diagnostics-ledger payload tests and embedded
    adapter tests.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_appends_inference_execution_diagnostic_summary`,
    `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_projects_inference_diagnostic_selected_facts`,
    `cargo test -p pantograph-embedded-runtime inference_diagnostic_event_adapter_builds_option_support_summary`,
    `cargo test -p pantograph-embedded-runtime inference_diagnostic_event_adapter_drops_path_shaped_runtime_metadata`,
    and `git diff --check`.
  - Remaining follow-up: run-list/run-detail projection columns, workflow
    diagnostics DTOs, and run inspection still need selected-device-class
    projection before consumers can query or display the field without reading
    raw payload JSON.
- 2026-05-10 slice: diagnostics projection selected-device class.
  - Smallest useful vertical slice: project
    `InferenceExecutionDiagnosticObservedPayload.selected_device_class` into
    durable run-list/run-detail SQLite columns, expose it on the public
    projection records and run-list query filter, and carry it through the
    existing workflow diagnostics API/contract snapshots.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/schema.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-diagnostics-ledger/src/README.md`,
    `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
    `crates/pantograph-workflow-service/tests/contract.rs`,
    `crates/pantograph-workflow-service/tests/fixtures/run_projection_contract.json`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: projection copies only the typed
    diagnostic payload field. It does not infer device class from
    `selected_device_id`, raw backend config strings, scheduler device ids, or
    runtime setting values.
  - Standards/blast-radius gate: diagnostics-ledger remains the durable schema
    owner; public facade impact is additive projection DTO/query fields plus a
    projection-version/schema-version bump; runtime lifecycle owners are
    unchanged; no path/resource access, feature flags, dependencies,
    lockfiles, generated files, frontend DOM behavior, or worker execution is
    touched; SQLite migrations are idempotent and covered by an existing-schema
    regression test.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_projects_inference_diagnostic_selected_facts`,
    `cargo test -p pantograph-diagnostics-ledger existing_v19_schema_adds_scheduler_resource_projection_columns`,
    `cargo test -p pantograph-workflow-service workflow_run_`, and
    `git diff --check`.
  - Deviation: the first workflow-service test command attempted to pass two
    test-name filters to Cargo. Cargo rejected that invocation, and it was
    rerun with the single broader `workflow_run_` filter.
  - Remaining follow-up: frontend/run-inspection presentation can now read the
    field from typed projection records, but scheduler admission, managed
    runtime variant readiness, and remaining legacy raw-device execution paths
    still need end-to-end replacement.
- 2026-05-10 slice: workflow technical-fit fallback admission block.
  - Smallest useful vertical slice: make workflow-service runtime preflight
    convert `ConservativeFallback`, `MissingCandidateData`, and
    `MissingRuntimeState` technical-fit decisions into blocking runtime
    diagnostics before workflow run or keep-alive session admission can
    proceed.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/workflow_preflight.rs`,
    `crates/pantograph-workflow-service/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: this slice does not preserve fallback
    selection as executable behavior. Even when a technical-fit decision names
    a selected runtime, fallback or incomplete candidate/runtime-state reasons
    now produce typed blocking `WorkflowRuntimeIssue` diagnostics instead of
    warning-only admission.
  - Standards/blast-radius gate: workflow-service remains the admission and
    runtime-preflight owner; public DTO shape is unchanged; runtime lifecycle,
    persisted schema, frontend behavior, generated files, feature flags,
    dependencies, and lockfiles are untouched; test isolation uses focused
    workflow-service unit/integration tests with mock hosts.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-workflow-service technical_fit_preflight_blocks_fallback_selected_backend`,
    `cargo test -p pantograph-workflow-service workflow_preflight`,
    `cargo test -p pantograph-workflow-service workflow_run_honors_blocking_backend_technical_fit_decision`,
    `cargo test -p pantograph-workflow-service session_runtime_preflight`, and
    `git diff --check`.
  - Discovered issue fixed in-slice: the first implementation still let
    ungrounded fallback decisions bypass runtime readiness through the
    `!enforce_runtime_readiness` early return. The final slice rejects
    fallback/incomplete-state decisions before that early return.
  - Remaining follow-up at commit time: embedded-runtime/runtime-registry
    producers still exposed fallback-named selection modes and reason codes.
    The later DTO cleanup slice retired those compatibility-shaped values.
- 2026-05-10 slice: runtime-registry technical-fit fallback removal.
  - Smallest useful vertical slice: stop runtime-registry from synthesizing
    override fallback candidates or selecting conservative fallback candidates
    when no eligible runtime candidate exists. The selector now returns an
    unselected decision with typed explicit-override, missing-candidate, and
    missing-runtime-state reasons.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`,
    `crates/pantograph-runtime-registry/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: unmatched overrides no longer create
    synthetic executable candidates, and incomplete runtime/candidate state no
    longer returns selected runtime/backend/model facts.
  - Standards/blast-radius gate: runtime-registry remains the backend selector
    owner; public DTO shape is unchanged; workflow-service, embedded-runtime
    adapters, persisted schema, generated files, frontend behavior, feature
    flags, dependencies, and lockfiles are not changed; test isolation uses
    crate-local selector tests plus embedded-runtime projection tests.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`, and
    `git diff --check`.
  - Remaining follow-up at commit time: workflow-service and embedded-runtime
    DTO enums still exposed fallback-named variants. The later DTO cleanup
    slice removed those values.
- 2026-05-10 slice: technical-fit fallback DTO cleanup.
  - Smallest useful vertical slice: remove fallback-named technical-fit DTO
    variants from runtime-registry and workflow-service contracts, remove the
    embedded-runtime projection arms, and update focused tests to use
    automatic/explicit decisions with `MissingCandidateData` or
    `MissingRuntimeState` reason codes.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`,
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/workflow_preflight.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/workflow_run.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/session_runtime_preflight.rs`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: `ConservativeFallback`,
    `OverrideFallback`, and `conservative_fallback` are no longer accepted as
    canonical Rust technical-fit DTO values in the touched contracts. Tests now
    assert typed missing-state diagnostics instead of fallback-shaped selected
    decisions.
  - Standards/blast-radius gate: the touched crates keep their existing
    ownership boundaries; this is a contract cleanup with no persisted schema,
    frontend, generated file, feature flag, dependency, lockfile, runtime
    lifecycle, path/resource, or worker-execution changes; test isolation uses
    focused runtime-registry, embedded-runtime, and workflow-service tests.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit_preflight_blocks_missing_candidate_selected_backend`,
    `cargo test -p pantograph-workflow-service workflow_preflight`,
    `cargo test -p pantograph-workflow-service workflow_run_honors_blocking_backend_technical_fit_decision`,
    `cargo test -p pantograph-workflow-service session_runtime_preflight`, and
    `git diff --check`.
  - Deviation: the first workflow-service verification command attempted
    multiple Cargo test filters and failed before tests ran. It was rerun as
    valid single-filter commands.
  - Remaining follow-up: code search found no fallback-named technical-fit DTO
    values in runtime-registry, workflow-service, or embedded-runtime after the
    slice. Remaining Milestone 5 work is now raw device-string execution,
    managed runtime variant state, frontend device options, and node-engine
    backend routing.
- 2026-05-10 slice: workflow capability `runtime_hint` removal.
  - Smallest useful vertical slice: stop workflow-service capability
    extraction from converting `runtime_hint`/`runtimeHint` values into
    executable backend requirements, and update the current Juggernaut image
    workflow to carry its backend requirement through `backend_key`.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/capabilities.rs`,
    `crates/pantograph-workflow-service/src/README.md`,
    `.pantograph/workflows/juggernaut-x-v10-sdxl.json`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: legacy `runtime_hint` values are no
    longer accepted as a backend-requirement source in workflow-service
    capability extraction. The slice does not add a compatibility mapper or
    infer backend requirements from task names; remaining typed backend
    preference work stays explicit in this milestone.
  - Standards/blast-radius gate: workflow-service remains the capability and
    admission owner; public DTO shape is unchanged; runtime lifecycle,
    persisted schema, frontend behavior, generated files, feature flags,
    dependencies, lockfiles, and worker execution are untouched; test isolation
    uses focused workflow-service capability unit tests plus the default
    capability integration path.
  - Discovered follow-up: other graph editor, embedded-runtime, and node-engine
    paths still mention or consume `runtime_hint`; those paths require separate
    slices because they cross execution, memory-impact, and producer ownership
    boundaries.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-workflow-service capabilities`,
    `cargo test -p pantograph-workflow-service default_capabilities_derive_runtime_requirements_from_workflow`,
    `node -e "JSON.parse(require('fs').readFileSync('.pantograph/workflows/juggernaut-x-v10-sdxl.json', 'utf8'))"`,
    and `git diff --check`.
  - Deviation: `jq empty .pantograph/workflows/juggernaut-x-v10-sdxl.json`
    could not run because `jq` is not installed in the environment. The JSON
    syntax check was rerun with Node.
- 2026-05-10 slice: runtime-setting selected-device raw value removal.
  - Smallest useful vertical slice: stop the embedded-runtime diagnostics
    ledger adapter from copying the raw runtime setting named `device` into
    `InferenceExecutionDiagnosticObservedPayload.selected_device_id`.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/node_execution_ledger.rs`,
    `crates/pantograph-embedded-runtime/src/node_execution_ledger_tests.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: runtime-setting diagnostics remain
    bounded metadata only. They no longer create selected device facts from raw
    backend strings such as `CUDA0`; canonical selected device fields stay
    reserved for lifecycle/device-decision contracts.
  - Standards/blast-radius gate: embedded-runtime remains the ledger adapter
    owner; public DTO shape and SQLite schema are unchanged; runtime lifecycle,
    frontend behavior, generated files, feature flags, dependencies, lockfiles,
    path/resource access, and worker execution are untouched; test isolation
    uses the focused node-execution-ledger adapter test.
  - Discovered follow-up: diagnostics-ledger unit tests can still construct
    synthetic runtime-settings payloads with selected device ids. That is
    ledger contract coverage for externally supplied payloads, not a producer
    path; revisit when the ledger contract is tightened around producer
    provenance.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-embedded-runtime runtime_settings_progress_detail_maps_to_bounded_inference_diagnostic_summary`,
    no-match search for
    `selected_device_id: runtime_setting_value`,
    `fn runtime_setting_value`, and
    `selected_device_id.as_deref(), Some("CUDA0")` in the embedded-runtime
    node-execution-ledger files, and `git diff --check`.
- 2026-05-10 slice: embedded host llama.cpp raw-auto start removal.
  - Smallest useful vertical slice: remove the session-load helper path that
    switched the gateway to llama.cpp and started the requested model with raw
    `device: "auto"` when the model was not already active.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/embedded_workflow_host_helpers.rs`,
    `crates/pantograph-embedded-runtime/src/lib_tests/host_helper_tests.rs`,
    `crates/pantograph-embedded-runtime/src/lib_tests/session_runtime_lifecycle_tests.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the helper now accepts only an already
    active matching llama.cpp runtime as proof. It does not synthesize a
    backend switch, raw auto device config, or GPU-layer default; absent a
    canonical runtime/device decision it fails closed with a runtime diagnostic.
  - Standards/blast-radius gate: embedded-runtime remains the composition-root
    owner; public DTO shape, persisted schema, frontend behavior, generated
    files, feature flags, dependencies, lockfiles, path validation, and worker
    execution are unchanged; test isolation uses a focused helper error test
    without starting a local service.
  - Discovered follow-up: this intentionally disables host-owned implicit
    llama.cpp model loading for inactive requested models until managed runtime
    variant state and selected device decisions are wired end to end.
  - Discovered issue fixed in-slice: an existing session-runtime lifecycle test
    asserted the retired implicit llama.cpp auto-start behavior. The test now
    asserts the runtime diagnostic and proves no backend start request is
    issued.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-embedded-runtime unresolved_llamacpp_device_decision_blocks_host_owned_auto_start`,
    `cargo test -p pantograph-embedded-runtime session_runtime_lifecycle`,
    no-match search for raw-auto llama.cpp start/switch patterns in
    `embedded_workflow_host_helpers.rs`, and `git diff --check`.
  - Deviation: the first verification pass failed on formatting, a missing
    helper-test import, and the old lifecycle test expectation that the helper
    starts llama.cpp automatically. Formatting, the import, and the test
    expectation were fixed before rerunning verification.
- 2026-05-10 slice: workflow graph helper `runtime_hint` signal removal.
  - Smallest useful vertical slice: remove `runtime_hint` from workflow-service
    graph edit input-priority metadata and KV-cache memory-impact
    backend-change detection, then update focused fixtures to use
    `backend_key`.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/graph/connection_insert.rs`,
    `crates/pantograph-workflow-service/src/graph/memory_impact.rs`,
    `crates/pantograph-workflow-service/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: workflow-service graph helpers no
    longer treat retired `runtime_hint` fields as current runtime/backend
    signals or preferred connection targets. No compatibility alias is added.
  - Standards/blast-radius gate: workflow-service remains the graph helper and
    memory-impact owner; public DTO shape, runtime lifecycle, persisted schema,
    frontend behavior, generated files, feature flags, dependencies, lockfiles,
    path/resource access, and worker execution are unchanged; test isolation
    uses focused workflow-service graph memory-impact tests.
  - Discovered follow-up: node-engine and embedded-runtime execution request
    contracts still parse `runtime_hint`; those paths cross execution
    ownership and require separate replacement slices.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-workflow-service graph::memory_impact`,
    no-match search for `runtime_hint`, `runtimeHint`, and
    `runtime_hint_details` in the touched graph helper files, and
    `git diff --check`.
  - Deviation: the first formatting check failed on one rustfmt line wrap.
    `cargo fmt --all` was run and verification was repeated successfully.
- 2026-05-10 slice: dependency preflight `runtime_hint` backend preference
  removal.
  - Smallest useful vertical slice: remove `runtime_hint`/`runtimeHint` from
    embedded-runtime dependency-environment backend preference selection and
    pin the behavior with a focused task-executor test.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/task_executor/dependency_environment.rs`,
    `crates/pantograph-embedded-runtime/src/task_executor_tests.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: dependency preflight no longer treats
    legacy `runtime_hint` as executable backend preference. It uses current
    `backend_key`, package facts, or dependency requirements only, with typed
    backend preference intent still a separate remaining milestone task.
  - Standards/blast-radius gate: embedded-runtime remains the dependency
    preflight adapter owner; public DTO shape, runtime lifecycle, persisted
    schema, frontend behavior, generated files, feature flags, dependencies,
    lockfiles, path/resource access, and worker execution are unchanged; test
    isolation uses focused task-executor unit tests with in-memory stubs.
  - Discovered follow-up: node-engine inference request DTOs and dependency
    preflight still parse `runtime_hint`; those require a node-engine contract
    replacement slice.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-embedded-runtime preferred_backend_key_ignores_legacy_runtime_hint`,
    `cargo test -p pantograph-embedded-runtime canonical_llm_inference_falls_through_to_core_executor`,
    no-match search for `runtime_hint`/`runtimeHint` in
    `task_executor/dependency_environment.rs`, and `git diff --check`.
- 2026-05-10 slice: embedded host llama.cpp `runtime_hint` detection removal.
  - Smallest useful vertical slice: remove `runtime_hint`/`runtimeHint` from
    embedded host llama.cpp inference-node detection and update the session
    runtime lifecycle fixture to use `backend_key`.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/embedded_workflow_host_helpers.rs`,
    `crates/pantograph-embedded-runtime/src/lib_tests/session_runtime_lifecycle_tests.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the helper no longer treats
    `runtime_hint` as evidence that a workflow should resolve a llama.cpp model
    path. Current backend/package facts are required.
  - Standards/blast-radius gate: embedded-runtime remains the workflow host
    helper owner; public DTO shape, runtime lifecycle, persisted schema,
    frontend behavior, generated files, feature flags, dependencies,
    lockfiles, path/resource access, and worker execution are unchanged; test
    isolation uses the session runtime lifecycle coverage.
  - Discovered follow-up: `embedding_workflow` still has runtime-hint-specific
    embedding selection helpers and fixtures that need their own replacement
    slice.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-embedded-runtime session_runtime_lifecycle`,
    no-match search for `runtime_hint`/`runtimeHint` in
    `embedded_workflow_host_helpers.rs`, and `git diff --check`.
- 2026-05-10 slice: embedding workflow `runtime_hint` detection removal.
  - Smallest useful vertical slice: remove `runtime_hint`/`runtimeHint` from
    embedded-runtime embedding workflow non-embedding llama.cpp detection and
    update focused helper fixtures to use `backend_key`.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/embedding_workflow.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: embedding workflow helpers no longer
    treat legacy runtime hints as evidence of llama.cpp targeting. They use
    current backend/package facts only.
  - Standards/blast-radius gate: embedded-runtime remains the embedding
    workflow helper owner; public DTO shape, runtime lifecycle, persisted
    schema, frontend behavior, generated files, feature flags, dependencies,
    lockfiles, path/resource access, and worker execution are unchanged; test
    isolation uses focused helper unit tests.
  - Discovered follow-up: node-engine inference request DTOs still expose and
    parse `runtime_hint`, which is a broader contract replacement slice.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-embedded-runtime workflow_graph_embedding_helpers_detect_embedding_and_llamacpp_nodes`,
    no-match search for `runtime_hint`/`runtimeHint` in
    `embedding_workflow.rs`, and `git diff --check`.
- 2026-05-10 slice: node-engine dependency-preflight `runtime_hint` backend
  preference removal.
  - Smallest useful vertical slice: remove `runtime_hint`/`runtimeHint` from
    node-engine dependency-preflight backend preference selection, update the
    focused dependency request test, and update retired-node guidance to name
    `backend_key`.
  - Allowed write set:
    `crates/node-engine/src/core_executor/dependency_preflight.rs`,
    `crates/node-engine/src/core_executor/inference_tests.rs`,
    `crates/node-engine/src/core_executor.rs`,
    `crates/node-engine/src/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: dependency preflight no longer treats
    legacy runtime hints as backend preference input. Current backend keys,
    package facts, or typed task/model inference drive preflight routing until
    typed backend preference intent is wired.
  - Standards/blast-radius gate: node-engine remains the host-agnostic node
    execution owner; public inference request DTO shape is unchanged in this
    slice; runtime lifecycle, persisted schema, frontend behavior, generated
    files, feature flags, dependencies, lockfiles, path/resource access, and
    worker execution are unchanged; test isolation uses focused node-engine
    dependency-preflight tests.
  - Discovered follow-up: `InferenceExecutionRequest.runtime_hint` remains a
    public request field and multiple inference-node builders still parse it.
    Removing that field is a broader contract replacement slice touching
    inference DTO fixtures and node-engine request builders.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p node-engine --features inference-nodes test_build_model_dependency_request_ignores_legacy_runtime_hint`,
    `cargo test -p node-engine --features inference-nodes test_build_model_dependency_request_uses_canonical_backend_key`,
    `cargo test -p node-engine --features inference-nodes test_build_model_dependency_request_uses_canonical_llamacpp_hint`,
    no-match search for `runtime_hint`/`runtimeHint` in
    `core_executor/dependency_preflight.rs`, and `git diff --check`.
  - Deviation: the first node-engine test commands omitted
    `--features inference-nodes` and matched zero feature-gated tests. They
    were rerun with the feature enabled.
- 2026-05-10 slice: typed inference request `runtime_hint` contract removal.
  - Smallest useful vertical slice: remove
    `InferenceExecutionRequest.runtime_hint` from the public inference DTO and
    stop node-engine inference request builders from reading
    `runtime_hint`/`runtimeHint` into typed execution requests.
  - Allowed write set:
    `crates/inference/src/types.rs`,
    `crates/inference/src/README.md`,
    `crates/inference/src/gateway_tests.rs`,
    `crates/inference/tests/model_contracts.rs`,
    `crates/node-engine/src/core_executor.rs`,
    `crates/node-engine/src/core_executor/inference_nodes.rs`,
    `crates/node-engine/src/core_executor/inference_tests.rs`,
    `crates/node-engine/src/engine/node_preparation.rs`,
    `crates/node-engine/src/engine/workflow_execution_session.rs`,
    `crates/node-engine/src/engine/workflow_execution_session/tests/workflow_execution_session_tests/kv_cache_memory.rs`,
    `crates/pantograph-embedded-runtime/src/lib_tests.rs`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: typed inference execution requests no
    longer carry backend/runtime preference strings. Backend/runtime/device
    decisions remain owned by scheduler-facing candidate/decision contracts;
    no compatibility field or alias was retained on the request DTO.
    Package-facts text-generation requests also stay on the typed gateway path
    instead of allowing llama.cpp backend hints to invoke the old direct
    execution branch.
  - Standards/blast-radius gate: inference remains the public typed execution
    DTO owner and node-engine remains the request-builder/execution-routing
    owner; persisted SQLite schema, frontend behavior, generated files, feature
    flags, dependencies, lockfiles, path/resource access, and worker execution
    are unchanged; test isolation uses focused inference serde/contract tests,
    gateway tests, node-engine request-builder/execution tests, and fixture
    projection tests.
  - Discovered issue fixed in slice: broad node-engine verification exposed
    package-facts text-generation requests being routed to the old llama.cpp
    execution branch when package backend hints resolved to llama.cpp. The
    executor now handles explicit text-generation contracts and prompt-bearing
    package-facts text requests through the typed gateway before backend-key
    legacy dispatch.
  - Discovered follow-up: one focused node-engine regression test still uses a
    `runtime_hint` input to prove dependency-preflight ignores the legacy field;
    remove that test input once all saved graph/producer paths have stopped
    emitting it.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference typed_execution_request`,
    `cargo test -p inference --test model_contracts inference_execution_request_wire_contract_preserves_tags_defaults_and_unknown_fields`,
    `cargo test -p inference gateway::tests`,
    `cargo test -p node-engine --features inference-nodes core_executor::tests::inference_tests`,
    `cargo test -p node-engine --features pytorch-nodes test_canonical_llm_pytorch_backend_key_dispatches_to_dependency_preflight`,
    `cargo test -p node-engine --features pytorch-nodes test_dependency_preflight_records_lifecycle_success_with_resolver`,
    `cargo test -p node-engine --features inference-nodes kv_cache_memory`,
    `cargo test -p node-engine --features inference-nodes node_preparation`,
    `cargo test -p pantograph-embedded-runtime workflow_graph_embedding_helpers_detect_embedding_and_llamacpp_nodes`,
    runtime-hint search over touched Rust/README scopes, and
    `git diff --check`.
  - Verification deviations: an initial model-contract command used the stale
    `canonical_rerank_execution_request_serde_defaults_and_ignores_unknown_fields`
    filter and matched zero tests, then the correct
    `inference_execution_request_wire_contract_preserves_tags_defaults_and_unknown_fields`
    test was run. The first full node-engine inference module run failed two
    package-facts text-routing tests; after routing prompt-bearing package-facts
    text requests through the typed gateway, the module was rerun successfully.
    A second intermediate run failed the image-generation package-facts test
    because the prompt-bearing shortcut ran before explicit image-generation
    dispatch; the guard was moved after all explicit typed task arms and the
    module was rerun successfully.
- 2026-05-10 slice: graph-visible `runtime_hint` descriptor removal.
  - Smallest useful vertical slice: replace the graph/frontend
    `llm-inference` descriptor input, mocks, built-in templates, and tracked
    Tiny SD saved workflow data from legacy `runtime_hint` to canonical
    `backend_key`.
  - Allowed write set:
    `crates/workflow-nodes/src/processing/inference.rs`,
    `crates/workflow-nodes/src/contracts.rs`,
    `crates/workflow-nodes/src/README.md`,
    `packages/svelte-graph/src/backends/MockWorkflowBackend.ts`,
    `src/services/workflow/mocks.ts`,
    `src/services/workflow/WorkflowService.commands.test.ts`,
    `src/services/workflow/templateService.test.ts`,
    `src/templates/workflows/tiny-sd-turbo-text-to-image.json`,
    `src/templates/workflows/gguf-reranker-workflow.json`,
    `.pantograph/workflows/tiny-sd-turbo-diffusion.json`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: no `runtime_hint` alias or descriptor
    compatibility port was retained. Executable diffusion image-generation
    graph intent now uses `backend_key = "pytorch"`; `diffusers` remains
    package/runtime capability evidence, not a graph-visible backend
    preference.
  - Standards/blast-radius gate: workflow-nodes remains the Rust descriptor
    owner, frontend mocks/tests remain frontend-only contract fixtures, and
    saved/template workflow edits are intentional current-shape fixture
    updates. No generated files, lockfiles, feature flags, dependencies,
    sqlite state, worker execution, or runtime process ownership changed.
  - Discovered issue fixed in slice: focused Node template verification failed
    after template updates because tracked `.pantograph` Tiny SD workflow data
    still persisted `runtime_hint`; the saved workflow fixture was rewritten to
    `backend_key`.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p workflow-nodes --lib inference`,
    `node --experimental-strip-types --test src/services/workflow/WorkflowService.commands.test.ts src/services/workflow/templateService.test.ts`,
    `npm run typecheck`,
    no-match search for runtime-hint descriptor/template data in the touched
    Rust/frontend/template/workflow scopes except explicit negative assertions
    and README history, and `git diff --check`.
  - Verification deviation: the first focused Node command failed
    `templateService.test.ts` because `.pantograph/workflows/tiny-sd-turbo-diffusion.json`
    still used `runtime_hint`; it was updated and the same command passed.
- 2026-05-10 slice: workflow-service current graph fixture backend-key cleanup.
  - Smallest useful vertical slice: update workflow-service current graph
    canonicalization/session test fixtures from `runtime_hint` to
    `backend_key`, while retaining the explicit capability tests that prove
    legacy runtime hints are ignored.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/graph/canonicalization_tests.rs`,
    `crates/pantograph-workflow-service/src/graph/session_tests.rs`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: current workflow-service graph
    fixtures now use canonical `backend_key`; no migration, alias, or
    compatibility behavior was added for old graph data.
  - Standards/blast-radius gate: this is a test-fixture-only workflow-service
    cleanup. Public DTOs, persistence code, generated files, frontend files,
    lockfiles, feature flags, dependencies, sqlite state, and runtime execution
    are unchanged.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-workflow-service graph::`,
    `cargo test -p pantograph-workflow-service extract_required_backends`,
    no-match search for `runtime_hint` in workflow-service graph tests, and
    `git diff --check`.
  - Verification deviation: an initial cargo command attempted to pass three
    test filters at once, which Cargo rejected; verification was rerun with the
    broader `graph::` filter.
- 2026-05-10 slice: llama.cpp sidecar startup device-selector validation.
  - Smallest useful vertical slice: validate backend-local llama.cpp
    `DeviceConfig.device` values before inference, embedding, or reranking
    sidecar startup stops an existing runtime or spawns `llama-server`.
  - Allowed write set:
    `crates/inference/src/server.rs`, `crates/inference/src/server_tests.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: malformed selectors such as `CUDAx`
    now fail before process spawn instead of being passed through to
    `llama-server` or erased from selected-device facts. No unknown-to-auto,
    ordinal-to-zero, or compatibility parser path was added.
  - Standards/blast-radius gate: this stays inside the inference crate's
    llama.cpp sidecar lifecycle owner. Public DTOs, managed runtime catalog
    state, frontend behavior, generated files, lockfiles, feature flags,
    dependencies, sqlite state, and worker execution are unchanged.
  - Remaining follow-up: `active_runtime_descriptor()` still exposes `Option`
    and therefore cannot return the typed parse diagnostic for already-mutated
    invalid test state. Startup now prevents that state for normal sidecar
    entry points; changing the descriptor API is a broader compatibility slice.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference sidecar_start_rejects_invalid_device_before_spawning`,
    `cargo test -p inference active_runtime_descriptor`,
    `cargo test -p inference start_sidecar_inference`, and `git diff --check`.
  - Verification deviation: the first format check failed after the new test
    because one assertion needed rustfmt compaction; `cargo fmt --all` was run
    and the format check passed afterward.
- 2026-05-10 slice: dedicated embedding sidecar device-selector validation.
  - Smallest useful vertical slice: validate backend-local llama.cpp
    `DeviceConfig.device` values before the dedicated embedding sidecar builds
    command arguments or calls the process spawner.
  - Allowed write set:
    `crates/inference/src/embedding_runtime.rs` and this plan directory.
  - No-fallback/no-legacy confirmation: malformed selectors now fail before
    process spawn in the dedicated embedding runtime path. No unknown-to-auto,
    ordinal-to-zero, or compatibility parser path was added.
  - Standards/blast-radius gate: this stays inside the inference crate's
    dedicated embedding sidecar lifecycle owner. Public DTOs, managed runtime
    catalog state, frontend behavior, generated files, lockfiles, feature
    flags, dependencies, sqlite state, and worker execution are unchanged.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference embedding_runtime::tests::start_server_rejects_invalid_device_before_spawning`,
    `cargo test -p inference embedding_runtime::tests`, and
    `git diff --check`.
- 2026-05-10 slice: runtime capability technical-fit source rename.
  - Smallest useful vertical slice: remove the remaining fallback-named
    runtime-registry candidate source kind by renaming
    `RuntimeCapabilityFallback` to `RuntimeCapabilityFacts` and updating the
    embedded-runtime producer to emit the new source.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`,
    `crates/pantograph-runtime-registry/README.md`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the retired
    `runtime_capability_fallback` wire value is rejected in a focused serde
    regression test. No alias, migration, or compatibility branch was added.
  - Standards/blast-radius gate: this stays inside the runtime-registry
    technical-fit contract and the embedded-runtime projection producer.
    Workflow-service DTOs, persisted schema, frontend behavior, generated
    files, lockfiles, feature flags, dependencies, sqlite state, managed
    runtime variant state, and worker execution are unchanged.
  - Discovered issue fixed in-slice: code search found that the earlier
    fallback DTO cleanup missed `RuntimeCapabilityFallback` because it was a
    candidate source kind rather than a selection mode or reason code.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry runtime_capability_source_kind`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `rg -n "RuntimeCapabilityFallback|runtime_capability_fallback" crates/pantograph-runtime-registry crates/pantograph-embedded-runtime crates/pantograph-workflow-service -g '!target'`,
    and `git diff --check`.
  - Verification deviation: the first regression-test attempt used
    `serde_json`, but `pantograph-runtime-registry` intentionally has no
    `serde_json` dependency. The test was rewritten to use `serde`'s typed
    string deserializer, avoiding dependency or lockfile changes.
- 2026-05-10 slice: frontend synthetic device option removal.
  - Smallest useful vertical slice: stop `DeviceConfig.svelte` from creating a
    synthetic CPU-only device list after backend device discovery fails and
    cover device option projection through the repository's existing Node test
    runner.
  - Allowed write set:
    `src/components/DeviceConfig.svelte`,
    `src/components/deviceConfigPresenters.ts`,
    `src/components/deviceConfigPresenters.test.ts`,
    `src/components/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: discovery failures now keep
    `availableDevices` empty and display unavailable state. The frontend does
    not add CPU, auto, or llama.cpp-specific executable choices unless the
    backend reports them.
  - Standards/blast-radius gate: this stays inside the settings-side frontend
    presentation layer. Backend contracts, generated files, lockfiles,
    persisted schema, runtime execution, sqlite state, and worker execution are
    unchanged; accessibility keeps the existing labeled native select and
    disables it when no backend-confirmed options exist.
  - Verification passed:
    `node --experimental-strip-types --test src/components/deviceConfigPresenters.test.ts`,
    `npm run typecheck`,
    `rg -n "Provide fallback options|CPU Only|let llama-server choose|buildBackendConfirmedDeviceOptions|deviceLoadError" src/components/DeviceConfig.svelte src/components/deviceConfigPresenters.ts src/components/deviceConfigPresenters.test.ts`,
    and `git diff --check`.
- 2026-05-10 slice: gateway mode-info selected-device projection.
  - Smallest useful vertical slice: stop gateway `mode_info()` runtime facts
    from promoting raw `BackendConfig.device` strings into
    `active_resolved_device` and instead source the field from the active
    llama.cpp runtime descriptor's canonical selected device id.
  - Allowed write set:
    `crates/inference/src/gateway.rs`,
    `crates/inference/src/gateway_tests.rs`,
    `crates/inference/src/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: config-only explicit device strings no
    longer appear as selected runtime facts. If the active backend does not
    expose canonical selected-device facts, `mode_info()` leaves the resolved
    device unset instead of inferring a scheduler decision.
  - Standards/blast-radius gate: this stays inside the inference gateway
    projection path. Public DTO shape, backend startup, managed runtime state,
    frontend files, generated files, lockfiles, persisted schema, sqlite state,
    feature flags, dependencies, and worker execution are unchanged.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference test_mode_info_runtime_facts`,
    `cargo test -p inference test_lifecycle_events_do_not_report_config_only_device_as_selected`,
    and `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` reported a
    rustfmt wrapping difference in `gateway.rs`; `cargo fmt --all` was run,
    and the format check then passed.
- 2026-05-10 slice: backend capability runtime-variant facts.
  - Smallest useful vertical slice: extend the existing backend capability
    facts with runtime variant facts and project those facts through
    workflow-service, embedded-runtime, and TypeScript workflow mirrors.
  - Allowed write set:
    `crates/inference/src/backend/`,
    `crates/pantograph-workflow-service/src/workflow/contracts.rs`,
    `crates/pantograph-workflow-service/src/lib.rs`,
    `crates/pantograph-workflow-service/tests/contract.rs`,
    `crates/pantograph-embedded-runtime/src/runtime_capabilities.rs`,
    `src/services/workflow/types.ts`, relevant module READMEs, and this plan
    directory.
  - No-fallback/no-legacy confirmation: runtime variant facts report explicit
    availability or typed unavailability diagnostics only. CUDA, Metal/MPS, and
    staged Candle facts are not converted into executable CPU/auto selections,
    and the projection does not infer readiness from backend names or raw
    device strings.
  - Standards/blast-radius gate: the slice changes append-only serde DTOs and
    static adapter facts. Runtime lifecycle ownership, scheduler ranking,
    managed runtime state, persisted schemas, lockfiles, workflow fixtures,
    worker execution, subprocess startup, and frontend executable-device state
    are unchanged. Test isolation uses Rust unit/contract tests plus root
    TypeScript typechecking with no external services.
  - Implemented facts: llama.cpp now reports CPU plus unavailable CUDA and
    macOS Metal variant facts; PyTorch reports CPU plus unavailable CUDA and
    macOS MPS variant facts; Candle reports unavailable CPU/CUDA and macOS
    Metal placeholder facts while keeping image generation unavailable.
  - Discovered follow-up: vLLM and MLX do not yet have registered backend
    capability providers in this crate, so their placeholder/runtime variant
    facts remain a later provider-registration slice rather than a hidden
    frontend or scheduler assumption.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference backend::capability_tests`,
    `cargo test -p inference --all-features test_capabilities`,
    `cargo check -p inference --no-default-features`,
    `cargo test -p pantograph-embedded-runtime runtime_capabilities`,
    `cargo test -p pantograph-workflow-service --test contract workflow_capabilities_contract_snapshot`,
    `npm run typecheck`, and `git diff --check`.
  - Verification deviation: `npm run -w frontend check:types` failed because
    this repository root has no `frontend` workspace. The equivalent current
    script is the root `npm run typecheck`, which passed.
- 2026-05-10 slice: future hardware/offload reservation notes.
  - Smallest useful vertical slice: document the unsupported hardware and
    offload families that must remain reserved contract space until a later
    provider/probe/admission plan implements them.
  - Allowed write set:
    `docs/plans/current-image-generation-graphs/06-device-runtime-selection.md`,
    this milestone file, and `05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the notes explicitly forbid exposing
    ROCm/HIP, Vulkan, XPU/iGPU, OpenVINO, hybrid/offload, remote plugins, or
    MLX as executable options, synthetic frontend choices, or implicit
    CPU/auto/MPS/Metal/PyTorch fallbacks before typed backend facts exist.
  - Standards/blast-radius gate: documentation-only slice; no source,
    generated files, lockfiles, workflow fixtures, schemas, runtime state,
    subprocess behavior, frontend behavior, or dependencies changed.
  - Verification passed: `rg -n "Future Support Reservation Notes|ROCm/HIP|Remote hardware plugins|MLX" docs/plans/current-image-generation-graphs/06-device-runtime-selection.md`
    and `git diff --check`.
- 2026-05-10 slice: technical-fit device policy intent.
  - Smallest useful vertical slice: add typed `auto`/explicit
    CPU/CUDA/Metal/MPS device policy intent to workflow-service and
    runtime-registry technical-fit requests, then project it through
    embedded-runtime without changing selector ranking.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/lib.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/lib.rs`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `src/services/workflow/types.ts`, relevant module READMEs, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the new DTO carries user intent only.
    It does not translate to backend-local device strings, choose a fallback
    runtime, or map unsupported future device families to CPU/auto/MPS/Metal.
    The device policy enums are intentionally closed for this slice so unknown
    wire values fail decoding until a later typed contract adds them.
  - Standards/blast-radius gate: this slice changes append-only technical-fit
    request DTOs and sync projection helpers only. Runtime selection ranking,
    backend startup/load, managed runtime state, persisted schemas, workflow
    fixtures, subprocess behavior, lockfiles, generated files, and worker
    execution are unchanged.
  - Discovered follow-up: explicit-device rejection is still not enforced by
    selector candidate facts. That remains blocked on the next technical-fit
    candidate/decision slice that carries runtime variant, device class/id, and
    device diagnostics.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `npm run typecheck`, and `git diff --check`.
- 2026-05-10 slice: technical-fit selected runtime/device facts.
  - Smallest useful vertical slice: extend runtime-registry technical-fit
    candidates and decisions with runtime variant id, typed device class,
    selected device id, resource estimate, observed-throughput hint, and
    bounded device diagnostic fields, then project those fields through
    workflow-service and embedded-runtime DTOs plus the TypeScript workflow
    mirror.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/lib.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/lib.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `src/services/workflow/types.ts`, relevant module READMEs, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the selector only copies explicit
    candidate facts into the selected decision. It does not derive device class
    or selected device id from backend keys, runtime ids, raw backend config
    strings, or unknown runtime variant facts. Unknown workflow runtime variant
    device classes become typed diagnostics instead of CPU/auto mappings.
  - Standards/blast-radius gate: this slice changes append-only DTO fields,
    synchronous normalization/projection helpers, focused Rust tests, and
    handwritten TypeScript mirrors only. Runtime ranking, backend startup/load,
    managed runtime installation state, persisted schemas, workflow fixtures,
    subprocess behavior, worker execution, generated files, dependencies, and
    lockfiles are unchanged.
  - Discovered follow-up: explicit-device rejection is still admission policy
    work. The selector can now carry the necessary runtime/device facts and
    diagnostics, but the next slices still need to compare explicit device
    policy intent against candidate facts and block unavailable devices before
    backend load.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `npm run typecheck`, and `git diff --check`.
- 2026-05-10 slice: runtime-registry explicit-device technical-fit rejection.
  - Smallest useful vertical slice: make runtime-registry technical-fit
    selection compare explicit device policy intent against candidate
    device-class/id facts and return an unselected decision with a bounded
    `ExplicitDeviceUnavailable` diagnostic when no candidate matches.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: explicit CUDA/Metal/MPS/CPU requests
    are not satisfied by another candidate, auto mode, CPU fallback, backend
    key inference, runtime id inference, or raw backend config strings. The
    selector uses only typed candidate device facts.
  - Standards/blast-radius gate: this slice changes synchronous selector
    filtering, typed diagnostics, focused unit tests, and local documentation
    only. Workflow-service admission, embedded-runtime projection, frontend,
    managed runtime state, persisted schemas, workflow fixtures, subprocess
    behavior, workers, dependencies, generated files, and lockfiles are
    unchanged.
  - Discovered follow-up: workflow admission still needs an end-to-end test
    proving explicit unavailable device diagnostics block run/session admission
    after host projection. That remains before marking the broader explicit
    device admission task complete.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`, and
    `git diff --check`.
- 2026-05-10 slice: technical-fit runtime preference intent.
  - Smallest useful vertical slice: extend workflow-service and
    runtime-registry technical-fit override intent with runtime id and runtime
    variant id, project those fields through embedded-runtime, and have the
    runtime-registry selector match the fields against candidate facts.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`,
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/`,
    `crates/pantograph-workflow-service/src/scheduler/store_tests.rs`,
    `crates/pantograph-workflow-service/src/README.md`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `crates/pantograph-embedded-runtime/src/lib_tests/workflow_run_execution_tests.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`,
    `src/services/workflow/types.ts`, `src/services/workflow/README.md`, and
    this plan directory.
  - No-fallback/no-legacy confirmation: unmatched runtime or runtime-variant
    intent returns an unselected technical-fit decision with explicit override
    and missing-candidate reasons. The selector does not synthesize runtime
    candidates, infer variants from backend keys, or preserve override-fallback
    behavior.
  - Standards/blast-radius gate: this slice changes append-only DTO fields,
    selector matching, projection helpers, focused Rust tests, TypeScript type
    mirrors, and documentation only. Runtime ranking policy, backend
    startup/load, managed runtime state, persisted schemas, workflow fixtures,
    subprocess behavior, workers, dependencies, generated files, and lockfiles
    are unchanged.
  - Discovered follow-up: task/model/platform incompatibility for explicit
    backend/runtime preferences remains broader admission policy work. This
    slice only makes runtime and variant intent representable and rejects
    unmatched selector candidates.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `npm run typecheck`, and `git diff --check`.
  - Verification deviation: the first chained verification attempt hit
    unrelated Pumas temporary SQLite `attempt to write a readonly database`
    failures in two embedded-runtime technical-fit tests. Rerunning
    `cargo test -p pantograph-embedded-runtime technical_fit` immediately
    passed.
- 2026-05-10 slice: explicit override eligibility gate.
  - Smallest useful vertical slice: make runtime-registry technical-fit
    explicit overrides reuse the canonical candidate eligibility predicate so
    matching backend/runtime/model/variant candidates are still rejected when
    task/model/runtime requirements say they cannot execute.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: explicit backend/runtime overrides no
    longer bypass compatibility facts. An ineligible matching candidate returns
    an unselected decision with explicit override and missing-candidate reasons
    rather than becoming an executable fallback selection.
  - Standards/blast-radius gate: this slice changes selector filtering,
    focused runtime-registry tests, and local documentation only. Cross-crate
    DTO shapes, workflow admission, embedded-runtime projection, frontend,
    managed runtime state, persisted schemas, workflow fixtures, subprocess
    behavior, workers, dependencies, generated files, and lockfiles are
    unchanged.
  - Discovered follow-up: broader backend/runtime preference rejection still
    needs end-to-end admission coverage for concrete task/model/platform cases
    after backend adapters publish complete candidate facts.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`, and
    `git diff --check`.
- 2026-05-10 slice: workflow preflight explicit-device diagnostic block.
  - Smallest useful vertical slice: make workflow-service runtime preflight
    treat error-severity technical-fit device diagnostics as blocking runtime
    issues, even when the selector decision has no legacy missing-candidate or
    missing-runtime-state reason.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: explicit unavailable device decisions
    remain typed diagnostics and are not converted into CPU, auto, selected
    runtime, selected backend, or warning-only admission behavior.
  - Standards/blast-radius gate for workflow-service technical-fit preflight:
    crate role remains admission/runtime-preflight orchestration; public DTO
    shape, runtime lifecycle ownership, persisted schemas, frontend behavior,
    generated files, feature flags, dependencies, lockfiles, worker execution,
    and workflow fixtures are untouched; test isolation uses focused
    workflow-service unit coverage with no external services.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-workflow-service technical_fit`, and
    `git diff --check`.
  - Remaining follow-up: broader explicit backend/runtime incompatibility
    cases still need concrete task/model/platform candidate facts before their
    end-to-end admission tests can be marked complete.
- 2026-05-10 slice: workflow technical-fit cross-layer fixture.
  - Smallest useful vertical slice: add a shared JSON fixture for the
    workflow-service technical-fit request/decision boundary and consume it
    from both Rust serde tests and the existing Node frontend test harness.
  - Allowed write set:
    `crates/pantograph-workflow-service/tests/contract.rs`,
    `crates/pantograph-workflow-service/tests/fixtures/technical_fit_contract.json`,
    `src/services/workflow/WorkflowService.commands.test.ts`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the fixture pins explicit typed
    runtime id, runtime variant id, backend key, device policy, selected device
    class/id, resource estimate, throughput hint, and bounded device
    diagnostics. It does not include legacy raw device strings, fallback-named
    selection modes, or runtime-hint fields.
  - Standards/blast-radius gate for cross-layer fixture coverage: workflow
    service remains the Rust DTO owner; frontend types remain a hand-maintained
    mirror consumed by the existing Node test platform; production behavior,
    runtime lifecycle ownership, persisted schemas, generated files, feature
    flags, dependencies, lockfiles, workers, and workflow fixtures are
    untouched.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-workflow-service --test contract workflow_technical_fit_cross_layer_fixture_deserializes`,
    `node --experimental-strip-types --test src/services/workflow/WorkflowService.commands.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining fixture coverage: runtime-registry, Tauri command transport,
    diagnostics-ledger technical-fit projections if added, Python worker, and
    persisted-state boundaries still need their own fixtures before the broad
    serde-fixture checklist item can be closed.
- 2026-05-10 slice: runtime-registry auto ambiguity rejection.
  - Smallest useful vertical slice: replace automatic deterministic
    candidate-id tie-break selection with an unselected
    `ambiguous_auto_resolution` diagnostic when multiple eligible candidates
    have equal selector priority.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: auto mode must now resolve exactly one
    highest-ranked candidate. Ambiguity is not converted into a selected
    runtime/backend/device by stable candidate ordering, CPU defaults, or old
    raw-device behavior.
  - Standards/blast-radius gate for runtime-registry selector policy:
    runtime-registry remains the owner of selector policy and reason/diagnostic
    contracts; workflow-service and embedded-runtime projection DTO shapes are
    unchanged; runtime lifecycle ownership, persisted schemas, frontend
    behavior, generated files, feature flags, dependencies, lockfiles, workers,
    and workflow fixtures are untouched; test isolation uses focused
    runtime-registry selector tests plus projection/admission smoke coverage.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`, and
    `git diff --check`.
  - Remaining follow-up: zero-valid-candidate auto-mode diagnostics still use
    missing-candidate/runtime-state reasons, and remaining old raw-device
    execution paths still need removal before the broad auto-mode checklist item
    can be closed.
- 2026-05-12 slice: runtime-registry controlled exploration policy.
  - Smallest useful vertical slice: replace the temporary equal-priority
    `ambiguous_auto_resolution` terminal automatic result with first-class
    selector policy evidence and deterministic controlled exploration for
    equal-ranked eligible candidates.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: automatic selection still hard-filters
    invalid candidates and returns typed diagnostics for no valid candidate,
    explicit incompatibility, or unrankable policy state. Equal-ranked valid
    candidates are selected by recorded scheduler policy; no old raw-device,
    backend-default, candidate-id tie-break, or compatibility shim is used.
  - Standards/blast-radius gate for runtime-registry selector policy:
    runtime-registry remains the pure synchronous selector owner; workflow
    graphs, Pumas facts, diagnostics-ledger persistence, runtime lifecycle,
    frontend behavior, generated files, lockfiles, workers, and workflow
    fixtures are untouched. The new trace is built from already-normalized
    candidate facts and uses checked count conversion before projecting summary
    counts into the public DTO.
  - Verification passed:
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo check -p pantograph-runtime-registry`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation fixed: the first formatting check reported local
    rustfmt changes in the selector helper block; ran `cargo fmt --all` and
    reran formatting successfully.
  - Remaining follow-up: ledger-history ranking inputs, scheduler retry/
    termination integration, and diagnostics-ledger policy projection remain
    later slices. The public `ambiguous_auto_resolution` enum value remains
    append-only DTO history but is no longer produced by equal-ranked valid
    automatic selection.
- 2026-05-12 re-plan trigger: diagnostics-ledger policy projection needs an
  explicit event boundary decision before source edits continue. Current
  `scheduler.run_admitted` events are written before technical-fit preflight
  resolves `WorkflowTechnicalFitDecision`, while runtime lifecycle events are
  load-oriented rather than scheduler-policy-oriented. Do not attach policy
  trace fields to the wrong event as a compatibility bridge. Choose one
  canonical path before implementation: move the admission event after
  technical fit, add a distinct scheduler technical-fit decision event, or
  define another bounded policy-summary projection that can be written after
  preflight without changing queue-admission semantics.
- 2026-05-12 resolution/slice: option 1 was implemented for the successful
  queue-admission path. `scheduler.run_admitted`, reservation-created, and
  run-started diagnostic events are now recorded after runtime technical-fit
  preflight, so admission payloads can carry selected runtime variant, selected
  backend key, and bounded technical-fit policy trace facts from the canonical
  `WorkflowTechnicalFitDecision`. No graph-visible Pumas facts, runtime
  lifecycle selection behavior, or selector fallback paths were introduced.
  Remaining follow-up: queryable policy-summary projection, ledger-history
  ranking inputs, and retry/termination policy remain separate scheduler
  slices.
- 2026-05-12 partial: diagnostics-ledger run-list and run-detail projections
  now populate existing selected backend/runtime-variant columns from
  `scheduler.run_admitted` payloads. This keeps admission-selected facts
  queryable without adding schema columns or inferring them from graph fields.
  Technical-fit policy trace remains payload-level evidence until a later
  compact policy-summary read-model is designed.
- 2026-05-10 slice: runtime-registry no-valid-auto diagnostic.
  - Smallest useful vertical slice: add a `no_valid_candidate` error
    diagnostic to automatic technical-fit decisions when candidate facts exist
    but no candidate is eligible, while preserving the more specific
    `explicit_device_unavailable` diagnostic for explicit device policy
    misses.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: auto-mode failure now returns an
    unselected typed diagnostic instead of relying on reason codes alone or
    selecting a lower-priority/ineligible candidate.
  - Standards/blast-radius gate for runtime-registry selector diagnostics:
    selector policy remains backend-owned in runtime-registry; workflow-service
    and embedded-runtime DTO shapes already carry the diagnostic code; runtime
    lifecycle ownership, persisted schemas, frontend behavior, generated files,
    feature flags, dependencies, lockfiles, workers, and workflow fixtures are
    untouched.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`, and
    `git diff --check`.
  - Remaining follow-up: remaining old raw-device/default-backend execution
    paths still need removal before the broad auto-mode checklist item can be
    closed.
- 2026-05-10 slice: retire deterministic tie-break reason contract.
  - Smallest useful vertical slice: remove the now-dead
    `deterministic_tie_break` technical-fit reason from runtime-registry,
    workflow-service, embedded-runtime projection, and the TypeScript workflow
    mirror after auto ambiguity began returning typed diagnostics.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `src/services/workflow/types.ts`, and this plan directory.
  - No-fallback/no-legacy confirmation: the retired wire value is no longer
    accepted or projected as a current technical-fit reason; ambiguous auto
    resolution must use `ambiguous_auto_resolution` diagnostics instead.
  - Standards/blast-radius gate for shared technical-fit DTO cleanup:
    runtime-registry remains selector-policy owner; workflow-service and
    frontend mirrors stay transport DTOs only; embedded-runtime only projects
    canonical reason codes; runtime lifecycle ownership, persisted schemas,
    generated files, feature flags, dependencies, lockfiles, workers, and
    workflow fixtures are untouched.
  - Verification passed:
    `rg -n "DeterministicTieBreak|deterministic_tie_break" crates/pantograph-runtime-registry crates/pantograph-workflow-service crates/pantograph-embedded-runtime src/services/workflow`,
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`,
    `npm run typecheck`, and `git diff --check`.
- 2026-05-10 slice: runtime-registry technical-fit serde fixture.
  - Smallest useful vertical slice: add a public runtime-registry
    technical-fit JSON fixture plus integration test that deserializes the
    request/decision DTOs, reserializes them byte-for-shape through
    `serde_json::Value`, and verifies selector output against the fixture.
  - Allowed write set:
    `crates/pantograph-runtime-registry/Cargo.toml`,
    `Cargo.lock`,
    `crates/pantograph-runtime-registry/tests/technical_fit_contract.rs`,
    `crates/pantograph-runtime-registry/tests/fixtures/technical_fit_contract.json`,
    `crates/pantograph-runtime-registry/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the fixture uses explicit typed
    runtime/backend/model/device intent and selected runtime variant/device
    facts. It does not include fallback source kinds, deterministic tie-break
    reasons, raw backend device strings, or legacy runtime hints.
  - Standards/blast-radius gate for runtime-registry fixture coverage:
    runtime-registry remains the selector contract owner; the only dependency
    impact is a crate-local `serde_json` dev-dependency already present in the
    workspace lockfile; production code, runtime lifecycle ownership, persisted
    schemas, frontend behavior, generated files, feature flags, workers, and
    workflow fixtures are untouched.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry --test technical_fit_contract`,
    `cargo test -p pantograph-runtime-registry technical_fit`, and
    `git diff --check`.
  - Remaining fixture coverage: Tauri command transport, Python worker, and
    any future persisted technical-fit state still need their own fixtures
    before the broad serde-fixture checklist item can be closed.
- 2026-05-10 slice: explicit override candidate diagnostics.
  - Smallest useful vertical slice: preserve bounded diagnostics from the best
    matching but ineligible explicit override candidate on the unselected
    runtime-registry decision.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: explicit backend/runtime/model
    overrides remain unselected when candidate eligibility fails. The selector
    now returns the candidate's typed incompatibility diagnostics instead of
    selecting the requested backend, synthesizing an override candidate, or
    falling back to another backend/device.
  - Standards/blast-radius gate for runtime-registry override diagnostics:
    runtime-registry remains selector-policy owner; workflow-service and
    embedded-runtime already transport device diagnostics; public DTO shape,
    runtime lifecycle ownership, persisted schemas, frontend behavior,
    generated files, feature flags, dependencies, lockfiles, workers, and
    workflow fixtures are untouched.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`, and
    `git diff --check`.
  - Remaining follow-up: unmatched explicit runtime/variant/platform requests
    with no matching candidate still need synthetic bounded diagnostics for
    cases such as MLX on Linux/Windows once those provider facts exist.
- 2026-05-10 slice: unmatched explicit override diagnostics.
  - Smallest useful vertical slice: synthesize bounded runtime-registry
    diagnostics when explicit override intent has no matching candidate at
    all, with runtime-variant overrides returning
    `missing_runtime_variant`.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: unmatched explicit override intent now
    remains unselected with typed diagnostics rather than being represented
    only by reason codes, synthesizing an executable candidate, or falling back
    to another backend/runtime/device.
  - Standards/blast-radius gate for runtime-registry synthetic diagnostics:
    selector policy remains runtime-registry-owned; workflow-service and
    embedded-runtime already transport the diagnostic codes; public DTO shape,
    runtime lifecycle ownership, persisted schemas, frontend behavior,
    generated files, feature flags, dependencies, lockfiles, workers, and
    workflow fixtures are untouched.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`, and
    `git diff --check`.
  - Remaining follow-up: provider-specific platform facts are still required
    before MLX-on-Linux/Windows and vLLM unsupported-model cases can be
    validated end to end.
- 2026-05-10 slice: frontend diagnostics selected-device class.
  - Smallest useful vertical slice: add `selected_device_class` to frontend
    diagnostics projection types and render it in the Node-tested diagnostics
    fact presenter from the backend-projected field.
  - Allowed write set:
    `src/services/diagnostics/types.ts`,
    `src/services/diagnostics/README.md`,
    `src/components/workbench/diagnosticsPagePresenters.ts`,
    `src/components/workbench/diagnosticsPagePresenters.test.ts`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: frontend presentation uses the typed
    backend projection field directly. It does not infer device class from
    selected device id strings, raw backend config, runtime settings, or payload
    JSON.
  - Standards/blast-radius gate for frontend diagnostics presentation:
    public backend contracts and generated files are unchanged; no new
    frontend test platform, dependencies, lockfiles, polling, subscriptions,
    or DOM behavior are introduced; accessibility impact is limited to an
    existing facts table row label covered by presenter tests.
  - Verification passed:
    `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining follow-up: run-inspection DTOs/presenters still need selected
    runtime variant facts before the full lifecycle/run-inspection checklist
    item can be closed.
- 2026-05-10 slice: scheduler run-list selected-device class presentation.
  - Smallest useful vertical slice: render scheduler run-list placement with
    backend-projected selected device class plus selected device id through the
    existing Node-tested scheduler presenter layer.
  - Allowed write set:
    `src/components/workbench/SchedulerPage.svelte`,
    `src/components/workbench/schedulerPagePresenters.ts`,
    `src/components/workbench/schedulerPagePresenters.test.ts`,
    `src/services/diagnostics/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: scheduler presentation uses typed
    `selected_device_class` and `selected_device_id` projection fields and does
    not infer class from device ids, raw scheduler payload JSON, runtime
    settings, or backend config strings.
  - Standards/blast-radius gate for scheduler frontend presentation:
    backend contracts, generated files, persisted schemas, lockfiles,
    dependencies, polling/subscription lifecycles, and scheduler command
    behavior are unchanged; tests use the existing Node presenter harness.
  - Verification passed:
    `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining follow-up: scheduler filters still target selected runtime,
    selected device id, and selected network node only; adding a selected
    device-class backend query filter should be a separate query-contract
    slice if needed.
- 2026-05-10 slice: scheduler selected-device class query filter.
  - Smallest useful vertical slice: add a frontend scheduler Device Class
    filter that forwards the backend-supported `selected_device_class`
    run-list query field and filters local presentation rows from the typed
    projection field.
  - Allowed write set:
    `src/services/diagnostics/types.ts`,
    `src/services/diagnostics/README.md`,
    `src/stores/schedulerRunListStore.ts`,
    `src/stores/schedulerRunListStore.test.ts`,
    `src/components/workbench/SchedulerPage.svelte`,
    `src/components/workbench/schedulerPagePresenters.ts`,
    `src/components/workbench/schedulerPagePresenters.test.ts`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the filter sends typed
    `selected_device_class` to the backend query and uses typed run-list
    projection fields for local option/filter state. It does not derive device
    class from `selected_device_id`, raw scheduler payload JSON, runtime
    settings, or backend config strings.
  - Standards/blast-radius gate for scheduler query presentation: no backend
    contracts, generated files, persisted schemas, lockfiles, dependencies,
    polling/subscription lifecycles, or scheduler command behavior changed;
    tests use the existing Node store/presenter harness.
  - Verification passed:
    `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts`,
    `node --experimental-strip-types --test src/stores/schedulerRunListStore.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining follow-up: backend run-list facets still expose selected device
    id rather than selected device class; adding a backend facet kind is a
    separate ledger/query contract slice.
- 2026-05-10 slice: diagnostics selected-device class facet contract.
  - Smallest useful vertical slice: add `selected_device_class` as a backend
    run-list facet kind and render the diagnostics comparison row from backend
    facet counts.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-diagnostics-ledger/src/README.md`,
    `src/services/diagnostics/types.ts`,
    `src/services/diagnostics/README.md`,
    `src/components/workbench/diagnosticsPagePresenters.ts`,
    `src/components/workbench/diagnosticsPagePresenters.test.ts`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: facet rows group the typed
    `selected_device_class` projection column and diagnostics presenters
    consume the typed `selected_device_class` facet kind. The slice does not
    parse selected device ids, scheduler payload JSON, runtime settings, or
    backend config strings to infer device class.
  - Standards/blast-radius gate for ledger/front-end facet contract:
    persisted projection schema already owns `selected_device_class`; no
    migration, generated files, lockfiles, dependencies, polling/subscription
    lifecycles, worker paths, or workflow fixtures changed.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_projects_inference_diagnostic_selected_facts`,
    `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining follow-up: diagnostics comparison UI filters can add a
    selected-device-class control in a separate frontend-only slice.
- 2026-05-10 slice: diagnostics selected-device class comparison filter.
  - Smallest useful vertical slice: add a diagnostics comparison filter for
    selected device class using the existing typed run-list projection field
    and Node-tested comparison presenter helpers.
  - Allowed write set:
    `src/components/workbench/DiagnosticsPage.svelte`,
    `src/components/workbench/diagnosticsPagePresenters.ts`,
    `src/components/workbench/diagnosticsPagePresenters.test.ts`,
    `src/services/diagnostics/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: comparison options and filtering read
    `selected_device_class` directly from typed projection rows. The slice does
    not infer class from selected device ids, scheduler payload JSON, runtime
    settings, or backend config strings.
  - Standards/blast-radius gate for frontend comparison filtering: no backend
    contracts, generated files, persisted schemas, lockfiles, dependencies,
    polling/subscription lifecycles, workers, workflow fixtures, or DOM test
    platform changed; accessible filter naming follows the existing select
    pattern and is covered by typechecked Svelte wiring plus Node presenter
    tests.
  - Verification passed:
    `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining follow-up: selected runtime variant facts are still absent from
    diagnostics run-list/detail projection and should be added as a separate
    backend contract slice.
- 2026-05-10 slice: diagnostics selected-backend comparison filter.
  - Smallest useful vertical slice: add a diagnostics comparison filter for
    selected backend using the existing typed `selected_backend_key` run-list
    projection field and Node-tested comparison presenter helpers.
  - Allowed write set:
    `src/components/workbench/DiagnosticsPage.svelte`,
    `src/components/workbench/diagnosticsPagePresenters.ts`,
    `src/components/workbench/diagnosticsPagePresenters.test.ts`,
    `src/services/diagnostics/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: comparison options and filtering read
    `selected_backend_key` directly from typed projection rows. The slice does
    not infer backend choice from runtime ids, selected device ids, scheduler
    payload JSON, runtime settings, or backend config strings.
  - Standards/blast-radius gate for frontend comparison filtering: no backend
    contracts, generated files, persisted schemas, lockfiles, dependencies,
    polling/subscription lifecycles, workers, workflow fixtures, or DOM test
    platform changed; accessible filter naming follows the existing select
    pattern and is covered by typechecked Svelte wiring plus Node presenter
    tests.
  - Verification passed:
    `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Discovered issue retained for follow-up: selected runtime variant needs a
    backend-owned source of truth before lifecycle and ledger projection can
    expose it. Current active llama.cpp runtime descriptors carry selected
    device class/id but not runtime variant id, so deriving a variant from the
    device id would violate the no-fallback/no-inference rule.
- 2026-05-10 slice: diagnostics selected-backend facet contract.
  - Smallest useful vertical slice: add `selected_backend_key` as a backend
    run-list facet kind and render the diagnostics comparison row from backend
    facet counts.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-diagnostics-ledger/src/README.md`,
    `src/services/diagnostics/types.ts`,
    `src/services/diagnostics/README.md`,
    `src/components/workbench/diagnosticsPagePresenters.ts`,
    `src/components/workbench/diagnosticsPagePresenters.test.ts`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: facet rows group the typed
    `selected_backend_key` projection column and diagnostics presenters consume
    the typed `selected_backend` facet kind. The slice does not infer backend
    selection from runtime ids, selected device ids, scheduler payload JSON,
    runtime settings, or backend config strings.
  - Standards/blast-radius gate for ledger/front-end facet contract: persisted
    projection schema already owns `selected_backend_key`; no migration,
    generated files, lockfiles, dependencies, polling/subscription lifecycles,
    worker paths, or workflow fixtures changed.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-diagnostics-ledger diagnostic_event_ledger_projects_inference_diagnostic_selected_facts`,
    `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining follow-up: run-list query contracts still do not expose a
    selected-backend filter; adding it should be a separate backend/frontend
    query-contract slice if scheduler or diagnostics pages need server-side
    backend filtering.
- 2026-05-10 slice: scheduler selected-backend query filter.
  - Smallest useful vertical slice: add `selected_backend_key` to the backend
    run-list query contract and wire the scheduler Backend filter through the
    existing typed store, presenter, and page path.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
    `crates/pantograph-workflow-service/tests/contract.rs`,
    `src/services/diagnostics/types.ts`,
    `src/services/diagnostics/README.md`,
    `src/stores/schedulerRunListStore.ts`,
    `src/stores/schedulerRunListStore.test.ts`,
    `src/components/workbench/SchedulerPage.svelte`,
    `src/components/workbench/schedulerPagePresenters.ts`,
    `src/components/workbench/schedulerPagePresenters.test.ts`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the backend query and frontend filter
    forward typed `selected_backend_key`, and local scheduler filters read typed
    `selected_backend_key` projection rows. The slice does not infer backend
    choice from runtime ids, selected device ids, scheduler payload JSON,
    runtime settings, or backend config strings.
  - Standards/blast-radius gate for ledger/workflow-service/frontend query
    contract: persisted projection schema already owns `selected_backend_key`;
    no migration, generated files, lockfiles, dependencies,
    polling/subscription lifecycle, worker path, or workflow fixture changed;
    tests use focused Rust contract/projection coverage plus the existing Node
    store/presenter harness.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-diagnostics-ledger run_list_projection_drains_lifecycle_events_incrementally`,
    `cargo test -p pantograph-workflow-service workflow_run_list_query_contract_snapshot`,
    `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts src/stores/schedulerRunListStore.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Verification deviations fixed during the slice: the first focused Rust test
    passes exposed two exhaustive DTO initializers that needed the new optional
    `selected_backend_key` field, and the workflow-service JSON contract
    expected request was updated to include the typed filter.
  - Remaining follow-up: selected runtime variant facts remain a separate
    backend-owned source-of-truth slice; diagnostics comparison can already use
    selected-backend facets, and scheduler/run-list queries now support
    server-side selected-backend filtering.
- 2026-05-10 slice: scheduler lifecycle selected runtime variant payload.
  - Smallest useful vertical slice: carry
    `WorkflowTechnicalFitDecision.selected_runtime_variant_id` into scheduler
    model lifecycle diagnostic payloads and post-preflight runtime-slot
    reservation release payloads.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/session_runtime.rs`,
    `crates/pantograph-workflow-service/src/workflow/session_runtime_load_lifecycle.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: selected runtime variant is copied only
    from backend-owned technical-fit decisions after runtime preflight. The
    slice does not derive variants from runtime ids, selected backend keys,
    device ids/classes, runtime settings, or scheduler payload JSON. Early
    admission/reservation-created events remain variant-free when technical fit
    has not supplied a selected variant yet.
  - Standards/blast-radius gate for ledger/workflow-service diagnostic
    payloads: this is additive serde payload metadata with `None` omitted from
    JSON, no projection schema, generated file, lockfile, dependency,
    frontend, worker, polling, or workflow fixture changes; lifecycle ownership
    stays in workflow-service runtime preflight/load composition.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
    `cargo test -p pantograph-diagnostics-ledger model_lifecycle_projects_canonical_error_link_without_counting_new_error`,
    and `git diff --check`.
  - Discovered issue retained for follow-up: broader verification with
    `cargo test -p pantograph-workflow-service workflow_execution_session_run_records_snapshot_before_execution`
    still fails at the existing Library usage projection assertion
    (`library_usage.assets.len()` is `0` instead of `1`). The failure remained
    after this slice's selected-variant assertions were moved out of that test,
    so it is recorded as a separate Library usage projection/test fragility
    investigation rather than folded into this runtime-variant payload slice.
  - Remaining follow-up: diagnostics run-list/run-detail projections still
    need a durable selected-runtime-variant column and frontend typed DTOs
    before run inspection can filter or render selected runtime variant without
    parsing event payload JSON.
- 2026-05-10 slice: diagnostics projection selected runtime variant.
  - Smallest useful vertical slice: add durable
    `selected_runtime_variant_id` fields to diagnostics-ledger run-list and
    run-detail projections, expose them through workflow-service/frontend DTOs,
    and populate them from typed scheduler model lifecycle payloads.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/schema.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-workflow-service/tests/contract.rs`,
    `src/services/diagnostics/types.ts`,
    `src/services/diagnostics/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: projection updates copy
    `selected_runtime_variant_id` only from typed scheduler lifecycle payloads.
    The slice does not infer variants from runtime ids, selected backend keys,
    device ids/classes, runtime settings, scheduler payload JSON, or backend
    config strings.
  - Standards/blast-radius gate for durable diagnostics projection: projection
    versions are bumped and schema repair adds nullable text columns; no
    generated files, lockfiles, dependencies, workers, polling, or workflow
    fixtures changed. Frontend impact is type-only DTO exposure with no UI
    rendering or accessibility surface yet.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-diagnostics-ledger model_lifecycle_projects_canonical_error_link_without_counting_new_error`,
    `cargo test -p pantograph-diagnostics-ledger current_schema_repairs_all_drifted_projection_tables`,
    `cargo test -p pantograph-diagnostics-ledger current_schema_repairs_missing_run_error_projection_columns`,
    `cargo test -p pantograph-workflow-service workflow_run_list_query_contract_snapshot`,
    `cargo test -p pantograph-workflow-service workflow_run_detail_query_contract_snapshot`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining follow-up: scheduler/diagnostics frontend presenters can now add
    selected-runtime-variant rendering or filters from typed DTO fields in a
    separate frontend slice; no component should parse lifecycle payload JSON
    for the variant.
- 2026-05-10 slice: scheduler selected runtime variant presentation.
  - Smallest useful vertical slice: render selected runtime variant in the
    scheduler run-list runtime placement column and include it in scheduler
    run-list search from typed frontend DTO fields.
  - Allowed write set:
    `src/components/workbench/SchedulerPage.svelte`,
    `src/components/workbench/schedulerPagePresenters.ts`,
    `src/components/workbench/schedulerPagePresenters.test.ts`,
    `src/services/diagnostics/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: the presenter reads
    `selected_runtime_variant_id` directly. It does not infer variants from
    runtime ids, selected backend keys, device ids/classes, runtime settings,
    backend config strings, or scheduler payload JSON.
  - Standards/blast-radius gate for frontend presentation: existing Node
    presenter tests are retained as the test platform; no backend contracts,
    schema, generated files, lockfiles, dependencies, polling paths, saved
    workflow fixtures, or worker code changed. The Svelte table still renders
    backend projection facts declaratively with existing title text for
    truncated values.
  - Verification passed:
    `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining follow-up: runtime-variant-specific facets or query filters
    remain a separate backend/frontend projection slice; this slice does not
    overload the existing selected-runtime filter with variant values.
- 2026-05-10 slice: diagnostics selected runtime variant comparison facet.
  - Smallest useful vertical slice: expose `selected_runtime_variant_id` as a
    run-list facet, include it in workflow-service/frontend contract fixtures,
    and add diagnostics comparison fact/filter rendering from typed projection
    fields.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-workflow-service/tests/contract.rs`,
    `crates/pantograph-workflow-service/tests/fixtures/run_projection_contract.json`,
    `src/services/diagnostics/types.ts`,
    `src/services/diagnostics/README.md`,
    `src/components/workbench/DiagnosticsPage.svelte`,
    `src/components/workbench/diagnosticsPagePresenters.ts`,
    `src/components/workbench/diagnosticsPagePresenters.test.ts`,
    `src/components/workbench/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: the facet and comparison controls read
    `selected_runtime_variant_id` directly from materialized projection rows.
    They do not split runtime ids, infer from selected backend keys, derive
    from device ids/classes, parse backend config strings, or inspect
    scheduler payload JSON.
  - Standards/blast-radius gate for diagnostics comparison: the backend change
    is an additive serde facet enum value over an existing nullable projection
    column; no schema version, generated file, lockfile, dependency, polling,
    worker, runtime execution, or saved workflow files changed. Frontend
    coverage remains in existing Node presenter tests with accessible Svelte
    select labels.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-diagnostics-ledger model_lifecycle_projects_canonical_error_link_without_counting_new_error`,
    `cargo test -p pantograph-workflow-service workflow_run_list_query_contract_snapshot`,
    `node --experimental-strip-types --test src/components/workbench/diagnosticsPagePresenters.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining follow-up: scheduler run-list query DTOs still do not accept
    `selected_runtime_variant_id` as a backend-side filter; add that only as a
    separate query/filter slice.
- 2026-05-10 slice: scheduler selected runtime variant query filter.
  - Smallest useful vertical slice: add `selected_runtime_variant_id` to
    run-list query DTOs, SQLite run-list/facet filters, workflow-service
    query mapping, frontend diagnostics request types, and the Scheduler page
    variant filter.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite/event_sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
    `crates/pantograph-workflow-service/tests/contract.rs`,
    `src/services/diagnostics/types.ts`,
    `src/services/diagnostics/README.md`,
    `src/stores/schedulerRunListStore.ts`,
    `src/stores/schedulerRunListStore.test.ts`,
    `src/components/workbench/SchedulerPage.svelte`,
    `src/components/workbench/schedulerPagePresenters.ts`,
    `src/components/workbench/schedulerPagePresenters.test.ts`,
    `src/components/workbench/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: the filter matches the typed
    `selected_runtime_variant_id` projection column directly. It does not
    infer variants from selected runtime ids, selected backend keys, device
    ids/classes, backend config strings, or scheduler payload JSON.
  - Standards/blast-radius gate for scheduler query filtering: this is an
    additive optional query field over an existing nullable projection column;
    no schema version, generated file, lockfile, dependency, worker, runtime
    execution, polling loop, or saved workflow fixture changed. Frontend uses
    existing Node store/presenter tests and declarative Svelte select wiring.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-diagnostics-ledger model_lifecycle_projects_canonical_error_link_without_counting_new_error`,
    `cargo test -p pantograph-workflow-service workflow_run_list_query_contract_snapshot`,
    `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts src/stores/schedulerRunListStore.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining follow-up: diagnostics comparison can use the same backend query
    field only if that page starts requesting backend-filtered comparison
    pages; current diagnostics comparison filtering remains projection-row
    based.
- 2026-05-10 slice: vLLM and MLX roadmap capability facts.
  - Smallest useful vertical slice: expose unavailable vLLM CPU/CUDA and MLX
    Metal runtime capability facts through embedded-runtime workflow
    capabilities.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/runtime_capabilities.rs`,
    `crates/pantograph-embedded-runtime/src/embedded_workflow_host.rs`,
    `crates/pantograph-embedded-runtime/src/README.md`,
    `crates/pantograph-embedded-runtime/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: vLLM and MLX are reported as
    unavailable roadmap facts with typed diagnostics only; no executable
    backend, startup/load path, synthetic device choice, CPU/auto fallback,
    raw-device parsing, generated files, lockfiles, workers, or saved workflow
    fixtures were introduced.
  - Standards/blast-radius gate: embedded-runtime remains workflow capability
    projection owner; runtime lifecycle ownership and scheduler ranking are
    unchanged; no persisted schema/dependency/feature changes; tests are
    crate-local unit tests with no external services.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-embedded-runtime roadmap_runtime_capabilities_report_vllm_and_mlx_placeholders`,
    `cargo test -p pantograph-embedded-runtime runtime_capabilities`,
    `cargo test -p pantograph-embedded-runtime technical_fit`, and
    `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` reported
    rustfmt wrapping; `cargo fmt --all` was run and the check passed.
  - Remaining follow-up: real vLLM/MLX provider, probe, and admission slices
    still need executable provider ownership before either can be selected; no
    execution is enabled here.
- 2026-05-10 slice: roadmap backend override rejection tests.
  - Smallest useful vertical slice: prove explicit vLLM and MLX backend
    overrides are rejected from roadmap-only capability facts without selecting
    an unavailable candidate or synthesizing a fallback candidate.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/technical_fit.rs` and this plan
    directory.
  - No-fallback/no-legacy confirmation: explicit roadmap backend preferences
    produce `CandidateUnavailable` diagnostics and an unselected explicit
    override decision; no legacy backend, CPU/auto fallback, synthetic
    candidate, runtime startup path, frontend path, worker path, fixture, or
    generated DTO changed.
  - Standards/blast-radius gate: this is a crate-local admission regression
    test only; embedded-runtime remains the projection owner and
    runtime-registry remains the selector owner.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-embedded-runtime roadmap_backend_overrides_reject_without_fallback_selection`,
    `cargo test -p pantograph-embedded-runtime technical_fit`, and
    `git diff --check`.
  - Remaining follow-up: the broad impossible-preference checklist item stays
    open until every listed case is covered by executable or admission tests
    against the canonical decision path.
- 2026-05-10 slice: llama.cpp device inventory fact projection.
  - Smallest useful vertical slice: keep existing `DeviceInfo` parsing for
    llama.cpp `--list-devices` and add canonical `LlamaCppDeviceInventoryFact`
    projection that maps CPU/CUDA selectors into validated device facts while
    reporting unsupported backend-local selectors as typed diagnostics.
  - Allowed write set: `crates/inference/src/device.rs`,
    `crates/inference/src/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: unsupported backend-local selectors
    such as Vulkan now produce `UnsupportedDeviceClass` diagnostics in the
    canonical inventory projection; no parser fallback, auto selection,
    ordinal-zero coercion, command resolution, runtime startup, frontend path,
    generated DTO, fixture, or lockfile changed.
  - Standards/blast-radius gate: the slice stays inside the inference
    backend-local llama.cpp adapter boundary, uses validated `BackendId` and
    `InferenceDeviceId` contracts, and keeps scheduler ranking/admission
    outside the inference crate.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference device::tests::parse_llamacpp_inventory_facts`,
    `cargo test -p inference device::tests`, and `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` reported
    rustfmt wrapping in `device.rs`; `cargo fmt --all` was run and the check
    passed.
  - Remaining follow-up: future slices still need to feed inventory facts into
    managed-runtime/runtime-capability refresh flows before scheduler admission
    can consume live host device inventory.
- 2026-05-10 slice: PyTorch device probe contract.
  - Smallest useful vertical slice: add `PyTorchDeviceProbeSnapshot` and a
    pure projection from host-observed CPU/CUDA/macOS MPS probe facts into
    canonical PyTorch runtime variant readiness facts.
  - Allowed write set:
    `crates/inference/src/backend/pytorch.rs`,
    `crates/inference/src/backend/pytorch_tests.rs`,
    `crates/inference/src/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: unavailable CUDA/MPS probe facts emit
    typed `CandidateUnavailable` runtime-variant diagnostics; no Python probe
    execution, worker path, runtime startup, scheduler selection, auto/CPU
    fallback, generated DTO, fixture, lockfile, or frontend path changed.
  - Standards/blast-radius gate: the probe contract is pure data projection in
    the PyTorch backend boundary; the caller remains responsible for probe
    lifecycle and scheduler admission remains outside the inference crate.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference --features backend-pytorch pytorch_device_probe`,
    `cargo test -p inference --features backend-pytorch test_capabilities`, and
    `git diff --check`.
  - Verification deviation: initial plain `cargo test -p inference
    pytorch_device_probe` and `cargo test -p inference test_capabilities`
    commands did not compile the PyTorch feature-gated tests; they were rerun
    with `--features backend-pytorch`.
  - Remaining follow-up: a later lifecycle-owned probe runner still needs to
    collect live PyTorch CUDA/MPS facts and feed this contract into runtime
    capabilities before scheduler admission consumes it.
- 2026-05-10 slice: frontend backend-confirmed device submit guard.
  - Smallest useful vertical slice: make the Device Configuration panel submit
    device config only when the selected device remains present in
    backend-confirmed device options, and update visible copy away from
    llama-server-owned auto/GPU selection.
  - Allowed write set: `src/components/DeviceConfig.svelte`,
    `src/components/deviceConfigPresenters.ts`,
    `src/components/deviceConfigPresenters.test.ts`, and this plan directory.
  - No-fallback/no-legacy confirmation: stale local `selectedDevice` values now
    fail closed with an unavailable-state message before `setDeviceConfig`;
    the frontend does not synthesize auto/CPU choices, does not submit stale
    executable devices, and does not infer runtime readiness from local form
    state.
  - Standards/blast-radius gate: this stays inside existing frontend presenter
    and component state, uses the existing Node test harness, adds no polling,
    subscriptions, dependencies, generated files, lockfiles, backend DTOs, or
    workflow fixtures.
  - Verification passed:
    `node --experimental-strip-types --test src/components/deviceConfigPresenters.test.ts`,
    `npm run typecheck`,
    `rg -n "llama-server owns|let llama-server choose|Select your GPU|frontend-owned auto|CPU Only|Provide fallback options" src/components/DeviceConfig.svelte src/components/deviceConfigPresenters.ts src/components/deviceConfigPresenters.test.ts`,
    and `git diff --check`.
  - Remaining follow-up: broader workbench model/runtime/device selectors
    still need to render backend-owned capability facts and submit canonical
    scheduler device policy intent.
- 2026-05-10 slice: llama.cpp inventory serde fixture.
  - Smallest useful vertical slice: make `LlamaCppDeviceInventoryFact` a
    public serde-tested inference DTO and add a fixture that round-trips a
    canonical CUDA projection.
  - Allowed write set: `crates/inference/src/device.rs`,
    `crates/inference/src/lib.rs`,
    `crates/inference/tests/device_contracts.rs`,
    `crates/inference/tests/fixtures/device_contracts/llamacpp_device_inventory_fact.json`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: the fixture preserves canonical
    `cuda:0` projection and typed diagnostics shape; it does not accept raw
    llama.cpp device selectors as canonical ids, does not add compatibility
    shims, and does not change runtime startup, scheduler selection, frontend
    paths, generated DTOs, lockfiles, or workflow fixtures.
  - Standards/blast-radius gate: this stays in the inference crate public
    contract and mirrored integration fixture test; no durable state, network,
    Python, or subprocess execution is involved.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference --test device_contracts llamacpp_device_inventory_fact_fixture_preserves_canonical_projection`,
    `cargo test -p inference --test device_contracts`,
    `cargo test -p inference device::tests::parse_llamacpp_inventory_facts`,
    and `git diff --check`.
  - Verification deviation: the first fixture run failed because
    `LlamaCppDeviceInventoryFact.diagnostics` did not default to an empty list;
    the DTO was fixed with a serde default and the fixture suite passed.
  - Remaining follow-up: the broad fixture checklist item stays open until all
    device/runtime DTOs crossing Rust, frontend, diagnostics-ledger, worker,
    and persisted-state boundaries have explicit fixtures.
- 2026-05-10 slice: llama.cpp `gpu_layers` device-policy guardrail.
  - Smallest useful vertical slice: add a regression test proving
    `gpu_layers` remains in `LlamaCppRuntimeSettings`/`DeviceConfig` and is
    absent from the canonical cross-backend `InferenceDevicePolicy` payload.
  - Allowed write set: `crates/inference/src/backend/mod.rs` and this plan
    directory.
  - No-fallback/no-legacy confirmation: this does not add hybrid/offload
    policy, does not synthesize backend flags from scheduler policy, and does
    not change command resolution, runtime startup, generated DTOs, lockfiles,
    frontend paths, or workflow fixtures.
  - Standards/blast-radius gate: crate-local unit test only; backend-specific
    llama.cpp runtime settings remain inside the inference backend boundary and
    scheduler-facing device policy stays generic.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p inference gpu_layers_remain_llamacpp_runtime_setting_not_device_policy`,
    and `git diff --check`.
  - Remaining follow-up: hybrid placement, CPU/GPU split, and offload
    capability reporting still need a separate backend-capability design slice.
- 2026-05-10 slice: device-resolution request and candidate fixtures.
  - Smallest useful vertical slice: add serde fixtures for
    `DeviceResolutionRequest` and `BackendExecutionCandidate` so scheduler
    admission input and candidate evidence have stable wire-contract coverage.
  - Allowed write set: `crates/inference/tests/device_contracts.rs`,
    `crates/inference/tests/fixtures/device_contracts/backend_execution_candidate.json`,
    `crates/inference/tests/fixtures/device_contracts/device_resolution_request.json`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: the fixtures use canonical
    `InferenceDevicePolicy`, `RuntimeVariantId`, `BackendId`, and
    `InferenceDeviceId` shapes; they do not normalize raw backend device
    strings or introduce compatibility shims.
  - Standards/blast-radius gate: test-only fixture expansion under the
    existing Rust integration-test harness; no runtime behavior, generated DTOs,
    lockfiles, workflow fixtures, or frontend contracts changed.
  - Verification passed:
    `cargo test -p inference --test device_contracts`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` reported
    rustfmt wrapping in the new test; `cargo fmt --all` was run and the check
    passed.
  - Remaining follow-up: the broad serde fixture checklist item stays open
    until every remaining device/runtime DTO crossing Rust, frontend,
    diagnostics-ledger, worker, and persisted-state boundaries has explicit
    fixture coverage.
- 2026-05-10 slice: device-resolution decision fixture.
  - Smallest useful vertical slice: add a serde fixture for
    `DeviceResolutionDecision`, the resolved device choice consumed by runtime
    load contracts.
  - Allowed write set: `crates/inference/tests/device_contracts.rs`,
    `crates/inference/tests/fixtures/device_contracts/device_resolution_decision.json`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: the fixture encodes a canonical
    explicit CUDA policy and selected canonical `cuda:0` device id; it does
    not preserve or translate backend-local raw selectors.
  - Standards/blast-radius gate: test-only fixture expansion under the
    existing Rust integration-test harness; no runtime behavior, generated DTOs,
    lockfiles, workflow fixtures, or frontend contracts changed.
  - Verification passed:
    `cargo test -p inference --test device_contracts`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` reported
    rustfmt import wrapping; `cargo fmt --all` was run and the check passed.
  - Remaining follow-up: DTO fixture coverage still needs non-inference
    device/runtime boundaries, especially frontend, diagnostics-ledger, worker,
    and persisted-state contracts.
- 2026-05-10 slice: typed lifecycle selected-device ids.
  - Smallest useful vertical slice: replace the inference lifecycle event
    `selected_device_id` raw string field and gateway lifecycle plumbing with
    canonical `InferenceDeviceId`.
  - Allowed write set: `crates/inference/src/types.rs`,
    `crates/inference/src/gateway.rs`, `crates/inference/src/gateway_tests.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: lifecycle event deserialization now
    rejects legacy backend-local selectors such as `CUDA0` instead of accepting
    them as diagnostic facts.
  - Standards/blast-radius gate: typed DTO boundary only; no scheduler
    selection policy, backend startup config, generated DTOs, lockfiles,
    frontend code, or workflow fixtures changed.
  - Verification passed:
    `cargo test -p inference inference_request_lifecycle_event`,
    `cargo test -p inference test_lifecycle_events_carry_active_runtime_selected_device`,
    `cargo fmt --all -- --check`,
    `rg -n "selected_device_id: Option<String>|selected_device_id.as_deref|selected_device_id: Some\\(\\\"cuda:0" crates/inference/src/types.rs crates/inference/src/gateway.rs crates/inference/src/gateway_tests.rs`,
    and `git diff --check`.
  - Remaining follow-up: other raw device string crossings remain, including
    backend startup config and frontend/settings boundaries.
- 2026-05-10 slice: explicit device candidate mismatch guard.
  - Smallest useful vertical slice: make
    `BackendExecutionDecision::try_from_selected_candidate` reject a selected
    candidate whose device class or concrete device id does not match an
    explicit `InferenceDevicePolicy`.
  - Allowed write set: `crates/inference/src/device_contracts/mod.rs`,
    `crates/inference/src/device_contracts/planning.rs`,
    `crates/inference/src/device_contracts/tests.rs`, and this plan directory.
  - No-fallback/no-legacy confirmation: explicit CUDA policy can no longer
    construct a selected CPU decision through the canonical decision DTO
    constructor; mismatches return typed `DeviceContractError` variants.
  - Standards/blast-radius gate: pure synchronous contract validation and unit
    tests only; no backend startup, frontend behavior, generated DTOs,
    lockfiles, workflow fixtures, or scheduler queue policy changed.
  - Verification passed:
    `cargo test -p inference device_contracts::tests::explicit_device_policy_rejects_mismatched_selected_candidate`,
    `cargo test -p inference device_contracts::tests::backend_execution_decision_requires_one_selected_candidate`,
    `cargo test -p inference device_contracts::tests`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` reported
    rustfmt wrapping in the new error attribute; `cargo fmt --all` was run and
    the check passed.
  - Remaining follow-up: full scheduler/runtime-load integration still needs to
    convert unavailable explicit requests into bounded device diagnostics before
    backend load.
- 2026-05-10 slice: typed runtime fact resolved-device ids.
  - Smallest useful vertical slice: replace `ServerModeInfo` and
    `RuntimeFactSnapshot` resolved-device raw strings with canonical
    `InferenceDeviceId`, preserving the same JSON string shape for frontend and
    host consumers.
  - Allowed write set: `crates/inference/src/types.rs`,
    `crates/inference/src/gateway.rs`, `crates/inference/src/gateway_tests.rs`,
    `crates/pantograph-embedded-runtime/src/host_runtime.rs`,
    `crates/pantograph-embedded-runtime/src/lib_tests/runtime_lifecycle_capability_tests.rs`,
    `crates/pantograph-embedded-runtime/src/node_execution_ledger.rs`,
    `crates/pantograph-embedded-runtime/src/node_execution_ledger_tests.rs`, and
    this plan directory.
  - No-fallback/no-legacy confirmation: status/runtime fact DTOs now carry
    validated canonical device ids; path-shaped selected-device ids cannot be
    constructed and then sanitized by diagnostics-ledger projection.
  - Standards/blast-radius gate: DTO and projection typing only; no backend
    startup config, frontend code, generated DTOs, lockfiles, workflow fixtures,
    scheduler queue policy, or runtime load behavior changed.
  - Verification passed:
    `cargo test -p inference runtime_fact_snapshot`,
    `cargo test -p inference test_mode_info_runtime_facts_report_active_runtime_selected_device`,
    `cargo test -p pantograph-embedded-runtime host_runtime_mode_snapshot_copies_runtime_facts_from_mode_info`,
    `cargo test -p pantograph-embedded-runtime hosted_runtime_constructor_syncs_registry_and_derives_capabilities_from_mode_info`,
    `cargo test -p pantograph-embedded-runtime inference_lifecycle_event_adapter_builds_node_status_event_with_backend_context`,
    `cargo test -p pantograph-embedded-runtime inference_diagnostic_event_adapter_drops_path_shaped_runtime_metadata`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first embedded-runtime compile exposed ledger
    consumers and tests still constructing raw selected-device strings; those
    were converted to canonical ids or removed where the invalid path-shaped id
    is now rejected before ledger projection.
  - Remaining follow-up: backend startup config and worker request device fields
    still carry backend-local raw strings and need separate adapter-owned
    boundary slices.
- 2026-05-10 slice: module README device-boundary refresh.
  - Smallest useful vertical slice: update inference and embedded-runtime
    README invariants to match the canonical selected/resolved device id
    boundaries implemented in Milestone 5.
  - Allowed write set: `crates/inference/src/README.md`,
    `crates/pantograph-embedded-runtime/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: documentation now states selected and
    resolved device facts come from canonical device DTOs and active runtime
    descriptors, not raw backend config strings or sanitized backend-local
    metadata.
  - Standards/blast-radius gate: documentation-only slice; no source,
    generated DTOs, lockfiles, workflow fixtures, or frontend behavior changed.
  - Verification passed:
    `rg -n "selected_device_id|InferenceDeviceId|raw backend config|backend-local selected-device" crates/inference/src/README.md crates/pantograph-embedded-runtime/src/README.md`,
    `rg -n "Update relevant module READMEs" docs/plans/current-image-generation-graphs/milestones/05-device-and-runtime-variant-selection.md`,
    and `git diff --check`.
  - Remaining follow-up: keep READMEs current as later slices move backend
    startup config and worker request device fields behind adapter-owned
    contracts.
- 2026-05-10 slice: runtime-load phase serde fixture.
  - Smallest useful vertical slice: add a JSON fixture and integration test for
    `RuntimeLoadPhaseRecord` so runtime readiness facts, resolved device
    decisions, and command facts have stable wire-contract coverage.
  - Allowed write set:
    `crates/inference/tests/runtime_load_contracts.rs`,
    `crates/inference/tests/fixtures/runtime_load/runtime_load_phase_record.json`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: the fixture carries a canonical
    `DeviceResolutionDecision` with explicit CPU policy and selected `cpu`
    device id; it does not infer readiness from command-line arguments or raw
    backend config strings.
  - Standards/blast-radius gate: test-only fixture expansion under the existing
    Rust integration-test harness; no runtime behavior, generated DTOs,
    lockfiles, workflow fixtures, frontend contracts, or worker contracts
    changed.
  - Verification passed:
    `cargo test -p inference --test runtime_load_contracts`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` reported
    rustfmt assertion wrapping; `cargo fmt --all` was run and the check passed.
  - Remaining follow-up: runtime-load behavior still needs end-to-end
    integration that consumes scheduler-selected device decisions before
    backend load.
- 2026-05-11 slice: typed PyTorch worker load device contract.
  - Smallest useful vertical slice: replace the PyTorch Transformers worker
    load request `device` raw string with `Option<InferenceDeviceId>` and keep
    `auto` as omitted worker-device intent at the adapter boundary.
  - Allowed write set: `crates/inference/src/backend/pytorch_worker_contract.rs`,
    `crates/inference/src/backend/pytorch.rs`,
    `crates/inference/src/backend/pytorch_tests.rs`,
    `crates/inference/tests/fixtures/pytorch_worker_contract/load_transformers_model_request.json`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: worker load envelopes now reject legacy
    backend-local selectors such as `CUDA0` during contract decoding or direct
    envelope construction instead of forwarding them to Python; omitted device
    remains the worker's backend-local auto request.
  - Standards/blast-radius gate: load-contract typing and fixture repair only;
    no Python worker behavior, audio transcription device handling, generated
    DTOs, lockfiles, workflow fixtures, frontend code, or scheduler policy
    changed.
  - Verification passed:
    `cargo test -p inference --features backend-pytorch test_pytorch_worker_load_envelope_decodes_fixture`,
    `cargo test -p inference --features backend-pytorch test_pytorch_worker_load_envelope_rejects_legacy_device_id`,
    `cargo test -p inference --features backend-pytorch test_pytorch_direct_load_envelope_rejects_legacy_device_id`,
    `cargo test -p inference --features backend-pytorch test_pytorch_load_envelope_maps_pumas_package_facts`,
    `cargo test -p inference --features backend-pytorch test_pytorch_direct_load_envelope_uses_transformers_contract`,
    `cargo test -p inference --features backend-pytorch test_pytorch_transformers_load_args_default_device_auto`,
    `cargo fmt --all -- --check`,
    `rg -n "payload\\.device\\.as_deref|device: Option<String>" crates/inference/src/backend/pytorch_worker_contract.rs crates/inference/src/backend/pytorch.rs crates/inference/src/backend/pytorch_tests.rs`
    reported no matches, and `git diff --check`.
  - Verification deviations fixed during the slice: an initial cargo command
    attempted to pass two test filters and failed; the filters were rerun as
    separate commands. The existing worker load fixture also failed
    `model_source.validate_for_backend_load()` because its nested
    `source_contract_version` was stale at `1`; the fixture was updated to the
    current contract version `2` and the fixture decode test passed.
  - Remaining follow-up: PyTorch audio transcription still carries a
    backend-local `device: String` because it accepts `"auto"` directly; that
    needs a separate worker adapter boundary slice.
- 2026-05-11 slice: reserve `auto` out of concrete device ids.
  - Smallest useful vertical slice: make `InferenceDeviceId::parse("auto")`
    return a typed reserved-identifier error and add a PyTorch worker
    load-envelope guard proving explicit `"auto"` in `payload.device` is
    rejected.
  - Allowed write set: `crates/inference/src/device_contracts/mod.rs`,
    `crates/inference/src/device_contracts/ids.rs`,
    `crates/inference/src/device_contracts/tests.rs`,
    `crates/inference/src/backend/pytorch_tests.rs`, and this plan directory.
  - No-fallback/no-legacy confirmation: automatic selection remains a scheduler
    policy (`InferenceDevicePolicy::Auto`) or an omitted backend-worker device
    field; it cannot cross boundaries as a concrete selected device id.
  - Standards/blast-radius gate: pure contract validation plus focused tests
    only; no generated DTOs, lockfiles, workflow fixtures, frontend code,
    runtime startup policy, Python worker behavior, or scheduler ranking
    changed.
  - Verification passed:
    `cargo test -p inference device_contracts`,
    `cargo test -p inference --features backend-pytorch test_pytorch_worker_load_envelope_rejects_auto_device_field`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: audio transcription worker requests still need their
    raw `"auto"` device field replaced with omitted/canonical device intent at
    the worker boundary.
- 2026-05-11 slice: typed PyTorch audio transcription worker device contract.
  - Smallest useful vertical slice: replace the audio transcription worker
    request's raw `device: "auto"` field with `Option<InferenceDeviceId>` in
    Rust, omit the device from the fixture and adapter-built envelope, and make
    the Python worker contract reject explicit `"auto"` or legacy device ids
    when a device field is present.
  - Allowed write set:
    `crates/inference/src/backend/pytorch_worker_contract.rs`,
    `crates/inference/src/backend/pytorch.rs`,
    `crates/inference/src/backend/pytorch_tests.rs`,
    `crates/inference/tests/fixtures/pytorch_worker_contract/audio_transcription_request.json`,
    `crates/inference/torch/worker_contract.py`, and this plan directory.
  - No-fallback/no-legacy confirmation: canonical audio worker envelopes now
    represent automatic placement by omitting `payload.device`; the Python
    worker maps omission to backend-local `auto` only inside the worker adapter
    and rejects explicit `"auto"`/`CUDA0` instead of normalizing them.
  - Standards/blast-radius gate: PyTorch worker audio contract, fixture, and
    worker-contract validation only; no generated DTOs, lockfiles, workflow
    fixtures, frontend code, runtime startup policy, scheduler ranking, or ASR
    runtime loading behavior changed.
  - Verification passed:
    `cargo test -p inference --features backend-pytorch audio_transcription`,
    `cargo test -p inference --features backend-pytorch test_python_worker_contract_projects_task_profile_loader`,
    `cargo test -p inference --features backend-pytorch test_python_worker_contract_tolerates_additive_load_fields`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation fixed during the slice: an attempted cargo command
    passed two test filters and was rerun as separate commands.
  - Remaining follow-up: Python mirrors the Rust device-id identifier shape in
    worker-contract validation because no generated cross-language worker DTO
    validator exists; the current slice covers the mirrored rule with focused
    Rust/Python contract tests.
- 2026-05-11 slice: typed PyTorch worker response selected-device facts.
  - Smallest useful vertical slice: change `LoadedModelInfo.device` and
    `PyTorchLiveKvInfo.device` from raw strings to `InferenceDeviceId` so
    PyTorch worker load/get-loaded/live-KV responses cannot report selected
    devices as `"auto"` or legacy backend-local ids.
  - Allowed write set: `crates/inference/src/backend/pytorch.rs`,
    `crates/inference/src/backend/pytorch_tests.rs`, and this plan directory.
  - No-fallback/no-legacy confirmation: worker response decode now fails for
    selected-device `"auto"` and `CUDA0`; selected runtime facts must be
    concrete canonical device ids before reuse checks, KV fingerprints, or
    runtime facts can consume them.
  - Standards/blast-radius gate: PyTorch worker response DTO typing and tests
    only; no Python worker code, generated DTOs, lockfiles, workflow fixtures,
    frontend code, runtime startup policy, scheduler ranking, or worker
    execution behavior changed.
  - Verification passed:
    `cargo test -p inference --features backend-pytorch worker_load_response`,
    `cargo test -p inference --features backend-pytorch save_kv_cache_response`,
    `cargo test -p inference --features backend-pytorch get_loaded_info_response`,
    `cargo test -p inference --features backend-pytorch restore_kv_cache_response`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviations fixed during the slice: the first negative
    load/save tests used shorthand canonical-code expectations; they were
    updated to the existing worker error codes and rerun successfully.
- 2026-05-11 re-plan trigger: remaining raw device fields are shared
  gateway/startup config and backend-local llama.cpp runtime settings.
  - Code search after the PyTorch worker response slice found remaining
    `device: Option<String>` / `device: String` fields in
    `crates/inference/src/gateway.rs`, `crates/inference/src/config.rs`,
    `crates/inference/src/backend/mod.rs`, and a test-only PyTorch load-args
    helper.
  - Directly replacing the shared startup fields with `InferenceDeviceId` would
    violate the adapter-boundary rule because llama.cpp startup still consumes
    backend-local selectors such as `CUDA0`, while PyTorch worker contracts now
    consume canonical selected ids or omitted auto intent.
  - Required re-plan decision: introduce a typed startup/device-intent design
    that distinguishes scheduler-facing canonical `InferenceDevicePolicy` /
    selected `InferenceDeviceId` facts from backend-local adapter selectors
    before changing `BackendConfig.device`, `InferenceStartRequest.device`,
    `EmbeddingStartRequest.device`, or legacy `DeviceConfig.device`.
  - Deferred until that design is explicit: replacing shared startup raw device
    fields, updating gateway callers, and deciding whether llama.cpp
    backend-local selector types live beside `DeviceBackend` or in a new
    backend-adapter intent DTO.
- 2026-05-11 slice: backend startup device-intent transition contract.
  - Smallest useful vertical slice: add `BackendStartupDeviceIntent` and a
    typed error so shared startup wiring can distinguish scheduler-facing
    `InferenceDevicePolicy`, concrete canonical `InferenceDeviceId`, and
    backend-local llama.cpp `DeviceBackend` selectors before `BackendConfig`
    is migrated.
  - Allowed write set: `crates/inference/src/backend/startup_device.rs`,
    `crates/inference/src/backend/mod.rs`, `crates/inference/src/lib.rs`,
    `crates/inference/src/backend/README.md`,
    `docs/plans/current-image-generation-graphs/06-device-runtime-selection.md`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: constructors are explicit by namespace;
    canonical ids reject `auto`/`CUDA0`, llama.cpp selectors accept
    backend-local `auto`/`CUDA0` but reject canonical `cuda:0`, and no branch
    infers one namespace from another.
  - Standards/blast-radius gate: additive contract and tests only; no runtime
    startup behavior, generated DTOs, lockfiles, workflow fixtures, frontend
    code, scheduler ranking, worker execution, or legacy field rewiring
    changed.
  - Verification passed:
    `cargo test -p inference startup_device`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: migrate startup request DTOs, `BackendConfig.device`,
    and legacy `DeviceConfig.device` through this explicit intent boundary in
    later slices without preserving raw-string compatibility shims.
- 2026-05-11 slice: typed llama.cpp effective runtime device.
  - Smallest useful vertical slice: change normalized
    `LlamaCppRuntimeSettings.device` from `String` to backend-local
    `DeviceBackend` while leaving the shared `BackendConfig.device` input and
    legacy sidecar `DeviceConfig` output untouched for separate migration
    slices.
  - Allowed write set: `crates/inference/src/backend/mod.rs` and this plan
    directory.
  - No-fallback/no-legacy confirmation: validation still requires an explicit
    llama.cpp selector, parses it once into `DeviceBackend`, rejects canonical
    `cuda:0` as a llama.cpp selector, and does not infer canonical selected
    facts from backend-local settings.
  - Standards/blast-radius gate: backend-local runtime-settings typing and
    focused tests only; no gateway API fields, sidecar command DTOs, generated
    DTOs, lockfiles, workflow fixtures, frontend code, scheduler ranking, or
    worker execution behavior changed.
  - Verification passed:
    `cargo test -p inference llamacpp_runtime_settings`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: type or replace legacy `DeviceConfig.device` and
    shared `BackendConfig.device` without accepting raw strings as trusted
    internal state beyond the adapter boundary.
- 2026-05-11 slice: typed legacy sidecar device config.
  - Smallest useful vertical slice: change `DeviceConfig.device` from `String`
    to backend-local `DeviceBackend`, preserve strict llama.cpp selector string
    serde at the DTO boundary, and update server, embedding, gateway, and
    llama.cpp tests that construct sidecar runtime state.
  - Allowed write set: `crates/inference/src/config.rs`,
    `crates/inference/src/server.rs`,
    `crates/inference/src/embedding_runtime.rs`,
    `crates/inference/src/backend/mod.rs`,
    `crates/inference/src/backend/llamacpp.rs`,
    `crates/inference/src/backend/llamacpp_support.rs`,
    `crates/inference/src/server_tests.rs`,
    `crates/inference/src/gateway_tests.rs`, and this plan directory.
  - No-fallback/no-legacy confirmation: invalid sidecar selectors can no longer
    be represented as `DeviceConfig` runtime state; serde rejects malformed
    selectors and canonical ids in the llama.cpp selector namespace; command
    construction uses `DeviceBackend::to_arg` and omits only typed `Auto`.
  - Standards/blast-radius gate: backend-local DTO typing and focused tests
    only; no generated DTOs, lockfiles, workflow fixtures, frontend code,
    scheduler ranking, managed-runtime installation state, or worker execution
    behavior changed.
  - Verification passed:
    `cargo test -p inference config::tests::device_config`,
    `cargo test -p inference active_runtime_descriptor`,
    `cargo test -p inference start_sidecar_inference_applies_runtime_settings_to_llama_server_args`,
    `cargo test -p inference llamacpp_runtime_settings`,
    `cargo test -p inference embedding_runtime::tests`,
    `cargo test -p inference test_mode_info_runtime_facts_report_active_runtime_selected_device`,
    `cargo test -p inference backend::llamacpp::tests`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Discovered issue fixed in-slice: llama.cpp model fingerprint hashing used
    the display representation of the typed device; it now uses the stable
    backend selector id via `DeviceBackend::to_id`.
  - Remaining follow-up: migrate shared `BackendConfig.device`,
    `InferenceStartRequest.device`, `EmbeddingStartRequest.device`, and the
    test-only PyTorch load args helper through explicit startup/device intent
    contracts without accepting raw strings as trusted internal state.
- 2026-05-11 slice: typed gateway startup request device intent.
  - Smallest useful vertical slice: change `InferenceStartRequest.device` and
    `EmbeddingStartRequest.device` from raw strings to
    `BackendStartupDeviceIntent`, then validate the active backend namespace
    while building backend startup config.
  - Allowed write set: `crates/inference/src/gateway.rs`,
    `crates/inference/src/gateway_tests/start_config.rs`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: PyTorch accepts canonical device ids
    or explicit auto policy omission only; llama.cpp accepts backend-local
    selectors only; unresolved explicit policies, wrong backend namespaces,
    external-runtime device intents, and Candle embedding device intents return
    typed `BackendError::Config` diagnostics instead of being preserved,
    inferred, or silently ignored.
  - Standards/blast-radius gate: gateway request DTO and focused tests only;
    no `BackendConfig.device` migration, generated DTOs, lockfiles, workflow
    fixtures, frontend code, managed-runtime install state, scheduler ranking,
    or worker execution behavior changed.
  - Verification passed:
    `cargo test -p inference gateway::tests::start_config`,
    `cargo test -p inference startup_device`,
    `cargo test -p pantograph-embedded-runtime edit_session_execution`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: migrate shared `BackendConfig.device`, node-engine
    llama.cpp runtime settings, and the test-only PyTorch load args helper
    through explicit device-intent adapters without accepting raw strings as
    trusted internal execution state.
- 2026-05-11 slice: typed shared backend config device intent.
  - Smallest useful vertical slice: change `BackendConfig.device` from
    `Option<String>` to `Option<BackendStartupDeviceIntent>`, then update
    llama.cpp, PyTorch, gateway, node-engine llama.cpp config construction,
    and focused tests to validate backend/device namespaces at adapter
    boundaries.
  - Allowed write set: `crates/inference/src/backend/mod.rs`,
    `crates/inference/src/backend/llamacpp.rs`,
    `crates/inference/src/backend/pytorch.rs`,
    `crates/inference/src/gateway.rs`,
    `crates/inference/src/gateway_tests.rs`,
    `crates/inference/src/gateway_tests/start_config.rs`,
    `crates/node-engine/src/core_executor/llamacpp_nodes.rs`,
    `crates/inference/src/README.md`,
    `crates/inference/src/backend/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: shared backend config no longer stores
    trusted raw device strings; llama.cpp accepts typed backend-local selectors
    or explicit auto policy only, PyTorch accepts canonical ids or auto policy
    only, wrong namespaces return `BackendError::Config`, and node-engine
    invalid llama.cpp device strings cannot become executable raw config.
  - Standards/blast-radius gate: public Rust startup config contract and
    focused node-engine adapter projection only; no generated DTOs, lockfiles,
    workflow fixtures, frontend code, managed-runtime install state, scheduler
    ranking, persisted state schema, or worker execution behavior changed.
  - Verification passed:
    `cargo test -p inference backend::tests`,
    `cargo test -p inference gateway::tests::start_config`,
    `cargo test -p inference backend::llamacpp::tests`,
    `cargo test -p node-engine --features inference-nodes backend_config_applies_llamacpp_runtime_settings`,
    `cargo test -p node-engine --features inference-nodes runtime_settings_match_compares_reload_required_performance_settings`,
    `cargo test -p node-engine --features inference-nodes gateway_match_rejects_different_runtime_settings`,
    `cargo test -p inference --features backend-pytorch test_pytorch_worker_load_envelope`,
    `cargo test -p pantograph-embedded-runtime edit_session_execution`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: one node-engine command attempted two Cargo test
    filters and failed before tests ran; both filters were rerun individually
    and passed.
  - Remaining follow-up: replace remaining test-only or backend-local worker
    load helper raw-device arguments and continue removing node-engine
    independent backend-routing choices in favor of canonical scheduler
    decisions.
- 2026-05-11 slice: typed PyTorch test load helper device.
  - Smallest useful vertical slice: change the test-only
    `PyTorchTransformersLoadArgs.device` helper from `String` to
    `Option<InferenceDeviceId>` so tests model omitted auto intent the same
    way as worker envelopes.
  - Allowed write set: `crates/inference/src/backend/pytorch.rs`,
    `crates/inference/src/backend/pytorch_tests.rs`, and this plan directory.
  - No-fallback/no-legacy confirmation: omitted auto intent remains `None`;
    explicit devices remain canonical `InferenceDeviceId`; no helper converts
    auto into a trusted raw device string.
  - Standards/blast-radius gate: test-only helper typing and focused
    feature-gated tests only; no generated DTOs, lockfiles, workflow fixtures,
    frontend code, runtime startup behavior, scheduler ranking, or worker
    execution behavior changed.
  - Verification passed:
    `cargo test -p inference --features backend-pytorch test_pytorch_transformers_load_args`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: continue removing backend-local PyTorch runtime load
    raw `"auto"` arguments as the executable startup adapter moves from direct
    worker strings to typed load intent.
- 2026-05-11 slice: typed PyTorch direct load device input.
  - Smallest useful vertical slice: change PyTorch direct/package model load
    APIs and direct worker-envelope builders to accept
    `Option<InferenceDeviceId>`, then parse node-engine PyTorch workflow
    device input at the adapter boundary.
  - Allowed write set: `crates/inference/src/backend/pytorch.rs`,
    `crates/inference/src/backend/pytorch_tests.rs`,
    `crates/node-engine/src/core_executor/pytorch_nodes.rs`,
    `crates/inference/src/backend/README.md`, and this plan directory.
  - No-fallback/no-legacy confirmation: omitted or explicit `auto` device
    intent remains `None`; explicit devices must be canonical
    `InferenceDeviceId`; node-engine rejects legacy backend-local ids such as
    `CUDA0` before backend load instead of forwarding raw strings.
  - Standards/blast-radius gate: PyTorch adapter API and node-engine adapter
    parsing only; no generated DTOs, lockfiles, workflow fixtures, frontend
    code, scheduler ranking, managed-runtime install state, or persisted schema
    changed.
  - Verification passed:
    `cargo test -p inference --features backend-pytorch test_pytorch_direct_load_envelope`,
    `cargo test -p inference --features backend-pytorch test_pytorch_load_envelope`,
    `cargo test -p inference --features backend-pytorch test_can_reuse_loaded_model_requires_matching_request`,
    `cargo test -p node-engine --features pytorch-nodes pytorch_load_device_from_inputs`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: one PyTorch test command attempted two Cargo test
    filters and failed before tests ran; both filters were rerun individually
    and passed.
  - Remaining follow-up: connect PyTorch node execution to canonical scheduler
    selected-device decisions rather than direct workflow input once node-engine
    backend routing is replaced.
- 2026-05-11 slice: managed runtime canonical root enforcement.
  - Smallest useful vertical slice: remove the legacy managed runtime root
    lookup from `managed_install_dir` and replace the old fallback test with a
    regression test that proves an existing `app_data/runtimes/llama-cpp`
    directory is ignored.
  - Allowed write set: `crates/inference/src/managed_runtime/paths.rs` and
    this plan directory.
  - No-fallback/no-legacy confirmation: managed runtime command resolution,
    projection, install, and remove paths can no longer discover executable
    runtime files from the retired `app_data/runtimes` tree. They resolve only
    through the canonical `app_data/third-party/runtimes` path.
  - Standards/blast-radius gate: path helper and crate-local unit coverage
    only; no persisted-state schema, generated DTOs, frontend code, lockfiles,
    workflow fixtures, runtime variant selection, subprocess launch behavior,
    or managed install job lifecycle changed.
  - Verification passed:
    `cargo test -p inference managed_runtime::paths`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Discovered issue fixed in-slice: `managed_install_dir` still accepted a
    legacy runtime root when it existed, which was incompatible with the
    milestone's no-fallback/no-legacy rule.
  - Remaining follow-up: implement shared allowed-root validation for runtime
    roots, executable paths, dynamic-library paths, Pumas package paths,
    artifact paths, and worker-visible paths.
- 2026-05-11 slice: Linux llama.cpp CUDA fallback removal.
  - Smallest useful vertical slice: update Linux llama.cpp command resolution
    so `--device CUDA*` requires the CUDA runtime executable at
    `cuda/llama-server`, and add focused tests for missing and present CUDA
    runtime executables.
  - Allowed write set:
    `crates/inference/src/managed_runtime/llama_cpp_platform/linux.rs` and
    this plan directory.
  - No-fallback/no-legacy confirmation: explicit CUDA runtime requests no
    longer fall through to the CPU executable when CUDA files are absent.
    Missing CUDA runtime files stop command resolution with a bounded error
    instead of constructing a CPU command.
  - Standards/blast-radius gate: Linux platform command selection and
    crate-local tests only; no generated DTOs, frontend code, lockfiles,
    workflow fixtures, managed runtime persisted schema, install jobs, or
    runtime catalog state changed.
  - Verification passed:
    `cargo test -p inference managed_runtime::llama_cpp_platform::linux::tests`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first focused test run failed because the test
    asserted the full `LD_LIBRARY_PATH` value even though existing environment
    entries are preserved; the test was narrowed to assert the prepended CUDA
    path entry and then passed. The first format check also reported
    rustfmt-only wrapping, then passed after `cargo fmt --all`.
  - Discovered issue fixed in-slice: Linux `resolve_command` treated explicit
    CUDA as a preference rather than a requirement and silently used the CPU
    executable if `cuda/llama-server` was absent.
  - Remaining follow-up: replace the managed-runtime command-resolution
    `String` error surface with typed runtime-variant diagnostics so missing
    CUDA variants can be reported through the canonical diagnostic DTO.
- 2026-05-11 slice: typed managed runtime command diagnostics.
  - Smallest useful vertical slice: replace the core managed-runtime
    `resolve_binary_command` `String` error surface with
    `ManagedRuntimeCommandResolutionError`, thread that typed error through the
    llama.cpp platform/definition boundary, and stringify it only at existing
    string-returning facades.
  - Allowed write set: `crates/inference/src/managed_runtime/contracts.rs`,
    `crates/inference/src/managed_runtime/operations.rs`,
    `crates/inference/src/managed_runtime/definitions.rs`,
    `crates/inference/src/managed_runtime/mod.rs`,
    `crates/inference/src/managed_runtime/neutral_contracts.rs`,
    `crates/inference/src/managed_runtime/llama_cpp_platform/`,
    `crates/inference/src/device.rs`, `crates/inference/src/lib.rs`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: explicit missing CUDA runtime variants
    now produce a typed `MissingRuntimeVariant` command-resolution error with
    the canonical `DeviceResolutionDiagnosticCode::MissingRuntimeVariant`
    payload. No broad `From<String>` conversion was added; legacy
    string-returning facades convert explicitly at their boundary.
  - Standards/blast-radius gate: managed-runtime command contract and focused
    adapter boundary conversions only; no generated DTOs, frontend code,
    lockfiles, workflow fixtures, managed runtime persisted schema, install
    jobs, runtime catalog state, or subprocess lifecycle changed.
  - Verification passed:
    `cargo test -p inference managed_runtime::contracts::tests`,
    `cargo test -p inference managed_runtime::llama_cpp_platform::linux::tests`,
    `cargo test -p inference managed_runtime::operations`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: initial compile failed on one remaining
    `resolve_runtime_install_dir` string error in command resolution; it was
    converted into an explicit typed `State` variant instead of a broad
    string-to-error conversion. The first format check reported rustfmt-only
    wrapping and passed after `cargo fmt --all`.
- 2026-05-11 slice: canonical runtime variant fixture ids.
  - Smallest useful vertical slice: replace slash-shaped runtime variant ids in
    technical-fit and scheduler/diagnostics fixture tests with canonical
    dot-shaped ids that match `RuntimeVariantId` validation.
  - Allowed write set:
    `docs/plans/current-image-generation-graphs/06-device-runtime-selection.md`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/tests/contract.rs`,
    `crates/pantograph-workflow-service/tests/fixtures/technical_fit_contract.json`,
    `src/components/workbench/diagnosticsPagePresenters.test.ts`,
    `src/components/workbench/schedulerPagePresenters.test.ts`,
    `src/services/workflow/WorkflowService.commands.test.ts`,
    `src/stores/schedulerRunListStore.test.ts`, and this plan directory.
  - No-fallback/no-legacy confirmation: fixtures no longer preserve
    slash-shaped runtime variant ids such as `llama_cpp/linux-x64/cuda` or
    `pytorch/linux-x64/cuda`; no compatibility parser, alias, or production
    fallback was added.
  - Standards/blast-radius gate: fixture/test data and plan examples only; no
    production routing, generated DTOs, lockfiles, persisted schema, saved
    workflow files, runtime catalog state, or frontend component behavior
    changed.
  - Verification passed:
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`,
    `cargo test -p pantograph-workflow-service --test contract workflow_technical_fit_cross_layer_fixture_deserializes`,
    `node --experimental-strip-types --test src/components/workbench/schedulerPagePresenters.test.ts src/components/workbench/diagnosticsPagePresenters.test.ts src/stores/schedulerRunListStore.test.ts src/services/workflow/WorkflowService.commands.test.ts`,
    `bash -lc 'if rg -n "[a-z0-9_-]+/[a-z0-9_-]+/(cpu|cuda|metal)|runtime-a/cuda|runtime-b/metal|llama_cpp/|pytorch/" crates/pantograph-embedded-runtime crates/pantograph-workflow-service src -g "*.rs" -g "*.ts" -g "*.svelte" -g "*.json"; then exit 1; else exit 0; fi'`,
    and `git diff --check`.
  - Verification deviation: the first Node run failed because the scheduler
    search expectation and shared technical-fit JSON fixture still contained
    old slash-shaped values; the first workflow-service contract run also
    started before the fixture update was complete. Both were corrected and
    rerun successfully.
  - Remaining follow-up: update saved workflow fixtures/files, if any still
    carry old raw-device or runtime-variant shapes, in a dedicated workflow
    fixture slice.
- 2026-05-11 slice: managed runtime version variant identity.
  - Smallest useful vertical slice: add typed `RuntimeVariantId` identity to
    managed-runtime catalog versions, projected version statuses, and persisted
    installed versions, while defaulting existing llama.cpp managed installs to
    `llama_cpp.cpu`.
  - Allowed write set: `crates/inference/src/managed_runtime/`,
    `crates/inference/src/runtime_load.rs`,
    `crates/pantograph-embedded-runtime/src/managed_runtime_manager.rs`,
    `crates/pantograph-embedded-runtime/src/lib_tests.rs`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the slice adds a canonical typed
    variant identity to managed-runtime state/projection instead of inferring
    variants from platform strings or preserving slash-shaped ids. It does not
    add duplicate binary managers, compatibility aliases, generated DTO shims,
    or variant selection fallback behavior.
  - Standards/blast-radius gate: additive DTO/persisted-state field plus
    focused tests only; no lockfiles, generated bindings, frontend code, saved
    workflow files, install job concurrency, catalog download behavior,
    subprocess launch behavior, or runtime selection policy changed.
  - Verification passed:
    `cargo test -p inference managed_runtime::catalog`,
    `cargo test -p inference managed_runtime::operations`,
    `cargo test -p inference runtime_load`,
    `cargo test -p pantograph-embedded-runtime managed_runtime`,
    `cargo test -p pantograph-embedded-runtime runtime_capabilities`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: embedded-runtime compile first failed because the
    fixture helper referenced `RuntimeVariantId` outside its test-module import
    scope; it was corrected to use `inference::RuntimeVariantId` and rerun
    successfully. The first format check reported rustfmt-only wrapping and
    passed after `cargo fmt --all`.
  - Remaining follow-up: include runtime variant id on managed install jobs,
    retained artifacts, progress snapshots, install history, selected variant
    state, command resolution, and variant-specific CUDA/Metal readiness.
- 2026-05-11 slice: shared allowed-root command path validation.
  - Smallest useful vertical slice: extract the existing node-engine
    allowed-root path validator into a shared `pantograph-path-security` crate,
    keep node-engine/workflow callers on that shared helper, and validate
    managed-runtime selected install roots plus resolved executable and
    working-directory paths before command handoff.
  - Allowed write set: workspace Cargo manifests/lockfile,
    `crates/pantograph-path-security/`, `crates/node-engine/src/path_validation.rs`,
    `crates/inference/src/managed_runtime/contracts.rs`,
    `crates/inference/src/managed_runtime/mod.rs`,
    `crates/inference/src/managed_runtime/operations.rs`,
    `crates/inference/src/managed_runtime/operations/projection.rs`,
    `crates/inference/src/managed_runtime/operations_tests.rs`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: selected persisted managed-runtime
    install roots that escape the canonical managed runtime root now fail with
    `ManagedRuntimeCommandResolutionError::PathValidation` instead of being
    treated as trusted executable state. The slice did not add legacy root
    probing, compatibility aliases, raw path fallback, or duplicate validation
    logic.
  - Standards/blast-radius gate: security/path-boundary helper extraction and
    managed-runtime command validation only; no generated DTOs, frontend code,
    saved workflow files, runtime feature flags, external dependencies,
    subprocess lifecycle behavior, or scheduler policy changed. The new crate
    has no third-party runtime dependencies.
  - Verification passed:
    `cargo test -p pantograph-path-security`,
    `cargo test -p node-engine path_validation`,
    `cargo test -p inference resolve_binary_command`,
    `cargo test -p inference managed_runtime_snapshot`,
    `cargo test -p pantograph-workflow-service persistence`,
    `cargo test -p workflow-nodes storage`, and
    `cargo test -p inference managed_runtime::operations`.
  - Verification deviations fixed during the slice: initially started several
    Cargo test commands in parallel, which serialized on the Cargo package and
    build-directory locks; the completed results above passed after the locks
    cleared. The first managed-runtime projection compile failed because the
    allowed-root projection helper needed `app_data_dir`; the projection
    call chain now passes that parameter explicitly.
  - Remaining follow-up: validate dynamic-library environment path entries,
    pid files, Pumas package paths, artifact paths, and worker-visible paths
    through the same shared validator before filesystem or subprocess access.
- 2026-05-11 slice: managed runtime dynamic-library path validation.
  - Smallest useful vertical slice: stop inheriting host library-search tails
    into managed llama.cpp command environment overrides, emit only the
    backend-owned runtime library path, and validate each command environment
    path value through the shared allowed-root validator before handoff.
  - Allowed write set: `crates/inference/src/managed_runtime/contracts.rs`,
    `crates/inference/src/managed_runtime/neutral_contracts.rs`,
    `crates/inference/src/managed_runtime/paths.rs`,
    `crates/inference/src/managed_runtime/mod.rs`,
    `crates/inference/src/managed_runtime/operations.rs`,
    `crates/inference/src/managed_runtime/llama_cpp_platform/`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: managed runtime command launch no
    longer preserves unvalidated inherited host `LD_LIBRARY_PATH`,
    `DYLD_LIBRARY_PATH`, or Windows `PATH` entries as fallback dynamic-library
    search locations. Only backend-owned, allowed-root-validated runtime paths
    are handed off.
  - Standards/blast-radius gate: managed-runtime command environment
    construction and validation only; no generated DTOs, frontend code, saved
    workflow files, runtime feature flags, dependency changes, scheduler
    policy, install state, or subprocess lifecycle ownership changed.
  - Verification passed:
    `cargo test -p inference managed_runtime::paths`,
    `cargo test -p inference managed_runtime::llama_cpp_platform::linux::tests`,
    `cargo test -p inference resolve_binary_command`, and
    `cargo test -p inference runtime_sidecar_command_projection_preserves_resolved_command_facts`.
  - Verification deviation: the focused Cargo tests were started in parallel
    and serialized on Cargo package/build locks; the completed results above
    passed after the locks cleared. A broader neutral-contract check exposed a
    stale test fixture still using the retired `app_data/runtimes` install
    root; the fixture now uses the canonical managed runtime version
    directory and the focused check passes.
  - Remaining follow-up: pid files, Pumas package paths, artifact paths, and
    worker-visible paths still need shared allowed-root validation before
    filesystem or subprocess access.
- 2026-05-11 slice: managed runtime pid-file path validation.
  - Smallest useful vertical slice: validate managed-runtime command
    `--pid-file` paths against `app_data_dir` before command handoff, while
    preserving relative pid-file intent by resolving it under the same root.
  - Allowed write set: `crates/inference/src/managed_runtime/contracts.rs`,
    `crates/inference/src/managed_runtime/operations.rs`,
    `crates/inference/src/managed_runtime/operations_tests.rs`,
    `crates/inference/src/managed_runtime/neutral_contracts.rs`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: arbitrary absolute pid-file paths are
    no longer preserved as subprocess write targets. Escaped pid-file requests
    fail with `ManagedRuntimeCommandResolutionError::PathValidation` and no
    alternate pid-file path is synthesized.
  - Standards/blast-radius gate: managed-runtime command path validation and
    command projection tests only; no generated DTOs, frontend code, saved
    workflow files, runtime feature flags, dependency changes, scheduler
    policy, install state, dynamic-library path behavior, or subprocess
    lifecycle ownership changed.
  - Verification passed:
    `cargo test -p inference resolve_binary_command` and
    `cargo test -p inference runtime_sidecar_command_projection_preserves_resolved_command_facts`.
  - Verification deviation: the focused Cargo tests were started in parallel
    and serialized on Cargo package/build locks; the completed results above
    passed after the locks cleared.
  - Remaining follow-up: Pumas package paths, artifact paths, and
    worker-visible paths still need shared allowed-root validation before
    filesystem or subprocess access.
- 2026-05-11 slice: artifact-store checked byte accounting.
  - Smallest useful vertical slice: replace unchecked or saturating
    artifact-store memory-cache and streaming chunk byte accounting with
    checked arithmetic and a typed `ArtifactStoreError::ArtifactAccountingOverflow`.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`,
    `crates/pantograph-workflow-service/src/workflow/artifact_store/cache.rs`,
    `crates/pantograph-workflow-service/src/workflow/artifact_store/stream.rs`,
    `crates/pantograph-workflow-service/src/workflow/artifact_api.rs`, and
    this plan directory.
  - No-fallback/no-legacy confirmation: overflow is rejected or skipped before
    mutating artifact accounting. The slice does not clamp overflowed stream
    byte lengths, preserve saturating totals as executable state, or infer
    alternate artifact sizes.
  - Standards/blast-radius gate: workflow artifact-store accounting and API
    error projection only; no generated DTOs, frontend code, saved workflow
    files, lockfiles, path roots, Pumas contracts, worker contracts, or runtime
    scheduler policy changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service memory_cache_capacity_check_rejects_overflow`,
    `cargo test -p pantograph-workflow-service stream_chunk_rejects_byte_length_overflow`,
    `cargo test -p pantograph-workflow-service --test artifact_store`, and
    `cargo test -p pantograph-workflow-service workflow::artifact_store`.
  - Verification deviations fixed during the slice: the first focused compile
    exposed that `artifact_api.rs` needed to project the new typed store error;
    `ArtifactAccountingOverflow` now maps to `WorkflowServiceError::InvalidRequest`.
    Focused Cargo tests were started in parallel and serialized on Cargo locks
    before passing.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    dimensions, context/token/batch limits, memory estimates, disk budget
    summation, artifact stats summation, byte-range projections, and
    worker/runtime request fields.
- 2026-05-11 slice: artifact-store disk-budget checked summation.
  - Smallest useful vertical slice: replace the artifact-store disk-budget
    `sum::<u64>().saturating_add(...)` projection with checked accumulation
    across retained artifacts, pending streams, and the replacement body size.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/artifact_store.rs` and
    this plan directory.
  - No-fallback/no-legacy confirmation: overflow now returns typed
    `ArtifactStoreError::ArtifactAccountingOverflow { field:
    "disk_usage_bytes" }` before any new artifact body, descriptor, manifest,
    or memory-cache state is written. The slice does not clamp or saturate
    projected disk usage.
  - Standards/blast-radius gate: artifact-store disk accounting only; no API
    DTOs, generated files, frontend code, saved workflow fixtures, lockfiles,
    path roots, Pumas contracts, worker contracts, runtime scheduler policy, or
    backend startup behavior changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service disk_limit_projection_rejects_total_byte_overflow`,
    `cargo test -p pantograph-workflow-service workflow::artifact_store`, and
    `cargo test -p pantograph-workflow-service --test artifact_store`.
  - Verification deviations: `cargo fmt --all -- --check` found rustfmt-only
    wrapping before verification; `cargo fmt --all` was applied and the tests
    above passed after formatting. Parallel Cargo tests serialized on package
    and build locks before passing.
  - Discovered issue/deferred follow-up: `artifact_store.rs` is already over
    the 500-line coding-standards decomposition-review trigger. The split was
    deferred because this slice intentionally stayed within disk-accounting
    behavior and the existing colocated unit-test pattern.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    dimensions, context/token/batch limits, memory estimates, artifact stats
    summation, byte-range projections, and worker/runtime request fields.
- 2026-05-11 slice: artifact-store stats checked summation.
  - Smallest useful vertical slice: make `ArtifactStore::stats()` fallible and
    replace unchecked stats counter/body-byte additions with checked arithmetic.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`,
    `crates/pantograph-workflow-service/src/workflow/artifact_api.rs`,
    `crates/pantograph-workflow-service/tests/artifact_store.rs`,
    `crates/pantograph-workflow-service/tests/artifact_store_policy.rs`, and
    this plan directory.
  - No-fallback/no-legacy confirmation: overflow now returns typed
    `ArtifactStoreError::ArtifactAccountingOverflow` instead of wrapping,
    saturating, or silently returning partial stats. The workflow service stats
    facade projects that typed store error through its existing `Result`
    boundary.
  - Standards/blast-radius gate: artifact stats accounting only; no generated
    files, frontend code, saved workflow fixtures, lockfiles, path roots, Pumas
    contracts, worker contracts, runtime scheduler policy, or backend startup
    behavior changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service stats_rejects_retained_body_byte_overflow`,
    `cargo test -p pantograph-workflow-service workflow::artifact_store`,
    `cargo test -p pantograph-workflow-service --test artifact_store`,
    `cargo test -p pantograph-workflow-service --test artifact_store_policy`,
    `cargo test -p pantograph-embedded-runtime workflow_artifact_store_stats`,
    and `cargo test -p pantograph-uniffi workflow_artifact_store_stats`.
  - Verification deviations/discovered issues: the embedded-runtime and UniFFI
    focused filters matched zero tests but still compiled their public stats
    facades successfully. The UniFFI compile surfaced pre-existing unused
    imports in `crates/pantograph-embedded-runtime/src/technical_fit.rs`
    (`WorkflowBackendCapabilityFacts` and
    `WorkflowRuntimeVariantCapability`); cleanup is deferred because it is
    unrelated to artifact stats accounting.
  - Discovered issue/deferred follow-up: `artifact_store.rs` remains over the
    500-line coding-standards decomposition-review trigger. The split remains
    deferred to avoid broadening this stats-accounting slice.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    dimensions, context/token/batch limits, memory estimates, byte-range
    projections, and worker/runtime request fields.
- 2026-05-11 slice: llama.cpp context-size fail-closed validation.
  - Smallest useful vertical slice: include `BackendConfig.context_size` in
    `LlamaCppRuntimeSettings::try_from_backend_config` positive-value
    validation so `Some(0)` cannot become an effective llama-server `-c 0`
    setting.
  - Allowed write set:
    `crates/inference/src/backend/mod.rs` and this plan directory.
  - No-fallback/no-legacy confirmation: invalid explicit context size now
    returns `BackendError::Config` through the existing typed backend startup
    boundary. The slice does not replace invalid zero with the default context
    size and does not preserve the previous executable zero-value path.
  - Standards/blast-radius gate: llama.cpp runtime setting validation only; no
    generated files, frontend code, saved workflow fixtures, lockfiles, path
    roots, Pumas contracts, worker contracts, runtime scheduler policy, or
    backend lifecycle behavior changed.
  - Verification passed:
    `cargo test -p inference llamacpp_runtime_settings_reject_zero_sized_performance_knobs`,
    `cargo test -p inference llamacpp_runtime_settings`, and
    `cargo fmt --all -- --check`.
  - Verification deviation: the two focused Cargo tests were started in
    parallel and serialized on Cargo package/build locks before passing.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    dimensions, context/token/batch limits outside this llama.cpp startup
    normalization boundary, memory estimates, byte-range projections, and
    worker/runtime request fields.
- 2026-05-11 slice: diagnostics projection rebuild batch-size validation.
  - Smallest useful vertical slice: remove the `.max(1)` fallback from
    `workflow_projection_rebuild` and reject explicit `batch_size: Some(0)`
    through the existing workflow-service invalid-request boundary.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/diagnostics.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: explicit zero no longer becomes
    batch size one. `None` still selects the canonical default because it is an
    absent option, not an invalid explicit numeric request.
  - Standards/blast-radius gate: diagnostics projection rebuild request
    validation only; no generated files, frontend code, saved workflow
    fixtures, lockfiles, path roots, Pumas contracts, worker contracts,
    runtime scheduler policy, or backend lifecycle behavior changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service workflow_projection_rebuild_validates_bounds`,
    `cargo test -p pantograph-workflow-service workflow_diagnostics_projection_refresh_validates_request`,
    and `cargo fmt --all -- --check`.
  - Verification deviation: the two focused Cargo tests were started in
    parallel and serialized on Cargo package/build locks before passing.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    dimensions, context/token/batch limits outside this projection rebuild
    validation boundary, memory estimates, byte-range projections, and
    worker/runtime request fields.
- 2026-05-11 slice: image generation zero-dimension validation.
  - Smallest useful vertical slice: validate typed image generation width and
    height at the inference gateway before backend dispatch.
  - Allowed write set:
    `crates/inference/src/gateway.rs`,
    `crates/inference/src/gateway_tests.rs`, and this plan directory.
  - No-fallback/no-legacy confirmation: explicit zero dimensions now fail with
    `BackendError::Config`; the gateway does not replace them with backend
    defaults, clamp to one, or pass zero through to backend implementations.
    `None` remains an absent option owned by the selected backend.
  - Standards/blast-radius gate: typed image generation request validation
    only; no generated files, frontend code, saved workflow fixtures,
    lockfiles, path roots, Pumas contracts, worker contracts, runtime
    scheduler policy, or backend lifecycle behavior changed.
  - Verification passed:
    `cargo test -p inference test_generate_image_rejects_zero_dimensions`,
    `cargo test -p inference test_execute_typed_forwards_image_generation_to_active_backend`,
    and `cargo fmt --all -- --check`.
  - Verification deviation: the two focused Cargo tests were started in
    parallel and serialized on Cargo package/build locks before passing.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    request limits beyond zero dimensions, context/token/batch limits, memory
    estimates, byte-range projections, and worker/runtime request fields.
- 2026-05-11 slice: image generation positive count validation.
  - Smallest useful vertical slice: extend typed image generation gateway
    validation to reject explicit zero `num_inference_steps` and
    `num_images_per_prompt` before backend dispatch.
  - Allowed write set:
    `crates/inference/src/gateway.rs`,
    `crates/inference/src/gateway_tests.rs`, and this plan directory.
  - No-fallback/no-legacy confirmation: explicit zero image count/request
    values now fail with `BackendError::Config`; the gateway does not replace
    them with backend defaults, clamp to one, or pass zero through to backend
    implementations. `None` remains an absent option owned by the selected
    backend.
  - Standards/blast-radius gate: typed image generation request validation
    only; no generated files, frontend code, saved workflow fixtures,
    lockfiles, path roots, Pumas contracts, worker contracts, runtime
    scheduler policy, or backend lifecycle behavior changed.
  - Verification passed:
    `cargo test -p inference test_generate_image_rejects_zero_positive_count_options`,
    `cargo test -p inference test_generate_image_rejects_zero_dimensions`, and
    `cargo fmt --all -- --check`.
  - Verification deviations: the first `cargo fmt --all -- --check` found
    rustfmt-only wrapping; `cargo fmt --all` was applied and verification was
    rerun successfully. The two focused Cargo tests were started in parallel
    and serialized on Cargo package/build locks before passing.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    request limits beyond positive-count validation, context/token/batch
    limits, memory estimates, byte-range projections, and worker/runtime
    request fields.
- 2026-05-11 slice: retention cleanup zero-limit validation.
  - Smallest useful vertical slice: remove the `.max(1)` fallback from
    `workflow_retention_cleanup_apply` and reject explicit `limit: Some(0)`
    through the existing workflow-service invalid-request boundary.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/diagnostics.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: explicit zero no longer becomes
    cleanup limit one. `None` still selects the canonical default because it
    is an absent option, not an invalid explicit numeric request.
  - Standards/blast-radius gate: retention cleanup request validation only; no
    generated files, frontend code, saved workflow fixtures, lockfiles, path
    roots, Pumas contracts, worker contracts, runtime scheduler policy, or
    backend lifecycle behavior changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service workflow_retention_cleanup_rejects_zero_limit`,
    `cargo test -p pantograph-workflow-service workflow_retention_cleanup_expires_artifacts_through_projection`,
    and `cargo fmt --all -- --check`.
  - Verification deviation: the two focused Cargo tests were started in
    parallel and serialized on Cargo package/build locks before passing.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    request limits, context/token/batch limits outside this retention-cleanup
    validation boundary, memory estimates, byte-range projections, and
    worker/runtime request fields.
- 2026-05-11 slice: diagnostics query zero-limit validation.
  - Smallest useful vertical slice: remove diagnostics query DTO `.max(1)`
    fallbacks and reject explicit zero `page_size`/`limit` values through the
    existing workflow-service invalid-request boundary.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/diagnostics.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: explicit zero no longer becomes page
    or result limit one. `None` still selects the canonical default because it
    is an absent option, not an invalid explicit numeric request.
  - Standards/blast-radius gate: diagnostics query request validation only; no
    generated files, frontend code, saved workflow fixtures, lockfiles, path
    roots, Pumas contracts, worker contracts, runtime scheduler policy, or
    backend lifecycle behavior changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service workflow_diagnostics_usage_query_validates_ids_and_bounds`,
    `cargo test -p pantograph-workflow-service workflow_scheduler_timeline_query_validates_bounds`,
    `cargo test -p pantograph-workflow-service workflow_run_list_query_validates_bounds`,
    `cargo test -p pantograph-workflow-service workflow_io_artifact_query_validates_bounds`,
    `cargo test -p pantograph-workflow-service workflow_node_status_query_rejects_zero_limit`,
    and
    `cargo test -p pantograph-workflow-service workflow_library_usage_query_validates_bounds`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` found
    rustfmt-only wrapping in the touched tests; `cargo fmt --all` was applied
    and final format verification passed before commit.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    request limits, context/token/batch limits outside this diagnostics query
    validation boundary, memory estimates, byte-range projections, and
    worker/runtime request fields.
- 2026-05-11 slice: loaded-runtime capacity limit validation.
  - Smallest useful vertical slice: replace the
    `set_loaded_runtime_capacity_limit` min/max clamp with explicit validation
    for zero and above-session-limit values.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/service_config.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/session_capacity_limits.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: explicit invalid capacity limits no
    longer become one or `max_sessions`; they return
    `WorkflowServiceError::InvalidRequest` and leave the last valid limit
    unchanged. `None` remains the canonical reset to the service session
    limit.
  - Standards/blast-radius gate: workflow-service capacity setter validation
    only; no generated files, frontend code, saved workflow fixtures,
    lockfiles, path roots, Pumas contracts, worker contracts, runtime
    scheduler policy, or backend lifecycle behavior changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service loaded_runtime_capacity_limit_validates_session_bounds`
    and `cargo test -p pantograph-workflow-service session_capacity_limits`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    request limits, context/token/batch limits outside this capacity setter
    validation boundary, memory estimates, byte-range projections, and
    worker/runtime request fields.
- 2026-05-11 slice: runtime-registry reserved resource accounting.
  - Smallest useful vertical slice: replace raw `sum()` aggregation for
    runtime admission reserved RAM/VRAM claims with checked addition and a
    typed registry accounting error.
  - Allowed write set: `crates/pantograph-runtime-registry/src/lib.rs`,
    `crates/pantograph-runtime-registry/src/lib_tests/admission.rs`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: reserved-resource overflow no longer
    depends on debug panic or release wrapping. Valid arithmetic still produces
    existing insufficient RAM/VRAM admission diagnostics.
  - Standards/blast-radius gate: runtime-registry admission accounting only;
    no generated files, frontend code, saved workflow fixtures, lockfiles,
    path roots, Pumas contracts, worker contracts, runtime scheduler policy,
    or backend lifecycle behavior changed.
  - Verification passed:
    `cargo test -p pantograph-runtime-registry reserved_resource_accounting_overflow_returns_typed_error`
    and `cargo test -p pantograph-runtime-registry admission`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` found
    rustfmt-only wrapping in the touched runtime-registry files;
    `cargo fmt --all` was applied and focused tests plus final format
    verification were rerun successfully.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    request limits, context/token/batch limits outside this reservation
    accounting boundary, memory estimates, byte-range projections, and
    worker/runtime request fields.
- 2026-05-11 slice: workflow capability memory estimate accounting.
  - Smallest useful vertical slice: replace saturating model-size megabyte
    rounding and raw peak-memory summation in
    `estimate_memory_requirements` with checked arithmetic and typed
    workflow-service errors.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/capabilities.rs`,
    `crates/pantograph-workflow-service/src/workflow/host.rs`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: invalid model metadata size arithmetic
    no longer saturates into a plausible memory estimate; it fails through
    `WorkflowServiceError::InvalidRequest`. Absent model sizes still produce
    the canonical unknown estimate.
  - Standards/blast-radius gate: workflow capability estimation only; no
    generated files, frontend code, saved workflow fixtures, lockfiles, path
    roots, Pumas contracts, worker contracts, runtime scheduler policy, or
    backend lifecycle behavior changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service memory_estimate` and
    `cargo test -p pantograph-workflow-service workflow_capabilities`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` found
    rustfmt-only wrapping in the touched capability tests; `cargo fmt --all`
    was applied and focused tests plus final format verification were rerun
    successfully.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    request limits, context/token/batch limits outside this capability
    estimation boundary, byte-range projections, and worker/runtime request
    fields.
- 2026-05-11 slice: inference embedding usage token accounting.
  - Smallest useful vertical slice: replace embedding usage `saturating_add`
    plus `u32::MAX` clamping with checked token aggregation and typed gateway
    failure when the total cannot fit the public `InferenceUsage` fields.
  - Allowed write set: `crates/inference/src/gateway.rs`,
    `crates/inference/src/gateway_tests.rs`, and this plan directory.
  - No-fallback/no-legacy confirmation: embedding usage overflow no longer
    produces a plausible capped token count; typed execution and lifecycle
    embedding paths fail with `BackendError::Config`.
  - Standards/blast-radius gate: embedding usage projection only; no generated
    files, frontend code, saved workflow fixtures, lockfiles, path roots, Pumas
    contracts, worker contracts, runtime scheduler policy, or backend
    lifecycle ownership changed.
  - Verification passed: `cargo test -p inference embedding_usage` and
    `cargo test -p inference embedding`, `cargo fmt --all -- --check`, and
    `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` found
    rustfmt-only wrapping in the touched gateway tests; `cargo fmt --all` was
    applied and focused tests plus final format verification were rerun
    successfully.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    request limits, context/batch limits outside this embedding usage
    projection boundary, byte-range projections, and worker/runtime request
    fields.
- 2026-05-11 slice: runtime-registry admission budget underflow validation.
  - Smallest useful vertical slice: replace runtime admission available-budget
    saturating subtraction with checked subtraction across total budget, safety
    margin, and reserved resource claims.
  - Allowed write set: `crates/pantograph-runtime-registry/src/lib.rs`,
    `crates/pantograph-runtime-registry/src/lib_tests/admission.rs`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: impossible budget arithmetic no longer
    becomes zero available resource; it returns typed
    `RuntimeRegistryError::ResourceBudgetUnderflow`. Valid exhausted budgets
    still produce existing insufficient RAM/VRAM admission diagnostics.
  - Standards/blast-radius gate: runtime-registry admission budget arithmetic
    only; no generated files, frontend code, saved workflow fixtures,
    lockfiles, path roots, Pumas contracts, worker contracts, runtime
    scheduler policy, or backend lifecycle ownership changed.
  - Verification passed:
    `cargo test -p pantograph-runtime-registry available_budget_underflow_returns_typed_error`
    and `cargo test -p pantograph-runtime-registry admission`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    request limits, context/batch limits outside this admission budget
    projection boundary, byte-range projections, and worker/runtime request
    fields.
- 2026-05-11 slice: workflow capability zero-size memory estimate validation.
  - Smallest useful vertical slice: reject explicit zero `size_bytes` in model
    metadata before projecting model memory estimates.
  - Allowed write set: `crates/pantograph-workflow-service/src/capabilities.rs`
    and this plan directory.
  - No-fallback/no-legacy confirmation: zero-byte model metadata no longer
    becomes a fabricated 1 MB estimate; it fails with
    `WorkflowServiceError::InvalidRequest`. Missing model metadata still
    produces the canonical unknown estimate.
  - Standards/blast-radius gate: workflow capability memory estimation only;
    no generated files, frontend code, saved workflow fixtures, lockfiles, path
    roots, Pumas contracts, worker contracts, runtime scheduler policy, or
    backend lifecycle ownership changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service memory_estimate`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    request limits, context/batch limits outside this memory estimate
    validation boundary, byte-range projections, and worker/runtime request
    fields.
- 2026-05-11 slice: artifact retention cleanup TTL arithmetic.
  - Smallest useful vertical slice: replace retention cleanup TTL
    second-to-millisecond saturation and cutoff saturation with checked
    arithmetic.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/artifact_store.rs` and
    this plan directory.
  - No-fallback/no-legacy confirmation: impossible retention TTL arithmetic no
    longer projects a saturated cleanup cutoff; it fails with
    `ArtifactStoreError::ArtifactAccountingOverflow`.
  - Standards/blast-radius gate: artifact retention cleanup arithmetic only;
    no generated files, frontend code, saved workflow fixtures, lockfiles, path
    roots, Pumas contracts, worker contracts, runtime scheduler policy, or
    backend lifecycle ownership changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service retention_cleanup_rejects_ttl_millisecond_overflow`
    and `cargo test -p pantograph-workflow-service artifact_store`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: broader checked arithmetic remains needed for image
    request limits, context/batch limits outside this retention cleanup
    boundary, byte-range projections, and worker/runtime request fields.
- 2026-05-12 slice: stale-graph diagnostic summary count validation.
  - Smallest useful vertical slice: replace presentation-only
    `saturating_sub` in stale-graph summary formatting with checked arithmetic
    and a typed internal error for impossible formatter state.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/validation.rs` and this
    plan directory.
  - No-fallback/no-legacy confirmation: impossible diagnostic formatter counts
    no longer become `0 more`; the formatter fails with
    `WorkflowServiceError::Internal`. Normal stale-graph validation still
    returns the existing structured stale-graph diagnostics.
  - Standards/blast-radius gate: stale-graph diagnostic message formatting
    only; no generated files, frontend code, saved workflow fixtures,
    lockfiles, path roots, Pumas contracts, worker contracts, runtime
    scheduler policy, or backend lifecycle ownership changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service stale_graph_remaining_count_rejects_formatter_underflow`
    and `cargo test -p pantograph-workflow-service stale_graph`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: checked arithmetic policy still needs implementation
    for duration/timing diagnostics, scheduler timestamp addition, runtime
    technical-fit rank overflow, cache counter drift, broader image request
    limits, context/batch limits, byte-range projections, and worker/runtime
    request fields.
- 2026-05-12 slice: scheduler runtime-admission retry timestamp validation.
  - Smallest useful vertical slice: replace the session runtime-admission retry
    `now_ms + WORKFLOW_SESSION_QUEUE_POLL_MS` saturation with checked
    arithmetic and a typed workflow-service error.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`
    and this plan directory.
  - No-fallback/no-legacy confirmation: impossible retry timestamp arithmetic
    no longer schedules a saturated retry instant; it fails with
    `WorkflowServiceError::Internal`. Normal runtime-admission delay events
    still carry the scheduler retry timestamp.
  - Standards/blast-radius gate: scheduler retry timestamp projection only; no
    generated files, frontend code, saved workflow fixtures, lockfiles, path
    roots, Pumas contracts, worker contracts, runtime technical-fit ranking, or
    backend lifecycle ownership changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service scheduler_delay_until_rejects_timestamp_overflow`
    and
    `cargo test -p pantograph-workflow-service workflow_execution_session_run_waits_for_runtime_admission`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` found
    rustfmt-only wrapping in the touched scheduler timestamp call;
    `cargo fmt --all` was applied and focused tests plus final format
    verification were rerun successfully.
  - Remaining follow-up: checked arithmetic policy still needs implementation
    for duration/timing diagnostics, runtime technical-fit rank overflow, cache
    counter drift, broader image request limits, context/batch limits,
    byte-range projections, and worker/runtime request fields.
- 2026-05-12 slice: artifact memory-cache removal counter validation.
  - Smallest useful vertical slice: replace memory-cache removal
    `saturating_sub` with checked byte-counter subtraction and return the
    existing artifact accounting overflow error on counter drift.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/artifact_store/cache.rs`,
    `crates/pantograph-workflow-service/src/workflow/artifact_store.rs`, and
    this plan directory.
  - No-fallback/no-legacy confirmation: cache byte-counter underflow no longer
    silently resets the counter to zero; removal fails with
    `ArtifactStoreError::ArtifactAccountingOverflow` and leaves the cached body
    intact for diagnosis.
  - Standards/blast-radius gate: artifact memory-cache accounting only; no
    generated files, frontend code, saved workflow fixtures, lockfiles, path
    roots, Pumas contracts, worker contracts, runtime technical-fit ranking, or
    backend lifecycle ownership changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service memory_cache_remove_rejects_counter_underflow`
    and `cargo test -p pantograph-workflow-service artifact_store`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: checked arithmetic policy still needs implementation
    for duration/timing diagnostics, runtime technical-fit rank overflow,
    broader image request limits, context/batch limits, byte-range projections,
    and worker/runtime request fields.
- 2026-05-12 slice: runtime technical-fit headroom rank validation.
  - Smallest useful vertical slice: reject automatic queue/budget-pressure
    candidate selection when an eligible runtime snapshot reports more active
    reservations than the selector can rank exactly.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/src/README.md`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: unrankable reservation counts no
    longer get capped to the lowest headroom rank. The selector returns an
    unselected automatic decision with an error diagnostic instead of selecting
    another candidate or preserving capped ranking behavior.
  - Standards/blast-radius gate: runtime-registry remains the selector-policy
    owner; existing workflow-service and embedded-runtime DTOs already
    transport `no_valid_candidate` diagnostics, so no generated files,
    frontend code, saved workflow fixtures, lockfiles, path roots, Pumas
    contracts, worker contracts, or backend lifecycle ownership changed.
  - Verification passed:
    `cargo test -p pantograph-runtime-registry selector_rejects_unrankable_headroom_under_queue_pressure`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first final `cargo fmt --all -- --check`
    found rustfmt-only wrapping in the touched selector/test code;
    `cargo fmt --all` was applied and focused tests plus final format
    verification were rerun successfully.
  - Remaining follow-up: checked arithmetic policy still needs implementation
    for duration/timing diagnostics, broader image request limits,
    context/batch limits, byte-range projections, and worker/runtime request
    fields.
- 2026-05-12 slice: workflow startup-repair diagnostics arithmetic.
  - Smallest useful vertical slice: replace startup-repair run-duration
    `saturating_sub` and repaired-count `saturating_add` with checked helpers
    and focused diagnostics tests.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/diagnostics.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: impossible repair timing or count
    state no longer becomes a successful repair with zero duration or a capped
    count. The service fails with a typed internal diagnostic error so startup
    repair does not hide corrupt projection state.
  - Standards/blast-radius gate: workflow-service remains the owner of startup
    repair and diagnostics-ledger projection refresh; no generated files,
    frontend code, saved workflow fixtures, lockfiles, path roots, Pumas
    contracts, worker contracts, runtime selector policy, or backend lifecycle
    ownership changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service startup_repair`,
    `cargo test -p pantograph-workflow-service diagnostics`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: model/runtime load and unload duration diagnostics
    still need the broader timing identity/history policy before they can be
    changed safely; broader image request limits, context/batch limits,
    byte-range projections, and worker/runtime request fields remain open.
- 2026-05-12 re-plan trigger: remaining runtime/model timing saturation is
  shared load/unload/warmup/trace policy, not isolated arithmetic.
  - Code search found remaining duration saturation in
    `crates/inference/src/gateway.rs`,
    `crates/inference/src/embedding_runtime.rs`,
    `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/session_runtime.rs`, and
    `crates/pantograph-workflow-service/src/trace/`.
  - Directly swapping those sites to checked subtraction would catch timestamp
    underflow, but it would not satisfy the agreed policy that load timing is
    diagnostic contract data with attribution, historical normal-range
    learning, and scheduler failure/retry behavior.
  - Required re-plan decision: define the canonical timing measurement
    contract for runtime/model load attempts, unload attempts, warmup attempts,
    and scheduler trace spans. The contract needs attempt ids, workflow run /
    session / runtime / model / device attribution, checked timestamp math,
    diagnostics-ledger payloads, baseline/deviation policy, and scheduler
    ownership for retry versus terminal failure.
  - Deferred until that design is explicit: replacing the remaining
    load/unload/warmup/trace `saturating_sub` sites and deciding which layer
    records timing outliers as retryable scheduler diagnostics versus terminal
    workflow failures.
- 2026-05-12 re-plan decision: use the contract-first timing diagnostics path
  now, with full timing policy enforcement later.
  - Selected option: define canonical timing attempt identities and
    diagnostics payloads first, then replace local duration saturation through
    those contracts in validated vertical slices.
  - Required next slice: introduce the minimum shared timing attempt contract
    for runtime/model load attempts, unload attempts, warmup attempts, and
    scheduler trace spans. The contract must include attempt id, workflow run,
    session, runtime, runtime variant, model/backend/device attribution where
    available, checked start/end/duration semantics, and typed diagnostics for
    impossible timestamp state.
  - Later policy target: after timing history is durably recorded, implement
    baseline/deviation analysis, scheduler reschedule policy, retry exhaustion,
    and terminal workflow failure semantics. This is the full policy path and
    must not be skipped; it is deferred only to keep the next implementation
    slice contract-first and reviewable.
  - 2026-05-13 policy decision: timing and memory-fit metrics are mandatory
    scheduler evidence, because the scheduler's main job is to find the least
    time-intensive valid execution path without overflowing system memory.
    History-backed runtime ranking starts only after each valid runtime for the
    same workflow identity has at least five completed runs. Before that
    threshold, automatic selection must rely solely on current facts and
    distribute runs across valid runtimes with recorded controlled exploration.
    Once the threshold is met, ranking may weigh load duration, warmup
    duration, execution duration, memory pressure, OOM/failure history, and
    already-resident runtime state.
  - No-fallback/no-legacy confirmation: timing outliers and impossible timing
    state must become typed diagnostics. They must not be silently normalized,
    capped, dropped from history, or converted into successful workflow
    execution without scheduler-owned retry/termination decisions.
  - 2026-05-14 policy-boundary decision: runtime-selection algorithms are
    expected to change often, so the implementation must make policy
    replacement cheap. Runtime ranking, controlled exploration, history
    weighting, retry/reschedule policy, and future learned placement must live
    behind a stable runtime-selection policy-engine contract. The orchestration
    layer gathers Pumas package facts, runtime registry candidate facts,
    resource snapshots, diagnostics-ledger history summaries, and user intent,
    then projects them into a normalized runtime-selection input DTO. The policy
    engine must be
    synchronous and side-effect free, returning a typed selected candidate or
    typed diagnostics plus bounded decision trace evidence. It must not query
    Pumas or the ledger directly, inspect workflow graph internals, load
    runtimes, mutate reservations, or hide ranking behavior in backend
    adapters, inference request normalization, frontend presenters, or
    diagnostics projections.
  - 2026-05-14 runtime-selection contract requirement: add or reuse explicit
    execution-placement contracts named for this policy domain, such as
    `RuntimeSelectionDecisionInput`, `RuntimeSelectionDecision`,
    `RuntimeSelectionPolicyTrace`, and `RuntimeSelectionHistorySummary`, before
    changing automatic ranking. Algorithm changes may be implemented by
    swapping policy modules or updating policy configuration; cross-layer DTO
    changes must be append-only and serde/fixture tested. This preserves the
    no-fallback rule because the policy returns typed no-decision diagnostics
    when canonical planning cannot legally select a candidate instead of
    falling through to legacy runtime, backend, device, or graph behavior.
  - 2026-05-14 codebase impact review: the current runtime selector is already
    synchronous and pure, but ranking, filtering, exploration, trace assembly,
    and technical-fit DTOs are still combined in
    `crates/pantograph-runtime-registry/src/technical_fit.rs`. The next slice
    must introduce the runtime-selection policy boundary by delegation: keep the
    existing technical-fit facade for current callers, move automatic
    ranking/exploration into a pure policy module, and prove focused tests keep
    current behavior unchanged before adding history-backed ranking.
  - 2026-05-14 candidate synthesis finding: embedded-runtime currently gathers
    Pumas package facts, backend capabilities, runtime snapshots, and runtime
    variants before calling the selector. That orchestration ownership is
    correct, but missing/unavailable Pumas facts can currently become
    capability-only selection, and one generic runtime-capability path can
    collapse runtime variants before policy sees them. Canonical planning must
    instead project required-fact absence as typed candidate diagnostics and
    emit all valid backend/runtime-variant/device candidates for policy to
    compare. The implementation must use one shared variant-expansion helper
    rather than separate first-variant and all-variants paths, and it must fail
    candidate selection with a typed runtime-selection diagnostic if normalized
    facts exceed a documented candidate cap. The cap and overflow diagnostic
    belong to candidate synthesis; the pure policy module receives an already
    bounded, normalized candidate set and must not silently truncate candidates
    or fabricate missing-fact diagnostics.
  - 2026-05-14 diagnostics/history finding: runtime-selection ranking must not
    reuse the existing workflow timing expectation API because that API has a
    different minimum sample policy and may broaden runtime-specific history
    when runtime-refined samples are insufficient. Add a separate bounded
    diagnostics-ledger runtime-selection history summary keyed by typed
    workflow identity, task/model, backend, runtime variant, and device facts.
    The five-run threshold applies per comparable
    workflow/task/model/backend/runtime-variant/device candidate key; until
    each valid candidate reaches that threshold, policy must rely on facts plus
    recorded controlled exploration rather than broadening history. The
    runtime-selection input receives summaries; the pure policy module must not
    query the ledger.
  - 2026-05-14 trace/admission finding: current policy traces expose string
    `ranking_reason` and `exploration_reason` fields through runtime-registry,
    workflow-service, diagnostics-ledger, and TypeScript DTOs. Future policy
    updates need append-only typed fields for policy phase, decision code,
    history-threshold state, and bounded per-candidate history evidence so
    frontend and ledger projections do not need to understand algorithm
    internals. Admission and reservation-change events must also propagate
    selected device class/id from the runtime-selection decision instead of
    leaving selected device data empty when a decision resolved it.
  - 2026-05-14 partial: workflow-session scheduler admission and
    reservation-changed events now propagate `selected_device_id` from the
    technical-fit decision through the existing reservation context and
    diagnostics-ledger payload fields. Remaining follow-up: append-only typed
    policy trace fields and selected device class propagation still need a
    cross-layer DTO slice.
  - 2026-05-14 partial: runtime-selection policy traces now carry append-only
    typed `policy_phase`, `decision_code`, and `history_threshold_state`
    fields through runtime-registry, embedded-runtime projection,
    workflow-service, diagnostics-ledger scheduler payloads, and frontend
    workflow types. The current automatic selector marks selected-candidate
    traces as candidate-ranking decisions with history not evaluated.
    Remaining follow-up: ledger-backed per-candidate runtime-selection history
    summaries.
  - 2026-05-14 partial: scheduler admission and reservation-changed diagnostic
    payloads now include `selected_device_class` and populate it from the
    canonical technical-fit decision alongside `selected_device_id`. Remaining
    follow-up: ledger-backed per-candidate runtime-selection history summaries.
  - 2026-05-14 standards iteration: implementation of the runtime-selection
    boundary must preserve layered dependency direction and
    sync-core/async-shell design. The pure policy module may depend only on
    runtime-selection contracts and deterministic helpers. It must not import
    workflow-service, embedded-runtime, diagnostics-ledger, inference gateway,
    Pumas, Tauri, TypeScript, filesystem/database/network/subprocess code, or
    create/own async runtimes or background tasks. Fact gathering, ledger
    reads, runtime loading, reservation mutation, and event persistence remain
    outer-shell responsibilities before or after policy invocation.
  - 2026-05-14 standards/blast-radius refinement: the first implementation
    slice for this boundary is limited to the existing runtime-registry
    technical-fit module, a new in-crate pure runtime-selection policy module,
    focused runtime-registry tests, and this plan directory. It must not create
    a new workspace crate, add workspace dependencies, modify
    `pantograph-workflow-service::scheduler`, change diagnostics-ledger schemas
    or event DTOs, update TypeScript mirrors, alter generated/fixture files, or
    touch workflow saved-state fixtures. Those cross-layer surfaces are
    explicitly reserved for later slices after the pure policy boundary and
    validated internal DTOs exist.
  - 2026-05-14 Rust/API and interop standards iteration: new or changed
    runtime-selection DTOs must parse raw strings into validated identity types
    at boundaries, prefer enums over stringly policy states, use typed fallible
    validation errors instead of `Result<T, String>`, derive useful
    `Debug`/`Clone`/`Eq`, apply `#[must_use]` to decisions/results/builders,
    and mark extension-prone public types `#[non_exhaustive]` where compatible
    with existing serde fixtures. Any cross-layer DTO shape change must update
    Rust producers/consumers, diagnostics-ledger payloads, TypeScript mirrors,
    and serde fixtures in the same atomic slice.
  - 2026-05-14 testing standards iteration: policy-boundary slices must keep
    tests aligned with existing repository strategy. Pure runtime-selection
    behavior needs fast Rust unit tests. Runtime-registry, workflow-service,
    embedded-runtime, diagnostics-ledger, and TypeScript mirrors need existing
    contract/fixture tests when their DTOs are touched. Frontend mirror
    verification must use the existing Node test path, not introduce Vitest or
    Playwright. Diagnostics-ledger history tests must use isolated SQLite
    roots and prove runtime-selection ranking history never broadens
    runtime-specific samples as a fallback.
  - 2026-05-14 naming/blast-radius finding: the repository already has
    `pantograph-workflow-service::scheduler` for workflow queue/admission
    decisions, including `WorkflowSchedulerDecisionReason`. The
    backend/runtime/device policy boundary must not add another generic
    `scheduler` module or reuse queue/admission reason types for execution
    placement. Use a specific module name such as `runtime_selection_policy`,
    `execution_placement_policy`, or `technical_fit_policy`; keep admission
    scheduling and execution placement as separate policy domains. Contract,
    trace, history, and diagnostic type names must use `RuntimeSelection*`,
    `ExecutionPlacement*`, or another similarly specific prefix rather than
    bare `Scheduler*`.
- 2026-05-14 required implementation order: (1) extract current automatic
    technical-fit selection into the named pure runtime-selection policy while
    preserving behavior through facade delegation tests, (2) add internal
    validated decision input/output types behind existing public serde DTOs,
    (3) fix candidate synthesis so required Pumas fact absence emits typed
    diagnostics, all valid backend/runtime-variant/device candidates are
    visible to policy through shared variant expansion, and candidate counts
    are bounded before policy invocation, (4)
    add append-only typed trace/admission fields and selected-device
    propagation to scheduler-admission and scheduler-reservation event paths,
    (5) add diagnostics-ledger runtime-selection history summaries with isolated
    SQLite tests and no broad-history fallback, and only then (6) implement the
    five-run threshold and history-backed ranking algorithm.
- 2026-05-14 slice: runtime-selection policy boundary extraction.
  - Smallest useful vertical slice: move the existing automatic technical-fit
    filtering, ranking, controlled exploration, and automatic no-decision
    diagnostic assembly behind an in-crate pure `runtime_selection_policy`
    module while preserving the public `select_runtime_technical_fit` facade
    and explicit override behavior.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/lib.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`, and
    this plan directory.
  - No-fallback/no-legacy confirmation: this is a behavior-preserving
    delegation slice. It does not add history ranking, candidate caps, Pumas
    fact fallback, compatibility aliases, workflow admission scheduler changes,
    diagnostics-ledger schema/DTO changes, TypeScript mirrors, generated files,
    lockfiles, or workflow fixtures.
  - Verification passed:
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-runtime-registry`, and `cargo fmt --package
    pantograph-runtime-registry`.
  - Deviation fixed: the first focused compile showed explicit override
    matching still needed the shared eligibility predicate. The predicate now
    lives in the runtime-selection policy module and is exposed only
    crate-internally for the technical-fit facade.
  - Remaining follow-up: add internal validated runtime-selection
    input/output types behind the existing serde facade before changing
    candidate synthesis, cross-layer DTOs, scheduler history summaries, or the
    five-run threshold ranking algorithm.
- 2026-05-14 slice: validated internal runtime-selection decision boundary.
  - Smallest useful vertical slice: add internal runtime-selection
    `RuntimeSelectionDecisionInput` and `RuntimeSelectionDecision` wrappers
    behind the existing `RuntimeTechnicalFitRequest`/`RuntimeTechnicalFitDecision`
    serde facade, with a focused guard that rejects unnormalized requests before
    policy execution.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: the public wire DTOs, selector facade,
    explicit override behavior, automatic ranking behavior, candidate
    synthesis, diagnostics-ledger contracts, TypeScript mirrors, generated
    files, lockfiles, and workflow fixtures were not changed. Invalid internal
    policy input now has a typed diagnostic path instead of unchecked policy
    execution.
  - Verification passed:
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-runtime-registry`, and `cargo fmt --package
    pantograph-runtime-registry`.
  - Remaining follow-up: candidate synthesis still needs required Pumas fact
    diagnostics and documented candidate-cap overflow diagnostics before
    cross-layer trace/admission/history work.
- 2026-05-14 slice: generic runtime-capability variant candidate expansion.
  - Smallest useful vertical slice: change the embedded-runtime generic
    runtime-capability technical-fit projection to emit one candidate per
    runtime variant instead of collapsing to the first available or first
    variant before runtime-selection policy can rank candidates.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/technical_fit.rs` and this plan
    directory.
  - No-fallback/no-legacy confirmation: the slice removes the pre-policy
    variant collapse. Unavailable variants are retained as non-selectable
    diagnostic candidates; no Pumas fallback, compatibility shim, public DTO,
    generated file, lockfile, workflow fixture, frontend, or ledger contract
    changed.
  - Verification passed:
    `cargo test -p pantograph-embedded-runtime runtime_request_projection_emits_all_runtime_variant_candidates`,
    `cargo test -p pantograph-embedded-runtime technical_fit`, and `cargo fmt
    --package pantograph-embedded-runtime`; `git diff --check`.
  - Deviation fixed: the first focused compile showed candidate readiness was
    checked after moving variant diagnostics into the candidate. Readiness is
    now computed before candidate construction, and the dead first-variant
    helper was removed with the collapse behavior it encoded.
  - Remaining follow-up: candidate synthesis still needs typed diagnostics for
    required Pumas fact absence and documented candidate-cap overflow before
    append-only trace/admission/history work.
- 2026-05-14 slice: missing required Pumas package-fact diagnostics.
  - Smallest useful vertical slice: add a typed
    `missing_model_package_facts` technical-fit diagnostic, synthesize
    non-selectable candidates for required models whose Pumas package facts
    were unavailable, and have automatic no-valid decisions return scoped
    candidate diagnostics when present.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `src/services/workflow/types.ts`, and this plan directory.
  - No-fallback/no-legacy confirmation: host technical-fit planning now fails
    candidate selection when required model package facts are absent instead of
    selecting from generic runtime capability facts. The slice adds a canonical
    diagnostic and TypeScript mirror value; it does not add compatibility
    aliases, legacy graph behavior, generated files, lockfile changes,
    workflow fixtures, diagnostics-ledger schema changes, or runtime loading.
  - Verification passed:
    `cargo test -p pantograph-runtime-registry selector_surfaces_scoped_candidate_diagnostics_when_no_candidate_is_valid`,
    `cargo test -p pantograph-embedded-runtime missing_required_package_facts_block_capability_only_selection`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`, `npm run
    typecheck`, and `cargo fmt --package pantograph-embedded-runtime --package
    pantograph-runtime-registry --package pantograph-workflow-service`; `git
    diff --check`.
  - Remaining follow-up: candidate synthesis still needs documented
    candidate-cap overflow diagnostics before append-only
    trace/admission/history work.
- 2026-05-14 slice: candidate-synthesis cap overflow diagnostics.
  - Smallest useful vertical slice: enforce a documented embedded-runtime
    technical-fit candidate cap before policy invocation, synthesize a
    non-selectable `candidate_set_overflow` diagnostic candidate when the cap
    is exceeded, and mirror the new diagnostic code through workflow-service
    and frontend workflow types.
  - Allowed write set:
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `src/services/workflow/types.ts`, and this plan directory.
  - No-fallback/no-legacy confirmation: oversized candidate sets now fail
    candidate selection with a typed diagnostic before policy ranking. The
    slice does not truncate candidates, select a generic fallback runtime, add
    compatibility aliases, change generated files, alter lockfiles, update
    workflow fixtures, or touch runtime loading.
  - Verification passed:
    `cargo test -p pantograph-embedded-runtime runtime_request_projection_rejects_candidate_set_overflow`,
    `cargo test -p pantograph-runtime-registry selector_surfaces_scoped_candidate_diagnostics_when_no_candidate_is_valid`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`, `npm run
    typecheck`, and `cargo fmt --package pantograph-embedded-runtime --package
    pantograph-runtime-registry --package pantograph-workflow-service`; `git
    diff --check`.
  - Remaining follow-up: executable candidate facts still need append-only
    trace/admission fields and diagnostics-ledger summary propagation.
- 2026-05-14 slice: scheduler selected device id propagation.
  - Smallest useful vertical slice: carry `selected_device_id` from the
    workflow technical-fit decision into the workflow-session scheduler
    reservation context and copy it into existing scheduler admission and
    reservation-changed diagnostics payload fields.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: selected device id is copied only from
    canonical technical-fit decision facts. The slice does not infer devices
    from raw backend config, runtime strings, graph hints, legacy device
    options, or active backend state, and it does not change ledger schemas,
    TypeScript mirrors, generated files, lockfiles, or workflow fixtures.
  - Verification passed:
    `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
    `cargo test -p pantograph-workflow-service session_execution`, and `cargo
    fmt --package pantograph-workflow-service`; `git diff --check`.
  - Remaining follow-up: selected device class and append-only typed
    runtime-selection trace fields still need a cross-layer DTO slice before
    diagnostics-ledger runtime-selection history work.
- 2026-05-14 slice: typed runtime-selection trace foundation.
  - Smallest useful vertical slice: add append-only typed policy trace fields
    for `policy_phase`, `decision_code`, and `history_threshold_state`, project
    them from runtime-registry through embedded-runtime and workflow-service,
    persist them in diagnostics-ledger scheduler admission payload JSON, and
    mirror them in frontend workflow types.
  - Allowed write set:
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
  - No-fallback/no-legacy confirmation: the slice is append-only trace
    metadata for canonical runtime-selection decisions. It does not query
    ledger history, alter ranking, infer runtime/device choices from legacy
    state, change generated files, alter lockfiles, update workflow fixtures,
    or add compatibility shims.
  - Verification passed:
    `cargo test -p pantograph-runtime-registry technical_fit_decision_normalizes_selected_identifiers`,
    `cargo test -p pantograph-embedded-runtime workflow_decision_projection_preserves_reason_codes`,
    `cargo test -p pantograph-workflow-service workflow_technical_fit_decision_normalizes_selected_backend`,
    `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted_payload_round_trips_policy_trace_contract`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`,
    `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted`,
    `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
    `npm run typecheck`, and `cargo fmt --package
    pantograph-runtime-registry --package pantograph-embedded-runtime --package
    pantograph-workflow-service --package pantograph-diagnostics-ledger`; `git
    diff --check`.
  - Remaining follow-up: selected device class propagation and
    diagnostics-ledger runtime-selection history summaries remain before the
    five-run threshold ranking algorithm.
- 2026-05-14 slice: scheduler selected device class propagation.
  - Smallest useful vertical slice: add optional `selected_device_class` to
    scheduler admission and reservation-changed diagnostics payloads and copy
    it from the canonical workflow technical-fit decision through the existing
    workflow-session reservation context.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: selected device class is copied only
    from canonical technical-fit decision facts and serialized as scheduler
    diagnostic attribution. The slice does not infer device class from raw
    device strings, backend config, runtime ids, graph hints, active backend
    state, or legacy frontend options.
  - Verification passed:
    `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
    `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted_payload_round_trips_policy_trace_contract`,
    `cargo test -p pantograph-workflow-service session_execution`,
    `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted`, and
    `cargo fmt --package pantograph-workflow-service --package
    pantograph-diagnostics-ledger`; `git diff --check`.
  - Remaining follow-up: diagnostics-ledger runtime-selection history
    summaries remain before the five-run threshold ranking algorithm.
- 2026-05-14 slice: diagnostics-ledger runtime-selection history summaries.
  - Smallest useful vertical slice: add a diagnostics-ledger-owned
    `RuntimeSelectionHistorySummary` query contract and SQLite read path over
    `run_list_projection` terminal runs, keyed exactly by workflow identity,
    task id, model id, backend key, runtime variant id, device class, and
    nullable selected device id.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/lib.rs`,
    `crates/pantograph-diagnostics-ledger/src/repository.rs`,
    `crates/pantograph-diagnostics-ledger/src/runtime_selection_history.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/sqlite/runtime_selection_history_sqlite.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the summary query does not call the
    UI-oriented workflow timing expectation API and does not broaden history
    when the five-run threshold is unmet. Every comparable key field is matched
    exactly; `selected_device_id = None` is treated as an exact no-device-id
    key rather than a wildcard.
  - Verification passed:
    `cargo test -p pantograph-diagnostics-ledger runtime_selection_history -- --nocapture`,
    `cargo test -p pantograph-diagnostics-ledger`, and `cargo fmt --package
    pantograph-diagnostics-ledger`.
  - Discovered issue/deferred scope: the existing run projection can provide
    terminal status, execution duration, and derived queue wait for comparable
    runtime-selection history. Canonical load duration, warmup duration, and
    memory/OOM pressure are not yet present in this summary source; future
    ranking must either consume only populated evidence or first add the
    canonical timing/memory producers before weighting those dimensions.
  - Completed follow-up: subsequent 2026-05-14 pure-policy and async-shell
    slices pass exact-key summaries into runtime-selection policy input and
    apply the five-run threshold without broad-history fallback.
- 2026-05-14 slice: pure policy five-run history threshold.
  - Smallest useful vertical slice: add request-provided
    `RuntimeTechnicalFitCandidateHistorySummary` evidence to the pure
    runtime-registry selector and enable history-backed candidate ordering only
    when every eligible candidate has an exact-key summary whose threshold is
    met.
  - Allowed write set:
    `crates/pantograph-runtime-registry/src/lib.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/runtime_selection_policy.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `src/services/workflow/types.ts`, and this plan directory.
  - No-fallback/no-legacy confirmation: the policy consumes only summaries
    supplied in normalized request input. It does not query diagnostics-ledger,
    Pumas, graph internals, frontend state, or runtime loaders; if any eligible
    candidate lacks threshold-met history, historical ranking is disabled and
    the trace records `insufficient_samples` instead of broadening to another
    key.
  - Verification passed:
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-runtime-registry`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`,
    `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted`,
    `npm run typecheck`, `cargo fmt --package pantograph-runtime-registry
    --package pantograph-embedded-runtime --package pantograph-workflow-service
    --package pantograph-diagnostics-ledger`, and `git diff --check`.
  - Completed follow-up: the 2026-05-14 async-shell history gathering slice
    gathers exact-key `RuntimeSelectionHistorySummary` records from
    diagnostics-ledger and projects them into
    `RuntimeTechnicalFitCandidateHistorySummary` by candidate id before policy
    invocation.
- 2026-05-14 slice: async-shell runtime-selection history gathering.
  - Smallest useful vertical slice: expose a workflow-service
    `runtime_selection_history_summary` wrapper over the diagnostics ledger,
    gather exact-key summaries in embedded-runtime after technical-fit
    candidate synthesis, and project successful summaries into
    `RuntimeTechnicalFitCandidateHistorySummary` by candidate id before
    invoking the pure runtime-selection policy.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the pure policy still has no
    diagnostics-ledger dependency. The orchestration shell only queries history
    for candidates with every exact key field: workflow id, task id from the
    backend compatibility report, model id, backend key, runtime variant id,
    device class, and nullable selected device id. It does not broaden missing
    history to another workflow, task, model, runtime, backend, or device key;
    configured-ledger query failures propagate as workflow-service errors.
    Candidates without exact history keys remain facts-only rather than using
    fabricated summaries.
  - Verification passed:
    `cargo test -p pantograph-embedded-runtime runtime_selection_history_summaries_project_exact_candidate_keys`,
    `cargo test -p pantograph-embedded-runtime technical_fit`,
    `cargo test -p pantograph-workflow-service technical_fit`,
    `cargo test -p pantograph-runtime-registry technical_fit`,
    `cargo test -p pantograph-diagnostics-ledger runtime_selection_history`,
    `cargo fmt --package pantograph-embedded-runtime --package
    pantograph-workflow-service -- --check`, and `git diff --check`.
  - Remaining follow-up: keep future ranking dimensions limited to populated
    evidence until canonical load/warmup/memory producers exist.
- 2026-05-12 slice: workflow timing attempt contract.
  - Smallest useful vertical slice: add the workflow-service timing attempt
    contract without wiring existing runtime execution paths yet. The contract
    defines canonical `timing_attempt_` ids, runtime/model load, unload,
    warmup, and scheduler trace attempt kinds, attribution fields, checked
    start/end/duration semantics, and typed timing diagnostics.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/timing_contracts.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/timing_contracts.rs`,
    `crates/pantograph-workflow-service/src/workflow.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests.rs`,
    `crates/pantograph-workflow-service/src/workflow/README.md`, and this
    plan directory.
  - No-fallback/no-legacy confirmation: this is an additive canonical contract
    gate only. It does not preserve or bless the existing saturated runtime
    timing behavior; instead it creates the typed attempt identity and
    diagnostic target required before replacing those producers.
  - Standards/blast-radius gate: workflow-service remains the owner of the
    contract; `contracts.rs` was not expanded because it already exceeds the
    decomposition-review threshold. No diagnostics-ledger schema, generated
    files, frontend code, saved workflow fixtures, lockfiles, Pumas contracts,
    worker contracts, scheduler retry policy, or backend lifecycle behavior
    changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service workflow_timing`,
    `cargo test -p pantograph-workflow-service contracts`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation: the first `cargo fmt --all -- --check` found one
    rustfmt-only line wrap in the new contract module; `cargo fmt --all` was
    applied and focused tests plus final format verification were rerun
    successfully.
  - Remaining follow-up: migrate runtime load/unload, inference warmup,
    embedding warmup, and scheduler trace producers to create timing attempt
    ids, emit attempt records or diagnostics, and use checked duration math.
    Full baseline/deviation enforcement and scheduler retry/termination policy
    remain the later required policy-completion path.
- 2026-05-12 slice: workflow runtime-load timing attempt producer.
  - Smallest useful vertical slice: migrate workflow-session runtime-load
    lifecycle diagnostics onto the timing attempt contract. Load requested,
    dependency-resolved, completed, and failed scheduler model lifecycle events
    now share one `timing_attempt_` id, and runtime-load duration uses checked
    contract arithmetic instead of `saturating_sub`.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`,
    `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/session_runtime_load_lifecycle.rs`,
    `crates/pantograph-workflow-service/src/workflow/session_runtime.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: runtime-load duration is no longer
    anonymously saturated in workflow-session execution. Impossible timing
    state returns a typed workflow-service internal error through the timing
    contract instead of producing a successful lifecycle duration.
  - Standards/blast-radius gate: diagnostics-ledger payload shape changed only
    by adding optional `timing_attempt_id` to scheduler model lifecycle
    payloads; no SQLite schema, generated files, frontend code, saved workflow
    fixtures, lockfiles, Pumas contracts, worker contracts, runtime unload
    policy, inference gateway warmup policy, or scheduler retry policy changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service workflow_execution_session_records_load_completed_only_with_runtime_proof`,
    `cargo test -p pantograph-workflow-service session_execution`,
    `cargo test -p pantograph-diagnostics-ledger model_lifecycle`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Verification deviation/discovered issue: broad session-execution
    verification exposed that
    `workflow_execution_session_run_records_snapshot_before_execution` queried
    Library usage without explicitly refreshing the Library usage projection,
    so the query could observe zero assets when the run emitted more events
    than the previous small projection batch covered. The test now refreshes
    the Library usage projection explicitly before asserting projected assets.
  - Remaining follow-up: migrate runtime unload, capacity-rebalance unload,
    inference gateway warmup, embedding warmup, and scheduler trace producers
    to timing attempt ids and checked duration math. Full baseline/deviation
    enforcement and scheduler retry/termination policy remain the later
    required policy-completion path.
- 2026-05-12 slice: workflow runtime-unload timing attempt producer.
  - Smallest useful vertical slice: migrate the keep-alive-disabled
    workflow-session runtime-unload lifecycle diagnostics onto timing attempt
    ids and checked duration math.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/session_execution.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: keep-alive-disabled runtime unload no
    longer reports an anonymous saturated duration. Unload scheduled, started,
    completed, and failed lifecycle events share one `timing_attempt_` id, and
    impossible timestamp state returns a typed workflow-service internal error
    through the timing contract.
  - Standards/blast-radius gate: workflow-service session execution only; no
    diagnostics-ledger schema, generated files, frontend code, saved workflow
    fixtures, lockfiles, Pumas contracts, worker contracts, capacity-rebalance
    unload policy, inference warmup policy, embedding warmup policy, scheduler
    trace policy, or retry/termination policy changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service workflow_execution_session_run_records_snapshot_before_execution`,
    `cargo test -p pantograph-workflow-service session_execution`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: migrate capacity-rebalance unload, inference gateway
    warmup, embedding warmup, and scheduler trace producers to timing attempt
    ids and checked duration math. Full baseline/deviation enforcement and
    scheduler retry/termination policy remain the later required
    policy-completion path.
- 2026-05-12 slice: capacity-rebalance unload timing attempt producer.
  - Smallest useful vertical slice: migrate workflow-session capacity
    rebalance unload lifecycle diagnostics onto timing attempt ids and checked
    duration math.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/workflow/session_runtime.rs`,
    `crates/pantograph-workflow-service/src/workflow/tests/session_capacity.rs`,
    and this plan directory.
  - No-fallback/no-legacy confirmation: capacity-rebalance unload no longer
    reports an anonymous saturated duration. Scheduled, started, completed,
    and failed rebalance lifecycle events share one `timing_attempt_` id, and
    impossible timestamp state returns a typed workflow-service internal error
    through the timing contract.
  - Standards/blast-radius gate: session runtime capacity-rebalance unload
    only; no diagnostics-ledger schema, generated files, frontend code, saved
    workflow fixtures, lockfiles, Pumas contracts, worker contracts,
    keep-alive unload policy, inference warmup policy, embedding warmup
    policy, scheduler trace policy, or retry/termination policy changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service session_capacity`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: migrate inference gateway warmup, embedding warmup,
    and scheduler trace producers to timing attempt ids and checked duration
    math. Full baseline/deviation enforcement and scheduler retry/termination
    policy remain the later required policy-completion path.
- 2026-05-12 slice: scheduler trace-span timing attempt producer.
  - Smallest useful vertical slice: migrate workflow trace run, node, and
    scheduler queue-wait duration producers onto timing attempt ids and checked
    duration math.
  - Allowed write set:
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
  - No-fallback/no-legacy confirmation: trace run, node, and queue-wait
    durations no longer report saturated values. Timestamp underflow omits the
    duration, attaches a `timing_attempt_` id to the affected trace contract,
    and emits a typed `timestamp_underflow` timing diagnostic instead of
    normalizing the impossible timestamp state.
  - Standards/blast-radius gate: workflow-service trace contracts changed only
    by adding optional timing attempt ids and timing diagnostic arrays to trace
    run, node, and queue DTOs. No diagnostics-ledger schema, generated files,
    frontend code, saved workflow fixtures, lockfiles, Pumas contracts, worker
    contracts, inference gateway warmup policy, embedding warmup policy, or
    retry/termination policy changed.
  - Verification passed:
    `cargo test -p pantograph-workflow-service workflow_trace_store_emits_timing_diagnostic`,
    `cargo test -p pantograph-workflow-service trace::tests`, and
    `cargo test -p pantograph-workflow-service workflow_trace_contract_snapshot`,
    `cargo fmt --all -- --check`, `git diff --check`, and
    `rg -n "saturating_sub" crates/pantograph-workflow-service/src/trace -g '*.rs' -g '!**/tests/**' -g '!**/tests.rs'`
    returned no production trace matches.
  - Verification deviation: one attempted focused test command used multiple
    Cargo test filters and failed at argument parsing before running tests; it
    was rerun with a valid shared filter.
  - Remaining follow-up: migrate inference gateway warmup and embedding
    warmup producers to timing attempt ids and checked duration math. Full
    baseline/deviation enforcement and scheduler retry/termination policy
    remain the later required policy-completion path.
- 2026-05-12 re-plan trigger: remaining inference warmup timing producers
  need shared timing-contract ownership outside workflow-service.
  - Code inspection found the remaining production warmup `saturating_sub`
    sites in `crates/inference/src/gateway.rs` and
    `crates/inference/src/embedding_runtime.rs`. The existing timing attempt
    contract lives in `pantograph-workflow-service`, while `inference` is a
    lower-level runtime crate.
  - Implementation boundary: making `inference` depend on
    `pantograph-workflow-service` would invert the crate layering, and copying
    the timing attempt id/diagnostic structs into `inference` would create a
    second non-canonical contract. Either path violates the no-fallback,
    no-legacy, no-contract-drift rule.
  - Required re-plan decision: choose the shared owner for timing attempt ids,
    checked duration semantics, and timing diagnostics before migrating
    inference warmup producers.
  - Viable option A: create a dedicated shared crate for timing attempts and
    diagnostics, then migrate workflow-service trace/load/unload producers and
    inference warmup producers to that crate.
  - Viable option B: move the timing attempt contract into an existing shared
    foundation crate such as `pantograph-runtime-attribution`, accepting that
    the crate's ownership expands beyond identity/attribution.
  - Rejected option: keep workflow-service as the owner and add an
    `inference` dependency on it. This couples runtime execution to workflow
    orchestration.
  - Rejected option: duplicate the contract in `inference`. This preserves the
    saturated producer migration mechanically but creates contract drift and is
    not canonical.
  - Recommendation: choose option A unless crate-count constraints are more
    important than a crisp ownership boundary. Option A keeps timing policy
    reusable for inference, workflow-service, scheduler diagnostics, and later
    baseline/deviation enforcement without overloading runtime-attribution.
- 2026-05-12 slice: shared timing-contract crate.
  - Smallest useful vertical slice: create `pantograph-timing-contracts` as
    the canonical shared owner for timing attempt ids, checked duration
    semantics, attribution DTOs, and timing diagnostics, then migrate
    workflow-service runtime-load/unload and trace producers to consume that
    crate directly.
  - Allowed write set: root `Cargo.toml`, `Cargo.lock`,
    `crates/pantograph-timing-contracts/**`,
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
  - No-fallback/no-legacy confirmation: the workflow-local timing contract was
    removed rather than preserved as a compatibility shim. Workflow-service now
    imports the shared canonical crate directly, so inference can migrate
    without duplicating timing DTOs or depending on workflow orchestration.
  - Standards/blast-radius gate: new crate has a README and no dependency on
    workflow-service, inference backends, scheduler, Tauri, or
    diagnostics-ledger. This slice does not migrate inference warmup producers,
    generated files, frontend code, saved workflow fixtures, Pumas contracts,
    worker contracts, or scheduler retry/termination policy.
  - Verification passed:
    `cargo test -p pantograph-timing-contracts`,
    `cargo test -p pantograph-workflow-service workflow_trace_store_emits_timing_diagnostic`,
    `cargo test -p pantograph-workflow-service trace::tests`,
    `cargo test -p pantograph-workflow-service session_capacity`,
    `cargo test -p pantograph-workflow-service session_execution`, and
    `cargo fmt --all -- --check`, `cargo check -p inference`, and
    `git diff --check`.
  - Verification deviation/discovered issue: the first
    `cargo test -p pantograph-timing-contracts` compile exposed that the new
    crate's serde contract tests used `serde_json` without declaring it as a
    dev-dependency. Added `serde_json.workspace = true` under
    `[dev-dependencies]` and reran successfully.
  - Remaining follow-up: migrate inference gateway warmup and embedding
    warmup producers onto `pantograph-timing-contracts` timing attempt ids and
    checked duration math. Full baseline/deviation enforcement and scheduler
    retry/termination policy remain the later required policy-completion path.
- 2026-05-12 slice: inference warmup timing attempt producers.
  - Smallest useful vertical slice: migrate inference gateway warmup and
    dedicated embedding runtime warmup producers onto
    `pantograph-timing-contracts` timing attempt ids and checked duration
    math.
  - Allowed write set: `Cargo.lock`, `crates/inference/Cargo.toml`,
    `crates/inference/src/types.rs`, `crates/inference/src/gateway.rs`,
    `crates/inference/src/embedding_runtime.rs`,
    `crates/inference/src/gateway_tests.rs`, and this plan directory.
  - No-fallback/no-legacy confirmation: inference warmup duration producers no
    longer use saturated subtraction. Warmup attempts now create
    `timing_attempt_` ids, successful warmups use checked duration math, and
    impossible timestamp order records typed `runtime_warmup`
    `timestamp_underflow` diagnostics instead of synthesizing a duration.
  - Standards/blast-radius gate: inference depends on the shared
    `pantograph-timing-contracts` crate rather than workflow-service. No
    workflow-service code, diagnostics-ledger schema, generated files,
    frontend code, saved workflow fixtures, Pumas contracts, worker contracts,
    or scheduler retry/termination policy changed.
  - Verification passed:
    `cargo test -p inference runtime_lifecycle_snapshot`,
    `cargo test -p inference test_runtime_lifecycle_snapshot`,
    `cargo check -p inference`, `cargo fmt --all -- --check`,
    `git diff --check`, and
    `rg -n "saturating_sub" crates/inference/src/gateway.rs crates/inference/src/embedding_runtime.rs`
    returned no matches.
  - Verification deviation/discovered issue: the first focused inference
    compile exposed that the gateway helper used
    `Option<InferenceStartRequest>` for saved previous backend config even
    though the field stores `Option<BackendConfig>`. Corrected the helper
    signature and reran focused tests successfully.
  - Remaining follow-up: full baseline/deviation enforcement, scheduler
    reschedule policy, retry exhaustion, and terminal workflow failure
    semantics remain the later required policy-completion path.
- 2026-05-12 slice: scheduler policy trace contract fields.
  - Smallest useful vertical slice: add append-only scheduler policy evidence
    fields and reason codes to backend execution, runtime technical-fit,
    workflow technical-fit, embedded-runtime projection, and TypeScript
    workflow DTOs before changing automatic selection behavior.
  - Allowed write set: `crates/inference/src/device_contracts/*`,
    `crates/inference/tests/device_contracts.rs`,
    `crates/inference/tests/fixtures/device_contracts/backend_execution_decision.json`,
    `crates/pantograph-runtime-registry/src/lib.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit.rs`,
    `crates/pantograph-runtime-registry/src/technical_fit_tests.rs`,
    `crates/pantograph-runtime-registry/tests/technical_fit_contract.rs`,
    `crates/pantograph-runtime-registry/tests/fixtures/technical_fit_contract.json`,
    `crates/pantograph-workflow-service/src/lib.rs`,
    `crates/pantograph-workflow-service/src/technical_fit.rs`,
    focused workflow-service tests, `crates/pantograph-workflow-service/tests/contract.rs`,
    `crates/pantograph-workflow-service/tests/fixtures/technical_fit_contract.json`,
    `crates/pantograph-embedded-runtime/src/technical_fit.rs`,
    `src/services/workflow/types.ts`,
    `src/services/workflow/WorkflowService.commands.test.ts`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: the slice adds optional policy trace
    DTO fields, `automatic_ranking`, and `controlled_exploration` reason
    codes only. It does not make the selector rank equal candidates, explore,
    synthesize candidates, read ledger history, or preserve ambiguity as a
    compatibility shim.
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
  - Verification deviations fixed during the slice: the first workflow-service
    compile exposed the missing local normalization helper for policy trace
    candidate ids; the first embedded-runtime compile exposed missing public
    re-exports for the new trace DTOs; the first runtime/workflow fixture runs
    showed that `None` trace fields serialize by omission rather than JSON
    `null`; the first broader workflow-service test compile found remaining
    test literals that needed `selection_policy_trace: None`.
  - Remaining follow-up: graph-boundary cleanup must remove graph-visible full
    Pumas package-fact wiring before scheduler integration. Runtime selector
    still needs joined executable candidate synthesis, bounded ledger
    summaries, and a pure policy replacement for the temporary
    `ambiguous_auto_resolution` result.
- 2026-05-12 slice: diagnostics-ledger admission policy-trace serde contract.
  - Smallest useful vertical slice: pin the scheduler admission
    `technical_fit_selection_policy_trace` payload shape with an inline serde
    contract test in the diagnostics-ledger crate.
  - Allowed write set:
    `crates/pantograph-diagnostics-ledger/src/tests.rs` and this plan
    directory.
  - No-fallback/no-legacy confirmation: this is contract coverage for
    canonical scheduler admission facts already emitted after technical-fit
    preflight. It does not add aliases, fallback inference, graph-visible Pumas
    fact flow, selector behavior, projections, schema columns, generated
    files, frontend code, worker contracts, lockfiles, or workflow fixtures.
  - Verification passed:
    `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted_payload_round_trips_policy_trace_contract`,
    `cargo test -p pantograph-diagnostics-ledger scheduler_timeline`,
    `cargo check -p pantograph-diagnostics-ledger`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: ledger-history ranking inputs, retry/termination
    policy, and optional compact read-model policy summaries remain later
    scheduler slices.
- 2026-05-12 slice: diagnostics-ledger policy-trace count validation.
  - Smallest useful vertical slice: make scheduler admission policy trace
    candidate summaries fail closed when eligible/rejected counts do not add
    up to the total candidate count, or when listed eligible candidate ids do
    not match the eligible candidate count.
  - Allowed write set: `crates/pantograph-diagnostics-ledger/src/event.rs`,
    `crates/pantograph-diagnostics-ledger/src/tests.rs`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: malformed scheduler policy evidence
    now returns typed diagnostics-ledger validation errors instead of being
    accepted, saturated, normalized, or silently trimmed. Selector policy,
    schema/projections, runtime execution, workflow graph facts, frontend
    behavior, workers, lockfiles, and workflow fixtures are unchanged.
  - Verification passed:
    `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted_rejects_inconsistent_policy_trace_counts`,
    `cargo test -p pantograph-diagnostics-ledger scheduler_run_admitted_payload_round_trips_policy_trace_contract`,
    `cargo check -p pantograph-diagnostics-ledger`,
    `cargo fmt --all -- --check`, and `git diff --check`.
  - Remaining follow-up: runtime-registry still owns canonical candidate
    selection and already uses checked candidate counts before producing these
    payloads. Ledger-history ranking inputs, retry/termination policy, and
    optional compact read-model policy summaries remain later scheduler
    slices.
- 2026-05-12 slice: inference gateway image output estimate overflow.
  - Smallest useful vertical slice: reject image-generation requests whose
    width, height, and image count overflow the conservative RGBA output byte
    estimate before dispatching to the active backend.
  - Allowed write set: `crates/inference/src/gateway.rs`,
    `crates/inference/src/gateway_tests.rs`, and this plan directory.
  - No-fallback/no-legacy confirmation: impossible image output sizes now fail
    with `BackendError::Config` at the gateway boundary instead of reaching
    backend execution or being saturated/clamped. The slice does not add
    semantic caps, alter planner/runtime selection, change worker contracts,
    touch frontend behavior, generated files, lockfiles, or workflow fixtures.
  - Verification passed:
    `cargo test -p inference test_generate_image_rejects_output_byte_estimate_overflow`,
    `cargo test -p inference test_generate_image_rejects_zero`,
    `cargo check -p inference`, `cargo fmt --all -- --check`, and
    `git diff --check`.
  - Remaining follow-up: broader semantic image request limits, context/batch
    limits, byte-range projections, and worker/runtime request fields remain
    open numeric-boundary work.
- 2026-05-15 slice: image planner unsupported option rejection.
  - Smallest useful vertical slice: make the canonical image-generation
    planner reject request fields that the current text-to-image execution
    plan cannot carry (`init_image`, `mask_image`, `strength`, and non-null
    `extra_options`) instead of silently dropping them before worker dispatch.
  - Allowed write set: `crates/inference/src/image_generation_planner.rs`,
    `crates/inference/src/image_generation_planner_tests.rs`, and this plan
    directory.
  - No-fallback/no-legacy confirmation: unsupported img2img/inpaint and
    opaque option fields now fail with typed `UnsupportedOption` planner
    diagnostics. The slice does not add generic Diffusers fallback behavior,
    compatibility shims, worker fields, frontend behavior, generated files,
    lockfiles, persisted schemas, workflow fixtures, or alternate execution
    paths.
  - Verification passed:
    `cargo test -p inference planner_rejects_unsupported_image_options_without_silent_ignore --lib`,
    `cargo check -p inference`, `cargo fmt -p inference`, and
    `git diff --check`.
  - Remaining follow-up: future img2img/inpaint and family-specific opaque
    option support needs explicit family option tables plus worker contract
    fields before these request fields can become executable.
- 2026-05-20 slice: scoped device refresh lifecycle.
  - Smallest useful vertical slice: move the `DeviceConfig.svelte` backend
    device refresh interval behind a tiny scoped lifecycle helper with
    deterministic start, duplicate-start prevention, stop, and restart
    semantics.
  - Allowed write set: `src/components/DeviceConfig.svelte`,
    `src/components/deviceConfigRefreshScope.ts`,
    `src/components/deviceConfigRefreshScope.test.ts`, and this plan file.
  - No-fallback/no-legacy confirmation: the slice only scopes an existing
    frontend refresh loop. It does not synthesize device options, infer runtime
    readiness, preserve frontend-owned fallback devices, or add a secondary
    scheduler/device-selection path.
  - Standards/blast-radius gate: frontend polling remains scoped to the device
    section owner, clears on collapse and unmount, prevents duplicate timers,
    and has deterministic Node tests using injected timer APIs. No generated
    files, lockfiles, workflow fixtures, backend contracts, persisted schemas,
    runtime startup paths, or accessibility surface changed.
  - Verification passed:
    `node --experimental-strip-types --test src/components/deviceConfigRefreshScope.test.ts`,
    `npm run typecheck`, and `git diff --check`.
  - Remaining follow-up: broader frontend runtime/device selector
    accessibility and backend-owned capability-fact presentation rows still
    need explicit reconciliation or focused UI coverage before their checklist
    items can close.
- 2026-05-20 slice: scheduler-owned workflow template runtime selection.
  - Smallest useful vertical slice: remove retired graph-visible `backend_key`
    fields from bundled image-generation/reranker templates and tracked
    image-generation saved workflow examples, then tighten the template tests
    and READMEs around scheduler-owned runtime selection.
  - Allowed write set: `src/templates/workflows/gguf-reranker-workflow.json`,
    `src/templates/workflows/tiny-sd-turbo-text-to-image.json`,
    `src/templates/workflows/README.md`,
    `.pantograph/workflows/tiny-sd-turbo-diffusion.json`,
    `.pantograph/workflows/juggernaut-x-v10-sdxl.json`,
    `.pantograph/workflows/README.md`,
    `src/services/workflow/templateService.test.ts`, and this plan file.
  - No-fallback/no-legacy confirmation: no compatibility alias, migration
    shim, executor fallback, or frontend runtime inference was added. The graph
    examples now carry model reference and task intent only, so scheduler
    policy remains the single runtime-selection authority unless a canonical
    runtime input is explicitly authored in a future workflow.
  - Verification passed:
    `node --experimental-strip-types --test src/services/workflow/templateService.test.ts`,
    `npm run typecheck`,
    `rg -n "backend_key|runtime_hint|resolved_model_source|resolved_model_package_facts" src/templates/workflows .pantograph/workflows/tiny-sd-turbo-diffusion.json .pantograph/workflows/juggernaut-x-v10-sdxl.json -g '*.json'`,
    and `git diff --check`.
  - Remaining follow-up: the broader canonical workflow/fixture checklist item
    remains open until non-image tracked examples and any formal workflow
    schema migration are explicitly reconciled.
- 2026-05-20 slice: path-derived Pumas model inference removal.
  - Smallest useful vertical slice: stop workflow-service capability extraction
    from converting Pumas-looking `model_path`, `entry_path`, or
    `selected_artifact_path` values into required model ids.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/capabilities.rs`,
    `crates/pantograph-workflow-service/src/README.md`, this milestone file,
    and `docs/plans/current-image-generation-graphs/05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the slice removes a legacy inference
    path instead of preserving it. Pantograph no longer derives model identity
    from graph-local filesystem paths; capability extraction consumes explicit
    `model_id` or `pumas_model_ref.model_id` facts only, leaving Pumas as the
    owner of path-to-model interpretation and artifact load-target resolution.
  - Per-slice standards evidence: workflow-service remains the host-agnostic
    capability/preflight owner and does not add dependencies on Pumas
    filesystem state, frontend, Tauri, diagnostics-ledger, generated DTOs,
    lockfiles, persisted schemas, worker execution, or runtime startup paths.
    The slice is synchronous pure JSON inspection with no lifecycle tasks,
    subprocesses, path access, feature changes, frontend accessibility surface,
    or cross-platform cfg. The changed file remains over the decomposition
    threshold; no responsibility was added, and this slice removes helper code.
  - Tests/fixtures: capability unit tests now prove Pumas library paths and
    selected artifact paths are ignored without explicit model identity, while
    nested `pumas_model_ref.model_id` remains accepted as the canonical model
    fact.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-workflow-service --lib capabilities`,
    `cargo check -p pantograph-workflow-service`, and
    `git diff --check -- crates/pantograph-workflow-service/src/capabilities.rs crates/pantograph-workflow-service/src/README.md`.
  - Verification deviation: `cargo test -p pantograph-workflow-service
    capabilities` failed before running this slice's integration path because
    `crates/pantograph-workflow-service/tests/contract.rs` has unrelated
    `RunListProjectionRecord`/`RunDetailProjectionRecord` initializers missing
    `memory_failure_kind`, `observed_peak_ram_bytes`, and
    `observed_peak_vram_bytes`. The focused library capability tests and crate
    check were run successfully for this slice.
  - Remaining follow-up: non-image tracked workflow examples and any formal
    workflow schema migration still need explicit reconciliation before the
    broad workflow/fixture checklist item can close.
- 2026-05-20 slice: workflow diagnostics contract memory-field compile
  unblock.
  - Smallest useful vertical slice: update the workflow-service contract
    snapshots that construct diagnostics-ledger run-list/run-detail projection
    records so they include the current observed memory fields.
  - Allowed write set:
    `crates/pantograph-workflow-service/tests/contract.rs`, this milestone
    file, and
    `docs/plans/current-image-generation-graphs/05-execution-management.md`.
  - No-fallback/no-legacy confirmation: this is a test-contract alignment
    slice only. It does not add compatibility defaults, change projection
    behavior, alter runtime execution, or relax memory diagnostics. The JSON
    snapshots explicitly pin the nullable fields instead of relying on
    unverified omission.
  - Per-slice standards evidence: diagnostics-ledger remains the owner of the
    projection record types; workflow-service tests consume the public contract
    shape. No production code, generated bindings, lockfiles, persisted schema,
    frontend UI, lifecycle tasks, subprocesses, path access, feature flags, or
    platform-specific code changed.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-workflow-service workflow_run_`,
    `cargo test -p pantograph-workflow-service capabilities`,
    `cargo check -p pantograph-workflow-service`, and
    `git diff --check -- crates/pantograph-workflow-service/tests/contract.rs`.
  - Verification deviation: the first attempt used two Cargo test filters in
    one command, which Cargo rejected before tests ran. Verification was rerun
    with the valid single broader `workflow_run_` filter.
  - Remaining follow-up: none for the compile blocker. Broader memory policy
    work remains under the scheduler/resource-observation closeout items.
- 2026-05-20 slice: graph edge-insert retired model fact priority removal.
  - Smallest useful vertical slice: stop workflow-service graph edge-insert
    helper priority from preferring retired `resolved_model_source` and
    `resolved_model_package_facts` ports as model-reference targets.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/graph/connection_insert.rs`,
    `crates/pantograph-workflow-service/src/README.md`, this milestone file,
    and `docs/plans/current-image-generation-graphs/05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the slice removes retired handle names
    from graph-edit helper behavior and adds no alias, migration, package-fact
    port recreation, scheduler bypass, or compatibility route. Canonical
    model-reference priority remains limited to explicit model identity ports
    such as `model_ref` and `pumas_model_ref`.
  - Per-slice standards evidence: workflow-service remains the graph helper
    owner; no public DTO, generated binding, persisted schema, lockfile,
    frontend UI, runtime lifecycle, subprocess, path access, feature flag,
    worker, or platform-specific code changed. A private unit test pins the
    helper ordering behavior.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-workflow-service graph::connection_intent`,
    `cargo check -p pantograph-workflow-service`, and `git diff --check`.
  - Verification deviation: the first format check failed and `cargo fmt
    --all` was run; the format check and focused graph tests passed afterward.
  - Remaining follow-up: KV-cache memory-impact still tracks
    `resolved_model_source` as a graph change signal and needs its own focused
    slice because it affects memory-impact semantics rather than edge-insert
    helper ordering.
- 2026-05-20 slice: KV-cache memory-impact retired source signal removal.
  - Smallest useful vertical slice: stop KV-cache memory-impact classification
    from treating `resolved_model_source` changes on canonical `llm-inference`
    nodes as model identity changes.
  - Allowed write set:
    `crates/pantograph-workflow-service/src/graph/memory_impact.rs`,
    `crates/pantograph-workflow-service/src/README.md`, this milestone file,
    and `docs/plans/current-image-generation-graphs/05-execution-management.md`.
  - No-fallback/no-legacy confirmation: the slice removes one retired graph
    signal from memory-impact semantics and adds no alias, package-fact
    inference, compatibility migration, scheduler bypass, or runtime fallback.
    `resolved_model_source` changes now fall through to ordinary
    tokenizer/config refresh behavior rather than model identity replacement.
  - Per-slice standards evidence: workflow-service remains the graph
    memory-impact owner; no public DTO, generated binding, persisted schema,
    lockfile, frontend UI, runtime lifecycle, subprocess, path access, feature
    flag, worker, or platform-specific code changed. The focused unit test
    pins the exact retired-field classification.
  - Verification passed:
    `cargo fmt --all -- --check`,
    `cargo test -p pantograph-workflow-service graph::memory_impact`,
    `cargo check -p pantograph-workflow-service`, and `git diff --check`.
  - Verification deviation: the first format check failed and `cargo fmt
    --all` was run; the format check and focused memory-impact tests passed
    afterward.
  - Remaining follow-up: `model_path` still participates in memory-impact
    model-change detection. Removing or scoping that field requires a separate
    workflow schema/legacy saved graph ownership decision because existing
    graph persistence tests still preserve legacy model-path data.

**Verification:**

- Unit tests cover parsing and rejection of invalid device policies, device
  ids, and runtime variant ids.
- Serde fixture tests prove Rust, persisted JSON, diagnostics, frontend, and
  worker payload shapes preserve device policy, variant capability, decisions,
  and diagnostics.
- Diagnostics-ledger serde contract tests prove `scheduler.run_admitted`
  policy trace payloads preserve selected runtime/backend facts, candidate set
  summary, ranking reason, exploration reason, and seed basis without relying
  on display text or graph-visible Pumas facts.
- Diagnostics-ledger validation tests prove scheduler admission policy trace
  candidate summary counts fail closed when totals are impossible or when the
  eligible-candidate id list no longer matches the eligible count.
- Adapter-boundary tests prove unknown llama.cpp device strings and malformed
  device ordinals produce diagnostics instead of silently becoming auto or
  device zero.
- Standards gate review is recorded for touched crates/modules and identifies
  no unresolved ownership, lifecycle, persisted-artifact, feature/dependency,
  path/resource, frontend, or test-isolation gaps.
- llama.cpp tests prove CPU, CUDA, macOS Metal if available, and `none` command
  resolution select the intended runtime variant without hidden fallback.
- Linux llama.cpp platform tests prove explicit CUDA command resolution fails
  when the CUDA runtime executable is missing instead of using the CPU
  executable.
- Managed runtime contract tests prove missing CUDA command resolution
  serializes as a typed `missing_runtime_variant` diagnostic for
  `llama_cpp.cuda`.
- Technical-fit and frontend presenter fixture tests prove runtime variant ids
  use canonical dot-shaped ids rather than slash-shaped legacy examples.
- Managed runtime tests prove one llama.cpp release can expose more than one
  installed/readiness variant under one `ManagedBinaryId::LlamaCpp` identity.
- Managed runtime path tests prove the retired `app_data/runtimes` tree is not
  accepted as a fallback source for executable runtime files.
- Managed runtime catalog/projection tests prove catalog versions, installed
  versions, and projected version statuses carry typed `RuntimeVariantId`
  values.
- Managed runtime job tests prove install/progress/history records identify
  their target runtime variant.
- Runtime-load tests prove requested variants are passed explicitly and missing
  requested variant executables fail with typed readiness diagnostics.
- Security tests prove runtime roots, executable paths, dynamic-library paths,
  Pumas package paths, artifact paths, and worker-visible paths cannot escape
  allowed roots.
- Resource tests prove dimensions, token/context limits, byte ranges, output
  size, and memory estimate calculations use checked arithmetic and typed
  failures at boundaries.
- Inference gateway tests prove image-generation output byte estimates fail
  closed on arithmetic overflow before backend dispatch.
- Local service/process tests prove any touched backend listener binds only to
  loopback, enforces bounded connection/request limits, fails readiness
  timeouts explicitly, and shuts down through the lifecycle owner.
- Recovery/idempotency tests prove interrupted managed-runtime variant installs,
  probe refreshes, and projection rebuilds do not leave contradictory selected
  version/variant/readiness state.
- PyTorch worker/probe tests prove `cpu`, `cuda:0`, and macOS `mps` map
  correctly or fail with typed diagnostics.
- Runtime registry technical-fit tests prove selected candidate facts include
  backend id, task/model compatibility, runtime variant, device class, selected
  device id, resource estimates where known, optional observed-throughput
  hints, device diagnostics, and bounded reasons.
- Admission tests prove explicit unavailable devices block execution and auto
  mode records the selected backend, runtime variant, device class, and device
  when available.
- Admission tests prove auto mode selects among multiple valid candidates
  through scheduler policy rather than candidate-id tie breaks or terminal
  ambiguity, records ranking/exploration reasons, and fails with bounded
  diagnostics only when no candidate is valid or policy cannot legally select.
- Scheduler policy unit tests prove candidate hard filters, readiness
  preference, ledger-history preference, no-history controlled exploration,
  resource-pressure rejection, explicit constraint rejection, and policy
  no-decision diagnostics in a synchronous pure policy module.
- Scheduler policy unit tests prove ledger-history preference is gated until
  every valid runtime for the same workflow identity has at least five
  completed runs; below that threshold, valid runtimes are selected through
  current facts and recorded controlled exploration.
- Scheduler boundary unit tests prove the existing technical-fit facade
  delegates to the pure policy module and preserves current automatic
  selection behavior before any history-backed ranking change lands.
- Scheduler contract tests prove typed policy phase, decision code,
  history-threshold state, candidate history summaries, and no-decision
  diagnostics serialize append-only across runtime-registry,
  workflow-service, embedded-runtime, diagnostics-ledger, and TypeScript
  mirrors without replacing typed fields with display strings.
- Diagnostics-ledger tests prove model/runtime history summaries are bounded,
  use isolated SQLite roots, and key observations by typed model/task/runtime/
  device facts rather than display strings.
- Diagnostics-ledger scheduler-history tests prove runtime-specific history is
  not broadened to workflow-level timing samples when scheduler sample counts
  are below threshold.
- Cross-layer fixture tests prove append-only automatic-selection fields on
  `BackendExecutionDecision` and scheduler lifecycle diagnostics deserialize
  through Rust and TypeScript mirrors without frontend ranking or candidate
  fabrication.
- Cross-layer fixture tests prove append-only scheduler policy trace fields
  deserialize through backend execution, runtime technical-fit, workflow
  technical-fit, embedded-runtime projection, and TypeScript workflow mirrors
  without changing selector behavior.
- Workflow template/contract tests prove inference nodes no longer require
  full resolved Pumas package facts as graph-visible edges. Tests should cover
  model reference plus optional runtime/device intent and verify package facts
  are resolved only by the host/planning boundary.
- Candidate synthesis tests prove selectable candidates include model/package,
  backend, runtime variant, device, resource, readiness, and bounded diagnostic
  facts. Missing or stale Pumas lookups must produce typed candidate
  diagnostics and must not silently degrade into capability-only selection.
- Admission tests prove explicit backend overrides fail when incompatible with
  the task/model/platform, including diffusion through llama.cpp and MLX on
  Linux/Windows.
- Technical-fit tests prove conservative fallback and override-fallback
  candidate synthesis are not reachable executable decisions.
- Lifecycle and diagnostics ledger tests prove selected runtime variant,
  selected backend id, device class, selected device id, device diagnostic
  facts, execution duration, terminal status, and artifact output-size facts
  are retained.
- Adapter-boundary tests prove backend adapters return facts to the scheduler
  without ranking candidates across backends.
- Frontend tests prove stale frontend-only device values cannot be submitted
  and device options come from backend capability facts, with accessible
  selectors, keyboard interaction where interactive, and deterministic cleanup
  for subscriptions or scoped polls.
- Frontend Device Configuration tests prove device config submit validation
  requires a backend-confirmed device and no longer creates synthetic fallback
  choices or stale executable-device submissions.
- Inference serde fixture tests prove llama.cpp canonical device inventory
  facts round-trip with backend-local selector attribution, canonical device
  id/class, VRAM facts, and default-empty diagnostics.
- Inference backend tests prove llama.cpp `gpu_layers` stays a backend-local
  runtime setting and is not serialized into canonical cross-backend device
  policy.
- Inference serde fixture tests prove device-resolution requests and
  scheduler-facing backend execution candidates preserve canonical ids, policy
  shape, task/model evidence, resource estimates, and throughput hints.
- Inference serde fixture tests prove resolved device decisions preserve the
  canonical runtime variant and selected device choice consumed by runtime
  load.
- Inference lifecycle tests prove request lifecycle events carry canonical
  `InferenceDeviceId` values and reject legacy backend-local selected-device
  strings during deserialization.
- Inference device-contract tests prove explicit device policy cannot produce
  a selected decision from a mismatched CPU/GPU class or different concrete
  device id.
- Inference and embedded-runtime tests prove runtime fact resolved-device ids
  use canonical `InferenceDeviceId` while preserving serialized string output
  for status and diagnostics consumers.
- Module READMEs now document canonical selected/resolved device id ownership
  and reject raw backend config or backend-local metadata as selected facts.
- Inference runtime-load fixture tests prove dependency-resolved phase records
  carry managed runtime readiness, canonical resolved device decisions, and
  command facts in a stable serde shape.
- PyTorch worker load-contract tests prove Transformers load envelopes carry
  canonical `InferenceDeviceId` values or omit device for backend-local auto,
  and reject legacy backend-local device ids.
- Device-contract tests prove `auto` is reserved for scheduler policy and is
  rejected as a concrete `InferenceDeviceId`; PyTorch worker load tests reject
  explicit `"auto"` device fields.
- PyTorch audio transcription worker tests prove auto device intent is omitted
  in Rust envelopes, explicit `"auto"` and legacy ids fail contract decoding or
  Python worker validation, and omission maps to backend-local `auto` only
  inside the worker adapter.
- PyTorch worker response tests prove selected device facts in loaded-model and
  live-KV responses are canonical `InferenceDeviceId` values and reject
  explicit auto or legacy ids.
- Startup-device intent tests prove scheduler policy, canonical device ids, and
  llama.cpp-local selectors stay in separate typed variants before shared
  startup config is migrated.
- llama.cpp runtime-settings tests prove normalized effective settings store
  `DeviceBackend`, reject canonical ids in the llama.cpp selector namespace,
  and only project to raw `DeviceConfig` at the sidecar DTO boundary.
- Sidecar device config tests prove legacy llama.cpp selector JSON decodes into
  typed `DeviceBackend`, rejects invalid selectors and canonical ids, and
  command/runtime projections cannot carry invalid raw device strings.
- Gateway start-config tests prove typed startup request device intent is
  namespace-checked before backend startup config construction and explicit
  device intent is not silently ignored by external or unsupported backends.
- Backend config and node-engine tests prove shared backend startup config
  carries typed device intent and adapter-local llama.cpp workflow settings are
  parsed before backend config construction.
- PyTorch helper tests prove omitted auto intent remains absent and explicit
  devices remain canonical `InferenceDeviceId` in test-only load args.
- PyTorch direct/package load tests prove executable load APIs accept
  canonical `InferenceDeviceId` values or omitted auto intent, and node-engine
  parser tests reject legacy PyTorch device strings before backend load.
- Embedded-runtime tests prove vLLM CPU/CUDA and MLX Metal roadmap capability
  facts are reported as unavailable typed diagnostics only and do not expose
  execution.
- Embedded-runtime technical-fit tests prove explicit vLLM and MLX roadmap
  backend overrides reject with typed unavailable diagnostics and do not select
  fallback candidates.
- Inference device tests prove llama.cpp `--list-devices` output can still be
  parsed as backend-local `DeviceInfo` and can also project CPU/CUDA into
  canonical inventory facts while unsupported backend-local selectors emit
  typed diagnostics.
- Feature-gated PyTorch tests prove CPU/CUDA and macOS-gated MPS probe facts
  project into canonical runtime variant readiness without executing Python or
  selecting fallback devices.
- Feature/dependency verification proves affected public crates still build
  with default, no-default-features, and all-features modes when runtime
  feature flags or optional dependencies change.
- Cross-layer acceptance tests prove the thinnest useful vertical slice:
  explicit CUDA intent is admitted only when CUDA is ready, records selected
  variant/device facts, and rejects with bounded diagnostics when the CUDA
  variant is missing.

**Status:** In progress. First slice is the device/runtime contract gate.
