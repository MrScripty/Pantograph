# packages/svelte-graph/src/components/nodes

## Purpose
Reusable graph UI components and rendering primitives for node and edge interactions.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| BaseNode.svelte | Source file used by modules in this directory. |
| GenericNode.svelte | Source file used by modules in this directory. |
| LlamaCppInferenceNode.svelte | Source file used by modules in this directory. |
| PumaLibNode.svelte | Source file used by modules in this directory. |
| TextInputNode.svelte | Source file used by modules in this directory. |
| TextOutputNode.svelte | Source file used by modules in this directory. |

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
