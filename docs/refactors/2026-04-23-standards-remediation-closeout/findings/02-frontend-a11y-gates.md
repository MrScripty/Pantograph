# Findings: Frontend Accessibility and No-New-Debt Gate

## Summary
Frontend behavior and tests are strong, but the no-new-debt gate still fails
because one a11y suppression comment does not satisfy the repo's own review
annotation rule.

## Findings

### F01: WorkflowGraph has an unpaired a11y-reviewed suppression
- Affected files:
  - `packages/svelte-graph/src/components/WorkflowGraph.svelte`
  - `scripts/check-svelte-a11y.mjs`
- Relevant code areas:
  - Svelte ignore comments around the focusable workflow container
- Evidence:
  - `npm run lint:no-new` fails with
    `reviewed-a11y-ignore` at
    `packages/svelte-graph/src/components/WorkflowGraph.svelte:809`
  - line 807 contains an `a11y-reviewed:` reason for the first ignore, but the
    second ignore is adjacent only to another ignore line, not to its own review
    rationale
- Standards constrained:
  - a11y gate compliance
  - comment standards for non-obvious exceptions
  - tooling/local verification trust
- Required remediation constraints:
  - keep the non-interactive container rationale explicit
  - make the review annotation layout satisfy the checker without weakening the
    gate
  - do not replace the rule with a broader suppression in tooling
- Classification:
  - standards-comment/layout violation, not UI logic failure

## Non-Blocking Context
- `npm run test:frontend` passed with 228 tests.
- `npm run typecheck` passed.
