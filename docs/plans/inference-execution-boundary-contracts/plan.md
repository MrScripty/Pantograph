# Plan: Inference Execution Boundary Contracts

## Objective

Improve `crates/inference` by making it a Rust-first execution interface over
Pumas-resolved model sources and Hugging Face/Transformers-style model package
and task semantics while keeping backend capability facts, runtime lifecycle
facts, and execution request/result semantics explicit. Pumas is the canonical
model source for Pantograph workflows, including GGUF, HF-compatible
directories, safetensors, diffusers bundles, ONNX, and future model artifact
formats. Transformers is the reference vocabulary for package metadata,
component loading, task ids, processor inputs, and generation semantics; it is
not the model registry authority.

The crate should also stop being the source of truth for general binary and
media dependency management. Inference should consume a neutral Pantograph
managed-dependency boundary for runtime executables such as llama.cpp and future
vLLM-style sidecars, while media conversion consumes the same boundary for
ffmpeg, OIIO, OCIO, and related tools. The crate should retire unnecessary
backend surface area without moving scheduling, runtime admission, runtime
reservation, binary-management policy, model-library policy, or workflow policy
into inference.

The result should let scheduler, runtime registry, workflow service, adapters,
and UI layers consume reliable backend-owned facts while preserving those
higher layers as the owners of "should this run here and now?" decisions. The
central design reference is a Rust contract that can describe Pumas model
references, resolved model package facts, Transformers-compatible task requests,
generation options, tokenizer/processor expectations, and multimodal payloads,
then map those semantics cleanly onto PyTorch/Transformers, vLLM, MLX, Candle,
and llama.cpp/GGUF backends.

The interface should include a strong task registry, a deeper generation
configuration contract, and an explicit `preprocess -> execute -> postprocess`
lifecycle. Those concepts should be exposed as stable Pantograph/Pumas-facing
semantics and compatibility diagnostics, while framework tensors, Python
objects, backend command flags, loaded model handles, logits processors, and
server-specific scheduling details remain hidden inside inference adapters.
Workflow-service and the diagnostics ledger should durably record the
workflow/run-scoped consequences of those facts: selected backend/runtime,
selected device/network node, selected model/task, compatibility outcomes,
option mapping summaries, lifecycle summaries, and canonical error links.
Inference produces bounded facts and typed errors; it must not write directly
to the ledger or store prompt/result payload bodies in diagnostics.

## Scope

### In Scope

- Strengthen `BackendCapabilities` as a raw backend support contract.
- Add or refine runtime lifecycle fact DTOs that report what actually started,
  what was reused, which model/device is active, and why startup failed.
- Add typed execution request/result contracts for generation, embeddings,
  reranking, image generation, and future task-specific execution paths.
- Define the canonical workflow-visible inference node shape that expresses
  Pumas model reference, task kind, runtime hint, modality inputs, task options,
  and capability-gated outputs without encoding backend-specific graph
  contracts.
- Migrate existing saved workflows, templates, graph node descriptors, and
  frontend node assumptions from backend-specific inference nodes into the new
  canonical inference node shape.
- Make Pumas the canonical model source for saved workflow references and
  runtime model resolution; direct raw paths, direct HF repo ids, and direct GGUF
  files remain import/debug/compatibility inputs rather than the preferred saved
  graph contract.
- Define how Pumas `ModelExecutionDescriptor` and package-fact descriptors feed
  the canonical inference request without making inference depend on Pumas as
  its only possible caller.
- Keep detailed Pumas-library changes in
  [pumas-library-plan.md](pumas-library-plan.md), with this plan owning only the
  Pantograph inference consumer boundary and cross-repo fixture expectations.
- Add Rust model-source and model-package contracts aligned with
  Hugging Face/Transformers conventions such as local model directories,
  model ids, config/tokenizer/processor files, generation config, chat
  templates, safetensors, and task/model type metadata.
- Add a strong task registry aligned with Transformers pipeline task ids and
  Pumas modality signatures, including task aliases, input/output modalities,
  result families, support tiers, processor requirements, and backend
  compatibility reporting.
- Expand generation configuration beyond `max_tokens` and `temperature` into
  typed length, sampling, search, cache, output, special-token, stopping, and
  backend-extension option groups.
- Add per-option compatibility diagnostics that distinguish options that were
  honored, mapped, defaulted, ignored, rejected, unsupported, or unavailable
  because of model/backend capability.
- Define how inference compatibility, backend/runtime choice, lifecycle
  summaries, and canonical errors are projected into the existing durable
  diagnostics ledger through workflow-service/node-execution boundaries.
- Define a backend-local task execution boundary that can separate
  preprocessing, execution, and postprocessing without becoming a scheduler.
- Bind the PyTorch backend through Transformers for broad HF-compatible model
  loading, tokenizers/processors, chat templates, generation config, and
  model-specific behavior.
- Define or extract a neutral managed-dependency boundary that becomes the
  single source of truth for managed binaries, runtime sidecars, media tools,
  native artifacts, install state, leases, activation, and command resolution.
- Migrate inference to consume that neutral managed-dependency boundary for
  runtime executables instead of owning binary-management state directly.
- Move ffmpeg, OIIO, OCIO, and related media dependency ownership out of the
  inference crate and into the shared managed-dependency/media-conversion
  boundary.
- Remove Ollama as a first-party inference backend and clean up managed runtime,
  feature, registry, gateway, UI, and documentation references that treat it as
  a supported backend.
- Prepare the Candle backend for real Rust-native execution by consuming shared
  capability, runtime fact, device, and execution contracts.
- Add vLLM as a candidate backend/runtime direction for future planning,
  preferably through a narrow OpenAI-compatible external-runtime boundary
  that consumes the same Pumas-resolved model-source/task contracts rather than
  a broad scheduler-owned integration.
- Record MLX as a macOS-only roadmap candidate, not a near-term implementation
  dependency, with the expectation that it should consume the same Rust
  model-source/task shape where Pumas-resolved HF-style package compatibility
  applies.
- Preserve existing public facade entry points where practical, especially
  `InferenceGateway`.
- Update source READMEs and plan traceability for touched source directories.
- Add tests proving inference emits facts and rejects unsupported work without
  making policy decisions.

### Out of Scope

- Scheduler queueing, priority, admission, fairness, reservation, eviction, or
  cross-node batching policy.
- Runtime registry selection policy such as "best backend for this workflow."
- Pumas model-library indexing, import, deduplication, migration, dependency
  binding, or storage policy.
- User/session workflow orchestration.
- UI-side fallback policy.
- Full Candle parity with Hugging Face Transformers.
- Broad model-family support beyond a narrow first native Candle slice.
- Reimplementing Transformers' Python model zoo in Rust.
- Treating a loaded Python Transformers object as the cross-backend runtime
  representation.
- Immediate vLLM implementation before the backend boundary, neutral managed
  dependency boundary, canonical workflow-node migration, and Ollama removal
  are complete.
- Immediate MLX implementation or non-macOS MLX support.
- Replacing llama.cpp or PyTorch backends.
- Implementing real media conversion process execution inside inference.
- Keeping inference as the owner of ffmpeg, OIIO, OCIO, or other non-inference
  media dependency lifecycle.
- Preserving old backend-specific inference node types as long-term supported
  graph contracts.
- Keeping legacy Ollama workflow execution support.
- Writing directly from `crates/inference` into `pantograph-diagnostics-ledger`.
- Persisting prompts, chat messages, raw media, generated content, embeddings,
  token arrays, logits, tensors, Python kwargs, backend CLI arguments, or
  unbounded stderr/stdout in the diagnostics ledger.

## Inputs

### Problem

Pantograph's inference crate is already the execution infrastructure owner for
multiple backends, managed sidecars, lifecycle facts, and request forwarding.
It currently risks two forms of drift:

- Higher layers may infer backend support or runtime state from backend names,
  UI state, logs, or transport-specific failures.
- Future inference improvements, especially native Candle, vLLM, or MLX work, could
  accidentally duplicate scheduler/runtime-registry policy unless the crate's
  fact-producing role is made explicit.
- The current backend-specific shape does not clearly separate canonical model
  identity, model package semantics, and runtime integration. Pumas should own
  stable model identity and artifact resolution. Supporting Transformers as a
  design reference should clean execution up by making "what task/options are
  requested for this resolved model package" a Rust contract that multiple
  backends can evaluate.
- Ollama adds an additional daemon/model-management layer that Pantograph does
  not want to own as a first-party backend abstraction. Keeping it increases
  backend surface area without improving Pantograph's execution boundary.
- Existing saved workflows and workflow-node descriptors currently encode
  backend-specific inference shapes such as Ollama, llama.cpp, PyTorch,
  embedding, and reranker nodes. The new Transformers-referenced direction
  changes the graph contract, not just the backend implementation. The graph
  contract should converge on Pumas model references rather than raw executable
  paths.
- Removing Ollama without a workflow migration would strand saved workflows.
  The intended behavior is to migrate old Ollama nodes into the canonical
  inference shape with an unresolved Pumas model reference when no replacement
  model is known, not to keep an Ollama node or Ollama execution path alive.
- Binary management is split across inference-owned runtime sidecars,
  inference-owned media redistributables, and an aggregate managed-binaries
  facade. This makes inference look like the owner of all Pantograph-managed
  binaries even when ffmpeg/OIIO/OCIO are artifact conversion dependencies, not
  inference runtimes.
- Embeddings are a normal inference task, but the current dedicated embedding
  runtime path can read like a separate inference system. Its useful behavior is
  backend-local runtime residency for parallel llama.cpp embedding modes.
- KV cache reuse is universal as a contract and graph-visible handle, but not
  universal as bytes. Backend-specific codecs and strict model/runtime
  fingerprints decide executable reuse.
- Current execution DTOs are too shallow to express common generation behavior
  across Transformers, llama.cpp, vLLM, MLX, and Candle. A request should be able
  to say which length, sampling, search, stopping, cache, and output semantics
  are requested, and inference should report how each backend actually handled
  those options.
- Current task semantics are split between backend-specific node types,
  embedding/rerank special cases, and coarse modality flags. A strong task
  registry is needed so Pumas evidence, workflow nodes, backend capabilities,
  processor requirements, and result schemas use the same vocabulary.
- Transformers-style preprocessing and postprocessing are currently implicit in
  backend adapters. Pantograph needs the lifecycle to be explicit for validation
  and diagnostics, while keeping tokenization, tensor construction, framework
  calls, and result decoding hidden inside inference.
- The diagnostics ledger already records scheduler admission, selected runtime,
  selected device/network node, model lifecycle, runtime capability, node
  execution status, and canonical errors. The inference plan should integrate
  with that spine instead of introducing a parallel inference trace store.
- Backend selection needs to be durable enough to answer why a workflow node ran
  on a specific backend/runtime, but it must remain a recorded decision/fact,
  not a mechanism for inference to choose scheduling policy.

The system needs stronger inference-produced contracts that improve objective
reliability while keeping scheduling and workflow policy outside inference.

### Standards Reviewed

- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CODING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/ARCHITECTURE-PATTERNS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CONCURRENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DEPENDENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/SECURITY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/INTEROP-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/LANGUAGE-BINDINGS-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TOOLING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CROSS-PLATFORM-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-API-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-ASYNC-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-DEPENDENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-INTEROP-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-SECURITY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TESTING-STANDARDS.md`

### Standards Compliance Findings

| Area | Finding | Standards Response |
| ---- | ------- | ------------------ |
| Plan artifact layout | The plan spans multiple crates and persisted contracts, so it belongs under `docs/plans/<slug>/` unless implementation produces multi-pass refactor artifacts. | Keep this artifact at `docs/plans/inference-execution-boundary-contracts/plan.md`; re-plan under `docs/refactors/` only if pass findings, worker waves, or coordination ledgers are created. |
| Layer ownership | Pumas, inference, workflow, runtime registry, media conversion, and managed dependencies each own different facts. | Freeze ownership before implementation; forbid scheduler, workflow, Pumas indexing, and media conversion policy from moving into inference. |
| Structured contracts | The work changes machine-consumed DTOs, node descriptors, saved workflow shape, package facts, and backend facts. | Treat affected DTOs as structured producer contracts; update source READMEs and add contract tests for producer-to-consumer projections. |
| Persisted artifacts | Saved workflows, settings, managed runtime state, and possible KV metadata may be affected. | Prefer append-only migrations; preserve graph topology and output bindings; require re-plan before changing persisted KV or managed runtime formats. |
| Dependency ownership | New or moved dependencies must live at the narrowest owner. | Pumas owns model-library/dependency-binding APIs; inference owns backend execution adapters; neutral managed-dependency owns binary/tool command resolution. |
| Security boundaries | Model refs, legacy paths, Python model loading, and `trust_remote_code` cross trust boundaries. | Parse and validate once at boundary; route paths through Pumas/managed-dependency resolution; require explicit custom-code/trust policy before PyTorch/Transformers execution. |
| Interop boundaries | Rust, Python workers, Tauri/TypeScript DTOs, Pumas APIs, and external HTTP runtimes cross process/language boundaries. | Align wire casing/tags, validate incoming JSON, and add round-trip or cross-layer acceptance tests for changed DTOs. |
| Executable contracts | The plan changes persisted workflows, runtime facts, Python worker messages, and host-facing DTOs that can drift independently. | Prefer executable contract fixtures or schemas with decode/normalize tests over compile-time type declarations alone. |
| Diagnostics ledger integration | Existing ledger contracts already own durable run/node/runtime diagnostics and selected runtime facts. | Record inference-related choices and failures through workflow-service/node-execution ledger adapters; keep inference as a fact/error producer only. |
| Feature contracts | Backend features are public crate contract surface and several are expensive or platform-specific. | Require default, no-default-features, all-features, and targeted backend-feature checks for touched crates. |
| Cross-platform gates | MLX is macOS-only and vLLM/PyTorch sidecars may be platform-sensitive. | Keep platform-specific dependencies feature-gated and document cfg/build behavior before implementation. |
| Public API docs | New Rust public contracts will include fallible parsing, validation, and lifecycle behavior. | Require crate-level/docs README updates plus `# Errors`, `# Panics`, and feature docs where Rust public APIs are touched. |
| Testing strategy | Isolated type checks are not enough for the workflow-to-inference migration. | Start with contract tests and at least one vertical slice from saved workflow/Pumas model ref through inference compatibility reporting. |

### Overlapping Constraint Resolution

- Pumas canonical source versus direct model inputs:
  saved workflows should store Pumas model references when resolvable; direct
  raw paths, HF repo ids, and GGUF files remain compatibility/import/debug inputs.
- Transformers vocabulary versus model registry authority:
  Transformers defines package/task/component semantics, while Pumas remains the
  model identity, artifact, dependency-binding, and provenance authority.
- Inference facts versus runtime policy:
  inference may answer whether a backend can execute a resolved request and what
  happened; runtime registry and scheduler decide placement, priority, and
  retention.
- Backend choice versus diagnostics:
  scheduler/runtime registry records selected runtime/device/network-node and
  policy context; inference records compatibility and observed execution facts;
  workflow-service/node execution links those facts into the diagnostics ledger.
- Managed dependency ownership versus backend launch:
  the neutral manager owns install/status/lease/command facts; inference consumes
  resolved runtime commands without owning global binary state.
- Workflow migration versus no legacy execution:
  old node shapes migrate structurally to the canonical node and diagnostics;
  Ollama execution paths are not preserved for compatibility.
- KV cache handles versus portable cache bytes:
  workflow/session layers can carry handles, but backend-specific codecs and
  fingerprints decide executable reuse.

### Constraints

- `crates/inference` remains an infrastructure and execution crate.
- Inference may answer "can this backend execute this request?" and "what
  happened during execution?"
- Inference must not answer "should this workflow/node/session run now?" or
  "which competing request should win?"
- Capability fields describe actual support, not aspirational support.
- Runtime lifecycle facts are raw observations, not scheduler conclusions.
- Binary and dependency lifecycle facts should come from one neutral managed
  dependency owner, not from inference-specific duplicate systems.
- Public request/response DTO changes must be append-only unless a coordinated
  breaking change is explicitly approved.
- Saved workflow model references should use stable Pumas model ids where a
  model can resolve through Pumas. Raw paths may remain only as temporary
  compatibility/import/debug inputs and must not become the preferred persisted
  graph contract.
- Legacy raw paths and external model identifiers must be parsed into validated
  domain types before internal use.
- User-supplied or persisted paths must pass through the repo's centralized path
  validation/resolution boundary; plan implementation must not add ad hoc path
  normalization in backend adapters or UI handlers.
- Transformers-compatible contracts must be expressed as Rust DTOs and
  validated domain types, not as Python objects or raw Python-specific kwargs.
- Python Transformers may be used inside the PyTorch backend, but must not
  become the public inference contract.
- Cross-language and persisted DTOs must have an executable validation path,
  such as serde round-trip fixtures, schema-backed fixtures, or producer/consumer
  decode tests that preserve casing, tags, defaults, and enum semantics.
