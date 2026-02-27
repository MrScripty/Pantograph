# src/lib/hotload-sandbox/services

## Purpose
Internal frontend library utilities and sandbox helpers shared across application features.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| ComponentRegistry.ts | Source file used by modules in this directory. |
| ErrorReporter.ts | Source file used by modules in this directory. |
| GlobRegistry.ts | Source file used by modules in this directory. |
| ImportManager.ts | Source file used by modules in this directory. |
| ValidationCache.ts | Source file used by modules in this directory. |

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
