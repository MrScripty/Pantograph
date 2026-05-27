# Source

## Purpose
This directory contains the dependency-environment service facade, provider
trait, not-implemented provider, and typed service errors.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Public facade, provider trait, no-I/O provider, and error type. |

## Problem
Workflow-service needs a backend-owned service contract that can be wired to
real Pumas/dependency providers later without preserving legacy model-path
resolvers.

## Constraints
- Keep this source boundary synchronous until provider I/O requires async.
- Accept only validated dependency-planning request types.
- Return only validated dependency-environment result types.
- Do not create concrete Pumas, filesystem, process, runtime, or install
  infrastructure here.

## Decision
Keep the first slice in one small `lib.rs` module. Split modules only when a
production provider or scheduler-readiness adapter adds enough behavior to make
separate ownership clearer.

## Alternatives Rejected
- Add provider modules now: rejected because the current slice has only a
  no-I/O provider and extra files would create empty abstraction.
- Make the facade async now: rejected because the current implementation has no
  awaited I/O.

## Invariants
- Service methods validate provider output before returning.
- The not-implemented provider never calls legacy resolvers.
- Provider wiring is supplied by the caller.

## Revisit Triggers
- Production Pumas provider implementation is added.
- Service methods need async provider I/O.
- Scheduler readiness/admission projection becomes part of this crate.

## Dependencies
**Internal:** `pantograph-dependency-planning`.

**External:** `thiserror`.

## Related ADRs
- None identified as of 2026-05-26.
- Reason: The active plan owns this early service-boundary decision.
- Revisit trigger: Provider lifecycle or scheduler admission ownership changes.

## Usage Examples
```rust
use pantograph_dependency_environment_service::DependencyEnvironmentService;
use pantograph_dependency_environment_service::NotImplementedDependencyEnvironmentProvider;

let _service = DependencyEnvironmentService::new(
    NotImplementedDependencyEnvironmentProvider::default(),
);
```

## API Consumer Contract
- Inputs are validated dependency-environment requests.
- Outputs are validated dependency-environment results.
- Provider failures and invalid provider output are typed service errors.
- The facade does not own background tasks, retries, or shutdown.

## Structured Producer Contract
- The facade produces shared dependency-environment result contracts.
- Not-implemented output includes typed not-implemented states and diagnostics.
- Result validation is mandatory before data crosses back to workflow-service.
- Result shape changes require updates to dependency-planning DTOs and tests.
