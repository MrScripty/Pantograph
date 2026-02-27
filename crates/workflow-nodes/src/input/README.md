# crates/workflow-nodes/src/input

## Purpose
Submodule source for this crate, grouped by responsibility.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| audio_input.rs | Source file used by modules in this directory. |
| human_input.rs | Source file used by modules in this directory. |
| image_input.rs | Source file used by modules in this directory. |
| linked_input.rs | Source file used by modules in this directory. |
| masked_text_input.rs | Source file used by modules in this directory. |
| mod.rs | Source file used by modules in this directory. |
| model_provider.rs | Source file used by modules in this directory. |
| puma_lib.rs | Source file used by modules in this directory. |
| text_input.rs | Source file used by modules in this directory. |

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
