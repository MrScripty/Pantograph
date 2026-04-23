# Final Plan: CI Quality Gates Reliability Remediation

## Objective
Restore GitHub Actions quality-gate reliability by making clean-checkout CI
runners capable of loading the Rust workspace, ensuring the no-new-debt lane
owns its required shell tooling, and fixing audit summary reporting so the
workflow tells the truth about non-blocking audit failures.

## Scope

### In Scope
- Rust CI bootstrap for the external `Pumas-Library` dependency
- Rustler smoke alignment with the same Rust workspace bootstrap contract
- traceability-lane shell tooling bootstrap or fallback behavior
- truthful quality-summary reporting for ratcheted audit jobs
- documentation updates directly tied to CI/bootstrap/reporting behavior

### Out of Scope
- unrelated asset worktree changes under `assets/`
- broad Rust dependency ownership redesign beyond what is required to make CI
  reproducible
- application feature changes
- promotion of ratcheted audits to blocking policy

## Inputs

### Problem
GitHub Actions run `24848578023` failed for two primary reasons:
1. Rust jobs could not load the workspace because the root manifest depends on
   `../Pumas-Library/rust/crates/pumas-core`, but CI only checked out the
   Pantograph repository.
2. `lint:no-new` failed because the decision-traceability script requires `rg`,
   but the CI lane does not install it and the script has no fallback.

A secondary workflow defect also surfaced: the quality summary prints the
non-blocking audit jobs as `success` even when those jobs actually failed,
because the summary reads `needs.<job>.result` from `continue-on-error` jobs.

### Constraints
- Preserve existing blocking versus non-blocking quality-gate policy.
- Keep the remediation auditable and narrowly scoped to CI reliability and
  truthful reporting.
- Preserve current public crate, smoke-harness, and workflow-service contract
  facades unless a separate decision approves a dependency-ownership change.
- Do not fold unrelated `assets/` churn into the remediation.

### Assumptions
- The intended current design is that Pantograph may rely on a sibling
  `Pumas-Library` checkout in local development, but CI must bootstrap that
  relationship explicitly.
- `ripgrep` is preferred for the traceability script but not the only
  acceptable implementation path if an equivalent fallback can preserve
  semantics.
- The audit jobs should remain non-blocking after this remediation.

### Dependencies
- `.github/workflows/quality-gates.yml`
- root `Cargo.toml`
- `scripts/check-decision-traceability.sh`
- `scripts/check-rustler-beam-smoke.sh`
- `docs/testing-and-release-strategy.md`
- `docs/toolchain-policy.md`

### Risks
- External repository bootstrap may require credentials, ref pinning, or
  visibility handling not yet encoded in the workflow.
- A minimal CI-only fix could entrench an undocumented cross-repo dependency if
  docs are not updated at the same time.
- Traceability tooling fixes could accidentally weaken the gate if they change
  matching semantics.
- Summary reporting fixes could accidentally turn audit lanes blocking if the
  workflow conditions are rewritten carelessly.

### Affected Structured Contracts
- GitHub Actions quality-gates workflow result contract
- Rust workspace dependency/bootstrap contract for CI
- decision-traceability gate execution contract

### Affected Persisted Artifacts
- documentation and planning Markdown only

### Concurrency / Race Review
- No runtime concurrency change is planned.
- Workflow YAML is a shared configuration surface and should have one explicit
  owner per implementation wave.
- Script and docs changes can run in parallel only when they do not overlap the
  same shared workflow file.

### Definition of Done
- Rust CI jobs can load the workspace manifests on clean GitHub runners.
- `lint:no-new` runs successfully in CI without undeclared runner tools.
- quality summary reports actual audit outcomes truthfully while preserving
  their non-blocking status.
- docs describing CI/testing/toolchain behavior match the implemented workflow.

## Standards Groups Reviewed
- CI and tooling bootstrap requirements
- clean-checkout reproducibility
- README/ADR and traceability-gate equivalence
- truthful audit and summary reporting

## Findings Grouped by Code Area

### Rust Workspace and External Dependency Bootstrap
- root `Cargo.toml` hard-codes `pumas-library` as a sibling path dependency
- every Rust CI lane currently assumes that sibling exists
- Rustler smoke is not a distinct failure; it inherits the same workspace load
  failure before its BEAM-specific logic begins

### Traceability Lint Tooling
- the traceability script hard-fails when `rg` is absent
- the Node-based lint lane does not provision `ripgrep` or another equivalent
  search tool
- current docs do not make the non-Node prerequisite explicit

### Quality Summary and Audit Reporting
- `continue-on-error` audit jobs are printed as `success` in `quality-summary`
  even when they fail
- docs claim the summary reports audit job status for review, which is not true
  in the observed run

### Unrelated Issues Recorded Separately
- unrelated dirty asset worktree changes remain outside remediation scope
- the GitHub connector token in this CLI session expired and had to be worked
  around with local `gh` authentication; this is session infrastructure, not a
  Pantograph repo issue

