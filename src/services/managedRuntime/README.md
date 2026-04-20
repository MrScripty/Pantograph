# src/services/managedRuntime

## Purpose
This directory projects the backend-owned managed-runtime manager contract into
Pantograph's Settings GUI. It exists so Svelte components can subscribe to
runtime snapshots and invoke install/pause/cancel/remove/version-selection
actions without reintroducing runtime lifecycle policy into the frontend.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `ManagedRuntimeService.ts` | App-facing service owner for runtime snapshot caching, Tauri command calls, and progress fan-out. |
| `types.ts` | TypeScript mirror of the backend-owned runtime-manager payloads projected into the GUI. |
| `index.ts` | Public re-export surface for the managed-runtime service and contract types. |

## Problem
Pantograph's runtime redistributables manager is backend-owned, but the GUI
still needs to list runtimes, inspect version catalogs, show install progress,
and let the user update selected/default version policy. Without a dedicated
frontend service boundary, Svelte components would each call Tauri commands
directly and drift into duplicate runtime snapshot or progress ownership.

## Constraints
- Runtime readiness, version selection policy, install state, and durable job
  state remain authoritative in backend Rust.
- This directory may keep app-local ephemeral snapshot copies only to fan
  backend updates out to multiple components.
- Callers must not bypass this service when it already owns the app-facing
  contract for managed-runtime UI flows.
- Changes to the projected payloads must stay aligned with the backend contract
  in `pantograph-embedded-runtime` and `inference`.

## Decision
Keep a single app-facing managed-runtime service boundary that owns a
synchronized snapshot cache of `ManagedRuntimeManagerRuntimeView` values.
`ManagedRuntimeService` invokes thin Tauri commands, updates the local cache
from backend responses and progress events, and notifies subscribers.
Components then render that projected state and call back through the service
for actions such as install, pause, cancel, remove, and version policy updates.

## Alternatives Rejected
- Let each Settings component call Tauri commands directly.
  Rejected because it would duplicate cache/update logic and make runtime
  status drift across components.
- Move runtime-manager business logic into TypeScript.
  Rejected because runtime selection, readiness, and install orchestration are
  backend-owned policy.

## Invariants
- `ManagedRuntimeManagerRuntimeView` mirrors the backend manager snapshot.
- Progress callbacks surface backend-owned runtime snapshots alongside byte
  progress so the GUI does not need ad hoc refresh loops during jobs.
- The service owns the app-local synchronized runtime snapshot cache and fans
  updates out to subscribers.
- UI callers use this service instead of calling redistributable Tauri commands
  directly when a service method already exists.
- Retained artifact, resumability, readiness, and selection facts remain
  backend-owned; this service only projects them.

## Revisit Triggers
- Another Pantograph host outside Tauri needs the same frontend-style runtime
  projection and justifies a host-agnostic client package.
- The backend introduces streaming/event subscription APIs that replace the
  current per-action progress channel shape.
- Runtime-manager payloads become large enough that the cache needs paging or a
  more selective invalidation strategy.

## Dependencies
**Internal:** `src-tauri/src/llm/commands/binary.rs`,
`pantograph-embedded-runtime` managed-runtime manager views, and GUI consumers
such as `src/components/runtime-manager/`.
**External:** Tauri's `invoke` and `Channel` APIs.

## Related ADRs
- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md` backend-owned
  redistributables manager boundary.

## Usage Examples
```ts
import { managedRuntimeService } from './managedRuntime';

const runtimes = await managedRuntimeService.listRuntimes();
const llama = await managedRuntimeService.inspectRuntime('llama_cpp');
```

## API Consumer Contract
- `listRuntimes()` refreshes the backend-owned runtime-manager view cache and
  returns a snapshot copy for callers.
- `inspectRuntime(runtimeId)` refreshes one runtime and updates the shared
  cache entry for that runtime.
- `installRuntime(runtimeId, onProgress)` forwards the backend install request
  and surfaces backend progress snapshots through `onProgress`.
- `pauseRuntimeJob`, `cancelRuntimeJob`, `removeRuntime`,
  `selectRuntimeVersion`, and `setDefaultRuntimeVersion` all route through thin
  backend commands and then update the local snapshot cache from backend
  responses.
- Subscribers receive cloned runtime snapshots and must treat them as read-only
  projections.

## Structured Producer Contract
- Stable fields mirror `ManagedRuntimeManagerRuntimeView`,
  `ManagedRuntimeVersionStatus`, `ManagedRuntimeJobStatus`,
  `ManagedRuntimeJobArtifactStatus`, and `ManagedRuntimeInstallHistoryEntry` in
  `types.ts`.
- Omitted nullable fields such as `selected_version`, `default_version`,
  `active_job`, and `job_artifact` retain their backend null semantics.
- Enum strings such as `install_state`, `readiness_state`, `state`, and
  `event` preserve backend labels verbatim.
- The service cache is ephemeral and regenerated from backend responses and
  progress channels; callers must not persist it as if it were an independent
  source of truth.
