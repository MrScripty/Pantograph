# Milestones

Use these milestone files as the implementation checklist. Each milestone owns
one validated vertical slice or one release/guardrail gate. Update the
milestone status in its file and summarize progress in
[Execution Management](05-execution-management.md).

## Milestone Index

0. [Contract Gate](milestones/00-contract-gate.md)
   - Freeze cross-layer contracts before implementation.
   - Define validated DTOs, boundary fixtures, execution normalization, and
     decomposition decisions.

1. [Current Juggernaut Graph Slice](milestones/01-current-juggernaut-graph-slice.md)
   - Keep one current Juggernaut workflow.
   - Use canonical `puma-lib -> llm-inference -> image-output`.

2. [Retired Node Producers Removed](milestones/02-retired-node-producers-removed.md)
   - Stop producers, probes, templates, and current app paths from emitting or
     rewriting retired `diffusion-inference` graphs.

3. [Backend Stale Graph Diagnostics](milestones/03-backend-stale-graph-diagnostics.md)
   - Add backend-owned stale graph diagnostics and bounded submit/admission
     reasons.

4. [IO Inspector Stale Graph Presentation](milestones/04-io-inspector-stale-graph-presentation.md)
   - Render stale saved graphs and selected stale node details in the IO
     inspector without frontend-inferred diagnostics.

5. [Device And Runtime Variant Selection](milestones/05-device-and-runtime-variant-selection.md)
   - Define canonical device policy, backend adapter candidate facts, runtime
     variant readiness, and deterministic scheduler-selected device resolution
     for CPU/CUDA on Linux/Windows and Metal/MPS on macOS.

P-a. [Pumas Library Contract Start](07-pumas-library-image-generation-facts.md)
   - Start Pumas P0-P1 immediately after Pantograph Milestone 0 freezes the
     expected facts contract.
   - Establish Pumas package-facts module ownership, selected-artifact identity,
     DTO/cache contract, cache freshness statuses, and shared fixture shape.

P-b. [Pumas Library Producer-Fact Completion](07-pumas-library-image-generation-facts.md)
   - Complete Pumas P2-P5 before Pantograph Milestone 6.
   - Provide richer diffusers component facts, image-family evidence, GGUF
     metadata, package-fact summaries, update cursors, and SQLite cache
     migration/backfill.
   - Publish or otherwise pin the Pumas version/commit Pantograph consumes for
     Milestone 6.

6. [PyTorch/Diffusers Image Generation Execution Slice](milestones/06-pytorch-diffusers-image-generation-execution-slice.md)
   - Implement deterministic PyTorch/diffusers image-generation planning,
     worker execution, and single-body artifact retention.

7. [Candle Future Capability Guardrail](milestones/07-candle-future-capability-guardrail.md)
   - Keep Candle unavailable for image-generation execution until executable
     Candle diffusion support exists.

8. [Release Build And User Validation](milestones/08-release-build-and-user-validation.md)
   - Run affected tests, build frontend/release binary, and record final
     standards/manual smoke checks.

## Execution Order

Milestone 0 is mandatory before parallel or broad implementation. After
Milestone 0, start Pumas P0-P1 so the external package-facts contract, selected
artifact semantics, cache freshness statuses, and fixture shape are frozen early
enough for Pantograph planner work to target the real producer contract.
Milestones 1 and 2 should be completed as one early graph-shape correction
slice because a fixed saved workflow is not complete while tracked producers
can still emit the retired graph shape. Milestones 3 and 4 may be implemented
after the stale diagnostic DTO is frozen. Milestone 5 must land before backend
execution slices that consume backend/runtime/device choices because it freezes
the adapter facts the scheduler will rank and the selected decision inference
will execute.

Pumas P2-P5 may proceed in parallel with Pantograph Milestones 1-5, but Pumas
producer-fact completion is a hard gate before Milestone 6. Milestone 6 must
wait for the execution planner contracts, backend normalization boundary,
scheduler-facing candidate facts, and device-resolution decision from Milestone
0 and Milestone 5. It must also consume the Pumas image-generation facts defined
in [Pumas Library Image Generation Facts](07-pumas-library-image-generation-facts.md)
from a pinned Pumas release or commit. If those facts, summaries, selected
artifact semantics, or cache migration/backfill are not available, stop with a
re-plan note instead of implementing name-derived or fallback behavior.
Milestone 8 is the final build and validation gate.

Recommended high-level order:

```text
Pantograph M0
Pumas P0-P1
Pantograph M1-M5 and Pumas P2-P5 in parallel
Pumas release/pin
Pantograph M6
Pantograph M7-M8
```
