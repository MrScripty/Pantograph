# Source

## Purpose
This directory contains the dependency-environment service facade, provider
trait, not-implemented provider, and typed service errors.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Public facade, provider trait, no-I/O provider, and error type. |
| `snapshot.rs` | Synchronous path-free readiness snapshot provider. |
| `work_queue.rs` | Typed readiness producer work-item DTOs and in-memory queue contract. |

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
Keep the facade and simple no-I/O provider in `lib.rs`. Put the readiness
snapshot provider in `snapshot.rs` because it is the first production-oriented
provider contract and owns separate matching/fail-closed behavior.

Put readiness producer work items in `work_queue.rs`. The queue is synchronous
and in-memory for this slice; it establishes the shared DTO and deterministic
dedupe/dequeue behavior before workflow-service emits items or embedded-runtime
drains them.
Snapshot helpers can convert queued work into explicit unavailable snapshots so
producers can publish fail-closed diagnostics before real host probes exist.

The snapshot provider keys readiness by canonical stable request identity:
action, path-free dependency identity key, dependency requirements id, and
request environment ref. It validates the producer planning request before
insertion, but it does not key on caller context such as workflow run id because
that context can legitimately change between scheduling runs without changing
the dependency environment requirements.

## Alternatives Rejected
- Key snapshots by full planning request: rejected because caller context is
  diagnostic/provenance data and would make precomputed readiness unusable for
  new workflow runs with identical dependency requirements.
- Make the facade async now: rejected because the current implementation has no
  awaited I/O.
- Record snapshot provider misses as producer work: rejected because provider
  reads must remain side-effect free from the caller's perspective.

## Invariants
- Service methods validate provider output before returning.
- The not-implemented provider never calls legacy resolvers.
- Provider wiring is supplied by the caller.
- Snapshot provider misses, stale snapshots, and identity/detail mismatches
  return validated non-ready results with typed diagnostics.
- Snapshot provider never probes filesystem, package managers, Pumas, runtime
  hosts, or executable load targets.
- Readiness work items carry scheduler/task provenance and validated requests;
  they do not represent readiness success and do not invoke probes by
  themselves.
- Unavailable snapshots created from work items are diagnostics, not fallback
  success paths.

## Revisit Triggers
- Production Pumas provider implementation is added.
- Service methods need async provider I/O.
- Scheduler readiness/admission projection becomes part of this crate.
- Readiness work requires durable queue storage, distributed leasing, or
  restart recovery.

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

Snapshot-backed readiness providers are built and populated by the backend
composition owner:

```rust
use pantograph_dependency_environment_service::DependencyEnvironmentReadinessSnapshotProvider;
use pantograph_dependency_environment_service::DependencyEnvironmentService;

let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
let _service = DependencyEnvironmentService::new(provider);
```

## API Consumer Contract
- Inputs are validated dependency-environment requests.
- Outputs are validated dependency-environment results.
- Provider failures and invalid provider output are typed service errors.
- The facade does not own background tasks, retries, or shutdown.
- Snapshot insertion is synchronous and deterministic; async producers must own
  their task lifecycle outside this crate and only publish validated snapshots.
- Work queue insertion and dequeue are synchronous and deterministic; async
  producer tasks must be owned outside this crate.

## Structured Producer Contract
- The facade produces shared dependency-environment result contracts.
- Not-implemented output includes typed not-implemented states and diagnostics.
- Result validation is mandatory before data crosses back to workflow-service.
- Result shape changes require updates to dependency-planning DTOs and tests.
