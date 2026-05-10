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
- [ ] Add a common backend-adapter capability contract for llama.cpp, PyTorch,
  vLLM, Candle, and future MLX. The contract reports facts and performs
  backend-specific translation; it must not rank candidates across backends or
  own cross-workflow scheduling policy.
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
- [ ] Add a runtime-variant dimension to managed runtime catalog/status state
  without reintroducing duplicate binary-management systems. Keep
  `ManagedBinaryId::LlamaCpp` as the binary-management identity and nest
  `RuntimeVariantId` readiness under it.
- [ ] Include runtime variant id on managed install jobs, retained job
  artifacts, progress snapshots, and install history entries. One active
  managed-binary install job at a time is acceptable initially if the job
  clearly identifies its target variant.
- [ ] Update selected version, selected variant, active job, retained artifact,
  and readiness through one durable state-transition path. Do not split related
  state across independent locks or cancellation points.
- [ ] Model llama.cpp CPU and CUDA builds on Linux/Windows as runtime variants
  for the same release version where artifacts or local installs expose those
  variants.
- [ ] Model llama.cpp Metal builds on macOS only when a Metal-capable runtime
  is available.
- [ ] Make llama.cpp command resolution select an explicit runtime variant
  before constructing `llama-server` arguments.
- [ ] Keep platform-specific executable names, archive names, dynamic-library
  paths, environment variables, and probes inside platform modules or narrow
  platform traits.
- [ ] Validate runtime roots, executable paths, dynamic-library paths, Pumas
  package paths, artifact paths, and worker-visible paths through shared
  allowed-root validation before filesystem or subprocess access.
- [ ] Use checked arithmetic and typed diagnostics for image dimensions, context
  lengths, token limits, batch sizes, memory estimates, output-size
  calculations, and byte ranges that cross IPC, persisted, worker, or runtime
  boundaries.
- [ ] If a touched backend starts or modifies a local service, require loopback
  binding, connection/request limits, readiness/startup/shutdown timeouts, and
  lifecycle-owned shutdown.
- [ ] Remove hidden llama.cpp CPU fallback when CUDA is requested but CUDA
  runtime files are missing. Return a typed device/runtime-variant diagnostic
  instead.
- [ ] Add backend device inventory facts for llama.cpp `--list-devices` and
  preserve existing parsing while moving it behind the canonical device
  contracts.
- [ ] Add PyTorch device probe contract for `cpu` and `cuda` on Linux/Windows,
  plus `mps` on macOS.
- [ ] Add vLLM device capability placeholder facts for CPU and CUDA only. Do
  not implement vLLM execution in this slice.
- [ ] Add Candle capability placeholder facts for CPU, CUDA, and macOS Metal.
  Do not expose Candle image generation until executable Candle support exists.
- [ ] Add future MLX capability facts as macOS-only roadmap facts. MLX must be
  rejected on Linux/Windows if explicitly requested.
- [ ] Add future-support notes for ROCm/HIP, Vulkan, XPU/iGPU, OpenVINO,
  hybrid/offload, remote hardware plugins, and MLX without implementing them.
- [ ] Update runtime registry technical-fit input/output so candidates can
  carry backend id, task/model support, runtime variant, device class,
  selected device id, resource estimates where known, optional observed
  throughput hints, and device diagnostics.
- [ ] Replace `ConservativeFallback`, override-fallback candidate synthesis,
  and any fallback-named executable technical-fit selection with typed
  rejection diagnostics. Missing candidate/runtime state may be reported as
  advisory diagnostics, but it must not select an executable backend.
- [ ] Add device policy intent to workflow/runtime technical-fit requests so
  scheduler admission can reject unavailable explicit devices before backend
  load.
- [ ] Add optional workflow backend/runtime preference intent to technical-fit
  requests. The scheduler may honor it only when the selected backend can
  execute the model/task on the requested platform/device.
- [ ] Reject impossible explicit backend/runtime preferences with bounded
  diagnostics. Examples: llama.cpp for diffusion image generation, MLX on
  Linux/Windows, Candle image generation before executable Candle support, or
  vLLM for unsupported model/task artifacts.
