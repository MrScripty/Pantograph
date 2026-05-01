# Plan: Managed Binary Runtime Readiness

## Objective

Make managed binary state the single source of truth for every
Pantograph-managed binary category: inference runtime sidecars such as
`llama.cpp` and `Ollama`, media tool binaries such as `ffmpeg`,
`ocioconvert`, and `oiiotool`, and native redistributable artifacts such as
OpenColorIO/OCIO. Runtime launch and scheduler model-load diagnostics must
consume that same source of truth so a workflow run cannot report
`model load completed` until the requested llama.cpp process is running the
requested model and has passed backend readiness checks.

## Scope

### In Scope

- Audit and align `crates/inference::managed_runtime`,
  `crates/inference::managed_redistributables`, Pantograph embedded-runtime
  projections, Tauri process spawning, and Settings managed-binary views.
- Define one backend-owned managed binary contract that exposes category,
  install/readiness state, selected version, resolved command when applicable,
  expected files, missing files, checksums/source metadata when available, and
  launch or activation validation facts.
- Preserve distinct categories for inference runtime sidecars and media
  redistributables while allowing systems that need binaries to query one
  authoritative registry/facade. `ffmpeg`, `ocioconvert`, `oiiotool`, and
  OpenColorIO/OCIO must be first-class managed binary entries, not UI-only
  exceptions.
- Make workflow runtime admission, conversion dependency planning, scheduler
  diagnostics, and process spawning consume the same managed binary readiness
  facts.
- Replace ambiguous `model load completed` emission with phase-specific events:
  dependency resolved, process spawned, HTTP ready, requested model active, and
  model load complete.
- Add regressions for missing/partial llama.cpp installs, selected-version
  drift, path migration/fallback, false ready events, and model mismatch reuse.
- Update touched README/API contracts and ADR links where ownership or
  structured contracts change.

### Out of Scope

- Replacing llama.cpp, Ollama, ffmpeg, or OCIO release-source policy.
- Adding new managed runtime families.
- Rebuilding the Settings UI beyond consuming the unified backend view.
- Changing Puma-Lib model metadata semantics except where model path resolution
  must be verified before runtime load.
- Fixing unrelated dirty files outside the managed binary/runtime readiness
  scope.

## Inputs

### Problem

The current user-visible failure is a workflow run that enters scheduler
execution and records:

```text
Scheduler Model Lifecycle Changed
model load completed
17 ms; model loaded; runtime admission loaded required models
```

A 17 ms load for an approximately 20 GB model indicates the event is not tied
to actual model residency. The codebase currently has multiple related
surfaces:

- `crates/inference/src/managed_runtime/` owns sidecar runtime catalog,
  install, selection, validation, state, and `resolve_binary_command`.
- `crates/inference/src/managed_redistributables/` owns non-runtime media
  dependencies such as `ffmpeg`, `ocioconvert`, `oiiotool`, and
  OpenColorIO/OCIO artifacts.
- `src-tauri/src/llm/process_tauri.rs` maps sidecar spawn requests to
  `resolve_binary_command` and launches the executable.
- `crates/pantograph-embedded-runtime/src/runtime_capabilities.rs` projects
  managed runtime snapshots into workflow runtime capabilities.
- `crates/pantograph-embedded-runtime/src/embedded_workflow_host.rs` combines
  managed runtime capabilities, gateway backends, Python, and host runtime
  facts for scheduler admission.
- `crates/inference/src/server.rs` waits for llama.cpp stdout/stderr
  "listening" plus `/health`, but the scheduler event wording does not prove
  that the requested model path is active or that process launch used the same
  managed runtime selected in Settings.

### Constraints

- ADR-003 keeps runtime catalog, install state, selection policy, retained
  artifacts, restart reconciliation, readiness validation, and command
  resolution backend-owned.
- Tauri remains adapter/composition only and must not invent runtime install or
  readiness truth.
- Workflow, scheduler, diagnostics, and process launch must consume additive
  backend contracts.
