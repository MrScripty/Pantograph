# ADR-014: Run-Centric Workbench Projection Boundary

## Status
Accepted

## Context
Pantograph is moving from two competing GUI modes into one Scheduler-first
workbench. The GUI needs to select a workflow run once and let Scheduler,
Diagnostics, Graph, I/O Inspector, Library, Network, and Node Editor pages show
facts for that selected run.

The same work also introduced a typed diagnostic event ledger and durable
materialized projections. Page-load code must not rebuild read models from raw
ledger rows because total event history will grow with runs, scheduler events,
I/O artifacts, retention changes, and Library/Pumas audits.

## Decision
The Pantograph GUI uses a run-centric workbench shell as the only root
workspace. Scheduler is the default page. Active run selection is transient
frontend state and is shared across workbench pages.

Workbench pages consume typed backend projection services, not raw diagnostic
ledger rows. Pages may keep local presentation state such as filters, sort
order, selected page, selected run, loading state, and visible panel mode. They
must not become sources of truth for scheduler status, diagnostics, Library
usage, retention state, historic graphs, or local Network facts.

Ledger-derived projections are durable materialized read models with
projection-version and event-sequence cursors. Rebuildability means projections
can be rebuilt for migration, repair, corruption recovery, or projection-version
changes. Normal Scheduler, Diagnostics, I/O, Library, and page-load reads use
stored projections and incremental cursor advancement.

I/O artifact ledger events store bounded metadata and payload references, not
raw workflow values. First-pass workflow input/output observations may record
role, node id, media type, size, content hash, retention state, and retention
reason while leaving value bodies outside the diagnostic event ledger.

Historic workflow graph viewing uses the run graph projection and renders it in
an isolated read-only surface. Historic graphs are not applied to the current
editable graph store.

## Consequences

### Positive
- Page navigation shares one run context without duplicating selection models.
- GUI pages can stay responsive as diagnostic event volume grows.
- Historic run inspection can show immutable version/presentation facts without
  mutating the current editor.
- Frontend tests can cover presenter logic without mocking raw ledger replay.

### Negative
- Pages cannot display richer scheduler, I/O, Library, or Network facts until
  backend projections expose typed fields for them.
- Some first-pass pages show projection summaries instead of deeply parsed
  event payloads.
- The old drawing/canvas feature remains as a retired feature boundary until a
  later product decision removes or rehomes it.

### Neutral
- Active run selection is intentionally not persisted across GUI restart.
- Pumas and Library mutation UI must wait for typed confirmed service methods
  so pages do not optimistically mutate backend-owned state.

## Guardrails
- Workbench pages must not query `diagnostic_events` directly.
- Workbench pages must not parse arbitrary `payload_json` for primary UI facts.
- Projection freshness must be displayed where a page depends on warm or
  cursor-backed read models.
- Full projection rebuild commands are maintenance/admin paths, not normal page
  refresh behavior.
- Scheduler remains the final authority for queue state, priority, cancellation,
  model load/unload, and run execution timing.
- Current editable graph state and historic run graph snapshots must remain
  separate.

## Implementation Notes
- Implementation plans:
  `docs/plans/run-centric-gui-workbench/03-diagnostics-retention-and-audit-ledgers.md`,
  `docs/plans/run-centric-gui-workbench/04-api-projections-and-frontend-data-boundaries.md`,
  `docs/plans/run-centric-gui-workbench/05-app-shell-active-run-navigation.md`,
  `docs/plans/run-centric-gui-workbench/06-run-centric-page-implementations.md`,
  and
  `docs/plans/run-centric-gui-workbench/07-verification-rollout-and-refactor-gates.md`.
- Related workflow versioning decision:
  `docs/adr/ADR-013-workflow-version-registry-and-run-snapshots.md`.
- Related scheduler ownership decision:
  `docs/adr/ADR-011-scheduler-only-workflow-execution.md`.
- Related workflow run identity decision:
  `docs/adr/ADR-012-canonical-workflow-run-identity.md`.
