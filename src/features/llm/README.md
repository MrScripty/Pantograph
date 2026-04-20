# src/features/llm

## Purpose
This directory exposes Pantograph's LLM-related feature entry points for the app
shell. It exists so server status, backend switching, and the mounted managed
runtime panel can be imported through one feature boundary instead of reaching
directly into unrelated component paths.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `index.ts` | Feature export surface for the LLM service, the mounted server-status shell, the compact backend selector, and the dedicated runtime-manager panel. |

## Problem
Pantograph needs a stable feature-level boundary for LLM/server UI so the app
shell can mount the right components without coupling itself to every internal
component path. This became more important once the runtime-manager UI split
into a mounted dedicated Settings panel and supporting server-status
subcomponents.

## Constraints
- This directory is only a feature export surface; it must not accumulate
  business logic.
- Exported components must still respect backend-owned runtime and workflow
  state.
- The export surface should reflect the mounted authoritative runtime-manager
  UI rather than legacy or unmounted one-off components.

## Decision
Keep `index.ts` as the single feature export entry point and update it to
publish the mounted `ManagedRuntimePanel` instead of the removed hardcoded
`BinaryDownloader` component. This keeps the feature surface aligned with the
current Settings UI and avoids preserving stale component entrypoints once the
dedicated runtime-manager screen is mounted.

## Alternatives Rejected
- Export every server-status child component from this directory.
  Rejected because those are internal decomposition helpers, not feature-level
  entry points.
- Keep exporting the old `BinaryDownloader` name for compatibility.
  Rejected because it would preserve a stale, unmounted runtime-manager path as
  though it were still authoritative.

## Invariants
- `index.ts` stays a thin export boundary.
- Exported components correspond to mounted or supported feature surfaces.
- Feature exports do not redefine backend-owned contracts locally.

## Revisit Triggers
- The LLM feature gains additional mounted panels that need their own public
  export surface.
- A reusable package-level feature boundary replaces this app-local directory.

## Dependencies
**Internal:** `src/components`, `src/services/LLMService`.
**External:** None beyond the dependencies already required by the exported
modules.

## Related ADRs
- None identified as of 2026-04-19.
- Reason: this directory is a local export boundary and does not change
  backend/runtime ownership.
- Revisit trigger: the feature surface becomes shared across multiple hosts or
  needs versioned compatibility guarantees.

## Usage Examples
```ts
import { ManagedRuntimePanel, ServerStatus } from './features/llm';
```

## API Consumer Contract
- Consumers should import feature-level LLM UI from this directory instead of
  hardcoding deeper component paths when a supported feature export exists.
- `ManagedRuntimePanel` is the mounted version-aware runtime-manager entry
  point. Consumers should not depend on removed or unmounted runtime-manager
  components.
- This directory does not provide backwards-compatibility guarantees for
  internal decomposition helpers that are intentionally not exported.

## Structured Producer Contract
- None identified as of 2026-04-19.
- Reason: this directory only re-exports feature modules and does not emit
  machine-consumed structured artifacts.
- Revisit trigger: the feature layer starts generating manifests or structured
  metadata for external consumers.
