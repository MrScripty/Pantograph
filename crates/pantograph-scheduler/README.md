# pantograph-scheduler

## Purpose
This crate owns Pantograph scheduler contracts for dynamic task dispatch:
queue state, scheduling policy boundaries, resource admission, dependency
readiness policy, batching, dispatch decisions, and scheduler lifecycle
ownership.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `src/lib.rs` | Curated public re-exports for scheduler boundary and task intent contracts. |
| `src/batching.rs` | Scheduler-owned batching candidate and policy decision contracts for compatible task groups. |
| `src/capability.rs` | Backend-owned capability hint contract for graph editor and option-provider consumers. |
| `src/dispatch.rs` | Scheduler-selected dispatch decision contract for runtime/device/model/dependency/reservation/batch facts. |
| `src/dispatch_selection.rs` | Pure scheduler-owned dispatch candidate validation and no-fallback selection contract that produces `SchedulerDispatchDecision` only from typed path-free facts. |
| `src/handoff.rs` | Non-legacy runtime handoff envelope consumed after scheduler readiness admission. |
| `src/intent.rs` | Path-free schedulable task intent contract, validated workflow/run/node/task ids, runtime/device constraints, typed trait settings, and bounded estimate hints. |
| `src/lifecycle.rs` | Backend-owned scheduler task lifecycle diagnostic snapshots for graph/run inspection. |
| `src/ownership.rs` | Scheduler-owned capability and non-scheduler consumer ownership boundary enums. |
| `src/queue.rs` | Durable scheduler task queue state and idempotent transition replay contract. |
| `src/readiness.rs` | Scheduler-owned readiness admission request/decision contracts and host-ready dependency readiness proof wrapper. |
| `src/resource.rs` | Platform-neutral resource/residency snapshot, reservation, runtime-readiness, load/warmup, batching-memory, and observer contracts. |
| `src/resource_types.rs` | Shared resource/residency enums and diagnostics used by scheduler observation contracts. |
| `src/supervision.rs` | Scheduler lifecycle supervision contract for long-running queue, dependency, resource, runtime-host, retry, and reservation-cleanup services. |
| `src/README.md` | Source-directory contract map and no-legacy invariants for scheduler modules. |
| `tests/` | Public serde and validation contract tests with JSON fixtures. |
| `tests/fixtures/` | Canonical structured payloads for persisted or transported scheduler DTOs. |

## Problem
Pantograph workflows can pause between ready DAG tasks while concurrent users,
batching, model residency, dependency readiness, and resource availability
change. Without a dedicated scheduler owner, policy can drift into graph
editing, node execution, frontend actions, Tauri commands, runtime adapters, or
diagnostics projections.

## Constraints
- The scheduler is the only owner of queue policy, runtime/device selection,
  resource admission, dependency-readiness action, batching, dispatch timing,
  retry/defer/fail decisions, and long-running scheduler lifecycle.
- Graph editor and node-engine remain path-free and may only submit typed
  intent or display backend-owned capability/task facts.
- Runtime hosts may consume short-lived dispatch decisions and resolve
  Pumas-approved load targets only at the runtime boundary.
- Frontend, Tauri, binding, and runtime-adapter crates may project or transport
  scheduler facts but must not own scheduler decisions.
- Core scheduler policy should remain synchronous unless a later slice records
  the I/O operation that requires an async shell.

## Decision
Create a focused scheduler crate before implementing dynamic task dispatch.
This makes scheduler ownership explicit in Cargo and gives future slices one
place to add task intent, queue state, dispatch decisions, resource observation,
batching, and lifecycle contracts without preserving legacy dependency
resolver behavior.

## Alternatives Rejected
- Put scheduler policy in `node-engine`: rejected because node-engine should
  validate graph semantics and submit task intent, not choose runtimes,
  resources, batching, or dependency actions.
- Put scheduler policy in `pantograph-embedded-runtime`: rejected because the
  embedded runtime is a composition/runtime host boundary, not the canonical
  policy owner.
- Put scheduler policy in Tauri/frontend adapters: rejected because adapters
  may transport or display scheduler state but must not become sources of
  backend execution semantics.

## Invariants
- `SchedulerOwnedCapability` entries are owned by this scheduler boundary.
- `SchedulerBoundaryConsumer` entries may consume scheduler facts but cannot
  own scheduler-owned capabilities.
- This crate must not inspect Pumas filesystem layout, join local model paths,
  execute workflow nodes, or launch runtimes.
- Legacy `ModelDependencyResolver`, `ModelDependencyRequest`, `ModelRefV2`,
  graph-visible `model_path`, and frontend `modelPath` paths are replacement
  targets, not compatibility contracts for this crate.
