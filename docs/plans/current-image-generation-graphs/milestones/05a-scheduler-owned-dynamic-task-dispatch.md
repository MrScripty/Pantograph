# Milestone 5a: Scheduler-Owned Dynamic Task Dispatch

**Goal:** Replace execution-time dependency/runtime discovery with
scheduler-owned dynamic task dispatch for ready workflow DAG nodes. Workflows
remain durable runs whose tasks can pause, defer, batch, or execute
independently across concurrent users.

This milestone is inserted after Milestone 5 because the current
device/runtime/dependency contracts are prerequisites. It gates future real
execution slices that would otherwise keep relying on `ModelRefV2` or
`model_path` dependency preflight output.

**Tasks:**

- [x] Establish the scheduler-owned implementation boundary before adding new
  execution behavior. Decide whether an existing crate cleanly owns scheduler
  queue, policy, resource admission, and dispatch contracts or create a focused
  scheduler crate/module. Record allowed write sets and keep shared contracts,
  generated DTOs, fixtures, lockfiles, and plan files under serial integration
  ownership.
- [x] Define a path-free schedulable task intent contract for ready workflow
  nodes. Include workflow/run/node/task correlation, user or fairness identity,
  task type, Pumas model ref, optional hard runtime/device constraints, typed
  trait/settings intent, dependency override intent, and bounded estimate
  hints. Raw graph or IPC input must parse into validated ids, enums, and
  bounded values before internal scheduler policy receives it.
- [x] Define a backend-owned capability hint contract for graph editor and
  option-provider consumers. It must expose possible runtimes/devices, trait
  options, unavailable/not-installed/not-implemented states, and typed
  diagnostics without exposing local paths, load targets, package-manager
  internals, or final scheduler decisions.
- [x] Define the scheduler-owned readiness admission contract before replacing
  node-engine preflight. It must accept validated `SchedulableTaskIntent`,
  apply scheduler dependency policy, and return typed ready/defer/fail
  admission results with dependency readiness proof when ready. It must not
  expose executable Pumas load targets, local paths, `ModelDependencyRequest`,
  or `ModelRefV2`.
- [x] Define the non-legacy runtime handoff seam that runtime/execution hosts
  will consume after readiness admission. The seam must carry correlation ids,
  scheduler-owned readiness proof, dependency environment refs when applicable,
  and later scheduler-selected runtime/device facts. It must not convert
  readiness proof back into `ModelRefV2` or require graph/node-engine access to
  executable load paths.
- [ ] Replace node-engine dependency preflight output with typed readiness
  proof after the readiness admission and runtime handoff seam exists.
  Non-ready states must fail or defer with typed diagnostics. Do not adapt
  readiness proof back into `ModelDependencyRequest` or `ModelRefV2`.
- [ ] Add durable scheduler queue state for task-level workflow progress:
  pending, ready, blocked, waiting for dependency readiness, waiting for
  resources, waiting for batch, running, paused/deferred, retryable failed,
  terminal failed, and completed. Queue transitions must be idempotent and
  replayable from persisted state.
- [ ] Add typed scheduler task lifecycle diagnostics so graph editor and run
  inspection can explain why a task is waiting, deferred, unavailable, failed,
  or completed without frontend inference.
- [ ] Add one scheduler lifecycle owner for long-running queue workers,
  dependency readiness actions, resource observation loops, and runtime host
  dispatch. It must own bounded queues, cancellation, shutdown, retry-loop
  termination, task panic handling, and reservation cleanup.
- [ ] Define the scheduler dispatch decision contract. It must carry
  correlation ids, selected runtime/runtime variant, selected device or device
  set, selected Pumas model/artifact identity, dependency readiness proof,
  environment ref when applicable, batching group id, reservation/lease id,
  runtime trait/options projection, and bounded diagnostics.
- [ ] Add a resource/residency manager abstraction for admission-time resource
  snapshots, per-device reservations, model residency, runtime readiness,
  load/warmup estimates, batching memory impact, and typed unavailable or
  impossible-fit diagnostics. Platform-specific collectors must live behind a
  shared observer trait with thin Linux, Windows, and macOS modules; scheduler
  policy consumes only platform-neutral observations.
- [ ] Move dependency readiness policy into scheduler admission/dispatch.
  Scheduler policy decides check-only, install-missing, defer, retry, or fail;
  node-engine must not perform dependency resolver discovery as an execution
  fallback.