- Task semantics must use canonical registry entries and validated aliases
  before backend routing; raw task strings should not flow through execution
  internals.
- Generation options must be typed before backend routing and must produce
  compatibility diagnostics for unsupported or lossy mappings.
- Preprocessing and postprocessing phases may be visible as lifecycle facts and
  diagnostics, but their internal artifacts remain backend-local unless a future
  explicit debug/export contract is planned.
- Ledger-bound diagnostics must be bounded metadata and safe identifiers. They
  may include backend/runtime choice, task id, Pumas model id, compatibility
  summaries, option-support summaries, lifecycle phases, durations, usage
  counts, cache-handle ids, artifact refs, and canonical error links.
- Ledger-bound diagnostics must not include prompt bodies, message bodies, raw
  images/audio/video, generated content, embeddings, token arrays, logits,
  tensors, raw Python kwargs, backend command flags, full local paths when a
  stable Pumas/artifact id exists, or unbounded process output.
- `trust_remote_code`, model custom-code requirements, and Python worker imports
  must be explicit security policy decisions, not implicit backend defaults.
- New or moved dependencies must be feature-gated and declared at the narrowest
  crate or package that owns execution.
- Expensive backend families remain feature-gated.
- Optional backend dependencies must not be pulled into default builds unless
  the owning crate's feature contract explicitly accepts that cost.
- Any new dependency with a large transitive tree, external process requirement,
  model-serving runtime, native artifact, or platform-specific build behavior
  requires written justification in the implementation PR or follow-up ADR.
- Backend families that add their own model-management policy must be justified
  against Pantograph's runtime ownership model before being kept or added.
- Saved workflow migrations must preserve graph topology, node ids where
  possible, positions, groups, labels, edge ids, output bindings, and user
  intent while changing the serialized node contract.
- After migration, validation should fail on unresolved Pumas model references,
  unsupported task/runtime combinations, or missing required inputs, not on the
  presence of old backend-specific inference node types.
- Inference must not own ffmpeg, OIIO, OCIO, OpenColorIO activation, or media
  conversion leases after the neutral managed-dependency boundary exists.
- Media conversion may consume managed dependency facts and leases, but
  conversion process execution remains outside inference.
- Embedding requests remain first-class inference execution requests; dedicated
  embedding runtimes are backend-local execution strategy.
- KV cache handles may flow through workflow and session layers, but cache
  compatibility and byte manipulation remain backend/inference concerns.
- Touched source directories must keep README/API/structured producer contracts
  current.
- Touched Rust public APIs use typed enums/newtypes where they prevent
  cross-boundary invalid states.
- Public Rust fallible APIs introduced by the plan must document error behavior,
  avoid `Result<T, String>`, avoid panics in request/lifecycle paths, and keep
  constructors private where validation is required.
- Shared contracts, schemas, fixtures, feature flags, generated bindings,
  lockfiles, and saved-workflow migrations are serial ownership points; parallel
  workers may not edit them without an explicit worker plan update.

### Assumptions

- `InferenceGateway` stays the primary facade for callers.
- Pumas is the canonical model source for Pantograph workflows and model
  selection. It owns stable model ids, artifact storage, entry-path resolution,
  dependency bindings, validation state, and imported/downloaded model
  provenance.
- Runtime registry and workflow service can consume richer facts without
  requiring inference to know their selection policy.
- Existing `RuntimeLifecycleSnapshot` can be evolved or wrapped rather than
  replaced immediately.
- Existing `BackendCapabilities` can be extended append-only.
- The shared representation should identify a canonical Pumas model reference
  first, then carry resolved model artifact facts such as local HF-compatible
  directories, Hugging Face repo provenance, GGUF files, diffusers bundles,
  safetensors, ONNX, and future formats.
- GGUF remains a first-class Pumas-resolved model artifact for llama.cpp even
  though GGUF is not the normal Transformers weight format.
- A Transformers-compatible Rust request shape can map cleanly to vLLM and
  future MLX because those runtimes commonly consume HF-style package metadata,
  even though they do not reuse a loaded Python Transformers model object.
- Ollama removal is acceptable as a planned backend support change, provided
  public docs, feature flags, managed runtime state, and consumers are migrated
  deliberately.
- vLLM is most likely useful as an external or managed server backend with an
  OpenAI-compatible API surface, but it should not pull batching/admission
  policy into inference.
- MLX should stay on the roadmap because it may be valuable for macOS native
  inference, but it is platform-specific and lower priority than Candle/vLLM
  boundary work.
- First native Candle work should target a narrow, measurable capability such
  as embeddings before general chat generation.
- Existing PyTorch and llama.cpp behavior should remain compatible through
  adapter mapping.
- The first version of the strong task registry can cover only tasks needed by
  the planned vertical slices, provided it preserves exact upstream task ids and
  explicit unsupported/roadmap states for neighboring Transformers tasks.
- The first version of generation configuration can expose a stable common
  subset and an advanced namespaced extension point, provided every backend
  reports unsupported or ignored options explicitly.
- Saved workflow and template migration can run through a deterministic graph
  migration layer before node execution, so old workflows can be made editable
  without preserving old runtime paths.
- Old `ollama-inference` nodes can be structurally migrated to a canonical
  chat/generation task with an unresolved Pumas model reference or a
  user-selected replacement Pumas model reference.
- Old `embedding` nodes can be structurally migrated to canonical inference
  nodes with `task_kind = embedding` and no public embedding-mode semantics.
- A neutral managed-dependency crate can serve both runtime sidecars and media
  dependencies without importing scheduler policy.
- `pantograph-media-conversion` is the right consumer-facing boundary for
  ffmpeg/OIIO/OCIO command planning and eventual host process execution.
- Existing inference managed runtime and media redistributable code can be
  migrated or wrapped incrementally before source files are removed.

### Dependencies

- `crates/inference/src/backend/`: backend trait, capabilities, backend config,
  and concrete adapters.
- `crates/inference/src/backend/ollama.rs`: removed retired backend adapter;
  stale behavior now lives in explicit migration guards.
- `crates/inference/src/managed_runtime/ollama_platform/`: removed retired
  managed-runtime platform adapters; serialized `ollama` ids project through
  unsupported compatibility records.
- `crates/inference/src/managed_runtime/`: current runtime sidecar manager that
  should move under or adapt to the neutral managed-dependency boundary.
- `crates/inference/src/managed_redistributables/`: current media dependency
  manager that should move out of inference.
- `crates/inference/src/managed_binaries.rs`: current aggregate facade that
  should become unnecessary once the neutral boundary owns all managed
  dependency status.
- `crates/inference/src/managed_media_dependencies.rs`: current media
  dependency lease planning that should move to the media conversion or neutral
  managed-dependency boundary.
- `crates/inference/src/gateway.rs`: public inference facade and lifecycle
  forwarding.
- `crates/inference/src/types.rs`: machine-consumed request and response DTOs.
- `crates/inference/src/kv_cache/`: persisted KV compatibility and storage
  contracts.
- `crates/inference/src/device.rs`: current llama.cpp-oriented device parsing.
- Any existing crate or module that owns shared DTO/schema definitions for
  Rust/Tauri/TypeScript/Python boundaries; if none is sufficient, Milestone 1
  must choose the narrowest contract home before implementation.
- Saved-workflow fixture and migration-test locations selected by
  `pantograph-workflow-service`, `workflow-nodes`, or node-engine conventions.
- `crates/inference/torch/worker.py` and sibling worker modules: current
  PyTorch execution path that should become the Python Transformers binding
  point behind Rust contracts.
- `crates/pantograph-embedded-runtime/src/runtime_registry*`: runtime registry
  consumers that must remain policy owners.
- `crates/pantograph-workflow-service/`: workflow orchestration and scheduling
  adjacent consumers.
- `crates/pantograph-workflow-service/src/workflow/diagnostic_errors.rs`:
  existing workflow-service diagnostic-error adapter and phase/scope registry
  that should record inference-related canonical errors.
- `crates/pantograph-diagnostics-ledger/`: durable diagnostics event ledger,
  selected runtime/device projection owner, canonical error event owner, and
  typed audit boundary for inference-visible run/node/runtime facts.
- `crates/pantograph-media-conversion/`: preferred contract boundary for
  ffmpeg/OIIO/OCIO conversion planning and eventual process execution.
- `crates/node-engine/src/core_executor/inference_nodes.rs`: workflow node
  execution bridge.
- `crates/workflow-nodes/src/processing/`: graph-visible processing node
  descriptors that must migrate from backend-specific inference nodes to the
  canonical inference contract.
- `crates/pantograph-workflow-service/src/graph/`: likely owner of saved graph
  canonicalization or migration hooks that should update persisted node shapes.
- Frontend node registry/renderers/templates: consumers that must stop treating
  old backend-specific inference node descriptors as the long-term surface.
- `src-tauri/` and frontend runtime/status views: consumers of projected
  backend facts.
- Reference libraries inspected for patterns:
  - `/media/jeremy/OrangeCream/Linux Software/repos/reference/frameworks-libraries/transformers`
  - `/media/jeremy/OrangeCream/Linux Software/repos/reference/frameworks-libraries/candle`
  - `/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library`

### Affected Structured Contracts

- `BackendCapabilities`: backend support facts and loaded-model capability
  facts.
- `BackendInfo`: backend display and automation metadata.
- Backend feature flags, especially `backend-ollama` removal and future
  candidate feature names for vLLM or MLX.
- Managed dependency contracts for runtime sidecars, media tools, native
  artifacts, install state, active/default versions, leases, and resolved
  commands.
- Pumas model reference and model package fact contracts for stable `model_id`,
  resolved artifact kind, executable entry path, storage kind, validation state,
  dependency bindings, provenance, and companion artifacts.
- New inference-side model-source/model-package contracts that consume
  Pumas-resolved facts for HF-compatible directories, Hugging Face provenance,
  GGUF files, diffusers bundles, safetensors, ONNX, and future model artifact
  formats.
- New Pumas-resolved package-facts contract covering artifact kind, entry path,
  companion artifacts, component-file presence, raw upstream task evidence,
  generation defaults, chat template presence, tokenizer/processor facts,
  quantization facts, custom-code/security facts, dependency binding state, and
  provenance.
- Pantograph-owned technical-fit candidate projections derived from Pumas
  package facts, Pumas dependency facts, backend capabilities, runtime
  registry state, and workflow requirements. Pumas must not own candidate
  derivation or exclusion semantics.
- New strong task registry contract covering canonical task ids, task aliases,
  input/output modality signatures, result families, support tiers, processor
  requirements, backend compatibility hooks, and lifecycle expectations.
- New task request contracts aligned with Transformers-style task semantics
  while remaining backend-neutral.
- New generation configuration contracts for length, sampling, search, stopping,
  cache, output/logprob/detail controls, special-token behavior, and
  backend-scoped extension options.
- New option-compatibility diagnostics that report honored, mapped, defaulted,
  ignored, unsupported, rejected, and model/backend-unavailable options.
- New lifecycle diagnostics for model package resolution, task validation,
  preprocessing readiness, backend execution, postprocessing, and result
  projection.
- Diagnostics-ledger projection contracts for inference-related backend choice,
  selected backend family/key where needed, selected runtime/device/network
  node, task id, Pumas model id, compatibility summary, option-support summary,
  lifecycle summary, usage summary, cache/artifact references, and canonical
  error-event links.
- Executable contract fixtures or schemas for Pumas package facts, canonical
  inference node shape, task registry entries, generation options,
  compatibility reports, lifecycle diagnostics, Python worker envelopes, and
  host-facing projections.
- New canonical workflow inference node descriptor, node data schema, port
  contract, settings schema, and migration diagnostics.
- Saved workflow graph/version migration records for old inference node types.
- `ModelExecutionDescriptor`: current compact Pumas execution-facing summary for
  model id, entry path, model/task summary, validation state, storage state,
  backend hints, and dependency resolution. It is not deprecated, but it should
  not be treated as the rich package-facts contract.
- `BackendConfig`: startup inputs that should remain backend-local and
  policy-free.
- `RuntimeLifecycleSnapshot` and any successor runtime fact DTO.
- `ServerModeInfo`: runtime status projection consumed by hosts.
- `ChatRequest`, `ImageGenerationRequest`, `RerankRequest`, and backend
  execution result DTOs.
- KV cache runtime/model fingerprints and executable handles.
- Future Candle-specific model load facts if native Candle execution is added.
- Future vLLM runtime facts if it is introduced as an external or managed HTTP
  runtime.
- Future MLX runtime facts if macOS native support is accepted later.
- Media conversion dependency status projections currently surfaced through
  inference exports.

### Affected Persisted Artifacts

- Managed runtime persisted state may need migration or graceful ignoring for
  previously installed Ollama artifacts.
- Managed runtime and managed redistributable persisted states may need a
  schema migration, adapter shim, or graceful import path into the neutral
  managed-dependency owner.
- Saved settings or workflows that refer to the Ollama backend may need a
  compatibility/migration path.
- Saved workflows and templates that persist raw model paths should migrate to
  Pumas model references where Pumas can resolve the model. Unresolved paths
  should become explicit migration diagnostics rather than remaining canonical
  model identity.
- Saved workflows and templates containing old `ollama-inference`,
  `llamacpp-inference`, `pytorch-inference`, `embedding`, or `reranker` nodes
  require deterministic migration into the canonical inference node shape.
- Saved workflow versions should not continue to persist old backend-specific
  inference node types after migration succeeds.
- KV cache metadata may be affected only if live execution cache contracts add
  append-only compatibility metadata.
- New contract fixtures, schemas, or migration fixtures may become persisted
  test artifacts. They must be versioned, reviewed as structured producers, and
  updated in the same slice as the code that consumes them.
- Diagnostics ledger event/projection schema may need additive fields or event
  payloads for selected backend family/key, inference compatibility summaries,
  option-support summaries, and lifecycle summaries. Reuse existing scheduler,
  runtime capability, model lifecycle, node status, and canonical error events
  where they are sufficient.
- Re-plan before changing persisted KV cache format, managed runtime state, or
  saved workflow request artifacts.

### Concurrency and Lifecycle Review

- Inference owns backend start, stop, health check, request execution, and
  backend-local cleanup.
- Higher layers own runtime admission, reservation, retention, eviction, and
  workflow scheduling.
- Runtime facts must be snapshots emitted after lifecycle transitions, not
  shared mutable state that higher layers mutate.
- Backend start/stop paths must prevent overlapping starts from producing stale
  active runtime facts.
- Request cancellation must define whether preprocessing, backend execution,
  postprocessing, stream cleanup, server leases, and KV-cache handles are
  abandoned, retained, or explicitly released.
- Blocking model load or native Candle work must be isolated from async request
  paths according to Rust async standards.
- Cancellation and cleanup behavior must be explicit for any new background
  tasks, local servers, or native model handles.
- Python workers, vLLM sidecars, llama.cpp processes, and future MLX/Candle
  handles require symmetric init/shutdown ownership and tests for stale process
  or stale runtime fact cleanup where implemented.
- Cross-layer consumers may observe facts, but must not write back scheduler
  conclusions into inference-owned lifecycle state.
- Diagnostics ledger appends remain workflow-service/node-execution concerns.
  Inference adapters return facts and typed errors; they do not depend on ledger
  repositories or mutate durable diagnostic state directly.

### Implementation Strategy

After the boundary vocabulary and Pumas/inference contracts are frozen, execute
implementation as validated vertical slices instead of broad horizontal layer
rewrites. Each slice should pass through the real affected layers, from saved
workflow or node descriptor input through Pumas model reference resolution,
inference compatibility validation, backend mapping, execution or explicit
unsupported diagnostics, and result projection.

For each slice, write or update the externally meaningful acceptance fixture
before implementation. The fixture should fail for the expected contract gap
until the slice is implemented, and it should assert producer input, validation
behavior, compatibility diagnostics, and consumer-visible output rather than
private intermediate implementation details.

The first slice should be the thinnest useful end-to-end path that proves the
new shape. Broaden shared layers only when a neighboring slice requires it. Do
not complete broad Pumas, inference, workflow, frontend, or backend rewrites in
isolation before at least one vertical slice proves that the contracts work
together.

Planned slice order:

1. GGUF text generation:
   Pumas model ref -> resolved GGUF artifact -> canonical inference node ->
   llama.cpp compatibility report -> execution or explicit unsupported
   diagnostic -> typed text result.
2. GGUF embeddings:
   same Pumas and llama.cpp path with `task_kind = embedding`, proving embeddings
   are normal inference tasks while dedicated embedding sidecars remain
   backend-local residency strategy.
3. HF/Transformers text generation:
   Pumas model ref -> HF-compatible package facts -> PyTorch/Transformers backend
   -> generation config mapping -> typed text result.
4. Rerank:
   Pumas model ref -> canonical rerank task -> backend compatibility report ->
   typed rerank result.
