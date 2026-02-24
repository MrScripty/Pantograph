# Follow-up Proposal: Wire InferenceGateway into NIF + Add Streaming

## Context

The native node execution proposal (Parts 1–7) has been implemented. The Tauri app
correctly wires `InferenceGateway` + `ProcessSpawner` into `CoreTaskExecutor`:

```rust
// src-tauri/src/workflow/commands.rs — working
let core = Arc::new(
    node_engine::CoreTaskExecutor::new()
        .with_project_root(project_root)
        .with_gateway(gateway.inner_arc()),
);
```

The NIF does **not** do this:

```rust
// crates/pantograph-rustler/src/lib.rs — current
let core = node_engine::CoreTaskExecutor::new();
// No .with_gateway() — inference nodes fail with
// "InferenceGateway not configured" and fall through to Elixir callbacks
```

This means NIF consumers cannot use `llamacpp-inference`, `llm-inference`,
`vision-analysis`, or `unload-model` natively. These nodes fall through to
`ElixirCallbackTaskExecutor`, forcing the host to reimplement inference — exactly what
the original proposal aimed to eliminate.

## Problem 1: No InferenceGateway in NIF

The `executor_new` NIF creates `CoreTaskExecutor::new()` without calling
`.with_gateway()`. The `inference-nodes` feature flag is enabled in the NIF's
`Cargo.toml`, the inference handlers are compiled in, and the `require_gateway()`
check runs — but `gateway` is `None` so every inference node fails with:

```
InferenceGateway not configured. Use CoreTaskExecutor::with_gateway().
```

This error contains `"requires host-specific executor"` ... it doesn't, actually. It
matches the sentinel string in `CoreFirstExecutor` and falls through to Elixir, but
the real issue is that the gateway was never configured.

### Proposed Fix: Wire gateway + spawner in `executor_new`

The NIF should create an `InferenceGateway` with `StdProcessSpawner` and pass it to
`CoreTaskExecutor`, same as Tauri does.

```rust
// crates/pantograph-rustler/src/lib.rs

use inference::{InferenceGateway, StdProcessSpawner};

#[rustler::nif(schedule = "DirtyCpu")]
fn executor_new(
    env: Env,
    graph_json: String,
    caller_pid: rustler::LocalPid,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    let _ = env;
    let graph: WorkflowGraph = serde_json::from_str(&graph_json)
        .map_err(|e| rustler::Error::Term(Box::new(format!("Parse error: {}", e))))?;

    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| rustler::Error::Term(Box::new(format!("Runtime error: {}", e))))?;

    // Create inference gateway with StdProcessSpawner
    let gateway = Arc::new(InferenceGateway::new());
    let binaries_dir = resolve_binaries_dir();
    let data_dir = resolve_data_dir();
    let spawner = Arc::new(StdProcessSpawner::new(binaries_dir, data_dir));
    runtime.block_on(async { gateway.set_spawner(spawner).await });

    let core = node_engine::CoreTaskExecutor::new()
        .with_gateway(gateway);
    let elixir = ElixirCallbackTaskExecutor::new(caller_pid);
    let task_executor: Arc<dyn TaskExecutor> =
        Arc::new(CoreFirstExecutor::new(core, elixir));

    let event_sink: Arc<dyn EventSink> = Arc::new(BeamEventSink::new(caller_pid));
    let executor = WorkflowExecutor::new("nif-execution", graph, event_sink);

    Ok(ResourceArc::new(WorkflowExecutorResource {
        executor: Arc::new(tokio::sync::RwLock::new(executor)),
        task_executor,
        runtime: Arc::new(runtime),
    }))
}
```

### Directory Resolution

The `StdProcessSpawner` needs two paths:

1. **`binaries_dir`** — directory containing the `llama-server` binary
2. **`data_dir`** — directory for PID files and runtime data

These should be configurable by the NIF consumer. Two options:

**Option A: NIF parameters** (preferred — gives host full control):

```rust
#[rustler::nif(schedule = "DirtyCpu")]
fn executor_new_with_inference(
    env: Env,
    graph_json: String,
    caller_pid: rustler::LocalPid,
    binaries_dir: String,
    data_dir: String,
) -> NifResult<ResourceArc<WorkflowExecutorResource>> {
    // ... same as above but with caller-provided paths
}
```

**Option B: Environment variables** (simpler but less flexible):

```rust
fn resolve_binaries_dir() -> PathBuf {
    std::env::var("PANTOGRAPH_BINARIES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/usr/local/bin"))
}

fn resolve_data_dir() -> PathBuf {
    std::env::var("PANTOGRAPH_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("pantograph")
        })
}
```

We recommend **Option A** — the NIF consumer (Bewilder, etc.) knows where its binaries
live and should pass those paths explicitly. This avoids hidden environment variable
coupling and makes testing straightforward.

### Cargo.toml Change

The NIF's `Cargo.toml` needs a direct dependency on the `inference` crate to access
`InferenceGateway` and `StdProcessSpawner`:

```toml
# crates/pantograph-rustler/Cargo.toml
[dependencies]
inference = { path = "../inference", features = ["std-process", "backend-llamacpp"] }
```

Currently `node-engine` depends on `inference` behind the `inference-nodes` feature,
but the NIF doesn't import it directly — it can't call `InferenceGateway::new()` or
`StdProcessSpawner::new()` without this.

## Problem 2: No Streaming Token Events

The `executor_demand_async` NIF sends a single `{:demand_complete, node_id, outputs_json}`
message when the entire graph finishes. For inference nodes that produce tokens
incrementally, there is no way for NIF consumers to stream tokens to users in real time.

The `execute_llamacpp_inference` handler in `CoreTaskExecutor` currently uses
`"stream": false` and collects the full response before returning:

