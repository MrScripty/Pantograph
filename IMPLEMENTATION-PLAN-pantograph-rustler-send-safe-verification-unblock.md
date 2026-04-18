# Plan: Pantograph Rustler Send-Safe Verification Unblock

## Status
Complete

Last updated: 2026-04-18

## Current Source-of-Truth Summary

This document is the dedicated source of truth for unblocking the current
Rustler verification failure caused by the `Send` mismatch between the
`node-engine` orchestration data-graph executor contract and the
`multi_demand` future chain.

Use this file when the work being discussed is specifically:

- restoring standards-compliant native verification for `pantograph_rustler`
- fixing the backend-owned async contract mismatch without moving policy into
  Rustler
- choosing between the preferred core `Send` fix and the backend-owned
  sequential fallback

Use `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`
for the broader BEAM-hosted verification lane, and
`IMPLEMENTATION-PLAN-pantograph-binding-platform.md` for broader binding
platform policy and support-tier questions.

## Objective

Restore standards-compliant Rustler native verification by resolving the
current `Send` mismatch at the correct ownership layer, while keeping canonical
workflow execution semantics in backend Rust crates and leaving Rustler as a
thin Layer 2 wrapper.

## Scope

### In Scope

- The `node_engine::DataGraphExecutor` async contract used by orchestration
- The `multi_demand` future chain in `crates/node-engine/src/engine/multi_demand.rs`
- The Rustler orchestration bridge in `crates/pantograph-rustler/src/lib.rs`
  and any focused module extraction required to keep touched code compliant
- Native verification needed to make `cargo check` / `cargo test` for
  `pantograph_rustler` meaningful again
- README and plan updates required by the resulting boundary shape

### Out of Scope

- New workflow-event semantics or reopening closed Phase 5 event-contract work
- Wrapper-local business-logic workarounds in Rustler
- BEAM-hosted NIF smoke/acceptance harness implementation beyond what is
  already covered by the broader Rustler NIF plan
- UniFFI, C#, or Python binding expansion work
- Codebase-wide cleanup unrelated to the touched async contract path

## Inputs

### Problem

`pantograph_rustler` verification is currently blocked because
`ElixirDataGraphExecutor::execute_data_graph` in
`crates/pantograph-rustler/src/lib.rs` awaits
`WorkflowExecutor::demand_multiple(...)`, while the core future alias in
`crates/node-engine/src/engine/multi_demand.rs` currently boxes a
non-`Send` future:

- `node_engine::DataGraphExecutor` uses `#[async_trait]`, which requires a
  `Send` future by default
- `DemandIsolatedTargetRunFuture<'a>` is currently
  `Pin<Box<dyn Future<Output = Result<DemandIsolatedTargetRun>> + 'a>>`
- the resulting mismatch prevents `pantograph_rustler` library and test
  verification from compiling

If fixed in the wrong place, the likely failure modes are predictable:

- Rustler grows wrapper-local execution policy to avoid the type mismatch
- the core trait is weakened globally for one binding lane without clear need
- verification is “solved” by avoiding the crate instead of restoring a valid
  backend-owned contract

### Constraints

- Backend crates own execution semantics and async contract shape.
- Rustler remains a Layer 2 wrapper under
  `LANGUAGE-BINDINGS-STANDARDS.md`.
- The fix must preserve existing public Rustler entrypoints unless an explicit
  break is approved.
- Any touched oversized file must be decomposed locally rather than deepened.
- Verification must satisfy `TESTING-STANDARDS.md`; excluding the broken crate
  is not an acceptable resolution.

### Assumptions

- The preferred outcome is that the core `multi_demand` path becomes `Send`
  safe without changing orchestration semantics.
- If that is not possible, the real reason will be a genuine execution-model
  incompatibility rather than a minor missing bound.
- A backend-owned sequential orchestration execution path is acceptable only if
  it is explicitly documented as the correct model for this boundary.
- Relaxing the shared trait to `?Send` is less desirable than either of the
  two backend-owned options above.

### Dependencies

- `crates/node-engine/src/orchestration/executor.rs`
- `crates/node-engine/src/engine/multi_demand.rs`
- `crates/pantograph-rustler/src/lib.rs`
- `crates/pantograph-rustler/src/README.md`
- `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/PLAN-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/ARCHITECTURE-PATTERNS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CODING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/CONCURRENCY-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/LANGUAGE-BINDINGS-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/TESTING-STANDARDS.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/DOCUMENTATION-STANDARDS.md`

