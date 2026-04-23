# Pass 02: Frontend Accessibility and No-New-Debt Gate

## Purpose
Inspect the remaining frontend standards-closeout gaps that keep
`npm run lint:no-new` red. Keep the standards prompt at
`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/prompts/full-codebase-standards-refactor.md`
prominent while reviewing this pass.

## Standards Focus
- Accessibility gate compliance
- Traceable local-vs-CI verification equivalence
- Comment and documentation standards for justified suppressions

## Code Areas to Inspect
- `packages/svelte-graph/src/components/WorkflowGraph.svelte`
- `scripts/check-svelte-a11y.mjs`
- `package.json`
- `.github/workflows/quality-gates.yml`

## Required Output
Write findings under `findings/02-frontend-a11y-gates.md`.

Each finding must include:
- affected files and relevant code areas
- violated or constraining standards
- required remediation constraints
- whether the fix is code, comment-policy, or tooling behavior

Record unrelated UI issues separately if they are not necessary for standards
closeout.
