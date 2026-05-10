# Milestone 7: Candle Future Capability Guardrail

**Goal:** Keep Candle diffusion visible as future-capability research without
allowing current workflows to select it for image generation.

**Tasks:**

- [ ] Confirm Candle backend capability facts report image generation as
  unavailable.
- [ ] Confirm Candle device capability facts distinguish compiled CPU, CUDA,
  Metal, MKL/Accelerate, and unavailable build-feature states without claiming
  executable image generation.
- [ ] Add or update tests that reject Candle as an executable image-generation
  backend.
- [ ] Document that upstream Candle has diffusion examples, but Pantograph
  requires executable Candle model loading before exposing it as a backend.
- [ ] Add a future-work note for Candle diffusion support with clear acceptance
  requirements.
- [ ] Ensure Candle guardrail diagnostics use the same structured readiness
  diagnostic path as PyTorch/diffusers planner failures.

**Verification:**

- Capability tests show Candle is unavailable for image-generation execution.
- Runtime readiness/admission tests produce a clear error if Candle is forced
  for image generation.

**Status:** Not started
