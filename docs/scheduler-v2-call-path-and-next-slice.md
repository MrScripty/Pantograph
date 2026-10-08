# Call path and next bounded completion slice

The follow-up implements a narrower pure/local two-task evaluator and hardens
the native snapshot fence; see [reviewed checkpoint](scheduler-v2-two-completion-checkpoint.md)
and [results](scheduler-v2-two-completion-results.md). The four-task proposal below now has a separate
[pure frozen-cohort kernel](scheduler-v2-cohort-completion.md). Native successor
activation is still gated on explicit owner cohort, serialization and conditional
release/capacity evidence; this kernel is advisory only.

## Current admitted-task path: native opt-in connected

Base main is `a8483e511dcec4f36e269e6e4debf181a318222f`. The pure selector
landed locally in `9d613c9f05dc9b15ed6e342ef0aa9f2fdfd2b4b3`, following plan
checkpoint `170f9aae97b008bc89471a2584dff9a46ced10a9`. The subsequent native
integration is described in [integration results](scheduler-v2-native-integration.md).

1. Native startup can supply `EmbeddedCompletionTimingOptIn` using
   `EmbeddedHostedStartupCompositionInput::with_completion_timing`. The public
   startup configuration and session JSON have no timing numbers or scheduling
   toggle. The option defaults to absent.
2. `session_scheduler_runner::ready_runtime_dispatch_context` reads only the
   opt-in task's referenced persisted scalar inputs, under the same store lock
   as its Ready record. Default providers do no additional input work. The
   store uses the same conversion function as runtime-host input mapping.
3. The selection boundary refreshes owned source facts, then calls the
   backward-compatible `runtime_dispatch_candidates_with_inputs` provider
   method. Existing providers delegate to their original method.
4. The embedded provider evaluates resource alternatives without acquiring
   leases. Before selected commit, the opt-in builds bounded native queries
   from model content, exact materialized inputs, task/Ready identity, effective
   trait settings, owner runtime/device/variant, residency and current resource
   observations. A trusted bounded native owner source must qualify the actual
   physical device, implementation/configuration and workload before returning
   a record. Unknown evidence retains the ordinary selector.
5. Qualified complete observations or explicitly enabled configured estimates
   reach `select_scheduler_candidate_with_completion`. Its typed ranked,
   fallback or refusal reason is retained as a dispatch diagnostic. No warmth
   bonus is added to measured load/transfer/execution costs.
6. Only the selected ID reaches the unchanged `reserve_provisional` call with
   its expected observation. Current capacity, instance and ownership are
   revalidated before publication. Ready-to-start checks, ordinary dispatch
   validation, reservation binding, custody transfer, cancellation and output
   materialization remain required.

Controlled public-session tests exercise this path through the real embedded
provider, registry, Pumas resolver, host port and reservation lifecycle port;
backend execution and timing values are explicit fixture data. No calibrated
production history producer is installed by default. This is one-task ranking,
not the research completion-search algorithm or a queue-policy replacement.

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
matching strings or a `Measured` enum value do not do this. Only controlled fixtures have supplied evidence here; the native integration
adds a trusted owner injection contract, not production calibration.

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
