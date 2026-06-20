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

## 2026-06-19 Re-Plan Decision: Bridge First, GUI Smoke Next

Selected option: **Option 4, command bridge harness first and workflow-editor
GUI smoke second.**

This re-plan keeps Tauri as an app/transport composition boundary only. Tauri
must not own workflow business logic, runtime/device/model policy, scheduler
admission rules, artifact retention policy, or UI projection semantics.

Ownership boundaries for the remaining Milestone 9 work:

- Backend Rust crates own workflow execution, scheduler admission,
  worker/runtime lifecycle, runtime/device/model/load-target decisions,
  diagnostics, artifact retention, artifact descriptors, and artifact bodies.
- Tauri owns command registration, IPC serialization/deserialization, app
  handles, event-channel bridging, app-directory composition, shared-state
  wiring, and backend error-envelope transport.
- Svelte/TypeScript owns editor interaction, transient UI state, declarative
  rendering, workbench navigation, and presentation of backend-owned
  projections. It must not infer runtime/model/device/artifact paths or
  duplicate retained media bodies.

Sequence:

1. Add a focused in-process Tauri command bridge harness. It must exercise the
   same command surfaces the editor uses and assert that canonical backend
   diagnostics, event-channel delivery, and artifact descriptor/body responses
   survive the IPC bridge unchanged. It may use missing-fixture fail-closed
   cases and backend-owned test fixtures, but it must not decide runtime,
   model, device, scheduler, or artifact policy in Tauri.
2. Add or run a configured desktop workflow-editor GUI smoke. It must submit
   the canonical image-generation workflow through the editor, use a Pumas
   model id/artifact id, reach scheduler admission and worker-owned
   PyTorch/Diffusers execution, and verify the retained image artifact through
   I/O Inspector descriptor/body commands.

The command bridge harness is not sufficient to close this milestone. Milestone
9 closes only after the configured workflow-editor GUI path produces a retained
UI-visible image artifact or records typed fail-closed diagnostics for missing
external prerequisites.

Bridge harness status:

- 2026-06-19: Completed the first focused bridge sub-slice. Added an
  in-process headless command bridge test proving that an image stream event
  sent through `TauriEventAdapter` emits artifact references on the Tauri
  channel, removes inline image payload ownership from the transport event,
  preserves diagnostics-event delivery, and reads the retained artifact through
  the command adapter's backend-owned artifact descriptor/body forwarding path.
- Allowed files touched for the bridge sub-slice:
  `src-tauri/src/workflow/headless_workflow_commands.rs`,
  `src-tauri/src/workflow/headless_workflow_commands_tests.rs`,
  `src-tauri/src/workflow/headless_workflow_commands_tests/README.md`,
  `src-tauri/src/workflow/headless_workflow_commands_tests/command_bridge.rs`,
  `src-tauri/src/workflow/headless_workflow_commands_tests/transport_responses.rs`,
  this milestone, and the rolling plan.
- No-fallback/no-legacy confirmation: the slice did not add runtime/model/
  device/scheduler/artifact-retention policy to Tauri, did not add a direct
  execution path, and did not preserve legacy graph/runtime behavior. The
  production change only extracts private command-adapter forwarding helpers
  that still call backend `WorkflowService` artifact APIs and preserve backend
  error envelopes.
- Discovered and fixed stale adjacent test coverage: two trace snapshot
  fixtures used queue item id `queue-1` while the canonical trace key was
  `run-1`, preventing scheduler queue timing from joining by backend
  `workflow_run_id`. The fixture now uses the backend run id.
- Verification passed:
  `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`,
  `cargo test --manifest-path src-tauri/Cargo.toml command_bridge_preserves_image_artifact_event_and_body_read`,
  and `cargo test --manifest-path src-tauri/Cargo.toml headless_workflow_commands`.
- Remaining bridge/app-path gap: create/run command fail-closed diagnostics
  still need app-path coverage if existing tests are insufficient, and the
  configured workflow-editor GUI smoke is still required before this milestone
  can close.

## 2026-06-19 Re-Plan Decision: Real Desktop GUI E2E Harness

Selected option: **Option 1, add a real desktop GUI E2E harness for the
workflow editor image-generation path.**