- [ ] Update runtime-load contracts so load readiness consumes a resolved
  device decision rather than inferring from command-line arguments or raw
  backend config strings.
- [ ] Update inference lifecycle events, diagnostics ledger projection, and run
  inspection facts to include selected backend id, selected runtime variant,
  device class, and selected device id.
- [ ] Add scheduler-learning fact fields without implementing learned
  scheduling policy: model id, task kind, selected backend, selected runtime
  variant, selected device class/id, resource estimate when known, execution
  duration, terminal status, and artifact descriptor output-size measures.
- [ ] Keep scheduler-learning facts descriptor-level. Do not require scheduler
  learning to inspect retained artifact bodies.
- [ ] Add lifecycle ownership for device probes, install jobs, progress streams,
  and refresh events. Each background task must have a tracked owner,
  cancellation path, shutdown behavior, and panic/error reporting.
- [ ] Ensure explicit device requests fail when unavailable. Auto mode may
  select a device, but must record the selected runtime variant and selected
  device.
- [ ] Ensure auto mode is a first-class policy, not a fallback. If auto cannot
  resolve exactly one valid backend/runtime/device decision, fail with typed
  diagnostics instead of reusing raw-device defaults or old backend behavior.
- [ ] Keep existing llama.cpp `gpu_layers` as a llama.cpp runtime setting, but
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
- [ ] Replace frontend copy and submit paths that imply `llama-server` owns
  final auto device choice. Frontend may render backend facts and submit user
  intent only.
- [ ] Remove optimistic frontend executable-device state. Frontend may keep
  transient form intent, but displayed runtime/device readiness must come from
  backend-confirmed snapshots.
- [ ] Remove frontend fallback device options such as synthetic CPU-only lists
  after backend device discovery failure. Discovery failures render backend
  diagnostics/unavailable state and cannot create executable choices.
- [ ] Replace or scope polling-heavy frontend refresh paths. Any remaining poll
  must have deterministic teardown and tests.
- [ ] Update canonical workflows and fixtures to the new device policy/runtime
  variant shape. Do not add legacy compatibility shims for old raw-device
  workflow shapes.
- [ ] If runtime feature flags or optional dependencies change, document the
  feature contract and run affected public crates through default,
  no-default-features, and all-features checks.
- [ ] Update relevant module READMEs for runtime variant ownership and device
  policy boundaries.

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
    `crates/inference/src/lib.rs`, `crates/inference/src/README.md`, and this
    plan directory.
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

**Verification:**

- Unit tests cover parsing and rejection of invalid device policies, device
  ids, and runtime variant ids.
- Serde fixture tests prove Rust, persisted JSON, diagnostics, frontend, and
  worker payload shapes preserve device policy, variant capability, decisions,
  and diagnostics.
- Adapter-boundary tests prove unknown llama.cpp device strings and malformed
  device ordinals produce diagnostics instead of silently becoming auto or
  device zero.
- Standards gate review is recorded for touched crates/modules and identifies
  no unresolved ownership, lifecycle, persisted-artifact, feature/dependency,
  path/resource, frontend, or test-isolation gaps.
- llama.cpp tests prove CPU, CUDA, macOS Metal if available, and `none` command
  resolution select the intended runtime variant without hidden fallback.
- Managed runtime tests prove one llama.cpp release can expose more than one
  installed/readiness variant under one `ManagedBinaryId::LlamaCpp` identity.
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
  mode records the selected device.
- Admission tests prove auto mode fails with bounded diagnostics when no
  candidate is valid and does not invoke old raw-device/default-backend paths.
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
- Feature/dependency verification proves affected public crates still build
  with default, no-default-features, and all-features modes when runtime
  feature flags or optional dependencies change.
- Cross-layer acceptance tests prove the thinnest useful vertical slice:
  explicit CUDA intent is admitted only when CUDA is ready, records selected
  variant/device facts, and rejects with bounded diagnostics when the CUDA
  variant is missing.

**Status:** In progress. First slice is the device/runtime contract gate.
