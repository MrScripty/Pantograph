# Plan: Pantograph Milestone 4 Technical-Fit Selection

## Status
Active

Last updated: 2026-04-15

## Current Source-of-Truth Summary

This document is the dedicated source of truth for runtime-registry Milestone 4.
It replaces the short Milestone 4 stub in
`IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
for detailed sequencing, standards compliance, and immediate refactor scope.

## Objective

Add a backend-owned technical-fit selection path that consumes Pumas feasible
execution candidates plus live Pantograph runtime state, preserves explicit
manual overrides, and stays compliant with Pantograph's architecture, coding,
documentation, testing, concurrency, tooling, and security standards.

## Scope

### In Scope

- Backend-owned technical-fit selection contracts and policy in Rust
- Candidate-consumption and decision-reason DTOs used by runtime-registry,
  workflow service, and embedded runtime
- Deterministic override precedence for explicit `model_id` and `backend_key`
  selections
- Conservative fallback behavior when candidate data or live runtime state is
  partial
- Refactors required to keep the immediate insertion points standards-compliant
  before adding more Milestone 4 logic
- Cross-layer verification from candidate input through workflow preflight and
  workflow execution output
- README, ADR, and source-of-truth updates required by touched boundaries

### Out of Scope

- Answer-quality, prompt-semantic, or benchmark-based model routing
- Moving technical-fit policy into Tauri, UniFFI, Rustler, or frontend code
- Replacing `InferenceGateway`, producer hosts, or Pumas candidate generation
- Scheduler V2 queue policy beyond the runtime-pressure facts Milestone 4 must
  consume as input
- Persistent storage for technical-fit decisions unless a later milestone
  explicitly requires it

## Inputs

### Problem

Pantograph now has a backend-owned runtime registry plus stronger runtime and
diagnostics boundaries, but Milestone 4 still lacks a dedicated technical-fit
selection plan. The current codebase can report runtime capabilities and can
validate runtime availability, yet it still does not have one backend-owned
policy that:

- consumes Pumas feasible candidates
- combines those candidates with live runtime-registry facts
- preserves explicit `model_id` and `backend_key` overrides deterministically
- returns a machine-consumable decision reason
- feeds the same decision semantics into both preflight and execution paths

Without a dedicated plan, implementation would likely spread more logic into
already oversized files or drift into adapters, which would violate the coding
and architecture standards that now govern this area.

### Constraints

- Business logic stays in backend Rust crates, not Tauri or frontend code.
- `crates/pantograph-runtime-registry` is the correct owner for technical-fit
  policy; `crates/pantograph-workflow-service` stays host-agnostic and
  `crates/pantograph-embedded-runtime` stays a producer/integration layer.
- External input such as run requests, override fields, and any debug filters
  must be validated once at the boundary and canonicalized before entering the
  selector.
- Public workflow and runtime facades should remain additive unless an explicit
  API break is approved.
- Machine-consumed decision contracts must avoid answer-quality claims and use
  verifiable runtime facts only.
- Oversized files in the immediate insertion area require decomposition review
  and refactor before absorbing additional Milestone 4 logic.
- The implementation must remain aligned with ADR-002 and the directory README
  ownership statements already landed in Milestone 1.

### Assumptions

- Pumas feasible candidates remain an upstream additive contract that
  Pantograph consumes rather than re-derives.
- Runtime-registry admission, residency, retention, and eviction work from
  Milestone 3 remains the live-state foundation Milestone 4 will consult.
- Manual `model_id` and `backend_key` choices remain supported and must keep
  deterministic precedence over automatic selection.
- A snapshot-based selection path is acceptable; technical-fit does not need to
  hold long-lived mutable access to registry state while computing a decision.
- Initial technical-fit diagnostics can remain in-memory and backend-owned.

### Dependencies

- `crates/pantograph-runtime-registry`
- `crates/pantograph-workflow-service`
- `crates/pantograph-embedded-runtime`
- `crates/inference`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- `IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
- Pumas feasible-candidate APIs and the existing Pantograph model descriptor
  resolution path

### Affected Structured Contracts

- New backend-owned technical-fit request, candidate, decision, and reason DTOs
- Workflow-service request normalization and preflight/execution selection
  contracts
- Embedded-runtime host-facing candidate ingestion and runtime-state projection
  contracts
