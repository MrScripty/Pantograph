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
| `templateService.ts` | Workflow template discovery/loading helpers, including the built-in tiny-sd-turbo and GGUF reranker starter workflows. |
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
Keep `templateService.ts` in the same boundary because built-in workflow
templates need the same service-level graph registration path and session-aware
loading behavior. That lets the app graph adopt the new backend-owned
eligibility model, the horseshoe insert flow, and built-in workflow bootstraps
without forcing every existing caller to migrate to package-level backends
immediately. Session-scoped graph mutation methods now also return the updated
graph snapshot from core so legacy callers can stay aligned with backend-owned
state.

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
- Edit mutation methods must forward backend-owned graph state rather than
  reconstructing local graph changes client-side.
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
- Built-in template loading grows into a separate catalog or remote-discovery
  subsystem with its own lifecycle and persistence concerns.
- Template count or complexity grows enough that per-template validation needs a
  dedicated service boundary.

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
- Add/update/remove/move mutation methods return the updated graph for callers
  that need to refresh rendered state directly.
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
- Workflow templates loaded through `templateService.ts` must remain valid
  `WorkflowTemplate` objects whose data-graph node/edge shapes match the
  workflow DTO contracts in `types.ts`.
- Built-in templates that demonstrate inference-family nodes must stay aligned
  with the backend-owned node registry and port contracts shipped in the same
  build.
