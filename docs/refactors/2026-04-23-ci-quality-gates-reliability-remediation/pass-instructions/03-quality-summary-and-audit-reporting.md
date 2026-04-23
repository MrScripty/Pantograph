# Pass Instruction: Quality Summary and Audit Reporting

## Objective
Inspect the GitHub Actions summary logic and non-blocking audit lanes to
determine why the summary misreports failed audits as successful, and record
the constraints any remediation must preserve.

## Standards Focus
- truthful CI reporting
- distinction between blocking and ratcheted audit lanes
- auditable quality-gate summaries

## Assigned Code Areas
- `.github/workflows/quality-gates.yml`
- documentation that describes audit job semantics and summary behavior

## Inspection Requirements
- Keep the standards constraints explicit while reviewing the code and the CI
  failure output.
- Do not discard the audit-reporting bug because the jobs are non-blocking; the
  summary is an explicit operator-facing contract.
- Record how `continue-on-error` interacts with `needs.<job>.result` and what
  summary design would preserve truthful reporting without re-blocking the lane.
- Record unrelated issues separately if discovered, such as inconsistent audit
  promotion language in docs.

## Required Findings Content
- affected files
- relevant code areas
- violated or constraining standards
- exact CI failure mode
- remediation constraints
- alternatives that should remain rejected

## Output
Write findings to:

`docs/refactors/2026-04-23-ci-quality-gates-reliability-remediation/findings/03-quality-summary-and-audit-reporting.md`
