# src

## Purpose
Frontend application source for Pantograph, including UI, services, stores, and shared utilities.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| App.svelte | Source file used by modules in this directory. |
| backends/ | Subdirectory containing related implementation details. |
| components/ | Subdirectory containing related implementation details. |
| config/ | Subdirectory containing related implementation details. |
| constants.ts | Source file used by modules in this directory. |
| features/ | Subdirectory containing related implementation details. |
| generated/ | Runtime-generated Svelte component workspace used by the hot-load sandbox; ignored by the outer repo because it owns a nested Git history for component undo/redo. |
| lib/ | Subdirectory containing related implementation details. |
| main.ts | Source file used by modules in this directory. |
| registry/ | Subdirectory containing related implementation details. |
| services/ | Subdirectory containing related implementation details. |
| shared/ | Subdirectory containing related implementation details. |
| stores/ | Subdirectory containing related implementation details. |
| styles.css | Source file used by modules in this directory. |
| templates/ | Subdirectory containing related implementation details. |
| types/ | Subdirectory containing related implementation details. |
| types.ts | Source file used by modules in this directory. |
| vite-env.d.ts | Source file used by modules in this directory. |

## Design Decisions
- Keep files in this directory scoped to a single responsibility boundary.
- Prefer explicit module boundaries over cross-cutting utility placement.
- Maintain predictable naming so callers can discover related modules quickly.
- Treat `generated/` as a temporary source-root exception: runtime component
  files and their nested Git history are ignored by the outer repository, while
  migration to a non-source runtime state directory is tracked in the standards
  compliance plan.

## Dependencies
**Internal:** Neighboring modules in this source tree and the nearest package/crate entry points.
**External:** Dependencies declared in the corresponding manifest files.

## Usage Examples
```text
Add modules in this directory and reference them from the nearest package/crate entry point.
```