### Affected Structured Contracts

- `node_engine::DataGraphExecutor` async future requirements
- `multi_demand` isolated-target future boxing and polling contract
- Rustler orchestration-to-data-graph bridge behavior
- Native verification expectations for `pantograph_rustler`

### Affected Persisted Artifacts

- `crates/node-engine/src/orchestration/executor.rs`
- `crates/node-engine/src/engine/multi_demand.rs`
- `crates/pantograph-rustler/src/lib.rs`
- Any extracted Rustler module(s) created to keep touched code compliant
- `crates/pantograph-rustler/src/README.md`
- This plan
- `IMPLEMENTATION-PLAN-pantograph-phase-5-rustler-nif-testability-and-beam-verification.md`
  if the verification lane needs updated traceability after implementation

### Concurrency / Race-Risk Review

- The preferred fix touches concurrent isolated-target execution and must not
  accidentally serialize or reorder core behavior unless the fallback path is
  explicitly selected.
- Any `Send` repair must avoid holding non-`Send` guards or references across
  `.await`.
- If the sequential fallback is chosen, ownership must remain backend-owned so
  wrapper lanes do not drift into different lifecycle rules.
- Verification changes must isolate any temp roots, environment variables, and
  host-runtime state they touch.

### Ownership And Lifecycle Note

- `node-engine` owns the orchestration/data-graph execution contract.
- `pantograph_rustler` owns only Rustler-specific adapter wiring for that
  contract.
- If a sequential fallback is needed, `node-engine` must own it and Rustler
  must only select or consume that backend-owned path.
- BEAM-hosted acceptance remains part of the broader Rustler NIF verification
  plan, not this unblock plan’s ownership surface.

### Public Facade Preservation Note

This is a facade-first plan. Existing Rustler exports and the current
orchestration bridge entrypoints should remain stable while the internal async
contract is repaired and any touched wrapper code is extracted into focused
modules.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Core `Send` repair reveals deeper non-`Send` captures across `multi_demand` | High | Treat the audit as the first milestone and fix captures in backend code rather than papering over them in Rustler |
| Wrapper-local workaround bypasses backend ownership | High | Explicitly reject Rustler-local execution-policy forks in the plan and review implementation against `LANGUAGE-BINDINGS-STANDARDS.md` |
| Global trait weakening with `?Send` reduces portability | High | Make `?Send` a non-preferred last resort that requires explicit proof the shared contract should be non-`Send` |
| Sequential fallback silently changes behavior for all callers | Medium | Introduce fallback only as an explicit backend-owned path with targeted contract tests and documentation |
| Touched oversized Rustler file grows again | Medium | Extract the Elixir data-graph bridge into a focused module before adding new behavior there if that path is touched materially |

## Definition of Done

- The repository has a dedicated plan for the current Rustler verification
  blocker and the preferred/fallback paths are explicit.
- The chosen implementation path keeps execution semantics backend-owned.
- `pantograph_rustler` native verification is restored to a standards-compliant
  state for the touched path.
- Touched code and immediate touched surroundings end in a cleaner,
  standards-compliant state than they started.
- README/plan traceability for the touched boundary is up to date.

## Standards Review Passes

### Draft Pass

Initial draft built from direct inspection of:

- `crates/pantograph-rustler/src/lib.rs`
- `crates/node-engine/src/orchestration/executor.rs`
- `crates/node-engine/src/engine/multi_demand.rs`
- the current compiler failure path

### Pass 1: Plan Structure And Traceability

Reviewed against:

- `PLAN-STANDARDS.md`
- `DOCUMENTATION-STANDARDS.md`

Corrections applied:

- Added explicit definition of done, milestones, re-plan triggers, and
  completion criteria
- Recorded touched artifacts and traceability expectations
- Scoped the plan narrowly to the verification blocker instead of mixing it
  into broader binding-platform planning

### Pass 2: Architecture And Binding Ownership

Reviewed against:

- `ARCHITECTURE-PATTERNS.md`
- `LANGUAGE-BINDINGS-STANDARDS.md`

Corrections applied:

