# Device And Runtime Variant Selection

## Objective

Introduce one backend-owned device and runtime-variant selection contract for
Pantograph inference. The contract must let users request a target execution
device while keeping feasibility facts in backend adapters, final ranking in
the scheduler, and runtime-specific command/API translation inside backend
runtime planning.

The design must support:

- CPU-only inference on Linux, Windows, and macOS.
- CUDA GPU selection on Linux and Windows.
- Metal/MPS selection on macOS only.
- Auto mode as a preference only. Auto may choose a device when the user did
  not ask for a concrete device, but it must record the chosen device and must
  not hide failures for explicit user choices.

ROCm/HIP, Vulkan, XPU/iGPU, OpenVINO, remote hardware plugins, and typed hybrid
offload remain future extensions. The contracts should leave room for them,
but Milestone 5 should not implement or require them.

## Backend Support Notes

These are planning facts, not implementation promises. Backend support must be
confirmed by probes during implementation.

| Backend | Device/Runtime Implication |
| ------- | -------------------------- |
| llama.cpp | Initial support is CPU and CUDA on Linux/Windows, plus Metal on macOS when a Metal-capable runtime exists. Official build notes state GPU acceleration can be fully disabled with `--device none`, backend devices can be selected with `--device`, available devices can be listed with `--list-devices`, and multiple backends can often be built into one binary or loaded dynamically. Pantograph still needs runtime variants because prebuilt release assets and local installs may separate CPU, CUDA, and Metal builds. Vulkan, HIP, SYCL/XPU, OpenVINO, and dynamic backend loading are future extensions. |
| PyTorch | Initial support is CPU and CUDA on Linux/Windows, plus MPS on macOS. PyTorch supports other backends such as XPU and ROCm/HIP, but those are future extensions. PyTorch HIP intentionally reuses `torch.cuda` device APIs, so future ROCm support should be capability-driven rather than a raw device-string assumption. |
| vLLM | vLLM is planned around CPU and CUDA first. Official docs also list ROCm, Intel XPU, and hardware plugins, but Pantograph should not expose those until vLLM execution and probes are implemented. |
| Candle | Candle capability facts should initially distinguish CPU, optional CUDA, and optional Metal build-feature readiness. MKL/Accelerate may remain CPU optimization metadata. Candle image generation remains unavailable until executable Candle support exists. |
| MLX | MLX remains roadmap and macOS-focused. Device selection may reserve an Apple accelerator family slot, but this plan does not implement MLX. |

## Backend Adapter And Scheduler Boundary

Every executable backend must sit behind an adapter boundary:

- `llama.cpp`
- `PyTorch` / `Transformers`
- `vLLM`
- `Candle`
- future `MLX`

The adapter boundary is common even when each backend has different executable
shape. Some adapters start a managed local binary, some call a Python worker,
some may call a server process, and some are roadmap-only capability providers.
The common contract is not a lowest-common-denominator runtime API; it is the
place where Pantograph-owned task/model/device semantics are validated and then
translated into backend-specific execution.

Adapters must expose facts and translation, not global scheduling policy:

- supported task kinds, modalities, model artifact families, and package
  source shapes;
- supported device classes and concrete host devices;
- runtime variants, dependency environments, install/readiness state, and
  backend-specific readiness diagnostics;
- static model/package resource estimates when the backend can provide them;
- backend-specific constraints such as llama.cpp GGUF support, PyTorch
  diffusers directory support, vLLM serving constraints, Candle staged support,
  or MLX platform limits;
- execution translation from a validated Pantograph request and scheduler
  decision into backend CLI/API/worker settings;
- selected backend, runtime variant, device class, device id, and observed
  execution facts back into lifecycle events, diagnostics, and ledger records.

The scheduler owns ranking and placement policy:

- choosing among feasible backend/runtime/device candidates;
- respecting explicit client workflow intent when valid and rejecting it when
  invalid;
- RAM/VRAM reservation, concurrent model placement, and model residency;
- deciding whether to place small models on CPU while preserving accelerator
  memory for larger models;
- choosing backend-specific split/offload strategies only after an adapter says
  the backend supports that strategy for the model and device set;
- balancing latency tolerance, throughput, workflow priority, and client
  workload frequency;
