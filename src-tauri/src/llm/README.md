# src-tauri/src/llm

## Purpose
LLM gateway, process management, and Tauri command handlers for model and server operations.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| backend/ | Subdirectory containing related implementation details. |
| commands/ | Subdirectory containing related implementation details. |
| gateway.rs | Source file used by modules in this directory. |
| health_monitor.rs | Source file used by modules in this directory. |
| mod.rs | Source file used by modules in this directory. |
| port_manager.rs | Source file used by modules in this directory. |
| process_tauri.rs | Source file used by modules in this directory. |
| recovery.rs | Source file used by modules in this directory. |
| server_discovery.rs | Source file used by modules in this directory. |
| startup.rs | Runtime startup request construction and shared model-path resolution helpers for Tauri hosts. |
| types.rs | Source file used by modules in this directory. |

## Design Decisions
- Keep files in this directory scoped to a single responsibility boundary.
- Prefer explicit module boundaries over cross-cutting utility placement.
- Maintain predictable naming so callers can discover related modules quickly.
- Keep Tauri focused on composition and command adaptation. The dedicated
  embedding runtime lifecycle now lives in `crates/inference`; GUI consumers
  may read those backend-owned facts but must not infer warmup/reuse decisions
  locally.
- Server lifecycle commands should return the backend-owned runtime status
  contract directly instead of translating it into narrower adapter-local DTOs.

## Dependencies
**Internal:** Neighboring modules in this source tree and the nearest package/crate entry points.
**External:** Dependencies declared in the corresponding manifest files.

## Usage Examples
```rust
// Example: expose modules from this directory in the crate root.
mod module_name;
```