Reason: Milestone 9 is a user-readiness milestone. The existing bridge and
frontend command tests now provide useful boundary confidence, but they do not
prove that a person can submit the canonical image workflow from the desktop
workflow editor and retrieve the generated image artifact through the app UI.
Testing Standards require a cross-layer acceptance path for this cross-layer
feature, and the accepted path must include the visible editor interaction.

Scope added to this milestone:

- Choose and document a desktop GUI automation approach for Tauri/Pantograph.
  The choice must be standards-aligned, deterministic enough for local/CI use,
  and explicit about any Linux display/runtime prerequisites.
- Add an opt-in workflow-editor E2E smoke harness that launches the desktop app
  or an equivalent Tauri desktop test target, drives the workflow editor UI,
  submits the canonical image-generation workflow through the toolbar, observes
  backend events through the app, opens the I/O Inspector result path, and
  verifies a retained image artifact descriptor/body.
- If the harness needs new JavaScript tooling, update `package.json` and
  `package-lock.json` in a dedicated serial tooling slice. Do not mix lockfile
  changes with backend runtime/model/device policy changes.
- Keep the real model requirement opt-in and fail closed. Missing
  `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID`, artifact id, Python runtime, or
  GUI display prerequisites must produce typed diagnostics or explicit smoke
  preflight failures, not silent fallback to mock execution.

Out of scope for the GUI harness:

- Moving workflow, scheduler, runtime, model, device, artifact retention, or
  diagnostics policy into Tauri.
- Moving backend-owned projection facts into Svelte/TypeScript state.
- Reusing the generated C#/native runtime smoke as a substitute for GUI
  editor interaction.
- Running direct script-only inference, retired graph nodes, direct model path
  variables, or request-scoped runtime execution.

Execution sequence:

1. **GUI harness selection slice**
   - Inspect feasible Tauri desktop GUI automation options in this repo.
   - Select the smallest standards-aligned toolchain.
   - Record prerequisites, write set, environment variables, and failure modes
     before adding dependencies or scripts.
   - Verification: documentation/plan update plus `git diff --check`.

2. **Tooling scaffold slice**
   - Add the selected GUI test harness, minimal config, and an opt-in smoke
     script.
   - If JavaScript dependencies are needed, include `package.json` and
     `package-lock.json` only in this slice.
   - Verification: harness preflight fails closed when GUI/model prerequisites
     are missing; existing frontend command tests still pass.

3. **Workflow-editor GUI smoke slice**
   - Drive the desktop workflow editor through the real submit path.
   - Use a Pumas model id/artifact id, reach scheduler admission and
     worker-owned PyTorch/Diffusers execution, and verify retained image
     artifact descriptor/body through I/O Inspector UI.
   - Verification: configured smoke passes on a machine with the declared real
     model/runtime/display prerequisites. Missing prerequisites fail closed.

Completion rule: this milestone closes only after sequence step 3 passes with a
real configured image workflow or records typed fail-closed diagnostics for
missing external prerequisites at the GUI/app boundary.

### 2026-06-19 GUI Harness Selection Slice

Selected harness: **WebdriverIO running against Tauri's WebDriver support via
`tauri-driver`.**

Why this is the selected standards-aligned option:

- It is the official Tauri desktop GUI automation path for Linux/Windows and
  drives the native desktop WebView through WebDriver instead of replacing the
  app with a browser-only harness.
- It can start from the visible workflow editor action and end at the retained
  image artifact in the app, which satisfies the Testing Standards
  cross-layer acceptance requirement for this user-facing milestone.
- It keeps architectural ownership intact: backend Rust owns workflow,
  scheduler, runtime/model/device/load-target, diagnostics, artifact retention,
  descriptor, and body decisions; Tauri owns command/event bridge mechanics;
  Svelte/TypeScript owns interaction and presentation only.
- It requires no workflow business logic in the harness. The harness should
  click/inspect the UI and wait for backend-projected state, not synthesize
  workflow outcomes or artifact bodies.

References reviewed:

- Tauri WebDriver documentation:
  `https://tauri.app/develop/tests/webdriver/`
