# Plan: Pantograph Runtime Redistributables Manager

## Source Of Truth Status

This is the active source of truth for Pantograph runtime redistributable
management work.

It supersedes the older
`IMPLEMENTATION-PLAN-managed-binary-cross-platform.md` plan because the
implementation scope is broader and now includes backend-owned persistent job
state, selected-version policy, workflow/scheduler readiness enforcement, and
required standards-compliance refactors in the immediate touched code.

## Objective

Build a backend-owned redistributables manager for Pantograph that can discover,
install, resume, pause, validate, select, and activate runtime binaries such
as `llama.cpp`, while enforcing workflow-safety gates so workflows never start
against a missing, partially installed, or not-yet-ready runtime.

## Scope

### In Scope

- Backend-owned runtime redistributable catalog, install-state, and selection
  contracts in Rust crates
- Persistent install/download job lifecycle with resume-safe state recovery
- Version-aware managed runtime support for `llama.cpp` first, with the design
  remaining reusable for additional redistributables
- Runtime readiness and activation checks before a version becomes selectable
- Workflow/runtime preflight integration so execution refuses unavailable or
  non-ready redistributables
- Thin Tauri transport commands and GUI-facing status projection only
- Standards-compliance refactors in immediate touched areas, especially where
  core logic currently sits too close to Tauri or where state ownership is
  split across layers
- Documentation and traceability updates for touched source directories and
  runtime-management architecture decisions

### Out of Scope

- New inference features unrelated to redistributable lifecycle management
- Full binding-platform rollout for Python, C#, or BEAM
- General package-manager features for arbitrary third-party tools outside
  Pantograph runtime dependencies
- Broad codebase-wide standards cleanup outside the files and directories
  directly touched by this implementation

## Inputs

### Problem

Pantograph currently has only a partial managed-binary system. It can detect
and install some runtimes, but it does not yet own version catalogs, selected
version policy, persistent install history, resume/pause lifecycle, strong
readiness checks, or workflow admission safety. The visible failure mode is the
current `llama.cpp` startup error when the expected binary is missing, but the
larger gap is architectural: runtime redistributable state is not yet modeled
as a backend-owned, executable contract that the workflow engine, scheduler,
and GUI can all trust.

### Constraints

- Follow `PLAN-STANDARDS.md`
- Follow `ARCHITECTURE-PATTERNS.md`
- Follow `CODING-STANDARDS.md`
- Follow `CONCURRENCY-STANDARDS.md`
- Follow `CROSS-PLATFORM-STANDARDS.md`
- Follow `DOCUMENTATION-STANDARDS.md`
- Follow `TESTING-STANDARDS.md`
- Core business logic must live in backend Rust crates, not in Tauri
- Tauri remains a transport/composition layer plus optional GUI adapter only
- Backend-owned data remains authoritative; the GUI displays projected state and
  sends commands back to the backend
- Supported required platforms remain Linux x86_64 and Windows x86_64, with
  macOS remaining best-effort
- Public command facades should be preserved where practical and only expanded
  additively

### Assumptions

- Vendor-provided redistributables continue to come from stable release sources
  such as GitHub releases
- `llama.cpp` is the first required managed redistributable and is the highest
  priority execution blocker
- Pantograph will eventually need the same system for additional host/runtime
  artifacts beyond `llama.cpp`
- Existing runtime-registry and workflow-preflight contracts are the correct
  integration point for execution safety
- The GUI needs runtime-management visibility, but the backend must own job
  state, selected-version state, and readiness decisions

### Dependencies

- `crates/inference/src/managed_runtime`
- `crates/pantograph-workflow-service`
- `crates/pantograph-embedded-runtime`
- `src-tauri/src/llm/commands`
- Existing runtime-registry ownership and workflow-preflight contracts
- Launcher/release workflows for installation, smoke, and release verification
- Directory READMEs and ADR traceability for touched source boundaries

### Immediate Targeted Codebase Findings

- `crates/inference/src/managed_runtime/mod.rs` is currently 676 lines and owns
  too many responsibilities at once: contracts, platform definition lookup,
  install validation, download/extract flow, transition coordination, and
  command resolution. Implementing more redistributable behavior directly in
  this file would violate the file-size and decomposition-review standards.
- `crates/inference/src/managed_runtime/` does not currently contain the
  required `README.md` for a source directory. The first implementation slice
  that touches this directory must add a standards-compliant README.
- `src-tauri/src/llm/commands/README.md` currently contains banned placeholder
  language such as “Source file used by modules in this directory.” Any work in
  this command boundary must replace that README with concrete adapter-boundary
  rationale before or alongside code changes.
- `crates/pantograph-workflow-service/src/workflow.rs` is currently 7662 lines.
  The runtime-readiness and preflight additions required by this plan must be
  extracted into focused helper modules under `workflow/` rather than added
  inline to the existing facade.
