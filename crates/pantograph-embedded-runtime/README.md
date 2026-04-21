# pantograph-embedded-runtime

Pantograph-owned embedded runtime composition crate.

## Purpose
This crate composes workflow service, node execution, inference gateway,
runtime registry helpers, Pumas model dependencies, Python sidecar execution,
and RAG adapters into a reusable backend runtime. The boundary exists so Tauri
and host-language bindings can consume one Rust runtime facade instead of each
assembling workflow execution infrastructure independently.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Crate manifest and feature flags for runtime backend families. |
| `src/` | Runtime composition modules and source-level README. |

## Problem
Pantograph needs direct workflow execution outside the desktop command layer.
Without a shared embedded runtime, Tauri, UniFFI, Rustler, and tests would each
own partial runtime wiring for inference, Python, Pumas, diagnostics, and
registry projection.

## Constraints
- Keep workflow contracts owned by `pantograph-workflow-service`.
- Keep graph execution owned by `node-engine`.
- Keep runtime registry policy in `pantograph-runtime-registry`.
- Keep Python execution out of process.
- Feature-gate expensive backend families.
- Own lifecycle resources explicitly as async supervision work lands.

## Decision
Use this crate as the backend runtime composition layer. It may translate
producer facts and host configuration into workflow-service/runtime-registry
contracts, but it must not become the owner of scheduler policy or generated
binding behavior.

## Alternatives Rejected
- Keep direct runtime wiring in Tauri: rejected because native embeddings and
  tests need the same runtime without desktop transport.
- Move runtime composition into UniFFI: rejected because binding wrappers
  should be thin adapters over product-native Rust APIs.
- Put inference process management in workflow service: rejected because the
  service should depend on host traits and runtime facts, not concrete
  infrastructure.

## Invariants
- The runtime facade composes existing backend-owned contracts rather than
  redefining workflow or registry policy.
- Python worker execution remains isolated from the Rust process.
- Python interpreter discovery should use explicit candidate iteration helpers
  so process-backed startup remains readable and clippy-clean.
- Runtime feature flags document which backend families are compiled.
- Shutdown and task lifecycle ownership must become explicit as M3 proceeds.

## Cargo Feature Contract
| Feature | Default | Contract |
| ------- | ------- | -------- |
| `backend-llamacpp` | Yes | Enables llama.cpp runtime support through `inference/backend-llamacpp`. |
| `backend-ollama` | Yes | Enables Ollama runtime support through `inference/backend-ollama`. |
| `backend-candle` | Yes | Enables local Candle support and its optional CUDA/tokenizer/http streaming dependencies. |
| `backend-pytorch` | No | Enables PyTorch/PyO3 runtime and node-engine PyTorch nodes. Requires Python/PyTorch runtime availability. |
| `backend-audio` | No | Enables Python-backed audio generation nodes. Requires audio Python dependencies. |
| `standalone` | No | Enables the standard process spawner path for non-Tauri embedding. |

Defaults mirror the current desktop-local runtime set. Python-backed families
remain explicit because they carry host interpreter and package requirements.

## Revisit Triggers
- Runtime lifecycle supervision requires a new dedicated crate.
- A binding surface needs runtime behavior not represented by this crate.
- Feature-contract checks show default runtime features are too heavy for
  reusable consumers.

## Dependencies
**Internal:** `inference`, `node-engine`, `workflow-nodes`,
`pantograph-workflow-service`, `pantograph-runtime-identity`, and
`pantograph-runtime-registry`.

**External:** `serde`, `serde_json`, `tokio`, `async-trait`, `thiserror`,
`log`, `futures-util`, `reqwest`, `which`, `uuid`, `chrono`, `dirs`,
`once_cell`, `pumas-library`, and optional Candle/tokenizer/http streaming
dependencies.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```rust
use pantograph_embedded_runtime::EmbeddedRuntimeConfig;
use std::path::PathBuf;

let config = EmbeddedRuntimeConfig::new(
    PathBuf::from(".launcher-state/pantograph"),
    PathBuf::from("."),
);
```

## API Consumer Contract
- Inputs: embedded runtime configuration, workflow request JSON/DTOs, Pumas API
  handles, runtime registry handles, and host runtime configuration.
- Outputs: workflow responses, diagnostics, runtime capability projections,
  graph-edit responses, and typed runtime/workflow errors.
- Lifecycle: callers create the runtime, use workflow/session methods, and must
  close or drop it according to the runtime lifecycle contract as supervision
  hardening lands.
- Errors: runtime setup, dependency resolution, host execution, and workflow
  failures are propagated through service/runtime error types.
- Versioning: public runtime methods should be additive; changing feature
  defaults or response shapes requires binding and Tauri migration.

## Structured Producer Contract
- Stable fields: runtime capability, diagnostics, dependency, and workflow
  response shapes are consumed by adapters and bindings.
- Defaults: default features currently include selected local backend families;
  future hardening must document any default changes.
- Enums and labels: runtime ids, backend keys, lifecycle labels, and failure
  reasons are semantic contracts.
- Ordering: diagnostics and registry projections preserve backend ordering
  where callers display or compare sequences.
- Compatibility: generated bindings may package this runtime surface, so
  response changes must be coordinated.
- Regeneration/migration: runtime API changes must update UniFFI/Rustler
  wrappers, host docs, and smoke tests in the same slice.

## Testing
```bash
cargo test -p pantograph-embedded-runtime
```

## Notes
- `src/lib.rs`, `task_executor.rs`, and related runtime files remain over the
  decomposition threshold and are tracked in the standards compliance plan.
