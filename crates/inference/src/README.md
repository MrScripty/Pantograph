# crates/inference/src

## Purpose

This directory contains the core inference facade used by Pantograph to talk to
multiple runtime engines through one Rust API. The boundary exists so callers
can depend on stable contracts for chat, embeddings, reranking, and image
generation without depending on backend-specific launch logic or model-family
details.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `backend/` | Backend trait definitions and concrete supported engine adapters such as llama.cpp, Candle, and PyTorch. |
| `device.rs` | Backend-local llama.cpp device inventory parsing, canonical inventory fact projection, and command-selector formatting. |
| `device_contracts/` | Canonical device policy, runtime variant, backend candidate, and selected execution decision DTOs with strict parser/serde validation. |
| `embedding_runtime.rs` | Dedicated llama.cpp embedding runtime lifecycle plus backend-owned coordination for parallel embedding modes. |
| `gateway.rs` | The single entry point that owns the active backend, temporary embedding-mode prepare/restore orchestration, and request forwarding through the frozen contracts. |
| `gateway_tests.rs` | Gateway lifecycle, request forwarding, runtime reuse, embedding prepare/restore, and mock-backend tests extracted from the production gateway facade. |
| `gateway_tests/` | Behavior-focused child modules for oversized gateway test families. |
| `image_generation_planner.rs` | Side-effect-free PyTorch/Diffusers image-generation planner contract that consumes Pumas package facts and scheduler-selected backend/device facts before worker execution. |
| `image_generation_planner_tests.rs` | Focused tests for the side-effect-free image-generation planner contract. |
| `managed_runtime/` | Backend-owned managed binary contracts and orchestration for installable runtime sidecars such as `llama.cpp`, plus temporary adapters into neutral managed-dependency DTOs. |
| `managed_media_dependencies.rs` | Transitional media dependency activation checks, conversion dependency lease plans, holder validation, and attribution-ready lease records for ffmpeg/OIIO/OCIO tooling while lease ownership moves behind the neutral managed-dependency boundary. |
| `managed_redistributables/` | Compatibility re-exports for media redistributable APIs now owned by `pantograph-managed-dependencies`, plus neutral status projection helpers for inference callers. |
| `model_contracts.rs` | Transformers-aligned model/package/task facts, generation defaults, Pumas package-facts summary snapshots, and model-library update feeds consumed by inference without taking runtime-selection policy. |
| `process.rs` | Sidecar process abstraction used by backends that need external runtimes, including the managed-binary launch error tag consumed before backend startup errors are classified. |
| `runtime_load.rs` | Pure runtime-load phase DTOs, command-fact projection, active-runtime descriptors, and managed-runtime readiness errors shared by llama.cpp startup owners. |
| `types.rs` | Shared request/response contracts consumed across backend and host boundaries. |
| `server.rs` | Legacy sidecar/server lifecycle helpers for llama.cpp-style backends. |
| `server_tests.rs` | Crate-local llama.cpp sidecar regression coverage for PID parsing, path scoping, and runtime matching. |
| `kv_cache/` | KV-cache contracts and helpers used by inference-capable hosts. |

## Problem

Pantograph needs one inference-facing crate that can swap execution engines
without forcing the rest of the backend to know whether a request is served by a
local sidecar, a daemon, or an in-process runtime. The same facade now has to
cover GGUF reranking without pretending rerank requests are text-generation
prompts. As Pantograph adds runtime residency and admission policy, this crate
must stay the execution/infrastructure boundary rather than becoming the owner
of application-level scheduler policy.

## Constraints

- The public contract must stay stable enough for multiple hosts to consume.
- Backends have different lifecycle models, so process ownership must be
  abstracted.
- Host process bridges that fail while resolving managed runtime commands must
  tag those failures with the `process` helper so backend startup maps them to
  `BackendError::ManagedBinary` instead of flattening them into a generic
  startup failure.
- Host-managed PID files must remain structured enough to guard stale-process
  cleanup against PID reuse and ownership ambiguity.
