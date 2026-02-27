# src/templates

## Purpose
Source files and submodules for this part of the codebase.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| Button.svelte | Source file used by modules in this directory. |
| Card.svelte | Source file used by modules in this directory. |
| Input.svelte | Source file used by modules in this directory. |
| Modal.svelte | Source file used by modules in this directory. |
| workflows/ | Subdirectory containing related implementation details. |

## Design Decisions
- Keep files in this directory scoped to a single responsibility boundary.
- Prefer explicit module boundaries over cross-cutting utility placement.
- Maintain predictable naming so callers can discover related modules quickly.

## Dependencies
**Internal:** Neighboring modules in this source tree and the nearest package/crate entry points.
**External:** Dependencies declared in the corresponding manifest files.

## Usage Examples
```ts
// Example: import API from this directory.
import { value } from './module';
```
