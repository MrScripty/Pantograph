# ADR-004: Verification Baseline Restoration

## Status
Accepted

## Context
Pantograph's standards-compliance refactor has been decomposing oversized files
and tightening local/CI quality gates in parallel. After the large-file pass,
the broad verification rerun surfaced two non-behavioral issues:

- `cargo fmt --all -- --check` failed because formatting drift had accumulated
  across multiple Rust directories.
- `npm run lint:full` failed because
  `packages/svelte-graph/src/backends/MockWorkflowBackend.ts` still imported an
  unused symbol after earlier extraction work.

Those failures were not architectural regressions, but leaving them unresolved
would keep the documented verification baseline out of sync with the actual
tree and make future standards work harder to trust.

## Decision
Restore the verification baseline immediately when a broad hardening rerun
finds non-behavioral drift:

1. Apply the canonical formatter across the affected Rust tree rather than
   carrying local formatting debt forward.
2. Resolve mechanical lint regressions in the same hardening slice when they
   are a direct result of recent refactors or extractions.
3. Record the restoration in the standards-compliance plan so later passes can
   distinguish real product/contract regressions from already-closed hygiene
   drift.

## Consequences

### Positive
- `cargo fmt --all -- --check` and `npm run lint:full` remain trustworthy local
  equivalents of the documented CI gates.
- Repo-wide formatting drift does not accumulate silently behind ratcheted
  checks.
- Future verification failures are more likely to indicate substantive issues
  instead of stale baseline noise.

### Negative
- A formatting restoration slice may touch many directories at once even when
  behavior does not change.
- Reviewers need an explicit record explaining why broad formatting churn was
  intentional.

## Implementation Notes
- Use one contained commit for the verification-baseline restoration so it does
  not hide product changes.
- Do not mix unrelated asset or generated-file churn into the restoration
  commit.
- Re-run the same gates that failed before claiming the baseline is restored.

## Compliance Mapping
- Standards enforcement and tooling hardening.
- Documented local verification equivalence to CI.
- Traceable non-behavioral cleanup when broad quality gates surface drift.