- Runtime sidecars, media tool binaries, and native redistributable artifacts
  remain distinct product categories even if exposed through one facade.
- Existing app-data installs may exist under legacy paths such as `runtimes/`
  while current local changes indicate a move toward `third-party/runtimes/`
  and `third-party/managed-dependencies/`; implementation must handle
  migration or explicit fallback safely.
- Install/remove/download jobs are asynchronous and can overlap with workflow
  submission, app restart reconciliation, and diagnostics refresh.
- Implementation must stay compliant with the external Coding Standards:
  backend-owned data, additive contracts, source-directory READMEs,
  decomposition review for files over 500 lines, sync-core/async-shell,
  tracked task ownership, path validation at boundaries, isolated durable-state
  tests, and narrow dependency ownership.

### Assumptions

- The active workflow failure is more likely caused by managed binary/runtime
  readiness drift than by the workflow save/submit fixes.
- A backend-owned facade can be added without breaking existing Tauri command
  names by preserving current public commands as adapters.
- llama.cpp model readiness can be proven by combining resolved command facts,
  process lifecycle, HTTP readiness, and the server's active mode/model path
  state.
- If upstream llama.cpp exposes a stronger model/status endpoint for the
  running model, implementation should prefer that over log-line inference.

### Dependencies

- `crates/inference/src/managed_runtime/contracts.rs`,
  `definitions.rs`, `operations.rs`, `operations/projection.rs`,
  `operations/state_transitions.rs`, `paths.rs`, and
  `llama_cpp_platform/`.
- `crates/inference/src/managed_redistributables/contracts.rs`,
  `operations.rs`, `state.rs`, and `paths.rs`.
- `crates/inference/src/server.rs`,
  `backend/llamacpp.rs`, `gateway.rs`, and `process.rs`.
- `src-tauri/src/llm/process_tauri.rs` and
  `src-tauri/src/llm/commands/binary.rs`.
- `crates/pantograph-embedded-runtime/src/managed_runtime_manager.rs`,
  `runtime_capabilities.rs`, `embedded_workflow_host.rs`,
  session runtime load code, runtime registry integration, and diagnostics
  projection.
- `src/services/managedRuntime/` and Settings/workbench managed binary panels.
- Existing ADRs and READMEs for runtime registry and managed redistributables.

### Standards Compliance Review

Codebase review against
`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`
found these implementation requirements:

- **Backend-owned data:** Settings, workflow, scheduler, diagnostics, and Tauri
  must display or invoke backend-managed binary facts. Frontend services may
  cache backend responses but must not synthesize readiness, selection, or
  install state.
- **Immutable/additive contracts:** New facade DTOs must be additive. Existing
  Tauri commands should remain adapters unless a migration plan and
  compatibility note are added.
- **Correct-by-construction Rust APIs:** New managed binary identifiers,
  categories, selected versions, install roots, activation modes, and resolved
  command requests should use enums/newtypes or validated structs rather than
  raw strings across module boundaries.
- **Typed errors:** New public or cross-crate managed binary APIs should use a
  structured error enum instead of adding more `Result<_, String>` surfaces.
  Existing string-error adapters can remain at Tauri boundaries.
- **Decomposition:** `crates/inference/src/managed_runtime/operations.rs`,
  `crates/inference/src/server.rs`, and
  `crates/pantograph-embedded-runtime/src/runtime_capabilities.rs` are already
  over the 500-line review threshold. Any touched logic must either extract
  focused modules or record why a narrow edit is safe.
- **Async safety:** Download, archive extraction, install finalization, process
  spawn, and diagnostics emission must not hold locks across blocking work.
  New blocking filesystem/archive work in async paths should use an async shell
  with synchronous helpers or `spawn_blocking` where appropriate.
- **Task lifecycle:** Existing Tauri process spawning tracks stdout, stderr,
  and monitor tasks. New runtime/background tasks must also have an owner,
  shutdown behavior, and panic/cancel handling.
