# pantograph-dependency-planning

## Purpose
This crate owns Pantograph's shared dependency-planning contracts: typed model
identity, scheduler intent, dependency override patches, planning request/result
DTOs, dependency-environment request/result DTOs, Pumas-facing load target
mirrors, and diagnostics used across graph execution, host planning, frontend
actions, persisted fixtures, and backend handoff boundaries.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `src/lib.rs` | Curated public re-exports for dependency-planning consumers. |
| `src/environment.rs` | Typed dependency-environment resolve/check/install request, result, requirement, binding, status-row, operation, validation-error, and environment-ref contracts. |
| `src/environment/` | Dependency-environment child modules for result payload rows and row-level validation helpers. |
| `src/execution.rs` | Path-free dependency-readiness execution context plus request/proof envelopes used to bind readiness checks to an admitted scheduler task and validation snapshot. |
| `src/model_ref.rs` | Pumas-compatible model reference and artifact load-target contract mirrors. |
| `src/preflight.rs` | Path-free preflight request/result contracts and shared dependency-planning identity/correlation key. |
| `src/producer.rs` | Pure dependency requirements proof producer that derives path-free requirements ids, override fingerprints, status, and diagnostics from validated planning requests plus optional typed availability facts. |
| `src/request.rs` | Typed dependency-planning request, caller context, scheduler intent, dependency overrides, and validated ids. |
| `src/result.rs` | Typed dependency-planning result states and diagnostics. |
| `tests/` | Public serde and validation contract tests with JSON fixtures. |

## Problem
Dependency preflight previously mixed graph identity, Pumas model lookup,
runtime dependency readiness, local filesystem paths, and worker handoff DTOs in
node-engine and embedded-runtime modules. That made it easy for legacy
`model_path` values to survive as graph identity. This crate gives those layers
one neutral contract boundary before runtime behavior is migrated.

## Constraints
- The crate must remain contract/domain-only.
- Pumas owns model-library lookup, selected artifact identity, storage kind,
  validation state, and approved load targets.
- Scheduler/planner code owns runtime/device selection policy.
- Node-engine and frontend code may send typed intent, but must not infer Pumas
  paths or select executable runtimes directly.
- Worker-visible local paths may appear only in host/planner result handoff
  shapes after Pumas has approved a load target.
- Dependency-environment actions are typed resolve/check/install operations.
  They are not raw frontend modes and they do not authorize path-based model
  lookup.
- Dependency-environment result payloads carry shared requirement, binding,
  status, operation, validation-error, and environment-ref facts as typed rows.
  Runtime-specific and package-manager-specific facts stay behind optional
  detail structs instead of becoming generic dependency fields.

## Decision
Create a small workspace crate with validated request/result DTOs and
Pumas-compatible model/load-target mirrors. Existing inference-facing Pumas DTOs
are re-exported from this crate so Pantograph has one canonical mirror location
instead of parallel artifact DTO families. Dependency-environment contracts live
in this crate because they are shared boundary DTOs for graph, frontend,
embedded-runtime, and host dependency systems; the actual package-manager and
Pumas I/O remains outside this crate.
Dependency readiness execution contexts and request/proof envelopes are
constructed through crate-owned constructors. Downstream crates must not build
those `#[non_exhaustive]` structs through JSON round-trips or duplicate local
DTOs.

## Alternatives Rejected
- Keep dependency-planning DTOs in `node-engine`: rejected because node-engine
  should forward validated graph intent, not own Pumas artifact semantics.
- Keep dependency-planning DTOs in `inference`: rejected because dependency
  planning spans graph, host, scheduler, frontend, and worker-adjacent
  boundaries, not only inference runtime contracts.
- Add a second Pumas artifact DTO family: rejected because it would create
  drift between package facts, dependency planning, and worker handoff.

## Invariants
- Requests are keyed by `PumasModelRef` and typed task/scheduler intent, never
  by local filesystem path.
- Manual dependency override patches are part of the shared request contract,
  not an adapter-local extension field.
- Platform context is a typed platform key, not arbitrary JSON forwarded
  through node-engine.
- Source node type is bounded caller context for diagnostics, not a runtime
  routing selector.