- Tauri WebdriverIO example:
  `https://tauri.app/develop/tests/webdriver/example/webdriverio/`

Local repo/tooling observations:

- `package.json` currently has no WebdriverIO, Selenium, or Playwright GUI E2E
  dependencies.
- `package-lock.json` exists, so any JavaScript dependency addition must update
  `package.json` and `package-lock.json` together in the next dedicated tooling
  scaffold slice.
- `which tauri-driver` and `which WebKitWebDriver` returned no local binary in
  this shell. The scaffolded smoke must fail closed with a clear prerequisite
  diagnostic when either dependency is missing.
- Existing frontend command tests already cover
  `workflow_create_execution_session` / `workflow_run_execution_session`
  request/channel/error-envelope preservation. Do not add duplicate bridge
  tests unless GUI harness implementation exposes a concrete missing boundary
  invariant.

Chosen next tooling scaffold:

- Add WebdriverIO dependencies/config in a dedicated slice.
- Add an opt-in script such as
  `scripts/check-workflow-editor-image-generation-gui-smoke.sh`.
- The script must preflight:
  `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID`,
  `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID`,
  `PANTOGRAPH_PYTHON_EXECUTABLE`, `tauri-driver`, `WebKitWebDriver`, and a GUI
  display (`DISPLAY` or an explicit documented headless display wrapper).
- The first scaffolded test may stop after launching the app and verifying the
  visible workflow editor shell if the real model prerequisites are missing;
  it must not silently switch to mocks or direct script execution.

Rejected alternatives:

- **Browser-only Playwright/Vite harness.** Rejected because it would not run
  the Tauri desktop command/event bridge and could pass while the app path is
  broken.
- **Selenium-only custom setup.** Rejected for this repo because Tauri documents
  WebdriverIO as the smallest ready example for the Node/npm stack already used
  here; Selenium can be revisited only if WebdriverIO cannot drive the needed
  WebView controls.
- **More in-process command bridge tests.** Rejected as the main next step
  because the milestone now needs visible workflow editor acceptance, not more
  headless confidence.
- **Generated C#/native runtime smoke as completion.** Rejected because it
  proves runtime execution but not workflow-editor usability.

Failure modes to preserve:

- Missing GUI driver/display/model/runtime prerequisites fail closed before
  execution with explicit diagnostics.
- Missing backend runtime/model/device decisions return backend typed
  diagnostics; the harness must display or assert those diagnostics rather
  than falling back to mocks.
- Any need to put scheduler/runtime/model/device/artifact policy in Tauri,
  Svelte, or the test harness is a re-plan trigger.

### 2026-06-19 Tooling Scaffold Slice

Scaffolded the selected desktop GUI harness without adding product runtime
behavior:

- Added root-owned WebdriverIO dependencies because the root package owns the
  desktop app E2E command boundary.
- Added `npm run test:workflow-editor-image-gui`, which delegates to
  `scripts/check-workflow-editor-image-generation-gui-smoke.sh`.
- Added a fail-closed Linux preflight for
  `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID`,
  `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID`,
  `PANTOGRAPH_PYTHON_EXECUTABLE`, `tauri-driver`, `WebKitWebDriver`, a GUI
  display, and the local WebdriverIO install.
- Added `tests/e2e/workflow-editor-image-generation/wdio.conf.mjs`, which
  builds the Tauri debug desktop app with `backend-pytorch`, starts
  `tauri-driver`, and drives the app through WebDriver.
- Added the first GUI spec that proves the desktop app opens, the Graph page is
  navigable, the workflow submit control is visible, and the I/O Inspector
  navigation exists. This is a scaffold acceptance shell only; it does not close
  this milestone until the next slice submits a configured image workflow and
  verifies a retained artifact descriptor/body through the UI.

No-fallback/no-legacy confirmation:

- The harness does not run direct script-only inference, direct model paths,
  retired graph nodes, request-scoped runtime execution, mocks, or frontend
  artifact path inference.
- The preflight rejects retired direct model path environment variables and
  requires canonical Pumas model/artifact identifiers.
- Tauri remains an app/WebDriver/IPC bridge; workflow, scheduler,
  runtime/model/device, diagnostics, and artifact policy remain backend-owned.

