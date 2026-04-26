# ADR-012: Canonical Workflow Run Identity

## Status
Accepted

## Context
Workflow diagnostics currently mix several identities that describe different
things. GUI graph runs reuse the edit-session id as the scheduler queue id,
scheduler run id, runtime execution id, and trace execution id. Diagnostic
trace metadata also carries mutable workflow display names as side-channel
labels. Repeated runs, workflow switching, and GUI restart history can therefore
show stale workflow labels or lose the relationship between scheduler queue
state, runtime events, traces, and timing observations.

ADR-011 already requires every public workflow run to enter through the
scheduler. The identity model now needs the same strictness: one backend-owned
run id must identify one submitted workflow run across every scheduler,
runtime, diagnostics, persistence, binding, and frontend projection.

## Decision
Use this identity vocabulary for workflow execution diagnostics:

| Identity | Owner | Meaning | Wire Field |
| -------- | ----- | ------- | ---------- |
| Workflow id | Workflow catalog / saved workflow storage | Stable id of the saved workflow definition. It is the only workflow identity and label used by diagnostics. | `workflow_id` |
| Session id | Workflow execution session service | Editor or loaded-session container id used for graph editing and runtime reuse. It is never a workflow run id. | `session_id` |
| Workflow run id | Scheduler admission path | Backend-generated id for exactly one submitted workflow execution. It is created once before scheduler queue visibility. | `workflow_run_id` |
| Runtime instance id | Runtime registry / host runtime | Optional runtime resource id used for diagnostics and health attribution. It is not a workflow run id. | `runtime_instance_id` |

The scheduler-created `workflow_run_id` is the same value used as scheduler
queue id, scheduler run id, runtime execution id, trace execution id, SQLite
timing execution id, and frontend active run id. Public and cross-layer
contracts should expose `workflow_run_id` rather than ambiguous
`execution_id`, `trace_execution_id`, `queue_id`, or `run_id` fields when the
field refers to the submitted workflow run.

Rust code must reuse the existing validated attribution id types where they
cross crate boundaries:

- `pantograph_runtime_attribution::WorkflowId`
- `pantograph_runtime_attribution::ClientSessionId`
- `pantograph_runtime_attribution::WorkflowRunId`

`pantograph-workflow-service` re-exports those types as part of its public
service boundary so adapters and bindings do not invent local id wrappers.

Diagnostics contracts must remove `workflow_name`. Display names may still
exist in non-diagnostics workflow catalog UI, but diagnostics history, trace
selection, timing lookup, and scheduler projection use `workflow_id`.

Old SQLite timing rows written with mixed identity semantics are unsupported.
New code may ignore, prune, or replace incompatible rows instead of adding a
compatibility lookup path.

## Consequences

### Positive
- A user can search one `workflow_run_id` and find scheduler, runtime, trace,
  timing, and frontend active-run state for the same run.
- Repeated runs from one edit session produce distinct diagnostic histories.
- Workflow labels in diagnostics no longer depend on mutable display-name
  side channels.
- Binding and frontend contracts become easier to audit because run identity is
  explicit at the wire boundary.

### Negative
- Existing public DTOs and tests that use `run_id`, `queue_id`,
  `trace_execution_id`, or `workflow_name` must change.
- Existing SQLite timing data with old identity semantics is not guaranteed to
  appear in new diagnostics history.

### Neutral
- Lower-level execution engines may keep a generic internal `execution_id`
  field if the Pantograph embedding layer passes the canonical
  `workflow_run_id` into that field and public Pantograph APIs expose the
  canonical name.
- `runtime_instance_id` remains a diagnostic resource fact and can have a
  runtime-specific format such as a sidecar instance key.

## Guardrails
- Public and binding-facing workflow execution APIs must not accept
  caller-authored workflow run ids.
- `session_id` must not be assigned to scheduler queue, runtime execution,
  trace, timing, or frontend active-run identity.
- Diagnostics DTOs must not reintroduce `workflow_name` as an identity or label
  source.
- Cross-layer tests must prove scheduler queue state, runtime events, trace
  state, timing observations, and frontend projections share the same
  `workflow_run_id`.

## Implementation Notes
- Implementation plan:
  `docs/plans/workflow-run-identity-redesign/plan.md`
- Related scheduler-only execution decision:
  `docs/adr/ADR-011-scheduler-only-workflow-execution.md`
- Related attribution type owner:
  `docs/adr/ADR-005-durable-runtime-attribution.md`
