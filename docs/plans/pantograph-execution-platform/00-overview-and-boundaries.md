# 00: Overview And Boundaries

## Status

Draft.

Last updated: 2026-04-23.

## Source Documents

- `docs/requirements/pantograph-node-system.md`
- `docs/requirements/pantograph-client-sessions-buckets-model-license-diagnostics.md`
- `DIAGNOSTICS-MODEL-LICENSE-USAGE.md`
- `docs/headless-embedding-api-v1.md`
- `docs/headless-native-bindings.md`
- `docs/plans/pantograph-binding-platform/final-plan.md`
- `/media/jeremy/OrangeCream/Linux Software/repos/owned/developer-tooling/Coding-Standards/`

## Objective

Define Pantograph's next execution platform so node authors can create typed,
observable workflow nodes without manually wiring platform diagnostics,
attribution, model/license tracking, output measurement, or binding metadata.

The target system should combine:

- ComfyUI-level graph authoring ergonomics
- Haystack-style component/socket clarity
- Prefect-style runtime ownership of execution state and observability
- Pantograph-specific Rust contracts, model/license diagnostics, and
  backend-owned binding projections

## Scope

In scope:

- ordered execution-platform planning for attribution, node contracts,
  runtime-managed observability, diagnostics ledger, composition, migration,
  and binding projections
- architecture boundaries that future implementation must preserve
- standards gates that apply across all numbered plan slices

Out of scope:

- source-code implementation
- exact crate names, storage engine selection, or binding generator finalization
- detailed GUI layout or visual design

## Core Decision

Pantograph nodes describe behavior. The Pantograph runtime owns execution
truth.

Normal node authors define:

- a stable node type contract
- typed input and output ports
- execution semantics and required capabilities
- execution logic that consumes typed inputs and returns typed outputs

The runtime injects:

- client/session/bucket/workflow-run/node attribution
- effective contract snapshots
- diagnostics spans and lifecycle events
- cancellation and progress handles
- managed model/runtime/resource/cache capabilities
- model/license usage capture
- output measurement
- lineage and composed-node trace mapping

## Architecture Boundaries

### Canonical Contract Layer

Owns stable backend definitions:

- node type contracts
- port contracts
- port value type semantics
- authoring metadata
- execution semantics
- capability requirements
- compatibility rules
- effective contract resolution inputs and outputs

This layer must test without GUI, UniFFI, Rustler, C#, Python, or Elixir/BEAM
runtimes.

### Attribution Layer

Owns durable caller and run identity:

- clients
- client credentials
- client sessions
- buckets
- workflow runs
- workflow-run attribution

This layer must resolve before node execution so runtime contexts and managed
capabilities never depend on node-authored attribution arguments.

### Runtime Execution Layer

Owns execution truth:

- topological scheduling and node invocation
- creation of `NodeExecutionContext`
- diagnostics span lifecycle
- cancellation and timeout propagation
- managed capability routing
- model/license usage event enrichment
- output measurement
- lineage capture
- composed-node trace projection

### Diagnostics Persistence Layer

Owns durable facts:

- model/license usage events
- output metrics
- usage query projections
- persisted run indexes needed for diagnostics lookup

Trace events may remain transient, but compliance-relevant model/license usage
facts must persist.

### Adapter And Binding Layer

Owns projection only:

- FFI-safe DTOs
- JSON request/response envelopes
- host-language lifecycle wrappers
- host-language smoke and acceptance harnesses
- support-tier-specific surface documentation

Adapters must not own canonical node semantics, compatibility rules,
diagnostics meaning, or model/license attribution policy.

### GUI Layer

Owns presentation:

- palette rendering
- node inspector rendering
- port editor UI
- diagnostics views
- user actions submitted back to backend graph/edit APIs

The GUI renders backend-owned contracts and projections. It must not invent
effective port shape or business semantics.

## Implementation Order

1. Durable attribution.
2. Node contracts and discovery.
3. Runtime-managed execution and observability.
4. Model/license diagnostics ledger.
5. Composition, factoring, and migration.
6. Binding projections and host verification.

Each implementation stage must start by applying
`08-stage-start-implementation-gate.md` and finish by applying
`09-stage-end-refactor-gate.md` before the next numbered stage begins.
If the start gate determines that a stage needs parallel workers, the stage
must first be expanded using `10-concurrent-phased-implementation.md`.

## Tasks

- Maintain this file as the cross-cutting boundary document for the numbered
  execution-platform plans.
- Keep implementation sequencing aligned with durable attribution before
  runtime observability and diagnostics ledger work.
- Update the affected numbered plan before implementation when a boundary,
  support tier, persistence model, or binding assumption changes.
- Convert finalized architecture decisions into ADRs when they become stable
  enough to outlive the planning phase.
- Apply the stage-start implementation gate before editing source files for
  each numbered stage and record the start outcome.
