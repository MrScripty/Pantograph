# Findings: Rust Workspace Bootstrap and External Dependencies

## Summary
Most failing Rust lanes in GitHub Actions are not exposing Rust code defects;
they are failing during workspace manifest resolution because the workspace
assumes a sibling checkout of `Pumas-Library` that CI does not materialize.

## Findings

### F01: Clean-checkout Rust CI cannot resolve the sibling `pumas-library` path dependency
- Affected files:
  - `Cargo.toml`
  - `.github/workflows/quality-gates.yml`
  - `scripts/check-rustler-beam-smoke.sh`
- Relevant code areas:
  - `[workspace.dependencies] pumas-library = { path = "../Pumas-Library/rust/crates/pumas-core" }`
  - every Rust CI job that only runs `actions/checkout` for Pantograph
- Evidence:
  - GitHub Actions run `24848578023` fails in `rust-check`, `rust-tests`,
    `rust-doc-tests`, `rust-format-audit`, `rust-clippy-audit`, and
    `rustler-beam-smoke` with
    `failed to read /home/runner/work/Pantograph/Pumas-Library/rust/crates/pumas-core/Cargo.toml`
    and `No such file or directory (os error 2)`.
- Standards constrained:
  - explicit tool/bootstrap ownership in CI
  - clean-checkout reproducibility
  - auditable CI/local verification equivalence
- Required remediation constraints:
  - CI must either materialize `../Pumas-Library` in the expected relative
    location for Rust jobs, or the workspace must stop requiring that relative
    path in clean-checkout lanes.
  - The chosen fix must preserve existing crate consumers and avoid ad hoc
    per-job path hacks that drift across lanes.
  - Any external repository checkout must pin the dependency source
    intentionally and document authentication expectations if the dependency is
    private.
- Alternatives that should remain rejected:
  - “Ignore the path dependency in CI”: rejected because the workspace cannot
    even load manifests.
  - “Treat failed audit lanes as irrelevant”: rejected because their failures
    are the same bootstrap defect and obscure whether audits are actually green.

### F02: Rustler smoke inherits the same unresolved workspace assumption
- Affected files:
  - `scripts/check-rustler-beam-smoke.sh`
  - `.github/workflows/quality-gates.yml`
- Relevant code areas:
  - `cargo build -p pantograph_rustler`
  - `rustler-beam-smoke` job bootstrap
- Evidence:
  - the smoke harness fails before BEAM execution begins because `cargo build`
    cannot resolve the sibling dependency tree.
- Standards constrained:
  - host-lane smoke reproducibility
  - smallest real-boundary acceptance path
- Required remediation constraints:
  - the smoke lane must share the same external dependency bootstrap strategy as
    the rest of the Rust workspace.
  - The fix should not special-case Rustler in a way that leaves other Rust
    jobs broken.

## Non-Blocking Context
- The local workspace currently passes these lanes only when the sibling
  dependency exists on disk.
- This is a CI contract failure first and a documentation failure second; the
  Rust code under test is not the primary issue in this run.
