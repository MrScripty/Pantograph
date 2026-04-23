# lib_tests

## Purpose

This directory contains behavior-focused runtime-registry facade tests that are
included by `../lib_tests.rs`. The parent file remains a small test index while
these modules cover registry lifecycle, observation, reservation, retention,
warmup, reclaim, and admission behavior.

## Contents

| File | Description |
| ---- | ----------- |
| `admission.rs` | Admission budget rejection, dry-run acquire checks, and peak RAM/VRAM capacity accounting. |
| `lifecycle.rs` | Runtime registration, canonicalization, transition validation, and basic reservation lifecycle coverage. |
| `observations.rs` | Full and single-runtime observation reconciliation behavior. |
| `reclaim.rs` | Reclaim decisions for inactive, active, and keep-alive runtimes. |
| `reservations.rs` | Reservation owner reuse, deterministic eviction ordering, and owner-filtered eviction lookup. |
| `retention_warmup.rs` | Retention disposition and warmup/reuse disposition coverage. |

## Problem

The root runtime-registry facade test file combined several policy areas in
one large file. That made admission, observation, reservation, retention,
warmup, and reclaim changes harder to review independently and exceeded the
source-size threshold.

## Constraints

- Tests must continue exercising the public `RuntimeRegistry` facade.
- Behavior modules should stay grouped by policy area, not by incidental helper
  setup.
- Production registry policy stays in the crate modules, not in test-only
  fixtures.

## Decision

Keep `../lib_tests.rs` as a test index and move behavior coverage into focused
modules under this directory. Each module imports the parent test scope so the
coverage remains close to the public facade without duplicating registry setup
helpers.

## Alternatives Rejected

- Leaving all facade tests in `lib_tests.rs`: rejected because the file
  exceeded the large-file threshold and mixed policy areas.
- Splitting tests by individual production file names only: rejected because
  some behavior, such as retention and warmup, intentionally crosses multiple
  production contracts.

## Invariants

- Registry facade tests use public registry APIs wherever possible.
- Admission tests remain separate from eviction-order tests so resource capacity
  policy changes stay reviewable.
- Observation tests stay separate from reclaim tests because observations
  reconcile producer facts while reclaim produces host actions.

## Usage Examples

Run the runtime-registry facade tests through the crate test filter:

```bash
cargo test -p pantograph-runtime-registry tests
```

When adding a new registry policy behavior, place it in the module that matches
the policy decision being asserted.

## Revisit Triggers

- A new registry policy area needs enough coverage to justify a new behavior
  module.
- Shared test setup starts repeating across modules and should be extracted into
  a narrow fixture helper.
- Registry policy moves out of the public facade and requires a different test
  boundary.

## Dependencies

**Internal:** `RuntimeRegistry`, reservation, admission, retention, warmup,
observation, reclaim, and snapshot contracts from the parent crate.

**External:** Rust test harness.

## Related ADRs

- [../../../../../docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md](../../../../../docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md)
