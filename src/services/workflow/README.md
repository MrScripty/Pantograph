# src/services/workflow

## Purpose
Client-side service modules that encapsulate runtime integrations and domain orchestration logic.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| WorkflowService.ts | Source file used by modules in this directory. |
| groupTypes.ts | Source file used by modules in this directory. |
| index.ts | Source file used by modules in this directory. |
| mocks.ts | Source file used by modules in this directory. |
| templateService.ts | Source file used by modules in this directory. |
| types.ts | Source file used by modules in this directory. |

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
