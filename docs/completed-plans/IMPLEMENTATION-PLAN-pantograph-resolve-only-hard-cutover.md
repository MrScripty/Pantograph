# Actionable Implementation Plan: Pantograph Resolve-Only Hard Cutover

## Status
Complete

## Current Source-of-Truth Summary

This document records the completed resolve-only dependency cutover baseline.
Pantograph now uses `resolve_model_dependency_requirements` as the dependency
requirements authority and owns the downstream readiness/check/install/runtime
lifecycle behavior itself.

Remaining dependency work now belongs to:
- `IMPLEMENTATION-PLAN-pantograph-dependency-environment-node.md` for
  environment lifecycle/manual override execution
- `IMPLEMENTATION-PLAN-pantograph-pumas-pinning-update.md` for remaining
  additive pin-awareness follow-on work


## Goal
Cut Pantograph to Pumas resolve-only dependency contracts and make Pantograph the sole owner of dependency check/install/runtime readiness.

## Constraints
1. No backward compatibility layer.
2. No migration support workflow.
3. No compatibility shim for removed Pumas methods.
4. Pumas resolver contract is authoritative for dependency requirements only.
5. Pantograph owns check/install/lifecycle state end-to-end.

## Definition of Done
1. Pantograph calls only `resolve_model_dependency_requirements`.
2. Pantograph rejects contract versions other than `dependency_contract_version == 1`.
3. Pantograph performs dependency check/install using resolver payload only.
4. Workflow preflight blocks execution unless dependency lifecycle is `ready`.
5. Stable-audio path succeeds from clean environment.
6. No Pantograph runtime code references removed Pumas APIs.

## Mandatory Commit Protocol (Apply After Every Completed Task)
1. Use one atomic commit per completed task.
2. Commit message must follow:
   - `<type>(<scope>): <imperative description>`
   - Keep description specific and under 72 chars when possible.
3. Recommended non-interactive sequence:
```bash
git status
git add <task-files...>
git diff --cached
# run task-appropriate checks from the matrix below
git commit -m "<type>(<scope>): <description>" -m "<why/approach>" -m "Agent: codex"
```
4. Commit type guidance:
   - `feat` new behavior
   - `fix` bug/incorrect behavior
   - `refactor` structure-only change
   - `docs` documentation-only change
   - `test` tests-only change
   - `chore` tooling/config/build changes

## Verification Matrix (Run Before Task Commit)
1. Rust backend changes:
```bash
cargo check -p node-engine
cargo check --manifest-path src-tauri/Cargo.toml
```
2. Frontend TypeScript/Svelte changes:
```bash
npm run typecheck
```
3. Frontend behavior changes:
```bash
npm run build
```
4. Cross-layer changes (Rust + frontend/IPC):
```bash
cargo check -p node-engine
cargo check --manifest-path src-tauri/Cargo.toml
npm run typecheck
npm run build
```

## Workstream 0: Freeze Pantograph Internal Contract
Owner: Integration lead

Tasks:
1. Define and freeze Pantograph-internal resolver DTOs for contract v1.
2. Define typed error model for contract mismatch and payload validation failure.
3. Record backend/frontend ownership boundaries for dependency lifecycle state.

Commit on completion:
```bash
git add src-tauri/src/workflow/model_dependencies.rs crates/node-engine/src/model_dependencies.rs docs/pantograph-resolve-only-contract-v1.md
git commit -m "feat(contract): freeze pantograph resolve-only dto v1" -m "Define strict resolver DTOs, validation errors, and ownership boundaries." -m "Agent: codex"
```

## Workstream 1: Remove Legacy Pumas API Usage
Owner: Runtime/backend engineer

Tasks:
1. Remove Pantograph calls to:
   - `get_model_dependency_profiles`
   - `resolve_model_dependency_plan`
   - `check_model_dependencies` (Pumas)
   - `install_model_dependencies` (Pumas)
2. Replace with resolver-only call path to `resolve_model_dependency_requirements`.
3. Delete legacy adapters and dead code paths.

Commit on completion:
```bash
git add crates/node-engine/src/model_dependencies.rs src-tauri/src/workflow/model_dependencies.rs src-tauri/src/workflow/commands.rs
git commit -m "refactor(deps): remove legacy pumas dependency api paths" -m "Hard-cut to resolver-only API and delete removed-method usage." -m "Agent: codex"
```

## Workstream 2: Enforce Contract Version and Boundary Validation
Owner: Runtime/backend engineer

Tasks:
1. Enforce `dependency_contract_version == 1` at Pantograph boundary.
2. Validate required top-level and per-binding fields before internal dispatch.
3. Return typed, actionable boundary errors.

Commit on completion:
```bash
git add src-tauri/src/workflow/model_dependencies.rs src-tauri/src/workflow/commands.rs crates/node-engine/src/model_dependencies.rs
git commit -m "fix(interop): enforce dependency contract v1 at boundary" -m "Add strict payload validation and fail-fast version gate handling." -m "Agent: codex"
```

