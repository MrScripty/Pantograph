# Milestone 6: PyTorch/Diffusers Image Generation Execution Slice

**Goal:** Prove canonical image-generation inference can execute through
PyTorch/diffusers and produce a retained image artifact.

**Tasks:**

- [ ] Add an image-generation execution planner that consumes
  `ImageGenerationRequest`, Pumas package facts, graph/runtime hints,
  dependency readiness, backend capabilities, and device policy, then returns
  one explicit execution plan or a bounded diagnostic.
- [ ] Consume the device resolution decision from Milestone 5 instead of
  parsing raw device strings or auto-selecting devices inside image-generation
  planning.
- [ ] Implement the planner in a focused inference module such as
  `backend/pytorch/image_generation` or `image_generation_planner` and keep
  `pytorch.rs` as a facade over load/generate calls.
- [ ] Keep planner parsing/validation synchronous and side-effect free. Async
  shells may gather Pumas facts, dependency readiness, and device facts before
  calling the planner.
- [ ] Add a planned image-generation gateway boundary before end-to-end gateway
  wiring. The gateway must carry reduced package facts and the scheduler-owned
  `BackendExecutionDecision` into `ImageGenerationExecutionPlan`; it must not
  dispatch image generation from `ImageGenerationRequest` alone or infer
  backend/runtime/device/package decisions from request fields.
- [ ] Add a first-class run-scoped workflow execution plan before successful
  end-to-end image execution. The plan is produced by scheduler/admission,
  contains per-node reduced execution decisions, and is consumed by node
  execution without writing scheduler facts into graph inputs.
- [ ] Project the workflow execution plan's image-generation node decision into
  inference's `BackendExecutionDecision` at the composition boundary before
  calling `generate_image_from_planning_input`.
- [ ] Verify backend runtime selection maps diffusion/image-generation package
  facts and graph hints to PyTorch execution, preserving `diffusers` as the
  dependency and package capability label.
- [ ] Replace scattered `diffusers` backend-key mappings that affect execution
  with calls into the single normalization boundary defined in Milestone 0.
- [ ] Audit existing dependency preflight, technical-fit, runtime capability,
  gateway, and workflow runtime preflight mappings so execution selection,
  runtime display, and dependency diagnostics do not each maintain conflicting
  `diffusers` rules.
- [ ] Add or update centralized normalization tests for graph
  `backend_key = pytorch` and Pumas Diffusers package hints. `diffusers`
  remains package/runtime capability evidence, not a graph-visible backend
  preference.
- [ ] Ensure `ImageGenerationRequest` is populated from canonical
  `llm-inference` inputs: prompt, negative prompt, width, height, steps,
  guidance scale, seed, scheduler, and image count.
- [ ] Implement `PyTorchBackend::generate_image` using the existing Python
  worker diffusion load/generate path and typed worker request/response
  envelopes.
- [ ] Change the Python worker image path to consume the validated Rust plan
  fields. It must not decide pipeline family, scheduler, custom-code trust, or
  device fallback on its own.
- [ ] Version or explicitly shape-check the Python worker image-generation
  envelope so Rust rejects unknown, missing, or incompatible worker request and
  response fields before trusting them.
- [ ] Ensure worker calls use the existing process lifecycle owner and do not
  introduce untracked `tokio::spawn`, unbounded queues, or blocking work while
  holding service locks.
- [ ] Put PyTorch image-generation worker request/response translation in a
  focused helper or submodule so the backend facade remains readable and
  testable.
- [ ] Add model-family adapters inside the PyTorch/diffusers bridge. Adapter
  selection must use package facts such as pipeline family/class and component
  layout, not model id or display-name string matching.
- [ ] Implement family requirements as explicit table data or small typed
  requirement structs, not as scattered `match` arms mixed with worker calls.
- [ ] Implement component-role extraction from Pumas facts before adapter
  selection. Extraction maps `ProcessorComponentFacts` and Transformers
  evidence into `ImageGenerationComponentRole` values and diagnostics.
- [ ] Implement adapter selection in two stages: first resolve family/variant
  from Pumas facts, then validate that the resolved family requirements match
  the request and package components.
- [ ] Add a missing-facts report that lists the exact absent or ambiguous facts
  needed from Pumas for a family to be supported.
- [ ] Use Transformers task/config/generation names as the external naming
  reference where they fit Pantograph's Rust contracts.
- [ ] Use ComfyUI and InvokeAI as reference implementations for diffusion
  family taxonomy and required-component validation, including SD/SDXL, FLUX,
  FLUX.2, qwen-image, lumina-image, glm-image, and z-image patterns.
- [ ] Keep reference-derived logic behind Pantograph-owned Rust planner types
  and Pumas package facts; do not mirror ComfyUI/InvokeAI graph/runtime
  architecture.
- [ ] Validate scheduler, dimensions, negative prompt, guidance scale, image
  count, dtype, device policy, dependency environment, and required package
  components before calling the worker.
- [ ] Validate option support per family. For example, guidance scale,
  negative prompt, image count, scheduler override, dtype, and dimensions must
  be accepted, ignored, or rejected by typed family rules before execution.
- [ ] Ensure gateway-level image option diagnostics are reconciled with planner
  diagnostics so users see one authoritative option-support answer.
- [ ] Validate component source ambiguity per family. Families such as Z-Image
  and FLUX.2 must reject ambiguous VAE/text-encoder sources instead of trying
  to assemble components heuristically.
- [ ] Use checked arithmetic for dimensions, image counts, estimated memory,
  and artifact size calculations. Reject overflow or unacceptable estimates
  through typed planner diagnostics.
