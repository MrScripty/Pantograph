# crates/node-engine/src/core_executor

## Purpose
This directory contains focused helper modules that support the backend-owned
`CoreTaskExecutor` facade.

The facade in `../core_executor.rs` remains the public entry point for
host-independent node execution. Submodules in this directory keep large
execution-path slices isolated by responsibility without moving business logic
into frontend, transport, or descriptor crates.

## Contents
| File | Responsibility |
| --- | --- |
| `dependency_preflight.rs` | Retired dependency-preflight input rejection and path-free dependency-planning projection helpers. |
| `file_io.rs` | Async read-file/write-file handlers that resolve paths through the project-root validation boundary before touching the filesystem. |
| `inference_nodes.rs` | Feature-gated shared canonical inference request builders, graph result projection, and unload-model handling. |
| `inference_tests.rs` | Focused tests for canonical inference request building, backend-key normalization, embedding failure behavior, and reranker parsing. |
| `kv_cache.rs` | Backend-owned execution handlers for KV-cache save/load/truncate nodes plus structured KV diagnostics emitted by `CoreTaskExecutor`. |
| `kv_cache_parsing_tests.rs` | Focused tests for KV-cache storage-policy and marker parsing helpers. |
| `kv_cache_test_support.rs` | Mock inference backend and process fixtures shared by KV-cache behavior tests. |
| `kv_cache_tests.rs` | Focused KV-cache store, handle restore/capture, and backend-owned truncation tests. |
| `model_nodes.rs` | Pure model-provider and Puma library payload projection handlers. |
| `processing_nodes.rs` | Pure processing handlers for code validation and JSON path extraction. |
| `pure_nodes.rs` | Synchronous built-in node handlers for input/output passthrough, model provider payloads, control-flow helpers, validation, JSON filtering, human input, and disabled tool execution. |
| `retrieval_nodes.rs` | Feature-gated reranking and embedding execution plus reranker document parsing. |
| `settings.rs` | Settings-schema expansion and shared optional-input readers used by pure settings nodes and runtime-backed adapters. |
| `settings_tests.rs` | Focused tests for settings expansion, optional input readers, and file-I/O traversal rejection. |
| `tests.rs` | Behavior tests for core executor node dispatch, input/output normalization, settings expansion, dependency preflight, and feature-gated inference parsing helpers. |

## Problem
`CoreTaskExecutor` owns several unrelated execution concerns: built-in pure
node handlers, file I/O, retired dependency input rejection, inference adapters,
audio adapters, and tests. Keeping every helper inline makes dispatcher changes
hard to review and encourages unrelated execution policies to grow together.

## Constraints
- Keep node execution behavior in `node-engine`.
- Descriptor crates such as `workflow-nodes` may declare metadata and ports,
  but they must not become a second execution owner.
- Structured execution diagnostics emitted here are backend facts. Tauri and
  frontend layers may forward them, but they must not reinterpret them into a
  second reuse-policy owner.
- Preserve existing `CoreTaskExecutor` call sites while shrinking oversized
  implementation blocks behind the facade.

## Decision
Use this directory for cohesive core-executor helper modules and behavior test
modules. Extract helpers when an execution family or test group becomes large
enough to warrant a local boundary, while keeping `CoreTaskExecutor` as the
stable public facade and dispatch owner.

## Alternatives Rejected
- Keep every helper and test inline in `core_executor.rs`.
  Rejected because the file is already too large for focused review.
- Move built-in node execution into `workflow-nodes`.
  Rejected because descriptor ownership and execution ownership are separate
  contracts.

## Invariants
- New helper modules must stay cohesive around one execution concern.
- Synchronous built-in node handlers stay in `pure_nodes.rs`; runtime-backed,
  file-backed, or feature-gated adapters should not be added there.
- Pure processing handlers stay in `processing_nodes.rs` once they need helper
  logic beyond direct input/output passthrough.
- Model-provider and Puma library projection handlers stay in `model_nodes.rs`
  because they normalize model payload contracts for runtime-backed adapters.
- File I/O handlers stay in `file_io.rs` and must continue resolving paths
  through `path_validation` before reading or writing host files.
- Settings expansion and optional-input readers stay in `settings.rs` so
  runtime adapters can share one normalization contract for schema defaults,
  connected port overrides, aliases, and boolean coercion.
- Boolean settings readers in `settings.rs` are an inference-family helper
  contract; audio-only feature builds may compile the module, but they must not
  treat those inference-only readers as live production paths.
- Dependency preflight and model-reference construction are retired as
  successful node-engine runtime execution paths. `dependency_preflight.rs`
  now owns only retired model-reference input rejection and path-free
  dependency-planning projection helpers. It must not perform resolver lookup,
  `ModelDependencyRequest` construction, path repair, compatibility
  acceptance, runtime-host dispatch, lifecycle preflight emission, or
  `ModelRefV2` output. Explicit workflow/backend inputs are the only
  graph-owned backend signal here; resolved package facts may provide factual
  task/model inputs, but `recommended_backend` and package backend hints must
  not become executable backend selection in node-engine.
