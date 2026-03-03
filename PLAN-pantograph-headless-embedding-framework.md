# Plan: Pantograph Refactor to Headless Embedding Framework API

## Status
Implemented (headless embedding v1 contract + service + Tauri/UniFFI/Rustler adapters)

## Objective
Refactor Pantograph workflow execution boundaries so headless embedding can be consumed as a stable Rust-first framework API (`embed_objects_v1`) with optional transport adapters, while bringing architecture into compliance with project coding standards.

## Scope

### In Scope
- Extract workflow application/use-case logic out of Tauri command modules into a host-agnostic service layer.
- Keep `node-engine` and `workflow-nodes` as framework/domain layers and reduce Tauri to thin transport + host resource adapter.
- Define and implement versioned headless embedding API contracts:
  - `embed_objects_v1`
  - `get_embedding_workflow_capabilities_v1`
- Add Rust-first API surface plus optional Tauri/IPC adapter wrappers.
- Add contract tests, integration tests, and one official Rust host example.
- Add migration/deprecation path for existing workflow execution commands.

### Out of Scope
- Full redesign of all workflow node types.
- Removing Tauri desktop support.
- Breaking all existing command consumers in a single change.
- HTTP server productionization beyond optional adapter scaffolding.

## Inputs

### Problem
Pantograph currently has core workflow capabilities but key execution use-cases are implemented in Tauri command modules, making headless external embedding integration unreliable and contract-unstable.

### Constraints
- Follow coding standards layering and service-independence rules.
- Preserve existing desktop behavior while refactoring.
- Deliver changes as atomic commits with passing verification at each step.
- Keep API changes versioned and additive where possible.

### Assumptions
- `node-engine` remains the domain framework runtime.
- Host-specific concerns (Tauri channel, app state, process/runtime wiring) stay in adapters.
- The initial headless API targets embedding workflows first, not all workflow categories.

### Dependencies
- `node-engine` and `workflow-nodes` crates.
- `src-tauri/src/workflow/*` command and adapter modules.
- Existing model dependency resolution (`model_ref` v2 and resolver wiring).
- Rustler/UniFFI crates for non-Tauri host surfaces.

### Risks
| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Refactor introduces regressions in desktop workflow execution | High | Add parity tests before and after extraction; keep compatibility wrapper commands during migration |
| Contract drift across Tauri, UniFFI, Rustler surfaces | High | Freeze shared request/response DTOs in one crate and reuse everywhere |
| Model signature fields not reliably available | High | Define canonical source chain and explicit fallback behavior; fail fast when required signature fields missing |
| Long-running refactor scope creep | Medium | Enforce milestone gates and atomic commit boundaries with no mixed-scope commits |
| Partial failure semantics conflict with current fail-fast behavior | Medium | Implement object-level result aggregation in service layer, not event parsing |

## Definition of Done
- Workflow application logic is host-agnostic and no longer implemented primarily inside Tauri commands.
- Tauri workflow commands are thin adapters over shared service APIs.
- `embed_objects_v1` returns deterministic structured results with model signature and per-object status.
- `get_embedding_workflow_capabilities_v1` is implemented and versioned.
- Contract tests enforce request/response schema and compatibility guarantees.
- One official Rust host example integration is added and verified.
- Documentation updated with architecture boundaries and migration notes.

## Target Architecture
- Layer 1 (Domain/Framework): `crates/node-engine`, `crates/workflow-nodes`, shared contract types.
- Layer 2 (Application Services): new host-agnostic workflow application crate/module (use-case orchestration).
- Layer 3 (Host Adapters):
  - Tauri commands (`src-tauri`) as transport adapters.
  - UniFFI/Rustler wrappers reusing the same service contracts.

## Milestones

### Milestone 1: Freeze Contracts and Refactor Boundary

**Goal:** Define immutable embedding API contracts and explicit layer boundaries before code movement.

