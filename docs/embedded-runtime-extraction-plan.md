# Plan: Embedded Runtime Extraction for Direct C# Binding

## Objective

Create a backend-owned embedded Pantograph runtime that can be linked directly
from C# through UniFFI without routing workflow/session operations through an
HTTP `base_url`, while keeping Tauri as an optional GUI consumer rather than a
runtime dependency.

## Scope

### In Scope

- Introduce a new backend-owned embedded runtime crate that acts as the
  composition root for direct Pantograph embedding.
- Extract direct workflow host/runtime wiring out of `src-tauri` into reusable
  backend modules.
- Keep `pantograph-workflow-service` as the canonical service contract and
  session orchestration layer.
- Make UniFFI expose a direct runtime object for workflow/session APIs.
- Convert Tauri to consume the new backend runtime instead of owning the direct
  execution path itself.
- Add verification and documentation so the resulting boundary stays aligned
  with Coding Standards, Architecture Patterns, Interop Standards, and Language
  Bindings Standards.

### Out of Scope

- Rewriting workflow/session DTO semantics in `pantograph-workflow-service`.
- Removing the optional frontend HTTP adapter used for modular GUI composition.
- Making foreign clients depend on Tauri.
- Re-introducing in-process Python embedding.
- GUI/editor feature work unrelated to direct runtime extraction.
- Broad graph API redesign outside what is required to preserve direct
  embedding boundaries.

## Inputs

### Problem

Pantograph already has a host-agnostic workflow service layer, but the only
direct runtime wiring for workflow/session execution currently lives in
`src-tauri`. The UniFFI surface therefore exposes workflow/session methods
through the optional `frontend-http` adapter instead of a native Rust runtime
facade. That violates the intended layering: Tauri is an optional GUI shell,
not part of Pantograph's embeddable backend.

### Constraints

- Tauri must remain an optional consumer only.
- `pantograph-workflow-service` stays the canonical contract and scheduler
  owner.
- Direct embedding must support image-generation workflows, including the
  Python-backed node path used by diffusion and related nodes.
- HTTP adapter support may remain available, but it must not be the primary
  native embedding story.
- Binding-generated code must remain generated-only and never hand-edited.
- Public contracts must remain explicit about thread/lifecycle ownership at
  process and FFI boundaries.

### Assumptions

- Existing workflow/session request and response DTOs in
  `pantograph-workflow-service` are the source of truth for direct embedding.
- `workflow_nodes::setup_extensions_with_path(...)` remains the standard way to
  bootstrap optional runtime extensions.
- `inference` can act as the runtime/process-management backend for embedded
  hosts without Tauri-specific process APIs when configured with an appropriate
  spawner.
- Python-backed execution remains out-of-process and host-managed per
  `docs/python-runtime-separation.md`.
- Current workflow JSON files and session semantics remain valid and do not
  need migration.

### Dependencies

- `crates/pantograph-workflow-service`
- `crates/node-engine`
- `crates/workflow-nodes`
- `crates/inference`
- `crates/pantograph-uniffi`
- `crates/pantograph-frontend-http-adapter`
- `src-tauri` as an extraction source and downstream consumer only
- `pumas-library`
- Python worker/runtime bridge assets currently referenced from
  `src-tauri/src/workflow/python_runtime.rs`
- Existing docs:
  - `docs/adr/ADR-001-headless-embedding-service-boundary.md`
  - `docs/headless-embedding-migration.md`
  - `docs/headless-embedding-implementation-notes.md`
  - `docs/python-runtime-separation.md`

### Affected Structured Contracts

- `WorkflowRunRequest` / `WorkflowRunResponse`
- `WorkflowCapabilitiesRequest` / `WorkflowCapabilitiesResponse`
- `WorkflowPreflightRequest` / `WorkflowPreflightResponse`
- `WorkflowSessionCreateRequest` / `WorkflowSessionCreateResponse`
- `WorkflowSessionRunRequest`
- `WorkflowSessionCloseRequest` / `WorkflowSessionCloseResponse`
- `WorkflowSessionStatusRequest` / `WorkflowSessionStatusResponse`
- `WorkflowSessionQueueListRequest` / `WorkflowSessionQueueListResponse`
- `WorkflowSessionQueueCancelRequest` / `WorkflowSessionQueueCancelResponse`
- `WorkflowSessionQueueReprioritizeRequest` /
  `WorkflowSessionQueueReprioritizeResponse`
- `WorkflowSessionKeepAliveRequest` / `WorkflowSessionKeepAliveResponse`
- UniFFI-generated C# binding surface and any generated binding metadata