- If concurrent implementation is warranted, create the stage-specific wave
  specs, report paths, and coordination ledger required by
  `10-concurrent-phased-implementation.md` before launching workers.
- Apply the stage-end refactor gate after each implementation stage and record
  whether no refactor was needed, an in-scope touched-file refactor was
  completed, or broader refactor pressure requires a separate plan.

## Standards Gates

- Files over 500 lines require decomposition review.
- New source directories require README coverage where standards require it.
- Core contract and runtime logic must test without binding frameworks.
- Supported bindings require native-language and host-language verification.
- Interop boundaries must validate inputs and preserve wire-format alignment.
- GUI state must render backend-owned facts and avoid optimistic mutation of
  backend-owned graph state.

## Affected Structured Contracts And Persisted Artifacts

- Node, port, effective contract, diagnostics, usage, and attribution DTOs are
  structured contracts and must have boundary validation plus native round-trip
  tests before being treated as stable.
- Persisted client/session/bucket/run records, model/license usage events, run
  indexes, saved workflow graphs, and migration artifacts must document
  versioning, retention, pruning, and migration behavior before implementation
  is complete.
- Generated binding artifacts must remain projections over backend-owned
  contracts and must be version-matched to the product-native library.

## Standards Compliance Iteration

- Planning: each numbered file must state purpose, scope, tasks, verification,
  risks, completion criteria, and re-plan triggers when its assumptions can
  break during implementation.
- Architecture and coding: core semantics belong in Rust contract/runtime
  crates, adapters and bindings project those semantics, and GUI code renders
  backend-owned facts.
- Rust API: public ids, lifecycle states, guarantee levels, and compatibility
  decisions should use validated newtypes or enums instead of raw strings or
  booleans where the bug cost crosses a crate or binding boundary.
- Async and concurrency: runtime creation belongs in composition roots;
  spawned work, cancellation, progress, retries, and shutdown must have explicit
  lifecycle owners.
- Interop and bindings: FFI wrappers are thin, unsafe is isolated, foreign
  buffers are copied immediately, wire DTOs use explicit serialization shape,
  and generated host bindings are not hand-edited.
- Security and dependencies: external paths, ids, payload sizes, queues, and
  listener surfaces are validated at ingress; new dependencies require owner,
  feature, transitive-cost, audit, and release-artifact review.
- Tooling, release, and cross-platform: implementation work must define local
  and CI verification, supported targets, artifact names, checksum/SBOM
  expectations, and changelog or migration notes for user-visible changes.
- Frontend and accessibility: any GUI changes must use semantic interactive
  elements, keyboard-accessible controls, backend-driven state, and accessible
  diagnostics views.

## Risks And Mitigations

- Risk: the plan accidentally moves canonical semantics into bindings or GUI.
  Mitigation: treat backend contracts as the only source of truth and reject
  host-local catalogs for supported surfaces.
- Risk: observability becomes incomplete because nodes bypass managed
  capabilities. Mitigation: classify guarantee levels and make reduced
  guarantees queryable instead of presenting incomplete records as complete.
- Risk: persisted diagnostics are added without migration and retention policy.
  Mitigation: ledger work is not complete until versioning, pruning, replay,
  and migration rules are documented and tested.

## Verification

- Every numbered plan is readable independently and in numeric order.
- Each implementation slice has affected contracts/artifacts, standards notes,
  risks, verification, completion criteria, and re-plan triggers.
- No plan file exceeds the 500-line decomposition threshold.
- Future implementation can trace each source-code compliance gate back to this
  overview, a numbered slice, or `07-standards-compliance-review.md`.
- Every implementation stage records the outcome of
  `08-stage-start-implementation-gate.md` before source edits begin.
- Any stage implemented with parallel workers has explicit wave specs,
  non-overlapping write sets, worker report paths, and a coordination ledger.
- Every completed implementation stage records the outcome of
  `09-stage-end-refactor-gate.md` before the next stage starts.

## Re-Plan Triggers

- Effective contracts require frontend reconstruction.
- Managed capabilities cannot observe model execution without invasive node
  boilerplate.
- Binding projections require host-local node catalogs.
- Diagnostics persistence forces a different storage architecture than assumed.
- Existing nodes cannot migrate without breaking persisted workflows.
- Escape hatches become common enough to weaken the normal guarantee model.

## Completion Criteria

- Every numbered plan has a standards compliance section or delegates to
  `07-standards-compliance-review.md` for the cross-plan matrix.
- Future implementation can be reviewed against explicit architecture, Rust,
  async, interop, security, dependency, tooling, release, frontend, and
  accessibility gates without inferring hidden requirements.
- The codebase enters each implementation stage only after plan readiness,
  worktree hygiene, write-set, verification, and commit-boundary checks pass.
- The codebase enters each next implementation stage with the files touched by
  the previous stage either standards-compliant or explicitly tracked in a
  separate refactor plan.
