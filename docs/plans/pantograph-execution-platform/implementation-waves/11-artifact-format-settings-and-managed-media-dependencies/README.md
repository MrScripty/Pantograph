# Stage 11 Implementation Waves

## Purpose

This directory defines implementation-wave coordination for Stage `11`,
artifact format settings and managed media dependencies.

## Contents

| File | Description |
| ---- | ----------- |
| `README.md` | Stage `11` wave structure, constraints, and verification expectations. |
| `coordination-ledger.md` | Host-owned status record for open decisions, dependency state, and verification notes. |

## Stage Objective

Implement ArtifactStore-backed binary-safe media payload handling, persistent
artifact format settings, canonical workbench Settings ownership, managed
OCIO/ffmpeg/OIIO dependencies, and the required API/binding/frontend
projections.

## Default Execution Mode

Stage `11` is broad enough that implementation should start with a serial
contract/source-audit wave. Later waves may run in parallel only after the
contract owner records non-overlapping write sets and the coordination ledger
names integration order.

## Proposed Waves

1. `preflight-contract-audit`
   - Apply `08-stage-start-implementation-gate.md`.
   - Audit current base64/data-url media paths.
   - Freeze ArtifactStore, settings, capability, and managed redistributable
     DTOs before source implementation.
2. `artifact-store-backend`
   - Artifact descriptors, physical payload storage, lifecycle, retention,
     streaming, consume acknowledgement, and cleanup.
3. `managed-redistributables`
   - Generalized/split redistributables boundary, OCIO managed dependency,
     ffmpeg/`ocioconvert`/`oiiotool` managed binaries, supply-chain validation,
     activation, and leases.
4. `execution-diagnostics-cutover`
   - Workflow output descriptor cutover, streaming artifact lifecycle events,
     format metadata capture, and base64/data-url producer migration.
5. `api-bindings-frontend-settings`
   - API/binding DTO projection, binary-safe read/stream surfaces, workbench
     Settings page canonicalization, output-node selectors, and frontend tests.

## Coordination Rules

- Do not start implementation before the preflight/source-audit wave records
  producer/consumer migration decisions.
- Do not run backend ArtifactStore and managed redistributables changes in
  parallel unless their shared contracts are frozen first.
- Do not run frontend Settings work before backend Settings/capability DTOs are
  available.
- Integrate one wave at a time and rerun affected verification before starting
  the next wave.

## Stage Verification

Use the verification list in
`../../11-artifact-format-settings-and-managed-media-dependencies.md`.
Every skipped command or unavailable managed dependency must be recorded in the
stage plan or this coordination ledger with residual risk.

## Problem

ArtifactStore, media conversion, managed dependencies, Settings ownership, and
API/binding/frontend projections cross several packages. This directory keeps
the first pass serial until contracts and migration decisions are frozen.

## Constraints

- Stage `11` must apply `../../08-stage-start-implementation-gate.md` before
  source edits.
- Backend DTOs and settings contracts must be frozen before frontend Settings
  work.
- Artifact bodies must not be transported as inline JSON.
- Managed OCIO/ffmpeg/OIIO work must not depend on unmanaged host PATH or
  system-library discovery.

## Decision

Start Stage `11` with a serial preflight/source-audit wave. Later waves may be
parallelized only after the coordination ledger records non-overlapping write
sets and integration order.

## Alternatives Rejected

- Start frontend Settings and media selectors first: rejected because backend
  capability DTOs and ArtifactStore descriptors must define the option space.
- Treat OCIO, ffmpeg, and OIIO as system prerequisites: rejected because the
  plan requires managed dependency installation, activation, and auditability.

## Invariants

- ArtifactStore owns physical payload bodies and storage-tier opacity.
- Workbench Settings is the only persistent global settings owner.
- Managed redistributables keep runtime sidecars, tool binaries, and native
  library artifacts as distinct product categories.

## Revisit Triggers

- Artifact payload handling requires inline JSON, raw paths, or frontend-visible
  storage tiers.
- Managed media dependencies cannot be activated without unmanaged host
  discovery.
- Existing settings surfaces cannot be relocated or embedded without preserving
  duplicate global ownership.

## Dependencies

**Internal:** `../../11-artifact-format-settings-and-managed-media-dependencies.md`,
`../../08-stage-start-implementation-gate.md`,
`../../10-concurrent-phased-implementation.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `None identified as of 2026-04-29.`
- `Reason: Stage 11 is still a planned extension; architecture decisions must
  be promoted to ADRs when implementation fixes crate ownership or public
  contracts.`
- `Revisit trigger: ArtifactStore, managed redistributable, or Settings
  ownership lands in source code.`

## Usage Examples

Read this file and `coordination-ledger.md` after the Stage `11` plan, then
record the preflight/source-audit result before assigning implementation waves.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume them as Stage `11` wave sequencing, dependency, and
  verification instructions.

## Structured Producer Contract

- The coordination ledger records stage status, open decisions, resolved
  decisions, and verification notes.
- Any future wave files must use stable slug names and define objective, write
  sets, forbidden files, verification, report paths, and integration order.
