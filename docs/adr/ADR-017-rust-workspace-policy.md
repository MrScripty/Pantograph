# ADR-017: Rust Workspace Metadata, Lint, And Dependency Policy

## Status
Accepted

## Context
The updated Rust standards require explicit package metadata, publish control,
workspace lint inheritance, and an unsafe-code policy. Pantograph had Rust
workspace members with repeated package metadata, no `rust-version`, no shared
repository metadata, no explicit `publish = false`, and no root
`[workspace.lints]` policy.

The workspace has carried warning and Clippy debt while these ownership rules
remained valid. Verification status is not part of this decision and is tracked
by the current standards audit and remediation portfolio.

The Rust dependency standards also require dependencies used by two or more
workspace members to inherit a shared version from `[workspace.dependencies]`
while each member still declares the dependencies it directly imports.

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
- Dead-code and unused warning cleanup history remains documented after M7
  brought the `cargo check` baseline to zero.

4. Unsafe exceptions require an explicit review.
- Exceptions must be scoped to the smallest possible module or item.
- Workspace-wide or crate-wide unsafe exceptions are not allowed.

5. Shared Rust dependency versions live in `[workspace.dependencies]`.
- Dependencies used by two or more workspace members inherit versions from the
  root manifest.
- Member manifests continue to declare direct dependency ownership.
- Optional feature ownership stays in member manifests even when the dependency
  version is inherited.

## Consequences

### Positive
- Workspace package metadata becomes consistent and auditable.
- Accidental crates.io publishing is blocked for all current local crates.
- New repo-owned unsafe code fails by default.
- Future Rust lint hardening can ratchet from a documented baseline.
- Repeated Rust dependency versions move to a single auditable root table
  without hiding which crate owns each import.

### Negative
- This ADR does not prove that current Cargo, formatting, or Clippy checks pass;
  those claims require current verification evidence.
- Current `rust-version` follows the active project toolchain and may need a
  deliberate MSRV review before public package publication.

## Compliance Mapping
- Rust package metadata and publish control.
- Rust API and release standards for Cargo feature and package contracts.
- Rust dependency standards for workspace dependency inheritance.
- Unsafe policy and exception documentation.
- Decision traceability for cross-crate manifest changes.

## Current References

- Workspace metadata and dependency authority: root `Cargo.toml`.
- Current evidence and gaps:
  `docs/audits/2026-09-03-current-standards/04-verification-and-tooling.md`.
- Remediation:
  `docs/plans/current-standards-remediation/verification-and-tooling/plan.md`.
