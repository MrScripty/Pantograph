# pantograph-workflow-service/src/workflow/tests

Behavior-focused tests for the workflow service facade.

## Purpose
This directory holds cohesive test modules extracted from the legacy
`workflow/tests.rs` module. The parent test module indexes fixture and behavior
modules, while child modules isolate behavior families so the service facade can
stay reviewable as more tests are split.

## Contents
| File | Description |
| ---- | ----------- |
| `contracts.rs` | Workflow DTO serialization and service error-envelope contract tests. |
| `fixtures.rs` | Re-export index for shared workflow test fixture families in `fixtures/`. |
| `fixtures/` | Shared workflow test hosts, runtime capabilities, scheduler diagnostics providers, and helper constructors split by fixture family. |
| `runtime_preflight.rs` | Runtime preflight matching tests for selected runtime precedence, fallback readiness, backend aliases, and selected-version readiness context. |
| `scheduler_snapshot.rs` | Scheduler snapshot facade tests covering workflow/edit-session snapshot shape, trace attribution, queue bypass, and ambiguous pending queue behavior. |
| `scheduler_snapshot_diagnostics.rs` | Scheduler snapshot diagnostics tests for admission details, runtime-registry provider merging, and runtime rebalance requirements. |
| `session_admission.rs` | Runtime capacity and runtime admission wait tests for queued session runs. |
| `session_capacity.rs` | Loaded runtime rebalance tests for host-selected unloads, affinity preservation, and shared resource reuse. |
| `session_capacity_limits.rs` | Session and loaded runtime capacity limit/error tests, including release after close and pinned loaded-runtime capacity details. |
| `session_execution.rs` | Workflow session create/run/close, run-option propagation, runtime retention-hint tests, and immutable run snapshot event coverage. |
| `session_queue.rs` | Session queue item metadata, admission outcome, warm reuse, queue-position, and starvation protection tests. |
| `session_runtime_preflight.rs` | Session runtime preflight cache invalidation and keep-alive preflight failure tests. |
| `session_runtime_state.rs` | Session runtime loaded-state invalidation tests. |
| `session_stale_cleanup.rs` | Stale session cleanup, session inspection, and stale cleanup worker lifecycle tests. |
| `workflow_capabilities.rs` | Workflow capability discovery and default capability derivation tests. |
| `workflow_io.rs` | Workflow I/O discovery and validation tests for bindable input/output nodes and port contracts. |
| `workflow_preflight.rs` | Workflow preflight facade tests for required inputs, target validation, technical-fit decisions, and override normalization. |
| `workflow_run.rs` | Private scheduler run implementation tests for host outputs, timeout cancellation, runtime readiness, input validation, and output-target enforcement. |
| parent `diagnostics` module | Diagnostics facade tests for projection query boundaries, including I/O artifact retention state and retention summary counts. |

## Problem
`workflow/tests.rs` remains too large to review efficiently. Moving every test
into standalone modules at once would risk unnecessary churn around shared
mocks, so behavior families need to be extracted incrementally.

## Constraints
- Preserve existing workflow facade behavior and test assertions.
- Keep shared fixture modules private to `workflow::tests`.
- Avoid production API changes while splitting test coverage.

## Decision
Use `workflow/tests/` for behavior-specific child modules under the parent
`workflow::tests` module. Child modules import parent test helpers with
`super::*`; the parent re-exports shared fixture families from
`workflow/tests/fixtures/`.

## Alternatives Rejected
- Move all tests into separate files immediately: rejected because shared host
  mocks and fixture setup would need a larger test-support refactor.
- Duplicate helper mocks per behavior module: rejected because it would make
  scheduler and runtime contract changes harder to update consistently.
- Keep all shared fixtures in one file: rejected because the extracted fixture
  boundary was still large enough to hide unrelated host responsibilities.

## Invariants
- Child modules use the parent test module fixture re-exports instead of
  duplicating host mocks and scheduler setup.
- Extracted tests preserve their original public facade paths and assertions.
- Diagnostics projection tests must verify typed projection fields rather than
  inferring retention, scheduler, or run-list facet facts from raw payload JSON.
- I/O artifact query tests must include expired-retention fixture data so
  service callers prove retention state, payload-reference removal, and
  retention summary counts through the public API.
- I/O artifact query tests must include no-active-run browsing coverage so
  global retained-artifact reads remain explicitly supported for the workbench
  gallery.
