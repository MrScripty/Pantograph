# packages/svelte-graph/src

## Purpose
Reusable Svelte graph package source shared with the Pantograph frontend.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| backends/ | Subdirectory containing related implementation details. |
| components/ | Subdirectory containing related implementation details. |
| constants/ | Subdirectory containing related implementation details. |
| context/ | Subdirectory containing related implementation details. |
| horseshoeDragSession.ts | Shared drag-session controller that tracks horseshoe visibility, queued-open state, anchor position, and blocked reasons during connection drags. |
| horseshoeInvocation.ts | Shared drag-session key matching, open resolution, and blocked-reason diagnostics for horseshoe invocation. |
| horseshoeSelector.ts | Shared horseshoe windowing, rotation, and typeahead helpers for drag-time insert UI. |
| index.ts | Source file used by modules in this directory. |
| stores/ | Subdirectory containing related implementation details. |
| types/ | Subdirectory containing related implementation details. |
| utils/ | Subdirectory containing related implementation details. |

## Design Decisions
- Keep files in this directory scoped to a single responsibility boundary.
- Prefer explicit module boundaries over cross-cutting utility placement.
- Maintain predictable naming so callers can discover related modules quickly.
- Keep drag-time insert ranking/windowing logic in shared source so package and
  app canvases do not drift on horseshoe behavior.
- Keep horseshoe invocation gating and blocked-reason diagnostics in shared
  source so package and app canvases use the same drag-session rules.
- Keep horseshoe drag-session state in shared source so package and app canvases
  do not maintain divergent queued-open, anchor, or blocked-state lifecycles.

## Dependencies
**Internal:** Neighboring modules in this source tree and the nearest package/crate entry points.
**External:** Dependencies declared in the corresponding manifest files.

## Usage Examples
```ts
// Example: import API from this directory.
import { value } from './module';
```
