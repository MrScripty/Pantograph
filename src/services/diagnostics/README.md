# src/services/diagnostics

## Purpose
This directory contains the diagnostics-domain service layer for Pantograph's
workflow debugger. It exists so trace accumulation, run/node summarization, and
diagnostics lifecycle policy stay outside Svelte components and outside Tauri
transport adapters.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `DiagnosticsService.ts` | Framework-agnostic owner for diagnostics state, selection, trace recording, and listener notifications. |
| `traceAccumulator.ts` | Trace accumulation helpers that normalize workflow events and graph context into diagnostics snapshots. |
| `types.ts` | Stable diagnostics DTOs shared by the diagnostics service and frontend store/view layers, including backend-owned trace snapshot mirrors and additive workflow-session inspection state. |

## Problem
Pantograph needs an internal diagnostics surface for workflow execution, but the
GUI should not infer timing, waiting, or node lifecycle state ad hoc inside
components. Without a dedicated diagnostics service boundary, trace logic would
spread across toolbar handlers, global stores, and view code.

## Constraints
- Diagnostics state must remain additive to the existing workflow execution
  path.
- Services in this directory must stay framework-agnostic; Svelte stores and
  components consume snapshots but do not own trace business logic.
- Trace accumulation must tolerate additive event contracts and partial event
  availability from the backend.
- Workflow-service-backed capability and scheduler snapshots must coexist with
  event-derived run traces without introducing a second source of truth for run
  execution state.
- Retained history must stay bounded so long sessions do not grow without
  limit.

## Decision
Use backend diagnostics projections as the source of truth for run history,
runtime state, scheduler state, timing expectations, and workflow labels.
Frontend stores mirror those projections by `workflow_id`, `session_id`, graph
context, and backend-authored `workflow_run_id`; Svelte components render the
projection without inventing fallback run identities or workflow display names.

## Alternatives Rejected
- Accumulate diagnostics directly inside `WorkflowToolbar.svelte`.
  Rejected because presentation components should not own workflow debugging
  state machines or retention rules.
- Add diagnostics state directly to `workflowStore.ts`.
  Rejected because workflow editing state and diagnostics inspection state have
  different lifecycles and would become harder to reason about together.

## Invariants
- `DiagnosticsService.ts` remains the only owner of diagnostics selection and
  retained trace history.
- Trace accumulation stays deterministic for the same ordered event stream.
- Missing optional workflow metadata must degrade cleanly to `null`, not throw.
- Per-run retained event history is bounded by the configured limit.
- Runtime and scheduler snapshots may refresh independently, but they must land
  in the same diagnostics state owner as run traces.
- Additive workflow-session inspection state may arrive through direct
  diagnostics fetches before equivalent event-driven projections, so consumers
  must tolerate partial producer coverage without discarding the last known
  backend snapshot.
- Edit-session diagnostics consumers must ignore workflow events whose
  `workflow_run_id` no longer matches the active workflow run id.
- I/O artifact DTOs carry typed `retention_state` values. Consumers must not
  infer deleted, expired, external, truncated, or too-large states from
  `payload_ref` presence.
- I/O artifact responses carry `retention_summary` counts derived from backend
  projections. Consumers should display those counts instead of rebuilding
  completeness summaries from raw ledger events.
- Run-list responses carry backend-owned `facets` derived from materialized
  projections. Consumers should use those counts for mixed-version and policy
  summaries instead of rebuilding them from raw ledger events or sampled pages.
- Library usage query DTOs include optional `workflow_run_id` filters so
  frontend consumers can request selected-run asset usage without reconstructing
  active-run Library state from raw ledger events.
- Retention cleanup DTOs mirror the backend cleanup command/result shape so GUI
  controls can display expired-artifact counts without mutating local
  diagnostics state optimistically.
- Pumas model delete audit DTOs mirror the backend command response so GUI
  controls can display delete/audit outcomes without synthesizing local event
  ids or Library state.
- Pumas HuggingFace search audit DTOs mirror the backend command response so
  GUI controls can display search results and audit outcomes without
  synthesizing Library usage facts.

## Revisit Triggers
- Diagnostics needs durable persistence or export/replay support.
- Multiple diagnostics consumers require partitioned state rather than one
  singleton owner.
- Backend diagnostics contracts expand enough that this directory needs a
  versioned facade of its own.

## Dependencies
**Internal:** `src/services/workflow/types`, app-level diagnostics store.
**External:** TypeScript runtime and standard structured-clone support.

## Related ADRs
- None identified as of 2026-04-12.
- Reason: the diagnostics service boundary is additive and local to the
  frontend architecture for now.
- Revisit trigger: diagnostics becomes a stable cross-process or persisted
  contract surface.

## Usage Examples
```ts
import { DiagnosticsService } from './DiagnosticsService';

const service = new DiagnosticsService();
service.updateWorkflowMetadata({
  workflowId: 'workflow-1',
});
service.recordWorkflowEvent({
  type: 'Started',
  data: { workflow_id: 'workflow-1', node_count: 3, workflow_run_id: 'run-1' },
});
```

## API Consumer Contract
- Callers update workflow metadata and graph context separately from event
  recording.
- `recordWorkflowEvent()` is append-only from the caller's perspective; callers
  do not mutate runs or nodes directly.
- Listener callbacks receive full diagnostics snapshots after each state change.
- `clearHistory()` removes retained runs and selection state but preserves the
  current workflow metadata context.
- Compatibility policy is additive for diagnostics DTOs unless a future plan
  explicitly versions the diagnostics contract.

## Structured Producer Contract
- `types.ts` is the structured producer for diagnostics snapshots.
- `types.ts` also mirrors backend-owned `WorkflowTraceSnapshotRequest` and
  `WorkflowTraceSnapshotResponse` contracts using the Rust wire casing for
  direct inspection reads.
- `WorkflowDiagnosticsState.runOrder` is ordered most-recent-first.
- `DiagnosticsRunTrace.events` is retained in arrival order and trimmed from the
  oldest end when the configured event limit is exceeded.
- `graphFingerprintAtStart` captures the graph fingerprint known when the run is
  first observed; later graph changes do not rewrite that field.
- Runtime and scheduler snapshots are last-write-wins views over workflow
  service responses keyed by current workflow and current session identity.
- `WorkflowDiagnosticsProjection.context` mirrors backend-owned requested
  filters, event source workflow run id, relevant workflow run id, and relevance
  decision for app stores that render diagnostics snapshots.
- `WorkflowDiagnosticsProjection.currentSessionState` is an additive,
  backend-owned session inspection mirror; producer paths may omit it until the
  backend explicitly forwards the inspection snapshot.
- Scheduler state is backend-owned. GUI runs must be submitted through the
  scheduler so queued/running rows and runtime traces share the same
  `workflow_run_id`.
- Scheduler projection DTOs include delayed run status and typed scheduler
  timeline event labels for delay/model lifecycle audit rows. Consumers must
  render those fields from projection DTOs instead of parsing payload JSON.
