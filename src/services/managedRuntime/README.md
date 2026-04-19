# src/services/managedRuntime

## Purpose
This service boundary projects the backend-owned managed-runtime manager
contract into the GUI without moving runtime lifecycle policy into Svelte
components.

## Scope
- List managed runtimes.
- Inspect a single managed runtime.
- Start or resume managed runtime installation with progress callbacks.
- Request cancellation for an active managed runtime install job.
- Remove a managed runtime.
- Update selected and default runtime versions.
- Surface backend-owned retained download-artifact facts for read-only GUI
  status rendering.
- Surface backend-owned runtime-view snapshots alongside install progress so
  GUI progress updates remain synchronized to backend state transitions.

## Constraints
- This layer stays transport-only.
- Runtime readiness, selection, versioning, and install-state policy remain in
  Rust.
- Components may keep presentation state locally, but they should consume the
  managed-runtime contract from this service rather than redefining the payload.

## Invariants
- `ManagedRuntimeManagerRuntimeView` mirrors the backend manager snapshot.
- Progress callbacks surface backend download state unchanged.
- Progress callbacks also carry the backend-owned runtime snapshot used by the
  GUI, so components do not need separate ad hoc refresh logic to reconcile
  install-state transitions during managed-runtime jobs.
- UI callers should not call redistributable Tauri commands directly when this
  service already owns the app-facing contract.
- Retained artifact, resumability, and readiness facts remain backend-owned;
  this service only projects them into the GUI contract.
