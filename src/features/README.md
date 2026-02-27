# src/features

## Purpose
Feature-level module entry points that group and expose domain functionality.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| agent/ | Subdirectory containing related implementation details. |
| config/ | Subdirectory containing related implementation details. |
| drawing/ | Subdirectory containing related implementation details. |
| index.ts | Source file used by modules in this directory. |
| llm/ | Subdirectory containing related implementation details. |
| rag/ | Subdirectory containing related implementation details. |

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