- Machine-consumed request/response payloads must preserve semantics across
  process and language boundaries.
- New capability areas such as diffusion and reranking must extend the contract
  additively.
- Pumas model/package facts consumed by this crate must be versioned DTO/API
  projections or fixtures, not Pumas SQLite, `metadata_json`, or search-cache
  internals.
- Pumas package facts are durable producer facts. Pantograph-owned
  technical-fit candidate derivation, live runtime placement, admission,
  loaded-state interpretation, and final backend choice remain outside this
  crate.
- Runtime-residency, admission, and eviction policy must stay outside this
  crate even when gateway lifecycle data becomes richer.
- Media conversion dependency leases must carry stable holder attribution so
  later conversion executors can audit the workflow run, node, port, and
  conversion that held active managed binaries or library artifacts.

## Decision

Use a gateway + backend trait architecture with shared request/response types.
Backends implement a common interface, while the gateway owns lifecycle and
routing. Shared payload types live in `types.rs` so chat, embedding, reranking,
and image-generation contracts stay explicit and testable. Model-library and
Transformers-aligned task/package facts live in `model_contracts.rs` so Pumas
facts, generation defaults, lifecycle phases, package-facts summary snapshots,
and update feeds can be tested before backend execution paths consume them.
llama.cpp reranking is modeled as its own capability and sidecar mode rather
than as a chat completion variant. The planned `RuntimeRegistry` sits above
this crate as a Pantograph application-layer coordinator; `InferenceGateway`
remains the execution facade and lifecycle fact source that the registry
consumes rather than replaces. Managed media dependency planning is currently
transitional in this crate because existing activation and lease state still
live here. Shared managed dependency DTOs now live in
`pantograph-managed-dependencies`, and real media conversion process execution
stays in the neutral `pantograph-media-conversion` boundary and host adapters.
Device and runtime-variant contracts live in `device_contracts/` so
backend adapters can report facts while scheduler admission owns selection.
The contracts reject invalid raw identifiers at the boundary instead of
normalizing them to executable defaults.

## Alternatives Rejected

- Exposing backend-specific request types directly: rejected because it would
  leak infrastructure details into callers and make runtime switching brittle.
- Keeping image generation outside this crate: rejected because diffusion is a
  backend capability and needs the same contract discipline as chat and
  embeddings.

## Invariants

- `InferenceGateway` is the only facade new callers should use for inference.
- New task execution consumers should prefer `InferenceExecutionRequest` and
  `InferenceExecutionResult` over backend-specific request shapes. Legacy
  facade methods remain adapter edges until their callers migrate.
- Typed gateway transport projection resolves model identity in this order:
  explicit `model_name`, explicit `model_ref.model_id`, then
  `resolved_model_package_facts.model_ref.model_id`. Direct typed callers must
  not depend on backend-specific empty-model fallbacks when Pumas package facts
  are available.
- Backend capability flags must reflect contract support, not aspirational
  future support.
- Shared request/response types are append-only unless a coordinated breaking
  change is approved.
- Image-generation execution plans carry explicit denoising scheduler intent as
  `DenoisingSchedulerOptionId`, a bounded lowercase primitive id. Display names
  and Diffusers/Pumas component class names must not be treated as executable
  option ids at the Rust planner boundary.
- Application-level runtime policy such as admission, reservation, retention,
  and eviction must not be implemented inside gateway or backend modules.
- `device_contracts/` owns canonical device policy intent, concrete device
  ids, runtime variant ids, backend candidate facts, and selected execution
  decisions. Backend-specific strings such as llama.cpp `CUDA0` values remain
  adapter-local translation details and must not cross scheduler-facing
  boundaries as trusted internal state.
- `BackendCapabilityFacts.runtime_variants` is the backend-owned capability
  surface for runtime variant readiness facts. It reports support and typed
  unavailability diagnostics only; it must not rank variants or silently turn
  unavailable explicit device requests into CPU/auto execution.
