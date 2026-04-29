# Wave 03 Host Workbench Shell Navigation

## Scope

Aligned the workbench shell/store navigation order with Stage `12` and repaired
stale active-run store coverage.

## Changes

- Workbench pages now follow the canonical pipeline order: Scheduler,
  Diagnostics, Graph, I/O Inspector, Library, Network, Node Editor, Settings.
- The shell outlet now follows the same page order as the store.
- Store tests now include Settings in the page order and explicitly cover that
  clearing active-run selection preserves the selected page.

## Verification

```bash
node --experimental-strip-types --test src/stores/workbenchStore.test.ts
npm run typecheck -- --pretty false
npm run build
```
