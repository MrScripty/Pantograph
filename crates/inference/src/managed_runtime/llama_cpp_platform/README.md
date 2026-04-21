# managed_runtime/llama_cpp_platform

Per-platform managed-runtime adapters for `llama.cpp`.

## Purpose
This directory owns platform-specific `llama.cpp` release assets, install
finalization, validation, and launch-command resolution. The boundary exists so
managed-runtime orchestration can stay generic while platform differences stay
behind narrow adapter traits.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Shared `llama.cpp` platform trait, current-platform selection, extraction copy helpers, and argument/env helpers. |
| `linux.rs` | Linux x86_64 release asset, validation, library path, CUDA, and command resolution behavior. |
| `windows.rs` | Windows x86_64 release asset, validation, and command resolution behavior. |
| `macos_arm64.rs` | macOS arm64 release asset, validation, and command resolution behavior. |
| `macos_x64.rs` | macOS x86_64 release asset, validation, and command resolution behavior. |

## Problem
`llama.cpp` release archives differ by platform, executable name, library
layout, and environment requirements. Duplicating those rules in orchestration
or host adapters would create drift between install validation and launch
behavior.

## Constraints
- Platform modules are selected at compile time.
- Archive extraction must copy only runtime-relevant binaries and libraries.
- Validation and command resolution must use the same platform expectations.
- Host adapters must not bypass this directory when launching managed
  `llama.cpp`.

## Decision
Keep `llama.cpp` platform behavior behind `LlamaPlatform`. The generic managed
runtime definition delegates release asset, install, validation, and command
resolution to the current platform adapter.

## Alternatives Rejected
- Branch on platform in `operations.rs`: rejected because orchestration should
  not own runtime-family packaging rules.
- Let Tauri resolve `llama.cpp` paths: rejected because backend command
  resolution is the source of execution truth.

## Invariants
- `LLAMA_CPP_RELEASE_TAG` remains the default release version until an
  intentional managed-runtime update changes it.
- Runtime library alias and environment handling stay inside platform helpers.
- CUDA-specific launch paths remain derived from command arguments and install
  layout, not adapter guesses.
- Missing binary/library names must be reported through backend validation.
- Archive copy destinations should pass borrowed path components directly to
  `Path::join` instead of allocating temporary strings.

## Revisit Triggers
- Vendor release asset naming changes.
- A supported platform needs materially different install finalization.
- Managed runtime launch moves to a supervised sidecar manager.

## Dependencies
**Internal:** parent managed-runtime contracts and path/env helpers.

**External:** platform filesystem APIs.

## Related ADRs
- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md`

## Usage Examples
These modules are private managed-runtime adapters:

```rust
let platform_key = current_platform_key();
let platform = current_platform();
```

## API Consumer Contract
- Inputs: extracted archive directories, install directories, and launch
  arguments supplied by managed-runtime orchestration.
- Outputs: release asset metadata, missing-file lists, install finalization,
  and `ResolvedCommand` values.
- Lifecycle: called during install validation and command resolution; no
  long-lived process is owned here.
- Errors: platform validation and command errors are returned as strings for
  managed-runtime error projection.
- Versioning: platform behavior is private, but release asset names and command
  resolution shape affect public managed-runtime contracts.

## Structured Producer Contract
- Stable fields: platform keys, archive names, executable names, missing file
  names, env overrides, and pid-file extraction feed machine-consumed runtime
  snapshots.
- Defaults: current platform selection is compile-target dependent.
- Enums and labels: platform keys and runtime ids are semantic labels.
- Ordering: missing file lists should remain deterministic where displayed.
- Compatibility: changing archive or executable naming affects install state
  and release packaging.
- Regeneration/migration: update managed-runtime catalog expectations, tests,
  release docs, and this README when platform rules change.

## Testing
```bash
cargo test -p inference managed_runtime
```

## Notes
- macOS adapters are best-effort until the release verification matrix adds
  bounded macOS managed-runtime checks.
