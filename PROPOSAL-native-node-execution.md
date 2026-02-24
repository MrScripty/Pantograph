# Proposal: Native Node Execution in Core

## Problem

Every node in Pantograph is a stub. The `llamacpp-inference`, `puma-lib`, `text-input`,
`text-output`, and all other nodes define metadata via `TaskDescriptor` but their `run()`
always returns an error directing callers to the "callback bridge." This forces every host
application to reimplement the execution logic for every node type.

Currently there are two hosts that each duplicate this work:

1. **Tauri app** (`src-tauri/src/workflow/task_executor.rs`): `PantographTaskExecutor`
   implements `TaskExecutor` with a 1200-line match statement covering 20+ node types —
   `text-input`, `text-output`, `llm-inference`, `llamacpp-inference`, `puma-lib`,
   `ollama-inference`, `conditional`, `merge`, `read-file`, `write-file`, `validator`,
   `json-filter`, `model-provider`, `unload-model`, `vision-analysis`, `human-input`,
   `tool-executor`, `image-input`, `linked-input`, `component-preview`, `rag-search`.

2. **NIF/Elixir** (`crates/pantograph-rustler/src/lib.rs`): `ElixirCallbackTaskExecutor`
   delegates every node to Elixir via the callback bridge. The host (Bewilder) then must
   implement handlers for each node type in Elixir — duplicating the same logic a third time.

This means:

- Adding a new node type requires changes in 3 places (descriptor, Tauri executor, Elixir
  callbacks)
- The core crate is not usable standalone — it cannot execute a single node without a host
- The `crates/inference/` crate has `InferenceGateway`, `LlamaCppBackend`, and
  `StdProcessSpawner` ready for headless use, but nothing wires them into node execution
- NIF consumers get `econnrefused` because no inference server is managed — that is supposed
  to be Pantograph's job

## Proposed Solution

### Part 1: Built-in `CoreTaskExecutor` in `node-engine`

Create a `CoreTaskExecutor` in the `node-engine` crate that handles all nodes whose logic
is not host-specific. This is where the bulk of the Tauri `PantographTaskExecutor` match
arms move to.

```rust
// crates/node-engine/src/core_executor.rs

pub struct CoreTaskExecutor {
    /// Inference gateway for LLM nodes
    gateway: Arc<InferenceGateway>,
    /// Extensions (PumasApi, etc.)
    extensions: Arc<RwLock<ExecutorExtensions>>,
}

impl CoreTaskExecutor {
    pub fn new(gateway: Arc<InferenceGateway>) -> Self { ... }

    /// Builder to inject extensions
    pub fn with_extensions(mut self, ext: Arc<RwLock<ExecutorExtensions>>) -> Self { ... }
}

#[async_trait]
impl TaskExecutor for CoreTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        context: &Context,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let node_type = resolve_node_type(task_id, &inputs);
        match node_type.as_str() {
            // --- Pure nodes (no host dependencies) ---
            "text-input"       => execute_text_input(&inputs),
            "text-output"      => execute_text_output(&inputs),
            "conditional"      => execute_conditional(&inputs),
            "merge"            => execute_merge(&inputs),
            "json-filter"      => execute_json_filter(&inputs),
            "validator"        => execute_validator(&inputs),
            "model-provider"   => execute_model_provider(&inputs),

            // --- Inference nodes (use InferenceGateway) ---
            "llamacpp-inference" => self.execute_llamacpp(&inputs).await,
            "ollama-inference"   => self.execute_ollama(&inputs).await,
            "llm-inference"      => self.execute_llm(&inputs).await,
            "unload-model"       => self.execute_unload(&inputs).await,

            // --- Setup nodes (use extensions) ---
            "puma-lib" => self.execute_puma_lib(&inputs, extensions).await,

            // --- Delegate to host for truly host-specific nodes ---
            _ => Err(NodeEngineError::ExecutionFailed(
                format!("Node type '{}' requires host-specific executor", node_type)
            )),
        }
    }
}
```

