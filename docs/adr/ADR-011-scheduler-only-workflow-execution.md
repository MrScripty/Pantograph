# ADR-011: Scheduler-Only Workflow Execution

## Status
Accepted

## Context
Pantograph workflow execution depends on scheduler-visible queue state,
runtime admission, runtime preflight, diagnostics trace correlation, and
workflow execution session identity. A public direct-run API lets callers bypass
those controls, which can leave diagnostics empty, route work around runtime
stability policy, and produce workflow ids or run ids that cannot be reconciled
with scheduler state.

The removed direct surfaces included Rust service and embedded-runtime methods,
Tauri raw graph execution commands, UniFFI and Rustler binding exports, and
frontend raw graph execution helpers.

## Decision
All public workflow runs must be submitted through workflow execution sessions.
The supported execution flow is:

1. Create or reuse a workflow execution session.
2. Submit work with the scheduler session run API.
3. Inspect queue, status, diagnostics, and traces through scheduler-owned
   session surfaces.
4. Close or keep alive the session according to the caller's lifecycle needs.

No compatibility facade is provided for direct workflow execution. The private
`workflow_run_internal` implementation remains a scheduler-owned service detail
and may only be called after queue admission from
`run_workflow_execution_session`.

Scheduler-owned queue controls, including cancellation and reprioritization,
must emit typed scheduler diagnostic events when diagnostics are configured.
Those events record control facts, actor scope, and the queue state observed
before the mutation, while terminal execution truth remains owned by `run.*`
lifecycle events.

Scheduler admission must also emit a typed scheduler diagnostic event before
the run lifecycle starts. This keeps the scheduler's admission decision
auditable without treating `run.started` as the owner of scheduler control
semantics.

Binding and frontend contracts must expose scheduler-backed session execution,
not raw graph or direct workflow-run helpers. Host/runtime traits may still own
low-level execution mechanics, but those mechanics are implementation surfaces,
not public caller entrypoints.

## Consequences

### Positive
- Queue diagnostics, runtime diagnostics, and trace state share one execution
  identity path.
- Runtime preflight and scheduler admission cannot be skipped by frontend,
  binding, or Rust API callers.
- The graph editor and language bindings converge on the same stable execution
  contract.
- Scheduler queue controls become auditable without allowing callers to author
  raw diagnostics facts.

### Negative
- Existing direct-run consumers must migrate to session create/run/close calls.
- Simple one-shot execution requires an explicit session lifecycle.

### Neutral
- Low-level host execution remains necessary behind the scheduler boundary.
- Historical plans and completed-plan notes may still mention removed direct
  APIs as past implementation context, but active source surfaces must not
  reintroduce them.

## Guardrails
- `scripts/check-scheduler-only-workflow-execution.sh` fails when active source
  reintroduces public direct execution APIs.
- `scripts/check-uniffi-embedded-runtime-surface.sh` rejects the removed UniFFI
  `workflow_run` method in generated metadata.
- Scheduler control APIs must route durable audit facts through typed
  diagnostic event payloads owned by the backend scheduler boundary.
