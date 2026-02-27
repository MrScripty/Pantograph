# Plan: Pantograph Dependency Integration (Pumas Metadata v2 Finalized Contract)

## Status
Implementation plan (updated for finalized Pumas dependency model)

## Objective
Implement dependency-aware workflow execution in Pantograph using finalized Pumas metadata/dependency contracts so model-backed nodes (starting with Stable Audio) run reliably on clean machines without bundling all model dependencies in base Pantograph install.

This plan assumes no backwards compatibility requirements.

## Scope
1. Align Pantograph contracts with Pumas Metadata v2 and multi-binding dependency APIs.
2. Replace single-profile dependency assumptions with binding-plan resolution.
3. Enforce dependency preflight gates before model-backed inference execution.
4. Update UI flows for per-binding dependency state, install, and manual intervention.
5. Validate end-to-end with Stable Audio workflow and at least one non-audio model.

## Out of Scope
1. Maintaining old Pantograph dependency command/DTO shapes.
2. Bundling all potential model dependencies into `./launcher.sh --install`.
3. Solving every custom-model edge case without manual-review fallback.

## Coding Standards Alignment
1. Follow layered architecture boundaries:
   - UI (`src/`) -> command/orchestration (`src-tauri/src/workflow/commands.rs`) -> resolver/service -> infrastructure API.
2. Treat backend as source of truth for dependency business state; frontend holds transient UI state only.
3. Validate contracts at boundaries (Tauri commands and node-engine model contracts), not repeatedly in deep internals.
4. Avoid adding dependencies unless justified under dependency standards.
5. Keep files within maintainable size targets and split by responsibility when needed.
6. Run verification layers in order: static analysis -> build -> dev server -> runtime verification.
7. Update relevant directory/docs documentation when structure/contracts change.

## Finalized Pumas Contract Assumptions
1. Metadata includes `dependency_bindings` projection; authoritative source remains Pumas dependency tables.
2. A model may have multiple active dependency bindings.
3. Resolver/API must be plan-based and deterministic (stable ordering + deterministic env key).
4. Non-ready dependency states include:
   - `unknown_profile`
   - `manual_intervention_required`
   - `profile_conflict`
5. `review_reasons` is a normalized list (`Vec<String>`), not a single `review_reason`.

## Current Gap Summary (Pantograph)
1. Pantograph currently uses single `dependency_profile_id`/`env_id` in runtime and UI contracts.
2. Tauri resolver currently performs local fallback heuristics and direct pip installs.
3. Node-engine dependency gate currently checks only legacy states.
4. UI only displays legacy dependency statuses and no per-binding plan details.
5. Workflow node data and `model_ref` payloads still encode single-profile dependency shape.

## Architecture Decisions
1. Pumas is dependency source of truth; Pantograph orchestrates.
2. Pantograph must request and execute dependency plans, not infer dependency requirements itself.
3. Dependency checks/installs are per-binding with deterministic environment identity.
4. Workflow execution blocks on all non-ready states unless explicit override policy exists.
5. Manual intervention is first-class and expected for ambiguous/custom models.

## Target Runtime Contracts (Pantograph)

### 1) Dependency Request Context
1. `model_id` (preferred) and `model_path` (fallback match key).
2. `node_type`, `backend_key`, and `platform_context`.
3. Optional `selected_binding_ids` for user-constrained install/check.

### 2) Dependency Plan Result
1. Ordered list of bindings from Pumas resolver:
   - `binding_id`
   - `profile_id`
   - `profile_version`
   - `profile_hash`
   - `binding_kind`
   - `backend_key`
   - `platform_selector`
2. Deterministic per-binding `env_id`:
   - `{environment_kind}:{profile_id}:{profile_version}:{profile_hash}:{platform_key}:{backend_key}`

### 3) Dependency Status/Install Result
1. Top-level `state`.
2. Per-binding status rows (`ready`, `missing`, `failed`, etc.).
3. Deterministic error codes including `required_binding_omitted`.

### 4) `model_ref` Contract (Pantograph Internal v2.1)
1. Keep existing required identity fields (`contract_version`, `engine`, `model_id`, `model_path`, `task_type_primary`).
2. Replace single dependency fields with binding-plan fields:
   - `dependency_bindings` (resolved subset used by execution)
   - optional `dependency_plan_id`/hash for traceability
3. Validation remains boundary-first; downstream assumes validated payloads.

## Components to Change

### 1) Node Engine (`crates/node-engine`)
1. Replace `ModelDependencyRequest`, `ModelDependencyStatus`, `ModelDependencyInstallResult`, and `DependencyState` with plan/binding-aware contracts.
2. Update `ModelDependencyResolver` trait:
   - `resolve_model_dependency_plan(...)`
   - binding-aware `check_dependencies(...)`
   - binding-aware `install_dependencies(...)`
   - `resolve_model_ref(...)` aligned with multi-binding output
3. Update preflight gate in `CoreTaskExecutor` to block on:
   - `unknown_profile`
   - `manual_intervention_required`
   - `profile_conflict`
   - `required_binding_omitted`

