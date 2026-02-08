# Unified Node Registry — Parallel Agent Orchestration Plan

## Context

The `llamacpp-inference` node doesn't appear in puma-bot because it exists only in src-tauri's hardcoded registry. Three disconnected registries (engine, Tauri, frontend) must be unified so `workflow-nodes` is the single source of truth. This plan orchestrates the implementation across parallel agents.

---

## Agent Dependency Graph

```
 WAVE 1 — All independent, run in parallel
 ┌─────────────────────────────────────────────────────────────────────────┐
 │                                                                         │
 │  [A] LlamaCpp       [B] PumaLib         [C] AgentTools    [D] Frontend │
 │  Inference Task      Task                Task + tool/       buildRegistry│
 │  (processing/)       (input/)            module             (svelte-graph)│
 │                                                                         │
 └──────┬──────────────────┬───────────────────┬───────────────────┬───────┘
        │                  │                   │                   │
        ▼                  ▼                   ▼                   ▼
 WAVE 2 — Depends on its Wave 1 predecessor
 ┌──────────────────────────────────────────┐  ┌───────────────────────────┐
 │                                          │  │                           │
 │  [E] Wire Rust modules                   │  │  [F] Rewrite frontend     │
 │  (mod.rs updates, lib.rs test counts)    │  │  registry + storeInstances│
 │                                          │  │                           │
 └─────────────────┬────────────────────────┘  └─────────────┬─────────────┘
                   │                                         │
                   ▼                                         ▼
 WAVE 3 — Verify each stack independently
 ┌──────────────────────────────────────────┐  ┌───────────────────────────┐
 │                                          │  │                           │
 │  [G] cargo test -p workflow-nodes        │  │  [H] npm run check        │
 │  Verify 23/25 inventory count            │  │  (svelte-graph + app)     │
 │                                          │  │                           │
 └─────────────────┬────────────────────────┘  └───────────────────────────┘
                   │
                   ▼
 WAVE 4 — Depends on workflow-nodes passing
 ┌──────────────────────────────────────────┐
 │                                          │
 │  [I] Rewrite src-tauri registry          │
 │  (register_builtins + extern crate)      │
 │                                          │
 └─────────────────┬────────────────────────┘
                   │
                   ▼
 WAVE 5 — Final verification
 ┌──────────────────────────────────────────┐
 │                                          │
 │  [J] cargo test -p pantograph            │
 │  + cargo build -p pantograph             │
 │                                          │
 └──────────────────────────────────────────┘
```

### Summary Table

| Wave | Agents | Parallelism | Depends On |
|------|--------|-------------|------------|
| 1 | A, B, C, D | 4 parallel | Nothing |
| 2 | E, F | 2 parallel | A+B+C → E, D → F |
| 3 | G, H | 2 parallel | E → G, F → H |
| 4 | I | 1 | G (tests pass) |
| 5 | J | 1 | I |

---

## Agent Prompts

### Agent A: LlamaCpp Inference Task

**File to create:** `crates/workflow-nodes/src/processing/llamacpp_inference.rs`

