# 07: Standards Compliance Review

## Purpose

Verify that the execution-platform plans conform to the planning standards and
that implementation following these plans should produce code compliant with
the repository standards.

## Scope

In scope:

- standards review for the numbered execution-platform plans
- implementation compliance gates implied by the standards
- residual risks that need later implementation plans or ADRs

Out of scope:

- source-code implementation
- replacing the authoritative standards files
- choosing exact crates, storage engines, or release automation tools

## Reviewed Standards

- Planning: `PLAN-STANDARDS.md`
- Architecture and coding: `ARCHITECTURE-PATTERNS.md`,
  `CODING-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`
- Runtime quality: `TESTING-STANDARDS.md`, `CONCURRENCY-STANDARDS.md`,
  `TOOLING-STANDARDS.md`
- Boundaries: `INTEROP-STANDARDS.md`, `LANGUAGE-BINDINGS-STANDARDS.md`,
  `CROSS-PLATFORM-STANDARDS.md`, `SECURITY-STANDARDS.md`,
  `DEPENDENCY-STANDARDS.md`
- Product surfaces: `FRONTEND-STANDARDS.md`, `ACCESSIBILITY-STANDARDS.md`,
  `LAUNCHER-STANDARDS.md`, `RELEASE-STANDARDS.md`,
  `COMMIT-STANDARDS.md`
- Rust specialization: `languages/rust/RUST-*.md`

## Tasks

- Review each numbered plan against the planning standard.
- Review each numbered plan against the implementation standards that would
  govern future Rust, FFI, binding, persistence, frontend, tooling, and release
  work.
- Record gates that future implementation must satisfy before a slice can be
  considered complete.
- Update this review when standards change or when the execution-platform plan
  set gains new numbered files.

## Cross-Plan Compliance Matrix

| Plan | Standards Focus | Required Gate |
| ---- | --------------- | ------------- |
| `00-overview-and-boundaries.md` | Planning, architecture, documentation, release scope | Preserve backend-owned semantics, explicit boundaries, risks, completion criteria, and re-plan triggers. |
| `01-client-session-bucket-run-attribution.md` | Rust API, security, persistence, concurrency, testing | Use validated ids and typed state, single lifecycle owner for session races, durable attribution before execution, recovery tests. |
| `02-node-contracts-and-discovery.md` | Rust API, architecture, frontend, accessibility, testing | Keep canonical contracts in backend Rust, publish effective contracts, reject host-local semantics, verify graph compatibility and GUI projection behavior. |
| `03-managed-runtime-observability.md` | Rust async, concurrency, observability, testing | Runtime owns spans, cancellation, progress, task lifecycle, and guarantee classification without node boilerplate. |
| `04-model-license-diagnostics-ledger.md` | Persistence, security, dependency, release, testing | Persist time-of-use license snapshots, typed measurements, retention policy, indexed queries, replay/recovery tests. |
| `05-composition-factoring-and-migration.md` | Architecture, documentation, release, testing | Preserve primitive trace facts, model/license attribution, stable ids, compatibility projections, and explicit migrations. |
| `06-binding-projections-and-verification.md` | Interop, language bindings, Rust unsafe, cross-platform, release | Keep three-layer binding architecture, isolate unsafe, version-match generated bindings and native artifacts, verify real host lanes. |
| `08-stage-start-implementation-gate.md` | Planning, worktree hygiene, commits, verification, concurrent worker readiness | Confirm plan readiness, standards context, dirty-file safety, write boundaries, verification, and commit expectations before source edits begin. |
| `09-stage-end-refactor-gate.md` | Planning, coding, testing, tooling, documentation | Decide whether touched files need a standards refactor before the next stage starts, and constrain any refactor to files touched by that stage. |
| `10-concurrent-phased-implementation.md` | Concurrent worker planning, implementation waves, reporting, coordination | Require explicit wave specs, non-overlapping write sets, report files, coordination ledger, one-wave-at-a-time execution, and one-at-a-time integration when parallel work is warranted. |

## Implementation Compliance Gates

- Planning gate: do not start a slice until the owning file has tasks,
  verification, risks, affected contracts/artifacts, completion criteria, and
  re-plan triggers.
- Architecture gate: canonical node, execution, attribution, diagnostics, and
  compatibility semantics must live in backend Rust crates, not GUI or host
  binding packages.
- Rust API gate: public cross-crate and cross-language contracts must use
  validated newtypes, enums, structured errors, `Result` for recoverable
  failures, and documented feature contracts.
- Async/concurrency gate: runtime creation belongs in composition roots;
  spawned work must have tracked handles, cancellation, shutdown, and panic
  handling at the lifecycle owner.
- Interop gate: FFI inputs are untrusted, unsafe is isolated, callback
  threading/lifetime rules are documented, foreign buffers are copied, and
  serialization shape is tested across boundaries.
