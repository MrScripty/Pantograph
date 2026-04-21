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
| `kv_cache.rs` | Backend-owned execution handlers for KV-cache save/load/truncate nodes plus live llama.cpp/PyTorch restore-capture helpers and structured KV diagnostics emitted by `CoreTaskExecutor`. |
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