- learning from diagnostics-ledger timing data, selected backend/device facts,
  retained artifact output measures, and repeated workflow history.

The inference crate must therefore provide scheduler-facing candidate facts and
execute the selected decision. It must not own cross-workflow resource policy,
queue ordering, fairness, learned throughput models, or residency decisions.
Pumas remains the canonical model/package source, but it does not select
Pantograph backends or devices.

## Transformers-Compatible Canonical Semantics

Pantograph should follow Transformers ecosystem conventions until a backend
boundary requires backend-specific vocabulary:

- task identifiers and modality signatures;
- architecture/model-family names where Pumas facts expose them;
- processor/tokenizer/preprocess/postprocess lifecycle naming;
- generation configuration names such as temperature, top-k/top-p, max tokens,
  seed, guidance, scheduler, width, height, and dtype/precision;
- model source distinctions such as safetensors/diffusers-directory/GGUF.

Backend-specific names stay inside adapters. Examples include llama.cpp
arguments, `torch.device`, vLLM engine/server flags, Candle `Device` values,
and future MLX device names. The scheduler should compare typed Pantograph
facts and should not reason over raw backend command strings.

## Canonical Contracts

Add Pantograph-owned contracts before backend implementation:

| Contract | Responsibility |
| -------- | -------------- |
| `InferenceDevicePolicy` | User intent: `auto`, `cpu`, `device_class`, or concrete `device_id`. This is not backend-specific command-line syntax. Hybrid/offload is reserved for a later plan. |
| `InferenceDeviceClass` | Initial stable classes: `Cpu`, `Cuda`, `Metal`, `Mps`, and `Unknown`. Future-reserved classes: `Rocm`, `Xpu`, `Vulkan`, `OpenVino`, and `Remote`. |
| `InferenceDeviceId` | Validated concrete id with backend display label, class, ordinal if known, and source backend/probe. |
| `RuntimeVariantId` | Stable runtime-artifact identity such as `llama_cpp.cpu`, `llama_cpp.cuda`, `llama_cpp.windows_cpu`, `llama_cpp.windows_cuda`, `llama_cpp.macos_metal`, or `pytorch.cuda`. |
| `RuntimeVariantCapability` | Backend-owned facts for supported device classes, required libraries, package/build source, version, install state, and readiness diagnostics. |
| `DeviceResolutionRequest` | Planner input: backend, task, model facts, user device policy, resource estimate, runtime candidates, and host device inventory. |
| `DeviceResolutionDecision` | Planner output: selected runtime variant, selected device id, selected device class, explicit/auto mode, and bounded reasons. |
| `DeviceResolutionDiagnostic` | Typed failure for unavailable device, missing runtime variant, incompatible backend build, insufficient memory, unsupported precision, unsupported platform, or unsafe/unknown device id. |
| `BackendExecutionCandidate` | Scheduler-facing fact package for one feasible or rejected backend/runtime/device/model/task combination. It includes backend id, task support, model artifact compatibility, runtime variant, device facts, static memory/resource estimates where known, optional observed-throughput hints, and bounded diagnostics. |
| `BackendExecutionDecision` | Scheduler-selected execution choice. It references one candidate, records whether the user requested an explicit backend/device/runtime preference, and carries rejection diagnostics when no candidate can satisfy the request. |
| `BackendStartupDeviceIntent` | Adapter-facing transition contract for startup wiring. It keeps scheduler-facing `InferenceDevicePolicy`, selected canonical `InferenceDeviceId`, and backend-local llama.cpp selectors in separate variants while old shared startup config is migrated. |

The first implementation slice must add these contracts and tests before any
managed-runtime, registry, backend, or frontend behavior is changed. The
existing `crates/inference/src/device.rs` behavior is llama.cpp adapter logic,
not a cross-backend contract; implementation should move or wrap it behind
llama.cpp-specific inventory and command-translation modules instead of
expanding it as the canonical device model.

Shared startup wiring must not infer between device namespaces. During the
migration away from raw `BackendConfig.device`, code that still needs to carry
startup intent should use an explicit intent source: scheduler policy,
canonical selected device id, or llama.cpp-local selector. `auto` is valid as
`InferenceDevicePolicy::Auto` or as a backend-local llama.cpp selector at the
adapter boundary; it is not a concrete `InferenceDeviceId`.