- `SchedulableTaskIntent` must remain path-free. It may carry a canonical
  `PumasModelRef`, optional hard runtime/device requirements, typed trait
  settings, dependency override patches, and bounded estimate hints, but it
  must not carry `model_path`, local load paths, executable Pumas load targets,
  or worker launch details.
- `SchedulerCapabilityHintSnapshot` is an editor/options hint contract, not a
  dispatch decision. It may describe possible runtimes, devices, trait options,
  availability states, and diagnostics, but must not contain selected runtime,
  selected device, load target, reservation, batching, worker launch, or final
  scheduler decision fields.
- `SchedulerReadinessAdmissionDecision` is the scheduler-owned admission result
  before runtime host handoff. A ready decision must carry
  `SchedulerDependencyReadinessProof`, whose `DependencyPreflightResult` is
  validated as path-free and ready. Deferred, retryable failed, and terminal
  failed decisions must carry typed diagnostics and must not carry ready proof.
  `plan_scheduler_readiness_admission` owns the check/install/defer/retry/fail
  policy mapping from host preflight state into scheduler admission decisions.
- `SchedulerRuntimeHandoff` is the path-free host-facing envelope after
  readiness admission. It carries task correlation, task intent, scheduler-owned
  readiness proof, matching dependency environment ref, and optionally the
  scheduler dispatch decision. It must not carry executable Pumas load targets,
  local paths, `ModelRefV2`, or worker launch facts.
- `SchedulerTaskStateRecord` and `SchedulerTaskStateTransition` are durable,
  replayable task-state contracts for one workflow task. They carry task
  correlation, phase-aware state, state version, and transition id only.
  Runtime schedulable phases carry `SchedulableTaskIntent`; source-input
  materialization phases carry `SchedulerSourceInputTaskIntent`;
  non-runtime node-engine phases carry `SchedulerNonRuntimeTaskIntent`;
  pre-intent and terminal invalid phases carry typed diagnostics instead. They
  must not carry
  executable Pumas load targets, local paths, `ModelRefV2`, worker launch
  details, reservations, or batching groups.
- `SchedulerTaskLifecycleDiagnosticSnapshot` is a backend-owned explanation of
  one scheduler queue task state for graph editor and run inspection. It may
  explain waiting, deferred, unavailable, failed, and completed states with
  typed diagnostics, but it must not expose frontend-inferred states,
  executable paths, runtime host internals, reservations, or batching groups.
- `SchedulerLifecycleOwnerSnapshot` is the single scheduler lifecycle owner
  contract for long-running scheduler services. It must include queue worker,
  dependency-readiness action, resource-observation loop, runtime-host
  dispatch, retry loop, and reservation-cleanup components exactly once, with
  bounded queues where asynchronous work may accumulate.
- `SchedulerDispatchDecision` is the scheduler-selected execution admission
  contract. It carries runtime/device/model/dependency/reservation/batch facts
  and runtime trait projection for host handoff, but it must not carry
  executable Pumas load targets, local paths, worker launch details, or graph
  inputs.
- `SchedulerResourceResidencySnapshot` is the scheduler-owned
  platform-neutral resource observation contract. It carries device resource
  snapshots, active reservation leases, runtime readiness, model residency,
  load/warmup estimates, batching memory impact, fit assessments, and typed
  diagnostics only. Platform-specific collectors must live behind
  `SchedulerResourceObserver`; scheduler policy must not depend on OS-specific
  resource APIs directly.
- `SchedulerBatchPolicyDecision` is the scheduler-owned batching compatibility
  contract. It groups compatible task candidates by task family, selected
  Pumas model ref, runtime, device set, input shape, memory impact, latency,
  residency state, and fairness-bearing task intent without exposing runtime
  worker inputs or executable load targets.

## Revisit Triggers
- A future implementation slice needs to put scheduler policy outside this
  crate.
- Runtime/resource observation cannot be represented behind a platform-neutral
  scheduler contract.
- Queue recovery or dispatch idempotency requires a persisted contract shape
  that does not fit this crate boundary.

## Dependencies
**Internal:** Future slices will consume contracts from dependency planning,
runtime identity/registry, diagnostics ledger, workflow service, and embedded
runtime host boundaries.

