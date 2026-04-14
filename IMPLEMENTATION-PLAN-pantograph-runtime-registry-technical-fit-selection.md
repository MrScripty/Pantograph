# Plan: Pantograph Runtime Registry And Technical-Fit Selection

## Status
Active

Last updated: 2026-04-13

## Current Source-of-Truth Summary

This plan is no longer "planning only." Milestone 2 is now complete and the
next work moves to admission, warmup, retention, and eviction.

The accurate status is:

- prerequisite runtime-contract and diagnostics groundwork: completed
- runtime-registry Milestone 1, Freeze Boundaries And Ownership: completed
- runtime-registry Milestone 2, Runtime Registry Foundation: completed
- runtime-registry Milestones 3-6: not started
- current next milestone: Milestone 3, Admission, Warmup, Retention, And
  Eviction
- stop rule remains active for later milestones: runtime-registry work must
  preserve ADR-002 and the
  README boundary decisions landed in Milestone 1

## Objective

Add a Pantograph-owned `RuntimeRegistry` and a two-level technical-fit selection
flow that stays aligned with the coding standards by preserving current
facades, keeping lifecycle ownership explicit, and validating the full
producer-to-consumer path across workflow service, runtime wiring, and host
adapters.

## Scope

### In Scope

- add a runtime-registry layer above the existing `InferenceGateway`
- track live runtime/model residency, admission, warmup, retention, and
  eviction state
- consume Pumas feasible execution candidates and select the best technical fit
  for a workflow run
- integrate registry decisions with workflow session lifecycle, dependency
  preflight, and `keep_alive`
- add diagnostics, contract coverage, and documentation/ADR traceability
  required by the standards

### Out of Scope

- replacing `InferenceBackend`, `BackendRegistry`, or `InferenceGateway`
- prompt-semantic or answer-quality model routing
- Pumas-owned live process/runtime lifecycle
- distributed or multi-host scheduling
- persisting runtime-registry state in milestone one unless later justified

## Accurate Current Position

### Completed prerequisite groundwork

The following has already landed in code and should be treated as completed
inputs to this plan rather than future work:

- shared Rust-side backend/runtime identity normalization
- frontend HTTP host backend identity normalization
- embedded runtime backend alias normalization
- workflow capability reporting for dedicated embedding and Python-backed
  runtimes
- runtime lifecycle normalization across gateway, workflow service, diagnostics,
  and workflow execution contracts
- canonical workflow `required_backends` reporting
- trace and diagnostics fallback matching aligned to canonical backend keys
- runtime producer snapshot preservation for diagnostics and trace surfaces

### Runtime-registry work already landed

The following Milestone 2 foundation slices have now landed in code:

- backend-owned `pantograph-runtime-registry` crate under
  `crates/pantograph-runtime-registry`
- canonical runtime registration and backend-key normalization at the registry
  boundary
- deterministic runtime state transitions for stopped, warming, ready, busy,
  unhealthy, stopping, and failed states
- reservation acquisition/release bookkeeping with deterministic snapshot output
- focused unit coverage for canonicalization, invalid transitions, reservation
  lifecycle, and reservation rejection while stopping
- Tauri composition-root creation and shared app-state management for the
  registry in `src-tauri/src/main.rs`
- Tauri-side `runtime_registry.rs` adapter that translates backend-owned
  `ServerModeInfo` lifecycle facts into backend-owned registry observations for
  active and embedding runtimes
- backend/server command synchronization that refreshes registry state after
  backend switches, runtime starts, runtime stops, external attachment, and
  status reads
- health-monitor and manual-recovery synchronization that refreshes registry
  state from host-owned runtime health observation paths
- headless workflow adapter synchronization that refreshes registry state
  before embedded-runtime capability and diagnostics snapshot reads

### What has not landed yet

- no registry-driven warmup, retention, or eviction policy exists yet
- no registry-driven cleanup or recovery policy exists yet
- no Pumas-driven technical-fit selector is integrated into workflow execution

