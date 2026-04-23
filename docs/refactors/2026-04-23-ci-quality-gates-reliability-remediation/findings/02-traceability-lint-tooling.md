# Findings: Traceability Lint Tooling

## Summary
The `lint:no-new` CI failure is not a traceability-content violation. The lane
fails because the decision-traceability script requires `rg`, but the CI job
does not install it and the script has no fallback path.

## Findings

### F01: The traceability gate depends on `rg` without CI bootstrap or fallback
- Affected files:
  - `scripts/check-decision-traceability.sh`
  - `.github/workflows/quality-gates.yml`
- Relevant code areas:
  - `if ! command -v rg >/dev/null 2>&1; then ... exit 1`
  - `lint-no-new` job setup
- Evidence:
  - GitHub Actions run `24848578023` fails `npm run lint:no-new` with
    `Missing required tool: rg`.
  - The lint job installs only Node and npm dependencies before invoking the
    shell script.
- Standards constrained:
  - every CI job must bootstrap the tools it invokes
  - gate reliability on clean runners
  - traceability enforcement must reflect repo state, not runner accidents
- Required remediation constraints:
  - the gate must become self-contained for CI, either by explicit installation
    of `ripgrep`, a standards-compliant fallback to another search tool, or a
    combined approach.
  - The chosen fix should preserve the script’s fast path with `rg` where
    available.
  - The fix must not silently weaken traceability semantics.
- Alternatives that should remain rejected:
  - “Ignore the failure because local environments have ripgrep”: rejected
    because CI is part of the enforcement contract.
  - “Remove the tool check entirely”: rejected because that hides the bootstrap
    problem rather than solving it.

### F02: The current gate contract is under-documented for non-Node tooling
- Affected files:
  - `docs/testing-and-release-strategy.md`
  - optionally `docs/toolchain-policy.md`
- Relevant code areas:
  - CI/local gate descriptions for `lint:no-new`
- Evidence:
  - the gate is documented as a Node-driven quality lane, but its actual shell
    script requires host tooling outside npm dependencies.
- Standards constrained:
  - accurate tooling ownership
  - auditable CI/local equivalence
- Required remediation constraints:
  - documentation should state any non-Node prerequisites or explicitly state
    that the workflow job provisions them.

## Non-Blocking Context
- `lint:critical` and the Svelte a11y sub-gate both passed in CI.
- The failure is isolated to the traceability shell script bootstrap contract.