- [ ] Add batching policy surface for compatible tasks across workflow runs.
  Compatibility must account for task family, model/ref identity, runtime,
  device, loaded residency, input shape, memory impact, latency, and fairness.
- [ ] Wire runtime/execution host handoff through dispatch decisions. The host
  may resolve Pumas-approved load targets only at the runtime boundary that
  needs executable facts.
- [ ] Update or add README/crate documentation for every new public contract,
  source directory, lifecycle owner, platform observer, persisted queue state,
  structured fixture, and host-facing API.
- [ ] Delete or replace retired successful paths:
  `ModelDependencyResolver`, `ModelDependencyRequest`, `ModelRefV2`,
  `model_path` dependency identity, frontend `modelPath` dependency actions,
  and saved/mock success fixtures using path-shaped dependency identity.

**Verification:**

- Contract tests and JSON fixtures for capability hints, schedulable task
  intent, queue state, dispatch decisions, and diagnostics.
- Boundary validation tests proving raw graph, IPC, saved-workflow, queue, and
  ledger payloads are parsed once into validated types and reject stringly
  states, wrong ids, path-shaped dependency identity, and invalid resource
  bounds.
- Node-engine tests proving graph inputs produce path-free task intent and fail
  closed when scheduler dispatch/readiness services are missing.
- Scheduler policy tests for explicit runtime hard requirements, omitted
  runtime automatic selection, unavailable/not-implemented diagnostics,
  resource rejection, retry/defer decisions, and batching compatibility.
- Lifecycle/recovery tests for bounded queue shutdown, cancellation,
  duplicate-dispatch prevention, retry/defer idempotency, persisted replay after
  restart, and reservation release on success, failure, and cancellation.
- Resource observer tests using deterministic fakes first, plus platform-module
  compile checks for supported Rust targets and local smoke tests where the
  collector is available.
- Cross-layer acceptance test from graph intent to scheduler dispatch to host
  execution without graph or node-engine seeing local paths or load targets.
- Readiness admission and runtime handoff tests proving the scheduler can
  produce a host-consumable non-legacy handoff before node-engine preflight is
  replaced, and proving no adapter can convert the handoff back to
  `ModelRefV2`.
- Multi-workflow acceptance test with at least two ready tasks from different
  workflow runs, proving the scheduler can defer one workflow while admitting
  another task or batch.
- README/plan updates in the same slice for every new public contract,
  ownership boundary, queue state, persisted artifact, or host-facing API.
- Public crates touched by this milestone must pass default, all-features, and
  no-default-features checks when they expose feature contracts.

**No-Fallback Requirements:**

- Do not keep `ModelRefV2` as the successful dependency preflight output.
- Do not introduce blank/fake/repaired `model_path` values.
- Do not let graph editor, node-engine, runtime adapters, or frontend actions
  make scheduler runtime/device/dependency decisions.
- Do not preserve old resolver calls as alternate successful branches after
  the new scheduler path is wired.
- Do not expose executable Pumas load targets before host/runtime dispatch.
- Do not place scheduler policy in graph editor, frontend adapters,
  node-engine, runtime adapters, or Tauri command handlers.
- Do not scatter platform-specific resource collection through scheduler
  business logic.
- Do not make core scheduler ranking/admission async unless the plan records
  the I/O operation that requires it; use a synchronous core with async shells
  by default.

**Status:**

- [ ] Not started.
- 2026-05-22: Created from the Option 4 re-plan discussion. This milestone
  defines dynamic scheduler-owned task dispatch, not whole-workflow static
  planning.
- 2026-05-22 standards pass: tightened the milestone against plan,
  architecture, concurrency, Rust API, testing, documentation, security, and
  cross-platform standards. Added explicit ownership, validated contract,
  lifecycle, resource observer, recovery/idempotency, README, and no-legacy
  gates before implementation.
