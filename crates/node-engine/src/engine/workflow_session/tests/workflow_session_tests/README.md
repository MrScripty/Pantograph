# workflow_session_tests

## Purpose

This directory contains focused tests for the workflow-session helper module in
`../../../workflow_session.rs`. The path mirrors Rust's module resolution for
submodules declared inside the inline `workflow_session::tests` module.

## Contents

| File | Description |
| ---- | ----------- |
| `kv_cache_memory.rs` | KV-cache node-memory projection, preserved cache reinjection, and suffix-only rerun reuse tests. |
| `memory_reconciliation.rs` | Recorded node-memory reconciliation and invalidation tests. |
| `session_helpers.rs` | Residency, checkpoint, workflow-session identity, and cache projection helper tests. |

## Problem

`workflow_session.rs` had a small production helper surface but a large inline
test module. Keeping all workflow-session memory, checkpoint, and KV-cache
coverage inline pushed the file above the large-file threshold and obscured the
production helper boundary.

## Constraints

- Production workflow-session helpers must remain private to the engine module
  facade.
- Tests must continue using the same `WorkflowExecutor` paths that production
  demand execution uses.
- KV-cache preservation coverage should stay separate from generic checkpoint
  and session identity tests.

## Decision

Keep shared test fixtures in the inline `workflow_session::tests` module and
move behavior coverage into this focused submodule directory. This preserves
access to private helper functions while reducing the production helper file
below the large-file threshold.

## Alternatives Rejected

- Moving tests to an external integration test: rejected because the coverage
  needs access to private helper functions and executor internals.
- Leaving all tests inline: rejected because the test body dominated the file
  and exceeded the large-file threshold.

## Invariants

- Workflow-session helper tests must exercise real `WorkflowExecutor` demand
  paths where cache synchronization is involved.
- KV-cache reference projection must keep model and runtime fingerprints in the
  preserved node-memory payload.
- Reconciliation tests must assert invalidation through the backend memory
  impact contract rather than test-only mutation shortcuts.

## Usage Examples

Run the focused workflow-session helper tests through the node-engine filter:

```bash
cargo test -p node-engine workflow_session
```

Add new tests to the module that matches the behavior under assertion.

## Revisit Triggers

- Workflow-session memory helpers become public enough to move coverage into
  external tests.
- KV-cache memory projection gains enough cases to justify another nested
  behavior module.
- Rust module layout changes remove the need for this nested inline-test path.

## Dependencies

**Internal:** `WorkflowExecutor`, workflow-session helper functions,
node-memory DTOs, graph-memory-impact contracts, and KV-cache compatibility
payloads.

**External:** `tokio`, Rust test harness.

## Related ADRs

- [../../../../../docs/standards-compliance-analysis/refactor-plan.md](../../../../../docs/standards-compliance-analysis/refactor-plan.md)