- Optional diagnostics/debug payloads if additive decision details are surfaced
  through existing backend-owned inspection endpoints

### Affected Persisted Artifacts

- None required for the first Milestone 4 implementation slice
- If checked-in JSON fixtures or debug snapshot examples are added, they become
  structured artifacts and must be validated with repo tooling per
  `TOOLING-STANDARDS.md`
- README/ADR plan and roadmap documents touched to keep source-of-truth status
  aligned with implementation reality

### Existing Codebase Non-Compliance In Immediate Surroundings

The immediate insertion points already exceed the coding standards' soft
thresholds and must not absorb new policy without refactor:

- `crates/pantograph-embedded-runtime/src/lib.rs` is approximately 4354 lines
- `crates/pantograph-workflow-service/src/workflow.rs` is approximately 6017
  lines
- `crates/pantograph-runtime-registry/src/lib.rs` is approximately 2337 lines

Milestone 4 therefore includes explicit extraction work so technical-fit logic
lands in focused modules rather than deepening existing catch-all files.

### Concurrency / Race-Risk Review

- Technical-fit selection reads live runtime facts that can change while queue
  admission, warmup, stop, recovery, and reclaim are happening.
- The selector should operate on immutable, backend-owned snapshot input rather
  than iterating over mutable registry state across `.await` points.
- Related runtime facts used for selection must be captured under one lock or
  one serialized registry snapshot path so scoring does not observe torn state.
- Workflow execution must treat the selector output as advisory until normal
  admission/reservation checks succeed; selection cannot bypass registry-owned
  safety checks.
- Retry, replay, restart, and recovery paths must not duplicate or contradict
  the recorded technical-fit decision reason for the same run attempt.
- Any concurrent debug/inspection reads must consume stable snapshot DTOs,
  never partially assembled adapter-local state.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Technical-fit logic leaks into Tauri or other adapters | High | Keep selection contracts and policy in backend Rust crates; adapters forward only validated payloads and backend results |
| Large files absorb even more responsibility during Milestone 4 | High | Make decomposition/extraction a required milestone before adding major selection logic |
| Workflow-service becomes a hidden runtime-policy owner | High | Limit workflow-service to request normalization, host-agnostic orchestration, and backend trait calls |
| Embedded runtime becomes a duplicate selector implementation | High | Keep embedded-runtime responsible for candidate collection and runtime projection only; call into registry-owned policy |
| Candidate data is partial, stale, or missing | High | Add conservative fallback contracts and tests for partial-data selection paths |
| Selection decisions drift between preflight and execution | High | Reuse one backend-owned decision contract in both paths and cover with acceptance tests |
| Runtime-state reads race with registry transitions | Medium | Use snapshot-based selection inputs and verify behavior under overlapping transition scenarios |
| Debug payloads drift from actual selector behavior | Medium | Build diagnostics from backend-owned decision DTOs instead of adapter-local reconstruction |
| New fixtures or docs drift from actual contracts | Medium | Add documentation/tooling review and structured-artifact validation if fixtures are introduced |

## Standards Review Passes

### Pass 1: `PLAN-STANDARDS.md` And `templates/PLAN-TEMPLATE.md`

Corrections applied to the draft:

- Added explicit objective, scope, dependencies, risks, definition of done,
  milestone verification, re-plan triggers, and completion-summary placeholders.
- Recorded affected structured contracts, affected persisted artifacts, and a
  dedicated concurrency review because Milestone 4 crosses long-lived runtime
  state plus workflow execution.
- Recorded this file as the dedicated Milestone 4 source of truth rather than
  leaving sequencing buried in the umbrella runtime-registry plan.

### Pass 2: `ARCHITECTURE-PATTERNS.md` And `CODING-STANDARDS.md`

Corrections applied to the draft:

- Fixed ownership so technical-fit policy lives in
  `crates/pantograph-runtime-registry`, not `src-tauri`, frontend code, or the
  workflow-service orchestration layer.
- Added explicit refactor milestones for the oversized immediate insertion
  files instead of planning to append more policy into them.
- Required public-facade preservation and additive contract changes unless an
  approved API break is recorded.
- Preserved package roles: workflow-service stays host-agnostic, embedded
  runtime stays producer/integration focused, adapters stay transport wrappers.

