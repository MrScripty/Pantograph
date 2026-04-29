# Stage 12 Coordination Ledger

## Current Status

Stage `12` is in progress.

The stage exists to make the execution-platform plan directory the canonical
home for the former `run-centric-gui-workbench` requirements. Initial source
implementation has started with the shell/store navigation slice.

## Required First Actions

1. Read `../../12-run-centric-workbench-consolidation.md`.
2. Re-check `../06-binding-projections-and-verification/coordination-ledger.md`
   for the reopened UniFFI/C# request-shape drift.
3. Re-check `../11-artifact-format-settings-and-managed-media-dependencies/coordination-ledger.md`
   for ArtifactStore/settings/media dependency status.
4. Confirm the selected implementation slice either depends on completed Stage
   `06`/`11` work or explicitly excludes affected binding/media surfaces.
5. Record the wave write sets before editing source.

## Open Decisions

- Whether Stage `12` implementation waits for all Stage `11` ArtifactStore and
  Settings work or starts with shell/pages that can render disabled/pending
  media surfaces.
- Exact verification command set for frontend accessibility and screenshot
  checks once the current frontend tooling is re-inspected.

## Resolved Decisions

- Historical review records from the former `run-centric-gui-workbench`
  directory moved under
  `docs/plans/pantograph-execution-platform/reviews/run-centric-workbench/`.
  Stage `12` must not depend on the old plan directory remaining present.

## Verification Ledger

- 2026-04-29 Wave `03` workbench shell navigation passed:
  `node --experimental-strip-types --test src/stores/workbenchStore.test.ts`,
  `npm run typecheck -- --pretty false`, and `npm run build`.
- 2026-04-29 Wave `05` frontend lint cleanup passed:
  `npm run lint:full`, `npm run typecheck -- --pretty false`,
  `npm run lint:a11y`, `npm run test:frontend`, and `npm run build`.
