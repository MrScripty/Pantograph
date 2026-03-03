# Plan: Workflow Capability Refactor Out of Tauri

## Objective

Refactor workflow capability computation so runtime requirement logic lives in
backend/core service code only, with Tauri/UniFFI/Rustler reduced to transport
and dependency wiring adapters.

## Scope

### In Scope

- Move `workflow_get_capabilities` runtime requirement computation out of:
  - `src-tauri/src/workflow/headless_workflow_commands.rs`
  - `crates/pantograph-uniffi/src/lib.rs`
  - `crates/pantograph-rustler/src/lib.rs`
- Centralize workflow graph loading/validation and runtime requirement
  extraction in backend service code.
- Keep existing API names (`workflow_run`, `workflow_get_capabilities`) and
  response contract shape (`runtime_requirements`).
- Remove duplicated capability logic across adapters.

### Out of Scope

- UI workflow editor features.
- New transport protocols.
- Backward-compatibility shims for pre-refactor internals.
- Broader workflow CRUD API expansion beyond capability/runtime refactor.

## Inputs

### Problem

Runtime requirement computation currently sits in adapter layers (including
Tauri), violating layered separation and service independence standards.

### Constraints

- Breaking internal refactor is allowed.
- Backend must be able to run headless without Tauri dependencies.
- Contract semantics for external consumers must remain deterministic.
- Adapters must remain thin (transport mapping + dependency injection only).

### Assumptions

- Workflow definitions remain JSON files under discoverable workflow roots.
- `node_engine::validation::validate_workflow` remains the graph validation
  authority.
- Model size metadata is available through Pumas metadata keys used today.

### Dependencies

- `crates/pantograph-workflow-service`
- `crates/node-engine`
- `pumas_library::PumasApi` for model metadata lookup
- Existing headless contract tests in `crates/pantograph-workflow-service/tests`

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Trait redesign breaks adapter compilation | High | Introduce changes in one slice with all adapters updated in same commit |
| Behavior drift in capability output fields | High | Keep existing response schema; add parity tests against previous snapshots |
| File/path loading assumptions differ by host runtime | Med | Centralize loader policy and test roots resolution with unit tests |
| Memory estimate accuracy differences after centralization | Med | Preserve current estimation algorithm first; improve later behind explicit confidence labels |

## Definition of Done

- No runtime requirement computation logic remains in Tauri/UniFFI/Rustler
  adapters.
- Backend service owns capability orchestration and computation.
- Adapters only:
  - decode/encode transport contracts
  - provide infrastructure dependencies through traits/ports
  - call service methods
- Contract tests and adapter parity checks pass.
- Documentation/ADR updated to reflect the final boundary.

## Milestones

### Milestone 1: Freeze Core Boundaries

**Goal:** Define backend-owned ports for capability computation.

**Tasks:**
- [ ] Add service-level ports/interfaces for:
  - workflow definition loading/validation
  - backend identity resolution
  - model metadata lookup
  - capability limits
- [ ] Remove adapter-owned `compute_runtime_requirements` responsibility from
  service trait surface (breaking internal API).
- [ ] Update service docs/comments to reflect strict boundary ownership.

**Verification:**
- `cargo check -p pantograph-workflow-service`
- Service unit tests compile.

**Status:** Not started

### Milestone 2: Implement Backend Runtime Requirements Module

**Goal:** Centralize extraction and estimation in backend/core.

**Tasks:**
- [ ] Create service module(s) for:
  - workflow file loading and graph validation
  - model/backend/extension extraction
  - RAM/VRAM estimation and confidence labeling
- [ ] Keep algorithm parity with current behavior.
- [ ] Add focused unit tests for extraction and estimation edge cases.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- `cargo test -p pantograph-workflow-service --test contract`

**Status:** Not started

### Milestone 3: Convert Tauri Adapter to Thin Transport

**Goal:** Remove business logic from Tauri command module.

**Tasks:**
- [ ] Delete workflow parsing/validation/estimation helpers from
  `src-tauri/src/workflow/headless_workflow_commands.rs`.
- [ ] Wire Tauri host dependencies into new service ports only.
- [ ] Keep Tauri command surface unchanged.

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Existing workflow command compile path passes.

**Status:** Not started

### Milestone 4: Adapter Parity for UniFFI and Rustler

**Goal:** Apply same thin-adapter model to language bindings.

**Tasks:**
- [ ] Remove duplicated workflow loading/extraction/estimation from:
  - `crates/pantograph-uniffi/src/lib.rs`
  - `crates/pantograph-rustler/src/lib.rs`
- [ ] Wire bindings to shared backend service capability path.
- [ ] Ensure outputs match Tauri/service behavior.

**Verification:**
- `cargo check -p pantograph-uniffi`
- `cargo check -p pantograph_rustler`
- `cargo test -p pantograph-uniffi test_workflow_get_capabilities_contract_success -- --nocapture`

**Status:** Not started

### Milestone 5: Contract and Documentation Closure

**Goal:** Lock in architecture and prevent regression into adapter coupling.

**Tasks:**
- [ ] Add/expand contract tests for capability parity and validation behavior.
- [ ] Update:
  - `docs/adr/ADR-001-headless-embedding-service-boundary.md`
  - `docs/headless-embedding-api-v1.md`
  - adapter READMEs if interfaces changed
- [ ] Add a CI check covering service contract + adapter compile parity.

**Verification:**
- `cargo test -p pantograph-workflow-service --test contract`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check -p pantograph-uniffi`
- `cargo check -p pantograph_rustler`

**Status:** Not started

## Execution Notes

Update during implementation:
- 2026-03-03: Plan created to remove capability/runtime logic from adapter
  layers and enforce backend-owned orchestration.

## Commit Cadence Notes

- Commit when a milestone slice is complete and verified.
- Keep commits atomic and reversible.
- Follow `COMMIT-STANDARDS.md` format and cleanup requirements.

### Planned Atomic Commit Sequence

1. `refactor(workflow-service): define capability dependency ports`
2. `feat(workflow-service): centralize runtime requirement computation`
3. `refactor(tauri): remove capability logic from adapter`
4. `refactor(bindings): remove duplicated capability logic`
5. `test(workflow): add capability parity coverage`
6. `docs(architecture): document backend-owned capability boundary`

## Re-Plan Triggers

- Service port design cannot represent one adapter without introducing
  framework-specific coupling.
- Capability outputs diverge from existing contract expectations.
- Workflow source/storage assumptions change (non-filesystem source introduced).
- Any milestone fails verification and requires sequencing changes.

## Recommendations (Only If Better Option Exists)

- Recommendation: add a small shared crate/module boundary for workflow
  capability internals (loader + extractor + estimator) under
  `pantograph-workflow-service` instead of embedding logic in large adapter
  files. This improves testability and reduces regression risk.

## Completion Summary

### Completed

- N/A (planning only)

### Deviations

- None yet.

### Follow-Ups

- Execute milestones in dependency order with atomic commits.

### Verification Summary

- N/A (planning only)

### Traceability Links

- Module README updated: N/A
- ADR added/updated: pending during Milestone 5
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: pending

## Brevity Note

Plan intentionally focuses on boundary ownership, sequencing, and verification.
Implementation detail depth should expand only if milestone risk increases.
