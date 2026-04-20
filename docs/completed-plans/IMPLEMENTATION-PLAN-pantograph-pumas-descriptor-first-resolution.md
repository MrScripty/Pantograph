# Plan: Pantograph Pumas Descriptor-First Resolution

## Objective

Harden Pantograph's Pumas integration so runtime-executable model facts come
from `ModelExecutionDescriptor` whenever a `model_id` is available, while
preserving the existing workflow-facing facades and keeping record metadata as a
fallback/display contract rather than the runtime source of truth.

## Scope

### In Scope

- Refactor `crates/workflow-nodes/src/input/puma_lib.rs` to prefer
  descriptor-based execution facts over metadata-shape heuristics
- Refactor `crates/pantograph-embedded-runtime/src/model_dependencies.rs` to
  use the same descriptor-first contract
- Centralize or align the descriptor-resolution decision so Pantograph does not
  maintain two diverging heuristics for the same upstream contract
- Preserve current `model_path`, `model_type`, and `task_type_primary` facades
  for downstream Pantograph consumers
- Add targeted tests that cover library-owned diffusers bundles, external
  diffusers bundles, and descriptor-resolution fallback behavior
- Update README and/or ADR traceability for touched architectural boundaries per
  the documentation standards

### Out of Scope

- Changing Pumas-Library's public `ModelExecutionDescriptor` contract
- Treating `metadata.json` or projected metadata as runtime authority
- Reworking unrelated model-search, inference-settings, or dependency-profile
  behavior outside descriptor resolution
- Broad Pantograph runtime-selection refactors unrelated to Pumas model
  execution path resolution

## Inputs

### Problem

Pantograph currently decides whether to call
`resolve_model_execution_descriptor(...)` by inspecting `ModelRecord.metadata`
for specific storage or bundle markers. That is fragile because Pumas already
publishes a runtime execution contract through `ModelExecutionDescriptor`, and
Pantograph duplicates knowledge of which record shapes should be treated as
runtime bundles. The result is compatibility risk whenever Pumas adds or changes
 executable model shapes.

### Constraints

- Preserve current external Pantograph facades unless an explicit API break is
  approved
- Keep dependency direction aligned with `ARCHITECTURE-PATTERNS.md`:
  Pantograph application/integration code consumes Pumas contracts; it does not
  infer new upstream runtime rules from projected metadata
- Keep metadata as a fallback/display contract only; do not reintroduce
  `metadata.json` as a source of truth
- Limit code movement to the Pumas integration boundary so the change remains
  reviewable and atomic
- Follow `TESTING-STANDARDS.md`, `COMMIT-STANDARDS.md`, and
  `DOCUMENTATION-STANDARDS.md` during execution

### Assumptions

- Pumas `resolve_model_execution_descriptor(model_id)` is the authoritative
  runtime-resolution API for executable model assets
- Requests that do not carry a usable `model_id` must continue to rely on the
  existing request/record fallback path
- Descriptor resolution may legitimately fail for some records, and Pantograph
  must degrade deterministically instead of hard failing every time
- Existing downstream consumers still expect the `model_path` facade even when
  its value now comes from `entry_path`

### Dependencies

- `crates/workflow-nodes`
- `crates/pantograph-embedded-runtime`
- `../Pumas-Library/rust/crates/pumas-core` runtime contracts consumed by
  Pantograph
- Existing Pantograph README/ADR surfaces under `crates/` and `docs/`

### Affected Structured Contracts

- Pantograph's internal mapping from `ModelRecord` and
  `ModelExecutionDescriptor` to workflow-facing node outputs
- Pantograph embedded-runtime descriptor resolution used for dependency
  preflight and execution
- Error/fallback semantics when descriptor resolution is unavailable or returns
  validation failures

### Affected Persisted Artifacts

- No Pantograph persisted runtime artifact is expected to change
- Test fixtures may change if new descriptor-oriented scenarios are added
- Documentation artifacts may change through README and/or ADR updates

### Concurrency / Race-Risk Review

- This work does not introduce new background workers, polling, or restart
  ownership
- Descriptor resolution happens inside existing async request paths, so the main
  risk is inconsistent fallback behavior between Pantograph call sites rather
  than cross-task mutation
- Mitigation: keep descriptor-resolution policy explicit, shared, and covered by
  focused tests for both integration points
- Ownership and lifecycle note:
  - The calling request path remains responsible for starting descriptor
    resolution
  - No new long-lived task or cleanup lifecycle is added
  - Fallback behavior must complete within the same request boundary that
    initiated resolution

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Descriptor resolution is applied inconsistently between workflow nodes and embedded runtime | High | Define one descriptor-first policy and apply it in both integration points with mirrored tests |
| Pantograph accidentally changes `model_path` semantics for non-bundle/file models | High | Preserve current fallback path when descriptor resolution is absent or not applicable; add regression tests for file-backed models |
| Descriptor-resolution failures leak as new hard errors where current behavior succeeds | Medium | Keep explicit fallback rules and test descriptor failure/absence paths |
| Documentation drift leaves the integration contract implicit | Medium | Update touched README/ADR surfaces in the same implementation slice as the contract change |
| Large helper extraction obscures ownership instead of clarifying it | Low | Keep helpers narrowly scoped to the Pumas integration boundary and review file responsibility before adding shared abstractions |

