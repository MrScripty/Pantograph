# pantograph-dependency-environment-service

Canonical backend service facade for dependency-environment resolve, check, and
install actions.

## Purpose
This crate owns the service boundary between validated dependency-environment
requests and validated dependency-environment results. It exists so
workflow-service can call one canonical backend service instead of preserving
legacy path-shaped model dependency resolvers.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Crate manifest and direct dependency ownership. |
| `src/` | Public service facade, provider trait, snapshot provider, readiness work queue, typed errors, and source README. |
| `tests/` | Public API contract tests for the no-I/O service boundary. |

## Problem
Dependency-environment actions currently risk routing through retired
`ModelDependencyRequest` and `model_path` contracts. Pantograph needs a typed
backend-owned service that consumes only canonical dependency-planning DTOs,
returns validated dependency-environment results, and leaves graph editor and
Tauri layers as intent transport.

## Constraints
- The crate must not depend on workflow-service, node-engine,
  pantograph-embedded-runtime, Tauri, frontend DTOs, or graph display state.
- The crate must not accept model paths, package-fact blobs, local load paths,
  or frontend platform context.
- Pure request/result validation and projection stay synchronous.
- Concrete Pumas, process, filesystem, runtime, or install-provider
  infrastructure is wired at the backend composition boundary.
- Missing provider support returns typed not-implemented diagnostics; it must
  not fall back to legacy resolvers.

## Decision
Expose a small service facade over provider traits. The facade accepts
`ValidatedDependencyEnvironmentRequest`, calls the selected provider, and
returns `ValidatedDependencyEnvironmentResult`. The no-I/O not-implemented
provider proves contract shape when production readiness is unavailable.

The first production-oriented provider is
`DependencyEnvironmentReadinessSnapshotProvider`. It reads backend-owned
readiness snapshots keyed by action, path-free dependency identity key,
dependency requirements id, and request environment ref. The snapshot validates
its producer planning request before insertion, but caller context such as a
workflow run id is not part of the readiness key because it does not change the
dependency environment requirements.

The production producer source is represented by `DependencyReadinessWorkQueue`
and `DependencyReadinessWorkItem`. Workflow-service or scheduler emits work
items when runtime tasks enter `WaitingDependencyReadiness`; infrastructure or
embedded-runtime lifecycle owners drain the queue and publish validated
snapshots. Work items carry task/run/session provenance, a validated
dependency-environment request, bounded diagnostic context, retry/freshness
policy, and cancellation scope without making the snapshot provider record
misses.

## Alternatives Rejected
- Reuse node-engine `ModelDependencyRequest`: rejected because it preserves
  path-shaped legacy behavior.
- Put dependency policy in Tauri: rejected because Tauri is transport-only.
- Let workflow-service build raw results directly: rejected because result
  validation and provider lifecycle need one backend service boundary.
- Key readiness snapshots by full planning request: rejected because caller
  context is provenance data and would prevent one validated readiness snapshot
  from serving later runs with identical dependency requirements.
- Use snapshot provider misses as the producer work source: rejected because
  the provider must stay read-only from the caller's perspective and must not
  hide write side effects in dependency-readiness resolution.
- Let the producer scan frontend, graph editor, technical-fit preview, or
  runtime-host load-target state: rejected because producer work must come from
  backend scheduler task state and validated dependency-environment requests.

## Invariants
- All public service inputs are validated dependency-planning contracts.
- All public service outputs are validated dependency-environment results.
- No service path accepts or emits model paths or local load paths.
- No provider is self-created by graph, frontend, Tauri, node-engine, or
  embedded-runtime code.
- Not-implemented behavior is explicit diagnostic output, not fallback logic.
- Snapshot provider misses, stale snapshots, and request-detail mismatches
  produce validated non-ready diagnostic results.
- Snapshot provider does not probe Pumas, package managers, filesystems,
  runtimes, or executable load targets.
- Readiness work queue items are task-correlated producer inputs, not readiness
  proof. They do not publish snapshots, probe hosts, or make dependency
  readiness successful.

## Revisit Triggers
- Pumas exposes concrete dependency-environment resolve/check/install APIs.
- Installs, polling, retries, or subprocess supervision require a lifecycle
  owner.
- Scheduler admission begins consuming dependency readiness proofs from service
  results.
- A second provider implementation needs async I/O in the provider boundary.
- Readiness work needs durable persistence, leasing across processes, or
  restart recovery beyond the in-memory queue contract.

## Dependencies
**Internal:** `pantograph-dependency-planning` for request/result contracts.

**External:** `thiserror` for typed service errors.

## Related ADRs
- None identified as of 2026-05-26.
- Reason: This slice records the boundary in the active implementation plan
  before production provider wiring exists.
- Revisit trigger: A production Pumas provider or scheduler-readiness adapter is
  added.

## Usage Examples
```rust
use pantograph_dependency_environment_service::{
    DependencyEnvironmentService, NotImplementedDependencyEnvironmentProvider,
};

let service = DependencyEnvironmentService::new(
    NotImplementedDependencyEnvironmentProvider::default(),
);
```

```rust
use pantograph_dependency_environment_service::{
    DependencyEnvironmentReadinessSnapshotProvider, DependencyEnvironmentService,
};

let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
let service = DependencyEnvironmentService::new(provider);
```

## API Consumer Contract
- Inputs: validated dependency-environment requests from workflow-service.
- Outputs: validated dependency-environment results with typed diagnostics.
- Lifecycle: callers own service construction and provider selection at the
  backend composition boundary.
- Snapshot lifecycle: producers outside this crate own async probing,
  cancellation, retries, tracing, and shutdown. This crate accepts only
  already-validated synchronous snapshots.
- Work queue lifecycle: backend application owners enqueue validated work
  items; infrastructure or embedded-runtime lifecycle owners drain them. Queue
  items are not provider misses and must not be derived from frontend or legacy
  path data.
- Error behavior: invalid provider output is returned as a typed service error;
  missing provider behavior is represented as a validated not-implemented
  result.
- Compatibility: public contracts are additive until a coordinated plan records
  a breaking change.

## Structured Producer Contract
- Stable fields: action, identity key, readiness/install/validation states,
  failure state, operation state, and diagnostics come from shared planning
  DTOs.
- Defaults: absent provider support is represented as not-implemented
  diagnostic state.
- Enum semantics: state and diagnostic enums are semantic contracts, not display
  strings.
- Ordering: provider output must remain deterministic for tests and diagnostics.
- Compatibility: persisted or displayed diagnostics may outlive a release, so
  field additions should be additive.
- Regeneration/migration: update shared dependency-planning DTOs and this
  README together when the service result shape changes.

## Testing
Run focused service tests from the workspace root:

```bash
cargo test -p pantograph-dependency-environment-service
```
