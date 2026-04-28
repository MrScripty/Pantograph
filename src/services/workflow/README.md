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
| `WorkflowCommandService.ts` | Focused backend-owned queue and retention command service inherited by `WorkflowService` and tested without loading the graph runtime. |
| `WorkflowService.commands.test.ts` | Tauri mock IPC tests proving queue and retention commands return backend-owned results without optimistic client replacement. |
| `WorkflowProjectionService.ts` | Focused projection service for scheduler timeline, scheduler estimate, run-list, selected-run, local Network, I/O artifact, and warm Library usage reads used by `WorkflowService` and projection boundary tests. |
| `WorkflowService.projections.test.ts` | Tauri mock IPC tests proving scheduler timeline events, run-list facets, selected-run scheduler estimate fields, local Network scheduler-load/placement facts, and warm projection freshness state survive the service boundary. |
| `workflowServiceErrors.ts` | Typed workflow command error normalizer and invoke wrapper for backend JSON error envelopes. |
| `workflowServiceErrors.test.ts` | Unit coverage for backend error-envelope parsing and transport-error fallback behavior. |
| `workflowConnectionActions.ts` | Focused Tauri invoke helpers for connection-intent candidate, commit, and edge-insert commands. |
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
- Backend workflow error envelopes must keep their `code`, `message`, and
  `details` through the TypeScript service boundary.

## Decision
Keep `WorkflowService.ts` as the legacy-friendly workflow adapter and extend it
with `getConnectionCandidates`, `connectAnchors`, `insertNodeAndConnect`,
`previewNodeInsertOnEdge`, and `insertNodeOnEdge`.
Keep `templateService.ts` in the same boundary because built-in workflow
templates need the same service-level graph registration path and session-aware
loading behavior. That lets the app graph adopt the new backend-owned
eligibility model, the horseshoe insert flow, cursor-hit edge insertion, and
built-in workflow bootstraps without forcing every existing caller to migrate
to package-level backends immediately. Session-scoped graph mutation methods
now also return the updated graph snapshot from core so legacy callers can stay
aligned with backend-owned state. The service also refreshes
`currentRunExecutionId` from the first execution-scoped workflow event while
preserving `currentExecutionId` as the editable session owner, so diagnostics
and legacy consumers can follow ad hoc or session-backed runs without
overwriting the session id that mutation commands still need.
Run identity updates now consume the package-level workflow event ownership
projection so `WorkflowService.ts` and workflow execution reducers agree on
active-run identity. Backend-authored event `ownership` payloads are
authoritative over raw execution-id fields when native Tauri events provide
them.
Connection-intent invoke wiring now lives in
`workflowConnectionActions.ts` so `WorkflowService.ts` stays focused on
session ownership, mock branching, and legacy app-facing method shapes.
Projection invoke wiring now lives in `WorkflowProjectionService.ts` so the
scheduler timeline, run-list, selected-run, I/O artifact, and warm Library
usage read paths can be tested without loading the graph package runtime.
`WorkflowService` inherits that boundary so existing GUI callers keep the same
method names while projection DTO tests stay focused on Tauri request/response
contracts.
Run-list and run-detail projection service tests consume the shared
`pantograph-workflow-service` contract fixture so frontend request/response
coverage stays aligned with Rust public DTO deserialization.
Run-list projection DTOs preserve backend-provided client, client-session,
bucket, and workflow execution-session scope fields so Scheduler pages can
render authority context and future queue-control targets without querying
selected-run details or raw diagnostic events.
Run-list projection DTOs also preserve scheduler-selected runtime, device, and
network-node placement fields and filters so Scheduler and Diagnostics pages
can consume placement facts without parsing scheduler payload JSON.
Run-list, run-detail, and scheduler-estimate DTOs preserve scheduler
model-cache posture as typed fields so workbench pages do not parse estimate
payload JSON for cache state.
Workflow command invoke paths that back workbench projections and queue,
retention, Network, and runtime-status reads use `invokeWorkflowCommand`, which
normalizes Tauri's JSON error strings into typed `WorkflowServiceError`
instances while preserving backend error codes and details.
Local Network scheduler-load DTOs include backend-provided active and queued
workflow run id arrays so the Network page can show selected-run placement
without inferring scheduler truth from counts or selected-run context.
They also preserve typed run-placement records with workflow execution-session
id, runtime-loaded posture, scheduler model-cache posture, and required
backend/model facts for selected-run Network panels.
Queue and retention command methods live in `WorkflowCommandService.ts` so
their backend-owned no-optimistic-update contract can be tested without
importing the full graph event runtime required by `WorkflowService.ts`.
Retention cleanup is a backend-owned command that returns cleanup counts and
projection-derived state; frontend services forward the request and do not
remove artifact cards optimistically.
Pumas model deletion is exposed as a backend-owned audited command; frontend
services forward the model id and preserve the backend delete/audit response
without inventing local Library state.
Pumas HuggingFace search is exposed as a backend-owned audited command;
frontend services forward query bounds and preserve backend model/audit
responses without synthesizing Library usage facts.
Pumas HuggingFace download startup is exposed as a backend-owned audited
command; frontend services forward the structured Pumas download request and
preserve the backend download/audit response.