- Auto device policy is a first-class scheduler intent. Invalid explicit
  identifiers, missing candidates, or ambiguous selected candidates must return
  typed diagnostics/errors and must not be converted into `auto`, CPU, or
  device zero.
- Backend-local llama.cpp device selectors in `device.rs` are fallible. Unknown
  selectors and malformed ordinals must be rejected with
  `DeviceBackendParseError` and must not become auto mode or ordinal zero.
- Backend-local selectors project to canonical scheduler facts only after they
  are resolved. `auto` and unsupported backend-local selectors such as Vulkan
  return `DeviceBackendContractError` instead of synthesizing selected device
  facts.
- llama.cpp `--list-devices` output may be parsed into backend-local
  `DeviceInfo` for existing callers or projected into canonical inventory
  facts with typed diagnostics for unsupported backend-local selectors.
- `BackendConfig::default()` carries explicit typed `auto` device policy
  intent. `BackendConfig.device` is a `BackendStartupDeviceIntent`, not a raw
  string; wrong backend namespaces and unresolved explicit policies fail before
  backend startup instead of being normalized to an executable default.
- Runtime-load phase records require a `DeviceResolutionDecision`, so
  dependency resolution cannot emit command facts without the selected runtime
  variant, device class, and selected device id facts.
- Active llama.cpp runtime descriptors expose canonical selected device facts
  only after backend-local selectors parse and project successfully. Unresolved
  `auto` and unsupported backend-local selectors omit selected facts, while
  malformed active device state fails closed without producing a descriptor.
- Request lifecycle events have optional typed `selected_device_class` and
  `selected_device_id` contract fields. Producers must populate them only from
  canonical device facts, not by inferring scheduler decisions from raw backend
  config strings.
- Gateway request lifecycle events source selected device class/id from the
  active llama.cpp runtime descriptor when it carries canonical facts.
  `BackendConfig.device` startup intent by itself is not emitted as a selected
  device.
- Gateway mode-info runtime facts follow the same rule: active resolved device
  fields are populated only from canonical active runtime descriptors, not raw
  backend config strings.
- Reranking mode selection must be explicit; callers must not infer reranker
  support from text-generation readiness.
- Matching llama.cpp sidecar starts should be reused when the requested mode,
  model, multimodal projection, and device config already match the live
  runtime so lifecycle metrics stay backend-owned and authoritative.
- The dedicated parallel embedding runtime is owned by this crate rather than
  by host adapters so lifecycle metrics and reuse decisions stay in one Rust
  backend boundary.
- Temporary embedding-mode switches for workflows or host features must be
  prepared and restored through backend-owned gateway operations rather than
  being orchestrated independently by adapters.
- Stale sidecar cleanup must accept legacy plain-PID files but prefer
  structured PID records containing owner, version, mode, start time, and
  executable facts from the host spawner.
- Product listener paths in this crate are managed sidecars, not in-process
  Rust HTTP servers. llama.cpp inference, embedding, and reranking sidecars
  must bind to the loopback host from `constants::hosts::LOCAL` unless a future
  ADR accepts LAN exposure.
- Pantograph does not currently own a sidecar max-connections policy; that
  remains delegated to the managed runtime. If max-connection limits become a
  product requirement, they need an explicit backend contract instead of a
  hidden adapter flag.
- Listener readiness and health checks are bounded by startup/readiness
  timeouts and HTTP request timeouts. Graceful shutdown is owned by the
  process handle and gateway stop paths, which remove PID records and stop
  managed sidecar processes.
- Llama.cpp sidecar regression coverage stays in `server_tests.rs` so
  `server.rs` remains focused on production lifecycle and HTTP slot behavior.
- Backend parsing and managed-runtime path handling should use standard-library
  helpers such as `strip_prefix`, `Path`, and direct `Path::join` inputs rather
  than manual slicing or temporary string allocations.
- `ModelExecutionDescriptor` remains a compact Pumas execution summary. Rich
  artifact kind, tokenizer/processor, generation default, custom-code, backend
  hint, and package diagnostic evidence belongs in `ResolvedModelPackageFacts`.
