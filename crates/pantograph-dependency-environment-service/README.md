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
| `src/` | Public service facade, provider trait, typed errors, and source README. |
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
returns `ValidatedDependencyEnvironmentResult`. The initial provider is a
no-I/O not-implemented provider used to prove contract shape before production
Pumas/install behavior is added.

## Alternatives Rejected
- Reuse node-engine `ModelDependencyRequest`: rejected because it preserves
  path-shaped legacy behavior.
- Put dependency policy in Tauri: rejected because Tauri is transport-only.
- Let workflow-service build raw results directly: rejected because result
  validation and provider lifecycle need one backend service boundary.

## Invariants
- All public service inputs are validated dependency-planning contracts.
- All public service outputs are validated dependency-environment results.
- No service path accepts or emits model paths or local load paths.
- No provider is self-created by graph, frontend, Tauri, node-engine, or
  embedded-runtime code.
- Not-implemented behavior is explicit diagnostic output, not fallback logic.

## Revisit Triggers
- Pumas exposes concrete dependency-environment resolve/check/install APIs.
- Installs, polling, retries, or subprocess supervision require a lifecycle
  owner.
- Scheduler admission begins consuming dependency readiness proofs from service
  results.
- A second provider implementation needs async I/O in the provider boundary.

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

## API Consumer Contract
- Inputs: validated dependency-environment requests from workflow-service.
- Outputs: validated dependency-environment results with typed diagnostics.
- Lifecycle: callers own service construction and provider selection at the
  backend composition boundary.
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
