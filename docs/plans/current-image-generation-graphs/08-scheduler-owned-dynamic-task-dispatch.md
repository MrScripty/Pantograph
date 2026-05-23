# Scheduler-Owned Dynamic Task Dispatch

## Objective

Move Pantograph toward scheduler-owned dynamic task dispatch. Workflows are
durable DAG runs whose ready nodes become schedulable task units. The scheduler
decides when each task runs, whether it can be batched with tasks from other
workflow runs, which runtime/device/dependency action applies, and whether the
task should run now, wait, defer, retry, or fail with typed diagnostics.

This replaces the older idea that a workflow receives one complete static plan
and then executes start to finish. Concurrent users may submit workflows at any
time, and a workflow run may pause between tasks while the scheduler admits
other compatible or higher-priority tasks.

## Design Model

Canonical flow:

```text
graph intent
  -> capability/type hints
  -> schedulable task intent
  -> scheduler queue/admission
  -> dispatch decision
  -> runtime host execution
  -> diagnostics/history
```

The graph editor and node-engine stay abstracted from technical execution
complexity. They describe intent and display backend-owned capability,
readiness, waiting, running, failure, and completion facts. They do not own
local model paths, executable artifact load targets, dependency environment
internals, package-manager state, live memory observations, model residency,
batching groups, warmup/load ordering, or runtime/device ranking policy.

## Responsibility Boundaries

- **Graph editor:** compose typed user intent and render backend-provided
  capability hints, option availability, task state, and diagnostics.
- **Node-engine:** validate graph semantics and submit path-free task intent.
  It must not resolve Pumas model files, dependency environments,
  runtime/device choices, executable paths, or batching policy.
- **Capability service:** answer "what is possible" for editor hints and port
  options from Pumas, inference/runtime capability, dependency readiness, and
  unavailable/not-implemented facts. It does not make final dispatch decisions.
- **Scheduler:** own queue state, fairness, batching, resource admission,
  runtime/device selection, dependency readiness policy, retry/reschedule
  decisions, and dispatch timing.
- **Dependency readiness service:** resolve/check/install dependency
  environments when directed by scheduler policy and return typed readiness
  proof or diagnostics.
- **Runtime/execution host:** consume a short-lived runtime-host execution
  request built from dispatch-selected scheduler handoff and execute it.
  Pumas-approved load targets may be resolved here only when the selected
  runtime needs executable facts.
- **Diagnostics/history ledger:** record typed waiting, unavailable, failed,
  completed, timing, dependency, resource, runtime, and batching facts for
  users and future scheduling policy.

## Standards Compliance Guardrails

- Establish one scheduler-owned backend boundary before implementation expands.
  If no existing crate cleanly owns queue, policy, resource admission, and
  dispatch contracts, create or designate a focused scheduler crate/module
  instead of spreading policy through `node-engine`, `pantograph-embedded-runtime`,
  Tauri commands, frontend adapters, or runtime adapters.
- Keep shared contracts in contract/core crates and implementation details in
  scheduler or infrastructure crates. Frontend, binding, and adapter crates may
  project or transport scheduler facts but must not own scheduler policy.
- Parse raw graph, IPC, saved workflow, queue, and ledger payloads once at the
  boundary into validated Rust types. Internal scheduler APIs should accept
  validated ids, enums, and request structs, not raw strings or unbounded maps.
- Public scheduler enums and DTOs must be explicit and typed. Use enums for
  task states, dependency actions, runtime requirements, resource states,
  dispatch states, and diagnostics. Avoid `Result<T, String>` and stringly
  typed modes in public APIs.
- Keep scheduling policy and pure admission/ranking logic synchronous unless it
  performs I/O. Async belongs at queue, ledger, dependency-service, runtime-host,
  or platform-observation boundaries.
- Long-running scheduler services must have one lifecycle owner responsible for
  startup, bounded queues, shutdown, cancellation, draining, task panics, and
  retry loop termination. Do not spawn untracked background work.
