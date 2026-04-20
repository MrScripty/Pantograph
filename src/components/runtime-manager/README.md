# src/components/runtime-manager

## Purpose
This directory contains the dedicated Settings-side runtime-manager UI for
Pantograph-managed redistributables such as `llama.cpp` and `Ollama`. It exists
so version-aware runtime inspection, selection, install progress, and retained
artifact controls live in one mounted Settings surface instead of being split
between compact backend-picker widgets and unmounted one-off components.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `ManagedRuntimePanel.svelte` | Mounted Settings entry point that loads runtime-manager snapshots from the shared frontend service and renders one card per managed runtime. |
| `ManagedRuntimeCard.svelte` | Per-runtime coordinator that owns user actions and composes focused runtime summary, job, catalog, and activity panels. |
| `ManagedRuntimeCatalogPanel.svelte` | Version policy selectors plus a bounded, scrollable backend-owned available-version table. |
| `ManagedRuntimeJobPanel.svelte` | Prominent live job progress, transfer state, and retained-download controls for one managed runtime. |
| `ManagedRuntimeActivityPanel.svelte` | Install history, missing-file disclosure, and install/remove actions for one managed runtime. |

## Problem
Pantograph needs a user-facing runtime manager that can explain why a sidecar
runtime is unavailable, which versions are known to the backend, which version
is selected or active, which remote catalog versions are installable on the
current platform, and what install/download work is currently happening. The
mounted Settings view must keep active transfer progress visually primary and
keep version inventories readable even when a backend catalog exposes several
versions.
Before this directory existed, that information was split between a compact
backend selector and an unmounted hardcoded runtime component, leaving the
mounted Settings UI without the version-aware management surface the backend now
supports.

## Constraints
- Backend Rust remains the source of truth for runtime readiness, selection,
  job state, and history.
- These components may keep ephemeral UI state only; they must not own runtime
  lifecycle policy or persisted install state.
- The UI must remain keyboard reachable and use semantic controls for runtime
  actions and version policy updates.
- Version catalogs must remain readable inside the mounted Settings panel by
  using bounded scroll regions and horizontal table scrolling instead of
  unbounded card stacks or over-compressed columns.
- The mounted Settings flow still needs a compact backend selector alongside
  the richer runtime-manager surface.

## Decision
Create a dedicated runtime-manager component boundary under `src/components/`
and keep it focused on presentation over the shared
`src/services/managedRuntime` contract. `ManagedRuntimePanel.svelte` owns the
mounted Settings integration and service subscription, while
`ManagedRuntimeCard.svelte` renders one backend-owned runtime snapshot at a
time. This keeps the existing backend selector focused on backend switching and
prevents version-aware runtime management from accreting inside unrelated
server-shell components.

## Alternatives Rejected
- Extend `BackendSelector.svelte` until it also handled full runtime management.
  Rejected because that component already mixed backend switching with runtime
  lifecycle details and had exceeded the preferred size/ownership boundary.
- Keep the richer runtime UI as an unmounted `BinaryDownloader.svelte`.
  Rejected because it hardcoded `llama_cpp` and left the real Settings surface
  without the authoritative runtime-manager screen.

## Invariants
- Runtime cards render backend-owned `ManagedRuntimeManagerRuntimeView`
  snapshots without redefining install/readiness policy locally.
- Version selection and default policy updates always route through
  `managedRuntimeService`.
- The mounted runtime-manager view remains inside the existing Settings GUI and
  does not create a second standalone shell for redistributable management.
- Runtime actions use semantic buttons, labeled selects, and keyboard-reachable
  interaction paths.
- Catalog rows remain projection-only: version availability and installability
  come from the backend-managed runtime snapshot, not from local GUI guesses.
- Active transfers remain visually primary and expose live progress feedback
  without requiring the user to inspect secondary panels or truncated labels.

## Revisit Triggers
- The runtime manager needs filtering or paging because the managed runtime set
  grows beyond a few cards.
- Another host surface outside Settings needs the same runtime-manager
  presentation and justifies a reusable package-level extraction.

## Dependencies
**Internal:** `src/services/managedRuntime`, `src/components/ServerStatus.svelte`.
**External:** Svelte 5.

## Related ADRs
- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md` backend-owned
  runtime redistributables manager boundary.

## Usage Examples
```svelte
<ManagedRuntimePanel />
```

## API Consumer Contract
- `ManagedRuntimePanel.svelte` is a mounted Settings component and assumes the
  Pantograph app shell has initialized Tauri-backed services.
- `ManagedRuntimeCard.svelte` expects a fully shaped
  `ManagedRuntimeManagerRuntimeView` and routes all mutations through
  `managedRuntimeService`.
- Callers should not mutate runtime-manager state directly; they should render
  the panel or cards and let the backend/service layer own refresh and action
  sequencing.

## Structured Producer Contract
- None identified as of 2026-04-19.
- Reason: this directory renders backend-owned runtime snapshots but does not
  itself publish persisted machine-consumed artifacts.
- Revisit trigger: the runtime-manager UI starts generating exported manifests
  or saved runtime-management metadata.