## Inputs

### Problem

Pantograph currently validates backend/runtime availability and dependency
readiness, but it does not maintain a live, app-owned view of runtime
residency, loaded models, or budget-aware admission. That creates duplication
risk in workflow execution paths and prevents strong, standards-aligned
ownership of long-lived runtime state.

### Constraints

- preserve the current public runtime facade unless an explicit API break is
  approved
- keep gateway/backends as execution infrastructure, not policy owners
- keep runtime lifecycle ownership in one Pantograph-owned boundary
- use technical-fit factors grounded in verifiable facts only
- follow directory README, ADR traceability, testing, and concurrency
  standards
- avoid creating oversized catch-all modules; perform decomposition review
  before crossing file-size thresholds

### Assumptions

- Pumas will expose ranked feasible execution candidates from durable facts
- Pantograph remains the owner of live runtime admission, reuse, and eviction
  policy
- milestone three can use conservative estimated RAM/VRAM data with explicit
  headroom rules
- manual `model_id` / `backend_key` selections remain supported as explicit
  overrides

### Dependencies

- `crates/inference`
- `src-tauri/src/llm`
- `src-tauri/src/workflow`
- `crates/pantograph-workflow-service`
- Pumas feasible-candidate proposal and its additive APIs
- existing docs and ADR surfaces under `docs/`

### Affected Structured Contracts

- internal runtime-registry state and reservation/admission contracts
- Pantograph host-to-service runtime capability and selection contracts
- workflow preflight/runtime-not-ready semantics where registry participation
  changes behavior
- optional diagnostics/debug payloads if exposed to UI or bindings

### Affected Persisted Artifacts

- none required in milestone three unless later justified
- `config.json` only if rollout toggles or runtime-policy defaults are added
  later
- documentation/ADR artifacts under `docs/`

### Concurrency / Race-Risk Review

- runtime start/stop, health monitoring, session `keep_alive`, workflow queue
  advancement, recovery, and eviction can overlap
- registry state must have a single owner and serialized transition rules
- related runtime fields must move together under one lock or one message-driven
  state transition path
- admission must reserve capacity before warmup begins
- eviction must exclude active, reserved, and pinned runtimes/models
- shutdown and recovery must cancel background work deterministically and remove
  stale reservations
- ownership and lifecycle note:
  - app composition root creates and owns the registry
  - workflow/session paths request explicit registry operations; they do not
    mutate runtime state directly
  - registry starts/stops any background tasks and owns their cleanup

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Runtime policy logic leaks into gateway/backends | High | Preserve facade-first layering; keep policy in a separate registry/application layer |
| Active workflow runs are impacted by eviction or restart races | High | Add reservation tokens, active-run exclusion, and serialized transition tests |
| Pumas candidate data is incomplete or delayed | High | Keep manual override and conservative fallback selection paths |
| Large files/modules absorb too many responsibilities | Medium | Include decomposition-review checkpoints and extract focused modules early |
| Cross-layer behavior drifts between service, Tauri host, and diagnostics surfaces | Medium | Add acceptance checks across producer input, selection, execution, and output reporting |

## Definition of Done

- a `RuntimeRegistry` exists as the single owner of live runtime/model
  residency state
- workflow session execution uses registry admission/reuse logic without
  breaking existing contracts
- Pantograph consumes Pumas feasible candidates and performs a technical-fit
  selection using live app state
- manual selection overrides still work predictably
- cross-layer acceptance checks cover candidate input -> selection -> runtime
  admission -> workflow execution/output path
- required READMEs and ADR/documentation updates are present for touched
  architectural boundaries

## Milestones

### Milestone 0: Runtime Contract Groundwork

**Goal:** Finish the prerequisite contract cleanup that the runtime registry will
rely on.

**Tasks:**
- [x] Centralize Rust-side backend/runtime identity normalization
- [x] Normalize workflow capability runtime ids and `required_backends`
- [x] Normalize diagnostics/runtime lifecycle fallback matching to the shared
  backend-key rules