- 2026-05-22 boundary crate slice completed. Smallest useful vertical slice:
  create `pantograph-scheduler` as the scheduler-owned boundary before adding
  execution behavior. Allowed write set: root `Cargo.toml`, `Cargo.lock`,
  `crates/README.md`, `crates/pantograph-scheduler/`, and this plan file plus
  execution notes. No-fallback confirmation: this slice does not wire
  `ModelDependencyResolver`, `ModelDependencyRequest`, `ModelRefV2`,
  `model_path`, frontend `modelPath`, or runtime/device/dependency fallback
  paths. Verification passed: `cargo fmt -p pantograph-scheduler`,
  `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
  `cargo check -p pantograph-scheduler --all-features`,
  `cargo check -p pantograph-scheduler --no-default-features`, and
  `cargo fmt -p pantograph-scheduler -- --check`. Remaining follow-up: define
  path-free schedulable task intent contracts in the scheduler crate without
  exposing local paths or executable Pumas load targets.
- 2026-05-22 schedulable task intent contract slice completed. Smallest useful
  vertical slice: add the path-free `SchedulableTaskIntent` DTO and validated
  wrapper inside `pantograph-scheduler` without wiring queue execution.
  Allowed write set: `Cargo.lock`, `crates/pantograph-scheduler/`, this
  milestone file, and execution notes. No-fallback confirmation: the contract
  carries `PumasModelRef`, optional hard runtime/device constraints, typed trait
  settings, dependency override patches, and bounded estimate hints; it rejects
  top-level `model_path` through the serde boundary and does not expose local
  load paths, executable Pumas load targets, `ModelDependencyRequest`,
  `ModelRefV2`, frontend `modelPath`, or worker launch facts. Verification
  passed: `cargo fmt -p pantograph-scheduler`,
  `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
  `cargo check -p pantograph-scheduler --all-features`,
  `cargo check -p pantograph-scheduler --no-default-features`, and
  `cargo fmt -p pantograph-scheduler -- --check`. Remaining follow-up: define
  backend-owned capability hints for graph editor and option-provider
  consumers without exposing final scheduler decisions.
- 2026-05-22 capability hint contract slice completed. Smallest useful
  vertical slice: add backend-owned `SchedulerCapabilityHintSnapshot` contracts
  and validation without wiring frontend or option-provider consumers.
  Allowed write set: `crates/pantograph-scheduler/`, this milestone file, and
  execution notes. No-fallback confirmation: capability hints expose possible
  runtimes, devices, trait options, availability states, and typed diagnostics
  only; they reject final selected runtime fields and executable load-target
  fields, and do not expose local paths, `ModelDependencyRequest`,
  `ModelRefV2`, graph `model_path`, frontend `modelPath`, reservations,
  batching groups, or worker launch facts. Verification passed:
  `cargo fmt -p pantograph-scheduler`, `cargo test -p pantograph-scheduler`,
  `cargo check -p pantograph-scheduler`,
  `cargo check -p pantograph-scheduler --all-features`,
  `cargo check -p pantograph-scheduler --no-default-features`, and
  `cargo fmt -p pantograph-scheduler -- --check`. Remaining follow-up: replace
  node-engine dependency preflight output with typed readiness proof rather
  than `ModelRefV2`.
- 2026-05-22 re-plan trigger before the next code slice: replacing node-engine
  dependency preflight output directly with `DependencyPreflightResult` would
  either break current runtime input assembly that still expects `ModelRefV2`,
  or require a compatibility adapter from typed readiness proof back to
  `ModelRefV2`. The first option is too broad for the next thin slice because
  host dispatch, queue state, and runtime handoff are not in place; the second
  option violates the no-fallback/no-legacy rule. Required re-plan decision:
  reorder Milestone 5a so node-engine preflight replacement is preceded by a
  scheduler-owned readiness admission/handoff seam that gives runtime hosts a
  non-legacy dispatch input, then delete `ModelRefV2` preflight production
  without a bridge.
- 2026-05-22 re-plan decision: use Option 3. Milestone 5a now inserts two
  prerequisite slices before node-engine preflight replacement: scheduler-owned
  readiness admission, then a non-legacy runtime handoff seam. This keeps the
  next implementation thin while preserving the no-fallback/no-legacy rule:
  runtime hosts get a new successful input contract before `ModelRefV2`
  production is deleted, and no compatibility adapter back to
  `ModelDependencyRequest` or `ModelRefV2` is permitted.