## Design Rules

- No fallback or legacy execution behavior is allowed. Existing old-path
  behavior must be removed or replaced by the canonical contracts in this plan,
  not preserved behind compatibility branches. If canonical planning cannot
  produce a valid backend/runtime/device decision, the run fails with a typed
  diagnostic and the design flaw is fixed in the canonical path.
- Device policy belongs to scheduler/runtime planning, not frontend, Tauri, or
  saved workflow JSON. The inference crate validates and executes selected
  decisions but does not own scheduler ranking or cross-workflow resource
  policy.
- Saved workflows may store user intent such as `device_policy = auto` or
  `device_policy = cuda:0`, backend preference, or latency/throughput intent,
  but not local runtime paths, stale device probe results, or scheduler
  decisions from another host.
- Runtime registry candidates must include backend id, task/model compatibility,
  runtime variant, device capability facts, resource estimates where known, and
  bounded diagnostics before workflow admission selects a candidate.
- Backends are chosen implicitly by default from model/package facts, task
  semantics, available runtime variants, host devices, and scheduler policy.
  Explicit workflow backend/device/runtime requests are allowed only when the
  chosen backend can actually execute the requested model and task on the
  requested platform/device.
- `ManagedBinaryId` must remain the single binary-management identity. Do not
  split it into device-specific ids such as separate CPU/CUDA llama.cpp
  binaries. Instead, llama.cpp managed runtime state must grow a nested
  `RuntimeVariantId` dimension. One release version can have multiple
  installed/readiness states, and command resolution must select the variant
  explicitly.
- Backend-specific device strings are generated only after validation.
- Auto mode records the resolved device and runtime variant. Explicit mode
  fails if the requested target is unavailable.
- Explicit backend overrides fail when the backend cannot run the model/task.
  For example, diffusion image generation cannot route to llama.cpp, MLX cannot
  route on Linux/Windows, and Candle image generation remains unavailable until
  executable Candle loading exists.
- Runtime command resolution must not infer a variant from raw command-line
  arguments. Runtime planning selects a `RuntimeVariantId` first, then backend
  adapters translate the resolved decision into backend command syntax.
- No hidden fallback is allowed anywhere in execution selection. If a user
  explicitly requests CUDA and the CUDA variant or CUDA device is unavailable,
  the run must fail with a typed device diagnostic rather than silently using
  CPU. If auto selection cannot produce a valid candidate, auto also fails; it
  does not reuse old raw-device behavior as a backup.
- Hybrid/offload is not implemented in this milestone. Existing llama.cpp
  `gpu_layers` may continue as a llama.cpp runtime setting, but it must not be
  presented as a general cross-backend hybrid policy until a later plan defines
  that behavior.
- Hybrid placement, CPU/GPU split, and offload semantics are backend-specific
  capabilities reported by adapters. The scheduler may choose such a strategy
  only from typed candidate facts; it must not synthesize backend-specific
  offload flags from generic policy.
- Device diagnostics must be visible in readiness, scheduler/admission,
  lifecycle events, run snapshots, and diagnostics ledger summaries.
- Scheduler decisions must record why the selected backend/runtime/device was
  chosen, including the selected backend id, selected runtime variant, selected
  device class, selected concrete device when available, and bounded rejection
  reasons for stronger candidates or invalid explicit preferences.
- Device probes must be bounded, non-blocking from async request paths, and
  cached through backend-owned snapshots with explicit refresh events.
- Frontend selectors may submit user intent, but executable device strings,
  runtime paths, and local probe results are backend-owned facts.

## Standards Compliance Guardrails

This milestone was checked against:

