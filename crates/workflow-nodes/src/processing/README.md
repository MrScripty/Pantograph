# crates/workflow-nodes/src/processing

## Purpose
Submodule source for this crate, grouped by responsibility.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| audio_generation.rs | Source file used by modules in this directory. |
| depth_estimation.rs | Source file used by modules in this directory. |
| diffusion_inference.rs | Source file used by modules in this directory. |
| embedding.rs | Source file used by modules in this directory. |
| expand_settings.rs | Source file used by modules in this directory. |
| inference.rs | Source file used by modules in this directory. |
| json_filter.rs | Source file used by modules in this directory. |
| llamacpp_inference.rs | Source file used by modules in this directory. |
| mod.rs | Source file used by modules in this directory. |
| ollama_inference.rs | Source file used by modules in this directory. |
| pytorch_inference.rs | Source file used by modules in this directory. |
| unload_model.rs | Source file used by modules in this directory. |
| validator.rs | Source file used by modules in this directory. |
| vision_analysis.rs | Source file used by modules in this directory. |

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