### Affected Persisted Artifacts

- Workflow JSON files loaded from workflow roots
- Generated C# binding outputs under a future `bindings/csharp/` path
- Documentation artifacts under `docs/`
- CI/build scripts that generate or validate bindings

### Concurrency and Race-Risk Review

- The embedded runtime must be the single lifecycle owner for startup and
  shutdown of long-lived resources: inference gateway, runtime extensions,
  Python process adapter, session-facing helpers, and any event buffers.
- `pantograph-workflow-service` remains the owner of session queueing,
  cancellation, reprioritization, and keep-alive semantics; adapters must not
  re-implement session state machines.
- Runtime initialization and shutdown must be symmetric and guarded against
  double-start/double-stop races.
- Host-visible polling, such as `drain_events()` or explicit session status
  reads, must remain at the host/core boundary and not turn into hidden UI
  loops or unmanaged background polling.
- Python-backed node execution must preserve argument-safe process APIs and
  explicit cleanup ownership; no shell-string process launching.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Tauri-specific types are more deeply entangled with direct execution than current inspection suggests | High | Freeze crate boundaries first; extract smallest reusable seams before moving public APIs |
| Direct image-generation path depends on GUI-only state or side effects | High | Treat Python runtime adapter and host task executor as first-class extraction targets with explicit acceptance tests for diffusion workflows |
| UniFFI direct facade drifts from service-layer DTO semantics | High | Preserve service contracts as source of truth; add conversion tests and one full-path C# acceptance check |
| Lifecycle leaks leave spawned runtimes or Python workers running after consumer shutdown | High | Make embedded runtime the single composition root and require symmetric init/shutdown tests |
| Existing docs imply the architecture is already fully implemented | Med | Update ADR and migration docs to distinguish completed service extraction from incomplete embedded runtime extraction |
| Generated binding packaging varies by platform | Med | Add cross-platform build matrix and artifact naming checks before treating the direct binding as complete |

## Definition of Done

- Pantograph has a backend-owned embedded runtime crate that can execute
  workflow/session APIs directly without Tauri and without the HTTP adapter.
- Tauri uses that runtime as a consumer instead of owning the direct execution
  path.
- UniFFI exposes a direct runtime object for C# embedding with no `base_url`
  requirement for workflow/session methods.
- Existing service contracts remain the source of truth for runtime behavior.
- Image-generation execution paths work through the extracted host/runtime
  stack, including Python-backed node execution.
- Generated C# bindings are produced from the direct runtime facade and covered
  by at least one end-to-end acceptance path.
- Architecture, migration, and binding docs clearly state that Tauri is not a
  client dependency and HTTP is optional transport, not the native embedding
  model.

## Lifecycle Ownership Note

- `pantograph-embedded-runtime` (working name) is the sole owner of runtime
  startup and shutdown for direct embedding.
- Tauri and UniFFI instantiate and dispose that runtime; they do not create
  gateway/process/runtime dependencies ad hoc.
- `pantograph-workflow-service` owns session scheduling and run admission.
- Python worker processes remain host-managed infrastructure under the embedded
  runtime, not under adapters.

## Public Facade Preservation Note

- Preserve `pantograph-workflow-service` request/response semantics.
- Preserve the optional `frontend-http` adapter for frontend/modular GUI
  composition.
- Add a new direct runtime facade instead of repurposing Tauri or requiring
  native clients to speak HTTP.
- Prefer additive migration for UniFFI: introduce the direct runtime object
  first, then de-emphasize or deprecate HTTP-shaped binding entrypoints in docs
  once parity is verified.

## Milestones

### Milestone 1: Freeze Runtime Boundary and Compatibility Contract

**Goal:** Establish the exact backend boundary so extraction work does not
re-couple runtime ownership to adapters.

**Tasks:**
- [ ] Define the canonical new crate boundary for the embedded runtime
      composition root and host implementation.
- [ ] Record the runtime ownership model: who creates gateway/process/runtime
      resources, who shuts them down, and where session scheduling stops.
- [ ] Document compatibility posture:
      - service DTOs preserved
      - HTTP adapter retained as optional transport
      - Tauri not part of client-facing embedding
- [ ] Add a short architecture decision note or ADR update that supersedes the
      remaining ambiguity in `ADR-001` implementation status.
- [ ] Decide whether direct UniFFI methods will expose typed `Ffi*` records,
      JSON request/response strings, or a staged hybrid approach.

