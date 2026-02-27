# src/lib/design-system

## Purpose
Internal frontend library utilities and sandbox helpers shared across application features.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| agentContext.ts | Source file used by modules in this directory. |
| icons.ts | Source file used by modules in this directory. |
| index.ts | Source file used by modules in this directory. |
| tokens.ts | Source file used by modules in this directory. |
| validator.ts | Source file used by modules in this directory. |

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