**Tasks:**
- [ ] Add `docs/headless-embedding-api-v1.md` with frozen request/response schema and versioning rules.
- [ ] Add architecture decision note describing service extraction and adapter responsibilities.
- [ ] Define model signature resolution policy (`model_id`, revision/hash, backend, dimensions) and required/optional fields.

**Verification:**
- Documentation lint/check as used by repository tooling.
- Review checklist confirms no implementation begins before contract freeze.

**Status:** Complete

### Milestone 2: Extract Host-Agnostic Workflow Application Service

**Goal:** Move workflow execution use-case orchestration out of Tauri command modules.

**Tasks:**
- [ ] Create new module/crate (for example `crates/pantograph-workflow-service`) with:
  - execution request DTOs
  - execution result DTOs
  - orchestrator service traits for host resources
- [ ] Move non-transport logic from `workflow_execution_commands.rs` into service layer.
- [ ] Keep host dependencies behind injected traits/interfaces.

**Verification:**
- `cargo check --workspace`
- Existing workflow execution tests pass or are ported with parity assertions.

**Status:** Complete

### Milestone 3: Thin Tauri Adapter Migration

**Goal:** Reduce `src-tauri` workflow commands to adapter-only behavior.

**Tasks:**
- [ ] Update Tauri commands to map request/response and delegate into shared service.
- [ ] Keep `TauriTaskExecutor` strictly host-resource implementation.
- [ ] Preserve backward-compatible command signatures where needed; mark deprecated surfaces.

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Command-level integration tests pass.

**Status:** Complete

### Milestone 4: Implement Headless Embedding Service API

**Goal:** Add first-class headless embedding operations in shared service layer.

**Tasks:**
- [ ] Implement `embed_objects_v1` service method with:
  - deterministic object order preservation
  - `object_id` correlation
  - per-object success/failure status
  - batch timing and run identifier
- [ ] Implement `get_embedding_workflow_capabilities_v1`.
- [ ] Add idempotency/correlation handling for optional `batch_id`.

**Verification:**
- Unit tests for ordering, correlation, partial failure handling, and idempotency behavior.
- `cargo test -p <service-crate>`

**Status:** Complete

### Milestone 5: Model Signature Reliability

**Goal:** Make model signature non-empty and stable for successful embedding responses.

**Tasks:**
- [ ] Implement canonical signature resolver using existing model contracts and backend metadata.
- [ ] Populate `model_signature` in every success response.
- [ ] Define hard failure behavior when signature cannot be established deterministically.

**Verification:**
- Tests asserting signature presence for all success cases.
- Tests for explicit failure path when signature requirements are unmet.

**Status:** Complete

### Milestone 6: Adapter Surfaces Beyond Tauri

**Goal:** Expose headless API to external Rust consumers and optional bindings consistently.

**Tasks:**
- [ ] Add Rust-native public API entrypoints (library-first).
- [ ] Add optional adapter methods in UniFFI and/or Rustler that delegate to same service contracts.
- [ ] Ensure no duplicated business logic in adapters.

**Verification:**
- Binding crate compile checks.
- Contract parity tests across at least Rust + one adapter surface.

**Status:** Complete

### Milestone 7: Contract Tests and Compatibility Guardrails

**Goal:** Enforce versioning guarantees and prevent accidental schema regressions.

**Tasks:**
- [ ] Add golden contract tests for `embed_objects_v1` and capabilities response.
- [ ] Add compatibility test policy (additive-only fields for v1 unless version bump).
- [ ] Add CI gate for contract snapshots.

**Verification:**
- Contract test suite in CI.
- Snapshot update process documented.

**Status:** Complete

### Milestone 8: Official Rust Example and Migration Documentation

**Goal:** Provide implementation-ready guidance for embedding Pantograph into host apps.

**Tasks:**
- [ ] Add `examples/` Rust host app demonstrating `embed_objects_v1` end-to-end.
- [ ] Add migration doc mapping legacy workflow commands to headless API.
- [ ] Update relevant `README.md` files for layer ownership and adapter responsibilities.