- Kept the preferred fix in `node-engine`, not in Rustler
- Marked any sequential fallback as backend-owned rather than wrapper-owned
- Explicitly rejected widening wrapper policy or weakening ownership boundaries

### Pass 3: Concurrency And Async Contract Safety

Reviewed against:

- `CONCURRENCY-STANDARDS.md`

Corrections applied:

- Made `Send`-safe async repair the preferred path
- Required explicit removal of non-`Send` captures across `.await`
- Required lifecycle ownership notes for any fallback that changes execution
  strategy

### Pass 4: Verification Requirements

Reviewed against:

- `TESTING-STANDARDS.md`

Corrections applied:

- Required restoration of native verification for `pantograph_rustler`
- Required targeted backend and wrapper checks instead of “skip the crate”
- Kept BEAM-hosted acceptance as a separate but traceable follow-on lane

## Milestones

### Milestone 1: Freeze The Blocking Contract And Repair Immediate Insertion Points

**Goal:** Confirm the exact async contract mismatch and ensure touched files
can absorb the fix without deepening standards debt.

**Tasks:**
- [x] Record the exact failure boundary between `DataGraphExecutor`,
      `ElixirDataGraphExecutor`, and `DemandIsolatedTargetRunFuture`.
- [x] If `crates/pantograph-rustler/src/lib.rs` needs more than a minimal call
      site change, extract the Elixir data-graph bridge into a focused module
      before adding behavior there.
- [x] Record whether the current `Send` failure is caused by missing bounds,
      non-`Send` captures, or a deeper execution-model mismatch.

**Milestone 1 result:**

- The blocking boundary is now recorded explicitly:
  `ElixirDataGraphExecutor::execute_data_graph` in
  `crates/pantograph-rustler/src/elixir_data_graph_executor.rs` must satisfy
  the default `Send` future requirement imposed by
  `node_engine::DataGraphExecutor` in
  `crates/node-engine/src/orchestration/executor.rs`.
- The immediate compile blocker is the non-`Send` boxed future alias
  `DemandIsolatedTargetRunFuture<'a>` in
  `crates/node-engine/src/engine/multi_demand.rs`, which currently omits
  `+ Send`.
- The current evidence points to a backend-owned missing-bound / non-`Send`
  capture problem in the core `multi_demand` chain, not to a wrapper-owned
  lifecycle mismatch. The preferred next slice therefore remains the core
  `Send` repair in `node-engine`.

**Verification:**
- Focused compiler inspection of the failing path
- File-size/responsibility review against `CODING-STANDARDS.md`
- `cargo check -p node-engine`

**Status:** Complete

### Milestone 2: Preferred Path, Make The Core Multi-Demand Chain Send-Safe

**Goal:** Repair the async contract in backend-owned code so the shared
orchestration bridge can satisfy the existing `Send` requirement.

**Tasks:**
- [x] Change `DemandIsolatedTargetRunFuture<'a>` to require `+ Send`.
- [x] Fix any non-`Send` captures or guard lifetimes revealed by that change.
- [x] Keep behavior and event semantics unchanged while making the future chain
      safe for the trait boundary.
- [x] Add focused backend tests that pin the repaired path where the compiler
      previously failed.

**Milestone 2 result:**

- `DemandIsolatedTargetRunFuture<'a>` in
  `crates/node-engine/src/engine/multi_demand.rs` now requires `+ Send`,
  matching the `DataGraphExecutor` async contract without weakening the shared
  trait boundary.
- No additional non-`Send` captures or guard-lifetime issues were exposed by
  the repair; existing multi-demand behavior remained intact.
- A focused compile-time backend test now asserts that the isolated-target
  future satisfies the `Send` boundary directly in `node-engine`.
- The repaired path restores native compilation for `pantograph_rustler`
  without moving orchestration policy into the Rustler wrapper.

**Verification:**
- `cargo test -p node-engine`
- `cargo check -p pantograph_rustler`
- `cargo test -p pantograph_rustler` if the library build is restored cleanly

**Status:** Complete

### Milestone 3: Conditional Fallback, Add A Backend-Owned Sequential Orchestration Path

**Goal:** Provide a standards-compliant fallback only if Milestone 2 proves the
current bridge should not use the concurrent isolated-target path.

**Tasks:**
- [ ] Introduce an explicit backend-owned sequential data-graph execution path
      in `node-engine` for orchestration use.
