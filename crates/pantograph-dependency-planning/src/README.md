# pantograph-dependency-planning/src

## Purpose
This source directory contains the Rust implementation of the dependency
planning contract crate. It exists to keep shared DTOs, validated identifiers,
and typed diagnostics separate from runtime execution, Pumas client access,
frontend state, and scheduler policy.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Public re-export surface for contract consumers. |
| `environment.rs` | Dependency-environment request/result DTOs, typed requirement and binding rows, status rows, operation timestamps, validation errors, environment refs, and validation helpers. |
| `environment/` | Child modules for dependency-environment result payload rows that would otherwise make the envelope module too broad. |
| `error.rs` | Typed validation errors for request parsing and load-target result invariants. |
| `model_ref.rs` | Pumas-compatible model reference, artifact entry path, artifact kind, storage, validation, and load-target mirrors. |
| `preflight.rs` | Path-free preflight request/result contracts and shared dependency-planning identity/correlation key. |
| `request.rs` | Dependency-planning request DTOs, caller context, scheduler intent, dependency overrides, and validated ids. |
| `result.rs` | Dependency-planning state, diagnostic, and result DTOs. |

## Problem
Dependency planning crosses graph, host, frontend, persisted fixture, and
backend-adjacent boundaries. Keeping the source files here prevents node-engine
or inference modules from becoming accidental owners of Pumas artifact
semantics or scheduler intent.

## Constraints
- Keep this crate free of Pumas clients, filesystem access, subprocess
  execution, frontend imports, scheduler policy, and worker execution.
- Validate raw payloads once at the boundary and pass typed domain values
  inward.
- Preserve serde field casing and enum spellings because multiple layers
  consume the same JSON shapes.

## Decision
Split the crate into small contract modules: errors, Pumas-facing model/load
target contracts, path-free preflight identity, requests, and results. `lib.rs`
is the only public facade.

## Alternatives Rejected
- Put all DTOs in `lib.rs`: rejected because request, result, and Pumas mirror
  responsibilities would grow into one hard-to-review file.
- Put resolver behavior here: rejected because Pumas lookup and scheduler
  policy belong to host/planner crates.

## Invariants
- Request identity is `PumasModelRef`, never local path.
- Preflight identity and host/planner load-target result contracts remain
  separate so graph/node-engine identity cannot carry executable paths.
- Load targets are result/handoff facts only.
- Public fallible validation returns typed errors.
- Serde fixture tests cover public wire shapes.
- Dependency override patches stay in the shared request contract so manual
  dependency-environment behavior can migrate without adapter-local fields.
- Platform context uses a validated platform key instead of raw JSON.
- Source node type stays in caller context for traceability and must not become
  runtime selection policy.
- Dependency-environment result rows use typed requirement kinds, environment
  kinds, binding status states, operation states, validation codes, ids,
  requirement names, validation field paths, and non-zero operation timestamps.
- Python/package-manager fields are contained in Python-specific detail structs
  so managed-binary, system-package, runtime-feature, device/toolchain, and
  non-Python dependencies can be added without overloading Python rows.

## Revisit Triggers
- The crate exceeds the decomposition thresholds.
- Pumas publishes a shared generated schema consumed directly by Pantograph.
- Dependency planning starts owning scheduler policy instead of scheduler
  intent.

## Dependencies
**Internal:** None.

**External:** `serde`, `serde_json`, and `thiserror`.

## Related ADRs
- None identified as of 2026-05-20.
- Reason: this is the implementation of the current Milestone 5 contract-owner
  plan.
- Revisit trigger: dependency-planning contracts become public extension APIs.

## Usage Examples
```rust
use pantograph_dependency_planning::DependencyTaskId;

let task_id = DependencyTaskId::parse("image_generation")?;
assert_eq!(task_id.as_str(), "image_generation");
# Ok::<(), pantograph_dependency_planning::DependencyPlanningContractError>(())
```

## API Consumer Contract
- Consumers import DTOs from `lib.rs` re-exports.
- Raw JSON should be decoded into the public request/result types and validated
  before internal use.
- Errors from validation use `DependencyPlanningContractError`.
- Dependency-environment unavailable, invalid, missing, failed, and
  not-implemented outcomes are result states with diagnostics, not string
  errors.

## Structured Producer Contract
- Stable fields and enum spellings are asserted by `tests/contract.rs`.
- Optional scheduler intent, selected bindings, caller context, and diagnostics
  default to empty values.
- New public DTO fields require fixture updates in the same slice.
- Boundary request/result/row structs reject unknown fields where they carry
  dependency-environment protocol state.
