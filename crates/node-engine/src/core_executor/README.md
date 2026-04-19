# `crates/node-engine/src/core_executor`

## Responsibility

This directory contains focused helper modules that support the
backend-owned `CoreTaskExecutor` facade.

The facade in `../core_executor.rs` remains the public entry point for
host-independent node execution. Submodules in this directory exist to keep
large execution-path slices isolated by responsibility without moving business
logic into frontend, transport, or descriptor crates.

## Current Modules

| File | Responsibility |
| --- | --- |
| `kv_cache.rs` | Backend-owned execution handlers for KV-cache save/load/truncate nodes plus live llama.cpp/PyTorch restore-capture helpers and structured KV diagnostics emitted by `CoreTaskExecutor`. |

## Ownership Rules

- Keep node execution behavior in `node-engine`.
- Descriptor crates such as `workflow-nodes` may declare metadata and ports, but
  they must not become a second execution owner.
- Structured execution diagnostics emitted here are backend facts. Tauri and
  frontend layers may forward them, but they must not reinterpret them into a
  second reuse-policy owner.
- Grow the `CoreTaskExecutor` facade additively by extracting focused helper
  modules when a handler family becomes large enough to warrant its own local
  boundary.

## Standards Notes

- New helper modules should stay cohesive around one execution concern.
- Preserve existing `CoreTaskExecutor` call sites while shrinking oversized
  implementation blocks behind the facade.
- Update this README whenever new execution helper modules are added here.