- `src-tauri/src/llm/commands/binary.rs` is currently thin enough in size, but
  the plan must keep it thin in responsibility as new runtime-manager commands
  are added.
- `src/components/BackendSelector.svelte` is currently 513 lines and mixes
  backend switching with managed-runtime lifecycle UI. The missing version-aware
  runtime-manager surface should not be implemented by expanding this mixed
  component further.
- `src/components/BinaryDownloader.svelte` is currently 383 lines, hardcodes
  `llama_cpp`, and is not mounted in the current Settings flow. It contains
  useful runtime-manager presentation logic, but it is not yet the standards-
  compliant mounted runtime-manager surface.
- `src/components/ServerStatus.svelte` is currently 438 lines. Expanding it
  further to absorb the missing runtime-manager detail view would violate the
  decomposition guidance for large UI components.
- `src/services/managedRuntime/README.md` exists, but it does not yet satisfy
  the required section set from `DOCUMENTATION-STANDARDS.md`. Any slice that
  touches this directory must bring that README to full compliance instead of
  extending a partial description.
- `src/features/llm/index.ts` currently exports both the mounted compact
  `BackendSelector` path and the unmounted `BinaryDownloader` path. The plan
  must reconcile those duplicate entrypoints so the runtime-manager UI has one
  clear mounted source of truth inside the existing Settings GUI.

### Affected Structured Contracts

- Managed runtime capability/status payloads
- Runtime catalog/version descriptors
- Install/download job state and progress payloads
- Selected/active/default runtime-version policy payloads
- Workflow runtime-capability and preflight issue payloads
- GUI-facing runtime manager view-model payloads transported through Tauri

### Affected Persisted Artifacts

- App-owned runtime install directories under the managed runtime root
- Persistent runtime catalog snapshot or manifest cache, if used
- Persistent install/download job state for recovery and history
- Persistent selected-version metadata per managed runtime
- Optional install history/audit records if the design needs durable operator
  visibility

### Concurrency / Race-Risk Review

- Install, remove, activate, and workflow-launch actions can overlap for the
  same runtime; a single backend owner must serialize transitions per runtime
  and per selected version
- Resume/pause/cancel recovery must not let two workers claim the same durable
  job record
- Workflow preflight must observe committed backend runtime state rather than a
  stale GUI projection
- Readiness checks and activation state must not race with partial file writes
  or background extraction
- Background job polling, retry, and cleanup ownership must be explicit:
  backend service starts/stops workers, Tauri only subscribes to projected
  state and does not own timers or job mutation

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Runtime-management logic drifts into Tauri command handlers | High | Move all policy, persistence, job ownership, and readiness logic into backend crates before adding new GUI surfaces |
| Version selection becomes UI-owned instead of backend-owned | High | Freeze executable backend contracts for selected/active/default version state before GUI work |
| Install/start overlap leaves a runtime half-installed but selectable | High | Add backend-owned per-runtime transition coordination and explicit readiness phases |
| Cross-platform archive handling contaminates business logic | High | Keep platform-specific fetch/extract/install behavior in thin per-platform adapters selected behind backend contracts |
| Durable job recovery becomes flaky under crashes or restarts | High | Persist job intent/state transitions explicitly and validate interrupted jobs on startup before accepting new work |
| Workflow execution bypasses redistributable readiness checks | High | Route workflow preflight and runtime launch through the same backend capability/readiness contract |
| Touched directories drift further from documentation standards | Medium | Update module READMEs and, where the architecture boundary changes materially, add or update an ADR in the same slices |

## Standards Planning Passes

### Pass 1: Planning And Architecture Standards

Checked against:
- `PLAN-STANDARDS.md`
- `ARCHITECTURE-PATTERNS.md`
- `CODING-STANDARDS.md`

Findings applied to the plan:
- The implementation must preserve backend ownership of persistent state,
  readiness policy, and workflow-safety decisions.
- Tauri must stay a presentation/transport layer and must not own retry loops,
  install-job state, selected-version policy, or readiness decisions.
- The plan must include explicit affected structured contracts, persisted
  artifacts, concurrency review, and public-facade preservation.
- The implementation must include immediate decomposition refactors for
  `managed_runtime/mod.rs` and runtime-readiness work in `workflow.rs`.

Resulting plan corrections:
- Milestone 1 now treats contract freezing and ownership boundaries as the
  first slice, not an incidental byproduct of later work.
- Milestone 4 requires helper-module extraction in
  `crates/pantograph-workflow-service/src/workflow/` instead of expanding the
  oversized facade.

### Pass 2: Documentation And Source-Tree Standards

Checked against:
- `DOCUMENTATION-STANDARDS.md`
- `CODING-STANDARDS.md`

Findings applied to the plan:
- `crates/inference/src/managed_runtime/` is missing its required README.
- `src-tauri/src/llm/commands/README.md` is non-compliant because it contains
  banned placeholder descriptions and insufficient module-specific rationale.