5. Multimodal:
   Pumas model ref -> processor/component facts -> image/audio/video content
   blocks -> backend compatibility report -> typed multimodal result or explicit
   unsupported diagnostics.

Each slice must include:
- A named owner for shared contracts, schemas, feature flags, generated
  bindings, lockfiles, fixtures, and saved-workflow migrations touched by the
  slice.
- A diagnostics-ledger mapping decision for selected backend/runtime, selected
  model/task, compatibility summary, lifecycle summary, option-support summary,
  usage/cache/artifact references, and canonical error links.
- A contract or fixture for the Pumas-resolved model facts used by the slice.
- A saved-workflow or canonical-node fixture where graph shape is affected.
- A task registry entry and task-alias normalization path for the requested
  task.
- A generation/task option fixture that proves default merging and unsupported
  option diagnostics for that slice.
- A visible lifecycle diagnostic path for package resolution, task validation,
  preprocessing, backend execution, postprocessing, and result projection.
- Backend compatibility verification before execution.
- Failure diagnostics for unresolved model refs, unsupported backend/task pairs,
  and unsupported options.
- A test path proving failed preflight or execution preserves the original
  inference error while recording either a canonical diagnostics event id or an
  explicit `diagnostics_unavailable` link.
- Contract decode/normalize tests for any wire or persisted shape crossing
  Rust/Python, Rust/Tauri/TypeScript, Pumas/inference, or external HTTP runtime
  boundaries.
- Source README and structured producer contract updates for touched
  directories.
- Slice-specific verification plus any cross-layer acceptance test required by
  `TESTING-STANDARDS.md`.

### Public Facade Preservation Note

Use facade-first preservation. Keep `InferenceGateway` and existing public DTOs
working while adding new typed contracts behind or beside them.

Breaking API changes require a re-plan that lists affected crates, bindings,
host DTOs, migration steps, and feature-flag compatibility checks.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Inference grows runtime selection policy while adding richer facts | High | Add explicit out-of-scope tests and README text. Keep policy fields out of inference DTOs. |
| Inference or workflows treat raw paths as canonical model identity | High | Make Pumas model ids the preferred saved graph contract and resolve entry paths at execution/preflight time. |
| Transformers becomes a competing model registry | High | Use Transformers as package/task vocabulary only; Pumas remains the canonical model source and artifact resolver. |
| Pumas metadata and inference package facts drift | High | Add explicit Pumas-to-inference projection contracts and tests for model id, artifact kind, task id, modalities, backend hints, and dependency bindings. |
| Pantograph expects Pumas to derive technical-fit candidates | High | Keep Pumas as the package/dependency fact producer and derive candidate/exclusion semantics inside Pantograph from Pumas facts plus runtime registry and workflow context. |
| Pantograph model-list views serve stale Pumas package facts | Medium | Cache details locally for responsiveness, but refresh by Pumas model-library update events or cursors for added, removed, modified, invalidated, or dependency-changed models. |
| Pantograph depends on Pumas SQLite or `models.metadata_json` internals | High | Consume only versioned Pumas DTO/API outputs and fixture contracts; add model-list/preflight tests that do not inspect Pumas storage internals. |
| Pantograph treats remote HF MLX/vLLM tags as installed-model compatibility | Medium | Treat remote search tags as discovery hints; installed compatibility requires resolved local Pumas package facts plus Pantograph inference/backend checks. |
| Executable contract fixtures fall behind DTO changes | High | Own fixtures in the same slice as DTO changes and require decode/normalize tests before implementation is considered complete. |
| Cross-language casing, tagging, or default semantics drift | High | Add serde/wire-shape tests and host-side binding checks for Rust/Python/Tauri/TypeScript boundaries touched by a slice. |
| Generation options silently degrade across backends | High | Add per-option compatibility diagnostics that state whether each option was honored, mapped, defaulted, ignored, unsupported, or rejected. |
| Task names drift between Pumas, workflow nodes, and inference backends | High | Add a strong task registry with canonical ids, aliases, modality signatures, result families, support tiers, and backend compatibility tests. |
| Preprocess/postprocess behavior leaks backend internals to workflows | Medium | Expose lifecycle phases and diagnostics, but keep tensors, token ids, Python objects, CLI flags, loaded handles, and logits processors adapter-local. |
| Capability contracts become too coarse for loaded-model decisions | Medium | Separate static backend capabilities from loaded-runtime or loaded-model facts. |
| Runtime facts duplicate runtime registry state | Medium | Inference publishes observed facts only; registry stores policy interpretation and selection state. |
| Diagnostics ledger becomes a second inference state machine | High | Record append-only run/node/runtime facts and canonical error links only; do not let ledger appends control scheduling, runtime selection, or inference execution. |
| Backend choice is not durable enough for debugging | Medium | Record selected runtime/device/network-node through existing admission/projection fields and add selected backend family/key when runtime id is not self-explanatory. |
| Ledger stores sensitive prompt/result payloads | High | Store only bounded metadata, safe ids, usage summaries, cache/artifact refs, and canonical error links; keep prompts, outputs, embeddings, tensors, logits, kwargs, CLI flags, and unbounded process output out. |
| Managed dependency extraction becomes another scheduler/runtime registry | High | Keep install/status/lease/command facts in the neutral manager and leave runtime admission/reservation policy above it. |
| Inference keeps duplicate binary state after extraction | High | Require inference to consume managed-dependency APIs for runtime command resolution and remove or shim old local state. |
| Media conversion depends on inference for ffmpeg/OCIO | High | Move media dependency contracts to the neutral manager and keep conversion process contracts in `pantograph-media-conversion`. |
| Ollama removal strands existing managed runtime state or user settings | High | Add migration/ignore behavior, tests, and documentation before removing public references. |
| Saved workflows keep old backend-specific node shapes | High | Add deterministic graph migration fixtures and validate that old inference node types do not persist after migration. |
| Ollama workflow migration silently picks the wrong replacement runtime/model | High | Migrate structurally with unresolved Pumas model-reference diagnostics unless a replacement Pumas model reference is explicitly available. |
| Existing edges/output bindings break during node-shape migration | High | Preserve node ids, edge ids, output bindings, labels, positions, groups, and mapped port semantics in migration tests. |
| Embedding mode removal breaks embedding workflows | Medium | Migrate embedding nodes to canonical `task_kind = embedding` and keep dedicated sidecars backend-local. |
| llama.cpp migration loses GGUF/mmproj semantics | High | Preserve GGUF and optional mmproj through Pumas artifact facts, then map those facts to llama.cpp backend loading. |
| PyTorch migration still exposes raw PyTorch as the graph standard | Medium | Migrate to canonical inference with a Transformers/PyTorch runtime hint rather than a public PyTorch-only node shape. |
| vLLM integration imports batching/admission policy into inference | High | Treat vLLM as an execution runtime candidate; keep scheduling and batching policy above inference. |
| MLX roadmap creates platform pressure on non-macOS builds | Medium | Keep MLX roadmap-only until macOS-specific feature gates and verification are planned. |
| Rust contracts mirror Python Transformers too literally | Medium | Model stable package/task semantics in Rust and keep Python kwargs/backend quirks inside adapters. |
| GGUF is forced into an HF-only model-package shape | Medium | Represent GGUF as a distinct Pumas-resolved artifact kind that shares task/generation semantics but maps to llama.cpp-specific loading. |
| Typed execution contracts break existing OpenAI-compatible paths | High | Preserve existing gateway methods and map typed requests to backend-specific transports incrementally. |
| Candle implementation scope expands into general Transformers parity | Medium | Start with one narrow model/task family and defer general model registry work. |
| Optional backend dependencies leak into default builds | Medium | Keep expensive/platform-specific dependencies behind explicit features and verify default, no-default-features, all-features, and targeted feature checks. |
| Path validation is duplicated across adapters or UI handlers | Medium | Route legacy paths, raw model paths, and executable paths through centralized validation/resolution utilities and test malformed path rejection. |
| Device resolution becomes backend-specific string parsing in many modules | Medium | Introduce typed resolved device facts and backend-local mapping. |
| KV cache contracts mix persisted storage with live execution cache internals | Medium | Keep persisted compatibility metadata separate from live runtime cache traits. |
| Embedding remains modeled as a separate system | Medium | Treat embeddings as a normal task contract and keep dedicated embedding sidecars backend-local. |
| KV cache is assumed portable across backends | High | Preserve strict model/runtime fingerprints and backend-specific codecs for executable reuse. |

## Clarifying Questions

- None blocking.
- Assumption: this plan should be implemented as staged contract work before
  native Candle execution is broadened.

## Unrelated Issues Not in Scope

- 2026-05-02: `cargo test -p inference` exposed an existing managed
  redistributable path expectation mismatch in
  `install_from_staging_validates_expected_files_before_finalizing`. The test
  expects `/managed-dependencies/ocioconvert/...`, while the implementation
  returns `/third-party/managed-dependencies/ocioconvert/...`.
- 2026-05-03: The same issue still fails under
  `cargo test -p inference --all-features`; the PyTorch worker-contract slice
  did not touch managed redistributable path construction.
- Reason: this is in the managed redistributable path contract and is unrelated
  to the model/package contract fixture slice.
- Revisit trigger: fix in a separate managed-dependency slice before treating
  full `cargo test -p inference` as green.
- 2026-05-02: Earlier `cargo check -p pantograph-embedded-runtime
  --no-default-features` failure from missing upstream Pumas package-fact
  helpers has been resolved in Pumas. The remaining Pantograph work is
  consumer-side alignment to the canonical Pumas DTO/API shape.
- 2026-05-03: `cargo check -p node-engine --all-features` completed for the
  Ollama graph-node retirement slice, but the dependency build surfaced an
  existing `crates/inference/src/process.rs` dead-code warning for
  `strip_managed_binary_spawn_error`.
- Reason: this warning is in the inference process helper surface and is
  unrelated to hiding the retired Ollama graph node or stale-node executor
  guard.
- 2026-05-03: The node-engine text/chat request builder still treats `task_id`
  and `taskId` as task-kind aliases for direct inference-node inputs. The
  strict task-registry validation slice now rejects unknown supplied values, so
  any saved/direct workflow input that used those fields as arbitrary node ids
  may fail earlier than before.
- Reason: the current slice preserves the existing alias list while preventing
  unvalidated backend execution. A later workflow-shape migration slice should
  decide whether `task_id` belongs in the public inference-node input contract
  or should be removed from task-kind alias parsing.
- 2026-05-03: Focused `pantograph-embedded-runtime puma_lib` validation for the
  canonical template slice compiled `pantograph-workflow-service` and surfaced
  dead-code warnings for the legacy inference migration inventory/spec helper
  types.
- Resolution: the warning set was cleared by scoping inventory/spec helpers to
  test builds and using canonical task-kind enum variants in runtime migration
  code.
- 2026-05-03: `cargo check -p pantograph-uniffi --all-features` exposed a
  missed `WorkflowErrorEnvelope.diagnostics` initializer in the optional
  frontend HTTP UniFFI adapter. The initializer was updated to preserve the
  canonical workflow error envelope shape under all features.
- Reason: this was a binding feature-matrix drift issue discovered while
  validating the Ollama backend feature-surface removal slice.
- Revisit trigger: add focused all-feature binding coverage when workflow error
  envelope fields change again.
- 2026-05-03: Focused `cargo test -p pantograph workflow::*_commands::tests`
  validation for canonical Tauri dependency command fixtures passed but surfaced
  an existing src-tauri dead-code warning set in workflow execution runtime and
  event adapter helpers.
- Reason: the warnings are outside the canonical inference node migration slice
  and do not affect dependency command request normalization.
- Revisit trigger: clean up or gate the unused Tauri workflow execution runtime
  helpers before requiring warning-free desktop command test builds.

## Definition of Done

- `crates/inference` exposes backend support facts, runtime lifecycle facts,
  and execution semantics as typed, documented contracts.
- Executable contract fixtures or schemas exist for changed persisted and
  cross-process DTOs, and they are verified by decode/normalize or round-trip
  tests.
- Pumas is documented as the canonical model source for Pantograph workflows,
  including GGUF, HF-compatible directories, safetensors, diffusers bundles,
  ONNX, and future artifact formats.
- Workflows persist Pumas model references where models can resolve through
  Pumas; raw paths remain compatibility/import/debug inputs rather than the
  preferred graph contract.
- `crates/inference` exposes Rust model-source and task contracts that consume
  Pumas-resolved package facts and follow Transformers ecosystem conventions
  without exposing Python objects as the public abstraction.
- `crates/inference` exposes a strong task registry with canonical task ids,
  aliases, modality signatures, result schemas, support tiers, and backend
  compatibility hooks for the implemented slices.
- `crates/inference` exposes typed generation configuration groups and
  per-option compatibility diagnostics instead of relying on shallow
  backend-specific parameter bags.
- `crates/inference` reports explicit package resolution, task validation,
  preprocessing, backend execution, postprocessing, and result projection
  lifecycle facts without exposing backend-local tensors, framework objects,
  command flags, or loaded handles.
- Workflow-service/node execution records inference-related backend/runtime
  choice, task/model identifiers, compatibility summaries, option-support
  summaries, lifecycle summaries, usage/cache/artifact references, and canonical
  errors into the diagnostics ledger where they are run- or node-scoped.
- Diagnostics ledger projections expose selected backend/runtime/device facts
  needed for debugging without requiring UI consumers to parse raw payload JSON
  or infer backend choice from runtime ids alone.
- Ledger append failure preserves the original inference failure and surfaces an
  explicit `diagnostics_unavailable` link rather than replacing the original
  error.
- Pumas can provide the package facts needed by inference, including raw
  upstream task evidence, normalized modality signatures, component-file
  presence, generation defaults, tokenizer/processor facts, chat template
  presence, companion artifacts, quantization facts, custom-code/security facts,
  and provenance.
- Pantograph model/library pages cache Pumas model rows, package summaries, and
  selected detail facts during startup or page population, then refresh
  affected entries from Pumas model-library update events or cursors.
- Pantograph derives technical-fit candidates and exclusion diagnostics from
  Pumas facts plus Pantograph backend/runtime/workflow context. Pumas does not
  provide final runtime candidates.
- Pantograph model-list and preflight tests prove code consumes versioned Pumas
  DTO/API outputs and fixtures without inspecting Pumas SQLite layout,
  `models.metadata_json`, or HF search-cache internals.
- Named Pumas fixture contracts cover GGUF text generation, GGUF embeddings,
  HF/Transformers text generation, multimodal processor facts,
  custom-code-required packages, unsupported Ollama hints, stale package facts,
  invalid generation config, missing tokenizer facts, and remote MLX/vLLM
  discovery hints.
- The PyTorch backend uses Transformers behind the Rust boundary for broad
  HF-compatible support.
- vLLM, future MLX, Candle, and llama.cpp/GGUF have clear mapping points from
  the same Rust model/task contracts, even when implementation is deferred.
- Pantograph has one planned managed-dependency source of truth for runtime
  sidecars, media tools, native artifacts, install state, leases, and command
  resolution.
- Saved workflows and templates with old inference node types migrate to the
  canonical inference node shape instead of keeping legacy backend-specific node
  contracts.
- Old `ollama-inference` nodes are structurally migrated and do not retain
  Ollama execution support. Missing replacement model information is represented
  as an unresolved Pumas model reference with validation diagnostics.
- Old llama.cpp nodes preserve GGUF and optional multimodal projection inputs
  through Pumas-resolved artifact facts and explicit backend mapping.
- Old PyTorch nodes migrate to canonical inference with a
  Transformers/PyTorch runtime hint where their Pumas model reference or
  HF-compatible source can be preserved.
- `crates/inference` no longer owns ffmpeg, OIIO, OCIO, or OpenColorIO media
  dependency lifecycle after the extraction slice is implemented.
- `crates/inference` consumes managed runtime executable facts through the
  neutral dependency boundary instead of maintaining a competing binary manager.
- Embeddings are represented as normal inference execution semantics; any
  dedicated embedding runtime is documented as backend-local residency strategy.
- Implementation proceeds through validated neighboring vertical slices, starting
  with GGUF text generation and expanding through GGUF embeddings,
  HF/Transformers text generation, rerank, and multimodal paths.
- KV cache remains a backend-owned artifact/compatibility contract with strict
  runtime/model fingerprints and graph-visible handles.
- Scheduler/runtime-registry/workflow layers can consume those facts without
  importing backend internals or duplicating request parsing.
- Inference contains no queueing, priority, reservation, admission, retention,
  eviction, or workflow scheduling policy.
- Existing public gateway behavior remains compatible or any breaking changes
  have a documented migration.
- Public Rust APIs touched by the implementation have crate/readme docs,
  feature docs, and `# Errors`/`# Panics` documentation where applicable.
- Unsupported operations fail explicitly and are covered by tests.
- Runtime lifecycle facts distinguish observed backend outcomes from higher
  layer policy decisions.
- Ollama is no longer exposed as a supported first-party backend, and stale
  managed runtime/settings references are either migrated or ignored safely.