Discovered issues and follow-ups:

- `npm install` added the WebdriverIO dependency tree and reported deprecated
  transitive packages. Follow-up `npm audit --json` reported `17` findings
  (`10` moderate, `7` high), while `npm audit --omit=dev` reported `4`
  production dependency findings (`3` moderate, `1` high). This does not block
  the opt-in scaffold slice, but the next dependency review must decide whether
  WebdriverIO remains acceptable, needs targeted overrides, or should be
  replaced before promoting this smoke into CI.
- The WebdriverIO tree is a large dev-only dependency addition. It is justified
  here because desktop WebDriver automation is specialized system-test tooling
  owned by the root app command boundary, but it must remain opt-in until the
  dependency/audit follow-up is resolved.
- This local shell still lacks `tauri-driver` and `WebKitWebDriver`, so the
  configured GUI session was not launched here. The verified behavior for this
  slice is the fail-closed preflight path plus static/tooling validation.
- Verification passed:
  `./scripts/check-workflow-editor-image-generation-gui-smoke.sh` failed
  closed on missing `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID`,
  `npm run test:workflow-editor-image-gui` failed closed on the same missing
  prerequisite, `node --check
  tests/e2e/workflow-editor-image-generation/wdio.conf.mjs`, `node --check
  tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs`,
  `npm run lint`, `npm run typecheck`, and `git diff --check`.

### 2026-06-19 GUI Selector And Artifact Path Slice

Extended the GUI harness from an editor shell check to the real configured
workflow-editor image artifact path:

- Added a required
  `PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID` preflight variable so the
  smoke selects an explicitly provisioned saved workflow instead of relying on
  whatever workflow happens to be last opened.
- Added stable, non-behavioral `data-testid` hooks for workbench navigation,
  the Graph page, graph selector workflow options, workflow submit/version/error
  controls, I/O Inspector, artifact cards, artifact media family, artifact read
  buttons, and image previews.
- Updated the WebdriverIO spec to open the desktop app, navigate to Graph,
  select the declared saved workflow, wait for submit readiness, submit through
  the visible toolbar, wait for I/O Inspector, locate a retained image artifact
  card, invoke the backend-owned artifact body read through the UI, and assert
  that the image preview renders.

No-fallback/no-legacy confirmation:

- The harness still does not create workflow outcomes, infer artifact paths,
  provide runtime/model/device policy, or call direct script-only inference.
- The new workflow id is an explicit test input, not a fallback. Missing it
  fails before WebDriver or app launch.
- Tauri remains the desktop/IPC bridge, Svelte/TypeScript exposes stable UI
  controls, and backend commands remain responsible for workflow submission,
  scheduler admission, runtime execution, diagnostics, artifact descriptor, and
  artifact body reads.

Verification passed:

- `node --check
  tests/e2e/workflow-editor-image-generation/workflow-editor-image-generation.e2e.mjs`.
- `npm run typecheck`.
- `npm run lint`.
- `npm run test:frontend -- workflowToolbarEvents`.
- `env PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID=dummy
  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID=dummy
  PANTOGRAPH_PYTHON_EXECUTABLE=/usr/bin/python3
  ./scripts/check-workflow-editor-image-generation-gui-smoke.sh` failed closed
  on missing `PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID`.
- The same command with
  `PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID=dummy` failed closed on
  missing `tauri-driver`, before app execution.

Remaining blocker:
the configured desktop GUI smoke still has not been launched in this shell
because `tauri-driver`, `WebKitWebDriver`, display/runtime prerequisites, and a
real saved image workflow id are not available here. The next slice must either
run it in a provisioned environment or record typed fail-closed diagnostics at
the GUI/app boundary.

### 2026-06-19 Saved Workflow Preflight Slice

Tightened the configured GUI smoke preflight after discovering the documented
example used the built-in template id while the workflow editor selector loads
saved workflows from `.pantograph/workflows/${id}.json`.

Implemented:

- Corrected the GUI smoke README example to use the tracked saved workflow id
  `tiny-sd-turbo-diffusion`.