**Verification:**
- Architecture review against:
  - `CODING-STANDARDS.md` service-independence and composition-root sections
  - `ARCHITECTURE-PATTERNS.md` transport-adapter and composition-root rules
  - `LANGUAGE-BINDINGS-STANDARDS.md` three-layer architecture rules
- No code movement yet; decision traceability captured in docs

**Status:** Completed in commits `bcdf714`, `aaa847b`, and `23ddd93`.

### Milestone 2: Extract Reusable Host Execution Modules Out of Tauri

**Goal:** Move direct execution dependencies into backend-owned modules that do
not depend on Tauri.

**Tasks:**
- [ ] Extract the reusable host implementation logic currently in
      `src-tauri/src/workflow/headless_workflow_commands.rs` into backend-owned
      modules.
- [ ] Extract the host task executor currently in
      `src-tauri/src/workflow/task_executor.rs` so Python-backed and
      host-dependent nodes are executable without Tauri.
- [ ] Extract the Python runtime adapter boundary currently in
      `src-tauri/src/workflow/python_runtime.rs` into a backend-owned module or
      crate.
- [ ] Move runtime extension snapshot/bootstrap helpers into backend-owned code.
- [ ] Ensure extracted modules use explicit configuration and constructor
      injection rather than Tauri app state.

**Verification:**
- `cargo check -p node-engine`
- `cargo check -p workflow-nodes`
- `cargo check -p pantograph-workflow-service`
- Targeted unit tests for extracted helper modules
- No dependency from extracted backend modules back into `src-tauri`

**Status:** Completed in commit `aaa847b`.

### Milestone 3: Introduce the Embedded Runtime Composition Root

**Goal:** Create a backend-owned runtime crate that is the canonical direct
embedding entrypoint.

**Tasks:**
- [ ] Add `crates/pantograph-embedded-runtime` (working name) to the workspace.
- [ ] Implement a concrete `WorkflowHost` for embedded execution.
- [ ] Add a public runtime/facade object that owns:
      - workflow roots
      - data/app paths
      - inference gateway
      - runtime extensions
      - optional `PumasApi`
      - KV cache store
      - Python runtime adapter
      - optional event buffering
- [ ] Make startup and shutdown explicit and symmetric.
- [ ] Use `inference` process-spawner abstractions appropriate for non-Tauri
      embedding.
- [ ] Keep session orchestration delegated to `WorkflowService`.

**Verification:**
- `cargo check -p pantograph-embedded-runtime`
- `cargo test -p pantograph-embedded-runtime`
- Direct Rust-host acceptance check:
  - create runtime
  - create session
  - run workflow session
  - close session
- Shutdown test confirming resources are disposed cleanly

**Status:** Completed in commit `aaa847b`.

### Milestone 4: Rebase Tauri Onto the Embedded Runtime

**Goal:** Make Tauri a consumer of Pantograph runtime instead of the owner of
direct execution logic.

**Tasks:**
- [ ] Replace Tauri-owned direct workflow runtime wiring with calls into
      `pantograph-embedded-runtime`.
- [ ] Keep Tauri command signatures stable unless a documented internal cleanup
      makes a change unavoidable.
- [ ] Remove duplicated lifecycle wiring from `src-tauri` once equivalent
      backend-owned functionality exists.
- [ ] Ensure Tauri still provides only GUI-specific transport/state concerns.

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Existing Tauri workflow compile path still passes
- At least one Tauri integration path verifies it delegates through the new
  runtime instead of local duplicate host logic

**Status:** Completed in commit `aaa847b`.

### Milestone 5: Add Direct UniFFI Runtime Facade for C#

**Goal:** Make the C# binding use the embedded runtime directly instead of the
HTTP adapter.

**Tasks:**
- [ ] Add a `#[uniffi::Object]` runtime wrapper over
      `pantograph-embedded-runtime`.
- [ ] Expose direct workflow/session methods with no `base_url` argument.
- [ ] Convert errors at the FFI boundary into dedicated UniFFI-safe error
      variants.
- [ ] Add `Ffi*` record wrappers where needed so generated C# APIs do not rely
      on non-FFI-safe Rust types.
- [ ] Keep generated bindings generated-only and document regeneration steps.
- [ ] Decide and implement compatibility posture for existing
      `frontend_http_workflow_*` exports:
      keep, document as frontend-only, and avoid presenting them as the
      preferred native embedding path.

**Verification:**
- `cargo check -p pantograph-uniffi --no-default-features`
- `cargo check -p pantograph-uniffi --features frontend-http`
- `cargo test -p pantograph-uniffi`
- Conversion tests for every new `Ffi*` wrapper and FFI error mapping
- Generated C# binding compile check against the new direct runtime object

