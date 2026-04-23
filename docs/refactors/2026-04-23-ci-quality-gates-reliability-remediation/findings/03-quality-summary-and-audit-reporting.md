# Findings: Quality Summary and Audit Reporting

## Summary
The workflow summary currently misreports non-blocking audit lane failures as
successful because it reads `needs.<job>.result` from jobs marked
`continue-on-error: true`.

## Findings

### F01: The summary prints false-success audit results
- Affected files:
  - `.github/workflows/quality-gates.yml`
  - `docs/testing-and-release-strategy.md`
- Relevant code areas:
  - `rust-format-audit`
  - `rust-clippy-audit`
  - `quality-summary`
- Evidence:
  - In run `24848578023`, both audit jobs failed due to the unresolved Rust
    workspace bootstrap issue, but `Quality summary` printed
    `rust-format-audit: success` and `rust-clippy-audit: success`.
  - The workflow marks both jobs `continue-on-error: true`, then echoes
    `${{ needs.rust-format-audit.result }}` and
    `${{ needs.rust-clippy-audit.result }}`.
- Standards constrained:
  - truthful CI reporting
  - ratcheted audit visibility
  - auditable summary behavior
- Required remediation constraints:
  - the summary must report actual audit outcome separately from blocking-gate
    pass/fail semantics.
  - The fix must preserve non-blocking behavior for ratcheted audits unless a
    deliberate policy change is made elsewhere.
  - The summary must not imply green audits when the jobs actually failed.
- Alternatives that should remain rejected:
  - “Ignore audit result mismatches because they are non-blocking”: rejected
    because the summary is supposed to provide operator-facing status.
  - “Make audits blocking just to fix the summary”: rejected because it changes
    policy rather than fixing the reporting bug.

### F02: Current docs overstate summary trustworthiness under `continue-on-error`
- Affected files:
  - `docs/testing-and-release-strategy.md`
- Relevant code areas:
  - wording about the summary reporting audit job status for review
- Evidence:
  - the current run disproves that the summary accurately reports the audit job
    status when those jobs use `continue-on-error`.
- Standards constrained:
  - CI documentation equivalence with actual workflow behavior
- Required remediation constraints:
  - docs must be revised alongside the workflow fix so the described behavior
    matches the implementation.

## Non-Blocking Context
- `Quality summary` itself fails correctly because required lanes failed.
- The distinct problem is that its audit-reporting lines are misleading, which
  undermines operator trust during triage.
