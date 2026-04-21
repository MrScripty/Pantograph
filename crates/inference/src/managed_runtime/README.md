# managed_runtime

## Purpose

This directory owns Pantograph's backend-managed runtime binary boundary for
installable sidecar runtimes such as `llama.cpp` and `Ollama`. The boundary
exists so inference callers and host adapters can consume one Rust-owned source
of truth for managed runtime contracts, install validation, command resolution,
and transition coordination without moving runtime lifecycle policy into Tauri.

## Contents

| File/Folder | Description |
| ----------- | ----------- |
| `catalog.rs` | Backend-owned GitHub release catalog refresh and fallback projection for managed runtime versions installable on the current platform. |
| `contracts.rs` | Stable managed-runtime DTOs for capability, snapshot, version, selection, job, and low-level archive/command contracts shared across backend and host boundaries. |
| `definitions.rs` | Binary-definition registry that maps managed runtime ids onto runtime-specific release, validation, and command-resolution behavior. |
| `operations.rs` | Backend-owned orchestration for status reads, install/remove transitions, and command resolution. |
| `paths.rs` | Managed runtime root/path helpers plus shared argument and environment helpers used by platform adapters. |
| `state.rs` | Durable managed runtime catalog, selection, and interrupted-job reconciliation helpers for restart-safe state projection. |
| `llama_cpp_platform/` | Thin per-platform `llama.cpp` install/finalization/launch adapters kept behind the managed runtime boundary. |
| `ollama_platform/` | Thin per-platform `Ollama` install and command-resolution adapters behind the same backend contracts. |
| `managed_binaries/` | Reserved marker documenting that runtime binary artifacts must not be stored under `src/`. |

## Problem

Pantograph needs installable runtime binaries, but those binaries differ by
platform, packaging format, validation rules, and launch requirements. The
system also needs one backend-owned place to answer whether a managed runtime is
available and how to launch it, so workflow execution and host adapters do not
rebuild runtime state independently.

## Constraints

- Core runtime-management logic must remain in backend Rust, not Tauri.
- Linux x86_64 and Windows x86_64 are required; macOS remains best-effort.
- Platform differences must stay behind thin adapter files rather than leaking
  inline platform checks into orchestration logic.
- Install/remove transitions can overlap with workflow launch requests, so the
  backend must coordinate state safely and reconcile interrupted work on
  restart.
- Managed runtime contracts must stay additive because multiple hosts consume
  them.

## Residual Platform Limits

- Linux x86_64 and Windows x86_64 remain the required managed-runtime targets
  for verification and release confidence.
- macOS x86_64 and arm64 adapters exist for current managed runtimes, but they
  remain best-effort until Pantograph adds explicit bounded verification for
  those lanes.
- `llama.cpp` managed installs depend on vendor release artifacts matching the
  current platform key; unsupported targets should surface an unsupported
  runtime state instead of attempting a partial install flow.
- `Ollama` keeps system-command precedence when `ollama` is already available
  on the host. Managed install support is additive, not a promise that
  Pantograph always replaces a host-provided `Ollama` binary.
- Runtime-family-specific packaging differences still belong in backend
  definition/platform modules; if a future runtime needs materially different
  packaging or validation, document that as a new bounded limit rather than
  silently extending host adapters.

## Decision

Use a small managed-runtime module tree with explicit responsibility splits:
contracts, catalog refresh, definition lookup, orchestration, archive
extraction, and path helpers. Runtime-specific platform details remain in
`llama_cpp_platform/` and `ollama_platform/`, while `operations.rs` owns the
backend-facing transition and availability flow. This keeps Tauri as an
adapter that calls backend services instead of becoming the owner of install or
launch policy.

## Alternatives Rejected

- Keeping all managed runtime logic in one `mod.rs`: rejected because the file
  had already exceeded the decomposition threshold and mixed contracts,
  orchestration, extraction, and path helpers in one place.
- Moving runtime install/availability policy into Tauri commands: rejected
  because runtime state must remain backend-owned for workflow and scheduler
  safety.

## Invariants

- `ManagedBinaryId` remains the canonical backend identifier for installable
  sidecar runtimes owned here.
- Platform-specific install and command behavior lives behind adapter modules,
  not inline in orchestration code.
- Availability and command resolution use the same backend validation path so
  hosts do not drift from execution reality.
- Managed runtime transitions are serialized per runtime id before install or
  removal mutates the filesystem.
- Archive extraction path validation is centralized in `archive.rs` so runtime
  install flows do not each reinvent root-containment checks.
- This directory owns binary-management facts, not higher-level workflow
  readiness policy; workflow-service and runtime-registry layers may consume the
  facts but must not be bypassed by host-local rebuilds.
- Managed-runtime orchestration should avoid avoidable allocations and lazy
  option substitutions when standard-library `Path` and `Option` helpers express
  the same behavior directly.

## Revisit Triggers

- The backend-owned runtime redistributables manager boundary moves out of
  `crates/inference` or stops using `operations.rs` as the orchestration seam.
- A third managed runtime family needs materially different install or launch
  behavior that stresses the current definition registry.
- Archive validation or extraction needs stricter root-safe guarantees than the
  current helper structure provides.

## Dependencies

**Internal:** `crate::inference`, `llama_cpp_platform`, `ollama_platform`.
**External:** `reqwest`, `tokio`, `parking_lot`, `flate2`, `tar`, `zstd`,
`zip`, `uuid`, `once_cell`, `which`.

## Related ADRs

- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md`
- Reason: managed runtime capability and launch facts here feed higher
  backend-owned runtime-registry, workflow policy, and redistributables
  manager layers.
- Revisit trigger: a future ADR moves managed runtime ownership out of the
  inference crate.

## Usage Examples

```rust
use inference::{list_binary_capabilities, resolve_binary_command, ManagedBinaryId};
use std::path::Path;