- Queue and reservation state must be durable, replayable, and idempotent.
  Recovery must not duplicate dispatch, leak reservations, or silently continue
  partially applied dependency/resource actions.
- Platform-specific resource observation belongs behind one resource observer
  trait and thin `linux`, `windows`, and `macos` modules selected at the
  platform boundary. Business logic must consume platform-neutral observations
  and typed unavailable diagnostics.
- Pumas-approved executable paths remain host/runtime-boundary facts. Scheduler
  and node-engine contracts may carry Pumas refs and selected artifact identity,
  but must not join paths, infer package layout, or trust user-supplied load
  paths.
- Every new source directory or public contract module added by this milestone
  needs README or crate-level documentation describing ownership, lifecycle,
  API consumer contract, structured producer contract, diagnostics, and
  compatibility/removal rules.

## Contracts To Add Or Replace

### Capability Hint Contract

Backend-owned type hints for graph editor and option providers:

- task type and model-ref compatibility
- possible runtimes and devices
- optional/required trait options and typed values
- available, unavailable, not-installed, not-implemented, stale, invalid, and
  ambiguous states
- diagnostics and hints for explicit user constraints

This contract must not expose local paths, load targets, package-manager
implementation details, worker launch data, or scheduler-selected decisions.

### Schedulable Task Intent

Path-free per-ready-node task intent:

- workflow id, workflow run id, node id, and task id
- user/session ownership or fairness identity
- task type and family
- canonical Pumas model ref or model capability reference
- optional hard runtime/device constraints from graph intent
- typed optional traits/settings such as denoising scheduler, sampling, seed,
  dimensions, batch size, and output requirements
- dependency override intent when explicitly provided
- bounded input-shape/resource estimate hints used for scheduling

This replaces execution-time discovery through `ModelDependencyRequest`,
`ModelRefV2`, and graph-visible `model_path`.

### Scheduler Queue State

Durable scheduler-owned task states:

- pending
- ready
- blocked
- waiting for dependency readiness
- waiting for resources
- waiting for batch
- running
- paused/deferred
- retryable failed
- terminal failed
- completed

State transitions must carry typed diagnostics and correlation ids so users can
see why a task is waiting, failed, or unavailable.

### Dispatch Decision

Short-lived decision created at or near execution admission:

- workflow/run/node/task correlation
- selected runtime and runtime variant
- selected device or device set
- selected Pumas model ref/artifact identity
- dependency readiness proof
- dependency environment ref where applicable
- batching group id when batched
- resource reservation/lease id
- runtime trait/options projection
- bounded diagnostics

The dispatch decision is not a graph contract and must not become persisted
graph input. Executable load targets remain behind host/runtime boundaries.

## Scheduler Policy Requirements

Scheduler policy must be isolated behind a small module/API so it can change
frequently without rewriting graph editor, node-engine, embedded-runtime, or
inference crate code.

Policy owns:

- runtime choice when graph runtime is omitted
- hard failure when graph explicitly requests a runtime that cannot execute
  the task
- device choice and multi-device admission
- dependency action: check only, install missing, defer, fail
- batching compatibility and fairness
- model residency and warmed-runtime affinity
- resource reservations and memory-fit checks
- retry, reschedule, and terminal failure decisions
- history use after enough observations exist

Before mature history exists for a model/runtime/task combination, scheduler
policy should distribute work across valid candidates based on facts and
exploration rules. Historical timing/failure data should influence ranking only
after the configured minimum observation threshold for each valid runtime is
met.

## Resource And Residency Requirements

The scheduler needs a resource/residency manager abstraction:

- current CPU/GPU/NPU memory observations
- per-device reservations and leases
- model loaded/warming/unloading state
- runtime process readiness
- estimated memory and load/warmup cost
- batching memory impact
- eviction/release policy
- typed diagnostics for observation failure, reservation failure, impossible
  estimates, overflow/underflow, stale data, and unavailable devices

The first implementation may use admission-time snapshots. The design must
leave a clean path to real-time resource observation during execution for
parallel runtimes, multi-model workflows, CPU plus multi-GPU/NPU execution, and
future batching policy.

