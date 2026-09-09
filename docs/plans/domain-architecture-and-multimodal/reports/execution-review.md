# Execution-path review

Date: 2026-09-08. Source baseline: `2ba2efb1cd1a06b657f7227bf74caae99f275dfc` plus existing workspace changes. Product source was read-only. No builds, tests, model loads, or GPU runs were performed. This is sufficient investigation for the first execution slice, not full DA-01 coverage or DA-03 acceptance.

## Authority and coverage

Read Core and Router from the plan's Coding-Standards checkout; applied canonical Architecture, Contracts, Concurrency, Verification, Implementation and Development Proportionality guidance to ownership, proof lifetime, lifecycle and slice admission. Requires closure is Core/Verification for Contracts and Concurrency; Architecture also requires Contracts; Proportionality also requires Implementation. No new public contract or architecture is authorized by this report. ADR-006, ADR-011, ADR-012, ADR-013 and ADR-015 govern the inspected path: canonical node semantics, scheduler-only entry, backend run identity, immutable admission facts and authoritative node type.

Inspected population: workflow-service graph effective definitions and submission validation; executable validation snapshot/projection; task graph and binding resolution; session admission; scheduler task orchestration and correlation; runtime dispatch assignments/broker; runtime branch worker and batch execution; host input/result mapping; the corresponding focused tests; embedded runtime-host dispatch sufficiently to establish its image-only limitation. Worker trust, real load targets, device policy, all cancellation interleavings, persistence reopen and desktop rendering remain with the other lanes or later bounded slices.

## Actual ownership and data path

1. Graph authoring uses `graph/effective_definition.rs:57`: `llm-inference` effective ports come from the validated authored inference snapshot (`:144`); an arbitrary `data.definition` is rejected for inference (`:107`). `graph/contract_validation.rs:33` validates resolved endpoint contracts and graph payloads. This is distinct from current model/dependency readiness, not evidence that duplicate owners should be collapsed blindly.
2. `workflow/session_execution_api.rs:244` generates one run id, creates the run snapshot, then builds scheduler tasks before enqueue. Its `scheduler_task_graph_for_session_run` (`:758`) requires a saved executable validation snapshot for runtime inference and projects its descriptor-backed inference facts. Missing model/interface/dependency proof cannot be fixed by bypassing this gate.
3. `workflow/task_graph.rs:130` converts executable topology into one task per node, with dependency task ids and source/target port bindings. Runtime selection constraints remain separate from values. `workflow/task_binding_resolution.rs:64` waits for completed upstream outputs; it does not insert prompt text into the scheduler intent.
4. Actual values are inserted at the host request boundary. `workflow/runtime_host_task_input_mapping.rs:45` finds a completed source result by **run id and source task id**, then source port, validates it and transfers its string/media value to the target port. `workflow/runtime_branch_batch_execution.rs:838` invokes this mapping using the active run's materialized results. Thus generated text can already flow along an edge; a separate prompt-copy implementation is not justified.
5. The task orchestrator owns attempts, dispatch lifecycle and result mutations. `scheduler/task_orchestrator.rs:2291` checks returned workflow/run/node/task identity against the started member. `workflow/runtime_host_task_result_mapping.rs:19` preserves outputs and terminal attempt metadata. `runtime_branch_batch_execution.rs:385` dispatches through the orchestrator, applies response mutations and finalizes member runs. Existing tests cover distinct run outputs and terminal diagnostics (`:2093` and `:1814`). These are useful supporting evidence, not a real mixed-model result.

## EX-01 — Solo runtime work cannot dispatch (high, confirmed)

Owner: workflow-service runtime dispatch broker/worker. Consequence: a single image task or the first text task in a dependent text→image run cannot reach its runtime host without another compatible ready assignment. Downstream image work cannot supply that peer because it is awaiting text.

Evidence:

- `workflow/task_execution_worker.rs:50` fixes the minimum to **2**. Every prepared runtime assignment enters the broker (`:1365`); no solo branch exists.
- `workflow/runtime_dispatch_assignment.rs:524` returns WaitingForPeers below that minimum. Its claim method independently rejects fewer than two assignments (`:563`), even though request validation accepts minimum 1 (`:1165`).
- Wait expiry at `task_execution_worker.rs:1519` marks the wait expired and defers the task event; it does not dispatch the anchor.
- Tests intentionally encode the defective product behavior: `task_execution_worker_defers_expired_batch_broker_wait_without_runtime_host_dispatch` (`:2927`) and `runtime_dispatch_assignment_batch_broker_rejects_one_member_ready_claim_without_mutating` (`runtime_dispatch_assignment.rs:2123`). These must change with the owning contract, not be retained as acceptance barriers.
- The downstream host batch request already accepts a nonempty one-member collection: `pantograph-runtime-host-contracts/src/runtime_host_execution.rs:424`. No wire schema migration is needed for cardinality 1.