- Touched source directories must receive README updates in the same slices as
  architectural changes, or an ADR must be updated when the boundary shifts
  materially.

Resulting plan corrections:
- Milestone 1 now includes README/ADR traceability as first-class work.
- Milestone 5 explicitly includes replacing the command-boundary README with a
  concrete adapter-only contract description.

### Pass 3: Concurrency, Security, And Cross-Platform Standards

Checked against:
- `CONCURRENCY-STANDARDS.md`
- `SECURITY-STANDARDS.md`
- `CROSS-PLATFORM-STANDARDS.md`

Findings applied to the plan:
- Install/remove/select/start flows need one backend owner and explicit
  transition coordination per runtime/version.
- Archive extraction and path handling must validate destination roots
  centrally; inline path checks scattered through install code would not be
  compliant.
- Platform-specific archive/download/install behavior must remain in thin
  adapter modules instead of spreading `cfg` or platform conditionals through
  business logic.
- Readiness cannot be inferred from file presence alone; activation must happen
  only after validation completes.

Resulting plan corrections:
- Milestone 2 now freezes durable job states and startup reconciliation before
  GUI work.
- Milestone 3 now explicitly requires root-safe extraction/validation helpers
  and thin per-platform runtime adapters.
- Milestone 4 keeps scheduler and restore paths on the same backend readiness
  contract so recovery does not bypass safety checks.

### Pass 4: Testing, Tooling, Dependency, And Frontend Standards

Checked against:
- `TESTING-STANDARDS.md`
- `TOOLING-STANDARDS.md`
- `DEPENDENCY-STANDARDS.md`
- `FRONTEND-STANDARDS.md`

Findings applied to the plan:
- This feature needs replay/recovery/idempotency coverage because interrupted
  install jobs and workflow restarts are core behavior, not edge cases.
- GUI state must remain backend-owned; no optimistic “installed” or “ready”
  projection is allowed before backend confirmation.
- GUI synchronization should prefer backend events/subscriptions over
  high-frequency polling loops.
- Any new dependencies added for download/catalog state must be justified at
  the crate ownership boundary and kept narrow, especially in reusable Rust
  crates.

Resulting plan corrections:
- Milestones 2 and 4 now require recovery/restart verification, not only happy
  path checks.
- Milestone 5 now explicitly restricts the GUI to view/state projection and
  bars UI-owned lifecycle state.
- The plan now assumes dependency additions must be minimal and justified in
  the backend crate that truly owns them.

### Pass 5: Mounted GUI Reality, Accessibility, And Source-Of-Truth Reconciliation

