# Milestone 7: Candle Future Capability Guardrail

**Goal:** Keep Candle diffusion visible as future-capability research without
allowing current workflows to select it for image generation.

**Tasks:**

- [x] Confirm Candle backend capability facts report image generation as
  unavailable.
- [x] Confirm Candle device capability facts distinguish compiled CPU, CUDA,
  Metal, MKL/Accelerate, and unavailable build-feature states without claiming
  executable image generation.
- [x] Add or update tests that reject Candle as an executable image-generation
  backend.
- [x] Document that upstream Candle has diffusion examples, but Pantograph
  requires executable Candle model loading before exposing it as a backend.
- [x] Add a future-work note for Candle diffusion support with clear acceptance
  requirements.
- [x] Ensure Candle guardrail diagnostics use the same structured readiness
  diagnostic path as PyTorch/diffusers planner failures.

**Verification:**

- Capability tests show Candle is unavailable for image-generation execution.
- Runtime readiness/admission tests produce a clear error if Candle is forced
  for image generation.

**Slice Record:**

- 2026-05-20 Candle guardrail slice:
  - Smallest useful vertical slice: strengthen Candle backend capability
    assertions, document the future-capability boundary, and verify explicit
    Candle image-generation requests fail through existing structured
    technical-fit diagnostics.
  - Allowed files touched: `crates/inference/src/backend/candle.rs`,
    `crates/inference/src/backend/README.md`, this milestone, and
    `docs/plans/current-image-generation-graphs/05-execution-management.md`.
  - No-fallback/no-legacy confirmation: Candle still advertises no executable
    `image_generation` task, all reported Candle runtime variants remain
    unavailable, and explicit Candle image-generation selection fails closed
    instead of falling back to PyTorch, generic Diffusers, CPU, or synthetic
    runtime candidates.
  - Verification passed:
    `cargo test -p inference --features backend-candle test_capabilities --lib`
    and
    `cargo test -p pantograph-embedded-runtime candle_image_generation_override_rejects_backend_incompatibility_without_selection --lib`.

**Status:** Complete
