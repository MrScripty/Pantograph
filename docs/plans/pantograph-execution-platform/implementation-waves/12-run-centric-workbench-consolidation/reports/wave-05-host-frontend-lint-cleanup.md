# Wave 05 Host Frontend Lint Cleanup

## Scope

Repaired frontend lint failures discovered while running the Stage `12`
workbench verification gate.

## Findings

- Output-node format selectors had unkeyed Svelte `{#each}` blocks.
- The artifact format settings query request DTO used an empty TypeScript
  interface that violates the shared lint gate.

## Changes

- Added stable keys to image, audio, and point-cloud output format selector
  loops.
- Replaced the empty artifact format settings query request interface with an
  explicit empty object type alias.

## Verification

```bash
npm run lint:full
npm run typecheck -- --pretty false
npm run lint:a11y
npm run test:frontend
npm run build
```
