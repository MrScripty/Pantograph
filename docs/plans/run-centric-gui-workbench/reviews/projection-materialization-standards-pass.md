# Projection Materialization Standards Pass

## Scope

Review the updated diagnostic projection strategy against the local coding,
architecture, testing, documentation, and plan standards after deciding that
rebuildable projections must be durable materialized read models with event
cursors.

## Standards Checked

- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/ARCHITECTURE-PATTERNS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TESTING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`

## Findings

- The materialized projection approach matches the backend-owned data and
  layered architecture standards because GUI/API readers consume backend-owned
  read models rather than raw event rows or frontend-rebuilt state.
- The plan now satisfies replay, recovery, and idempotency expectations by
  requiring `event_seq`, `projection_state`, `last_applied_event_seq`,
  projection versions, duplicate-application tests, and reopen recovery tests.
- The plan now treats projection rebuild as an explicit maintenance,
  migration, repair, projection-version, and test path. This avoids a
  standards risk where normal page-load behavior would hide an O(total events)
  replay behind user-facing API reads.
- The hot/warm/cold projection split gives lifecycle ownership enough shape to
  satisfy standards for stateful flows and background work. Warm drains still
  need a concrete owner, overlap prevention, shutdown behavior, and stale-state
  API contract during implementation.
- The event granularity rule is required for security and performance:
  producers must store bounded metadata and payload references, not stream
  chunks, token-by-token output, image/audio bytes, or raw artifact bodies.
- Existing transitional diagnostics filters added before the typed ledger
  bootstrap are acceptable only if Stage `03` either migrates them into
  materialized projections or documents them as intentionally temporary query
  paths during cutover.

## Required Plan Controls Added

- `diagnostic-event-ledger-architecture.md` now defines `event_seq`,
  `projection_state`, hot/warm/cold projection classes, terminal summary rows,
  bounded event granularity, and explicit rebuild semantics.
- Stage `03` now requires projection cursor storage, incremental projection
  application, warm drain ownership, terminal compact summaries, non-trivial
  event-count tests, idempotency tests, and no-full-replay startup/page-load
  tests.
- Stage `04` now requires API projections to read materialized read models and
  expose freshness/catching-up state for warm projections.
- Stage `07` now includes verification and source-audit gates for event
  cursors, projection state, materialized readers, and prevention of full
  replay in normal GUI/API paths.

## Result

The updated design direction is standards-compatible if implementation follows
the new gates. The main remaining risk is lifecycle ownership for warm
projection drains; Stage `03` must choose and document that owner before code
adds asynchronous or lazy projection maintenance.
