# pantograph-workflow-service/tests

Integration and contract tests for the workflow service public surface.

## Purpose
This directory verifies serialized workflow-service behavior from the
perspective of crate consumers. The tests live outside `src/` so they exercise
the public crate API used by Tauri, UniFFI, Rustler, and other host adapters.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `contract.rs` | Public contract snapshots for workflow run, capabilities, preflight, sessions, queues, traces, and scheduler diagnostics. |

## Problem
Workflow-service DTOs are consumed across frontend and native binding
boundaries. Unit tests alone can miss accidental public response-shape drift
because they often exercise private helpers directly.

## Constraints
- Tests must use public crate exports.
- Snapshot assertions should keep deterministic fields explicit.
- Test hosts should be small fakes that model service contracts, not alternate
  workflow engines.
- Contract drift must update adapters and docs in the same implementation
  slice.

## Decision
Keep public behavior coverage here as integration tests. These tests should
pin serialized service contracts and host-trait interactions that are observed
by external adapters.

## Alternatives Rejected
- Keep all coverage in module unit tests: rejected because private tests do not
  prove the public crate boundary used by adapters.
- Use generated golden files for every response: rejected for now because the
  current contract set is small enough to keep expected JSON inline.

## Invariants
- Tests must not depend on Tauri, UniFFI, Rustler, or frontend packages.
- Expected JSON should preserve semantic field names, enum labels, and ordering.
- Projection-state snapshots should preserve projection version changes because
  those versions trigger rebuild behavior in native diagnostics storage.
- Test hosts should return backend-shaped facts and avoid duplicating policy
  logic under test.
- Public diagnostics projection snapshots must include typed retention fields
  when those fields are observable through serialized service contracts.
- I/O artifact contract snapshots must include retention summary counts because
  the GUI depends on those counts for retention-completeness display.
- Retention cleanup contract snapshots must include cleanup counts and
  `last_event_seq` because the GUI uses the response to report backend-owned
  cleanup results without local artifact mutation.
- Run-list contract snapshots must include backend-owned facet records when
  those records are observable through serialized service contracts.
- Library usage query contract snapshots must include active-run
  `workflow_run_id` filters because the GUI highlights assets used by the
  selected run through the public diagnostics API.
- Library asset access audit contract snapshots must preserve typed operation,
  cache-status, source-instance, and event-sequence fields because adapters use
  this API instead of raw ledger writes for Pumas/Library actions.

## Revisit Triggers
- Contract snapshots become large enough to justify fixture files.
- Binding release tests need to consume the same expected payloads.
- A public DTO migration requires versioned contract expectations.

## Dependencies
**Internal:** `pantograph-workflow-service` public exports and test-only fake
host implementations.

**External:** `tokio`, `async-trait`, and `serde_json`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```bash
cargo test -p pantograph-workflow-service --test contract
```

## API Consumer Contract
- Inputs: public request DTOs, fake `WorkflowHost` implementations, and
  deterministic workflow ids.
- Outputs: public response DTOs serialized through `serde_json`.
- Lifecycle: tests construct a fresh service per contract scenario.
- Errors: negative cases should assert public `WorkflowServiceError` categories
  rather than private implementation messages unless the message is part of the
  contract.
- Versioning: changing expected JSON is a contract change and must be reviewed
  with adapter/binding consumers.

## Structured Producer Contract
- Stable fields: expected serialized response fields are machine-consumed by
  tests and represent public API behavior.
- Defaults: omitted request fields should prove service defaults explicitly
  when defaults are part of the contract.
- Enums and labels: expected strings for states, issue categories, and queue
  statuses are semantic contracts.
- Ordering: arrays in expected JSON should match service-defined ordering.
- Compatibility: test updates must accompany any intentional public response
  migration.
- Regeneration/migration: if fixtures are introduced, document the generation
  command and keep fixture updates in the same commit as code changes.

## Testing
```bash
cargo test -p pantograph-workflow-service --test contract
```

## Notes
- When a backend bug is found while updating these tests, record it in the
  standards issue register unless it blocks compilation.