## Definition of Done

- Both Pantograph Pumas integration points use a descriptor-first runtime
  resolution policy keyed by `model_id`, not metadata-shape heuristics
- Pantograph preserves current public/workflow-facing fields while sourcing
  runtime-executable path/type/task facts from `ModelExecutionDescriptor` when
  available
- Fallback behavior is deterministic when descriptor resolution is unavailable,
  invalid, or not possible
- Tests cover at least one acceptance path across producer contract -> Pantograph
  binding -> runtime descriptor consumption for library-owned and external
  diffusers bundles
- Touched architectural directories have standards-compliant README and/or ADR
  traceability updates

## Milestones

### Milestone 1: Freeze The Integration Contract

**Goal:** Make the runtime contract and facade-preservation decision explicit
before changing code.

**Tasks:**
- [ ] Record the descriptor-first policy at the Pantograph/Pumas integration
      boundary
- [ ] Record public facade preservation: keep `model_path`, `model_type`, and
      `task_type_primary` facades stable while changing how they are sourced
- [ ] Identify the narrow shared decision surface to avoid two divergent
      implementations
- [ ] Identify touched directories that require README and/or ADR updates under
      `DOCUMENTATION-STANDARDS.md`

**Verification:**
- Architecture review against
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/ARCHITECTURE-PATTERNS.md`
- Documentation review of touched directories against
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`

**Status:** Complete

### Milestone 2: Make `puma-lib` Descriptor-First

**Goal:** Remove metadata-shape gating from workflow-node model output
resolution.

**Tasks:**
- [ ] Replace `should_use_execution_descriptor(...)` gating with a
      `model_id`-driven descriptor-resolution path
- [ ] Preserve current fallback behavior for records that cannot resolve a
      descriptor
- [ ] Keep inference-settings and non-runtime display metadata behavior scoped
      to their existing responsibilities
- [ ] Add focused tests for library-owned diffusers bundles, external diffusers
      bundles, and non-bundle fallback behavior

**Verification:**
- `cargo test -p workflow-nodes --features model-library test_bundle_models_resolve_execution_descriptor_entry_path --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`
- Additional targeted `workflow-nodes` tests covering fallback semantics per
  `TESTING-STANDARDS.md`
- `cargo check --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`

**Status:** Complete

### Milestone 3: Make Embedded Runtime Descriptor-First

**Goal:** Align dependency preflight/runtime resolution with the same upstream
runtime contract.

**Tasks:**
- [ ] Replace `record_uses_execution_descriptor(...)` gating with the same
      `model_id`-driven descriptor-resolution policy
- [ ] Preserve deterministic fallback when requests only have `model_path` or
      when descriptor resolution fails
- [ ] Confirm dependency preflight still receives the correct executable path
      for both library-owned and external bundles
- [ ] Add targeted tests for descriptor success and fallback semantics in
      `model_dependencies.rs`

**Verification:**
- `cargo test -p pantograph-embedded-runtime resolve_descriptor_uses_entry_path_for_external_diffusers_bundle --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`
- Additional targeted embedded-runtime tests for descriptor absence/failure and
  file-backed model regressions
- `cargo check --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`

**Status:** Complete

### Milestone 4: Add Cross-Layer Contract Coverage

**Goal:** Prove the producer-to-consumer execution path still agrees with the
descriptor contract once both call sites change.

**Tasks:**
- [ ] Add at least one cross-layer acceptance check that exercises Pumas record
      input -> Pantograph node/runtime binding -> executable path selection
- [ ] Verify library-owned diffusers, external diffusers, and plain file-backed
      models behave as expected across both call sites
- [ ] Review test naming, placement, and scope against
      `TESTING-STANDARDS.md`

**Verification:**
- Run the targeted acceptance test(s) added for the end-to-end contract path
- Re-run affected package tests after the full slice lands
- Confirm no isolated unit-only verification is standing in for the required
  cross-layer acceptance path

**Status:** Complete

### Milestone 5: Traceability And Cleanup

**Goal:** Leave the architectural boundary explicit and standards-compliant.

**Tasks:**
- [ ] Update/add README content for touched directories so the Pantograph/Pumas
      contract, fallback rules, and structured producer/consumer expectations
      are explicit
- [ ] Add or update an ADR only if the boundary change is large enough that the
      existing READMEs are insufficient for future maintenance
- [ ] Review helper placement and module size so no catch-all integration
      utility is introduced
- [ ] Remove dead heuristic-only helpers once the descriptor-first path is in
      place

**Verification:**
- README/ADR review against
  `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`
