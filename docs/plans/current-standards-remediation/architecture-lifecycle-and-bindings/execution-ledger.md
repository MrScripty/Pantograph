# Execution Ledger

Current authority: [plan.md](plan.md)

Issue log: [issues.md](issues.md)

## 2026-09-03 — Plan creation

- Phase: planning only; no remediation code changed and no acceptance claim was run.
- Source: [architecture, lifecycle, and bindings audit](../../../audits/2026-09-03-current-standards/02-architecture-lifecycle-and-bindings.md).
- Decision: use a serial, deletion-led cutover to one scheduler/runtime-host authority.
- Assumption requiring confirmation before M2/M4 deletion: unpublished Rustler and legacy UniFFI surfaces have no supported external consumers.
- Next slice: M1 in [plan.md](plan.md).
- Deviations/blockers: none recorded.

## Entry format for execution

Record only execution facts here: date/revision, milestone and operation, environment, exact command or observation, result/evidence location, decision/deviation, and next slice. Keep acceptance criteria and design authority in `plan.md`.
