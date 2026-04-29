# Stage 13 Implementation Waves

## Purpose

This directory defines implementation-wave coordination for Stage `13`, managed
media conversion leases.

## Contents

| File | Description |
| ---- | ----------- |
| `README.md` | Stage `13` wave structure, constraints, and verification expectations. |
| `coordination-ledger.md` | Host-owned status record for boundary decisions, worker splits, and verification notes. |
| `waves/` | Wave specs with write sets, forbidden files, verification, report paths, and integration order. |
| `reports/` | Worker and host reports generated during Stage `13` implementation. |

## Stage Objective

Implement real managed media conversion/transcoding through Pantograph-managed
dependencies and record per-conversion active-version lease attribution in
artifact descriptors and diagnostics.

## Default Execution Mode

Stage `13` starts serially. The host must freeze the conversion boundary,
attribution fields, ArtifactStore handoff rules, and process execution owner
before launching workers. Later waves may run in parallel only after the
coordination ledger records non-overlapping write sets.

## Proposed Waves

1. `wave-01-boundary-design`
   - Apply `08-stage-start-implementation-gate.md`.
   - Confirm the conversion boundary does not couple
     `pantograph-workflow-service` to host process execution or the `inference`
     crate.
   - Freeze conversion request/result/error and attribution field design.
2. `wave-02-conversion-executor`
   - Implement managed process invocation, path validation, timeout,
     cancellation, stderr truncation, and temporary-file cleanup using a fake
     process runner for deterministic tests.
3. `wave-03-lease-attribution`
   - Acquire/release active-version leases around conversion, record exact
     dependency attribution, and add activation/removal race tests.
4. `wave-04-media-type-coverage`
   - Add image, audio, video, and supported 3D conversion coverage with
     fixture or golden tests and capability validation.
5. `wave-05-api-gui-rollout`
   - Surface conversion status/failures through diagnostics and I/O Inspector
     artifact lifecycle fields, then apply the stage-end refactor gate.

## Coordination Rules

- Do not invoke system PATH tools or user-supplied executable paths.
- Do not run conversion while holding ArtifactStore locks across process
  execution.
- Do not advertise a converter capability until activated managed dependencies
  and executable behavior are verified.
- Integrate one worker output at a time and rerun affected verification before
  starting the next wave.

## Stage Verification

Use the verification list in
`../../13-managed-media-conversion-leases.md`. Every skipped command,
unavailable managed dependency, or platform-specific behavior must be recorded
in the stage plan or this coordination ledger with residual risk.

## Global Forbidden Files

Workers must not edit these files without host approval:

- `.pantograph/**`
- `assets/**`
- generated package output under `target/**`, `dist/**`, or `src/generated/**`
- root manifests or lockfiles unless their wave spec assigns ownership
- files owned by another active worker in the same wave

## Problem

Real media conversion spans host process execution, managed dependency leases,
ArtifactStore storage, diagnostics, and GUI/API projection. This directory
keeps the first pass serial until the shared boundary is explicit.

## Constraints

- Stage `13` must apply `../../08-stage-start-implementation-gate.md` before
  source edits.
- `pantograph-workflow-service` remains host-agnostic.
- Artifact bodies must not be transported as inline JSON.
- Conversion capabilities must be derived from managed dependency readiness,
  not frontend-hard-coded assumptions.

## Decision

Start Stage `13` with a serial boundary design wave. Later waves may be
parallelized only after the coordination ledger records non-overlapping write
sets and integration order.

## Alternatives Rejected

- Put process execution directly in `pantograph-workflow-service`: rejected
  because it would couple canonical workflow semantics to host dependency
  management and process spawning.
- Use unmanaged system `ffmpeg`, `oiiotool`, or `ocioconvert`: rejected because
  Pantograph requires managed dependency activation and auditable versions.

## Invariants

- Managed dependencies are the only allowed source of converter executables and
  OCIO/OIIO assets.
- Per-conversion attribution comes from leases acquired around the actual
  conversion, not ambient active-version snapshots.
- Conversion errors remain typed and bounded.

## Revisit Triggers

- A converter cannot run without unmanaged host PATH discovery.
- Conversion requires long-running worker services instead of bounded process
  invocation.
- ArtifactStore cannot provide private temporary inputs/outputs without a
  storage API refactor.

## Dependencies

**Internal:** `../../13-managed-media-conversion-leases.md`,
`../../08-stage-start-implementation-gate.md`,
`../../10-concurrent-phased-implementation.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `None identified as of 2026-04-29.`
- `Reason: Stage 13 is a planned extension; architecture decisions should be
  promoted to ADRs when implementation fixes crate ownership or public
  conversion contracts.`
- `Revisit trigger: The conversion boundary lands in source code or changes a
  public ArtifactStore/API contract.`

## Usage Examples

Read this file and `coordination-ledger.md` after the Stage `13` plan, then
record the boundary design result before assigning implementation waves.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume them as Stage `13` wave sequencing, dependency, and
  verification instructions.

## Structured Producer Contract

- The coordination ledger records stage status, open decisions, resolved
  decisions, and verification notes.
- Wave files must use stable slug names and define objective, write sets,
  forbidden files, verification, report paths, and integration order.
