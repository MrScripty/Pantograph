# Scheduler v2 production migration

Plan checkpoint: 2026-10-07. Base: remote `main` verified by `git ls-remote`
and fetched at `a8483e511dcec4f36e269e6e4debf181a318222f`.

## Evidence and scope

The Library thesis (`libfile_6c1d9aa6e1f08191b4a14e0de0846220`) and evaluation
(`libfile_8b6d205bd260819187ef9fbac029a243`) were read through Library.
The evaluation describes a standalone CPU simulator with no production adapter.
Its C3 fresh cohort reports search mean run CPU 116.79 seconds versus earliest
approximately 0.19 seconds, and one iterative budget failure among 56 runs.
This supports a bounded baseline first, not embedding reference search.
The scheduler is to be designed afresh in production Rust from the research
requirements and Pantograph's ownership boundaries. POC code is a semantic and
feasibility reference only: no copying, translation, port or runtime dependency.
The first slice is an incremental foundation, not a replacement for the full
researched multi-event completion algorithm.

Local materialization of both reports and `pantograph-scheduler-poc-v2.zip`
(`libfile_cfe8b8ef4fbc81918d2155af3100c90f`) failed through the current Library
helper, including the authorized network retry. No local bytes or hashes are
verified. The delegated C3 `policy.py` SHA256
`ccb07590767bd27b715102314d86077f4a7d93938dc35586780caabe4761a712`
and its agreement with `review/c3-source.sha256` remain supplied evidence,
not an independently checked claim by this executor. The actual archive,
54-row coverage matrix, and independent audit must be materialized and inspected
before claiming simulator differential acceptance.

No repository AGENTS.md or .agents skills were present in the verified checkout;
the workspace .agents directory is empty. Existing workers own inspector
normalization and rerank/audio adapters; those surfaces are outside this slice.

## Implemented versus missing

| Area | Verified production foundation | Missing for researched v2 |
| --- | --- | --- |
| CPU/GPU capacity | Registry RAM/VRAM claims, shared declared resource domains, host RAM capacity source, resident envelopes, advisory evaluation and commit-time revalidation | Complete device discovery and phase-level compute/link capacity; unknown budgets remain unknown; no new GPU execution claim |
| App priorities | Session run priorities, FIFO ordering, starvation bypass boosts, one-position warm-reuse window | Live app-wide boost entitlement, protected service accounting and resource-drain guarantees |
| Queues | Scheduler task states, dependency readiness, session admission, first Ready runtime task | Joint ready-frontier completion ranking and bounded owner forecasts |
| Batching | Typed batch/readiness contracts and adapter-specific batching surfaces | Text/chat/embedding host batches execute sequentially; no native or iteration-level batching capability is inferred |
| Residency | Runtime instances, acknowledged resident envelopes, selected lease custody | Multi-instance staging, overlapping loads, eviction/preload planning and acknowledged stage lifetimes |
| Timing | Load/unload/warmup attempts, attributed trace spans and historical technical-fit ranking | Comparable workload/host/artifact/instance/condition-bound serialized completion profiles, transfer and execution samples, uncertainty calibration |
| Remote nodes | Local network status is LocalOnly with no peers | Worker offers, transfer pricing, durable attempt fencing and distributed custody |

Selection seams are `pantograph-scheduler/src/dispatch_selection_policy.rs`,
the embedded runtime candidate provider's non-mutating evaluation followed by
selected provisional lease commit, and later the session runner's first-Ready
choice. Historical-fit ranking in runtime-registry is a separate policy whose
residency-first order is not the completion planner.

## Exact first slice

Add an opt-in, pure local pre-reservation selector. Rank only alternatives for
one already admitted Ready task. Reuse existing eligibility (explicit runtime
and device constraints and Fits assessment), preserve cold alternatives, and
minimize `preparation/load + required transfer + execution` in integer
microseconds for the supported serialized path. Zero is accepted only when
explicitly supplied; missing components never become zero. Warm residency has
no separate bonus. Equal totals use stable candidate ID ordering.

Evidence must identify the exact immutable request/offer snapshot plus current
host, runtime instance, immutable artifact, workload, resource condition and
residency condition. Historical timing evidence must match those current
identities; all alternatives must share a comparison population/host/workload
and measurement convention. Preserve measured versus synthetic provenance.
Require disclosed sample counts and age; reject future timestamps and overflow.
The local API must bound candidate count, evidence count and comparison text
before ranking. Missing/stale/incomparable evidence invokes existing
sole-eligible/otherwise-ambiguous behavior with an explicit reason. Over-budget
input refuses without scanning an unbounded fallback population.

Return only a reservation candidate ID. Never manufacture a lease, readiness
proof, dispatch decision or executable authority. Existing selected-only commit
must revalidate capacity, runtime generation and ownership; ordinary dispatch
validation and cancellation/output custody remain mandatory. No default
scheduling, admission/session/runtime or wire-contract changes in this slice.

## Acceptance and rollout gates

1. Synthetic hand calculation from thesis section 12.1: warm CPU 12 seconds,
   cold GPU preparation 2 plus execution 3 seconds chooses cold; add explicit
   transfer cost until warm legitimately wins. Labels must remain synthetic.
2. Missing, stale, future, insufficient, foreign-snapshot, identity-mismatched,
   overflowing and incomparable samples never pick a partially observed winner.
   Stable ties, hard constraints, rejected fits and bounded refusal are tested.
3. Ranking acquires no speculative leases. Existing registry tests establish
   selected commit refuses changed capacity/instance and preserves lease custody;
   integration tests must repeat this around the opt-in selector before wiring.
4. Compare with the archive's earliest-finish baseline for the declared serialized
   single-task subset after verifying archive hashes. Do not equate these fixtures
   with the full two-event policy or the 54 requirements.
5. Measure release-build CPU and wall cost with maximum-size candidate/evidence
   fixtures and report p50/p95/max and environment. Separate deterministic work
   limits from observed latency. For default rollout, shadow-score actual dispatch
   events with real profile coverage; freeze a dispatch latency budget (proposed
   p95 <= 1 ms and <= 1% of predicted task duration), retain overhead in outcome
   comparisons, and test matched profiles, cold/warm paths and output semantics.
6. Before enabling a default, run public workflow/session/output/cancellation
   contract tests with a qualified native runtime build. No ORT downloads:
   `ORT_SKIP_DOWNLOAD=1`. Missing native prerequisites are blockers, not passes.

## Later milestones and decision boundary

First resolve who produces comparable timing identities and components, profile
aggregation/uncertainty, resource-condition invalidation, real adapter capability
coverage and how profile diagnostics reach users. Then wire the opt-in selector
at the existing evaluate/select/commit seam and validate it in shadow mode.

The later research planner is a separate milestone: bounded disclosed cohort,
mode-diverse candidates, two actual event completions, unfinished liabilities in
a common continuation, frozen priorities/background obligations, deterministic
work budget and first-action authoritative revalidation. Forecasts, staged split
execution, native batching, preload/eviction and remote workers each require
explicit adapter capabilities and separate acceptance. Session/runtime ownership
and routing groundwork are not the completed researched scheduler.

No push, public write, main merge, runtime/model download, credentials or network
setting changes are authorized by this migration.
