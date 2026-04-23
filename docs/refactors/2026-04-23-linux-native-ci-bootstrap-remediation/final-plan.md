# Final Plan: Linux Native CI Bootstrap Remediation

## Objective
Restore the remaining failing Rust CI lanes by making the Ubuntu runner own the
native development packages required by the full Pantograph workspace checks and
audit jobs, then align the CI/toolchain documentation with that behavior.

## Scope

### In Scope
- Ubuntu native package bootstrap for `rust-check`
- Ubuntu native package bootstrap for `rust-clippy-audit`
- verification of whether additional GTK/WebKit desktop packages are required
- documentation updates directly tied to the implemented native bootstrap

### Out of Scope
- unrelated asset worktree changes under `assets/`
- reworking the sibling `Pumas-Library` bootstrap that is already functioning
- changing audit-summary semantics again
- broad workspace-scope redesign unless a re-plan is triggered

## Inputs

### Problem
After the earlier CI reliability fixes, GitHub Actions run `24850496670` still
fails in two places:
1. `Rust workspace check` fails during `cargo check --workspace --no-default-features`
   because `glib-sys` cannot find `glib-2.0.pc` on the Ubuntu runner.
2. `Rust clippy warning audit` fails for the same native-library reason during
   `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

The quality summary is now truthful; it is not the remaining problem.

### Constraints
- Preserve the current full-workspace check policy unless the bootstrap path
  proves unworkable and triggers a re-plan.
- Keep the remediation narrowly scoped to Ubuntu native package ownership and
  directly related docs.
- Preserve the already-working sibling dependency bootstrap and audit-result
  reporting.

### Assumptions
- The current workspace intentionally includes desktop-linked crates in the
  blocking `rust-check` lane.
- Installing `glib` development packages may not be sufficient; additional
  GTK/WebKit/Tauri-related packages may still be required on `ubuntu-latest`.
- The correct next step is workflow bootstrap first, not lane removal.

### Dependencies
- `.github/workflows/quality-gates.yml`
- `docs/testing-and-release-strategy.md`
- `docs/toolchain-policy.md`
- root `Cargo.toml`

### Risks
- The full native dependency set may be larger than the first missing package
  reveals.
- Over-fixing by adding a broad desktop package list without verification could
  hide unnecessary dependencies.
- If the runner image changes, the chosen package list could drift.

### Affected Structured Contracts
- GitHub Actions Ubuntu runner bootstrap contract for Rust lanes
- CI documentation for native prerequisites

### Affected Persisted Artifacts
- documentation and planning Markdown only

### Concurrency / Race Review
- No runtime concurrency change is planned.
- Workflow YAML remains a shared serialized surface and should have one owner
  during implementation.

### Definition of Done
- `rust-check` runs on GitHub-hosted Ubuntu without missing `glib-2.0.pc`
  failures.
- `rust-clippy-audit` runs on GitHub-hosted Ubuntu without the same native
  bootstrap failure.
- docs describing CI/toolchain ownership match the implemented native package
  bootstrap.

## Standards Groups Reviewed
- CI/native dependency bootstrap ownership
- clean-runner reproducibility
- truthful audit reporting follow-through
- CI/documentation equivalence

## Findings Grouped by Code Area

### Ubuntu Native Bootstrap
- the remaining Rust failures are now native Linux package failures, not Cargo
  manifest failures
- `glib-sys` is the first observed blocker on Ubuntu 24.04 runners
- additional GTK/WebKit packages may still be required after `glib`

### Audit Lane Follow-Through
- clippy audit reporting is already truthful
- the remaining clippy failure is the same native bootstrap issue as
  `rust-check`

### Unrelated Issues Recorded Separately
- unrelated dirty asset worktree changes remain outside remediation scope

## Overlapping Constraints and Unified Resolution
- `rust-check` and `rust-clippy-audit` should be fixed together because they
  compile overlapping desktop-linked workspace surfaces on the same Ubuntu
  runner class.
- documentation should only be finalized after the native package set is proven
  by a rerun, because `glib` may not be the last missing package.

## Milestones

### M1: Add Ubuntu Native Package Bootstrap
Goal: make the remaining Rust CI lanes capable of compiling their desktop-linked
workspace dependencies on Ubuntu.

Tasks:
1. Add the required Ubuntu package installation step(s) to the Rust workspace
   check and clippy audit jobs.
2. Reuse the same native bootstrap across any other Rust lane that compiles the
   same workspace surface if needed.
3. Keep the bootstrap explicit in workflow YAML rather than assuming runner
   defaults.

Affected files/areas:
- `.github/workflows/quality-gates.yml`

Standards satisfied:
- explicit native dependency bootstrap
- clean-runner reproducibility

Dependencies:
- none

Risks:
- additional missing packages may surface after the first bootstrap pass

Validation:
- `cargo check --workspace --no-default-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- follow-up GitHub Actions run on the updated workflow

