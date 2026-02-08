# Proposal: Extract Node Graph as `@pantograph/svelte-graph`

## Summary

Extract Pantograph's node graph editor into a reusable npm package so it can be consumed by other Svelte applications (e.g., PumaBot). The Pantograph desktop app continues to work identically by consuming the package internally.

## Problem

The node graph editor is tightly coupled to:

1. **Tauri `invoke` calls** -- `WorkflowService.ts` calls 20+ Tauri commands directly. No other backend can drive the graph.
2. **Global Svelte stores** -- `workflowStore`, `graphSessionStore`, `viewStore` are global singletons. Multiple graph instances on one page would conflict.
3. **Hardcoded node types** -- `WorkflowGraph.svelte` has a static `nodeTypes` map with 22 entries. Consumers can't register their own node types.

## Solution

Create `packages/svelte-graph/` as an npm workspace package that exports the core graph components with three abstraction points.

### 1. `WorkflowBackend` Interface

Replaces direct Tauri `invoke` calls with a transport-agnostic interface:

```typescript
export interface WorkflowBackend {
  // Node definitions
  getNodeDefinitions(): Promise<NodeDefinition[]>;
  validateConnection(sourceType: string, targetType: string): Promise<boolean>;

  // Session management
  createSession(graph: WorkflowGraph): Promise<string>;
  removeSession(sessionId: string): Promise<void>;

  // Execution
  executeWorkflow(graph: WorkflowGraph): Promise<void>;
  runSession(sessionId: string): Promise<void>;

  // Graph mutation (during session)
  addEdge(edge: GraphEdge, sessionId: string): Promise<WorkflowGraph>;
  removeEdge(edgeId: string, sessionId: string): Promise<WorkflowGraph>;
  addNode(node: GraphNode, sessionId: string): Promise<void>;
  updateNodeData(nodeId: string, data: Record<string, unknown>, sessionId: string): Promise<void>;

  // Undo/redo
  getUndoRedoState(sessionId: string): Promise<UndoRedoState>;
  undo(sessionId: string): Promise<WorkflowGraph>;
  redo(sessionId: string): Promise<WorkflowGraph>;

  // Persistence
  saveWorkflow(name: string, graph: WorkflowGraph): Promise<string>;
  loadWorkflow(path: string): Promise<WorkflowFile>;
  listWorkflows(): Promise<WorkflowMetadata[]>;
  deleteWorkflow(name: string): Promise<void>;

  // Node groups
  createGroup(name: string, selectedNodeIds: string[], graph: WorkflowGraph): Promise<CreateGroupResult>;
  updateGroupPorts(group: NodeGroup, exposedInputs: PortMapping[], exposedOutputs: PortMapping[]): Promise<NodeGroup>;

  // Events
  subscribeEvents(listener: (event: WorkflowEvent) => void): () => void;
}
```

Pantograph implements this as `TauriWorkflowBackend` (maps each method to a Tauri `invoke` call, identical to current `WorkflowService`). Other consumers provide their own implementation.

### 2. Svelte Context Instead of Global Stores

Replace global store imports with a context-based system for multi-instance support:

```typescript
// Consumer creates a context at the top of their component tree
import { createGraphContext } from '@pantograph/svelte-graph';

const context = createGraphContext(myBackend, myNodeTypeRegistry);

// Child components retrieve it
import { useGraphContext } from '@pantograph/svelte-graph';

const { stores, actions } = useGraphContext();
```

Store factories create per-instance stores:
- `createWorkflowStores(backend)` -- nodes, edges, definitions, execution states, groups
- `createViewStores()` -- view level, group stack, zoom target, animations
- `createSessionStores(backend)` -- current graph ID, session ID, read-only state

### 3. `NodeTypeRegistry`

Injectable map of node type string to Svelte component:

```typescript
export interface NodeTypeRegistry {
  workflow: Record<string, typeof SvelteComponent>;   // required
  fallback: typeof SvelteComponent;                   // required (unknown types)
  architecture?: Record<string, typeof SvelteComponent>; // optional
}
```

## Package Structure

