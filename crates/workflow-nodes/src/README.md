# crates/workflow-nodes/src

## Purpose
Core library source files for this crate's runtime and domain behavior.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| control/ | Subdirectory containing related implementation details. |
| input/ | Subdirectory containing related implementation details. |
| lib.rs | Source file used by modules in this directory. |
| output/ | Subdirectory containing related implementation details. |
| processing/ | Subdirectory containing related implementation details. |
| setup.rs | Source file used by modules in this directory. |
| storage/ | Subdirectory containing related implementation details. |
| system/ | Subdirectory containing related implementation details. |
| tool/ | Subdirectory containing related implementation details. |

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