- Dependency requirements proof production is pure domain logic. It derives
  stable ids from validated typed request fields and optional typed
  availability facts; it does not call Pumas, inspect files, select
  runtime/device policy, or query scheduler state.
- Path-free preflight identity is separate from host/planner load-target
  results. `DependencyPreflightRequest`, `DependencyPreflightResult`, and
  `DependencyPlanningIdentityKey` must never contain `PumasArtifactLoadTarget`,
  `model_path`, local load paths, `entry_path`, or `selected_artifact_path`.
- Pumas load targets are result/handoff facts, not graph identity.
- Dependency-environment requests carry a typed action and a matching
  `DependencyPlanningIdentityKey` plus `DependencyPlanningRequest`. The identity
  key and planning request must agree on model ref, task, artifact kind,
  platform, selected bindings, and scheduler intent.
- Dependency-environment check/install requests require a
  `dependency_requirements_id`; resolve requests may omit it.
- Dependency-environment readiness, install, validation, and failure states are
  typed enums.
- Dependency-environment requirement kinds, environment kinds, binding status
  states, operation states, validation codes, selected binding ids, profile ids,
  requirement names, validation field paths, and operation timestamps are typed
  contract values.
- Python/package-manager facts are scoped to `PythonRequirementDetails` and
  `PythonBindingDetails`; non-Python dependency classes must not reuse those
  fields.
- Raw graph JSON and frontend payloads must parse once into validated domain
  types before internal use.
- Missing, stale, invalid, unavailable, ambiguous, needs-detail, and
  not-implemented states remain distinct typed result states.
- Diagnostics use typed codes and severities rather than string parsing.
- Dependency requirements ids and override fingerprints are stable `blake3`
  hashes over canonical typed identity fields. Caller context and migration
  diagnostics do not influence requirements identity.

## Revisit Triggers
- Pumas publishes generated schemas or a stable shared Rust contract crate that
  Pantograph can consume directly.
- Dependency planning starts carrying scheduler policy instead of intent and
  must move into a scheduler-owned crate.
- Host-language bindings require generated schemas for these DTOs.

## Dependencies
**Internal:** None.

**External:** `serde`, `serde_json`, `thiserror`, and `blake3`. `blake3` is
used only for stable dependency requirements ids and override fingerprints.

## Related ADRs
- None identified as of 2026-05-20.
- Reason: this crate implements the existing Milestone 5 dependency-planning
  contract-owner decision.
- Revisit trigger: dependency planning becomes a public extension API or a
  generated cross-language schema.

## Usage Examples
```rust
use pantograph_dependency_planning::{
    DependencyEnvironmentAction, DependencyEnvironmentRequest, DependencyNodeTypeId,
    DependencyPlanningCallerContext, DependencyPlanningIdentityKey,
    produce_dependency_requirements_proof, DependencyPlanningPlatformContext,
    DependencyPlanningRequest, DependencyTaskId, PumasModelRef,
    ValidatedDependencyEnvironmentRequest, ValidatedDependencyPlanningRequest,
};

let model_ref = PumasModelRef {
    model_id: "models/example".to_string(),
    revision: None,
    selected_artifact_id: Some("artifact-1".to_string()),
    selected_artifact_path: None,
    migration_diagnostics: Vec::new(),
};

let request = DependencyPlanningRequest {
    model_ref: model_ref.clone(),
    task_id: DependencyTaskId::parse("image_generation")?,
    task_type: None,
    expected_artifact_kind: None,
    scheduler_intent: Default::default(),
    platform_context: Some(DependencyPlanningPlatformContext::from_os_arch(
        std::env::consts::OS,
        std::env::consts::ARCH,
    )?),
    selected_binding_ids: Vec::new(),
    dependency_override_patches: Vec::new(),
    trait_intents: Vec::new(),
    caller_context: DependencyPlanningCallerContext {
        source_node_type: Some(DependencyNodeTypeId::parse("dependency-environment")?),
        ..Default::default()
    },
};

let identity_key = DependencyPlanningIdentityKey {
    model_ref,
    task_id: request.task_id.clone(),
    task_type: request.task_type.clone(),
    expected_artifact_kind: request.expected_artifact_kind.clone(),
    scheduler_intent: request.scheduler_intent.clone(),
    platform_context: request.platform_context.clone(),
    selected_binding_ids: request.selected_binding_ids.clone(),
};

let environment_request = DependencyEnvironmentRequest {
    contract_version: 1,
    action: DependencyEnvironmentAction::Resolve,
    identity_key,
    planning_request: request.clone(),
    dependency_requirements_id: None,
    environment_ref: None,
};

let _validated_environment =
    ValidatedDependencyEnvironmentRequest::try_from(environment_request)?;

let planning_request = DependencyPlanningRequest {
    model_ref: PumasModelRef {
        model_id: "models/example".to_string(),
        revision: None,
        selected_artifact_id: Some("artifact-1".to_string()),
        selected_artifact_path: None,
        migration_diagnostics: Vec::new(),
    },
    task_id: DependencyTaskId::parse("image_generation")?,
    task_type: None,
    expected_artifact_kind: None,
    scheduler_intent: Default::default(),
    platform_context: Some(DependencyPlanningPlatformContext::from_os_arch(
        std::env::consts::OS,
        std::env::consts::ARCH,
    )?),
    selected_binding_ids: Vec::new(),
    dependency_override_patches: Vec::new(),
    trait_intents: Vec::new(),
    caller_context: DependencyPlanningCallerContext {
        source_node_type: Some(DependencyNodeTypeId::parse("llm-inference")?),
        ..Default::default()
    },
};

let _validated = ValidatedDependencyPlanningRequest::try_from(planning_request)?;
let _proof = produce_dependency_requirements_proof(&_validated, None)?;
# Ok::<(), pantograph_dependency_planning::DependencyPlanningContractError>(())
```

