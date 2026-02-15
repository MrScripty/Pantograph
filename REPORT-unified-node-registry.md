# Implementation Report: Unified Node Registry

**For:** puma-bot team
**Date:** 2026-02-08
**Status:** Fully implemented and all tests passing
**Reference:** PROPOSAL-unified-node-registry.md, PLAN-unified-node-registry.md

---

## Executive Summary

The three disconnected node registries (engine, Tauri, frontend) identified in the proposal have been unified into a single-source-of-truth architecture. The `workflow-nodes` crate now owns all node metadata, and all consumers — including puma-bot via the Rustler NIF — automatically discover every node through the `inventory`-based `register_builtins()` mechanism.

**The `llamacpp-inference` node (and all other previously Tauri-only nodes) now appear in puma-bot's node palette with zero additional work required by puma-bot.**

---

## What Changed

### Change 1: All Node Descriptors Moved Into the Engine

The three "Tauri-only" nodes that were invisible to puma-bot have been added to `workflow-nodes` as proper `TaskDescriptor` implementations with `inventory::submit!()`:

| Node | File | Category |
|------|------|----------|
| `llamacpp-inference` | `crates/workflow-nodes/src/processing/llamacpp_inference.rs` | Processing |
| `puma-lib` | `crates/workflow-nodes/src/input/puma_lib.rs` | Input |
| `agent-tools` | `crates/workflow-nodes/src/tool/agent_tools.rs` | Tool |

Each follows the stub descriptor pattern: full `TaskMetadata` for discovery, but `Task::run()` returns an error indicating execution must be handled by the host via the callback bridge. This is the same pattern puma-bot already uses — no behavioral change needed.

**New `tool/` module** was created for the `agent-tools` node, establishing a new category directory.

### Change 2: Tauri App Uses `register_builtins()` Instead of Manual Listing

`src-tauri/src/workflow/registry.rs` was rewritten from ~25 manual `register_from_task::<T>()` calls + 3 hardcoded `NodeDefinition` structs to a 4-line auto-discovery loop:

```rust
let engine_registry = node_engine::NodeRegistry::with_builtins();
for meta in engine_registry.all_metadata() {
    let def = convert_metadata(meta.clone());
    definitions.insert(def.node_type.clone(), def);
}
```

The `extern crate workflow_nodes;` force-link statement was added to `src-tauri/src/main.rs` to ensure inventory statics are included by the linker (same pattern already used in `pantograph-rustler`).

**Removed:** All manual registration methods, hardcoded definition builders, and the `register_from_task` helper. Net reduction: ~183 lines deleted, replaced by the auto-discovery loop.

### Change 3: Frontend Registry Builds From Definitions

A `buildRegistry()` utility was added to `@pantograph/svelte-graph` and the Pantograph desktop app now uses it:

```typescript
// packages/svelte-graph/src/utils/buildRegistry.ts
export function buildRegistry(
  definitions: NodeDefinition[],
  specializedNodes?: Record<string, Component<any>>,
): NodeTypeRegistry

// src/registry/pantographNodeTypes.ts
export function buildPantographRegistry(definitions: NodeDefinition[] = []): NodeTypeRegistry {
  const registry = buildRegistry(definitions, SPECIALIZED_NODES);
  Object.assign(registry.nodeTypes, EXTRA_NODES);
  return registry;
}
```

**This is the same pattern puma-bot already follows.** The `buildRegistry` helper is now a shared export from `@pantograph/svelte-graph` that both Pantograph desktop and puma-bot can use.

---

## Node Inventory

After implementation, `register_builtins()` discovers **23 nodes** (or **25 with the `desktop` feature flag**):

| Category | Nodes | Count |
|----------|-------|-------|
| **Input** | `text-input`, `image-input`, `human-input`, `model-provider`, `linked-input`, `puma-lib` | 6 |
| **Processing** | `llm-inference`, `ollama-inference`, `llamacpp-inference`, `embedding`, `vision-analysis`, `json-filter`, `validator` | 7 |
| **Output** | `text-output`, `component-preview` | 2 |
| **Storage** | `read-file`, `write-file`, `vector-db`, `lancedb` | 4 |
| **Control** | `conditional`, `merge`, `tool-loop`, `tool-executor` | 4 |
| **Tool** | `agent-tools` | 1 |
| **System** | `process` | 1 |
| | **Subtotal (no desktop flag)** | **23** |
| **Desktop-only** | `read-file` (filesystem), `write-file` (filesystem) | +2 |
| | **Total (with desktop flag)** | **25** |

All 25 nodes have `inventory::submit!(node_engine::DescriptorFn(...))` and are auto-discovered.

---

## Test Results

All tests pass across the three key crates:

| Crate | Tests | Result |
|-------|-------|--------|
| `node-engine` | 89 passed | OK |
| `workflow-nodes` | 59 passed | OK |
| `pantograph` (src-tauri) | 138 passed | OK |

Key test validations:
- `test_inventory_collects_all_builtins` — verifies 23/25 nodes discovered via `register_builtins()`
- `test_registry_has_builtin_nodes` — spot-checks 21+ node types including `llamacpp-inference`, `puma-lib`, `agent-tools`
- `test_descriptor_conversion` — validates `TaskMetadata` → `NodeDefinition` conversion for port metadata, category, and execution mode
- Per-node descriptor tests — each stub node verifies correct `node_type`, port counts/IDs, and that `run()` returns the expected callback-bridge error

**Note:** `pantograph-rustler` cannot run `cargo test` (requires Erlang runtime for `enif_*` symbols) — this is expected and unchanged. The NIF crate's `register_builtins()` call at line 897 uses the identical code path validated by the other test suites.

---

## Impact on puma-bot