- Binding gate: generated host bindings are artifacts, not handwritten
  semantics; supported surfaces require native and host-language verification.
- Persistence gate: durable attribution, usage ledger, saved workflow, and
  migration artifacts require versioning, retention or migration behavior, and
  restart/replay tests.
- Security gate: credentials, paths, ids, payload sizes, queue limits, and
  listener exposure are validated at ingress and represented internally as
  trusted domain types.
- Dependency gate: new crates or host tooling require owner, transitive-cost,
  feature, audit, and release-artifact review.
- Tooling gate: implementation PRs must define or use canonical formatter,
  lint, typecheck, test, feature, audit, and artifact validation commands.
- Stage-start implementation gate: before editing source files, read the stage
  plan and applicable standards, inspect git status, identify write
  boundaries, resolve overlapping dirty files, and record the start outcome.
- Concurrent implementation gate: parallel workers require stage-specific wave
  specs, disjoint write sets, report paths, integration order, and a
  coordination ledger before worker prompts are issued.
- Stage-end refactor gate: after each implementation stage, inspect only files
  touched during that stage for standards drift and either record no refactor
  needed, execute an in-scope touched-file refactor, or create a separate
  refactor plan for issues that exceed the touched-file boundary.
- Frontend/accessibility gate: GUI work renders backend-owned facts, avoids
  optimistic mutation of backend-owned graph state, and uses semantic,
  keyboard-accessible controls.
- Release gate: public or binding-facing changes require changelog or migration
  notes, explicit artifact naming, checksums, SBOM expectations where released,
  and version-matched native/binding packages.

## Per-Plan Findings

- `00`: now includes cross-cutting standards gates, affected structured
  contracts, risks, and completion criteria.
- `01`: now includes durable artifact scope, credential/security notes,
  reconnect/takeover concurrency constraints, and recovery verification.
- `02`: now includes canonical contract ownership, host-local semantics
  rejection, discovery DTO documentation, and graph compatibility tests.
- `03`: now includes runtime task ownership, cancellation, progress, guarantee
  classification, and shutdown verification.
- `04`: now includes ledger persistence, retention, privacy, dependency, and
  replay/recovery requirements.
- `05`: now includes migration artifacts, compatibility projections, release
  notes, and composed-node trace preservation.
- `06`: now includes binding architecture, unsafe isolation, artifact version
  matching, cross-platform release packaging, and host-lane verification.
- `08`: defines the stage-start implementation readiness gate so source edits
  begin only after plan, standards, worktree, verification, and commit-boundary
  checks pass.
- `09`: defines the stage-end refactor decision and execution gate so each next
  stage starts from a standards-compliant touched-file baseline.
- `10`: defines the phased parallel implementation scaffold required before a
  stage can use concurrent workers.

## Residual Risks

- The plans intentionally do not choose exact crate names, storage engines, or
  binding generators beyond established direction. Those decisions need smaller
  implementation plans or ADRs before code changes.
- The root `../../../DIAGNOSTICS-MODEL-LICENSE-USAGE.md` remains outside `docs/` because
  it was requested as a root orientation document. It should stay short and
  point into `docs/` for durable planning details.
- `LAUNCHER-STANDARDS.md` does not directly change these plan files, but any
  implementation that adds canonical verification commands should expose them
  through `launcher.sh` or explicitly document why they remain workspace-native.

## Verification

- All numbered plans have explicit affected contract/artifact scope.
- All implementation categories from the standards are represented by at least
  one compliance gate.
- The review calls out standards that are indirect for this work, such as
  launcher, commit, frontend, accessibility, and release standards.
- The review does not weaken or duplicate the authoritative standards; it
  maps this plan set to them.

## Risks And Mitigations

- Risk: the review becomes stale as implementation decisions are made.
  Mitigation: update this file or replace the relevant point with an ADR when a
  decision is finalized.
- Risk: implementers treat the matrix as a substitute for reading the
  standards. Mitigation: keep the reviewed standards list explicit and treat
  this file as a mapping layer only.
- Risk: standards for host bindings evolve after these plans are written.
  Mitigation: re-run this review before binding implementation starts.

## Re-Plan Triggers

- A new numbered plan is added to this directory.
- A standards document changes in a way that affects Rust runtime, FFI,
  persistence, release, or frontend implementation.
- An implementation slice cannot satisfy one of the listed gates without
  changing scope or architecture.

## Completion Criteria

- Each execution-platform plan contains explicit standards constraints for its
  slice of work.
- Future implementation can be reviewed against this file and the numbered
  plans without rediscovering the standards category by category.
- Any implementation that cannot satisfy one of these gates must update the
  relevant plan or create an ADR before proceeding.
