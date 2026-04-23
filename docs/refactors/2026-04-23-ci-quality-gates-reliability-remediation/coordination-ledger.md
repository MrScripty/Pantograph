# Coordination Ledger

## Objective
Track sequencing, shared constraints, verification targets, and future worker
handoffs for the CI quality-gates reliability remediation.

## Current Status
| Item | Status | Notes |
| ---- | ------ | ----- |
| Planning | complete | Findings and implementation waves recorded |
| Source implementation | in progress | User approved continuing with unrelated `assets/` dirt left untouched |
| M1 Rust bootstrap | complete | Workflow now materializes pinned `Pumas-Library` checkout for every Rust lane |
| M2 traceability tooling | complete | Script now falls back to standard grep when ripgrep is unavailable |
| CI verification | pending | Awaiting post-implementation CI run |

## Shared Constraints
- Do not modify unrelated asset worktree changes under `assets/`.
- Preserve current blocking versus ratcheted audit policy unless explicitly
  changed by a separate standards decision.
- Keep the remediation focused on CI reliability and truthful reporting; do not
  widen into unrelated Rust feature or dependency refactors.
- Preserve existing public crate and smoke-harness facades unless a deliberate,
  documented dependency-ownership change is approved.

## Execution Notes
- 2026-04-23: planning package created from GitHub Actions run `24848578023`
  after authenticated log inspection.
- Observed local worktree has unrelated dirty asset changes that remain outside
  this remediation scope.
- 2026-04-23: user explicitly approved proceeding after committing the Markdown
  planning artifacts while leaving unrelated dirty asset changes under
  `assets/` untouched.

## Dependency Order
1. Resolve Rust external dependency bootstrap for clean CI runners.
2. Resolve traceability lane tool bootstrap or fallback semantics.
3. Fix quality summary audit reporting.
4. Align testing/toolchain docs with the implemented workflow behavior.

## Validation Ledger
| Check | Current observed result | Source |
| ----- | ----------------------- | ------ |
| `Dependency audit` CI job | pass | GitHub Actions run `24848578023` |
| `TypeScript typecheck` CI job | pass | GitHub Actions run `24848578023` |
| `Full lint` CI job | pass | GitHub Actions run `24848578023` |
| `Frontend tests` CI job | pass | GitHub Actions run `24848578023` |
| `Rust workspace check` CI job | fail | missing sibling `Pumas-Library` manifest |
| `Rust focused tests` CI job | fail | missing sibling `Pumas-Library` manifest |
| `Rust doc tests` CI job | fail | missing sibling `Pumas-Library` manifest |
| `Rustler BEAM smoke` CI job | fail | missing sibling `Pumas-Library` manifest |
| `Rust format audit` CI job | fail | same manifest bootstrap failure |
| `Rust clippy warning audit` CI job | fail | same manifest bootstrap failure |
| `Critical lint and traceability` CI job | fail | missing `rg` in runner |
| `Quality summary` CI job | fail | required lanes failed; audit lines misreported as success |
| `cargo check --workspace --no-default-features` | pass | 2026-04-23 M1 local verification |
| `cargo check --workspace --all-features` | pass | 2026-04-23 M1 local verification |
| `cargo test -p node-engine --lib` | pass | 2026-04-23 M1 local verification |
| `cargo test -p workflow-nodes --lib` | pass | 2026-04-23 M1 local verification |
| `cargo test -p pantograph-workflow-service --test contract` | pass | 2026-04-23 M1 local verification |
| `cargo test --workspace --doc --no-default-features` | pass | 2026-04-23 M1 local verification |
| `cargo fmt --all -- --check` | pass | 2026-04-23 M1 local verification |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | pass | 2026-04-23 M1 local verification |
| `./scripts/check-rustler-beam-smoke.sh` | blocked | local environment missing `mix`; CI lane remains required |
| `npm run lint:no-new` | pass | 2026-04-23 M2 local verification |
| `env PATH=/usr/bin:/bin ./scripts/check-decision-traceability.sh` | pass | 2026-04-23 M2 fallback verification without `rg` |

## Worker Report Paths
- `reports/wave-01-worker-ci-rust-bootstrap.md`
- `reports/wave-01-worker-traceability-tooling.md`
- `reports/wave-02-worker-summary-and-docs.md`

## Re-Plan Triggers
- The external `Pumas-Library` dependency cannot be fetched in CI with the
  repository’s available credentials or visibility model.
- Fixing the Rust bootstrap issue requires changing workspace dependency
  ownership instead of workflow bootstrap only.
- The traceability script cannot preserve equivalent semantics without a
  workflow bootstrap change that overlaps the shared Rust bootstrap work.
- Truthful audit reporting cannot be achieved within the current summary job
  structure.