- Dependency preflight must not special-case retired direct inference node
  shapes such as `diffusion-inference`. Image-generation preflight enters
  through canonical `llm-inference` task metadata and resolved package facts.
- Node-engine must not emit dependency-preflight lifecycle events for runtime
  launch. Bounded inference lifecycle diagnostics may still be emitted by the
  canonical inference request path where those diagnostics describe task
  validation, package resolution, backend execution, result projection, and
  cleanup without reintroducing preflight authority.
- Gateway-backed inference handlers stay in `inference_nodes.rs`. Node-engine
  PyTorch launch has been retired; successful PyTorch execution must come from
  scheduler task state/results and runtime-host responses, not this directory.
  Node-engine Stable Audio launch has also been retired; successful audio
  execution must come from scheduler task state/results and runtime-host
  responses, not a node-engine Python-worker adapter or graph `model_path`.
  The retired direct `vision-analysis` HTTP path must not bypass canonical
  `llm-inference` image-understanding task contracts.
- Canonical `llm-inference` request builders in `inference_nodes.rs` accept
  `pumas_model_ref` or `model_ref` as the graph-authored model identity.
  Resolved Pumas package facts may be forwarded as host/planning facts for
  compatibility reporting, but they must not be promoted into graph model
  identity when a model reference was not explicitly wired.
- Canonical runtime readiness must come from scheduler task state and
  runtime-host readiness proofs, not node-engine dependency preflight.
  Path-only graph data is not a successful preflight identity.
- Canonical image-generation execution exposes the first generated image body on
  the graph-visible `image` output and compact per-image summaries in
  `results`. `results` must not duplicate generated image base64 bodies; the
  workflow artifact conversion path owns retention of the `image` body.
- Canonical image-generation request construction reads optional sampling
  intent from `denoising_scheduler`. The old graph/API `scheduler` key is not a
  compatibility alias; downstream inference DTOs and PyTorch image worker
  envelopes use the same canonical field name.
- Canonical `llm-inference` task/request compatibility checks should consume
  inference task registry request contracts instead of hard-coded backend names
  or raw task labels.
- Canonical text-generation request builders read public task semantics from
  `task_kind`/`taskKind` only. `task_id`/`taskId` remain workflow/node identity
  fields and must not be interpreted as task-kind aliases.
- Contract-only canonical tasks such as `image_understanding`,
  `depth_estimation`, and `video_understanding` fail during task validation
  before backend execution. Their lifecycle diagnostics may include bounded
  task option-support facts and stable artifact refs from graph inputs, but
  must not store local paths or fabricate executable backend behavior.
- Canonical text/chat usage projection in `inference_nodes.rs` is bounded
  graph metadata copied from typed gateway results or terminal stream chunks.
  It must not be recomputed from prompt or generated text.
- Reranking and embedding execution stay in `retrieval_nodes.rs`.
- PyTorch lifecycle operations invoked from shared canonical handlers, such as
  model unload in `inference_nodes.rs`, must go through the inference crate's
  typed worker-envelope helpers instead of importing the embedded Python worker
  directly.
- Node-engine must not directly load PyTorch models, call PyTorch text
  generation helpers, capture PyTorch KV-cache snapshots, or emit PyTorch
  `ModelRefV2` outputs. Unsupported or missing scheduler task state/results
  must fail closed with typed diagnostics.
- Node-engine llama.cpp launch has been retired; successful llama.cpp
  execution must come from scheduler task state/results and runtime-host
  responses. Node-engine must not start a llama.cpp server from graph
  `model_path`, call completion endpoints as the runtime-launch owner, or emit
  llama.cpp `ModelRefV2` outputs. The old live llama.cpp KV restore/capture
  helpers were removed with that launch path; explicit KV-cache nodes may
  still save, load, and truncate typed cache handles through `kv_cache.rs`.
- Python-worker handlers should pass worker parameters directly into their
  blocking closures and avoid redundant rebinding so the feature-gated path
  stays clippy-clean without changing runtime behavior.
- The public facade remains `CoreTaskExecutor`; helper modules are private
  implementation details unless a separate public contract is explicitly
  introduced.
- Tests in this directory may use private facade helpers through `super::*`
  while they verify core-executor implementation behavior.
- Test modules should split by behavior family once a single module becomes too
  large for focused review.

## Revisit Triggers
- Another execution family grows enough to deserve a focused helper module.
- The test module becomes large enough to split by behavior area.
- A helper module needs to become public API for external hosts.

## Dependencies
**Internal:** `node-engine` core executor facade, runtime dependency contracts,
workflow events, and optional feature-gated inference/audio support.

**External:** `serde_json`, async runtime support, and optional dependencies
declared by `node-engine` features.

## Related ADRs
- `docs/standards-compliance-analysis/refactor-plan.md`
- `crates/node-engine/src/README.md`

## Usage Examples
```rust
#[cfg(test)]
#[path = "core_executor/tests.rs"]
mod tests;
```
