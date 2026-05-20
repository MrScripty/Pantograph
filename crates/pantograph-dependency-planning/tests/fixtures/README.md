# pantograph-dependency-planning/tests/fixtures

## Purpose
This directory stores JSON fixtures for the dependency-planning public contract
tests. The fixtures show the serialized request/result shapes that Rust,
frontend, persisted, and worker-adjacent consumers must preserve.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `dependency_planning_request.json` | Valid graph/host dependency-planning request keyed by `pumas_model_ref`. |
| `dependency_planning_ready_result.json` | Ready result carrying a Pumas-approved load target for backend handoff. |
| `dependency_planning_unavailable_result.json` | Unavailable result with a typed Pumas diagnostic and no load target. |

## Problem
Serialized contract drift can compile in Rust while breaking frontend,
persisted, or host-boundary consumers. These fixtures make the intended wire
shape reviewable and testable.

## Constraints
- Request fixtures must not contain local model path identity.
- Load paths may appear only in ready result fixtures as Pumas-approved handoff
  targets.
- Diagnostics must use typed codes and severities.

## Decision
Keep fixtures small and scenario-focused so they can be reused by later
cross-layer tests without importing unrelated runtime behavior.

## Alternatives Rejected
- Store only inline JSON in tests: rejected because committed fixtures are
  easier for downstream consumers and plan reviews to inspect.
- Add broad saved-workflow fixtures now: rejected because graph migration is a
  later Milestone 5 slice.

## Invariants
- `dependency_planning_request.json` uses `pumas_model_ref`.
- Ready results include exactly one host/planner load target.
- Unavailable results include diagnostics and no load target.

## Revisit Triggers
- The frontend or worker starts consuming generated fixture copies.
- Pumas publishes a newer artifact load-target wire shape.
- Dependency planning adds versioned schemas.

## Dependencies
**Internal:** `tests/contract.rs`.

**External:** None.

## Related ADRs
- None identified as of 2026-05-20.
- Reason: fixture shape is currently owned by the Milestone 5 plan.
- Revisit trigger: serialized dependency-planning contracts become public
  extension APIs.

## Usage Examples
```bash
cargo test -p pantograph-dependency-planning dependency_planning_request_fixture_decodes_and_validates
```

## API Consumer Contract
- Fixtures are read by public integration tests through `include_str!`.
- Fixture changes must preserve serde casing and typed diagnostic semantics.

## Structured Producer Contract
- JSON field names use snake_case.
- Optional fields may be omitted only when the DTO default has explicit
  semantics.
- New enum values require fixture and consumer updates in the same slice.
