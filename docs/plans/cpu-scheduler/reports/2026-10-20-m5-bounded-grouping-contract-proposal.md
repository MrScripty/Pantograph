# M5 bounded grouping contract proposal

Status: review candidate. This proposal prepares the next shared-contract review;
it does not admit M5 implementation or change milestone acceptance.

The first M5 slice should define a bounded test-only grouping prototype and independent
oracles before proposing a production advisory API or connecting it to execution. Runtime integration must settle how
each member retains its existing M3 service opportunity. Dispatching an entire
sequential group under one current M3 grant would change the grant's meaning.

## Authority and current baseline

The source baseline is PR83 head
`bd66da4e0ba20d17f30a70140a448b55d57fce1e`, tree
`555a1cfaafe6a2328a34e4cc8ba634b07b4a655c`. Accepted M3 remains
`b63aa3ebe5fab1531aef952a35ce74932bf76b30`; PR84's original repair history remains
at `9e068bbc2a040f20de88dfc70d914339cd5a8910`.

The 2026-10-20 continuation asks for useful independent work while the browser
qualification is blocked. This proposal is a separate branch and one report;
PR83, PR84, the shared plan/ledger and production source remain unchanged.
The plan requires independent review of the next slice's contracts and write set
before its first code change ([plan](../plan.md#milestones-and-runnable-checkpoints)).

Publication completed and all four hosted workflows are terminal. CPU Composition,
Runtime Separation and Headless Workflow passed. Quality Gates failed at browser
image qualification; its Rust focused tests, explicit inventory/native residency
step and native text controls passed. These results do not independently accept
overall M4 or resolve PR83/PR84 integration. The local browser attempt failed at
startup because the Chromium sandbox helper is owned by `nobody`, not root;
this is separate from the hosted failure's still-unavailable root cause.

## Existing contracts that constrain M5

| Source | Consequence |
| --- | --- |
| `crates/pantograph-scheduler/src/batching.rs::SchedulerBatchCandidate` | Candidate identity includes workflow/run/task, intent, model, runtime, devices and input-shape signature. Preserve correlation validation and exact compatibility checks. |
| `SchedulerBatchPolicyDecision::validate` | The general serialized contract supports a configurable positive maximum; it does not impose M5's fixed eight-member policy limit. Do not silently narrow or version that general API in this slice. |
| `crates/pantograph-workflow-service/src/scheduler/service_opportunities.rs` | One granted member holds the turn through execution/drain/cleanup. Limits are 64 pending members and 32 units; protection debt saturates at two. Preserve this owner and its documented opportunity guarantee. |
| `crates/pantograph-embedded-runtime/src/runtime_host_execution_port.rs::execute_runtime_host_sequential_batch_request` | Existing supported text/chat/embedding/rerank/audio envelopes execute members sequentially. That capability does not supply an M3 fairness re-entry boundary between members. |

The second row describes the same batching module. No backend-native parallel or
continuous-batching capability follows from these contracts.

## Proposed C1 policy boundary

C1 is a test-only contract prototype with no Start, lease, reservation, queue mutation,
retention change, execution port call or public serialization. Its first consumer
and any public advisory API require a later reviewed delivery slice. This avoids
adding unused production-private functions or suppressing warning-deny checks.

1. Accept a borrowed, ordered window of one through eight bounded comparison
   fact views, constructed within the test-only prototype. Do not accept an
   arbitrary `SchedulerBatchCandidate` or claim full intent validation. The
   workflow remains the owner of Ready proofs, immutable inputs,
   authenticated entitlement, selected CPU capability and live custody. The
   planner cannot establish any of those facts from an identifier or estimate.
2. Treat the first candidate as the ordinary selected anchor. Admit grouping only
   when every member in the window has the same task family, exact selected model
   reference, runtime, ordered device set and input-shape signature. Initial
   execution qualification is restricted to the existing CPU embedding envelope.
3. Preserve input order. A mixed or incompatible window produces an explicit
   singleton proposal for the anchor; it never scans beyond the window or skips
   an incompatible member to construct a more favorable group.
4. Propose immediately from currently available work. One member is a normal
   singleton outcome. C1 introduces no fill timer, waiting state, background
   collector or extra readiness/preparation attempt.
5. Return bounded indices and a typed disposition: singleton, compatible group,
   invalid window or arithmetic refusal. Memory aggregation uses checked addition
   and reflects exactly the proposed members. Overflow creates no dispatchable
   group. C1 estimates do not authorize physical capacity or resident reuse.
6. After bounded-view construction, inspect at most eight members and seven
   compatibility comparisons against the anchor. Borrow inputs without cloning
   payloads. Diagnostics contain scalar counts,
   indices and a reason, without a new history store or raw input/proof text.

An empty or oversized direct policy input is invalid. A future workflow caller
may form the first eight-member window from its already bounded Ready capture;
the ninth member stays in the authoritative queue. That caller contract and
capture-before-clone proof belong to C2, not to this pure selector.

### Bounded comparison views and refusal order

The prototype's private member view borrows existing typed workflow-run/task,
family, runtime and device identities, a complete selected `PumasModelRef`, the
shape string and one memory increment. It contains no task-intent vectors, raw
inputs, proof payloads or execution capabilities. Its constructor checks lengths
before scanning, trimming, equality, hashing or copying:

- One through eight members and one through eight device IDs per member.
- Raw model ID/revision/artifact-ID fields at most 512 bytes each; raw selected
  artifact path and shape at most 1024 and 128 bytes respectively.
- At most eight model-reference migration diagnostics; raw diagnostic code,
  message and optional input at most 128, 1024 and 1024 bytes respectively.
- Existing opaque scheduler/dependency identifiers retain their own constructor
  validation; the view does not manufacture or normalize them.

These are local prototype eligibility limits. They do not narrow the generic
serialized DTO, and exceeding one refuses the prototype without traversing its
contents. Check a vector's count before iterating it and each raw string's byte
length before character validation. This bounds the actual complete model-ref
equality, including migration metadata; do not compare only a shortened model ID.
The prototype may use the existing model-ref and shape validators only after
these preflights. It does not call the unbounded full task-intent validator.

Scalar validity means valid existing opaque IDs, the existing model-ref and
input-shape rules after raw-length preflight, and an incremental memory value
greater than zero. `u64::MAX` is a valid individual increment; only checked
aggregation determines whether a compatible group overflows. Migration diagnostic
strings remain opaque bounded metadata for exact equality; C1 does not add a
semantic content rule to that generic metadata.

Refusal order is: window size, every view's nested bounds and scalar validity,
duplicate `(workflow_run_id, task_id)` identities, compatibility, then aggregation
over the selected members. Invalid or duplicate input cannot be hidden by an
earlier incompatibility. An incompatible but otherwise valid window selects only
the anchor, so unselected-member aggregate overflow cannot reject that singleton.
Compatible groups aggregate every member with checked addition.

## Independent C1 oracle matrix

Expected outcomes must be written from the contract before implementing selection.
Tests must check unchanged input order and absence of policy side effects.

| Input | Required outcome |
| --- | --- |
| One compatible member | Immediate singleton, index 0. |
| Two or eight compatible members | Group of exactly 2 or 8 in input order. |
| Zero or nine direct policy inputs | Explicit invalid-window result; no partial proposal. |
| Any mismatch in family, model, runtime, device ordering or shape | Explicit anchor singleton; no skipped-member group. |
| Equal keys with different workflow/run/task identities | Preserve each member's distinct identity; the prototype explicitly rejects duplicate run/task identities. |
| Zero memory increment, including in an incompatible window | Invalid scalar input takes precedence; no singleton or group proposal. |
| Two memory increments summing above `u64::MAX` | Arithmetic refusal; no truncated or saturated aggregate. |
| Oversized device vector, migration diagnostic vector or raw string | Bound refusal before iteration, comparison, validation scan or copying. |
| Duplicate member combined with an earlier incompatibility | Invalid duplicate input takes precedence; no singleton proposal. |
| Valid incompatible members whose combined increments overflow | Anchor singleton with exactly its own memory total; unselected increments are not aggregated. |

The prototype's hand-authored fact views do not validate a real candidate's full
correlated intent or override constraints. The production view constructor must
be separately reviewed before accepting arbitrary DTOs: existing
`SchedulerBatchCandidate::validate` is private, while
`ValidatedSchedulerBatchPolicyDecision` also rejects incompatible windows and
therefore cannot represent this singleton-fallback input boundary.

The future C2 oracle set additionally requires: nine queued members leave the
ninth untouched; stale Ready/owner/entitlement snapshots refuse before Start;
pre-Start cancellation does not create a Running attempt; running cancellation
retains execution charges until matched cleanup; and a protected competing unit
receives its existing M3 opportunity before another member can start. C1 tests
cannot establish these integration properties.

## Required decision before runtime integration

The current service turn is member-scoped. Keep one current M3 grant per actual
member Start and release it only after that member's existing scope and matched
cleanup end. A pure grouping proposal is never an extra grant. New arrivals and
protected units must be considered at the next existing fairness boundary.

The current host sequential batch method does not return to that workflow
boundary between members. Therefore C2 must either use the ordinary member path
with grouping limited to safe planning reuse, or independently review a concrete
member-boundary integration that preserves the single fairness owner. Whether
the first option satisfies the plan's supported-envelope M5 objective is an
integrator decision; this proposal does not silently substitute it for that goal.
Holding one old grant over eight members is not an admitted shortcut.

C2 review must identify actual broker/orchestrator/host files, ownership across
await points, final Start revalidation, cancellation disposition and output/attempt
joins. It must account for any changed residency/retention behavior against M4's
acceptance limits. No new batching wait, fairness owner or cleanup authority is
implied by C1.

## Proposed write set and checkpoints

The C1 code proposal is limited to a clearly labelled `#[cfg(test)]` prototype and
hand-specified oracles in `crates/pantograph-scheduler/src/batching.rs`. It is
contract preparation, not a shipped grouping feature or production validation
boundary. A later public nonserialized advisory API needs an explicit export and
write-set review covering `lib.rs`; an unused private production implementation
and warning suppression are excluded. Preserve serialized contract version 1 and all
existing constructors/default policies. No workflow, native host, registry,
dependency, UI, IPC, persistence or CI edit is included.

Checkpoint 1 is this source-backed contract and independent review. Checkpoint 2,
after the integrator reconciles the next slice and accepts the exact contract,
is the test-only prototype plus hand-specified oracle cases and existing
scheduler regressions. Checkpoint 3 is a separately reviewed C2 integration and
bounded actual CPU grouped-versus-singleton cohort. Only the third checkpoint
can qualify execution, overhead/reuse improvement or an M5 completion claim.

## Independent review disposition

The independent source review requested changes to the initial proposal: an
eight-member count did not bound device/model-metadata equality work; no public
validated-candidate wrapper supported the proposed input; and an unconsumed
private production selector had no warning-clean delivery boundary. The bounded
view, refusal precedence and test-only C1 delivery above address those findings
without changing the generic batching DTO. Production advisory API, full-candidate
construction and runtime member-boundary contracts remain later explicit reviews.
The revised proposal passed independent review as a bounded test-only
contract/oracle preparation checkpoint; it is not a shipped grouping feature.

## Separate remaining milestones

M6 remains the later explicit host/application choice with truthful diagnostics,
consumer contract review and bounded rollback. It depends on accepted upstream
milestones and does not authorize default activation.

Real-model product qualification remains the desktop-authored dependent
`text -> actual text inference -> generated prompt -> actual image inference`
workflow with shared run identity, distinct attempts, selected real model facts
and a retrieved decodable image ([product contract](../../domain-architecture-and-multimodal/plan.md#product-contract-and-scope)).
Untrained CPU fixtures and controlled browser presentation have narrower scope.
No model download, paid inference, GPU/remote qualification or merge is part of
this proposal.