- 2026-05-22 forward re-plan audit through Milestone 8: likely future
  boundaries are now expected design checkpoints, not permission to preserve
  legacy behavior. Stop and re-plan if any slice cannot keep scheduler queue
  persistence owned by scheduler/infrastructure, isolate resource observation
  behind platform modules, keep dependency readiness under scheduler policy,
  hand executable Pumas load targets only to runtime hosts, represent batching
  across concurrent workflow runs without graph/editor execution facts, project
  capability hints without final scheduler decisions, or run Milestone 8
  release validation without deterministic environment assumptions.
- 2026-05-22 readiness admission contract slice completed. Smallest useful
  vertical slice: add scheduler-owned readiness admission request/decision
  contracts and validation in `pantograph-scheduler`, without wiring
  node-engine or runtime host execution. Allowed write set:
  `crates/pantograph-scheduler/`, this milestone file, and execution notes.
  No-fallback confirmation: ready admission requires matching path-free
  `DependencyPreflightResult` proof and rejects ready states without proof;
  deferred and terminal failed states require typed diagnostics and cannot
  carry ready proof. The contract does not expose executable Pumas load
  targets, local paths, `ModelDependencyRequest`, `ModelRefV2`, graph
  `model_path`, frontend `modelPath`, selected runtime/device dispatch
  decisions, reservations, batching groups, or worker launch facts.
  Verification passed: `cargo fmt -p pantograph-scheduler`,
  `cargo test -p pantograph-scheduler`, `cargo check -p pantograph-scheduler`,
  `cargo check -p pantograph-scheduler --all-features`,
  `cargo check -p pantograph-scheduler --no-default-features`, and
  `cargo fmt -p pantograph-scheduler -- --check`. Remaining follow-up: define
  the non-legacy runtime handoff seam that runtime/execution hosts can consume
  after readiness admission without converting readiness proof back to
  `ModelRefV2`.
- 2026-05-22 runtime handoff seam contract slice completed. Smallest useful
  vertical slice: add path-free `SchedulerRuntimeHandoff` contracts and
  validation in `pantograph-scheduler`, without wiring node-engine or host
  execution. Allowed write set: `crates/pantograph-scheduler/`, this milestone
  file, and execution notes. No-fallback confirmation: the handoff carries
  correlation ids, validated task intent, scheduler-owned readiness proof,
  matching dependency environment ref, and optional scheduler dispatch
  selection only. It rejects path/load-target fields through typed serde
  boundaries, validates correlation against task intent, validates
  environment refs against readiness proof, requires dispatch selection only
  in dispatch-selected state, and enforces explicit runtime/device hard
  requirements when dispatch selection is present. It does not expose
  executable Pumas load targets, local paths, `ModelDependencyRequest`,
  `ModelRefV2`, graph `model_path`, frontend `modelPath`, reservations,
  batching groups, or worker launch facts. Verification passed:
  `cargo fmt -p pantograph-scheduler`, `cargo test -p pantograph-scheduler`,
  `cargo check -p pantograph-scheduler`,
  `cargo check -p pantograph-scheduler --all-features`,
  `cargo check -p pantograph-scheduler --no-default-features`, and
  `cargo fmt -p pantograph-scheduler -- --check`. Remaining follow-up:
  replace node-engine dependency preflight output with typed readiness proof
  after this non-legacy host handoff seam, then delete the legacy
  `ModelRefV2` preflight production path without a bridge.
- 2026-05-22 re-plan trigger before replacing node-engine preflight output:
  codebase inspection found that `enforce_dependency_preflight` is only one
  part of the successful legacy execution path. PyTorch, llama.cpp, and audio
  execution still read `model_path` inputs and emit `ModelRefV2` outputs through
  `build_model_ref_v2`; embedded-runtime dependency preflight also still
  resolves `ModelRefV2`. Changing only the preflight return type to
  `DependencyPreflightResult` would leave successful `model_path`/`ModelRefV2`
  execution intact, while converting readiness proof back to those shapes would
  be a forbidden compatibility bridge. Required re-plan: choose the source
  replacement sequence for runtime-host load-target resolution and node-engine
  legacy removal before editing node-engine execution. Options: replace the
  runtime host path so host dispatch consumes scheduler handoff and resolves
  Pumas-approved load targets at the runtime boundary, then remove
  `ModelRefV2`/`model_path` successful execution paths; temporarily fail closed
  for affected runtime nodes until host dispatch is wired; or reject the legacy
  replacement as too broad and split a new milestone. Do not implement a
  `SchedulerRuntimeHandoff` to `ModelRefV2` adapter.
