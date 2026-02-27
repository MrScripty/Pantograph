# src-tauri/src/agent/tools

## Purpose
Agent orchestration, retrieval, and tool implementations used by the backend assistant runtime.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| error.rs | Source file used by modules in this directory. |
| list.rs | Source file used by modules in this directory. |
| mod.rs | Source file used by modules in this directory. |
| read.rs | Source file used by modules in this directory. |
| tailwind.rs | Source file used by modules in this directory. |
| validation.rs | Source file used by modules in this directory. |
| write.rs | Source file used by modules in this directory. |

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