- **Path security:** App-data migrations, retained artifacts, staging dirs,
  install dirs, and activation paths must be resolved through centralized path
  helpers and root-containment checks. Renderer/IPC paths are never trusted.
- **Cross-platform:** Platform-specific binary layout, executable naming,
  library path, and activation rules must stay in thin platform modules behind
  shared traits. Business logic must not grow inline `cfg()` branches.
- **Dependency ownership:** Prefer existing crates and standard library helpers.
  New dependencies require a written justification and must be declared at the
  narrowest owning crate.
- **Testing:** Cross-layer changes require at least one acceptance path from
  backend binary state through Tauri/frontend projection or workflow execution.
  Tests that touch durable state must use isolated temp roots and cover
  replay/recovery/idempotency for interrupted jobs and migrations.

### Affected Structured Contracts

- Managed binary facade DTOs for id, category, install/readiness state,
  selected/default/active version, install root, expected files, missing files,
  resolved command or activation support, active job, and unavailable reason.
- Managed runtime DTOs:
  `ManagedRuntimeSnapshot`, `ManagedRuntimeVersionStatus`,
  `ManagedRuntimeSelectionState`, `ResolvedCommand`, and job/history entries.
- Managed redistributable DTOs for media dependency status, activation state,
  and leases.
- Workflow runtime capability DTOs:
  `WorkflowRuntimeCapability`, runtime readiness/install state, selected
  version, missing files, and unavailable reason.
- Scheduler diagnostics event payloads for model lifecycle and runtime
  lifecycle phases.
- Frontend TypeScript mirrors in `src/services/managedRuntime/types.ts` and
  workflow diagnostics types.

### Affected Persisted Artifacts

- Managed runtime state under Pantograph app data.
- Managed redistributable state under Pantograph app data.
- Installed runtime and dependency directories, including possible legacy
  `runtimes/` and `managed-dependencies/` paths and proposed
  `third-party/...` paths.
- llama.cpp PID records and runtime diagnostics ledger snapshots.

### Concurrency and Lifecycle Review

- Managed runtime install/remove transitions are serialized per runtime id
  today; the unified facade must preserve that locking and add equivalent
  category-appropriate serialization for media redistributable install,
  activation, lease, and removal transitions.
- Workflow submission must not race an active install/remove and mark a runtime
  ready from stale filesystem state.
- Runtime launch must preserve ownership of stdout/stderr reader tasks, process
  monitor tasks, PID files, and cleanup on failure/cancel.
- Scheduler model lifecycle events must be emitted in dependency order and
  terminal failures must close or fail the workflow run rather than leaving it
  stuck in `running`.
- Diagnostics refresh may overlap runtime startup; refresh must display current
  backend facts without mutating readiness state.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Legacy app-data paths hide already installed runtimes | High | Add path migration/fallback tests before changing launch or status behavior. |
| A unified facade could blur runtime sidecars and media tools | Medium | Model category explicitly and keep category-specific validation behind backend adapters. |
| Scheduler events may currently be emitted from admission success rather than real runtime readiness | High | Split admission, process spawn, HTTP ready, model active, and completed events with tests asserting no completed event before model-active proof. |
| llama.cpp `/health` may not prove requested model identity | High | Verify active backend mode/model path from `LlamaServer` state and add a stronger endpoint probe if available. |
| Install/remove races with workflow launch | High | Block or fail launch with explicit `RuntimeNotReady` when selected runtime has an active mutating job. |
| Existing Tauri/frontend consumers rely on current DTO shapes | Medium | Keep fields additive and preserve command names while moving decision logic behind backend-owned facade. |
| Dirty unrelated source files exist before implementation | Medium | Do not start implementation until those files are committed, stashed, or explicitly assigned. |
| Standards drift during broad refactor | High | Add standards checks to every milestone and stop implementation when decomposition, typed-error, path-safety, async-task, or contract-additivity requirements are not met. |

## Clarifying Questions (Only If Needed)

- None required before plan creation.

