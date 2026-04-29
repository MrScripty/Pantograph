# Stage 12 Coordination Ledger

## Current Status

Stage `12` is complete with follow-ups.

The stage exists to make the execution-platform plan directory the canonical
home for the former `run-centric-gui-workbench` requirements. Source audit,
shell/store navigation, frontend lint cleanup, and Settings retention ownership
are integrated. Stage-end refactor gate outcome is recorded as
`not_warranted`. GUI smoke coverage and remaining Stage `11` media work are
recorded follow-ups rather than hidden Stage `12` blockers.

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

- None.

## Resolved Decisions

- Historical review records from the former `run-centric-gui-workbench`
  directory moved under
  `docs/plans/pantograph-execution-platform/reviews/run-centric-workbench/`.
  Stage `12` must not depend on the old plan directory remaining present.
- Stage `12` does not wait for all Stage `11` follow-ups. Current workbench
  pages may use the implemented ArtifactStore/settings/media APIs while Stage
  `11` keeps producer-specific preview streams, diffusion child/revision
  artifacts, active converter/library version capture, and OCIO ABI validation
  open.
- Persistent diagnostics retention policy edits belong to the workbench
  Settings page. I/O Inspector may display policy/state and run cleanup, but it
  must not own persistent retention policy edits.
- GUI/screenshot verification is tracked by
  `docs/plans/pantograph-execution-platform/follow-up-gui-smoke-harness.md`
  because this repository currently has no browser smoke harness.

## Verification Ledger

- 2026-04-29 Wave `01` source audit and crosswalk recorded current
  dependencies, source-audit findings, verification evidence, and remaining
  blockers in
  `reports/wave-01-host-source-audit-and-crosswalk.md`.
- 2026-04-29 Wave `03` workbench shell navigation passed:
  `node --experimental-strip-types --test src/stores/workbenchStore.test.ts`,
  `npm run typecheck -- --pretty false`, and `npm run build`.
- 2026-04-29 Wave `05` frontend lint cleanup passed:
  `npm run lint:full`, `npm run typecheck -- --pretty false`,
  `npm run lint:a11y`, `npm run test:frontend`, and `npm run build`.
- 2026-04-29 Wave `06` Settings retention ownership passed:
  `node --experimental-strip-types --test src/components/workbench/settingsPagePresenters.test.ts src/components/workbench/ioInspectorPresenters.test.ts src/services/workflow/WorkflowService.commands.test.ts`,
  `npm run typecheck -- --pretty false`, `npm run lint:full`,
  `npm run lint:a11y`, `npm run build`, and `npm run traceability`.
- 2026-04-29 Wave `07` workbench DTO parity passed:
  `cargo test -p pantograph-workflow-service --test contract workbench_settings_network_cross_layer_fixture_deserializes`,
  `node --experimental-strip-types --test src/services/workflow/WorkflowService.projections.test.ts src/services/workflow/WorkflowService.commands.test.ts`,
  `npm run typecheck -- --pretty false`,
  `cargo test -p pantograph-workflow-service --test contract`,
  `npm run lint:full`, `cargo fmt --all -- --check`, and
  `npm run traceability`.
- 2026-04-29 Stage-end refactor gate outcome:
  `reports/stage-end-refactor-gate.md` records `not_warranted`.
