# Wave 02: Doc and Policy Alignment

## Goal
Align CI/testing/toolchain documentation with the final Ubuntu native bootstrap
behavior once the workflow change is validated.

## Ownership Slice

### worker-native-docs
- Assigned scope:
  - record the Ubuntu package prerequisites and their ownership in the CI docs
  - update the remediation ledger and final plan with observed CI results
- Expected output contract:
  - docs match the implemented workflow and runner prerequisites
- Primary write set:
  - `docs/testing-and-release-strategy.md`
  - `docs/toolchain-policy.md`
  - this remediation package
- Allowed adjacent write set:
  - none
- Read-only context:
  - `.github/workflows/quality-gates.yml`
- Forbidden/shared files:
  - workflow YAML after Wave 01 is verified
- External-change escalation rule:
  - if CI reveals that the package set is still incomplete, record that in the
    remediation package and defer doc finalization until the runner behavior is
    stable
- Worker report path:
  - `reports/wave-02-worker-native-docs.md`

## Required Verification
- inspect the post-change GitHub Actions run
- local review of the final workflow YAML and docs

## Cleanup Requirements
- preserve the worker report as the final rationale trail