Resource observation implementation requirements:

- Define a platform-neutral observation contract before adding OS-specific
  implementations.
- Put Linux, Windows, and macOS collection code in separate platform modules.
- Emit typed "not installed", "not supported", "permission denied",
  "stale observation", and "collector failed" diagnostics rather than silently
  treating missing observations as zero capacity or unlimited capacity.
- Treat reservation overflow, impossible estimates, and negative/underflowing
  release accounting as scheduler errors that fail or defer the candidate with
  ledger diagnostics.
- Verify admission-time snapshots with deterministic fakes first, then add
  platform-specific smoke coverage where local tooling supports it.

## Graph Editor Abstraction

The graph editor should remain simple:

- user selects model/task/options
- editor asks backend for option/capability hints
- unavailable/not-installed/not-implemented options can be shown disabled with
  typed reasons
- explicit runtime/device inputs are optional hard constraints
- omitted runtime/device means scheduler decides

The editor must not rank runtimes, infer devices, resolve model paths, inspect
Pumas storage, query local dependency environments directly, or display
optimistic backend-owned readiness.

## Implementation Stages

Option 4 is the target architecture. Milestone 5a established the scheduler
contracts and policy surfaces; Milestone 5c owns the cross-layer task
orchestration integration that replaces whole-workflow output-node demand as
the progress driver. Milestone 5b then consumes that task-level path to remove
legacy runtime execution.

1. Establish the scheduler ownership boundary and crate/module placement,
   including which shared contracts remain serial integration-owner work.
2. Define capability hint and schedulable task intent contracts.
3. Add scheduler-owned readiness admission and the non-legacy runtime handoff
   that runtime hosts will consume.
4. Add the task-level orchestration milestone before production runtime wiring
   because current workflow/session execution still advances through
   whole-workflow node-engine demand and stores only a reduced execution plan.
5. Add scheduler queue state and typed task lifecycle diagnostics.
6. Add dispatch decision contract and host handoff shape.
7. Move dependency readiness policy into scheduler-owned admission/dispatch.
8. Add resource/residency snapshot admission and reservation ids behind the
   platform-neutral observer abstraction.
9. Add batching policy surface for compatible ready tasks across workflows.
10. Persist run-scoped task graphs, task states, task results, and task-state
    read models so workflow progress can pause, resume, batch, and recover at
    task granularity.
11. Wire scheduler dispatch to call runtime-host execution directly with the
    actual dispatch-selected `SchedulerRuntimeHandoff`; do not launch runtime
    inference from reduced workflow execution-plan projections.
12. Retire node-engine planned-inference launch ownership for runtime
    inference nodes after scheduler-to-runtime-host dispatch is wired.
13. Delete retired resolver/path contracts and successful legacy fixtures.

## Legacy Removal Targets

- `ModelDependencyResolver`
- `ModelDependencyRequest`
- `ModelRefV2`
- graph-visible `model_path` dependency identity
- node-engine dependency preflight as a resolver
- frontend dependency-environment actions keyed by `modelPath`/`model_path`
- saved/mock workflow fixtures that treat local paths as successful dependency
  identity
- tests that validate old path-shaped contracts as current success behavior

Retired systems must be removed or updated to the canonical contracts. Do not
add compatibility shims or alternate successful branches.

## Verification Strategy

- Contract fixtures for capability hints, schedulable task intent, queue state,
  dispatch decisions, and diagnostics.
- Boundary validation tests proving raw IPC/saved-workflow/queue payloads parse
  into validated types and reject unknown states, wrong ids, path-shaped
  dependency identity, and out-of-range resource fields.
- Node-engine tests proving task intent is path-free and scheduler-owned.
- Scheduler policy tests for explicit runtime hard requirements, omitted
  runtime automatic selection, unavailable/not-implemented diagnostics,
  resource rejection, and batching compatibility.
- Lifecycle and recovery tests for bounded queue shutdown, cancellation,
  retry/defer idempotency, replay after restart, reservation release, and
  duplicate dispatch prevention.
