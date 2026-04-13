# ADR-002: Runtime Registry Ownership And Lifecycle

## Status
Accepted

## Context
Pantograph has already converged runtime identity, capability, and diagnostics
contracts across `crates/inference`, `crates/pantograph-embedded-runtime`,
`crates/pantograph-workflow-service`, and the Tauri host adapters. That
groundwork makes runtime state more observable, but Pantograph still lacks a
single owner for live runtime residency, reservation, admission, retention, and
eviction policy.

Current ownership seams:
- `crates/inference` owns the execution facade, backend lifecycle operations,
  and dedicated embedding runtime management.
- `crates/pantograph-embedded-runtime` owns Pantograph-specific runtime
  capability exposure, dependency-aware task execution, and Python sidecar
  integration.
- `crates/pantograph-workflow-service` owns host-agnostic workflow/session
  orchestration and must stay free of app-specific runtime policy.
- `src-tauri/src/main.rs` is the current Pantograph app composition root that
  creates the shared gateway, workflow service, diagnostics stores, and Tauri
  state injection.
- `src-tauri/src/llm` and `src-tauri/src/workflow` currently coordinate
  gateway-backed runtime interactions through adapter code and app state.

Without an explicit architectural decision, runtime policy could drift into the
gateway, workflow service, or Tauri adapter modules as scheduler and
technical-fit work expands.

## Decision
Adopt the following ownership and lifecycle boundary for the planned
`RuntimeRegistry`:

1. `RuntimeRegistry` is a Pantograph-owned application-layer coordinator.
- It sits above `inference::InferenceGateway`.
- It owns live runtime residency state, admission/reservation policy,
  warmup/reuse coordination, retention hints, and eviction decisions.
- It is not part of `crates/inference` and is not owned by
  `pantograph-workflow-service`.

2. `InferenceGateway` remains the execution facade.
- `crates/inference` stays the infrastructure boundary for backend lifecycle
  operations, runtime process control, runtime capability facts, and inference
  request forwarding.
- Gateway and backend modules must not become the owner of scheduler or
  technical-fit policy.

3. `pantograph-workflow-service` remains host-agnostic.
- The workflow service continues to own workflow/session orchestration
  contracts.
- It does not own Pantograph app-global runtime residency state.
- Host layers may consult the registry before or around service calls, but the
  service crate must not become the registry implementation home.

4. Pantograph host adapters consume registry operations; they do not own policy.
- Tauri command modules and other future app adapters remain transport/composer
  layers.
- They may inject the registry and request operations from it, but they must
  not implement competing runtime residency or admission logic.

5. The app composition root creates and tears down the registry.
- In the current app, that composition root is `src-tauri/src/main.rs`.
- Future non-Tauri app roots may create the same registry boundary, but the
  ownership rule stays the same: the top-level app creates it, injects it, and
  stops it.

6. Lifecycle and background-task ownership are explicit.
- The registry owns any policy-level background tasks it starts, including
  health polling, runtime-state refresh, admission bookkeeping, retention
  timers, and cleanup coordination.
- Gateway/backends continue to own backend-internal runtime lifecycle work for
  the runtime instances they manage.
- Cleanup and cancellation must be symmetric: the same owner that starts a
  registry-managed background task must stop it.

7. Reservation/admission is a first-class contract.
- Workflow/session execution must obtain a registry reservation before warmup
  and technical-fit admission proceed.
- Reservations must be released on successful completion, cancellation, and
  failure paths.
- Eviction decisions must exclude active, reserved, or pinned runtimes/models.

8. `crates/pantograph-embedded-runtime` remains a runtime producer and executor.
- It continues to expose Pantograph-specific runtime capabilities and execute
  Pantograph-owned task paths.
- It does not become the owner of app-global runtime residency policy.
- Later milestones may make it consume registry-backed decisions, but that does
  not change its role into the policy owner.

## Consequences

### Positive
- Runtime residency and admission policy gain a single owner instead of being
  spread across gateway, workflow service, and adapters.
- Scheduler V2 and technical-fit selection can build on an explicit application
  boundary instead of ad hoc Tauri glue.
- Existing inference and workflow facades remain stable while policy moves
  outward into a Pantograph-owned coordinator.
- Documentation and README traceability can point to one accepted boundary for
  future implementation commits.

### Negative
- Pantograph adds another application-layer object that must be injected and
  shut down correctly.
- Some current Tauri-side coordination responsibilities will need to move or be
  wrapped by the registry in later milestones.
- Implementers must be disciplined about not taking shortcuts inside gateway or
  adapter code when policy work becomes urgent.

### Neutral
- This ADR freezes ownership and lifecycle boundaries only; it does not choose
  the final file/module layout for Milestone 2.
- Non-Tauri hosts may still adopt the same registry architecture, but they need
  their own composition-root wiring.
