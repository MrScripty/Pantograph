# src/services/managedRuntime

## Purpose
This service boundary projects the backend-owned managed-runtime manager
contract into the GUI without moving runtime lifecycle policy into Svelte
components.

## Scope
- List managed runtimes.
- Inspect a single managed runtime.
- Start managed runtime installation with progress callbacks.
- Remove a managed runtime.
- Update selected and default runtime versions.

## Constraints
- This layer stays transport-only.
- Runtime readiness, selection, versioning, and install-state policy remain in
  Rust.
- Components may keep presentation state locally, but they should consume the
  managed-runtime contract from this service rather than redefining the payload.

## Invariants
- `ManagedRuntimeManagerRuntimeView` mirrors the backend manager snapshot.
- Progress callbacks surface backend download state unchanged.
- UI callers should not call redistributable Tauri commands directly when this
  service already owns the app-facing contract.