- Added a fail-closed shell preflight that rejects path/display-name workflow
  ids and verifies the explicit
  `PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID` resolves to a saved
  workflow file before checking GUI driver availability.

No-fallback/no-legacy confirmation:

- The script still requires an explicit saved workflow id. It does not select a
  default workflow, search for alternatives, or fall back to a built-in template.
- The check mirrors the current editor loading contract and does not move
  workflow, scheduler, runtime/model/device, diagnostics, or artifact policy out
  of the backend.

Verification passed:

- Missing `PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID` still fails
  closed.
- `PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID='Tiny SD Turbo Diffusion'`
  fails closed as an invalid saved workflow id.
- `PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID=tiny-sd-turbo-text-to-image`
  fails closed because no saved workflow file exists for that template id.
- `PANTOGRAPH_WORKFLOW_EDITOR_IMAGE_SMOKE_WORKFLOW_ID=tiny-sd-turbo-diffusion`
  passes the saved-workflow preflight and then fails closed on missing
  `tauri-driver`, before app execution.

### 2026-06-19 GUI Driver Environment Diagnostic Slice

Attempted to continue from the configured GUI smoke slice by provisioning the
desktop WebDriver prerequisites for this shell.

Environment observations:

- `DISPLAY=:0.0` is present.
- `tauri-driver` was missing at slice start. Installed user-level
  `tauri-driver v2.0.6` with `cargo install tauri-driver --locked`, matching
  the official Tauri WebDriver setup.
- `WebKitWebDriver` is still missing from `PATH`.
- The system is Linux Mint 22.2 / Ubuntu Noble. `apt-cache policy
  webkit2gtk-driver` reports no installed package and candidate
  `2.52.3-0ubuntu0.24.04.1`.
- `sudo apt-get update && sudo apt-get install -y webkit2gtk-driver` could not
  run here because `sudo` requires an interactive password.

No-fallback/no-legacy confirmation:

- The configured desktop GUI smoke was not run without `WebKitWebDriver`.
- No browser-only, headless-only, direct script, direct model path, retired
  graph, request-scoped runtime, or frontend-inferred artifact substitute was
  used.

Current blocker:

