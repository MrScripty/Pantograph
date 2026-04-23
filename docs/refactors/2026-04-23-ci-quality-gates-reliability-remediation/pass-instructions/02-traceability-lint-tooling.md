# Pass Instruction: Traceability Lint Tooling

## Objective
Inspect the no-new-debt lint lane and the decision-traceability script to
determine why CI fails before evaluating actual traceability content, and
record the constraints any remediation must preserve.

## Standards Focus
- explicit tool bootstrap for every CI job
- reliable lint gate execution in clean runners
- traceability enforcement without hidden host prerequisites

## Assigned Code Areas
- `.github/workflows/quality-gates.yml`
- `scripts/check-decision-traceability.sh`
- documentation that describes the no-new-debt gate

## Inspection Requirements
- Keep the standards constraints explicit while reviewing the code and the CI
  failure output.
- Do not treat the missing `rg` executable as a harmless runner quirk; it is a
  broken gate contract.
- Record whether the proper remediation is CI installation, script fallback, or
  a combined approach, and what each option implies for portability.
- Record unrelated issues separately if discovered, such as other undeclared
  shell-tool dependencies or non-portable script assumptions.

## Required Findings Content
- affected files
- relevant code areas
- violated or constraining standards
- exact CI failure mode
- remediation constraints
- alternatives that should remain rejected

## Output
Write findings to:

`docs/refactors/2026-04-23-ci-quality-gates-reliability-remediation/findings/02-traceability-lint-tooling.md`