### M2: Verify Follow-On Native Dependencies
Goal: confirm whether the first Ubuntu package set is sufficient or whether the
runner still lacks additional desktop/native libraries.

Tasks:
1. Inspect the next GitHub Actions run after M1.
2. If another native package is missing, update the package bootstrap and
   record the finding before proceeding.
3. Repeat until the lane fails for a real Rust/code issue or passes.

Affected files/areas:
- `.github/workflows/quality-gates.yml`
- remediation package ledger/report files

Standards satisfied:
- iterative clean-runner validation
- auditable re-plan trigger handling

Dependencies:
- M1 implemented

Risks:
- more than one follow-up CI iteration may be required

Validation:
- GitHub Actions rerun inspection

### M3: Align Docs With Final Native Bootstrap
Goal: make CI/testing/toolchain docs match the implemented Ubuntu prerequisites.

Tasks:
1. Update `docs/testing-and-release-strategy.md` with the native package
   prerequisite ownership for the Rust workspace lanes.
2. Update `docs/toolchain-policy.md` if the runner-owned Ubuntu packages are now
   part of the documented CI contract.
3. Record final observed runner results in the remediation package.

Affected files/areas:
- `docs/testing-and-release-strategy.md`
- `docs/toolchain-policy.md`
- this remediation package

Standards satisfied:
- CI/documentation equivalence
- auditable runner prerequisite ownership

Dependencies:
- M1 and M2 complete

Risks:
- docs will drift again if finalized before the runner bootstrap is stable

Validation:
- manual review against final workflow YAML
- post-change GitHub Actions run status

## Safe Parallel Waves

### Wave 01
Serial/shared-surface wave:
- `worker-native-ci-bootstrap`

Required reports:
- `reports/wave-01-worker-native-ci-bootstrap.md`

### Wave 02
Serial documentation wave:
- `worker-native-docs`

Required reports:
- `reports/wave-02-worker-native-docs.md`

## Ownership Slices
- Workflow/bootstrap owner:
  `.github/workflows/quality-gates.yml`
- Docs owner:
  `docs/testing-and-release-strategy.md`, `docs/toolchain-policy.md`, and this
  remediation package

## Risks and Mitigations
- Risk: package list is incomplete.
  Mitigation: require at least one follow-up CI inspection before finalizing
  docs.
- Risk: workspace-scope expectations are unclear.
  Mitigation: keep lane scope unchanged unless a re-plan trigger is hit.

## Re-Plan Triggers
- the required Ubuntu package set becomes large enough to challenge the current
  workspace-scope policy materially
- even after native package bootstrap, the runner still cannot support the
  intended desktop-linked workspace surface
- the appropriate fix becomes lane-scope reduction rather than bootstrap

## Completion Criteria
- `rust-check` no longer fails on missing native Ubuntu packages
- `rust-clippy-audit` no longer fails on the same native package gap
- docs describing CI/toolchain behavior match the implemented native bootstrap
