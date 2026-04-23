# Findings: Audit Lane Follow-Through and Summary

## Summary
The quality summary is now truthful. The remaining clippy audit failure is a
real environment/bootstrap failure caused by the same missing Ubuntu native
packages that break `rust-check`, not a new reporting bug.

## Findings

### F01: `rust-clippy-audit` is failing for the same native package gap as `rust-check`
- Affected files:
  - `.github/workflows/quality-gates.yml`
- Relevant code areas:
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - audit result capture and summary output
- Evidence:
  - GitHub Actions run `24850496670` shows `glib-sys` failing in the clippy
    audit with the same missing `glib-2.0.pc` error.
  - `quality-summary` prints `rust-clippy-audit: failure`, which matches the
    actual job outcome.
- Standards constrained:
  - truthful audit reporting
  - explicit native bootstrap for audit lanes
- Required remediation constraints:
  - solve the native package bootstrap for the clippy lane rather than changing
    its reporting behavior again.
  - keep the explicit audit-result output path that already proved truthful in
    this run.

### F02: CI/toolchain docs now need to own Linux native package bootstrap
- Affected files:
  - `docs/testing-and-release-strategy.md`
  - `docs/toolchain-policy.md`
- Relevant code areas:
  - descriptions of Rust CI prerequisites and host-lane toolchains
- Evidence:
  - the workflow now depends not just on Rust and optional BEAM toolchains, but
    also on Ubuntu package bootstrap for desktop-linked Rust workspace lanes.
- Standards constrained:
  - documentation equivalence with actual workflow behavior
  - auditable CI prerequisites
- Required remediation constraints:
  - update docs in the same implementation slice that adds the Ubuntu package
    bootstrap.
  - distinguish language toolchains from OS package prerequisites clearly.

## Non-Blocking Context
- `rust-format-audit` now passes and is reported correctly.
- `quality-summary` is failing only because a blocking lane (`rust-check`) is
  still red, which is expected.
