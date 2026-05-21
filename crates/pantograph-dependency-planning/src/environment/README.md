# pantograph-dependency-planning/src/environment

## Purpose
This directory contains child modules for dependency-environment contracts that
would make `environment.rs` too broad if kept inline.

## Contents
| File | Description |
| ---- | ----------- |
| `payload.rs` | Shared dependency-environment result payload rows, typed ids, operation timestamps, validation errors, and row-level validation helpers. |
| `scalar.rs` | Validated scalar values and helpers for profile ids, requirement names, validation field paths, operation timestamps, diagnostics, and selected binding uniqueness. |
| `state.rs` | Dependency-environment action, readiness, install, validation, and failure enums. |

## Problem
Dependency-environment results need more structure than the request envelope:
requirements, bindings, per-binding status, operation timing, validation
errors, and runtime-specific detail rows. Keeping those rows in the envelope
module crosses the decomposition threshold and makes ownership harder to
review.

## Constraints
- Keep this directory contract-only.
- Do not call Pumas, inspect files, install packages, start workers, or select
  runtimes here.
- Result payload rows must stay reusable by node-engine, embedded-runtime,
  frontend DTO mirrors, persisted fixtures, and future worker-adjacent
  contracts.

## Decision
Place result payload rows in `payload.rs` and re-export their public contract
types from `environment.rs` and `lib.rs`. The parent module keeps the
dependency-environment request/result envelope and action/state enums.

## Alternatives Rejected
- Keep all rows in `environment.rs`: rejected because it crosses the file-size
  review threshold and mixes envelope validation with row contracts.
- Move payload rows into `result.rs`: rejected because they are specific to
  dependency-environment resolve/check/install results, not generic Pumas load
  target planning results.

## Invariants
- Payload structs use snake_case serde and deny unknown fields.
- Python/package-manager facts stay in Python-specific detail structs.
- Operation timestamps are non-zero milliseconds.
- Validation field paths are contract field paths, not filesystem paths.

## Revisit Triggers
- Dependency planning grows a full dependency-domain model separate from
  dependency-environment resolve/check/install.
- Frontend or worker schemas are generated from these contracts.

## Dependencies
**Internal:** `super::DependencyEnvironmentValidationState`,
`crate::request::DependencyBindingId`, and
`crate::result::DependencyPlanningDiagnostic`.

**External:** `serde`.

## Related ADRs
- None identified as of 2026-05-21.
- Reason: this split implements the Milestone 5 standards pass and
  decomposition review.

## API Consumer Contract
- Consumers import public payload types through the crate root.
- Parent module validation calls row-level validation helpers before accepting
  a dependency-environment result.

## Structured Producer Contract
- Producers must preserve selected binding order and avoid duplicate selected
  binding ids.
- Producers must not put path-shaped values in validation field paths.