### Smallest coherent proposed slice

Use the existing dispatch path for nonempty groups, including a singleton; claim already-ready compatible peers up to the existing maximum without requiring one to arrive. This is an ordinary reversible scheduling policy repair. Keep scheduler selection, reservation lifecycle, attempt identity and responder fan-out; do not create a demo/single-run executor. If retaining an aggregation delay is an actual product requirement, decide it before implementation and dispatch the anchor at its deadline; the current indefinite defer behavior cannot remain.

Exact initial source/test write set (tests are inline):

- `crates/pantograph-workflow-service/src/workflow/runtime_dispatch_assignment.rs`
- `crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs`
- `crates/pantograph-workflow-service/src/workflow/runtime_branch_batch_execution.rs`

The first two own the behavior; the third owns the terminal host-boundary regression. No scheduler crate or host contract change is evidenced as necessary. Remove worker-only unreachable wait/defer policy if immediate singleton admission makes it obsolete; do not broaden into a scheduler reorganization. Existing lower-level broker waiting APIs may remain only for demonstrated consumers; consumer search decides deletion.

Deciding evidence: a ready singleton reaches the recording host exactly once and completes its run; its reservation start/terminal facts and task/attempt/run identities are retained. Keep compatible preexisting peers grouped and incompatible assignments independent. Preserve rejection of empty/stale/reentered claims. Extend the existing batch execution fixture to a one-member case and a dependent text-result→image-input case whose host observes the exact generated string. Existing input-mapping missing/failed-output tests remain relevant. A cancelled/shutdown singleton must not leave an active responder or reservation.

Focused commands after implementation: `cargo test -p pantograph-workflow-service runtime_dispatch_assignment`, `cargo test -p pantograph-workflow-service task_execution_worker`, `cargo test -p pantograph-workflow-service runtime_branch_batch_execution`, and `cargo test -p pantograph-workflow-service runtime_host_task_input_mapping`; run affected Rust formatting/static gates selected by the integrator. None were run in this investigation. These tests prove orchestration with a controlled host, not real text/image inference.

## Adjacent blocker and unresolved concern

**EX-02 — canonical host is image-only (confirmed; runtime lane owns disposition).** Both single and batch entrypoints in `pantograph-embedded-runtime/src/runtime_host_execution_port.rs` construct image projections and call image generation (`:205`, `:370`). Solo admission alone cannot make text execute. The runtime reviewer owns the exact task-specific host extension, trust prerequisites and tests; do not duplicate that implementation here.

**Snapshot consistency requires a bounded later check; not yet an admitted defect.** `session_execution_api.rs:837` fetches the graph for snapshot creation, and `:765` fetches it again for task topology. The second path looks up inference facts by the first snapshot's fingerprint. Determine whether the concrete host guarantees immutability across these reads, or whether a save during the awaits can mix topology and validation facts. A changing-host regression can decide this without real models. Do not claim a race fix is required before establishing the actual host contract.

## Pilot and stopping decision

EX-01 is one reversible **multi-file orchestration behavior repair**, not two independent pilot tasks. It should not be split into a minimum-constant edit and a singleton-claim edit because neither alone delivers the outcome. With smaller confirmed pilot candidates available from other lanes, use those first; retain the scheduler slice for normal reviewed implementation. No second small execution bug was confirmed within this bounded pass; inventing one or extending the audit to fill a quota is not justified.

Decision: sufficient facts for the solo-dispatch slice; stop broad execution investigation. Real mixed-workflow acceptance remains blocked by the text-host/trust/model prerequisites reported by the runtime lane, plus EX-01.

Available tools do not expose ordinary delegated-task billed cost or token accounting. `get_goal` advertises goal-level token/elapsed usage, which is not per-task billing and no goal was created here. Shell tool output token counts are output truncation metadata, not model usage. Pilot cost is unknown unless the integrator has a separate authoritative usage source.

## EX-01 admission follow-up — lease lifetime blocks an expiry-only repair

Follow-up on 2026-09-08, read-only while RT01 owns the build slot. No Cargo commands or production edits. This narrows the earlier proposed slice; it does not authorize implementation.