- I/O artifact fixture events must use diagnostics-ledger typed artifact roles
  and assert the projected canonical labels through public query responses.
- Library usage query tests must cover warm projection catching-up status so
  service callers preserve backend freshness state when bounded projection
  batches leave later Library events unapplied.
- Library usage query tests must cover active-run filtering by
  `workflow_run_id` so the workbench can ask for selected-run assets without
  scanning raw ledger events.
- Diagnostics and session-execution tests that emit Library audit facts must
  use diagnostics-ledger typed operation/cache-status values, not arbitrary
  payload strings.
- Retention policy update tests must assert the typed actor scope on
  `retention.policy_changed` events so policy mutations remain auditable.
- Session admission tests with diagnostics enabled must verify durable
  scheduler delay events for runtime admission waits without depending on raw
  scheduler-store internals.
- Session execution tests with attribution and diagnostics enabled must verify
  `run.snapshot_accepted` events carry the immutable workflow version and node
  behavior-version set before scheduler admission.
- New behavior families should be added here only when they are cohesive enough
  to reduce `workflow/tests.rs` without hiding shared test setup.

## Revisit Triggers
- Fixture families become broadly reusable outside workflow facade tests.
- `workflow/tests.rs` regains shared fixture definitions or behavior-specific
  test groups.
- Scheduler, runtime, or graph tests need fixture builders that would simplify
  multiple child modules.

## Dependencies
**Internal:** parent `workflow::tests` fixture re-exports, workflow service
facade methods, scheduler store contracts, graph edit-session contracts, and
technical-fit override DTOs.

**External:** none beyond the crate's existing test dependencies.

Reason: child modules inherit the parent module context so extraction does not
create new package-level coupling.

Revisit trigger: add direct dependencies only if a child module owns reusable
test infrastructure that cannot live in the parent test module.

## Related ADRs
- `docs/adr/ADR-003-rust-workspace-policy.md`

## Usage Examples
Run the scheduler snapshot behavior slice:

```bash
cargo test -p pantograph-workflow-service workflow::tests::contracts
cargo test -p pantograph-workflow-service workflow::tests::session_admission
cargo test -p pantograph-workflow-service workflow::tests::session_capacity
cargo test -p pantograph-workflow-service workflow::tests::session_capacity_limits
cargo test -p pantograph-workflow-service workflow::tests::session_execution
cargo test -p pantograph-workflow-service workflow::tests::session_runtime_preflight
cargo test -p pantograph-workflow-service workflow::tests::session_runtime_state
cargo test -p pantograph-workflow-service workflow::tests::session_stale_cleanup
cargo test -p pantograph-workflow-service workflow::tests::workflow_capabilities
cargo test -p pantograph-workflow-service workflow::tests::scheduler_snapshot_diagnostics
cargo test -p pantograph-workflow-service workflow_get_scheduler_snapshot
cargo test -p pantograph-workflow-service workflow_session_queue
cargo test -p pantograph-workflow-service workflow_get_io
cargo test -p pantograph-workflow-service workflow_preflight
cargo test -p pantograph-workflow-service workflow::tests::runtime_preflight
cargo test -p pantograph-workflow-service workflow::tests::workflow_run
```

## Testing
```bash
cargo test -p pantograph-workflow-service workflow::tests::contracts
cargo test -p pantograph-workflow-service workflow::tests::session_admission
cargo test -p pantograph-workflow-service workflow::tests::session_capacity
cargo test -p pantograph-workflow-service workflow::tests::session_capacity_limits
cargo test -p pantograph-workflow-service workflow::tests::session_execution
cargo test -p pantograph-workflow-service workflow::tests::session_runtime_preflight
cargo test -p pantograph-workflow-service workflow::tests::session_runtime_state
cargo test -p pantograph-workflow-service workflow::tests::session_stale_cleanup
cargo test -p pantograph-workflow-service workflow::tests::workflow_capabilities
cargo test -p pantograph-workflow-service workflow::tests::scheduler_snapshot_diagnostics
cargo test -p pantograph-workflow-service workflow_get_scheduler_snapshot
cargo test -p pantograph-workflow-service workflow_session_queue
cargo test -p pantograph-workflow-service workflow_get_io
cargo test -p pantograph-workflow-service workflow_preflight
cargo test -p pantograph-workflow-service workflow::tests::runtime_preflight
cargo test -p pantograph-workflow-service workflow::tests::workflow_run
```