- [ ] Route Rustler to that backend-owned path without adding wrapper-local
      lifecycle semantics.
- [ ] Document why the fallback is necessary and why `?Send` was not chosen.
- [ ] Add focused tests that pin the fallback’s waiting/cancel/error behavior.

**Verification:**
- `cargo test -p node-engine`
- `cargo check -p pantograph_rustler`
- `cargo test -p pantograph_rustler`

**Status:** Not needed

### Milestone 4: Reconcile Verification And Traceability

**Goal:** Leave the verification lane and touched documentation in a clean
state after the blocking contract is fixed.

**Tasks:**
- [x] Update `crates/pantograph-rustler/src/README.md` if module ownership or
      execution-path notes changed.
- [x] Update the broader Rustler NIF verification plan if the blocking native
      verification issue is resolved.
- [x] Record whether BEAM-hosted acceptance remains the only distinct
      verification gap after native verification is restored.

**Milestone 4 result:**

- The Rustler README already reflects the extracted Elixir data-graph executor
  module introduced during Milestone 1.
- The broader BEAM/NIF verification plan now records that the `Send` mismatch
  is resolved and that the remaining distinct gap is real BEAM-hosted symbol
  linkage and acceptance coverage.
- This unblock plan closes with the native compile boundary restored and the
  residual `enif_*` symbol problem handed back to the dedicated Rustler NIF
  verification lane.

**Verification:**
- Documentation review against `DOCUMENTATION-STANDARDS.md`
- Plan-to-code consistency review
- Focused verification summary for the chosen milestone path

**Status:** Complete

## Execution Notes

Update during implementation:
- 2026-04-18: Plan created from the current `pantograph_rustler` `Send`
  blocker after Phase 5 event-contract close-out.
- 2026-04-18: Milestone 1 completed. The Elixir orchestration data-graph bridge
  was extracted into a focused Rustler module, and the exact blocking contract
  mismatch was frozen against `DataGraphExecutor` and
  `DemandIsolatedTargetRunFuture<'a>`.
- 2026-04-18: Milestone 2 completed. `DemandIsolatedTargetRunFuture<'a>` now
  carries `+ Send`, and `node-engine` includes a focused compile-time test that
  pins the repaired async boundary.
- 2026-04-18: Native verification is restored for the compile boundary:
  `cargo test -p node-engine` passes and `cargo check -p pantograph_rustler`
  succeeds. `cargo test -p pantograph_rustler` is now blocked only by the
  separate BEAM-hosted `enif_*` linker-symbol boundary tracked in the broader
  Rustler NIF plan.

## Commit Cadence Notes

- Commit when a logical slice is complete and verified.
- Follow commit format/history cleanup rules from `COMMIT-STANDARDS.md`.

## Re-Plan Triggers

- The core `Send` attempt reveals a genuine architecture reason the shared
  contract should not be `Send`.
- The required fix materially changes orchestration semantics or caller-visible
  behavior.
- Restoring native verification exposes an additional independent blocker not
  covered by this plan.

## Recommendations

- Recommendation 1: prefer the core `Send` repair first. It best matches
  `ARCHITECTURE-PATTERNS.md`, `LANGUAGE-BINDINGS-STANDARDS.md`, and
  `CONCURRENCY-STANDARDS.md` because it fixes the shared backend-owned async
  contract rather than adding wrapper policy.
- Recommendation 2: use the backend-owned sequential fallback only if the core
  audit proves the current bridge should not use the concurrent isolated-target
  path. This keeps the fallback explicit and reviewable without weakening the
  shared trait globally.
- Recommendation 3: do not choose `#[async_trait(?Send)]` or wrapper-local
  loops as the first fix. Both options reduce clarity of ownership and are more
  likely to violate the binding and architecture standards unless a stronger
  justification emerges.

## Completion Summary

### Completed

- N/A

### Deviations

- N/A

### Follow-Ups

- BEAM-hosted acceptance still belongs to the broader Rustler NIF verification
  plan once the native verification blocker is removed.

### Verification Summary

- N/A

### Traceability Links

- Module README updated: `crates/pantograph-rustler/src/README.md` or N/A
- ADR added/updated: N/A
- PR notes completed per `templates/PULL_REQUEST_TEMPLATE.md`

## Brevity Note

Keep the plan concise. Expand detail only where execution decisions or risk
require it.