**Concrete additional finding (EX-03, high): there is no inspected task-event lease renewal path.** `task_execution_worker.rs:47` grants a 30,000 ms event claim, while `runtime_dispatch_assignment.rs:26` also declares a 30,000 ms aggregation window. The wait constructor (`:865`) caps its deadline at event-claim expiry minus **1 ms**. The worker scans every **250 ms** (`task_execution_worker.rs:53`). `runtime_branch_task_event.rs:903` validates the active claim on completion, and `:1056` rejects a terminal transition at or after expiry. Expired Running events are eligible for reclaim (`:1065`), which increments attempt generation (`:730`). Searches for renewal/heartbeat/lease-extension across workflow-service and inspection of the task-event repository trait found claim/reclaim/release/terminal methods but no renewal operation. The worker JoinSet observes branch lifetime; it does not renew the repository lease.

Therefore changing only wait expiry from defer to execute leaves at most 1 ms before claim expiry (and commonly observes it after expiry). Real inference cannot be safely finalized under that contract. Immediate singleton dispatch avoids spending the lease on coalescing but does not repair existing executions lasting over 30 seconds. Merely increasing a fixed duration, ignoring claim expiry, or inventing a shorter aggregation timeout is not an admitted repair.

### Policy decision and exact candidate scope

There are two materially different choices:

- **Immediate opportunistic 1..N dispatch:** minimum 1, retain current compatibility predicate, maximum 8, anchor inclusion and ordered peer selection. Existing ready compatible work can still batch; a later arrival cannot join an already dispatched request. This eliminates the existing wait-to-collect behavior. No ADR or durable guide inspected promises that delay; the concrete worker and its two-peer test do. Treat removal as an explicit scheduling policy decision, not behavior-preserving refactoring. It improves singleton latency without selecting a new fairness algorithm or changing task inputs/results.
- **Retain collection window:** preserve pre-deadline peer grouping and dispatch the anchor at expiry, without letting a late incompatible/compatible peer renew its wait. This requires a live claim ownership solution first and expiry dispatch must enter the existing worker JoinSet rather than blocking its command/cancellation loop. The current synchronous expiry scanner cannot await real inference inline. No separate broker worker or detached task is justified.

For the immediate option the exact source/test set is the earlier three files **plus** `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs` to remove the now-unreachable `BatchBrokerWaitWindowExpired` outcome projection if that policy is deleted. Preserve declared per-member timeouts, existing cancellation handle construction, scheduler reservation checks, attempt/run ids and result correlation. Do not change priority ordering or add batching across incompatible contexts. The concrete policy tradeoff must be accepted by the integrator, and **EX-03 must be resolved before this can be called a safe real-model dispatch slice**. The event repository owner (`runtime_branch_task_event.rs`) and worker are the first bounded lifecycle-design population; exact additional lifecycle writes depend on whether renewal or a different existing ownership lifetime is selected. That decision is intentionally not guessed here.

### Existing red-capable seams and evidence

`task_execution_worker_defers_expired_batch_broker_wait_without_runtime_host_dispatch` at `task_execution_worker.rs:2927` already supplies a recording canonical host, prepared ready run, responder registry and controllable expiry timestamp. For retained waiting, change its expected result to one-member host dispatch and terminal completion. For immediate policy, require completion without invoking expiry and replace its obsolete defer assertions. `runtime_dispatch_assignment_batch_broker_rejects_one_member_ready_claim_without_mutating` at `runtime_dispatch_assignment.rs:2123` supplies the claim-boundary red test. The existing two-peer worker fixture and `runtime_branch_batch_execution_owner_dispatches_and_finalizes_claimed_batch` supply grouping, input, reservation and output-attribution observations.

Before admitting the lifecycle fix, add a deterministic repository/worker case covering a host result beyond the original event lease, with competing reclaim attempted while the original owner remains active; require one execution owner, one terminal result and no stale-attempt publication. Existing `runtime_branch_task_event_rejects_expired_claim_terminal_transition` and Running-event reclaim tests define the current contract and must remain valid for genuinely abandoned ownership. Preserve cancellation during waiting/execution and shutdown completion; test waits must not be real 30-second sleeps. Select focused existing-module Cargo filters only after the implementation write set is settled; no new harness is needed.

**Stopping decision:** bounded design blocker, not implementation-ready. Root must select the supported aggregation policy and resolve live event-claim lifetime. Sufficient facts are available to reject the unsafe expiry-only patch; no broader scheduler audit is required.

## EX-01/EX-03 implementation admission — scoped live claims

The integrator has selected immediate opportunistic groups of **1–8** compatible ready assignments, with no collection delay. This supersedes the preceding unresolved aggregation choice. This follow-up is design evidence only; no source edits or Cargo runs were performed.

### Chosen ownership contract