- `ModelLoadSecurityPolicy` is the public Rust-owned model loading trust
  contract. It records remote-code, local/offline, cache, auth-token source,
  revision, code-revision, decision id, and accepted custom-code source policy
  without carrying secret token values. Backend-local trust structs must adapt
  from this contract instead of inventing independent policy fields.
- Remote MLX/vLLM search tags are discovery hints only; installed-model
  compatibility must use resolved local package facts plus backend checks.
- PyTorch device probes are host-observed data projected through
  `PyTorchDeviceProbeSnapshot`; CPU/CUDA and macOS MPS readiness facts must be
  reported as runtime variants before scheduler admission consumes them.
- Embeddings are normal task evidence in `model_contracts.rs`; dedicated
  embedding runtime state remains a backend-local residency strategy.
- Candle embedding support is staged behind the optional `backend-candle`
  feature. Its package-source and load-plan helpers consume Pumas package facts
  for HF-compatible safetensors/config/tokenizer directories, but the backend
  remains unavailable for runtime selection until executable Candle model
  loading is implemented.
- Audio transcription now has stable typed request/result DTOs and executable
  typed gateway dispatch. The backend trait exposes a speech-to-text method
  that fails closed by default, while implemented backends opt into the typed
  boundary explicitly.
- Depth estimation has stable typed request/result DTOs matching the task
  registry's roadmap contract, but remains `execution_supported=false` until a
  backend implements the task. It is a serializable contract surface for
  validation, diagnostics, and future backend mapping, not an executable
  fallback through image-generation or video-understanding paths.
- PyTorch audio transcription currently accepts only encoded in-memory audio at
  the backend edge. `audio_ref` resolution remains host-owned so artifact
  lookup and media payload handling do not move into the inference crate.
- Typed audio transcription task-validation and backend-execution lifecycle
  events may carry bounded `audio_ref` artifact references for host-owned
  diagnostics. They must not carry encoded audio bytes, prompt text, generated
  content, local filesystem paths,
  `file://` URLs, or scheduler/runtime selection state.
- Contract-only image understanding, depth estimation, video understanding, and
  multimodal generation requests use the same bounded artifact-ref projection
  for stable `image_ref`, `video_ref`, and multimodal artifact parts during
  task validation, without implying backend execution support.
- Typed text/chat streaming lifecycle events expose preprocessing,
  backend-execution, postprocessing, and result-projection phases around the
  typed request boundary. Legacy raw chat streaming remains a backend-execution
  lifecycle only.
- Gateway lifecycle, request forwarding, runtime reuse, and shared mock-backend
  fixtures stay in `gateway_tests.rs`, while oversized behavior families split
  under `gateway_tests/` so `gateway.rs` remains focused on production gateway
  behavior.
- Media conversion dependency lease holders use
  `workflow_run:{workflow_run_id}/node:{node_id}/port:{port_id}/conversion:{conversion_id}`
  and are validated before any managed redistributable lease is acquired.
- Media conversion dependency plans expose dependency id, active version, lease
  id, holder, install root, and expected files so host-owned conversion code can
  record per-conversion attribution without depending on ambient active-version
  snapshots. New cross-crate managed dependency status and command consumers
  should use `pantograph-managed-dependencies` DTOs.
- Media conversion executable paths must be resolved through the typed managed
  media dependency resolver. Host adapters must not assume that the first
  `expected_files` entry is executable for every dependency; OpenColorIO is a
  native library artifact and must not be launched as a process.

## Revisit Triggers

- A second non-diffusion image-generation family requires materially different
  request semantics.
- Process spawning must support arbitrary commands or per-env interpreter
  selection inside this crate.
- A backend needs streaming image-generation events as a first-class contract.
- Runtime policy ownership moves into this crate instead of a higher Pantograph
  application layer.
- Media conversion dependency planning needs to depend directly on
  `pantograph-media-conversion` instead of staying as a host-mapped managed
  dependency boundary.

