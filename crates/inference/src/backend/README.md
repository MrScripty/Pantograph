# crates/inference/src/backend

## Purpose

This directory defines the backend contract and the concrete engine adapters
that implement it. The boundary exists so inference callers can depend on one
trait while backend-specific launch, health, and request translation stays
isolated here.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | The backend trait, capability model, shared config, and backend error contract. |
| `registry.rs` | Compile-time backend registration and backend discovery helpers. |
| `llamacpp.rs` | llama.cpp backend adapter for OpenAI-compatible chat and embeddings. |
| `ollama.rs` | Ollama backend adapter. |
| `candle.rs` | Candle backend placeholder and capability declaration. |
| `pytorch.rs` | PyTorch backend implementation used for HuggingFace-style runtimes. |

## Problem

Different inference engines expose incompatible lifecycle and request models.
Pantograph still needs a single backend-facing abstraction for capability
checks, startup, health, and request execution.

## Constraints

- Backends must remain swappable at runtime.
- Capability flags must stay honest because upstream callers gate behavior on
  them.
- Backend configuration must remain generic enough to cover multiple engines.
- New backend features must extend the trait additively where possible.

## Decision

Keep one `InferenceBackend` trait with explicit lifecycle hooks and typed
capability declarations. Backend-specific translation lives in per-engine files,
while `registry.rs` handles discovery and instantiation.

## Alternatives Rejected

- Per-backend public APIs: rejected because callers would need branching logic
  and would lose runtime backend switching.
- Hiding capability differences behind panics or implicit fallback behavior:
  rejected because unsupported features must fail predictably.

## Invariants

- Unsupported backend features must return explicit errors.
- `BackendCapabilities` fields describe actual runtime support.
- Registry entries and backend implementations must stay in sync.

## Revisit Triggers

- A backend needs a capability that does not fit the current trait shape.
- Process-backed PyTorch becomes the canonical runtime and needs a different
  config/lifecycle model than the current adapter.
- Image generation requires streaming/progress as a first-class backend API.

## Dependencies

**Internal:** `crate::gateway`, `crate::process`, and shared contracts from
`crate::types`.
**External:** runtime-specific crates such as `reqwest`, Candle, or PyO3.

## Related ADRs

- None identified as of 2026-03-07.
- Reason: Backend trait evolution has been handled in code without formal ADRs.
- Revisit trigger: Contract expansion for diffusion or process-backed PyTorch
  changes compatibility expectations.

## Usage Examples

```rust
use inference::backend::{BackendConfig, BackendRegistry};

fn create_backend() {
    let registry = BackendRegistry::new();
    let _backend = registry.create("llama.cpp").unwrap();
    let _config = BackendConfig::default();
}
```

## API Consumer Contract

- Consumers should reach these backends through `InferenceGateway`.
- Backend `start()` owns runtime initialization; `stop()` must release runtime
  resources.
- Capability checks are stable inputs for higher-level orchestration.
- Backends may reject requests for unsupported features even if other methods
  are available.

## Structured Producer Contract

- `BackendCapabilities` is a machine-consumed contract used for runtime gating.
- `BackendConfig` fields have additive semantics; absent values mean backend
  defaults or backend-specific auto-detection.
- `InferenceBackend` method additions must preserve compatibility expectations
  for existing callers or be versioned through coordinated contract changes.
