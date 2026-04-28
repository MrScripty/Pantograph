# Run-Centric GUI Workbench Plans

## Purpose

This directory contains the staged implementation plan set for the run-centric
Pantograph GUI workbench described in
`../../requirements/pantograph-gui-run-centric-workbench.md`.

The plan is split into dependency-ordered stages because the GUI depends on
backend-owned versioning, run snapshots, scheduler estimates, scheduler events,
retention state, Library/Pumas audit facts, and API projections. The frontend
should render those facts instead of reconstructing them locally.

## Contents

| File | Description |
| ---- | ----------- |
| `00-overview-and-boundaries.md` | Source documents, architecture boundaries, implementation order, and cross-cutting standards gates. |
| `01-workflow-identity-versioning-and-run-snapshots.md` | Stable workflow identity, semantic version/fingerprint enforcement, presentation revisions, immutable queued runs, and run audit snapshots. |
| `02-scheduler-estimates-events-and-control.md` | Pre-run scheduler estimates, durable scheduler events, model load/unload observability, delay reasons, and client/admin queue authority. |
| `03-diagnostics-retention-and-audit-ledgers.md` | Typed diagnostic event ledger, version-aware projections, global retroactive retention policy, I/O artifact metadata, and Pumas/Library audit records. |
| `04-api-projections-and-frontend-data-boundaries.md` | Backend-owned API projections for runs, estimates, events, graph versions, I/O artifacts, Library usage, and local Network state. |
| `05-app-shell-active-run-navigation.md` | Scheduler-first app shell, top-level pages, active-run context, no-active-run states, and cutover away from the current canvas/workflow toggle. |
| `06-run-centric-page-implementations.md` | Scheduler table, Diagnostics, Graph, I/O Inspector, Library, Network, and Node Lab page work. |
| `07-verification-rollout-and-refactor-gates.md` | Standards verification, staged rollout, worktree hygiene, refactor gates, and future implementation-wave expansion rules. |
| `diagnostic-event-ledger-architecture.md` | Accepted planning direction for typed diagnostic events, strict backend-owned writers, validation, retention, and rebuildable projections. |
| `reviews/architecture-requirements-against-current-code.md` | Investigation record mapping each staged plan to current code support, architectural gaps, and required ownership changes. |
| `reviews/architecture-compatibility-risk-review.md` | Compatibility and blast-radius review covering cross-system effects, standards-driven refactors, and regression controls. |
| `reviews/blast-radius-analysis.md` | Source blast-radius review by stage, including affected code areas, implementation-wave risks, and required source-audit gates. |
| `reviews/diagnostics-code-against-event-ledger.md` | Detailed source analysis comparing current diagnostics code to the planned typed event-ledger architecture. |
| `reviews/plan-continuity-review.md` | Consistency pass over the plan set, including corrected ordering, event ownership, and anti-pattern controls. |
| `reviews/requirements-coverage-review.md` | Requirement-by-requirement coverage review proving the staged plans satisfy the GUI workbench requirements and recording remaining open decisions. |

The numbered files and `diagnostic-event-ledger-architecture.md` are the
current implementation authority. Files under `reviews/` are analysis
snapshots that explain why the plans are shaped this way; use them for context,
not as parallel implementation plans.

## Problem

Pantograph currently has separate GUI surfaces for drawing-to-Svelte and graph
workflow views. The target product is an execution workbench where the
Scheduler page is the default entry point and every other page can present a
different view over the selected run.

That product shift requires backend and architecture changes before broad
frontend work can be reliable. Workflow versions, scheduler estimates, run
snapshots, typed diagnostic events, retention state, audit events, and local
system/node facts must be owned by backend contracts and durable stores where
appropriate.

## Constraints

- The requirement source is
  `../../requirements/pantograph-gui-run-centric-workbench.md`.