## Workstream 3: Implement Pantograph-Owned Dependency Lifecycle
Owner: Runtime/backend engineer

Tasks:
1. Implement local lifecycle states:
   - `unresolved`
   - `invalid`
   - `resolved`
   - `checking`
   - `missing`
   - `installing`
   - `ready`
   - `failed`
2. Use Pumas `validation_state` only as resolver quality input.
3. Map resolver requirements into deterministic local check/install actions.

Commit on completion:
```bash
git add crates/node-engine/src/model_dependencies.rs src-tauri/src/workflow/model_dependencies.rs
git commit -m "feat(deps): add pantograph-owned dependency lifecycle engine" -m "Move readiness logic to Pantograph and derive actions from resolver requirements." -m "Agent: codex"
```

## Workstream 4: Environment and Installer Engine
Owner: Runtime/backend engineer

Tasks:
1. Use resolver `env_id` as canonical environment key.
2. Block check/install when `env_id` is null or binding validation is non-resolved.
3. Implement Python package check/install (`python_package` kind only).
4. Add deterministic requirement ordering and single-flight guard per `env_id`.

Commit on completion:
```bash
git add src-tauri/src/workflow/model_dependencies.rs src-tauri/src/workflow/python_runtime.rs crates/node-engine/src/model_dependencies.rs
git commit -m "feat(python): implement env-keyed dependency check install flow" -m "Add env_id gating, deterministic installs, and single-flight execution." -m "Agent: codex"
```

## Workstream 5: Workflow Preflight Integration
Owner: Runtime engineer

Tasks:
1. Run Pantograph dependency lifecycle in preflight before Python execution.
2. Fail fast with actionable diagnostics if lifecycle is not `ready`.
3. Ensure no execution path bypasses dependency preflight for model-backed nodes.

Commit on completion:
```bash
git add crates/node-engine/src/core_executor.rs src-tauri/src/workflow/task_executor.rs src-tauri/src/workflow/model_dependencies.rs
git commit -m "fix(preflight): gate execution on pantograph dependency readiness" -m "Integrate dependency lifecycle into preflight and block non-ready runs." -m "Agent: codex"
```

## Workstream 6: Frontend Puma-Lib and Node UX Cutover
Owner: Frontend engineer

Tasks:
1. Remove UI assumptions tied to legacy Pumas plan/check/install methods.
2. Show resolver results and validation errors as read-only contract data.
3. Wire UI actions to Pantograph-owned check/install endpoints.
4. Keep dependency business state backend-owned (no optimistic updates).

Commit on completion:
```bash
git add src/components/nodes/workflow/PumaLibNode.svelte src/stores/workflowStore.ts src/services/workflow/types.ts
git commit -m "feat(ui): cut puma-lib node to pantograph dependency actions" -m "Render resolver-only data and route check/install through Pantograph lifecycle." -m "Agent: codex"
```

## Workstream 7: Tests and Runtime Verification
Owner: QA + implementation owners

Tasks:
1. Add unit tests for:
   - contract parsing/validation
   - version gate
   - state transitions
   - deterministic ordering
2. Add integration tests for:
   - clean install path
   - missing dependency path
   - invalid/mismatch contract path
3. Verify stable-audio end-to-end run from clean environment.

Commit on completion:
```bash
git add crates/node-engine/src/*tests* src-tauri/src/workflow/*tests* src/components/**/*.test.* docs/pumas-v2-verification-log.md
git commit -m "test(deps): add resolve-only lifecycle and stable-audio coverage" -m "Add unit/integration tests for contract gate, lifecycle transitions, and e2e run." -m "Agent: codex"
```

## Workstream 8: Final Cleanup
Owner: Tech lead + reviewers

Tasks:
1. Remove dead files/types/constants tied to removed API surface.
2. Update or remove outdated dependency docs to match resolve-only architecture.
3. Verify command registration exposes only current dependency command surface.

Commit on completion:
```bash
git add src-tauri/src/main.rs src-tauri/src/workflow docs/*.md crates/node-engine/src
git commit -m "chore(cutover): remove legacy dependency surface after hard cut" -m "Delete dead code and align docs/command registration with resolve-only model." -m "Agent: codex"
```

## Strict Execution Order
1. Workstream 0
2. Workstream 1
3. Workstream 2
4. Workstream 3
5. Workstream 4
6. Workstream 5
7. Workstream 6
8. Workstream 7
9. Workstream 8



## Completion Summary

### Completed

- Pantograph cut over to resolver-owned dependency requirements via
  `resolve_model_dependency_requirements`.
- Backend-owned dependency lifecycle state, model-ref binding payloads, and
  preflight gating landed in code.
- The remaining work is follow-on environment management and additive pinning,
  not the baseline resolve-only cutover itself.
