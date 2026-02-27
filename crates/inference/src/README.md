# crates/inference/src

## Purpose
Core library source files for this crate's runtime and domain behavior.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| backend/ | Subdirectory containing related implementation details. |
| config.rs | Source file used by modules in this directory. |
| constants.rs | Source file used by modules in this directory. |
| device.rs | Source file used by modules in this directory. |
| gateway.rs | Source file used by modules in this directory. |
| kv_cache/ | Subdirectory containing related implementation details. |
| lib.rs | Source file used by modules in this directory. |
| process.rs | Source file used by modules in this directory. |
| server.rs | Source file used by modules in this directory. |
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
