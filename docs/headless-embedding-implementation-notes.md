# Headless Workflow Implementation Notes

## Scope Completed

The headless workflow plan is implemented with generic workflow I/O:

- Contract freeze and service boundary ADR.
- Host-agnostic workflow service (`pantograph-workflow-service`).
- Tauri, UniFFI, and Rustler adapters delegating to shared service contracts.
- Dedicated frontend HTTP adapter crate (`pantograph-frontend-http-adapter`).
- Feature-gated frontend HTTP binding exports (`frontend-http`) so default
  bindings do not imply HTTP/Tauri-based headless integration.
- Backend-owned embedded runtime crate (`pantograph-embedded-runtime`) that owns
  the direct workflow host, host task executor, Python runtime adapter,
  Python worker bridge asset, and model dependency resolver plumbing.
- Tauri workflow/session commands delegating through `EmbeddedRuntime` instead
  of owning the direct host implementation.
- Default UniFFI `FfiPantographRuntime` object for direct workflow/session
  methods without `base_url`.
- C# UniFFI generation, offline compile, and runtime smoke for the direct
  `FfiPantographRuntime` surface.
- Opt-in generated-C# diffusion smoke script that runs through UniFFI,
  `pantograph-embedded-runtime`, the process Python adapter, and the real
  torch/diffusers worker when supplied with a local model bundle.
- Contract tests plus CI contract gate.
- Rust host example and migration guide.

## Remaining Binding/Acceptance Closure

The backend-owned runtime, Rust/UniFFI direct facade, generated-C# compile
surface, and model-free generated-C# runtime smoke are implemented. Remaining
acceptance work:

- Run and record the opt-in full-path image-generation smoke for the target
  release/platform/model bundle.
- Optionally expand the C# runtime smoke from a model-free text workflow to a
  Pixapillars-style fixture once stable sample assets/models are available.
- Decide whether generated C# should be packaged as an artifact or distributed
  by downstream applications during their build.

Tracked plan:

- `docs/embedded-runtime-extraction-plan.md`

## Key Design Outcomes

1. Service logic is centralized in `pantograph-workflow-service`.
2. Adapter layers are transport/runtime wrappers, not business-rule owners.
3. `workflow_run` now operates on generic `inputs[]` and `outputs[]` bindings.
4. Workflow execution semantics are output-node/target based, not embedding-type based.
5. Workflow validation enforces existence + logical graph validity.
6. Headless API usage is explicit: core service first, optional frontend HTTP adapter only for modular GUI composition.

## Verification Commands

- `cargo test -p pantograph-workflow-service --test contract`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check -p pantograph-frontend-http-adapter`
- `cargo test -p pantograph-frontend-http-adapter`
- `cargo check -p pantograph-uniffi --no-default-features`
- `cargo check -p pantograph-uniffi --features frontend-http`
- `cargo test -p pantograph-uniffi`
- `cargo test -p pantograph-uniffi --features frontend-http`
- `./scripts/check-uniffi-embedded-runtime-surface.sh`
- `./scripts/check-uniffi-csharp-smoke.sh`
- `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH=/path/to/model PANTOGRAPH_PYTHON_EXECUTABLE=.venv/bin/python ./scripts/check-uniffi-csharp-diffusion-smoke.sh`
- `cargo check -p pantograph_rustler --no-default-features`
- `cargo check -p pantograph_rustler --features frontend-http`

## Test Environment Notes

- Some adapter runtime tests require local TCP bind and/or NIF runtime link targets.
- In restricted sandbox environments, those tests may be ignored or compile-only validated.
