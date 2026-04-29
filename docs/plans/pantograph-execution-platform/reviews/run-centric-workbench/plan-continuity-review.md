# Plan Continuity Review

## Status

Planning review. No source implementation.

Last updated: 2026-04-27.

## Purpose

Record a consistency pass over the run-centric GUI workbench plans after the
typed diagnostic event-ledger decision and current diagnostics code review.

This pass looked for plan errors, architectural anti-patterns, likely bug
causes, and continuity gaps across the staged design.

## Executive Findings

The plan direction is coherent, but several areas needed tighter language to
prevent implementation drift:

- Scheduler event persistence cannot use a separate scheduler-specific sibling
  repository. It must use the shared typed diagnostic event ledger boundary.
- The shared event-ledger core is a prerequisite for durable scheduler event
  persistence, even though scheduler work is numbered before diagnostics work.
- `run.*` and `scheduler.*` event ownership must be split cleanly so terminal
  run facts are not recorded twice with competing meanings.
- Workflow semantic versions cannot be reliably inferred by the system. The
  system can compute fingerprints and reject conflicts; semantic version
  labels must be supplied by the client/user contract or explicitly generated
  by a simple policy that does not claim semantic correctness.
- Node version data must not remain optional in executable workflow identity.
  Nodes must either provide version/fingerprint facts or be rejected/quarantined
  from versioned execution.
- Frontend DTO drift is already present, so Stage `04` needs a hard contract
  gate before pages consume new projections.
- Headless diagnostics reads currently mutate projection state; event-ledger
  implementation must distinguish read-time observations from execution facts.

## Corrections Applied

### Shared Event Ledger Boundary

Updated Stage `02` and Stage `03` language so durable scheduler events use the
shared typed diagnostic event ledger. A scheduler-specific sibling event store
would be an anti-pattern because it would duplicate envelope validation,
source ownership, retention/privacy classification, and projection rebuild
logic.

Implementation implication: if the shared ledger core is not available when
Stage `02` reaches durable event persistence, execute the Stage `03` ledger
bootstrap first instead of inventing a temporary scheduler store.

### Cross-Stage Ordering

The numbered files remain useful as product-area stages, but implementation
has one dependency inversion:

```text
Stage 01 foundations
Stage 03 ledger envelope/storage bootstrap
Stage 02 durable scheduler events
Stage 03 remaining diagnostics/retention/Library work
Stage 04 projections
Stage 05 shell
Stage 06 pages
Stage 07 gates throughout
```

Stage `02` can still define scheduler estimates, authority rules, payload
contracts, and event producer points before the ledger exists. It must not
complete durable scheduler event persistence until the shared ledger append and
validation boundary exists.

### Event Family Ownership

`run.*` owns execution lifecycle truth: run snapshot accepted, execution
started, execution completed, execution failed, execution cancelled, and run
status changes.

`scheduler.*` owns scheduling decisions and controls: submission accepted into
queue, estimate produced, queue placement, delay, promotion, cancellation
request/decision, reservation, admission, runtime/device selection, model
load/unload decisions, retry/fallback, client actions, and admin overrides.

Scheduler projections can join run lifecycle events, but scheduler producers
must not duplicate terminal run lifecycle records as separate scheduler facts.

### Semantic Version And Fingerprint Strictness

The plans now treat semantic version as an explicit contract label, not a
magic inference problem. The backend computes execution fingerprints and can
reuse existing workflow versions or reject semantic-version/fingerprint
conflicts. It should not claim to infer whether a change is major, minor, or
patch unless a later dedicated policy is designed and documented.

### Mandatory Node Version Facts

The current code has optional node contract version/digest data. That is not
safe enough for version-aware diagnostics. The updated plan requires Stage `01`
to make the behavior explicit: either executable nodes provide node version
facts, or they are rejected/quarantined from versioned execution. Later stages
should not carry fallback diagnostics identity logic.

## Anti-Patterns To Avoid

- Creating one event repository per event family.
- Persisting Tauri `WorkflowEvent` as a durable ledger event.
- Treating `graph_fingerprint` as workflow execution identity after Stage
  `01`.
- Recording scheduler terminal-run events separately from `run.*` lifecycle
  events.
- Letting frontend pages parse raw event payloads to reconstruct page facts.
- Adding new projection DTOs without TypeScript/Rust contract parity tests or
  generated bindings.
- Storing large I/O payloads inline before retention/privacy policy exists.
- Letting read requests silently create durable audit facts without source
  classification.
- Holding scheduler/session locks while writing durable events.
- Keeping both old mode navigation and new workbench navigation active.

## Likely Bug Causes

### Mixed Execution Identity

If `graph_fingerprint` remains active as a diagnostics grouping key while
workflow version ids are introduced, diagnostics can silently compare runs
that used different node versions.

Control: Stage `01` source-audit gate must remove or quarantine old active
fingerprint semantics.

### Duplicate Event Truth

If scheduler events also record completed/failed terminal lifecycle facts,
run status projections can disagree with run lifecycle events.

Control: `run.*` owns execution lifecycle. Scheduler timelines reference or
join run events where needed.

### Optional Node Version Fallbacks

If missing node versions are accepted as "unknown" in executable fingerprints,
two behaviorally different nodes can collapse into one workflow version.

Control: reject or quarantine executable nodes without version facts until a
documented local development version policy exists.

### DTO Drift

Current diagnostics already have Rust/TypeScript drift around scheduler
diagnostics fields. The new projection surface is much larger.

Control: Stage `04` must choose generated DTOs or paired contract tests before
frontend page implementation.

### Read-Time State Mutation

Headless diagnostics currently refresh scheduler/runtime projection state
during snapshot reads. If this becomes durable event writing without clear
source semantics, audits will mix execution observations and inspection-time
observations.

Control: event kinds must distinguish execution facts from observer facts, or
read-time refreshes must stay non-durable projections.

## Continuity Checklist For Implementation

Before implementing each stage:

- Verify all required predecessor contracts are available.
- Confirm whether the stage emits durable events; if yes, the shared event
  ledger core must already exist.
- Confirm event family ownership for every event kind touched by the stage.
- Confirm which read models are rebuildable projections and which are direct
  authoritative-state projections.
- Confirm frontend work consumes projections only.
- Confirm old active terminology has either been removed or redefined in an
  owning README with tests.

## Remaining Open Decisions

- Exact workflow identity grammar.
- Exact storage owner for workflow version registry.
- Whether typed diagnostic event ledger lives inside
  `pantograph-diagnostics-ledger` or a new shared diagnostics-event crate. This
  decision must apply to all event families, not one family at a time.
- Whether DTO parity is handled by generated bindings or paired contract tests.
- Initial I/O payload storage location and payload preview policy.
- First-pass local system metrics source for the Network page.