- [x] Surface dedicated embedding and Python-backed runtimes through workflow
  capability contracts
- [x] Preserve concrete producer/runtime observations in diagnostics and trace
  payloads

**Verification:**
- Shared identity helper tests pass
- Workflow-service capability/trace alias tests pass
- Tauri diagnostics runtime-fallback tests pass
- Recent atomic commits document each runtime-convergence slice

**Status:** Completed

### Milestone 1: Freeze Boundaries And Ownership

**Goal:** Lock the architecture and lifecycle boundaries before implementation
spreads across modules.

**Milestone 1 Objective:**

Create the architecture and documentation freeze that allows later registry
implementation to proceed without boundary drift. Milestone 1 is complete only
when Pantograph has explicit, reviewable answers for registry ownership,
composition-root placement, lifecycle control, reservation semantics, facade
preservation, and documentation traceability.

**Concrete Deliverables:**
- `docs/adr/README.md`
  - add the ADR index required by the documentation standards
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
  - record the accepted boundary decision for `RuntimeRegistry` ownership,
    facade preservation, composition-root placement, and lifecycle control
- update `src-tauri/src/llm/README.md`
  - document where runtime-registry ownership will live relative to Tauri app
    composition and runtime transport wiring
- update `src-tauri/src/workflow/README.md`
  - document how workflow/session paths consume registry operations without
    becoming policy owners
- update `crates/inference/src/README.md`
  - document that `InferenceGateway` remains an execution facade and
    infrastructure boundary rather than the owner of runtime policy
- update `crates/pantograph-embedded-runtime/src/README.md`
  - document embedded-runtime responsibilities after registry ownership is
    introduced
- update this implementation plan
  - mark Milestone 1 progress and record any approved boundary decisions or
    scope corrections that emerge during the doc freeze

**Boundary Decisions That Must Be Frozen In This Milestone:**
- `RuntimeRegistry` owner
  - identify the single Pantograph layer/module that owns live runtime
    residency state
- composition root
  - identify where the registry is created, injected, started, and stopped
- facade preservation
  - explicitly preserve `InferenceGateway` as the execution facade and forbid
    moving runtime policy ownership into gateway/backends
- workflow/service interaction
  - define how workflow/session paths request registry operations without
    mutating long-lived runtime state directly
- embedded-runtime interaction
  - define whether embedded runtime consumes registry decisions, exposes
    registry-backed capabilities, or remains a runtime producer only
- background-task ownership
  - define who owns health polling, recovery checks, warmup tracking, and
    cleanup responsibilities
- reservation contract
  - freeze the high-level admission/reservation token shape and lifecycle
    expectations before implementation

**Ordered Work Packages:**

1. Boundary inventory and evidence capture
- [x] inventory the current ownership seams across `crates/inference`,
  `crates/pantograph-embedded-runtime`, `src-tauri/src/llm`, and
  `src-tauri/src/workflow`
- [x] map which modules currently start runtimes, stop runtimes, track active
  state, expose capabilities, and surface diagnostics
- [x] capture the current producer/consumer contract surfaces that the registry
  must preserve or coordinate

2. Architecture decision draft
- [x] draft `ADR-002-runtime-registry-ownership-and-lifecycle.md` using the ADR
  format from the documentation standards
- [x] record the chosen architectural role for `RuntimeRegistry` using the
  layered and monorepo package-role guidance from `ARCHITECTURE-PATTERNS.md`
- [x] record rejected alternatives with reasons, including at minimum:
  - gateway-owned runtime policy
  - workflow-service-owned runtime residency
  - adapter-owned runtime policy in Tauri host layers

3. Lifecycle and reservation freeze
- [x] define the registry lifecycle note in concrete terms:
  - who creates it
  - who starts background work
  - who stops background work
  - how cancellation/cleanup happens
  - how overlap and restart races are prevented
- [x] define the registry state-machine outline for milestone-2 implementation:
  - stopped
  - warming
  - ready
  - busy
  - unhealthy
  - stopping
  - failed
