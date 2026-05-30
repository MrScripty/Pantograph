# pantograph-dependency-planning/tests

## Purpose
This directory contains public contract tests for the dependency-planning crate.
The tests decode committed JSON fixtures through the crate's public API so
wire-shape drift is caught before downstream Rust, frontend, or persisted
consumers depend on stale fields.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `contract.rs` | Public serde and validation tests for request/result DTOs and Pumas entry paths. |
| `observation_projection_contract.rs` | Public serde and validation tests for selected-binding inventory observation projection, including bounded provider alternatives. |
| `provider_source_contract.rs` | Public serde and validation tests for runtime-feature and device-toolchain provider-source snapshots. |
| `readiness_execution_contract.rs` | Public serde and validation tests for execution freshness request/proof envelopes. |
| `readiness_contract.rs` | Public serde and validation tests for the host readiness input contract. |
| `fixtures/` | Versioned JSON examples for ready and unavailable planning states. |

## Problem
Compile-time Rust types alone do not prove serialized field names, enum
spellings, defaults, or diagnostic shapes. Fixture tests provide executable
contracts for multi-layer consumers.

## Constraints
- Tests must use the public crate API, not private modules.
- Fixtures must avoid graph-visible local filesystem path identity.
- Ready results may include Pumas-approved load targets because those are
  host/planner handoff facts, not graph identity.
- Readiness request tests must stay path-free and proof-free; requirements ids,
  environment refs, package facts, and executable paths belong to host-produced
  result/handoff contracts.

## Decision
Use a small public integration test with JSON fixtures rather than broad
workspace tests in this crate. Broader cross-layer acceptance belongs in later
Milestone 5 slices after node-engine, host resolver, and frontend consumers
migrate to the shared contract.

## Alternatives Rejected
- Unit-only tests: rejected because the contract crosses serialized boundaries.
- Full graph execution tests here: rejected because this crate owns DTOs, not
  runtime behavior.

## Invariants
- Invalid Pumas model identity fails validation.
- Ready results require load targets.
- Non-ready results do not carry load targets.
- Pumas artifact entry paths reject local absolute paths.
- Readiness requests use typed policy enum values and reject unknown/path-shaped
  fields before host wiring consumes them.
- Readiness execution envelopes bind readiness input and preflight proof to
  active-run freshness identity without carrying paths, load targets, or raw
  provider request payloads into scheduler proof.
- Provider-source snapshots use canonical feature/toolchain/device-class ids
  and reject display-shaped, graph-shaped, shell-derived, or scheduler-ranking
  evidence fields.
- Inventory observation projections preserve bounded provider alternatives as
  typed per-binding status evidence without changing readiness state or
  selecting alternatives.

## Revisit Triggers
- Host-language generated schemas are added.
- Frontend or worker contracts consume these fixtures directly.
- New result states or diagnostic codes are introduced.

## Dependencies
**Internal:** `pantograph-dependency-planning`.

**External:** `serde_json`.

## Related ADRs
- None identified as of 2026-05-20.
- Reason: test placement follows the Milestone 5 contract-owner plan.
- Revisit trigger: dependency-planning contracts become generated schemas.

## Usage Examples
```bash
cargo test -p pantograph-dependency-planning
```

## API Consumer Contract
- Tests exercise public crate imports only.
- Failure messages should identify the broken fixture or invariant.

## Structured Producer Contract
- Fixture field names and enum values are part of the executable contract.
- Fixture changes must accompany corresponding DTO changes.
- Ready and unavailable examples intentionally cover both success and typed
  diagnostic flows.
- Readiness fixtures intentionally cover host input only; preflight result
  fixtures cover readiness proof after host processing.
- Readiness execution envelope fixtures cover scheduler-admission freshness and
  proof retention identity. They are not runtime-host handoff fixtures.
- Provider-source fixtures cover source facts consumed by future inventory
  providers. They are not provider behavior fixtures and do not authorize
  embedded-runtime to infer readiness from workflow-service, runtime-registry,
  graph, or shell-shaped fields.
- Inventory observation projection fixtures may carry bounded provider
  alternatives, but those alternatives are evidence only and must not change
  readiness state or trigger scheduler auto-selection.
