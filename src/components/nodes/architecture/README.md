# src/components/nodes/architecture

## Purpose
Svelte UI components for the frontend experience, organized by feature and composition boundaries.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| ArchBackendNode.svelte | Source file used by modules in this directory. |
| ArchBaseNode.svelte | Source file used by modules in this directory. |
| ArchCommandNode.svelte | Source file used by modules in this directory. |
| ArchComponentNode.svelte | Source file used by modules in this directory. |
| ArchServiceNode.svelte | Source file used by modules in this directory. |
| ArchStoreNode.svelte | Source file used by modules in this directory. |
| index.ts | Source file used by modules in this directory. |

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
