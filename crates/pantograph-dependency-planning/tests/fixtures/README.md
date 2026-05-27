# pantograph-dependency-planning/tests/fixtures

## Purpose
This directory stores JSON fixtures for the dependency-planning public contract
tests. The fixtures show the serialized request/result shapes that Rust,
frontend, persisted, and worker-adjacent consumers must preserve.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `dependency_environment_check_request.json` | Typed dependency-environment check request keyed by path-free identity and requirements id. |
| `dependency_environment_install_request.json` | Typed dependency-environment install request with dependency override patches and environment ref. |
| `dependency_environment_invalid_result.json` | Invalid dependency-environment install result with typed failure state and diagnostic. |
| `dependency_environment_no_binding_result.json` | Unavailable dependency-environment resolve result with no selected binding ids and typed diagnostics. |
| `dependency_environment_ready_result.json` | Ready dependency-environment check result with typed readiness/install/validation states and environment ref. |
| `dependency_environment_resolve_request.json` | Typed dependency-environment resolve request that carries planning facts without path identity. |
| `dependency_environment_unavailable_result.json` | Unavailable dependency-environment resolve result with typed failure state and diagnostic. |
| `dependency_planning_identity_key.json` | Path-free cache/activity/preflight identity keyed by Pumas model ref, task, scheduler intent, platform, and selected bindings. |
| `dependency_readiness_request.json` | Path-free host readiness input with typed policy and no readiness proof or executable handoff facts. |
| `dependency_readiness_request_envelope.json` | Path-free readiness request plus active-run freshness identity for provider invocation. |
| `dependency_readiness_proof_envelope_ready.json` | Ready scheduler proof envelope that binds preflight proof to active-run freshness identity. |
| `dependency_preflight_ready_result.json` | Ready path-free preflight result with dependency-environment identity and readiness proof. |
| `dependency_preflight_request.json` | Path-free preflight request with graph intent and dependency-environment identity. |
| `dependency_preflight_unavailable_result.json` | Unavailable path-free preflight result with ordered typed diagnostics. |
| `dependency_planning_request.json` | Valid graph/host dependency-planning request keyed by `pumas_model_ref`, including selected binding ids and manual dependency override patches. |
| `dependency_planning_ready_result.json` | Ready result carrying a Pumas-approved load target for backend handoff. |
| `dependency_planning_unavailable_result.json` | Unavailable result with a typed Pumas diagnostic and no load target. |

## Problem
Serialized contract drift can compile in Rust while breaking frontend,
persisted, or host-boundary consumers. These fixtures make the intended wire
shape reviewable and testable.

## Constraints
- Request fixtures must not contain local model path identity.
- Dependency-environment request fixtures must carry typed
  resolve/check/install actions, not raw mode strings.
- Dependency-environment check/install request fixtures require
  `dependency_requirements_id`.
- Identity and preflight fixtures must not contain `model_path`,
  `selected_artifact_path`, `entry_path`, local load paths, or load targets.
- Readiness request fixtures must not contain readiness proof fields such as
  `dependency_requirements_id` or `environment_ref`.
- Readiness request fixtures must use typed policy enum values, not raw mode
  strings or booleans.
- Readiness execution envelope fixtures must not contain Pumas load targets,
  local paths, frontend display state, runtime-host payloads, or raw provider
  request payloads in scheduler proof.
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
- Request fixtures keep manual dependency override patches in the shared
  contract shape.
- Dependency-environment request fixtures keep `DependencyPlanningIdentityKey`
  and `DependencyPlanningRequest` aligned on model ref, task, artifact kind,
  platform, selected bindings, and scheduler intent.
- Dependency-environment result fixtures use typed readiness, install,
  validation, failure, binding status, operation, and validation-error states.
- Dependency-environment result fixtures keep requirement rows, binding rows,
  status rows, operation timestamps, validation errors, and environment refs in
  the shared contract shape.
- Python/package-manager facts appear only in Python-specific detail structs;
  generic requirement and binding rows must not flatten Python-only fields.
- Ready results include exactly one host/planner load target.
- Unavailable results include diagnostics and no load target.
- Operation timestamps are non-zero Unix epoch milliseconds, and completed
  timestamps must not precede started timestamps.
- Selected binding ids preserve producer order and must be unique.
- Path-free identity/preflight fixtures stay separate from ready result
  fixtures so backend handoff facts cannot become graph identity.
- Readiness fixtures stay separate from preflight result fixtures so host input
  cannot be mistaken for host-produced dependency readiness proof.
- Readiness execution request envelopes may wrap host readiness input, but
  readiness proof envelopes carry preflight proof only. They must not embed the
  raw readiness request or dependency override patch values in scheduler proof.
- Preflight request/result fixtures reject legacy selected runtime/device
  fields; runtime/device values in these fixtures are scheduler intent only.

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
- Path-shaped fields such as `model_path`, `modelPath`, `entry_path`,
  `selected_artifact_path`, or `local_load_path` are rejected in
  dependency-environment request fixtures.
- Readiness fixtures reject unknown, path-shaped, executable handoff, and
  proof-bearing fields before host wiring consumes them.
- Readiness execution envelope fixtures reject unknown, path-shaped, executable
  handoff, mismatched freshness, mismatched requirements, and zero proof version
  cases through public integration tests.
