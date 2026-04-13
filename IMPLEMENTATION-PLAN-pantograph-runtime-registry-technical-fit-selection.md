# Plan: Pantograph Runtime Registry And Technical-Fit Selection

## Status
Active

Last updated: 2026-04-13

## Current Source-of-Truth Summary

This plan is no longer "planning only," but the core runtime-registry work has
also not started yet.

The accurate status is:

- prerequisite runtime-contract and diagnostics groundwork: completed
- runtime-registry milestones 1-6: not started
- current next milestone: Milestone 1, Freeze Boundaries And Ownership
- stop rule: do not begin runtime-registry state/admission implementation until
  Milestone 1 docs and ownership boundaries are landed

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

### What has not landed yet

- no Pantograph-owned `RuntimeRegistry` exists yet
- no registry-owned reservation/admission state machine exists yet
- no registry-driven warmup, retention, or eviction policy exists yet
- no Pumas-driven technical-fit selector is integrated into workflow execution
- no milestone-1 documentation/ADR boundary freeze has been committed yet

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
- keep runtime lifecycle ownership in one Pantograph module
- use technical-fit factors grounded in verifiable facts only
- follow directory README, ADR traceability, testing, and concurrency
  standards
- avoid creating oversized catch-all modules; perform decomposition review
  before crossing file-size thresholds

### Assumptions

- Pumas will expose ranked feasible execution candidates from durable facts
- Pantograph remains the owner of live runtime admission, reuse, and eviction
  policy
- milestone one can use conservative estimated RAM/VRAM data with explicit
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

- none required in milestone one
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

**Tasks:**
- [ ] Define the `RuntimeRegistry` responsibility boundary relative to
  `InferenceGateway`, workflow service, and Tauri host wiring
- [ ] Record public facade preservation: `InferenceGateway` remains the
  execution facade; the registry becomes an upper-layer coordinator
- [ ] Define the registry state machine, reservation contract, and explicit
  ownership of background tasks, health checks, and cleanup
- [ ] Add/update architecture documentation and ADR coverage for the new
  boundary
- [ ] Identify touched directories that require README updates under
  documentation standards

**Verification:**
- architecture/doc review against `CODING-STANDARDS.md`,
  `ARCHITECTURE-PATTERNS.md`, and `DOCUMENTATION-STANDARDS.md`
- review confirms no implementation starts before ownership/lifecycle is
  explicit

**Status:** Next

### Milestone 2: Runtime Registry Foundation

**Goal:** Introduce the live runtime state layer without breaking current
runtime callers.

**Tasks:**
- [ ] Add focused runtime-registry modules for runtime records, model
  residency, reservation/admission decisions, and error/reporting types
- [ ] Keep the composition root near app startup; do not create ad hoc global
  infrastructure inside feature modules
- [ ] Track runtime states such as stopped, warming, ready, busy, unhealthy,
  stopping, and failed
- [ ] Add deterministic state-transition rules for start, connect-external,
  stop, recovery, and stale cleanup
- [ ] Perform decomposition review on any touched files approaching
  file-size/responsibility thresholds

**Verification:**
- `cargo check --workspace`
- unit tests for state transitions, reservation lifecycle, and cleanup paths
- compile review confirms existing gateway callers still depend on preserved
  facades

**Status:** Not started

### Milestone 3: Admission, Warmup, Retention, And Eviction

**Goal:** Make runtime placement budget-aware and concurrency-safe.

**Tasks:**
- [ ] Add admission checks using estimated RAM/VRAM with explicit safety margins
  and failure reasons
- [ ] Add warmup/reuse paths for session create/run and explicit release paths
  after execution
- [ ] Extend `keep_alive` into a retention hint interpreted by registry policy
  rather than raw direct ownership
- [ ] Implement eviction v1 with active/reserved/pinned exclusion and
  deterministic candidate ordering
- [ ] Ensure no async path holds a lock across blocking work or long-running
  awaits contrary to concurrency standards

**Verification:**
- unit tests for admission acceptance/rejection, warmup reuse, and eviction
  ordering
- integration tests for overlapping session runs, restart/recovery, and
  keep-alive transitions
- concurrency review against `CONCURRENCY-STANDARDS.md`

**Status:** Not started

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

1. Update ADR/README/architecture documentation for runtime-registry ownership
   before coding the registry itself.
2. Record the `RuntimeRegistry` boundary and composition-root ownership.
3. Start Milestone 2 only after Milestone 1 is committed.

### What should not happen next

- Do not add registry state to `InferenceGateway` directly.
- Do not start admission/eviction policy inside adapters.
- Do not let workflow execution paths mutate long-lived runtime residency state
  ad hoc.

## Execution Notes

Update during implementation:

- 2026-03-21: Plan created after comparing Pantograph runtime/workflow
  boundaries with SmarterRouter orchestration patterns and narrowing the scope
  to technical-fit selection plus live runtime ownership in Pantograph.
- 2026-03-21: Plan updated to avoid “best model” language and align all
  recommendation semantics to feasible-candidate input plus host-owned
  technical-fit selection.
- 2026-04-13: Plan updated to reflect reality: runtime-registry implementation
  has not started, but prerequisite runtime-contract convergence and
  diagnostics groundwork is already complete in code.

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

### In Progress

- Planning/documentation catch-up so the repo matches the implementation state
- Runtime producer-convergence hardening outside the runtime-registry plan

### Not Started

- Milestones 1 through 6 of this plan

### Deviations

- Runtime unification groundwork landed before Milestone 1 documentation freeze.
  This is now recorded explicitly and should not be treated as permission to
  start runtime-registry state/admission code without the boundary work.

### Follow-Ups

- Land Milestone 1 docs/ADR/README updates
- Start runtime-registry foundation only after Milestone 1 is committed
- Keep this plan updated as implementation advances rather than allowing status
  drift to build again

### Verification Summary

- Shared identity helper tests and downstream workflow/diagnostics alias tests
  are green for the prerequisite groundwork
- Runtime-registry implementation verification has not started because the
  runtime registry does not exist yet