**Verification:**
- Example build and run smoke test.
- Docs review checklist complete.

**Status:** Complete

## Atomic Commit Sequence

1. `docs(architecture): freeze headless embedding api v1 contract and boundaries`
- Add contract doc + ADR note.
- No runtime code changes.

2. `refactor(workflow): introduce host-agnostic workflow service interfaces`
- Scaffold service crate/module and DTOs.
- Compile-only wiring.

3. `refactor(workflow): move execution orchestration from tauri commands into service`
- Port logic with behavior parity tests.

4. `refactor(tauri): make workflow commands thin adapters over shared service`
- Tauri command layer delegation only.

5. `feat(api): implement embed_objects_v1 service operation`
- Core embedding operation with per-object result aggregation.

6. `feat(api): implement get_embedding_workflow_capabilities_v1`
- Capability endpoint and validation.

7. `feat(api): enforce stable model_signature in success responses`
- Signature resolver + failure semantics.

8. `feat(bindings): expose headless embedding api through rust-first surface`
- Public Rust API entrypoint + adapter integration.

9. `test(contract): add v1 contract snapshots and compatibility guards`
- Golden tests and CI checks.

10. `docs(example): add official rust host embedding integration example`
- Example app + migration docs.

## Verification Matrix by Milestone
- Static checks: `cargo check --workspace`
- Unit/integration: targeted `cargo test` by crate/module
- Contract tests: snapshot/golden suite
- Adapter checks: Tauri + selected binding compile tests
- Example smoke: build/run official Rust integration example

## Re-Plan Triggers
- Contract freeze changes after Milestone 1.
- Any adapter requires business logic duplication to proceed.
- Model signature cannot be made deterministic from available runtime metadata.
- Verification failures requiring architectural rollback.

## Recommendations
- Create the service layer as a separate crate to make dependency boundaries mechanically enforceable.
- Treat Tauri, UniFFI, and Rustler as peers over one shared API contract; avoid introducing a second “headless-only” logic path.
- Keep legacy command wrappers temporarily, but route them through new service immediately to avoid dual maintenance.

## Execution Notes
- Start implementation only after contract and boundary freeze is merged.
- Keep each commit single-purpose and independently buildable.
- Do not batch milestone-crossing changes into one commit.

## Completion Summary

### Completed
- Milestone 1 contract freeze docs and architecture ADR.
- Milestone 2 host-agnostic workflow service crate scaffolding and DTO/trait contracts.
- Milestone 3 Tauri command adapter integration for headless embedding API.
- Milestone 4 `embed_objects_v1` and capabilities service operations.
- Milestone 5 model signature validation and non-empty success guarantees.
- Milestone 6 adapter parity across UniFFI and Rustler wrappers delegating to shared service contracts.
- Milestone 7 contract tests for v1 request/response shapes.
- Milestone 7 CI gate for contract checks and binding parity compile checks.
- Milestone 8 official Rust host example and migration guide.

### Deviations
- None.

### Follow-Ups
- None for the v1 scope in this plan.

### Verification Summary
- `cargo test -p pantograph-workflow-service --test contract_v1`
- `cargo check -p pantograph-uniffi`
- `cargo check -p pantograph_rustler`
- `cargo test -p pantograph-uniffi --no-run`
- `cargo test -p pantograph-uniffi test_get_embedding_workflow_capabilities_v1_contract_success -- --nocapture`
- `cargo test -p pantograph-uniffi test_parse_embedding_payload_rejects_non_numeric -- --nocapture`

### Traceability Links
- API contract: `docs/headless-embedding-api-v1.md`
- Service boundary ADR: `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- Migration guide: `docs/headless-embedding-migration.md`
- Service contract tests: `crates/pantograph-workflow-service/tests/contract_v1.rs`
- CI gate: `.github/workflows/headless-embedding-contract.yml`
