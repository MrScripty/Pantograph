# ADR-003: Rust Workspace Metadata And Lint Policy

## Status
Accepted

## Context
The updated Rust standards require explicit package metadata, publish control,
workspace lint inheritance, and an unsafe-code policy. Pantograph had Rust
workspace members with repeated package metadata, no `rust-version`, no shared
repository metadata, no explicit `publish = false`, and no root
`[workspace.lints]` policy.

The workspace also has known warning debt. Cargo checks pass, but `clippy -D
warnings` and `cargo fmt --all -- --check` are not yet appropriate as blocking
gates without a separate cleanup pass.

## Decision
Adopt a root Rust package and lint policy:

1. Shared package metadata lives in `[workspace.package]`.
- Reusable local crates inherit `version`, `edition`, `rust-version`,
  `license`, and `repository`.
- The Tauri application keeps its product version but inherits the shared
  toolchain, license, and repository metadata.

2. Rust workspace members are not published to crates.io.
- Every current workspace member declares `publish = false`.
- A future crate can become publishable only after a packaging review updates
  this ADR or supersedes it.

3. Every Rust workspace member opts into `[workspace.lints]`.
- Repo-owned unsafe code is denied by default.
- Clippy policy starts with a narrow ratchet for debug macros, TODOs, and
  unsafe documentation requirements.
- Existing dead-code and unused warnings remain a documented baseline until M7
  classifies them.

4. Unsafe exceptions require an explicit checklist.
- The checklist lives in `docs/rust-workspace-policy.md`.
- Exceptions must be scoped to the smallest possible module or item.
- Workspace-wide or crate-wide unsafe exceptions are not allowed.

## Consequences

### Positive
- Workspace package metadata becomes consistent and auditable.
- Accidental crates.io publishing is blocked for all current local crates.
- New repo-owned unsafe code fails by default.
- Future Rust lint hardening can ratchet from a documented baseline.

### Negative
- The workspace still emits known warnings until the M7 cleanup milestone.
- `cargo fmt --all -- --check` remains a separate cleanup slice because current
  formatting drift spans several modules.
- Current `rust-version` follows the active project toolchain and may need a
  deliberate MSRV review before public package publication.

## Compliance Mapping
- Rust package metadata and publish control.
- Rust API and release standards for Cargo feature and package contracts.
- Unsafe policy and exception documentation.
- Decision traceability for cross-crate manifest changes.

## Implementation Notes
- Root policy file: `docs/rust-workspace-policy.md`.
- Compliance plan tracking:
  `docs/standards-compliance-analysis/refactor-plan.md`.
- Warning cleanup remains owned by M7.