Nodes that are truly host-specific (like `component-preview` which needs Tauri UI,
`rag-search` which needs Tauri's RagManager, `human-input` which needs UI interaction)
remain delegated. Everything else lives in core.

### Part 2: `CompositeTaskExecutor` for host overrides

Hosts may need to override or extend core behavior. A composite executor tries the
host-specific executor first, falls back to core:

```rust
// crates/node-engine/src/composite_executor.rs

pub struct CompositeTaskExecutor {
    /// Host-specific overrides (tried first)
    host: Option<Arc<dyn TaskExecutor>>,
    /// Core executor (fallback)
    core: Arc<CoreTaskExecutor>,
}

#[async_trait]
impl TaskExecutor for CompositeTaskExecutor {
    async fn execute_task(&self, task_id, inputs, context, extensions) -> Result<...> {
        // Try host first (for host-specific nodes)
        if let Some(ref host) = self.host {
            match host.execute_task(task_id, inputs.clone(), context, extensions).await {
                Ok(result) => return Ok(result),
                Err(NodeEngineError::ExecutionFailed(msg))
                    if msg.contains("requires host-specific") => {
                    // Host doesn't handle this type, fall through to core
                }
                Err(e) => return Err(e),
            }
        }
        // Fall back to core
        self.core.execute_task(task_id, inputs, context, extensions).await
    }
}
```

### Part 3: Wire `InferenceGateway` into core execution

The `llamacpp-inference` handler in `CoreTaskExecutor` uses the `InferenceGateway` to:

1. Check if a server is running with the right model
2. Start one if not (using the injected `ProcessSpawner`)
3. Make the inference call
4. Return the result

```rust
impl CoreTaskExecutor {
    async fn execute_llamacpp(&self, inputs: &HashMap<String, Value>) -> Result<...> {
        let model_path = inputs.get("model_path")
            .and_then(|m| m.as_str())
            .ok_or(ExecutionFailed("Missing model_path input"))?;

        let model_path = resolve_gguf_path(model_path)?;

        // Ensure gateway is ready with this model
        if !self.gateway.is_ready().await {
            let config = BackendConfig {
                model_path: Some(PathBuf::from(&model_path)),
                ..Default::default()
            };
            self.gateway.start(&config).await?;
        }

        // Make inference call through gateway
        let request = build_chat_request(inputs);
        let stream = self.gateway.chat_completion_stream(request).await?;
        collect_response(stream).await
    }
}
```

### Part 4: `ProcessSpawner` injection

The `InferenceGateway` needs a `ProcessSpawner` to start llama.cpp. Each host provides
the appropriate one:

| Host | ProcessSpawner | Source |
|------|----------------|--------|
| Tauri | `TauriProcessSpawner` | `tauri-plugin-shell` |
| NIF/Elixir | `StdProcessSpawner` | `std::process::Command` (already in `crates/inference/`) |
| CLI | `StdProcessSpawner` | Same |

The gateway is initialized once with the spawner, then shared with `CoreTaskExecutor`:

```rust
// In NIF (pantograph-rustler):
let spawner = Arc::new(StdProcessSpawner::new(binaries_dir, data_dir));
let gateway = Arc::new(InferenceGateway::new());
gateway.set_spawner(spawner).await;
let core_executor = CoreTaskExecutor::new(gateway);

// In Tauri:
let spawner = Arc::new(TauriProcessSpawner::new(app_handle));
let gateway = Arc::new(InferenceGateway::new());
gateway.set_spawner(spawner).await;
let core_executor = CoreTaskExecutor::new(gateway);
```

### Part 5: Update Tauri app to use `CoreTaskExecutor`

Replace the 1200-line `PantographTaskExecutor` with a thin host-specific executor that
only handles Tauri-specific nodes, composed with `CoreTaskExecutor`:

```rust
// src-tauri/src/workflow/task_executor.rs (simplified)

pub struct TauriTaskExecutor {
    rag_manager: Arc<RwLock<RagManager>>,
    app_handle: AppHandle,
}

#[async_trait]
impl TaskExecutor for TauriTaskExecutor {
    async fn execute_task(&self, task_id, inputs, context, extensions) -> Result<...> {
        let node_type = resolve_node_type(task_id, &inputs);
        match node_type.as_str() {
            "rag-search"        => self.execute_rag_search(&inputs).await,
            "human-input"       => self.execute_human_input(&inputs).await,
            "component-preview" => self.execute_component_preview(&inputs).await,
            "linked-input"      => self.execute_linked_input(&inputs).await,
            _ => Err(NodeEngineError::ExecutionFailed(
                format!("{} requires host-specific executor", node_type)
            )),
        }
    }
}

// Usage:
let composite = CompositeTaskExecutor::new(
    Some(Arc::new(tauri_executor)),
    Arc::new(core_executor),
);
```

This reduces the Tauri executor from 1200 lines to ~100 lines.

### Part 6: Update NIF to use `CoreTaskExecutor`

The NIF's `ElixirCallbackTaskExecutor` becomes a fallback for nodes that need Elixir-side
handling (if any). For most nodes, the core executor handles everything in Rust:

```rust
// crates/pantograph-rustler/src/lib.rs

fn executor_new(graph_json, caller_pid) -> WorkflowExecutorResource {
    let gateway = Arc::new(InferenceGateway::new());
    // StdProcessSpawner uses the llama-server binary from a configured path
    let spawner = Arc::new(StdProcessSpawner::new(binaries_dir(), data_dir()));
    gateway.set_spawner(spawner);

    let core = Arc::new(CoreTaskExecutor::new(gateway));
    let elixir_fallback = Arc::new(ElixirCallbackTaskExecutor::new(caller_pid));
    let composite = CompositeTaskExecutor::new(Some(elixir_fallback), core);

    // ...
}
```

This means the NIF can execute `text-input`, `text-output`, `llamacpp-inference`,
`puma-lib`, and all other core nodes natively in Rust — without round-tripping through
the BEAM for every node.

### Part 7: Async Demand API

The current NIF `executor_demand` blocks a BEAM DirtyCpu scheduler thread until the entire
graph execution completes:

```rust
// Current (blocking) — crates/pantograph-rustler/src/lib.rs
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_demand(resource: ResourceArc<WorkflowExecutorResource>, node_id: String)
    -> NifResult<String>
{
    let rt = &resource.runtime;
    rt.block_on(async {
        let exec = executor.read().await;
        let result = exec.demand(&node_id, task_exec.as_ref()).await
            .map_err(|e| rustler::Error::Term(Box::new(format!("Demand error: {}", e))))?;
        serde_json::to_string(&result).map_err(...)
    })
}
```

This is problematic because:

- It ties up a DirtyCpu scheduler for the entire duration of graph execution (which may
  include LLM inference taking seconds or minutes)
- The BEAM has a limited pool of dirty schedulers (typically equal to CPU cores)
- Multiple concurrent workflows can exhaust the dirty scheduler pool, starving the system
- There is no way to receive streaming tokens during execution — the caller only gets the
  final result after the entire graph completes

#### Proposed: `executor_demand_async`

Replace the blocking demand with an async version that returns immediately and delivers
results via BEAM messages:

```rust
// New (async) — crates/pantograph-rustler/src/lib.rs

#[rustler::nif]
fn executor_demand_async(
    env: Env,
    resource: ResourceArc<WorkflowExecutorResource>,
    node_id: String,
) -> Atom {
    let caller_pid = env.pid();
    let executor = resource.executor.clone();
    let task_exec = resource.task_executor.clone();
    let msg_env = OwnedEnv::new();

    // Spawn on the tokio runtime — returns immediately
    resource.runtime.spawn(async move {
        let exec = executor.read().await;
        let result = exec.demand(&node_id, task_exec.as_ref()).await;

        // Send result back to the calling Elixir process
        msg_env.send_and_clear(&caller_pid, |env| {
            match result {
                Ok(outputs) => {
                    let json = serde_json::to_string(&outputs).unwrap_or_default();
                    (atoms::demand_complete(), node_id, json).encode(env)
                }
                Err(e) => {
                    (atoms::demand_error(), node_id, format!("{}", e)).encode(env)
                }
            }
        });
    });

    atoms::ok()
}
```

The Elixir caller receives messages asynchronously:

```elixir
# Bewilder.Workflow.Session (GenServer)

def demand_async(session, node_id) do
  GenServer.cast(session, {:demand_async, node_id})
end

def handle_cast({:demand_async, node_id}, state) do
  Native.executor_demand_async(state.executor, node_id)
  {:noreply, state}
end

# Results arrive as messages to the GenServer
def handle_info({:demand_complete, node_id, outputs_json}, state) do
  outputs = Jason.decode!(outputs_json)
  Phoenix.PubSub.broadcast(Bewilder.PubSub, state.topic,
    {:workflow_output, node_id, outputs})
  {:noreply, state}
end

def handle_info({:demand_error, node_id, error}, state) do
  Phoenix.PubSub.broadcast(Bewilder.PubSub, state.topic,
    {:workflow_error, node_id, error})
  {:noreply, state}
end
```

#### Streaming Events

For inference nodes that produce tokens incrementally, the executor should also send
streaming events during execution — not just the final result:

```rust
// During llamacpp-inference execution in CoreTaskExecutor:
// Each token chunk is sent to the caller as it arrives

while let Some(chunk) = stream.next().await {
    if let Ok(ChatChunk { content: Some(text), .. }) = chunk {
        msg_env.send_and_clear(&caller_pid, |env| {
            (atoms::node_stream(), node_id.clone(), text).encode(env)
        });
    }
}
```

The Elixir side handles streaming tokens:

```elixir
def handle_info({:node_stream, node_id, chunk}, state) do
  Phoenix.PubSub.broadcast(Bewilder.PubSub, state.topic,
    {:workflow_stream, node_id, chunk})
  {:noreply, state}
end
```

This gives NIF consumers real-time token streaming without polling, without blocking
scheduler threads, and without requiring an external inference server.

#### Deprecation

The existing `executor_demand` (blocking) should be deprecated and eventually removed.
All new code should use `executor_demand_async`.

## Migration Path

1. **Phase 1**: Create `CoreTaskExecutor` with pure nodes only (text-input, text-output,
   conditional, merge, json-filter, validator, model-provider). No breaking changes —
   hosts still handle inference nodes.

2. **Phase 2**: Move inference nodes to core (llamacpp-inference, ollama-inference,
   llm-inference, unload-model). Wire in `InferenceGateway`. Update Tauri to use
   `CompositeTaskExecutor`.

3. **Phase 3**: Update NIF to use `CoreTaskExecutor`. NIF consumers can then build
   workflows and execute them without any callback handlers for standard nodes.

4. **Phase 4**: Add `executor_demand_async` NIF alongside the existing blocking
   `executor_demand`. Add streaming event forwarding for inference nodes. NIF consumers
   can opt in to the async API immediately.

5. **Phase 5**: Remove duplicate code from Tauri's `PantographTaskExecutor`. Remove
   callback handlers from NIF consumers for nodes now handled by core. Deprecate the
   blocking `executor_demand`.

## Benefits

- **Single source of truth**: Node execution logic lives in one place (core crate)
- **New nodes work everywhere**: Add a node to core, it works in Tauri, NIF, and CLI
- **Headless inference**: NIF/CLI can run llamacpp-inference without Tauri or external servers
- **Smaller host code**: Tauri executor shrinks from 1200 lines to ~100
- **No more stubs**: Nodes can actually execute via `run()` with a proper executor
- **ProcessSpawner abstraction already exists**: `StdProcessSpawner` is implemented and tested
- **Non-blocking execution**: Async demand API frees BEAM scheduler threads during graph execution
- **Real-time streaming**: Token-by-token delivery to NIF consumers without polling

## Files Affected

### New

- `crates/node-engine/src/core_executor.rs` — CoreTaskExecutor
- `crates/node-engine/src/composite_executor.rs` — CompositeTaskExecutor

### Modified

- `crates/node-engine/src/lib.rs` — export new modules
- `crates/pantograph-rustler/src/lib.rs` — use CoreTaskExecutor + StdProcessSpawner + async demand
- `src-tauri/src/workflow/task_executor.rs` — slim down to host-only nodes
- `src-tauri/src/workflow/commands.rs` — use CompositeTaskExecutor
- `crates/inference/Cargo.toml` — enable `std-process` feature for NIF builds

### Deprecated

- `executor_demand` (blocking NIF) — replaced by `executor_demand_async`