- [ ] Validate Pumas-provided paths and artifact entry paths against the
  approved Pumas/model roots before worker execution.
- [ ] Return a terminal planning/readiness diagnostic when validation fails.
  Do not try alternate backends, generic Diffusers loading, default schedulers,
  CPU fallback, or alternate dependency environments.
- [ ] Update PyTorch capability facts so image generation is advertised only
  when the PyTorch/diffusers execution path is actually available.
- [ ] Ensure PyTorch worker loading uses Pumas-resolved diffusers-directory
  package facts.
- [ ] Ensure dependency/runtime readiness reports missing `diffusers`,
  `transformers`, `accelerate`, `torch`, or Pillow as explicit readiness
  diagnostics.
- [ ] Retain final generated image output through ArtifactStore and IO
  projections.
- [ ] Ensure `image` and `results` outputs do not persist duplicate full image
  base64 bodies after artifact conversion.
- [ ] Change node-engine image-generation output shaping so large generated
  bodies are not duplicated in `image` and `results` before artifact
  conversion. `results` should contain descriptors/metadata or compact
  summaries once artifacts exist.
- [ ] Add a small model smoke path using Tiny SD Turbo or another bounded
  fixture before attempting Juggernaut.
- [ ] Add model-family fixtures or table-driven tests for Pumas facts shaped
  like z-image turbo, qwen-image, lumina-image, glm-image, and FLUX.2 where
  available. These tests should validate routing/request construction and may
  use mocked generation rather than loading large models.

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
- Planner tests prove unsupported pipeline family, missing component facts,
  incompatible scheduler, invalid dimensions, unsupported options, unavailable
  dependency environment, unavailable device, and unacceptable resource
  estimates fail with diagnostics and no fallback attempt.
- Planner tests cover component-role extraction from Pumas facts and reject
  missing or ambiguous family evidence.
- Planner tests cover generation-default merge order: request value,
  model-provided default, then family default only when explicitly allowed.
- Planner tests cover missing-facts diagnostics with exact field paths and
  expected evidence for insufficient Pumas facts.
- Family requirement tests cover accepted/rejected options for SD/SDXL, FLUX,
  FLUX.2, Qwen Image, Lumina Image, GLM Image, and Z-Image.
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

2026-05-14 re-plan boundary before workflow async-shell wiring:

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
- Standards constraint: this must be a narrow async-shell integration owned at
  the workflow/embedded-runtime composition boundary. The inference planner and
  gateway stay side-effect free below that boundary; node-engine must not
  invent backend/runtime/device decisions from request fields, active backend
  state, or graph hints.

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

Staged Option 3 implementation plan:

1. Contract foundation slice:
   - Add a small workflow execution-plan DTO at the workflow/embedded-runtime
     boundary. The initial DTO should contain run id/workflow id, a schema
     version, and a map keyed by stable node id to reduced execution decisions.
   - The first per-node decision shape must include selected backend key,
     selected runtime id/variant id, selected device class/id, selected task id,
     selected model ref when available, and bounded diagnostics/trace ids.
   - Do not include full Pumas facts, worker envelopes, raw graph node payloads,
     local paths beyond existing approved model/package refs, or mutable
     scheduler internals.
   - Verification: contract serde tests, append-only/default behavior tests,
     and a no-graph-input test proving scheduler decisions are not written into
     workflow node inputs.

2. Admission production slice:
   - Build the initial execution plan immediately after runtime preflight and
     scheduler admission, using the existing `WorkflowTechnicalFitDecision`
     already computed before run start.
   - Store it as run-scoped execution context, not as saved workflow content.
     If persistence is needed for diagnostics or recovery, persist only the
     execution-plan record with explicit schema/version and source ids.
   - Verification: session admission tests prove the admitted run has an
     execution plan when technical-fit selected a candidate, and no plan is
     produced when technical-fit fails.

3. Projection adapter slice:
   - Add a focused adapter that projects a workflow execution-plan node
     decision into inference's `BackendExecutionDecision`.
   - Keep this adapter at the composition boundary. Inference planner remains
     side-effect free, and node-engine does not know scheduler ranking policy.
   - Verification: adapter tests cover selected backend/runtime/device
     projection, missing fields, unknown device/runtime identifiers, and
     diagnostic propagation.

4. Node-engine consumption slice:
   - Thread the execution plan into node execution through a typed runtime
     context, likely `ExecutorExtensions`, without serializing it into graph
     inputs.
   - `execute_image_generation_inference` reads the current node's per-node
     decision, combines it with the existing `ImageGenerationRequest` and
     Pumas `ResolvedModelPackageFacts`, and calls
     `generate_image_from_planning_input`.
   - Missing plan, missing node decision, missing package facts, or failed
     projection must terminate the workflow task with typed diagnostics.
   - Verification: node-engine tests prove successful planned image execution
     and fail-closed behavior for absent/invalid execution-plan decisions.

5. Lifecycle/diagnostics slice:
   - Attach execution-plan identifiers and selected per-node decision facts to
     existing scheduler, runtime-load, inference lifecycle, and diagnostics
     ledger records without duplicating large payloads.
   - Verification: diagnostics tests prove selected backend/runtime/device
     facts come from the execution plan and that planner failures preserve
     diagnostic codes.

6. Recovery and future expansion slice:
   - Define how execution plans participate in retry/recovery. A retry may
     reuse a still-valid plan or request a new scheduler plan, but the policy
     must be explicit and diagnostic-backed.
   - Keep later additions append-only: multi-node placement, memory
     reservations, exploration cohorts, warmed-runtime affinity, historical
     performance summaries, and artifact-retention decisions can extend the
     plan without changing node graph ergonomics.