### Pass 3: `TESTING-STANDARDS.md` And `CONCURRENCY-STANDARDS.md`

Corrections applied to the draft:

- Added unit, integration, and cross-layer acceptance checks from candidate
  input through preflight/execution output.
- Added replay, restart, recovery, and idempotency verification because the
  selector consults live runtime state that can overlap with lifecycle events.
- Required snapshot-based selection and explicit lock/`.await` hygiene so the
  selector does not hold mutable runtime state across async boundaries.
- Added deterministic tie-break verification so concurrency or ordering changes
  cannot silently alter selection semantics.

### Pass 4: `DOCUMENTATION-STANDARDS.md` And `TOOLING-STANDARDS.md`

Corrections applied to the draft:

- Added README and ADR update requirements for any new source directories or
  architectural boundary changes created by Milestone 4.
- Added source-of-truth update tasks so roadmap and umbrella-plan wording stay
  synchronized with the dedicated Milestone 4 plan.
- Added validation expectations for any new structured fixtures, snapshots, or
  checked-in artifacts introduced during implementation.

### Pass 5: `SECURITY-STANDARDS.md`

Corrections applied to the draft:

- Added a validate-once boundary rule for external override fields,
  candidate-request payloads, and any future debug filters.
- Required canonicalization before the selector runs so internal technical-fit
  logic works on trusted backend-owned DTOs instead of repeatedly validating raw
  payloads.
- Explicitly forbade adapter-local duplicate validation or selector-local
  payload parsing as an alternative policy path.

## Definition of Done

- Pantograph has one backend-owned technical-fit selector that consumes Pumas
  feasible candidates plus live runtime-registry facts.
- Explicit `model_id` and `backend_key` overrides are preserved with documented,
  tested precedence.
- Workflow preflight and workflow execution use the same backend-owned decision
  semantics rather than diverging on selection behavior.
- Immediate insertion-point refactors have reduced Milestone 4 logic pressure on
  oversized files and left new code in focused modules with clear ownership.
- No core technical-fit logic lives in `src-tauri` or frontend code.
- Decision reasons are machine-consumable, deterministic, and limited to
  verifiable technical/runtime facts.
- README, ADR, and plan/roadmap updates required by the touched boundaries are
  complete.
- Cross-layer acceptance coverage proves the full path from candidate input to
  workflow result remains compliant and deterministic.

## Milestones

### Milestone 1: Freeze Technical-Fit Contracts And Boundaries

**Goal:** Define the backend-owned contracts and ownership boundaries before the
selector spreads across runtime and workflow code.

**Tasks:**
- [x] Define registry-owned technical-fit request, candidate, decision, and
      decision-reason DTOs with additive serde semantics.
- [x] Define the override contract for explicit `model_id` and `backend_key`
      selections, including deterministic precedence and fallback semantics.
- [x] Define which technical factors are legal selector inputs and explicitly
      exclude answer-quality or prompt-semantic scoring.
- [x] Define the workflow-service to embedded-runtime to runtime-registry flow:
      workflow-service normalizes request intent, embedded-runtime gathers
      feasible candidates and live runtime facts, runtime-registry owns the
      selection policy.
- [x] Record any Tauri or binding surfaces that consume the resulting decision
      as transport-only wrappers.
- [x] Update the umbrella runtime-registry plan so this dedicated file is the
      explicit Milestone 4 source of truth.

**Verification:**
- Architecture review against ADR-002 and `ARCHITECTURE-PATTERNS.md`
- Contract review against `PLAN-STANDARDS.md` and append-only contract rules
- Serialization/DTO tests for any new machine-consumed technical-fit structs
- Review confirms no answer-quality claims enter API/docs/decision reasons

**Status:** Completed

### Milestone 2: Refactor Immediate Insertion Points To Compliance

**Goal:** Create standards-compliant module boundaries before major technical-
fit logic lands.

**Tasks:**
- [x] Extract technical-fit contracts and selector implementation out of
      `crates/pantograph-runtime-registry/src/lib.rs` into focused module files
      so the registry facade remains composition-oriented.
- [x] Extract workflow-service selection-request normalization,
      override-precedence helpers, and related preflight/execution glue out of
      `crates/pantograph-workflow-service/src/workflow.rs` into focused modules.
