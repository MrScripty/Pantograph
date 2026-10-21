# Plan: preserve CPU text terminal outcomes

**Plan status:** Blocked
**Current phase:** bounded T4 qualified; trained T3 input blocked
**Next slice:** draft PR92 review/CI and trained T3 when verified inputs arrive
**Acceptance status:** F1/F2/F3 and bounded T4 satisfied; trained T3 blocked
**Execution ledger:** [execution-ledger.md](execution-ledger.md)
**Issues:** [issues.md](issues.md)
**Reports:** [qualified T4 evidence](reports/2026-10-19-public-terminal-outcomes.md)
**Related ADRs:** [index](../../adr/README.md)

## Objective and authority

The 2026-10-21 repository integration delegation continues the
[text-readiness plan](../text-generation-readiness/plan.md#current-integration-admission-2026-10-21).
It preserves this repair and its historical evidence through a normal merge with
main `9ec7e7f3`, then qualifies the composed source. The old M3 coordination and
parent-publication wording below describes the original bounded handoff; it does
not block the newly authorized integration. T3 remains blocked.
The [current integration report](../text-generation-readiness/reports/2026-10-21-current-main-integration.md)
now records passing composed-source libraries and all four public CPU scenarios
at `3de39827`, including preserved failure/cancellation and owned cleanup.

Preserve actual runtime-host failure and cancellation meaning in the public
workflow result. PR80 proved real untrained CPU generation and cleanup, but its
negative public calls became InvalidRequest/TerminalFailed despite identity-bound
runtime diagnostics. User delegation authorizes this separate T4 implementation,
review, qualification, checkpoints, push and focused PR. Parent confirmed M3
leaves this finalization owner untouched; M3 owns only Start changes in
session_scheduler_runner.rs and task_execution_worker.rs at e9d89eac. No M3
caller or fairness changes are authorized here.

Baseline is frozen PR80 head 040969de5245001e0c13d96caa46fb86b2c1df3a.
Worktree /workspace/pantograph-text-t4, branch
fix/cpu-text-terminal-outcomes-2026-10-19, owner CPU text delegate, target PR80
branch for focused review; parent coordinates eventual main integration. PR80
branch/worktree/evidence remain unchanged. Retain the protected new branch for
review and blocked trained-model follow-up; synchronize only by parent-coordinated
patches, no history rewrite/merge/deletion. Parent decides terminal retirement.

No AGENTS.md exists in project/workspace. Applicable standards already read from
MrScripty/Coding-Standards 0fc92a24: Core/Router; implementation, verification,
planning, proportionality, commit, documentation and Rust/API/async/library;
contracts, concurrency, code design and tooling. No new dependency is introduced.

## Acceptance

| ID | Observable claim | Kind | Environment | Mode | Status |
| --- | --- | --- | --- | --- | --- |
| F1 | Coherent staged runtime failures preserve code/message; exact cancellation/shutdown diagnostics produce public Cancelled; incomplete/incoherent facts fail closed | focused | not-applicable | automated | satisfied |
| F2 | Public Pumas/Resolve/Check/scheduler CPU probe returns generation, loader failure cause and typed cancellation, with completed cleanup, on exact new source | integration | representative | automated | satisfied |
| F3 | Shared finalization regressions and affected crate/static gates pass; M3 callers and frozen PR80 source/evidence unchanged | integration | representative | automated | satisfied |
| T3 | Licensed trained-model public CPU generation | system | required-real | automated | blocked |

T3 remains blocked on verified trained assets/config/tokenizer/license evidence.
No model host retry/bypass, credentials or new legal agreement. Untrained weights
prove only F2 integration; no model-quality, GPU, UI, performance or full
text-to-image acceptance. Public cancellation is early active-task cancellation,
not proof of interruption after generation begins.

## Binding decisions

The existing task-result mapping preserves ordered Error diagnostics but the
finalization completeness guard discards their cause. Move the result read
before that guard and use only coherent TerminalFailed record + Failed result
with matching workflow/run/node/task identity. The first Error diagnostic keeps
producer order. Exact runtime_host.cancellation_requested/shutdown_requested
codes yield Cancelled; all other Error diagnostics yield Internal with code and
message preserved. Do not infer cancellation from human text or invent a lost
RuntimeModelLoad phase hint. Missing/non-Failed/incoherent/no-error evidence uses
the existing incomplete-state failure. Search causal terminal records before
blocked descendants so their ordering cannot mask the real failure.

Result projection owns this change. State transitions, admission, resource
cleanup, diagnostic producer/mapping and serialized contracts retain their owners.
Serial, resumed and batch continuation already call this same helper. The new
probe requires correct public projection and cleanup, rather than observational
success alone. PR80 artifacts remain their original baseline evidence.

## M1: projection and qualification

State: Complete for bounded T4. Write set:

- crates/pantograph-workflow-service/src/workflow/runtime_branch_run_finalization.rs
  (production projection and inline regressions)
- crates/pantograph-embedded-runtime/examples/cpu_text_generation_workflow_probe.rs
  (new-source public-outcome regression)
- docs/cpu-text-generation-workflow-probe.md
- docs/plans/cpu-text-terminal-outcomes/**

Gates: independent design admission; focused preservation/identity/precedence
regressions; affected workflow and embedded suites; Clippy and formatting;
actual bounded public CPU three-scenario probe; clean-checkpoint script;
source hashes and retained exact evidence; independent final review; focused PR/CI.

Composed-design review: not-applicable. This changes an existing owner's terminal
projection using already-owned records/results; no composition root, lifetime,
state authority or caller Interface changes. No helper registry/cache/observer
framework is introduced. Caller knowledge and cleanup sequencing stay unchanged.

## Blockers and replan triggers

T3 input is unavailable after the recorded official model-host proxy HTTP403;
that frozen denial is not retried. CI may remain queued; absence of CI execution
is not success. Replan before any contract/state/mapping/caller/fairness edit,
new shared-file overlap, or evidence suggesting cleanup must move. Missing
identity/cause cannot justify guessing or accepting generic public errors as F2.
