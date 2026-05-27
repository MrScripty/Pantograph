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
| `preflight.rs` | Path-free preflight request/result contracts, shared dependency-planning identity/correlation key, and the validated projection from dependency-environment result to scheduler preflight proof. |
| `producer.rs` | Pure dependency requirements proof producer that derives path-free requirements ids, override fingerprints, proof status, and diagnostics. |
| `readiness.rs` | Path-free host readiness input contract and typed readiness policy for producing preflight proof without leaking executable handoff facts. |
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
- Dependency readiness input is a host request, not readiness proof; it carries
  typed policy and path-free planning identity while the host produces
  `DependencyPreflightResult`.
- Dependency-environment results are projected into scheduler preflight proof
  through the preflight module. The projection preserves only path-free
  identity, readiness state, requirements/environment identity, and diagnostics;
  it does not carry requirements tables, binding rows, operation timing, local
  paths, Pumas package facts, or runtime load targets into scheduler admission.
- Dependency requirements proof production stays in this crate as pure domain
  logic. It can consume optional typed availability facts, but it cannot call
  Pumas, inspect files, select runtime/device policy, or query scheduler state.

## Revisit Triggers
- The crate exceeds the decomposition thresholds.
- Pumas publishes a shared generated schema consumed directly by Pantograph.
- Dependency planning starts owning scheduler policy instead of scheduler
  intent.

## Dependencies
**Internal:** None.

**External:** `serde`, `serde_json`, `thiserror`, and `blake3`. `blake3` is
used only to derive stable requirements ids and override fingerprints from
canonical typed identity payloads.

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
- Dependency readiness requests are host inputs. They may influence whether the
  host checks only or prepares missing dependencies, but they do not carry
  requirements ids, environment refs, Pumas package facts, local paths, or
  executable worker handoff data.
- Dependency requirements proof producer APIs accept validated planning
  requests plus optional typed availability facts and return path-free proof
  records. They are synchronous because the producer performs no I/O.
- Scheduler readiness callers must use
  `dependency_preflight_result_from_environment_result` when turning a
  dependency-environment provider result into `DependencyPreflightResult`.
  They must not synthesize preflight proof from graph node data, technical-fit
  preview diagnostics, reduced execution-plan projections, or frontend/Tauri
  display state.

## Structured Producer Contract
- Stable fields and enum spellings are asserted by `tests/contract.rs`.
- Optional scheduler intent, selected bindings, caller context, and diagnostics
  default to empty values.
- New public DTO fields require fixture updates in the same slice.
- Boundary request/result/row structs reject unknown fields where they carry
  dependency-environment protocol state.
- Readiness request fixtures use typed `policy` enum values and reject unknown,
  path-shaped, and executable handoff fields before host wiring can consume
  them.
- Requirements ids include only canonical typed identity fields: model ref,
  task, artifact kind, scheduler intent, platform key, selected bindings,
  dependency override fingerprint, and dependency-planning-local trait intents.
  Caller context, migration diagnostics, paths, package facts, runtime load
  targets, scheduler dispatch decisions, and frontend display state are
  excluded.