```
Create the file crates/workflow-nodes/src/processing/llamacpp_inference.rs in the
Pantograph workspace at "/media/jeremy/OrangeCream/Linux Software/Pantograph".

This is a stub node descriptor for llamacpp-inference. It provides metadata so
that register_builtins() discovers the node, but actual execution is handled by
the host app via the callback bridge.

Follow the EXACT same pattern as the existing ModelProviderTask in
crates/workflow-nodes/src/input/model_provider.rs — same imports, same structure.
Read that file first to match the pattern precisely.

Requirements:
- Struct: LlamaCppInferenceTask with task_id: String field
- Constructor: new(task_id: impl Into<String>)
- TaskDescriptor impl with this metadata:
  - node_type: "llamacpp-inference"
  - category: NodeCategory::Processing
  - label: "LlamaCpp Inference"
  - description: "Run inference via llama.cpp server (no model duplication)"
  - inputs:
    - model_path (String, required)
    - prompt (Prompt, required)
    - system_prompt (String, optional)
    - temperature (Number, optional)
    - max_tokens (Number, optional)
  - outputs:
    - response (String, required)
    - model_path (String, optional)
  - execution_mode: ExecutionMode::Stream
- inventory::submit!(node_engine::DescriptorFn(LlamaCppInferenceTask::descriptor));
- Task impl where run() returns:
  Err(GraphError::TaskExecutionFailed(
      "llamacpp-inference requires host-specific execution via the callback bridge".into()
  ))
- Use port name constants (const PORT_MODEL_PATH: &str = "model_path"; etc.)
- Module doc comment explaining this is a stub descriptor

Tests (in #[cfg(test)] mod tests):
- test_descriptor_has_correct_node_type: verify descriptor().node_type == "llamacpp-inference"
- test_descriptor_has_correct_ports: verify input/output counts and IDs
- test_run_returns_error: verify run() returns Err with appropriate message

Coding standards to follow:
- Target < 500 lines (this should be ~120 lines)
- Use PortMetadata::required() and PortMetadata::optional() constructors
- Test names: test_<function>_<scenario>_<expected_result>
- Arrange-Act-Assert pattern in tests
- Comment the "why" not the "what"
- Use graph_flow::{Context, GraphError, Task, TaskResult} for error variant
```

---

### Agent B: PumaLib Task

**File to create:** `crates/workflow-nodes/src/input/puma_lib.rs`

```
Create the file crates/workflow-nodes/src/input/puma_lib.rs in the Pantograph
workspace at "/media/jeremy/OrangeCream/Linux Software/Pantograph".

This is a stub node descriptor for puma-lib. It provides metadata so that
register_builtins() discovers the node. Actual execution is handled by the host
app via the callback bridge (the host provides the model file path from its
local pumas-core library).

Read crates/workflow-nodes/src/input/model_provider.rs first to match the
existing pattern precisely.

Requirements:
- Struct: PumaLibTask with task_id: String field
- Constructor: new(task_id: impl Into<String>)
- TaskDescriptor impl with this metadata:
  - node_type: "puma-lib"
  - category: NodeCategory::Input
  - label: "Puma-Lib"
  - description: "Provides AI model file path"
  - inputs: none (empty vec)
  - outputs:
    - model_path (String, optional)
  - execution_mode: ExecutionMode::Reactive
- inventory::submit!(node_engine::DescriptorFn(PumaLibTask::descriptor));
- Task impl where run() returns:
  Err(GraphError::TaskExecutionFailed(
      "puma-lib requires host-specific execution via the callback bridge".into()
  ))
- Module doc comment explaining this is a stub descriptor

Tests:
- test_descriptor_has_correct_node_type
- test_descriptor_has_correct_ports
- test_run_returns_error

Do NOT gate behind #[cfg(feature = "desktop")] — this node must be discoverable
by all consumers (including puma-bot NIF).

Coding standards:
- Target < 500 lines (this should be ~80 lines)
- Use PortMetadata::optional() constructor
- Test names: test_<function>_<scenario>_<expected_result>
- Arrange-Act-Assert in tests
```

---

### Agent C: AgentTools Task + tool/ Module

**Files to create:**

- `crates/workflow-nodes/src/tool/mod.rs`
- `crates/workflow-nodes/src/tool/agent_tools.rs`