## Definition of Done

- A single backend-owned managed binary facade can answer status/readiness for
  runtime sidecars, media tool binaries, and native redistributable artifacts
  without duplicating truth in Tauri, workflow, scheduler, conversion, or
  frontend code.
- llama.cpp workflow runs fail early with explicit missing/invalid managed
  runtime errors when the selected binary cannot launch.
- Scheduler `model load completed` is emitted only after the requested
  runtime/model has passed real readiness checks.
- Failed runtime startup transitions the workflow run out of `running` and
  records a useful diagnostics failure.
- Settings, scheduler diagnostics, runtime capabilities, and process launch
  show consistent selected version, install root, missing files, and
  unavailable reason.
- Affected Rust and frontend tests pass, and the release build succeeds.
- Implementation satisfies the standards compliance review above: additive
  contracts, typed new APIs, isolated durable-state tests, path-safe migration,
  tracked task ownership, and documented decomposition decisions.

## Milestones

### Milestone 1: Establish Current Failure Boundary

**Goal:** Reproduce or isolate where the false model-load-complete event is
emitted relative to managed binary resolution and llama.cpp startup.

**Tasks:**
- [ ] Inspect current app-data managed runtime state and selected llama.cpp
      version for legacy/current path mismatch.
- [x] Inspect managed redistributable state for `ffmpeg`, `ocioconvert`,
      `oiiotool`, and OpenColorIO/OCIO so the facade design covers all
      binary categories.
- [x] Trace workflow run events from admission through
      `load_session_runtime`, gateway start, `LlamaServer::wait_for_ready`,
      scheduler diagnostics emission, and terminal run state.
- [ ] Add a failing regression that proves `model load completed` can be
      emitted without requested model readiness.
- [x] Record whether the active failure is launch resolution, install
      validation, model path resolution, process startup, readiness probing, or
      diagnostics wording.
- [x] Record decomposition candidates and standards exceptions before editing
      files that already exceed the 500-line review threshold.

**Verification:**
- Focused Rust test reproducing false completion or asserting the missing guard.
- Manual diagnostic note in this plan's execution notes with observed selected
  version/install root and emitted event sequence.
- Standards note covering dirty-worktree ownership, decomposition targets, and
  whether any new dependencies are needed.

**Status:** In progress.

### Milestone 2: Introduce Managed Binary Facade

**Goal:** Provide one backend-owned query/mutation surface for all managed
binaries while preserving runtime-vs-media categories.

**Tasks:**
- [x] Add a category-aware managed binary facade in `crates/inference` that
      aggregates runtime sidecars and media redistributables without merging
      their category-specific validation internals.
- [x] Include id, category, display name, install/readiness state,
      selected/default/active version, install root, expected files, missing
      files, active job, unavailable reason, source/checksum metadata when
      present, resolved command support for runtime/tool binaries, and
      activation support for native artifacts where applicable.
- [x] Use correct-by-construction enums/newtypes for binary category, ids,
      activation mode, selected version, install root, readiness state, and
      resolved command requests.
- [x] Add a structured managed binary error enum and keep string conversion at
      adapter boundaries.
- [x] Preserve existing runtime and media command functions as adapters over
      the facade.
- [ ] Add path migration/fallback behavior or an explicit one-time migration
      for legacy app-data paths.
- [x] Extract facade modules instead of expanding existing over-threshold
      operations files.

**Verification:**
- `cargo test -p inference managed_runtime`
- `cargo test -p inference managed_redistributables`
- New facade tests covering runtime sidecar, media tool, legacy path fallback,
  active mutating job, and partial install.
- Contract tests proving additive serialization for runtime sidecars, media
  tools, and native artifact categories.

**Status:** In progress.

### Milestone 3: Route Launch and Admission Through the Facade

**Goal:** Ensure workflow admission, runtime capability projection, and Tauri
process spawning consume the same managed binary facts.

**Tasks:**
- [ ] Update `TauriProcessSpawner` to request resolved sidecar commands from
      the facade and include selected version/install root in errors.
