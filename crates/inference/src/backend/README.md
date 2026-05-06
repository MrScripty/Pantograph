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
| `llamacpp.rs` | llama.cpp backend adapter for chat, embeddings, and sidecar reranking. |
| `llamacpp_support.rs` | Shared llama.cpp request parsing, SSE usage parsing, rerank response normalization, sidecar start helpers, and KV-cache fingerprint helpers used by `llamacpp.rs`. |
| `candle.rs` | Feature-gated Candle backend staging for embedding-only HF-compatible safetensors package facts, including capability declaration, package-source mapping, and local load-plan validation while executable model loading remains disabled. |
| `pytorch.rs` | PyTorch backend implementation used for HuggingFace-style runtimes. |
| `pytorch_worker.rs` | Embedded PyTorch worker loader, sibling-module registration, and Python result extraction helpers used by `pytorch.rs`. |
| `pytorch_worker_contract.rs` | Backend-local Rust/Python worker envelope, Transformers load request, trust policy, and response/error DTOs used to migrate PyTorch behind the canonical inference contracts. |
| `pytorch_tests.rs` | PyTorch backend capability, lifecycle, KV-cache fingerprint, prompt extraction, and system prompt tests extracted from the production adapter. |

## Problem

Different inference engines expose incompatible lifecycle and request models.
Pantograph still needs a single backend-facing abstraction for capability
checks, startup, health, and request execution. GGUF reranking adds a third
text-adjacent workload that must not be collapsed into the chat contract.

## Constraints

- Backends must remain swappable at runtime.
- Capability flags must stay honest because upstream callers gate behavior on
  them.
- Structured task/modality facts refine legacy capability flags; consumers
  should prefer the structured facts when deciding whether a backend supports a
  canonical task.
- Backend configuration must remain generic enough to cover multiple engines.
- New backend features must extend the trait additively where possible.
- Sidecar-backed backends must expose mode-specific readiness when one process
  mode cannot safely serve every capability at once.

## Decision

Keep one `InferenceBackend` trait with explicit lifecycle hooks and typed
capability declarations. Backend-specific translation lives in per-engine files,
while `registry.rs` handles discovery and instantiation. Reranking is exposed as
its own typed method and capability bit so callers can request it directly and
the llama.cpp adapter can switch into a dedicated reranking mode when needed.

## Alternatives Rejected

- Per-backend public APIs: rejected because callers would need branching logic
  and would lose runtime backend switching.
- Hiding capability differences behind panics or implicit fallback behavior:
  rejected because unsupported features must fail predictably.

## Invariants

- Unsupported backend features must return explicit errors.
- `BackendCapabilities` fields describe actual runtime support.
- `BackendCapabilityFacts` names canonical task ids and modality signatures; it
  must not encode scheduler admission, runtime placement, or queue policy.
- Registry entries and backend implementations must stay in sync.
- If a backend needs a distinct process mode for reranking, that requirement
  must surface through config and readiness checks instead of hidden fallback.
- Feature-gated Candle support must remain unavailable for runtime selection
  until executable model loading exists. Its current load-plan helpers are
  factual validation of Pumas package files, dtype, model type, and device hints,
  not runtime admission, residency, or scheduling policy.

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
- `rerank()` must either return ordered scores or fail explicitly; it must not
  degrade into prompt completion semantics.

## Structured Producer Contract

- `BackendCapabilities` is a machine-consumed contract used for runtime gating.
- `BackendConfig` fields have additive semantics; absent values mean backend
  defaults or backend-specific auto-detection.
- `InferenceBackend` method additions must preserve compatibility expectations
  for existing callers or be versioned through coordinated contract changes.
- `reranking_mode` is backend-consumed lifecycle metadata, not a UI hint; host
  layers should treat it as part of sidecar startup configuration.
- Backend `start()` results own lifecycle reuse facts when a backend can attach
  to an already-loaded runtime. Callers should consume that outcome instead of
  inferring reuse from adapter-local state.
- `ChatChunk.usage` is optional append-only stream metadata for bounded token
  counts. Backends may emit it on content chunks or usage-only terminal
  payloads; consumers must not expect prompts, generated text, logits, tensors,
  Python kwargs, CLI flags, or local paths in this field.
- `ChatChunk.cache_handle_id` is optional append-only stream metadata for
  backend-local KV checkpoint handles. Backends should emit stable handle ids
  only, never KV bytes, prompt text, generated text, tensors, temp paths, or
  scheduler/runtime reuse policy.
- PyTorch worker errors must preserve request ids and canonical worker codes
  while sanitizing traceback frames and local path tokens before they become
  `BackendError` messages. PyTorch task-join failures use the same sanitizer
  even when the failure is produced by Rust async task boundaries rather than a
  structured worker envelope. Malformed worker response JSON must also route
  through the canonical worker failure helpers instead of returning raw decode
  errors without request ids.
- Streaming response parsers should use `strip_prefix` for SSE `data:` lines
  so prefix handling stays explicit and warning-clean under the Rust clippy
  audit.
- llama.cpp request parsing, rerank normalization, generation-option mapping,
  sidecar start error mapping, and KV-cache fingerprint helpers stay in
  `llamacpp_support.rs` so
  `llamacpp.rs` remains focused on the backend facade and trait methods.
- Candle embedding load plans may validate local HF-compatible package
  directories for config, tokenizer, safetensors weights, dtype, first supported
  model family, and CPU/CUDA device hints, but must continue to fail closed as
  unavailable until the backend constructs real Candle tokenizers, tensors, and
  model modules.
- PyTorch backend capability, lifecycle, KV-cache fingerprint, prompt
  extraction, and system prompt tests stay in `pytorch_tests.rs` so
  `pytorch.rs` remains focused on production adapter behavior.
- The PyTorch backend must register `worker.py` sibling modules, including
  private runtime and transformers helpers, before loading the embedded worker
  facade.
- Embedded PyTorch worker source registration and Python result extraction stay
  in `pytorch_worker.rs` so `pytorch.rs` remains focused on the backend facade,
  model lifecycle, generation, and KV-cache trait methods.
- PyTorch Rust/Python envelope DTOs stay in `pytorch_worker_contract.rs`.
  They are backend-local implementation contracts, not public Pantograph graph
  node shapes or scheduler/runtime-registry policy.
- PyTorch audio transcription request inputs cross the embedded Python boundary
  through the same versioned worker-envelope pattern as Transformers load and
  text generation. Unsupported non-null audio `extra_options` fail closed until
  they are deliberately mapped to backend-local kwargs.
- PyTorch transport exceptions from the embedded Python boundary must be
  normalized before becoming `BackendError` messages: keep request ids,
  canonical worker codes, and bounded exception summaries, but strip Python
  traceback frames and redact local path tokens.
- PyTorch audio transcription worker results must decode through the typed
  result helper so missing or malformed `text`, `language`, and
  `duration_seconds` fields become canonical
  `pytorch_worker_audio_transcription_failed` errors instead of silent default
  transcript values.
- Backend-native generation fields and kwargs must stay inside backend-local
  mapping helpers. PyTorch maps canonical generation options to
  Transformers-style kwargs, while llama.cpp maps them to bounded
  OpenAI-compatible request fields; callers consume compatibility diagnostics
  instead of backend-native flags.
- dLLM/Sherry controls such as masked prompt JSON, denoising steps, and block
  length are PyTorch worker-envelope fields only. They preserve backend-local
  custom generation behavior without becoming public Pantograph graph/request
  contract fields.
