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
| headless_workflow_commands.rs | Source file used by modules in this directory. |
| mod.rs | Source file used by modules in this directory. |
| model_dependencies.rs | Source file used by modules in this directory. |
| orchestration.rs | Source file used by modules in this directory. |
| python_runtime.rs | Python sidecar adapter boundary for workflow nodes. |
| python_runtime_bridge.py | Python bridge entrypoint used by `python_runtime.rs`. |
| registry.rs | Source file used by modules in this directory. |
| task_executor.rs | Source file used by modules in this directory. |
| types.rs | Source file used by modules in this directory. |
| validation.rs | Source file used by modules in this directory. |
| workflow_execution_commands.rs | Source file used by modules in this directory. |
| workflow_model_review_commands.rs | Source file used by modules in this directory. |
| workflow_persistence_commands.rs | Source file used by modules in this directory. |
| workflow_port_query_commands.rs | Source file used by modules in this directory. |

## ONNX/KittenTTS Notes
- `onnx-inference` executes through the Python sidecar path in `task_executor.rs`.
- Model-specific options must come from `inference_settings` metadata (Puma-Library),
  not hardcoded ONNX node ports.
- Audio stream chunks are emitted as workflow stream events and consumed by the
  frontend audio output node for buffered playback.

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