- Install `webkit2gtk-driver` (or otherwise place a compatible
  `WebKitWebDriver` on `PATH`) in the host environment, then rerun
  `npm run test:workflow-editor-image-gui` with the declared Pumas model id,
  artifact id, saved workflow id, and Python executable.

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

   **2026-06-18 inventory status:** In progress.
   - Dirty-worktree inspection before this slice found unrelated proposal docs:
     `PROPOSAL-pumas-artifact-load-target-resolution.md` and
     `PROPOSAL-pumas-library-fast-model-snapshot.md`. No source, test,
     config, lockfile, generated, build-output, SQLite WAL/SHM, or workflow
     fixture file dirtiness was used for this inventory slice.
   - Standards reviewed for this user-facing cross-layer lane:
     `PLAN-STANDARDS.md`, `TESTING-STANDARDS.md`,
     `FRONTEND-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`, and
     `COMMIT-STANDARDS.md`.
   - Workflow editor execution path identified:
     `src/components/WorkflowToolbar.svelte` creates the execution session,
     publishes the executable validation snapshot, and calls
     `WorkflowCommandService.runWorkflowExecutionSession`.
     `src/services/workflow/WorkflowCommandService.ts` invokes
     `workflow_create_execution_session` and
     `workflow_run_execution_session` over Tauri with a workflow event
     channel. `src-tauri/src/workflow/headless_workflow_commands.rs` builds a
     `TauriEventAdapter`, calls `build_runtime(...)`, then runs
     `EmbeddedRuntime::run_workflow_execution_session_with_event_sink`.
     `src-tauri/src/workflow/headless_runtime.rs` composes the embedded
     runtime from app data dir, project root, host runtime mode info, shared
     extensions, shared workflow service, runtime registry, gateway, and RAG
     backend.
   - Artifact retrieval path identified:
     `WorkflowCommandService.artifactDescriptor`, `readArtifactBody`, and
     `readArtifactStream` invoke Tauri artifact commands. The backend command
     layer forwards those to the workflow service artifact store. The
     I/O inspector uses `workflow_io_artifact_query` projections, verifies the
     descriptor, reads retained bodies or streams, and renders image previews
     from transient browser object URLs. This means the UI already has the
     primitive needed to inspect an image artifact if a real run produces one.
   - Existing smoke and validation scripts classified:
     `scripts/check-current-image-workflow-smoke.mjs` validates canonical
     saved workflow shape only; it does not run a model.
     `scripts/diffusion_cli_smoketest.py` is worker-only and raw-model-path
     oriented; it is useful for isolation but is not the app/editor path.
     `scripts/check-uniffi-csharp-diffusion-smoke.sh` is the closest existing
     real-model smoke: it requires
     `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID`, optional
     `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID`, and a Python executable
     able to import `torch`, `diffusers`, `transformers`, `accelerate`, and
     `Pillow`. It rejects retired path env vars and still runs the generated
     C#/native runtime path, not the workflow editor/Tauri path.
     `scripts/check-runtime-redistributables-smoke.sh` is the release
     headless contract smoke and explicitly does not prove a full desktop GUI
     model execution session.
   - Runtime and model provisioning facts identified:
     Python selection is controlled by the documented environment mapping
     path: `PANTOGRAPH_PYTHON_ENV_MAP_JSON`,
     `PANTOGRAPH_PYTHON_ENV_MAP_FILE`, `PANTOGRAPH_PYTHON_EXECUTABLE`,
     `PYO3_PYTHON`, PATH lookup, or the project `.venv` fallback. Pumas model
     selection must use a model id and artifact id through the canonical
     Puma-Lib node and package/load-target facts path, not a graph-visible or
     script-only local model path. Runtime capability facts are sourced from
     the shared runtime registry; if no runtime is registered or a runtime
     lacks backend keys/dispatch identity, dispatch planning emits typed
     diagnostics instead of fabricating a selection.
   - Existing docs drift discovered:
     `docs/python-runtime-separation.md`,
     `docs/headless-embedding-implementation-notes.md`, and older completed
     plan notes still show retired
     `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH` examples. Current script
     behavior rejects that variable and requires Pumas model id based
     selection. This is documentation drift, not a reason to add a path
     fallback.
   - Immediate implementation gap:
     no current harness proves `WorkflowToolbar.svelte`/Tauri
     `workflow_run_execution_session` can submit the canonical Tiny SD Turbo
     workflow, reach PyTorch/Diffusers execution, and then display the retained
     image via I/O inspector artifact commands. The next source slice must add
     a fail-closed app-path diagnostic check or app-path smoke harness before
     any real-model completion claim.