Use **scope-owned live claim proofs in the two existing in-memory repositories**, not a heartbeat/renewal subsystem. `service_config.rs:59` constructs the only task-event and dispatch-assignment repository implementations; both are private in-memory services. Repository consumer search found no separately deployed lease owner or durable lease store. Their record snapshots and existing replay tests do not establish distributed recovery authority. The worker's existing JoinSet is the live execution supervisor. Finite leases remain meaningful for an unowned/abandoned claim; they must not expire an actively owned execution.

The same invariant covers **both** task-event claims (30 seconds) and batch claims (1 second). `runtime_dispatch_assignment.rs` currently makes a previously claimed Running assignment eligible for another batch once its one-second claim expires. Fixing the task-event lease alone would therefore leave duplicate host dispatch possible.

Each repository keeps a weak liveness reference keyed by its existing fenced claim identity (lease id, owner and attempt generation; batch id for a batch claim). A narrow strong ownership token is returned only to the execution owner. Immutable event/assignment/attempt snapshots continue carrying ordinary identities and timestamps and **never** carry a strong token. A scoped ownership result/wrapper may hold the token separately from the existing record. Share a small private token representation between these two owners if useful; do not create a new service, global liveness registry, background renewal task or persistence schema. The finite deadline is not lengthened or ignored: an operation must prove either current live ownership or the existing unexpired unowned claim, and it must always match the current fence.

Repository claim/reclaim and terminal operations enforce this contract under their existing lock:

- A matching live-owned claim cannot be reclaimed or selected into a second batch at any timestamp. Its terminal transition remains valid beyond the original finite deadline.
- A different/superseded lease id, owner or generation cannot mutate the record, even if a token for that old identity remains alive.
- An ordinary claim with no admitted live-dispatch ownership retains existing finite expiry/reclaim behavior. Existing synthetic unowned Running-record tests can continue to exercise that behavior.
- Once owned execution has crossed the host-dispatch boundary, abandonment is **not** proof that the external operation stopped. Record that fact with the owned claim. If its strong proof disappears abnormally, it is fenced against automatic replay and against accepting an old success. The worker reports failure through its existing responder/outcome path and terminalizes the event/assignment as failed; it must not turn that state into Ready/Deferred or select it for another batch. Reclaim of genuinely unowned pre-dispatch work remains available. No automatic recovery subsystem is added.

### Exact transfer and supervision rules

1. Acquire the event's strong proof atomically with the worker claim, before the first preparation await. Keep it in the local branch scope while preparation runs. Ordinary preparation failure/defer settles the event before dropping ownership.
2. At assignment attachment, transfer that proof to the existing responder registration keyed by assignment. This lets a compatible peer be dispatched by another branch without dropping the first event's ownership. A copied registration handle is not an owner.
3. Claim a nonempty compatible group under the assignment repository lock and obtain its separate batch strong proof. Move all selected event proofs from the responder registrations into the claimed-batch execution scope before host dispatch. Missing proof or a concurrent successful claim is a typed outcome; no partial group may dispatch. A branch whose assignment is already owned by the winning group must leave completion to that group's fan-out, not fail its registered responder.
4. The executing scope retains the batch proof and every event proof through host response validation, scheduler task-result/reservation transitions, event/assignment terminal publication and responder fan-out. Cancellation requests do not release them. Cooperative drain retains them until the awaited host outcome is settled. No proof is stored in an immutable execution-plan snapshot solely to extend lifetime accidentally.
5. Associate supervised branch/group identity with the existing JoinSet task so a panic, abort or unexpected return is observed and its registrations receive the existing worker failure outcome. Do not discard `join_result`. Synchronous abandonment fencing can occur when scope ownership is lost; asynchronous cancellation/drain belongs to the worker, not `Drop`. The worker stops admission during shutdown and drains/supervises existing scopes before releasing their proofs. Any scope whose external completion is unproved is terminal-failed/fenced, never silently eligible for replay.

Keep the narrow token/claim-fencing primitive internal to `runtime_branch_task_event.rs` (or an internal type already available there) and reuse it from the assignment repository. A helper extraction into another file is unnecessary for this slice. Marking owned host dispatch and recording abandonment must remain with these repository owners, not with frontend, runtime host or diagnostics projections.

### Exact admitted production/test population

All tests are existing inline modules, so the coherent source and regression write set is:

