# Actionable Implementation Plan: Pantograph x Pumas Metadata v2 Dependency Cutover

## Status
Historical precursor

## Source Of Truth Status

This document is a historical precursor for the earlier Pumas v2 multi-binding
cutover framing.

The codebase later converged on a narrower completed resolve-only baseline in
`docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-resolve-only-hard-cutover.md`,
with the remaining open work tracked in
`IMPLEMENTATION-PLAN-pantograph-dependency-environment-node.md` and
`IMPLEMENTATION-PLAN-pantograph-pumas-pinning-update.md`.


## Goal
Cut Pantograph over from legacy single-profile dependency handling to finalized Pumas v2 multi-binding dependency contracts, with Stable Audio as first acceptance workflow.

## Constraints
1. No backward compatibility required.
2. Pumas dependency APIs are authoritative.
3. Pantograph must not guess unresolved dependencies.
4. Coding standards in `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/` are mandatory release gates.

## Definition of Done
1. Pantograph uses plan/binding-aware dependency contracts end-to-end.
2. Preflight blocks execution on `unknown_profile`, `manual_intervention_required`, `profile_conflict`, and `required_binding_omitted`.
3. UI shows per-binding dependency status and remediation actions.
4. Stable Audio workflow runs successfully on a clean machine via on-demand install.
5. One non-audio model path validates the same architecture.
6. Work passes layered architecture, boundary validation, dependency, testing, and documentation standards checks.

## Standards Compliance Gates
1. Layering:
   - Presentation (`src/`) must not call infrastructure directly.
   - Tauri commands/controllers orchestrate only; business rules stay in resolver/service layer.
2. Backend-owned data:
   - Dependency business state is backend-owned.
   - Frontend may only hold transient UI state; no optimistic updates for dependency results.
3. Boundary validation:
   - Validate inbound/outbound payloads at Tauri command boundaries and node-engine contract boundaries.
4. Dependency management:
   - No new third-party packages unless justified against `DEPENDENCY-STANDARDS.md`.
   - Prefer standard library/in-house solution unless justified.
5. File organization:
   - Keep touched files under 500 lines where feasible; split by responsibility when exceeded.
6. Documentation:
   - Update/add directory `README.md` files when structure or responsibilities change.
   - Document public contracts and non-obvious algorithms.
7. Testing:
   - Follow verification layers order: static analysis -> build -> dev server -> runtime verification.

## Workstream 0: Contract Freeze (Day 0)
Owner: Integration lead

Tasks:
1. Confirm final Pumas API request/response JSON shapes for:
   - `resolve_model_dependency_plan`
   - `check_model_dependencies`
   - `install_model_dependencies`
2. Confirm canonical error codes (at minimum):
   - `unknown_profile`
   - `manual_intervention_required`
   - `profile_conflict`
   - `required_binding_omitted`
3. Freeze Pantograph internal DTO names and field names before coding.
4. Record layering boundaries (UI -> commands -> resolver/service -> infrastructure API client) and make them explicit in the memo.

Output:
1. Short contract memo in repo (`docs/pumas-v2-contract-freeze.md`).

Gate:
1. No implementation starts until this is complete.

## Workstream 1: Node-Engine Contract Refactor (Days 1-2)
Owner: Runtime/backend engineer

Primary files:
1. `crates/node-engine/src/model_dependencies.rs`
2. `crates/node-engine/src/core_executor.rs`
3. `crates/node-engine/src/lib.rs`

Tasks:
1. Replace legacy dependency DTOs with binding-plan DTOs:
   - request context includes `backend_key`, `platform_context`, optional `selected_binding_ids`.
   - status/install results include per-binding rows.
2. Expand `DependencyState` enum to finalized states.
3. Update `ModelDependencyResolver` trait to include:
   - `resolve_model_dependency_plan(...)`
   - binding-aware `check_dependencies(...)`
   - binding-aware `install_dependencies(...)`
4. Update preflight gate in `core_executor`:
   - parse per-binding results
   - block on finalized non-ready states
   - include structured error payload for UI.
5. Update `ModelRef` internal contract to carry resolved binding data (not single profile).
6. Ensure contract validation happens at boundary APIs only; internal logic consumes typed validated structs.

Acceptance:
1. `cargo build` passes for node-engine crates.
2. Unit tests added for state handling and model_ref validation.
3. No new framework coupling introduced into domain/service logic.

## Workstream 2: Tauri Resolver Migration (Days 2-4)
Owner: Tauri/backend engineer

Primary files:
1. `src-tauri/src/workflow/model_dependencies.rs`
2. `src-tauri/src/workflow/mod.rs` (if exports/types are updated)

Tasks:
1. Replace local-authority resolver logic with Pumas-authority flow:
   - call Pumas plan resolver/check/install APIs.
2. Keep local pip/import probing only as guarded fallback when Pumas dependency APIs are unavailable.
3. Ensure fallback returns conservative states (`manual_intervention_required` / `unknown_profile`) instead of speculative installs.
4. Add cache keys including:
   - model id/path
   - backend key
   - platform context
   - selected binding set.
5. Return deterministic per-binding payloads to callers.
6. Keep resolver logic framework-agnostic where possible (separate pure mapping/decision helpers from Tauri command glue).

Acceptance:
1. Resolver can resolve/check/install using Pumas APIs.
2. Resolver never silently succeeds on ambiguous dependency cases.
3. Resolver remains the business-logic layer; Tauri commands remain orchestration only.

## Workstream 3: Tauri Command Surface Cutover (Days 3-4)
Owner: Tauri/backend engineer

