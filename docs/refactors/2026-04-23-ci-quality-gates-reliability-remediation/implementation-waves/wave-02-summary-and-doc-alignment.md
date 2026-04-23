# Wave 02: Summary and Documentation Alignment

## Goal
Make the quality summary truthful for non-blocking audit lanes and align the
written CI policy with the implemented bootstrap behavior.

## Ownership Slice

### worker-summary-and-docs
- Assigned scope:
  - fix audit result reporting in `quality-summary`
  - align CI/testing/toolchain docs with the implemented bootstrap behavior
- Expected output contract:
  - quality summary distinguishes required-gate failures from actual audit lane
    outcomes
  - docs no longer overstate audit reporting or CI prerequisites
- Primary write set:
  - `.github/workflows/quality-gates.yml`
  - `docs/testing-and-release-strategy.md`
  - `docs/toolchain-policy.md`
- Allowed adjacent write set:
  - remediation package Markdown under this refactor root
- Read-only context:
  - `scripts/check-decision-traceability.sh`
  - `Cargo.toml`
- Forbidden/shared files:
  - none; this wave is serial after Wave 01
- External-change escalation rule:
  - if truthful audit reporting requires a broader workflow architecture change,
    stop and record the incompatibility rather than partially rewriting summary
    semantics
- Worker report path:
  - `reports/wave-02-worker-summary-and-docs.md`

## Integration Sequence
1. Start from the verified Wave 01 integration commit.
2. Integrate the summary/reporting slice.
3. Run the wave verification set.

## Required Verification
- `gh run view` or equivalent inspection of a dry-run / subsequent CI result to
  confirm audit outcomes are reported truthfully
- local review of `.github/workflows/quality-gates.yml`
- targeted local commands for any docs-updated gates referenced in the text

## Cleanup Requirements
- Remove temporary worker worktrees after integration verification passes.
- Preserve the worker report as the audit-reporting rationale trail.