**Status:** Completed for the staged JSON DTO facade in commit `23ddd93`;
generated-C# compile smoke was added in the binding-smoke follow-up.

Rust/UniFFI status:
- `FfiPantographRuntime` now wraps `pantograph-embedded-runtime`.
- Workflow/session methods are direct object methods and do not accept
  `base_url`.
- The staged binding surface keeps workflow/session DTOs as JSON strings.
- Existing `frontend_http_workflow_*` exports remain feature-gated
  compatibility APIs.

Binding closure:
- C# bindings are generated by `uniffi-bindgen-cs` rather than the checked-in
  official UniFFI helper.
- `scripts/check-uniffi-csharp-smoke.sh` generates C# under
  `target/uniffi/csharp/` and compiles it against the direct-runtime smoke
  harness in `bindings/csharp/`.
- The same smoke script also runs the generated C# harness against the compiled
  native library and executes a model-free workflow/session round trip.

### Milestone 6: End-to-End Binding, Image Workflow, and Documentation Closure

**Goal:** Prove that direct C# embedding works for the real image-generation
path and lock in the architecture.

**Tasks:**
- [ ] Add a C# smoke/integration harness that loads the generated binding,
      starts the runtime, and executes at least one direct workflow/session path.
- [ ] Add one acceptance path that exercises image-generation execution through
      the direct embedded runtime, including the Python-backed node path.
- [ ] Update:
      - `docs/adr/ADR-001-headless-embedding-service-boundary.md`
      - `docs/headless-embedding-migration.md`
      - `docs/headless-embedding-implementation-notes.md`
      - `docs/python-runtime-separation.md`
      - adapter/runtime READMEs as needed
- [ ] Add CI/build automation for:
      - embedded runtime compile/test
      - UniFFI generation
      - C# compile/smoke verification
- [ ] Add traceability notes that state Tauri is not a client-facing runtime
      dependency.

**Verification:**
- `cargo test -p pantograph-workflow-service --test contract`
- `cargo test -p pantograph-embedded-runtime`
- `cargo test -p pantograph-uniffi`
- Generated bindings regenerate cleanly
- C# smoke or integration test passes
- One full-path image-generation acceptance check passes

**Status:** In progress; direct UniFFI metadata, generated-C# compile, and
model-free C# runtime execution are CI-guarded. An opt-in generated-C#
diffusion acceptance script exists for environments with a local model bundle.
Remaining work is generated-C# packaging policy and recording the real-image
acceptance run for the target release/platform/model.

## Execution Notes

Update during implementation:
- 2026-04-08: Plan created to extract Pantograph's direct runtime out of Tauri
  and make direct UniFFI/C# embedding the canonical native binding path.
- 2026-04-08: Added `pantograph-embedded-runtime`, moved direct host/task
  executor/Python runtime/model dependency modules out of `src-tauri`, and
  rebased Tauri workflow commands onto the backend runtime facade.
- 2026-04-08: Added default UniFFI `FfiPantographRuntime` object over the
  embedded runtime. The native workflow/session binding path no longer uses
  `base_url`; frontend HTTP functions remain optional compatibility exports.
- 2026-04-08: Added `scripts/check-uniffi-embedded-runtime-surface.sh` and a CI
  workflow step that validates `FfiPantographRuntime` plus direct
  workflow/session methods are present in UniFFI cdylib metadata.
- 2026-04-08: Verified the repo's current `pantograph-uniffi-bindgen` binary
  does not support `--language csharp`. C# smoke coverage requires adding an
  explicit C# generator/tooling path instead of using the existing helper.
- 2026-04-08: Added an embedded-runtime mocked diffusion acceptance test. It
  proves a workflow run can demand an `image-output`, traverse
  `diffusion-inference`, and route through the injected `PythonRuntimeAdapter`.
  Real-model image generation remains separate hardware/model acceptance.
- 2026-04-08: Added `scripts/check-uniffi-csharp-smoke.sh` and
  `bindings/csharp/`. The script builds `pantograph-uniffi`, generates C# with
  `uniffi-bindgen-cs` into `target/uniffi/csharp/`, compiles an offline
  C# harness against the direct `FfiPantographRuntime` API, and runs a
  model-free C# workflow/session smoke through the native library.
- 2026-04-08: Added opt-in
  `scripts/check-uniffi-csharp-diffusion-smoke.sh`. It reuses the generated-C#
  smoke harness in diffusion mode and runs prompt-to-image through
  C# -> UniFFI -> `pantograph-embedded-runtime` -> process Python adapter ->
  torch/diffusers worker when `PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH` and a
  suitable Python executable are supplied.