### 2) Tauri Resolver (`src-tauri/src/workflow/model_dependencies.rs`)
1. Remove local dependency authority behavior as primary path.
2. Integrate finalized Pumas dependency APIs as authoritative source.
3. Keep local inference/probe logic only as guarded fallback when Pumas dependency APIs are unavailable; fallback must return conservative non-ready states rather than guessing.
4. Implement per-binding caching keyed by model + platform + backend + selected bindings.

### 3) Tauri Commands (`src-tauri/src/workflow/commands.rs`)
1. Replace legacy commands/payloads with binding-plan command surface.
2. Add explicit command for plan resolution.
3. Return per-binding details and deterministic error codes to frontend.

### 4) Workflow Nodes (`crates/workflow-nodes`)
1. Update `puma-lib` options metadata to include `dependency_bindings` and `review_reasons`.
2. Stop relying on singular `dependency_profile_id` field.
3. Preserve minimal fallback task inference only when metadata is absent.

### 5) Frontend (`src/components/nodes/workflow/PumaLibNode.svelte`)
1. Update dependency request payload to include `backend_key`, `platform_context`, optional binding selections.
2. Render per-binding status list and top-level state.
3. Add UX for:
   - manual intervention required
   - profile conflict
   - required binding omitted
4. Surface `review_reasons[]` and remediation actions.
5. No optimistic updates for backend-owned dependency state.

### 6) Workflow Data Shape
1. Replace stored node data:
   - remove singular `dependency_profile_id`/`env_id` assumptions
   - store `dependency_bindings` and latest plan/status result
2. Keep execution graph deterministic by persisting selected bindings when user chooses subsets.

## Execution Phases

### Phase 1: Contract Refactor (Node Engine + Shared Types)
1. Introduce new plan/binding DTOs and states.
2. Update trait interfaces and compile callers.
3. Update preflight state handling and model_ref builder.

Exit criteria:
1. Node-engine compiles with no legacy dependency DTO usage.
2. Preflight blocks correct finalized non-ready states.

### Phase 2: Resolver + Command Migration (Tauri)
1. Implement authoritative Pumas API-backed resolver path.
2. Replace command handlers with new payload/result shapes.
3. Add deterministic cache keys for plan/status results.

Exit criteria:
1. Commands return per-binding responses end-to-end.
2. Local fallback does not silently auto-resolve ambiguous dependencies.

### Phase 3: Puma-Lib Node Metadata + UI Migration
1. Update puma-lib metadata extraction to dependency_bindings/review_reasons.
2. Update PumaLib node UI badges/actions and per-binding panels.
3. Add user selection flow for optional bindings where supported.

Exit criteria:
1. UI can inspect, check, and install per binding.
2. Manual intervention and conflicts are explicit and actionable.

### Phase 4: Preflight + Inference Integration Validation
1. Validate PyTorch and audio-generation nodes with new resolver contracts.
2. Ensure generated `model_ref` payloads are contract-valid and include resolved bindings.
3. Validate unload/execution consumers accept updated `model_ref`.

Exit criteria:
1. Inference cannot start when finalized non-ready states are returned.
2. Successful runs propagate resolved binding metadata through outputs.

### Phase 5: Acceptance and Hardening
1. Stable Audio first-run on clean machine:
   - plan resolution
   - missing dependencies surfaced
   - install and rerun succeeds
2. One second model family (LLM or diffusion) validates multi-model behavior.
3. Add telemetry/logs for plan selection, check/install, and gate failures.

Exit criteria:
1. Deterministic dependency behavior across repeated runs.
2. Actionable errors for all dependency failure classes.

## Testing and Verification
1. Unit tests:
   - DTO validation
   - state transitions
   - deterministic plan ordering
2. Integration tests:
   - resolve/check/install happy path
   - `unknown_profile`
   - `manual_intervention_required`
   - `profile_conflict`
   - `required_binding_omitted`
3. End-to-end workflow tests:
   - Stable Audio minimum example
   - at least one non-audio model

## Practical Implications for Pantograph
1. Data model churn: node data, resolver DTOs, and command payloads will change together; partial migration is fragile.
2. UI complexity increases: users now see binding-level dependency decisions instead of a single status badge.
3. Fewer hidden failures: ambiguous/custom dependency cases become explicit manual intervention instead of silent install guesses.
4. Better reproducibility: deterministic env identity and per-binding plan handling reduce “works once, breaks later” behavior.
5. Short-term implementation cost rises: refactor spans node-engine, tauri backend, and frontend simultaneously.
6. Long-term maintenance improves: Pantograph stops owning dependency inference logic and follows authoritative Pumas outputs.

## Completion Criteria
1. Pantograph uses finalized Pumas dependency contracts without legacy single-profile assumptions.
2. Preflight gating enforces all finalized non-ready states.
3. Stable Audio workflow runs end-to-end with dependency install/check from UI.
4. At least one non-audio workflow confirms multi-binding architecture readiness.
5. Dependency failures are deterministic, structured, and actionable.
