# Plan: Workflow Model Inventory and Core Sessions

## Objective

Add generic `models[]` capability inventory and core scheduler-managed workflow
sessions without coupling logic to Tauri or other adapters.

## Scope

### In Scope

- Extend core workflow capability contract with `models[]`.
- Add core session APIs in `pantograph-workflow-service`:
  - `create_workflow_session`
  - `run_workflow_session`
  - `close_workflow_session`
- Add scheduler/session errors:
  - `session_not_found`
  - `session_evicted`
  - `scheduler_busy`
- Implement adapter parity for Tauri, UniFFI, Rustler.
- Add contract and behavior tests.

### Out of Scope

- Consumer-owned runtimes.
- UI redesign of session flows.
- Full scheduler preemption engine beyond bounded admission + eviction.

## Inputs

### Problem

Consumers need deterministic model compatibility metadata and repeat-run
sessions with Pantograph-owned resource control.

### Constraints

- Backend/service must own business logic.
- Adapters stay thin.
- Breaking contract change is allowed.

### Assumptions

- Pumas `ModelRecord` provides `model_type` and `hashes`.
- Missing hashes are treated as integrity failures and must fail closed.

### Dependencies

- `crates/pantograph-workflow-service`
- `src-tauri/src/workflow/headless_workflow_commands.rs`
- `crates/pantograph-uniffi/src/lib.rs`
- `crates/pantograph-rustler/src/lib.rs`

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Contract break across adapters | High | Implement in service first, then adapter parity in one slice |
| Session state regressions | High | Add lifecycle tests (create/run/close/evicted/busy) |
| Ambiguous model role mapping | Medium | Keep deterministic derivation and allow empty `roles` |

## Definition of Done

- `workflow_get_capabilities` returns deterministic `models[]` inventory.
- Model hash/type are populated from host model records when available.
- Core service exposes session APIs and stateful scheduler behavior.
- Session APIs are available in Tauri/UniFFI/Rustler adapters.
- Tests cover multi-model inventory, nullable fields, and session eviction/busy behavior.

## Milestones

### Milestone 1: Contract and Capability Model Inventory

**Goal:** Extend core DTOs and capability computation.

**Tasks:**
- [x] Add `models[]` capability types to service contracts.
- [x] Derive `node_ids` and `roles` from workflow graph usage.
- [x] Add deterministic hash selection (`sha256` then `blake3`).
- [x] Update capability contract tests.

**Verification:**
- `cargo test -p pantograph-workflow-service --test contract`
- `cargo test -p pantograph-workflow-service`

**Status:** Completed

### Milestone 2: Core Session and Scheduler APIs

**Goal:** Implement service-owned session lifecycle.

**Tasks:**
- [x] Add session request/response DTOs and methods.
- [x] Add scheduler admission/eviction and busy signaling.
- [x] Add new service errors (`session_not_found`, `session_evicted`, `scheduler_busy`).
- [x] Add unit tests for lifecycle and error paths.

**Verification:**
- `cargo test -p pantograph-workflow-service`

**Status:** Completed

### Milestone 3: Adapter Parity

**Goal:** Expose new contract and sessions consistently across adapters.

**Tasks:**
- [x] Tauri adapter parity for new capabilities and session methods.
- [x] UniFFI parity for new capabilities and session methods.
- [x] Rustler parity for new capabilities and session methods.
- [x] Update adapter error mapping for new service errors.

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check -p pantograph-uniffi`
- `cargo check -p pantograph_rustler`
- `cargo test -p pantograph-uniffi test_workflow_get_capabilities_contract_success -- --nocapture`

**Status:** Completed

### Milestone 4: Documentation and Migration Notes

**Goal:** Publish contract changes and scheduler semantics.

**Tasks:**
- [x] Update headless API contract docs.
- [x] Update migration notes for session API direction.
- [x] Update ADR status notes for this extension.

**Verification:**
- Manual doc consistency check against contract tests.

**Status:** Completed

## Commit Cadence Notes

- Commit each completed milestone slice after verification.
- Follow conventional commit format and atomicity rules.

## Re-Plan Triggers

- Pumas model record data is unavailable for hashes/type in target runtime.
- Session admission/eviction semantics require cross-crate scheduler extraction.
- Adapter API constraints force contract shape adjustments.