- vLLM is represented as a future candidate direction with explicit boundaries
  that keep scheduler/batching policy outside inference.
- MLX is recorded as a macOS-only roadmap candidate with no near-term
  cross-platform build impact.
- Touched READMEs document API consumer and structured producer contracts.
- Required feature-matrix checks for `crates/inference` are documented and run
  for implementation slices.
- Cross-language DTO changes are verified on both native and host sides when
  Rust/Python, Rust/Tauri/TypeScript, or external HTTP runtime boundaries are
  affected.

## Milestones

### Milestone 1: Freeze Boundary Vocabulary

**Goal:** Define the exact fact-producing boundary before changing behavior.

**Tasks:**
- [x] Define terms for Pumas model reference, resolved model source,
  HF-compatible model package, GGUF artifact, task request, static backend
  capabilities, loaded-runtime facts, loaded-model facts, and execution
  request/result semantics.
- [x] Define terms for task registry entry, canonical task id, task alias,
  modality signature, result family, support tier, generation option group,
  option compatibility result, preprocessing phase, execution phase, and
  postprocessing phase.
- [x] Document the "facts, not policy" invariant in `crates/inference` and
  relevant consumer READMEs.
- [x] Choose the source-of-truth location for executable contracts and fixtures
  that multiple consumers need, or document why existing crate-local fixtures
  are sufficient for the first slice.
- [x] Define contract versioning, compatibility, and default semantics for
  Pumas package facts, task registry entries, generation options,
  compatibility reports, lifecycle diagnostics, and canonical inference nodes.
- [ ] Identify current fields that are raw facts versus fields that risk
  policy interpretation.
- [ ] Decide whether runtime facts extend `RuntimeLifecycleSnapshot` directly
  or use a new wrapper DTO.
- [ ] Define naming rules that avoid scheduler-language fields such as
  priority, reservation, admission, eviction, or selected-best-backend.
- [ ] Freeze the vertical-slice implementation order and the minimum
  cross-layer acceptance fixture required for each slice.

**Verification:**
- Review against `ARCHITECTURE-PATTERNS.md` layered separation and immutable
  contract guidance.
- Review against `RUST-API-STANDARDS.md` for typed enums/newtypes and
  append-only public contracts.
- Review against `TESTING-STANDARDS.md` vertical slice verification guidance.
- Review against executable boundary contract guidance in
  `ARCHITECTURE-PATTERNS.md` and interop validation guidance in
  `INTEROP-STANDARDS.md`.
- `git diff --check`.

**Status:** Partially implemented. First contract-only slice added
`crates/inference/src/model_contracts.rs`, public re-exports, README contract
docs, and crate-local executable fixtures/tests for package facts, task
evidence, generation defaults, option diagnostics, lifecycle phases,
Pantograph-local technical-fit candidate facts, compact
`ModelExecutionDescriptor`, and Pumas model-library cache invalidation events.

### Milestone 2: Align Pumas as Canonical Model Source

**Goal:** Make Pumas the authoritative model identity and artifact-resolution
boundary that feeds inference without making inference own model-library policy.
Detailed Pumas-side work is split into
[pumas-library-plan.md](pumas-library-plan.md).

**Tasks:**
- [ ] Treat [pumas-library-plan.md](pumas-library-plan.md) as the Pumas-side
  source for model identity, artifact facts, task evidence, generation defaults,
  custom-code/security facts, backend hints, legacy reference resolution, and
  Pumas/Pantograph fixture expectations.
- [ ] Define Pantograph-owned technical-fit candidate derivation from canonical
  Pumas package facts, Pumas dependency facts, backend capabilities, runtime
  registry state, and workflow requirements. Pumas supplies facts only;
  Pantograph owns model/backend candidate projection, exclusion diagnostics,
  live runtime selection, loaded-state interpretation, memory admission, and
  final backend choice.
- [ ] Keep the Pantograph inference plan focused on consuming Pumas package
  facts rather than specifying Pumas indexing, import, deduplication,
  migration, dependency binding, or storage implementation details.
- [x] Replace the initial Pantograph-side `PumasModelRef` and
  `ResolvedModelPackageFacts` temporary consumer contracts with adapters or DTOs
  that align to the canonical Pumas producer shape after the cross-repo
  fixture gate completes.
- [x] Replace the temporary Pantograph cache-invalidation event shape with
  canonical Pumas `ModelLibraryUpdateFeed`, `ModelLibraryUpdateEvent`,
  `ModelPackageFactsSummaryResult`, and
  `ModelPackageFactsSummarySnapshot` DTOs verified by inference contract tests.
- [x] Define Pantograph's model-list detail cache for Pumas model rows,
  package-fact summaries, Pantograph-derived technical-fit summaries where
  needed, and selected detail facts during application startup or library-page
  population.
- [ ] Subscribe to or poll Pumas model-library update events/cursors so
  Pantograph refreshes affected cached rows when models are added, removed, or
  modified, package facts are invalidated or regenerated, or dependency
  bindings change.
- [ ] Ensure Pantograph consumes versioned Pumas DTO/API outputs only. It must
  not depend on Pumas `models.metadata_json`, SQLite table layouts, or
  search-cache internals.
- [x] Treat Pumas HF search MLX/vLLM tags as remote discovery hints only.
  Installed-model compatibility and workflow preflight must use resolved local
  Pumas package facts plus Pantograph inference/backend checks.
- [x] Require the first inference vertical slice to use either real Pumas
  package facts or fixtures that match the Pumas library plan.
- [x] Use the named fixture set from [pumas-library-plan.md](pumas-library-plan.md),
  including GGUF text generation, GGUF embeddings, HF/Transformers text
  generation, multimodal processor, custom-code-required, unsupported Ollama
  hint, stale package facts, invalid generation config, missing tokenizer, and
  remote MLX/vLLM hint fixtures.
- [ ] Record any cross-repo DTO or fixture drift in both plans before
  implementation proceeds.

**Verification:**
- Review [pumas-library-plan.md](pumas-library-plan.md) against the Pumas
  repository before Pantograph implementation begins.
- Cross-repo fixture review for GGUF, HF-compatible directory, safetensors,
  diffusers bundle, ONNX, missing components, unsupported Ollama hint, and
  custom-code-required cases.
- Model-list cache tests proving startup/page population loads Pumas facts and
  update events/cursors invalidate only affected cached rows.
- Tests proving Pantograph model-list and workflow preflight code do not inspect
  Pumas `metadata_json`, SQLite layout, or search-cache internals.
- `git diff --check`.

**Status:** Partially implemented. Pantograph now has crate-local package-fact
contracts/fixtures that decode the canonical Pumas full-detail producer shape
and an embedded-runtime projection that derives `pumas_package_facts`
technical-fit candidates from those facts. Remote MLX/vLLM search tags do not
project into executable candidates. Inference now exposes Pumas-aligned update
feed and package-fact summary snapshot DTOs. Model-list cache and update-event
consumption remain pending. The `puma-lib` model-list option cache now has a
bounded `ModelLibraryUpdateFeed` invalidation helper: fresh cursors invalidate
summary/detail-scoped model ids and always invalidate removed models, while
stale cursors or snapshot-required feeds clear the cache so the next population
uses a new Pumas snapshot. The `puma-lib` option cache polls Pumas after the
startup/page snapshot and before bounded summary regeneration, so updates
observed during population are applied without inspecting Pumas storage
internals. A longer-lived application startup cache loop is still pending before
the event-consumption task is complete.

### Milestone 3: Define Transformers-Aligned Rust Model Contracts

**Goal:** Create the Rust-side resolved model-source and task shape that can
consume Pumas package facts and feed PyTorch/Transformers, vLLM, MLX, Candle,
and llama.cpp/GGUF without exposing a Python object as the shared abstraction.

**Tasks:**
- [x] Define a `ResolvedModelSource`-style contract that is normally produced
  from a Pumas model reference and distinguishes artifact kinds such as
  HF-compatible directories, GGUF files, safetensors, diffusers bundles, ONNX,
  and future formats.
- [x] Define direct-source variants only for import/debug/compatibility use,
  including local HF-compatible directories, Hugging Face repo ids, and raw GGUF
  paths.
- [x] Define model package facts such as model type, architecture hints,
  tokenizer presence, processor presence, chat template presence, generation
  config presence, weight format, and supported modalities.
- [x] Define component facts and overrides for tokenizer, processor,
  image_processor, video_processor, feature_extractor/audio_processor, chat
  template, and generation config.
- [ ] Define task request contracts aligned with Transformers task semantics
  for text generation/chat, embeddings, rerank, image/depth/audio/video-ready
  extensions, and multimodal payloads.
- [ ] Define the strong task registry shape, including canonical task id,
  aliases, task family, input modalities, output modalities, result schema,
  processor/component requirements, streaming support, generative/scoring
  behavior, default lifecycle phases, support tier, and backend compatibility
  hooks.
- [x] Seed the task registry from the planned vertical slices first:
  GGUF text generation, GGUF embeddings, HF/Transformers text generation,
  rerank, and one multimodal validation path.
- [x] Preserve neighboring Transformers task ids as explicit unsupported,
  roadmap, or contract-only entries where they are useful for compatibility
  diagnostics, rather than silently collapsing them into generic task labels.
- [x] Preserve the original Transformers pipeline task id where known, plus
  normalized Pumas modality/task signatures for routing and validation.
- [x] Define task alias handling for common upstream aliases while preventing
  raw unvalidated task strings from reaching backend execution.
- [x] Define explicit security policy fields for `trust_remote_code`,
  custom-code requirements, local-only/offline loading, cache use, auth-token
  source, revision, and code revision.
- [x] Define public versus hidden lifecycle fields for task execution:
  package resolution, task validation, preprocessing, backend execution,
  postprocessing, result projection, and diagnostics are public facts; tensors,
  token ids, raw framework objects, logits processors, loaded handles, and
  backend command flags are hidden adapter details.
- [ ] Define which fields are stable Pantograph contracts versus backend-local
  extras.
- [ ] Define serde/wire casing, enum tagging, optional field defaults, and
  unknown-field behavior for every contract that crosses Rust/Python,
  Rust/Tauri/TypeScript, Pumas/inference, saved workflow, or external HTTP
  runtime boundaries.
- [x] Add validation rules so internal code consumes parsed model/task types
  rather than raw strings or ad hoc JSON.
- [ ] Document that Python Transformers is one implementation of these
  contracts, not the source of public truth.

**Verification:**
- Review against `RUST-API-STANDARDS.md` parse-once and typed public contract
  requirements.
- Review against `ARCHITECTURE-PATTERNS.md` immutable contract guidance.
- Unit tests for model source parsing and invalid-state rejection once code is
  added.
- [x] Unit tests or fixtures for task registry alias normalization, unsupported task
  diagnostics, modality mismatch diagnostics, and missing processor/component
  diagnostics once code is added.
- Serde/wire-shape tests for any DTO crossing Rust/Python/Tauri/TypeScript
  boundaries.
- Contract fixture decode tests that prove omitted defaults, enum labels,
  unsupported task states, and option compatibility statuses retain their
  documented semantics.
- `git diff --check`.

**Status:** In progress. The first task-registry slice added seeded
Transformers-aligned registry entries for the current vertical slices and
nearby roadmap diagnostics: text generation, chat completion, embeddings,
rerank, image understanding, audio transcription, video understanding, and
multimodal generation. The registry now exposes canonical task labels, alias
normalization, modality signatures, task family, execution behavior, streaming
support, component requirements, support tier, and upstream task ids as typed
facts without making backend or scheduler decisions.
The first resolved-model-source slice added a typed backend-load source
contract that projects from Pumas package facts and distinguishes
Pumas-resolved artifacts from direct local/Hugging Face debug or import sources
without selecting a backend or runtime policy.
Task-evidence and modality-evidence matching now live on `TaskRegistryEntry`,
so backend compatibility checks consume the canonical registry validation path
instead of duplicating task-label and modality normalization locally.
The task-registry boundary now also exposes
`resolve_task_registry_entry_from_evidence`, which parses Pumas package task
evidence into a validated registry entry or a bounded
`TaskRegistryResolutionDiagnostic` for missing task labels, unsupported task
labels, conflicting upstream labels, and modality mismatches. PyTorch/
Transformers load profiling now consumes this parsed task entry rather than
walking raw task strings locally.
The model-loading boundary now exposes `ModelLoadSecurityPolicy` as the public
Rust-owned trust contract for remote-code permission, local/offline loading,
cache policy, auth-token source class, weight revision, code revision, decision
id, and accepted custom-code sources. PyTorch/Transformers adapts its
backend-local trust envelope from this public policy and passes local-file,
revision, code-revision, and cache-policy facts to the embedded worker without
exposing secret token values.
The PyTorch worker load-envelope fixture now explicitly carries the
local/offline, cache, auth-token source, revision, and code-revision policy
fields so Rust/Python wire-shape tests exercise the stable security contract
instead of relying only on serde defaults.
Node-engine text/chat request construction now validates supplied `task_kind`,
`taskKind`, `task_id`, and `taskId` labels through the inference task registry.
Missing task labels still default to text generation for existing text nodes,
but non-string labels, unknown labels, or labels that resolve to non-text tasks
now fail before gateway/backend execution instead of silently collapsing to text
generation.

### Milestone 4: Define Neutral Managed-Dependency Boundary

**Goal:** Make one Pantograph-owned dependency manager the source of truth for
runtime binaries, media tools, native artifacts, status, leases, activation,
and command resolution.

**Tasks:**
- [ ] Define the target crate/module boundary for managed dependencies, such as
  `pantograph-managed-dependencies`, including whether the first slice extracts
  code or introduces an adapter facade over existing inference modules.
- [ ] Define dependency ownership explicitly so runtime sidecar dependencies,
  media tool dependencies, and native artifact dependencies are declared by the
  crate/package that owns their execution or command resolution.
- [ ] For any new dependency or native artifact, record why in-house code is
  insufficient, transitive dependency cost, license/security notes, platform
  support, and feature-gating behavior.
- [ ] Define typed dependency categories for runtime sidecars, media tools, and
  native artifacts without making those categories scheduler-loadable runtime
  policy.
- [ ] Define common status, install state, selected/default/active version,
  expected-file, missing-file, lease, activation, and resolved-command DTOs.
- [ ] Define which operations are common and which remain category-specific:
  runtime command resolution, media executable resolution, native artifact
  activation, lease acquisition/release, install/remove, and catalog refresh.
- [ ] Define migration/import behavior for existing inference managed runtime
  and managed redistributable state files.
- [ ] Document that inference consumes runtime executable facts from this
  boundary and media conversion consumes ffmpeg/OIIO/OCIO facts from the same
  boundary.
- [ ] Keep scheduler admission, runtime reservation, runtime retention, and
  workflow policy out of the managed-dependency owner.

**Verification:**
- Boundary review against `ARCHITECTURE-PATTERNS.md`.
- Dependency ownership review against `DEPENDENCY-STANDARDS.md`.
- Dependency review against `RUST-DEPENDENCY-STANDARDS.md` for feature
  ownership, optional dependency declarations, and workspace inheritance.
- API review against `RUST-API-STANDARDS.md` for typed enums/newtypes and
  append-only DTOs.
- `git diff --check`.

**Status:** Not started.

### Milestone 5: Migrate Binary and Media Dependency Ownership

**Goal:** Move inference away from owning all managed binaries while preserving
runtime launch behavior and media conversion readiness facts.

**Tasks:**
- [ ] Inventory current uses of `managed_runtime`, `managed_redistributables`,
  `managed_binaries`, and `managed_media_dependencies`.
- [ ] Move or adapt runtime sidecar management so inference resolves llama.cpp
  commands through the neutral managed-dependency boundary.
- [ ] Move or adapt ffmpeg, oiiotool, ocioconvert, and OpenColorIO dependency
  status/lease behavior out of inference and into the neutral
  managed-dependency/media-conversion boundary.
- [ ] Update `pantograph-media-conversion` and workflow artifact conversion
  consumers so conversion dependency facts do not come from inference exports.
- [ ] Keep `pantograph-media-conversion` responsible for command planning and
  eventual conversion execution contracts.
- [ ] Add temporary compatibility shims only where needed, and document their
  removal trigger.
- [ ] Update READMEs so inference is not described as the owner of media
  dependencies.

**Verification:**
- `rg -n "managed_redistributables|managed_media_dependencies|ManagedRedistributable|Ffmpeg|OpenColorIo|Oiiotool|Ocioconvert" crates/inference`
  reviewed for intentional remaining references.
- Managed-dependency crate tests once the crate/module exists.
- `cargo test -p inference`.
- Relevant `pantograph-media-conversion` and workflow-service tests selected
  by touched files.
- `git diff --check`.

**Status:** Not started.

### Milestone 6: Retire Ollama Backend Surface

**Goal:** Remove Ollama as a first-party backend without leaving dangling
feature flags, runtime selectors, managed runtime state assumptions, or UI
entry points.

