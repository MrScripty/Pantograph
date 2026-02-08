# PROPOSAL: Unify Node Registration Architecture

## Context

The `llamacpp-inference` node doesn't appear in puma-bot's node palette despite having a GUI component (`LlamaCppInferenceNode.svelte`) in `@pantograph/svelte-graph`. Investigation reveals this is a symptom of a broader architectural problem: **three separate systems that should be unified are maintained independently, leading to silent breakage when they drift apart.**

---

## Problem: Three Disconnected Node Registries

Pantograph currently maintains node awareness in three independent places:

### 1. Engine Registry (Rust — `workflow-nodes` crate)
Each node implements `TaskDescriptor` and registers via `inventory::submit!()`. The `register_builtins()` method auto-discovers all nodes at link time. This is the **source of truth** for what nodes the engine can execute.

**Currently registers 22 nodes** (text-input, ollama-inference, embedding, etc.)

### 2. Tauri App Registry (`src-tauri/src/workflow/registry.rs`)
Manually lists every node by calling `register_from_task::<T>()` for each type, then **hardcodes 3 additional "Tauri-only" nodes** that have no engine implementation:
- `puma-lib`
- `agent-tools`
- `llamacpp-inference`

### 3. Frontend Component Registry (`src/registry/pantographNodeTypes.ts`)
Manually maps each `node_type` string to a Svelte component. Must be updated every time a node is added or the node renders as `GenericNode` at best, or doesn't appear at all.

### How They Break

| Scenario | What Happens |
|----------|-------------|
| New node added to engine, not to Tauri registry | Tauri app doesn't show it (even though `register_builtins()` has it) |
| Node added to Tauri registry but not engine | Works in Tauri, **missing in puma-bot** and any other consumer |
| Node added to engine but not frontend registry | Shows as GenericNode (acceptable fallback) |
| Tauri registry lists nodes manually instead of using `register_builtins()` | Tauri doesn't benefit from auto-discovery |

The `llamacpp-inference` case hits the worst combination: defined only in the Tauri registry (not the engine), so puma-bot — which correctly uses `register_builtins()` — never sees it.

---

## Root Causes

### 1. The Tauri App Duplicates What `register_builtins()` Already Does

`src-tauri/src/workflow/registry.rs` manually calls `register_from_task::<T>()` for each of the 22 engine nodes instead of using the `inventory`-based `register_builtins()` that the Rustler NIF already uses. This means:
- Adding a node to `workflow-nodes` requires also updating `registry.rs`
- The Tauri app can't benefit from auto-discovery

### 2. "Tauri-only" Nodes Bypass the Engine

`puma-lib`, `agent-tools`, and `llamacpp-inference` are defined only in Tauri's `registry.rs` with hardcoded `NodeDefinition` structs. They have no `TaskDescriptor` in the engine, so:
- They don't exist for any non-Tauri consumer
- They can't be executed by the engine
- Their metadata drifts from what the engine would provide

### 3. No Contract Between Engine and GUI

The `@pantograph/svelte-graph` package requires consumers to provide a `NodeTypeRegistry` mapping node types to Svelte components. This is a reasonable design — but there's no mechanism to ensure the GUI knows about all engine-provided nodes. The GUI and engine evolve independently.

---

## Proposed Changes

### Change 1: Move All Node Descriptors Into the Engine

**Every node that appears in the palette must have a `TaskDescriptor` in `workflow-nodes`.**

The 3 "Tauri-only" nodes (`puma-lib`, `agent-tools`, `llamacpp-inference`) should be moved to `workflow-nodes` as proper node implementations with `inventory::submit!()`. Even if their execution is handled by the host app via the callback bridge, their **metadata** must live in the engine so that `register_builtins()` discovers them.

