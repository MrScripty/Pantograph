# src/services/workflow

## Purpose
This directory contains Pantograph’s workflow-domain service layer. It wraps
Tauri commands, mock fallbacks, and workflow-specific DTOs so app code can load
definitions, mutate session graphs, and coordinate execution without depending
on raw invoke payloads.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `WorkflowService.ts` | Main client-side workflow service, including session lifecycle, graph mutation, connection-intent commands, and atomic insert-and-connect. |
| `types.ts` | App-local workflow DTO mirrors used by the service and legacy callers. |
| `mocks.ts` | Mock workflow data and behaviors used when the app runs in mock mode. |
| `templateService.ts` | Workflow template discovery/loading helpers. |
| `groupTypes.ts` | Node-group result and mapping types used by workflow editing flows. |

## Problem
The app still has workflow-aware callers outside the reusable graph package.
They need a stable service boundary for Tauri commands, especially while the
graph UI migrates from boolean validation toward revision-aware connection
intent.

## Constraints
- The service must tolerate both real Tauri mode and mock mode.
- Existing app callers still expect `WorkflowService` to track the current
  execution/session id internally.
- DTOs must stay aligned with Rust serialization and the package contracts.

## Decision
Keep `WorkflowService.ts` as the legacy-friendly workflow adapter and extend it
with `getConnectionCandidates`, `connectAnchors`, and `insertNodeAndConnect`.
That lets the app graph adopt the new backend-owned eligibility model and the
horseshoe insert flow without forcing every existing caller to migrate to
package-level backends immediately.

## Alternatives Rejected
- Remove `WorkflowService` and switch every app caller to `TauriWorkflowBackend`
  in one step.
  Rejected because the app still has non-package consumers of the workflow
  service boundary.
- Keep connection-intent methods only in the package backend adapter.
  Rejected because the app graph still routes through this service today.

## Invariants
- `currentExecutionId` must refer to the active editable session before any
  session-scoped graph mutation method runs.
- Expected connection rejection is returned as structured data, not thrown as an
  exception.
- Insert-and-connect must remain atomic from the caller’s perspective: the
  service returns either an updated graph or a structured rejection.
- Mock-mode payload shapes must remain compatible enough for callers to compile
  and branch safely.

## Revisit Triggers
- The app graph and all remaining callers migrate to package backends directly.
- Workflow service state needs to support multiple simultaneous active sessions.
- Mock mode requires full connection-intent semantics instead of shape-only
  placeholders.

## Dependencies
**Internal:** `src-tauri` workflow commands, `src/backends`, app workflow
  stores/components, mirrored workflow types.

**External:** `@tauri-apps/api/core` invoke support.

## Related ADRs
- None.
- Reason: the service boundary is still transitional.
- Revisit trigger: the service is either removed or promoted to a formal app SDK
  surface.

## Usage Examples
```ts
import { workflowService } from './WorkflowService';

const candidates = await workflowService.getConnectionCandidates(
  { node_id: 'llm', port_id: 'response' },
  undefined,
  'graph-revision-token'
);

const inserted = await workflowService.insertNodeAndConnect(
  { node_id: 'llm', port_id: 'response' },
  'text-output',
  'graph-revision-token',
  { position: { x: 480, y: 160 } }
);
```

## API Consumer Contract (Host-Facing Modules)
- Callers must establish or inherit `currentExecutionId` before using
  session-scoped graph mutation methods.
- `getConnectionCandidates` returns compatible existing targets and insertable
  node types for one source anchor.
- `connectAnchors` and `insertNodeAndConnect` require the caller to provide the
  graph revision it used to derive UI state; a stale revision returns
  `accepted: false` with a rejection.
- Mock mode may return placeholder data for some methods; callers should not
  assume mock behavior fully matches native runtime semantics.

## Structured Producer Contract (Machine-Consumed Modules)
- Service DTOs in `types.ts` mirror Rust field names and rejection enums.
- `ConnectionCommitResponse.rejection.reason` uses stable snake_case labels.
- `graph_revision` is opaque and volatile; callers must refresh it from the
  latest graph before retrying a rejected stale commit.
