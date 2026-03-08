# crates/inference/src

## Purpose

This directory contains the core inference facade used by Pantograph to talk to
multiple runtime engines through one Rust API. The boundary exists so callers
can depend on stable contracts for chat, embeddings, and image generation
without depending on backend-specific launch logic or model-family details.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `backend/` | Backend trait definitions and concrete engine adapters such as llama.cpp, Ollama, Candle, and PyTorch. |
| `gateway.rs` | The single entry point that owns the active backend and forwards requests through the frozen contracts. |
| `process.rs` | Sidecar process abstraction used by backends that need external runtimes. |
| `types.rs` | Shared request/response contracts consumed across backend and host boundaries. |
| `server.rs` | Legacy sidecar/server lifecycle helpers for llama.cpp-style backends. |
| `kv_cache/` | KV-cache contracts and helpers used by inference-capable hosts. |

## Problem

Pantograph needs one inference-facing crate that can swap execution engines
without forcing the rest of the backend to know whether a request is served by a
local sidecar, a daemon, or an in-process runtime.

## Constraints

- The public contract must stay stable enough for multiple hosts to consume.
- Backends have different lifecycle models, so process ownership must be
  abstracted.
- Machine-consumed request/response payloads must preserve semantics across
  process and language boundaries.
- New capability areas such as diffusion must extend the contract additively.

## Decision

Use a gateway + backend trait architecture with shared request/response types.
Backends implement a common interface, while the gateway owns lifecycle and
routing. Shared payload types live in `types.rs` so chat, embedding, and image
generation contracts stay explicit and testable.

## Alternatives Rejected

- Exposing backend-specific request types directly: rejected because it would
  leak infrastructure details into callers and make runtime switching brittle.
- Keeping image generation outside this crate: rejected because diffusion is a
  backend capability and needs the same contract discipline as chat and
  embeddings.

## Invariants

- `InferenceGateway` is the only facade new callers should use for inference.
- Backend capability flags must reflect contract support, not aspirational
  future support.
- Shared request/response types are append-only unless a coordinated breaking
  change is approved.

## Revisit Triggers

- A second non-diffusion image-generation family requires materially different
  request semantics.
- Process spawning must support arbitrary commands or per-env interpreter
  selection inside this crate.
- A backend needs streaming image-generation events as a first-class contract.

## Dependencies

**Internal:** `backend`, `gateway`, `process`, `types`, `server`, `kv_cache`.
**External:** `tokio`, `serde`, `reqwest`, `async-trait`, and feature-gated
runtime crates such as Candle or PyO3-backed components.

## Related ADRs

- None identified as of 2026-03-07.
- Reason: The gateway/backend split already exists but has not yet been captured
  in an ADR.
- Revisit trigger: The PyO3-to-process-runtime migration changes the ownership
  model for PyTorch backends.

## Usage Examples

```rust
use inference::{BackendConfig, ImageGenerationRequest, InferenceGateway};

async fn run_image_request(gateway: &InferenceGateway, config: &BackendConfig) {
    gateway.start(config).await.unwrap();
    let _ = gateway
        .generate_image(ImageGenerationRequest {
            model: "model-id".to_string(),
            prompt: "paper lantern in the rain".to_string(),
            negative_prompt: None,
            width: Some(1024),
            height: Some(1024),
            num_inference_steps: Some(30),
            guidance_scale: Some(4.0),
            seed: Some(42),
            scheduler: None,
            num_images_per_prompt: Some(1),
            init_image: None,
            mask_image: None,
            strength: None,
            extra_options: serde_json::Value::Null,
        })
        .await;
}
```

## API Consumer Contract

- Callers talk to `InferenceGateway`, not backend implementations directly.
- Backend startup must happen before inference calls.
- `generate_image()` is synchronous-at-contract-level and returns final images;
  streaming progress is not yet part of the facade.
- Process-backed diffusion loaders may infer narrow bundle-root load overrides
  such as consistent safetensors variants when the diffusers directory layout
  makes them deterministic.
- Unsupported capabilities return backend errors rather than silent no-ops.
- Additive fields may be introduced to request/response structs; existing field
  semantics must remain stable.

## Structured Producer Contract

- `types.rs` defines the stable machine-consumed request and response shapes.
- Optional fields preserve meaning when omitted; callers may rely on omission as
  “backend default”.
- `ImageGenerationRequest` reserves optional `init_image`, `mask_image`, and
  `strength` for later img2img/inpaint support.
- Contract changes that affect persisted consumers or saved workflows must be
  append-only or accompanied by migration guidance.
