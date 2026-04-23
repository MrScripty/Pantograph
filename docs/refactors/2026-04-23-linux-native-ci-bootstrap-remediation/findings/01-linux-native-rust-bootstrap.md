# Findings: Linux Native Rust Bootstrap

## Summary
After fixing the sibling `Pumas-Library` dependency, the remaining Rust CI
failure is now a clean-runner Ubuntu native-library bootstrap gap. The full
workspace check and clippy audit pull in crates that require `glib-2.0`
development metadata, but the workflow does not install those system packages.

## Findings

### F01: `rust-check` now fails on missing Ubuntu GLib development packages
- Affected files:
  - `.github/workflows/quality-gates.yml`
  - root `Cargo.toml`
- Relevant code areas:
  - `cargo check --workspace --no-default-features`
  - full workspace membership including desktop/Tauri crates
- Evidence:
  - GitHub Actions run `24850496670` fails `Rust workspace check` with
    `The system library glib-2.0 required by crate glib-sys was not found` and
    `glib-2.0.pc` missing from the runner.
- Standards constrained:
  - explicit CI bootstrap for native dependencies
  - clean-runner reproducibility
  - truthful CI/local equivalence
- Required remediation constraints:
  - the workflow must install the required Ubuntu development packages before
    the workspace check lane compiles desktop-linked crates, or the lane scope
    must be reduced deliberately and documented.
  - the fix must preserve the current intent of `cargo check --workspace` if
    that breadth remains policy.
  - package bootstrap should be centralized or clearly repeated only where
    necessary to avoid drift between Rust lanes.
- Alternatives that should remain rejected:
  - “Assume hosted runners have GTK/GLib dev packages”: rejected because the
    observed runner does not.
  - “Ignore `rust-check` because focused tests already pass”: rejected because
    the workspace check is a distinct blocking policy lane.

### F02: More Linux desktop packages may still be missing after `glib`
- Affected files:
  - `.github/workflows/quality-gates.yml`
  - docs that describe CI prerequisites
- Relevant code areas:
  - Rust workspace compilation of desktop/Tauri-linked crates on Ubuntu
- Evidence:
  - the current failure stops at the first missing package (`glib-2.0`), so it
    does not yet prove the rest of the GTK/WebKit stack is present.
- Standards constrained:
  - auditable CI bootstrap completeness
  - staged remediation and re-plan triggers
- Required remediation constraints:
  - the plan must expect at least one verification rerun after installing the
    first package set.
  - document the likely need to add the full Ubuntu dev package set used by the
    desktop compilation surface rather than treating `glib` as necessarily
    sufficient.

## Non-Blocking Context
- The sibling checkout fix is working: focused Rust tests, doc tests, and the
  Rustler smoke lane now pass in CI.
- This failure is a different layer of CI bootstrap, not a regression in the
  earlier fix.
