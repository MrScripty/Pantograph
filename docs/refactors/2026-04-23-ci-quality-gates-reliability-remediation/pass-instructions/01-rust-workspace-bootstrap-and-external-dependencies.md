# Pass Instruction: Rust Workspace Bootstrap and External Dependencies

## Objective
Inspect the Rust CI lanes and workspace dependency ownership to determine why
clean-checkout GitHub Actions jobs cannot load the Rust workspace, and record
the constraints any remediation must preserve.

## Standards Focus
- CI and tooling bootstrap requirements
- clean-checkout reproducibility
- workspace dependency ownership
- facade-first compatibility for Rust workspace structure

## Assigned Code Areas
- `.github/workflows/quality-gates.yml`
- root `Cargo.toml`
- Rust crate manifests that inherit `pumas-library`
- `scripts/check-rustler-beam-smoke.sh`
- docs that define CI/local verification equivalence

## Inspection Requirements
- Keep the standards constraints explicit while reviewing the code and the CI
  failure output.
- Do not dismiss the sibling path dependency issue as “environment-specific”;
  treat clean-checkout CI reproducibility as a required contract.
- Record whether the current design assumes an adjacent checkout, a vendored
  dependency, or an optional feature gate, and what that implies for CI.
- Record unrelated issues separately if discovered, including fragile smoke
  harness assumptions or undocumented external repository coupling.

## Required Findings Content
- affected files
- relevant code areas
- violated or constraining standards
- exact CI failure mode
- remediation constraints
- alternatives that should remain rejected

## Output
Write findings to:

`docs/refactors/2026-04-23-ci-quality-gates-reliability-remediation/findings/01-rust-workspace-bootstrap-and-external-dependencies.md`
