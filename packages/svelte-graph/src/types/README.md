# packages/svelte-graph/src/types

## Purpose
This directory defines the stable TypeScript contracts shared by the reusable
graph package: workflow DTOs, backend interfaces, view state, group state, and
registry typing. The boundary exists to keep serialized graph shapes and
transport payloads in one place so frontend and Rust changes can stay aligned.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `workflow.ts` | Graph, execution, and connection-intent DTOs mirrored from Rust serialization. |
| `backend.ts` | Transport-agnostic editing/session interface consumed by stores and graph components. |
| `groups.ts` | Node-group editing contracts and port-mapping types. |
| `view.ts` | Viewport and navigation state types for group/orchestration transitions. |
| `registry.ts` | Type-safe registry contracts for node and edge component lookup. |

## Problem
Pantograph now supports interactive connection guidance across Rust, the package
graph UI, and the app wrapper. Without a shared type layer, boolean validation,
candidate payloads, and rejection enums would drift between transports.

## Constraints
- These types must remain compatible with Rust serde output where they cross the
  Tauri boundary.
- Workflow graph types are consumed by both reusable package code and app-level
  wrappers, so renames carry broad cost.
- Connection-intent payloads need revision tokens and rejection enums stable
  enough for UI caching and error display.

## Decision
Keep all graph-editing and connection-intent DTOs in `workflow.ts`, with the
transport-facing method surface in `backend.ts`. New interactive editing
contracts were added here first so both the mock backend and the Tauri adapter
could share the same shapes.
Node group create, ungroup, and port-mapping edits are modeled as
`WorkflowBackend` graph mutation methods instead of group-only return DTOs so
the backend remains the owner of collapsed graph structure.
Workflow event DTOs now include optional backend-authored `ownership` payloads
that mirror the Tauri workflow-event serializer and let reducers prefer
backend-projected execution identity over raw `execution_id` fallback fields.

## Alternatives Rejected
- Define transport payloads inline inside each backend implementation.
  Rejected because the package and app would duplicate the same contracts.
- Keep connection-intent types local to the GUI only.
  Rejected because headless and native transports need the same revision-aware
  payload semantics.

## Invariants
- Field names that cross the Rust boundary stay snake_case-compatible with the
  backend serializer.
- `ConnectionCommitResponse` represents expected failure in-band with
  `accepted: false` and a structured `rejection`.
- `ConnectionIntentState` is UI-owned transient state derived from backend
  candidate responses, not a persisted workflow artifact.
- Node group mutation methods in `WorkflowBackend` return
  `WorkflowGraphMutationResponse`; group DTOs are serialized inside graph node
  data and are not authoritative by themselves.
- Workflow event `ownership` payloads are backend-authored transport context;
  consumers may fall back to `execution_id` only for mock or legacy producers.

## Revisit Triggers
- Workflow persistence adopts a versioned schema that needs explicit migration
  metadata in these types.
- Connection intent adds server-owned lifecycle state instead of client-owned
  derived state.
- Third-party consumers require a separately versioned public SDK contract.

## Dependencies
**Internal:** `packages/svelte-graph/src/backends`, `packages/svelte-graph/src/stores`,
`packages/svelte-graph/src/components`, mirrored Rust workflow types.

**External:** TypeScript, Svelte, and any transport implementation consuming the
declared interfaces.

## Related ADRs
- None.
- Reason: the contract remains internal to Pantograph and the reusable graph
  package.
- Revisit trigger: these types become a supported external SDK surface.

## Usage Examples
```ts
import type { ConnectionAnchor, ConnectionCommitResponse } from '@pantograph/svelte-graph';

const source: ConnectionAnchor = { node_id: 'llm', port_id: 'response' };

function isAccepted(result: ConnectionCommitResponse): boolean {
  return result.accepted;
}
```

## API Consumer Contract (Host-Facing Modules)
- `WorkflowBackend` methods in `backend.ts` are the supported editing/session
  calls package consumers implement or call through adapters.
- Workflow execution is session-scoped. Consumers must call
  `runSession(sessionId)` with a backend-owned session id; raw graph execution
  is intentionally absent from the transport contract.
- Consumers must treat `graph_revision` as an opaque token and echo it back on
  revision-aware commit operations.
- New transport methods should be additive; removing or renaming existing fields
  requires an explicit migration decision.

## Structured Producer Contract (Machine-Consumed Modules)
- `WorkflowGraph` persists `nodes`, `edges`, and optional `derived_graph`.
- `WorkflowDerivedGraph.graph_fingerprint` is volatile and should be
  regenerated, not hand-authored.
- `ConnectionCandidatesResponse.compatible_nodes[].anchors[]` enumerates only
  eligible target inputs for the requested source anchor.
- `ConnectionRejectionReason` labels are stable machine-consumed enums shared
  with Rust: `stale_revision`, `unknown_source_anchor`, `unknown_target_anchor`,
  `duplicate_connection`, `target_capacity_reached`, `self_connection`,
  `cycle_detected`, and `incompatible_types`.
- If the serialized shape changes, update Rust mirrors and any persisted
  migration notes in the same change.
- `WorkflowEventOwnershipData.ownership` mirrors the Tauri event serializer's
  camelCase execution ownership projection.
