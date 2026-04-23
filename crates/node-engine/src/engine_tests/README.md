# engine_tests

## Purpose

This directory contains focused workflow-engine facade tests included by
`../engine_tests.rs`. The parent module owns shared graph and executor fixtures,
while these modules keep cache state, demand execution, workflow events,
multi-demand behavior, human input, and snapshot coverage below the source-size
threshold.

## Contents

| File | Description |
| ---- | ----------- |
| `cache_state.rs` | Version tracking, cache invalidation, cache stats, and isolated-run reconciliation tests. |
| `demand.rs` | Single-target demand execution, caching, partial recompute, diamond graph, and event emission tests. |
| `human_input.rs` | Waiting-for-input emission and continuation with supplied responses. |
| `multi_demand.rs` | Multi-target incremental execution, attribution, redundant targets, and stopping behavior. |
| `snapshot.rs` | Workflow executor snapshot projection. |
| `workflow_events.rs` | Workflow executor demand, graph mutation, and graph-modified event tests. |

## Problem

The root workflow-engine test module mixed fixtures with several behavior
families. As the engine facade split into focused production helpers, the test
file remained above the large-file threshold and made behavior-specific review
harder.

## Constraints

- Tests must continue exercising the public workflow engine and executor facade.
- Shared executor fixtures should stay in the parent module unless they become a
  separate test support boundary.
- Multi-demand tests stay separate from single-demand tests because their event
  attribution and stopping semantics differ.

## Decision

Keep shared graph and executor fixtures in `../engine_tests.rs`, and split
behavior coverage into modules aligned with cache state, demand execution,
workflow event emission, multi-demand execution, human-input waiting, and
snapshot projection.

## Alternatives Rejected

- Leaving all tests in `engine_tests.rs`: rejected because the file exceeded the
  large-file threshold and mixed unrelated engine behavior.
- Moving fixtures into each behavior module: rejected because it would duplicate
  graph setup and test executor implementations.

## Invariants

- Fixture graphs and test executors remain shared by behavior modules.
- Cache and state reconciliation tests do not mix with event attribution tests.
- Human-input tests stay isolated from ordinary failure/waiting executor tests.

## Usage Examples

Run the workflow-engine facade tests through the node-engine test filter:

```bash
cargo test -p node-engine engine::tests
```

Add new tests to the module that owns the behavior under assertion.

## Revisit Triggers

- A new workflow-engine behavior family needs dedicated test coverage.
- Shared fixtures grow enough to need a test-support submodule.
- Production engine facade ownership changes and the test module split no
  longer mirrors the production boundary.

## Dependencies

**Internal:** workflow engine facade, graph DTOs, workflow events, task
executor traits, and engine error contracts.

**External:** `tokio`, Rust test harness.

## Related ADRs

- [../../../../docs/standards-compliance-analysis/refactor-plan.md](../../../../docs/standards-compliance-analysis/refactor-plan.md)