2. **Fail-closed app path diagnostics**
   - Run the workflow editor command path without a configured model/runtime
     fixture and verify the UI/API receives typed diagnostics instead of
     fallback execution or silent success.
   - Add focused tests only where existing coverage does not prove diagnostic
     projection to the workflow editor path.
   - Verification: affected frontend/Tauri/backend tests plus focused
     diagnostic smoke.

   **2026-06-18 command-boundary diagnostics slice:** Completed.
   - Added frontend command-service coverage proving
     `workflow_run_execution_session` failures preserve backend typed error
     envelopes and diagnostics links at the same Tauri command boundary the
     workflow editor uses.
   - This does not replace the required real app/model smoke. It proves the
     frontend command layer does not collapse missing runtime/model diagnostics
     into an untyped transport error while the real fixture harness is still
     pending.
   - No-fallback/no-legacy confirmation: the test expects a typed
     `runtime_not_ready` backend envelope and does not add fallback execution,
     mock success, or direct model-path behavior.
   - Verification passed:
     `npm run test:frontend -- WorkflowService.commands`,
     `npm run typecheck`, and
     `git diff --check -- src/services/workflow/WorkflowService.commands.test.ts docs/plans/current-image-generation-graphs/milestones/09-workflow-editor-e2e-image-generation.md docs/plans/current-image-generation-graphs/plan.md`.

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

   **2026-06-18 real-backend smoke wrapper slice:** Completed.
   - Added `scripts/check-workflow-image-generation-real-smoke.sh` as an
     opt-in lane command for configured machines. It requires
     `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID`, accepts optional
     `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID`, rejects retired direct
     model-path variables, validates canonical image workflow shape, verifies
     the desktop Tauri crate builds with `backend-pytorch`, runs the
     generated-C#/native-runtime real Diffusers session smoke, and runs focused
     frontend command projection checks.
   - This is an intermediate real-backend gate, not milestone completion. It
     proves the configured real Diffusers session path and editor-adjacent
     command projections under one command, but it still does not drive the
     desktop workflow toolbar/Tauri event channel and I/O Inspector in one GUI
     session.
   - No-fallback/no-legacy confirmation: the wrapper requires Pumas model id
     selection and refuses `PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH` and
     `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH`; it does not introduce
     direct model-path graph inputs or alternate worker execution logic.
   - Verification passed:
     `bash -n scripts/check-workflow-image-generation-real-smoke.sh`,
     missing-env fail-closed wrapper run exited 2 with the expected
     `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID` diagnostic,
     `cargo check --manifest-path src-tauri/Cargo.toml --features backend-pytorch`,
     `node scripts/check-current-image-workflow-smoke.mjs`, and
     `npm run test:frontend -- workflowToolbarEvents WorkflowService.commands`.
     Real Diffusers execution was not run in this slice because the required
     Pumas model fixture environment was not configured in this shell.

   **2026-06-19 desktop PyTorch build-gate slice:** Completed.
   - Extended the opt-in real image-generation smoke wrapper so configured
     runs must compile the desktop Tauri crate with the `backend-pytorch`
     feature before claiming real-runtime smoke success.
   - No-fallback/no-legacy confirmation: the wrapper still requires Pumas
     model id based selection and rejects retired direct model path variables;
     the added gate only validates the canonical desktop backend surface.
   - Verification passed:
     `cargo check --manifest-path src-tauri/Cargo.toml --features backend-pytorch`,
     `bash -n scripts/check-workflow-image-generation-real-smoke.sh`, and the
     missing-env fail-closed wrapper run exited 2 with the expected
     `PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID` diagnostic,
     `node scripts/check-current-image-workflow-smoke.mjs`, and
     `npm run test:frontend -- workflowToolbarEvents WorkflowService.commands`.
   - Remaining gap: this still does not prove a configured workflow editor GUI
     session can submit the workflow, receive events, and show the retained
     image artifact in I/O Inspector.

4. **Workflow editor usability acceptance**
   - First add the focused Tauri command bridge harness selected by the
     2026-06-19 re-plan. It must prove bridge wiring only: create/run command
     forwarding, backend typed diagnostics preservation, event-channel
     delivery, and artifact descriptor/body command forwarding.
   - Then confirm the workflow editor can start the configured real run,
     surface progress or diagnostics, and show or open the generated artifact
     without requiring direct script use.
   - Ensure frontend state remains declarative and backend-owned; no optimistic
     model/runtime/device/artifact state is introduced.
   - Verification: focused Tauri bridge test plus frontend interaction test or
     manual Playwright/app smoke, plus artifact retrieval check.

   **2026-06-18 partial usability slice:** In progress.
   - The workflow toolbar now selects the I/O Inspector after a successful run
     whose backend `WorkflowRunResponse.outputs` contain image/media artifact
     output facts. Non-image runs preserve the previous Scheduler-page
     behavior.
   - This is not a completion claim for real image generation. It only closes
     a UI usability gap so a real editor-submitted image run lands on the
     backend-owned artifact inspection surface instead of leaving the generated
     image hidden behind a separate manual navigation step.
   - No-fallback/no-legacy confirmation: the frontend does not store image
     bodies, infer artifact paths, decide runtime/model/device, or bypass
     backend artifact projections. The I/O Inspector remains responsible for
     reading descriptors/bodies through backend commands.

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
- The Tauri bridge harness would need to own or duplicate workflow business
  logic, runtime/device/model policy, scheduler decisions, artifact retention
  policy, or frontend projection semantics.
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

**Status:** In progress. The diagnostic preservation, artifact-navigation, and
real-image smoke wrapper slices are complete. The milestone remains open until
a configured desktop workflow-editor run produces a retained image artifact
that is visible or retrievable through the app UI.
