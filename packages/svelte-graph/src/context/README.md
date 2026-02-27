# packages/svelte-graph/src/context

## Purpose
Supporting package modules, constants, utilities, and types for the graph library.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| createGraphContext.ts | Source file used by modules in this directory. |
| keys.ts | Source file used by modules in this directory. |
| types.ts | Source file used by modules in this directory. |
| useGraphContext.ts | Source file used by modules in this directory. |

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
