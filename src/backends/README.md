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
`connectAnchors`, and `insertNodeAndConnect` alongside the existing session
editing calls so the package graph can use revision-aware connection guidance
and drag-time insert flows unchanged inside the app.

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
- Graph revisions are treated as opaque values and echoed unchanged to Rust.
- Insert commands must remain atomic at the adapter boundary; the adapter should
  not split insert and connect into separate invokes.

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
const sessionId = await backend.createSession({ nodes: [], edges: [] });
```

## API Consumer Contract (Host-Facing Modules)
- App code consuming this adapter should use package-level backend methods, not
  hardcoded invoke names.
- `getConnectionCandidates` accepts a source anchor, session id, and optional
  graph revision; `connectAnchors` and `insertNodeAndConnect` require a
  revision and return structured rejection data when a commit is denied.
- Session lifecycle ordering remains: create/load session before graph mutation,
  remove session when the consumer is done.
- Compatibility policy is additive: new invoke-backed methods should extend the
  adapter without silently changing existing method semantics.

## Structured Producer Contract (Machine-Consumed Modules)
- None.
- Reason: this adapter forwards structured payloads but does not define new
  persisted artifacts of its own.
- Revisit trigger: the adapter begins caching or persisting serialized command
  payloads.