```rust
// core_executor.rs line 806
let request_body = serde_json::json!({
    "prompt": full_prompt,
    "n_predict": max_tokens,
    "temperature": temperature,
    "stop": ["</s>", "<|im_end|>", "<|end|>"],
    "stream": false  // <-- collects entire response
});
```

### Proposed: Streaming Callback Channel

Add a streaming callback mechanism to `CoreTaskExecutor` that forwards tokens as they
arrive. The NIF can then relay them to the BEAM.

**Step 1: Add a `StreamSink` trait**

```rust
// crates/node-engine/src/stream_sink.rs

#[async_trait]
pub trait StreamSink: Send + Sync {
    /// Called for each token chunk during streaming inference.
    async fn on_token(&self, node_id: &str, token: &str);

    /// Called when streaming is complete for a node.
    async fn on_stream_done(&self, node_id: &str);
}
```

**Step 2: Inject into CoreTaskExecutor**

```rust
pub struct CoreTaskExecutor {
    project_root: Option<PathBuf>,
    #[cfg(feature = "inference-nodes")]
    gateway: Option<Arc<InferenceGateway>>,
    stream_sink: Option<Arc<dyn StreamSink>>,
}

impl CoreTaskExecutor {
    pub fn with_stream_sink(mut self, sink: Arc<dyn StreamSink>) -> Self {
        self.stream_sink = Some(sink);
        self
    }
}
```

**Step 3: Use streaming in inference handlers**

```rust
// In execute_llamacpp_inference:
let request_body = serde_json::json!({
    "prompt": full_prompt,
    "n_predict": max_tokens,
    "temperature": temperature,
    "stop": ["</s>", "<|im_end|>", "<|end|>"],
    "stream": self.stream_sink.is_some()  // stream when sink available
});

if let Some(ref sink) = self.stream_sink {
    // SSE streaming — forward each token
    let mut full_response = String::new();
    let stream = client.post(&url).json(&request_body).send().await?;
    let mut reader = stream.bytes_stream();

    while let Some(chunk) = reader.next().await {
        let text = parse_sse_token(&chunk?);
        if let Some(token) = text {
            full_response.push_str(&token);
            sink.on_token(task_id, &token).await;
        }
    }
    sink.on_stream_done(task_id).await;

    // Return full response as output
    outputs.insert("response".to_string(), json!(full_response));
} else {
    // Non-streaming — existing behavior
    // ...
}
```

**Step 4: NIF implementation of StreamSink**

```rust
// crates/pantograph-rustler/src/lib.rs

struct BeamStreamSink {
    pid: rustler::LocalPid,
}

#[async_trait]
impl StreamSink for BeamStreamSink {
    async fn on_token(&self, node_id: &str, token: &str) {
        let mut env = OwnedEnv::new();
        let nid = node_id.to_string();
        let tok = token.to_string();
        let _ = env.send_and_clear(&self.pid, |env| {
            (atoms::node_stream(), nid, tok).encode(env)
        });
    }

    async fn on_stream_done(&self, node_id: &str) {
        let mut env = OwnedEnv::new();
        let nid = node_id.to_string();
        let _ = env.send_and_clear(&self.pid, |env| {
            (atoms::node_stream_done(), nid).encode(env)
        });
    }
}
```

**Step 5: Wire into executor_new**

```rust
let stream_sink = Arc::new(BeamStreamSink { pid: caller_pid });
let core = node_engine::CoreTaskExecutor::new()
    .with_gateway(gateway)
    .with_stream_sink(stream_sink);
```

### Message Format

NIF consumers receive:

| Message | When |
| ------- | ---- |
| `{:node_stream, node_id, token}` | Each token during streaming inference |
| `{:node_stream_done, node_id}` | Streaming finished for a node |
| `{:demand_complete, node_id, outputs_json}` | Full demand result (includes complete response) |
| `{:demand_error, node_id, error}` | Demand failed |

The `:node_stream` messages arrive **during** the demand, before `:demand_complete`.
The host can forward them to the UI for real-time display while still getting the full
result at the end.

## Problem 3: Parity Gap Between Tauri and NIF

The Tauri app has full inference capability:

| Capability | Tauri | NIF |
| ---------- | ----- | --- |
| Pure nodes (text-input, etc.) | CoreTaskExecutor | CoreTaskExecutor |
| Inference (llamacpp) | CoreTaskExecutor + gateway | Falls through to Elixir |
| Process spawning | TauriProcessSpawner | Not configured |
| Streaming tokens | Via Tauri events | Not available |
| Async demand | N/A (Tauri is async) | executor_demand_async |

After this proposal:

| Capability | Tauri | NIF |
| ---------- | ----- | --- |
| Pure nodes | CoreTaskExecutor | CoreTaskExecutor |
| Inference (llamacpp) | CoreTaskExecutor + gateway | CoreTaskExecutor + gateway |
| Process spawning | TauriProcessSpawner | StdProcessSpawner |
| Streaming tokens | Via Tauri events | Via BEAM messages |
| Async demand | N/A | executor_demand_async |

Full parity. NIF consumers get the same inference capabilities as Tauri.

## New Atoms

Add to the `atoms!` block in `pantograph-rustler`:

```rust
rustler::atoms! {
    // ... existing atoms ...
    node_stream,
    node_stream_done,
}
```

## Files Affected

### New

- `crates/node-engine/src/stream_sink.rs` — StreamSink trait

### Modified

- `crates/node-engine/src/core_executor.rs` — add `stream_sink` field, use streaming in inference handlers
- `crates/node-engine/src/lib.rs` — export `StreamSink`
- `crates/pantograph-rustler/src/lib.rs` — wire gateway + spawner + stream sink in `executor_new`
- `crates/pantograph-rustler/Cargo.toml` — add direct `inference` dependency