## Dependencies

**Internal:** `backend`, `device_contracts`, `embedding_runtime`, `gateway`,
`managed_media_dependencies`, `managed_redistributables`, `model_contracts`,
`process`, `types`, `server`, `kv_cache`.
**External:** `tokio`, `serde`, `reqwest`, `async-trait`, and feature-gated
runtime crates such as Candle or PyO3-backed components.

## Related ADRs

- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Reason: it freezes `InferenceGateway` as the execution facade below the
  planned `RuntimeRegistry` policy layer.
- Revisit trigger: a future ADR changes gateway ownership or introduces a
  breaking facade split.

## Usage Examples

Reason: the examples use Rust `None` values to show omitted optional request
fields explicitly.
Revisit trigger: update these examples when inference request defaults or
optional field semantics change.

```rust
use inference::{BackendConfig, ImageGenerationPlanningInput, InferenceGateway};

async fn run_image_request(
    gateway: &InferenceGateway,
    config: &BackendConfig,
    planning_input: ImageGenerationPlanningInput<'_>,
) {
    gateway.start(config).await.unwrap();
    let _ = gateway.generate_image_from_planning_input(planning_input).await;
}
```

## API Consumer Contract

- Callers talk to `InferenceGateway`, not backend implementations directly.
- `InferenceGateway` does not expose a public raw-backend accessor. Consumers
  must use typed gateway methods, backend capabilities, and lifecycle facts
  instead of mutating backend instances directly.
- Backend startup must happen before inference calls.
- Typed task execution callers pass `InferenceExecutionRequest` values whose
  `task_id`, tagged `input`, Pumas model reference, resolved package facts,
  generation options, and scoped backend extras have already been validated at
  the adapter edge.
- `InferenceExecutionRequest` does not carry backend/runtime preference strings
  such as legacy `runtime_hint`; backend and device selection must come from
  scheduler-facing candidate/decision contracts.
- Typed task execution results return `InferenceExecutionResult` variants plus
  bounded usage, cache-handle, and option-diagnostic facts. Callers should not
  infer scheduler admission, reservation, priority, eviction, or final backend
  selection from these payloads.
- `InferenceRequestLifecycleEventSink` returns a structured sink error when a
  host cannot persist or forward lifecycle diagnostics. Gateway producers log
  sink failures and continue returning the original inference result or error.
- `bounded_inference_artifact_ref` is the shared filter for lifecycle artifact
  references that may reach diagnostics. Producers and host ledger adapters
  should use it for stable refs such as `artifact://...` and drop local
  path-shaped values before they can become durable metadata. Typed text
  generation applies the same rule to cache-handle projection before returning
  non-streaming typed results or lifecycle cache-handle facts.
- Raw `generate_image()` validates request shape but does not dispatch to a
  backend. Image generation must use a validated `ImageGenerationExecutionPlan`
  or `ImageGenerationPlanningInput` built from the request, Pumas package
  facts, and the scheduler-owned backend/runtime/device decision. Streaming
  progress is not yet part of the facade.
- `rerank()` accepts one query plus candidate documents and returns scored,
  ordered results; callers should treat response order, not input order, as
  authoritative.
- Typed text/chat generation maps grouped generation options such as
  `length.max_new_tokens`, `sampling.temperature`, `sampling.top_p`, and
  `sampling.top_k` into the OpenAI-compatible chat edge. Backend-native names
  stay adapter-local.
- Process-backed diffusion loaders may infer narrow bundle-root load overrides
  such as consistent safetensors variants when the diffusers directory layout
  makes them deterministic.
- Unsupported capabilities return backend errors rather than silent no-ops.
- Additive fields may be introduced to request/response structs; existing field
  semantics must remain stable.

## Structured Producer Contract

- `types.rs` defines the stable machine-consumed request and response shapes.
  Its lifecycle sink error contract uses `diagnostics_unavailable` for
  secondary diagnostics failures and must not carry prompt/result payload text.
  It also owns bounded artifact-ref filtering for lifecycle diagnostics so
  gateway producers and host adapters do not duplicate local-path rules.
