# Session Capacity Observability

Date: 2026-10-02. Status: `Verifying`.
Authority: [CI-01 session-capacity decision](../plan.md#ci-01--session-capacity-observability-repair).
Branch: `fix/session-capacity-observability-2026-10-02`, based on fixture
PR #7 at `74723b5249ad35404506c6c01ab6c087c8db3b58`. Retain until integrated,
rejected or superseded. The workflow-service owner implements; the domain
integrator independently reviews the admitted decision and final patch.
Coding standards inspected at `MrScripty/Coding-Standards@dcc56f26e884ade260770beceba2501d3746200d`.

## Source-Grounded Decision

The five old capacity tests invoked a non-runtime graph and expected session
runtime eviction. Current non-runtime execution deliberately performs no runtime
load. Session-count eviction is owned by explicit keep-alive creation/enablement:
`ensure_keep_alive_session_runtime_ready` calls the existing capacity loader,
which consults the host victim selector and default affinity policy. Runtime-task
dependency admission and resource reservations have separate owners.

The loader's optional diagnostic context previously required a workflow run,
but its sole caller passed no context. Moving tests to keep-alive without repairing
observability would lose their ordered eviction, timing and failure-diagnostic
assertions. The integrator accepted the session-scoped design before implementation;
the plan was updated first. No obsolete non-runtime loading path is restored.

## Resulting Contract

Existing capacity selection, host unload/load calls and affinity remain unchanged.
Keep-alive admission supplies the genuine execution-session summary. Model
lifecycle payloads now optionally identify the target and unloaded session as a
validated pair. Real snapshot run attribution remains optional; session-only
operations have no run ID or semantic run version. Old payloads omit new fields
and retain their former round trip. Other run-only event validation remains strict.

The registered `session_runtime_admission` error phase uses `session_runtime`
scope and diagnostics-only projection semantics. Error payloads contain the real
execution-session ID. Original operational errors retain their code/message;
recording failures become diagnostic-unavailability detail instead of replacing
the unload error. Existing caller rollback removes a failed newly created session
or reverses failed keep-alive enablement. A failed victim unload leaves its loaded
state intact. Session-only ledger records do not invent run projections.

The five tests now exercise explicit keep-alive enablement and disablement,
preserving victim choice, workflow/model/backend affinity, target cleanup,
unload error and ordered timing/event assertions. Additional lifecycle tests
cover recorded capacity rejection and rollback. Ledger regressions cover session
identity, strict legacy validation and absent fabricated run projections.

## Evidence And Remaining Qualification

- Final fresh Cargo compilation and full library runs: workflow-service 874/874
  and diagnostics-ledger 103/103 passed, zero ignored. This includes all former
  capacity assertions, the non-runtime no-load regression, runtime-dispatch tests,
  new creation/enablement rollback cases and three ledger scope regressions.
- The diagnostic registry check includes its admitted fourteenth phase and asserts
  session scope, error severity and diagnostics-only projection behavior.
- Cargo format, staged whitespace and staged traceability passed. Independent
  source review and exact-head hosted qualification remain pending.
- Baseline integration-contract, Rustler, frontend lint, audit and Clippy failures
  remain separate. Hosted exact-head CI and applicable review are required;
  these focused results do not establish whole-repository merge qualification.

## Independent Review Repair — 2026-10-03

Review of frozen tree `363367538bf09d950e4661903253cc6676459d18` requested two
changes. Successful host unload could be followed by a failed terminal append
before residency changed, leaving a falsely loaded victim. Failed-unload lifecycle
append errors were discarded, including validation failures caused by multiline
host errors, even when separate canonical error recording succeeded.

The repair commits a successful host residency transition immediately, before
fallible timing/terminal reporting. The existing failed-admission response policy
is retained: target creation/enablement rolls back after a terminal telemetry
failure, while the genuinely unloaded victim stays unloaded and can be reloaded.
Lifecycle text is sanitized; missing lifecycle-event detail survives canonical
error recording and does not replace the original failed-unload code/message.

Four fault regressions exercise SQLite triggers rejecting only terminal lifecycle
writes, with canonical error recording still healthy: creation and enablement
both verify victim residency and actual reload recovery; a healthy-ledger multiline
host error verifies a complete ordered lifecycle; a selectively rejected failed
terminal event verifies explicit unavailable detail alongside a canonical error ID.
The earlier frozen tree remains available for comparison. Narrow re-review is
required before publication; verification for this repair is recorded below.

Repair verification: fresh Cargo compilation and 15/15 capacity tests passed.
The freshly built repair binary then passed all 878 workflow-service library
tests, including non-runtime no-load and runtime-dispatch regressions. Ledger
source is unchanged from the previously passing 103/103 run. Formatting,
whitespace and staged traceability passed. The three earlier integration-contract
failures remain unchanged; no new integrated or hosted pass is claimed.

Independent narrow re-review accepted tree
`260422c5c67332280527781ed13c312683b0143d`: both P1/P2 repairs and their four
fault regressions were inspected; all 15 capacity tests were independently rerun
from the repair binary and passed. No remaining narrow source finding was raised.
This report-only addition records acceptance after that frozen review. Hosted
exact-head qualification, unrelated integration failures and whole CI remain open.