- `PLAN-STANDARDS.md`
- `ARCHITECTURE-PATTERNS.md`
- `CONCURRENCY-STANDARDS.md`
- `FRONTEND-STANDARDS.md`
- `ACCESSIBILITY-STANDARDS.md`
- `INTEROP-STANDARDS.md`
- `SECURITY-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`
- `CROSS-PLATFORM-STANDARDS.md`
- `TESTING-STANDARDS.md`
- `DOCUMENTATION-STANDARDS.md`
- `languages/rust/RUST-API-STANDARDS.md`
- `languages/rust/RUST-ASYNC-STANDARDS.md`
- `languages/rust/RUST-CROSS-PLATFORM-STANDARDS.md`
- `languages/rust/RUST-SECURITY-STANDARDS.md`
- `languages/rust/RUST-INTEROP-STANDARDS.md`
- `languages/rust/RUST-DEPENDENCY-STANDARDS.md`
- `languages/rust/RUST-TOOLING-STANDARDS.md`

Implementation must preserve these standards constraints:

- **Validated contracts first:** raw strings from workflow JSON, settings, IPC,
  Python workers, subprocess output, and persisted managed-runtime state must be
  parsed once into validated DTOs at the boundary. Internal planning APIs should
  accept typed device policy, device id, class, and runtime variant values.
- **Executable boundary contracts:** DTOs that cross Rust crate, Tauri,
  diagnostics-ledger, frontend, Python worker, or persisted-state boundaries
  must be serde-tested with fixtures. TypeScript shape updates must be verified
  from backend-produced data instead of hand-maintained frontend assumptions.
- **Backend-owned data:** runtime/device capability facts, selected runtime
  variant, selected device id, install state, and probe results are backend
  state. The frontend may own only transient form state before submission and
  must wait for backend-confirmed snapshots before displaying persisted or
  executable facts.
- **Scheduler/inference separation:** backend adapters expose feasibility,
  estimates, diagnostics, and execution translation. Scheduler code owns
  ranking, reservations, learned placement policy, and queue/resource decisions.
  Do not hide scheduling policy in adapter modules or inference request
  normalization.
- **Composition-root runtime ownership:** adapters may describe required
  runtimes and expose narrow startup/load operations, but runtime creation,
  subprocess/server lifecycle, worker pools, and shutdown wiring belong to the
  embedded-runtime composition root or an explicit lifecycle owner. Library/core
  crates must not create global Tokio runtimes or self-wire long-lived
  infrastructure.
- **Sync core, async shell:** parsing, normalization, device resolution,
  readiness decisions, and command construction should stay synchronous and
  unit-testable. Async code should be limited to probes, subprocess execution,
  filesystem/network I/O, event delivery, and lifecycle orchestration.
- **Owned background work:** device probes, install jobs, refresh events, and
  runtime-manager progress streams must have a lifecycle owner, cancellation
  path, bounded concurrency, and deterministic cleanup. No spawned task may be
  fire-and-forget.
- **Lock consistency:** related managed-runtime state for selected version,
  selected variant, active job, retained artifact, and readiness must update
  under one durable transition path. Do not split a state transition across
  unrelated locks or cancellation points unless it is idempotent or
  transactionally recoverable.
- **Cross-platform isolation:** platform-specific runtime asset names,
  executable paths, dynamic-library paths, environment variables, and device
  probes must remain in platform modules or narrow platform traits. Domain
  planning code must not accumulate inline `cfg()` blocks.
- **Security and path safety:** runtime paths and artifact roots must be derived
  from validated managed-runtime state or Pumas package facts. User intent may
  name a device policy, not an executable path, dynamic-library path, or
  arbitrary runtime root.
- **Centralized path and resource validation:** runtime roots, dynamic-library
  paths, Pumas package paths, artifact paths, and worker-visible paths must be
  canonicalized and validated within allowed roots through shared validation
  utilities before filesystem access. Image dimensions, context lengths, token
  limits, batch sizes, memory estimates, output-size math, and byte ranges must
  use checked arithmetic and typed bound failures.
- **Process and network safety:** local backend servers, if introduced or
  modified, must bind only to loopback, define maximum connection/request
  limits, apply startup/readiness/shutdown timeouts, and expose graceful
  shutdown through the lifecycle owner. No backend adapter may start an
  unbounded listener or worker queue.
- **Rust API surface:** new public or cross-crate types should derive or
  implement useful `Debug`, use explicit serde casing, use `#[non_exhaustive]`
  where additive extension is likely, and use `#[must_use]` for validated
  decisions/builders/results that should not be ignored. Public fallible APIs
  must use structured error enums, not `Result<T, String>`, and production
  request/lifecycle paths must not add `unwrap()` or `expect()`.