```
Create TWO files in the Pantograph workspace at
"/media/jeremy/OrangeCream/Linux Software/Pantograph":

1. crates/workflow-nodes/src/tool/mod.rs
2. crates/workflow-nodes/src/tool/agent_tools.rs

This creates a new top-level "tool" module in workflow-nodes for the
agent-tools stub descriptor.

Read crates/workflow-nodes/src/input/model_provider.rs first to match the
existing node pattern.
Read crates/workflow-nodes/src/input/mod.rs for the module structure pattern.

FILE 1: crates/workflow-nodes/src/tool/mod.rs
- Module doc comment: "//! Tool nodes\n//!\n//! Nodes that provide tool configurations for agent workflows."
- mod agent_tools;
- pub use agent_tools::AgentToolsTask;

FILE 2: crates/workflow-nodes/src/tool/agent_tools.rs
- Struct: AgentToolsTask with task_id: String field
- Constructor: new(task_id: impl Into<String>)
- TaskDescriptor impl with this metadata:
  - node_type: "agent-tools"
  - category: NodeCategory::Tool
  - label: "Agent Tools"
  - description: "Configures available tools for agent"
  - inputs: none (empty vec)
  - outputs:
    - tools (Tools, optional)
  - execution_mode: ExecutionMode::Reactive
- inventory::submit!(node_engine::DescriptorFn(AgentToolsTask::descriptor));
- Task impl where run() returns:
  Err(GraphError::TaskExecutionFailed(
      "agent-tools requires host-specific execution via the callback bridge".into()
  ))
- Module doc comment explaining this is a stub descriptor

Tests:
- test_descriptor_has_correct_node_type
- test_descriptor_has_correct_ports: verify output port id is "tools" and data_type is Tools
- test_run_returns_error

Coding standards:
- Target < 500 lines (agent_tools.rs should be ~80 lines)
- Use PortMetadata::optional() constructor
- PortDataType::Tools for the output port
- Test names: test_<function>_<scenario>_<expected_result>
```

---

### Agent D: Frontend buildRegistry Helper

**Files to create/modify:**

- `packages/svelte-graph/src/utils/buildRegistry.ts` (new)
- `packages/svelte-graph/src/index.ts` (modify)

```
Add a buildRegistry() function export to the @pantograph/svelte-graph package.

Read these files first:
- packages/svelte-graph/src/index.ts (the file to modify)
- packages/svelte-graph/src/types/registry.ts (NodeTypeRegistry interface)
- packages/svelte-graph/src/types/workflow.ts (NodeDefinition type)

Add a new section at the end of packages/svelte-graph/src/index.ts, after the
"Node/Edge Components" section:

// --- Registry Builder ---
export { buildRegistry } from './utils/buildRegistry.js';

Then create the file packages/svelte-graph/src/utils/buildRegistry.ts:

import type { Component } from 'svelte';
import type { NodeDefinition } from '../types/workflow.js';
import type { NodeTypeRegistry } from '../types/registry.js';
import GenericNode from '../components/nodes/GenericNode.svelte';
import ReconnectableEdge from '../components/edges/ReconnectableEdge.svelte';

/**
 * Build a NodeTypeRegistry from engine-provided definitions.
 *
 * Maps each NodeDefinition to either a specialized component (if provided
 * in the overrides map) or the GenericNode fallback. Consumers can add
 * non-engine node types (e.g., architecture nodes) on top.
 */
export function buildRegistry(
  definitions: NodeDefinition[],
  specializedNodes?: Record<string, Component<any>>,
): NodeTypeRegistry {
  const nodeTypes: Record<string, Component<any>> = {};
  for (const def of definitions) {
    nodeTypes[def.node_type] = specializedNodes?.[def.node_type] ?? GenericNode;
  }
  return {
    nodeTypes,
    fallbackNode: GenericNode,
    edgeTypes: { reconnectable: ReconnectableEdge },
  };
}

This keeps the function in its own file per the coding standards (<500 lines,
single responsibility). The index.ts just re-exports it.

Coding standards:
- JSDoc comment on the public function
- Use the package's own types (NodeDefinition, NodeTypeRegistry)
- Don't over-engineer — simple loop, no extra abstractions
```

---

### Agent E: Wire Rust Modules

**Files to modify:**

- `crates/workflow-nodes/src/processing/mod.rs`
- `crates/workflow-nodes/src/input/mod.rs`
- `crates/workflow-nodes/src/lib.rs`

