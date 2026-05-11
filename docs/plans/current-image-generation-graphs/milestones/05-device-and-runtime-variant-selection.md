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
- [x] Add Candle capability placeholder facts for CPU, CUDA, and macOS Metal.
  Do not expose Candle image generation until executable Candle support exists.
- [ ] Add future MLX capability facts as macOS-only roadmap facts. MLX must be
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
- [x] Remove frontend fallback device options such as synthetic CPU-only lists
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
