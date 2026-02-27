# src/lib

## Purpose
Internal frontend library utilities and sandbox helpers shared across application features.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| design-system/ | Subdirectory containing related implementation details. |
| hotload-sandbox/ | Subdirectory containing related implementation details. |

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
