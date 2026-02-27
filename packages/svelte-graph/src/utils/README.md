# packages/svelte-graph/src/utils

## Purpose
Supporting package modules, constants, utilities, and types for the graph library.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| buildRegistry.ts | Source file used by modules in this directory. |
| geometry.ts | Source file used by modules in this directory. |

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
