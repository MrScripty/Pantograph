# src-tauri/src/llm

## Purpose
LLM gateway, process management, and Tauri command handlers for model and server operations.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| backend/ | Subdirectory containing related implementation details. |
| commands/ | Subdirectory containing related implementation details. |
| embedding_server.rs | Source file used by modules in this directory. |
| gateway.rs | Source file used by modules in this directory. |
| health_monitor.rs | Source file used by modules in this directory. |
| mod.rs | Source file used by modules in this directory. |
| port_manager.rs | Source file used by modules in this directory. |
| process_tauri.rs | Source file used by modules in this directory. |
| recovery.rs | Source file used by modules in this directory. |
| server_discovery.rs | Source file used by modules in this directory. |
| types.rs | Source file used by modules in this directory. |

## Design Decisions
- Keep files in this directory scoped to a single responsibility boundary.
- Prefer explicit module boundaries over cross-cutting utility placement.
- Maintain predictable naming so callers can discover related modules quickly.
- Keep sidecar lifecycle ownership in Rust adapters. The dedicated embedding
  sidecar now records backend-owned runtime lifecycle snapshots in
  `embedding_server.rs`; GUI consumers may read those facts but must not infer
  warmup/reuse decisions locally.

## Dependencies
**Internal:** Neighboring modules in this source tree and the nearest package/crate entry points.
**External:** Dependencies declared in the corresponding manifest files.

## Usage Examples
```rust
// Example: expose modules from this directory in the crate root.
mod module_name;
```
