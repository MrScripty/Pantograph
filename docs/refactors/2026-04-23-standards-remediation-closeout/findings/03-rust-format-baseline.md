# Findings: Rust Formatting Baseline Restoration

## Summary
The formatter baseline is still not restored. The drift appears narrow, but it
directly contradicts the accepted restoration ADR.

## Findings

### F01: rustfmt still rewrites embedded-runtime imports
- Affected files:
  - `crates/pantograph-embedded-runtime/src/workflow_session_execution.rs`
- Relevant code areas:
  - top-level import ordering
- Evidence:
  - `cargo fmt --all -- --check` fails and prints a reorder diff for the import
    block in `workflow_session_execution.rs`
- Standards constrained:
  - formatter-as-source-of-truth baseline
  - verification baseline restoration
  - CI/local gate equivalence
- Required remediation constraints:
  - apply rustfmt to the file rather than encoding manual style variance
  - rerun the same formatter gate after the change
  - keep the restoration isolated from unrelated source edits
- Classification:
  - isolated formatting drift with release-gate impact

### F02: ADR-004 and the current tree are out of sync
- Affected files:
  - `docs/adr/ADR-004-verification-baseline-restoration.md`
  - `.github/workflows/quality-gates.yml`
- Relevant code areas:
  - baseline-restoration claims
  - non-blocking format audit status
- Evidence:
  - ADR-004 says the formatting baseline was restored, but `cargo fmt --all -- --check`
    still fails
- Standards constrained:
  - auditable refactor traceability
  - documented verification equivalence
- Required remediation constraints:
  - either finish the restoration slice or amend the documentation to state that
    restoration is still in progress
  - do not leave the repo claiming a restored baseline while the formatter is red
- Classification:
  - documentation/verification mismatch

## Unrelated Worktree Context
- Current dirty worktree entries are asset deletions/additions under `assets/`.
  They are outside this remediation scope and should remain excluded.