- [x] define the reservation/admission contract at the architecture level:
  - reservation creation point
  - release point
  - failure semantics
  - exclusion rules for eviction

4. Documentation traceability updates
- [x] add `docs/adr/README.md` with an ADR index entry for ADR-001 and the new
  ADR-002
- [x] update the affected directory READMEs listed above so each one records the
  runtime-registry boundary impact using the required sections from
  `DOCUMENTATION-STANDARDS.md`
- [x] ensure the touched READMEs include explicit `Related ADRs` references to
  the runtime-registry ADR or an explicit `None` statement only where allowed

5. Freeze review and milestone close
- [x] review the milestone outputs against `PLAN-STANDARDS.md`,
  `ARCHITECTURE-PATTERNS.md`, `DOCUMENTATION-STANDARDS.md`, and
  `CONCURRENCY-STANDARDS.md`
- [x] update this plan's Milestone 1 status and completion notes
- [x] record any re-plan result immediately if the architecture review changes
  milestone 2 sequencing or scope

**Milestone-Specific Risks:**

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| The ADR stays too abstract and does not freeze implementable ownership decisions | High | Require explicit decisions for owner, composition root, lifecycle owner, and reservation boundaries before closing the milestone |
| README updates drift into filler instead of real traceability | Medium | Use the documentation template sections and require project-specific rationale in each touched README |
| Milestone 2 starts before lifecycle ownership is fully frozen | High | Keep the stop rule explicit in this plan and do not open registry implementation commits before Milestone 1 closeout |
| The chosen boundary forces a package-role violation | High | Review the ADR against layered separation and monorepo package-role rules before accepting it |

**Verification:**
- architecture/doc review against `CODING-STANDARDS.md`,
  `ARCHITECTURE-PATTERNS.md`, `DOCUMENTATION-STANDARDS.md`, and
  `CONCURRENCY-STANDARDS.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md` exists and
  includes `Context`, `Decision`, and `Consequences` sections per ADR standards
- `docs/adr/README.md` exists and indexes both ADR-001 and ADR-002
- each touched README includes the required reasoning sections and traceability
  links rather than generic placeholders
- milestone-close review confirms no runtime-registry code or state-machine
  implementation began before the ownership/boundary freeze landed

**Milestone 1 Completion Checklist:**
- [x] architecture boundary accepted in ADR form
- [x] composition root and lifecycle owner named explicitly
- [x] facade-preservation decision recorded explicitly
- [x] reservation/admission high-level contract frozen
- [x] README and ADR traceability updated
- [x] plan status updated before Milestone 2 work begins

**Status:** Completed

### Milestone 2: Runtime Registry Foundation

**Goal:** Introduce the live runtime state layer without breaking current
runtime callers.

**Tasks:**
- [x] Add focused runtime-registry modules for runtime records, model
  residency, reservation/admission decisions, and error/reporting types
- [x] Keep the composition root near app startup; do not create ad hoc global
  infrastructure inside feature modules
- [x] Track runtime states such as stopped, warming, ready, busy, unhealthy,
  stopping, and failed
- [x] Add deterministic state-transition rules for start, connect-external,
  stop, and recovery-ready status changes
- [x] Perform decomposition review on any touched files approaching
  file-size/responsibility thresholds
- [x] Integrate registry-backed lifecycle observation into the main backend and
  server command callers without moving policy into command handlers
- [x] Extend registry synchronization into host-owned health and manual
  recovery flows
- [x] Extend registry synchronization into the remaining headless workflow
  runtime flows

**Verification:**
- `cargo check --workspace`
- unit tests for state transitions, reservation lifecycle, and cleanup paths
- compile review confirms existing gateway callers still depend on preserved
  facades

**Status:** Completed

### Milestone 3: Admission, Warmup, Retention, And Eviction

**Goal:** Make runtime placement budget-aware and concurrency-safe.