- `model_contracts.rs` defines additive producer/consumer facts for
  Transformers-aligned package evidence, canonical task ids, generation
  defaults, option compatibility, lifecycle phases, and model-library cache
  invalidation events.
- Task registry entries publish `TaskRequestContract` metadata for canonical
  typed input and result payload families. Consumers should use that contract
  for task/request compatibility checks instead of deriving payload shape from
  backend names or raw task strings.
- Optional fields preserve meaning when omitted; callers may rely on omission as
  “backend default”.
- Cross-boundary task contracts use stable snake_case labels, serde defaults for
  omitted optional collections, and additive unknown-field handling. Python
  Transformers is one backend implementation target for these Rust-owned
  contracts, not the source of the public Pantograph contract.
- `ChatRequest` is an edge DTO; additive fields must be introduced only when
  they preserve typed `GenerationOptions` semantics for gateway consumers.
- `ServerModeInfo` is the backend-owned runtime status contract for GUI and host
  adapters; hosts should consume it directly instead of deriving reduced local
  status shapes. Runtime fact snapshots projected from `ServerModeInfo` may
  include validated explicit non-`auto` `InferenceDeviceId` facts from active
  runtime descriptors without extending `RuntimeLifecycleSnapshot` itself, but
  must not derive resolved devices from raw backend config strings.
- Gateway lifecycle and capability payloads are backend-owned runtime facts; a
  higher Pantograph policy layer may interpret them, but this crate must not
  publish scheduler-policy conclusions as if they were raw backend facts.
- `InferenceGateway::execute_typed_with_lifecycle` records typed task
  validation and backend execution as lifecycle facts for host ledger adapters;
  the inference crate still does not import or write diagnostics-ledger events.
  Completed task-validation events may include bounded backend/model
  compatibility summaries when the typed request carries resolved Pumas package
  facts, and completed backend-execution events may include bounded option
  compatibility diagnostics for generation, embedding, and rerank request
  options plus the canonical task id so host adapters can persist support
  summaries without seeing backend-local payloads. Model package resolution
  lifecycle facts may carry the bounded resolved artifact kind derived from
  Pumas package facts. Lifecycle facts may also carry explicit non-`auto`
  selected device ids from backend start config, bounded usage counts, and
  backend-local cache-handle ids when typed execution results provide them.
  Compatibility issue summaries drop local path-shaped issue paths before
  lifecycle emission when a stable model id is present, while retaining bounded
  relative component paths such as `tokenizer.json`.
  Text generation may receive bounded stream usage and backend-local cache
  handle ids from backend `ChatChunk` metadata. The llama.cpp SSE parser and
  PyTorch worker stream parser both accept usage-only stream chunks and drop
  oversized usage counts before typed lifecycle projection. Embedding execution
  aggregates backend embedding item token counts into bounded prompt/total usage
  summaries without exposing vectors or input text; prompt text, messages,
  generated content, embeddings, tensors, token arrays, Python kwargs, backend
  CLI flags, local paths, raw KV state, and raw process output stay outside
  lifecycle diagnostics.
- `InferenceGateway::stream_typed_text` and
  `InferenceGateway::stream_typed_text_with_lifecycle` keep streaming
  text/chat requests on the same canonical `InferenceExecutionRequest` boundary
  as non-streaming execution while leaving token event shaping to host/node
  layers.
- `ImageGenerationRequest` reserves optional `init_image`, `mask_image`, and
  `strength` for later img2img/inpaint support.
- `RerankRequest`, `RerankResult`, and `RerankResponse` are append-only
  contracts shared across gateway, backend, and host layers.
- `MediaConversionDependencyPlan` and lease-token records are transitional
  append-only managed dependency contracts consumed by host-owned conversion
  executors until lease ownership moves behind `pantograph-managed-dependencies`
  adapters.
- Contract changes that affect persisted consumers or saved workflows must be
  append-only or accompanied by migration guidance.
