# Frozen workflow objectives in the bounded cohort kernel

This slice implements the research thesis §§1.1 and 10.1 distinction between
workflow makespan and a speed profile's weighted workflow flow time. It extends
the existing two-event search and common continuation in `pantograph-scheduler`;
it adds no competing scheduler, native dispatch path or artifact admission.
It is based on public PR 69's normal PR 70 merge `01a31721`. The separately
qualified placement checkpoint `3aaa0829` is preserved and is not its ancestry.

## Explicit mathematical contract

`evaluate_scheduler_cohort_workflow_objective` is an opt-in pure API. Its owner
freezes the cohort, objective, exact required-task partition, output acceptance
contracts, release ages and positive dimensionless integer weights. Each group
identifies one workflow/run pair; duplicate groups, duplicate/foreign/omitted
tasks, zero weights and unknown/future release states refuse. The one-to-four
task limit is unchanged. Workflow IDs/run IDs may differ only in this explicit
mode; task IDs must remain globally unique and prerequisite edges stay within
their workflow run. Exactly one task still borrows genuine admitted-first
readiness. Other tasks remain advisory forecasts without admission or leases.

The owner must explicitly declare its entire required accepted-completion
population, including output, branch and retry obligations, and must qualify
acceptance before cleanup/retention. Four captured runtime tasks alone do not
certify that contract. Labels and assertions are trusted input declarations,
not independent authentication. Current native coverage cannot supply this
closed-workflow contract or a complete timing/capacity evidence matrix.

The new versioned timing convention identifies disjoint serialized stages:
reload, setup, required transfer and execution finish at accepted output;
cleanup and retention occur afterward. For task i, output end is its full
six-stage end minus its explicit cleanup and retention durations. Reload is
before output, never deducted. Every duration must remain explicitly present;
unknown timing is not zero. Workflow completion C_w is the latest required
task output. The next serialized task still waits for the FULL six-stage end.
No capacity is subtracted, predicted to be freed or converted into a lease.

The batch profile compares `(max C_w, sum C_w, stable action IDs)`. The speed
profile compares `(sum h_w(age_w + C_w), max C_w, sum C_w, stable action IDs)`.
Times are decision-relative microseconds, ages are frozen elapsed time since
release, and h_w is a positive integer with no units. All workflow arithmetic
uses checked `u128`; serialized time/stage bounds remain checked `u64`. An
irrelevant node-completion sum cannot invalidate workflow mode. The result
reports both workflow output milestones and the complete serialized drain.

## Search, bounds and defaults

Both APIs execute the same search: optimize the already-admitted first task's
placement and the next legal completion event, then finish every remaining
obligation by earliest full resource-end completion. The continuation is not
silently changed to output-end ordering. All required alternatives are compared
under the same immutable objective and continuation. Unknown/stale/incomplete
evidence, dead ends, overflow or exhausted budgets return no partial winner.

At most four workflow groups and four total required memberships are accepted
before validation/search/cloning. Existing bounds on 158 evidence rows, two
placements, 64 optimized expansions, 512 completion events and 131072 work units
remain. Group validation, arithmetic and stable ordering consume the same work
budget. Bounded operations do not promise a wall-time limit.

The legacy evaluator's public result/score structures and refusal enum, same-workflow restriction,
six-stage makespan/task-sum ordering and work counters remain unchanged.
`PriorityThenFifoSchedulerPolicy`, starvation boosts and the one-position warm
reuse window remain the production defaults. No existing reviewed reference,
PR 68 description, runtime adapter, inspector or phase/timing producer changes.

This is a deterministic conditional-trajectory objective, not an expectation
over calibrated joint scenarios. It provides no app entitlement, boost expiry,
protected progress, global optimality, stochastic nonanticipation or hardware
speedup guarantee. Queue first-task choice is still fixed by actual admission.
Arbitrary graph splitting can change the two-event horizon and continuation;
only equivalent accepted-output grouping has invariant objective values.

## Qualification and integration dependency

Controlled fixtures exercise first-placement speed reversal, workflow grouping,
final-cleanup separation, serialized cleanup blocking, exact default counters,
membership/release/acceptance refusal, stale/unknown evidence, wide arithmetic,
exact budgets, weight scaling and release-age invariance. An independent numeric
oracle enumerates complete tiny schedules, filters by the same declared
two-event/full-end continuation rule, and computes workflow losses independently.
192 generated objective cases also permute tasks, placements, rows and groups.
Directed tests cover refusal and boundaries outside that fitting oracle family.
A separate ignored cost probe measures the actual pure Rust kernel on the
maximum four-task/two-placement 158-row population, excluding capture, evidence
production, admission and execution. Its measurements are recorded separately.

Positive providers are controlled synthetic fixtures. There is no actual native
closed-workflow producer or scheduler dispatch integration in this slice. The
existing Pumas HF-directory/weights-entry mismatch and missing authoritative
content fingerprint still block actual artifact qualification; neither boundary
is weakened. Missing six-stage/path-conditioned timing, release reconciliation,
capacity, workload/acceptance qualification and progress protection also remain
end-to-end requirements. The unavailable private instrumentation archive is not
retried or bypassed, and no duplicate instrumentation is introduced.

Research source: Library thesis `libfile_6c1d9aa6e1f08191b4a14e0de0846220`,
version 1, §§1.1, 10.1–10.3 and 11; evaluation
`libfile_8b6d205bd260819187ef9fbac029a243` describes a standalone CPU simulator
and high planning cost. No proof-of-concept implementation is copied.
Exact source, commands, raw results, dispatch cost and independent review are
preserved with this branch's handoff; this document makes no unrun CI claim.