- **Feature and dependency contracts:** platform-specific or heavy runtime
  dependencies for CUDA, Metal/MPS, vLLM, Candle, MLX, Python workers, or
  managed binaries must be optional or isolated behind documented runtime
  variants/features where the crate boundary exposes them. If feature flags
  change, default, all-features, and no-default-features checks must be planned
  for affected public crates.
- **Interop boundaries:** Tauri IPC DTOs, frontend types, Python worker
  envelopes, subprocess JSON/stdout parsing, and persisted managed-runtime
  state must be validated at their boundary, updated in the same implementation
  slice, and covered by wire-shape tests or fixtures. Worker and subprocess
  interfaces must document init/shutdown and callback/thread expectations.
- **Frontend accessibility and state:** device/runtime selectors and diagnostic
  actions must render backend-owned facts declaratively, use accessible names
  and keyboard interaction where interactive, avoid optimistic backend-owned
  state, and clean up event subscriptions or polling on unmount.
- **Test isolation and recovery:** tests that touch managed runtime state,
  runtime catalog files, sqlite/projections, environment variables, subprocess
  state, temp package roots, ports, or cache directories must use isolated
  roots or explicit serialization. Durable state transitions need replay,
  recovery, and idempotency checks.
- **No fallback or legacy compatibility:** this plan may make breaking changes
  to old saved workflow/runtime-device shapes. When it does, remove stale code
  and update canonical checked-in workflows and fixtures rather than preserving
  compatibility shims. Runtime `fallback`, `legacy`, `migration`,
  `ConservativeFallback`, and old raw-device acceptance paths are removal or
  replacement targets unless they are read-only stale-diagnostic fixtures that
  cannot influence execution.
- **Documentation traceability:** update module READMEs when ownership,
  lifecycle, or structured producer contracts change. Document invariants and
  rejected alternatives, not file listings.

## Implementation Slices

1. **Device Contract Gate**
   - Add canonical device policy, class, id, variant, capability, decision, and
     diagnostic DTOs in the inference crate.
   - Add a standards spot-check note for touched crates/modules before code
     changes begin: crate role, facade boundary, owner for runtime lifecycle,
     persisted artifacts, feature/dependency impact, and test isolation needs.
   - Add strict parsing tests that reject unknown classes, malformed ordinals,
     unsupported current-scope classes, and unsafe backend strings.
   - Add serde roundtrip and fixture tests for every DTO that crosses crate,
     IPC, diagnostics, persisted-state, TypeScript, or worker boundaries.
   - Add `TryFrom`/`FromStr` conversions and structured error enums. Public APIs
     must not return `Result<T, String>` for validated contract failures.
   - Keep future-reserved classes representable as roadmap facts, but do not
     make them executable in this milestone.
   - List every old-path device/backend/runtime behavior that will be removed
     or replaced, including raw `DeviceConfig` execution, `unknown -> auto`,
     malformed ordinal `-> 0`, frontend fallback device options, technical-fit
     conservative fallback execution, and node-engine backend routing branches
     that choose a backend independently of the scheduler decision.

2. **Backend Adapter Boundary**
   - Add one adapter-facing capability contract used by llama.cpp, PyTorch,
     vLLM, Candle, and future MLX.
   - Make each adapter produce scheduler-facing `BackendExecutionCandidate`
     facts for supported, unavailable, and rejected model/task/device
     combinations.
   - Move current llama.cpp device parsing/probing behind llama.cpp-specific
     adapter code.
   - Replace silent `unknown -> auto` and bad ordinal `-> 0` behavior with typed
     diagnostics.
   - Remove old adapter startup/config paths that accept raw device strings as
     executable decisions. Adapters may translate only a validated scheduler
     decision into backend syntax.
   - Keep `gpu_layers` as a llama.cpp setting only. It must not appear in the
     cross-backend device policy.
   - Keep backend-specific string translation in adapter modules. Do not pass
     llama.cpp, PyTorch, or vLLM device strings across planning boundaries.
   - Keep candidate production policy-light. Adapters may report feasibility,
     estimated memory, supported placement strategies, and known observed
     performance hints, but must not rank candidates across backends.
   - Document each adapter's process/thread/lifecycle expectations and ensure
     adapter startup/load functions can be driven by a composition-root owner
     rather than self-spawning untracked work.

