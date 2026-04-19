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
| `contracts.rs` | Stable managed-runtime DTOs for capability, snapshot, version, selection, job, and low-level archive/command contracts shared across backend and host boundaries. |
| `definitions.rs` | Binary-definition registry that maps managed runtime ids onto runtime-specific release, validation, and command-resolution behavior. |
| `operations.rs` | Backend-owned orchestration for status reads, install/remove transitions, and command resolution. |
| `paths.rs` | Managed runtime root/path helpers plus shared argument and environment helpers used by platform adapters. |
| `state.rs` | Durable managed runtime catalog, selection, and interrupted-job reconciliation helpers for restart-safe state projection. |
| `llama_cpp_platform/` | Thin per-platform `llama.cpp` install/finalization/launch adapters kept behind the managed runtime boundary. |
| `ollama_platform/` | Thin per-platform `Ollama` install and command-resolution adapters behind the same backend contracts. |

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

## Decision

Use a small managed-runtime module tree with explicit responsibility splits:
contracts, definition lookup, orchestration, archive extraction, and path
helpers. Runtime-specific platform details remain in `llama_cpp_platform/` and
`ollama_platform/`, while `operations.rs` owns the backend-facing transition and
availability flow. This keeps Tauri as an adapter that calls backend services
instead of becoming the owner of install or launch policy.

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
- This directory owns binary-management facts, not higher-level workflow
  readiness policy; workflow-service and runtime-registry layers may consume the
  facts but must not be bypassed by host-local rebuilds.

## Revisit Triggers

- Runtime redistributable work adds version catalogs, durable job state, or
  selected-version policy that no longer fits the current `operations.rs`
  boundary.
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
- Reason: managed runtime capability and launch facts here feed higher
  backend-owned runtime-registry and workflow policy layers.
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
  mutations and surface progress/errors through backend-owned contracts.
- `resolve_binary_command()` returns the executable path, working directory,
  sanitized arguments, environment overrides, and optional pid-file path needed
  for host launchers.
- System-provided runtimes take precedence when a definition explicitly supports
  them, as `Ollama` currently does.

## Structured Producer Contract

- `ManagedBinaryCapability` is the stable machine-consumed payload for managed
  runtime availability, install state, and user-action affordances.
- `ManagedRuntimeSnapshot` is the additive broader runtime-manager contract for
  readiness, version, selection, and job-state projection. Current
  implementations may still leave some version metadata sparse, but the
  contract is now backed by a durable state file rather than only ephemeral
  process memory.
- `ManagedBinaryInstallState` values are authoritative backend facts and remain
  append-only unless a coordinated breaking change is approved.
- `DownloadProgress` is the backend-owned progress payload surfaced to adapters;
  `done=true` only means the current transfer/install operation finished, not
  that higher-level workflow readiness policy has been evaluated.
- `state.json` under the managed runtime root is the persisted runtime-manager
  artifact for versions, selection state, interrupted-job reconciliation, and
  install history. Unknown or missing files default to an empty state.
- `ResolvedCommand` is the backend-produced launch contract for host adapters.
- `ArchiveKind` and `ReleaseAsset` are internal producer contracts used by
  runtime definitions and platform adapters; changes here must keep the adapter
  boundary coherent across all supported platforms.
