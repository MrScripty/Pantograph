# Coordination Ledger

## Objective
Track sequencing, constraints, and future worker handoffs for the Linux native
package bootstrap remediation that remains after the earlier CI reliability
fixes.

## Current Status
| Item | Status | Notes |
| ---- | ------ | ----- |
| Planning | complete | Findings and waves recorded |
| Source implementation | pending | This package is planning-only |
| CI verification | pending | Awaiting implementation |

## Shared Constraints
- Do not modify unrelated asset worktree changes under `assets/`.
- Preserve the existing sibling `Pumas-Library` bootstrap and truthful
  audit-summary reporting.
- Keep this remediation focused on Ubuntu native package bootstrap and any
  required doc alignment; do not widen into unrelated workspace refactors
  unless a re-plan is recorded.

## Execution Notes
- 2026-04-23: planning package created from GitHub Actions run `24850496670`
  after authenticated log inspection.
- The previous CI remediation already fixed sibling dependency bootstrap,
  traceability tooling fallback, and audit-summary truthfulness.

## Dependency Order
1. Add Ubuntu native package bootstrap for the remaining Rust lanes.
2. Re-run GitHub Actions and inspect the next missing package or success state.
3. Align docs once the runner bootstrap is stable.

## Validation Ledger
| Check | Current observed result | Source |
| ----- | ----------------------- | ------ |
| `Critical lint and traceability` CI job | pass | GitHub Actions run `24850496670` |
| `Rust focused tests` CI job | pass | GitHub Actions run `24850496670` |
| `Rust doc tests` CI job | pass | GitHub Actions run `24850496670` |
| `Rustler BEAM smoke` CI job | pass | GitHub Actions run `24850496670` |
| `Rust format audit` CI job | pass | GitHub Actions run `24850496670` |
| `Rust workspace check` CI job | fail | missing `glib-2.0.pc` / Ubuntu native packages |
| `Rust clippy warning audit` CI job | fail | same missing `glib-2.0.pc` / Ubuntu native packages |
| `Quality summary` CI job | fail | correctly reflecting `rust-check` failure and clippy audit failure |

## Worker Report Paths
- `reports/wave-01-worker-native-ci-bootstrap.md`
- `reports/wave-02-worker-native-docs.md`

## Re-Plan Triggers
- Installing `glib` development packages reveals a broader required GTK/WebKit
  package set that changes the expected bootstrap surface materially.
- The workspace check lane must be narrowed rather than bootstrapped on Ubuntu.
- The runner image changes in a way that invalidates the chosen package list.