## API Consumer Contract
- Inputs: dependency-planning and dependency-environment request DTOs decoded
  from graph, frontend, or host payloads.
- Outputs: typed dependency-planning results, dependency-environment results,
  requirement rows, binding rows, binding status rows, operation facts,
  validation-error rows, environment refs, and diagnostics.
- Lifecycle: this crate has no runtime lifecycle and starts no tasks.
- Errors: boundary validation returns `DependencyPlanningContractError`; planning
  and dependency-environment failures are represented as typed result states and
  diagnostics.
- Dependency requirements producer: callers pass a validated
  `DependencyPlanningRequest` and optional typed
  `DependencyRequirementsAvailabilityFacts`. The producer returns a path-free
  `DependencyRequirementsProof` with requirements id, override fingerprint,
  status, and dependency-planning diagnostics. It never performs I/O.
- Compatibility: fields and enum meanings are machine-consumed by Rust,
  frontend, persisted fixtures, and worker-adjacent code. Breaking wire-shape
  changes require a coordinated migration.

## Structured Producer Contract
- Stable fields: request/result field names, dependency-environment action and
  state enum spellings, requirement/binding/status/operation enum spellings,
  typed diagnostic codes, environment-ref ids, and Pumas-compatible
  model/load-target shapes.
- Defaults: omitted scheduler intent, caller context, selected bindings, and
  diagnostics mean empty intent/context/bindings/diagnostics. Omitted
  dependency-environment requirements, bindings, binding statuses, validation
  errors, and operation facts mean the producer has no facts for that row set.
  Omitted dependency-environment contract version means version 1.
- Enum semantics: result states distinguish unavailable, invalid, stale,
  ambiguous, needs-detail, missing, and not-implemented; dependency-environment
  readiness/install/validation/failure states are distinct because scheduler,
  activity, and UI decisions depend on those differences.
- Ordering: selected binding ids and diagnostics preserve producer order.
- Requirements identity: canonical requirements ids include model identity,
  task, artifact kind, scheduler intent, platform key, selected bindings,
  dependency override fingerprint, and dependency-planning-local trait intents.
  Caller context, local paths, Pumas load targets, package facts, runtime load
  targets, scheduler dispatch decisions, and frontend display state are
  excluded.
- Validation: operation timestamps must be non-zero and completion must not
  precede start; selected binding ids must be unique; validation field paths are
  contract paths, not filesystem paths.
- Compatibility: serde fixture tests are required for public DTO changes.
- Regeneration/migration: when Pumas or frontend wire shapes change, update this
  crate's fixtures and downstream consumers in the same slice.

## Testing
```bash
cargo test -p pantograph-dependency-planning
```