### What puma-bot gets automatically (no code changes needed)

1. **`llamacpp-inference` appears in the node palette** — the original issue that triggered this work
2. **`puma-lib` and `agent-tools` are now discoverable** — previously invisible to NIF consumers
3. **All future nodes added to `workflow-nodes` will auto-appear** — no manual registry sync required

### How puma-bot already consumes the registry

The `pantograph-rustler` NIF already has the correct pattern:

```rust
// crates/pantograph-rustler/src/lib.rs:47
extern crate workflow_nodes;  // force-links inventory statics

// crates/pantograph-rustler/src/lib.rs:897-901
fn node_registry_register_builtins(resource: ResourceArc<NodeRegistryResource>) {
    let mut registry = resource.registry.blocking_write();
    registry.register_builtins();
}
```

When puma-bot calls `node_registry_register_builtins/1` from Elixir, it now receives all 25 nodes (previously 22).

### Frontend integration

If puma-bot's frontend uses `@pantograph/svelte-graph`, it can now use the shared `buildRegistry()` helper:

```typescript
import { buildRegistry } from '@pantograph/svelte-graph';

const registry = buildRegistry(nodeDefinitions, {
  // puma-bot's specialized components (if any)
  'text-input': PumaBotTextInputNode,
});
```

Unmapped node types automatically render as `GenericNode`.

---

## Stub Node Execution Pattern

The three previously-Tauri-only nodes are registered as **metadata-only stubs**. Their `Task::run()` returns:

```rust
Err(GraphError::TaskExecutionFailed(
    "<node-type> requires host-specific execution via the callback bridge".into()
))
```

This is the correct pattern for nodes whose execution depends on host-specific resources (e.g., `puma-lib` needs the host's local model file path, `llamacpp-inference` needs the host's llama.cpp server). The host app (Tauri or puma-bot) handles actual execution via `register_callback()` on the `NodeRegistry`, which overrides the stub `Task::run()` with a real executor.

**No behavioral change for puma-bot** — callback-bridged nodes work exactly as before. The only difference is that their metadata is now discoverable via `register_builtins()`.

---

## Files Changed

**60 files touched**, net **+2,488 lines** across the implementation:

| Area | Key Files | Nature of Change |
|------|-----------|-----------------|
| New stub nodes | `llamacpp_inference.rs`, `puma_lib.rs`, `agent_tools.rs`, `tool/mod.rs` | New files (~320 lines total) |
| Engine registry | `node-engine/src/registry.rs`, `descriptor.rs` | Added `with_builtins()`, `register_metadata()` |
| Inventory wiring | 22 existing node `.rs` files | Added `inventory::submit!()` line to each |
| Module tree | `workflow-nodes/src/lib.rs`, `input/mod.rs`, `processing/mod.rs` | Added module declarations and re-exports |
| Tauri registry | `src-tauri/src/workflow/registry.rs`, `main.rs` | Replaced manual listing with auto-discovery |
| Frontend | `buildRegistry.ts`, `pantographNodeTypes.ts`, `storeInstances.ts` | Dynamic registry from definitions |
| svelte-graph | `index.ts` + new components | `buildRegistry` export, node components |

---

## Commit History

Implementation was delivered across 7 commits (oldest to newest):

| Commit | Description |
|--------|-------------|
| `3f73a4a` | `feat: Add built-in node registration via inventory crate` |
| `6c7feb8` | `feat(workflow-nodes): Add PumaLibTask stub descriptor for callback bridge` |
| `9401a7d` | `feat(workflow-nodes): Add LlamaCppInferenceTask and AgentTools nodes` |
| `5db36fa` | `feat(workflow-nodes): Add tool module and update node count tests` |
| `7cc9882` | `feat(svelte-graph): Add buildRegistry utility and export` |
| `0a4b026` | `refactor(registry): Use buildRegistry() for node type registry` |
| `386f9cd` | `refactor(registry): Use inventory-based node discovery in src-tauri` |

---

## Architecture After Changes

```
workflow-nodes crate (SINGLE SOURCE OF TRUTH)
  ├── Every node: impl TaskDescriptor + inventory::submit!()
  │   25 nodes across 7 categories
  │
  ├── register_builtins() discovers ALL nodes automatically
  │
  ├── src-tauri (Pantograph Desktop)
  │   ├── extern crate workflow_nodes  (force-link)
  │   ├── NodeRegistry::with_builtins() → convert → all_definitions()
  │   └── Frontend: buildPantographRegistry(definitions, SPECIALIZED_NODES)
  │
  └── pantograph-rustler (puma-bot NIF)
      ├── extern crate workflow_nodes  (force-link)
      ├── register_builtins() → list() → JSON to Elixir
      └── Frontend: buildRegistry(definitions, pumaBotOverrides)
```

**Adding a new node now requires only:**
1. Create `.rs` file in `workflow-nodes/{category}/` with `TaskDescriptor` + `inventory::submit!()`
2. Add `mod` + `pub use` in the category's `mod.rs`
3. (Optional) Create a specialized `.svelte` component if `GenericNode` isn't sufficient

No manual registry updates needed in src-tauri, pantograph-rustler, or any frontend consumer.

---

## Recommendation

The implementation is complete and ready for puma-bot integration testing. We recommend:

1. **Rebuild `pantograph-rustler`** with the latest `workflow-nodes` dependency to pick up the 3 new nodes
2. **Verify from Elixir** that `node_registry_register_builtins/1` returns 25 entries (or 23 without desktop feature)
3. **Check puma-bot's palette** for `llamacpp-inference`, `puma-lib`, and `agent-tools`
4. **Adopt `buildRegistry()`** from `@pantograph/svelte-graph` if not already using a similar pattern on the frontend
