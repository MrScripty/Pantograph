# gateway_tests

Behavior-focused child modules for the inference gateway test facade.

## Purpose
This directory keeps large gateway test behavior groups below the source-size
threshold while preserving the root `gateway_tests.rs` fixture boundary.

## Contents
| File | Responsibility |
| ---- | -------------- |
| `start_config.rs` | Gateway start-config, embedding-runtime preparation, and restart-config behavior tests. |

## Problem
The gateway tests combine reusable mock backends with start configuration,
runtime lifecycle, forwarding, and KV-cache behavior. Keeping every assertion
in the root module makes the fixture file hard to review and pushes it over
the large-file standard.

## Constraints
- Keep mock backend fixtures in `gateway_tests.rs` until they need their own
  shared fixture module.
- Keep tests private to the gateway implementation boundary.
- Preserve existing feature gates and gateway facade coverage.

## Decision
Use this directory for gateway test slices that can share the root fixtures
through child-module access. Each child module owns one behavior family and
must stay below the large-file threshold.

## Alternatives Rejected
- Move fixtures first: rejected for this pass because extracting one behavior
  family reduces the root below the threshold with less fixture churn.
- Keep all gateway tests in one file: rejected because the file already exceeded
  the large-file limit.

## Invariants
- Child modules must not introduce public test-only APIs.
- Tests must continue exercising `InferenceGateway` through its facade methods.
- Shared mocks remain behavior-neutral and must not encode test-specific
  assertions.

## Revisit Triggers
- The root fixture section grows above the threshold again.
- Another gateway behavior family becomes large enough for its own module.
- Mock backends need to be reused outside gateway tests.

## Dependencies
**Internal:** `crate::gateway`, `crate::backend`, and root gateway test mocks.

**External:** `tokio` for async gateway tests.

## Related ADRs
- `docs/standards-compliance-analysis/refactor-plan.md`

## Usage Examples
```rust
#[path = "gateway_tests/start_config.rs"]
mod start_config;
```
