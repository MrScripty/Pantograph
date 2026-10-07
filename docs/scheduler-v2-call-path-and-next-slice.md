# Call path and next bounded completion slice

## Current admitted-task path: the new selector is not connected

References below are unchanged production paths at base
`a8483e511dcec4f36e269e6e4debf181a318222f`; the additive selector was committed
as `9d613c9f05dc9b15ed6e342ef0aa9f2fdfd2b4b3`, tree
`370fe80def1267bfbcc82eb24ae47d50a1cbb498`. Its preceding plan checkpoint is
`170f9aae97b008bc89471a2584dff9a46ced10a9`, tree
`2041a07b0cce9c4ebb766104ec5c13775fc93e1e`.

1. `session_scheduler_runner.rs:101` prepares runtime dispatch, admits dependency
   readiness, then calls `runtime_dispatch_progress` at line 114. The latter at
   line 759 preserves first Ready runtime task selection.
2. The branch attempt at lines 132–155, and the ready-task loop at lines 808–845,
   obtain the task's readiness proof and call
   `WorkflowRuntimeDispatchSelectionBoundary::prepare_ready_runtime_task_dispatch`.
3. `workflow/runtime_dispatch_selection.rs:309` refreshes sources (line 317) and
   invokes the configured candidate provider (line 322). Embedded production
   `runtime_dispatch_candidate_provider.rs:147` enters
   `resource_backed_candidate_set` at line 158.
4. The embedded provider evaluates alternatives at lines 322–327 without leases,
   constructs unreserved offers and a validated request at lines 341–371, then
   calls **existing** `select_scheduler_candidate_for_reservation` at line 378.
   It does not call the new completion selector or construct its evidence.
5. The selected ID alone reaches `reserve_provisional` at line 404. The resource
   source's `evaluate` at `runtime_dispatch_resource_facts.rs:70` uses registry
   `evaluate_reservation`; `reserve_provisional` at line 19 uses
   `acquire_reservation_provisional` at line 37 with the expected observation.
   Current capacity/instance/ownership are revalidated before publication.
6. The runner starts the Ready task and calls
   `select_prepared_started_runtime_task_dispatch` (branch lines 158–182; loop
   lines 845–870). That boundary at `runtime_dispatch_selection.rs:328` invokes
   orchestrator selection, whose `task_orchestrator.rs:645` still calls ordinary
   `select_scheduler_dispatch`. Binding the started attempt's selected reservation
   and transferring custody remain separate required operations.

The new `completion_ranking.rs:151` function has no production call site.
Only `tests/completion_ranking.rs` and `examples/completion_policy_cost.rs` call
it. The intended eventual opt-in insertion is immediately before the provider's
current line 378 selector, after resource evaluation and before selected commit.
This requires a profile producer and a way to preserve its typed diagnostic;
neither exists in this slice. Returning a candidate ID does not bypass steps 5–6.

## Callable evidence contract and its limits

The function borrows a validated request, evidence rows, and a policy specifying
`now_ms`, maximum sample age, minimum successful sample count and synthetic
opt-in. Every eligible offer needs exactly one complete row. Each row borrows
that exact request and candidate; its sample must point at that candidate too.
Snapshot clones and old/foreign offers fail pointer association.

Each row has current and sample context: host, runtime instance, immutable
artifact fingerprint, workload fingerprint, resource condition, residency
condition and timing convention. Fields must be nonempty, control-free and
at most 128 bytes. Contexts must match; the cohort must share host/workload/
resource condition/convention and source class. Measured/synthetic provenance,
sample count and timestamp are supplied by the caller; age/future timestamps,
counts, missing stage values and checked integer overflow are validated.

The sum is preparation/load/owner-delay + required transfer + execution in
microseconds. It is a scalar prediction, not an authoritative duration or a
quantile guarantee. A trusted producer must establish actual sample provenance,
workload conditioning, artifact identity and consistent aggregation; arbitrary
matching strings or a `Measured` enum value do not do this. Only synthetic
fixtures have supplied evidence here.

Invalid, stale, incomplete or incomparable evidence retains the exact legacy
sole-eligible/otherwise-ambiguous selection and returns a separate typed fallback
reason. Invalid evidence cannot discard a feasible alternative. Oversized input
refuses entirely, before any unbounded fallback. No runtime calls or leases occur
inside the new selector. The cost example's 0.355642 ms value is **p95 of a
synthetic maximum-field, final-row-invalid fallback fixture**, not a bound on
production scheduling. It excludes validation, profile lookup, resource
evaluation/commit and runtime work; observed scheduling outliers were much larger.

## Exact Library outcome

Preparation succeeded for all three requested Library identities and returned
transfer metadata. Every attempted local transfer exited 1 with exactly:

```text
library file transfer failed: download failed
```

No HTTP status or automatic-approval rejection reason was returned, so the cause
cannot be narrowed to authorization denial versus reachability from that message.
Retries used the same documented helper/authorized transfer route. No further
retry or alternative storage route is planned. `/workspace/research-inputs` is
empty: no readable report or archive bytes and no byte/C3 hashes were verified.

Library **text** reads succeeded: the complete 197-line evaluation and relevant
thesis windows (lines 1–100, 300–429 and 490–589), plus targeted text matches.
The archive itself was not inspected. Parent-provided research statements can
ground the next design without transferring or translating POC implementation.

## Proposed next concrete slice: native bounded two-event shadow evaluation

Add a pure scheduler-owned cohort snapshot and two-event evaluator, leaving
dispatch/defaults unchanged. Use production task/candidate identities and
capability facts; define new Rust projected-state/event types instead of copying
POC data structures. Freeze one owner-certified closed cohort, current objective,
priority/protection inputs and conditional evidence at entry. Start with at most
4 represented tasks and 2 supported serialized plans per task; decline incomplete
cohort obligations or unsupported modes explicitly. Preserve the already admitted
first Ready task as the executable target for this increment.

Explore legal first runtime choices, advance to a predicted completion event,
then explore the next legal choice after a second completion event. Evaluate every
unfinished obligation under the same fixed earliest-completion continuation.
Score declared cohort terminal completion, retaining setup/transfer/owner-delay
and transition-induced reloads. Hypothetical readiness/releases belong only to
projected state; actual owner readiness and acknowledged resources stay untouched.
Evidence must cover changed residency/condition after each projected transition;
unavailable costs mark a score incomplete rather than becoming zero.

Proposed local test bounds are at most 64 branch expansions and 512 projected
event advances, independent of elapsed wall time; exceeding either rejects the
incomplete comparison and records why. Freeze these bounds before experiments.
Do not choose a winner from a partially evaluated population. The result should
contain the first action, cohort/objective identity, completion breakdown,
coverage/work counts and fallback reason. Execution would still require current
first-action resource/instance/readiness revalidation through the existing seam.

Acceptance fixtures should include a locally faster first action that worsens
terminal cohort completion because of later reloads; a beneficial cold choice;
unfinished third/fourth obligations beyond the two explored events; stale
transition-conditioned evidence; fixed-observation counterfactual equality;
deterministic ordering; hard work-budget exhaustion; and zero registry mutation.
Hand-computed tiny cases and independent bounded enumeration should validate the
declared action space. Measure CPU/wall/RSS separately from modeled completion.

This is a proposed serialized subset of the researched multi-event planner,
not the final scheduler or a default change. A trustworthy production profile
producer, owner-certified cohort boundary, objective/priority service contract,
condition invalidation and diagnostic transport remain decisions to resolve
before production wiring. Rich stages, overlap, preload/eviction, native batching
and remote offers require further explicit adapter contracts and separate gates.
