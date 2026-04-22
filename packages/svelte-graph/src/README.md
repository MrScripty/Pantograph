# packages/svelte-graph/src

## Purpose
This directory contains the reusable source for Pantograph's Svelte graph
editor package. It exists to keep graph interaction rules, workflow store
factories, and package-facing contracts in one place so the app shell does not
fork core canvas behavior.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `components/` | Reusable graph UI, including the main canvas, reconnect affordances, and horseshoe selector. |
| `connectionDragState.ts` | Shared state machine helpers that distinguish normal connect drags from explicit reconnect drags and gate reconnect cleanup. |
| `horseshoeDragSession.ts` | Shared visibility and anchor lifecycle for the horseshoe insert UI during active drags. |
| `horseshoeInsertFeedback.ts` | Shared pending/rejection feedback state and status-label resolution for horseshoe insert outcomes. |
| `horseshoeInvocation.ts` | Shared `Space` open/confirm decisions, pointer-anchor freeze rules, and user-facing blocked-reason strings for horseshoe invocation. |
| `stores/` | Package store factories for workflow, session, and view state. |
| `types/` | Stable TypeScript contracts for graph, backend, and group APIs. |
| `workflowEventOwnership.ts` | Projection helper for backend-provided workflow execution ids, active run identity, and stale-event relevance. |
| `workflowMiniMap.ts` | Shared minimap color projection for backend node categories and graph group nodes. |
| `workflowMiniMap.test.ts` | Unit coverage for reusable minimap color projection. |
| `index.ts` | Package export surface consumed by the app shell and any external package users. |

## Problem
Pantograph needs one reusable graph interaction core across both the package
editor and the app-specific editor. Without a shared source tree, connect,
reconnect, horseshoe insertion, and workflow-store behavior drift quickly and
produce mismatched user-visible bugs.

## Constraints
- Package code must stay reusable across backend implementations exposed through
  the package context and backend interfaces.
- Backend-owned graph mutations must not be reimplemented locally in the UI.
- Drag interaction helpers must stay small and testable because the host graph
  components are already large.

## Decision
Keep reusable interaction policy in small top-level helpers such as
`connectionDragState.ts`, `horseshoeDragSession.ts`, and
`horseshoeInvocation.ts`, then compose them from the package graph component
and the app graph wrapper. Export those helpers through `index.ts` so the app
layer can consume the same connect/reconnect rules instead of copying them.
Keep workflow execution event identity in `workflowEventOwnership.ts` as an
explicit projection object. Backend-authored event `ownership` payloads from
Tauri are authoritative for event identity, active run identity, and relevance;
the helper only falls back to legacy `execution_id` fields for events that do
not yet carry that backend projection.
Keep reusable minimap color projection in `workflowMiniMap.ts` so the graph
component does not own backend category presentation policy inline.

## Alternatives Rejected
- Keep connect/reconnect state management inline in both graph components.
  Rejected because the occupied-output bug showed that duplicated interaction
  rules drifted and became harder to reason about.
- Move all graph interaction policy into the app layer only.
  Rejected because the package graph would lose the behavior contract the app is
  supposed to share.

## Invariants
- Shared graph interaction helpers in this directory remain the source of truth
  for package/app horseshoe gating and connect-vs-reconnect ownership.
- Horseshoe keyboard policy stays shared: first `Space` requests open, second
  `Space` confirms the highlighted insert only after the menu is open and has a
  valid selection.
- Open horseshoe sessions freeze anchor updates; pointer motion is interpreted
  as menu selection input instead of menu repositioning.
- Horseshoe insert feedback stays visible until the backend accepts the insert
  or the interaction is explicitly cleared; rejected inserts must not collapse
  into a silent no-op.
- Package exports must remain additive and backward compatible unless a planned
  breaking change is documented separately.
- Helpers in this directory do not perform backend mutations directly; they
  compute state and decisions for callers that own side effects.
- `workflowEventOwnership.ts` must only project backend-supplied execution ids
  and active-run relevance; when backend `ownership` is present, it must not
  override relevance with package-local current-run comparisons.
- Backend-authored workflow event `ownership` payloads take precedence over
  legacy raw `execution_id` fields.
- `workflowMiniMap.ts` must keep group-node coloring ahead of backend category
  coloring and provide a stable fallback color for unknown node types.

## Revisit Triggers
- A second non-Pantograph consumer needs a different reconnect policy.
- Graph interaction logic outgrows helper-level extraction and needs a dedicated
  package submodule or ADR.
- The app stops wrapping package graph behavior and can consume the package
  component directly.

## Dependencies
**Internal:** `packages/svelte-graph/src/components`,
`packages/svelte-graph/src/stores`, `packages/svelte-graph/src/types`.

**External:** Svelte 5 and `@xyflow/svelte`.

## Related ADRs
- None identified as of 2026-03-07.
- Reason: the interaction-policy extraction stays inside the package boundary.
- Revisit trigger: package exports become a documented external SDK surface.

## Usage Examples
```ts
import {
  createConnectionDragState,
  startConnectionDrag,
  supportsInsertFromConnectionDrag,
} from '@pantograph/svelte-graph';

const dragState = startConnectionDrag();
const canInsert = supportsInsertFromConnectionDrag(dragState);
```

## API Consumer Contract
- Consumers may import helpers from `index.ts`; deep imports into internal
  files are not part of the stable package contract.
- Connection-drag helpers expose pure state transitions only. Callers own event
  wiring, backend calls, and UI cleanup.
- Horseshoe helpers return explicit blocked reasons instead of silently failing,
  and callers should surface or log those reasons consistently.
- New exports in this directory should preserve existing names and semantics for
  current app consumers unless a migration is documented.
- Consumers that need execution-event relevance should use
  `projectWorkflowEventOwnership()` instead of composing lower-level helper
  calls.

## Structured Producer Contract
- `index.ts` is the structured producer for package consumers. Export names and
  the shape of exported helper inputs/outputs are the stable contract.
- Top-level helper modules return plain objects with explicit boolean/string
  fields so callers can serialize, test, or inspect decisions without framework
  internals.
- `WorkflowEventOwnershipProjection` contains event execution id, active
  execution id, and relevance as the current package-level execution event
  projection contract.
- If a helper contract changes, package exports and downstream app imports must
  be updated in the same change set; do not leave mixed old/new interaction
  semantics in the repo.
