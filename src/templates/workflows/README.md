# src/templates/workflows

## Purpose
Source files and submodules for this part of the codebase.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| svelte-code-agent.json | Source file used by modules in this directory. |

## Design Decisions
- Keep files in this directory scoped to a single responsibility boundary.
- Prefer explicit module boundaries over cross-cutting utility placement.
- Maintain predictable naming so callers can discover related modules quickly.

## Dependencies
**Internal:** Neighboring modules in this source tree and the nearest package/crate entry points.
**External:** Dependencies declared in the corresponding manifest files.

## Usage Examples
```text
Add modules in this directory and reference them from the nearest package/crate entry point.
```
