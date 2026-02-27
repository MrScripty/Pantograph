# src-tauri/src/workflow

## Purpose
Workflow command handlers, execution plumbing, and validation utilities for graph operations.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| commands.rs | Source file used by modules in this directory. |
| event_adapter.rs | Source file used by modules in this directory. |
| events.rs | Source file used by modules in this directory. |
| execution_manager.rs | Source file used by modules in this directory. |
| groups.rs | Source file used by modules in this directory. |
| mod.rs | Source file used by modules in this directory. |
| model_dependencies.rs | Source file used by modules in this directory. |
| orchestration.rs | Source file used by modules in this directory. |
| registry.rs | Source file used by modules in this directory. |
| task_executor.rs | Source file used by modules in this directory. |
| types.rs | Source file used by modules in this directory. |
| validation.rs | Source file used by modules in this directory. |

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
