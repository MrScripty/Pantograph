# Milestone 9: Workflow Editor End-To-End Image Generation

**Goal:** Prove image generation is usable from the workflow editor by running
a canonical image-generation workflow through the real app path and producing a
retained image artifact that the UI can inspect or display.

Milestone 8 closed with a headless, contract-based release smoke. That
validation is useful, but it is not user-ready image generation. The minimum
usable path is workflow editor input to backend scheduler admission, runtime
dispatch, PyTorch/Diffusers execution, artifact retention, and UI-visible
result projection.

## Scope

### In Scope

- Use the workflow editor or the same Tauri command path the workflow editor
  uses to submit a canonical `llm-inference` image-generation workflow.
- Use a bounded local Diffusers fixture, initially Tiny SD Turbo or another
  small Pumas-registered text-to-image bundle.
- Verify runtime provisioning prerequisites before execution: Python
  executable/env mapping, PyTorch/Diffusers worker importability, Pumas model
  package facts, artifact load-target readiness, runtime-registry dispatch
  identity, device policy, resource fit, and scheduler admission.
- Execute through the canonical scheduler, worker-owned runtime branch,
  dispatch assignment, runtime-host batch/solo path, and PyTorch
  image-generation backend.
- Verify one retained image media body plus descriptor/projection facts for the
  workflow run, with no duplicate embedded image bodies in workflow JSON or
  secondary projections.
- Verify the workflow editor/run UI can observe completion and retrieve or
  render the generated artifact through backend-owned artifact commands.
- Verify fail-closed diagnostics for missing model package, missing Python
  environment, missing Diffusers dependencies, unavailable device, missing
  load target, and scheduler/runtime dispatch failure.
- Add a repeatable smoke command or test harness that can be run locally when a
  real local model fixture is provisioned.

### Out Of Scope

- Reintroducing retired `diffusion-inference` nodes or saved-graph migration.
- Preserving request-scoped runtime execution, singleton runtime-host dispatch,
  direct `ModelDependencyRequest`, `ModelRefV2`, graph-visible `model_path`, or
  worker-side fallback execution.
- Making frontend, Tauri, saved workflow JSON, or test fixtures the source of
  truth for runtime, device, model, or load-target selection.
- Hardcoding Tiny SD Turbo behavior beyond using it as the first bounded smoke
  fixture.
- Implementing image-to-image, inpainting, ControlNet, LoRA composition, or
  multi-image UX beyond existing extension points.
- Solving generalized batch observability beyond per-run/per-member terminal
  diagnostics unless a separate re-plan selects that diagnostics contract.

## Tasks

1. **Readiness inventory and harness gate**
   - Identify the exact app command path used by the workflow editor to create,
     run, observe, and inspect workflow execution sessions.
   - Identify current local model/runtime provisioning knobs, including Python
     executable/env map, Pumas model id/artifact id, runtime registry
     registration, and device policy.
   - Add or document a smoke harness entrypoint that runs only when a real
     local model fixture is explicitly configured.
   - Verification: no production execution change; plan notes record command
     path, required environment variables/configuration, expected diagnostics,
     and dirty-worktree status.

2. **Fail-closed app path diagnostics**
   - Run the workflow editor command path without a configured model/runtime
     fixture and verify the UI/API receives typed diagnostics instead of
     fallback execution or silent success.
   - Add focused tests only where existing coverage does not prove diagnostic
     projection to the workflow editor path.
   - Verification: affected frontend/Tauri/backend tests plus focused
     diagnostic smoke.

3. **Provisioned Tiny Diffusers smoke**
   - Register or load the bounded Pumas text-to-image model fixture through the
     canonical package/load-target/runtime readiness path.
   - Run the canonical workflow through the same app path the workflow editor
     uses.
   - Verify prompt input, scheduler admission, selected backend/runtime/device,
     worker-owned dispatch, PyTorch/Diffusers execution, retained artifact, and
     UI-readable artifact descriptor/body.
   - Verification: local real-model smoke with recorded prerequisites and
     output artifact metadata. If the model fixture is unavailable, record the
     missing external prerequisite and keep the milestone open.

4. **Workflow editor usability acceptance**
   - Confirm the workflow editor can start the run, surface progress or
     diagnostics, and show or open the generated artifact without requiring
     direct script use.
   - Ensure frontend state remains declarative and backend-owned; no optimistic
     model/runtime/device/artifact state is introduced.
   - Verification: frontend interaction test or manual Playwright/app smoke,
     plus artifact retrieval check.

5. **Regression gate and closure**
   - Run focused backend, embedded-runtime, frontend, and release-smoke checks
     affected by the app end-to-end path.
   - Update plan notes with exact verification, deviations, and remaining
     external prerequisites.
   - Mark this milestone complete only after a real configured
     image-generation workflow produces a retained UI-visible image artifact.

## Risks And Mitigations

- **Runtime fixture unavailable locally:** keep the diagnostic harness runnable
  without a model, but do not mark this milestone complete until a provisioned
  model generates an artifact.
- **Frontend accidentally owns backend state:** use backend commands/events as
  source of truth and add tests around artifact projection rather than local
  optimistic state.
- **Model provisioning bypasses Pumas/load-target contracts:** require Pumas
  package facts and approved load-target readiness before worker execution.
- **Device/runtime fallback masks failures:** explicit unavailable device or
  runtime choices must fail with typed diagnostics; auto must record one
  selected valid decision or fail.
- **Smoke becomes machine-specific and unrepeatable:** record required
  environment variables, fixture identity, and skip/fail conditions.

## Re-Plan Triggers

- The workflow editor cannot invoke the canonical workflow execution session
  path without a frontend/Tauri contract change.
- The current runtime provisioning model cannot register a real
  PyTorch/Diffusers runtime without changing runtime-registry, Pumas, generated
  DTOs, lockfiles, or saved workflow fixtures.
- The real model smoke would require direct script-only execution, retired
  graph nodes, request-scoped runtime execution, or graph-visible local model
  paths.
- Artifact display requires frontend-inferred artifact paths or duplicated
  media bodies instead of backend-owned artifact descriptors/bodies.
- Missing prerequisites cannot be represented as typed diagnostics.

## Completion Criteria

- A real configured workflow-editor image-generation run produces a retained
  image artifact.
- The generated artifact is visible or retrievable through the app UI path.
- The same path reports typed diagnostics for missing runtime/model/device
  prerequisites.
- No legacy/fallback execution path is reachable for successful image
  generation.
- Verification commands, manual smoke notes, environment prerequisites, and
  deviations are recorded in the plan.

**Status:** Planned. This is the next user-readiness milestone after the
headless contract validation closure.
