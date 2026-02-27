# src/stores

## Purpose
State management stores that coordinate reactive application data across the frontend.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| accordionStore.ts | Source file used by modules in this directory. |
| architectureStore.ts | Source file used by modules in this directory. |
| canvasStore.ts | Source file used by modules in this directory. |
| chunkPreviewStore.ts | Source file used by modules in this directory. |
| graphSessionStore.ts | Source file used by modules in this directory. |
| interactionModeStore.ts | Source file used by modules in this directory. |
| linkStore.ts | Source file used by modules in this directory. |
| orchestrationStore.ts | Source file used by modules in this directory. |
| panelStore.ts | Source file used by modules in this directory. |
| promptHistoryStore.ts | Source file used by modules in this directory. |
| sidePanelTabStore.ts | Source file used by modules in this directory. |
| storeInstances.ts | Source file used by modules in this directory. |
| timelineStore.ts | Source file used by modules in this directory. |
| undoStore.ts | Source file used by modules in this directory. |
| viewModeStore.ts | Source file used by modules in this directory. |
| viewStore.ts | Source file used by modules in this directory. |
| workflowStore.ts | Source file used by modules in this directory. |

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