- [ ] Update embedded runtime capability projection to consume facade status
      rather than independent managed runtime snapshots where that causes
      drift.
- [ ] Reject workflow runtime loading when the selected managed runtime is
      missing, failed, partial, unsupported, or actively mutating.
- [ ] Preserve explicit system-command precedence for definitions like Ollama
      where backend definitions declare it.
- [ ] Preserve task ownership for process stdout/stderr/monitor tasks and add
      lifecycle tests for cancellation or spawn failure where behavior changes.
- [ ] Ensure any blocking filesystem/archive or command-resolution work added
      to async paths is isolated behind sync helpers or `spawn_blocking`.

**Verification:**
- Rust tests for missing llama.cpp binary, partial install, selected version
  drift, system Ollama precedence, and active install/remove rejection.
- Existing runtime-registry and session-runtime tests.
- Cross-layer acceptance test or smoke path proving backend managed binary
  state reaches Tauri launch or workflow admission unchanged.

**Status:** Not started.

### Milestone 4: Make Model Readiness Explicit

**Goal:** Tie scheduler model lifecycle events to actual requested model
readiness instead of admission or process spawn success.

**Tasks:**
- [ ] Split lifecycle events into dependency resolved, process spawning,
      process spawned, HTTP ready, requested model active, model load complete,
      and model load failed.
- [ ] Extend llama.cpp backend/server state so readiness checks can assert the
      active model path and mode match the requested run.
- [ ] Ensure reused runtimes validate model path, mmproj path, mode, device,
      and port before emitting model load complete.
- [ ] Make startup failures mark the scheduler/run terminally failed with the
      managed binary or llama.cpp error.
- [ ] Keep readiness state changes in one owner module so scheduler,
      diagnostics, and runtime registry do not each run independent lifecycle
      state machines.

**Verification:**
- Regression that a 17 ms admission path cannot emit `model load completed`
  before model-active proof.
- Regression that a reused llama.cpp process for a different model is not
  treated as loaded.
- Regression that process spawn/HTTP failure moves the workflow run out of
  `running`.
- Replay/recovery test for interrupted runtime startup or app restart during a
  managed binary transition.

**Status:** Not started.

### Milestone 5: Align UI, Diagnostics, and Documentation

**Goal:** Make user-facing state consistent across Settings, scheduler,
diagnostics, and release artifacts.

**Tasks:**
- [ ] Update TypeScript DTO mirrors and managed runtime service projections for
      the unified backend facade.
- [ ] Update Settings to show runtime sidecars, media tools, and native
      artifacts from the same source while preserving category labels/actions.
- [ ] Update scheduler diagnostics wording so "model loaded" is reserved for
      proven model readiness.
- [ ] Update touched READMEs and ADR references for facade ownership and
      lifecycle semantics.
- [ ] Preserve backend-owned data rules in the frontend: no optimistic install,
      selection, activation, or readiness updates.

**Verification:**
- `npm run -w frontend check:types`
- `npm run -w frontend test:run`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `bash launcher.sh --build-release`
- Documentation review that touched source directories still have meaningful
  README/API consumer contract updates.

**Status:** Not started.

## Execution Notes

- 2026-04-30: Plan created after workflow save/submit fixes were committed.
- 2026-04-30: Initial code review found runtime sidecar management in
  `crates/inference/src/managed_runtime`, media redistributables in
  `crates/inference/src/managed_redistributables`, Tauri launch resolution in
  `src-tauri/src/llm/process_tauri.rs`, and workflow runtime capability
  projection in `crates/pantograph-embedded-runtime/src/runtime_capabilities.rs`.
- 2026-04-30: Worktree still contains unrelated dirty source and asset files.
  Implementation must not begin until ownership is resolved per the plan
  standards.
- 2026-04-30: Follow-up standards review added explicit compliance gates for
  additive contracts, typed Rust APIs, structured errors, decomposition,
  async/task lifecycle, path safety, cross-platform adapters, dependency
  ownership, and isolated/recovery tests.