**Tasks:**
- [x] Inventory every `backend-ollama`, `OllamaBackend`, Ollama managed
  dependency/runtime id, registry, gateway, managed runtime, node-engine,
  Tauri, frontend, README, and test reference.
- [x] Decide the migration behavior for existing user settings, saved workflows,
  and managed runtime state that reference Ollama.
- [x] Remove graph-visible `ollama-inference` entry points from the
  workflow-node inventory and frontend node maps while preserving a stale-node
  migration guard.
- [x] Remove Ollama backend registration and public backend feature surface.
- [x] Remove or quarantine managed Ollama runtime/platform code so it cannot be
  selected as an active backend.
- [x] Update user-facing and developer documentation to explain that Pantograph
  does not wrap Ollama as a first-party runtime.
- [x] Add first-slice tests proving stale Ollama workflow nodes fail with a
  clear migration error instead of contacting an Ollama daemon.
- [x] Add broader tests proving stale Ollama selections fail with a clear migration
  error or are mapped to a supported default according to the migration
  decision.

**Verification:**
- `rg -n "Ollama|backend-ollama|ollama" crates src src-tauri packages docs`
  reviewed for intentional remaining references.
- `cargo test -p inference`.
- `cargo check -p inference --no-default-features`.
- `cargo check -p inference --all-features`.
- Relevant consumer tests selected by touched files.

**Status:** Implemented. The first Ollama retirement slice removed
the graph-visible `ollama-inference` registration from Rust workflow-node
inventory and frontend node maps, changed node-engine stale `ollama-inference`
execution to return a migration-focused error without contacting Ollama, and
updated touched source READMEs. The second slice removed public/default
`backend-ollama` feature surfaces from inference, embedded runtime, UniFFI, and
Tauri; stopped registering Ollama in the inference backend registry; and added
registry/gateway checks for retired Ollama behavior. The third slice
quarantined managed-runtime exposure by removing Ollama from supported runtime
enumeration, rejecting direct Ollama command resolution, and preventing stale
Ollama runtime ids from projecting selectable backend keys. The fourth slice
added saved-workflow canonicalization for legacy `ollama-inference` nodes by
migrating them to `llm-inference`, removing unsupported Ollama-only ports,
recording append-only upgrade diagnostics, preserving compatible response/stream
edges, and retaining unresolved Pumas model-reference metadata for user repair.
The fifth slice made the persisted `ollama_vlm_model` setting a scrubbed
compatibility field, stopped configured startup requests from forwarding it, and
removed the frontend model-setting/runtime-manager type surfaces that could
reintroduce Ollama as a user-selectable runtime. The sixth slice deleted the
retired inference Ollama backend implementation and per-platform managed
Ollama adapters while keeping small unsupported compatibility definitions for
old serialized runtime ids. Milestone 6 is implemented; remaining Ollama
references are intentional migration guards, unsupported-id compatibility,
tests/fixtures, or historical documentation.

### Milestone 7: Strengthen Capability Facts

**Goal:** Make backend and loaded-runtime support explicit enough for consumers
to gate behavior without backend-name conditionals.

**Tasks:**
- [x] Audit existing `BackendCapabilities` fields and consumers.
- [x] Split static backend capabilities from runtime/model-specific capability
  facts if one struct cannot honestly represent both.
  - [x] Static backend support now lives in `BackendCapabilityFacts`,
    Pumas/model/task fit lives in `BackendCompatibilityReport`, and observed
    loaded-runtime facts remain reserved for Milestone 8 runtime snapshots.
- [x] Add typed capability enums or nested structs for modality, streaming,
  embeddings, reranking, image generation, external connection, device
  selection, and KV-cache support.
  - [x] First slice added canonical task ids, modality signatures, and
    component lifecycle facts.
  - [x] Second slice added structured feature facts for streaming, device
    selection, external-connection, and KV-cache support while task ids cover
    embeddings, reranking, image generation, and other execution semantics.
- [x] Add capability checks that answer whether each backend can consume a
  given Pumas-resolved model source, task request, and option set.
  - [x] First slice added inference-local `BackendCompatibilityRequest` and
    `BackendCompatibilityReport` APIs that evaluate backend facts against a
    `TaskRegistryEntry`, `ResolvedModelPackageFacts`, and requested execution
    options.
  - [x] Wire compatibility reports into embedded-runtime technical-fit
    candidate construction so package-fact candidates no longer rely only on
    backend hints and artifact validity.
    - [x] Added a report-aware package-fact candidate constructor that consumes
      available backend facts and marks candidates compatible only when the
      inference compatibility report passes.
    - [x] Resolve/fetch package facts during the live host technical-fit path
      so `workflow_technical_fit_decision` can use the report-aware constructor
      without test-only fixture injection.
- [x] Make capability checks consume task registry entries and package facts
  rather than backend names, raw task strings, or graph-node type names.
  - [x] The inference-local compatibility API consumes canonical task registry
    entries and resolved package facts; broader workflow/runtime consumers
    still need migration.
  - [x] Workflow runtime capability projection now exposes backend model-source
    facts so downstream consumers can inspect artifact kinds, backend hints, and
    custom-code support without backend-name conditionals.
- [x] Report preprocessing and postprocessing capability separately from model
  execution capability where component files or backend processors are missing.
- [x] Keep existing fields compatible until consumers migrate.
- [x] Add tests proving declared unsupported operations return explicit errors.
  - [x] Compatibility-report tests now prove missing package components and
    unsupported requested KV-cache options produce explicit issue/option
    diagnostics.
  - [x] Backend trait default-operation tests prove unsupported image generation
    and KV-cache fingerprint requests return explicit `BackendError::Inference`
    messages.

**Verification:**
- `cargo test -p inference` focused on backend capability and gateway tests.
- `cargo check -p inference --no-default-features`.
- `cargo check -p inference --all-features`.
- README structured producer contract review for changed capability fields.

**Status:** Implemented. The first capability-facts slice audited the
flat `BackendCapabilities` bool consumers in the inference gateway, backend
registry/list commands, Tauri status commands, and frontend badges. It added
additive structured `BackendCapabilityFacts` with canonical task ids, modality
signatures, component lifecycle facts, and task-support helpers while preserving
the legacy boolean fields for existing consumers. Llama.cpp, Candle, and PyTorch
now populate initial task/modality facts. The second slice added workflow-owned
runtime-capability fact DTOs, projected backend facts from `BackendInfo` into
managed and host runtime capabilities, updated TypeScript workflow contracts,
and verified runtime preflight behavior stays unchanged when typed facts are
present. The third slice added structured static feature facts for streaming,
device selection, external connection, and KV-cache support; populated the facts
for Llama.cpp, Candle, and PyTorch; projected them through workflow runtime
capabilities; and updated the TypeScript workflow contract. The fourth slice
added backend model-source facts and an inference-local compatibility report API
that checks task registry entries, Pumas-resolved package facts,
preprocessing/postprocessing component needs, and requested options without
selecting a runtime or moving scheduler policy into inference. Broader consumer
migration remains pending. The fifth slice added a report-aware
`pantograph-embedded-runtime/src/technical_fit.rs` package-fact candidate path
that consumes available backend facts and compatibility reports instead of only
trusting package backend hints and artifact validity. The sixth slice wired the
live embedded host technical-fit path to fetch Pumas package facts for required
models when the Pumas API extension is available, decode them through the shared
JSON contract into inference package facts, and use the report-aware candidate
constructor with the gateway's available backend facts. Workflow-service preflight
can now consume the improved host decision through its existing path. The
seventh slice projected backend model-source capability facts into workflow-owned
runtime capability DTOs and TypeScript contracts so graph/preflight consumers
can inspect artifact-kind, backend-hint, and custom-code support without
depending on inference crate internals. The eighth slice added default
unsupported-operation tests for image generation and KV-cache fingerprint
requests at the backend trait boundary. Static capability facts, model/task
compatibility reports, workflow capability projection, and explicit unsupported
behavior are now covered. The ninth slice added an inference-owned
`RuntimeFactSnapshot` DTO plus readiness, reuse, and absence semantics on top of
the existing `RuntimeLifecycleSnapshot`/`ServerModeInfo` flow, without changing
runtime-registry scheduler policy. The tenth slice tightened gateway stop
cleanup facts so stopped runtimes no longer retain stale ready reasons, while
failed startup facts preserve their backend error and `runtime_start_failed`
reason through cleanup. The eleventh slice added a llama.cpp sidecar process
lease cleanup fixture that verifies startup-error cleanup kills the spawned
process, removes the PID file, and returns the server to `none` mode. The
twelfth slice fixed KV-cache `MemoryAndDisk` publication cleanup so failed disk
writes roll back the memory entry instead of exposing a cache handle from a save
that returned an error. The thirteenth slice tightened failed-restart cleanup in
the gateway so failed starts clear active runtime config and attempted mode flags
while preserving the previous successful inference config for restoration flows.
The fourteenth slice added derived backend request-lifecycle facts that map
existing preprocessing, backend execution, postprocessing, streaming, and
KV-cache capability facts into explicit cancellation and cleanup semantics
without changing backend execution methods. The fifteenth slice projected those
facts through workflow-owned capability DTOs and TypeScript contracts so
graph/preflight consumers can inspect lifecycle semantics without importing
inference crate internals. The sixteenth slice added request-scoped lifecycle
event DTOs and an opt-in gateway streaming API that records backend-execution
started, completed, failed, cancelled, and cleanup-completed events for
diagnostics-aware callers. The seventeenth slice extended the opt-in lifecycle
API to non-streaming gateway execution for embeddings, rerank, and image
generation, recording completion/failure plus cleanup facts without changing the
existing public execution methods. The eighteenth slice added an embedded
runtime diagnostics-ledger adapter that maps inference lifecycle started,
completed, failed, and cancelled events into bounded node execution status
ledger append requests while carrying the selected backend and runtime instance
as diagnostic context.

### Milestone 8: Add Runtime Fact Snapshots

**Goal:** Report observed lifecycle, device, model, and reuse facts without
publishing scheduler conclusions.

**Tasks:**
- [x] Define a runtime fact DTO containing backend key, runtime id, runtime
  instance id, active model target, resolved device, warmup timings, reuse
  result, readiness, and last backend error.
- [x] Define absence semantics for unloaded, failed, and unsupported backends.
- [x] Define cancellation and cleanup semantics for preprocessing, streaming,
  backend execution, postprocessing, process leases, and KV-cache handle
  publication.
- [x] Map llama.cpp, PyTorch, and placeholder Candle behavior into the
  DTO.
- [x] Reserve fact shape extension points for future vLLM and MLX candidates
  without adding implementation commitments.
- [x] Ensure facts are snapshots owned by inference and not mutable policy
  state owned by consumers.
- [x] Add stale-start/restart tests where existing gateway fixtures support
  them.

**Verification:**
- `cargo test -p inference gateway`.
- Lifecycle tests for stale-start, cancelled request, failed process startup,
  and cleanup-after-error cases where the slice touches runtime ownership.
- `cargo test -p pantograph-embedded-runtime runtime_registry` if runtime
  registry projections consume the new facts.
- Cross-layer acceptance check from backend start to runtime registry or host
  projection if consumer DTOs change.

**Status:** Implemented. Runtime fact DTO and normalization helpers are
implemented in `crates/inference/src/types.rs`; stop cleanup and failed-start
preservation are covered in gateway lifecycle tests; llama.cpp startup-error
process lease cleanup is covered in server tests; KV-cache `MemoryAndDisk`
publication cleanup is covered in store tests; backend request-lifecycle facts
define preprocessing, streaming, backend execution, postprocessing, process
lease, and KV-cache publication cleanup semantics; workflow capability
projection exposes static lifecycle facts to graph/preflight consumers; opt-in
gateway lifecycle APIs emit backend-execution lifecycle events for streaming and
non-streaming requests; and embedded-runtime ledger adapters can persist those
events as bounded node execution status diagnostics.

**Boundary decision:** Preprocessing and postprocessing remain backend-adapter
internal request phases for this plan. Pantograph exposes their cancellation and
cleanup semantics as static backend lifecycle facts, but does not emit
request-scoped pre/postprocessing events until a later typed execution pipeline
introduces explicit pre/post hooks. Backend execution is the request-scoped
event boundary for Milestone 8.

### Milestone 9: Bind PyTorch Through Transformers

**Goal:** Make the PyTorch backend consume the new Rust model/task contracts by
using Python Transformers behind the boundary for broad HF-compatible support.

**Tasks:**
- [ ] Replace direct/ad hoc PyTorch model loading paths with a clear
  Transformers-backed model load path where compatible.
- [ ] Map Pumas-resolved Rust model-source and task contracts to `AutoModel`,
  `AutoTokenizer`, processors, chat templates, and generation config inside the
  Python worker.
- [x] Map Rust generation option groups to Transformers generation inputs and
  return per-option compatibility diagnostics instead of accepting arbitrary
  Python kwargs as the public contract.
- [ ] Map Rust task registry entries to Transformers pipeline/model loading
  behavior inside the worker while keeping upstream Python task names as
  adapter-local implementation details after validation.
- [x] Make `trust_remote_code` and custom-code loading explicit inputs from the
  validated contract and default them closed unless an accepted policy says
  otherwise.
- [ ] Keep Python-specific compatibility shims and kwargs inside the PyTorch
  backend worker modules.
- [x] Define the Rust-to-Python worker envelope, init/shutdown lifecycle,
  error mapping, request correlation, cancellation behavior, and schema
  fixtures before changing worker behavior.
- [ ] Preserve dLLM/Sherry-specific behavior if it still requires custom worker
  logic, but describe it as backend-local support rather than a public
  Pantograph model standard.
- [ ] Normalize Transformers/Python errors into Pantograph backend errors and
  runtime facts.

**Verification:**
- PyTorch backend tests for model-source mapping and error normalization.
- Rust/Python worker contract tests or fixtures covering request decode,
  response encode, error encode, missing field rejection, enum casing, and
  default handling.
- Security-path tests for custom-code/trust policy defaults and explicit opt-in.
- Existing PyTorch request tests continue to pass.
- `cargo test -p inference`.

**Status:** In progress. The first contract-only slice added backend-local
PyTorch worker envelope DTOs, Transformers load request fields, request
correlation, init/shutdown operations, cancellation metadata, trust policy
defaults, response/error DTOs, and JSON fixtures for load and error envelopes.
The PyTorch worker now defaults `trust_remote_code` closed and rejects
Transformers packages that declare custom code unless Rust passes an explicit
trust-policy opt-in. Worker behavior still needs to move from ad hoc Python
method calls to the full envelope contract. Rust can now derive a
Transformers load envelope from Pumas package facts with contract-version,
artifact-kind, task-evidence, generation-default, and custom-code trust
validation before invoking Python.
Typed worker failures now normalize into Pantograph `BackendError` categories
with request correlation and canonical worker error codes preserved in the
bounded message.
PyTorch now has a backend-local mapper from canonical `GenerationOptions` to
Transformers-style generation kwargs plus per-option diagnostics for honored,
mapped, and currently unsupported options.
PyTorch/Transformers load envelopes now carry the typed `ResolvedModelSource`
projected from Pumas package facts as an append-only worker payload field while
retaining the older artifact fields for compatibility with existing worker
fixtures.
PyTorch/Transformers load envelopes also include a backend-local task profile
derived from the canonical task registry, mapping validated text/chat tasks to
the current causal-LM loader and rejecting registry-resolved tasks that do not
yet have a PyTorch loader instead of matching raw task strings ad hoc.
The Rust package-derived load envelope now reaches the PyTorch backend load
edge through `load_transformers_package`: package facts build and validate the
worker envelope, then Rust sends that envelope to a backend-local Python
worker entrypoint that validates the contract and adapts into the current
embedded `load_model` implementation. Full envelope dispatch for generation,
streaming, and KV operations remains a later adapter-local step.
The torch-free Python `worker_contract.py` module now owns pure envelope
validation/projection helpers so malformed contract version, operation,
payload, and trust-policy shapes can be tested without importing torch or
Transformers.

### Milestone 10: Introduce Typed Execution Contracts

**Goal:** Replace backend-transport-shaped request handling with stable
execution semantics that backends can map locally.

**Tasks:**
- [x] Add a typed generation options contract covering max tokens, temperature,
  top-p, top-k, seed, stop conditions, and cache preference where supported.
- [x] Split generation options into explicit groups such as length, sampling,
  search/beam behavior, stopping, cache, output details, special tokens,
  assistant/speculative generation where planned, and backend-scoped extensions.
- [x] Align generation options with Transformers `GenerationConfig` concepts,
  including max/new token distinction, sampling/search strategy, stop strings,
  cache preference, logits/output controls, and backend-specific unsupported
  option reporting.
- [x] Define precedence for model-provided generation defaults, workflow/node
  defaults, runtime presets, and request-level overrides.
- [x] Define a typed option-support report for every requested option:
  honored, mapped, defaulted, ignored, unsupported, rejected, conflict,
  requires model support, or requires backend support.