1. `crates/pantograph-workflow-service/src/workflow/runtime_branch_task_event.rs` — scoped proof, fenced owned transitions and distinction between replayable unowned expiry and non-replayable owned-dispatch abandonment.
2. `crates/pantograph-workflow-service/src/workflow/runtime_dispatch_assignment.rs` — corresponding batch proof, one-member claims, existing compatibility/max-size/order, removal of collection-window policy with no remaining supported caller.
3. `crates/pantograph-workflow-service/src/workflow/task_execution_worker.rs` — immediate policy, proof transfer, JoinSet completion/failure supervision, cancellation/drain retention and removal of the wait timer/expiry deferral.
4. `crates/pantograph-workflow-service/src/workflow/runtime_branch_batch_execution.rs` — strong execution-scope ownership through all result mutation/finalization and canonical host boundary tests.
5. `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs` — remove the retired batch-window deferred projection; preserve existing failure/cancellation projection.

No public API, scheduler crate, runtime-host contract or durable schema change is required by this claim-lifetime design. If implementation needs a new scheduler cancellation API, a change to host termination guarantees, or a different recovery policy, return the exact missing obligation to medium analysis rather than silently expanding this write set.

### Deciding deterministic regressions

Use repository methods' explicit `now_ms` arguments and controlled host gates/oneshot channels. No real 30-second waits, new timer harness or Tokio test-util dependency is required.

- Live-owned event: advance supplied time beyond 30 seconds, attempt competing reclaim, complete the original fenced claim once. Expect exclusion, valid terminal completion and rejection of a second terminal mutation. Snapshot cloning must not keep ownership alive.
- Live-owned batch: advance beyond its one-second lease and attempt selection/claim from another anchor. Expect exclusion until the original batch settles. Stale old proof cannot mutate a replacement claim.
- Abandonment: dropped unowned/pre-dispatch claim retains the existing expired-reclaim behavior; an admitted owned dispatch whose scope panics/aborts is failed and excluded from replay, with a meaningful responder failure and no old success publication.
- Worker singleton: one ready task reaches the canonical recording batch port with one member and completes without another command or expiry tick. Exact input, task/run/attempt identity and terminal reservation/result facts remain intact.
- Compatible ready group: prearrange two compatible assignments before claiming; preserve one group and separate member results. Incompatible peers remain independent. Verify competing claims cannot double-dispatch a shared member. Do not retain a timing-dependent test that requires an arbitrary later arrival to be batched.
- Hold the controlled host open while testing cancellation and shutdown; ownership stays live until terminal handling/drain, and responder registration is removed exactly once. Inject supervised branch failure and observe cleanup/failure rather than silently discarded JoinError.

Run the existing `runtime_branch_task_event`, `runtime_dispatch_assignment`, `task_execution_worker`, `runtime_branch_batch_execution` and `runtime_host_task_input_mapping` Cargo test filters for `pantograph-workflow-service`, plus affected Rust static gates. Root baseline already reports the existing task-event filter passing 48 tests; that does not cover the new owned lifetime.

### Admission limit and stopping decision

This design is implementable for EX-01 and the live/fenced claim invariant of EX-03. It does **not** prove external worker termination after cancellation or abort. The runtime reviewer identified detached PyTorch text work and absent cancellation completion evidence; RT-02 owns that gap. An indeterminate host execution remains terminal-failed/fenced and may retain unresolved resources until its real owner establishes stop—do not label that full DA-04 cleanup acceptance. The implementation must not release or reuse a reservation by pretending external stop was observed.

Proceed to the bounded Astra-low slice after the integrator records this exact write set. No further broad scheduler review is needed to choose this claim ownership mechanism. Review the token transfer, abnormal-exit fencing and claim-before-result ordering especially closely; passing singleton tests alone cannot close them.

### Final composition and consumer migration

Independent medium source review accepted the repaired composition: evaluation,
claim and proof transfer share the required lock scope; owned event transitions
require the exact proof, not a copied claim identifier. All-member validation and
one-time publication precede scheduler mutations. Observed Failed/Rejected
settles the old assignment as Failed while retaining scheduler retry policy;
Deferred settles it as Deferred with existing handback semantics. Accepted is
indeterminate and rejects before any member publication. Supervised failure
preserves settlement diagnostics. Inference prerequisite RT-03 now owns actual
worker termination; this supersedes the earlier RT-02 ownership shorthand.

The full-crate gate additionally admitted six assertion migrations in
`workflow/tests/session_execution.rs`. Independently submitted runs need not
share a host call: every group must contain 1–8 members and the flattened
population must contain the exact two submitted runs once each, with distinct
execution/assignment identities. Failed/rejected APIs omit generated run IDs;
correlate host identities through persisted records to the exact independently
retained submitted session IDs instead. Preserve result, prompt, recovery and
typed-error assertions. Exact batch grouping remains covered by prearranged-peer
tests. The ledger owns final execution results and remaining baseline failures.
