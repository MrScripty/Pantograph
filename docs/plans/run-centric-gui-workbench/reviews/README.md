# Run-Centric Workbench Review Records

## Purpose

This directory contains analysis snapshots for the run-centric GUI workbench
plans. These files record codebase investigations, blast-radius reviews,
requirements coverage, and continuity checks that informed the numbered plans
in the parent directory.

The implementation authority remains in the parent directory:

- `../00-overview-and-boundaries.md`
- `../01-workflow-identity-versioning-and-run-snapshots.md`
- `../02-scheduler-estimates-events-and-control.md`
- `../03-diagnostics-retention-and-audit-ledgers.md`
- `../04-api-projections-and-frontend-data-boundaries.md`
- `../05-app-shell-active-run-navigation.md`
- `../06-run-centric-page-implementations.md`
- `../07-verification-rollout-and-refactor-gates.md`
- `../diagnostic-event-ledger-architecture.md`

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