- `cargo check --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`
- Final targeted test rerun for all touched packages

**Status:** Complete

## Execution Notes

Update during implementation:
- 2026-04-12: Plan created after reviewing Pantograph's two Pumas integration
  points against the current Pumas `ModelExecutionDescriptor` contract and the
  recent library-owned diffusers path fix in Pumas-Library.
- 2026-04-12: Milestone 1 completed by recording the descriptor-first boundary
  in the implementation plan and module READMEs.
- 2026-04-12: Milestone 2 completed by updating `workflow-nodes` `puma-lib` to
  resolve execution descriptors for any record with a model id and adding
  bundle/file-backed tests.
- 2026-04-12: Milestones 3-5 completed by updating the embedded runtime
  resolver, adding cross-layer acceptance coverage against the real `puma-lib`
  options provider, and keeping README traceability aligned with the contract
  change.

## Commit Cadence Notes

- Do not create commits for planning-only work.
- When implementation begins, commit after each verified milestone or smaller
  logical slice that remains atomic and reviewable.
- Prefer one commit for contract/README boundary freeze, one for
  `workflow-nodes`, one for `pantograph-embedded-runtime`, one for cross-layer
  tests, and one for any final documentation/cleanup if those slices remain
  independent.
- Use conventional commits with module-scoped descriptions and detailed bodies
  that explain the contract change, fallback behavior, and verification run per
  `COMMIT-STANDARDS.md`.
- Before each commit, review staged diff, recent unpushed history, and rerun at
  least the affected tests required by the completed slice.

## Optional Subagent Assignment

| Owner/Agent | Scope | Output Contract | Handoff Checkpoint |
| ----------- | ----- | --------------- | ------------------ |
| None | None | None | Revisit only if documentation and test additions can be split cleanly from the main integration change |

## Re-Plan Triggers

- Pumas descriptor resolution semantics change in a way that requires a Pantograph
  facade change
- Affected callers exist outside the two reviewed Pantograph integration points
  and need the same contract change
- Fallback behavior cannot remain deterministic without a broader request-model
  refactor
- Documentation review finds a larger architectural ambiguity than this plan
  assumes

## Recommendations (Only If Better Option Exists)

- Recommendation 1: Extract a small Pantograph-local helper for
  descriptor-resolution policy instead of keeping two similar implementations.
  This reduces drift risk without introducing a broad shared abstraction.
  Scope/timeline impact: low.
- Recommendation 2: Prefer README contract updates first and add an ADR only if
  the helper placement or facade-preservation decision spans more than the two
  touched integration modules. This keeps documentation proportional to the
  change. Scope/timeline impact: low.

## Completion Summary

### Completed

- Milestone 1: Descriptor-first integration boundary recorded in the plan and
  module READMEs.
- Milestone 2: `workflow-nodes` `puma-lib` now resolves executable model paths
  from `ModelExecutionDescriptor` whenever a record has a model id.
- Milestone 3: `pantograph-embedded-runtime` now resolves execution descriptors
  by model id and only falls back for missing-descriptor responses.
- Milestone 4: Added a cross-layer acceptance test that feeds the real
  `puma-lib` option value into the embedded dependency resolver.
- Milestone 5: README traceability for the Pantograph/Pumas boundary is in
  place; no ADR was required for this scope.

### Deviations

- None.

### Follow-Ups

- None required for this descriptor-first compatibility slice.

### Verification Summary

- Plan inputs validated against:
  - `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`
  - `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/ARCHITECTURE-PATTERNS.md`
  - `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/COMMIT-STANDARDS.md`
  - `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TESTING-STANDARDS.md`
  - `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`
- Implementation verification:
  - `cargo test -p workflow-nodes --features model-library test_bundle_models_resolve_execution_descriptor_entry_path --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`
  - `cargo test -p workflow-nodes --features model-library test_file_models_resolve_execution_descriptor_primary_file_path --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`
  - `cargo test -p pantograph-embedded-runtime resolve_descriptor_uses_entry_path_for_external_diffusers_bundle --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`
  - `cargo test -p pantograph-embedded-runtime resolve_descriptor_uses_primary_file_for_library_owned_file_model --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`
  - `cargo test -p pantograph-embedded-runtime descriptor_lookup_fallback_is_allowed_only_for_missing_descriptor_cases --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`
  - `cargo test -p pantograph-embedded-runtime puma_lib_option_and_dependency_resolver_agree_on_primary_file_path --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`
  - `cargo check --manifest-path /media/jeremy/OrangeCream/Linux Software/repos/owned/ai-systems/Pantograph/Cargo.toml`

### Traceability Links

- Module README updated:
  - `crates/workflow-nodes/src/input/README.md`
  - `crates/pantograph-embedded-runtime/src/README.md`
- ADR added/updated: N/A; README coverage was sufficient for this scope
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`: N/A

## Brevity Note

Keep implementation aligned to the narrow descriptor-resolution boundary. Expand
scope only if a re-plan trigger is hit.