- Plans must follow
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`.
- Frontend state must follow backend-owned data rules from the coding standards
  and `src/README.md`.
- Scheduler authority remains backend-owned.
- Existing dirty user/generated files outside this directory must not be
  reverted or included in implementation commits.
- This plan set is the current implementation authority for the run-centric
  workbench effort. The numbered files record both planned scope and
  implementation progress.
- Backwards compatibility with existing saved workflow files, old run history,
  and old graph fingerprint semantics is not required. Implementation may
  delete, ignore, or regenerate old records when the clean contracts land.

## Decision

Use this directory as the planning home for the GUI workbench effort. Keep
numbered files in dependency order:

1. Establish durable workflow/run/version identity.
2. Add scheduler estimates and scheduler event observability.
3. Extend diagnostics, retention, and audit ledgers.
4. Project backend-owned facts through stable APIs.
5. Rebuild the app shell around active-run navigation.
6. Implement pages against the backend projections.
7. Verify, stage rollout, and apply refactor gates.

This order prevents the frontend from inventing placeholder scheduler,
retention, version, or audit truth.

Implementation has one important dependency refinement: the shared typed event
ledger envelope, append boundary, and validation rules must exist before any
durable scheduler event persistence is completed. Stage `02` may define
scheduler estimates, authority rules, and producer points first, but it must
not create a scheduler-specific event repository. If the shared ledger core is
not available, execute the Stage `03` ledger bootstrap before Stage `02`
durable event persistence.

Implementation has a second diagnostics refinement: projections are durable
materialized read models with event-sequence cursors. They are rebuildable from
the typed ledger for migrations, repair, projection-version changes, and tests,
but normal startup, page load, and API queries must not replay the full event
history. Hot projections stay current for page-critical run and scheduler
views; warm projections may expose catching-up state; cold rebuilds are
explicit maintenance paths.

## Alternatives Rejected

- Start with the frontend shell and mock missing backend facts.
  Rejected because the standards require backend-owned persistent facts to
  remain authoritative.
- Keep all work in one plan file.
  Rejected because the requirements span versioning, scheduler, diagnostics,
  APIs, frontend layout, and future networking.
- Treat Network and Node Lab as separate products now.
  Rejected because the toolbar and route slots need early ownership, while the
  full distributed/node-authoring implementations remain future work.

## Invariants

- The Scheduler page is the target default GUI landing page.
- Runs are the shared context object across Scheduler, Diagnostics, Graph, I/O
  Inspector, Library, and Network.
- Backend contracts own durable run, scheduler, diagnostics, retention, and
  audit facts.
- The frontend may own only transient UI state and active-run selection.
- Workflow identity/versioning and semantic-version/fingerprint enforcement
  must land before run-centric diagnostics are treated as reliable.
- The old overloaded graph fingerprint model must be replaced by explicit
  topology, execution, workflow-version, and presentation-revision contracts
  before downstream stages depend on run/version data.
- Scheduler estimates and events must exist before the Scheduler page claims to
  explain queue behavior.
- Diagnostics, scheduler events, I/O observations, retention changes, and
  Library/Pumas audits use the typed diagnostic event ledger pattern described
  in `diagnostic-event-ledger-architecture.md`.
- Ledger-derived page projections are materialized with projection versions and
  event-sequence cursors; full replay is not a normal GUI/API read path.
- Payload retention may expire, but audit metadata must remain queryable.

## Revisit Triggers

- The backend storage boundary changes from local SQLite/in-process services to
  a shared service.
- Workflow versioning requires an ADR before implementation can proceed.
- Iroh networking starts and needs a dedicated distributed-execution plan.
- Node Lab starts and needs an agent/runtime-authoring plan.
- The frontend migration becomes large enough to require parallel worker wave
  specs under this directory.

## Dependencies

**Internal:** `../../requirements/pantograph-gui-run-centric-workbench.md`,
`../../requirements/pantograph-client-sessions-buckets-model-license-diagnostics.md`,
`../../requirements/pantograph-node-system.md`,
`../pantograph-execution-platform/README.md`,
`../diagnostics-run-history-projection/plan.md`,
`../scheduler-only-workflow-execution/plan.md`.

**External:** `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`,
Pumas APIs, future Iroh integration.

## Related ADRs

- `../../adr/ADR-011-scheduler-only-workflow-execution.md`
- `../../adr/ADR-012-canonical-workflow-run-identity.md`
- `../../adr/ADR-008-durable-model-license-diagnostics-ledger.md`
- `../../adr/ADR-007-managed-runtime-observability-ownership.md`
- `Revisit trigger: add or update ADRs when implementation accepts workflow
  versioning, typed diagnostic event ledger ownership, retention policy
  ownership, or privileged GUI admin authority boundaries.`

## Implementation Entry Point

Start with `00-overview-and-boundaries.md`, then implement numbered stages in
order. Apply `07-verification-rollout-and-refactor-gates.md` before each stage
begins and again before moving to the next stage.

Do not begin source implementation while implementation files in the selected
stage write set are dirty unless the user explicitly allows those changes.

## Usage Examples

For a future implementation pass:

```text
README.md
00-overview-and-boundaries.md
07-verification-rollout-and-refactor-gates.md
01-workflow-identity-versioning-and-run-snapshots.md
```

## API Consumer Contract

- This directory does not expose runtime APIs.
- API expectations are planning inputs only until the matching implementation
  stages add concrete contracts and module documentation.
- Later implementation plans must update owning module READMEs and ADRs when
  runtime API behavior becomes stable.

## Structured Producer Contract

- Stable artifact category: numbered Markdown implementation plans.
- File numbers encode dependency order.
- These files are manually maintained and are not generated from schemas.
- Implementation status, deviations, and verification notes should be recorded
  in the relevant numbered file during execution.