## Alternatives Rejected
- Remove `WorkflowService` and switch every app caller to `TauriWorkflowBackend`
  in one step.
  Rejected because the app still has non-package consumers of the workflow
  service boundary.
- Keep connection-intent methods only in the package backend adapter.
  Rejected because the app graph still routes through this service today.
- Keep Tauri connection-intent invoke wiring in `WorkflowService.ts`.
  Rejected because the file had become an oversized insertion point for both
  session state and backend connection-command normalization.

## Invariants
- `currentExecutionId` must refer to the active editable session before any
  session-scoped graph mutation method runs.
- `currentRunExecutionId` must only represent the active workflow run and must
  be reset when session ownership changes.
- `currentRunExecutionId` updates must use the shared workflow event ownership
  projection; backend-authored `ownership` relevance must not be filtered again
  with a service-local current-run comparison.
- Native event `ownership` payloads are authoritative for run identity when
  present; mock and legacy event paths may still fall back to `execution_id`.
- Edit mutation methods must forward backend-owned graph state rather than
  reconstructing local graph changes client-side.
- Expected connection rejection is returned as structured data, not thrown as an
  exception.
- Insert-and-connect must remain atomic from the caller’s perspective: the
  service returns either an updated graph or a structured rejection.
- Edge insertion preview must stay side-effect free; replacing the existing edge
  is only allowed through `insertNodeOnEdge`.
- Connection-intent invoke helpers stay in `workflowConnectionActions.ts` so
  the service keeps one legacy-facing wrapper surface while the raw Tauri
  command wiring remains focused and reusable.
- Scheduler timeline, run-list, selected-run, local Network, I/O artifact, and
  Library usage projection invoke helpers stay in
  `WorkflowProjectionService.ts`; `WorkflowService` must not reimplement those
  methods separately.
- Run-list request coverage includes scope and accepted-at range filters so
  frontend services preserve the backend projection contract.
- Run-list request coverage includes selected runtime, selected device, and
  selected network-node filters so frontend services preserve the placement
  projection contract.
- Run projection service-boundary coverage includes typed scheduler
  model-cache posture so frontend callers preserve cache-state DTO fields
  without parsing raw payloads.
- Run-list and run-detail service-boundary tests must keep consuming the shared
  Rust contract fixture for cross-layer DTO acceptance.
- Library usage request coverage includes active-run `workflow_run_id`
  filtering so frontend services preserve the backend projection contract.
- Local Network request coverage includes active/queued run ids and
  run-placement facts, including scheduler model-cache posture, so frontend
  services preserve the backend scheduler-load contract.
- Retention cleanup requests must use `workflow_retention_cleanup_apply` and
  preserve the backend cleanup result exactly.
- Mock-mode payload shapes must remain compatible enough for callers to compile
  and branch safely.
- Workflow execution must use a backend-owned session. Raw graph execution is
  not exposed because scheduler diagnostics and runtime admission depend on
  session-scoped run lifecycle state.