3. **Managed Runtime Variant State**
   - Keep one `ManagedBinaryId::LlamaCpp`.
   - Add variant-aware catalog/status/readiness facts under each release
     version.
   - Include variant id on active install jobs, retained job artifacts, and
     installation history so UI progress can distinguish CPU and CUDA work.
   - Serializing install jobs per managed binary is acceptable for the initial
     slice, but job status must identify the target variant.
   - Update selected version, selected variant, active job, retained artifact,
     and readiness through one durable state-transition path with recovery tests
     for interrupted install/probe/update flows.

4. **Runtime Load And Command Resolution**
   - Make runtime load consume a `DeviceResolutionDecision` containing
     `RuntimeVariantId`, `InferenceDeviceClass`, and selected device id.
   - Make llama.cpp platform command resolution receive the selected variant
     explicitly. It must error when the variant executable is missing.
   - Treat macOS Metal as available only when capability probes or runtime
     metadata prove the installed runtime supports it.
   - Keep platform-specific executable, library, environment, and probe behavior
     behind platform modules or traits. Domain/runtime planning must stay
     platform-neutral.
   - Validate selected executable roots, dynamic-library roots, model paths, and
     artifact paths through shared allowed-root validation before command
     construction.
   - Keep local server/process startup bounded by readiness timeouts, loopback
     binding, connection/request limits where applicable, and lifecycle-owned
     shutdown.

5. **Admission, Diagnostics, And Frontend Projection**
   - Add device policy to workflow/runtime technical-fit requests and selected
     variant/device facts to decisions.
   - Add scheduler-facing backend execution candidates to technical-fit output
     so admission can choose the backend/runtime/device combination without
     inspecting backend-specific command strings.
   - Replace executable `ConservativeFallback` and override-fallback behavior
     with typed rejection diagnostics. Advisory diagnostics may describe
     missing candidate data, but they must not select an executable fallback
     candidate.
   - Validate explicit workflow backend/device/runtime preferences against
     candidate facts and fail with bounded diagnostics when the request is
     impossible.
   - Persist selected runtime variant, device class, selected device id, and
     bounded device diagnostics in lifecycle events, diagnostics projections,
     run snapshots, and run inspection.
   - Persist selected backend id and scheduler decision reason summaries in the
     diagnostics ledger so later scheduling work can learn from actual
     backend/device/model outcomes.
   - Update frontend device selectors to render backend-owned capability facts
     and submit only validated policy intent.
   - Remove frontend fallback device lists and optimistic executable-device
     state. Missing backend facts should render diagnostics or unavailable
     state, not synthetic executable options.
   - Verify frontend interactions with accessible selectors and keyboard paths
     for any new runtime/device controls or diagnostics actions.
   - Replace polling-heavy frontend device refresh paths with backend push or
     explicitly scoped refresh ownership where feasible. If a poll remains, it
     must have deterministic teardown and tests.

6. **Scheduler Learning Extension Point**
   - Define the ledger/artifact fields needed by future scheduler learning
     without implementing the optimizer in this plan.
   - Record input model id, task kind, selected backend, selected runtime
     variant, selected device class/id, resource estimate when known, execution
     duration, terminal status, and output-size measures derived from retained
     artifact descriptors.
   - Keep the learned scheduling math out of the inference crate. This slice
     only creates reliable facts for a later scheduler policy implementation.

## Blast Radius Review

The implementation will touch these areas, and each area has an explicit
standards constraint:

