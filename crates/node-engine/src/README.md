# crates/node-engine/src

## Purpose
Core library source files for this crate's runtime and domain behavior.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| builder.rs | Source file used by modules in this directory. |
| composite_executor.rs | Source file used by modules in this directory. |
| core_executor.rs | Source file used by modules in this directory. |
| descriptor.rs | Source file used by modules in this directory. |
| engine.rs | Source file used by modules in this directory. |
| error.rs | Source file used by modules in this directory. |
| events.rs | Source file used by modules in this directory. |
| extensions.rs | Source file used by modules in this directory. |
| groups.rs | Source file used by modules in this directory. |
| lib.rs | Source file used by modules in this directory. |
| model_dependencies.rs | Source file used by modules in this directory. |
| orchestration/ | Subdirectory containing related implementation details. |
| path_validation.rs | Source file used by modules in this directory. |
| port_options.rs | Source file used by modules in this directory. |
| registry.rs | Source file used by modules in this directory. |
| tasks/ | Subdirectory containing related implementation details. |
| types.rs | Source file used by modules in this directory. |
| undo.rs | Source file used by modules in this directory. |
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