fn inspect_runtime(app_data_dir: &Path) {
    let capabilities = list_binary_capabilities(app_data_dir).unwrap();
    let _command = resolve_binary_command(app_data_dir, ManagedBinaryId::LlamaCpp, &["--port", "8080"]);
    assert!(!capabilities.is_empty());
}
```

## API Consumer Contract

- Callers use the public managed-runtime functions exported from `inference`,
  not platform modules directly.
- `list_binary_capabilities()` and `binary_capability()` expose current managed
  availability plus allowed install/remove actions.
- `download_binary()` and `remove_binary()` serialize per-runtime filesystem
  mutations, persist backend-owned job/version/selection state, and surface
  progress/errors through backend-owned contracts.
- `refresh_managed_runtime_catalog()` and
  `refresh_managed_runtime_catalogs()` are the backend-owned path for release
  catalog refresh. Host adapters may request a refresh, but they must not
  scrape vendor release APIs or synthesize installable version rows locally.
- `select_managed_runtime_version()` and
  `set_default_managed_runtime_version()` are the backend-owned mutation path
  for version selection policy; host adapters must call them instead of
  mutating `state.json` or rebuilding selection rules locally.
- `resolve_binary_command()` returns the executable path, working directory,
  sanitized arguments, environment overrides, and optional pid-file path needed
  for host launchers. When a managed runtime has persisted selected/default/
  active version state, command resolution uses that backend-owned version
  policy and its recorded install root instead of rebuilding the launch target
  from Tauri-local assumptions.
- System-provided runtimes take precedence when a definition explicitly supports
  them, as `Ollama` currently does.

## Adding Another Managed Runtime

When Pantograph adds another managed runtime family, extend this boundary in the
following order:

1. Add the backend identifier and contract hooks first.
- Extend `ManagedBinaryId` and any additive managed-runtime contract fields
  needed for the new runtime family.
- Keep new fields additive so existing hosts and GUI views do not break.

2. Add a runtime definition before adding transport.
- Implement a new definition in `definitions.rs` that owns release-source,
  validation, version, and command-resolution behavior.
- Do not branch on runtime ids in Tauri or workflow code to recreate those
  rules.

3. Keep platform behavior behind a dedicated adapter directory.
- Add per-platform install/finalization/launch helpers under a runtime-specific
  platform module such as `foo_platform/`.
- Inline platform checks in orchestration code are not an acceptable shortcut.

4. Reuse durable state and selection policy.
- Persist versions, selected/default version state, retained artifacts, and
  install history through `state.rs` rather than inventing a runtime-local side
  file or Tauri cache.
- New runtimes must participate in the same restart reconciliation and
  selected-version validation flow used by existing managed runtimes.

5. Project Pantograph-facing views through backend-owned adapters.
- If Pantograph-specific GUI/workflow views are needed, extend the
  `pantograph-embedded-runtime` managed-runtime manager projection rather than
  creating a second host-local DTO path.
- Workflow readiness, diagnostics, and restore/reuse paths must consume that
  shared projection instead of resolving binaries independently.

6. Add host transport only after the backend contract exists.
- Tauri commands may expose list/install/remove/select/inspect operations for
  the new runtime family only after this backend boundary can answer them.
- Frontend services should keep using the shared managed-runtime service
  boundary instead of introducing runtime-specific GUI transport.

## Structured Producer Contract

- `ManagedBinaryCapability` is the stable machine-consumed payload for managed
  runtime availability, install state, and user-action affordances.
- `ManagedRuntimeSnapshot` is the additive broader runtime-manager contract for
  readiness, version, selection, and job-state projection. Current
  implementations now merge persisted installed versions with a backend-owned
  cached release catalog instead of leaving version rows as install-only facts.
- `ManagedRuntimeVersionStatus` now carries backend-owned compatibility facts
  for runtime key, platform key, install root, executable name, executable
  readiness, and catalog/installability state so execution-adjacent consumers
  do not infer those fields from host-local assumptions.
- `ManagedRuntimeCatalogVersion` is the backend-owned persisted catalog entry
  for one installable vendor release on the current platform, including the
  exact archive name and download URL selected by the backend definition layer.
- `ManagedBinaryInstallState` values are authoritative backend facts and remain
  append-only unless a coordinated breaking change is approved.
- `DownloadProgress` is the backend-owned progress payload surfaced to adapters;
  `done=true` only means the current transfer/install operation finished, not
  that higher-level workflow readiness policy has been evaluated.
- `state.json` under the managed runtime root is the persisted runtime-manager
  artifact for catalog versions, installed versions, selection state,
  interrupted-job reconciliation, and install history. Install/remove/catalog
  transitions mutate this file as part of the backend lifecycle flow, and
  unknown or missing files default to an empty state.
- Managed runtime installs are version-scoped under the runtime root, while
  command resolution keeps a legacy fallback path for older single-directory
  installs that predate versioned layout support.
- Projection reads such as capability and snapshot queries degrade to the
  legacy runtime root when persisted selection state is stale, while strict
  execution-time command resolution still rejects invalid selected-version
  state explicitly.
- Selection changes only become durable through the exported backend mutation
  functions, which validate installed versions before persisting new
  selected/default version state, reject non-ready versions, and append an
  install-history event for audit visibility.
- `ResolvedCommand` is the backend-produced launch contract for host adapters.
- `ArchiveKind` and `ReleaseAsset` are internal producer contracts used by
  runtime definitions and platform adapters; changes here must keep the adapter
  boundary coherent across all supported platforms.