**External:** `serde` for scheduler DTO serialization.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```rust
use pantograph_scheduler::{
    owner_for_capability, SchedulerBoundaryOwner, SchedulerOwnedCapability,
    SchedulableTaskIntent, ValidatedSchedulableTaskIntent,
};

assert_eq!(
    owner_for_capability(SchedulerOwnedCapability::DispatchDecision),
    SchedulerBoundaryOwner::Scheduler
);

let fixture = r#"{
  "contract_version": 1,
  "workflow_id": "workflow.image_generation",
  "workflow_run_id": "run.001",
  "node_id": "node.llm_inference",
  "task_id": "task.001",
  "task_type": "image_generation",
  "model_ref": { "model_id": "pumas://models/example" }
}"#;

let intent: SchedulableTaskIntent = serde_json::from_str(fixture)?;
let _validated = ValidatedSchedulableTaskIntent::try_from(intent)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## API Consumer Contract
- Inputs: scheduler-owned capability labels, non-scheduler consumer labels, and
  raw `SchedulableTaskIntent` DTOs decoded from graph, IPC, saved-workflow, or
  queue payloads. Graph editor and option-provider consumers may also consume
  raw `SchedulerCapabilityHintSnapshot` DTOs produced by backend services.
  Scheduler admission may consume raw `SchedulerReadinessAdmissionRequest`
  values containing validated task intent plus dependency readiness policy.
- Outputs: the canonical owner for scheduler capabilities, explicit deny-policy
  checks for non-scheduler consumers, and `ValidatedSchedulableTaskIntent`
  values for internal scheduler policy. Capability consumers receive
  `ValidatedSchedulerCapabilityHintSnapshot` values after boundary validation.
  Runtime host handoff slices may consume
  `ValidatedSchedulerReadinessAdmissionDecision` values once this crate has
  validated that ready decisions carry matching path-free dependency readiness
  proof. Scheduler admission policy may call
  `plan_scheduler_readiness_admission` with a validated request and optional
  preflight result to produce check, install-missing, defer, retry, fail, or
  admit decisions without node-engine resolver discovery. Runtime hosts may
  consume `ValidatedSchedulerRuntimeHandoff` values once correlation,
  environment refs, readiness proof, and optional dispatch decision have been
  validated. Queue persistence and replay consumers may use
  `ValidatedSchedulerTaskStateRecord`,
  `ValidatedSchedulerTaskStateTransition`, and
  `apply_scheduler_task_state_transition` to validate idempotent task-state replay.
  Graph editor and run-inspection consumers may display
  `ValidatedSchedulerTaskLifecycleDiagnosticSnapshot` values after the backend
  validates state-compatible diagnostic codes and bounded messages.
  Composition roots and health checks may consume
  `ValidatedSchedulerLifecycleOwnerSnapshot` values after the backend validates
  required component ownership, bounded queues, cancellation state, panic
  state, shutdown state, and diagnostics.
  Runtime-host handoff consumers may use
  `ValidatedSchedulerDispatchDecision` after selected runtime, selected
  devices, selected Pumas model/artifact identity, dependency proof,
  environment ref, reservation lease, batching group, and runtime trait
  projection are validated.
  Scheduler resource policy may use
  `ValidatedSchedulerResourceResidencySnapshot` values returned by a
  `SchedulerResourceObserver` after the backend validates resource arithmetic,
  reservation identity, runtime readiness diagnostics, residency diagnostics,
  batching memory impact, and impossible-fit diagnostics.
  Scheduler batching policy may use
  `ValidatedSchedulerBatchPolicyDecision` values after candidate correlation,
  selected runtime/device/model facts, input-shape compatibility, checked
  memory totals, batch size bounds, and rejection diagnostics are validated.
  Scheduler dispatch-selection policy may use
  `ValidatedSchedulerDispatchSelectionRequest` values to select one
  `SchedulerDispatchDecision` from typed path-free dispatch candidates or
  return a no-selection decision with typed diagnostics. Candidate ids,
  selected runtime/device/model facts, resource fit, reservation leases,
  batching group ids, and source diagnostics are evidence for this scheduler
  policy only; executable load targets and graph-local paths are rejected.
- Lifecycle: this crate currently starts no tasks and owns no runtime handles.
  Later scheduler services must add one lifecycle owner for queues, workers,
  cancellation, shutdown, and reservation cleanup.
- Errors: scheduler boundary validation returns `SchedulerContractError`.
  Runtime scheduling, dependency, resource, and dispatch failures will be
  represented by later typed diagnostics instead of string parsing.
- Compatibility: `SCHEDULER_CONTRACT_VERSION` is the starting version for
  persisted or transported scheduler DTOs.
  `SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION` is the starting version for ready
  DAG task intent. Breaking persisted shape changes require migration planning.

## Structured Producer Contract
- Stable fields: `SCHEDULER_CONTRACT_VERSION`,
  `SchedulerOwnedCapability`, `SchedulerBoundaryConsumer`, and
  `SchedulerBoundaryOwner` enum semantics; `SchedulableTaskIntent` field names,
  task correlation ids, `PumasModelRef`, runtime/device constraint fields,
  typed trait setting value kinds, and estimate hint kinds;
  `SchedulerCapabilityHintSnapshot` task type, optional model ref, runtime
  hints, device hints, trait option hints, availability states, diagnostic
  severities, and diagnostic codes; `SchedulerReadinessAdmissionRequest` task
  intent and policy fields; `SchedulerReadinessAdmissionDecision` action,
  state, readiness proof, and typed diagnostics; `SchedulerRuntimeHandoff`
  correlation fields, state, readiness proof, environment ref, and optional
  dispatch decision fields; `SchedulerTaskStateRecord` correlation, task
  intent, state, state version, and last transition id fields;
  `SchedulerTaskStateTransition` correlation, task intent, expected previous state,
  next state, and transition id fields;
  `SchedulerTaskLifecycleDiagnosticSnapshot` correlation, queue state,
  diagnostic severity, code, bounded message, and optional hint fields;
  `SchedulerLifecycleOwnerSnapshot` owner id, component kind, component state,
  cancellation state, panic state, queue bounds, and lifecycle diagnostic
  fields; `SchedulerDispatchDecision` correlation, selected runtime, optional
  runtime variant, selected device set, selected model ref, readiness proof,
  environment ref, optional batching group, reservation lease, runtime trait
  settings, and dispatch diagnostic fields;
  `SchedulerResourceResidencySnapshot` observation timestamp, device resource
  kind and byte counts, active reservation lease ids, runtime readiness state,
  model residency state, load/warmup timing, batching memory impact, fit
  assessment state, and resource diagnostic fields;
  `SchedulerBatchPolicyDecision` batch group id, state, max and selected batch
  sizes, total incremental memory bytes, candidate task correlation,
  task-family, selected runtime, selected device set, selected model ref,
  input shape signature, latency estimate, memory impact, residency state, and
  batch diagnostic fields; `SchedulerDispatchSelectionRequest` task intent,
  readiness proof, environment ref, candidate id, selected runtime, optional
  runtime variant, selected devices, selected Pumas model ref, runtime trait
  settings, reservation fact, resource fit fact, optional batching group, and
  typed diagnostic fields.
- Defaults: no default scheduler owner other than `Scheduler` exists.
  Omitted runtime/device constraints mean scheduler policy decides.
- Enum semantics: owned capability variants identify decisions and state that
  belong to the scheduler boundary; consumer variants identify components that
  may consume scheduler facts but not own policy. Estimate hint variants are
  hints only; scheduler policy owns final resource admission. Capability
  availability states are display and validation hints only; scheduler policy
  owns final dispatch/admission decisions. Readiness admission state describes
  whether dependency readiness allows dispatch now, defers for later work,
  failed in a retryable way, or fails terminally; it is not a runtime/device
  selection result. Runtime handoff state separates readiness-admitted handoff
  from dispatch-selected handoff so runtime/device/model/reservation/batch
  facts are added only by the scheduler dispatch decision.
  Queue task states describe durable scheduler progress only; lifecycle
  diagnostics, resource reservations, batching groups, and runtime dispatch
  facts are separate contracts. Source-input materialization may complete an
  awaiting-inputs task only by carrying `SchedulerSourceInputTaskIntent`; it
  must not pretend the task ran through runtime or non-runtime execution.
  Lifecycle diagnostic codes must be compatible with the queue state they
  explain. Lifecycle supervision component states describe service ownership
  and health only; they do not select runtime, device, dependency, resource,
  or batching policy. Dispatch decisions are
  short-lived scheduler-owned facts and must not become graph inputs or
  executable load-target carriers. Resource snapshot states describe
  scheduler-observed availability and fit only; they do not authorize graph,
  node-engine, frontend, or runtime adapter code to choose executable load
  targets or bypass dispatch policy. Batching decisions validate candidate
  compatibility and queue grouping facts only; runtime execution still flows
  through scheduler dispatch and runtime host handoff. Dispatch-selection
  decisions are pure scheduler policy over supplied typed facts. They must not
  alias runtime-registry technical-fit candidate shapes or scheduler batching
  candidate shapes, and they must not use candidate ids as a ranking fallback
  when multiple candidates remain eligible.
- Ordering: enum declaration order is not a runtime contract.
- Compatibility: new capability or consumer variants may be added as scheduler
  contracts expand; existing meanings must not be repurposed. New task trait
  values must be added as typed enum variants or typed structs, not arbitrary
  JSON maps.
- Regeneration/migration: fixtures, bindings, adapters, and README content must
  update in the same slice when persisted scheduler DTOs are introduced or
  changed.
