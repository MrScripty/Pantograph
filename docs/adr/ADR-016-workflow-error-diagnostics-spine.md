# ADR-016: Workflow Error Diagnostics Spine

## Status
Accepted

## Context
Workflow failures must be debuggable from the run trace. Earlier behavior could
surface an error in the graph editor while durable projections still showed the
run as running, or could bury the useful failure text inside a secondary event.

Pantograph already has a local diagnostics ledger and projection system. The
error path needs to use that primary system rather than creating duplicate JSON
fallback traces.

## Decision
Workflow-run failures are recorded as first-class
`diagnostic.error_occurred` ledger events. These events carry phase, component,
severity, recoverability, scope, sanitized message text, bounded technical
detail, related event IDs, and optional direct causality.

Secondary events such as `run.terminal` and scheduler model lifecycle failures
may carry `canonical_error_event_id` links back to the authoritative error
event. They remain compatibility and lifecycle facts, but they are not the
source of detailed error truth.

Workflow-service domain code must transition live scheduler/session state out
of running before diagnostics projections are refreshed. The diagnostics ledger
records what happened; it is not the state machine that mutates active session
state.

If the diagnostics ledger cannot record an error, callers receive an explicit
diagnostics-unavailable state. Pantograph does not write duplicate JSONL
fallback diagnostics for workflow-run errors.

Frontend surfaces consume backend error links and projections. They may keep
local focus/filter state, but they must not infer or repair run status from
error strings.

## Consequences

### Positive
- Workflow errors become durable, ordered facts in the run trace.
- GUI error messages can deep-link to the diagnostic event that explains them.
- Failed projections can recover from canonical error events when terminal
  events are absent or older streams only contain legacy terminal/node failure
  events.
- Scheduler/session state ownership remains explicit and testable.

### Negative
- Error-producing paths must use the recorder facade consistently.
- Some secondary event links can only be populated when the canonical error is
  recorded before the secondary event.
- Ledger unavailability is a visible failure mode instead of being hidden by a
  duplicate fallback file.

### Neutral
- Old diagnostics ledgers are not migrated into the new event shape.
- Node authors do not import diagnostics ledger APIs; runtime/host wrappers own
  run and node error context.

## Guardrails
- Do not parse free-form error text to choose diagnostics phases when typed
  producer context is available.
- Do not append workflow-run diagnostics to alternate JSON files.
- Keep recorder APIs phase- and scope-specific enough that call sites do not
  hand-build event payloads.
- Keep frontend diagnostics navigation tied to backend-provided
  `workflow_run_id` and `diagnostic_event_id` values.

## Implementation Notes
- Implementation plan:
  `docs/plans/workflow-error-diagnostics-spine/plan.md`.
- Related projection boundary:
  `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`.
- Related scheduler execution boundary:
  `docs/adr/ADR-011-scheduler-only-workflow-execution.md`.
- Related workflow run identity boundary:
  `docs/adr/ADR-012-canonical-workflow-run-identity.md`.