**Tasks:**
- [ ] Keep admission, retention, and eviction policy in
  `crates/pantograph-runtime-registry`; adapters and gateway callers may invoke
  explicit registry operations but must not become policy owners
- [ ] Add admission checks using estimated RAM/VRAM with explicit safety margins
  and failure reasons
- [ ] Add warmup/reuse orchestration for session create/run through explicit
  registry-owned decisions plus explicit release paths after execution,
  cancellation, and failure
- [ ] Extend `keep_alive` into a retention hint interpreted by registry policy
  rather than raw direct ownership
- [ ] Implement eviction v1 with active/reserved/pinned exclusion and
  deterministic candidate ordering
- [ ] Keep async paths non-blocking: do not hold synchronous locks across
  blocking work or long-running awaits; if policy coordination becomes
  long-lived or await-heavy, introduce an explicit async owner/message path
  instead of stretching the current synchronous critical section model
- [ ] Bound and document any new admission, retention, or cleanup queues/timers
  before they are introduced so ownership, cancellation, and overflow behavior
  stay explicit

**Progress:**
- 2026-04-13: Session runtime load/unload in the embedded host now translates
  into explicit registry reservation acquire/release operations without moving
  policy ownership into Tauri adapters.
- 2026-04-13: `crates/pantograph-runtime-registry` now owns initial admission
  budget primitives plus RAM/VRAM rejection reasons and tested safety-margin
  checks for reservation requests.
- 2026-04-13: The live embedded-runtime reservation path now forwards workflow
  memory estimates into registry reservation requests, and repeated runtime
  registration preserves any previously configured admission budget instead of
  wiping it.
- 2026-04-13: `WorkflowService` now derives a typed retention hint from session
  `keep_alive` state and forwards it through embedded-runtime into registry
  reservation records, while direct retention policy remains to be implemented
  inside the registry boundary.
- 2026-04-13: `crates/pantograph-runtime-registry` now exposes deterministic
  eviction candidates with active-reservation and pinned-model exclusion so the
  next retention/eviction slice can consume a backend-owned ordering primitive.
- 2026-04-13: Reservation release and direct retention inspection now return a
  backend-owned retention disposition, including keep-alive-backed retention
  reasons for shared runtimes, so adapters do not need to infer policy from raw
  reservation state.
- 2026-04-13: The registry now also exposes an idempotent reservation-release
  path so overlapping cleanup or retry flows can remain concurrency-safe
  without adapter-local duplicate-release suppression logic.
- 2026-04-13: Session runtime acquire now has a backend-owned owner-key reuse
  path so repeated session loads can converge on the same reservation inside
  the registry lock instead of depending on adapter-local duplicate-load
  guards.
- 2026-04-13: The registry now exposes ordered reservation-owner eviction
  candidates derived from backend-owned runtime pressure and retention policy
  so higher layers can consume one policy order instead of rebuilding unload
  ranking from local state alone.

**Verification:**
- `cargo test -p pantograph-runtime-registry`
- unit tests for admission acceptance/rejection, warmup reuse, release-on-
  completion/cancellation/failure, and eviction ordering
- cross-layer acceptance check from workflow/session request through registry
  reservation/admission, runtime warmup or reuse, execution completion, and
  release
- integration tests for overlapping session runs, restart/recovery, duplicate
  release/idempotency behavior, and keep-alive transitions
- compile review confirms Tauri/workflow adapters remain transport wrappers and
  do not absorb registry business logic
- concurrency review against `CONCURRENCY-STANDARDS.md`
- README/ADR update review for any newly introduced directory boundary or
  machine-consumed contract changes per `DOCUMENTATION-STANDARDS.md`

**Status:** In progress

### Milestone 4: Technical-Fit Selection Integration

**Goal:** Choose the best run-time technical fit from Pumas feasible candidates
and local runtime state.

**Tasks:**
- [ ] Define Pantograph’s candidate-consumption contract and decision reason
  payload
