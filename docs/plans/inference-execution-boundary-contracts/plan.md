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
- Consume Pumas' fast `ModelLibrarySelectorSnapshot` contract as the first
  model-list and graph-selector read path, then batch-hydrate only selected
  models for package summaries, execution descriptors, and inference settings.
- Choose an explicit Pumas access role per Pantograph process: owning
  `PumasApi` when Pantograph owns the instance, explicit `PumasLocalClient`
  when attaching to a running local instance, or `PumasReadOnlyLibrary` for
  indexed local snapshots that must not start lifecycle work.
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
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/COMMIT-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CODING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/ARCHITECTURE-PATTERNS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CONCURRENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DEPENDENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/SECURITY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/FRONTEND-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/INTEROP-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/LANGUAGE-BINDINGS-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TOOLING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CROSS-PLATFORM-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-API-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-ASYNC-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-CROSS-PLATFORM-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-DEPENDENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-INTEROP-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-LANGUAGE-BINDINGS-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-SECURITY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/languages/rust/RUST-TOOLING-STANDARDS.md`
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
| Blast radius | The plan touches model-library selectors, workflow graph contracts, node execution, runtime registry facts, diagnostics, frontend stores, native bindings, and managed dependencies. | Add an explicit blast-radius and serial-ownership section. Each implementation slice must name its primary write set, adjacent write set, forbidden shared files, affected contracts, and verification gate before code changes. |
| Pumas selector access roles | The settled Pumas API removed hidden local-client fallback and added owner/local-client/read-only roles. | Pantograph must implement an explicit access-role adapter and test owner, local-client, and read-only behavior. Read-only access must receive the model-library root containing `models.db`. |
| Frontend synchronization | Library and graph selectors must not replace slow backend composition with frontend polling or optimistic model-list state. | Treat Pumas/Pantograph backend projections as source of truth, refresh through cursor/update events, and add cleanup/stale-response tests for any frontend subscription hook. |
| Binding blast radius | UniFFI, Rustler, Tauri commands, TypeScript mirrors, and frontend service DTOs can drift from Rust contracts. | Assign binding/generated DTO updates to serial slices. Verify native-side conversion tests and host-side type/smoke tests when any boundary shape changes. |
| Async lifecycle | Selector updates, local-client streams, backend sidecars, Python workers, and diagnostics sinks all involve cancellable async work. | New long-lived tasks must have owners, cancellation, shutdown, panic handling, and no discarded `JoinHandle`s. Core validation/projection remains sync unless I/O requires async. |
| Path and payload safety | Pumas roots, artifact paths, runtime commands, media paths, and diagnostics payloads cross trust boundaries. | Parse once into validated domain values, use centralized path validation, sanitize local paths in diagnostics, and reject prompt/result/tensor/cache bytes from ledger-bound payloads. |
| File-size/decomposition risk | Existing touched modules are already large in several areas, and implementation could worsen reviewability. | Any touched file over 500 lines or module crossing the standards thresholds must include a decomposition decision in the slice notes: split now, add focused helper module, or document why the local change remains safe. |
| Tooling and feature matrix | Backend features, generated bindings, and frontend mirrors make partial checks insufficient. | Each slice must list default/no-default/all-feature Rust checks and focused frontend/binding checks needed by its write set; skipped checks need an explicit reason. |

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
- Pumas fast selector implementation is settled as of commit `6c564404` in the
  Pumas repository. Pantograph should treat
  `model_library_selector_snapshot` as the normal list/selector entry point and
  use package-summary, execution-descriptor, and inference-settings batch APIs
  only for selected or expanded models.
- `PumasReadOnlyLibrary::open` takes the Pumas model-library root containing
  `models.db`; Pantograph must not pass the Pumas launcher root, source repo
  root, or generic application data root unless that path is resolved to the
  model-library directory first.
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
- Old `ollama-inference` nodes are not a supported compatibility target.
  Workflows should be updated to canonical `llm-inference` with a Pumas model
  reference before execution rather than relying on a synthesized Ollama
  placeholder.
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
- `crates/inference/src/managed_runtime/`: runtime sidecar manager for
  llama.cpp launch compatibility, adapted to neutral managed-dependency status
  and command facts.
- `crates/inference/src/managed_redistributables/`: private projection adapter
  from managed redistributable status into neutral managed-dependency status.
- `crates/inference/src/managed_binaries.rs`: runtime-launch compatibility
  facade that resolves through neutral runtime sidecar command facts; it no
  longer owns aggregate runtime/media/native status DTOs.
- `crates/inference/src/managed_media_dependencies.rs`: compatibility adapter
  that maps legacy inference media-planning DTOs onto
  `pantograph-media-conversion` ownership.
- `crates/pantograph-managed-dependencies/`: neutral managed dependency
  contract crate for runtime sidecars, media tools, native artifacts, status,
  leases, activation facts, command facts, and operation scopes.
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
- Pumas settled fast selector plan and contract:
  - `/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library/docs/plans/pumas-fast-model-snapshot-and-client-api/plan.md`
  - `/media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pumas-Library/docs/plans/pumas-fast-model-snapshot-and-client-api/selector-snapshot-contract.md`
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
- Pumas `ModelLibrarySelectorSnapshot`, `ModelLibrarySelectorSnapshotRow`,
  `ModelEntryPathState`, `ModelArtifactState`, and
  `ModelLibrarySelectorDetailState` contracts for fast Library page and graph
  model-selector population.
- Pumas explicit instance/client/read-only access contracts:
  `PumasApi`/`FfiPumasApi` are owner-only, `PumasLocalClient` is the same-device
  client transport, and `PumasReadOnlyLibrary` is a local indexed snapshot
  reader opened against the model-library root that contains `models.db`.
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
- Saved settings or workflows that refer to the Ollama backend should surface
  explicit unsupported-contract feedback; Pantograph should not synthesize a
  replacement Ollama runtime or placeholder model reference.
- Saved workflows and templates that persist raw model paths should migrate to
  Pumas model references where Pumas can resolve the model. Unresolved paths
  should become explicit migration diagnostics rather than remaining canonical
  model identity.
- Saved workflows and templates containing old `llamacpp-inference`,
  `pytorch-inference`, `embedding`, or `reranker` nodes require deterministic
  migration into the canonical inference node shape. `ollama-inference` remains
  removed rather than migrated.
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

### Blast Radius And Standards Guardrails

The implementation blast radius is intentionally broad because the plan changes
how Pumas model facts enter Pantograph, how graph-visible inference nodes are
represented, and how backend execution facts are diagnosed. Each slice must
declare its affected row in this table before code changes begin and must not
touch another row's primary write set without updating this plan.

| Area | Likely Primary Write Set | Blast Radius | Guardrail |
| ---- | ------------------------ | ------------ | --------- |
| Pumas selector integration | `crates/workflow-nodes/`, `src-tauri/src/llm/commands/`, frontend Library/model-selector services | Library page, graph `puma-lib` selector, startup cache, update cursor handling | Use `ModelLibrarySelectorSnapshot` first; no per-row hydration; explicit owner/local-client/read-only access role; no Pumas SQLite or `metadata_json` reads. |
| Pumas selected-detail hydration | `crates/workflow-nodes/`, `crates/pantograph-embedded-runtime/`, `src-tauri/` Pumas commands | Selected model details, inference settings display, execution preflight | Use Pumas batch APIs where available; hydrate only selected/expanded models; preserve update cursor invalidation. |
| Canonical inference contracts | `crates/inference/src/model_contracts*`, `crates/inference/src/types*`, inference fixtures/tests | Backend adapters, node-engine request builders, workflow-service capability facts, bindings | Contract fixtures first; append-only DTO changes; typed enums/newtypes; serde default/unknown-field tests. |
| Graph/node migration | `crates/pantograph-workflow-service/src/graph/`, `crates/workflow-nodes/src/processing/`, `crates/node-engine/` | Saved workflows, templates, frontend node registry, output bindings | Preserve topology, node ids, edge ids, port intent, positions, labels, and migration diagnostics. |
| Runtime registry and technical fit | `crates/pantograph-embedded-runtime/`, `crates/pantograph-runtime-registry/`, `crates/pantograph-workflow-service/` | Backend/runtimes shown as available, candidate explanations, workflow preflight | Pantograph derives candidates; Pumas and inference do not own scheduler/admission/selection policy. |
| Diagnostics ledger projection | `crates/pantograph-workflow-service/`, `crates/pantograph-embedded-runtime/`, `src/services/diagnostics/` | Run/node diagnostics, selected backend visibility, canonical errors | Workflow/node execution owns ledger writes; inference returns bounded facts/errors only; no prompt/result/tensor/cache bytes. |
| Backend adapters and workers | `crates/inference/src/backend/`, `crates/inference/torch/`, future vLLM/MLX/Candle modules | Model loading, execution, streaming, KV cache, subprocess lifecycle | Keep Python/CLI/framework objects adapter-local; validate request contracts before execution; own spawned task/process lifecycle. |
| Managed dependency extraction | `crates/pantograph-managed-dependencies/`, `crates/inference/src/managed_runtime/`, `crates/pantograph-media-conversion/` | Runtime sidecars, media tools, UniFFI/frontend runtime settings | One source of truth for install/status/lease/command facts; media conversion does not depend on inference for ffmpeg/OIIO/OCIO ownership. |
| Frontend projections | `src/services/`, `src/components/`, `packages/svelte-graph/` | Library page, graph editor, diagnostics/workbench views | Backend-confirmed state only; no optimistic backend-owned updates; event/cursor-driven refresh preferred over polling; accessible selector tests. |
| Language bindings | `crates/pantograph-uniffi/`, `crates/pantograph-rustler/`, generated host mirrors | C#/Beam/native consumers, smoke harnesses, release artifacts | Binding layer stays thin; core compiles without bindings; native and host-side tests for changed DTOs. |

Serial ownership points:

- Shared public contracts, schema/fixture files, generated bindings, lockfiles,
  feature flags, root manifests, saved-workflow migration fixtures, and
  diagnostics-ledger event/projection DTOs must be owned by exactly one slice.
- Implementation slices may read broadly, but edits outside the declared
  primary or allowed adjacent write set require a plan update before editing.
- Source files over 500 lines, UI components over 250 lines, or modules with
  mixed responsibilities require a decomposition decision in the slice notes.
  The decision may be "split now", "extract a focused helper", or "defer with
  reason", but it must be explicit before adding more responsibility.
- Generated or host-language mirrors must not be hand-edited without the
  owning generation or projection contract in the same slice.

Standards gate per implementation slice:

- Start from a clean implementation worktree except for accepted plan/docs
  changes.
- Write or update the failing contract/acceptance fixture before implementation
  when the slice crosses layers.
- Validate boundary inputs once into typed domain values and keep raw strings,
  raw paths, Python kwargs, backend command flags, and scheduler terms out of
  internal APIs.
- Keep pure parsing/projection synchronous unless the slice owns real I/O.
  Async tasks, streams, subprocesses, local-client connections, and polling
  hooks must have one lifecycle owner, cancellation, cleanup, and observed
  errors.
- Run verification selected from the blast-radius row: Rust unit/integration
  tests, feature-matrix checks, frontend type/tests, binding smoke tests, and
  `git diff --check`. Skipped checks require a recorded reason.
- Commit each verified logical slice atomically before starting the next
  implementation slice.

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
- Pumas local-client update streams and Pantograph cache refresh loops must have
  one owner that can stop polling/subscription work on app shutdown, Library
  page teardown, or graph-selector disposal. Do not add global frontend polling
  loops for model-list state.
- If frontend selectors subscribe to backend-projected updates, stale async
  responses must be discarded by request id or cursor, timers must be cleaned
  up on unmount, and tests must prove cleanup under completion and teardown.
- Read-only selector snapshots must be treated as local indexed reads with no
  lifecycle ownership. They must not start Pumas reconciliation, downloads,
  watchers, package-fact regeneration, or hidden IPC fallback.
- Local-client connections must validate loopback/same-device endpoint facts
  through Pumas' public discovery output and must surface token/connection
  failures as typed integration diagnostics, not silent downgrade to another
  access role.

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

The first slice after the settled Pumas fast-snapshot implementation should be
the thinnest useful end-to-end model-selection path that proves Pantograph can
populate Library and graph model selectors without deep per-model Pumas
hydration. Broaden shared layers only when a neighboring slice requires it. Do
not complete broad Pumas, inference, workflow, frontend, or backend rewrites in
isolation before at least one vertical slice proves that the contracts work
together.

Planned slice order:

0. Pumas fast selector integration:
   Pumas explicit access role -> `model_library_selector_snapshot` page/filter
   request -> Pantograph Library and `puma-lib` graph selector row projection ->
   cursor handoff -> selected-row batch hydration only.
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
- For the Pumas fast selector slice, an explicit access-role decision and a
  test proving Pantograph passes the Pumas model-library root to read-only
  access rather than the launcher/source repository root.
- For the Pumas fast selector slice, a no per-row hydration acceptance test:
  list population consumes selector rows first and uses batch summary,
  descriptor, or settings hydration only for selected or explicitly expanded
  models.
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

Minimum verification matrix by slice type:

- Contract-only Rust DTO slice: targeted crate tests, serde/default/unknown
  field tests, default/no-default/all-feature checks for affected public
  feature contracts, and README/fixture updates.
- Pumas selector/list slice: Rust adapter tests, model-list or graph-selector
  acceptance test, no per-row hydration assertion, no Pumas storage-internal
  access assertion, cursor stale/update recovery tests, and frontend type/test
  coverage when UI stores or components change.
- Graph migration slice: saved-workflow fixture migration tests, topology/edge/
  output-binding preservation assertions, stale-node failure diagnostics, and
  frontend node-definition parity tests.
- Backend adapter slice: compatibility preflight tests, typed request/result
  tests, lifecycle/cancellation/error tests, no prompt/result diagnostics
  leakage tests, and backend feature-matrix checks.
- Binding slice: native conversion/error tests plus host-language smoke or
  generated-binding tests for every touched supported binding.
- Managed-dependency/media slice: state migration/import tests, lease/command
  resolution tests, cross-platform path handling tests where paths are touched,
  and consumer tests proving inference is not the owner of media tool lifecycle.

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
| Pantograph keeps using slow Pumas list/summary/detail composition for model selectors | High | Make `model_library_selector_snapshot` the first read for Library and graph model selectors; use batch hydration only for selected or expanded rows. |
| Pantograph opens read-only Pumas snapshots with the wrong root | Medium | Resolve and test the Pumas model-library root that contains `models.db`; do not pass launcher/source roots to `PumasReadOnlyLibrary::open`. |
| Pantograph silently falls back to hidden Pumas RPC/client behavior | Medium | Use explicit owner, local-client, or read-only access roles and surface role-selection failures as typed integration diagnostics. |
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

- 2026-05-03: The managed dependency adapter slice resolved the known managed
  redistributable path expectation mismatch by updating
  `install_from_staging_validates_expected_files_before_finalizing` to assert
  the canonical `/third-party/managed-dependencies/...` install root while
  preserving separate legacy install-root fallback coverage.
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
- 2026-05-05: `cargo check -p pantograph-embedded-runtime` completed for the
  inference lifecycle sink return-channel slice, but surfaced an existing
  `crates/pantograph-embedded-runtime/src/model_dependency_descriptors.rs`
  dead-code warning for `map_pipeline_tag_to_task`.
- Reason: this warning is in the model dependency descriptor helper surface and
  unrelated to the lifecycle sink `diagnostics_unavailable` return contract.
- 2026-05-03: The node-engine text/chat request builder still treats `task_id`
  and `taskId` as task-kind aliases for direct inference-node inputs. The
  strict task-registry validation slice now rejects unknown supplied values, so
  any saved/direct workflow input that used those fields as arbitrary node ids
  may fail earlier than before.
- 2026-05-08 resolution: canonical text-generation request building now reads
  task semantics from `task_kind`/`taskKind` only. `task_id` and `taskId` are no
  longer public task-kind aliases, so node-id-shaped values default to ordinary
  text generation instead of failing task-registry validation.
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
- 2026-05-04: Full `cargo test -p pantograph-embedded-runtime` validation for
  the KV-cache option-diagnostics slice surfaced a deterministic unrelated
  failure in
  `tests::runtime_preflight_tests::workflow_preflight_reports_candle_runtime_as_available`.
  Focused KV-cache ledger validation and `cargo check -p
  pantograph-embedded-runtime` pass.
- Reason: the failure is in Candle runtime preflight availability
  (`response.blocking_runtime_issues` is not empty) and is outside the KV-cache
  progress/ledger option-diagnostics boundary touched by this slice.
- Resolution: active backend capability projection now overlays the registry
  factory row with `InferenceGateway::current_backend_info`, so host-injected
  current backends are represented as available even when the compile-time
  registry factory reports missing system prerequisites.
- 2026-05-06: The Pumas fast selector Pantograph slice hit an active Pumas
  producer checkout parse error while rerunning `cargo fmt --all` and
  `cargo test --manifest-path src-tauri/Cargo.toml puma_lib_commands`.
  `Pumas-Library/rust/crates/pumas-core/src/api/system.rs` currently fails to
  parse at the dirty `get_disk_space`/`get_system_resources` area before
  Pantograph code is compiled.
- Reason: Pumas is actively implementing
  `docs/plans/backend-owned-status-resource-telemetry/plan.md`, and the dirty
  files match that producer-side telemetry slice. The Pantograph selector
  changes are outside that Pumas write set and can be revalidated after the
  Pumas telemetry slice restores a parsing path-dependency checkout.
- Resolution: after Pumas completed the telemetry bug-fix slice and restored a
  clean parsing checkout, the Pantograph selector slice revalidated with
  `cargo test -p workflow-nodes --features model-library --lib puma_lib` and
  `cargo test --manifest-path src-tauri/Cargo.toml puma_lib_commands`.

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
- Pantograph Library page and graph model selectors populate first from Pumas
  `ModelLibrarySelectorSnapshot` rows and do not hydrate every listed model.
- Pantograph selected-model detail panes and execution preflight use Pumas batch
  hydration APIs for package-fact summaries, cheap execution descriptors, and
  inference settings where those APIs cover the selected set.
- Pantograph has an explicit Pumas access-role adapter for owner, local-client,
  and read-only selector paths, including a guard that read-only access receives
  the model-library root containing `models.db`.
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
- Old `ollama-inference` nodes are not structurally migrated and do not retain
  Ollama execution support. Replacement workflows must use canonical
  `llm-inference` with a Pumas model reference.
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
  with Pumas fast selector integration after the settled Pumas producer
  baseline, then expanding through GGUF text generation, GGUF embeddings,
  HF/Transformers text generation, rerank, and multimodal paths.
- Each implementation slice records its blast radius, primary write set,
  adjacent write set, forbidden shared files, shared contract owner,
  decomposition decision for oversized touched files, and verification gate
  before code changes begin.
- Shared contracts, generated bindings, feature flags, lockfiles, saved-workflow
  fixtures, and diagnostics projection DTOs are changed only by their assigned
  serial owner for the slice.
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

### Milestone 0: Standards And Blast-Radius Gate

**Goal:** Prevent the implementation from turning the broad inference plan into
unbounded cross-repo churn.

**Tasks:**
- [x] Review the plan family against the Coding Standards set, including plan,
  architecture, documentation, testing, security, dependency, interop,
  concurrency, frontend, tooling, language-binding, cross-platform, and Rust API
  standards.
- [x] Add the blast-radius table that identifies primary write sets, likely
  consumer impact, and standards guardrails for each implementation area.
- [x] Add serial ownership rules for shared contracts, fixtures, generated
  bindings, feature flags, lockfiles, saved-workflow migrations, and diagnostics
  projection DTOs.
- [x] Add per-slice verification matrix guidance so implementation cannot rely
  on typecheck-only validation for cross-layer changes.
- [x] Add Pumas fast selector access-role and read-only-root guardrails after
  the settled Pumas implementation review.
- [x] Before the next code slice, record the exact Milestone 2 fast selector
  primary write set, adjacent write set, forbidden shared files, and acceptance
  test names.

**Verification:**
- Standards review produces no further required plan edits for the current
  known Pumas fast selector implementation baseline.
- `git diff --check` passes for plan artifacts.
- No implementation code starts until source/test/config/generated/lockfile
  dirtiness is either absent or explicitly accepted.

**Status:** Completed. The 2026-05-06 standards pass recorded the
slice-specific write set for Pumas fast selector integration, and the
implementation now satisfies it: `puma-lib` model options query the explicit
Pumas selector-access role, selector rows are populated from
`model_library_selector_snapshot`, read-only setup opens the model-library root
containing `models.db`, and the frontend shares the selector snapshot/cache
handoff between the Library page and graph `puma-lib` node.

**Milestone 2 Fast Selector Slice Boundary:**

- Primary write set:
  `crates/workflow-nodes/src/input/puma_lib.rs`,
  `src-tauri/src/workflow/puma_lib_commands.rs`,
  `src/components/nodes/workflow/PumaLibNode.svelte`, and
  `src/components/workbench/LibraryPage.svelte` if UI behavior needs to expose
  selector-row or selected-detail state.
- Adjacent write set:
  `src-tauri/src/workflow/workflow_port_query_commands.rs`,
  `src-tauri/src/workflow/commands.rs`,
  `src-tauri/src/app_setup.rs`,
  `crates/workflow-nodes/src/setup.rs`, and
  `src/services/workflow/types.ts` only if the slice adds a new typed command,
  explicit access-role adapter, or frontend DTO mirror.
- Forbidden shared files unless the plan is updated first:
  external `../Pumas-Library/**`, `crates/node-engine/src/port_options.rs`,
  `crates/pantograph-embedded-runtime/**`, `packages/svelte-graph/**`,
  `shared-resources/models/models.db`, `Cargo.lock`, generated bindings, and
  diagnostics-ledger event/projection DTOs.
- Contract owner:
  Pantograph owns projection from Pumas selector rows into `PortOption`
  metadata and selected-node data; Pumas owns the producer DTO/API contract.
- Decomposition decision:
  `crates/workflow-nodes/src/input/puma_lib.rs` and
  `src-tauri/src/workflow/puma_lib_commands.rs` are oversized existing modules.
  This slice may add focused helpers inside those modules to keep the validated
  vertical change narrow; broader file splitting is deferred because moving
  module boundaries would enlarge the blast radius beyond selector behavior.
- Acceptance tests:
  `cargo test -p workflow-nodes --features model-library --lib puma_lib`,
  `cargo test --manifest-path src-tauri/Cargo.toml puma_lib_commands`, and a
  focused `git diff --check`.
- Acceptance criteria:
  model-list and graph selector population call
  `model_library_selector_snapshot` first, do not call per-row descriptor or
  inference-settings hydration, use `row.model_ref.model_id` as canonical
  identity, treat `indexed_path` as display/debug data only, expose `entry_path`
  as executable only when entry and artifact states are both `ready`, and
  hydrate package summaries, execution descriptors, and inference settings only
  for the selected model or explicit expanded-row detail path.
- 2026-05-07 validation: `cargo test -p workflow-nodes --features
  model-library --lib puma_lib` passes and includes
  `test_model_options_use_selector_snapshot_without_detail_hydration`,
  `test_selector_row_option_uses_entry_path_only_when_ready`, and
  read-only selector coverage.
- 2026-05-07 follow-up: the selector cursor is now also projected into
  result-level `PortOptionsResult.metadata.package_facts_summary_cursor`, so an
  empty selector page can still hand off to
  `list_model_library_updates_since(cursor)`. Frontend cache code reads the
  result-level cursor before the older per-option fallback, and Rust/TypeScript
  tests cover the empty-snapshot path.

**Milestone 2 Selector Access-Role Slice Boundary:**

- Primary write set:
  `crates/workflow-nodes/src/setup.rs`,
  `crates/workflow-nodes/src/input/puma_lib.rs`,
  `crates/workflow-nodes/src/README.md`,
  `crates/workflow-nodes/src/input/README.md`, and this plan.
- Adjacent write set:
  `crates/node-engine/src/extensions.rs` only if the explicit role keys need to
  move into the shared extension-key contract, and
  `crates/pantograph-embedded-runtime/src/runtime_extensions.rs` only if
  executor-scoped extension snapshots must preserve the read-only selector
  object.
- Forbidden shared files unless the plan is updated first:
  external `../Pumas-Library/**`, `src-tauri/**`, frontend components/stores,
  generated bindings, `Cargo.lock`, diagnostics-ledger DTOs, and saved-workflow
  migration fixtures.
- Contract owner:
  `workflow-nodes` owns host extension setup for selector access. Pumas owns
  the owner/local-client/read-only API behavior and the read-only requirement
  that callers pass the model-library root containing `models.db`.
- Decomposition decision:
  `crates/workflow-nodes/src/input/puma_lib.rs` is already oversized, but the
  selector access change is a focused extension lookup replacement plus one
  acceptance test. Broader model-list decomposition is deferred to avoid
  mixing structural refactors with access-role behavior.
- Acceptance tests:
  `cargo test -p workflow-nodes --features model-library read_only`,
  `cargo test -p workflow-nodes --features model-library test_model_options_use_read_only_selector_snapshot_without_pumas_api`,
  `cargo test -p workflow-nodes --features model-library test_model_options_use_selector_snapshot_without_detail_hydration`,
  `cargo fmt --all`, and `git diff --check`.
- Acceptance criteria:
  `setup_extensions_with_path` records an explicit selector access role when
  owner API, local-client, or read-only access is available; read-only access
  resolves launcher roots and build-output roots to `shared-resources/models`
  before opening; `puma-lib` options query through the selector role without
  requiring full `PUMAS_API`; and `model_id` selected-model detail hydration
  routes through the same selector-access role, using owner/local-client batch
  detail APIs or read-only selector-row projection as available.

**Milestone 2 Selector Cursor Handoff Slice Boundary:**

- Primary write set:
  `src/services/workflow/pumaModelOptionsCache.ts`,
  `src/services/workflow/pumaModelOptionsCache.test.ts`,
  `src/components/nodes/workflow/PumaLibNode.svelte`,
  `src/components/nodes/workflow/README.md`,
  `src/components/workbench/LibraryPage.svelte`,
  `src/components/workbench/README.md`, `package.json`, and this plan.
- Adjacent write set:
  `src/services/workflow/types.ts` only if the update-feed wire shape needs a
  shared TypeScript mirror. Backend Tauri commands are adjacent only if the
  current `list_model_library_updates_since` command cannot support the
  frontend handoff.
- Forbidden shared files unless the plan is updated first:
  Rust Pumas access-role setup, `src-tauri/**`, external `../Pumas-Library/**`,
  generated bindings, `Cargo.lock`, diagnostics-ledger DTOs, and saved-workflow
  migration fixtures.
- Contract owner:
  Frontend workflow services own the in-memory selector-row cache and the
  snapshot-to-update-feed handoff. Pumas remains the update cursor/event
  producer and backend commands remain the source of truth.
- Decomposition decision:
  `PumaLibNode.svelte` already has module-level cache state. Move cache
  ownership into a small workflow service helper instead of adding more
  module-script state to the component. Keep LibraryPage and PumaLibNode as
  consumers only.
- Acceptance tests:
  `node --experimental-strip-types --test src/services/workflow/pumaModelOptionsCache.test.ts`,
  `npm run test:frontend`, `npm run typecheck`, and `git diff --check`.
- Acceptance criteria:
  the first selector snapshot cursor is extracted from option metadata; the
  frontend asks the backend for updates since that cursor before publishing a
  reusable cache; stale cursors, snapshot-required feeds, or non-empty update
  feeds invalidate and reload the options once; read-only/update-feed
  unavailable errors preserve the loaded selector rows without spinning or
  polling; and both LibraryPage and graph PumaLibNode share the same cache
  helper.
- 2026-05-07 validation: `node --experimental-strip-types --test
  src/services/workflow/pumaModelOptionsCache.test.ts` passes and covers the
  cursor extraction, update-feed refresh, read-only unavailable-feed fallback,
  and shared cache behavior used by LibraryPage and PumaLibNode.

**Milestone 2 Local-Client Update Feed Slice Boundary:**

- Primary write set:
  `crates/workflow-nodes/src/setup.rs`,
  `src-tauri/src/workflow/puma_lib_commands.rs`, this plan, and source READMEs
  for directories whose API consumer contract changes.
- Adjacent write set:
  `src/services/workflow/pumaModelOptionsCache.ts` only if the frontend command
  response shape changes. Pumas producer code remains read-only.
- Forbidden shared files unless the plan is updated first:
  external `../Pumas-Library/**`, frontend components, generated bindings,
  `Cargo.lock`, diagnostics-ledger DTOs, and saved-workflow migration fixtures.
- Contract owner:
  `workflow-nodes` owns the selector-access adapter. `src-tauri` owns command
  routing from app calls to the active selector access role. Pumas owns the
  local-client subscription handshake and update DTO shapes.
- Decomposition decision:
  Keep the local-client feed conversion inside the selector-access adapter so
  Tauri commands do not learn owner/local-client internals. `puma_lib_commands`
  may gain only a focused helper that clones the extension role before awaiting.
- Acceptance tests:
  `cargo test -p workflow-nodes --features model-library local_client_update_feed`,
  `cargo test --manifest-path src-tauri/Cargo.toml puma_lib_commands`,
  `cargo fmt --all`, and `git diff --check`.
- Acceptance criteria:
  owner access still uses `PumasApi::list_model_library_updates_since`;
  local-client access converts `subscribe_model_library_update_stream_since`
  handshake recovery into `ModelLibraryUpdateFeed`; read-only access reports
  update-feed unavailability without starting lifecycle work; Tauri
  `list_model_library_updates_since` requires the explicit selector-access
  role so update-feed routing has one source of truth.

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
- [x] Identify current fields that are raw facts versus fields that risk
  policy interpretation.
- [x] Decide whether runtime facts extend `RuntimeLifecycleSnapshot` directly
  or use a new wrapper DTO.
- [x] Define naming rules that avoid scheduler-language fields such as
  priority, reservation, admission, eviction, or selected-best-backend.
- [x] Freeze the vertical-slice implementation order and the minimum
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
Runtime facts now use the wrapper `RuntimeFactSnapshot` projected from
`ServerModeInfo`/`RuntimeLifecycleSnapshot`; explicit non-`auto` device facts
are carried through the wrapper instead of extending the lifecycle snapshot.
The implementation strategy now freezes the validated vertical-slice order and
minimum cross-layer fixture requirements, while public inference contract tests
guard representative DTO JSON keys against scheduler-policy terminology such
as admission, reservation, priority, eviction, scheduler-policy, and
selected-best-backend.

### Milestone 2: Align Pumas as Canonical Model Source

**Goal:** Make Pumas the authoritative model identity and artifact-resolution
boundary that feeds inference without making inference own model-library policy.
Detailed Pumas-side work is split into
[pumas-library-plan.md](pumas-library-plan.md).

**Tasks:**
- [x] Treat [pumas-library-plan.md](pumas-library-plan.md) as the Pumas-side
  source for model identity, artifact facts, task evidence, generation defaults,
  custom-code/security facts, backend hints, legacy reference resolution, and
  Pumas/Pantograph fixture expectations.
- [x] Define Pantograph-owned technical-fit candidate derivation from canonical
  Pumas package facts, Pumas dependency facts, backend capabilities, runtime
  registry state, and workflow requirements. Pumas supplies facts only;
  Pantograph owns model/backend candidate projection, exclusion diagnostics,
  live runtime selection, loaded-state interpretation, memory admission, and
  final backend choice. Workflow-service owns the technical-fit request and
  decision DTOs, embedded-runtime projects Pumas package facts plus backend
  capabilities into runtime-registry candidate facts, and runtime-registry
  selection remains the live runtime/admission authority.
- [x] Keep the Pantograph inference plan focused on consuming Pumas package
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
- [x] Subscribe to or poll Pumas model-library update events/cursors so
  Pantograph refreshes affected cached rows when models are added, removed, or
  modified, package facts are invalidated or regenerated, or dependency
  bindings change. `workflow-nodes` now populates package-facts summaries from
  the Pumas startup snapshot cursor, polls `ModelLibraryUpdateFeed` before and
  after sparse-row regeneration, invalidates affected rows, and clears the cache
  when Pumas reports a stale cursor requiring a fresh snapshot.
- [x] Ensure Pantograph consumes versioned Pumas DTO/API outputs only. It must
  not depend on Pumas `models.metadata_json`, SQLite table layouts, or
  search-cache internals. Embedded-runtime `puma-lib` execution no longer reads
  task/backend facts directly from `ModelRecord.metadata`; model entry path,
  model type, task type, and package facts now come from Pumas public execution
  descriptor and package-facts APIs, with saved node inputs retained as the
  compatibility fallback when Pumas lookup is unavailable. The model-list
  option provider now also prefers the Pumas execution descriptor's
  non-`unknown` `task_type_primary` over projected record metadata so stale
  metadata cannot override the versioned DTO. Model-list display metadata now
  also treats sparse, missing, or invalid package-summary API results as
  authoritative DTO outputs: backend hints, custom-code flags,
  custom-code sources, and review reasons use bounded empty/default values
  instead of falling through to raw `ModelRecord.metadata`. Embedded-runtime
  `puma-lib` execution now also
  rebinds stale saved `recommended_backend` values from the Pumas public
  `ModelExecutionDescriptor` when the descriptor provides a concrete backend.
  Workflow-nodes Puma-Lib model-list option metadata now ignores raw
  `ModelRecord.metadata` fallbacks for task, backend hint, custom-code, review,
  and dependency-binding facts; versioned execution descriptors and package
  summary DTOs are used when present, with conservative public-record defaults
  only when versioned facts are absent.
  Embedded-runtime model dependency descriptors now also avoid raw
  `ModelRecord.metadata` for entry-path matching and task fallback: path lookup
  uses public Pumas execution-descriptor `entry_path`, and unknown descriptor
  tasks fall back to saved request data rather than storage metadata.
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
- [x] Record any cross-repo DTO or fixture drift in both plans before
  implementation proceeds. Cross-repo review found no active
  `ResolvedModelPackageFacts`, summary, update-feed, selector snapshot, batch
  hydration, or package-facts fixture wire-shape drift between Pantograph and
  the settled Pumas implementation. Pumas still retains older proposal-level
  planning artifacts that mention feasible execution candidates, but the
  active fast-snapshot and package-facts plans keep candidate/exclusion
  derivation in Pantograph.
- [x] Add a Pantograph Pumas access-role adapter that selects owner API,
  explicit local client, or read-only selector access without relying on hidden
  Pumas client fallback behavior.
- [x] Resolve the Pumas model-library root used by read-only selector access and
  test that the adapter opens the directory containing `models.db`, not the
  launcher/source repository root.
- [x] Replace Library page and graph `puma-lib` model-list population with
  `model_library_selector_snapshot` as the first read path.
- [x] Project selector rows into Pantograph model-list and graph selector DTOs
  using `row.model_ref` as canonical identity, treating `indexed_path` as
  display/debug data only and `entry_path` as executable only when both row
  readiness states are `ready`.
- [x] Hydrate selected or expanded models with Pumas batch APIs for package
  summaries, cheap execution descriptors, and inference settings instead of
  resolving full details for every listed row.
- [x] Connect selector cursors to Pantograph cache invalidation through the
  existing model-library update feed or explicit local-client update stream so
  startup/page snapshots cannot miss updates before polling/subscription begins.
- [x] Pin Pantograph to the published Pumas Library `v0.6.0` release once it is
  available, replacing the moving sibling path dependency for normal
  integration builds so future Pumas development cannot break Pantograph
  unexpectedly. CI-only Pumas fixes after the release are assumed internal
  unless they change public API/DTO contracts. Pantograph now pins the released
  upstream tag at commit `6d038ff8fd955351fbd77daeea4a01a972c6e886`.

**Verification:**
- Review [pumas-library-plan.md](pumas-library-plan.md) against the Pumas
  repository before Pantograph implementation begins.
- Cross-repo fixture review for GGUF, HF-compatible directory, safetensors,
  diffusers bundle, ONNX, missing components, unsupported Ollama hint, and
  custom-code-required cases.
- Model-list cache tests proving startup/page population loads Pumas facts and
  update events/cursors invalidate only affected cached rows.
- Selector integration tests proving Library and graph selectors populate from
  `ModelLibrarySelectorSnapshot` without per-row detail hydration.
- Access-role tests proving owner, local-client, and read-only paths are
  explicit, and read-only access receives the model-library root that contains
  `models.db`.
- Tests proving Pantograph model-list and workflow preflight code do not inspect
  Pumas `metadata_json`, SQLite layout, or search-cache internals.
- `git diff --check`.

**Status:** Partially implemented. Pantograph now has crate-local package-fact
contracts/fixtures that decode the canonical Pumas full-detail producer shape
and an embedded-runtime projection that derives `pumas_package_facts`
technical-fit candidates from those facts. Remote MLX/vLLM search tags do not
project into executable candidates. The executable package-fact fixture set now
includes current-contract safetensors, diffusers bundle, and ONNX artifact
cases, closing a previous Pantograph fixture gap against the Milestone 2
cross-repo review list. The fixture set now also resolves representative Pumas
task evidence through the canonical inference task registry, proving package
task labels and modalities map to typed task ids before backend compatibility
or execution sees them. Backend compatibility now treats a present diffusers
`model_index` component as valid image-generation preprocessing evidence for
Pumas-resolved diffusers bundles, so the fixture shape can pass compatibility
without inventing a generic image-processor fact.
Backend compatibility coverage also verifies the standalone safetensors and
ONNX embedding package-fact fixtures match declared backend source capabilities
and required tokenizer preprocessing through the same factual report path.
The HF-compatible Candle embedding package-fact fixture is now registered in
the public fixture set, resolves to the canonical embedding task, projects into
a backend-loadable source, and passes static Candle backend compatibility
without invoking real model loading.
Inference now exposes Pumas-aligned update feed and package-fact summary
snapshot DTOs. Those DTOs remain useful for selected-model detail refresh, but
the settled Pumas implementation added the faster list-first
`ModelLibrarySelectorSnapshot` contract. Pantograph now uses selector rows for
Library and graph-selector population, preserves the bounded
`ModelLibraryUpdateFeed` invalidation behavior, and hydrates
summary/descriptor/settings detail only for selected or expanded rows. This
closed the user-visible loading problem where Pumas model selectors could wait
on full detail or sparse summary regeneration before showing any rows.
Backend-checked Pumas package-fact
candidate projection now preserves bounded compatibility report and issue
summaries on `RuntimeTechnicalFitCandidate` so rejected/degraded candidates can
be explained without asking Pumas to own candidate derivation or scheduler
selection. Continued model-list review found one remaining consumer-boundary
gap that was superseded by the fast-snapshot slice: the remaining model-list
fallbacks were audited while replacing the initial list path with selector
rows.
Runtime-facing backend hints, dependency binding display facts,
`requires_custom_code`, bounded review reasons from summary diagnostic codes,
bounded custom-code source omission, and API-unavailable inference-settings
fallbacks already prefer Pumas DTOs or bounded defaults over raw record metadata
when those DTOs are available. The selector-snapshot slice removed the active
list-rendering need to inspect Pumas storage internals.

The first Pantograph fast selector implementation slice now populates
`puma-lib` options from Pumas `model_library_selector_snapshot` rows before any
detail hydration. `PortOption` metadata uses `row.model_ref.model_id` as the
canonical identity, keeps `indexed_path` as display/debug metadata, exposes an
executable `entry_path` only when the row reports both entry and artifact states
as ready, and leaves per-model inference settings empty in list rows. The
existing Library page and graph selector both consume this shared
`query_port_options` path. `hydrate_puma_lib_node` now resolves the selected
Pumas model ref directly and hydrates only that selected model through Pumas
package-summary, execution-descriptor, and inference-settings batch/detail APIs
instead of re-running and scanning the full model option list. Later cursor
handoff slices connected selector cursors to frontend cache invalidation.

The selector access-role slice now registers a workflow-nodes selector access
extension that can route selector snapshot reads through owner `PumasApi`,
same-device `PumasLocalClient`, or local read-only `PumasReadOnlyLibrary`.
Explicit configured roots are resolved before discovery fallback, and read-only
setup resolves direct model roots, launcher roots, and Pumas build-output paths
to the model-library directory containing `models.db` without creating a
missing index. `puma-lib` model options now use the selector access extension
as the only selector snapshot source instead of reconstructing selector access
from raw `PUMAS_API` injection. Later selected-detail slices routed
`model_id` hydration through the same explicit selector-access role.

The local-client/detail hydration slice now routes `model_id` hydration through
the explicit Pumas selector-access adapter before falling back to raw owner API
access. Owner and same-device local-client roles hydrate the selected model via
Pumas batch descriptor, package-summary, and inference-settings APIs; read-only
access projects the bounded selector row only, leaving inference settings empty
instead of pretending to own unavailable detail facts. Path-only hydration still
uses owner-side `resolve_pumas_model_ref` because that migration belongs to
Pumas owner access. Validation for this slice passed with
`cargo test --manifest-path src-tauri/Cargo.toml puma_lib_commands`,
`cargo test -p workflow-nodes --features model-library --lib setup`,
`cargo check -p workflow-nodes --features model-library`, `cargo fmt --all`,
and `git diff --check`. A follow-up validation hardening slice added a
Pantograph-side local-client selected-detail IPC fixture proving the same
adapter method calls Pumas batch descriptor, package-summary, and
inference-settings methods for same-device clients.

The Tauri hydration hardening slice removed raw owner `PUMAS_API` fallback for
selected `model_id` hydration. App commands now require the explicit selector
access role whenever a canonical model id is provided, and reserve raw owner
API access for path-only Pumas model-ref migration.

The technical-fit package-facts access slice now snapshots and reapplies the
explicit Pumas selector-access role for workflow execution, and technical-fit
full package-fact resolution consumes only owner selector access. Local-client
and read-only selector roles degrade explicitly because Pumas v0.6.0 exposes
package summaries, not full `ResolvedModelPackageFacts`, through those roles.
If Pantograph needs local-client technical-fit candidates from full package
facts, Pumas needs a full package-facts IPC contract; Pantograph must not
promote selector summaries into full inference facts.

Embedded-runtime `puma-lib` execution now applies the same boundary:
selected `model_id` refresh prefers the explicit selector-access role, read-only
selector rows may rebind executable path/backend/task metadata without being
promoted to full package facts, and owner access is used only for optional full
package-facts enrichment. Validation
passed with `cargo test -p pantograph-embedded-runtime puma_lib`,
`cargo check -p pantograph-embedded-runtime`,
`cargo test -p workflow-nodes --features model-library --lib setup`,
`cargo fmt --all`, and `git diff --check`.

The embedded-runtime execution hardening slice removed the remaining raw
`PUMAS_API` selected-model fallback from `puma-lib` execution. Saved `model_id`
values now rehydrate only through explicit selector access; raw owner API alone
preserves saved node data with an unverified identity instead of silently
rebinding model path/backend facts.

The selected-detail inference-settings execution slice now replaces stale saved
`puma-lib` `inference_settings` with non-empty settings from Pumas selected
detail when owner or local-client selector access can hydrate the selected
model. Read-only selector access continues to leave saved settings intact
because selector rows do not own rich inference-setting detail.

Validation for the selector slice passed with
`cargo test -p workflow-nodes --features model-library --lib puma_lib`,
`cargo test --manifest-path src-tauri/Cargo.toml puma_lib_commands`,
`cargo fmt --all`, and `git diff --check` after Pumas completed its
backend-owned status/resource telemetry performance slice. Validation refreshed
`Cargo.lock` because the current Pumas path dependency removed
`notify-debouncer-mini`; that lockfile refresh is dependency graph drift from
the Pumas producer update and should be committed separately from the selector
code slice. A follow-up dependency slice should pin Pantograph to Pumas Library
`v0.6.0` when the release is published.

Validation for the selector access-role slice passed with
`cargo test -p workflow-nodes --features model-library read_only`,
`cargo test -p workflow-nodes --features model-library test_model_options_use_selector_snapshot_without_detail_hydration`,
`cargo test -p workflow-nodes --features model-library --lib puma_lib`,
`cargo check -p workflow-nodes --features model-library`, and
`cargo fmt --all`, and `git diff --check`.

The selector access hardening slice removed the remaining `puma-lib` model
option fallback that wrapped raw `PUMAS_API` as owner selector access at query
time. Hosts must register the explicit selector access role during setup, so
owner, local-client, and read-only selector paths remain visible at the
Pantograph/Pumas boundary. Validation passed with
`cargo test -p workflow-nodes --features model-library test_model_options_use_selector_snapshot_without_detail_hydration`,
`cargo test -p workflow-nodes --features model-library test_model_options_require_selector_access_instead_of_raw_pumas_api`,
`cargo test -p workflow-nodes --features model-library test_model_options_use_read_only_selector_snapshot_without_pumas_api`,
`cargo test -p workflow-nodes --features model-library --lib puma_lib`,
`cargo fmt --all`, and `git diff --check`.

A follow-up embedded-runtime fixture audit found a stale test path that still
queried `puma-lib` options with raw `PUMAS_API` only. The fixture now registers
`PUMAS_SELECTOR_ACCESS` for the option query while keeping raw Pumas API access
only for selected-model dependency resolution, matching the production
selector/list versus detail/hydration boundary.

The host-binding follow-up applies the same boundary to UniFFI embedded
runtime construction: an optional Pumas handle now registers both raw
selected-detail API access and owner selector access, and UniFFI runtime tests
prove `workflow_graph_query_port_options` can populate `puma-lib` options plus
the selector cursor through that role.

Rustler executor Pumas injection now mirrors the same extension pairing for
BEAM workflow execution resources. Rustler registry port-option queries still
initialize selector/list access through `extensions_setup`, keeping that
resource boundary explicit instead of treating raw `PUMAS_API` as a selector
fallback.

The selector cursor handoff slice now centralizes frontend Pumas model-option
caching in `src/services/workflow/pumaModelOptionsCache.ts`. The service loads
selector rows through `query_port_options`, extracts the selector cursor from
row metadata, asks `list_model_library_updates_since` for changes since that
cursor before publishing reusable cache state, and reloads once when the feed
reports a stale cursor, required snapshot, or changed rows. LibraryPage and
PumaLibNode both consume the shared cache, and LibraryPage's manual refresh
explicitly invalidates it. If the update-feed command is unavailable, such as
read-only selector access without owner API, the frontend preserves the loaded
snapshot and does not start a polling loop.

The cache freshness follow-up now checks the cached selector cursor before
returning cached rows, reloads when the update feed reports changes after the
initial handoff, and gives manual Library/Puma-Lib refresh actions a
force-refresh path that bypasses the cache. A later 2026-05-08 hardening slice
narrows frontend cache preservation to the explicit read-only/no-update-feed
boundary; other update-feed failures now propagate instead of silently serving
cached selector rows.

The legacy `modelName` rehydration follow-up removed execution/preload lookups
that scanned the full Pumas library by display name. `puma-lib` execution now
uses Pumas detail hydration only when a canonical `model_id`/model ref is
available; otherwise it preserves saved path data without list scanning.

Validation for the selector cursor handoff slice passed with
`node --experimental-strip-types --test src/services/workflow/pumaModelOptionsCache.test.ts`,
`npm run test:frontend`, `npm run typecheck`, and `git diff --check`.

The local-client update-feed slice now routes Tauri
`list_model_library_updates_since` through the explicit selector-access role
instead of requiring raw owner `PUMAS_API`. Owner access still uses
Pumas' list API; local-client access converts Pumas'
`subscribe_model_library_update_stream_since` recovery handshake into the same
`ModelLibraryUpdateFeed` DTO used by the frontend cache handoff; and read-only
selector access reports update-feed unavailability without starting lifecycle
work. The command clones extension handles before awaiting Pumas APIs or IPC so
the Tauri shared-extension lock is not held across async work. Raw `PUMAS_API`
injection is intentionally not a fallback for update-feed routing because the
selector-access role is the Pantograph boundary source of truth.

The summary snapshot slice now routes Tauri package-facts summary snapshot and
single-summary commands through the same selector-access role. Owner access can
call Pumas summary APIs directly; local-client and read-only access project
bounded selector-row summary status and cached summary facts without requiring
raw owner `PUMAS_API` for list views.

The host-binding selector parity slice now makes UniFFI and Rustler Pumas
resources carry an explicit owner selector-access role and routes public
package-facts summary snapshot, single-summary, and update-feed methods through
that role. This keeps host-language bindings aligned with Tauri and
workflow-node selector/list freshness boundaries while preserving raw owner API
access only for owner operations that are not selector/list reads.
Validation for this slice passed with `cargo check -p pantograph-uniffi`,
`cargo check -p pantograph_rustler`, and
`cargo test -p pantograph-uniffi pumas`. `cargo test -p pantograph_rustler pumas`
still cannot link as a standalone Rust test binary in this workspace because
Rustler's Erlang NIF symbols (`enif_*`) are unresolved outside the BEAM loader;
this is a host-binding validation limitation, not a selector-access contract
failure.

Validation for the selector-access summary slice passed with
`cargo test -p workflow-nodes --features model-library local_client_update_feed`,
`cargo test -p workflow-nodes --features model-library read_only_update_feed`,
`cargo test -p workflow-nodes --features model-library --lib setup`,
`cargo test --manifest-path src-tauri/Cargo.toml puma_lib_commands`,
`cargo check -p workflow-nodes --features model-library`,
`cargo fmt --all`, and `git diff --check`.

The Pumas release-pin slice is implemented as of 2026-05-08. The published
`v0.6.0` tag now points at commit `6d038ff8fd955351fbd77daeea4a01a972c6e886`
and the Rust workspace package metadata declares version `0.6.0`, so
Pantograph now pins `pumas-library` to that released git tag instead of the
sibling checkout path.

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
- [x] Define task request contracts aligned with Transformers task semantics
  for text generation/chat, embeddings, rerank, image/depth/audio/video-ready
  extensions, and multimodal payloads. Public inference integration coverage
  now freezes the task request contract matrix for text generation, chat,
  embeddings, rerank, image generation, image understanding,
  audio transcription, video understanding, and multimodal generation,
  including input/result kind, execution support, streaming support, and
  required/output modalities. Node-engine now has an executable
  `image_generation` request-construction slice for canonical `llm-inference`,
  including direct and `task_options` image controls, Pumas package-facts
  forwarding, and typed gateway projection without diagnostic prompt/image
  payload leakage. The contract matrix now also includes a roadmap
  `depth_estimation` task aligned to image input and depth-map/point-cloud
  output semantics without making the existing DepthPro node an inference
  backend path. Follow-up validation found embedded-runtime workflow capability
  and technical-fit projections were not yet exhaustive over the new task id;
  those mappings now preserve `depth_estimation` as a roadmap workflow task
  contract and classify its result family separately from generated images.
  The inference crate now exposes serializable `DepthEstimationRequest` and
  `DepthEstimationResult` typed DTOs for that registry contract while keeping
  the task `execution_supported=false`, and node-engine task-validation
  lifecycle coverage now rejects canonical `depth_estimation` before backend
  execution with bounded option diagnostics. Depth-estimation input and result
  serde coverage now freezes the roadmap task's artifact-ref-oriented request
  and result shape without promoting it to an executable backend path.
- [x] Define the strong task registry shape, including canonical task id,
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
- [x] Define which fields are stable Pantograph contracts versus backend-local
  extras.
- [x] Define serde/wire casing, enum tagging, optional field defaults, and
  unknown-field behavior for every contract that crosses Rust/Python,
  Rust/Tauri/TypeScript, Pumas/inference, saved workflow, or external HTTP
  runtime boundaries. Public inference integration coverage now freezes the
  canonical `InferenceExecutionRequest` wire shape for snake_case fields,
  internally tagged task input payloads, absent optional fields, null
  `extra_options`, and additive unknown request/input fields. Inference
  lifecycle event coverage now also freezes snake_case phase/kind decoding,
  absent optional field defaults, empty diagnostics vectors, and additive
  unknown-field tolerance for diagnostics producers that evolve independently.
  Public task request contract integration coverage now freezes snake_case
  task/input/result/streaming/modality labels, default omission of empty
  modality arrays, execution support preservation, and additive unknown-field
  tolerance at the crate boundary. PyTorch Rust/Python worker contract coverage
  now also freezes additive unknown-field tolerance for load, generate, and
  response envelopes while preserving backend-local `transformers_kwargs`
  allowlist enforcement. Backend stream chunk coverage now freezes
  append-only `ChatChunk.usage` serialization so absent usage is omitted and
  bounded token counts remain nested metadata rather than prompt/result
  payloads. The same append-only stream contract now covers
  `ChatChunk.cache_handle_id`, with absent handles omitted and present handles
  serialized as stable backend-local ids rather than KV bytes, prompt/result
  text, or runtime reuse policy. Depth-estimation request/result coverage now
  also freezes snake_case task/input/result labels, null/absent optional
  defaults, and additive unknown-field tolerance for future Transformers-style
  depth metadata.
- [x] Add validation rules so internal code consumes parsed model/task types
  rather than raw strings or ad hoc JSON.
- [x] Document that Python Transformers is one implementation of these
  contracts, not the source of public truth.

**Verification:**
- Review against `RUST-API-STANDARDS.md` parse-once and typed public contract
  requirements.
- Review against `ARCHITECTURE-PATTERNS.md` immutable contract guidance.
- [x] Unit tests for model source parsing and invalid-state rejection once code is
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

**Status:** Implemented. The first task-registry slice added seeded
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
Resolved model sources now expose `validate_for_backend_load()` so adapters and
saved-workflow migrations can reject malformed Pumas-resolved sources, direct
sources that incorrectly carry Pumas identity, unknown source/artifact kinds,
empty entry paths, invalid artifacts, and Hugging Face repo sources without a
repo id before backend loading. Contract tests also freeze omitted optional
collection defaults and additive unknown-field behavior for this DTO.
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
Generation option contract tests now freeze the stable typed option groups
versus backend-local `backend_extensions` escape hatches: extension keys are
scoped as `<backend-or-adapter>:<option>`, missing option groups default, and
unknown future groups are additive rather than public raw kwargs.
`GenerationOptions::backend_extension_scope_diagnostics()` now enforces that
stable scoping rule with bounded `OptionCompatibilityDiagnostic` entries,
letting adapters reject unscoped backend extensions without interpreting them
as canonical generation fields.
The PyTorch/Transformers and llama.cpp generation-option mappers now consume
that shared scope diagnostic before backend-local extension mapping, so raw
kwargs are rejected consistently while correctly scoped foreign extensions
remain backend-specific unsupported diagnostics.
Node-engine text/chat request construction now validates supplied `task_kind`
and `taskKind` labels through the inference task registry. Missing task labels
still default to text generation for existing text nodes, while `task_id` and
`taskId` stay workflow/node identity fields instead of task-kind aliases. For
public task-kind inputs, non-string labels, unknown labels, or labels that
resolve to non-text tasks now fail before gateway/backend execution instead of
silently collapsing to text generation.
The task registry now publishes canonical typed request/result payload contracts
through `TaskRequestContract`, including executable mappings for text/chat,
embedding, rerank, and image generation plus visible contract-only mappings for
audio, video, image-understanding, and multimodal roadmap tasks. Typed request
validation now consumes this contract instead of carrying an independent
task/input table, and `crates/inference/src/README.md` documents the consumer
rule that payload compatibility comes from the registry contract rather than
backend names or raw task strings. Neighboring graph/runtime consumer migration
remains tracked under Milestone 14.
Typed request serde coverage now freezes the stable wire shape for current
executable text, embedding, rerank, image-generation, and audio-transcription
inputs, including tagged `input_type` payloads and append-only `extra_options`
where backend-local options are still needed.
Contract-only typed request/result wire shapes now also cover image
understanding, video understanding, and multimodal generation payloads with
artifact-reference-friendly media fields while preserving unsupported-task
validation until executable backends exist. Depth estimation is now seeded as a
contract-only roadmap task with typed request/result DTOs and validation
coverage while remaining non-executable.
Request wire-shape coverage now also freezes the top-level stable
`InferenceExecutionRequest` fields and keeps backend-local escape hatches under
`generation_options.backend_extensions` or `extra_options`, preventing raw
Transformers kwargs, backend CLI flags, or scheduler policy fields from
becoming public request contract keys.
Minimal typed request decode coverage now proves omitted optional fields
default to `None`/`null` and unknown future request fields are additive while
still validating through the parsed task/input contract.
Executable task-registry invariant coverage now freezes the complete public
shape for every seeded task entry: unique canonical ids, known family,
execution-behavior, streaming, and support-tier classifications, non-empty
modality/result facts, a typed request contract, and labels that avoid
scheduler/runtime policy language.
Typed execution request validation now rejects whitespace-only text prompts,
blank embedding items, blank rerank queries, and blank rerank documents at the
contract boundary, with indexed validation errors for list payloads so
backends do not receive semantically empty task inputs.
The first neighboring node-engine consumer slice now uses the exported
`TaskRequestContract` to decide whether a canonical task can be built through
the text-generation typed request path. Text/chat aliases still map to the text
payload contract, while embedding and image-generation task aliases are rejected
because their registry contracts require different input payload families.
Workflow-service backend task capability DTOs now carry optional
`WorkflowTaskRequestContract` payload metadata, and embedded-runtime projects
that metadata from inference backend task facts into workflow runtime
capabilities. Node-engine canonical `llm-inference` dispatch now also uses
`TaskRequestContract` input payload families to route embedding and rerank
tasks, so dispatch no longer needs to treat those task ids as special backend
selectors. Node-engine typed result projection now checks the task contract's
`result_kind` before unpacking text, embedding, or rerank output payloads,
keeping projection errors tied to canonical request/result semantics while
preserving the existing graph-visible output shapes.
Node-contract and workflow-node descriptor projections now also carry
transport-neutral inference task and port payload metadata for the canonical
`llm-inference` text/chat, embedding, and rerank families. This makes graph
authoring and workflow consumers able to inspect request/result families from
backend-owned contracts while leaving frontend `NodeDefinition` rendering and
runtime backend selection unchanged. The workflow-node projection now derives
the task contract fields from `inference::model_contracts` rather than carrying
a separate task detail table.
The canonical `llm-inference` descriptor now declares `task_kind` and
`runtime_hint` as optional graph-visible inputs, aligning the descriptor with
the saved-workflow migration data schema, workflow preflight, and node-engine
execution inputs that already consume those fields.
The task-request wire-shape tests now assert stable snake_case labels, omitted
collection defaults, and additive unknown-field behavior for task contracts.
`crates/inference/src/README.md` also records that Python Transformers is a
backend implementation target for the Rust-owned contracts rather than the
public contract source of truth.
Legacy llama.cpp and PyTorch migration now writes generation options into the
canonical grouped `GenerationOptions` shape (`sampling.temperature` and
`length.max_new_tokens`) instead of flat fields. Node-engine text-generation
request construction still accepts the older flat migrated shape for
compatibility, with grouped fields taking precedence.
Node-engine rerank request construction now reads migrated canonical
`task_options.top_k` and `task_options.return_documents` values, while
connected/top-level runtime inputs keep precedence over saved task options.
Workflow-node contract projection now advertises audio transcription on
canonical `llm-inference` from the inference registry: the `audio` input maps
to audio transcription input, `response` maps to the transcription result, and
`execution_supported = true` reflects the typed gateway/backend boundary while
artifact payload resolution remains host-owned.
Node-engine typed request construction now also derives `PumasModelRef`
identity from `resolved_model_source.model_ref` when a canonical node receives
fully resolved Pumas model-source facts instead of a separate
`pumas_model_ref` input.
Node-engine canonical `llm-inference` dispatch still rejects any remaining
contract-only task registry entries before gateway or backend preflight
execution. `audio_transcription` now routes through the typed audio
transcription gateway path instead of falling through to text/prompt handling.
The canonical llama.cpp execution path now derives request generation
parameters from grouped `generation_options` first, while retaining legacy
top-level `max_tokens` and `temperature` inputs as fallback compatibility.
Node-engine dependency preflight coverage now includes a positive
HF-compatible Transformers/PyTorch fixture proving Pumas-resolved model-source
identity, entry path, task semantics, and the canonical PyTorch backend key
reach the host dependency resolver before execution.
Node-engine task-contract validation failures now project bounded
task-validation lifecycle events through the existing host-owned inference
lifecycle sink when available, including request id, backend key, runtime id,
model id, and the contract failure detail.
Node-engine dependency preflight can now receive stable task/execution context
from canonical node dispatch and emit bounded model-package-resolution lifecycle
failure facts through the existing host-owned inference lifecycle sink without
inventing node-engine ledger writes.
The 2026-05-06 Milestone 3 checklist reconciliation closed the remaining
task-request and wire-shape checklist items against existing executable
coverage: `task_request_contracts_publish_transformers_aligned_execution_matrix`
freezes the Transformers-aligned matrix for text/chat, embeddings, rerank,
image generation, image understanding, depth estimation, audio transcription,
video understanding, and multimodal generation;
`task_request_contract_wire_shape_preserves_snake_case_defaults_and_unknown_fields`
freezes snake_case task contract labels, omitted collection defaults, and
additive unknown-field tolerance; and
`inference_execution_request_wire_contract_preserves_tags_defaults_and_unknown_fields`
freezes canonical request tagging, optional defaults, and additive top-level
and input-payload fields. Validation passed with those three focused
`cargo test -p inference ...` filters and `git diff --check`.

### Milestone 4: Define Neutral Managed-Dependency Boundary

**Goal:** Make one Pantograph-owned dependency manager the source of truth for
runtime binaries, media tools, native artifacts, status, leases, activation,
and command resolution.

**Tasks:**
- [x] Define the target crate/module boundary for managed dependencies, such as
  `pantograph-managed-dependencies`, including whether the first slice extracts
  code or introduces an adapter facade over existing inference modules.
- [x] Define dependency ownership explicitly so runtime sidecar dependencies,
  media tool dependencies, and native artifact dependencies are declared by the
  crate/package that owns their execution or command resolution.
- [x] For any new dependency or native artifact, record why in-house code is
  insufficient, transitive dependency cost, license/security notes, platform
  support, and feature-gating behavior.
- [x] Define typed dependency categories for runtime sidecars, media tools, and
  native artifacts without making those categories scheduler-loadable runtime
  policy.
- [x] Define common status, install state, selected/default/active version,
  expected-file, missing-file, lease, activation, and resolved-command DTOs.
- [x] Define which operations are common and which remain category-specific:
  runtime command resolution, media executable resolution, native artifact
  activation, lease acquisition/release, install/remove, and catalog refresh.
- [x] Define migration/import behavior for existing inference managed runtime
  and managed redistributable state files.
- [x] Document that inference consumes runtime executable facts from this
  boundary and media conversion consumes ffmpeg/OIIO/OCIO facts from the same
  boundary.
- [x] Keep scheduler admission, runtime reservation, runtime retention, and
  workflow policy out of the managed-dependency owner.

**Verification:**
- Boundary review against `ARCHITECTURE-PATTERNS.md`.
- Dependency ownership review against `DEPENDENCY-STANDARDS.md`.
- Dependency review against `RUST-DEPENDENCY-STANDARDS.md` for feature
  ownership, optional dependency declarations, and workspace inheritance.
- API review against `RUST-API-STANDARDS.md` for typed enums/newtypes and
  append-only DTOs.
- `git diff --check`.

**Status:** Contract boundary established. `pantograph-managed-dependencies`
now owns the neutral managed dependency DTO surface for runtime sidecars, media
tools, native artifacts, status/readiness/install state, selected/default/active
version state, expected and missing files, leases, native artifact activation,
resolved command facts, and operation scopes. It also owns the media
redistributable implementation for ffmpeg/OIIO/OCIO catalog, state, staging
install, activation, lease, and removal behavior. Runtime sidecar implementation
still lives in inference behind neutral adapters. The crate must not perform
network downloads, process spawning, scheduler admission, runtime reservation,
workflow policy, frontend projection, or converter execution.

**Dependency review:** The managed-dependency boundary slices added internal
workspace dependencies on `pantograph-managed-dependencies` from inference,
workflow-service, and media-conversion. No new third-party crate, binary,
native artifact, license obligation, platform artifact, or feature-gated
external dependency was introduced. The managed-dependency crate uses existing
workspace `serde`, `serde_json`, and `uuid` dependencies for DTO
serialization, state persistence, and lease/install staging ids.

**State migration/import behavior:** The neutral managed-dependency owner must
preserve the existing app-data roots as import sources before it writes a new
canonical state file. Runtime sidecar state imports from
`third-party/managed-runtime/state.json` plus the legacy `runtimes/` fallback
already used by managed runtime path resolution. Media tool/native artifact
state imports from `third-party/managed-dependencies/state.json`; legacy
install directories under `managed-dependencies/<id>/versions/<version>` remain
read-only install-root fallbacks, but legacy `managed-dependencies/state.json`
must be either imported by an explicit migration slice or rejected with a
typed, user-visible unsupported-legacy-state diagnostic before the inference
state owner is removed. Import must be idempotent: if the neutral state file
already exists, it is authoritative and legacy files are read only for explicit
repair/migration commands. Active leases must fail closed during migration:
unknown, malformed, or stale lease records are not silently promoted to active
neutral leases.

### Milestone 5: Migrate Binary and Media Dependency Ownership

**Goal:** Move inference away from owning all managed binaries while preserving
runtime launch behavior and media conversion readiness facts.

**Tasks:**
- [x] Inventory current uses of `managed_runtime`, `managed_redistributables`,
  `managed_binaries`, and `managed_media_dependencies`.
- [x] Move or adapt runtime sidecar management so inference resolves llama.cpp
  commands through the neutral managed-dependency boundary.
- [x] Move or adapt ffmpeg, oiiotool, ocioconvert, and OpenColorIO dependency
  status/lease behavior out of inference and into the neutral
  managed-dependency/media-conversion boundary.
- [x] Update `pantograph-media-conversion` and workflow artifact conversion
  consumers so conversion dependency facts do not come from inference exports.
- [x] Keep `pantograph-media-conversion` responsible for command planning and
  eventual conversion execution contracts.
- [x] Add temporary compatibility shims only where needed, and document their
  removal trigger.
- [x] Update READMEs so inference is not described as the owner of media
  dependencies.

**Verification:**
- `rg -n "managed_redistributables|managed_media_dependencies|ManagedRedistributable|Ffmpeg|OpenColorIo|Oiiotool|Ocioconvert" crates/inference`
  reviewed for intentional remaining references.
- Managed-dependency crate tests once the crate/module exists.
- `cargo test -p inference`.
- Relevant `pantograph-media-conversion` and workflow-service tests selected
  by touched files.
- `git diff --check`.

**Status:** Implemented for the current boundary. Inventory is complete and
inference now depends on the neutral `pantograph-managed-dependencies` contract
crate. Media redistributable catalog, install state, activation, selection,
lease, removal, and status records moved to
`pantograph-managed-dependencies::redistributables`; inference keeps only
private projection helpers that adapt those facts into
`ManagedDependencyStatus` for aggregate status callers. Runtime sidecar
catalog, install state, job state, and command resolution still live in
`crates/inference/src/managed_runtime/` because llama.cpp launch is an
inference-owned runtime concern, but runtime status and command resolution now
project through neutral managed-dependency facts,
`ManagedDependencyKey::RuntimeSidecar(..)`, and
`ResolvedManagedDependencyCommand`. Media planning moved to
`pantograph-media-conversion`; the inference media-planning module is now a
legacy wire-shape adapter over media-conversion-owned plan, activation,
holder-validation, release, and executable-resolution functions. The remaining
managed-binary facade is limited to runtime launch command compatibility for
Tauri process startup and resolves through the neutral runtime sidecar command
path; the legacy aggregate managed-binary status DTOs and
`list_managed_binary_statuses` export have been retired. UniFFI and Tauri
managed-media action/status commands now import redistributable operations
directly from `pantograph-managed-dependencies`, and desktop Settings now calls
the neutral `list_managed_dependencies` Tauri command returning
`ManagedDependencyStatus`.

Runtime sidecar command resolution is exposed through
`resolve_managed_dependency_command(ManagedDependencyKey::RuntimeSidecar(..))`;
media tool command resolution and native artifact activation explicitly reject
there so conversion execution remains owned by media conversion.
Artifact-format dependency-version synchronization now derives from neutral
`ManagedDependencyStatus` values through
`ArtifactFormatDependencyVersions::from_managed_dependency_statuses`, filtering
runtime sidecars out and using stable dependency keys such as `opencolorio` for
media/native dependencies.
`pantograph-media-conversion` now has an explicit key bridge between
`ManagedMediaDependencyId` and neutral `ManagedDependencyKey`, and it rejects
runtime sidecar keys as non-media-conversion dependencies. Embedded workflow
runtime capability projection reads runtime sidecar snapshots directly instead
of an aggregate managed-binary facade, so workflow capabilities no longer
stitch media/native dependency status through a runtime-only surface while
preserving runtime install/remove readiness facts.

Temporary compatibility shims are limited to the legacy inference
media-planning DTO adapter and the legacy managed-binary facade that still
returns `ResolvedCommand`. Removal trigger: delete those shims after Tauri
process startup consumes `ResolvedManagedDependencyCommand` directly and no
callers rely on the inference-shaped media conversion plan DTOs.

**Implementation findings:** Do not move media conversion DTOs by type alias
without a JSON compatibility decision: inference `MediaConversionJobKind::ThreeD`
does not currently serialize the same way as `pantograph-media-conversion`'s
`3d` media-kind shape. The OpenColorIO identifier also has multiple public
spellings across surfaces (`open_color_io` serde, `opencolorio` key/display
paths), so the migration needs an explicit compatibility decision before moving
DTO ownership. Current install directory fallback covers legacy version
directories, and the neutral redistributable state loader now imports legacy
`managed-dependencies/state.json` only when the canonical
`third-party/managed-dependencies/state.json` file is absent. Imported legacy
state normalizes schema version `0` and clears active leases so stale process
ownership is not promoted to neutral leases.

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
That fourth-slice compatibility path was later removed: workflow-service now
leaves `ollama-inference` unsupported instead of rewriting it to a canonical
placeholder.
The fifth slice made the persisted `ollama_vlm_model` setting a scrubbed
compatibility field, stopped configured startup requests from forwarding it, and
removed the frontend model-setting/runtime-manager type surfaces that could
reintroduce Ollama as a user-selectable runtime. The sixth slice deleted the
retired inference Ollama backend implementation and per-platform managed
Ollama adapters while keeping small unsupported compatibility definitions for
old serialized runtime ids. Milestone 6 is implemented; remaining Ollama
references are intentional migration guards, unsupported-id compatibility,
tests/fixtures, or historical documentation. A follow-up cleanup removed
Ollama from the Tauri Pumas dependency-runtime probe backend-binding candidate
matrix so Pumas Ollama compatibility facts are ignored instead of queried as
Pantograph-executable backend options.
A later tooling cleanup removed the deleted Ollama managed-runtime platform
directory from the decision-traceability structured-producer defaults, keeping
the guardrail focused on active producer paths instead of retired backend code.

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
- [x] Replace direct/ad hoc PyTorch model loading paths with a clear
  Transformers-backed model load path where compatible.
- [x] Map Pumas-resolved Rust model-source and task contracts to `AutoModel`,
  `AutoTokenizer`, processors, chat templates, and generation config inside the
  Python worker.
- [x] Map Rust generation option groups to Transformers generation inputs and
  return per-option compatibility diagnostics instead of accepting arbitrary
  Python kwargs as the public contract.
- [x] Map Rust task registry entries to Transformers pipeline/model loading
  behavior inside the worker while keeping upstream Python task names as
  adapter-local implementation details after validation.
- [x] Make `trust_remote_code` and custom-code loading explicit inputs from the
  validated contract and default them closed unless an accepted policy says
  otherwise.
- [x] Keep Python-specific compatibility shims and kwargs inside the PyTorch
  backend worker modules.
- [x] Define the Rust-to-Python worker envelope, init/shutdown lifecycle,
  error mapping, request correlation, cancellation behavior, and schema
  fixtures before changing worker behavior.
- [x] Preserve dLLM/Sherry-specific behavior if it still requires custom worker
  logic, but describe it as backend-local support rather than a public
  Pantograph model standard. PyTorch worker envelopes now carry backend-local
  masked prompt JSON, denoising step, and block length controls through
  Rust/Python worker fixtures and projection tests while public Rust callers
  continue to default those fields to absent.
- [x] Normalize Transformers/Python errors into Pantograph backend errors and
  runtime facts. The non-streaming PyTorch generate-text worker response now
  decodes through the same typed worker-response normalization pattern as model
  load and stream setup: `runtime_unavailable` maps to `BackendError::NotRunning`,
  `generation_failed` maps to `BackendError::Inference`, request ids and
  canonical worker error codes are retained, and success returns generated text.
  PyTorch worker initialization failures now map to
  `BackendError::StartupFailed` with a generated request id and canonical
  `pytorch_worker_init_failed` code. PyTorch worker initialization now also
  validates a versioned `init_worker` envelope after sibling-module
  registration and decodes a typed initialization response before `start()`
  proceeds to model loading or ready-state publication. PyTorch backend health
  checks now reuse the same typed init-worker boundary when the backend is
  ready, returning `false` on init-envelope failure instead of treating raw
  module importability as the only health signal.
  PyTorch model-load worker transport failures now also route through the typed
  worker failure shape with the load request id and canonical
  `pytorch_worker_model_load_failed` code.
  PyTorch audio-transcription worker transport failures now share the text
  generation runtime-unavailable classifier, so `No model loaded` failures map
  to `BackendError::NotRunning` with request id and canonical
  `pytorch_worker_audio_transcription_failed` code rather than a generic
  inference failure.
  PyTorch Python load-envelope validation is now separated from actual
  Transformers loading: envelope/projection `ValueError`s remain
  `invalid_request`, while loader-time `ValueError`, `OSError`, `ImportError`,
  missing-file failures, and unexpected loader-time exceptions return
  `model_load_failed` with canonical `pytorch_worker_model_load_failed` so
  tokenizer/model `from_pretrained` failures surface as startup failures rather
  than config drift.
  PyTorch audio-transcription worker lookup and ASR invocation failures now
  retain a generated request id and canonical
  `pytorch_worker_audio_transcription_failed` code.
  PyTorch model unload worker lookup and cleanup failures now retain a
  generated request id and canonical `pytorch_worker_unload_failed` code.
  PyTorch live-KV helper worker lookup and save/restore/clear failures now
  retain generated request ids and canonical `pytorch_worker_kv_*_failed`
  codes without exposing cache bytes or file paths in the canonical code.
  PyTorch live-KV loaded-model and live-KV metadata extraction failures now
  use the same canonical KV worker failure shape when Python returns no active
  model or malformed KV metadata. PyTorch KV-cache truncation temp-file
  read/write failures now also use the canonical
  `pytorch_worker_kv_truncate_failed` path so local temp paths are sanitized
  before backend errors can become workflow diagnostics.
  PyTorch backend trait KV slot save/restore/clear/truncate worker failures
  now use the same canonical KV worker failure shape.
  Non-streaming PyTorch generate-text worker transport failures now also retain
  the generated request id and canonical `pytorch_worker_generate_text_failed`
  code instead of returning raw Python bridge errors.
  PyTorch streaming worker lookup failures, setup call failures, generator
  creation failures, iterator failures, token extraction failures, and
  generator item errors now also route through the same canonical stream worker
  failure code and generated request id instead of returning ad hoc raw Python
  exception strings. PyTorch worker error-kind normalization coverage now
  freezes every worker error kind's mapped `BackendError` variant while
  preserving request ids, canonical codes, and bounded worker messages across
  load, stream setup, and generate response boundaries. PyTorch worker
  transport exception messages now also strip Python traceback frames and
  redact local path tokens before becoming backend errors, while retaining
  request ids, canonical worker codes, and bounded exception summaries.
  Structured PyTorch worker error responses now apply the same traceback-frame
  stripping, local-path redaction, and bounded summary rules before converting
  to `BackendError`, closing the remaining path where Python worker
  `error.message` text could carry traceback frames or local paths.
  PyTorch task-join failures now route through the same sanitizer before
  becoming `BackendError` messages, closing the Rust async task-boundary path
  where panic text or join diagnostics could otherwise carry traceback frames
  or local paths. Malformed PyTorch worker response JSON now also routes
  through canonical worker failure helpers for model load, non-streaming
  generation, and stream setup, preserving generated request ids and canonical
  worker codes instead of returning raw decode errors. PyTorch audio
  transcription worker responses now decode through the typed worker response
  envelope so missing or malformed `text`, `language`, and `duration_seconds`
  fields, malformed response JSON, and structured Python worker failures route
  through canonical audio worker errors instead of silently returning
  empty/default transcript data. Typed PyTorch worker response decoders now
  enforce caller-generated request-id correlation for model load, generation,
  stream setup, and audio transcription before trusting success or structured
  error payloads.
  PyTorch audio transcription requests now cross the Rust/Python worker
  boundary through the same versioned envelope contract as load and generation,
  with Rust and torch-free Python projection tests covering operation/version
  validation, required model/audio inputs, additive-field tolerance, and
  fail-closed handling for unsupported non-null `extra_options`.
  PyTorch model unload now also crosses the Rust/Python worker boundary through
  a versioned envelope and typed response decoder, with request-id correlation,
  malformed-response rejection, structured invalid-request mapping, and
  Rust-side loaded-model state cleared only after a correlated unload success.
  PyTorch loaded-model info now crosses the embedded Python boundary through a
  versioned `get_loaded_info` worker envelope and typed response decoder,
  preserving request-id correlation and canonical sanitized KV loaded-info
  errors before node-engine KV-cache preparation consumes active model facts.
  PyTorch live-KV clear now crosses the Rust/Python boundary through a
  versioned `clear_kv_cache` worker envelope and typed response decoder shared
  by the free helper and trait slot-clear path after slot validation.
  PyTorch live-KV save and restore now cross the Rust/Python boundary through
  versioned `save_kv_cache` and `restore_kv_cache` worker envelopes and typed
  live-KV metadata response decoders shared by the free helpers and trait
  slot save/restore paths after slot validation.
  PyTorch persisted KV truncation now crosses the Rust/Python boundary through
  a versioned `truncate_kv_cache` worker envelope and typed response decoder
  before the temporary KV file is read back.
  PyTorch backend `stop()` now uses the same versioned unload worker envelope
  as async model unload for best-effort synchronous cleanup, logging cleanup
  failures instead of calling raw Python worker lifecycle functions directly.

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
trust-policy opt-in. Direct and Pumas-resolved model loading now use the typed
worker envelope for compatible load paths. Remaining work is to preserve
dLLM/Sherry local behavior as documented backend-local support and normalize
remaining Python/Transformers errors into typed backend/runtime facts.

Update during implementation:
- 2026-05-04: PyTorch direct local model loading now builds the same
  `load_transformers_model` worker envelope as Pumas-resolved package loading,
  representing the direct path as `DirectHfCompatibleDirectory` import/debug
  source rather than a Pumas-owned model. The Python worker remains the
  adapter-local owner of `AutoModelForCausalLM`, `AutoTokenizer`,
  `AutoModelForSpeechSeq2Seq`, `AutoProcessor`, chat template repair, and
  generation-config behavior behind validated task-profile loader labels.
- 2026-05-04: Rust can now derive a Transformers load envelope from Pumas
  package facts with contract-version, artifact-kind, task-evidence,
  generation-default, and custom-code trust validation before invoking Python.
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
Load-envelope validation now runs
`ResolvedModelSource::validate_for_backend_load()` when that source is present,
so malformed Pumas-resolved sources are rejected before Python worker loading.
PyTorch/Transformers load envelopes also include a backend-local task profile
derived from the canonical task registry, mapping validated text/chat tasks to
the current causal-LM loader and rejecting registry-resolved tasks that do not
yet have a PyTorch loader instead of matching raw task strings ad hoc.
PyTorch static backend capability facts now advertise audio transcription
alongside text generation, and the backend-local Transformers task profile maps
canonical `audio_transcription` / `automatic-speech-recognition` evidence to an
ASR loader family. The Python worker now validates the Rust-derived task
profile loader, dispatches causal-LM profiles to `AutoModelForCausalLM` and ASR
profiles to the existing `pipeline("automatic-speech-recognition")` path, and
preserves the Rust trust policy/local-files/revision/cache facts during both
load paths instead of switching on raw upstream task strings.
Broader PyTorch-feature validation exposed a stale worker-contract fixture that
still passed `use_cache` as a raw `transformers_kwargs` field after the
generation envelope allowlist was narrowed to `top_k`; the fixture now matches
the typed Rust/Python validation contract.
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
The PyTorch non-streaming generation path now sends a typed `generate_text`
worker envelope instead of ad hoc PyO3 keyword arguments. Python validates the
envelope through `worker_contract.py`, adapts it to the existing backend-local
`generate` implementation, and returns the Rust worker response envelope so
generation failures normalize through `PyTorchWorkerFailure` with request
correlation. Streaming generation and full typed `GenerationOptions` threading
remain follow-up slices.
The PyTorch streaming generation path now sends the same typed
`PyTorchGenerateTextRequest` payload through a `generate_text_stream` worker
envelope before delegating to the existing Python `generate_tokens` generator,
keeping streaming-specific Python kwargs behind the backend adapter boundary.
PyTorch text generation request builders now thread the typed `sampling.top_k`
generation option into backend-local Transformers kwargs for both streaming and
non-streaming worker envelopes, while leaving broader option threading as a
follow-up.
PyTorch generation envelope validation now rejects unallowlisted
`transformers_kwargs` in both Rust and torch-free Python worker-contract
projection. `top_k` remains the only current adapter-local forwarded kwarg, so
raw Transformers kwargs and scheduler-like policy keys cannot bypass the typed
Rust generation option boundary.
Python-specific compatibility shims and Transformers kwargs are now bounded to
the PyTorch backend worker surface: Rust worker DTOs remain `pub(super)`,
backend-local mappers create Transformers kwargs, `worker_contract.py` projects
validated envelopes into Python calls, and backend README guidance describes
these fields as adapter-local rather than Pantograph public contracts.
PyTorch/Transformers envelope load failures now return the same structured
worker response shape as generation requests. Rust decodes load success and
failure envelopes, preserves request ids and canonical worker error codes, and
normalizes worker load failures through the existing `BackendError` mapping
instead of surfacing raw PyO3 exception text.
PyTorch streaming generation now runs a structured worker setup probe before
iterating the Python generator, so invalid stream envelopes and missing-model
setup failures normalize through `PyTorchWorkerFailure` with request ids and
canonical error codes like load and non-streaming generation failures.
The canonical PyTorch/Transformers load-request fixture now includes the
backend-local task profile, freezing the Rust-to-worker loader family and
required processor-component projection alongside the package-facts mapper
tests.
PyTorch/Transformers load-envelope validation now rejects empty load
`entry_path` values, empty explicit device strings, mismatched task-profile
ids, and mismatched resolved-model-source identity before crossing into the
Python worker. The torch-free Python worker-contract projection also rejects
empty load `entry_path` values, so Rust and Python validate the same required
model-load identity field.
PyTorch now has an explicit versioned `shutdown_worker` envelope, torch-free
Python projection, structured response decoder, and worker entrypoint that
clears text-generation, diffusion, and ASR module state without terminating the
embedded interpreter. Backend `stop()` uses this shutdown envelope for
best-effort cleanup instead of treating model unload as the worker lifecycle
boundary.
Node-engine PyTorch text execution now treats model/Pumas-provided
`inference_settings` as typed generation settings for `max_new_tokens` /
`max_tokens`, `temperature`, `top_p`, and `top_k`, with direct node ports taking
precedence. Unsupported settings still fail at the node-engine boundary rather
than becoming arbitrary Transformers/Python kwargs.

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
Implementation found and fixed a stale node-engine rerank typed-result match
that still destructured the pre-diagnostic result shape; the canonical rerank
execution path now ignores unknown/additive result fields instead of breaking
when result diagnostics are present.
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
embedding, rerank, image generation, and audio transcription.
Audio transcription now has stable typed input/result DTOs, serde coverage,
validation that requires either encoded audio or a host-owned audio artifact
reference, and canonical executable registry status. `InferenceBackend` and
`InferenceGateway` expose a typed `transcribe_audio` method that fails closed
by default, giving PyTorch and future external-runtime slices a single backend
edge while keeping artifact lookup outside the inference crate.
The PyTorch backend now implements the `transcribe_audio` method for encoded
in-memory audio only, while rejecting `audio_ref` requests with a config error
so artifact lookup stays in host/runtime adapters rather than moving media
payload ownership into the inference crate.
The embedded Python runtime bridge now recognizes canonical
`audio_transcription`, Hugging Face `automatic-speech-recognition`, and legacy
`audio-to-text` labels at a single PyTorch ASR branch so future gateway work
does not inherit another task-label drift point.
Node-engine canonical `llm-inference` now builds typed audio transcription
requests from encoded audio or artifact references, routes them through
`InferenceGateway::execute_typed`, validates the typed result kind, and projects
bounded text/language/duration/segment outputs without serializing raw audio
payloads into diagnostics.
Roadmap registry tasks for image understanding, video understanding, and
multimodal generation now have guardrail coverage proving their task/request
contracts remain non-executable and typed request validation rejects them before
any backend/input-kind dispatch.
Public inference contract JSON keys now have guardrail coverage against
scheduler-policy terminology such as admission, reservation, priority,
eviction, scheduler-policy, and selected-best-backend in representative
runtime, lifecycle, capability, model-source, and typed execution
request/result DTOs.
The inference crate READMEs now document typed execution requests/results,
Pumas package facts, compatibility diagnostics, and the policy boundary that
keeps scheduler admission, reservation, priority, eviction, and final backend
choice outside the inference crate. Other touched consumer directories still
need README updates as their migration slices land.

### Milestone 11: Canonical Workflow Node Migration

**Goal:** Update graph-visible inference nodes and saved workflows to the new
canonical inference shape instead of preserving backend-specific node contracts.

**Tasks:**
- [x] Define the canonical inference node descriptor, node data schema, port
  contract, settings schema, Pumas model-reference field, resolved model-source
  projection field, task-kind field, runtime-hint field, and migration
  diagnostics.
- [x] Store canonical task registry ids and typed generation/task options in
  the canonical node shape rather than backend node names, embedding-mode flags,
  or backend-specific parameter bags.
- [x] Define how node validation displays task registry, package-fact,
  lifecycle, and option-compatibility diagnostics without exposing backend
  internals. The canonical `llm-inference` contract now exposes `diagnostics`
  and `metadata` output payloads as backend-neutral task diagnostics roles for
  every supported task, with contract coverage proving those payload markers do
  not surface backend keys, runtime ids, scheduler language, or reservation
  language. Workflow-service node definitions and frontend/shared TypeScript
  port DTOs now preserve those `inference_payloads`, giving UI/node-validation
  consumers the canonical role metadata without reaching into backend internals.
  UI display wiring now routes through a pure frontend presenter that reads
  only backend-neutral `inference_payloads` metadata and renders compact task,
  model-fact, option, diagnostics, and usage rows on the canonical LLM
  inference node without exposing backend/runtime/scheduler internals or raw
  payload bodies.
- [x] Define saved-workflow schema versioning and append-only migration records
  for the canonical inference node shape.
- [x] Define executable migration fixtures for old and new saved workflow
  shapes, including expected diagnostics and default semantics when fields are
  omitted.
- [x] Inventory old graph-visible inference node types and related ports,
  including removed `ollama-inference`, `llamacpp-inference`, `pytorch-inference`,
  generic `inference`, `embedding`, `reranker`, model provider, unload model,
  KV ports, and embedding-mode fields.
- [x] Define old-to-new port mappings for prompt/messages, system prompt,
  model source, Pumas model reference, image/audio/video inputs, generation
  options, embedding options, rerank inputs, model refs, stream outputs, and KV
  cache handles.
- [x] Remove old `ollama-inference` from the saved-graph migration target set;
  do not synthesize `retired_ollama` runtime hints or unresolved Ollama model
  placeholders.
- [x] Migrate old llama.cpp nodes to canonical inference with
  a Pumas model reference where the GGUF can resolve through Pumas. Preserve GGUF
  path, optional mmproj path, task options, and KV cache ports through
  Pumas-resolved artifact facts where compatible.
- [x] Migrate old PyTorch nodes to canonical inference with a
  Transformers/PyTorch runtime hint and a Pumas model reference where the
  HF-compatible source can resolve through Pumas.
- [x] Migrate old embedding nodes to canonical inference with
  `task_kind = embedding`; remove public embedding-mode semantics from the
  graph contract while preserving embedding inputs/outputs.
- [x] Migrate reranker nodes to canonical inference or a canonical rerank task
  shape without backend-specific request semantics.
- [x] Preserve graph topology, node ids where possible, edge ids, positions,
  labels, groups, output bindings, and user-authored settings.
- [x] Update node registry, workflow-node descriptors, graph validation,
  frontend node renderers, and templates so new saved workflows use the
  canonical node shape.
- [x] Add validation that old backend-specific inference node types do not
  persist after migration succeeds.

**Verification:**
- Fixture tests for saved workflow migration covering llama.cpp, PyTorch,
  embedding, reranker, multimodal, stream, and KV-cache cases.
- Tests proving `ollama-inference` is not migrated to a canonical placeholder
  and stale `retired_ollama` hints are not silently ignored.
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

**Status:** In progress. Ollama inference nodes had structural migration
coverage from an earlier slice, but that compatibility target has now been cut
so saved `ollama-inference` nodes are no longer rewritten to canonical
`llm-inference` placeholders. The workflow-service canonicalization boundary
now has an executable legacy inference-node migration inventory covering
`llamacpp-inference`, `pytorch-inference`, `embedding`, `reranker`, and the
existing generic `llm-inference` shape, including
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
diagnostics. Legacy llama.cpp migration now preserves `mmproj_path` evidence in
unresolved Pumas model-reference diagnostics, and node-engine canonical input
hydration carries resolved `.mmproj` companion artifacts into llama.cpp runtime
startup/matching. Pumas-backed GGUF/HF resolution, frontend/template, and
validation slices remain open. Frontend mock registries now expose canonical
Puma-Lib and `llm-inference` ports for Pumas model refs, resolved model
sources, task/runtime hints, task options, diagnostics, and dependency facts so
frontend-only sessions exercise the same graph-facing contract as Rust-backed
sessions. Rust workflow-node
inventory no longer registers retired `llamacpp-inference`,
`pytorch-inference`, `embedding`, or `reranker` node descriptors as
graph-visible built-ins, and the remaining retired `llamacpp-inference`,
dedicated `embedding`, and dedicated `reranker` processing modules have now
been deleted from the public workflow-nodes surface. Workflow-service
canonicalization fixtures own old-shape migration semantics instead of local
retired descriptor structs or direct legacy execution helpers. Frontend node
maps and workflow mocks no longer expose retired backend-specific inference
renderers, the GGUF reranker template now uses canonical `llm-inference` with
`task_kind = rerank`, and `puma-lib` exposes a canonical JSON
`pumas_model_ref` output for Pumas-to-inference graph wiring.
The canonical `workflow-nodes` `llm-inference` descriptor now also fails closed
from its local `Task::run` implementation, so standalone graph execution cannot
issue direct OpenAI-compatible HTTP requests outside the typed inference gateway
and node-engine lifecycle/diagnostic boundary. The descriptor-local
`InferenceConfig`/`base_url` public export was removed with that path; canonical
task and generation options must flow through graph ports and typed inference
requests instead of a hidden descriptor config.
The old graph-visible `vision-analysis` node has also been retired from
workflow-node inventory, frontend mock registries, and node-engine execution.
Image understanding remains represented by the canonical `llm-inference`
`image_understanding` task contract until a typed executable backend slice
promotes it; stale `vision-analysis` execution attempts now receive the same
canonical migration error as other retired inference node shapes.
Workflow-service capability extraction no longer infers llama.cpp or PyTorch
requirements from retired node type names; it derives backend requirements from
canonical `runtime_hint`, `backend_key`, or Pumas `recommended_backend` data and
surfaces stale `retired_ollama` hints as invalid required backend data instead
of hiding them.
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
Embedded-runtime edit-session metadata emission sync now uses the inference task
registry to detect canonical `llm-inference` embedding tasks, including task
aliases such as `feature-extraction`, and no longer treats the retired
`embedding` node type as the executable shape.
Workflow-service saved-workflow migration coverage now includes a mixed legacy
inference graph fixture proving llama.cpp, embedding, and reranker nodes migrate
to canonical `llm-inference` nodes while preserving cross-node topology and
output bindings.
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
that supported retired `llamacpp-inference`, `pytorch-inference`, `embedding`,
and `reranker` node types are rewritten to canonical `llm-inference` nodes and
still produce migration records. A separate regression test proves removed
`ollama-inference` nodes are not rewritten into canonical placeholders.
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
Saved-workflow canonicalization now also migrates legacy same-type
`llm-inference` nodes that still carry flat `temperature`/`max_tokens`
settings into grouped `generation_options`, preserving node id, position,
user-authored data, prompt/response topology, and migration diagnostics while
dropping stale flat option edges.
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
Inference gateway start request DTOs no longer expose Ollama model-name fields;
retired Ollama gateway/backend checks now trigger only from the active backend
identity, not from a request shape that suggests Ollama remains configurable.
Persisted workflow files now carry append-only `contract_upgrades` records, and
filesystem save/load canonicalization appends legacy inference migration records
without duplicating existing records. This makes saved workflow migration
diagnostics durable while keeping the graph shape canonical.
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
Filesystem saved-workflow persistence now canonicalizes graphs on both
`save_workflow` and `load_workflow`, so retired inference nodes are serialized
and returned as canonical `llm-inference` nodes with preserved node ids,
positions, compatible topology, Pumas migration evidence, runtime hints, and
derived graph fingerprints even when callers bypass edit sessions.
Filesystem persistence coverage now also includes mixed legacy embedding and
reranker nodes, proving dedicated embedding/rerank node data migrates into
canonical `task_kind`, `runtime_hint`, `task_options`, migration diagnostics,
and output topology before workflows are written.
Milestone 11 checklist status was reconciled against the executable
canonicalization, persistence, workflow-node, frontend mock, and template
coverage. Descriptor/schema definition, canonical task/options storage,
executable migration fixtures, topology preservation, and registry/template
updates are now marked complete. Pumas-backed package resolution, validation
display behavior, and persisted schema-version records remain open.
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
Node-engine request-builder coverage now proves canonical embedding execution
forwards `resolved_model_package_facts` from the Pumas package-facts fixture,
and rerank/audio transcription builders reject malformed package-facts payloads
through the same explicit parse boundary as text generation. Audio transcription
request-builder coverage now also proves package facts supply the model
identity when no explicit model field is supplied, avoiding backend-local
`"default"` fallbacks for Pumas-backed ASR workflows.
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
Milestone 11 checklist status now reflects validated workflow-service
canonicalization coverage for legacy llama.cpp and PyTorch nodes:
`canonicalize_workflow_graph_migrates_legacy_llamacpp_nodes`,
`canonicalize_workflow_graph_migrates_legacy_pytorch_nodes`, and
`legacy_inference_migration_inventory_maps_model_sources_and_task_options`
all pass and cover the old-to-canonical model source and task option mapping
requirements.
Workflow-node processing descriptors no longer preserve retired
`ollama-inference` or `pytorch-inference` task structs as public graph-contract
surface. Saved workflow upgrades remain owned by workflow-service
canonicalization fixtures instead of local retired descriptors.
Node-engine no longer keeps explicit executor branches for stale
`ollama-inference` or `pytorch-inference` task ids; if an uncanonicalized graph
reaches execution, it follows the normal unknown/host-specific executor error
path rather than a preserved compatibility handler.
The inference managed-runtime contract no longer carries an `Ollama`
`ManagedBinaryId` variant or retired Ollama binary definition. Managed runtime
state, capabilities, command resolution, and neutral dependency projection now
type-check only the supported llama.cpp managed runtime family.
Tauri no longer models the retired `ollama_vlm_model` config field or carries a
dedicated Ollama sidecar spawn rejection branch; unknown old config keys are
ignored by serde, and unsupported sidecar names use the normal unsupported
target error path.
Pantograph UniFFI and Rustler bindings no longer re-export Pumas'
`is_ollama_running` helper. Pumas may keep provider-agnostic facts internally,
but Pantograph host bindings no longer expose Ollama-specific runtime probes.
Shared runtime identity no longer treats `ollama` as a known runtime or backend
alias. Old Ollama workflow migration strings remain local to workflow-service
canonicalization instead of global runtime identity.
Runtime-registry and embedded-runtime registry tests no longer use `ollama` as
generic live runtime sample data; supported PyTorch samples cover replacement
observation and explicit override behavior instead.
The inference gateway no longer contains a dedicated Ollama startup-config
rejection branch. Ollama remains unregistered in the backend registry, while
manually constructed unsupported backend names use the normal backend config
paths instead of a preserved Ollama-specific policy error.

### Milestone 12: Prepare Native Candle Slice

**Goal:** Use the new contracts to implement or stage a narrow Candle runtime
without broad scheduler or Transformers duplication.

**Tasks:**
- [x] Choose one initial Candle task family, preferably embeddings, with a
  documented model format and tokenizer requirement.
- [x] Add backend-local Candle model load facts, resolved device facts, and
  explicit unsupported capability behavior.
- [x] Map selected Pumas-resolved HF-compatible safetensors/config/tokenizer
  packages through the Rust model-source contract.
- [x] Use Candle safetensors, tokenizer, device, dtype, and model loading
  patterns without creating a general model scheduler.
- [x] Keep feature-gated Candle dependencies optional and documented.
- [x] Document Candle dependency cost, feature behavior, supported platforms,
  model-family limitation, and why the selected dependency set is justified for
  the first slice.
- [x] Add tests that compile with and without the Candle feature.

**Verification:**
- `cargo check -p inference --features backend-candle`.
- `cargo check -p inference --no-default-features`.
- `cargo check -p inference --all-features`.
- Focused Candle backend tests that do not require heavyweight model downloads
  unless explicitly marked as ignored/integration.
- README feature contract review.

**Status:** In progress.

The first Candle staging slice keeps embeddings as the only advertised task
family for HF-compatible local package directories containing safetensors
weights, config, and tokenizer files, and records unsupported streaming,
external-connection, custom-code, device-selection, and KV-cache facts. The
`backend-candle` feature remains optional and compiles, but the registry reports
Candle unavailable because executable safetensors/tokenizer model loading is
not implemented yet; this prevents runtime selection from treating a staged
backend as executable. A backend-local load plan now resolves Pumas package
facts into concrete local config, tokenizer, safetensors, dtype, model-type, and
device-hint facts. The next resource-probe slice consumes that plan through
real Candle/tokenizers APIs and loads tokenizer plus safetensors tensor
resources, but it still stops before constructing Candle model modules,
runtime residency, or executable inference paths.

Update during implementation:
- 2026-05-03: Candle backend availability now fails closed behind
  `backend-candle` until executable model loading exists. Registry and backend
  tests cover the staged embedding-only capability facts and unavailable state,
  while README feature docs call out the CUDA-oriented optional dependency cost.
- 2026-05-04: Candle now has a staged embedding package mapper from canonical
  Pumas package facts into `ResolvedModelSource`. The mapper accepts only valid
  HF-compatible embedding package directories with safetensors weights, a
  present tokenizer, and no custom-code requirement; rejects
  GGUF/non-embedding/missing-tokenizer/custom-code packages; and still does not
  advertise executable model loading.
- 2026-05-04: Tightened the staged Candle mapper to enforce all declared
  package components for the first embedding slice: config, safetensors weights,
  and tokenizer must be present before a Pumas package can map to
  `ResolvedModelSource`.
- 2026-05-04: Added a staged Candle embedding load plan that validates local
  package files exist, rejects unsafe component paths, resolves float32/float16/
  bfloat16 dtype facts, accepts only the first `bert` embedding model family,
  and parses CPU/CUDA device hints without advertising executable Candle
  loading or adding scheduler/runtime residency policy.
- 2026-05-04: Added a feature-gated Candle embedding resource probe that maps
  staged dtype/device facts into Candle types, parses `tokenizer.json` through
  `tokenizers`, loads safetensors weights through Candle, rejects invalid
  tokenizer/safetensors files, and keeps backend availability closed until an
  actual model module and execution path exist.

### Milestone 13: Evaluate vLLM and MLX Roadmap Boundaries

**Goal:** Record future backend directions without expanding the immediate
implementation beyond the cleaned execution boundary.

**Tasks:**
- [x] Evaluate vLLM as an external or managed HTTP runtime candidate and record
  which facts inference can observe directly.
- [x] Record dependency, process, platform, network binding, security, and
  feature-gating implications for vLLM before accepting implementation work.
- [x] Map the new Pumas-resolved Rust model-source/task contracts to likely vLLM
  startup and request inputs without implementing the backend.
- [x] Identify vLLM-specific behavior that must remain outside inference, such
  as request admission, batching policy, model residency policy, and serving
  cluster orchestration.
- [x] Define a candidate vLLM capability/fact mapping without implementing the
  backend.
- [x] Record MLX as a macOS-only candidate and list platform feature gates,
  verification requirements, likely first task family, and how HF-compatible
  packages would flow through the same Rust contracts.
- [x] Record cross-platform build expectations for non-macOS targets before any
  MLX feature or dependency is introduced.
- [x] Add follow-up plan or ADR trigger criteria for accepting vLLM or MLX as
  implementation work.

**Verification:**
- Boundary review against this plan's out-of-scope policy list.
- Feature contract review against `RUST-API-STANDARDS.md` before adding any
  future `backend-vllm` or `backend-mlx` feature.
- No code verification required unless implementation is pulled into scope by a
  re-plan.

**Status:** Implemented as a roadmap-boundary slice. Runtime identity now
recognizes `vllm` and `mlx` as stable roadmap runtime ids and display labels so
diagnostics, capability matching, and future registry observations can name them
consistently without registering executable backends, managed binaries,
scheduler policy, or platform dependencies.

vLLM remains a candidate external or managed HTTP runtime, not an in-process
model library. The inference crate may eventually observe static backend
capabilities, server endpoint identity, supported task families, accepted model
source kinds, served-model identity, health/readiness, bounded load/startup
errors, and request option support. It must not own vLLM admission control,
continuous batching policy, queue policy, model residency policy, eviction,
cluster placement, replica selection, or serving topology. A future vLLM slice
must consume Pumas-resolved HF-compatible package facts and canonical task
requests, then map them at the adapter edge into vLLM server startup/request
inputs. Remote Pumas/HF `vllm` tags remain discovery hints until local package
facts and Pantograph backend checks produce executable compatibility reports.

The candidate vLLM capability shape is: text/chat generation first, embeddings
only if the selected vLLM deployment advertises them, streaming when exposed by
the HTTP endpoint, OpenAI-compatible request/response projection at the adapter
edge, no GGUF execution path, no scheduler ownership, and no managed binary
installation until the neutral managed-runtime/binary source-of-truth can own
the process artifact. vLLM implementation requires a follow-up ADR or
implementation plan that records dependency/process ownership, local versus
remote network binding, authentication/TLS requirements, model-source mapping,
feature gates, diagnostics-ledger projection fields, and validation fixtures.

MLX remains a macOS-only roadmap candidate. A future MLX backend must be
feature-gated behind macOS build/runtime checks, compile and test as absent on
non-macOS targets, and start with a narrow HF-compatible package slice that
flows through the same Pumas-resolved model-source, task registry, generation
option, and diagnostics contracts. MLX must not become a required dependency
for Linux or Windows builds, and MLX search tags from Pumas remain discovery
hints rather than installed-model compatibility facts until local package facts
and backend capability checks prove execution support.

### Milestone 14: Consumer Migration and Guardrails

**Goal:** Migrate consumers to facts and contracts while preventing policy
drift back into inference.

**Tasks:**
- [ ] Update runtime registry, workflow service, node engine, Tauri, or
  frontend consumers only as needed to consume the new facts. Workflow-service
  graph registry conversion now preserves canonical port `inference_payloads`
  and reverse `PortDefinition` contract projection keeps those payloads intact;
  the frontend/shared graph types expose the append-only payload contract.
  Node-engine canonical `llm-inference` rerank execution now projects typed
  option diagnostics onto the existing `diagnostics` graph output like text,
  embedding, and audio paths, without copying query or document payloads into
  diagnostic metadata. Node-engine canonical `llm-inference` image-generation
  execution now routes through the same typed gateway boundary, projects
  generated image results plus bounded metadata/diagnostics onto existing graph
  outputs, and keeps prompt text plus generated image bytes out of diagnostics.
  Frontend and shared svelte-graph mock `llm-inference` definitions now expose
  payload roles for the executable text/chat, embedding, rerank,
  image-generation, and audio-transcription task families across
  task/model-reference/options/input/output/usage/diagnostic ports, keeping
  mock-backed tests aligned with the Rust registry contract without introducing
  backend/runtime policy fields. Rust node-contract and mirrored TypeScript
  payload role contracts now distinguish `cache_handle` from generic
  `task_output`, so `llm-inference.kv_cache_out` stays cache-handle metadata
  instead of a generated content/result payload. App-level workflow service
  TypeScript types, mocks, and inference payload display presenters now mirror
  the same `cache_handle` role rather than treating `kv_cache_out` as generic
  task output. Frontend workflow service and shared svelte-graph TypeScript
  execution-kind mirrors now include the roadmap `depth_estimation` input and
  result kind that Rust already exposes. App-level workflow capability DTOs now
  also include the roadmap `depth_estimation` task id so host capability facts
  can round-trip the Rust workflow-service task registry. The same TypeScript
  mirror now carries workflow task request contracts on backend task
  capabilities, including execution input/result kinds, streaming support, and
  required/output modalities. Workflow-service host capability contract
  snapshots now also freeze a roadmap `depth_estimation` task request contract
  next to executable embedding, proving contract-only task metadata uses the
  same snake_case wire shape. Workflow-service registry and frontend mock tests
  now also scan every canonical `llm-inference.inference_payloads` entry for
  forbidden backend/runtime/scheduler policy fields such as backend keys,
  runtime ids, admission, reservation, eviction, and priority.
  Workflow-service effective definition coverage now proves saved
  `node.data.definition` dynamic port overlays preserve structured
  `inference_payloads` task/result/role metadata through both effective
  definition and effective contract projection.
- [ ] Remove backend-name conditionals where new capability/runtime facts are
  sufficient. Embedded-runtime host capability projection now uses the shared
  Python-sidecar runtime-id set instead of a direct `pytorch` backend-key
  comparison, so aliases such as `torch` and neighboring Python sidecar
  backends are excluded from duplicate host-runtime projection consistently.
  Node-engine dependency preflight now keeps explicit workflow/backend hints
  first, then derives advisory backend key, task type, and model id from
  resolved package facts before falling back to legacy `llm-inference`
  heuristics for sparse saved graphs. Tauri RAG indexing errors no longer
  recommend retired Ollama when a backend lacks a GUI-compatible embedding HTTP
  endpoint; the related Tauri startup helpers no longer construct removed
  Ollama model-name fields, and the app-facing runtime-manager README now names
  supported managed redistributables without implying Ollama is available.
  Canonical text-generation, embedding, and rerank request builders now promote
  resolved Pumas package facts into typed `model_ref` when no explicit
  `pumas_model_ref`/`model_ref` input is wired, and dependency input merging now
  carries model context through direct `resolved_model_package_facts` and
  `model_package_facts` edges. The inference gateway itself now has the same
  package-facts model-id fallback for direct typed callers, so backend chat
  transport projection no longer depends on every caller pre-populating
  `model_ref`. Embedded-runtime dependency preflight now uses Pumas resolved
  package facts for backend hint, task, and model id before falling back to
  dependency-requirement or node-type heuristics, aligning the host Python
  preflight path with node-engine canonical `llm-inference` preflight without
  adding runtime selection policy.
  Embedded-runtime host execution no longer treats canonical PyTorch
  `llm-inference` as a Python-sidecar node: canonical LLM execution falls
  through to node-engine/inference, while the host Python adapter remains scoped
  to diffusion, audio generation, and ONNX nodes that still require
  host-managed Python resources.
  The unused `InferenceGateway::backend()` legacy escape hatch has been
  removed so repo consumers cannot bypass typed gateway methods and backend
  capability contracts by taking a raw mutable backend reference. Node-engine
  dependency preflight dispatch now uses the same explicit-hint-then-Pumas
  package-facts backend precedence as dependency request construction, so
  Transformers/PyTorch package facts enter preflight even when no raw
  `runtime_hint` input is present.
  Workflow-service graph canonicalization no longer migrates saved
  `ollama-inference` nodes into canonical `llm-inference` with a
  `retired_ollama` placeholder. Ollama is treated as removed compatibility
  surface rather than an upgrade target; workflows must already use the
  canonical Pumas-backed inference shape.
  Node-engine no longer treats `ollama-inference` as a stale backend-specific
  task id in dependency preflight or executor-branch guard tests; direct stale
  Ollama node execution now follows the generic unsupported-node path.
  The orphan frontend `OllamaInferenceNode.svelte` renderer has been removed,
  and neighboring frontend comments no longer describe llama.cpp styling in
  relation to Ollama.
  Tauri RAG indexing comments no longer describe Ollama as part of the active
  local embedding backend set.
  Node-engine dependency-preflight compatibility-report tests are now
  cfg-gated to `pytorch-nodes`, matching the PyTorch producer that emits those
  reports.
- [ ] Update host-facing README `API Consumer Contract` and `Structured Producer
  Contract` sections for every touched source directory that publishes or
  consumes machine-readable inference/Pumas/workflow DTOs. Frontend workflow
  service README contract sections now document workflow backend capability
  `request_contract` DTOs as task/modality/execution metadata, not scheduler or
  runtime selection policy. Workflow-nodes input and Tauri workflow README
  contract sections now document selected Puma-Lib detail hydration as
  selector-access-role dependent: owner/local-client access may expose batch
  detail facts, read-only access may expose selector-row facts only, and neither
  boundary may infer runtime/scheduler policy or inspect Pumas storage
  internals. Workflow-nodes and Tauri workflow README contract notes now also
  make package-facts summary snapshot/list views selector-access-role backed,
  with read-only/local-client paths projecting selector-row summaries instead
  of requiring raw owner `PUMAS_API`.
- [ ] Update generated bindings, host-language types, or shared schemas in the
  same implementation slice as native DTO changes when those bindings are
  public contract surface. Frontend workflow service and shared svelte-graph
  TypeScript `PortDefinition` types now mirror workflow-service
  `inference_payloads`, and their execution-kind unions now include
  `depth_estimation` so public TypeScript contracts stay aligned with the Rust
  roadmap task registry. App-level workflow backend-capability DTOs now include
  `depth_estimation` in `WorkflowInferenceTaskId`, covering host capability
  facts as well as port payload metadata, and now expose the optional
  `WorkflowTaskRequestContract` attached to backend task capabilities.
  Workflow-nodes canonical contract projection now includes executable
  image-generation task metadata and prompt/results payload annotations, and
  workflow-service registry coverage verifies the projected diagnostics
  payloads still round-trip without backend/runtime policy fields.
  Workflow-service registry coverage now also verifies image-generation
  prompt/result payload metadata survives the node-definition projection as
  snake_case task/result labels.
- [x] Add tests proving scheduler/runtime-registry policy remains outside
  inference.
- [ ] Update source READMEs and any ADR links affected by boundary changes.
  Node-engine and workflow-nodes source READMEs now document executable
  canonical image-generation as part of the typed `llm-inference` boundary and
  preserve the diagnostic hygiene rules for prompt/generated image payloads.
  Frontend workflow service and shared svelte-graph backend READMEs now
  document mock `inference_payloads` as registry contract facts for executable
  task families and explicitly reject backend/runtime policy fields on the
  canonical mock `llm-inference` surface. Inference source and backend READMEs
  now document append-only `ChatChunk.usage` stream metadata and the llama.cpp
  SSE usage parser boundary as bounded diagnostics facts that must not carry
  prompt/result payloads, Python kwargs, CLI flags, or local paths. Node-engine
  source and core-executor READMEs now document canonical text/chat `usage`
  output as bounded token-count metadata copied from typed gateway results or
  terminal stream chunks, never recomputed from prompt/generated text.
  Workflow-nodes and svelte-graph backend READMEs now document cache-handle
  payload roles as graph metadata facts rather than runtime policy or generated
  result payloads. Workflow service frontend READMEs and inference payload
  presenter docs now mirror that cache-handle role boundary for app-level
  mocks and display rows. Inference and node-engine source READMEs now document
  package-facts model identity fallback for typed gateway transport,
  canonical `llm-inference` request builders, and direct package-facts
  dependency-input edges. Embedded-runtime task-executor README now documents
  dependency preflight request precedence: explicit workflow inputs first,
  Pumas resolved package facts second, then dependency-requirement or node-type
  legacy heuristics. Workflow-service graph README now documents that
  `ollama-inference` is not a supported saved-graph compatibility target and
  graph canonicalization does not synthesize `retired_ollama` placeholders.
  Node-engine source README now mirrors that canonical `llm-inference` routing
  may use resolved Pumas backend hints to enter the PyTorch/Transformers
  dependency preflight path when no explicit runtime hint or backend key is
  wired, without moving scheduling or admission policy into
  node-engine/inference. Inference source README now documents compatibility
  issue path redaction before lifecycle emission when a stable model id is
  present.
  Inference crate README contract sections now document that consumers must use
  typed `InferenceGateway` methods and capability facts rather than a raw
  backend-handle accessor.
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
as the host-visible streaming surface. Inference contract fixture coverage now
asserts representative request, result, lifecycle, capability, compatibility,
Pumas update/snapshot, model-source, and load-security wire shapes avoid
scheduler-policy terminology such as admission, reservation, priority,
eviction, scheduler-policy, and selected-best-backend.
2026-05-04 update: inference source READMEs now document the staged Candle
package-source/load-plan contract, the unavailable-until-executable guardrail,
and the boundary that keeps Candle package validation separate from runtime
selection, residency, and scheduling policy.
2026-05-04 update: `crates/inference/README.md` now also documents that
PyTorch direct local loads and Pumas-resolved loads share the backend-local
`load_transformers_model` worker envelope, that direct paths remain
import/debug sources rather than Pumas model refs, and that Candle staged
resource probes do not imply executable runtime availability.
2026-05-04 update: node-engine canonical `llm-inference` image generation now
uses typed `InferenceExecutionRequest::ImageGeneration` instead of falling back
to backend-name dispatch. The slice proves Pumas model refs, Pumas package
facts, direct image controls, and nested `task_options` image controls map into
the typed gateway while bounded graph diagnostics omit prompt text and encoded
image payloads.

### Milestone 15: Diagnostics Ledger Integration

**Goal:** Make inference-related backend choice, compatibility, lifecycle, and
error facts durable through the existing diagnostics ledger without making
inference write ledger events directly.

**Tasks:**
- [x] Map selected backend/runtime facts to existing scheduler admission,
  reservation, run-list, and run-detail projection fields where sufficient.
  Node execution status now backfills run-list and run-detail
  `selected_runtime_id` from the observed runtime id when scheduler admission
  has not already supplied a selected runtime, preserving scheduler-owned
  runtime selection precedence while making execution-observed backend facts
  queryable.
- [x] Decide whether the diagnostics ledger needs an additive
  `selected_backend_key` or `selected_backend_family` field when
  `selected_runtime_id` does not clearly identify llama.cpp,
  Transformers/PyTorch, Candle, vLLM, or MLX.
- [x] Map Pumas model id, resolved artifact kind, canonical task id, selected
  runtime id, selected backend key/family, selected device id, selected network
  node id, and scheduler policy id into durable run/node/runtime diagnostics.
  Durable `InferenceExecutionDiagnosticObserved` events now feed run-list and
  run-detail selected runtime, backend, model, and task rollups from typed event
  fields. Inference lifecycle events and durable diagnostic payloads now carry
  bounded `resolved_artifact_kind` metadata from Pumas package facts without
  parsing backend-specific payload details. Inference lifecycle events now also
  carry explicit non-`auto` selected device ids from backend start config;
  embedded-runtime copies them into
  `InferenceExecutionDiagnosticObserved.selected_device_id`, and run-list/detail
  projections persist them in the existing selected-device columns. Inference
  lifecycle events and durable diagnostic payloads now also carry optional
  execution-observed selected network node ids, and run-list/detail projections
  persist them in the existing selected-network-node columns when supplied.
- [x] Map lifecycle-carried inference compatibility summaries into durable
  bounded metadata: accepted/rejected/degraded status dimensions, missing
  components, unsupported backend/task pairs, custom-code/trust blockers, and
  model/backend unavailable reasons.
- [ ] Emit `BackendCompatibilityReport` summaries from execution/preflight
  producers wherever resolved model package facts and selected backend facts are
  available. The durable lifecycle-to-ledger mapping is complete and the typed
  gateway now emits summaries when `InferenceExecutionRequest` carries package
  facts. Embedded-runtime now persists completed task-validation compatibility
  summaries as durable inference diagnostic summaries as well as backend
  execution summaries, but upstream graph/preflight paths still need to
  populate package facts consistently for every executable task.
  Typed non-streaming backend-execution completion events now retain the same
  bounded compatibility summary computed during task validation, so downstream
  ledger adapters can persist selected-backend compatibility facts from the
  actual execution phase as well as validation.
  Typed streaming backend-execution terminal events now carry the same bounded
  compatibility summary through the private lifecycle stream wrapper, keeping
  public chat streaming lifecycle calls diagnostic-free unless a typed request
  supplied package facts. Coverage includes completed and failed typed stream
  terminal events while cleanup remains diagnostic-free. Node-engine
  graph-visible `llm-inference` streaming execution now has package-facts
  lifecycle coverage proving task-validation and backend-execution terminal
  events carry the bounded compatibility summary and stable model id.
  Canonical rerank package-facts fixtures now decode through the public
  inference contract, resolve through the task registry as `rerank`, and have
  graph-visible `llm-inference` coverage proving rerank request construction
  forwards package facts while task-validation and backend-execution lifecycle
  events carry bounded compatibility summaries with the stable Pumas model id.
  HF audio-transcription package-facts fixtures now decode through the public
  inference contract, resolve through the task registry as `audio_transcription`,
  and have graph-visible `llm-inference` coverage proving audio request
  construction forwards package facts while task-validation and
  backend-execution lifecycle events carry bounded compatibility summaries with
  the stable Pumas model id. Audio transcription request construction now also
  derives `model_ref`, `model_name`, and backend request model identity from
  resolved package facts when no explicit model input is present. Diffusers
  image-generation package-facts fixtures now have graph-visible
  `llm-inference` coverage proving image-generation
  request construction forwards package facts while task-validation and
  backend-execution lifecycle events carry bounded compatibility summaries with
  the stable Pumas model id. Canonical PyTorch dependency preflight now also
  computes a bounded backend compatibility summary from resolved package facts
  and selected PyTorch backend facts, attaching it only to the completed
  model-package-resolution lifecycle event while keeping started and cleanup
  events free of option, usage, cache, prompt, tensor, and local-path payloads.
  Validation passed with
  `cargo test -p node-engine --features inference-nodes,pytorch-nodes dependency_preflight`,
  `cargo test -p node-engine --features inference-nodes,pytorch-nodes test_dependency_preflight_records_lifecycle_success_with_resolver`,
  `cargo test -p pantograph-embedded-runtime compatibility`,
  `cargo fmt --all`, and `git diff --check`.
- [x] Map generation option support summaries into durable bounded
  metadata: honored, mapped, defaulted, ignored, rejected, unsupported,
  conflicts, requires-model-support, and requires-backend-support. Canonical
  `llm-inference` request construction now projects non-null `kv_cache_in`
  graph input and explicit `kv_cache_checkpoint_requested` task options into
  typed cache generation intent, so the existing typed gateway option
  diagnostics can report cache support without moving cache reuse, residency,
  or scheduler decisions into inference. Typed text gateway diagnostics now
  classify `cache.use_cache` as requiring backend/runtime cache support and
  `cache.kv_cache_checkpoint_requested` as mapped to Pantograph KV-cache
  publication outside backend-native chat request fields. Typed text generation
  option diagnostics now appear on task-validation completion as well as
  backend-execution completion so validation surfaces can report generation and
  cache option support before backend execution.
- [x] Map non-generation task option support summaries into durable bounded
  metadata for embedding, rerank, image/video/audio, and KV-cache task options
  after those task option diagnostics are emitted by the corresponding typed
  execution paths. Embedding/rerank/image-generation typed gateway diagnostics
  now emit bounded option support for extra options, rerank controls, and
  first-class image request settings. Audio transcription diagnostics now cover
  first-class language, prompt, task, and chunk-length request settings without
  copying prompt text or audio payload values into lifecycle events. Typed
  non-generation option diagnostics now appear on task-validation completion as
  well as backend-execution completion so node validation views can surface the
  same bounded option facts before execution. Embedding and rerank typed result
  DTOs now also return bounded option diagnostics so direct `execute_typed`
  callers see the same option facts as lifecycle consumers, with serde
  contract coverage freezing the embedding/rerank result fields for generated
  bindings and host consumers. Result diagnostic extraction now goes through
  the canonical typed result accessor and covers audio transcription result
  diagnostics as well as text, embedding, rerank, and image. Serde contract
  coverage now freezes image-generation and audio-transcription result
  diagnostics alongside embedding/rerank diagnostics. KV-cache task progress
  now carries bounded option diagnostics for truncate marker/token-position
  handling, and embedded-runtime maps those summaries into durable
  option-support metadata. Contract-only `video_understanding` task validation
  now emits bounded backend-unavailable diagnostics for recognized video
  sampling/window task options and embedded-runtime persists those option
  support counts without storing raw video payloads.
- [ ] Map lifecycle summaries into durable bounded metadata for package
  resolution, task validation, preprocessing, backend execution, postprocessing,
  result projection, duration, cancellation, and cleanup. Diagnostic-observed
  inference payloads now preserve lifecycle phase and event kind for events that
  carry compatibility or option summaries, and workflow-owned lifecycle sinks
  now copy matched terminal durations into durable bounded diagnostic summaries.
  Completed package-resolution, preprocessing, postprocessing, and
  result-projection events can persist duration-only summaries when a matching
  start is known. Cancelled lifecycle events now persist duration-only bounded
  diagnostic summaries when a matching start is known, while durationless
  cancellation and failed events plus cleanup remain non-persisted to avoid
  ledger noise. Failed lifecycle events with matched duration now persist
  duration-only bounded diagnostic summaries without copying failure detail into
  the inference diagnostic payload. Typed non-streaming and streaming gateway
  requests that carry resolved Pumas package facts now emit explicit
  model-package-resolution started/completed/cleanup lifecycle facts before
  task validation, and lifecycle model identity prefers the package-facts model
  id over backend transport model names. Typed non-streaming text generation
  now emits duration-observable preprocessing, postprocessing, and
  result-projection lifecycle phases around the validated backend execution
  path. The same explicit preprocessing, postprocessing, and result-projection
  producer coverage now applies to typed embedding, rerank, image generation,
  and audio transcription requests, giving ledger adapters coverage for those
  Transformers-aligned boundaries without storing prompts, raw media,
  embeddings, documents, or generated content. Embedded-runtime ledger adapter
  coverage now also proves image-generation backend-execution lifecycle
  summaries persist bounded task/backend/artifact/compatibility/option facts
  while omitting prompt text, encoded image bytes, backend flags, and local
  paths. Typed text/chat streaming now emits explicit preprocessing lifecycle
  events around OpenAI-compatible request projection and emits postprocessing
  plus result-projection lifecycle events after successful stream completion,
  while failed or cancelled streams stop at backend-execution terminal/cleanup
  and legacy raw chat streaming remains unchanged. Node-engine dependency
  preflight now also emits model-package-resolution started/completed/cleanup
  lifecycle facts for successful canonical PyTorch preflight with bounded
  request/task/backend/runtime/model identity plus the resolved artifact-kind
  label when known and without usage, cache, artifact refs, compatibility,
  option, tensor, prompt, or local-path payloads. Embedded-runtime
  lifecycle-to-ledger projection now treats `resolved_artifact_kind` as bounded
  label metadata: label-shaped values are persistable even without other
  payload metadata, while path-shaped or unsafe values are dropped before
  durable diagnostics. Diagnostics-ledger scheduler timeline projection now
  also drains `inference.execution_diagnostic_observed` rows and formats
  bounded lifecycle/task/backend/device/artifact/compatibility/option/usage/
  cache-reference summaries without promoting prompt/result bodies, tensors,
  raw media, Python kwargs, CLI flags, or local paths into timeline detail.
- [ ] Record usage/cache/artifact references where available, including token or
  request usage counts, cache-handle ids, KV checkpoint ids, and artifact refs,
  without storing prompt/result payload bodies. Typed lifecycle events and
  durable inference diagnostic payloads now carry bounded usage counts and
  cache-handle ids. Typed embedding execution now aggregates backend embedding
  item token counts into bounded prompt/total usage summaries without storing
  vectors or input text. Gateway lifecycle projection now reads usage and
  cache-handle summaries through canonical typed result accessors instead of
  duplicating result-shape matches in producer code. Typed text/chat streaming
  now carries backend terminal cache-handle ids through `ChatChunk`, typed
  `TextGeneration` results, backend-execution lifecycle completion events, and
  the canonical `llm-inference.kv_cache_out` graph port without storing
  prompt/result text, KV bytes, tensors, temp paths, or scheduling decisions.
  Host-owned workflow event sinks now persist structured
  KV-cache progress references for action, outcome, cache id, backend key, reuse
  source, token count, and reason without cache bytes/fingerprints/temp paths;
  I/O artifact projection now exposes selected-backend context and derives
  missing runtime/model/backend facts only from the latest raw producer-node
  execution status event at or before the artifact observation. Typed text
  generation now propagates optional bounded usage counts from terminal backend
  stream chunks into typed results and backend-execution lifecycle events
  without storing prompt text or generated content. The llama.cpp SSE parser
  now retains OpenAI-compatible bounded usage payloads from content or
  usage-only stream events while dropping oversized counts. PyTorch worker
  streaming now applies the same usage-only and bounded-count behavior for
  dictionary stream chunks, so typed streaming lifecycle completion can retain
  PyTorch token usage metadata without storing prompt or generated text. Typed
  streaming lifecycle completion now also carries the last bounded stream-chunk
  usage summary onto the terminal backend-execution event without storing prompt
  or generated text. Additional backend producers remain open. Canonical
  `llm-inference` embedding execution
  now projects typed embedding `usage` and bounded `option_diagnostics` onto
  graph output ports while keeping input text and embedding vectors out of
  those metadata outputs; node lifecycle tests now match the explicit
  preprocessing, postprocessing, and result-projection phase model for
  non-streaming typed execution. Canonical `llm-inference` text/chat execution
  now projects bounded terminal token usage to the existing graph `usage` port
  and stable backend-local cache-handle ids to `kv_cache_out` for streaming and
  non-streaming paths without storing prompt text, generated content, KV bytes,
  tensors, temp paths, or scheduler/runtime reuse decisions. Typed audio
  transcription now propagates bounded `audio_ref` artifact references through
  task-validation and backend-execution lifecycle events and durable inference
  diagnostics while keeping encoded audio bytes, prompt text, generated
  content, local paths, `file://` URLs, and scheduler/runtime selection state
  out of the payload. Contract-only image understanding, depth estimation,
  video understanding, and multimodal generation typed requests now use the
  same bounded artifact-ref collector for stable `image_ref`, `video_ref`, and
  multimodal artifact parts during task validation, without promoting those
  roadmap tasks to executable backend paths.
  Non-typed `embeddings_with_lifecycle` now also projects bounded token usage
  from backend `EmbeddingResult.token_count` values onto the backend-execution
  lifecycle completion event without serializing input text or embedding
  vectors.
  Node-engine graph-level contract-only task validation now also propagates
  bounded stable artifact refs from roadmap task inputs onto the failed
  task-validation lifecycle event while filtering local path and `file://`
  values and leaving started/cleanup events artifact-free.
- [x] Continue using `diagnostic.error_occurred` for canonical errors and add
  inference-specific phases only where existing runtime preflight, model
  dependency, runtime model load, runtime launch, node execution, or output
  validation phases are insufficient. Workflow-service diagnostic registry
  coverage now asserts no inference-prefixed canonical error phases or codes
  exist while the runtime preflight, runtime model load, runtime launch, model
  dependency, node execution, and output validation phases remain registered.
  Embedded-runtime inference lifecycle ledger coverage now proves failed
  lifecycle events preserve producer-supplied `canonical_error_event_id` links
  through node-status projection without creating parallel
  `diagnostic.error_occurred` payloads.
- [x] Preserve canonical error links from run terminal, node status, model
  lifecycle, and inference lifecycle summaries where the direct causal event is
  known. Run terminal projection now carries `canonical_error_event_id` into
  run-list and run-detail `latest_error_event_id` without duplicating the
  detailed error payload or incrementing error counters. Scheduler model
  lifecycle failed transitions now do the same when they carry
  `canonical_error_event_id`. Node execution status payloads and projections
  now preserve `canonical_error_event_id` separately from direct node-scoped
  `error_event_id`. Inference request lifecycle events now expose an
  append-only `canonical_error_event_id` field, and embedded-runtime lifecycle
  ledger adapters pass it through to node-status payloads when a producer
  already knows the direct causal error event.
- [x] Ensure ledger append failure returns or projects `diagnostics_unavailable`
  while preserving the original inference/preflight/execution error. Failed
  inference lifecycle detail is now sanitized and bounded before node-status
  diagnostic projection so oversized backend text does not itself cause a
  secondary ledger append failure; workflow-service append-failure projection
  now preserves the original workflow-run error and attaches
  `diagnostics_unavailable` when node execution, output validation, artifact
  conversion, or terminal diagnostic append fails. Runtime unload failures now
  remain authoritative when keep-alive shutdown or capacity-rebalance lifecycle
  diagnostic appends are unavailable, including the runtime-load admission
  error path used to surface capacity-rebalance unload failures. Frontend
  workflow-service error parsing now has explicit coverage proving
  `diagnostics_unavailable` links survive backend error envelopes even when no
  diagnostic event id exists. Embedded-runtime KV-cache workflow-event sinks now
  return a `diagnostics_unavailable` event-sink error when durable KV diagnostic
  append fails, after forwarding the original workflow event to the inner sink
  so stream/progress delivery is preserved. Inference lifecycle sinks now return
  structured `diagnostics_unavailable` errors when durable lifecycle or bounded
  inference diagnostic appends fail; gateway and node-engine producer edges log
  those secondary sink failures and preserve the original inference,
  preflight, or execution result.
- [ ] Keep prompts, chat messages, raw media, generated content, embeddings,
  token arrays, logits, tensors, Python kwargs, backend CLI flags, full local
  paths where stable ids exist, and unbounded stderr/stdout out of ledger
  payloads. Usage/cache diagnostic adapter coverage now asserts that raw-looking
  lifecycle detail markers are not serialized into durable inference diagnostic
  payloads. Embedded-runtime compatibility issue projection now omits absolute
  local issue paths when a stable model id is already present, while preserving
  relative component paths for bounded compatibility diagnostics. The same
  embedded-runtime payload-hygiene regression now covers raw-looking rerank
  documents, embedding vectors, Python kwargs, backend CLI flags, and full local
  model paths in lifecycle detail, proving they are not copied into durable
  inference diagnostic summary payloads. Durable inference option diagnostics
  now drop producer message text for both typed inference and KV-cache progress
  summaries, retaining bounded option path/state/backend keys and support
  counts while closing a remaining channel for prompts, Python kwargs, backend
  CLI flags, token arrays, logits, tensors, and local paths. Typed audio
  transcription artifact-ref projection now filters local path-shaped
  `audio_ref` values, including absolute/relative home paths, `file://` URLs,
  and Windows drive paths, before lifecycle events can reach durable
  diagnostics. Embedded-runtime ledger projection now defensively reapplies
  the same artifact-ref safety filter before persistence, so unsafe-only
  producer refs are dropped and stable refs remain bounded. Embedded-runtime
  ledger projection now also applies that bounded-ref filter to lifecycle
  `cache_handle_id` values before persistence, so local temp-path-shaped cache
  handles are not treated as durable metadata. The diagnostics ledger now also
  rejects local-path-shaped inference `artifact_refs` and `cache_handle_id`
  values at append time, so direct durable writes cannot bypass the producer
  filters. Inference typed execution and lifecycle producers now apply the same
  bounded-ref filter to text-generation cache handles before returning
  non-streaming typed results or emitting lifecycle cache facts. Node-engine
  streaming text collection now also drops
  path-shaped terminal cache handles before exposing `kv_cache_out` to graph
  outputs. Embedded-runtime ledger projection now drops path-shaped runtime
  instance ids, runtime ids, backend keys, model ids, device ids, network-node
  ids, KV-cache backend keys, option diagnostic backend keys, and compatibility
  issue model/path fields before durable inference lifecycle or diagnostic
  persistence. While validating the streaming graph slice, a stale node-engine
  lifecycle regression assertion was updated to match the full typed lifecycle
  event sequence emitted by the current gateway boundary. The shared
  artifact-ref filter now lives in the inference contract surface so gateway
  producers and host ledger adapters use one rule. PyTorch backend transport
  error normalization now strips Python traceback frames and local path tokens
  before those backend errors can be projected into workflow diagnostics.
  PyTorch task-join failure normalization now uses the same sanitizer, so Rust
  async task-boundary failures do not bypass traceback-frame stripping or local
  path redaction. Embedded-runtime lifecycle-to-ledger projection now also
  filters unsafe `resolved_artifact_kind` values instead of copying path-shaped
  strings into durable inference diagnostics. Inference compatibility issue
  summary production now also drops local path-shaped issue paths before
  lifecycle emission when a stable model id is present, while retaining bounded
  relative component paths such as `tokenizer.json`. Node-engine dependency
  preflight now redacts local path-shaped tokens from lifecycle failure detail
  before sink emission, while preserving the original execution error for the
  caller.
- [x] Update diagnostics-ledger, workflow-service diagnostics, runtime
  projection, and UI/API README contract sections for any added event or
  projection fields. Source README updates now cover inference lifecycle
  usage/cache fields, durable inference diagnostic summaries, node-derived
  run-list/run-detail selected backend/model/task rollups, embedded-runtime
  lifecycle sink projection rules, bounded resolved artifact kind diagnostics,
  and workflow-service `diagnostics_unavailable` behavior. Frontend TypeScript
  contracts and I/O Inspector presenter docs now cover projected
  runtime/backend/model artifact context. Inference, embedded-runtime, and
  diagnostics-ledger SQLite READMEs now document selected-device lifecycle and
  projection behavior. Frontend diagnostics TypeScript contracts, diagnostics
  service README guidance, and workbench diagnostics README guidance now also
  accept and document the backend-authored
  `inference_execution_diagnostic_observed` timeline event kind without making
  frontend consumers parse option, usage, cache, artifact, compatibility, or
  lifecycle payload JSON. Embedded-runtime source README now documents
  path-shaped lifecycle `cache_handle_id` filtering before durable diagnostic
  persistence.

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
- 2026-05-03: Selected-backend diagnostics slice added an additive
  `selected_backend_key` field to node execution status payloads and the
  queryable `node_status_projection`, with projection-version rebuild semantics
  and embedded-runtime lifecycle adapter population from backend-owned
  lifecycle facts. This keeps backend choice durable even when runtime ids such
  as `pytorch.transformers` are distinct from backend keys. Generation option
  support summaries remain a separate follow-up event/projection slice.
- 2026-05-04: Inference diagnostic observation now normalizes
  `selected_backend_family` from backend/runtime evidence while preserving the
  raw `selected_backend_key`, so durable diagnostics distinguish families such
  as `transformers_pytorch` and `llama_cpp` without making runtime id parsing a
  UI or query-consumer responsibility.
- 2026-05-04: Selected-device diagnostics slice added
  `selected_device_id` to inference lifecycle events and durable inference
  diagnostic observations. The gateway derives it only from explicit backend
  start config device values, omits `auto`, and the diagnostics-ledger projects
  it into the existing run-list and run-detail selected-device columns.
- 2026-05-05: Inference artifact-reference diagnostics slice added bounded
  `artifact_refs` to inference lifecycle events and durable
  `inference.execution_diagnostic_observed` payloads. The gateway initially
  derives artifact refs only from typed audio transcription `audio_ref` values,
  and embedded-runtime projects those refs into diagnostics-ledger rows without
  storing encoded media, prompts, generated content, local paths, or scheduler
  decisions.
- 2026-05-05: Audio artifact-ref hygiene slice tightened the gateway projection
  so local path-shaped `audio_ref` values are not copied into lifecycle events
  or durable diagnostics; stable refs such as `artifact://...` remain available
  as bounded artifact metadata.
- 2026-05-05: Embedded-runtime inference diagnostic projection now filters
  lifecycle artifact refs again at the ledger boundary, preventing future
  producers from persisting local path-shaped refs if they bypass upstream
  gateway filtering.
- 2026-05-05: Moved bounded lifecycle artifact-ref filtering into the inference
  public contract surface and switched both gateway lifecycle production and
  embedded-runtime ledger projection to the shared helper, avoiding duplicated
  local-path rules.
- 2026-05-05: Node-engine audio transcription request construction now uses
  resolved package facts as the model identity fallback, so ASR workflows with
  Pumas facts no longer send `"default"` to the typed backend request when no
  explicit model input is supplied.
- 2026-05-05: PyTorch worker transport error normalization now strips embedded
  Python traceback frames and redacts local path tokens before transport
  exceptions become canonical backend errors, while preserving request ids and
  worker error codes.
- 2026-05-05: Inference lifecycle sink append-failure slice changed
  `InferenceRequestLifecycleEventSink::record` to return a structured sink
  error, added a `diagnostics_unavailable` error contract, and made the
  embedded-runtime workflow ledger sink return it when durable lifecycle or
  bounded inference diagnostic appends fail. Gateway and node-engine producers
  keep those secondary diagnostics failures out of the primary inference and
  preflight result path.
- 2026-05-05: Typed streaming lifecycle boundary slice added preprocessing,
  postprocessing, and result-projection lifecycle phases to
  `stream_typed_text_with_lifecycle`. Successful typed streams now expose the
  same Transformers-aligned public lifecycle boundaries as non-streaming typed
  text execution; failed and cancelled streams do not emit postprocessing or
  result-projection, and legacy raw chat streaming is unchanged.
- 2026-05-06: KV-cache diagnostic hygiene follow-up now filters local
  path-shaped cache ids, reuse sources, and reason strings before durable
  inference diagnostic persistence. Stable KV labels remain queryable, but
  temp paths and `file://` refs cannot persist through KV-cache progress
  metadata.
- 2026-05-03: Generation option diagnostics ledger slice added append-only
  `inference.execution_diagnostic_observed` ledger events with bounded
  option-support counts and per-option summaries. `InferenceRequestLifecycleEvent`
  now carries canonical task ids and bounded option diagnostics, while
  embedded-runtime maps completed backend-execution lifecycle facts into durable
  metadata without importing the diagnostics ledger into `crates/inference` or
  storing prompt/result bodies. Remaining task-option diagnostics for video
  stay open until that contract-only task becomes executable; embedding,
  rerank, image, audio, and KV-cache task-option diagnostics now have bounded
  producers.
- 2026-05-03: Node-status task projection slice added an additive `task_id`
  field to `node_execution.status` payloads and the queryable
  `node_status_projection`, bumped the rebuildable projection version, and
  updated Rust/TypeScript consumer DTOs. This also corrected frontend diagnostics
  DTO drift by exposing the already-durable `selected_backend_key` field in the
  TypeScript projection contract.
- 2026-05-03: Typed streaming lifecycle follow-up now carries the canonical
  `task_id` through backend-execution stream events as well as task-validation
  events. Raw chat lifecycle calls still record no task id unless a typed wrapper
  supplies one.
- 2026-05-03: Compatibility diagnostics slice added ledger-neutral
  `InferenceCompatibilityReportSummary` and
  `InferenceCompatibilityIssueSummary` metadata to inference lifecycle events,
  conversion helpers from backend-owned `BackendCompatibilityReport`, and
  bounded diagnostics-ledger fields on `inference.execution_diagnostic_observed`
  events. Embedded-runtime maps those lifecycle facts into durable metadata
  without importing the diagnostics ledger into `crates/inference`. The open
  follow-up is producer coverage: execution and preflight paths that already
  have package facts must attach the compatibility summaries instead of only
  option diagnostics.
- 2026-05-03: Compatibility-only diagnostic mapping follow-up fixed a coverage
  bug in the embedded-runtime ledger adapter. Completed backend-execution
  lifecycle events now persist `inference.execution_diagnostic_observed` when
  they contain either option diagnostics or compatibility summaries, so
  rejected/degraded compatibility reports are not dropped just because no
  generation option diagnostics were emitted.
- 2026-05-04: Failed lifecycle duration follow-up extended the
  embedded-runtime ledger adapter so failed lifecycle events with a matched
  start can persist bounded `inference.execution_diagnostic_observed` duration
  summaries while keeping failure detail in the node-status/error path instead
  of copying it into inference diagnostic payloads. A follow-up acceptance test
  now proves the host-owned workflow sink records failed node-status lifecycle
  facts through `WorkflowService` with selected backend context and terminal
  duration.
- 2026-05-08: Duration-only lifecycle coverage now table-checks completed
  model-package-resolution, preprocessing, postprocessing, and
  result-projection summaries through the embedded-runtime ledger adapter. The
  coverage freezes that these phases can persist bounded duration metadata
  without option, compatibility, usage, cache, artifact, prompt, or result
  payloads.
- 2026-05-04: Capacity-rebalance unload diagnostics follow-up preserved the
  original runtime unload error when recording the scheduler model lifecycle or
  runtime-load admission diagnostics is unavailable, matching the post-run
  keep-alive unload behavior and preventing diagnostics-ledger availability
  from replacing the execution failure.

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
  freeze, implementation should start with the thinnest useful slice and expand
  through neighboring slices rather than building broad layers in isolation.
  This originally meant GGUF text generation; after the settled 2026-05-06
  Pumas fast selector implementation, Pantograph first completed the Pumas fast
  selector integration, followed by GGUF text generation, GGUF embeddings,
  HF/Transformers text generation, rerank, and multimodal support.
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
- 2026-05-03: Embedded-runtime `puma-lib` execution now resolves full Pumas
  package facts when the Pumas API is available and emits the canonical
  `resolved_model_package_facts` JSON output for downstream `llm-inference`
  nodes. `workflow-nodes` exposes the matching optional JSON output port so
  workflows can connect facts explicitly instead of relying on option metadata.
- 2026-05-03: Canonical `llm-inference` node descriptors and authoring
  contracts now expose `resolved_model_package_facts` as an optional JSON input
  with model-reference payload semantics, completing the graph-visible edge
  between `puma-lib` package-fact output and node-engine typed request parsing.
- 2026-05-03: Node-engine dependency input resolution now carries
  `resolved_model_package_facts` through existing `puma-lib` model-reference
  edges as bounded model context, so saved workflows that already connect
  `pumas_model_ref` can reach package-facts validation without a new edge.
- 2026-05-03: Workflow-service legacy inference migration inventory now treats
  `resolved_model_package_facts` as part of the canonical `llm-inference` graph
  data field set, keeping saved-workflow migration tests aligned with the
  package-facts edge contract.
- 2026-05-03: Frontend and svelte-graph mock node definitions now mirror the
  backend `puma-lib` package-facts output and canonical `llm-inference`
  package-facts input so mock-backed UI tests expose the same graph contract.
- 2026-05-03: The bundled GGUF reranker workflow template now connects
  `puma-lib.resolved_model_package_facts` to canonical
  `llm-inference.resolved_model_package_facts`, and a template regression test
  enforces that Pumas-to-canonical-inference template edges carry both model refs
  and package facts.
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
- 2026-05-04: Tightened typed execution request validation to reject
  whitespace-only prompt, embedding, query, and rerank-document payload strings
  before backend adapters see them.
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
- 2026-05-03: Added task request/result payload contracts to the inference task
  registry. `TaskRequestContract` now defines canonical input/result payload
  families and executable versus contract-only task status, and typed request
  validation consumes that registry contract instead of duplicating the
  task/input map.
- 2026-05-03: Migrated the node-engine canonical text-generation request
  builder to consume `TaskRequestContract` input payload metadata instead of a
  hard-coded text/chat task allowlist, with regression coverage for embedding
  and text-to-image aliases.
- 2026-05-03: Added workflow-service and embedded-runtime projection of
  inference task request/result contracts into runtime capability DTOs, keeping
  the projection as workflow-visible facts rather than scheduler/runtime
  selection policy.
- 2026-05-03: Switched node-engine canonical `llm-inference` dispatch from
  direct embedding/rerank task-id matches to request-contract input payload
  families, preserving the same handlers while reducing duplicated task
  semantics in dispatcher code.
- 2026-05-03: Added node-engine typed result-kind validation before projecting
  text, embedding, and rerank gateway results, so output projection now checks
  the task registry result contract before matching result variants.
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
- 2026-05-03: Added contract-only inference task and port payload metadata to
  `pantograph-node-contracts`, then projected `llm-inference` text/chat,
  embedding, and rerank request/result families from `workflow-nodes` without
  changing frontend node rendering or runtime backend selection.
- 2026-05-03: Migrated `workflow-nodes` inference task descriptor metadata to
  derive task fields from `inference::model_contracts::TaskRequestContract`,
  reducing duplicated task semantics while preserving the graph-facing
  `pantograph-node-contracts` DTOs.
- 2026-05-03: Added canonical `task_kind` and `runtime_hint` inputs to the
  graph-visible `llm-inference` descriptor and annotated them as inference
  option payloads, aligning workflow authoring with existing migration,
  preflight, and execution data fields.
- 2026-05-03: Added task-request contract serde coverage for omitted defaults
  and additive unknown fields, and documented Python Transformers as an
  implementation target rather than the public contract source of truth.
- 2026-05-03: Canonicalized legacy llama.cpp/PyTorch migration output for
  generation options into grouped `GenerationOptions` fields and kept
  node-engine tolerant of already-migrated flat option objects.
- 2026-05-03: Added node-engine rerank request support for migrated
  `task_options.top_k` and `task_options.return_documents`, preserving
  connected/top-level input precedence over saved task options.
- 2026-05-03: Added contract-only audio transcription task and port payload
  metadata to the canonical `llm-inference` descriptor projection, preserving
  the then-current registry status while documenting the audio/response shape
  for graph consumers.
- 2026-05-03: Extended node-engine typed request construction to recover the
  canonical Pumas model identity from `resolved_model_source.model_ref`, so
  resolved model-source facts can carry identity into typed backend requests
  without requiring a duplicate `pumas_model_ref` input.
- 2026-05-03: Added a node-engine guard for contract-only canonical inference
  tasks, so descriptor-visible but non-executable tasks reject with a
  task-contract diagnostic before gateway, backend, or prompt validation.
- 2026-05-03: Updated the node-engine llama.cpp text execution path to honor
  grouped `generation_options.length.max_new_tokens` and
  `generation_options.sampling.temperature` before legacy flat fallbacks.
- 2026-05-03: Added a mixed saved-workflow migration fixture that keeps legacy
  llama.cpp, embedding, and reranker node topology and output bindings stable
  after canonical `llm-inference` migration.
- 2026-05-04: Canonicalized filesystem saved-workflow persistence on save and
  load, so callers that bypass edit sessions still serialize and receive
  canonical `llm-inference` nodes instead of retired inference node types.
- 2026-05-04: Added filesystem persistence coverage for mixed legacy embedding
  and reranker nodes migrating into canonical `llm-inference` task shapes
  before serialization.
- 2026-05-03: Added a positive node-engine dependency preflight fixture for
  HF-compatible Transformers/PyTorch model sources, proving canonical runtime
  hints and Pumas model-source identity reach the host resolver.
- 2026-05-03: Routed contract-only canonical inference task failures through
  the existing host-owned lifecycle diagnostics sink as bounded
  task-validation failed facts when a sink is installed.
- 2026-05-04: Added executable coverage for the contract-only
  `video_understanding` path, proving node-engine emits task-validation
  started/failed/cleanup lifecycle facts before backend execution and the
  embedded-runtime ledger adapter persists the bounded failed task diagnostic
  with selected backend, model, and task context.
- 2026-05-03: Threaded stable task/execution context into canonical PyTorch
  dependency preflight and emits host-owned model-package-resolution lifecycle
  failure facts when preflight blocks execution.
- 2026-05-03: Added same-node saved-workflow canonicalization for legacy
  generic `llm-inference` flat generation settings, moving them into grouped
  `generation_options` and removing stale flat option edges without changing
  node identity or prompt/response topology.
- 2026-05-03: Preserved llama.cpp multimodal `.mmproj` evidence across
  saved-workflow migration and node-engine canonical model-source hydration, so
  llama.cpp runtime matching/startup distinguishes text GGUF from VLM GGUF plus
  companion projection artifacts.
- 2026-05-03: Aligned frontend mock node registries with the canonical
  inference/Puma-Lib descriptors so local frontend tests and mock backends use
  Pumas model-reference, task/runtime hint, diagnostics, and dependency ports
  instead of the old raw-path-only shape.
- 2026-05-03: Removed unused Ollama model-name fields from inference gateway
  start request DTOs and clarified node-engine comments around retired direct
  backend nodes.
- 2026-05-03: Moved embedded-runtime embedding metadata emission detection from
  the retired `embedding` node type to canonical `llm-inference` task-registry
  semantics.
- 2026-05-04: Enabled canonical `audio_transcription` execution through the
  typed registry, gateway, PyTorch backend edge, and node-engine
  `llm-inference` dispatch path. Node-engine now accepts encoded audio or
  host-owned artifact refs, projects text/language/duration/segment outputs,
  and keeps raw audio payloads out of option diagnostics.
- 2026-05-04: Added bounded contract-only `video_understanding` option
  diagnostics for recognized video sampling/window task options, so failed
  task-validation lifecycle facts expose backend-unavailable option support
  summaries without adding a video backend or storing media payloads.
- 2026-05-04: Added node execution status canonical error links to ledger
  payloads, node-status projections, workflow-service contracts, and frontend
  run-graph/diagnostics focus helpers while keeping direct node fatal
  `error_event_id` projections distinct.
- 2026-05-04: Added append-only inference lifecycle canonical error links and
  embedded-runtime pass-through into node-status diagnostics so lifecycle
  summaries can point at directly-known `diagnostic.error_occurred` events
  without duplicating error payloads.
- 2026-05-04: Added contract-only typed payload DTOs for image understanding,
  video understanding, and multimodal generation so the registry's roadmap task
  contracts have stable request/result wire shapes without enabling execution.
- 2026-05-04: Added optional execution-observed selected network node facts to
  inference lifecycle events and durable inference diagnostic payloads, with
  run-list/run-detail projection into the existing selected-network-node
  columns.
- 2026-05-04: Tightened `puma-lib` package-facts summary cache population so
  it polls Pumas update cursors after bounded summary regeneration as well as
  after the startup/page snapshot, closing the stale window for updates that
  arrive while sparse summary rows are regenerated and refilling affected page
  rows against the newest cursor.
- 2026-05-04: Added typed non-streaming text lifecycle producer coverage for
  preprocessing, postprocessing, and result projection, reusing existing
  duration-only durable inference diagnostic mapping without adding ledger
  schema fields.
- 2026-05-04: Extended the same typed non-streaming lifecycle producer coverage
  to embedding, rerank, image generation, and audio transcription so all
  currently executable typed tasks expose preprocessing, backend execution,
  postprocessing, and result projection phases through one inference-boundary
  path.
- 2026-05-04: Validation of the typed text lifecycle slice exposed a Rust
  inference ambiguity in llama.cpp managed-runtime file-name path joins;
  binding the lossy file name as `&str` keeps the platform copy path explicit
  and restores inference crate compilation.
- 2026-05-04: Fixed a workflow-service issue found during full validation:
  edge-insert preview for canonical `llm-inference` could choose text-compatible
  configuration/context ports before the primary `prompt` port after
  task-registry expansion. Insert bridge resolution now prefers primary content
  inputs before secondary context and configuration inputs.
- 2026-05-04: Added typed audio transcription lifecycle coverage proving
  backend-execution events carry canonical `audio_transcription` task ids,
  selected model ids, bounded extra-option diagnostics, and no raw audio
  payloads.
- 2026-05-04: Fixed workflow-node contract drift found during continued
  implementation: canonical `llm-inference` task metadata now mirrors the
  inference task registry's executable `audio_transcription` contract instead
  of retaining stale contract-only wording/assertions.
- 2026-05-03: Follow-up issue discovered during focused embedded-runtime
  validation: `node-engine` still emits a dead-code warning for
  `enforce_dependency_preflight_with_lifecycle` when compiled without the
  PyTorch feature set. Resolve by tightening cfg ownership or adding a focused
  non-PyTorch validation slice.
- 2026-05-03: Resolved the non-PyTorch dead-code warning by cfg-gating
  `enforce_dependency_preflight_with_lifecycle` to PyTorch builds and crate
  tests, preserving direct test coverage without exposing unused symbols to
  non-PyTorch consumers.
- 2026-05-03: Carried selected backend compatibility summaries through the
  technical-fit decision path. Runtime-registry decisions now retain the
  selected candidate's compatibility report and bounded issues, embedded-runtime
  projects those facts into workflow-service DTOs, and workflow-service keeps
  the selected decision in the session preflight cache so post-preflight
  scheduler reservation events can use the selected runtime id. This narrows
  the diagnostics producer gap while leaving full lifecycle/preflight event
  emission open for execution paths that have node context.
- 2026-05-03: Extended scheduler model-lifecycle diagnostics to use the
  post-preflight selected runtime id when technical-fit has chosen a concrete
  runtime, instead of continuing to infer runtime identity only from required
  backend strings.
- 2026-05-03: Added the first execution producer for lifecycle-carried
  compatibility summaries. `InferenceExecutionRequest` can now carry resolved
  Pumas package facts, node-engine forwards those facts from canonical
  inference inputs, and typed gateway lifecycle task-validation completion
  emits backend/model compatibility summaries without importing the diagnostics
  ledger or moving compatibility derivation into node-engine.
- 2026-05-03: Node-engine canonical text, embedding, and rerank request builders
  now reject malformed resolved Pumas package-facts payloads as execution input
  errors instead of silently dropping them before inference compatibility
  validation.
- 2026-05-03: Added the first non-generation option diagnostics producer.
  Typed embedding and rerank lifecycle completion events now emit bounded
  option support summaries for backend extra-option keys and rerank controls,
  leaving image, video, audio, and KV-cache task-specific option diagnostics
  open at that point. Later slices added image, audio, and KV-cache producers;
  video remains contract-only.
- 2026-05-03: Extended typed image-generation results and lifecycle completion
  events with bounded option diagnostics for first-class image settings and
  backend-specific image extra-option keys, without recording prompts or image
  payload bodies.
- 2026-05-03: Added lifecycle phase/kind metadata to durable inference
  diagnostic-observed payloads, so compatibility and option summaries retain
  the lifecycle phase that produced them without requiring consumers to infer it
  from task ids or node-status projections.
- 2026-05-03: Workflow run-detail queries now return matching node-status
  projection rows plus node projection state, exposing selected node task,
  backend, runtime, and model context without requiring API/UI consumers to
  parse raw ledger payload JSON.
- 2026-05-03: Diagnostics run-list and run-detail projections now roll up
  node-derived selected backend key, model id, and task id from
  `node.execution_status` events while preserving scheduler-owned selected
  runtime/device/network-node semantics.
- 2026-05-03: Inference lifecycle and durable diagnostic payloads now carry
  bounded usage summaries and cache-handle ids for completed typed backend
  execution events without storing prompt, output, token-array, tensor, or raw
  artifact payloads. Backend-specific usage producers, KV checkpoint ids, and
  artifact refs remain follow-up work.
- 2026-05-03: Typed embedding backend execution now aggregates embedding item
  token counts into bounded prompt/total usage summaries for lifecycle
  diagnostics without storing vectors or input text.
- 2026-05-03: Failed inference lifecycle detail is now sanitized and capped
  before it is copied into `node.execution_status` diagnostics, reducing the
  chance that secondary diagnostic appends fail because a backend surfaced
  oversized or control-character-heavy error text. The original execution
  error path remains separate from this bounded diagnostic copy.
- 2026-05-03: Added a payload-hygiene regression for usage/cache inference
  diagnostics proving raw-looking prompt, result, and tensor markers in
  lifecycle detail are not serialized into durable inference diagnostic summary
  payloads.
- 2026-05-03: Workflow-run error paths now preserve the original workflow
  execution, timeout, output-validation, and artifact-conversion errors when
  their diagnostic record append fails, attaching `diagnostics_unavailable`
  instead of replacing the run failure with a diagnostics failure. The
  workflow-session terminal append path has the same preservation behavior for
  already-failed run results.
- 2026-05-03: Updated source README contracts for the diagnostics slices:
  inference lifecycle usage/cache metadata, diagnostics-ledger bounded
  inference summaries and node-derived selected backend/model/task rollups,
  embedded-runtime lifecycle sink projection rules, and workflow-service
  `diagnostics_unavailable` semantics are now documented at the owning source
  boundaries.
- 2026-05-03: Embedded-runtime now persists bounded task-validation
  compatibility summaries from inference lifecycle events into
  `inference.execution_diagnostic_observed` payloads, so typed gateway
  compatibility reports emitted before backend execution are durable without
  adding ledger dependencies to `crates/inference`.
- 2026-05-03: Durable inference diagnostic summaries now include optional
  `duration_ms` when the workflow-owned lifecycle sink can correlate a terminal
  inference lifecycle event with its matching start. The stateless adapter still
  emits summaries without synthetic timing, so duration remains an observed
  fact rather than an inferred one.
- 2026-05-03: Added bounded KV-cache diagnostic references to durable
  `inference.execution_diagnostic_observed` payloads and introduced a
  host-owned workflow event sink that maps structured node-engine
  `TaskProgressDetail::KvCache` facts into ledger events. The slice wires this
  sink into request-scoped workflow, warm workflow-session, data-graph, and
  edit-session execution paths while keeping cache bytes, fingerprints, and temp
  paths out of durable diagnostics.
- 2026-05-03: I/O artifact projection now exposes `selected_backend_key` and
  fills missing runtime/model/backend context from the latest raw
  `node.execution_status` event for the same workflow run and producer node with
  `event_seq` at or before the artifact event. Workflow-service artifact events
  remain payload-boundary facts and do not pre-enrich model identity from a
  latest-node projection that could drift after later executions.
- 2026-05-03: Frontend diagnostics types and I/O Inspector presenter rows now
  expose projected runtime, selected backend, and model identifiers for
  artifacts when those facts are present, while keeping missing facts out of the
  compact descriptor view.
- 2026-05-03: I/O Inspector page filters can now pass `selected_backend_key`
  through the workflow projection service and artifact detail rows display the
  selected backend alongside runtime/model projection facts.
- 2026-05-03: Workflow projection service README now names
  `selected_backend_key` as a preserved I/O artifact projection filter and
  response fact so frontend services keep backend-owned artifact context intact.
- 2026-05-03: Embedded-runtime lifecycle diagnostics now persist
  duration-only completed summaries for model-package resolution,
  preprocessing, postprocessing, and result projection when a matched lifecycle
  start supplies observed timing. Durationless lifecycle events without bounded
  diagnostics remain suppressed.
- 2026-05-03: Workflow-session keep-alive-disabled unload failures now preserve
  the original unload error when the secondary scheduler lifecycle diagnostic
  append fails, preventing diagnostics ledger availability from replacing the
  runtime failure returned to the caller.
- 2026-05-04: Typed gateway lifecycle now emits package-resolution phase facts
  when `InferenceExecutionRequest` carries resolved Pumas package facts, for
  both non-streaming and streaming typed text paths. Streaming backend lifecycle
  events reuse the package-derived model id so task-validation and backend
  execution diagnostics do not drift back to transport-local model names.
- 2026-05-04: Typed request validation now rejects clean package/task
  mismatches before backend execution. Package facts with sparse or unresolved
  task evidence still flow to compatibility diagnostics, but a text request
  carrying embedding package facts fails during task validation with bounded
  request task, package task, and model identity.
- 2026-05-04: Node-engine canonical `llm-inference` execution now has
  consumer-boundary coverage proving forwarded Pumas package facts participate
  in the same task validation. A text node carrying embedding package facts
  emits package-resolution and task-validation lifecycle facts and never reaches
  the mock backend.
- 2026-05-04: Node-engine canonical `llm-inference` execution now also covers
  the successful package-facts path. Text package facts flow from graph inputs
  into typed gateway lifecycle events, task-validation and backend-execution
  compatibility summaries are emitted with package-derived model identity, and
  package-resolution completion remains compatibility-summary free.
- 2026-05-04: Node-engine canonical embedding execution now has matching
  package-facts lifecycle coverage. Embedding package facts flow from graph
  inputs through typed gateway execution, and task-validation plus
  backend-execution lifecycle events carry compatibility summaries with the
  package-derived embedding model id.
- 2026-05-04: Node-engine canonical image-generation execution now routes
  `llm-inference` `task_kind = image_generation` through the typed gateway.
  The slice parses direct and nested image-generation options, falls back to
  resolved Pumas package facts for model identity, projects generated images
  through existing graph outputs, and verifies diagnostics do not include
  prompt text or encoded image bytes. The 2026-05-08 output contract slice
  additionally exposes the first generated image on the graph-visible `image`
  port so canonical image-generation templates can wire directly into
  `image-output` without parsing the typed `results` envelope.
- 2026-05-08: The bundled tiny-sd-turbo text-to-image template now uses
  canonical `llm-inference` with `task_kind = image_generation` and
  `runtime_hint = diffusers`, carries `pumas_model_ref`,
  `resolved_model_package_facts`, and `inference_settings` from `puma-lib`, and
  connects the canonical `image` output to `image-output`. A frontend template
  regression test now prevents built-in image-generation starters from drifting
  back to direct `diffusion-inference` or raw `model_path` handoff. A follow-up
  guardrail now scans every registered built-in workflow template for retired
  direct inference node types so new starters stay on canonical `llm-inference`
  task shapes.
- 2026-05-08: The retired direct `diffusion-inference` descriptor is no longer
  submitted to the workflow-node inventory, so built-in node definitions and
  frontend registry maps no longer expose it as an authorable graph node.
- 2026-05-08 follow-up: removed the now-unreferenced direct-diffusion workflow
  descriptor module and frontend renderer component. Remaining
  `diffusion-inference` references are limited to historical plan text,
  explicit retired-node test guardrails, and documentation invariants that
  prevent the retired shape from re-entering authoring or execution paths.
- 2026-05-08 follow-up: node-engine dependency preflight no longer
  special-cases the retired direct `diffusion-inference` node shape for backend
  selection, task inference, or dependency resolver dispatch. Image-generation
  dependency checks now enter through canonical `llm-inference` task metadata
  and resolved package facts.
- 2026-05-08 follow-up: embedded-runtime host dispatch no longer sends retired
  direct `diffusion-inference` nodes to the Python sidecar, and the process
  bridge no longer exposes the diffusion worker path. Host Python coverage now
  exercises audio and ONNX sidecar behavior while canonical image generation
  stays on the inference task boundary.
- 2026-05-08 follow-up: embedded-runtime model dependency descriptors no
  longer preserve a direct `diffusion-inference` backend default. Diffusers
  descriptor tests now use canonical `llm-inference` dependency requests with
  explicit task/backend facts from Pumas.
- 2026-05-08 follow-up: embedded-runtime saved-workflow and data-graph
  integration fixtures no longer construct retired direct `diffusion-inference`
  graphs. Python-sidecar scheduler, registry, and edit-session coverage now
  uses audio/ONNX fixtures while image-generation execution remains on the
  canonical inference task contract.
- 2026-05-08: Puma-Lib hydration dependency preflight now infers canonical
  `llm-inference` for `text-to-image` and `image-to-image` package facts instead
  of routing the request through the retired direct `diffusion-inference` node
  shape. Backend and task metadata remain in the dependency request so Pumas
  facts and selected bindings still drive dependency resolution.
- 2026-05-04: Node-engine canonical image-generation execution now has
  package-facts lifecycle coverage. Diffusers package facts flow from graph
  inputs through typed gateway execution, and task-validation plus
  backend-execution lifecycle events carry compatibility summaries with the
  package-derived image model id while omitting prompt and image payloads.
- 2026-05-04: Updated node-engine and workflow-nodes source README contracts
  for executable canonical image-generation so the documented consumer and
  producer boundaries match the typed gateway implementation and descriptor
  payload metadata.
- 2026-05-04: Embedded-runtime ledger adapter coverage now persists bounded
  image-generation backend-execution lifecycle summaries, including task id,
  backend family, artifact kind, compatibility summary, and option support
  counts, while excluding prompt text, encoded image bytes, backend flags, and
  local paths.
- 2026-05-04: Fixed descriptor drift after enabling executable
  image-generation: `workflow-nodes` now includes image-generation in canonical
  `llm-inference` task contracts, prompt input payloads, Pumas model-reference
  payloads, and `results` output payloads, with workflow-service registry
  projection coverage.
- 2026-05-04: Added workflow-service registry coverage for image-generation
  payload projection so prompt input and `results` output metadata survive the
  graph registry boundary with snake_case task/result labels.
- 2026-05-04: Fixed a model-list option-provider drift where
  `ModelExecutionDescriptor.task_type_primary = unknown` was treated as
  authoritative. The provider now matches embedded-runtime dependency
  descriptor semantics by falling back to record task metadata only when the
  descriptor task is missing or `unknown`.
- 2026-05-04: Tightened `puma-lib` model-list option metadata so runtime-engine
  hints and the `requires_custom_code` flag prefer Pumas package-summary DTO
  facts over raw record metadata whenever summaries are available. Remaining
  metadata fallbacks are recorded under Milestone 2 as consumer-boundary gaps
  until Pumas exposes equivalent summary/detail facts or Pantograph omits them.
- 2026-05-04: Tightened the same model-list option boundary for review/custom
  code display facts: summary diagnostic codes now populate bounded
  `review_reasons`, and raw record `custom_code_sources` are omitted whenever a
  package-summary DTO is available because Pumas does not expose source-level
  details through that summary contract yet.
- 2026-05-04: Tightened the remaining `puma-lib` model-list boundary for
  dependency/settings display facts: dependency binding metadata now comes from
  the public Pumas execution descriptor dependency-resolution DTO when
  available, and API-unavailable inference-settings fallback computes bounded
  model-type defaults without reading stored record metadata.
- 2026-05-04: Added PyTorch Rust/Python worker envelope additive-field
  tolerance coverage for load, generate, and response contracts while keeping
  backend-owned `transformers_kwargs` allowlist enforcement strict.
- 2026-05-04: Reconciled Milestone 2 with the implemented Pumas package-summary
  cache path: Pantograph treats the Pumas plan as the producer-side source,
  keeps Pumas storage/indexing out of inference scope, and `workflow-nodes`
  consumes startup snapshot cursors plus update feeds to invalidate or refresh
  cached package-facts summaries.
- 2026-05-04: Added PyTorch worker error-kind normalization coverage for the
  complete worker error enum, including load, stream setup, and generate
  response-boundary tests that preserve request ids and canonical worker codes.
- 2026-05-04: Tightened `puma-lib` model-list sparse-summary handling so
  versioned Pumas summary API outputs with missing or invalid summary payloads
  suppress stale raw record metadata for backend hints, custom-code facts,
  custom-code sources, and review reasons.
- 2026-05-04: Removed a direct PyTorch backend-name conditional from
  embedded-runtime host capability projection by deriving Python-sidecar
  exclusion from the same runtime-id set used to publish Python runtime
  capabilities, with alias coverage for `torch` and neighboring sidecars.
- 2026-05-04: Tightened embedded-runtime `puma-lib` execution DTO consumption
  so stale saved `recommended_backend` node data is replaced by the Pumas
  `ModelExecutionDescriptor.recommended_backend` fact when descriptor lookup
  succeeds, while preserving saved fallback behavior when Pumas facts are
  unavailable.
- 2026-05-08: Tightened embedded-runtime `puma-lib` inference-settings
  consumption so selected-detail batch settings from owner/local-client Pumas
  selector access replace stale saved node settings when non-empty, while
  read-only selector access preserves saved settings because selector rows do
  not own rich settings facts.
- 2026-05-08: Removed the remaining Tauri `puma-lib` `model_id` hydration
  fallback to raw owner `PUMAS_API`; selected model ids now require explicit
  selector access, with raw owner API retained only for path-only model-ref
  migration.
- 2026-05-08: Moved technical-fit full package-fact resolution to explicit
  Pumas selector access. Owner selector access can produce full facts;
  local-client/read-only roles remain summary-only in Pumas v0.6.0 and now
  degrade rather than promoting summaries to full inference package facts.
- 2026-05-08: Removed embedded-runtime `puma-lib` execution fallback from saved
  `model_id` to raw owner `PUMAS_API`; execution now requires selector access
  for selected model rehydration and keeps raw owner access only for optional
  full package-facts enrichment.
- 2026-05-04: Added append-only terminal `ChatChunk` usage counts and threaded
  typed text-generation usage into result and backend-execution lifecycle
  summaries, with regression coverage proving prompt/result text stays out of
  lifecycle diagnostics.
- 2026-05-04: Preserved OpenAI-compatible llama.cpp SSE usage payloads in
  backend stream chunks, including usage-only terminal events, while bounding
  token counts to the public `InferenceUsage` contract.
- 2026-05-04: Updated inference source and backend README contracts for
  append-only stream usage metadata and llama.cpp SSE usage parsing, keeping
  Milestone 14 documentation aligned with the public chunk/result contracts.
- 2026-05-04: Threaded terminal typed-stream chunk usage into backend-execution
  lifecycle completion events, with regression coverage proving prompt and
  generated stream content remain absent from lifecycle diagnostics.
- 2026-05-04: Updated node-engine dependency preflight to prefer resolved Pumas
  package facts for backend/task/model facts after explicit hints and before
  legacy `llm-inference` heuristics, keeping preflight factual and out of
  scheduler/runtime selection.
- 2026-05-05: Added backend stream `ChatChunk` serde coverage for omitted
  usage and bounded usage-count metadata, freezing the append-only stream
  diagnostics contract introduced by typed text streaming.
- 2026-05-05: Node-engine `llm-inference` output projection now preserves
  bounded text/chat token usage from typed gateway results and terminal stream
  chunks on the existing `usage` graph port, with tests for both streaming and
  non-streaming paths.
- 2026-05-05: Updated node-engine source and core-executor README contracts for
  canonical text/chat `usage` output, documenting bounded token-count metadata
  hygiene and the rule against deriving usage from prompt or generated content.
- 2026-05-05: Node-engine dependency preflight now emits bounded
  model-package-resolution lifecycle facts on successful canonical PyTorch
  preflight, mirroring the existing failure lifecycle shape while keeping
  scheduler/runtime selection and backend-local payloads out of the producer
  contract.
- 2026-05-05: Node-engine dependency preflight lifecycle context now carries
  the bounded resolved artifact-kind label from package facts or
  `resolved_model_source`, so package-resolution diagnostics can distinguish
  HF-compatible directories from GGUF and other artifact families without
  adding artifact refs, local paths, compatibility summaries, or ledger writes.
- 2026-05-05: PyTorch task-join failures now use the shared worker-message
  sanitizer before becoming `BackendError` values, preserving the existing
  error variants while stripping traceback frames and local path tokens from
  async task-boundary failure text.
- 2026-05-06: Embedded-runtime inference lifecycle ledger projection now keeps
  only label-shaped `resolved_artifact_kind` values and drops path-shaped or
  unsafe values before persistence, while allowing a bounded artifact-kind
  label to make completed lifecycle diagnostics persistable without artifact
  refs or local paths.
- 2026-05-06: PyTorch worker response decode failures now normalize through
  canonical worker failure codes for model load, non-streaming generation, and
  stream setup, preserving request ids and avoiding raw malformed response
  payload text in `BackendError` messages.
- 2026-05-06: PyTorch audio transcription worker result decoding now fails
  closed through the typed worker response envelope and canonical
  `pytorch_worker_audio_transcription_failed` errors when required transcript
  text is missing, optional language/duration fields are malformed, response
  JSON is malformed, or Python returns a structured worker error, preventing
  silent empty/default transcript values from malformed Python worker output.
- 2026-05-06: PyTorch audio transcription request inputs now use a versioned
  Rust/Python worker envelope with typed Rust DTOs, torch-free Python projection,
  fixture coverage, required model/audio validation, additive-field tolerance,
  and explicit rejection of unsupported non-null audio `extra_options`.
- 2026-05-06: Typed PyTorch worker response decoders now reject mismatched
  response request ids for model load, non-streaming generation, stream setup,
  and audio transcription through the operation-specific canonical worker
  failure path, so stale or cross-wired worker responses cannot be trusted as
  the active request.
- 2026-05-06: PyTorch KV-cache loaded-model and live-KV metadata extraction now
  routes unavailable active-model state and malformed Python result shapes
  through canonical `pytorch_worker_kv_*_failed` backend errors with generated
  request ids instead of ad hoc inference errors.
- 2026-05-06: PyTorch model unload now uses a versioned Rust/Python worker
  envelope and typed response decoder with request-id correlation,
  malformed-response rejection, and canonical sanitized unload errors before
  clearing Rust-side loaded-model state.
- 2026-05-06: PyTorch loaded-model info now crosses the embedded Python
  boundary through a versioned `get_loaded_info` worker envelope and typed
  response decoder, preserving request-id correlation and canonical sanitized
  KV loaded-info errors before node-engine KV-cache preparation consumes active
  model facts.
- 2026-05-06: PyTorch live-KV clear now crosses the embedded Python boundary
  through a versioned `clear_kv_cache` worker envelope and typed response
  decoder, with operation/version validation, request-id correlation, malformed
  response rejection, and canonical sanitized KV clear errors shared by the free
  helper and trait slot-clear path.
- 2026-05-06: PyTorch live-KV save and restore now cross the embedded Python
  boundary through versioned `save_kv_cache` and `restore_kv_cache` worker
  envelopes and typed live-KV metadata response decoders, with path validation,
  request-id correlation, malformed response rejection, and canonical sanitized
  KV save/restore errors shared by the free helpers and trait slot paths.
- 2026-05-06: PyTorch persisted KV truncation now crosses the embedded Python
  boundary through a versioned `truncate_kv_cache` worker envelope and typed
  response decoder, with temp-path and token-position validation, request-id
  correlation, malformed response rejection, and canonical sanitized KV
  truncate errors before the temporary KV file is read back.
- 2026-05-06: PyTorch backend `stop()` now uses the same versioned unload
  worker envelope and typed response decoder as async model unload for
  best-effort synchronous cleanup, logging cleanup failures instead of calling
  raw Python worker lifecycle functions directly.
- 2026-05-06: PyTorch worker initialization now crosses the embedded Python
  boundary through a versioned `init_worker` envelope and typed response decoder
  after sibling-module registration, preserving startup request ids and
  canonical sanitized init errors before backend ready-state publication.
- 2026-05-06: PyTorch backend health checks now reuse the same versioned
  `init_worker` envelope and typed response decoder when the backend is ready,
  returning `false` on init-envelope failure instead of validating only raw
  Python module importability.
- 2026-05-06: Node-engine PyTorch unload now calls the inference crate's
  exported embedded-worker unload helper, so shared canonical unload handling
  no longer imports `pantograph_torch_worker` or calls raw Python lifecycle
  methods directly.
- 2026-05-06: Validation exposed that node-engine's remaining legacy PyTorch
  worker initializer did not embed `worker_contract.py` even though the shared
  worker imports it. The initializer now mirrors the inference backend loader
  for that sibling module while the remaining generation path is migrated.
- 2026-05-06: Node-engine's legacy PyTorch text execution path now checks the
  active loaded model through `inference::backend::pytorch::active_loaded_model_info`,
  preserving the old "no model loaded means load" behavior by treating
  `BackendError::NotRunning` as a load signal while other typed lookup failures
  fail closed.
- 2026-05-06: Node-engine PyTorch text model loading now calls
  `PyTorchBackend::load_model`, so direct-path loads cross the same
  Transformers-backed `load_transformers_model` worker envelope as the
  inference backend instead of calling the raw Python `load_model` function.
- 2026-05-06: Node-engine non-streaming PyTorch text generation without legacy
  custom `inference_settings` now uses `PyTorchBackend::generate`, moving the
  common generation path onto the inference-owned `generate_text` worker
  envelope while retaining the direct worker path for custom kwargs until they
  are modeled as typed generation options.
- 2026-05-06: Node-engine now also routes single-option `top_k` PyTorch
  generation settings through the typed inference path by exposing
  `PyTorchBackend::generate_with_top_k`; mixed or unsupported legacy kwargs
  still use the compatibility fallback.
- 2026-05-06: PyTorch streaming token projection now accepts both plain string
  chunks and worker dictionaries such as `{"mode": "replace", "text": ...}`,
  preventing the inference-owned stream adapter from rejecting dLLM replacement
  chunks before node-engine streaming can migrate.
- 2026-05-06: Node-engine PyTorch streaming now uses
  `PyTorchBackend::generate_stream_with_top_k` when settings are empty or only
  typed `top_k`, leaving the direct `generate_tokens` compatibility path only
  for mixed custom kwargs that still require legacy event-mode projection.
- 2026-05-06: Project direction was clarified to cut old Pantograph-internal
  compatibility paths instead of preserving them. Node-engine PyTorch execution
  now rejects unsupported `inference_settings`, removes its embedded
  `pantograph_torch_worker` loader and direct PyO3 generation calls, and relies
  on the inference crate as the only PyTorch worker owner.
- 2026-05-06: Embedded-runtime canonical `llm-inference` no longer routes
  PyTorch/Transformers hints through the host Python runtime bridge. The bridge
  now rejects legacy `pytorch-inference`/`llm-inference` dispatch, dependency
  preflight for host Python execution is scoped to diffusion, audio generation,
  and ONNX nodes, and tests prove canonical LLM work falls through to the core
  node-engine/inference boundary.
- 2026-05-06: Workflow-node processing contracts no longer export retired
  `ollama-inference` or `pytorch-inference` descriptor structs. Saved graph
  upgrade knowledge remains in workflow-service canonicalization fixtures,
  while the workflow-node crate exposes only current graph-visible contracts.
- 2026-05-06: Node-engine removed the retained stale-node executor branches for
  `ollama-inference` and `pytorch-inference`, including the retired Ollama guard
  module. Canonicalization remains the saved-workflow upgrade path, and direct
  stale task execution now follows the standard unsupported node branch.
- 2026-05-06: Inference managed-runtime contracts dropped the retired Ollama
  binary id and definition entirely. Runtime capability projection, neutral
  dependency projection, and managed-runtime operation tests now cover only
  supported managed runtime families instead of preserving Ollama compatibility
  states.
- 2026-05-06: Tauri config and process spawning no longer preserve the retired
  `ollama_vlm_model` field or a dedicated Ollama sidecar rejection branch.
  Existing unknown config keys are ignored by serde, while process spawning
  routes old Ollama targets through the standard unsupported target error.
- 2026-05-06: Pantograph UniFFI and Rustler bindings removed the Pumas
  `is_ollama_running` wrapper and smoke binding entry, so host-language
  Pantograph APIs no longer expose an Ollama-specific probe even if Pumas keeps
  provider-neutral discovery helpers internally.
- 2026-05-06: Reviewed the settled Pumas fast selector implementation through
  commit `6c564404`. The next Pantograph implementation slice should consume
  `ModelLibrarySelectorSnapshot` as the list/selector entry point, select an
  explicit owner/local-client/read-only Pumas access role, pass the model-library
  root to `PumasReadOnlyLibrary`, and reserve package-summary, execution-
  descriptor, and inference-settings batch hydration for selected or expanded
  models.
- 2026-05-07: Reconciled the Pantograph implementation against that settled
  Pumas selector contract. `workflow-nodes` now consumes
  `ModelLibrarySelectorSnapshot` for puma-lib options through the explicit
  owner/local-client/read-only selector-access role; frontend model-option
  consumers share the cursor handoff cache; and validation confirms selector
  population does not perform per-row detail hydration.
- 2026-05-07: Closed a Pumas selector cursor edge case discovered during the
  cache handoff review. `PortOptionsResult` now has optional structured
  metadata, `puma-lib` writes the selector cursor at result scope, and the
  frontend cache can poll the Pumas update feed even when the startup snapshot
  contains zero options.
- 2026-05-06: Standards pass added a blast-radius and implementation guardrail
  section covering selector integration, selected-detail hydration, canonical
  contracts, graph migration, runtime technical fit, diagnostics ledger
  projection, backend adapters, managed dependencies, frontend projections, and
  language bindings. Future code slices must declare write sets, serial owners,
  decomposition decisions, and verification gates before implementation.
- 2026-05-06: Shared runtime identity dropped Ollama from known runtime/backend
  alias normalization and display-name lookup. Workflow-service canonicalization
  remains the only owner of old Ollama saved-workflow migration semantics.
- 2026-05-06: Runtime-registry and embedded-runtime registry tests replaced
  generic live `ollama` runtime fixtures with supported PyTorch fixtures, so
  registry observation and technical-fit override coverage no longer presents
  Ollama as a selectable runtime candidate.
- 2026-05-06: Inference gateway startup-config builders removed their dedicated
  Ollama rejection branch and helper. Backend registry tests still prove Ollama
  is not available, and manual unsupported gateway names now follow normal
  backend config errors rather than a retained Ollama policy path.
- 2026-05-06: PyTorch KV-cache truncation temp-file read/write failures now
  route through canonical `pytorch_worker_kv_truncate_failed` errors with a
  generated request id and shared path sanitizer instead of ad hoc inference
  errors that could expose local temp paths.
- 2026-05-05: Added append-only `ChatChunk.cache_handle_id` stream metadata and
  threaded terminal text/chat cache-handle ids through typed gateway results,
  backend-execution lifecycle completion events, and `llm-inference.kv_cache_out`
  projection while keeping raw KV state and prompt/result bodies out of durable
  diagnostics.
- 2026-05-05: Added the backend-neutral `cache_handle` inference payload role
  to Rust node contracts and mirrored svelte-graph types, then moved
  `llm-inference.kv_cache_out` contract metadata from generic task output to
  cache-handle metadata with workflow-nodes and TypeScript mock coverage.
- 2026-05-05: Aligned app-level workflow service TypeScript types, mocks, and
  inference payload display presenters with the `cache_handle` payload role so
  frontend mock `kv_cache_out` metadata no longer drifts from Rust contracts.
- 2026-05-05: Updated the frontend workflow service source README to document
  `WorkflowBackendTaskCapability.request_contract` as host capability task
  metadata and to require TypeScript mirrors to carry append-only task ids such
  as `depth_estimation` with Rust DTO changes.
- 2026-05-05: Strengthened the workflow-service host capability contract
  snapshot with a roadmap `depth_estimation` task request contract so Rust
  fixture coverage matches the TypeScript capability DTO mirror.
- 2026-05-05: Removed stale app-facing Ollama guidance from the Tauri RAG
  indexing fallback and runtime-manager README after consumer migration review
  found Ollama still named as a recommended RAG/embedding option. Validation
  also exposed Tauri test-build drift from removed Ollama start-request fields,
  new resolved-device runtime mode fields, and KV-cache option diagnostics; the
  same slice aligned those fixtures with current Rust DTOs.
- 2026-05-05: Canonical non-audio `llm-inference` package-facts request
  builders now derive typed request `model_ref` from resolved Pumas package
  facts for text generation, embedding, and rerank when no explicit model ref
  input is present; dependency input merging now preserves the same context
  through direct package-facts edges. Node-engine validation also exposed one
  remaining PyTorch KV-cache diagnostic initializer missing the append-only
  `option_diagnostics` field, which was aligned in the same validated slice.
- 2026-05-05: Inference gateway typed model-name projection now falls back to
  resolved Pumas package facts after explicit `model_name` and `model_ref`,
  covering direct typed callers that bypass node-engine request builders.
- 2026-05-05: Updated inference and node-engine source READMEs for the
  package-facts model identity fallback and dependency-input context
  propagation contracts.
- 2026-05-05: Structured PyTorch worker error responses now sanitize traceback
  frames and local path tokens before `PyTorchWorkerFailure::into_backend_error`
  formats the backend error, matching the existing transport-exception
  hygiene path.
- 2026-05-05: Embedded-runtime KV-cache workflow-event sinks now surface
  durable diagnostic append failures as `diagnostics_unavailable` event-sink
  errors after forwarding the original workflow event, and the plan records the
  remaining fire-and-forget inference lifecycle sink limitation explicitly.
- 2026-05-05: Fixed embedded-runtime workflow capability and technical-fit
  projections for the roadmap `depth_estimation` task after validation exposed
  non-exhaustive matches introduced by the depth task contract slice.
- 2026-05-05: Removed workflow-nodes Puma-Lib model-list raw
  `ModelRecord.metadata` fallbacks for executable option metadata. Task,
  backend-hint, custom-code, review, and dependency-binding facts now come from
  versioned Pumas execution descriptors/package-summary DTOs or bounded
  public-record defaults.
- 2026-05-05: Removed embedded-runtime model dependency descriptor raw
  `ModelRecord.metadata` fallbacks for entry-path matching and task fallback.
  Dependency lookup now matches public execution-descriptor entry paths and
  unknown descriptor tasks fall back to saved request facts.
- 2026-05-05: Marked the Pantograph-owned technical-fit candidate derivation
  item complete after focused embedded-runtime and workflow-service
  `technical_fit` tests verified Pumas package facts, backend capabilities,
  runtime registry state, and workflow requirements project through Pantograph
  contracts without moving selection policy into Pumas or inference.
- 2026-05-05: Added roadmap `depth_estimation` task/request contract coverage
  to the inference registry and node-contract projection enums. The task is
  image-input, image/point-cloud-output, non-streaming, and not executable
  through the typed inference gateway yet.
- 2026-05-05: Aligned frontend workflow-service and shared svelte-graph
  TypeScript execution-kind mirrors with the Rust roadmap `depth_estimation`
  task so public input/result payload contracts compile against the same task
  registry labels.
- 2026-05-05: Aligned the app-level workflow backend-capability TypeScript
  task-id mirror with Rust `WorkflowInferenceTaskId::DepthEstimation`, with
  frontend coverage for optional task request contracts in host capability
  facts.
- 2026-05-04: Aligned frontend and shared svelte-graph mock
  `llm-inference` node definitions with the Rust executable
  `inference_payloads` contracts for text/chat, embedding, rerank,
  image-generation, and audio-transcription task families, with frontend test
  coverage proving the mock surface carries payload facts without adding
  backend/runtime policy inputs.
- 2026-05-05: Strengthened Rust workflow-service registry and frontend mock
  tests to scan every canonical `llm-inference.inference_payloads` payload for
  forbidden backend/runtime/scheduler policy fields, preventing payload metadata
  from growing backend key, runtime id, admission, reservation, eviction, or
  priority semantics.
- 2026-05-05: Added workflow-service effective-definition coverage for dynamic
  saved-workflow port overlays that include `inference_payloads`, proving the
  overlay path preserves snake_case task id, role, and result-kind metadata
  through both API-facing node definitions and node-contract projection without
  introducing backend/runtime/scheduler policy.
- 2026-05-04: Updated the touched frontend workflow-service and shared
  svelte-graph backend README contract sections so mock `inference_payloads`
  are documented as task payload facts only, not backend choice, runtime
  residency, or scheduler policy.
- 2026-05-04: Closed Milestone 1 vocabulary checklist items that are now backed
  by the implementation strategy, raw-facts-versus-policy boundary language,
  and public inference DTO guardrail tests for scheduler-policy terminology.
- 2026-05-04: Added frontend workflow-service error parsing coverage for
  backend envelopes that carry `diagnostics_unavailable` without a diagnostic
  event id, preserving the original error code/message while exposing the
  diagnostics append failure link to callers.
- 2026-05-04: KV-cache task progress now carries bounded option diagnostics for
  truncate marker/token-position controls. Marker truncation is reported as
  honored, token-position truncation is reported as ignored when both are
  supplied, and embedded-runtime maps those facts into durable option-support
  summaries without storing cache bytes, token arrays, or backend handles.

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
- Current plan setup has known documentation-only dirtiness under
  `docs/plans/inference-execution-boundary-contracts/`. The existing untracked
  root `PROPOSAL-pumas-library-fast-model-snapshot.md` is not part of this
  standards pass and should be resolved separately before implementation
  commits. Do not treat documentation dirtiness as permission to start
  implementation if source, test, config, lockfile, generated, or
  build-manifest files are also dirty.

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

- An implementation slice cannot identify a bounded primary write set,
  adjacent write set, forbidden shared files, and verification gate.
- A slice needs to edit shared public contracts, generated bindings, lockfiles,
  root manifests, saved-workflow fixtures, diagnostics projection DTOs, or
  feature flags already owned by another active slice.
- A touched file crosses the decomposition thresholds and the slice cannot
  safely split, extract, or explicitly defer decomposition.
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
- Pumas fast selector snapshots cannot populate Library or graph selectors
  without per-row package-fact/detail hydration.
- Pantograph cannot resolve the Pumas model-library root for read-only selector
  access without relying on launcher/source repository paths.
- Pantograph requires hidden Pumas client fallback behavior instead of explicit
  owner/local-client/read-only access roles.
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
- 2026-05-07: Decision-traceability tooling no longer names the removed Ollama
  managed-runtime platform directory as an active structured producer.
- 2026-05-07: Embedded-runtime dependency preflight no longer scans the full
  Pumas library and hydrates each execution descriptor to guess from path-only
  workflow data. Path-only requests now use Pumas model-ref resolution as the
  bounded migration aid, and unresolved paths remain explicit unresolved input.
- 2026-05-07: The reusable `@pantograph/svelte-graph` Puma-Lib node no longer
  persists selector option labels as `modelName`. Selection now writes
  canonical Pumas model/task/backend metadata from option metadata and leaves
  labels as UI-only display data.
- 2026-05-08: Pantograph pinned `pumas-library` to the released upstream
  `v0.6.0` tag after verifying the release commit and Rust crate metadata.

### Deviations

- Full `cargo test -p inference` is blocked by an unrelated managed
  redistributable path expectation mismatch recorded above.
- Earlier embedded-runtime no-default feature checking was unblocked after
  Pumas restored package-fact helper support.
- Earlier node-engine dependency-preflight feature checking was unblocked after
  PyTorch compatibility-report assertions were cfg-gated to `pytorch-nodes`,
  matching the producer that emits those reports.

### Follow-Ups

- Keep selected-detail hydration separate from selector-row population if
  future UI work adds expanded-row package facts, execution descriptors, or
  inference-settings panels.
- Continue treating Pumas package-summary/update-feed integration as
  invalidation and selected-detail infrastructure, not as the primary list-row
  population path.
- If path-only model dependency requests need to resolve external Pumas
  `entry_path` values without saved `model_id`, add a bounded Pumas resolver
  API instead of reintroducing Pantograph-side full-library descriptor scans.

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
- `cargo test -p workflow-nodes --features model-library --lib puma_lib`
  passed after reconciling the Pumas fast selector integration.
- `cargo test -p workflow-nodes --features model-library read_only` passed
  after reconciling the selector access-role/read-only root behavior.
- `cargo test --manifest-path src-tauri/Cargo.toml puma_lib_commands` passed
  after reconciling the selector access-role command routing.
- `node --experimental-strip-types --test
  src/services/workflow/pumaModelOptionsCache.test.ts` passed after reconciling
  the frontend selector cursor handoff cache.
- `cargo check -p workflow-nodes --features model-library` passed after pinning
  `pumas-library` to the released upstream `v0.6.0` tag.
- `cargo test -p workflow-nodes --features model-library --lib puma_lib`
  passed against the released upstream `pumas-library v0.6.0` tag.
- `cargo test --manifest-path src-tauri/Cargo.toml puma_lib_commands` passed
  against the released upstream `pumas-library v0.6.0` tag, with existing
  dead-code warnings in unregistered legacy Tauri execution-runtime modules.
- `cargo check -p pantograph-uniffi` and `cargo check -p pantograph_rustler`
  passed against the released upstream `pumas-library v0.6.0` tag.
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
