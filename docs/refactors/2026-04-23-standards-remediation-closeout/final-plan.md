# Final Plan: Standards Remediation Closeout

## Objective
Close the remaining standards-compliance gaps after the April 23, 2026
reassessment by restoring all claimed local verification gates, aligning
blocking CI with the actual host-facing contract surface, and updating
traceability docs so they match the tree.

## Scope

### In Scope
- workflow-service contract snapshot drift for runtime capability serialization
- the remaining frontend no-new-debt accessibility gate failure
- the remaining rustfmt baseline failure
- CI coverage needed to keep the restored standards gates from drifting again
- standards/ADR traceability updates directly tied to those fixes

### Out of Scope
- unrelated asset worktree changes under `assets/`
- broader architecture work already completed in the standards refactor
- feature work or behavior changes not required for gate restoration

## Inputs

### Problem
The codebase is substantially improved and most core validation is green, but
three standards-closeout failures remain:
1. `cargo test -p pantograph-workflow-service --test contract` is still red.
2. `npm run lint:no-new` is still red.
3. `cargo fmt --all -- --check` is still red.

The current CI configuration also does not block on the workflow-service
contract suite, which means one of those failures can still merge unnoticed.

### Constraints
- Preserve backend ownership of workflow contracts and diagnostics decisions.
- Prefer facade-first compatibility; do not silently change public wire shape.
- Keep fixes narrow and auditable.
- Do not fold unrelated asset churn into this remediation.
- Do not weaken lint/a11y/format rules just to make the tree green.

### Assumptions
- `readiness_state` on `WorkflowRuntimeCapability` is likely intended behavior,
  but that must be confirmed during implementation.
- The remaining formatter drift is mechanical, not architectural.
- The WorkflowGraph accessibility suppression remains justified; the failure is
  annotation layout, not missing product rationale.

### Dependencies
- `docs/adr/ADR-004-verification-baseline-restoration.md`
- `docs/testing-and-release-strategy.md`
- `docs/standards-compliance-analysis/refactor-plan.md`
- `.github/workflows/quality-gates.yml`
- pass instructions and findings in this refactor package

### Risks
- Contract-test remediation may surface a real compatibility choice rather than
  a stale assertion.
- CI coverage expansion may expose new host-facing failures.
- rustfmt may normalize more files than the initial isolated drift suggests.

### Affected Structured Contracts
- workflow-service capabilities JSON contract
- workflow-service CI verification contract
- local a11y-ignore annotation policy enforced by `check-svelte-a11y.mjs`

### Affected Persisted Artifacts
- none expected beyond Markdown standards artifacts

### Concurrency / Race Review
- No runtime concurrency change is planned.
- Concurrent execution is safe only across non-overlapping write sets in Wave 01.
- CI and standards docs are shared surfaces and should be integrated serially in
  Wave 02.

### Definition of Done
- `cargo test -p pantograph-workflow-service --test contract` passes
- `npm run lint:no-new` passes
- `cargo fmt --all -- --check` passes
- blocking CI includes the workflow-service contract surface
- standards docs no longer overstate closeout status

## Findings Grouped by Code Area

### Workflow-Service Contract Surface
- stale capability snapshot versus current `WorkflowRuntimeCapability`
- missing blocking CI coverage for that contract surface

### Frontend Graph Surface
- `WorkflowGraph.svelte` contains an a11y-ignore annotation layout that fails
  the repo's own checker

### Rust Verification Baseline
- `workflow_session_execution.rs` still disagrees with rustfmt
- ADR-004 currently overstates the restored baseline

### Unrelated Issue Recorded Separately
- dirty asset changes under `assets/` remain outside remediation scope

## Overlapping Constraints and Unified Resolution
- Contract traceability and CI trust must be solved together: updating the
  snapshot without adding CI coverage leaves the same failure mode in place.
- Accessibility comment hygiene and no-new-debt trust must be solved together:
  passing frontend tests is not enough if the enforcement script still fails.
- Formatting restoration and ADR truthfulness must be solved together: a green
  formatter gate is required before the restoration claim is credible.

## Milestones

### M1: Restore Red Local Gates
Goal: make the currently red local gates pass without broadening scope.

Status:
- Complete on 2026-04-23.
- Implemented by updating the expected workflow capability contract snapshot to
  include the intentionally emitted `readiness_state`, adding a second explicit
  `a11y-reviewed` rationale for the workflow graph container suppressions, and
  restoring rustfmt-consistent import ordering in
  `workflow_session_execution.rs`.

Tasks:
1. Resolve the workflow-service capability contract mismatch by choosing one:
   - update the expected snapshot to include `readiness_state`, or
   - change the producer so `readiness_state` is not emitted for this contract
     path.
2. Fix the `a11y-reviewed` annotation layout in
   `packages/svelte-graph/src/components/WorkflowGraph.svelte`.
3. Apply rustfmt-consistent import ordering in
   `crates/pantograph-embedded-runtime/src/workflow_session_execution.rs`.

