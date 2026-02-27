# src-tauri/src/agent/rag

## Purpose
Agent orchestration, retrieval, and tool implementations used by the backend assistant runtime.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| error.rs | Source file used by modules in this directory. |
| lancedb.rs | Source file used by modules in this directory. |
| manager.rs | Source file used by modules in this directory. |
| mod.rs | Source file used by modules in this directory. |
| types.rs | Source file used by modules in this directory. |

## Design Decisions
- Keep files in this directory scoped to a single responsibility boundary.
- Prefer explicit module boundaries over cross-cutting utility placement.
- Maintain predictable naming so callers can discover related modules quickly.

## Dependencies
**Internal:** Neighboring modules in this source tree and the nearest package/crate entry points.
**External:** Dependencies declared in the corresponding manifest files.

## Usage Examples
```rust
// Example: expose modules from this directory in the crate root.
mod module_name;
```
