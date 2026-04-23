# ADR-004: Verification Baseline Restoration

## Status
Accepted

## Context
Pantograph's standards-compliance refactor has been decomposing oversized files
and tightening local/CI quality gates in parallel. An earlier hardening pass
restored the broad formatter and full-lint baseline after the large-file
extractions. A later closeout reassessment on 2026-04-23 found three remaining
trust gaps:

- `cargo test -p pantograph-workflow-service --test contract` still failed
  because the workflow capability snapshot had not been updated to match the
  intentionally emitted `readiness_state`.
- `npm run lint:no-new` still failed because one `WorkflowGraph.svelte`
  accessibility suppression was missing its own adjacent `a11y-reviewed`
  rationale.
- `cargo fmt --all -- --check` still failed because
  `workflow_session_execution.rs` had drifted from rustfmt ordering.

The reassessment also showed that blocking CI still omitted the
workflow-service contract suite, so one host-facing contract drift could merge
even after the local gate was fixed.

## Decision
Restore the verification baseline immediately when a hardening rerun or
standards closeout finds non-behavioral drift:

1. Apply the canonical formatter across the affected Rust tree rather than
   carrying local formatting debt forward.
2. Resolve mechanical lint regressions in the same hardening slice when they
   are a direct result of recent refactors or extractions.
3. Update contract snapshots or equivalent gate fixtures when the producer's
   supported wire shape has changed intentionally.
4. Add blocking CI coverage when the restored local gate protects a
   host-facing contract surface that CI does not yet exercise.
5. Record the restoration in the standards-compliance plan and ADR trail so
   later passes can distinguish real product/contract regressions from
   already-closed hygiene drift.

## Consequences

### Positive
- `cargo fmt --all -- --check` and `npm run lint:full` remain trustworthy local
  equivalents of the documented CI gates.
- `cargo test -p pantograph-workflow-service --test contract` is treated as a
  required host-facing contract gate rather than an optional cleanup task.
- Repo-wide formatting drift does not accumulate silently behind ratcheted
  checks.
- Future verification failures are more likely to indicate substantive issues
  instead of stale baseline noise.
- ADR and standards documentation stay aligned with the actual tree state after
  closeout slices, not just after the first large cleanup pass.

### Negative
- A formatting restoration slice may touch many directories at once even when
  behavior does not change.
- Reviewers need an explicit record explaining why broad formatting churn was
  intentional.
- Small follow-up closeout slices may still be required after a broad cleanup if
  one contract fixture, audit comment, or isolated file drifts later.

## Implementation Notes
- Use one contained commit for the verification-baseline restoration so it does
  not hide product changes.
- Do not mix unrelated asset or generated-file churn into the restoration
  commit.
- Re-run the same gates that failed before claiming the baseline is restored.
- When the remaining issue is a host-facing contract suite, extend blocking CI
  coverage in the same closeout effort so the restored local gate stays
  enforceable.

## Compliance Mapping
- Standards enforcement and tooling hardening.
- Documented local verification equivalence to CI.
- Traceable non-behavioral cleanup when broad quality gates surface drift.