- [x] Keep backend-native flags hidden behind adapter mapping, including
  llama.cpp CLI/API names, Transformers/PyTorch kwargs, vLLM server/request
  options, MLX-specific names, and Candle-native execution knobs.
- [x] Define typed execution request/result shapes for chat/generation,
  embeddings, rerank, and image generation while preserving current DTOs.
- [x] Define task-specific result schemas for text generation/chat,
  embeddings/feature extraction, rerank/scoring, and multimodal results,
  including optional usage, score, logprob, cache-handle, and diagnostic
  sections where supported.
- [x] Keep OpenAI-compatible JSON mapping at the backend or adapter edge.
- [x] Add validation at the boundary so internal code consumes parsed,
  validated values rather than raw JSON.
- [x] Add serialization and deserialization coverage for changed cross-language
  DTOs, including enum casing and optional field defaults.
- [x] Record backend-specific unsupported option behavior.
- [x] Add public Rust API documentation requirements for new fallible parsing,
  validation, and execution methods, including `# Errors` and `# Panics`
  sections where applicable.

**Verification:**
- Unit tests for request parsing and validation.
- Unit tests for generation option precedence and per-backend unsupported option
  diagnostics.
- Serde round-trip tests for machine-consumed request/result contracts.
- Backend mock tests proving gateway forwards typed execution semantics.
- Rustdoc/README review for public contract docs and feature docs where public
  APIs or features change.
- Existing llama.cpp/PyTorch request tests continue to pass.
- `cargo test -p inference`.

**Status:** Implemented. `GenerationOptions` now aggregates the existing
length, sampling, search, stopping, cache, output, and special-token groups
with backend-scoped extension values. Sampling includes a typed seed field.
PyTorch has an initial Transformers kwargs mapper with diagnostics for honored,
mapped, and unsupported options. Generation option precedence is now explicit:
model defaults, then workflow/node defaults, then runtime preset, then request
overrides. Resolution diagnostics identify the source layer for each resolved
option. `OptionSupportState` now includes conflict and model/backend support
requirement variants for later backend compatibility reports.
Typed execution request/result DTOs now cover text generation/chat, embeddings,
rerank, and image generation while preserving the existing OpenAI-compatible
and task-specific DTOs. Serde coverage proves stable task/input/result casing,
usage fields, cache-handle ids, and option diagnostics.
OpenAI-compatible chat requests now map into typed execution requests at the
adapter edge, and typed request validation rejects task/input mismatches and
missing required payloads before backend execution.
`GenerationOptions::requested_option_paths()` now provides the canonical list
of requested option paths, and the PyTorch mapper test proves every requested
path receives an `OptionCompatibilityDiagnostic`. PyTorch unsupported option
behavior is covered for seed, stop strings, logprobs, token ids, and
non-Transformers backend extensions. llama.cpp now has equivalent staged
mapping coverage for supported request fields, unsupported beam/min-length/
token override/output options, KV-cache publication mapping, and non-llama.cpp
backend extensions.
Backend-native generation flags are now represented through backend-local
mappers for PyTorch/Transformers and llama.cpp instead of the public
`GenerationOptions` contract. Future vLLM, MLX, and Candle slices must follow
the same boundary. `InferenceGateway::execute_typed` validates typed execution
requests and bridges them into existing backend paths for text/chat,
embedding, rerank, and image generation.

### Milestone 11: Canonical Workflow Node Migration

**Goal:** Update graph-visible inference nodes and saved workflows to the new
canonical inference shape instead of preserving backend-specific node contracts.

**Tasks:**
- [ ] Define the canonical inference node descriptor, node data schema, port
  contract, settings schema, Pumas model-reference field, resolved model-source
  projection field, task-kind field, runtime-hint field, and migration
  diagnostics.
- [ ] Store canonical task registry ids and typed generation/task options in
  the canonical node shape rather than backend node names, embedding-mode flags,
  or backend-specific parameter bags.
- [ ] Define how node validation displays task registry, package-fact,
  lifecycle, and option-compatibility diagnostics without exposing backend
  internals.
- [ ] Define saved-workflow schema versioning and append-only migration records
  for the canonical inference node shape.
- [ ] Define executable migration fixtures for old and new saved workflow
  shapes, including expected diagnostics and default semantics when fields are
  omitted.
- [x] Inventory old graph-visible inference node types and related ports,
  including `ollama-inference`, `llamacpp-inference`, `pytorch-inference`,
  generic `inference`, `embedding`, `reranker`, model provider, unload model,
  KV ports, and embedding-mode fields.
- [x] Define old-to-new port mappings for prompt/messages, system prompt,
  model source, Pumas model reference, image/audio/video inputs, generation
  options, embedding options, rerank inputs, model refs, stream outputs, and KV
  cache handles.
- [x] Migrate old `ollama-inference` nodes structurally to canonical
  chat/generation tasks without keeping Ollama execution support. If no
  replacement Pumas model reference is known, emit an unresolved model-reference
  diagnostic instead of retaining an Ollama node.
- [ ] Migrate old llama.cpp nodes to canonical inference with
  a Pumas model reference where the GGUF can resolve through Pumas. Preserve GGUF
  path, optional mmproj path, task options, and KV cache ports through
  Pumas-resolved artifact facts where compatible.
- [ ] Migrate old PyTorch nodes to canonical inference with a
  Transformers/PyTorch runtime hint and a Pumas model reference where the
  HF-compatible source can resolve through Pumas.
- [x] Migrate old embedding nodes to canonical inference with
  `task_kind = embedding`; remove public embedding-mode semantics from the
  graph contract while preserving embedding inputs/outputs.
- [x] Migrate reranker nodes to canonical inference or a canonical rerank task
  shape without backend-specific request semantics.
- [ ] Preserve graph topology, node ids where possible, edge ids, positions,
  labels, groups, output bindings, and user-authored settings.
- [ ] Update node registry, workflow-node descriptors, graph validation,
  frontend node renderers, and templates so new saved workflows use the
  canonical node shape.
- [x] Add validation that old backend-specific inference node types do not
  persist after migration succeeds.

**Verification:**
- Fixture tests for saved workflow migration covering Ollama, llama.cpp,
  PyTorch, embedding, reranker, multimodal, stream, and KV-cache cases.
- Tests proving unresolved Ollama/Pumas model references produce migration
  diagnostics and validation errors without preserving Ollama execution.
- Tests proving raw path model fields migrate to Pumas model refs where
  resolvable and remain explicit unresolved diagnostics where not resolvable.
- Tests proving graph topology and output bindings are preserved across
  migration.
- Decode/normalize tests proving new saved workflow nodes preserve enum casing,
  option defaults, task ids, diagnostics, and unknown/unsupported field
  behavior.
- Cross-layer acceptance test from saved workflow containing a Pumas model ref
  through graph canonicalization and inference compatibility diagnostics.
- `cargo test -p workflow-nodes --lib` if descriptors change.
- Relevant `pantograph-workflow-service` graph/canonicalization tests.
- Relevant frontend typecheck/tests if node registry or renderer DTOs change.
- `git diff --check`.

**Status:** In progress. Ollama inference nodes have structural migration
coverage from an earlier slice. The workflow-service canonicalization boundary
now has an executable legacy inference-node migration inventory covering
`ollama-inference`, `llamacpp-inference`, `pytorch-inference`, `embedding`,
`reranker`, and the existing generic `llm-inference` shape, including
canonical node-data fields, task-kind/runtime-hint semantics, migration
diagnostic codes, and old-to-new port actions for model sources, generation
options, task options, KV-cache handles, streams, embedding outputs, and rerank
outputs. The `workflow-nodes` canonical `llm-inference` descriptor now exposes
optional Pumas model reference, resolved model source, typed generation options,
typed task options, model reference, diagnostics, and usage ports so migrations
have a stable graph-visible target before backend execution uses every field.
Saved-workflow canonicalization now structurally migrates legacy
`llamacpp-inference` nodes to `llm-inference`, rewrites compatible model-ref
ports, preserves generation option values into canonical node data, and records
unresolved Pumas model-reference diagnostics. It also migrates legacy
`pytorch-inference` nodes to `llm-inference`, preserves PyTorch/Transformers
runtime evidence, maps ASR-style legacy model types to `audio_transcription`,
and keeps audio input topology compatible with the canonical descriptor.
The canonical descriptor now treats task-specific inputs such as prompt, text,
and audio as optional graph ports so task registry validation can enforce the
right required input per task instead of making text generation semantics
universal. Saved-workflow canonicalization now migrates legacy `embedding`
nodes to `llm-inference` with `task_kind = embedding`, preserves text input,
embedding output, and metadata output topology, and records unresolved Pumas
model-reference diagnostics. Saved-workflow canonicalization now also migrates
legacy `reranker` nodes to `llm-inference` with `task_kind = rerank`,
preserves query/document/result/top-document topology, maps rerank options into
canonical task options, and records unresolved Pumas model-reference
diagnostics. Pumas-backed GGUF/HF resolution, mmproj-specific preservation,
frontend/template, and validation slices remain open. Rust workflow-node
inventory no longer registers retired `llamacpp-inference`,
`pytorch-inference`, `embedding`, or `reranker` node descriptors as
graph-visible built-ins; the descriptor structs remain only as migration
references and compatibility test surfaces. Frontend node maps and workflow
mocks no longer expose retired backend-specific inference renderers, the GGUF
reranker template now uses canonical `llm-inference` with
`task_kind = rerank`, and `puma-lib` exposes a canonical JSON
`pumas_model_ref` output for Pumas-to-inference graph wiring.
Workflow-service capability extraction no longer infers llama.cpp or PyTorch
requirements from retired node type names; it derives backend requirements from
canonical `runtime_hint`, `backend_key`, or Pumas `recommended_backend` data and
ignores `retired_ollama` hints.
The legacy migration inventory/spec helpers are now scoped to test builds, and
runtime migration task-kind literals use the canonical task-kind enum so focused
embedded-runtime validation no longer surfaces the workflow-service dead-code
warning set.
Embedded-runtime embedding preparation now detects canonical `llm-inference`
nodes with `task_kind = embedding` and reads model identity from canonical
Pumas model references instead of the retired `embedding` node and `model` port.
Workflow-service session graph hydration/sync now derives embedding metadata
emission from canonical embedding inference nodes instead of the retired
`embedding` node type.
Workflow-service graph memory-impact analysis now treats canonical
`llm-inference` as the KV-capable node shape and keys model/runtime invalidation
off canonical Pumas model-source and runtime-hint fields.
Embedded-runtime model dependency descriptor inference no longer derives engines
from retired PyTorch/llama.cpp/reranker node type names; canonical rerank model
type evidence maps to llama.cpp, explicit backend keys still take precedence,
and other canonical inference requests default through the existing backend
fallback path.
Workflow-node contract projection no longer assigns capability requirements to
retired backend-specific inference or dedicated embedding node descriptors.
Workflow-service capability fixtures now model inference and KV-cache extension
requirements with canonical `llm-inference` node data instead of retired
backend-specific inference node names.
Workflow-service canonicalization now has an aggregate regression test proving
that retired `ollama-inference`, `llamacpp-inference`, `pytorch-inference`,
`embedding`, and `reranker` node types are all rewritten to canonical
`llm-inference` nodes and still produce migration records.
Embedded-runtime Python dependency preflight and host dispatch now route
canonical `llm-inference` through the Python adapter only when canonical backend
data resolves to PyTorch/Transformers, and the Python bridge accepts
`llm-inference` as the canonical PyTorch execution node type.
Tauri workflow dependency commands and Puma-Lib hydration now use canonical
`llm-inference` for text-generation/PyTorch inference request shapes instead of
suggesting or normalizing to the retired `pytorch-inference` node type.
The Pumas dependency runtime probe now constructs canonical `llm-inference`
dependency scenarios for GGUF, rerank, and PyTorch-compatible LLM models instead
of emitting retired backend-specific inference node names.
Embedded-runtime session model preload now finds llama.cpp model-load requests
from canonical `llm-inference` nodes with llama.cpp runtime/backend evidence and
resolves GGUF paths from canonical `pumas_model_ref` data or Puma-Lib source
edges, so saved workflow load no longer depends on a retired
`llamacpp-inference` node shape.
Node-engine canonical `llm-inference` dispatch now routes `task_kind =
embedding` through the embedding handler, `task_kind = rerank` through the
reranker handler, and explicit llama.cpp backend hints through the llama.cpp
handler. Dependency request inference now understands canonical task kinds,
runtime hints, and Pumas model references while preserving diffusion
`recommended_backend` precedence.
Node-engine canonical task dispatch and dependency requests now resolve task
aliases through the inference task registry, so upstream labels such as
`feature-extraction`, `sentence-similarity`, `text-ranking`, and
`text-reranking` map to canonical embedding/rerank behavior and resolver-facing
task labels without ad hoc string branching.
Node-engine canonical `llm-inference` hydration now consumes the typed
`resolved_model_source` input by parsing the inference contract, filling
`model_path` from `entry_path`, and preserving the stable Pumas model id in
fallback `ModelRefV2` output without deriving backend selection from artifact
kind.
Direct node-engine execution of retired `embedding`, `reranker`,
`llamacpp-inference`, and `pytorch-inference` node types now fails with a
canonical migration error, while canonical `llm-inference` with an explicit
PyTorch/Transformers hint dispatches through the PyTorch dependency preflight
and handler when that feature is compiled.
The llama.cpp and PyTorch node-engine handlers now derive emitted
`model_ref.task_type_primary` from canonical `llm-inference` semantics, and
node-engine preflight coverage now exercises canonical runtime hints instead of
retired backend node names.
Embedded-runtime edit-session embedding fixtures, synthetic KV-cache memory
fixtures, and node-engine KV-cache session/preparation fixtures now use
canonical `llm-inference` node data instead of retired embedding or llama.cpp
node types.
Embedded-runtime Python-sidecar tests and executor documentation now refer to
canonical `llm-inference` with PyTorch/Transformers runtime evidence instead of
the retired `pytorch-inference` node name.
Node-engine dependency inference no longer treats retired backend-specific node
types as backend selectors; canonical task/runtime evidence is now required for
backend inference outside migration diagnostics.
Node-engine canonical inference hydration now rejects unresolved
`pumas_model_ref` or `resolved_model_source` migration evidence before deriving
`model_path` or requiring an inference gateway, so legacy llama.cpp/PyTorch
paths remain validation/preflight blockers until Pumas resolves them.
Node-engine non-streaming canonical `llm-inference` text/chat execution now
builds `InferenceExecutionRequest` from prompt, context, task kind, runtime
hint, Pumas model reference, and grouped generation options, then executes
through `InferenceGateway::execute_typed`.
The typed text/chat gateway path now returns per-option compatibility
diagnostics for mapped chat request fields and unsupported typed generation
options, and node-engine projects those diagnostics onto the canonical
non-streaming `llm-inference` output without exposing backend-native request
objects.
Node-engine generic streaming text/chat execution now also routes through the
gateway streaming facade instead of posting directly over HTTP from node-engine,
preserving graph `TaskStream` event shape while making backend/lifecycle facts
available through the gateway.
The streaming path now builds the same canonical `InferenceExecutionRequest`
shape and calls typed gateway stream methods, so OpenAI-compatible streaming
JSON stays inside the gateway adapter while graph-visible `TaskStream` output
remains unchanged.
Node-engine canonical `llm-inference` embedding execution now also builds an
`InferenceExecutionRequest` from text, runtime hint, model alias, and Pumas
model reference, then executes through `InferenceGateway::execute_typed` while
preserving the graph-visible embedding output shape.
Node-engine canonical `llm-inference` rerank execution now builds an
`InferenceExecutionRequest` from query, documents, top-N options, return-document
policy, runtime hint, and Pumas/model path identity, then executes through
`InferenceGateway::execute_typed` while preserving the graph-visible rerank
outputs.
The old llama.cpp-only embedding and reranker helpers have been removed from
node-engine after canonical embedding and rerank moved to the typed gateway
boundary; stale tests that preserved direct helper callers were removed with
them.
The `unload-model` node no longer contains a live Ollama HTTP unload path:
Ollama `model_ref` inputs now fail locally with a canonical/Pumas migration
message, and supported-engine diagnostics no longer list Ollama.

### Milestone 12: Prepare Native Candle Slice

**Goal:** Use the new contracts to implement or stage a narrow Candle runtime
without broad scheduler or Transformers duplication.

**Tasks:**
- [ ] Choose one initial Candle task family, preferably embeddings, with a
  documented model format and tokenizer requirement.
- [ ] Add backend-local Candle model load facts, resolved device facts, and
  explicit unsupported capability behavior.
- [ ] Map selected Pumas-resolved HF-compatible safetensors/config/tokenizer
  packages through the Rust model-source contract.
- [ ] Use Candle safetensors, tokenizer, device, dtype, and model loading
  patterns without creating a general model scheduler.