- [ ] Select using technical-fit factors only:
  - required context length
  - task/runtime requirements
  - current residency and reuse value
  - warmup cost
  - budget pressure
  - workflow/session queue pressure
- [ ] Preserve explicit `model_id` / `backend_key` overrides with deterministic
  precedence
- [ ] Add conservative fallback behavior when candidate data is partial or
  unavailable

**Verification:**
- unit tests for deterministic tie-breaking, override precedence, and fallback
  behavior
- cross-layer acceptance check from candidate input through workflow run output
  per `TESTING-STANDARDS.md`
- contract review confirms no answer-quality claims are encoded in the API or
  docs

**Status:** Not started

### Milestone 5: Workflow And Adapter Integration

**Goal:** Integrate registry-driven runtime selection without reintroducing
business logic into adapters.

**Tasks:**
- [ ] Wire registry decisions through Tauri workflow host/task-execution paths
  while keeping adapters thin
- [ ] Keep workflow service and host adapters aligned with service-independence
  standards
- [ ] Ensure runtime-not-ready and admission-failure paths stay deterministic
  and machine-consumable
- [ ] Verify binding/adapter surfaces remain transport wrappers rather than
  policy owners

**Verification:**
- `cargo check --manifest-path src-tauri/Cargo.toml`
- targeted workflow service and Tauri integration tests
- acceptance check proving producer semantics survive adapter binding and
  execution

**Status:** Not started

### Milestone 6: Diagnostics, Documentation, And Rollout Safety

**Goal:** Make the system observable and keep the codebase aligned after the
refactor lands.

**Tasks:**
- [ ] Expose runtime-registry snapshots and recent decision diagnostics for
  internal/UI debugging
- [ ] Add rollout toggles only if needed, with explicit config/default
  semantics
- [ ] Update/add READMEs for touched directories with architectural
  responsibility, consumer contract, and producer contract sections as required
- [ ] Add/update ADR and operator/developer docs covering registry ownership,
  technical-fit selection, and recovery behavior
- [ ] Add any needed artifact validation or tooling hooks if new persisted
  structured outputs are introduced

**Verification:**
- documentation review against `DOCUMENTATION-STANDARDS.md`
- tooling/traceability review against `TOOLING-STANDARDS.md`
- manual smoke verification for startup, shutdown, recovery, and diagnostics
  visibility

**Status:** Not started

## Current Implementation Guidance

### What must happen next

1. Start Milestone 3 by adding backend-owned admission, warmup, retention, and
   eviction policy on top of the completed runtime-registry foundation.
2. Add warmup/reuse plus retention-hint interpretation inside the registry boundary
   without moving those decisions into gateway or adapter layers.
3. Consume the new backend-owned eviction candidate ordering from registry
   policy rather than rebuilding candidate selection in workflow service or
   adapters.
4. Keep gateway, workflow-service, embedded-runtime, and Tauri adapter roles
   aligned with the README and ADR boundary decisions now reflected in the
   backend-owned registry refactor.
5. Re-plan immediately if Milestone 3 implementation pressures any of the
   frozen ownership decisions, requires a different async ownership model, or
   forces contract changes larger than assumed.

### What should not happen next

- Do not add registry state to `InferenceGateway` directly.
- Do not start admission/eviction policy inside adapters.
- Do not let workflow execution paths mutate long-lived runtime residency state
  ad hoc.
- Do not move budget, retention, or eviction logic back into `src-tauri` now
  that the backend crate boundary has been restored.

## Execution Notes

Update during implementation:

- 2026-03-21: Plan created after comparing Pantograph runtime/workflow
  boundaries with SmarterRouter orchestration patterns and narrowing the scope
  to technical-fit selection plus live runtime ownership in Pantograph.
- 2026-03-21: Plan updated to avoid “best model” language and align all
  recommendation semantics to feasible-candidate input plus host-owned
  technical-fit selection.
- 2026-04-13: Plan updated to reflect reality: prerequisite runtime-contract
  convergence and diagnostics groundwork were already complete in code before
  the runtime-registry implementation wave.