| Area | Expected Change | Standards Constraint |
| ---- | --------------- | -------------------- |
| `crates/inference` device contracts | New validated DTOs, parser errors, adapter-local translations | Correct-by-construction types, parse-once boundary, sync core |
| `crates/inference` backend adapters | Common capability/candidate contract for llama.cpp, PyTorch, vLLM, Candle, future MLX | Facts and translation only; no cross-workflow scheduler policy |
| `crates/inference` managed runtime | Variant-aware state, jobs, catalog projection, command resolution | One binary-management source of truth, durable transition ownership, no hidden fallback |
| Backend subprocess/server lifecycle | llama.cpp/vLLM/PyTorch worker startup, readiness, local ports, shutdown, bounded queues | Composition-root ownership, loopback-only listeners, bounded connection/request limits, cancellation and panic reporting |
| `crates/pantograph-runtime-registry` | Technical-fit candidates and decisions include backend/device/variant/resource facts | Backend-owned facts, scheduler-owned ranking, executable contract tests |
| `crates/pantograph-workflow-service` | Admission receives candidate facts, selected backend/device/variant facts, and diagnostics | Cross-layer vertical-slice acceptance test required |
| `crates/pantograph-embedded-runtime` | Bridges workflow, registry, inference, diagnostics, scheduler selection, and run inspection facts | Adapter ownership only; no frontend assumptions or raw backend strings |
| `crates/pantograph-diagnostics-ledger` | Projections persist selected backend, selected variant, device class, device id, diagnostics, duration, and output-size facts | Durable schema/projection recovery and bounded metadata |
| Python worker bridge | PyTorch receives validated explicit device decisions | Worker cannot auto-select a device unless the scheduler decision explicitly resolved auto |
| Frontend services/components | Render backend capability snapshots and submit policy intent | Backend-owned data, no optimistic executable state, cleanup tests for refresh owners |
| Canonical workflows/fixtures | Replace old raw device/runtime shapes | No legacy shims; update canonical artifacts and validation hooks |
| Build features/dependencies | Optional platform/runtime dependency exposure for CUDA, Metal/MPS, vLLM, Candle, Python worker integration | Minimal defaults, documented features, no-default/all-features checks when touched |

## Initial Device Vocabulary

Use a small vocabulary at the Pantograph boundary and backend-specific
translation inside backend adapters:

| Pantograph Class | Example Backend Translation |
| ---------------- | --------------------------- |
| `Cpu` | llama.cpp `--device none`; PyTorch `cpu`; Candle `Device::Cpu`; vLLM CPU runtime. |
| `Cuda` | llama.cpp CUDA variant plus `--device CUDA0`; PyTorch `cuda:0`; vLLM CUDA package/server. |
| `Metal` / `Mps` | macOS only: llama.cpp Metal runtime; PyTorch `mps`; Candle `metal` feature; future MLX. |

Future-reserved classes: `Rocm`, `Xpu`, `Vulkan`, `OpenVino`, `Remote`, and
`Hybrid`.

## Future Support Reservation Notes

These device/runtime families are reserved contract space only. Milestone 5
must not expose them as executable choices, synthetic frontend options, or
implicit scheduler fallbacks:

- **ROCm/HIP:** future PyTorch and vLLM support must be capability-driven.
  PyTorch HIP may reuse CUDA-shaped Python APIs, so Pantograph must not infer
  ROCm availability from raw `cuda` strings or package names.
- **Vulkan:** llama.cpp Vulkan support remains backend-specific runtime
  metadata. Pantograph must not map unsupported Vulkan selectors into `auto`,
  CPU, CUDA, or Metal device facts.
- **XPU/iGPU:** Intel XPU and integrated GPU support require explicit backend
  probe facts and typed device classes before admission may select them.
- **OpenVINO:** OpenVINO remains a future execution-provider/runtime family.
  It must enter through backend adapter facts, not through generic CPU fallback
  or ONNX metadata alone.
- **Hybrid/offload:** CPU/GPU split, layer offload, and memory tiering remain
  backend-specific placement capabilities. The scheduler may choose them only
  from typed candidate facts after a later plan defines the policy.
- **Remote hardware plugins:** remote acceleration requires trust,
  authentication, resource, and lifecycle contracts. It must not reuse local
  device ids or local runtime readiness fields.
- **MLX:** MLX is macOS-focused roadmap support. Until an MLX capability
  provider exists, explicit MLX requests on any platform must be rejected with
  typed diagnostics rather than mapped to MPS, Metal, CPU, or PyTorch.

## Verification

- Unit tests parse and reject invalid `InferenceDevicePolicy`,
  `InferenceDeviceId`, and `RuntimeVariantId` values.
- Serde fixture tests prove Rust, persisted JSON, diagnostics, and frontend
  shapes agree for device policy, runtime variant capability, device decision,
  and device diagnostics.
