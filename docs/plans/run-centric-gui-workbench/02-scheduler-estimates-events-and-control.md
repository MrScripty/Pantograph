# 02: Scheduler Estimates, Events, And Control

## Status

Draft plan. Not implemented.

## Objective

Extend the backend scheduler into an observable control plane that produces
pre-run estimates, records durable scheduler events, exposes model load/unload
state, supports intentional delay for better cache/model/runtime conditions,
and enforces client/session-scoped queue influence separately from privileged
Pantograph GUI admin actions.

## Scope

### In Scope

- Pre-execution scheduler estimate contracts and query behavior.
- Typed scheduler event contracts for run, queue, reservation, model
  load/unload, runtime/device selection, delay, retry, fallback, client
  actions, and admin overrides.
- Model/cache state visibility needed by scheduler estimates and Network page.
- Client/session/bucket-scoped queue controls.
- Privileged GUI/admin queue controls.
- Scheduler event persistence through the shared typed diagnostic event ledger.
- Tests for queue ordering, delay reasons, estimates, and authority boundaries.

### Out of Scope

- Full distributed scheduling across Iroh peers.
- Final scheduler machine-learning or advanced optimization model.
- Frontend Scheduler table implementation.
- Retention policy mechanics beyond recording policy ids in events where
  needed.

## Inputs

### Problem

The run-centric GUI must show future/queued work, estimates before execution,
why runs are delayed, when models are loaded/unloaded, and who influenced queue
order. These facts must be backend-owned and auditable instead of hidden in
logs or inferred in the frontend.

### Constraints

- Workflows do not declare authoritative resources.
- Scheduler derives resource needs from node facts, graph settings, model
  metadata, file sizes, runtime metadata, local/future node capabilities,
  current load, queue state, and diagnostics history.
- Scheduler alone decides reservations, placement, model load/unload, retries,
  fallback, and delay.
- Normal clients may influence only their own session/bucket work.
- Pantograph GUI is a privileged admin/developer surface.
- Scheduler events are typed diagnostic ledger events with allowlisted event
  kinds, schema versions, source ownership, privacy/retention classes, and
  validated payloads.

### Assumptions

- Existing scheduler modules in `pantograph-workflow-service` remain the
  first owner for in-process queue policy.
- Durable scheduler events use the shared typed diagnostic event ledger
  architecture. If the shared ledger core is not available when durable event
  persistence begins, execute the Stage `03` ledger bootstrap first instead of
  creating a scheduler-specific repository.
- Initial estimates can be rule-based and low-confidence when historical data
  is sparse.

### Dependencies

- Stage `01` run snapshots and workflow version ids.
- `diagnostic-event-ledger-architecture.md`.
- `pantograph-workflow-service/src/scheduler`.
- Runtime registry and managed runtime load/unload facts.
- Library/model metadata from Pumas and runtime registry.
- Diagnostics history from `pantograph-diagnostics-ledger`.
- Client/session/bucket attribution.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Events are emitted but not persisted or queryable. | High | Define typed event repository/query contract before UI work. |
| Scheduler events become arbitrary metadata. | High | Emit only allowlisted typed payloads through backend event builders. |
| Estimates become confused with observed facts. | High | Store estimate and observation records separately with timestamps. |
| Client queue actions accidentally affect other sessions. | High | Add authority tests around each queue mutation. |
| Scheduler event or estimate writes happen while session/runtime locks are held. | High | Define lock-boundary rules and emit durable events through a narrow owner outside critical locks. |
| Scheduler delay looks like starvation. | Medium | Emit explicit delay reason and fairness evidence in scheduler events. |
| Model load/unload state splits across runtime and scheduler. | Medium | Define one projection owner for scheduler-visible cache/model state. |
| Scheduler events duplicate `run.*` lifecycle truth. | High | Keep terminal execution lifecycle in `run.*`; scheduler events record decisions, controls, admission, and resource actions. |
| Scheduler timeline queries replay all scheduler events as history grows. | High | Treat scheduler timeline as a hot materialized projection maintained through the shared event ledger cursor model. |

## Definition of Done

- Queued/future runs have pre-run scheduler estimates.
- Estimates are queryable by run id and visible through API projections.
- Scheduler emits queryable typed events for model load/unload and queue
  decisions.
- Scheduler timeline projections include relevant `run.*` and `node.*`
  lifecycle events for audit visibility without duplicating those facts as
  `scheduler.*` event truth.
- Scheduler run-list/detail/timeline views read materialized hot projections;
  they do not replay all scheduler events on normal Scheduler page load.
- Delay for better cache/model/runtime state is explicit and auditable.
- Client/session-scoped actions cannot modify other sessions.
- Privileged GUI/admin actions are represented distinctly from normal client
  actions.
- Scheduler tests cover estimates, event emission, authority boundaries, and
  delay reasons.

## Milestones

### Milestone 1: Estimate And Event Contracts

**Goal:** Define backend-owned contracts before changing scheduler behavior.

**Tasks:**