- [ ] Keep feature-gated Candle dependencies optional and documented.
- [ ] Document Candle dependency cost, feature behavior, supported platforms,
  model-family limitation, and why the selected dependency set is justified for
  the first slice.
- [ ] Add tests that compile with and without the Candle feature.

**Verification:**
- `cargo check -p inference --features backend-candle`.
- `cargo check -p inference --no-default-features`.
- `cargo check -p inference --all-features`.
- Focused Candle backend tests that do not require heavyweight model downloads
  unless explicitly marked as ignored/integration.
- README feature contract review.

**Status:** Not started.

### Milestone 13: Evaluate vLLM and MLX Roadmap Boundaries

**Goal:** Record future backend directions without expanding the immediate
implementation beyond the cleaned execution boundary.

**Tasks:**
- [ ] Evaluate vLLM as an external or managed HTTP runtime candidate and record
  which facts inference can observe directly.
- [ ] Record dependency, process, platform, network binding, security, and
  feature-gating implications for vLLM before accepting implementation work.
- [ ] Map the new Pumas-resolved Rust model-source/task contracts to likely vLLM
  startup and request inputs without implementing the backend.
- [ ] Identify vLLM-specific behavior that must remain outside inference, such
  as request admission, batching policy, model residency policy, and serving
  cluster orchestration.
- [ ] Define a candidate vLLM capability/fact mapping without implementing the
  backend.
- [ ] Record MLX as a macOS-only candidate and list platform feature gates,
  verification requirements, likely first task family, and how HF-compatible
  packages would flow through the same Rust contracts.
- [ ] Record cross-platform build expectations for non-macOS targets before any
  MLX feature or dependency is introduced.
- [ ] Add follow-up plan or ADR trigger criteria for accepting vLLM or MLX as
  implementation work.

**Verification:**
- Boundary review against this plan's out-of-scope policy list.
- Feature contract review against `RUST-API-STANDARDS.md` before adding any
  future `backend-vllm` or `backend-mlx` feature.
- No code verification required unless implementation is pulled into scope by a
  re-plan.

**Status:** Not started.

### Milestone 14: Consumer Migration and Guardrails

**Goal:** Migrate consumers to facts and contracts while preventing policy
drift back into inference.

**Tasks:**
- [ ] Update runtime registry, workflow service, node engine, Tauri, or
  frontend consumers only as needed to consume the new facts.
- [ ] Remove backend-name conditionals where new capability/runtime facts are
  sufficient.
- [ ] Update host-facing README `API Consumer Contract` and `Structured Producer
  Contract` sections for every touched source directory that publishes or
  consumes machine-readable inference/Pumas/workflow DTOs.
- [ ] Update generated bindings, host-language types, or shared schemas in the
  same implementation slice as native DTO changes when those bindings are
  public contract surface.
- [ ] Add tests proving scheduler/runtime-registry policy remains outside
  inference.
- [ ] Update source READMEs and any ADR links affected by boundary changes.
- [ ] Record any deferred consumer migrations in this plan.

**Verification:**
- `cargo test -p inference`.
- Relevant consumer crate tests, selected by actual touched files.
- `npm run typecheck` only if TypeScript DTOs or frontend consumers change.
- Native-side and host-side binding checks when Rust/Tauri/TypeScript or
  Rust/Python wire DTOs change.
- Generated binding/package smoke checks when generated or packaged host
  artifacts change.
- Cross-layer acceptance check for any changed host-facing DTO projection.

**Status:** In progress. Node-engine now consumes the typed inference boundary
for canonical `llm-inference` text/chat streaming and non-streaming execution,
embedding, and rerank execution while preserving graph `TaskStream` event output
as the host-visible streaming surface.

### Milestone 15: Diagnostics Ledger Integration

**Goal:** Make inference-related backend choice, compatibility, lifecycle, and
error facts durable through the existing diagnostics ledger without making
inference write ledger events directly.

**Tasks:**
- [ ] Map selected backend/runtime facts to existing scheduler admission,
  reservation, run-list, and run-detail projection fields where sufficient.
- [ ] Decide whether the diagnostics ledger needs an additive
  `selected_backend_key` or `selected_backend_family` field when
  `selected_runtime_id` does not clearly identify llama.cpp,
  Transformers/PyTorch, Candle, vLLM, or MLX.
- [ ] Map Pumas model id, resolved artifact kind, canonical task id, selected
  runtime id, selected backend key/family, selected device id, selected network
  node id, and scheduler policy id into durable run/node/runtime diagnostics.
- [ ] Map inference compatibility summaries into durable bounded metadata:
  accepted/rejected/degraded status, missing components, unsupported
  backend/task pairs, custom-code/trust blockers, and model/backend unavailable
  reasons.
- [ ] Map generation/task option support summaries into durable bounded
  metadata: honored, mapped, defaulted, ignored, rejected, unsupported,
  conflicts, requires-model-support, and requires-backend-support.
- [ ] Map lifecycle summaries into durable bounded metadata for package
  resolution, task validation, preprocessing, backend execution, postprocessing,
  result projection, duration, cancellation, and cleanup.
- [ ] Record usage/cache/artifact references where available, including token or
  request usage counts, cache-handle ids, KV checkpoint ids, and artifact refs,
  without storing prompt/result payload bodies.
- [ ] Continue using `diagnostic.error_occurred` for canonical errors and add
  inference-specific phases only where existing runtime preflight, model
  dependency, runtime model load, runtime launch, node execution, or output
  validation phases are insufficient.
- [ ] Preserve canonical error links from run terminal, node status, model
  lifecycle, and inference lifecycle summaries where the direct causal event is
  known.
- [ ] Ensure ledger append failure returns or projects `diagnostics_unavailable`
  while preserving the original inference/preflight/execution error.
- [ ] Keep prompts, chat messages, raw media, generated content, embeddings,
  token arrays, logits, tensors, Python kwargs, backend CLI flags, full local
  paths where stable ids exist, and unbounded stderr/stdout out of ledger
  payloads.
- [ ] Update diagnostics-ledger, workflow-service diagnostics, runtime
  projection, and UI/API README contract sections for any added event or
  projection fields.

**Verification:**
- `cargo test -p pantograph-diagnostics-ledger` for new event/projection fields,
  payload validation, schema/projection version behavior, and bounded payload
  rejection.
- `cargo test -p pantograph-workflow-service diagnostic` for workflow-service
  recorder mapping, canonical error links, and `diagnostics_unavailable`
  behavior.
- Cross-layer acceptance test from a failed inference preflight or execution
  through workflow-service/node execution into ledger projection.
- Tests proving selected backend/runtime/device/model/task facts are visible in
  run-list or run-detail projections without UI consumers parsing raw payload
  JSON.
- Tests proving prompt/result bodies and unbounded process output are not
  persisted in ledger payloads.
- `git diff --check`.

**Status:** In progress. The first diagnostics-ledger integration slice added
an append-only optional `model_id` to inference request lifecycle events and
projects it into the existing ledger append request top-level `model_id` field
alongside the already-recorded selected backend/runtime context, without adding
a new ledger event type or persisting prompt/result payload bodies. The next
slice populated gateway-produced lifecycle events with bounded model identity
from chat, embeddings, rerank, and image-generation request contracts so
started, terminal, cancellation, and cleanup facts carry the same selected
model context before they reach the ledger adapter. The following
embedded-runtime slice added a stateful inference lifecycle ledger recorder that
correlates request-scoped started and terminal events to populate bounded
`duration_ms` node execution status facts when a matching start is known, while
leaving cleanup non-persisted and preserving the existing stateless adapter for
one-off conversions.
- 2026-05-03: Diagnostics follow-up added an append-only optional
  `runtime_id` to inference request lifecycle events and changed the
  embedded-runtime ledger adapter to prefer that canonical runtime id while
  retaining backend key as separate backend context. This records the
  backend-versus-runtime distinction without a diagnostics-ledger schema change.
- 2026-05-03: Typed execution diagnostics follow-up added
  `InferenceGateway::execute_typed_with_lifecycle`, which emits task-validation
  and backend-execution lifecycle facts around canonical typed requests. This
  gives host ledger adapters bounded request id, backend, runtime, model, phase,
  terminal status, and cleanup facts without making `crates/inference` import or
  write the diagnostics ledger.
- 2026-05-03: Node-engine diagnostics bridge follow-up added the
  `INFERENCE_LIFECYCLE_SINK` executor extension and routes non-streaming typed
  `llm-inference` text/chat, embedding, and rerank calls through
  `execute_typed_with_lifecycle` when a host provides that sink. Node-engine
  assigns stable request ids for correlation but still leaves diagnostics-ledger
  writes to host/runtime adapters.
- 2026-05-03: Embedded-runtime diagnostics follow-up wires a host-owned
  inference lifecycle sink into normal workflow runs, session runs, data-graph
  execution, and edit-session graph execution. The sink maps gateway lifecycle
  facts into existing `node_execution.status` ledger events through a
  workflow-service diagnostic append helper, preserving selected backend,
  runtime id, runtime instance, model id, lifecycle phase status, and duration
  without giving inference or node-engine a diagnostics-ledger dependency.

## Execution Notes

Update during implementation:
- 2026-05-01: Initial plan created from local comparison of Pantograph
  inference, Hugging Face Transformers, and Candle, with explicit boundary
  guardrails to prevent scheduling policy from moving into inference.
- 2026-05-01: Updated plan to retire Ollama as a first-party backend, add vLLM
  as a future execution-runtime candidate, and keep MLX as a macOS-only roadmap
  item.
- 2026-05-01: Updated plan direction to make the core target a Rust
  Transformers-aligned model-source/task interface, with Python Transformers
  bound behind the PyTorch backend and the same contracts mapping to vLLM, MLX,
  Candle, and llama.cpp/GGUF where applicable.
- 2026-05-01: Updated plan to extract binary and dependency management into one
  neutral Pantograph managed-dependency boundary. Inference should consume that
  boundary for runtime executables, while media conversion consumes it for
  ffmpeg/OIIO/OCIO and related artifacts. Embeddings remain normal inference
  task semantics, and dedicated embedding runtimes are backend-local residency
  strategy. KV cache remains a strict backend/model compatibility contract, not
  portable bytes across runtimes by default.
- 2026-05-01: Updated plan to treat existing inference nodes and saved
  workflows as migration targets. Old backend-specific nodes should be
  structurally converted to the canonical inference shape. Ollama nodes should
  not keep legacy execution support; when no replacement Pumas model reference
  is known, migration should produce an unresolved model-reference diagnostic on
  the new canonical node.
- 2026-05-01: Updated plan to make Pumas the canonical model source for
  Pantograph workflows. Pumas owns stable model ids, artifact kinds, entry-path
  resolution, dependency bindings, validation state, and provenance for GGUF,
  HF-compatible directories, safetensors, diffusers bundles, ONNX, and future
  formats. Transformers remains the reference vocabulary for package/task
  semantics, not a competing model registry.
- 2026-05-02: Iterated for standards compliance against the Coding Standards
  directory. Added large-refactor findings, overlapping constraint resolution,
  dependency ownership, security/trust policy, interop DTO verification, README
  contract update requirements, worktree hygiene, and explicit unrelated-issue
  tracking.
- 2026-05-02: Added vertical-slice implementation strategy. After contract
  freeze, implementation should start with GGUF text generation and expand
  through neighboring slices for GGUF embeddings, HF/Transformers text
  generation, rerank, and multimodal support rather than building broad layers
  in isolation.
- 2026-05-02: Expanded the plan with a deeper generation configuration
  contract, strong task registry, explicit preprocess/execute/postprocess
  lifecycle diagnostics, richer Pumas package-facts requirements, and
  per-option compatibility reporting. These changes keep backend internals
  hidden inside inference while exposing stable task, model, and option
  semantics to Pantograph and Pumas consumers.
- 2026-05-02: Iterated again for Coding Standards compliance. Added executable
  contract/schema ownership, decode/normalize fixture requirements, stricter
  path validation, cross-language DTO verification, dependency-cost and
  feature-gating controls, public Rust API documentation requirements, lifecycle
  cancellation/cleanup checks, and serial ownership rules for shared contracts
  and generated artifacts.
- 2026-05-02: Added explicit diagnostics-ledger integration. The ledger should
  durably record selected backend/runtime/device/model/task facts,
  compatibility summaries, option-support summaries, lifecycle summaries,
  usage/cache/artifact references, and canonical error links through
  workflow-service/node-execution adapters. Inference remains a bounded
  fact/error producer and must not write directly to the ledger or persist
  prompt/result payload bodies.
- 2026-05-02: Split Pumas-library-specific work into
  [pumas-library-plan.md](pumas-library-plan.md). The main inference plan now
  owns the Pantograph consumer boundary and cross-repo fixture expectations,
  while the Pumas plan owns model identity, artifact facts, task evidence,
  generation defaults, custom-code/security facts, backend hints, and legacy
  reference resolution details.
- 2026-05-02: Clarified the Pumas/Pantograph split after Pumas plan updates.
  `ModelExecutionDescriptor` is treated as a current compact Pumas
  execution-facing summary, not a deprecated legacy contract. Pantograph should
  cache Pumas model-list/package-fact projections for UI responsiveness,
  refresh them through Pumas model-library update events or cursors, and derive
  technical-fit candidates inside Pantograph from Pumas facts plus
  Pantograph-owned backend/runtime/workflow context.
- 2026-05-02: Implemented the first contract-only slice in `crates/inference`.
  Added Transformers-aligned model/package/task/generation/lifecycle DTOs,
  a compact Pumas `ModelExecutionDescriptor` mirror, initial Pantograph-local
  technical-fit candidate facts, Pumas model-library change events, and named
  JSON package-fact fixtures verified through public inference integration
  tests.
- 2026-05-02: Implemented the neighboring technical-fit projection slice in
  `pantograph-embedded-runtime`. Pantograph-local package-fact candidates can
  become runtime-registry candidates without synthesizing live runtime id,
  residency, warmup, queue, or memory-admission facts. Remote MLX/vLLM
  discovery hints stay out of executable candidate projection.
- 2026-05-02: Follow-up Pumas/Pantograph boundary review moved feasible
  execution candidate derivation out of the Pumas plan. Pumas remains the
  producer of package facts, dependency facts, summaries, and update cursors;
  Pantograph owns candidate derivation and technical-fit exclusion semantics.
- 2026-05-03: Reviewed the updated plan against the current Pumas implementation.
  Pumas now builds and exposes `list_model_library_updates_since`,
  `model_package_facts_summary_snapshot`, and
  `resolve_model_package_facts_summary`; Pantograph follow-up is consumption
  and DTO alignment rather than waiting for those producer APIs.
- 2026-05-03: Implemented the first Pumas API consumption contract slice in
  `crates/inference`. The temporary cache-invalidation event DTO was replaced
  with Pumas-aligned update-feed and package-fact summary snapshot/result DTOs,
  including cursor, stale-cursor, snapshot-required, refresh-scope, selected
  artifact, and producer revision fields.
- 2026-05-03: Resolved full-detail package-fact DTO drift in the inference
  contract slice. Pantograph now decodes Pumas'
  `package_facts_contract_version`, nested `artifact`, `components[]`, single
  `task`, `backend_hints.accepted/raw/unsupported`, custom-code diagnostics,
  and raw `generation_defaults` producer shape. The old flattened package-fact
  fixture shape and Pumas-owned feasible candidate list were removed from
  `ResolvedModelPackageFacts`.
- 2026-05-03: Renamed runtime technical-fit candidate source from
  `pumas_feasible` to `pumas_package_facts` and changed embedded-runtime
  projection to derive runtime candidates from Pumas backend hints plus
  Pantograph validation context.
- 2026-05-03: Implemented the first Pantograph model-list package-summary cache
  consumer in `workflow-nodes` `puma-lib` options. Page population now reads
  Pumas summary snapshots for a producer cursor, resolves missing/invalid
  summaries through Pumas' public summary API, and attaches bounded summary
  status/payload/cursor metadata to model options without inspecting Pumas
  storage internals.
- 2026-05-03: Exposed Pumas package-summary snapshot, single-summary, and
  model-library update-feed APIs through Pantograph host surfaces: Tauri
  commands, UniFFI JSON methods, and Rustler NIFs. This unblocks UI/runtime
  consumers from polling updates after a cached snapshot cursor without
  depending on Pumas storage internals.
- 2026-05-03: Started the PyTorch/Transformers binding milestone with a
  contract-only slice. Added backend-local Rust/Python worker envelope DTOs,
  request correlation, init/shutdown operation names, cancellation metadata,
  explicit closed-by-default custom-code trust policy, typed worker errors, and
  JSON fixtures before changing the Python worker behavior.
- 2026-05-03: Hardened the PyTorch Transformers load path so
  `trust_remote_code` is an explicit Rust-owned input and defaults closed.
  The Python worker now rejects packages declaring `auto_map` custom code until
  Rust passes a validated trust-policy opt-in, while leaving the public gateway
  config shape unchanged for this slice.
