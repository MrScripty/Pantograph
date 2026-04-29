# Wave 01 Host Source Audit And Crosswalk

## Scope

Verified that the former `run-centric-gui-workbench` requirements and review
records are represented in the execution-platform plan path, then audited the
current codebase against the Stage `12` workbench boundaries.

## Dependency Status

- Stage `06` binding request-shape drift is repaired for supported workbench
  lanes. Python remains unsupported and BEAM remains experimental by policy,
  but the UniFFI/C# workbench-relevant surfaces are verified.
- Stage `11` has implemented the workbench-facing ArtifactStore, Settings,
  format capability, stream read, and managed media dependency APIs needed by
  current pages.
- Stage `11` remains open for producer-specific preview stream conversion,
  diffusion child/revision artifacts, active converter/library version capture,
  and OpenColorIO ABI validation.

## Source Audit Results

- GUI root defaults to `WorkbenchShell`, and `workbenchStore.ts` defaults the
  selected page to Scheduler.
- Active-run selection is an in-memory Svelte store value and is not persisted
  to local storage, session storage, IndexedDB, or backend state.
- Workbench pages consume workflow projection/command services instead of raw
  diagnostic event rows or page-load projection rebuilds.
- Workbench/service paths do not transport image, audio, video, 3D, or generic
  binary bodies through inline JSON. Legacy graph-node fallback paths still
  contain inline media compatibility handling and remain a Stage `11`/graph
  follow-up rather than a workbench projection path.
- Settings is the canonical owner for persistent ArtifactStore policy,
  diagnostics retention policy, artifact format defaults, managed media
  dependency controls, and embedded runtime/app settings.
- I/O Inspector now displays retention state and can apply cleanup, but it does
  not edit persistent retention policy.
- Network renders local-only status/projection facts and does not imply Iroh or
  distributed execution is implemented.
- Node Editor renders a truthful unavailable/reserved state with no authoring
  controls.

## Verification

```bash
cargo test -p pantograph-runtime-attribution
cargo test -p pantograph-node-contracts
cargo test -p pantograph-diagnostics-ledger
cargo test -p pantograph-workflow-service diagnostics
cargo test -p pantograph-workflow-service session_execution
cargo test -p pantograph-uniffi
npm run lint:full
npm run typecheck -- --pretty false
npm run lint:a11y
npm run test:frontend
npm run build
npm run traceability
cargo fmt --all -- --check
```

## Remaining Blockers

- Stage-end refactor gate for Stage `12` has not been recorded yet.
- GUI/screenshot verification remains pending because the repository does not
  currently provide a Playwright or equivalent harness.
- DTO parity is paired/manual rather than generated and is strongest for run
  list/detail fixtures. Network, Settings, and remaining Stage `11` media
  surfaces should gain stronger shared-fixture or generated parity before they
  are treated as complete external contracts.
- Legacy graph/editor media fallback paths still contain inline base64/data-url
  compatibility handling outside workbench projection reads.
