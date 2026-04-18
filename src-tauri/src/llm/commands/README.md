# src-tauri/src/llm/commands

## Purpose
LLM gateway, process management, and Tauri command handlers for model and server operations.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| agent.rs | Source file used by modules in this directory. |
| backend.rs | Source file used by modules in this directory. |
| binary.rs | Source file used by modules in this directory. |
| config.rs | Source file used by modules in this directory. |
| docs.rs | Source file used by modules in this directory. |
| embedding.rs | Source file used by modules in this directory. |
| health.rs | Source file used by modules in this directory. |
| mod.rs | Source file used by modules in this directory. |
| port.rs | Source file used by modules in this directory. |
| rag.rs | Source file used by modules in this directory. |
| registry.rs | Runtime-registry and runtime-debug Tauri command entrypoints. |
| registry/ | Focused helpers extracted from the runtime-registry command boundary. |
| registry/tests.rs | Runtime-registry and runtime-debug command regression coverage. |
| sandbox.rs | Source file used by modules in this directory. |
| server.rs | Source file used by modules in this directory. |
| shared.rs | Source file used by modules in this directory. |
| version.rs | Source file used by modules in this directory. |
| vision.rs | Source file used by modules in this directory. |

## Design Decisions
- Keep files in this directory scoped to a single responsibility boundary.
- Prefer explicit module boundaries over cross-cutting utility placement.
- Maintain predictable naming so callers can discover related modules quickly.
- Command handlers that affect embedding-runtime availability should reuse the
  shared host-side RAG sync helper instead of caching embedding endpoints with
  command-local logic.
- Keep `registry.rs` as the command facade while extracting request contracts,
  runtime-debug aggregation, and tests into `registry/` support modules.
- Keep runtime-debug snapshot DTOs and cross-layer aggregation helpers in
  `registry/debug.rs` so the command facade stays focused on host-state lookup
  and command-level error envelopes.
- When runtime-debug trace requests resolve to multiple backend traces, return
  additive trace-selection ambiguity metadata instead of silently collapsing to
  incidental ordering in the command layer.

## Dependencies
**Internal:** Neighboring modules in this source tree and the nearest package/crate entry points.
**External:** Dependencies declared in the corresponding manifest files.

## Usage Examples
```rust
// Example: expose modules from this directory in the crate root.
mod module_name;
```