- Run-list projection reads must preserve backend-owned facets, projection
  state, scheduler estimate fields, queue-placement fields, and delayed status
  without reconstructing them client-side.
- Scheduler estimate projection reads must return backend-authored estimate
  DTOs exactly and must not parse raw scheduler event payloads client-side.
- I/O artifact projection reads must preserve backend query/filter shapes,
  including producer and consumer node filters, instead of widening requests
  and filtering artifact pages in `WorkflowService`.
- Workbench-facing workflow command methods must throw `WorkflowServiceError`
  when the backend returns a `WorkflowErrorEnvelope`; callers must not parse
  raw JSON error strings.
- Queue and retention command methods must return backend-authored command
  responses exactly. Frontend code may show pending state while waiting, but it
  must not synthesize replacement queue state or retention policy facts.
- Pumas model delete commands must return backend-authored delete/audit
  responses exactly. Frontend code may refresh Library projections afterward,
  but it must not synthesize audit event ids or local deletion state.
- Pumas HuggingFace search commands must return backend-authored model lists and
  audit event ids exactly. Frontend code may display the result, but it must not
  turn searches into Library usage rows locally.
- Pumas HuggingFace download-start commands must return backend-authored
  download ids and audit event ids exactly. Frontend code may display progress
  from Pumas, but it must not synthesize download audit facts locally.

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

const preview = await workflowService.previewNodeInsertOnEdge(
  'edge-42',
  'embedding',
  'graph-revision-token',
);
```

## API Consumer Contract (Host-Facing Modules)
- Callers must establish or inherit `currentExecutionId` before using
  session-scoped graph mutation methods.
- Diagnostics and other run-scoped consumers must use `currentRunExecutionId`
  rather than reusing `currentExecutionId`, because session owners and run ids
  are intentionally distinct for session-backed execution.
- Add/update/remove/move mutation methods return the updated graph for callers
  that need to refresh rendered state directly.
- `getConnectionCandidates` returns compatible existing targets and insertable
  node types for one source anchor.
- `connectAnchors`, `insertNodeAndConnect`, `previewNodeInsertOnEdge`, and
  `insertNodeOnEdge` require the caller to provide the graph revision it used
  to derive UI state; a stale revision returns `accepted: false` with a
  rejection.
- `previewNodeInsertOnEdge` must not mutate the graph; `insertNodeOnEdge`
  atomically replaces one existing edge with two when a valid bridge exists.
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
- `WorkflowIoArtifactQueryResponse` carries backend-authored
  `retention_state` values. Mock and native responses must preserve that field
  shape so I/O Inspector callers can render retention state without guessing
  from `payload_ref`. The same response carries `retention_summary` counts
  derived from backend projections.
- Mock projection responses must track backend projection versions so page
  logic exercises the same rebuild/freshness contracts in mock and native
  modes.
- `WorkflowRunListQueryResponse.facets` is a backend-owned comparison summary.
  Native and mock paths must preserve the field so workbench pages do not infer
  workflow-version or policy counts from partial client-side pages.
- `WorkflowProjectionService` forwards `workflow_scheduler_timeline_query`,
  `workflow_run_list_query`, `workflow_run_detail_query`,
  `workflow_io_artifact_query`, and `workflow_library_usage_query` requests
  under a `{ request }` envelope and preserves backend DTO fields exactly for
  projection consumers.
- Workbench-facing workflow commands preserve backend error categories through
  `WorkflowServiceError.code`. Non-envelope IPC or setup failures are
  classified as `transport_error` so they are not confused with backend policy
  rejections.
- `updateRetentionPolicy`, `cancelSessionQueueItem`,
  `adminCancelQueueItem`, `adminReprioritizeQueueItem`,
  `adminPushQueueItemToFront`, `reprioritizeSessionQueueItem`, and
  `pushSessionQueueItemToFront` resolve only with backend response DTOs.
  Callers refresh projections or apply the returned backend policy; they do not
  optimistically mutate queue rows or retention facts.
- Mock retention policy responses include the same first-pass settings groups
  as native responses so policy panels exercise one DTO shape in mock and
  native modes.