Affected files/areas:
- `crates/pantograph-workflow-service/tests/contract.rs`
- optionally `crates/pantograph-workflow-service/src/workflow/contracts.rs`
- `packages/svelte-graph/src/components/WorkflowGraph.svelte`
- `crates/pantograph-embedded-runtime/src/workflow_session_execution.rs`

Standards satisfied:
- verification baseline restoration
- public contract traceability
- accessibility gate compliance
- formatter baseline discipline

Dependencies:
- none; this is the first execution milestone

Risks:
- contract remediation could imply consumer-visible wire change

Validation:
- `cargo test -p pantograph-workflow-service --test contract`
- `npm run lint:no-new`
- `cargo fmt --all -- --check`

Observed result:
- all three checks passed on 2026-04-23

### M2: Align Blocking CI with Restored Gates
Goal: prevent the same classes of drift from recurring unobserved.

Status:
- Complete on 2026-04-23.
- Implemented by extending the blocking `rust-tests` job in
  `.github/workflows/quality-gates.yml` to run
  `cargo test -p pantograph-workflow-service --test contract`.

Tasks:
1. Extend `.github/workflows/quality-gates.yml` so a blocking Rust test job
   covers the workflow-service contract suite.
2. Add the new job to `quality-summary`.

Affected files/areas:
- `.github/workflows/quality-gates.yml`

Standards satisfied:
- CI/local verification equivalence
- auditable blocking-gate coverage

Dependencies:
- M1 must be green locally first

Risks:
- added CI runtime and possible newly exposed failures

Validation:
- review workflow YAML for explicit coverage
- rerun `cargo test -p pantograph-workflow-service --test contract`

Observed result:
- blocking Rust CI now includes the workflow-service contract suite
- `cargo test -p pantograph-workflow-service --test contract` passed on 2026-04-23

### M3: Repair Standards Traceability
Goal: make standards and ADR docs accurately describe the restored baseline.

Status:
- Complete on 2026-04-23.
- Implemented by updating `ADR-004`, `docs/testing-and-release-strategy.md`,
  and `docs/standards-compliance-analysis/refactor-plan.md` so they now
  describe the closeout slice accurately: the workflow-service contract suite is
  part of blocking CI coverage, targeted local verification names the contract
  suite explicitly, and the ADR records that the final baseline closeout
  happened in this remediation.

Tasks:
1. Update `ADR-004` if the formatter baseline was not actually restored until
   this closeout slice.
2. Update any standards/refactor documentation that currently implies the
   remediation is already fully complete.
3. Record final verification results in the coordination ledger and reports.

Affected files/areas:
- `docs/adr/ADR-004-verification-baseline-restoration.md`
- `docs/standards-compliance-analysis/refactor-plan.md`
- `docs/testing-and-release-strategy.md`
- this refactor package

Standards satisfied:
- traceable documentation
- README/ADR equivalence with actual tree state

Dependencies:
- M1 and M2 completed

Risks:
- docs can become stale immediately if verification is not rerun after edits

Validation:
- rerun the same M1 checks
- confirm docs match observed outcomes

Observed result:
- standards traceability docs now match the restored gates and blocking CI scope
- final closeout checks passed on 2026-04-23

## Safe Parallel Waves

### Wave 01
Parallel slices with non-overlapping primary write sets:
- worker-contracts
- worker-frontend-a11y
- worker-rustfmt

Shared files forbidden during the wave:
- `.github/workflows/quality-gates.yml`
- broad standards docs

Required reports:
- `reports/wave-01-worker-contracts.md`
- `reports/wave-01-worker-frontend-a11y.md`
- `reports/wave-01-worker-rustfmt.md`

### Wave 02
Serial/shared-surface wave:
- worker-ci
- worker-docs

Required reports:
- `reports/wave-02-worker-ci.md`
- `reports/wave-02-worker-docs.md`

## Ownership Slices
- Workflow contract owner:
  `crates/pantograph-workflow-service/tests/contract.rs` and adjacent contract
  modules only.
- Frontend a11y owner:
  `packages/svelte-graph/src/components/WorkflowGraph.svelte` and adjacent
  checker context only.
- Formatting owner:
  targeted embedded-runtime file plus ADR note if needed.
- CI owner:
  `.github/workflows/quality-gates.yml` and summary logic only.
- Docs owner:
  ADR/standards plan artifacts only.

## Coordination Ledger Structure
Use `coordination-ledger.md` to track:
- wave status
- dependency order
- validation results
- worker report links
- re-plan triggers
- unresolved follow-ups

## Re-Plan Triggers
- `readiness_state` requires an explicit compatibility decision affecting
  external consumers.
- rustfmt touches a materially larger set of Rust files than the isolated fix.
- CI broadening exposes failing suites outside this closeout scope.
- the user decides to fold dirty asset changes into this work.

## Completion Criteria
- all M1 checks pass locally
- CI config blocks on the workflow-service contract surface
- standards docs no longer claim completion ahead of the tree
- coordination ledger is updated with final outcomes