For nodes whose execution is host-specific (like `puma-lib` which provides a model path from the host's file system), the `TaskDescriptor` provides metadata and the `Task::run` implementation can be a no-op or return an error indicating the host must handle it via the callback bridge — which is already how the NIF executor works.

**Files to create/change:**
- `crates/workflow-nodes/src/processing/llamacpp_inference.rs` — descriptor + `inventory::submit!()`
- `crates/workflow-nodes/src/input/puma_lib.rs` — descriptor + `inventory::submit!()`
- `crates/workflow-nodes/src/input/agent_tools.rs` — descriptor + `inventory::submit!()`
- Update `crates/workflow-nodes/src/processing/mod.rs` and `crates/workflow-nodes/src/input/mod.rs`

### Change 2: Tauri App Should Use `register_builtins()` Instead of Manual Listing

Replace the manual `register_from_task::<T>()` calls in `src-tauri/src/workflow/registry.rs` with the same `inventory`-based approach the NIF uses:

```rust
// Before (manual, fragile — 25 lines of register calls):
Self::register_from_task::<TextInputTask>(&mut definitions);
Self::register_from_task::<ImageInputTask>(&mut definitions);
// ... 20 more ...
Self::register(&mut definitions, Self::llamacpp_inference_definition());

// After (automatic — 4 lines):
let mut engine_registry = node_engine::NodeRegistry::new();
engine_registry.register_builtins();
for meta in engine_registry.all_metadata() {
    definitions.insert(meta.node_type.clone(), convert_metadata(meta.clone()));
}
```

This makes the Tauri app automatically pick up any new node added to `workflow-nodes`, same as puma-bot already does via the NIF.

**File to change:** `src-tauri/src/workflow/registry.rs`

### Change 3: Frontend Registry Should Build From Definitions, Not Hardcode

The frontend should build its `NodeTypeRegistry` dynamically from the definitions received from the backend, with an optional override map for specialized Svelte components. Puma-bot already does this correctly:

```typescript
// puma-bot's approach (correct pattern):
for (const def of nodeDefinitions) {
  registry.nodeTypes[def.node_type] = SPECIALIZED_NODES[def.node_type] || GenericNode;
}
```

The Pantograph desktop app should adopt this same pattern instead of its hardcoded `PANTOGRAPH_NODE_REGISTRY`. The `@pantograph/svelte-graph` package could export a helper to make this the standard pattern:

```typescript
// Proposed helper export from @pantograph/svelte-graph
export function buildRegistry(
  definitions: NodeDefinition[],
  specializedNodes?: Record<string, Component>,
): NodeTypeRegistry {
  const nodeTypes: Record<string, Component> = {};
  for (const def of definitions) {
    nodeTypes[def.node_type] = specializedNodes?.[def.node_type] || GenericNode;
  }
  return { nodeTypes, fallbackNode: GenericNode, edgeTypes: { reconnectable: ReconnectableEdge } };
}
```

**Files to change:**
- `packages/svelte-graph/src/index.ts` — export `buildRegistry` helper
- `src/registry/pantographNodeTypes.ts` — use `buildRegistry` with specialized overrides only

---

## Architecture After Changes

```
workflow-nodes crate (SINGLE SOURCE OF TRUTH)
  ├── Every node has TaskDescriptor + inventory::submit!()
  ├── register_builtins() discovers ALL nodes automatically
  │
  ├── Tauri: register_builtins() → convert → all_definitions()
  │     └── buildRegistry(definitions, { 'text-input': TextInputNode, ... })
  │
  └── puma-bot NIF: register_builtins() → list() → JSON
        └── buildRegistry(definitions, { 'text-input': TextInputNode, ... })
```

**Adding a new node requires only:**
1. Create the `.rs` file in `workflow-nodes` with `TaskDescriptor` + `inventory::submit!()`
2. (Optional) Create a specialized `.svelte` component if `GenericNode` isn't sufficient

No manual registry updates needed anywhere.

---

## What This Fixes

- `llamacpp-inference` automatically appears in puma-bot (and all consumers)
- Future engine nodes automatically appear in all consumers
- No more silent drift between engine, Tauri, and frontend registries
- Single source of truth for node metadata
- Consumers can still override rendering with specialized components
