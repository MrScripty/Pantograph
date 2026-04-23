# Findings: Verification Contracts and CI Coverage

## Summary
Core Rust behavior is green, but one public contract snapshot is still stale and
the current blocking CI matrix does not cover that surface.

## Findings

### F01: Workflow capability contract snapshot is stale
- Affected files:
  - `crates/pantograph-workflow-service/tests/contract.rs`
  - `crates/pantograph-workflow-service/src/workflow/contracts.rs`
- Relevant code areas:
  - `workflow_capabilities_contract_snapshot`
  - `WorkflowRuntimeCapability.readiness_state`
- Evidence:
  - `cargo test -p pantograph-workflow-service --test contract` fails because
    the serialized response now includes `runtime_capabilities[].readiness_state`
    while the expected JSON omits it.
- Standards constrained:
  - public contract traceability
  - verification baseline equivalence
  - facade-first compatibility discipline
- Required remediation constraints:
  - decide explicitly whether `readiness_state` is part of the supported
    contract
  - if yes, update the snapshot and any supporting contract docs
  - if no, remove or suppress emission at the producer contract boundary rather
    than letting tests silently drift
  - preserve snake_case wire stability either way
- Classification:
  - contract/test drift, not core runtime instability

### F02: Blocking CI does not cover the workflow-service contract suite
- Affected files:
  - `.github/workflows/quality-gates.yml`
- Relevant code areas:
  - `rust-tests`
  - `quality-summary`
- Evidence:
  - blocking Rust jobs run `cargo test -p node-engine --lib` and
    `cargo test -p workflow-nodes --lib`, but not
    `cargo test -p pantograph-workflow-service --test contract`
- Standards constrained:
  - CI/local verification equivalence
  - documented quality-gate trustworthiness
- Required remediation constraints:
  - add an explicit blocking workflow-service contract test command or broaden
    the Rust test matrix to include host-facing contract suites
  - surface the new job in `quality-summary`
  - avoid masking contract failures behind non-blocking audit jobs
- Classification:
  - CI coverage gap

## Non-Blocking Context
- `cargo test -p pantograph-workflow-service -p pantograph-embedded-runtime --lib`
  passed.
- `cargo check` passed.
- The remaining issue is at the public contract snapshot layer.
