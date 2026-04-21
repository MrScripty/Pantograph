# packages/svelte-graph/src/backends

## Purpose
Transport adapters in this directory implement the reusable `WorkflowBackend`
contract for graph-editing consumers. The boundary exists so `WorkflowGraph`
and the store factories can ask for node definitions, session mutations, and
connection-intent candidates without coupling directly to Tauri or any other
transport.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `MockWorkflowBackend.ts` | In-memory backend used for package development, UI prototyping, and tests that need session-scoped graph edits without a Tauri runtime. |

## Problem
The graph package needs one editing API surface that works for embedded GUI
consumers, headless-style tests, and the Pantograph app adapter. Without a
backend abstraction, every consumer would reimplement session lifecycle,
connection validation, and graph synchronization differently.

## Constraints
- The backend surface must stay transport-agnostic so the package can run in
  tests, Storybook-style environments, and the Tauri app.
- Interactive connection guidance must be revision-aware to avoid accepting
  stale drag intents after the graph changes.
- Mock behavior must be close enough to real backend semantics that UI behavior
  around candidate highlighting and rejection handling is still meaningful.

## Decision
Keep a single `WorkflowBackend` interface in `types/backend.ts`, and implement
package-local behavior through `MockWorkflowBackend`. The mock stores a
session-local graph with derived revision metadata so the same
`getConnectionCandidates` and `connectAnchors` flow can be exercised without a
native backend.
Group create, ungroup, and port-mapping operations are also session-scoped
graph mutation methods; mock and production backends must return the full graph
mutation response rather than group-only DTOs.

## Alternatives Rejected
- Call Tauri `invoke` directly from package components.
  Rejected because the reusable package would no longer be transport-neutral.
- Keep the mock backend limited to boolean type validation.
  Rejected because the UI now depends on candidate discovery, revision checks,
  and structured rejection reasons.

## Invariants
- `WorkflowBackend` implementations must treat sessions as the authority for
  graph mutation methods.
- Node group create, ungroup, and port edits return graph mutation responses so
  stores can render backend-owned collapsed group state directly.
- Session-scoped graph mutation methods may return an additive backend-owned
  `workflow_event` envelope alongside the updated graph so GUI consumers can
  react to canonical `GraphModified` semantics without synthesizing them
  locally.
- `getConnectionCandidates` must describe eligible targets for one source
  anchor, not a whole-graph recommendation pass.
- `connectAnchors` must return structured rejection data on failure instead of
  throwing for expected incompatibility cases.

## Revisit Triggers
- A second production transport (HTTP/WebSocket) needs adapter-specific retry or
  auth behavior.
- Insert-and-connect becomes a first-class backend operation rather than a
  follow-on UI flow.
- Mock behavior drifts from the native backend often enough that package UI
  tests stop predicting app behavior.

## Dependencies
**Internal:** `packages/svelte-graph/src/types`, `packages/svelte-graph/src/graphRevision.ts`,
`packages/svelte-graph/src/portTypeCompatibility.ts`.

**External:** Svelte package consumers and whatever runtime hosts the chosen
backend implementation.

## Related ADRs
- None.
- Reason: the backend abstraction remains internal to the package/app boundary.
- Revisit trigger: a new transport becomes a supported integration surface for
  third-party consumers.

## Usage Examples
```ts
import { MockWorkflowBackend } from '@pantograph/svelte-graph';

const backend = new MockWorkflowBackend();
const session = await backend.createSession({ nodes: [], edges: [] });
const candidates = await backend.getConnectionCandidates(
  { node_id: 'source-node', port_id: 'text' },
  session.session_id,
);
```

## API Consumer Contract (Host-Facing Modules)
- `WorkflowBackend` consumers must create a session before calling graph
  mutation or connection-intent methods.
- `createSession()` returns a backend-owned session handle so consumers do not
  invent local session classification rules.
- `WorkflowBackend` consumers should also prefer `runSession(sessionId)` for
  normal editor execution once a session exists; `executeWorkflow(graph)` is the
  fallback path for raw graph snapshots without an active session owner.
- `getConnectionCandidates` accepts a source anchor plus optional graph revision
  and returns compatible existing targets plus insertable node types.
- `connectAnchors` requires a graph revision and returns either
  `{ accepted: true, graph }` or `{ accepted: false, rejection }`.
- Expected incompatibility is reported in-band through rejection payloads; only
  transport/session failures should surface as thrown errors.
- Compatibility policy is additive: new backend methods should extend the
  interface without breaking older graph-editing paths immediately.

## Structured Producer Contract (Machine-Consumed Modules)
- Candidate responses always include `graph_revision`, `revision_matches`,
  `source_anchor`, `compatible_nodes`, and `insertable_node_types`.
- Graph-mutation responses may include a backend-owned `workflow_event`; when
  present, consumers should forward or consume that contract read-only instead
  of rebuilding `GraphModified` semantics from the graph snapshot alone.
- Compatible target ordering is backend-defined; consumers must not infer
  semantic priority unless the backend documents one.
- Rejection reasons use stable snake_case labels shared with Rust/Tauri DTOs.
- Graph revisions are volatile snapshots; consumers must refresh candidates when
  the revision changes.
- If response field names or rejection enums change, update the README and the
  mirrored Rust/TypeScript types in the same change.