- [x] Extract embedded-runtime candidate gathering and runtime-state projection
      helpers out of `crates/pantograph-embedded-runtime/src/lib.rs` into the
      appropriate focused runtime modules.
- [x] Add `README.md` coverage for any new `src/` directories created during the
      extraction, and update existing README contents if responsibilities shift.
- [x] Preserve public facade behavior while moving implementation into smaller,
      ownership-aligned modules.

**Verification:**
- Decomposition review against `CODING-STANDARDS.md`
- `cargo check -p pantograph-runtime-registry`
- `cargo check -p pantograph-workflow-service`
- `cargo check -p pantograph-embedded-runtime`
- Review confirms no core technical-fit policy migrated into Tauri adapters

**Status:** Completed

### Milestone 3: Implement Registry-Owned Technical-Fit Policy

**Goal:** Build the deterministic backend selector on top of frozen contracts and
refactored module boundaries.

**Tasks:**
- [x] Normalize candidate and override inputs at the boundary and convert them
      into trusted backend-owned technical-fit DTOs.
- [ ] Implement deterministic selection using only approved technical factors:
      required context length, task/runtime requirements, current residency and
      reuse value, warmup cost, budget pressure, and workflow/session queue
      pressure.
- [ ] Implement deterministic tie-breaking and stable reason-code generation.
- [ ] Implement conservative fallback behavior when candidate data or runtime
      state is partial, stale, or unavailable.
- [ ] Ensure selection consumes immutable runtime-registry snapshot input rather
      than holding mutable registry state across async work.
- [ ] Keep the selector reusable by both preflight and execution paths.

**Verification:**
- `cargo test -p pantograph-runtime-registry`
- Focused unit tests for factor ordering, tie-breaking, override precedence,
  partial-data handling, and conservative fallback behavior
- Concurrency review against `CONCURRENCY-STANDARDS.md`
- Contract review confirms reason payloads remain machine-consumable and
  verifiable

**Status:** In progress

### Milestone 4: Integrate Workflow Service And Embedded Runtime

**Goal:** Feed backend-owned technical-fit decisions through workflow preflight
and execution without turning orchestration or adapters into policy owners.

**Tasks:**
- [ ] Add workflow-service request-normalization paths that derive one
      backend-owned technical-fit request for both preflight and run/session
      execution flows.
- [ ] Preserve explicit `model_id` and `backend_key` overrides with the frozen,
      deterministic precedence contract.
- [ ] Add embedded-runtime candidate collection and live-runtime snapshot
      projection that call into the registry selector instead of recomputing
      policy locally.
- [ ] Ensure workflow preflight and runtime-not-ready reporting surface the same
      selector decision/reason semantics as execution where applicable.
- [ ] Keep any Tauri or binding integration limited to validated input parsing
      and transport of backend-owned decision DTOs.

**Verification:**
- `cargo test -p pantograph-workflow-service`
- `cargo test -p pantograph-embedded-runtime`
- Cross-layer acceptance check from candidate input through selection,
  admission, workflow execution, and output/result reporting
- Review confirms workflow-service remains host-agnostic and Tauri remains an
  adapter only

**Status:** Not started

### Milestone 5: Documentation, Diagnostics, And Hardening

**Goal:** Keep the new selector observable, reviewable, and standards-aligned
once wired through the runtime path.

**Tasks:**
- [ ] Expose additive technical-fit diagnostics only through backend-owned
      inspection surfaces where the decision is already machine-consumed.
- [ ] Add replay, retry, recovery, and idempotency verification where registry
      state and technical-fit decisions can overlap.
- [ ] Update `README.md` files for touched directories and update ADRs only if
      the architectural ownership boundary changes.
- [ ] Update the umbrella runtime-registry plan and roadmap status wording so
      Milestone 4 progress remains accurate.
- [ ] Validate any new checked-in fixtures or machine-consumed debug snapshots
      with the repo tooling expected by `TOOLING-STANDARDS.md`.

**Verification:**
- Documentation review against `DOCUMENTATION-STANDARDS.md`
- Tooling review against `TOOLING-STANDARDS.md`
- Recovery/idempotency checks required by `TESTING-STANDARDS.md`
- Final compile/test pass for all touched crates and adapter boundaries

**Status:** Not started

## Execution Notes

