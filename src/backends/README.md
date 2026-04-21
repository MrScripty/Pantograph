# src/backends

## Purpose
This directory contains app-level backend adapters that translate Pantograph UI
calls into concrete transport operations. The boundary exists so the app can use
the reusable graph package’s `WorkflowBackend` contract without exposing Tauri
`invoke` details to components or stores.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `TauriWorkflowBackend.ts` | Maps the package backend interface onto Pantograph’s Tauri commands and session lifecycle. |

## Problem
Pantograph uses the reusable graph package internally, but the app still needs a
native bridge for node definitions, graph sessions, and interactive
connection-intent operations. Without a dedicated adapter, graph components
would depend directly on invoke command names and payload conventions.

## Constraints
- The adapter must preserve the package-level `WorkflowBackend` contract.
- Tauri invoke names and payloads must stay aligned with Rust serde shapes.
- The app already has legacy services and stores, so the adapter cannot assume
  every caller has migrated at once.

## Decision
Use `TauriWorkflowBackend.ts` as the only app-layer implementation of
`WorkflowBackend`. It now forwards `getConnectionCandidates`,
`connectAnchors`, `insertNodeAndConnect`, `previewNodeInsertOnEdge`, and
`insertNodeOnEdge` alongside the existing session editing calls so the package
graph can use revision-aware connection guidance, drag-time insert flows, and
cursor-hit edge replacement unchanged inside the app. Structural graph edits
now return authoritative graph snapshots from the backend, which the shared
stores apply directly instead of reconstructing local state first.
Group create, ungroup, and update-port methods now call session-scoped Tauri
commands that return the same graph mutation response shape, keeping collapsed
group nodes and boundary edges backend-owned.

## Alternatives Rejected
- Call Tauri `invoke` directly from `WorkflowGraph.svelte`.
  Rejected because components would duplicate transport and error handling.
- Maintain a separate app-only graph backend interface.
  Rejected because it would diverge from the reusable package contract.

## Invariants
- `TauriWorkflowBackend.ts` must continue to satisfy the package
  `WorkflowBackend` interface.
- Backend errors caused by transport/session failure should throw; expected
  incompatibility should come back as structured commit rejection.
- Add/update/remove/move edit commands must return the graph snapshot produced
  by core so app stores can render backend-owned state directly.
- Group create, ungroup, and port edits must use backend-owned mutation
  responses, not the legacy group-only Tauri commands.
- Graph revisions are treated as opaque values and echoed unchanged to Rust.
- Insert commands must remain atomic at the adapter boundary; the adapter should
  not split insert and connect into separate invokes.
- Edge-insert preview must remain non-mutating and must not synthesize client
  fallback state when Rust rejects the candidate bridge.

## Revisit Triggers
- A non-Tauri production transport is introduced for the app.
- Invoke payloads become versioned independently from the package contracts.
- Legacy app services are removed and the adapter can be simplified further.

## Dependencies
**Internal:** `src-tauri` workflow commands, `packages/svelte-graph/src/types`,
`packages/svelte-graph/src/backends`.

**External:** `@tauri-apps/api/core` invoke support.

## Related ADRs
- None.
- Reason: the adapter remains a straightforward transport binding.
- Revisit trigger: multiple backend adapters need an explicit selection or
  fallback strategy.

## Usage Examples
```ts
import { TauriWorkflowBackend } from '../backends/TauriWorkflowBackend';

const backend = new TauriWorkflowBackend();
const session = await backend.createSession({ nodes: [], edges: [] });
```

## API Consumer Contract (Host-Facing Modules)
- App code consuming this adapter should use package-level backend methods, not
  hardcoded invoke names.
- `getConnectionCandidates` accepts a source anchor, session id, and optional
  graph revision; `connectAnchors`, `insertNodeAndConnect`,
  `previewNodeInsertOnEdge`, and `insertNodeOnEdge` require a revision and
  return structured rejection data when a preview or commit is denied.
- `addNode`, `removeNode`, `updateNodeData`, `updateNodePosition`, `addEdge`,
  and `removeEdge` return updated graphs for store synchronization.
- `createGroup`, `ungroup`, and `updateGroupPorts` return updated graphs for
  the same store synchronization path.
- Session lifecycle ordering remains: create/load session before graph mutation,
  consume the returned backend session handle instead of hardcoding local
  session classification,
  use `runSession()` as the preferred execution path for an active editor
  session, and remove the session when the consumer is done.
- Compatibility policy is additive: new invoke-backed methods should extend the
  adapter without silently changing existing method semantics.

## Structured Producer Contract (Machine-Consumed Modules)
- None.
- Reason: this adapter forwards structured payloads but does not define new
  persisted artifacts of its own.
- Revisit trigger: the adapter begins caching or persisting serialized command
  payloads.
