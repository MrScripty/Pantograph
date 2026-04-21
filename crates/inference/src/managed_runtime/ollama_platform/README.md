# managed_runtime/ollama_platform

Per-platform managed-runtime adapters for `Ollama`.

## Purpose
This directory owns platform-specific `Ollama` release assets, install
validation, archive copying, and command resolution. The boundary keeps
Ollama-specific packaging rules out of generic managed-runtime orchestration and
host adapters.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Shared `OllamaPlatform` trait, current-platform selection, recursive install copy helpers, and executable search. |
| `linux.rs` | Linux x86_64 release asset, validation, and command resolution behavior. |
| `windows.rs` | Windows x86_64 release asset, validation, and command resolution behavior. |
| `macos_arm64.rs` | macOS arm64 release asset, validation, and command resolution behavior. |
| `macos_x64.rs` | macOS x86_64 release asset, validation, and command resolution behavior. |

## Problem
Ollama may be available as a system command or as a Pantograph-managed
redistributable, and vendor archives vary by platform. Those rules need one
backend-owned implementation so launch behavior matches capability and
installation state.

## Constraints
- System `ollama` command precedence is owned by the managed-runtime
  definition.
- Platform modules are selected at compile time.
- Install copying must preserve files and executable permissions.
- Host adapters must use resolved backend commands instead of reconstructing
  launch paths.

## Decision
Keep Ollama platform behavior behind `OllamaPlatform`. The managed-runtime
definition delegates release asset, install, validation, and command resolution
to the current platform adapter while preserving system-command support.

## Alternatives Rejected
- Treat Ollama as only a system dependency: rejected because Pantograph also
  supports managed redistributable state.
- Put Ollama path detection in Tauri: rejected because backend runtime facts
  must remain consistent across hosts.

## Invariants
- `OLLAMA_RELEASE_TAG` remains the default managed release until an intentional
  runtime update changes it.
- Install validation and launch command resolution use the same executable
  expectations.
- Platform modules do not own selection, catalog, or user-action policy.
- System and managed runtime behavior remains explicit in backend facts.
- Static release asset names should be constructed directly instead of through
  formatting macros so platform metadata stays clippy-clean.

## Revisit Triggers
- Vendor archive layout changes.
- Managed Ollama support changes system-command precedence.
- A supported platform needs custom install finalization.

## Dependencies
**Internal:** parent managed-runtime contracts and definitions.

**External:** platform filesystem APIs and optional system `ollama` discovery
owned by the parent definition.

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
- Outputs: release asset metadata, missing-file lists, copied install trees,
  and `ResolvedCommand` values.
- Lifecycle: called during install validation and command resolution; no
  long-lived daemon is owned here.
- Errors: platform validation and command errors are returned as strings for
  managed-runtime error projection.
- Versioning: platform behavior is private, but release asset names and command
  resolution shape affect public managed-runtime contracts.

## Structured Producer Contract
- Stable fields: platform keys, archive names, executable names, missing file
  names, and resolved command paths feed machine-consumed runtime snapshots.
- Defaults: current platform selection is compile-target dependent.
- Enums and labels: platform keys and runtime ids are semantic labels.
- Ordering: copied entries and executable search are deterministic.
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
