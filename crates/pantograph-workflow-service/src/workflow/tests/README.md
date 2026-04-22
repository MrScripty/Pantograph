# pantograph-workflow-service/src/workflow/tests

Behavior-focused tests for the workflow service facade.

## Purpose
This directory holds cohesive test modules extracted from the legacy
`workflow/tests.rs` module. The parent test module keeps shared mocks and
helpers, while child modules isolate behavior families so the service facade
can stay reviewable as more tests are split.

## Contents
| File | Description |
| ---- | ----------- |
| `scheduler_snapshot.rs` | Scheduler snapshot facade tests covering workflow/edit-session snapshot shape, trace attribution, admission diagnostics, and runtime-registry diagnostics provider merging. |
| `session_queue.rs` | Session queue item metadata, admission outcome, warm reuse, queue-position, and starvation protection tests. |

## Problem
`workflow/tests.rs` remains too large to review efficiently. Moving every test
into standalone modules at once would risk unnecessary churn around shared
mocks, so behavior families need to be extracted incrementally.

## Constraints
- Preserve existing workflow facade behavior and test assertions.
- Keep shared mocks in the parent test module until a stable shared test-support
  boundary is worth introducing.
- Avoid production API changes while splitting test coverage.

## Decision
Use `workflow/tests/` for behavior-specific child modules under the parent
`workflow::tests` module. Child modules import parent test helpers with
`super::*`, keeping the extraction mechanical while reducing the parent file.

## Alternatives Rejected
- Move all tests into separate files immediately: rejected because shared host
  mocks and fixture setup would need a larger test-support refactor.
- Duplicate helper mocks per behavior module: rejected because it would make
  scheduler and runtime contract changes harder to update consistently.

## Invariants
- Child modules use the parent test module helpers instead of duplicating host
  mocks and scheduler setup.
- Extracted tests preserve their original public facade paths and assertions.
- New behavior families should be added here only when they are cohesive enough
  to reduce `workflow/tests.rs` without hiding shared test setup.

## Revisit Triggers
- Shared mocks become stable enough to move into a dedicated test-support
  module.
- `workflow/tests.rs` stops owning any behavior-specific test groups.
- Scheduler, runtime, or graph tests need fixture builders that would simplify
  multiple child modules.

## Dependencies
**Internal:** parent `workflow::tests` mocks, workflow service facade methods,
scheduler store contracts, graph edit-session contracts, and technical-fit
override DTOs.

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
cargo test -p pantograph-workflow-service workflow_get_scheduler_snapshot
cargo test -p pantograph-workflow-service workflow_session_queue
```

## Testing
```bash
cargo test -p pantograph-workflow-service workflow_get_scheduler_snapshot
cargo test -p pantograph-workflow-service workflow_session_queue
```