Primary files:
1. `src-tauri/src/workflow/commands.rs`
2. `src-tauri/src/main.rs`

Tasks:
1. Replace legacy command argument signatures with binding-plan signatures.
2. Add explicit plan resolution command:
   - `resolve_model_dependency_plan`.
3. Update `check_model_dependencies` and `install_model_dependencies` command signatures:
   - include `backend_key`, `platform_context`, optional `selected_binding_ids`.
4. Remove/replace commands that only make sense for legacy single-profile status.
5. Keep invoke handler registration in sync.
6. Validate inbound/outbound payload shapes at command boundary.

Acceptance:
1. Frontend can call new commands without shape mismatches.
2. Commands return per-binding structured details and deterministic errors.
3. Boundary error messages are actionable and deterministic.

## Workstream 4: Workflow Node Metadata Alignment (Days 4-5)
Owner: Workflow/runtime engineer

Primary files:
1. `crates/workflow-nodes/src/input/puma_lib.rs`
2. `crates/node-engine/src/core_executor.rs` (input extraction paths)

Tasks:
1. Update Puma-Lib option metadata payload:
   - add `dependency_bindings`
   - add `review_reasons`
   - stop depending on singular `dependency_profile_id`.
2. Update task-type extraction to prioritize Pumas v2 metadata fields.
3. Ensure inference nodes receive dependency context needed for preflight.

Acceptance:
1. Puma-Lib node option metadata reflects finalized Pumas contract.
2. Inference node execution context includes dependency plan inputs.

## Workstream 5: Frontend Node UX Migration (Days 5-7)
Owner: Frontend engineer

Primary files:
1. `src/components/nodes/workflow/PumaLibNode.svelte`
2. `src/stores/workflowStore.ts` (or relevant stores)
3. `src/services/workflow/types.ts` (if command DTO typings live here)

Tasks:
1. Replace legacy dependency state model with new finalized states.
2. Add per-binding status rendering:
   - binding id / backend / profile version
   - state / missing components / message.
3. Add user controls:
   - resolve plan
   - check dependencies
   - install dependencies
   - optional binding selection (where applicable).
4. Add remediation UX paths:
   - `manual_intervention_required`
   - `profile_conflict`
   - `required_binding_omitted`.
5. Surface `review_reasons[]` clearly.
6. Do not optimistically mutate backend-owned dependency state; update view only from backend command responses.

Acceptance:
1. UI no longer assumes single dependency profile.
2. UI shows actionable guidance per binding and state.
3. UI state ownership follows backend-owned data standard.

## Workstream 6: End-to-End Gate and ModelRef Consumers (Day 7)
Owner: Runtime engineer

Primary files:
1. `crates/node-engine/src/core_executor.rs`
2. Any nodes consuming `model_ref` (including unload paths)

Tasks:
1. Verify all model-backed node paths use updated preflight.
2. Verify all `model_ref` consumers can parse updated binding-aware model_ref payload.
3. Ensure unload/cleanup logic still works with updated model_ref fields.

Acceptance:
1. No runtime path relies on removed single-profile fields.
2. Execution errors are structured and UI-readable.

## Workstream 7: Test Matrix and Verification (Days 7-8)
Owner: QA + engineers

Required tests:
1. Unit:
   - DTO validation
   - dependency state transitions
   - deterministic binding plan ordering.
2. Integration:
   - resolve/check/install happy path
   - `unknown_profile`
   - `manual_intervention_required`
   - `profile_conflict`
   - `required_binding_omitted`.
3. E2E:
   - Stable Audio: missing -> install -> ready -> execute -> audio output.
   - One non-audio model path with same dependency gate behavior.
4. Verification layers (must run in order):
   - static analysis (`cargo check`, TS checks)
   - build verification
   - dev server validation
   - runtime verification

Acceptance:
1. All tests pass in CI/local equivalent.
2. No silent fallback to legacy behavior.
3. Verification layers completed and logged in PR checklist.

## Workstream 8: Documentation and Standards Wrap-Up (Day 8)
Owner: Tech lead + reviewers

Tasks:
1. Update docs for new dependency contract surfaces and command payloads.
2. Ensure touched directories with non-obvious purpose or 3+ files have updated `README.md`.
3. Add/refresh ADR if architecture decisions changed materially.
4. Confirm no orphaned TODOs introduced.

Acceptance:
1. Documentation standards checklist passes.
2. PR includes explicit coding-standards compliance notes.

## Cutover Checklist
1. Remove legacy single-profile fields from active runtime paths.
2. Remove stale UI badges/states that do not exist in finalized contract.
3. Confirm command list in `main.rs` only exposes current dependency command surface.
4. Update internal docs:
   - `docs/historical-plans/PLAN-pantograph-model-dependency-integration.md`
   - any frontend dev docs for Puma-Lib node behavior.
5. Confirm dependency additions (if any) include explicit standards justification.
6. Confirm directory documentation updates are complete.

## Practical Implementation Implications
1. This is a cross-layer cutover; branch strategy should avoid long-lived divergence between backend and frontend DTOs.
2. Intermediate builds may break while contracts are transitioning; sequence Workstreams 1-3 tightly.
3. Stable Audio is a valid first target, but architecture now covers LLM/custom models by design through binding plans.
4. Manual intervention is expected behavior for ambiguous models, not a bug.

## Suggested Execution Order (Strict)
1. Workstream 0
2. Workstream 1
3. Workstream 2
4. Workstream 3
5. Workstream 4
6. Workstream 5
7. Workstream 6
8. Workstream 7
9. Workstream 8
10. Cutover Checklist