Update during implementation:
- 2026-04-15: Drafted the dedicated Milestone 4 plan from the existing runtime-
  registry umbrella plan plus ADR/README ownership constraints.
- 2026-04-15: Performed five explicit standards passes covering plan shape,
  architecture/coding, testing/concurrency, documentation/tooling, and
  security.
- 2026-04-15: Recorded immediate refactor work because the current insertion
  files already exceed the coding standards' decomposition thresholds.
- 2026-04-16: Added backend-owned technical-fit request, candidate, override,
  factor, and decision contracts in `pantograph-runtime-registry` so later
  workflow integration can share one selector vocabulary without routing logic
  leaking into adapters.
- 2026-04-16: Added host-agnostic workflow technical-fit request and decision
  contracts plus service-owned normalization helpers so workflow and session
  context can be projected into the runtime layer without moving selector
  policy into `pantograph-workflow-service`.
- 2026-04-16: Added the embedded-runtime technical-fit bridge that converts
  workflow-service request context into runtime-registry selector input and
  projects backend selector decisions back into workflow-service DTOs without
  moving policy into Tauri or workflow adapters.
- 2026-04-16: Moved the workflow-service technical-fit request/session entry
  points into `technical_fit.rs`, leaving `workflow.rs` focused on core service
  orchestration while preserving the public service API.
- 2026-04-16: Moved workflow-service technical-fit session context and queue
  pressure assembly into `technical_fit.rs`, closing the remaining service-side
  Milestone 2 extraction still living in `workflow.rs`.
- 2026-04-16: Moved the embedded-runtime host-side technical-fit entrypoint and
  runtime snapshot/candidate assembly into `technical_fit.rs`, leaving
  `lib.rs` as a thinner workflow-host facade while preserving the public host
  behavior.
- 2026-04-16: Added the first runtime-registry selector entrypoint in
  `technical_fit.rs`, which now normalizes request input, preserves explicit
  override precedence, enriches candidate ranking with runtime-snapshot
  residency/warmup facts, and emits conservative fallback decisions when the
  registry lacks enough data for a stronger automatic selection.


## Commit Cadence Notes

- Commit when each Milestone 4 slice is complete and verified.
- Keep refactor commits separate from new selector-behavior commits whenever
  that separation preserves a reviewable history.
- Follow `COMMIT-STANDARDS.md` for detailed, atomic commit messages.

## Re-Plan Triggers

- Pumas feasible-candidate APIs change shape in a way that alters Pantograph's
  candidate-consumption contract.
- Milestone 3 runtime-registry work changes admission or residency semantics in
  a way that invalidates frozen technical-fit inputs.
- Workflow-service or embedded-runtime refactors reveal an additional boundary
  violation in the immediate insertion path.
- Milestone 4 requires a new persisted artifact, rollout toggle, or public API
  break not covered by this plan.
- The selector cannot remain backend-owned without revisiting ADR-002.

## Completion Summary

### Completed

- Dedicated Milestone 4 plan created.
- Standards review passes recorded against the active coding standards.

### Deviations

- None yet.

### Follow-Ups

- Implement Milestone 1 of this dedicated plan before any further Milestone 4
  feature work lands.
- Keep the umbrella runtime-registry plan and roadmap in sync as Milestone 4
  implementation progresses.

### Verification Summary

- Standards review completed against `PLAN-STANDARDS.md`,
  `ARCHITECTURE-PATTERNS.md`, `CODING-STANDARDS.md`,
  `TESTING-STANDARDS.md`, `CONCURRENCY-STANDARDS.md`,
  `DOCUMENTATION-STANDARDS.md`, `TOOLING-STANDARDS.md`, and
  `SECURITY-STANDARDS.md`.
- Repository inspection confirmed oversized immediate insertion points in
  `pantograph-runtime-registry`, `pantograph-workflow-service`, and
  `pantograph-embedded-runtime` that require planned extraction.

### Traceability Links

- Dedicated plan: `IMPLEMENTATION-PLAN-pantograph-milestone-4-technical-fit-selection.md`
- Umbrella plan: `IMPLEMENTATION-PLAN-pantograph-runtime-registry-technical-fit-selection.md`
- Roadmap: `ROADMAP-pantograph-workflow-graph-scheduling-runtime.md`
- ADR: `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
