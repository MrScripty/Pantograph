# Wave 02: CI and Traceability Closeout

## Objective
Align blocking CI coverage and standards documentation with the restored local
verification baseline.

## Wave Type
Mostly serial because CI config and standards docs are shared surfaces.

## Slices

### Slice A: Blocking CI Coverage
- Owner: worker-ci
- Primary write set:
  - `.github/workflows/quality-gates.yml`
- Allowed adjacent write set:
  - `package.json`
  - `scripts/README.md`
- Read-only context:
  - `crates/pantograph-workflow-service/tests/contract.rs`
  - `docs/adr/ADR-004-verification-baseline-restoration.md`
- Forbidden/shared files:
  - source files already stabilized in Wave 01 unless required by integration
- Output contract:
  - blocking CI includes the workflow-service contract surface and reports it in
    `quality-summary`
- Required report:
  - `reports/wave-02-worker-ci.md`

### Slice B: Standards Traceability Update
- Owner: worker-docs
- Primary write set:
  - `docs/adr/ADR-004-verification-baseline-restoration.md`
  - `docs/standards-compliance-analysis/refactor-plan.md`
  - `docs/testing-and-release-strategy.md`
- Allowed adjacent write set:
  - `docs/refactors/2026-04-23-standards-remediation-closeout/*`
- Read-only context:
  - `.github/workflows/quality-gates.yml`
  - Wave 01 reports
- Forbidden/shared files:
  - product source files
- Output contract:
  - standards docs no longer claim a restored baseline or complete closeout
    unless the corresponding gates are actually green
- Required report:
  - `reports/wave-02-worker-docs.md`

## Integration Sequence
1. Merge CI coverage changes.
2. Merge standards-traceability updates after the final verification rerun.

## Wave Verification
- `cargo test -p pantograph-workflow-service --test contract`
- `npm run lint:no-new`
- `cargo fmt --all -- --check`
- review rendered CI matrix and summary job inputs

## Risks
- CI runtime may increase if a new blocking job is added.
- Documentation can become stale again if verification is not rerun after merge.

## Re-Plan Trigger
If CI broadening exposes new failing contract or binding suites, pause and add a
new remediation slice rather than silently downgrading the gate.