- 2026-05-01: User instructed implementation to begin while unrelated dirty
  worktree files remain. Implementation will avoid those files unless a plan
  task explicitly requires them and will keep commits path-scoped.
- 2026-05-01: Milestone 1 trace found the false `model load completed` wording
  is emitted in `workflow/session_execution_api.rs` immediately after
  `ensure_session_runtime_loaded` returns. The host load path calls
  `ensure_workflow_runtime_ready_for_session_load`,
  `ensure_workflow_inference_model_loaded`, and then runtime reservation, so
  the remaining defect boundary is either model-path resolution bypass,
  gateway readiness semantics, or diagnostics treating host-load success as
  final model residency. A failing regression still needs to lock this down.
- 2026-05-01: Managed redistributable review found `ffmpeg`,
  `ocioconvert`, `oiiotool`, and OpenColorIO/OCIO already modeled in
  `managed_redistributables` with distinct tool-binary vs native-artifact
  categories, but no shared facade with runtime sidecars.
- 2026-05-01: Added additive `crates/inference::managed_binaries` facade with
  typed keys/categories/action support, structured facade errors, runtime
  sidecar projection, media tool projection, native artifact projection, and
  focused category/source tests. Existing runtime and redistributable functions
  remain unchanged adapters for compatibility.
- 2026-05-01: Decomposition decision: the first facade slice added a new
  focused `managed_binaries.rs` module instead of expanding over-threshold
  files (`managed_runtime/operations.rs`, `server.rs`, and
  `runtime_capabilities.rs`). No new dependencies were added.

## Commit Cadence Notes

- Commit after each verified milestone or after each compile-unblocking slice
  when the slice is independent.
- Keep facade contract changes, launch/admission changes, diagnostics wording,
  and frontend projection changes in separate commits unless a smaller atomic
  boundary is clearer.
- Follow `COMMIT-STANDARDS.md`; do not include command output in commit
  messages.

## Optional Subagent Assignment

No parallel workers are assigned for the first implementation pass. If the
facade work grows, split only after Milestone 1 identifies non-overlapping
write sets:

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| TBD | `crates/inference` facade and tests | Backend contract patch plus verification notes | After Milestone 2 tests pass |
| TBD | Workflow/runtime diagnostics consumers | Admission and event semantics patch plus verification notes | After facade contract is stable |
| TBD | Frontend Settings/diagnostics projection | TypeScript/UI patch plus verification notes | After backend DTOs are stable |

## Re-Plan Triggers

- The 17 ms event is proven to come from Puma-Lib model path resolution rather
  than managed binary/runtime launch.
- Existing app-data path migration cannot be made backwards-compatible.
- llama.cpp lacks a reliable way to prove active model identity beyond local
  server state.
- A required fix would break existing Tauri command contracts or persisted
  state without migration.
- Dirty pre-existing source changes overlap required implementation files and
  cannot be assigned cleanly.
- Standards compliance review finds a required architecture change that cannot
  fit the planned facade without breaking existing contracts.

## Recommendations

- Prefer an additive facade over renaming existing commands first. This lowers
  risk by allowing current Settings and workflow code to migrate one consumer
  at a time.
- Treat "model load complete" as a terminal readiness assertion, not a generic
  admission success label. This makes diagnostics useful when runtime startup
  fails.
- Keep runtime sidecar readiness, media tool readiness, and native artifact
  activation readiness in separate category adapters under one facade instead
  of forcing all binaries into one runtime-shaped DTO.

## Completion Summary

### Completed

- Not started.

### Deviations

- None.

### Follow-Ups

- None yet.

### Verification Summary

- Plan creation only. No implementation verification has run for this plan.

### Traceability Links

- Module README updated: `docs/plans/managed-binary-runtime-readiness/README.md`
- ADR added/updated: N/A for plan creation; ADR-003 is the current ownership
  authority.
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A until
  implementation begins.