```
packages/svelte-graph/
  package.json                          # @pantograph/svelte-graph
  src/
    index.ts                            # Public exports
    types/
      backend.ts                        # WorkflowBackend interface
      workflow.ts                       # NodeDefinition, GraphNode, GraphEdge, WorkflowGraph, PortDataType
      groups.ts                         # NodeGroup, PortMapping, CreateGroupResult
      events.ts                         # WorkflowEvent types
      registry.ts                       # NodeTypeRegistry
      view.ts                           # ViewLevel, ZoomTarget, BreadcrumbItem
    context/
      createGraphContext.ts             # Factory: backend + registry -> context
      useGraphContext.ts                # getContext helper for child components
    stores/
      createWorkflowStores.ts           # Per-instance workflow stores
      createViewStores.ts               # Per-instance view/navigation stores
      createSessionStores.ts            # Per-instance session stores
    components/
      WorkflowGraph.svelte              # Main graph component
      WorkflowToolbar.svelte            # Toolbar (execute, save, undo/redo)
      NodePalette.svelte                # Drag-to-add node picker
      NavigationBreadcrumb.svelte       # Group navigation breadcrumb
      ZoomTransition.svelte             # Animated zoom between levels
      edges/
        ReconnectableEdge.svelte        # Custom edge with reconnection
      nodes/
        BaseNode.svelte                 # Base node shell (ports, status, header)
        GenericNode.svelte              # Fallback node renderer
    utils/
      geometry.ts                       # Line intersection helpers
```

## What Stays in Pantograph (Not Extracted)

These are Pantograph-specific and should NOT be in the package:

- `architectureStore.ts` -- hardcoded Pantograph system architecture
- `orchestrationStore.ts` -- 15+ orchestration-specific Tauri commands (can be extracted later if needed)
- `linkStore.ts` -- Pantograph-specific UI element linking
- Architecture node components (`ArchComponentNode`, etc.)
- Orchestration node components (`StartNode`, `EndNode`, etc.)
- Canvas, drawing, hotload, design-system features

## Implementation Phases

### Phase 1: Package Skeleton + Types
- Create `packages/svelte-graph/` directory
- Add `package.json` with `@pantograph/svelte-graph`, peer deps: `svelte ^5.0`, `@xyflow/svelte ^1.5`
- Add npm workspaces to root `package.json`: `"workspaces": ["packages/*"]`
- Extract all types from `src/services/workflow/types.ts` and `groupTypes.ts`
- Define `WorkflowBackend` interface

### Phase 2: Store Factories
- Convert `workflowStore.ts` (14 stores, 20+ actions) to `createWorkflowStores(backend)`
- Convert `viewStore.ts` (8 stores, 10+ nav functions) to `createViewStores()`
- Convert `graphSessionStore.ts` (6 stores) to `createSessionStores(backend, workflowStores, viewStores)`
- Route all `invoke` calls through the `backend` parameter

### Phase 3: Context System
- Implement `createGraphContext(backend, nodeTypeRegistry)`
- Implement `useGraphContext()` helper
- Context assembles all store factories and exposes unified API

### Phase 4: Extract Components
- Extract `WorkflowGraph.svelte` -- replace global store imports with `useGraphContext()`, replace `nodeTypes` map with registry prop
- Extract `BaseNode.svelte`, `GenericNode.svelte` -- replace store imports with context
- Extract `ReconnectableEdge.svelte`
- Extract toolbar, palette, breadcrumb, zoom transition

### Phase 5: TauriWorkflowBackend
- Create `src/backends/TauriWorkflowBackend.ts` implementing `WorkflowBackend`
- Each method maps 1:1 to current `WorkflowService` methods
- Event subscription uses Tauri `Channel` forwarded to listeners

### Phase 6: Refactor Pantograph App
- Update `App.svelte` / `UnifiedGraphView.svelte` to instantiate `TauriWorkflowBackend`
- Register all 22 node types via `NodeTypeRegistry`
- Call `createGraphContext(backend, registry)` at the top
- Remove old global store imports
- Verify Pantograph desktop app works identically

## Peer Dependencies

```json
{
  "peerDependencies": {
    "svelte": "^5.0.0",
    "@xyflow/svelte": "^1.5.0"
  }
}
```

## Verification

- Pantograph desktop app works identically after the refactor (all features: drag, connect, cut, group, undo/redo, execute)
- Package can be consumed by an external Svelte app with a custom `WorkflowBackend`
- Multiple `WorkflowGraph` instances can coexist on one page without store conflicts