## Overlapping Constraints and Unified Resolution
- Rust bootstrap and Rustler smoke must be solved together because the smoke
  lane cannot even reach BEAM execution until the workspace loads cleanly.
- Traceability reliability must be solved as a bootstrap problem, not as a lint
  content problem, because the current lane never reaches the actual diff
  inspection.
- Audit reporting must be solved without changing ratcheted-audit policy;
  truthful reporting and blocking semantics are separate concerns and should not
  be conflated.
- Documentation changes must land with the workflow/script changes so CI/local
  equivalence remains auditable.

## Milestones

### M1: Restore Clean-Checkout Rust CI Bootstrap
Goal: make Rust CI lanes able to load the workspace from a clean GitHub runner.

Status:
- Complete on 2026-04-23.
- Implemented by pinning `Pumas-Library` to commit
  `66c0c11a57b8bfe8fb70d827efced0fbc442b156` in the workflow, checking that
  repository out in every Rust lane, and linking it into the sibling path that
  the Cargo workspace already expects.

Tasks:
1. Choose and document the external dependency bootstrap strategy for
   `Pumas-Library`:
   - explicit checkout/bootstrap in CI to the sibling path expected by Cargo, or
   - a bounded dependency-ownership change if CI checkout is not viable.
2. Apply the same bootstrap contract to:
   - `rust-check`
   - `rust-tests`
   - `rust-doc-tests`
   - `rust-format-audit`
   - `rust-clippy-audit`
   - `rustler-beam-smoke`
3. Ensure the Rustler smoke lane does not carry a divergent bootstrap path.

Affected files/areas:
- `.github/workflows/quality-gates.yml`
- potentially a dedicated CI helper script
- docs describing CI bootstrap if needed

Standards satisfied:
- clean-checkout reproducibility
- explicit CI bootstrap ownership
- CI/local verification equivalence

Dependencies:
- none; this is the first implementation milestone

Risks:
- external repo checkout may need authentication or ref pinning decisions
- a manifest-level redesign would broaden scope materially

Validation:
- `cargo check --workspace --no-default-features`
- `cargo check --workspace --all-features`
- `cargo test -p node-engine --lib`
- `cargo test -p workflow-nodes --lib`
- `cargo test -p pantograph-workflow-service --test contract`
- `cargo test --workspace --doc --no-default-features`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `./scripts/check-rustler-beam-smoke.sh` or the CI-equivalent lane

Observed result:
- all Rust workspace validation commands passed locally on 2026-04-23
- local Rustler BEAM smoke remained environment-blocked because `mix` is not
  installed on this workstation; the CI lane remains the authoritative check

### M2: Make the No-New-Debt Gate Self-Contained
Goal: remove the CI-only bootstrap failure from the traceability lane.

Status:
- Complete on 2026-04-23.
- Implemented by teaching `scripts/check-decision-traceability.sh` to use
  `grep` when `rg` is unavailable, preserving the existing ripgrep fast path
  while removing the hard CI bootstrap dependency.

Tasks:
1. Choose the portability strategy:
   - provision `ripgrep` explicitly in the lint CI job, or
   - add a standards-compliant fallback path in
     `scripts/check-decision-traceability.sh`, optionally keeping CI bootstrap
     explicit as well.
2. Preserve the existing traceability semantics and fast path with `rg` where
   available.
3. Update docs if the gate has non-Node prerequisites or workflow-owned shell
   bootstrap.

Affected files/areas:
- `scripts/check-decision-traceability.sh`
- optionally `.github/workflows/quality-gates.yml`
- `docs/testing-and-release-strategy.md`
- `docs/toolchain-policy.md`

Standards satisfied:
- explicit tool bootstrap
- reliable lint gate execution
- truthful CI/local gate equivalence

Dependencies:
- can begin after the shared workflow ownership for M1 is understood

Risks:
- fallback logic may drift from `rg` semantics if implemented carelessly
- workflow edits may overlap M1 ownership if not sequenced clearly

Validation:
- `npm run lint:no-new`
- direct invocation of `./scripts/check-decision-traceability.sh` in a
  provisioned CI-like shell

Observed result:
- `npm run lint:no-new` passed on 2026-04-23
- `env PATH=/usr/bin:/bin ./scripts/check-decision-traceability.sh` passed on
  2026-04-23, confirming the non-`rg` fallback path works

### M3: Repair Quality Summary Audit Reporting
Goal: keep the summary truthful without changing audit blocking policy.

Status:
- Complete on 2026-04-23.
- Implemented by giving the non-blocking Rust audit jobs explicit
  `audit-result` outputs, recording each audit step outcome before any final
  failure marker, and teaching `quality-summary` to print those outputs instead
  of `needs.<job>.result`.

Tasks:
1. Replace the current summary logic for `rust-format-audit` and
   `rust-clippy-audit` with a mechanism that reports actual audit outcome even
   when the jobs use `continue-on-error: true`.
2. Keep required-gate pass/fail semantics unchanged.
3. Make the summary text clearly distinguish:
   - required gate status
   - ratcheted audit status

