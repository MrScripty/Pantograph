# src/services

## Purpose
Client-side service modules that encapsulate runtime integrations and domain orchestration logic.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| AgentService.ts | Source file used by modules in this directory. |
| CanvasExport.ts | Source file used by modules in this directory. |
| ConfigService.ts | Source file used by modules in this directory. |
| DrawingAnalyzer.ts | Source file used by modules in this directory. |
| DrawingEngine.ts | Source file used by modules in this directory. |
| HealthMonitorService.ts | Source file used by modules in this directory. |
| HitTestService.ts | Source file used by modules in this directory. |
| HotLoadRegistry.ts | Source file used by modules in this directory. |
| LLMService.ts | Source file used by modules in this directory. |
| Logger.ts | Source file used by modules in this directory. |
| RagService.ts | Source file used by modules in this directory. |
| RuntimeCompiler.ts | Source file used by modules in this directory. |
| agent/ | Subdirectory containing related implementation details. |
| architecture/ | Subdirectory containing related implementation details. |
| workflow/ | Subdirectory containing related implementation details. |

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
