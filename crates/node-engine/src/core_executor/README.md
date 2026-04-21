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
| `audio_nodes.rs` | Feature-gated Stable Audio Python-worker initialization and text-to-audio execution. |
| `dependency_preflight.rs` | Model dependency binding, backend-key normalization, task-type inference, model-reference construction, and dependency resolver preflight used before runtime-backed execution. |
| `file_io.rs` | Async read-file/write-file handlers that resolve paths through the project-root validation boundary before touching the filesystem. |
| `inference_nodes.rs` | Feature-gated shared inference helpers plus reranking, embeddings, OpenAI-compatible chat, vision, and unload-model handlers. |
| `kv_cache.rs` | Backend-owned execution handlers for KV-cache save/load/truncate nodes plus live llama.cpp/PyTorch restore-capture helpers and structured KV diagnostics emitted by `CoreTaskExecutor`. |
| `llamacpp_nodes.rs` | Feature-gated llama.cpp completion execution, streaming response parsing, and KV-cache integration. |
| `ollama.rs` | Standalone Ollama HTTP generation handler and response-to-model-reference projection for the `ollama-inference` node. |
| `pure_nodes.rs` | Synchronous built-in node handlers for input/output passthrough, model provider payloads, control-flow helpers, validation, JSON filtering, human input, and disabled tool execution. |
| `pytorch_nodes.rs` | Feature-gated PyTorch Python-worker initialization, inference execution, streaming, and KV-cache integration. |
| `settings.rs` | Settings-schema expansion and shared optional-input readers used by pure settings nodes and runtime-backed adapters. |
| `tests.rs` | Behavior tests for core executor node dispatch, input/output normalization, settings expansion, dependency preflight, and feature-gated inference parsing helpers. |

## Problem
`CoreTaskExecutor` owns several unrelated execution concerns: built-in pure
node handlers, file I/O, runtime dependency preflight, inference adapters,
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
- File I/O handlers stay in `file_io.rs` and must continue resolving paths
  through `path_validation` before reading or writing host files.
- Settings expansion and optional-input readers stay in `settings.rs` so
  runtime adapters can share one normalization contract for schema defaults,
  connected port overrides, aliases, and boolean coercion.
- Dependency preflight and model-reference construction stay in
  `dependency_preflight.rs` so runtime adapters share backend-key and
  dependency-state validation without growing dispatch code.
- The standalone Ollama HTTP handler stays in `ollama.rs`; gateway-backed
  inference handlers should not be added there.
- Gateway-backed inference handlers stay in `inference_nodes.rs`; PyTorch and
  audio Python-worker handlers remain separate feature families.
- Llama.cpp completion execution stays in `llamacpp_nodes.rs`; reranking and
  embedding execution stay with the shared inference helpers until they deserve
  their own module.
- PyTorch Python-worker execution stays in `pytorch_nodes.rs`; Stable Audio
  Python-worker execution stays in `audio_nodes.rs`.
- The public facade remains `CoreTaskExecutor`; helper modules are private
  implementation details unless a separate public contract is explicitly
  introduced.
- Tests in this directory may use private facade helpers through `super::*`
  while they verify core-executor implementation behavior.

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
