# Wave 01: Bootstrap and Lint Foundation

## Goal
Restore clean-checkout CI viability by fixing Rust external dependency
bootstrap and the traceability lane’s undeclared tool dependency.

## Ownership Slices

### worker-ci-rust-bootstrap
- Assigned scope:
  - materialize or otherwise resolve the external `Pumas-Library` dependency in
    Rust CI lanes
  - ensure the Rustler smoke lane uses the same bootstrap path
- Expected output contract:
  - Rust jobs can load the workspace manifests on clean runners
  - workflow YAML clearly owns the bootstrap behavior
- Primary write set:
  - `.github/workflows/quality-gates.yml`
  - any new CI helper script created specifically for dependency checkout/setup
- Allowed adjacent write set:
  - `docs/testing-and-release-strategy.md`
  - `docs/toolchain-policy.md`
- Read-only context:
  - `Cargo.toml`
  - `scripts/check-rustler-beam-smoke.sh`
- Forbidden/shared files:
  - `scripts/check-decision-traceability.sh`
  - `quality-summary` block if another worker owns it in a later wave
- External-change escalation rule:
  - if the fix requires changing workspace dependency ownership rather than CI
    bootstrap, stop and record it instead of editing manifests opportunistically
- Worker report path:
  - `reports/wave-01-worker-ci-rust-bootstrap.md`

### worker-traceability-tooling
- Assigned scope:
  - remove the `rg` bootstrap failure from the no-new-debt lane without
    weakening traceability semantics
- Expected output contract:
  - `lint:no-new` can run on clean CI runners
  - shell tooling ownership is explicit
- Primary write set:
  - `scripts/check-decision-traceability.sh`
- Allowed adjacent write set:
  - `docs/testing-and-release-strategy.md`
  - `docs/toolchain-policy.md`
- Read-only context:
  - `.github/workflows/quality-gates.yml`
- Forbidden/shared files:
  - workflow YAML bootstrap steps owned by `worker-ci-rust-bootstrap`
- External-change escalation rule:
  - if the only safe fix requires editing the workflow bootstrap, record it in
    the worker report instead of overlapping the shared workflow file
- Worker report path:
  - `reports/wave-01-worker-traceability-tooling.md`

## Integration Sequence
1. Integrate the Rust CI bootstrap slice first.
2. Integrate the traceability tooling slice second.
3. Run the wave verification set after both slices are integrated.

## Required Verification
- `cargo check --workspace --no-default-features`
- `cargo check --workspace --all-features`
- `cargo test -p node-engine --lib`
- `cargo test -p workflow-nodes --lib`
- `cargo test -p pantograph-workflow-service --test contract`
- `cargo test --workspace --doc --no-default-features`
- `./scripts/check-rustler-beam-smoke.sh` or CI-equivalent bounded smoke
- `npm run lint:no-new`

## Cleanup Requirements
- Remove any temporary worker worktrees after integration verification passes.
- Record any unresolved external repository credential assumptions in the wave
  reports before cleanup.
