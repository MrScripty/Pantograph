# Preparation-refusal workflow finalization

**Plan status:** Verifying
**Operation:** start, authorized by parent delegation 2026-10-20
**Current phase:** exact-checkpoint regression capture and handoff
**Next slice:** retain clean-checkpoint evidence and focused public repair proposal
**Acceptance status:** L1/L2/L3 locally satisfied; actual M4 frozen
**Ledger:** [execution-ledger.md](execution-ledger.md)
**Issues:** [issues.md](issues.md)
**Reports:** pending qualification
**Related ADRs:** [index](../../adr/README.md)

The M4 owner reported safe pressure refusal with no native work/claims, but no
Failed workflow terminal and session close returning scheduler_busy. This slice
repairs that lifecycle gap using already public source. The parent subsequently approved and published PR83 at exact
5b4fc6dda17342e6b27f90ebb852ffd2b7ea47eb. Only then was that public ref fetched;
no blocked payload was transferred through an alternate route. No actual M4 cohort, performance or native claim is authorized.

Base is public M4 contract tip 2da63ae9908572905a98c0c3b5a14e6952e5d3e3 atop
M3 b63aa3ebe5fab1531aef952a35ce74932bf76b30. Fresh main is still 4aec0d5c.
Worktree /workspace/pantograph-preparation-lifecycle and branch
fix/workflow-preparation-terminal-lifecycle-2026-10-20 isolate this repair.
CPU text/terminal delegate owns it; parent owns integration with public PR83 M4
and PR82. Target is public M4 branch; no merge, rewrite or deletion. Retain the
protected branch for cross-environment patch handoff/review until parent decides
integration/retirement. Frozen PR80/PR82, README and unrelated work are preserved.

## Binding decisions

Only an existing registered worker owner with no dispatch assignment/batch
custody may close a refused pre-Start run. Settle its event first; any settlement
failure prevents the safe finalization claim. Skip deferred responses. Preserve
Failed/Cancelled response meaning and original message; typed cancellation gets
a Cancelled workflow terminal. Duplicate responder rejection never owns run
finalization. Already finalized/nonadmitted runs must not create another terminal;
a different current run/workflow is protected.

Under one session-store lock, validate matching active graph/records, no Running
state, no current attempt (including bound reservation) and no pending completion
cleanup gate, then finish the run. Reuse the existing terminal-event projection;
no new lifecycle state, schema, retry policy or native cleanup is introduced.
A completed earlier task is eligible only after its attempt and cleanup settle.
Post-Start/assigned/unresolved-cleanup failures retain ownership and require their
existing cleanup/recovery owner. Do not infer safe cleanup from an error string.

This covers the worker's registered pre-Start terminal completion family:
serial admission/capture/claim, materialization/readiness preparation, event
claim/dispatching and candidate-preselection refusal. Supervision exits and API
pre-enqueue admitted-run failures are separately tracked gaps outside this slice.

Exact write set:

- crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs
- crates/pantograph-workflow-service/src/workflow/workflow_run_finalization.rs
- crates/pantograph-workflow-service/src/workflow/preparation_terminal_lifecycle_tests.rs
- docs/plans/workflow-preparation-terminal-lifecycle/**

No scheduler fairness, M4 native implementation, runtime_branch_run_finalization,
session_execution_api or frozen CPU text source/evidence edits. Cross-thread tool
is unavailable; this declared public-source boundary and final patch handoff go
through parent, without assuming shared paths or uploading the blocked payload.

## Acceptance and gates

| ID | Claim | Kind | Environment | Mode | Status |
| --- | --- | --- | --- | --- | --- |
| L1 | Controlled preparation refusal produces one matching Failed workflow terminal, released active run, successful close and zero native/Start/reservation-port calls | integration | simulated | automated | satisfied |
| L2 | Deferred, duplicate/stale/foreign owner and already-finalized cases preserve ownership/terminal cardinality; post-Start or pending cleanup cannot use this finalizer | focused | not-applicable | automated | satisfied |
| L3 | Existing cancellation/shutdown custody and affected service suites, Clippy, format/traceability pass; independent review admits source/evidence | integration | representative | automated | satisfied |
| M4 | Actual V2 cohort after repair and prospective protocol coordination | system | required-real | automated | blocked |

Composed-design review: not-applicable. This repairs an existing owner boundary
using existing store/terminal projection and responder ownership; no composition,
new observer/state authority or native cleanup mechanism. Atomic check+finish
is required to avoid Start-versus-finish races. Independent source/design review
admitted that boundary and required assignment/cleanup gates.

Replan before changing Start/native/resource cleanup, broadening to assigned
failures, accessing blocked data, changing the M4 native implementation, or running an actual
cohort. M4 remains frozen until parent/owner coordinate repair and prospective
protocol; public controlled regression is not V2 requalification.