```
Wire up the 3 new task files into the workflow-nodes crate module tree.

Read these files first:
- crates/workflow-nodes/src/processing/mod.rs
- crates/workflow-nodes/src/input/mod.rs
- crates/workflow-nodes/src/lib.rs

CHANGE 1: crates/workflow-nodes/src/processing/mod.rs
Add after the existing module declarations:
  mod llamacpp_inference;
Add to the pub use section:
  pub use llamacpp_inference::LlamaCppInferenceTask;

CHANGE 2: crates/workflow-nodes/src/input/mod.rs
Add after the existing module declarations:
  mod puma_lib;
Add to the pub use section:
  pub use puma_lib::PumaLibTask;

CHANGE 3: crates/workflow-nodes/src/lib.rs
Add in the module declarations (after the existing pub mod lines):
  pub mod tool;
Add in the re-exports (after the existing pub use lines):
  pub use tool::*;
Update the test in mod tests:
- Change expected count from 20 to 23 (without desktop)
- Change expected count from 22 to 25 (with desktop)
- Add spot-check assertions:
  assert!(registry.has_node_type("llamacpp-inference"));
  assert!(registry.has_node_type("puma-lib"));
  assert!(registry.has_node_type("agent-tools"));
```

---

### Agent F: Rewrite Frontend Registry + storeInstances

**Files to modify:**

- `src/registry/pantographNodeTypes.ts`
- `src/stores/storeInstances.ts`

```
Rewrite the Pantograph frontend node registry to use buildRegistry() from
@pantograph/svelte-graph instead of hardcoding every node mapping.

Read these files first:
- src/registry/pantographNodeTypes.ts (the main file to rewrite)
- src/stores/storeInstances.ts (consumer of the registry)
- packages/svelte-graph/src/index.ts (to see buildRegistry export)

CHANGE 1: Rewrite src/registry/pantographNodeTypes.ts

Replace the static PANTOGRAPH_NODE_REGISTRY constant with a function-based
approach:

/**
 * Pantograph Node Type Registry
 *
 * Uses buildRegistry() from @pantograph/svelte-graph to map engine-provided
 * definitions to Svelte components, then adds Pantograph-specific nodes.
 */
import type { NodeTypeRegistry, NodeDefinition } from '@pantograph/svelte-graph';
import { buildRegistry } from '@pantograph/svelte-graph';

// Specialized workflow node components (Pantograph-only overrides)
import TextInputNode from '../components/nodes/workflow/TextInputNode.svelte';
import LLMInferenceNode from '../components/nodes/workflow/LLMInferenceNode.svelte';
import OllamaInferenceNode from '../components/nodes/workflow/OllamaInferenceNode.svelte';
import LlamaCppInferenceNode from '../components/nodes/workflow/LlamaCppInferenceNode.svelte';
import ModelProviderNode from '../components/nodes/workflow/ModelProviderNode.svelte';
import TextOutputNode from '../components/nodes/workflow/TextOutputNode.svelte';
import PumaLibNode from '../components/nodes/workflow/PumaLibNode.svelte';
import AgentToolsNode from '../components/nodes/workflow/AgentToolsNode.svelte';
import VectorDbNode from '../components/nodes/workflow/VectorDbNode.svelte';
import NodeGroupNode from '../components/nodes/workflow/NodeGroupNode.svelte';
import LinkedInputNode from '../components/nodes/workflow/LinkedInputNode.svelte';

// Architecture node components (Pantograph-only, not engine nodes)
import ArchComponentNode from '../components/nodes/architecture/ArchComponentNode.svelte';
import ArchServiceNode from '../components/nodes/architecture/ArchServiceNode.svelte';
import ArchStoreNode from '../components/nodes/architecture/ArchStoreNode.svelte';
import ArchBackendNode from '../components/nodes/architecture/ArchBackendNode.svelte';
import ArchCommandNode from '../components/nodes/architecture/ArchCommandNode.svelte';

/** Specialized component overrides for engine node types */
const SPECIALIZED_NODES: Record<string, any> = {
  'text-input': TextInputNode,
  'llm-inference': LLMInferenceNode,
  'ollama-inference': OllamaInferenceNode,
  'llamacpp-inference': LlamaCppInferenceNode,
  'model-provider': ModelProviderNode,
  'text-output': TextOutputNode,
  'puma-lib': PumaLibNode,
  'agent-tools': AgentToolsNode,
  'vector-db': VectorDbNode,
  'linked-input': LinkedInputNode,
};

/** Non-engine nodes (architecture + grouping, Pantograph desktop only) */
const EXTRA_NODES: Record<string, any> = {
  'node-group': NodeGroupNode,
  'arch-component': ArchComponentNode,
  'arch-service': ArchServiceNode,
  'arch-store': ArchStoreNode,
  'arch-backend': ArchBackendNode,
  'arch-command': ArchCommandNode,
};

/**
 * Build the Pantograph node registry from engine definitions.
 *
 * @param definitions - NodeDefinition[] from the backend (via Tauri command).
 *   Defaults to empty — specialized nodes still work and fallbackNode handles the rest.
 */
export function buildPantographRegistry(definitions: NodeDefinition[] = []): NodeTypeRegistry {
  const registry = buildRegistry(definitions, SPECIALIZED_NODES);
  Object.assign(registry.nodeTypes, EXTRA_NODES);
  return registry;
}

CHANGE 2: Update src/stores/storeInstances.ts

Replace:
  import { PANTOGRAPH_NODE_REGISTRY } from '../registry/pantographNodeTypes';
  export const registry = PANTOGRAPH_NODE_REGISTRY;
With:
  import { buildPantographRegistry } from '../registry/pantographNodeTypes';
  export const registry = buildPantographRegistry();

This works synchronously — the fallbackNode (GenericNode) handles any node type
not in SPECIALIZED_NODES. No changes needed in App.svelte.

Coding standards:
- JSDoc on the public function
- Delete unused code (the old static PANTOGRAPH_NODE_REGISTRY — don't keep as deprecated)
- Don't add backwards-compatibility shims
```