- Resource observer tests using deterministic fakes plus platform-module
  compile checks for supported Rust targets.
- Cross-layer acceptance path from graph intent to scheduler dispatch to host
  execution without graph or node-engine path exposure.
- Scheduler-to-runtime-host acceptance proving `RuntimeHostExecutionRequest`
  is built from the real dispatch-selected handoff, not from
  `WorkflowExecutionPlanNodeDecision` or backend-decision projections.
- Multi-workflow acceptance covering at least two ready tasks from different
  workflow runs, proving queue ownership and fair deferral rather than
  whole-workflow static execution.
- Public crates touched by this milestone must run default, all-features, and
  no-default-features checks when they define feature contracts.

## Re-Plan Triggers

- A task cannot be represented without local paths or load targets before host
  dispatch.
- Batching requires graph/editor-visible execution facts.
- Dependency readiness needs to bypass scheduler policy to execute.
- Resource reservation requires shared mutable state outside the scheduler or
  resource manager owner.
- A runtime/device decision is made in graph editor, node-engine, or runtime
  adapter instead of scheduler policy.
- Scheduler policy requires async I/O in the core ranking/admission API instead
  of an async shell around a synchronous core.
- Platform resource observation cannot be isolated behind a platform-neutral
  trait and thin OS modules.
- Queue recovery cannot be made idempotent without changing persisted contract
  shape or ledger semantics.

## Known Forward Boundaries Before Milestone 8

These are expected design checkpoints. They are not permission to add fallback
branches or preserve retired systems.

- **Readiness admission ordering:** replacing node-engine preflight directly
  would either break the current runtime input path or require converting back
  into `ModelRefV2`. Use the selected Option 3 ordering: add
  scheduler-owned readiness admission and non-legacy runtime handoff first,
  then complete Milestone 5b to wire scheduler dispatch directly into
  runtime-host execution and delete `ModelRefV2`/`model_path` successful
  paths.
- **Reduced execution-plan boundary:** `WorkflowExecutionPlanNodeDecision` is
  an inspection/diagnostics projection, not executable scheduler state. If
  runtime launch needs full dispatch facts, use the actual
  `SchedulerRuntimeHandoff`; do not synthesize handoff or backend execution
  decisions from the reduced plan.
- **Scheduler queue persistence:** durable task state must have one owner. If
  existing ledger tables cannot represent replayable queue state without
  overloading diagnostics, create scheduler-owned persistence instead of
  storing operational queue state as incidental metadata.
- **Resource observation and reservations:** platform collectors must remain
  behind a neutral observer contract with Linux, Windows, and macOS modules.
  If admission-time snapshots are not enough for concurrent runtime execution,
  re-plan the real-time observer upgrade without changing graph/editor
  contracts.
- **Dependency readiness ownership:** dependency check/install/defer/fail
  policy must move under scheduler admission. If an existing host API can only
  return `ModelDependencyRequest`, `ModelRefV2`, or executable paths, replace
  that API rather than bridging through it.
- **Runtime host load-target resolution:** Pumas-approved executable paths may
  be resolved only by the runtime host that needs them. If inference or worker
  APIs require paths earlier than dispatch, re-plan the host request shape
  rather than passing paths through node-engine or graph contracts.
- **Batching and fairness:** batching must work across concurrent workflow
  runs and task pauses. If the existing execution loop assumes whole-workflow
  uninterrupted execution, re-plan queue/dispatch ownership instead of making
  batching graph-visible.
- **Capability hint projection:** graph editor option providers may show
  possible, unavailable, not-installed, or not-implemented choices, but not
  final dispatch decisions. If existing option-provider APIs cannot express
  disabled typed options, replace the projection contract rather than adding
  frontend inference.
- **Milestone 8 validation environment:** release validation needs
  deterministic fixtures and explicit local environment assumptions for Pumas,
  managed runtimes, resource collectors, and smoke workflows. If those
  assumptions are missing, record them and re-plan the validation harness
  instead of accepting nondeterministic release checks.