## Commit Cadence Notes

- Commit after each milestone slice is complete and verified.
- Keep extraction commits reviewable and dependency-ordered.
- Prefer the following atomic sequence:
  1. `docs(architecture): freeze embedded runtime boundary`
  2. `refactor(runtime): extract host execution from tauri`
  3. `feat(runtime): add embedded runtime crate`
  4. `refactor(tauri): consume embedded runtime`
  5. `feat(uniffi): add direct runtime facade`
  6. `test(bindings): add csharp embedded runtime smoke coverage`
  7. `docs(bindings): document direct embedding path`

## Re-Plan Triggers

- Extracted runtime code still requires Tauri-only types after Milestone 2.
- Image-generation execution depends on GUI-only state that cannot be expressed
  as injected infrastructure.
- UniFFI typed wrapper scope becomes too large for one release and requires a
  staged JSON-wrapper first release.
- Direct runtime shutdown semantics prove insufficient to prevent orphaned
  runtime or Python worker processes.
- Cross-platform binding generation or packaging constraints materially change
  milestone ordering.

## Recommendations

- Recommendation: Use a dedicated `pantograph-embedded-runtime` crate rather
  than expanding `pantograph-uniffi` into a mixed runtime-plus-FFI crate. This
  keeps the three-layer binding architecture intact and preserves testability.
- Recommendation: Treat the current `frontend_http_workflow_*` UniFFI exports
  as retained compatibility/convenience surfaces only; do not remove them until
  the direct runtime facade reaches parity and docs have been updated.
- Recommendation: Add a small `bindings/csharp/` smoke project early once the
  UniFFI object shape is frozen. It will catch naming, async, and packaging
  issues earlier than Rust-only verification.

## Completion Summary

### Completed

- Runtime boundary/ADR freeze.
- Backend-owned `crates/pantograph-embedded-runtime` crate.
- Reusable Python runtime adapter and Python bridge asset moved out of
  `src-tauri`.
- Reusable host task executor and RAG abstraction moved out of `src-tauri`.
- Reusable model dependency resolver module moved out of `src-tauri`.
- Tauri workflow/session commands rebased onto `EmbeddedRuntime`.
- Direct `FfiPantographRuntime` UniFFI object with workflow/session JSON
  service-contract methods.
- UniFFI docs updated to present native embedded runtime as the default
  workflow/session binding path.

### Deviations

- The first UniFFI direct facade intentionally uses JSON request/response
  strings to preserve service DTOs and keep the C#-binding migration additive.
  Typed UniFFI request/response records remain a follow-up.
- Generated C# bindings are not checked into this repo today; generated-binding
  compile smoke is now automated by `scripts/check-uniffi-csharp-smoke.sh`.

### Follow-Ups

- Run `scripts/check-uniffi-csharp-diffusion-smoke.sh` in a model-equipped
  environment and attach the generated PNG/log to release validation. Current
  always-on coverage remains model-free.
- Decide whether generated C# is packaged by Pantograph or generated by
  downstream applications during their build.
- Consider typed UniFFI `Record` wrappers once the JSON service-contract facade
  has shipped and host-language call sites are stable.

### Verification Summary

- `cargo check -p pantograph-embedded-runtime`
- `cargo test -p pantograph-embedded-runtime`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check -p pantograph-uniffi --no-default-features`
- `cargo check -p pantograph-uniffi`
- `cargo test -p pantograph-uniffi`
- `cargo check -p pantograph-uniffi --features frontend-http`
- `cargo test -p pantograph-uniffi --features frontend-http`
- `./scripts/check-uniffi-embedded-runtime-surface.sh`
- `./scripts/check-uniffi-csharp-smoke.sh`
- `PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH=/path/to/model PANTOGRAPH_PYTHON_EXECUTABLE=.venv/bin/python ./scripts/check-uniffi-csharp-diffusion-smoke.sh`

### Traceability Links

- Module README updated: `crates/pantograph-uniffi/src/README.md`
- ADR updated: `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- Runtime plan: `docs/embedded-runtime-extraction-plan.md`
- Metadata smoke: `scripts/check-uniffi-embedded-runtime-surface.sh`
- Generated C# smoke: `scripts/check-uniffi-csharp-smoke.sh`
- Opt-in generated C# diffusion smoke:
  `scripts/check-uniffi-csharp-diffusion-smoke.sh`
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: pending

## Brevity Note

This plan is intentionally concise by default and expands detail only where
boundary ownership, lifecycle control, or verification risk affects execution.
