# Inference

Multi-backend AI inference infrastructure for Pantograph.

## Purpose
This crate owns backend execution, managed runtime resolution, process
spawning contracts, backend lifecycle facts, and OpenAI-compatible inference
facades. Host crates provide process/app-data integration, but backend
capability and lifecycle behavior stay here.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Crate manifest and backend feature declarations. |
| `src/` | Backend implementations, gateway facade, process contracts, KV-cache support, and managed-runtime lifecycle code. |
| `audio/`, `depth/`, `onnx/`, `torch/` | Python/runtime helper assets used by optional backend families. |

## Problem
Pantograph supports llama.cpp, Ollama, Candle, and PyTorch-style execution
paths. Without one infrastructure crate, runtime startup, backend capabilities,
process spawning, managed downloads, and reuse diagnostics drift into adapters
and workflow business logic.

## Constraints
- Keep host transport and workflow policy out of this crate.
- Keep expensive backend families behind explicit Cargo features.
- Report unsupported capabilities explicitly instead of silently succeeding.
- Preserve backend-owned lifecycle facts for diagnostics and runtime registry
  consumers.

## Decision
Keep inference as the infrastructure owner for backend execution and runtime
process control. Consumers use `InferenceGateway` plus feature-gated backend
families rather than calling backend modules directly.

## Alternatives Rejected
- Put backend lifecycle logic in Tauri commands: rejected because runtime
  behavior must be reusable by non-Tauri hosts.
- Put scheduler or technical-fit policy in inference: rejected because this
  crate owns execution infrastructure, not workflow admission policy.
- Always compile every backend: rejected because PyTorch, Candle, and audio
  paths have heavyweight runtime costs.

## Invariants
- Backends expose explicit capabilities and unsupported behavior.
- Managed runtime install/remove/resolve operations remain backend-owned.
- Process spawning is injected through `ProcessSpawner`.
- Feature flags are public contracts and must stay documented.
- Runtime reuse, attach, and start facts are emitted by backend-owned code.

## Revisit Triggers
- A backend requires host-specific policy that cannot fit behind injected
  process/app-data contracts.
- A backend feature becomes part of the default desktop product surface or is
  removed from supported builds.
- Managed runtime state becomes a generated or externally versioned schema.

## Dependencies
**Internal:** `pantograph-runtime-identity`.

**External:** `tokio`, `serde`, `reqwest`, `async-trait`, compression/archive
crates, optional Candle crates, optional PyO3, and process/runtime utilities.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `docs/adr/ADR-003-rust-workspace-policy.md`

## Usage Examples
```rust
use inference::{BackendConfig, InferenceGateway};

async fn start_gateway() -> Result<(), Box<dyn std::error::Error>> {
    let gateway = InferenceGateway::new();
    let config = BackendConfig {
        model_path: Some("/path/to/model.gguf".into()),
        mmproj_path: Some("/path/to/mmproj.gguf".into()),
        ..Default::default()
    };

    gateway.start(&config).await?;
    Ok(())
}
```

## Feature Flags
| Feature | Default | Contract |
| ------- | ------- | -------- |
| `backend-llamacpp` | Yes | llama.cpp sidecar and GGUF support. |
| `backend-ollama` | No | Ollama daemon integration. |
| `backend-candle` | No | In-process Candle inference; pulls CUDA-oriented dependencies. |
| `backend-pytorch` | No | In-process PyTorch/PyO3 backend support. |
| `std-process` | No | Standard-library process spawner for non-Tauri hosts. |

## API Consumer Contract
- Inputs: backend configuration, process spawner implementations, managed
  runtime IDs, and inference requests.
- Outputs: chat, embedding, rerank, KV-cache, runtime lifecycle, and managed
  runtime DTOs.
- Lifecycle: callers configure a gateway, inject host process behavior, start
  or attach backends, and stop them through the gateway.
- Errors: backend and lifecycle failures are surfaced as typed or structured
  errors; unsupported capabilities must not return successful placeholder data.
- Versioning: Cargo features, backend capability fields, and runtime lifecycle
  payloads are public contracts for workspace consumers.

## Structured Producer Contract
- Managed runtime state and runtime lifecycle payloads are structured producer
  outputs consumed by adapters and diagnostics.
- Reason: these payloads describe install state, runtime readiness, reuse, and
  backend attachment facts.
- Revisit trigger: payloads become externally versioned schemas or are consumed
  outside the Pantograph workspace.

## Testing
Run focused inference checks from the workspace root:

```bash
cargo test -p inference
cargo check -p inference --all-features
cargo check -p inference --no-default-features
```

## Notes
- Keep workflow scheduling, technical-fit policy, and adapter transport logic
  outside this crate.