- 2026-05-03: Added the Rust-side PyTorch load-envelope mapper from Pumas
  package facts. The mapper validates contract version, artifact kind, artifact
  validity, text/chat task evidence, generation defaults, and custom-code trust
  before Python receives a Transformers load request.
- 2026-05-03: Added typed PyTorch worker failure normalization into
  Pantograph `BackendError` categories, preserving request ids and canonical
  worker error codes without exposing raw Python exception payloads.
- 2026-05-03: Added the public `GenerationOptions` aggregate over length,
  sampling, search, stopping, cache, output, special-token, and backend
  extension groups. Sampling now includes an explicit seed field so typed
  generation requests cover the option families needed by Transformers,
  llama.cpp, vLLM, MLX, and Candle mappings.
- 2026-05-03: Added the first PyTorch/Transformers generation option mapper.
  It projects canonical generation groups into bounded Transformers-style
  kwargs and emits per-option diagnostics for honored, mapped, unsupported, and
  backend-extension cases without changing live generation behavior yet.
- 2026-05-03: Added canonical generation option precedence resolution:
  model-provided defaults, workflow/node defaults, runtime presets, and
  request-level overrides resolve in that order. The resolver emits typed source
  diagnostics and extends option support states with conflict and model/backend
  support requirement variants for later compatibility reports.
- 2026-05-03: Added canonical typed execution request/result DTOs for text
  generation/chat, embeddings, rerank, and image generation. The new contracts
  preserve current DTOs, keep OpenAI-compatible JSON as an edge mapping, and
  include result usage, cache handle ids, and option diagnostics.
- 2026-05-03: Added typed execution request boundary validation and an
  OpenAI-chat edge mapper. Internal typed execution can now reject task/input
  mismatches, missing text inputs, empty embedding batches, and invalid rerank
  payloads before backend adapters see the request.
- 2026-05-03: Added canonical requested-option path enumeration and PyTorch
  coverage proving every requested generation option gets a typed compatibility
  diagnostic. This closes the report contract while leaving equivalent
  unsupported-option coverage for non-PyTorch backends open.
- 2026-05-03: Added staged llama.cpp generation option mapping and diagnostics
  coverage. The mapper records supported OpenAI-compatible request fields,
  llama.cpp-specific extension mapping, unsupported option behavior, and
  all-requested-option diagnostic coverage without changing live request
  forwarding yet.
- 2026-05-03: Documented the backend-native option hiding rule in
  `crates/inference/src/backend/README.md` and marked the current
  PyTorch/Transformers and llama.cpp generation mappers as the pattern future
  vLLM, MLX, and Candle slices must follow.
- 2026-05-03: Added `InferenceGateway::execute_typed`, a validating adapter
  from canonical typed execution requests into existing chat, embedding,
  rerank, and image-generation backend methods. This provides gateway mock
  coverage for typed execution semantics without replacing existing facade
  entry points.
- 2026-05-03: Expanded the typed text/chat generation edge mapping so
  `sampling.top_p` and `sampling.top_k` travel with `max_new_tokens` and
  `temperature` through `ChatRequest` and `InferenceGateway::execute_typed`
  instead of being dropped at the OpenAI-compatible adapter edge.
- 2026-05-03: Added typed text/chat gateway option compatibility diagnostics
  for mapped chat fields and unsupported canonical generation options, then
  projected them through node-engine canonical non-streaming `llm-inference`
  outputs as bounded metadata.
- 2026-05-03: Added node-engine execution guards that reject unresolved
  migration model evidence (`pumas_model_ref.status = unresolved` or
  `resolved_model_source.status = unresolved`) before canonical inference can
  derive model paths or reach backend execution.
- 2026-05-03: Added `InferenceGateway::execute_typed_with_lifecycle` so typed
  request validation and backend execution emit separate lifecycle phases for
  diagnostics-ledger adapters while preserving inference as a fact producer
  rather than a ledger writer.
- 2026-05-03: Added a node-engine `INFERENCE_LIFECYCLE_SINK` extension so hosts
  can opt canonical non-streaming typed inference nodes into lifecycle emission
  without introducing a diagnostics-ledger dependency into node-engine.
- 2026-05-03: Removed the stale llama.cpp-only embedding and reranker execution
  helpers from node-engine after canonical embedding/rerank execution moved to
  typed gateway requests.
- 2026-05-03: Routed generic `llm-inference` streaming text/chat execution
  through the inference gateway stream facade instead of node-engine direct HTTP
  transport while preserving `TaskStream` event output.
- 2026-05-03: Added typed text/chat stream gateway methods and moved
  node-engine streaming `llm-inference` onto `InferenceExecutionRequest`,
  including task-validation lifecycle facts for diagnostics-aware hosts.
- 2026-05-03: Wired embedded-runtime execution paths to provide the
  `INFERENCE_LIFECYCLE_SINK` and persist bounded inference lifecycle facts
  through workflow-service diagnostics, with regression coverage against the
  node-status projection.

## Commit Cadence Notes

- Commit when a logical slice is complete and verified.
- Keep contract-only changes separate from backend implementation changes when
  practical.
- Follow commit format/history cleanup rules from `COMMIT-STANDARDS.md`.
- Do not start implementation while unrelated implementation files are dirty
  unless the user explicitly allows them.
- Markdown plan files may remain dirty during plan setup. Before source code,
  test, config, generated-file, lockfile, or build-manifest implementation
  begins, inspect `git status` and resolve unrelated dirty implementation files.
- Current plan setup has known documentation-only dirtiness:
  `docs/plans/README.md` and
  `docs/plans/inference-execution-boundary-contracts/`. Do not treat that as
  permission to start implementation if source, test, config, lockfile,
  generated, or build-manifest files are also dirty.

## Optional Subagent Assignment

- None planned.
- Reason: the first implementation slices touch shared public contracts and
  should be handled serially until the boundary vocabulary is frozen.
- Revisit trigger: after Ollama retirement and capability contracts are
  complete, backend-local implementation work may be split by backend if write
  sets are non-overlapping.
- If subagents or parallel workers are introduced later, update this plan with
  worker wave ownership, primary write sets, allowed adjacent write sets,
  forbidden shared files, report paths, integration sequence, and cleanup
  requirements before any parallel edits begin.

## Re-Plan Triggers

- A new contract field would encode scheduling, priority, reservation,
  admission, eviction, or workflow policy.
- Runtime registry requires a fact that inference cannot observe directly.
- Ollama removal requires preserving runtime behavior rather than migration or
  clear unsupported errors.
- vLLM implementation is pulled into scope before the external-runtime boundary
  is documented.
- Old backend-specific inference node types must remain as saved workflow
  contracts after the canonical migration is implemented.
- Saved workflow migration cannot preserve graph topology, output bindings, or
  user-authored settings.
- Saved workflow migration cannot use Pumas model references for resolvable
  models and would need to keep raw paths as canonical model identity.
- Inference must become the owner of Pumas model-library policy, indexing,
  import, deduplication, or dependency binding state.
- Pantograph requires Pumas to derive feasible execution candidates,
  technical-fit exclusions, final live runtime selection, inference
  configuration, loaded-state interpretation, memory admission, or scheduler
  policy.
- Pumas host-agnostic model-library update events or cursors cannot satisfy
  Pantograph model-list cache invalidation without Pantograph inspecting Pumas
  storage internals.
- Pantograph model-list or preflight code would need to inspect Pumas SQLite
  layout, `models.metadata_json`, or HF search-cache internals instead of
  versioned DTO/API output.
- Remote HF MLX/vLLM discovery tags would need to be treated as installed-model
  compatibility evidence.
- Ollama saved workflow migration requires a hidden compatibility execution
  path instead of structural migration to the canonical node shape.
- Managed-dependency extraction requires scheduler admission, runtime
  reservation, or workflow policy fields.
- Media conversion requires direct inference dependency for ffmpeg/OIIO/OCIO
  status after the neutral managed-dependency boundary is accepted.
- Inference cannot consume runtime executable facts through the neutral
  managed-dependency owner without breaking runtime launch behavior.
- MLX becomes a near-term requirement or must support non-macOS platforms.
- Native Candle work requires broad model registry behavior rather than a
  narrow backend-local task slice.
- Existing gateway facade compatibility cannot be preserved.
- A persisted artifact format must change.
- Feature matrix checks fail because optional backend features conflict.
- Cross-layer consumers need breaking DTO changes.
- An executable contract fixture or schema cannot be placed at a single
  producer/consumer boundary without circular dependencies or duplicated truth.
- Native DTO changes require generated or host-language binding changes that
  cannot land in the same implementation slice.
- A new dependency cannot be justified under dependency-cost, license, security,
  platform, or feature-gating standards.
- Centralized path validation cannot safely represent a required legacy model
  path, import path, managed binary path, or Pumas artifact path.
- Cancellation or shutdown behavior for a runtime sidecar, Python worker,
  stream, or native handle cannot be made deterministic.
- Diagnostics ledger integration would require inference to import or write the
  ledger directly.
- Diagnostics ledger append failure would replace the original inference error
  instead of preserving it with `diagnostics_unavailable`.
- Selected backend/runtime/device/model/task facts cannot be exposed through
  typed ledger projections without forcing UI consumers to parse raw payload
  JSON.
- Required inference diagnostics would persist prompt/result bodies, embeddings,
  tensors, logits, raw Python kwargs, backend CLI flags, full local paths when
  stable ids exist, or unbounded process output.
- Generation option compatibility cannot be expressed without leaking backend
  native flags or raw Python kwargs into public contracts.
- Task registry entries cannot align Pumas evidence, workflow node semantics,
  and backend capability checks without collapsing important upstream task ids.
- Pumas package facts cannot provide enough component, task, generation-default,
  custom-code, or companion-artifact information for inference compatibility
  checks.
- Pumas-library implementation details need to be edited back into this main
  inference plan instead of remaining isolated in
  [pumas-library-plan.md](pumas-library-plan.md).
- Preprocess/postprocess diagnostics require exposing tensors, raw framework
  objects, backend CLI flags, or loaded model handles to workflows.

## Recommendations

- Start with contracts and tests before native Candle implementation.
  This reduces the risk that Candle work hardcodes policy or transport-specific
  assumptions into the inference crate.
- Make the Pumas-resolved Rust model-source/task contract the center of the
  inference crate, not Python Transformers itself.
  This lets Pantograph use Pumas as the canonical model source and Transformers
  for PyTorch execution while keeping vLLM, MLX, Candle, and llama.cpp/GGUF as
  separate runtime mappings.
- Keep Pumas and inference responsibilities distinct.
  Pumas owns stable model identity, artifact facts, dependency bindings, and
  provenance. Inference owns backend compatibility, execution, runtime facts,
  and result semantics.
- Remove Ollama before adding vLLM.
  This prevents backend expansion from hiding the intended simplification and
  keeps the feature matrix easier to verify, but workflow migration must define
  how old Ollama nodes become canonical unresolved-model tasks before removal.
- Treat saved workflows and node descriptors as first-class migration targets.
  The backend refactor is incomplete if old backend-specific node shapes remain
  the graph contract.
- Extract or define the neutral managed-dependency owner before expanding
  backends.
  This gives llama.cpp, future vLLM-style runtimes, ffmpeg, OIIO, OCIO, and
  native artifacts one install/status/lease/command-resolution source of truth.
- Move ffmpeg/OIIO/OCIO concerns out of inference and keep them behind
  `pantograph-media-conversion` plus the neutral managed-dependency boundary.
  These are artifact conversion dependencies, not inference runtimes.
- Model embeddings as a normal inference task. Keep dedicated llama.cpp
  embedding sidecars as an internal runtime strategy for parallel residency,
  not as a separate public inference subsystem.
- Treat vLLM as a serving runtime candidate, not as a reason to move batching,
  admission, or residency policy into inference.
- Keep MLX roadmap-only until macOS-specific product value justifies the
  platform-gated backend cost.
- Treat `BackendCapabilities`, runtime facts, and execution requests as
  structured producer contracts.
  This lets higher layers make policy decisions without duplicating backend
  internals.
- Treat the task registry as the stable semantic bridge between Pumas evidence,
  workflow nodes, inference compatibility, and backend adapters.
  Do not let graph node names or backend names become the task vocabulary.
- Treat generation configuration as typed semantics plus compatibility reports,
  not as a raw parameter passthrough.
  A backend may map or reject options, but it should not silently drop them.
- Use explicit lifecycle diagnostics to make preprocessing and postprocessing
  observable without making tokenizer, processor, tensor, or model-handle
  internals part of the Pantograph contract.
- Record backend choice and inference diagnostics through the existing
  diagnostics ledger spine.
  Use selected runtime/device/network-node facts that already exist where they
  are sufficient, add selected backend family/key only if runtime ids are not
  clear enough, and link detailed failures to canonical error events.
- Keep live KV execution traits separate from persisted KV artifact metadata.
  This preserves the existing cache store role while allowing backend-specific
  decode cache improvements later.

## Completion Summary

### Completed

- 2026-05-02: Plan documents committed as
  `docs(inference): add execution boundary implementation plans`.
- 2026-05-02: First inference contract/fixture slice implemented locally:
  `model_contracts.rs`, public exports, README updates, ten named package-fact
  fixtures, and `cargo test -p inference --test model_contracts`.
- 2026-05-02: Embedded-runtime technical-fit slice implemented locally:
  Pantograph-local package-fact candidates project to advisory
  runtime-registry candidates, while remote discovery hints
  project to none.

### Deviations

- Full `cargo test -p inference` is blocked by an unrelated managed
  redistributable path expectation mismatch recorded above.
- Earlier embedded-runtime no-default feature checking was unblocked after
  Pumas restored package-fact helper support.

### Follow-Ups

- Resolve the `managed_redistributables` install-root path expectation mismatch
  as a separate managed-dependency slice.
- Continue Milestone 1 by identifying raw fact fields versus policy-risk fields
  and deciding whether runtime facts extend `RuntimeLifecycleSnapshot` directly
  or use a new wrapper DTO.
- Continue Milestone 2 with Pumas update-feed polling/subscription consumption
  and a no-missed-updates startup snapshot acceptance test around summary
  snapshots and update cursors.

### Verification Summary

- `cargo test -p inference --test model_contracts` passed.
- `cargo check -p inference --no-default-features` passed with an existing
  dead-code warning for `strip_managed_binary_spawn_error`.
- `cargo check -p inference --all-features` passed.
- `cargo test -p pantograph-embedded-runtime technical_fit` passed.
- `cargo build -p pumas-library` passed in the Pumas Rust workspace.
- `cargo check -p pantograph-embedded-runtime --no-default-features` passed
  after the Pumas package-fact helper blocker was resolved upstream.
- `cargo test -p inference --test model_contracts` passed after adding
  Pumas-aligned update-feed and package-fact summary snapshot DTOs.
- `cargo check -p inference --all-features` passed after adding Pumas-aligned
  update-feed and package-fact summary snapshot DTOs.
- `cargo test -p inference --test model_contracts` passed after replacing the
  flattened package-fact fixture shape with the canonical Pumas full-detail
  producer shape.
- `cargo test -p pantograph-embedded-runtime technical_fit` passed after
  deriving runtime candidates from Pumas backend hints and renaming their
  source to `pumas_package_facts`.
- `cargo test -p pantograph-runtime-registry technical_fit` passed after the
  source-kind rename.
- `cargo check -p inference --all-features` passed after the full-detail Pumas
  DTO alignment.
- `cargo check -p pantograph-embedded-runtime --no-default-features` passed
  after the full-detail Pumas DTO alignment, with the existing
  `strip_managed_binary_spawn_error` dead-code warning.
- `cargo test -p workflow-nodes --features model-library puma_lib` passed after
  adding Pumas package-summary cache population to `puma-lib` options.
- `cargo check -p workflow-nodes --features model-library` passed after adding
  Pumas package-summary cache population to `puma-lib` options.
- `cargo check -p pantograph-uniffi` passed after exposing Pumas summary/update
  APIs and fixing the existing workflow error envelope diagnostics field.
- `cargo check --manifest-path crates/pantograph-rustler/Cargo.toml` passed
  after exposing Pumas summary/update NIFs.
- `cargo check --manifest-path src-tauri/Cargo.toml` passed after exposing
  Pumas summary/update Tauri commands, with existing dead-code warnings in the
  Tauri workflow modules.
- `cargo test -p inference` failed in
  `managed_redistributables::install_from_staging_validates_expected_files_before_finalizing`
  due to the unrelated managed-dependency path mismatch recorded above.
- `git diff --check`.
- Trailing-whitespace scan for
  `docs/plans/inference-execution-boundary-contracts/plan.md`.

### Traceability Links

- Module README updated: N/A for initial plan-only artifact.
- ADR added/updated: N/A for initial plan-only artifact.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A until
  implementation PR.

## Brevity Note

Keep implementation updates concise. Expand this plan only when execution
decisions, compatibility impact, or risk controls change.
