# Source-reviewed integration candidate and next local slice

Remote main was reverified as `a8483e511dcec4f36e269e6e4debf181a318222f`.
The native one-decision candidate is local `42f31c65e5df32f018d1be2c45260eda761b8e6a`.
The follow-up branch is `scheduler/bounded-two-completion`; nothing is pushed.

Independent source review found a Ready snapshot gap: refresh awaits after the
task/Ready/input snapshot, while start constructs its transition from the current
record. Fence exact task, record/version, persisted readiness proof and opt-in
materialized inputs under the same store lock as start. On mismatch, reject the
prepared selection; its provisional custody rolls back. Do not hold the store
lock around external provider callbacks. A provisional claim may briefly exist
before rejection, but stale work must never start or transfer custody.

The native path's cost remains material: controlled debug full-path p95 was
0.570 ms for two offers, 8.135 ms for 64 small-input offers and 12.183 ms for 64
60-KiB-input offers. These exclude the proposed additional snapshot checks and
are measurements, not a dispatch deadline guarantee. No calibrated production
timing producer or default activation exists. Keep the public startup/session
configuration and PriorityThenFifoSchedulerPolicy unchanged.

## Exact implementable next slice

Implement a fresh Rust pure evaluator for a **declared two-task serialized
prefix**, keeping the admitted first task fixed. This is an explicitly scoped
objective, not global workflow completion or a closed global queue. Freeze both
task identities, complete successor offer universe, evidence and bounds.

For each legal first placement `a`, use a separately qualified post-completion
successor snapshot. Preserve every successor's hard identity and constraints;
only its resource-fit assessment may change. Every placement must have explicit
conditional `Fits`, `WaitingForResources` or `ImpossibleFit` evidence; Unknown/missing evidence invalidates the
comparison. First-completion release, runtime reconciliation, retention and any
reload belong to the transition/evidence contract. Never infer future capacity
by subtracting the first lease, or warmth from equal model identity.

For each feasible continuation `b`, let `x(a)` and `y(b | a)` be complete
serialized preparation + transfer + execution/cleanup costs. Minimize the
lexicographic tuple `(x + y, 2*x + y, first ID, successor ID)`. Thus terminal
completion is primary and the represented completion sum is the tie-breaker.
Checked integer arithmetic is required for both. Return only an advisory first
action and modeled path/coverage; no leases or owner mutations occur.

Hard limits: four offers per snapshot, four first branches, sixteen plan
evaluations and thirty-two completion-event evaluations. All rows and branches
must be checked before selecting a winner. Deterministic smaller test budgets
invalidate incomplete comparisons; never select from a partially searched
population. Incomplete evidence retains the existing one-decision result.
Oversized inputs refuse before either selector scans them.

Acceptance: hand-calculated cold/reuse reversal, independent exhaustive tiny
oracle, terminal-versus-completion-sum objective distinction, conditional
capacity gained/lost, unknown capacity, missing branches/stages, stale/foreign
snapshots, changed successor universe, integer overflow, stable permutation
ties, fixed-observation counterfactual equality and exact budget exhaustion.
Measure actual evaluator CPU/wall/RSS separately from modeled completion.

## Architectural stop before native activation

The existing timing source describes one current workload. It cannot certify an
actual successor cohort, conditional capacity, release/reconcile/retention or
serialized execution across the branch/batch paths. Therefore this increment
remains pure/local with synthetic qualified projections. Native lookahead wiring
requires those explicit owner contracts and first-action/cohort version fences.
This boundary is unresolved; neither missing archive bytes nor POC translation
is required to implement the independent Rust evaluator.

Cold-Candle candidate `6338b71a55371317c5ace6adbeacdec469c96950` remains separate.
It overlaps workflow_service_composition.rs; no cherry-pick or merge is implied.
Inspector normalization and rerank/audio adapter working paths are untouched.
