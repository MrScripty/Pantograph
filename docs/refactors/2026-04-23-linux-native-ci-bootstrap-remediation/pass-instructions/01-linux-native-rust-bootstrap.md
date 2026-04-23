# Pass Instruction: Linux Native Rust Bootstrap

## Objective
Inspect the remaining Rust CI failures after the sibling `Pumas-Library`
bootstrap fix, determine which Linux system packages or workspace-scope choices
are now blocking the Rust lanes, and record the constraints any remediation
must preserve.

## Standards Focus
- explicit CI bootstrap for every tool and system dependency a lane invokes
- clean-runner reproducibility on GitHub-hosted Ubuntu
- truthful separation between workspace-scope policy and runner bootstrap

## Assigned Code Areas
- `.github/workflows/quality-gates.yml`
- root `Cargo.toml`
- any crate manifests or features that pull in Linux desktop/native GUI stacks
- docs describing CI/toolchain ownership

## Inspection Requirements
- Keep the standards constraints explicit while reviewing the workflow and the
  latest CI failure logs.
- Treat missing `glib-2.0.pc` and related native libraries as CI bootstrap
  defects first, not vague environment differences.
- Record whether the correct remediation is native package installation,
  workspace scope reduction, or a combined approach.
- Record unrelated issues separately if discovered, such as over-broad
  workspace checks or undocumented Ubuntu package assumptions.

## Required Findings Content
- affected files
- relevant code areas
- violated or constraining standards
- exact CI failure mode
- remediation constraints
- alternatives that should remain rejected

## Output
Write findings to:

`docs/refactors/2026-04-23-linux-native-ci-bootstrap-remediation/findings/01-linux-native-rust-bootstrap.md`
