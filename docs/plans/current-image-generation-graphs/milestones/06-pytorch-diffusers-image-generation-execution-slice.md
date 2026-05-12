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

**Status:** Dependency boundary resolved; planner implementation not started.

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