---

### Agent G: Verify workflow-nodes

```
Run tests for the workflow-nodes crate to verify the 3 new stub nodes are
discovered by inventory and the module wiring is correct.

cd "/media/jeremy/OrangeCream/Linux Software/Pantograph"

Run:
  cargo test -p workflow-nodes

Expected:
- test_inventory_collects_all_builtins passes with 23 nodes (or 25 with desktop)
- All new node tests pass (descriptor and run-returns-error tests)
- No compilation errors

If tests fail, diagnose using the verification layers approach:
1. Read the full error message
2. Check if it's a compiler error (fix types/imports)
3. Check if it's a test assertion (fix expected counts)
4. Fix and re-run
```

---

### Agent H: Verify svelte-graph

```
Run type checking for the svelte-graph package and the Pantograph app to verify
the buildRegistry export and the rewritten pantographNodeTypes.ts compile.

cd "/media/jeremy/OrangeCream/Linux Software/Pantograph"

Run in order:
1. cd packages/svelte-graph && npm run check (or npx svelte-check)
2. cd ../.. && npm run check (or npx svelte-check in the app root)

If there's no check script, try:
  npx tsc --noEmit
  npx svelte-check

Expected:
- No TypeScript errors from buildRegistry.ts
- No import errors in pantographNodeTypes.ts
- No type errors in storeInstances.ts

If errors occur, read the full error, fix the types, and re-run.
```

---

### Agent I: Rewrite src-tauri Registry

**Files to modify:**

- `src-tauri/src/main.rs`
- `src-tauri/src/workflow/registry.rs`

