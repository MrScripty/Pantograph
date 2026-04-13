# Inference

A Rust library for multi-backend AI inference, supporting llama.cpp, Ollama, and Candle.

## Features

- **Multi-backend support**: Unified interface for different inference engines
- **ProcessSpawner abstraction**: Pluggable runtime process management for different environments
- **Managed runtimes**: `llama.cpp` and `Ollama` install, remove, resolve, and launch through the inference crate
- **Feature-gated backends**: Include only what you need
- **OpenAI-compatible API**: All backends expose the same chat/embedding interface

## Backends

| Backend | Feature Flag | Description |
|---------|-------------|-------------|
| llama.cpp | `backend-llamacpp` (default) | Local inference via GGUF models |
| Ollama | `backend-ollama` | Integration with Ollama daemon |
| Candle | `backend-candle` | In-process CUDA inference |

## Usage

```rust
use inference::{InferenceGateway, BackendConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create gateway
    let gateway = InferenceGateway::new();

    // Set up process spawner (implement ProcessSpawner trait)
    // gateway.set_spawner(your_spawner).await;

    // Configure backend
    let config = BackendConfig {
        model_path: Some("/path/to/model.gguf".into()),
        mmproj_path: Some("/path/to/mmproj.gguf".into()),
        ..Default::default()
    };

    // Start inference
    gateway.start(&config).await?;

    // Use the gateway for chat completions or embeddings
    Ok(())
}
```

## ProcessSpawner

The library uses a `ProcessSpawner` trait to abstract process management. This allows it to work in different environments:

- **Tauri apps**: Provide an app-data-backed spawner while inference owns runtime resolution
- **CLI tools**: Use `StdProcessSpawner` (enable `std-process` feature)
- **Custom**: Implement `ProcessSpawner` for your environment

```rust
use inference::process::{ProcessSpawner, StdProcessSpawner};
use std::path::PathBuf;

// For standalone use (CLI tools, servers)
let spawner = StdProcessSpawner::new(
    PathBuf::from("/path/to/binaries"),
    PathBuf::from("/path/to/data"),
);

// Managed runtime downloads/install paths are owned by inference.
// Hosts only provide process spawning and app-data locations.
```

## Managed Runtime Lifecycle

- Hosts query managed runtime capability state from inference.
- Runtime installation is explicit and selective per backend.
- Inference owns install/remove/resolve/launch behavior for managed runtimes.
- Workflow execution should preflight runtime requirements instead of triggering
  implicit downloads at run time.
- Backend start paths also own runtime reuse facts. For example, the Ollama
  backend reports whether it attached to an existing daemon so the gateway can
  preserve lifecycle attribution without guessing in adapters.

## Feature Flags

```toml
[dependencies]
inference = { version = "0.1", features = ["backend-llamacpp", "backend-ollama"] }
```

Available features:
- `backend-llamacpp` (default): llama.cpp managed-runtime support
- `backend-ollama`: Ollama daemon integration
- `backend-candle`: In-process Candle inference (requires CUDA)
- `std-process`: Standard library process spawner

## License

MIT OR Apache-2.0