Affected files/areas:
- `.github/workflows/quality-gates.yml`

Standards satisfied:
- truthful CI reporting
- auditable ratcheted-audit visibility

Dependencies:
- M1 and M2 should land first so the summary can be validated against a cleaner
  workflow state

Risks:
- a naive summary rewrite could accidentally re-block audit jobs or hide them
  entirely

Validation:
- inspect a subsequent CI run and confirm audit lines match actual job outcome
- verify the summary still fails only when required gates fail

Observed result:
- local workflow parse sanity check passed on 2026-04-23
- a post-implementation GitHub Actions run is still required to confirm the
  end-to-end summary output on a real runner

### M4: Align CI and Tooling Documentation
Goal: ensure written standards match the implemented workflow after the fixes.

Status:
- Complete on 2026-04-23.
- Implemented by updating `docs/testing-and-release-strategy.md` and
  `scripts/README.md`, plus the host-owned remediation ledger, to match the
  implemented Rust bootstrap, traceability fallback, and audit-summary
  behavior.

Tasks:
1. Update `docs/testing-and-release-strategy.md` to describe:
   - the external Rust dependency bootstrap contract
   - the traceability lane prerequisites/bootstrap
   - truthful audit summary behavior
2. Update `docs/toolchain-policy.md` if workflow-owned tools or host lanes now
   have additional explicit provisioning requirements.
3. Update this refactor package with implementation results and verification.

Affected files/areas:
- `docs/testing-and-release-strategy.md`
- `docs/toolchain-policy.md`
- this refactor package

Standards satisfied:
- documentation equivalence with actual tree state
- auditable CI policy

Dependencies:
- M1 through M3 complete

Risks:
- docs can drift again if updated before the workflow is stable

Validation:
- manual review against final workflow YAML
- rerun the affected local/CI-equivalent commands named in the docs

Observed result:
- `npm run lint:no-new` passed on the committed tree on 2026-04-23
- `env PATH=/usr/bin:/bin ./scripts/check-decision-traceability.sh` passed on
  the committed tree on 2026-04-23
- `quality-gates.yml` parsed successfully via Python/YAML on 2026-04-23

## Safe Parallel Waves

### Wave 01
Parallel slices with non-overlapping primary write sets:
- `worker-ci-rust-bootstrap`
- `worker-traceability-tooling`

Shared files forbidden during the wave:
- `.github/workflows/quality-gates.yml` for the traceability worker

Required reports:
- `reports/wave-01-worker-ci-rust-bootstrap.md`
- `reports/wave-01-worker-traceability-tooling.md`

### Wave 02
Serial/shared-surface wave:
- `worker-summary-and-docs`

Required reports:
- `reports/wave-02-worker-summary-and-docs.md`

## Ownership Slices
- Rust bootstrap owner:
  `.github/workflows/quality-gates.yml` and any CI helper script that resolves
  `Pumas-Library` for clean runners.
- Traceability tooling owner:
  `scripts/check-decision-traceability.sh` and related docs, unless the chosen
  fix is workflow-only.
- Summary owner:
  `quality-summary` and audit job reporting logic in the workflow.
- Docs owner:
  `docs/testing-and-release-strategy.md`, `docs/toolchain-policy.md`, and this
  refactor package.

## Risks and Mitigations
- Risk: external repository checkout requires credentials not available to the
  workflow.
  Mitigation: validate repository visibility/auth early; if unavailable,
  re-plan before touching unrelated lanes.
- Risk: script fallback changes traceability semantics.
  Mitigation: preserve `rg` as canonical behavior and compare fallback output on
  representative diffs.
- Risk: summary logic becomes more complex and brittle.
  Mitigation: keep required-gate failure logic and audit reporting logic
  separate and explicit.

## Re-Plan Triggers
- CI cannot fetch or materialize `Pumas-Library` with available credentials.
- The smallest viable Rust fix requires changing workspace dependency ownership.
- The traceability lane cannot be made self-contained without overlapping the
  shared workflow slice in a way that invalidates Wave 01.
- GitHub Actions summary semantics do not permit truthful audit reporting with
  the current job model.

## Completion Criteria
- A follow-up CI run proves Rust jobs can load the workspace from a clean
  checkout.
- `lint:no-new` passes in CI without undeclared shell-tool failures.
- The summary no longer reports failed audits as `success`.
- Docs describing CI/testing/toolchain behavior match the implemented workflow
  and helper scripts.

## Completion Summary
- M1 is complete: Rust quality-gate jobs now bootstrap the pinned external
  `Pumas-Library` checkout into the sibling path expected by the workspace.
- M2 is complete: the decision-traceability script now works with either
  `ripgrep` or standard `grep`, and the committed tree passes `lint:no-new`
  locally.
- M3 is complete in code: the workflow now reports explicit audit outcomes for
  the non-blocking Rust audit jobs; a follow-up GitHub Actions run is still the
  remaining validation step for that summary behavior on a real runner.
- M4 is complete: the testing and script documentation now reflect the
  implemented CI bootstrap and traceability behavior.
