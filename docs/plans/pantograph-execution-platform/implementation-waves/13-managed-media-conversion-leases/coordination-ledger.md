# Stage 13 Coordination Ledger

## Current Status

Stage `13` is planned and not yet started.

## Boundary Snapshot

- Stage `11` completed descriptor attribution, binary-safe ArtifactStore
  payload handling, managed media dependency scaffolding, active-version
  descriptor metadata, GUI/API/binding projections, and stream artifactization.
- Stage `13` owns real converter process execution and per-conversion
  active-version lease-token attribution.
- `pantograph-workflow-service` must remain host-agnostic; conversion process
  execution should live in a host-owned or neutral conversion boundary.
- Managed dependency state and leases currently live under `inference` managed
  redistributable/media dependency modules.
- Ambient active-version snapshots are not sufficient for Stage `13`; actual
  conversion attribution must come from leases acquired immediately before
  converter invocation.

## Required First Actions

1. Apply `08-stage-start-implementation-gate.md`.
2. Inspect current dirty files and confirm Stage `13` write-set safety.
3. Re-read Stage `11` completion notes and Stage `13` plan.
4. Freeze conversion request/result/error and attribution field design.
5. Choose crate/module ownership for the conversion executor.
6. Record non-overlapping worker write sets before launching parallel workers.

## Proposed Worker Split

Do not launch these workers until Wave `01` freezes shared contracts and the
host records concrete write sets.

| Owner | Scope | Primary Write Set | Forbidden Shared Files | Report |
| ----- | ----- | ----------------- | ---------------------- | ------ |
| Conversion executor worker | Managed process invocation, path validation, timeout, cancellation, stderr truncation, and temp cleanup with fake process-runner tests. | To be assigned after Wave `01`. | ArtifactStore contract DTOs unless assigned, frontend files, generated bindings, lockfiles, `.pantograph/**`, `assets/**`. | `reports/wave-02-worker-conversion-executor.md` |
| Lease attribution worker | Active-version lease acquisition/release around conversion and descriptor/diagnostics attribution propagation. | To be assigned after Wave `01`. | Process execution helpers unless assigned, frontend files, generated bindings, lockfiles, `.pantograph/**`, `assets/**`. | `reports/wave-03-worker-lease-attribution.md` |
| Media fixture worker | Fixture/golden tests for image/audio/video/3D conversion and capability coverage. | To be assigned after Wave `01`. | Production contracts, frontend files, generated bindings, lockfiles, `.pantograph/**`, `assets/**`. | `reports/wave-04-worker-media-fixtures.md` |

## Verification Notes

- No verification has run for Stage `13` yet.
- First verification gate must include traceability after this scaffold lands:
  `npm run traceability`.

## Open Decisions

- Whether the conversion boundary is a new crate
  `crates/pantograph-media-conversion` or a neutral module in an existing host
  crate.
- Exact conversion attribution fields and whether lease tokens are stored as
  raw ids, redacted ids, or stable fingerprints.
- Whether unsupported 3D conversion starts as fail-closed capability behavior
  or includes a concrete managed tool in the first Stage `13` implementation
  wave.
