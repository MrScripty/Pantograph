# Pass 01: Verification Contracts and CI Coverage

## Purpose
Inspect the standards-closeout gaps around workflow-service contract snapshots
and verification coverage. Keep the standards prompt at
`/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/prompts/full-codebase-standards-refactor.md`
prominent while reviewing this pass.

## Standards Focus
- Combined standards constraints for verification baseline restoration,
  tooling/CI equivalence, contract traceability, and public facade stability.
- Do not discard broad or qualitative standards; record how they constrain the
  remediation even when they are not reducible to one lint rule.

## Code Areas to Inspect
- `crates/pantograph-workflow-service/tests/contract.rs`
- `crates/pantograph-workflow-service/src/workflow/contracts.rs`
- `crates/pantograph-embedded-runtime/src/runtime_capabilities.rs`
- `.github/workflows/quality-gates.yml`
- `docs/adr/ADR-004-verification-baseline-restoration.md`

## Required Output
Write findings under `findings/01-verification-contracts.md`.

Each finding must include:
- affected files and relevant code areas
- violated or constraining standards
- required remediation constraints
- whether the issue is a true product/contract bug, stale test fixture, or CI
  coverage gap

Record unrelated but relevant issues separately if they will not be resolved by
this remediation slice.
