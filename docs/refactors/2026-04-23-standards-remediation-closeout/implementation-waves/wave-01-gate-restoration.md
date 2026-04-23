# Wave 01: Gate Restoration

## Objective
Restore the currently red standards gates without changing unrelated product
behavior.

## Wave Type
Parallelizable with serial integration.

## Slices

### Slice A: Workflow-Service Contract Snapshot
- Owner: worker-contracts
- Primary write set:
  - `crates/pantograph-workflow-service/tests/contract.rs`
- Allowed adjacent write set:
  - `crates/pantograph-workflow-service/src/workflow/contracts.rs`
  - `crates/pantograph-workflow-service/src/workflow/tests/workflow_capabilities.rs`
- Read-only context:
  - `crates/pantograph-embedded-runtime/src/runtime_capabilities.rs`
  - `docs/adr/ADR-004-verification-baseline-restoration.md`
- Forbidden/shared files:
  - `.github/workflows/quality-gates.yml`
  - `packages/svelte-graph/src/components/WorkflowGraph.svelte`
- Output contract:
  - either the snapshot is updated to the accepted wire shape or the producer is
    changed to preserve the old contract, with explicit validation evidence
- Required report:
  - `reports/wave-01-worker-contracts.md`

### Slice B: Frontend A11y Gate Closeout
- Owner: worker-frontend-a11y
- Primary write set:
  - `packages/svelte-graph/src/components/WorkflowGraph.svelte`
- Allowed adjacent write set:
  - `scripts/check-svelte-a11y.mjs`
- Read-only context:
  - `package.json`
  - `.github/workflows/quality-gates.yml`
- Forbidden/shared files:
  - `crates/pantograph-workflow-service/tests/contract.rs`
  - `crates/pantograph-embedded-runtime/src/workflow_session_execution.rs`
- Output contract:
  - `npm run lint:no-new` passes without weakening the gate
- Required report:
  - `reports/wave-01-worker-frontend-a11y.md`

### Slice C: Rust Formatting Baseline
- Owner: worker-rustfmt
- Primary write set:
  - `crates/pantograph-embedded-runtime/src/workflow_session_execution.rs`
- Allowed adjacent write set:
  - `docs/adr/ADR-004-verification-baseline-restoration.md`
- Read-only context:
  - `.github/workflows/quality-gates.yml`
- Forbidden/shared files:
  - `crates/pantograph-workflow-service/tests/contract.rs`
  - `packages/svelte-graph/src/components/WorkflowGraph.svelte`
- Output contract:
  - `cargo fmt --all -- --check` passes or the report explains any additional
    files rustfmt still rewrites
- Required report:
  - `reports/wave-01-worker-rustfmt.md`

## Integration Sequence
1. Integrate Slice C first if rustfmt touches files outside the initial target.
2. Integrate Slice A and rerun contract tests.
3. Integrate Slice B and rerun frontend gates.
4. Run full wave verification.

## Wave Verification
- `cargo fmt --all -- --check`
- `cargo test -p pantograph-workflow-service --test contract`
- `npm run lint:no-new`
- `cargo check`
- `npm run typecheck`

## Risks
- Slice A may reveal a real compatibility decision rather than a stale test.
- Slice C may expand to nearby files if rustfmt normalizes additional imports.

## Re-Plan Trigger
If Slice A requires a public contract decision affecting consumers beyond the
snapshot, stop and add a compatibility note before starting Wave 02.
