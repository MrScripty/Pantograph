# Stage 12 Implementation Waves

## Purpose

This directory defines implementation-wave coordination for Stage `12`, the
run-centric workbench consolidation.

## Contents

| File | Description |
| ---- | ----------- |
| `README.md` | Stage `12` wave structure, constraints, and verification expectations. |
| `coordination-ledger.md` | Host-owned status record for open decisions, moved review evidence, and verification notes. |

## Stage Objective

Implement the run-centric GUI workbench from the canonical execution-platform
plan in `../../12-run-centric-workbench-consolidation.md`.

## Default Execution Mode

Stage `12` started serially. Page implementation may proceed only after the
source-audit/crosswalk wave verifies that all former `run-centric-gui-workbench`
requirements are covered by execution-platform tasks and that Stage `06`/Stage
`11` dependencies are either complete or explicitly out of scope for the slice.

## Proposed Waves

1. Source audit and crosswalk closure. Completed by
   `reports/wave-01-host-source-audit-and-crosswalk.md`.
   - Verify every run-centric source and review finding maps to an
     execution-platform task.
   - Record any obsolete source requirement with reason.
2. Backend projection and service readiness.
   - Close remaining run/scheduler/diagnostics/I/O/Library/Network/Settings DTO
     gaps before frontend pages consume them.
3. Workbench shell and active-run navigation. Initial shell/store slice
   completed by `reports/wave-03-host-workbench-shell-navigation.md`.
   - Replace old root mode navigation with Scheduler-default page shell and
     transient active-run state.
4. Page implementation.
   - Implement Scheduler, Diagnostics, Graph, I/O Inspector, Library, Network,
     Node Editor, and Settings pages against backend projections.
5. Frontend service/type parity.
   - Verify services, stores, presenter tests, DTO validation, and explicit
     backend error preservation.
6. Rollout, accessibility, source audits, and refactor gate.
   - Run GUI accessibility/interaction checks, source audits, and
     `09-stage-end-refactor-gate.md`.

## Coordination Rules

- Do not let page components query raw diagnostic events for normal user flows.
- Do not rebuild projections during normal page load or navigation.
- Do not add a second persistent settings owner.
- Do not pass media payload bodies through inline JSON.
- Do not infer backend format/dependency capabilities in frontend code.
- Do not overclaim Network or Node Editor features that remain future work.

## Stage Verification

- Backend projection tests for any touched services.
- Frontend unit/type/lint checks for workbench stores, services, presenters, and
  pages.
- GUI checks for Scheduler default route, navigation, active-run propagation,
  no-active-run states, artifact preview lifecycle, Settings ownership, and
  accessibility.
- Source audits for raw event reads, full-replay page reads, inline media JSON,
  duplicate settings owners, host PATH media dependency probing, and old root
  navigation ambiguity.

## Problem

The workbench crosses backend projections, frontend services, app-shell
navigation, page implementation, Settings ownership, ArtifactStore usage, and
accessibility. This directory prevents page work from starting before the
source-audit and dependency checks prove the canonical plan is ready.

## Constraints

- Stage `12` must apply `../../08-stage-start-implementation-gate.md` before
  source edits.
- Stage `06` binding verification and Stage `11` ArtifactStore/settings/media
  dependencies must be complete or explicitly excluded from the selected slice.
- Workbench pages must consume backend materialized projections, not raw event
  rows or frontend-rebuilt state.
- Active-run state remains frontend-transient.

## Decision

Start Stage `12` serially with a source-audit/crosswalk wave. Backend
projection readiness comes before shell/page implementation, and verification
closes with source audits plus the stage-end refactor gate.

Current status: source audit, shell navigation, frontend lint cleanup, and
Settings retention ownership slices are integrated. Stage-end refactor gate and
remaining parity/media follow-ups remain open.

## Alternatives Rejected

- Keep the retired run-centric plan directory as a live dependency: rejected
  because execution-platform is the canonical plan path.
- Implement the page shell first: rejected because page data boundaries and
  Stage `06`/Stage `11` dependency state must be known first.

## Invariants

- Scheduler is the GUI landing page.
- Active-run selection is not persisted.
- Settings is the only persistent global settings owner.
- Artifact previews use ArtifactStore APIs rather than inline media JSON.

## Revisit Triggers

- A former run-centric requirement is not represented by Stage `12`, Stage
  `11`, or completed execution-platform stages.
- Frontend implementation requires raw event reads or page-load projection
  replay.
- Network/Iroh or Node Editor authoring becomes active feature scope.

## Dependencies

**Internal:** `../../12-run-centric-workbench-consolidation.md`,
`../../11-artifact-format-settings-and-managed-media-dependencies.md`,
`../../06-binding-projections-and-verification.md`,
`../../reviews/run-centric-workbench/`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../adr/ADR-014-run-centric-workbench-projection-boundary.md`
- `Reason: ADR-014 records the workbench projection boundary that Stage 12 must
  preserve during implementation.`
- `Revisit trigger: Stage 12 changes projection ownership, active-run
  persistence, or page/backend state boundaries.`

## Usage Examples

Read this file and `coordination-ledger.md` after the Stage `12` plan. Record
the source-audit/crosswalk result before shell or page implementation starts.

## API Consumer Contract

- These files are planning artifacts, not runtime APIs.
- Implementers consume them as Stage `12` wave sequencing, dependency, and
  verification instructions.

## Structured Producer Contract

- The coordination ledger records stage status, open decisions, resolved
  decisions, and verification notes.
- Any future wave files must use stable slug names and define objective, write
  sets, forbidden files, verification, report paths, and integration order.
