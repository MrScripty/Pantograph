# Node Architecture Analysis - Pantograph

## Executive Summary

The Pantograph codebase implements a **custom node-graph visual programming system** in Rust with a Svelte frontend using a trait-based architecture. The architecture is well-designed for its current scope but has specific limitations for real-time user-editable graphs.

**Key Finding:** The existing trait-based system will be replaced with graph-flow's Task model.

**Recommended Approach:**
- **Full refactor** to graph-flow (no backwards compatibility with current Node trait)
- **Demand-driven lazy evaluation** (pull-based, not push-based dirty propagation)
- **Undo/redo** via compressed immutable snapshots
- **Future-proof for distributed** - keep architecture open for Timely Dataflow later (don't implement now)

## Current Architecture Overview

### Core Components

| Component | Location | Purpose |
|-----------|----------|---------|
| Node Trait | `src-tauri/src/workflow/node.rs` | Interface all nodes implement |
| WorkflowEngine | `src-tauri/src/workflow/engine.rs` | Executes graphs via topological sort |
| NodeRegistry | `src-tauri/src/workflow/registry.rs` | Factory pattern for node creation |
| Types | `src-tauri/src/workflow/types.rs` | Graph, Edge, Port definitions |
| Validation | `src-tauri/src/workflow/validation.rs` | Cycle detection, type checking |
| Events | `src-tauri/src/workflow/events.rs` | Real-time streaming to frontend |

### How Nodes Connect (Rust)

The system uses a **trait-based architecture**:

```rust
// src-tauri/src/workflow/node.rs - Core Node Trait
#[async_trait]
pub trait Node: Send + Sync {
    fn definition(&self) -> &NodeDefinition;
    fn id(&self) -> &str;
    async fn execute(
        &self,
        inputs: NodeInputs,           // HashMap<String, serde_json::Value>
        context: &ExecutionContext,   // Shared resources (LLM, RAG, etc.)
        channel: &Channel<WorkflowEvent>,  // Real-time events to frontend
    ) -> Result<NodeOutputs, NodeError>;
}

// src-tauri/src/workflow/types.rs - Edge Connection
pub struct GraphEdge {
    pub id: String,
    pub source: String,        // Source node ID
    pub source_handle: String, // Output port name
    pub target: String,        // Target node ID
    pub target_handle: String, // Input port name
}

// Port types with compatibility rules
pub enum PortDataType {
    Any, String, Image, Component, Stream, Prompt,
    Tools, Embedding, Document, Json, Boolean, Number
}
```

### Execution Flow (engine.rs:71-149)

```
1. Validate graph (cycles, types, required inputs)
       ↓
2. Topological sort via Kahn's algorithm
       ↓
3. FOR each node in order:
   ├─ Check abort signal
   ├─ Resolve inputs (upstream outputs + node.data)
   ├─ Create node instance via registry
   ├─ Execute node asynchronously
   └─ Stream events to frontend
       ↓
4. Return all outputs
```

**Critical Code Path:**
- [engine.rs:95-141](src-tauri/src/workflow/engine.rs#L95-L141) - Main execution loop (SEQUENTIAL)
- [engine.rs:154-199](src-tauri/src/workflow/engine.rs#L154-L199) - Topological sort
- [engine.rs:206-233](src-tauri/src/workflow/engine.rs#L206-L233) - Input resolution

---

## Analysis: Problems & Deficiencies

### 1. **No True Parallel Execution**
- Current: Nodes execute **sequentially** even when independent
- The topological sort *identifies* parallelizable nodes but doesn't exploit it
- Impact: Slow execution for complex graphs with independent branches

### 2. **No Hot-Reloading / Live Modification**
- Workflows cannot be modified during execution
- No support for adding/removing nodes at runtime
- User must stop execution, edit, restart

### 3. **Limited State Management**
- Node state only persists within single execution
- No persistent node state between runs
- No undo/redo for graph modifications during execution

### 4. **Tight Coupling: Registry ↔ Node Types**
- Adding new node types requires modifying `registry.rs`
- No plugin system for user-defined nodes
- Cannot dynamically load node definitions

### 5. **No Reactive Data Flow**
- Current: Pull-based execution (run entire graph)
- Missing: Push-based reactivity (re-execute when inputs change)
- No "dirty flag" propagation when upstream values update

### 6. **Memory Inefficiency**
- All node outputs stored in `HashMap<String, NodeOutputs>`
- Large outputs (images, documents) kept in memory entire execution
- No streaming of intermediate results to disk

### 7. **Limited Type System**
- Port types are checked but not generic
- No parametric types (e.g., `List<T>`, `Optional<T>`)
- No automatic type coercion pipeline

### 8. **No Node Versioning**
- Node definitions have no version field
- Breaking changes to nodes invalidate saved workflows
- No migration system for updated nodes

---

## Established Methods for Node Systems

### 1. **Data Flow Architectures**

| Pattern | Description | Used By |
|---------|-------------|---------|
| **Pull-based** | Execute from outputs backward | Houdini, USD |
| **Push-based** | Propagate changes forward | Max/MSP, PureData |
| **Reactive** | Automatic re-execution on change | Shader graphs, Svelte |
| **Hybrid** | Combine pull + push | Unreal Blueprints |

### 2. **Popular Node Graph Libraries**

| Library | Language | Key Features |
|---------|----------|--------------|
| **egui-node-graph** | Rust | Immediate mode, simple |
| **imnodes** | C++ | Dear ImGui integration |
| **NodeEditor** | C++ | Blueprint-style |
| **litegraph.js** | JS | Comfy-UI, flexible |
| **rete.js** | JS | Framework-agnostic |
| **XYFlow/React Flow** | JS | React-based (you use Svelte port) |

### 3. **Graph Execution Strategies**

| Strategy | Pros | Cons |
|----------|------|------|
| **Topological Sort** | Simple, deterministic | No parallelism |
| **Task Graph** (async) | Parallel execution | Complex dependency tracking |
| **Incremental Compute** | Only re-run dirty nodes | Complex invalidation |
| **Demand-driven** | Lazy evaluation | Hard to stream |

---

## Existing Rust Libraries for Graph Execution

### Comparison Matrix

| Library | Async | Parallel | Conditional | Cycles | Human-in-Loop | Distributed | Status |
|---------|-------|----------|-------------|--------|---------------|-------------|--------|
| [**graph-flow**](https://crates.io/crates/graph-flow) | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ | Active ⭐ |
| [**async_dag**](https://docs.rs/async_dag) | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | Stable |
| [**Timely Dataflow**](https://github.com/TimelyDataflow/timely-dataflow) | ❌ | ✅ | ❌ | ✅ | ❌ | ✅ | Mature (future Layer 2) |
| ~~dagrs~~ | - | - | - | - | - | - | ❌ Archived (merged into rk8s) |

**Note:** Demand-driven lazy evaluation will be implemented on top of graph-flow.

### Detailed Analysis

#### 1. **graph-flow** - Best for LLM/Agent Workflows ⭐

```rust
// graph-flow example
use graph_flow::{GraphBuilder, Task, Context, NextAction};

struct InferenceTask;
#[async_trait]
impl Task for InferenceTask {
    fn id(&self) -> &str { "inference" }
    async fn run(&self, ctx: &mut Context) -> Result<NextAction> {
        let prompt = ctx.get::<String>("prompt")?;
        let result = run_inference(&prompt).await?;
        ctx.set("result", result);
        Ok(NextAction::Continue)  // or GoTo("other_task"), WaitForInput, etc.
    }
}

// Build graph with conditional edges
let graph = GraphBuilder::new()
    .add_task(InferenceTask)
    .add_task(ValidationTask)
    .add_edge("inference", "validation")
    .add_conditional_edge("validation", |ctx| {
        if ctx.get::<bool>("is_valid")? { "output" } else { "retry" }
    })
    .build();
```

**Pros:**
- **Built for LLM/agent workflows** (your exact use case!)
- Human-in-the-loop with `WaitForInput`
- Conditional routing built-in
- `GoTo` and `GoBack` enable loops
- Session persistence (PostgreSQL, in-memory)
- Async-native, Tokio-based
- Active development (227 stars, 65 commits)
- Parallel execution via `FanOutTask`

**Cons:**
- No built-in lazy/pull-based evaluation
- No custom scheduler hooks
- Focused on agent workflows, may need extension for general compute

**Why graph-flow:**
1. Built specifically for LLM/agent workflows
2. Human-in-the-loop support (`WaitForInput`)
3. Session persistence (PostgreSQL backend)
4. Flexible control flow (`GoTo`/`GoBack`)

---

#### 2. **async_dag** - Simple Parallel DAG Execution

```rust
// async_dag example
let mut dag = async_dag::Dag::new();
let a = dag.add_node(async { compute_a().await });
let b = dag.add_node(async { compute_b().await });
let c = dag.add_node_with_deps(
    async { compute_c().await },
    vec![a, b]  // c depends on a and b
);
dag.run().await;  // Maximum parallelism automatically
```

**Pros:**
- Dead simple API
- Maximum parallelism by default
- Lightweight

**Cons:**
- No conditional branching
- No cycles
- No lazy evaluation
- Minimal features

#### 3. **Timely/Differential Dataflow** - Distributed Scale

```rust
// Timely Dataflow example
timely::execute_from_args(std::env::args(), |worker| {
    worker.dataflow::<u64,_,_>(|scope| {
        let input = scope.new_input();
        input
            .map(|x| x * 2)
            .filter(|x| *x > 10)
            .inspect(|x| println!("{:?}", x));
    });
}).unwrap();
```

**Pros:**
- Distributed execution across LAN cluster (your future goal!)
- Extremely high throughput (millions of records/sec)
- Mature, used in production (Materialize, etc.)
- Cyclic dataflow support (feedback loops)
- Sub-millisecond latency possible (750μs on 64 machines)

**Cons:**
- **NOT async/await** - uses its own `worker.step()` execution model
- **Epoch-based** - batches work, not ideal for interactive single-node edits
- **Push-based only** - no lazy/pull evaluation
- **No conditional branching** - you'd need to implement routing yourself
- Steep learning curve
- Requires serializable types for distribution
- Memory management challenges with unbounded operators

**Critical Issue for Your Use Case:**
The [official docs](https://timelydataflow.github.io/timely-dataflow/chapter_0/chapter_0_2.html) state Timely is designed for **throughput over latency**. For interactive graph editing where users modify nodes in real-time, the epoch-based coordination model adds overhead that makes it **poorly suited** for responsive UI interactions.

**When Timely Makes Sense:**
- Processing large data streams (logs, events, sensor data)
- Batch analytics on big datasets
- Distributed computation across many machines
- Incremental view maintenance (Differential Dataflow)

---

## Architecture: graph-flow with Future Extensibility

### What We're Building Now

Full refactor to **graph-flow** for local, interactive execution:
- Async/await native with Tokio
- Conditional routing and loops (`GoTo`/`GoBack`)
- Human-in-the-loop with `WaitForInput`
- Session persistence for long-running workflows
- Built specifically for LLM/agent workflows
- Add demand-driven lazy evaluation on top
- Add undo/redo via compressed snapshots

### Keeping Distributed Execution Possible (Minimum Viable)

We're **not implementing Timely now**, but keeping the door open by:

1. **Trait-based execution abstraction** - nodes implement a trait that could work with different backends
2. **Serializable graph state** - graphs can be serialized for transmission
3. **No hard dependencies on local-only features** in core node definitions

```rust
// Minimal abstraction to keep options open
pub trait ExecutionBackend: Send + Sync {
    async fn execute(&self, graph: &WorkflowGraph, demand: &[NodeId]) -> Result<Outputs>;
}

// What we implement now
pub struct GraphFlowBackend {
    demand_engine: DemandEngine,
    undo_stack: UndoStack,
}

// What we might add later (not implementing now)
// #[cfg(feature = "distributed")]
// pub struct TimelyBackend { ... }
```

**That's it.** No DistributedBackend implementation, no Timely dependencies, just a trait that doesn't preclude adding it later.

---

## Custom Components

### 1. Demand-Driven Lazy Evaluation

**Key insight:** Don't push dirty flags forward, pull dependencies backward.

```rust
pub struct DemandEngine {
    // Each node has a version that increments when its inputs change
    versions: HashMap<NodeId, u64>,
    // Cached outputs with the version they were computed at
    cache: HashMap<NodeId, (u64, NodeOutputs)>,
}

impl DemandEngine {
    /// Demand output from a node - only recomputes if dependencies changed
    pub async fn demand(&mut self, node_id: &NodeId, graph: &WorkflowGraph) -> Result<NodeOutputs> {
        // 1. Compute hash of all input versions
        let input_version = self.compute_input_version(node_id, graph);

        // 2. Check cache - if version matches, return cached result
        if let Some((cached_version, outputs)) = self.cache.get(node_id) {
            if *cached_version == input_version {
                return Ok(outputs.clone()); // Cache hit - no recomputation
            }
        }

        // 3. Cache miss - recursively demand dependencies first
        let mut inputs = HashMap::new();
        for dep_id in graph.get_dependencies(node_id) {
            let dep_output = self.demand(&dep_id, graph).await?;
            inputs.insert(dep_id, dep_output);
        }

        // 4. Execute this node
        let outputs = self.execute_node(node_id, inputs).await?;

        // 5. Cache with current version
        self.cache.insert(node_id.clone(), (input_version, outputs.clone()));
        Ok(outputs)
    }

    fn compute_input_version(&self, node_id: &NodeId, graph: &WorkflowGraph) -> u64 {
        graph.get_dependencies(node_id)
            .map(|dep| self.versions.get(&dep).unwrap_or(&0))
            .fold(0u64, |acc, v| acc.wrapping_add(*v))
    }
}
```

**Why this is efficient:**
- Only traverses the subgraph leading to requested outputs
- Skips entire branches that aren't needed
- O(path length) not O(graph size) for cache hits

### 2. Undo/Redo System

Using **compressed immutable snapshots** (simpler than command pattern, faster than git):

```rust
pub struct UndoStack {
    snapshots: VecDeque<Vec<u8>>,  // zstd-compressed graph states
    current: usize,
    max_snapshots: usize,
}

impl UndoStack {
    pub fn push(&mut self, graph: &WorkflowGraph) -> Result<()> {
        let json = serde_json::to_vec(graph)?;
        let compressed = zstd::encode_all(&json[..], 3)?;  // Level 3 = fast

        // Truncate any redo history
        self.snapshots.truncate(self.current + 1);
        self.snapshots.push_back(compressed);
        self.current = self.snapshots.len() - 1;

        // Trim old snapshots if over limit
        while self.snapshots.len() > self.max_snapshots {
            self.snapshots.pop_front();
            self.current -= 1;
        }
        Ok(())
    }

    pub fn undo(&mut self) -> Option<WorkflowGraph> {
        if self.current > 0 {
            self.current -= 1;
            self.decompress(self.current)
        } else {
            None
        }
    }

    pub fn redo(&mut self) -> Option<WorkflowGraph> {
        if self.current + 1 < self.snapshots.len() {
            self.current += 1;
            self.decompress(self.current)
        } else {
            None
        }
    }
}
```

**Why snapshots over command pattern:**
- No need to implement inverse for every operation
- Simple, reliable, works with any graph mutation
- zstd compression is fast (~500MB/s) and effective (~10x reduction)

### 3. Performance Metrics (Future Enhancement)

The inference library can provide memory/timing data via its API. Add performance logging:

```rust
pub struct NodeMetrics {
    pub execution_time_ms: u64,
    pub memory_peak_bytes: usize,
    pub input_size_bytes: usize,
    pub output_size_bytes: usize,
}

// Collect during execution, use for future scheduling decisions
impl NodeEngine {
    async fn execute_with_metrics(&self, node_id: &NodeId) -> (NodeOutputs, NodeMetrics) {
        let start = Instant::now();
        let mem_before = get_memory_usage();

        let outputs = self.execute_node(node_id).await?;

        let metrics = NodeMetrics {
            execution_time_ms: start.elapsed().as_millis() as u64,
            memory_peak_bytes: get_peak_memory() - mem_before,
            // ...
        };

        self.metrics_store.record(node_id, metrics.clone());
        (outputs, metrics)
    }
}
```

This data can inform future scheduler optimizations without implementing a complex scheduler now.

---

## Standalone Library Design (`node-engine`)

### Yes, Absolutely Possible

The workflow engine can be extracted into a reusable crate. The current code in `src-tauri/src/workflow/` is already fairly decoupled - the main Tauri dependencies are:
- `Channel<WorkflowEvent>` for streaming (replaceable with generic callback/channel)
- `State<'_, T>` for dependency injection (replaceable with constructor params)

### Proposed Crate Structure

```
node-engine/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── node.rs          # Node trait (generic, no Tauri)
│   ├── types.rs         # Graph, Edge, Port types
│   ├── engine.rs        # Execution engine
│   ├── runtime.rs       # Live execution state
│   ├── validation.rs    # Graph validation
│   ├── algorithms/
│   │   ├── mod.rs
│   │   ├── toposort.rs      # Topological sorting
│   │   ├── dirty.rs         # Dirty propagation
│   │   ├── reachability.rs  # Connected/disconnected detection
│   │   └── parallel.rs      # Parallel execution scheduling
│   └── events.rs        # Generic event types
```

### Key Abstraction: Generic Event Channel

```rust
// Instead of Tauri's Channel, use a generic trait
pub trait EventSink: Send + Sync {
    fn send(&self, event: WorkflowEvent) -> Result<(), EventError>;
}

// Tauri adapter (in pantograph app, not the library)
impl EventSink for tauri::ipc::Channel<WorkflowEvent> {
    fn send(&self, event: WorkflowEvent) -> Result<(), EventError> {
        Channel::send(self, event).map_err(|_| EventError::ChannelClosed)
    }
}

// For testing or CLI usage
pub struct VecSink(Arc<Mutex<Vec<WorkflowEvent>>>);
impl EventSink for VecSink { ... }
```

### Architecture: Operations as Nodes

The library provides the **execution engine only** - no built-in LLM/RAG/DB nodes. These are implemented as nodes in the consuming application:

```
┌─────────────┐     ┌─────────────┐     ┌──────────────┐
│ Text Input  │────▶│ Model Node  │────▶│  Inference   │
└─────────────┘     │ (llama-7b)  │     │    Node      │
                    └─────────────┘     └──────┬───────┘
                                               │
┌─────────────┐     ┌─────────────┐            │
│ Image Input │────▶│ Embedding   │◀───────────┘
└─────────────┘     │    Node     │
                    └──────┬──────┘
                           │ vector output
                           ▼
                    ┌─────────────┐     ┌──────────────┐
                    │  LanceDB    │────▶│ Unload Model │
                    │   Store     │     │     Node     │
                    └─────────────┘     └──────────────┘
```

**Library provides:**
- Graph structure (`WorkflowGraph`, `GraphNode`, `GraphEdge`)
- Execution engine (topological sort, dirty propagation, parallel scheduling)
- Node trait interface
- Event streaming
- Validation

**Application provides:**
- Concrete node implementations (InferenceNode, EmbeddingNode, LanceDBNode, etc.)
- Resource management (model loading/unloading)
- Backend integrations

```rust
// Library: Generic node trait
pub trait Node: Send + Sync {
    fn definition(&self) -> &NodeDefinition;
    fn id(&self) -> &str;

    async fn execute(
        &self,
        inputs: NodeInputs,
        context: &dyn ExecutionContext,  // App-provided context
        events: &dyn EventSink,
    ) -> Result<NodeOutputs, NodeError>;
}

// Library: Minimal context trait
pub trait ExecutionContext: Send + Sync {
    fn is_aborted(&self) -> bool;
    fn project_root(&self) -> &Path;
}

// Application: Extended context with app-specific resources
pub struct PantographContext {
    pub abort_signal: Arc<AtomicBool>,
    pub project_root: PathBuf,
    pub gateway: Arc<InferenceGateway>,      // App-specific
    pub rag_manager: Arc<RwLock<RagManager>>, // App-specific
    pub model_registry: Arc<ModelRegistry>,   // App-specific
}

impl ExecutionContext for PantographContext { ... }
```

### New Port Types for Model/Resource Flow

```rust
pub enum PortDataType {
    // Existing
    Any, String, Image, Number, Boolean, Json,

    // NEW: Resource handles (not data, but references)
    ModelHandle,      // Reference to a loaded model
    EmbeddingHandle,  // Reference to embedding model
    DatabaseHandle,   // Reference to LanceDB/vector store connection

    // NEW: ML-specific data
    Vector,           // Embedding vector (Vec<f32>)
    Tensor,           // Generic tensor (for advanced use)
    AudioSamples,     // Raw audio data
}
```

### Example: Model Lifecycle as Nodes

```rust
// LoadModelNode - outputs a ModelHandle
pub struct LoadModelNode { ... }
impl Node for LoadModelNode {
    async fn execute(&self, inputs: NodeInputs, ctx: &dyn ExecutionContext, events: &dyn EventSink)
        -> Result<NodeOutputs, NodeError>
    {
        let model_path = inputs.get_string("model_path")?;
        let ctx = ctx.downcast_ref::<PantographContext>()?;

        let handle = ctx.model_registry.load(model_path).await?;
        Ok(outputs!["model" => handle.to_json()])
    }
}

// InferenceNode - takes ModelHandle + prompt, outputs text
pub struct InferenceNode { ... }
impl Node for InferenceNode {
    async fn execute(&self, inputs: NodeInputs, ctx: &dyn ExecutionContext, events: &dyn EventSink)
        -> Result<NodeOutputs, NodeError>
    {
        let model_handle = inputs.get_string("model")?;  // ModelHandle as JSON
        let prompt = inputs.get_string("prompt")?;
        let ctx = ctx.downcast_ref::<PantographContext>()?;

        let response = ctx.gateway.infer(model_handle, prompt).await?;
        Ok(outputs!["response" => response])
    }
}

// UnloadModelNode - takes ModelHandle, triggers cleanup
pub struct UnloadModelNode { ... }
impl Node for UnloadModelNode {
    async fn execute(&self, inputs: NodeInputs, ctx: &dyn ExecutionContext, events: &dyn EventSink)
        -> Result<NodeOutputs, NodeError>
    {
        let model_handle = inputs.get_string("model")?;
        let ctx = ctx.downcast_ref::<PantographContext>()?;

        ctx.model_registry.unload(model_handle).await?;
        Ok(outputs!["done" => true])
    }
}
```

### Graph Sequencing for Resource Management

The topological sort naturally handles the ordering:
1. LoadModelNode executes first (no dependencies)
2. InferenceNode waits for model handle
3. EmbeddingNode waits for inference output
4. LanceDBNode waits for vector
5. UnloadModelNode waits for all downstream nodes

This ensures models are loaded before use and unloaded after all dependent operations complete.

### Challenge: Resource Cleanup in Live Editing

When the graph is modified mid-execution, resource nodes need special handling:

**Problem:** User deletes connection to UnloadModelNode → model stays loaded forever

**Solutions:**

1. **Reference counting on handles:**
   ```rust
   pub struct ResourceHandle {
       id: Uuid,
       ref_count: Arc<AtomicUsize>,
   }

   impl Drop for ResourceHandle {
       fn drop(&mut self) {
           if self.ref_count.fetch_sub(1, Ordering::SeqCst) == 1 {
               // Last reference, cleanup resource
           }
       }
   }
   ```

2. **Cleanup phase after execution:**
   ```rust
   impl WorkflowRuntime {
       async fn post_execution_cleanup(&self) {
           // Find all resource handles that are no longer referenced
           // by any node in the graph
           let orphaned = self.find_orphaned_resources();
           for handle in orphaned {
               self.resource_manager.release(handle).await;
           }
       }
   }
   ```

3. **Explicit "scope" nodes** (like RAII blocks):
   ```
   ┌─────────────────────────────────────┐
   │  Model Scope Node                   │
   │  ┌───────────┐    ┌───────────┐    │
   │  │Load Model │───▶│ Inference │    │
   │  └───────────┘    └─────┬─────┘    │
   │                         │          │
   │  (auto-unload on scope exit)       │
   └─────────────────────────────────────┘
   ```

---

## Established Algorithms for Node Graphs at Scale

### 1. Topological Sort (Current: Kahn's Algorithm) ✓

You already have this. **O(V + E)** - scales linearly with graph size.

### 2. Dirty Flag Propagation

**Algorithm:** BFS/DFS from modified node through outgoing edges.

```rust
// O(V + E) worst case, but typically touches small subgraph
fn propagate_dirty(graph: &Graph, start: NodeId, dirty: &mut HashSet<NodeId>) {
    let mut queue = VecDeque::from([start]);
    while let Some(node) = queue.pop_front() {
        if dirty.insert(node) {
            for edge in graph.outgoing_edges(node) {
                queue.push_back(edge.target);
            }
        }
    }
}
```

**Optimization for hundreds of nodes:** Use **generation counters** instead of HashSet:
```rust
struct DirtyTracker {
    generation: u64,
    node_generations: HashMap<NodeId, u64>,
}

impl DirtyTracker {
    fn mark_dirty(&mut self, node: NodeId) {
        self.node_generations.insert(node, self.generation);
    }

    fn is_dirty(&self, node: NodeId) -> bool {
        self.node_generations.get(&node) == Some(&self.generation)
    }

    fn next_generation(&mut self) {
        self.generation += 1;  // O(1) clear all dirty flags
    }
}
```

### 3. Disconnected/Inactive Node Detection

**Algorithm:** Reverse reachability from output nodes.

```rust
// Find all nodes reachable from outputs (working backward)
fn find_active_nodes(graph: &Graph, output_node_ids: &[NodeId]) -> HashSet<NodeId> {
    let mut active = HashSet::new();
    let mut queue: VecDeque<NodeId> = output_node_ids.iter().cloned().collect();

    while let Some(node) = queue.pop_front() {
        if active.insert(node) {
            // Add all nodes that feed INTO this node
            for edge in graph.incoming_edges(node) {
                queue.push_back(edge.source);
            }
        }
    }
    active
}

// Disconnected = all nodes NOT in active set
let disconnected: Vec<_> = graph.nodes()
    .filter(|n| !active.contains(&n.id))
    .collect();
```

### 4. Parallel Execution Scheduling

**Algorithm:** Level-based scheduling (nodes at same topological level can run in parallel).

```rust
fn compute_execution_levels(graph: &Graph) -> Vec<Vec<NodeId>> {
    let mut levels: Vec<Vec<NodeId>> = Vec::new();
    let mut in_degree: HashMap<NodeId, usize> = HashMap::new();
    let mut node_level: HashMap<NodeId, usize> = HashMap::new();

    // Initialize in-degrees
    for node in graph.nodes() {
        in_degree.insert(node.id, 0);
    }
    for edge in graph.edges() {
        *in_degree.get_mut(&edge.target).unwrap() += 1;
    }

    // BFS with level tracking
    let mut queue: VecDeque<(NodeId, usize)> = in_degree.iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| (id, 0))
        .collect();

    while let Some((node, level)) = queue.pop_front() {
        // Ensure level exists
        while levels.len() <= level {
            levels.push(Vec::new());
        }
        levels[level].push(node);
        node_level.insert(node, level);

        for edge in graph.outgoing_edges(node) {
            let target_degree = in_degree.get_mut(&edge.target).unwrap();
            *target_degree -= 1;
            if *target_degree == 0 {
                // Target's level = max(source levels) + 1
                let target_level = level + 1;
                queue.push_back((edge.target, target_level));
            }
        }
    }

    levels
}

// Execute levels in parallel
for level in levels {
    let futures: Vec<_> = level.iter()
        .map(|node_id| tokio::spawn(execute_node(node_id)))
        .collect();
    futures::future::join_all(futures).await;
}
```

### 5. Incremental Computation (Advanced)

For truly large graphs (1000+ nodes), consider Timely Dataflow for distributed execution (Layer 2 of our architecture).

**Key insight for incremental computation:** Instead of eager dirty propagation, use **demand-driven** recomputation:
1. Mark inputs as changed
2. When output is requested, walk backward checking if dependencies changed
3. Only recompute what's actually needed for the requested output

This avoids the "push problem" where eager propagation can cause exponential recomputation.

### 6. Cycle Detection

Already implemented via Kahn's algorithm (if result.len() < nodes.len(), cycle exists).

For **feedback loops** (intentional cycles), use:
- Iteration limits
- Convergence detection (output unchanged from previous iteration)
- Explicit "delay" nodes that break the cycle

---

## Scaling Considerations for Hundreds of Nodes

| Concern | Solution |
|---------|----------|
| **Memory** | Stream large outputs to disk, use output handles instead of values |
| **Execution time** | Parallel level-based execution |
| **UI responsiveness** | Debounce dirty propagation, batch updates |
| **Graph queries** | Index edges by source/target (HashMap, not linear search) |
| **Serialization** | Lazy loading of node data, only load visible nodes |

### Current Code Bottleneck

In [types.rs:243-250](src-tauri/src/workflow/types.rs#L243-L250), edge queries are O(E):

```rust
// Current: Linear scan
pub fn incoming_edges(&self, node_id: &str) -> impl Iterator<Item = &GraphEdge> {
    self.edges.iter().filter(move |e| e.target == node_id)
}
```

**Fix for scale:** Add edge indices:

```rust
pub struct WorkflowGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    // NEW: Indices for O(1) edge lookup
    edges_by_source: HashMap<String, Vec<usize>>,
    edges_by_target: HashMap<String, Vec<usize>>,
}
```

---

## Implementation Plan: Real-Time Graph Editing

### Goal
Allow users to modify node graphs while execution is running, with changes taking effect immediately.

---

### Architecture Changes Required

#### 1. Persistent Execution State

Currently, `WorkflowEngine::execute()` is a one-shot function. Need a **long-running execution context**:

```rust
// NEW: src-tauri/src/workflow/runtime.rs
pub struct WorkflowRuntime {
    graph: Arc<RwLock<WorkflowGraph>>,
    node_outputs: Arc<RwLock<HashMap<String, NodeOutputs>>>,
    dirty_nodes: Arc<RwLock<HashSet<String>>>,
    execution_state: Arc<RwLock<RuntimeState>>,
    context: ExecutionContext,
}

pub enum RuntimeState {
    Idle,
    Running,
    Paused,
    Modified,  // Graph changed, awaiting re-execution
}
```

#### 2. Dirty Flag Propagation

When a node or edge changes, mark downstream nodes as dirty:

```rust
impl WorkflowRuntime {
    pub fn mark_dirty(&self, node_id: &str) {
        let graph = self.graph.read().await;
        let mut dirty = self.dirty_nodes.write().await;

        dirty.insert(node_id.to_string());

        // Propagate to all downstream nodes
        let mut queue = VecDeque::from([node_id]);
        while let Some(current) = queue.pop_front() {
            for edge in graph.outgoing_edges(current) {
                if dirty.insert(edge.target.clone()) {
                    queue.push_back(&edge.target);
                }
            }
        }
    }
}
```

#### 3. Incremental Re-execution

Only execute dirty nodes in topological order:

```rust
impl WorkflowRuntime {
    pub async fn execute_dirty(&self, channel: &Channel<WorkflowEvent>) -> Result<(), WorkflowError> {
        let dirty_ids: Vec<String> = self.dirty_nodes.read().await.iter().cloned().collect();

        // Get topological order of ONLY dirty nodes
        let order = self.topological_sort_subset(&dirty_ids).await;

        for node_id in order {
            // Execute node, update outputs
            let result = self.execute_node(&node_id, channel).await?;
            self.node_outputs.write().await.insert(node_id.clone(), result);
            self.dirty_nodes.write().await.remove(&node_id);
        }

        Ok(())
    }
}
```

#### 4. Live Modification Commands

New Tauri commands for real-time editing:

```rust
// src-tauri/src/workflow/commands.rs

#[command]
pub async fn runtime_add_node(
    runtime: State<'_, SharedRuntime>,
    node: GraphNode,
) -> Result<(), String> {
    let mut graph = runtime.graph.write().await;
    graph.nodes.push(node.clone());
    runtime.mark_dirty(&node.id).await;
    Ok(())
}

#[command]
pub async fn runtime_update_node_data(
    runtime: State<'_, SharedRuntime>,
    node_id: String,
    data: serde_json::Value,
) -> Result<(), String> {
    let mut graph = runtime.graph.write().await;
    if let Some(node) = graph.nodes.iter_mut().find(|n| n.id == node_id) {
        node.data = data;
    }
    runtime.mark_dirty(&node_id).await;
    runtime.execute_dirty(&channel).await?;  // Auto re-execute
    Ok(())
}

#[command]
pub async fn runtime_add_edge(
    runtime: State<'_, SharedRuntime>,
    edge: GraphEdge,
) -> Result<(), String> {
    let mut graph = runtime.graph.write().await;
    graph.edges.push(edge.clone());
    runtime.mark_dirty(&edge.target).await;
    runtime.execute_dirty(&channel).await?;
    Ok(())
}
```

#### 5. Frontend Integration

New events for runtime state:

```rust
// src-tauri/src/workflow/events.rs
pub enum WorkflowEvent {
    // ... existing events ...

    // NEW: Runtime events
    RuntimeStarted { runtime_id: String },
    RuntimePaused,
    RuntimeResumed,
    GraphModified { dirty_nodes: Vec<String> },
    IncrementalExecutionStarted { nodes: Vec<String> },
}
```

---

### Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `src-tauri/src/workflow/runtime.rs` | **CREATE** | Long-running execution state |
| `src-tauri/src/workflow/engine.rs` | MODIFY | Extract reusable execution logic |
| `src-tauri/src/workflow/commands.rs` | MODIFY | Add runtime commands |
| `src-tauri/src/workflow/events.rs` | MODIFY | Add runtime events |
| `src-tauri/src/workflow/mod.rs` | MODIFY | Export runtime module |
| `src-tauri/src/main.rs` | MODIFY | Register SharedRuntime state |
| `src/services/workflow/WorkflowService.ts` | MODIFY | Add runtime methods |
| `src/stores/workflowStore.ts` | MODIFY | Track runtime state |

---

### Implementation Steps

1. **Create `WorkflowRuntime` struct** with graph, outputs, dirty set
2. **Implement `mark_dirty()`** with downstream propagation
3. **Implement `execute_dirty()`** for incremental execution
4. **Add Tauri commands** for live modification
5. **Add runtime events** for frontend sync
6. **Update frontend** to use runtime commands during editing
7. **Add pause/resume** functionality

---

### Testing Strategy

1. Unit test dirty propagation (A→B→C, modify A, B and C marked dirty)
2. Integration test: modify node data during execution, verify re-run
3. E2E test: frontend edits while workflow running

---

## Implementation Order

This is a **full refactor** to graph-flow, not an extraction with adapters. The current Node trait and WorkflowEngine will be replaced.

### Phase 1: Research & Spike (Sequential)

Before committing to graph-flow, verify it works for our use case:

1. **Add graph-flow dependency** to a test project
2. **Implement one node** (e.g., InferenceTask) using graph-flow's Task trait
3. **Test WaitForInput** for human-in-the-loop
4. **Verify event streaming** works with Tauri channels
5. **Document any issues** or required workarounds

### Phase 1 Results: Spike Complete

The spike is complete and **graph-flow works well** for our use case. Key findings:

#### What Works
- **Task trait** is simple: implement `id()` and async `run()` methods
- **Context API** is clean: `context.get()` returns `Option<T>`, `context.set()` returns `()`
- **WaitForInput** pauses execution correctly - just return `NextAction::WaitForInput`
- **NextAction enum** provides all control flow we need: `Continue`, `WaitForInput`, `GoTo`, `GoBack`, `End`
- **GraphError** has useful variants: `TaskExecutionFailed`, `ContextError`, etc.

#### API Details Learned
```rust
// Context operations are async but don't return Result
context.set("key", value).await;                    // returns ()
let value: Option<T> = context.get("key").await;    // returns Option<T>
let value: Option<T> = context.get_sync("key");     // sync version for conditional edges

// TaskResult construction
TaskResult::new(Some("response".to_string()), NextAction::Continue);
TaskResult::new(Some("waiting".to_string()), NextAction::WaitForInput);

// Error handling uses GraphError
Err(GraphError::TaskExecutionFailed("error message".to_string()))
```

#### Gotchas Found
1. **Context returns Option, not Result** - The initial research suggested `Result<T>` but actual API returns `Option<T>`
2. **context.set() returns ()** - No error handling needed for set operations
3. **GraphError not Error** - The error type is `GraphError`, not `graph_flow::Error`
4. **Need explicit lifetime annotations** for iterator methods returning closures

#### Files Created
```
crates/node-engine/
├── Cargo.toml              # graph-flow 0.2, reqwest, zstd, tokio, serde
├── src/
│   ├── lib.rs              # Re-exports key types
│   ├── engine.rs           # DemandEngine with version tracking
│   ├── error.rs            # NodeEngineError enum
│   ├── events.rs           # EventSink trait + WorkflowEvent enum
│   ├── types.rs            # WorkflowGraph, GraphNode, GraphEdge, PortDataType
│   ├── undo.rs             # UndoStack with zstd compression
│   └── tasks/
│       ├── mod.rs          # ContextKeys helper
│       ├── inference.rs    # InferenceTask (OpenAI-compatible API)
│       └── human_input.rs  # HumanInputTask (WaitForInput demo)
```

#### Tests Passing
All 20 tests pass:
- DemandEngine: version tracking, cache invalidation, cache stats
- UndoStack: push/undo, redo, truncation, max snapshots
- Events: NullEventSink, VecEventSink
- Types: port compatibility, graph edges
- Tasks: InferenceTask config, HumanInputTask WaitForInput flow

**Verdict:** Proceed with Phase 2. graph-flow is the right choice.

---

### Phase 2: Setup Workspace (Sequential)

1. Create `Cargo.toml` (workspace root)
2. Create `crates/node-engine/Cargo.toml` with graph-flow dependency
3. Create `crates/node-engine/src/lib.rs`
4. Update `src-tauri/Cargo.toml` to use workspace

### Phase 3: Core Engine (2 Parallel Agents)

| Agent | Task | Details |
|-------|------|---------|
| **Agent A** | Create DemandEngine | Implement demand-driven lazy evaluation with version tracking |
| **Agent B** | Create UndoStack | Implement compressed snapshot-based undo/redo |

### Phase 4: Rewrite Nodes (Parallel per node type)

Convert each existing node to graph-flow Task trait:

| Current Node | New Task | Notes |
|--------------|----------|-------|
| InferenceNode | InferenceTask | Use `ctx.get()`/`ctx.set()` instead of inputs/outputs HashMap |
| EmbeddingNode | EmbeddingTask | Same pattern |
| RAGNode | RAGTask | May use `WaitForInput` for user confirmation |
| etc. | etc. | Each node is independent, can parallelize |

### Phase 5: Integration (Sequential) ✅ COMPLETE

1. ✅ Update Tauri commands to use new engine
2. ✅ Wire up event streaming from graph-flow to frontend
3. ✅ Implement undo/redo Tauri commands
4. ✅ Test with frontend (build passes)

**Files Created:**
- `src-tauri/src/workflow/event_adapter.rs` - Bridges node-engine EventSink → Tauri Channel
- `src-tauri/src/workflow/execution_manager.rs` - Manages execution state with undo/redo
- `src-tauri/src/workflow/task_executor.rs` - Bridges node-engine tasks with Tauri resources

**Files Modified:**
- `src-tauri/src/workflow/mod.rs` - Added new module exports
- `src-tauri/src/workflow/commands.rs` - Added V2 commands (execute_workflow_v2, undo_workflow, redo_workflow, update_node_data, etc.)
- `src-tauri/src/main.rs` - Added ExecutionManager to Tauri state, registered new commands
- `src/services/workflow/WorkflowService.ts` - Added executeWorkflowV2, undo, redo, graph modification methods

**New Tauri Commands:**
- `execute_workflow_v2` - Node-engine based execution with demand-driven evaluation
- `get_undo_redo_state` - Get current undo/redo state
- `undo_workflow` / `redo_workflow` - Undo/redo graph modifications
- `update_node_data` - Update node data during execution
- `add_node_to_execution` / `add_edge_to_execution` / `remove_edge_from_execution` - Live graph modification
- `get_execution_graph` - Get current graph state
- `remove_execution` - Cleanup execution

### Phase 6: Cleanup (Sequential) ✅ COMPLETE

1. ✅ Remove old workflow/engine.rs, workflow/node.rs
2. ✅ Remove old node implementations (nodes/ directory)
3. ✅ Update frontend types (PortValue now defined locally)

**Files Deleted:**
- `src-tauri/src/workflow/engine.rs` - Old V1 synchronous execution engine
- `src-tauri/src/workflow/node.rs` - Old V1 Node trait definition
- `src-tauri/src/workflow/nodes/` - All old V1 node implementations (input.rs, output.rs, processing.rs, tools.rs, control.rs, mod.rs)

**Files Modified:**
- `src-tauri/src/workflow/mod.rs` - Removed old module exports (engine, node, nodes)
- `src-tauri/src/workflow/commands.rs` - Removed old execute_workflow command
- `src-tauri/src/workflow/registry.rs` - Rewritten to only provide node definitions (no Node trait dependency)
- `src-tauri/src/workflow/events.rs` - Added local PortValue type alias
- `src-tauri/src/workflow/event_adapter.rs` - Added local PortValue type alias
- `src-tauri/src/main.rs` - Removed old execute_workflow command registration

**Verification:**
- ✅ `cargo build --package pantograph` passes
- ✅ `cargo test --package node-engine` - All 70 tests pass
- Note: Pre-existing test issue in `candle.rs` (CandleBackend.description() method missing) is unrelated to Phase 6

---

### Proposed Crate Structure

```
Pantograph/
├── Cargo.toml                    # Workspace root
├── crates/
│   └── node-engine/
│       ├── Cargo.toml            # Depends on graph-flow
│       └── src/
│           ├── lib.rs
│           ├── engine.rs         # GraphFlowBackend + DemandEngine
│           ├── undo.rs           # UndoStack
│           ├── types.rs          # Graph, Edge, Port types (keep)
│           ├── validation.rs     # Graph validation (keep)
│           └── tasks/            # All node implementations as Tasks
│               ├── mod.rs
│               ├── inference.rs
│               ├── embedding.rs
│               ├── rag.rs
│               └── ...
├── src-tauri/
│   ├── Cargo.toml               # Depends on node-engine
│   └── src/
│       └── workflow/
│           ├── mod.rs           # Tauri integration
│           ├── commands.rs      # Tauri commands
│           └── events.rs        # Event streaming to frontend
└── src/                          # Frontend (unchanged)
```

### Cargo.toml (workspace root)

```toml
[workspace]
resolver = "2"
members = [
    "crates/node-engine",
    "src-tauri",
]

[workspace.dependencies]
node-engine = { path = "crates/node-engine" }
graph-flow = "0.x"  # Check latest version
zstd = "0.13"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
```

---

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `Cargo.toml` (root) | CREATE | Workspace definition |
| `crates/node-engine/Cargo.toml` | CREATE | Library crate with graph-flow dep |
| `crates/node-engine/src/lib.rs` | CREATE | Library exports |
| `crates/node-engine/src/engine.rs` | CREATE | GraphFlowBackend + DemandEngine |
| `crates/node-engine/src/undo.rs` | CREATE | UndoStack with zstd compression |
| `crates/node-engine/src/types.rs` | KEEP | Graph, Edge, Port types (reuse) |
| `crates/node-engine/src/validation.rs` | KEEP | Graph validation (reuse) |
| `crates/node-engine/src/tasks/*.rs` | CREATE | New Task implementations |
| `src-tauri/Cargo.toml` | MODIFY | Add workspace dep |
| `src-tauri/src/workflow/mod.rs` | REWRITE | Use new engine |
| `src-tauri/src/workflow/commands.rs` | REWRITE | New Tauri commands |
| `src-tauri/src/workflow/node.rs` | DELETE | Replaced by graph-flow Task |
| `src-tauri/src/workflow/engine.rs` | DELETE | Replaced by node-engine |
| `src-tauri/src/workflow/nodes/*.rs` | DELETE | Replaced by tasks/ |

---

## Verification

1. **Spike passes:**
   - graph-flow Task works with async inference
   - WaitForInput pauses execution correctly
   - Events stream to Tauri channel

2. **Library compiles standalone:**
   ```bash
   cd crates/node-engine && cargo build
   ```

3. **No Tauri dependencies in library:**
   ```bash
   cargo tree -p node-engine | grep -i tauri  # Should be empty
   ```

4. **App works end-to-end:**
   ```bash
   pnpm tauri dev
   # Test: Create graph, run, modify during execution, undo/redo
   ```

---

## Summary

**What we're doing:**
1. Full refactor from custom Node trait to graph-flow's Task model
2. Add demand-driven lazy evaluation (pull-based, not push-based)
3. Add undo/redo via compressed snapshots
4. Keep architecture open for future Timely Dataflow (but don't implement now)

**What we're NOT doing:**
- No adapter layers or backwards compatibility
- No Timely implementation (just don't preclude it)
- No complex scheduler (collect metrics for future optimization instead)

**Key files to keep:**
- `types.rs` - Graph structure definitions
- `validation.rs` - Graph validation logic

**Key files to delete/replace:**
- `node.rs` - Replaced by graph-flow Task
- `engine.rs` - Replaced by node-engine with graph-flow
- `nodes/*.rs` - Replaced by tasks/*.rs
