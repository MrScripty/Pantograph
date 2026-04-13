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
| `types.ts` | Stable diagnostics DTOs shared by the diagnostics service and frontend store/view layers, including backend-owned trace snapshot mirrors. |

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
- GUI edit-session runs may not have workflow-service scheduler state, so the
  service must also support a deterministic synthetic scheduler view derived
  from execution lifecycle events until authoritative snapshots arrive.
- Retained history must stay bounded so long sessions do not grow without
  limit.

## Decision
Use `DiagnosticsService.ts` as the single owner of diagnostics state and keep
event normalization in `traceAccumulator.ts`. The service accepts workflow
metadata, graph snapshots, execution events, runtime capability snapshots, and
session queue snapshots, then emits derived state for the frontend store to
expose declaratively. Runtime and scheduler tabs now consume additive
`RuntimeSnapshot` and `SchedulerSnapshot` workflow events when the backend emits
them, while the service can synthesize a minimal scheduler session lifecycle
for edit-session runs that do not resolve through workflow-service session
APIs. The frontend store also rejects event updates from older edit-session
execution ids before they reach the diagnostics service so switching sessions
does not splice stale run events into the current workflow view.

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
- Synthetic scheduler fallback must clear when authoritative scheduler snapshot
  data for the same session arrives.
- Edit-session diagnostics consumers must ignore workflow events whose
  `execution_id` no longer matches the active session id.

## Revisit Triggers
- Diagnostics needs durable persistence or export/replay support.
- Multiple diagnostics consumers require partitioned state rather than one
  singleton owner.
- Backend diagnostics contracts expand enough that this directory needs a
  versioned facade of its own.

## Dependencies
**Internal:** `src/services/workflow/types`, app-level diagnostics store.
**External:** None beyond the TypeScript runtime and standard structured-clone
support.

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
  workflowName: 'Example Workflow',
});
service.recordWorkflowEvent({
  type: 'Started',
  data: { workflow_id: 'workflow-1', node_count: 3, execution_id: 'run-1' },
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
- Scheduler state may also be synthesized from execution lifecycle events for
  edit-session GUI runs; that fallback remains additive and is superseded by
  streamed or fetched workflow-service snapshots when available.
