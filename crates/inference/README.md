# Inference

Multi-backend AI inference infrastructure for Pantograph.

## Purpose
This crate owns backend execution, managed runtime resolution, process
spawning contracts, backend lifecycle facts, and OpenAI-compatible inference
facades. Host crates provide process/app-data integration, while shared managed
dependency DTOs live in `pantograph-managed-dependencies`.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Crate manifest and backend feature declarations. |
| `src/` | Backend implementations, gateway facade, model/package contract DTOs, process contracts, KV-cache support, managed-runtime lifecycle code, and temporary adapters into the neutral managed-dependency contracts. |
| `audio/`, `depth/`, `onnx/`, `torch/` | Python/runtime helper assets used by optional backend families. |

## Problem
Pantograph supports llama.cpp, Candle, and PyTorch-style execution
paths. Without one infrastructure crate, runtime startup, backend capabilities,
process spawning, managed downloads, and reuse diagnostics drift into adapters
and workflow business logic.

## Constraints
- Keep host transport and workflow policy out of this crate.
- Keep expensive backend families behind explicit Cargo features.
- Report unsupported capabilities explicitly instead of silently succeeding.
- Preserve backend-owned lifecycle facts for diagnostics and runtime registry
  consumers.
- Consume Pumas model/package facts through versioned DTOs or fixtures, not
  Pumas storage internals.
- Keep Pumas package facts separate from Pantograph-owned technical-fit
  candidate derivation; runtime registry and scheduler layers own final
  backend selection and admission policy.

## Decision
Keep inference as the infrastructure owner for backend execution and runtime
process control. Consumers use `InferenceGateway` plus feature-gated backend
families rather than calling backend modules directly. Transformers-aligned
model package, task, generation, lifecycle, and cache-invalidation contracts
live in `model_contracts.rs` so later backend slices can consume Pumas facts
without depending on Pumas SQLite or turning inference into a model library.

## Alternatives Rejected
- Put backend lifecycle logic in Tauri commands: rejected because runtime
  behavior must be reusable by non-Tauri hosts.
- Put scheduler or technical-fit policy in inference: rejected because this
  crate owns execution infrastructure, not workflow admission policy.
- Always compile every backend: rejected because PyTorch, Candle, and audio
  paths have heavyweight runtime costs.

## Invariants
- Backends expose explicit capabilities and unsupported behavior.
- Managed runtime install/remove/resolve operations remain backend-owned until
  the neutral managed-dependency owner takes over implementation state.
- Managed binary status for runtime sidecars, media tools, and native
  redistributable artifacts is projected through
  `pantograph-managed-dependencies` DTOs rather than frontend or
  Tauri-synthesized state.
- Process spawning is injected through `ProcessSpawner`.
- Feature flags are public contracts and must stay documented.
- Runtime reuse, attach, and start facts are emitted by backend-owned code.

## Revisit Triggers
- A backend requires host-specific policy that cannot fit behind injected
  process/app-data contracts.
- A backend feature becomes part of the default desktop product surface or is
  removed from supported builds.
- Managed runtime state becomes a generated or externally versioned schema.
- Model-library facts require live runtime placement, admission, queueing, or
  scheduler policy inside this crate.

## Dependencies
**Internal:** `pantograph-managed-dependencies` and
`pantograph-runtime-identity`.

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
| `backend-candle` | No | Staged in-process Candle embedding backend; pulls CUDA-oriented dependencies but reports unavailable until executable model loading exists. |
| `backend-pytorch` | No | In-process PyTorch/PyO3 backend support. |
| `std-process` | No | Standard-library process spawner for non-Tauri hosts. |

Ollama is retired as a first-party Pantograph backend. Legacy managed-runtime
state may still be ignored or migrated by later cleanup slices, but new backend
selection must use supported runtimes through Pumas model references.

## API Consumer Contract
- Inputs: backend configuration, process spawner implementations, managed
  runtime IDs, Pumas-resolved model/package facts, typed execution requests,
  and legacy facade requests that have not yet migrated.
- Outputs: typed execution results, chat, embedding, rerank, audio
  transcription, KV-cache, runtime lifecycle, backend compatibility summaries,
  managed runtime DTOs, neutral managed-dependency DTO projections, additive
  managed-binary facade DTOs, and model/package fact DTOs.
- Lifecycle: callers configure a gateway, inject host process behavior, start
  or attach backends, and stop them through the gateway.
- Errors: backend and lifecycle failures are surfaced as typed or structured
  errors; unsupported capabilities must not return successful placeholder data.
- Versioning: Cargo features, backend capability fields, typed execution
  request/result payloads, model/package fact DTOs, and runtime lifecycle
  payloads are public contracts for workspace consumers.
- Policy: runtime placement, scheduler admission, reservation, priority,
  eviction, and final backend choice are caller-owned policy. This crate emits
  factual capabilities, compatibility diagnostics, lifecycle events, and
  backend execution results.

## Structured Producer Contract
- Managed runtime state and runtime lifecycle payloads are structured producer
  outputs consumed by adapters and diagnostics.
- Managed binary facade payloads are structured producer outputs consumed by
  Settings, workflow admission, diagnostics, and process launch adapters.
- `ResolvedModelPackageFacts`, task registry entries, generation defaults,
  option compatibility diagnostics, lifecycle phases, Pumas package-facts
  summary snapshots, and model-library update feeds are structured
  producer/consumer contracts for later inference slices.
- `InferenceExecutionRequest`, `InferenceExecutionInput`, and
  `InferenceExecutionResult` are the canonical task execution wire contracts.
  They use task ids and input/result tags from the task registry and keep
  backend-local extensions under typed generation options or `extra_options`.
- Stable generation behavior belongs in typed `GenerationOptions` groups.
  Backend-local generation escape hatches are limited to
  `backend_extensions` keys scoped as `<backend-or-adapter>:<option>`, and
  adapters must report support diagnostics instead of accepting unscoped raw
  kwargs as public contract fields.
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
