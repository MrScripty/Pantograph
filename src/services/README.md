# src/services

## Purpose
This directory contains Pantograph's app-level service boundaries. It exists so
UI components can call stable, testable orchestration and integration modules
instead of coupling directly to Tauri commands, runtime wiring, or low-level
transport details.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `workflow/` | Workflow-domain service boundary for graph sessions, execution, and backend-owned connection-intent commands. |
| `diagnostics/` | Diagnostics-domain service boundary for workflow trace accumulation and inspection snapshots. |
| `agent/` | Agent-facing orchestration helpers that keep prompt/tool flows out of the component tree. |
| `architecture/` | Services that translate architecture data into app-facing graph behavior. |
| `managedRuntime/` | Thin app-facing service boundary for backend-owned managed-runtime manager contracts and progress events. |
| `LLMService.ts` | App-facing service for model/runtime interactions that do not belong in UI components. |

## Problem
Pantograph has multiple long-lived product surfaces such as workflow execution,
diagnostics, drawing, and local runtime management. Without service boundaries,
Svelte components would accumulate transport logic, lifecycle ownership, and
cross-module orchestration that is difficult to test and easy to race.

## Constraints
- Services in this directory should stay framework-agnostic unless they are
  explicitly app-only integration layers.
- UI components may depend on services, but services must not depend on UI
  components.
- Long-lived runtime resources and subscriptions need explicit ownership rather
  than ad hoc component-local lifecycles.
- Existing transitional services such as `workflow/` must remain compatible
  with legacy callers while the app converges on newer package boundaries.

## Decision
Keep service responsibilities grouped by domain boundary rather than by UI
screen. Workflow execution and diagnostics therefore live in separate service
subdirectories even though the workflow GUI consumes both; that keeps trace
accumulation, execution orchestration, and rendering concerns from collapsing
into one module.

## Alternatives Rejected
- Put all workflow-adjacent logic directly into `src/components/`.
  Rejected because it would mix transport, lifecycle, and presentation.
- Collapse diagnostics into the workflow service boundary.
  Rejected because inspection state has different retention and selection rules
  than workflow execution itself.

## Invariants
- Service modules remain the only place where app code should assemble
  cross-component orchestration rules.
- UI components consume service APIs and snapshots; they do not own core
  transport or runtime integration logic.
- Long-lived stateful flows keep one owner per lifecycle.

## Revisit Triggers
- A service subdirectory grows large enough that it needs versioned contracts or
  an ADR-backed split.
- Package-level reusable services replace current app-only boundaries.
- The app introduces a second host environment that requires cleaner transport
  partitioning.

## Dependencies
**Internal:** `src/backends`, `src/stores`, `src/registry`, Rust/Tauri command
boundaries under `src-tauri`.
**External:** Tauri APIs, browser APIs where required by app integration
services, and runtime libraries declared in the project manifest.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- Reason: the workflow service boundary follows the same service-vs-adapter
  separation even though this directory also contains frontend-facing services.

## Usage Examples
```ts
import { workflowService } from './workflow/WorkflowService';

const definitions = await workflowService.getNodeDefinitions();
```

## API Consumer Contract
- Components should treat services in this directory as the stable app-facing
  boundary for orchestration work.
- Services may expose additive methods over time, but callers should not depend
  on private implementation modules within each subdirectory.
- When a service owns subscriptions, timers, or retained trace state, callers
  should use explicit start/stop or subscribe/unsubscribe entrypoints rather
  than assuming implicit global behavior.

## Structured Producer Contract
- None.
- Reason: this directory groups service boundaries; individual subdirectories
  such as `workflow/` and `diagnostics/` document any machine-consumed contract
  details they own.
- Revisit trigger: the directory itself starts publishing shared service
  manifests or generated metadata.
