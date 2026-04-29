# Run-Centric Workbench Review Records

## Purpose

This directory contains analysis snapshots moved from the former run-centric
workbench review directory. These files record codebase investigations,
blast-radius reviews, requirements coverage, and continuity checks that
informed the execution-platform plans.

The implementation authority is now in the execution-platform directory:

- `../../00-overview-and-boundaries.md`
- `../../01-client-session-bucket-run-attribution.md`
- `../../02-node-contracts-and-discovery.md`
- `../../03-managed-runtime-observability.md`
- `../../04-model-license-diagnostics-ledger.md`
- `../../05-composition-factoring-and-migration.md`
- `../../06-binding-projections-and-verification.md`
- `../../07-standards-compliance-review.md`
- `../../11-artifact-format-settings-and-managed-media-dependencies.md`
- `../../12-run-centric-workbench-consolidation.md`

## Contents

| File | Description |
| ---- | ----------- |
| `architecture-requirements-against-current-code.md` | Investigation record mapping each staged plan to current code support, architectural gaps, and required ownership changes. |
| `architecture-compatibility-risk-review.md` | Compatibility and blast-radius review covering cross-system effects, standards-driven refactors, and regression controls. |
| `blast-radius-analysis.md` | Source blast-radius review by stage, including affected code areas, implementation-wave risks, and required source-audit gates. |
| `diagnostics-code-against-event-ledger.md` | Detailed source analysis comparing current diagnostics code to the planned typed event-ledger architecture. |
| `plan-continuity-review.md` | Consistency pass over the plan set, including corrected ordering, event ownership, and anti-pattern controls. |
| `projection-materialization-standards-pass.md` | Standards pass for the projection cursor/materialized read-model update. |
| `requirements-coverage-review.md` | Requirement-by-requirement coverage review proving the staged plans satisfy the GUI workbench requirements and recording remaining open decisions. |

## Usage

Use these files for rationale and historical context. If a review record and a
numbered plan disagree, update the numbered plan or open a new review pass; do
not treat review records as parallel implementation plans.

## Problem

The retired run-centric workbench plan directory contained useful codebase
analysis and risk review evidence. Deleting that directory without preserving
the review records would make Stage `12` harder to audit and would force later
implementers to rediscover current-code constraints.

## Constraints

- Review records are historical evidence, not authoritative implementation
  plans.
- Execution-platform Stage `12` is the canonical workbench plan.
- Review records may mention former source labels; current implementation must
  resolve conflicts in the numbered execution-platform plans.

## Decision

Keep the useful review records under execution-platform ownership in this
appendix directory, and point Stage `12` implementation at them for rationale,
source-audit terms, and requirement evidence.

## Alternatives Rejected

- Delete review records with the former plan directory: rejected because the
  diagnostics, projection, and blast-radius reviews contain implementation
  evidence not fully duplicated elsewhere.
- Keep the former plan directory as a live reference: rejected because the
  project now needs one canonical plan path.

## Invariants

- Numbered execution-platform plans are authoritative when they conflict with
  review records.
- Review records remain useful only as investigation evidence and audit
  context.
- Any newly discovered requirement gap must update Stage `12` or create a new
  review pass.

## Revisit Triggers

- Stage `12` implementation finds a review finding that is not represented in
  an execution-platform task.
- A review record becomes misleading after source implementation changes.
- A new workbench requirement changes projection, settings, artifact, or
  active-run ownership.

## Dependencies

**Internal:** `../../12-run-centric-workbench-consolidation.md`,
`../../11-artifact-format-settings-and-managed-media-dependencies.md`,
`../../04-model-license-diagnostics-ledger.md`, and
`../../../../requirements/pantograph-gui-run-centric-workbench.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.

## Related ADRs

- `../../../../adr/ADR-013-workflow-version-registry-and-run-snapshots.md`
- `../../../../adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples

Before implementing Stage `12`, read:

```text
../../12-run-centric-workbench-consolidation.md
diagnostics-code-against-event-ledger.md
blast-radius-analysis.md
requirements-coverage-review.md
```

## API Consumer Contract

- These files are not runtime APIs.
- Implementers consume them as rationale and source-audit evidence.
- Conflicts are resolved by updating the numbered execution-platform plan or
  opening a new review pass.

## Structured Producer Contract

- Review records are manually maintained Markdown investigation artifacts.
- New review records must include status, purpose, findings, risks or gaps, and
  verification notes.
- Review records must not become parallel implementation plans.