- 2026-04-13: Milestone 1 completed with ADR-002, ADR index creation, and
  runtime-boundary README updates for `src-tauri/src/llm`,
  `src-tauri/src/workflow`, `crates/inference/src`, and
  `crates/pantograph-embedded-runtime/src`.
- 2026-04-13: Milestone 2 completed and standards-corrected by moving the
  runtime-registry state machine out of `src-tauri` into
  `crates/pantograph-runtime-registry`, leaving Tauri as composition and
  observation-translation only.
- 2026-04-13: Milestone 3 reviewed against architecture, concurrency,
  documentation, testing, and plan standards; tasks and verification were
  tightened so the next implementation slice preserves the backend-owned
  registry boundary.
- 2026-04-13: Milestone 3 started with backend-owned reservation lifecycle
  translation in the embedded host and backend-owned admission budget/rejection
  primitives in `crates/pantograph-runtime-registry`.
- 2026-04-13: Milestone 3 live reservation wiring now forwards workflow memory
  estimates into registry requests, and repeated registration no longer clears
  a preconfigured admission budget.
- 2026-04-13: Milestone 3 now also carries typed keep-alive retention intent
  from workflow service into registry reservation records so the next policy
  slice can consume an explicit backend-owned hint instead of adapter-local
  behavior.
- 2026-04-13: Milestone 3 eviction groundwork now includes deterministic
  backend-owned candidate ordering with reserved and pinned runtimes excluded.

## Commit Cadence Notes

- Commit when a logical slice is complete and verified.
- Keep architecture/docs, registry foundation, policy logic, and adapter
  integration in separate reviewable commits where practical.
- Follow `COMMIT-STANDARDS.md`.

## Re-Plan Triggers

- Pumas candidate APIs require Pantograph contract changes larger than assumed
- runtime-registry state needs durable persistence earlier than planned
- eviction/reuse logic cannot be made safe without changing workflow session
  contracts
- file/module growth forces a different decomposition than assumed
- diagnostics surfaces require new binding/API compatibility commitments

## Recommendations

- Prefer the name `RuntimeRegistry` over `BackendRegistry` for the new layer.
  Why: Pantograph already has a backend registry in `crates/inference`; a
  distinct name preserves architectural clarity.
- Deliver runtime ownership and budget policy before candidate-selection
  refinement.
  Why: resource safety and lifecycle correctness are easier to verify than
  selector sophistication.
- Treat the recent runtime unification work as prerequisite infrastructure, not
  as Milestone 2 progress.
  Why: it reduces contract drift, but it does not yet establish single-owner
  runtime residency state.

## Completion Summary

### Completed

- Milestone 0 prerequisite runtime-contract groundwork
- Milestone 1 architecture/documentation freeze for runtime-registry ownership
- Milestone 2 runtime-registry foundation

### In Progress

- Runtime producer-convergence hardening outside the runtime-registry plan
- Milestone 3 admission, warmup, retention, and eviction work

### Not Started

- Milestones 4 through 6 of this plan

### Deviations

- Runtime unification groundwork landed before Milestone 1 documentation freeze.
  This is now recorded explicitly and should not be treated as permission to
  start runtime-registry state/admission code without the boundary work.
- Runtime-registry foundation first landed under `src-tauri` and was then
  refactored into a backend crate to comply with architecture standards. This
  correction is now part of the recorded Milestone 2 outcome.

### Follow-Ups

- Start Milestones 3 through 6 only after preserving the Milestone 2 backend
  ownership boundary
- Keep this plan updated as implementation advances rather than allowing status
  drift to build again

### Verification Summary

- Shared identity helper tests and downstream workflow/diagnostics alias tests
  are green for the prerequisite groundwork
- `cargo test -p pantograph-runtime-registry` is green for the backend-owned
  registry crate
- `cargo check --manifest-path src-tauri/Cargo.toml` is green after replacing
  the Tauri-owned registry implementation with a thin adapter
