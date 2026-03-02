# Implementation Plan: Pantograph Update for Pumas Dependency Pinning

## Status
Draft (ready to execute)

## Goal
Update Pantograph to consume the newly implemented Pumas dependency pinning model while remaining tolerant to variable metadata shape and additive API fields.

## Standards Alignment
This plan is aligned to:
- `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/COMMIT-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/INTEROP-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/TESTING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/Coding-Standards/TOOLING-STANDARDS.md`

Key enforcement:
1. Conventional commits for every task.
2. Cross-language contract updates in the same commit when wire DTOs change.
3. Static analysis and affected tests before every commit.
4. Small, atomic commits (one logical change per commit).

## Verified Upstream Pumas Changes (Consumer-Relevant)
1. Additive dependency fields now exist:
   - top-level: `missing_pins`
   - per-binding: `pin_summary`, `required_pins`, `missing_pins`
2. Deterministic pinning codes now exist:
   - `unpinned_dependency`
   - `modality_resolution_unknown`
3. New audit API exists:
   - `audit_dependency_pin_compliance`

## Implementation Tasks

### Task 1: Extend Pantograph dependency DTOs (Rust) for pin fields
Files:
- `crates/node-engine/src/model_dependencies.rs`
- `src-tauri/src/workflow/model_dependencies.rs`

Changes:
1. Add additive fields to Pantograph DTOs:
   - top-level `missing_pins`
   - per-binding `pin_summary`, `required_pins`, `missing_pins`
2. Add per-binding `code` for status/install rows.
3. Keep serde defaults so unknown fields remain non-breaking.

Validation before commit:
1. `cargo check --manifest-path src-tauri/Cargo.toml -p pantograph`

Commit after completion:
1. `feat(deps): add pin-aware dependency dto fields`
2. Include footer:
   - `Agent: codex`

### Task 2: Map all new Pumas fields in resolver
Files:
- `src-tauri/src/workflow/model_dependencies.rs`

Changes:
1. Preserve pin fields from `pumas_library::model_library::ModelDependencyBindingPlan`.
2. Preserve top-level `missing_pins` from plan/check/install.
3. Preserve per-binding error code for UI remediation.

Validation before commit:
1. `cargo test --manifest-path src-tauri/Cargo.toml workflow::model_dependencies::tests -- --nocapture`
2. `cargo check --manifest-path src-tauri/Cargo.toml -p pantograph`

Commit after completion:
1. `feat(workflow): map pumas dependency pin payload fields`
2. Include footer:
   - `Agent: codex`

### Task 3: Make dependency state/code handling frontend-tolerant
Files:
- `src/components/nodes/workflow/PumaLibNode.svelte`
- `src/components/nodes/workflow/PyTorchInferenceNode.svelte`
- `src/components/nodes/workflow/AudioGenerationNode.svelte`

Changes:
1. Replace closed dependency-state assumptions with tolerant string handling.
2. Render new pinning codes (`unpinned_dependency`, `modality_resolution_unknown`) explicitly.
3. Add safe fallback rendering for unknown state/code values.
4. Keep dynamic arrays unbounded (`required_pins`, `missing_pins`, inference settings).

Validation before commit:
1. `npm run typecheck`
2. `npm run lint:full`

Commit after completion:
1. `feat(ui): render dynamic pinning states and remediation codes`
2. Include footer:
   - `Agent: codex`

### Task 4: Display per-binding and aggregate pin details in UI
Files:
- `src/components/nodes/workflow/PumaLibNode.svelte`

Changes:
1. Show top-level `missing_pins`.
2. Show per-binding:
   - `pin_summary`
   - `required_pins` with reasons
   - `missing_pins`
3. Keep rendering resilient when any field is absent.

Validation before commit:
1. `npm run typecheck`
2. `npm run lint:full`

Commit after completion:
1. `feat(ui): surface dependency pin summaries and required pin reasons`
2. Include footer:
   - `Agent: codex`

### Task 5: Add audit command surface in Pantograph
Files:
- `src-tauri/src/workflow/model_dependency_commands.rs` (or adjacent command module)
- `src-tauri/src/workflow/commands.rs`
- Optional: new UI wiring file if audit is exposed in the interface

Changes:
1. Add Tauri command wrapper for `audit_dependency_pin_compliance`.
2. Return raw structured report for downstream rendering/logging.

Validation before commit:
1. `cargo check --manifest-path src-tauri/Cargo.toml -p pantograph`
2. Command-level tests for argument and response shape.

Commit after completion:
1. `feat(workflow): add dependency pin compliance audit command`
2. Include footer:
   - `Agent: codex`

### Task 6: Update Pantograph dependency contract docs
Files:
- `docs/pumas-v2-contract-freeze.md`
- `docs/pumas-v2-verification-log.md` (if updated as part of rollout evidence)

Changes:
1. Replace frozen-code assumptions with additive-field/unknown-key tolerant consumer policy.
2. Document new pin fields and deterministic pin error codes.
3. Document that Pantograph interprets known fields and safely ignores extra upstream fields.

Validation before commit:
1. Markdown lint/preview sanity check (project-standard doc checks if available).

Commit after completion:
1. `docs(deps): align pantograph consumer contract with pumas pinning`
2. Include footer:
   - `Agent: codex`

### Task 7: Add regression tests for contract tolerance and pin behavior
Files:
- `src-tauri/src/workflow/model_dependencies.rs` test module
- Relevant frontend test locations (if present)

Changes:
1. Tests for mapping and preserving pin fields.
2. Tests for known pinning codes and fallback behavior on unknown code/state.
3. Tests for variable-length arrays in dependency and metadata payloads.

Validation before commit:
1. `cargo test --manifest-path src-tauri/Cargo.toml workflow::model_dependencies::tests -- --nocapture`
2. `npm run test` (or closest available affected test command)
3. `cargo check --manifest-path src-tauri/Cargo.toml -p pantograph`

Commit after completion:
1. `test(deps): add pinning and unknown-field tolerance coverage`
2. Include footer:
   - `Agent: codex`

### Task 8: Final end-to-end verification and integration commit
Changes:
1. Run full verification stack for integration confidence.
2. Confirm runtime probe behavior for pin codes/messages.

Validation before commit:
1. `npm run check`
2. `cargo check --manifest-path src-tauri/Cargo.toml -p pantograph`
3. Optional runtime probe:
   - `cargo run -p pantograph --bin pumas_dependency_runtime_probe -- --json`

Commit after completion:
1. `chore(deps): finalize pumas pinning consumer integration`
2. Include footer:
   - `Agent: codex`

## Pre-Commit Gate (Required For Every Task Commit)
1. `git status`
2. `git diff --cached`
3. Run lint/typecheck for touched areas.
4. Run affected tests.
5. Use conventional commit format:
   - `<type>(<scope>): <description>`
6. Include footer on agent-generated commits:
   - `Agent: codex`

## Acceptance Criteria
1. Pantograph preserves and exposes Pumas pin fields without breaking on unknown additive fields.
2. Pantograph renders actionable remediation for `unpinned_dependency` and `modality_resolution_unknown`.
3. Pantograph supports variable-length metadata and dependency arrays without fixed-size assumptions.
4. Pantograph includes audit command support for dependency pin compliance reporting.
5. All tasks land as atomic, standards-compliant commits.