- Adapter-boundary tests prove llama.cpp parsing no longer accepts unknown
  device strings as auto or malformed ordinals as device zero.
- llama.cpp variant tests prove CPU, CUDA, Metal on macOS if available, and
  explicit `none` select the expected command/runtime variant without fallback.
- Managed runtime tests prove one `ManagedBinaryId::LlamaCpp` release exposes
  variant-specific readiness and install job status.
- Runtime-load tests prove the selected variant is passed explicitly and a
  missing requested variant produces a typed readiness error.
- Path/security tests prove executable, dynamic-library, model, package, and
  artifact paths are rejected outside allowed roots.
- Resource-bound tests prove invalid dimensions, context lengths, token limits,
  memory estimates, output-size calculations, and byte ranges fail through
  checked arithmetic diagnostics rather than overflow or allocation.
- Local process/server tests prove any touched local backend listener binds to
  loopback, enforces bounded connection/request limits, reports readiness
  timeout failures, and shuts down through the lifecycle owner.
- Feature/dependency checks run default, no-default-features, and all-features
  builds for touched public crates when feature flags or optional runtime
  dependencies change.
- PyTorch worker contract tests prove requested `cpu`, `cuda:0`, and macOS
  `mps` map only when available.
- Runtime registry tests prove technical-fit decisions include selected
  backend id, runtime variant, device class, selected device id, resource
  estimates where known, bounded candidate rejection reasons, and bounded
  device diagnostics.
- Admission tests prove explicit unavailable devices block execution with a
  bounded diagnostic.
- Admission tests prove invalid explicit backend overrides block execution,
  including diffusion image generation through llama.cpp and MLX on non-macOS
  hosts.
- Lifecycle/ledger tests prove selected backend id, selected runtime variant,
  device class, selected device id, execution duration, terminal status, and
  artifact output-size facts are recorded.
- Frontend tests prove device choices are rendered from backend capability
  facts, cannot submit stale frontend-only device values, use accessible
  selectors for interactive controls, support keyboard interaction where
  interactive, and clean up subscriptions or scoped polls on unmount.
- Recovery/idempotency tests prove interrupted managed-runtime variant installs,
  probe refreshes, and projection rebuilds do not leave contradictory selected
  version/variant/readiness state.
- Cross-layer acceptance tests prove the thinnest useful slice: a workflow with
  explicit CUDA intent is admitted only when CUDA is ready, records the selected
  variant/device facts, and rejects cleanly when the CUDA variant is missing.

## Re-Plan Triggers

Re-plan before implementation continues if any of these occur:

- A backend requires raw device strings to cross a boundary that was planned as
  typed.
- A runtime variant cannot be modeled under one `ManagedBinaryId` without
  duplicating binary-management ownership.
- A device probe or install job needs long-lived background work without a clear
  lifecycle owner and cancellation path.
- A platform implementation requires scattered inline `cfg()` logic in domain
  or planning modules.
- A frontend flow needs to display or persist executable device/runtime facts
  before the backend confirms them.
- A cross-layer fixture or acceptance test reveals contract drift between Rust,
  TypeScript, persisted JSON, diagnostics, or Python worker payloads.
- Candidate generation starts ranking backends or making cross-workflow
  placement decisions inside an adapter rather than returning facts to the
  scheduler.
- Scheduler learning needs require detailed artifact payload inspection rather
  than descriptor-level output-size facts.
- A backend adapter needs to create a global runtime, untracked task, unbounded
  queue, unbounded listener, or non-loopback local service.
- Runtime/device support requires feature flags or dependencies that cannot
  pass default, no-default-features, and all-features checks for affected public
  crates.

## Sources

- vLLM installation docs list CUDA and CPU support for the initial plan, with
  ROCm, Intel XPU, and hardware plugins reserved for later support.
- llama.cpp build notes describe `--device`, `--list-devices`, multi-backend
  builds, dynamic backend loading, and `--device none`.
- PyTorch docs describe CUDA and MPS for initial support, with HIP/ROCm and XPU
  reserved for later support.
- Candle docs describe CPU, CUDA, and Metal for initial capability facts, with
  MKL/Accelerate and WASM treated as optimization/build metadata.
