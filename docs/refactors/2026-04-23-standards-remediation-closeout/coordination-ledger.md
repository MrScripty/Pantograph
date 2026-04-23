# Coordination Ledger

## Objective
Track host-owned sequencing, dependencies, validation results, and handoffs for
the standards-closeout remediation.

## Current Status
| Item | Status | Notes |
| ---- | ------ | ----- |
| Wave 01 planning | complete | Gate restoration slices defined |
| Wave 01 implementation | complete | M1 local gate restoration finished and verified |
| Wave 02 planning | complete | CI and traceability alignment defined |
| Wave 02 implementation | complete | Blocking CI now covers workflow-service contract tests |
| Source implementation | complete | M1-M3 finished with unrelated `assets/` dirt left untouched by user approval |

## Shared Constraints
- Do not modify unrelated asset worktree changes under `assets/`.
- Preserve backend-owned workflow contracts unless a compatibility decision is
  explicitly recorded.
- Keep formatter restoration isolated from unrelated behavior changes.
- Do not weaken the no-new-debt or accessibility gates to make them pass.

## Execution Notes
- 2026-04-23: user explicitly authorized continuing implementation while
  leaving unrelated dirty asset changes under `assets/` untouched.

## Dependency Order
1. Resolve Wave 01 source-level gate failures.
2. Re-run local verification and capture results.
3. Expand blocking CI coverage.
4. Update standards traceability text to match actual gate state.

## Validation Ledger
| Check | Current observed result | Source |
| ----- | ----------------------- | ------ |
| `cargo test -p node-engine --lib` | pass | 2026-04-23 reassessment |
| `cargo test -p pantograph-workflow-service -p pantograph-embedded-runtime --lib` | pass | 2026-04-23 reassessment |
| `cargo test -p pantograph-workflow-service --test contract` | fail | `workflow_capabilities_contract_snapshot` |
| `cargo check` | pass | 2026-04-23 reassessment |
| `npm run typecheck` | pass | 2026-04-23 reassessment |
| `npm run test:frontend` | pass | 228 tests |
| `npm run lint:no-new` | fail | `reviewed-a11y-ignore` in `WorkflowGraph.svelte` |
| `cargo fmt --all -- --check` | fail | import ordering drift in `workflow_session_execution.rs` |
| `cargo test -p pantograph-workflow-service --test contract` | pass | 2026-04-23 M1 verification |
| `npm run lint:no-new` | pass | 2026-04-23 M1 verification |
| `cargo fmt --all -- --check` | pass | 2026-04-23 M1 verification |
| blocking CI coverage for workflow-service contract suite | pass | 2026-04-23 M2 workflow update |
| `cargo test -p pantograph-workflow-service --test contract` | pass | 2026-04-23 final closeout verification |
| `npm run lint:no-new` | pass | 2026-04-23 final closeout verification |
| `cargo fmt --all -- --check` | pass | 2026-04-23 final closeout verification |
| standards traceability docs match gate state | pass | 2026-04-23 M3 doc update |

## Worker Report Paths
- `reports/wave-01-worker-contracts.md`
- `reports/wave-01-worker-frontend-a11y.md`
- `reports/wave-01-worker-rustfmt.md`
- `reports/wave-02-worker-ci.md`
- `reports/wave-02-worker-docs.md`

## Re-Plan Triggers
- Contract remediation changes the supported wire shape for workflow capability
  consumers.
- rustfmt touches broader file sets than the isolated baseline-restoration slice.
- CI broadening exposes additional failing host-facing suites.
- The user decides the dirty asset worktree must be folded into the remediation.
