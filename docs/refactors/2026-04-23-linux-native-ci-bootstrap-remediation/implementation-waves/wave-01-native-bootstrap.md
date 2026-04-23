# Wave 01: Native Bootstrap

## Goal
Restore the remaining Rust CI lanes by provisioning the required Ubuntu native
development packages for the full workspace check and audit jobs.

## Ownership Slice

### worker-native-ci-bootstrap
- Assigned scope:
  - add Ubuntu native package bootstrap for the Rust workspace check and clippy
    audit lanes
  - keep package installation aligned across any Rust lanes that compile the
    same desktop-linked workspace surface
- Expected output contract:
  - `rust-check` and `rust-clippy-audit` can compile far enough to test actual
    Rust code or lints on GitHub-hosted Ubuntu
- Primary write set:
  - `.github/workflows/quality-gates.yml`
- Allowed adjacent write set:
  - `docs/testing-and-release-strategy.md`
  - `docs/toolchain-policy.md`
- Read-only context:
  - root `Cargo.toml`
  - existing CI remediation package
- Forbidden/shared files:
  - no unrelated source or crate manifest changes unless a re-plan is recorded
- External-change escalation rule:
  - if Ubuntu package bootstrap is insufficient and the lane must be scoped
    differently, stop and record the re-plan trigger instead of silently
    weakening the workspace check
- Worker report path:
  - `reports/wave-01-worker-native-ci-bootstrap.md`

## Required Verification
- `cargo check --workspace --no-default-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- follow-up GitHub Actions run on the updated workflow

## Cleanup Requirements
- preserve the worker report with any additional Ubuntu package discoveries
- do not proceed to later waves until the post-change CI result is reviewed