- [ ] Define `SchedulerEstimate` fields, estimate timestamp/version, and
  confidence/quality semantics.
- [ ] Define `SchedulerEvent` event types as a typed diagnostic event family
  with allowlisted event kinds, schema versions, payload structs, source
  components, privacy classes, retention classes, and validation rules.
- [ ] Define model/cache state enum used by estimates and events.
- [ ] Define client action versus admin override event vocabulary.
- [x] Confirm the shared typed diagnostic event ledger bootstrap is available
  before durable scheduler event persistence. If it is not available, execute
  that bootstrap before Milestone 3.
- [ ] Define lock-boundary rules for estimate calculation and event
  persistence so scheduler/session locks are not held during durable writes.
- [ ] Define the ownership split between `run.*` lifecycle events and
  `scheduler.*` decision/control events.
- [ ] Define scheduler hot projection ownership for run list, run detail,
  current status, and scheduler timeline updates through the shared
  `projection_state` cursor model.

**Verification:**

- Contract unit tests cover serialization and stable enum values if exposed
  over transport.
- README or ADR updates record storage/ownership decisions.

**Status:** In progress. The shared typed diagnostic event ledger is available
in `pantograph-diagnostics-ledger`, and the workflow-session scheduler emits
durable `scheduler.estimate_produced` and `scheduler.queue_placement` events
after queue insertion when a diagnostics ledger is configured. The hot
run-list and run-detail projections now promote typed scheduler queue position,
priority, estimate confidence, estimated queue wait, estimated duration, and
scheduler reason fields from those events. The typed scheduler event family
also has validated delay and model lifecycle payload contracts, including a
model lifecycle transition enum, and the timeline projection can materialize
those rows when emitted. Richer estimate DTO semantics, admin/client action
vocabulary, production model-load emitters, and complete hot projection
ownership remain pending.

### Milestone 2: Estimate Production

**Goal:** Produce estimates for queued and future runs before execution.

**Tasks:**

- [ ] Build scheduler estimate inputs from run snapshot, node metadata, graph
  settings, model metadata, runtime state, local node capacity, queue state,
  and diagnostics history where available.
- [ ] Add estimate generation/update on run submission and relevant queue/cache
  state changes.
- [ ] Distinguish blocking conditions, delay reasons, and missing assets.
- [ ] Record candidate runtimes/devices/network nodes where known.

**Verification:**

- Tests cover estimates at submission time.
- Tests cover estimate updates after queue/model/cache state changes.
- Tests cover sparse-data estimates returning explicit low-confidence state.

**Status:** In progress. Queue insertion now records a low-confidence
submission-time scheduler estimate for the queued run, and the stable estimate
facts are queryable from hot run-list/run-detail projections. Typed delay
events can now record a concrete delay reason and delayed-until timestamp into
run-list/run-detail status and scheduler timeline projections when a scheduler
emitter produces them, and workflow-session runtime admission waits now emit
the first production delay event. Rich estimate inputs from model metadata,
runtime state, local node capacity, diagnostics history, cache/model state
changes, and missing-asset analysis remain pending.

### Milestone 3: Scheduler Event Emission And Persistence

**Goal:** Record auditable scheduler behavior for each run and for global
scheduler activity.

**Tasks:**

- [ ] Emit scheduler decision/control events: submission accepted into queue,
  estimate produced, queue placement, delayed, promoted, cancellation
  requested/accepted/denied, admitted, reservation created/released.
- [ ] Emit reservation/runtime/device selection events.
- [ ] Emit model load requested/started/completed/failed and unload
  scheduled/cancelled/started/completed events.
- [ ] Emit retry/fallback events.
- [ ] Emit client queue action and admin override events.
- [ ] Join or reference `run.*` lifecycle events in scheduler projections
  without duplicating terminal execution lifecycle facts as scheduler events.
- [ ] Persist typed scheduler events through the event ledger owner.
- [ ] Build scheduler event projections from ledger events for run-scoped and
  system-scoped queries using incremental materialized projection cursors.
- [ ] Include relevant `run.*` and `node.*` lifecycle events in scheduler
  timeline projections where the requirements call for execution/node
  visibility.

**Verification:**

- Repository/query tests cover typed run-scoped and system-scoped events.
- Validation tests reject malformed scheduler events, missing required
  correlation ids, unsupported schema versions, and disallowed source
  components.
- Scheduler store/policy tests assert expected events for representative
  transitions.
- Replay/recovery tests cover event visibility after restart if persistence is
  implemented in this stage.
- Tests prove Scheduler page projection queries read materialized hot
  projections and do not full-replay all scheduler events.