Checked against:
- `ACCESSIBILITY-STANDARDS.md`
- `DOCUMENTATION-STANDARDS.md`
- `IMPLEMENTATION-PLAN-managed-binary-cross-platform.md`
- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`

Findings applied to the plan:
- The active plan and roadmap had drifted ahead of the mounted GUI state. The
  backend-owned runtime-manager contracts, service boundary, and richer
  presentation logic existed before the mounted Settings surface caught up.
- The current mounted/unmounted split creates duplicate frontend entrypoints
  for the same runtime-manager responsibility. Leaving both paths in place as
  peers would violate the single-owner intent for UI state orchestration and
  make the Settings surface harder to reason about.
- The missing mounted runtime-manager view will introduce more interactive
  controls for version selection and job actions. The plan therefore has to
  require semantic buttons, labels, and keyboard-reachable control flow instead
  of allowing click-only generic elements.
- The existing `src/services/managedRuntime/README.md` is still incomplete
  relative to the directory documentation standard, so frontend-adjacent
  documentation cleanup remains an active part of the implementation work.

Resulting plan corrections:
- Milestone 5 remains in progress and explicitly included mounting a dedicated
  version-aware runtime-manager view inside the existing Settings GUI.
- Milestone 5 now also explicitly includes decomposition refactors for the
  immediate touched frontend files so the version-aware screen lands in a
  standards-compliant shape instead of growing already-large components.
- The roadmap and this plan must describe the GUI accurately as implementation
  proceeds: the mounted runtime-manager view is now landed, while deeper
  frontend decomposition remains open.

## Definition of Done

- Pantograph has a backend-owned runtime redistributables manager with frozen
  executable contracts for catalog, install state, job state, version
  selection, and readiness
- `llama.cpp` installs are version-aware, durable across restart, and blocked
  from activation until readiness checks pass
- Workflow preflight and execution use the backend readiness contract and fail
  early with explicit operator-facing diagnostics when a required runtime is not
  ready
- Tauri commands remain thin and do not own runtime lifecycle policy,
  persistent state, or background workers
- The existing Settings GUI contains one mounted version-aware runtime-manager
  surface that renders available/installed versions, selected/default/active
  state, readiness, job progress, retained-artifact state, and install history
  from the backend-owned managed-runtime contract
- Immediate touched files/directories are refactored into standards-compliant
  shape, including README or ADR traceability where required
- Linux x86_64 and Windows x86_64 verification exist for the new backend-owned
  path; macOS best-effort behavior degrades explicitly

## Milestones

### Milestone 1: Freeze Backend Contracts And Ownership

**Goal:** Define the backend-owned redistributable-management boundary before
behavior expands.

**Tasks:**
- [ ] Create a dedicated backend contract surface for runtime catalog entries,
  install states, readiness states, job states, and version-selection policy
- [ ] Decompose `crates/inference/src/managed_runtime/mod.rs` into focused
  modules so contract types, service/orchestration logic, persistence, and
  command-resolution helpers no longer accumulate in one oversized file
- [ ] Refactor existing managed-runtime contracts so `src-tauri` consumes them
  instead of defining policy locally
- [ ] Add an ownership/lifecycle note for background workers, recovery passes,
  and per-runtime transition coordination
- [ ] Review immediate touched files for decomposition and lock-choice
  compliance; split oversized or mixed-responsibility modules as needed
- [ ] Add `crates/inference/src/managed_runtime/README.md` with full required
  sections and runtime-manager contract rationale
- [ ] Record traceability updates required for touched directories and runtime
  architecture

**Verification:**
- `cargo check --workspace`
- Targeted contract serialization/deserialization tests for new payloads
- Review against `ARCHITECTURE-PATTERNS.md`, `CODING-STANDARDS.md`, and
  `CONCURRENCY-STANDARDS.md`

**Status:** Complete

### Milestone 2: Persistent Catalog, Selection, And Job State

**Goal:** Introduce durable backend state for runtime versions, selected
version policy, and install/download jobs.

**Tasks:**
- [ ] Add a backend-owned persisted state module for installed versions,
  selected/default/active version state, and install history
- [ ] Add durable job records that support queued, downloading, paused,
  extracting, validating, ready, failed, and canceled states
- [ ] Add startup reconciliation for interrupted jobs and stale in-progress
  state
- [ ] Centralize path and extraction-root validation for install artifacts so
  download/extract code does not perform ad hoc inline path checks
- [ ] Keep persisted-artifact and state-store ownership out of Tauri; Tauri
  consumes projected state only
- [ ] Update immediate touched README files to document persisted artifacts,
  invariants, and recovery expectations

**Verification:**
- Targeted persistence/recovery integration tests with isolated temp state
  roots per `TESTING-STANDARDS.md`
- Re-run recovery tests to prove restart safety and absence of shared-state
  leakage
- `cargo check --workspace`

**Status:** Complete

### Milestone 3: `llama.cpp` Versioned Install And Readiness Pipeline

**Goal:** Land the first full runtime on the new backend-owned system.

**Tasks:**
- [ ] Refactor `llama.cpp` install/download/extract/validate behavior onto the
  frozen redistributable contracts
- [ ] Add thin per-platform adapters for release discovery, archive handling,
  executable validation, and runtime-specific readiness checks
- [ ] Keep one platform per file and isolate all runtime/platform differences
  behind the adapter/factory boundary
- [ ] Separate install completion from activation; a version is only selectable
  after validation passes
- [ ] Add explicit compatibility metadata required by execution paths, such as
  runtime identifier, platform, version, and executable readiness
- [ ] Refactor immediate touched modules if the current `managed_runtime`
  directory exceeds standards-compliant responsibility boundaries

**Verification:**
- Targeted backend tests for install state progression, validation failures,
  and selection rules
- Linux x86_64 and Windows x86_64 path verification per
  `CROSS-PLATFORM-STANDARDS.md`
- `cargo check --workspace`

**Status:** Complete

### Milestone 4: Workflow And Scheduler Safety Integration

**Goal:** Ensure redistributable readiness is enforced by the execution system,
not only displayed in the GUI.

**Tasks:**
- [x] Extract runtime-readiness/preflight helpers from
  `crates/pantograph-workflow-service/src/workflow.rs` into focused modules
  under `crates/pantograph-workflow-service/src/workflow/` before adding
  substantial new redistributable policy there
- [x] Extend workflow runtime capability and preflight projection to expose
  selected version, readiness phase, and actionable failure reasons
- [x] Block workflow start, resume, and restore paths when required
  redistributables are missing, paused, failed, or not ready
- [x] Ensure scheduler/runtime restore paths observe the same backend
  redistributable readiness contract instead of bypassing it
- [x] Add replay/recovery coverage for workflows encountering interrupted
  runtime installs or post-restart validation failures
- [x] Refactor immediate touched workflow-service or embedded-runtime modules
  to keep state-machine ownership single-owner and backend-owned

**Verification:**
- Cross-layer acceptance tests exercising runtime state -> preflight ->
  execution refusal/allowance
- Replay/recovery/idempotency tests for restart and interrupted install cases
- `cargo check --workspace`

**Status:** Completed

### Milestone 5: Thin Tauri Commands And Runtime Manager View Contract

**Goal:** Expose the backend-owned system cleanly to the GUI without moving
core logic into Tauri.

**Tasks:**
- [x] Replace command-level binary-specific logic with thin transport commands
  that call backend-owned services for list, install, pause, resume, cancel,
  remove, select-version, and inspect-history
- [x] Define a GUI view contract for available versions, installed versions,
  selected/default/active status, job progress, readiness, and error state
- [x] Ensure the GUI can render resumable/pausable download progress,
  retained-artifact state, and install history without becoming the state owner
- [x] Prefer backend event/subscription projection for job progress and state
  changes; if polling remains necessary anywhere, scope it narrowly and
  document why an event-driven path was not feasible
- [x] Ensure workflows surface readiness failures and install progress through
  existing diagnostics/event boundaries rather than ad hoc GUI polling rules
- [x] Replace the non-compliant placeholder content in
  `src-tauri/src/llm/commands/README.md` and update any touched
  frontend-adjacent README files so their adapter-only role remains explicit
- [x] Mount a dedicated version-aware runtime-manager view in the existing
  Settings GUI instead of leaving version-aware controls split between the
  compact `BackendSelector` surface and the unmounted `BinaryDownloader`
  component
- [x] Refactor `BackendSelector.svelte`, `BinaryDownloader.svelte`, and
  `ServerStatus.svelte` into focused components so backend selection, runtime
  manager presentation, and server-shell concerns do not continue to accrete in
  oversized files
- [x] Generalize the richer runtime-manager presentation path so it consumes a
  runtime id or backend-owned runtime view input instead of hardcoding
  `llama_cpp`
- [x] Ensure the mounted runtime-manager surface exposes available versions,
  installed versions, selected/default/active state, retained-artifact actions,
  and install history directly from the backend-owned view contract
- [x] Reconcile duplicate runtime-manager exports and entrypoints after the
  mounted Settings surface becomes authoritative
- [x] Bring `src/services/managedRuntime/README.md` and any newly introduced
  frontend runtime-manager directories to full
  `DOCUMENTATION-STANDARDS.md` compliance
- [x] Ensure runtime-manager controls follow
  `ACCESSIBILITY-STANDARDS.md`, including semantic buttons/selects, accessible
  labels, and keyboard-reachable action flows

**Verification:**
- Targeted command/transport tests for additive facades
- Contract tests for projected view payloads
- Architecture review proving no business logic regressed into Tauri
- Frontend lint/typecheck plus a targeted accessibility review for the mounted
  runtime-manager controls

**Status:** Completed

### Milestone 6: Rollout, Follow-On Runtime Reuse, And Source-Of-Truth Closeout

**Goal:** Finish the redistributable-management slice so it can be reused for
additional runtimes and referenced as the canonical runtime-manager design.

**Tasks:**
- [x] Reconcile existing managed-binary plans/roadmap wording so the new plan
  is the source of truth and stale narrower wording is removed or superseded
- [x] Capture the reuse path for additional redistributables such as Ollama or
  other managed runtime hosts without implementing all of them immediately
- [x] Add launcher/release-smoke integration where required so install/readiness
  failures are visible in bounded verification flows
- [x] Update ADR/README traceability for the final backend-owned runtime
  manager boundary
- [x] Record residual best-effort platform limits explicitly instead of leaving
  them implicit

**Verification:**
- `./launcher.sh --build-release`
- `./launcher.sh --release-smoke` or the closest standards-compliant bounded
  smoke path available for Pantograph
- Source-of-truth review across roadmap, plan, README, and ADR links

**Status:** Complete

## Execution Notes

Update during implementation:
- 2026-04-19: Plan created after comparing the proposed Pantograph runtime
  redistributables system against Pumas Library's stronger version-manager and
  download-management model, then reconciling the needed Pantograph-specific
  workflow-safety requirements against the coding standards.
- 2026-04-19: Plan iterated in four explicit standards-review passes covering
  planning/architecture, documentation/source-tree compliance,
  concurrency/security/cross-platform concerns, and testing/tooling/dependency/
  frontend constraints. Immediate targeted codebase deviations were folded into
  milestone tasks so touched files are driven toward compliance rather than
  merely worked around.
- 2026-04-19: Milestone 1 started with a standards-driven decomposition of
  `crates/inference/src/managed_runtime`, a new directory README, and workspace
  dependency alignment for `parking_lot` so the synchronous transition-lock map
  can move off poison-based `std::sync::Mutex`.
- 2026-04-19: Milestone 1 slice 2 froze additive backend runtime-manager
  contracts for readiness, versions, selection, and job-state projection, and
  introduced snapshot helpers that project the existing managed binary state
  into those contracts without changing current runtime behavior.
- 2026-04-19: Milestone 2 started with a durable backend state store for
  managed runtime selection, versions, interrupted-job reconciliation, and
  install history, and runtime snapshots now merge that persisted state instead
  of remaining purely ephemeral projections.
- 2026-04-19: Milestone 2 slice 2 centralized archive extraction path
  validation in the backend managed runtime archive helpers so zip and tar
  installs share one root-containment check instead of relying on scattered
  archive-specific assumptions.
- 2026-04-19: Milestone 2 slice 3 wired successful and failed install/remove
  transitions into the durable runtime state file so backend snapshots now
  record concrete version, selection, job-failure, and install-history changes
  instead of only exposing additive DTOs.
- 2026-04-19: Milestone 2 slice 4 added explicit backend selection-mutation
  APIs for selected/default version policy, exported them through the public
  inference surface, and covered installed-versus-unknown version behavior so
  host adapters no longer need to mutate durable state directly.
- 2026-04-19: Milestone 3 started by routing managed runtime command
  resolution through persisted selected/default/active version policy and
  recorded install roots, so execution no longer ignores backend-owned version
  selection state once it exists.
- 2026-04-19: Milestone 3 slice 2 moved managed runtime installs onto a
  version-scoped filesystem layout under the runtime root while keeping legacy
  fallback resolution for preexisting single-directory installs, so future
  runtime versions can coexist without collapsing back to one shared install
  path.
- 2026-04-19: Milestone 3 slice 3 split strict execution-time install-root
  resolution from projection-time fallback resolution, so stale selected
  version state now degrades cleanly for capability/snapshot reads without
  weakening launch-time validation.
- 2026-04-19: Milestone 3 slice 4 added explicit backend compatibility
  metadata for runtime key, platform key, install root, executable name, and
  executable readiness across persisted versions and runtime snapshots, so
  later execution and host layers can consume one backend-owned compatibility
  contract instead of re-deriving those facts.
- 2026-04-19: Milestone 3 slice 5 tightened backend selection policy so only
  ready versions can become selected/default targets, closing the remaining
  gap where failed versions still existed in state but should not have been
  selectable.
- 2026-04-19: Milestone 4 started with the embedded-runtime capability bridge
  switching from flat managed-binary capability reads to backend-owned managed
  runtime snapshots, so workflow/runtime capability projection now preserves
  readiness-phase and selected-version context through `configured` and
  `unavailable_reason` instead of rebuilding that context in workflow or Tauri
  layers.
- 2026-04-19: Milestone 4 slice 2 extended the backend-owned
  `WorkflowRuntimeCapability` contract with explicit readiness-phase and
  selected-version fields, and updated workflow preflight/runtime issue
  shaping to consume that contract directly so diagnostics and execution see
  the same selected-version readiness context.
- 2026-04-19: Milestone 4 slice 3 routed keep-alive session creation and
  keep-alive re-enable through the same backend session preflight contract
  before runtime load, so those session-residency entry points no longer
  bypass runtime-readiness enforcement.
- 2026-04-19: Milestone 4 slice 4 exported the workflow-service preflight
  helpers as backend utilities and applied them to embedded-runtime
  host-side `load_session_runtime`, so direct session-load callers now fail
  before reservation/warmup when the selected redistributable is missing or
  not ready. The remaining Milestone 4 restore work is now narrowed to
  replay/recovery-specific paths instead of the generic host load boundary.
- 2026-04-19: Milestone 4 slice 5 added resume/recovery coverage for
  checkpointed keep-alive sessions whose selected redistributable becomes
  unavailable between unload and restore. The resume path now has regression
  coverage proving it returns `RuntimeNotReady`, preserves the checkpoint for
  retry, and resumes successfully after the runtime becomes ready again.
- 2026-04-19: Milestone 4 slice 6 added a backend-owned
  `invalidate_all_session_runtimes` contract in `pantograph-workflow-service`
  and routed embedded-runtime shutdown plus Tauri recovery/window-close stop
  composition through it. Producer stop paths now clear stale
  `runtime_loaded` session state before teardown so the next keep-alive run
  re-enters the shared backend preflight/load boundary instead of trusting
  stale residency state after a stop or restart.
- 2026-04-19: Milestone 4 slice 7 now projects persisted selected-version
  failure state and startup-reconciled interrupted install jobs into the
  backend managed-runtime snapshot, keeps conservative technical-fit fallback
  anchored to the workflow's required backend, and adds restart-path coverage
  proving preflight and workflow run block when `llama.cpp` is failed after
  restart instead of silently falling back to an unrelated ready runtime.
- 2026-04-19: Milestone 5 slice 1 introduced a backend-owned managed-runtime
  manager view contract in `pantograph-embedded-runtime`, moved Tauri
  redistributable commands onto that backend facade for list/install/remove/
  select/default/inspect operations, and replaced the placeholder
  `src-tauri/src/llm/commands/README.md` text with an explicit adapter-boundary
  description. Pause/resume/cancel and event-driven progress follow-up remain
  open inside Milestone 5.
- 2026-04-19: Milestone 5 slice 2 added a dedicated
  `src/services/managedRuntime` frontend service boundary that mirrors the
  backend-owned managed-runtime manager contract, moved current Svelte callers
  off duplicated local runtime DTOs, and documented that the GUI consumes
  backend snapshots/progress without becoming the policy owner.
- 2026-04-19: Milestone 5 slice 3 expanded the current redistributable GUI
  projection to render backend-owned readiness, selected/default version,
  active-job summary, and recent install-history state from the shared
  managed-runtime service. Pause/resume/cancel controls still remain open, but
  the frontend now consumes the richer backend snapshot instead of hiding that
  state behind binary-present/missing booleans.
- 2026-04-19: Milestone 5 slice 4 wired the existing backend-owned
  selected/default version commands into the managed-runtime GUI, so runtime
  version policy can now be updated through the shared frontend service
  boundary without reintroducing version-selection logic into Tauri or Svelte.
- 2026-04-19: Milestone 5 slice 5 added backend-owned managed-runtime install
  cancellation in `inference`, routed the cancel request through
  `pantograph-embedded-runtime`, Tauri command transport, and the shared
  `managedRuntime` frontend service, and surfaced cancel controls in the
  current GUI while leaving pause/resume and broader event-stream behavior as
  the remaining open job-control work.
- 2026-04-19: Milestone 5 slice 6 now persists backend-owned retained
  download-artifact metadata alongside the active managed-runtime job, projects
  that additive state through the embedded-runtime manager view and shared
  frontend service contract, and renders the retained artifact read-only in
  the current GUI. This narrows the remaining Milestone 5 gap to true
  pause/resume orchestration and broader event-driven state delivery rather
  than basic retained-download visibility.
- 2026-04-19: Milestone 5 slice 7 extends the managed-runtime progress stream
  to carry the backend-owned runtime-manager view snapshot alongside byte-level
  progress, so the GUI can reconcile active-job, readiness, retained-artifact,
  and install-history changes directly from the manager stream without falling
  back to ad hoc refresh calls during install/cancel flow. True pause/resume
  orchestration remains open.
- 2026-04-19: Milestone 5 slice 8 adds backend-owned download resume behavior
  for retained managed-runtime artifacts. Reinstall attempts now reuse a
  matching retained archive when present, attempt HTTP range resume against the
  upstream release source, and fall back to a fresh full download when the
  remote server does not honor the range request. The current GUI also now
  surfaces that retained state as an explicit resume action label while pause
  behavior remains the remaining open job-control gap.
- 2026-04-19: Milestone 5 slice 9 splits the stop path into backend-owned
  pause versus destructive cancel semantics. Active downloads can now be
  paused with retained artifacts, resumed through the existing install path,
  or cancelled destructively, including discarding a previously paused
  retained artifact without reintroducing lifecycle policy into Tauri or the
  GUI. The remaining Milestone 5 work was the broader event/diagnostics lane,
  not basic pause/resume/cancel controls.
- 2026-04-19: Milestone 5 slice 10 extends workflow diagnostics/runtime
  snapshot projection to carry backend-owned managed-runtime manager views
  through the existing workflow event and diagnostics store path. Headless
  diagnostics reads and execution-time runtime snapshot emission now publish
  redistributable readiness/install-progress state without inventing a second
  GUI-only polling contract, and targeted diagnostics/adapter tests now pin
  that projection end to end.
- 2026-04-19: Milestone 5 slice 11 promotes the frontend-managed-runtime
  service into a shared snapshot owner that fans backend progress and
  post-action runtime views out to subscribers. The current redistributable
  GUI surfaces now reconcile managed-runtime status through that shared
  service state instead of each maintaining their own install-state updates.
  The remaining Milestone 5 gap is narrower still: only backend capability
  refresh paths that still rely on `list_backends` need audit/documentation or
  a later event-driven replacement.
- 2026-04-19: Milestone 5 slice 12 removes the remaining post-action
  `list_backends` refresh dependency from the current backend selector by
  projecting backend-owned managed-runtime availability/installability fields
  from the shared runtime snapshot cache. Initial catalog load still reads the
  backend list once, but subsequent managed-runtime transitions no longer need
  a GUI-triggered capability polling loop, so the Milestone 5 event-driven
  projection task is now complete for the current redistributable surfaces.
- 2026-04-19: Planning pass 5 reconciled this plan against the historical
  managed-binary plan, the roadmap wording, the mounted Settings UI, and the
  remaining accessibility/documentation standards. That pass corrected a stale
  claim that the version-aware runtime-manager screen was already present in
  the mounted GUI. Milestone 5 remains open until the existing Settings surface
  mounts one authoritative version-aware runtime manager and the immediate
  touched frontend files are decomposed/documented to standards.
- 2026-04-19: Milestone 5 slice 13 mounts the dedicated version-aware runtime
  manager inside the existing Settings sidecar flow, removes the stale
  hardcoded `BinaryDownloader` path, introduces focused `runtime-manager/` and
  `server-status/` component directories, and updates the feature/service
  README set so the mounted GUI now matches the backend-owned runtime-manager
  contract. The remaining Milestone 5 work is narrower frontend decomposition
  of the still-large selector/card shell components rather than missing
  version-aware runtime-manager functionality.
- 2026-04-19: Milestone 5 slice 14 finishes the runtime-manager/frontend
  decomposition pass by splitting backend-option rendering, connection-mode
  tabs, and runtime-card summary/job/catalog/activity blocks into focused
  components. `BackendSelector.svelte`, `ServerStatus.svelte`, and
  `ManagedRuntimeCard.svelte` now sit back at or under the preferred component
  size threshold, so Milestone 5 is functionally complete.
- 2026-04-19: Milestone 2 and 3 close-out now include a backend-owned cached
  release catalog plus version-targeted install resolution. `managed_runtime`
  persists catalog entries and their last refresh timestamp in durable state,
  refreshes vendor release rows through a focused `catalog.rs` boundary, and
  resolves install/download requests against exact catalog archive URLs instead
  of only one baked-in release tag. `ManagedRuntimeSnapshot` version rows now
  merge cached catalog entries with installed versions so later host and GUI
  layers can distinguish installable missing versions from already-installed
  ones without rebuilding release knowledge locally.
- 2026-04-19: The mounted runtime-manager GUI now consumes that backend-owned
  catalog through the shared managed-runtime service, refreshes catalog rows on
  load, and exposes per-version install actions without moving vendor release
  or install-resolution policy into TypeScript. The runtime-manager/service
  README set and roadmap wording are reconciled so the mounted Settings view is
  documented as a real catalog-backed runtime manager rather than a generic
  “version-aware” surface.
- 2026-04-19: Milestone 6 slice 1 finishes the source-of-truth and
  traceability closeout for the current redistributables boundary. The older
  managed-binary plan and roadmap wording now clearly defer to this plan, and
  ADR/README links now point to an accepted redistributables-manager boundary
  record instead of forcing later work to reconstruct the architecture from
  plan prose alone.
- 2026-04-19: Milestone 6 slice 2 records the additive reuse path for future
  managed runtimes in the backend managed-runtime README and Tauri host README.
  New runtime families now have an explicit extension order: backend id and
  contracts first, runtime definition second, platform adapters third, durable
  state and Pantograph-facing view projection next, and only then host
  transport reuse. This keeps future `Ollama` or other runtime work from
  reopening binary-specific Tauri or GUI ownership patterns.
- 2026-04-19: Milestone 6 slice 3 records the remaining managed-runtime
  platform limits explicitly in the backend README. Required confidence remains
  Linux x86_64 plus Windows x86_64, macOS remains best-effort, `llama.cpp`
  unsupported targets must fail as unsupported instead of partially
  installing, and `Ollama` continues to prefer a host-provided system command
  when present.
- 2026-04-19: Milestone 6 slice 4 adds a bounded launcher-integrated
  redistributables smoke path via `./launcher.sh --release-smoke` and
  `scripts/check-runtime-redistributables-smoke.sh`. Because the desktop
  release binary still lacks a true headless smoke entrypoint, the smoke now
  verifies that a built release artifact exists and then runs targeted managed
  runtime manager, workflow preflight, and diagnostics projection tests that
  cover the current redistributables safety boundary.

## Commit Cadence Notes

- Commit when a logical slice is complete and verified.
- Keep code, README, and ADR traceability updates in the same atomic commit
  when they describe the same slice.
- Follow `COMMIT-STANDARDS.md`.

## Re-Plan Triggers

- Runtime selection or readiness requires breaking public transport contracts
  instead of additive extension
- Durable job recovery cannot be implemented without changing persisted-artifact
  format assumptions
- `llama.cpp` platform packaging differences require special-casing that breaks
  the reusable backend runtime-manager core
- Workflow preflight or scheduler restore paths cannot consume the new runtime
  readiness contract without broader runtime-registry redesign
- The GUI requirements force optimistic/UI-owned state instead of backend-owned
  truth

## Recommendations

- Prefer a descriptor-driven runtime definition model early. It keeps runtime
  reuse achievable for future redistributables without baking `llama.cpp`
  assumptions into the core.
- Treat "installed" and "ready/selectable" as separate states. This reduces
  execution risk and aligns the design with the user's requirement that
  workflows never run against incomplete redistributables.
- Keep history and interrupted-job reconciliation in scope now. Those are not
  premature hardening for this feature because the UI and workflow-safety goals
  depend on accurate durable state after interruption.

## Completion Summary

### Completed

- N/A

### Deviations

- N/A

### Follow-Ups

- N/A

### Verification Summary

- N/A

### Traceability Links

- Module README updated: N/A
- ADR added/updated: N/A
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`

## Brevity Note

This plan stays execution-focused while capturing the additional ownership,
durability, and workflow-safety requirements that make this broader than the
older managed-binary plan.
