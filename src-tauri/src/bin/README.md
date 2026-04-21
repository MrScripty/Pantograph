# src-tauri/src/bin

Tauri crate helper binary boundary.

## Purpose
This directory owns helper binaries compiled from the Tauri crate for targeted
runtime probes and developer diagnostics.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `pumas_dependency_runtime_probe.rs` | Probe binary for dependency/runtime behavior involving Pumas-backed model assets. |

## Problem
Some runtime dependency behavior is easier to isolate outside the full desktop
app. Helper binaries need clear scope so they do not become hidden product
entrypoints.

## Constraints
- Helper binaries are diagnostic/developer tools.
- Product runtime behavior must remain in shared backend services.
- Probe outputs should stay understandable without being treated as public API.

## Decision
Keep focused helper binaries here and document each as diagnostic tooling over
existing backend paths.

## Alternatives Rejected
- Put probes in ad hoc local scripts only: rejected because checked-in binaries
  can compile against current Rust contracts.
- Treat probes as product commands: rejected because they lack desktop
  lifecycle and UI integration.

## Invariants
- Helper binaries must not own runtime policy.
- Probes should compile with the Tauri crate and fail clearly when assumptions
  drift.
- Probe behavior should stay scoped to the named runtime concern.

## Revisit Triggers
- Probe output becomes machine-consumed by CI.
- The binary graduates into a supported CLI.
- Runtime dependency probing moves into shared test fixtures.

## Dependencies
**Internal:** Tauri crate modules and backend runtime/dependency helpers.

**External:** Rust standard library and any runtime dependencies used by the
probe path.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin pumas_dependency_runtime_probe
```

## API Consumer Contract
- Inputs: command-line/environment configuration expected by the probe.
- Outputs: diagnostic stdout/stderr and process exit status.
- Lifecycle: helper binaries run once and exit.
- Errors: probe failures should return non-zero exit status with context.
- Versioning: diagnostic output is not stable unless CI begins parsing it.

## Structured Producer Contract
- Stable fields: the binary target name is consumed by Cargo.
- Defaults: probe defaults should be documented in the binary source.
- Enums and labels: exit status carries success/failure semantics.
- Ordering: stdout ordering follows probe execution order.
- Compatibility: CI adoption would require a stricter output contract.
- Regeneration/migration: update Cargo config, scripts, and this README if
  helper target names change.

## Testing
```bash
cargo check --manifest-path src-tauri/Cargo.toml --bins
```

## Notes
- Keep probes separate from product command handlers.