**Status:** In progress. Estimate-produced and queue-placement events are now
persisted through the typed event ledger for queued workflow-session runs.
The first scheduler timeline projection now drains those scheduler events plus
`run.snapshot_accepted` into materialized timeline rows by event cursor.
Scheduler delay and model lifecycle events now have validated ledger payloads
and materialized timeline summaries. Workflow-session runtime admission waits
now emit one durable `scheduler.run_delayed` event per wait. Queue
cancel/reprioritize/push-front controls now emit typed
`scheduler.queue_control` events with accepted outcome, actor scope, previous
queue position, previous priority, new priority where applicable, and a bounded
reason. Run-list, run-detail, and scheduler timeline projections now
materialize those queue-control events.
Queue admission now emits a typed `scheduler.run_admitted` event before
`run.started`, so scheduler admission decisions are auditable without making
`run.*` lifecycle events carry scheduler control semantics. Workflow-session
runtime admission now emits production `scheduler.model_lifecycle_changed`
events for required-model load requested/completed/failed transitions using
preflight required model/backend facts, and ephemeral session teardown emits
required-model unload scheduled/started/completed/failed transitions from the
same immutable run facts. Run-triggered capacity rebalance now emits unload
scheduled/started/completed/failed transitions for the selected candidate's
required models. Broader client/admin action vocabulary and frontend page
wiring remain pending. Workflow-service now has a query boundary for the
materialized scheduler timeline and a narrow scheduler estimate query boundary
backed by the hot run-detail projection.

### Milestone 4: Queue Authority And Admin Controls

**Goal:** Enforce client/session/bucket queue authority while allowing
Pantograph GUI privileged actions.

**Tasks:**

- [ ] Add or harden normal client actions: cancel own run, query own estimate,
  priority changes within policy, push to front of own session queue,
  clone/resubmit own run.
- [ ] Add privileged admin actions for GUI: cancel any run, reorder across
  sessions, pause/resume queues or buckets, override priority, force estimate
  recomputation, force reschedule where supported.
- [ ] Record authority context in typed scheduler events.
- [ ] Ensure scheduler remains final authority after an action request.

**Verification:**

- Tests prove normal clients cannot affect other sessions.
- Tests prove admin actions produce admin override events.
- Tests prove scheduler can deny or normalize requested priorities based on
  policy.

**Status:** In progress. The existing backend queue cancel, reprioritize, and
push-front APIs still enforce session ownership through session id plus run id
matching, and they now record accepted and denied queue-control facts in the
typed diagnostics ledger when diagnostics are configured. The Scheduler page
now gates its first cancel/front controls on projected workflow
execution-session ids before calling those backend commands. Push-front is a
scheduler-owned operation that computes the next priority from the current
session queue and denies the request if the priority ceiling prevents a real
move. Query-own-estimate now has a workflow-service and frontend projection
method that returns estimate-shaped hot projection facts for a run without raw
ledger access. Session-owned queue-control events now use the `client_session`
actor scope. The first GUI-admin queue boundary can cancel a queued run by run
id across sessions, leaves the scheduler store as the authority, and emits
`gui_admin` queue-control events for accepted/correlated denied decisions.
Clone/resubmit, running-run cancellation, privileged cross-session reorder,
pause/resume, and other admin-scope event emitters remain pending.

## Ownership And Lifecycle Note

Scheduler event production must have one backend owner. Runtime/model load
callbacks may provide facts, but scheduler-facing state transitions should be
recorded through typed event builders and one scheduler event pathway to avoid
duplicate, unvalidated, or contradictory event streams.

## Re-Plan Triggers

- Shared typed event ledger bootstrap cannot support scheduler event volume,
  validation, or query needs.
- Scheduler timeline projection cannot stay current without blocking run
  admission or replaying full event history on page load.
- Scheduler estimates require asynchronous model/library metadata loading.
- Existing runtime load/unload callbacks do not expose enough facts.
- Admin GUI authority needs a full user/auth model rather than the current
  special-case GUI privilege.

## Completion Summary

### Completed

- Confirmed the shared typed diagnostic event ledger is available for
  scheduler event persistence.
- Added durable `scheduler.estimate_produced`,
  `scheduler.queue_placement`, `scheduler.run_delayed`, and
  `scheduler.queue_control` event paths for workflow-session scheduling
  behavior implemented so far.
- Added durable `scheduler.run_admitted` events so scheduler admission is
  recorded separately from `run.started` lifecycle truth.
- Added materialized projection coverage for scheduler estimates,
  placements, delays, admissions, queue controls, and run lifecycle visibility
  required by current Scheduler page query boundaries.

### Deviations

- Queue-control events currently represent backend control API actions. The
  future client/admin authority split still needs explicit actor scopes and
  priority-normalization outcomes.

### Follow-Ups

- Decide whether estimate quality should be enum-only or include numeric
  confidence.
- Add admission/reservation events and any remaining non-run-triggered runtime
  lifecycle emitters.
- Add explicit client/admin action vocabulary, denial outcomes, and authority
  tests for cross-session controls.

### Verification Summary

- `cargo test -p pantograph-diagnostics-ledger`
- `cargo test -p pantograph-workflow-service`

### Traceability Links

- Requirement sections: Scheduler Page Requirements, Scheduler Estimates,
  Scheduler Authority and Resource Inference, Scheduler Events and Auditability,
  Client Session Bucket and Admin Control.