```
Replace the manual node listing in src-tauri's NodeRegistry with the
inventory-based register_builtins() approach.

Read these files first:
- src-tauri/src/workflow/registry.rs (the main file to rewrite)
- src-tauri/src/main.rs (needs extern crate for linker)
- crates/node-engine/src/registry.rs (NodeRegistry::with_builtins API)
- crates/pantograph-rustler/src/lib.rs line 47 (extern crate pattern)

CHANGE 1: src-tauri/src/main.rs
Add after line 1 (#![cfg_attr...]):
  // Force linker to include workflow-nodes' inventory::submit!() statics
  extern crate workflow_nodes;

CHANGE 2: src-tauri/src/workflow/registry.rs

Keep these functions (still needed for type conversion):
- convert_metadata()
- convert_category()
- convert_execution_mode()
- convert_port()
- convert_data_type()

Replace the NodeRegistry::new() method body. Remove the 22 register_from_task
calls and 3 hardcoded definitions. New implementation:

pub fn new() -> Self {
    let mut definitions = HashMap::new();

    // Single source of truth: discover all nodes via inventory
    let engine_registry = node_engine::NodeRegistry::with_builtins();
    for meta in engine_registry.all_metadata() {
        let def = convert_metadata(meta.clone());
        definitions.insert(def.node_type.clone(), def);
    }

    Self { definitions }
}

Remove these items (now unused):
- use node_engine::TaskDescriptor; (line 9)
- The entire use workflow_nodes::{...} block (lines 10-21)
- fn register_from_task<T: TaskDescriptor>() method
- fn register(map, def) helper method
- fn puma_lib_definition() method
- fn agent_tools_definition() method
- fn llamacpp_inference_definition() method

Keep all public methods (get_definition, all_definitions, etc.) and the
NodeRegistry struct unchanged.

Update the tests:
- test_descriptor_conversion: rewrite to test via the registry instead of
  calling TextInputTask::descriptor() directly. Get definition from
  registry.get_definition("text-input") and verify fields.
- Remove the now-unnecessary use workflow_nodes::TextInputTask; import if present
- All other tests (test_registry_has_builtin_nodes, etc.) should pass unchanged.

Coding standards:
- Delete unused code completely (no commented-out code, no deprecated stubs)
- Validate at boundaries: the convert_* functions handle the type boundary
  between node-engine and src-tauri type systems
```

---

### Agent J: Verify src-tauri

```
Run tests and build for the src-tauri crate to verify the registry rewrite
works with register_builtins() and the extern crate linker inclusion.

cd "/media/jeremy/OrangeCream/Linux Software/Pantograph"

Run in order:
1. cargo check -p pantograph (fast type check first)
2. cargo test -p pantograph (run tests)
3. cargo build -p pantograph (full build to verify linker)

Note: The crate name for src-tauri may be "pantograph" or something else.
Check src-tauri/Cargo.toml for the [package] name field.

Expected:
- test_registry_has_builtin_nodes passes (now discovers via register_builtins)
- test_descriptor_conversion passes (rewritten to use registry)
- All 25 node types present (including llamacpp-inference, puma-lib, agent-tools)
- No linker errors (extern crate workflow_nodes forces inclusion)

If register_builtins() returns 0 nodes:
- Verify extern crate workflow_nodes; is in main.rs
- Check that workflow-nodes is a dependency in src-tauri/Cargo.toml
```

---

## Verification After All Agents

After all agents complete, perform a final integration check:

```bash
# Full Rust workspace build
cargo build --workspace

# Full Rust workspace tests
cargo test --workspace

# Frontend build
cd packages/svelte-graph && npm run build
cd ../..
npm run build  # or equivalent app build command
```

## Commit Plan

Following conventional commits, one logical change per commit:

```
1. feat(workflow-nodes): add llamacpp-inference stub descriptor
2. feat(workflow-nodes): add puma-lib stub descriptor
3. feat(workflow-nodes): add agent-tools stub descriptor with tool/ module
4. feat(workflow-nodes): wire new nodes into module tree
5. refactor(src-tauri): replace manual registry with register_builtins()
6. feat(svelte-graph): add buildRegistry() helper export
7. refactor(app): use buildPantographRegistry() for dynamic node registry
```

Commits 1-4 can be squashed into one if preferred:

```
feat(workflow-nodes): move Tauri-only node descriptors into engine
```
