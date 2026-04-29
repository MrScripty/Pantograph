# 07: Standards Compliance Review

## Purpose

Verify that the execution-platform plans conform to the planning standards and
that implementation following these plans should produce code compliant with
the repository standards.

## Objective

Maintain the standards compliance map for the execution-platform plan set and
record stage-gate or plan updates needed before implementation continues.

## Implementation Progress

### 2026-04-25 Stage-Start Preflight

Start outcome: `ready_with_recorded_assumptions`.

- Selected stage: Stage `07`, standards compliance review.
- Prior-stage gates at the time of the original review: Stage `01` through
  Stage `06` were recorded as implemented, their architecture ADR checkpoints
  were recorded, and their stage-end refactor gates were recorded. The
  2026-04-29 audit reopened Stage `06` completion because current UniFFI/C#
  binding verification no longer passes the workflow-run request contract.
  Stage `05` required a separate refactor plan; the module split was completed
  before the original Stage `06` closeout.
- Current dirty files before Stage `07`: unrelated asset deletions and
  untracked asset files under `assets/`. Stage `07` must not stage, reformat,
  or revert them.
- Intended write set: execution-platform review docs, Stage `06` closeout
  ledger corrections, and documentation-only status reconciliation. No source,
  test, manifest, generated artifact, or build metadata edits are required.
- Concurrency decision: single-worker. Stage `07` is a documentation review and
  does not warrant a new concurrent implementation-wave folder.
- Expected verification: Markdown/status consistency inspection, `git diff`
  review, and `git status --short` to confirm only intended documentation files
  are staged.

### 2026-04-25 Post-Implementation Review Progress

- Reconciled the review with the then-completed Stage `01` through Stage `06`
  architecture ADRs. This reconciliation is stale for Stage `06` completion
  status after the 2026-04-29 audit.
- Replaced stale future-tense residual risks for Stage `01`, Stage `04`, and
  Stage `06` with current implementation evidence and remaining host/toolchain
  limitations.
- Confirmed Stage `06` support-tier decisions now live in ADR-010:
  Native Rust supported for implemented surfaces, C# supported for verified
  generated/native surfaces, Python unsupported, and BEAM experimental on this
  host.
- 2026-04-29 audit correction: C# support remains an ADR-010 target and
  ownership decision, but Stage `06` completion is reopened for
  execution-session run calls until UniFFI/C# request bodies include
  `workflow_semantic_version` and pass verification.

### 2026-04-25 Stage-End Refactor Gate

Outcome: `not_warranted`.

- Touched files reviewed:
  `docs/plans/pantograph-execution-platform/07-standards-compliance-review.md`
  and
  `docs/plans/pantograph-execution-platform/implementation-waves/06-binding-projections-and-verification/coordination-ledger.md`.
- Applicable standards groups: planning, documentation, commit history,
  implementation-wave traceability, and stage-end gate reporting.
- Findings: the Stage `07` diff is a documentation/status reconciliation only.
  No source, test, manifest, generated artifact, or build metadata file was
  touched, and no additional decomposition or refactor is warranted.
- Residual unrelated dirty files under `assets/` remain outside the Stage `07`
  touched-file boundary.

## Scope

In scope:

- standards review for the numbered execution-platform plans
- implementation compliance gates implied by the standards
- residual risks that need later implementation plans or ADRs

Out of scope:

- source-code implementation
- replacing the authoritative standards files
- changing the crate ownership, storage engine, or release automation decisions
  recorded in the numbered plans without updating those plans first

## Milestones

1. Review active execution-platform plans against the external standards.
2. Record required compliance gates and residual risks.
3. Reconcile stale findings after Stage `06`, Stage `11`, or Stage `12`
   changes plan or implementation status.
4. Update completion criteria only when every residual compliance issue has an
   owner or a recorded follow-up plan.

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
| `01-client-session-bucket-run-attribution.md` | Rust API, security, persistence, concurrency, testing | Add a dedicated attribution crate, use validated ids and typed state, single lifecycle owner for session races, durable attribution before execution, SQLite recovery tests. |
| `02-node-contracts-and-discovery.md` | Rust API, architecture, frontend, accessibility, testing | Add a dedicated node-contract crate, keep canonical contracts in backend Rust, publish effective contracts, reject host-local semantics, verify graph compatibility and GUI projection behavior. |
| `03-managed-runtime-observability.md` | Rust async, concurrency, observability, testing | Runtime owns spans, cancellation, progress, task lifecycle, and guarantee classification without node boilerplate. |
| `04-model-license-diagnostics-ledger.md` | Persistence, security, dependency, frontend/accessibility, release, testing | Persist time-of-use license snapshots, typed measurements, attribution history projections, retention policy, indexed queries, replay/recovery tests, and accessible backend-driven GUI diagnostics views. |
| `05-composition-factoring-and-migration.md` | Architecture, documentation, release, testing | Preserve primitive trace facts and model/license attribution while cleanly upgrading or rejecting old persisted workflow artifacts without indefinite compatibility shims. |
| `06-binding-projections-and-verification.md` | Interop, language bindings, Rust unsafe, cross-platform, release | Resolve the native Rust base API first, keep three-layer binding architecture, isolate unsafe, version-match generated bindings and native artifacts, and verify every supported host lane with language-native tests. |
| `08-stage-start-implementation-gate.md` | Planning, worktree hygiene, commits, verification, concurrent worker readiness | Confirm plan readiness, standards context, dirty-file safety, write boundaries, verification, and commit expectations before source edits begin. |
| `09-stage-end-refactor-gate.md` | Planning, coding, testing, tooling, documentation | Decide whether touched files need a standards refactor before the next stage starts, and constrain any refactor to files touched by that stage. |
| `10-concurrent-phased-implementation.md` | Concurrent worker planning, implementation waves, reporting, coordination | Require explicit wave specs, non-overlapping write sets, report files, coordination ledger, one-wave-at-a-time execution, and one-at-a-time integration when parallel work is warranted. |
| `11-artifact-format-settings-and-managed-media-dependencies.md` | Architecture, security, dependency, interop, frontend, testing, cross-platform | Move media payload bodies to ArtifactStore, make workbench Settings canonical, generalize/split managed redistributables, isolate OCIO FFI, verify supply-chain metadata, and migrate base64 media paths. |
| `12-run-centric-workbench-consolidation.md` | Frontend, accessibility, API projection, persistence, diagnostics, rollout, testing | Keep Scheduler-default workbench pages backend-driven, preserve active-run as transient UI state, consume materialized projections, avoid duplicate settings owners, import run-centric review findings, and verify shell/page accessibility. |

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
  C#, Python, and BEAM supported lanes must each load the real native artifact
  from their language runtime and test the projected API natively.
- Persistence gate: durable attribution, usage ledger, saved workflow, and
  migration artifacts require versioning, retention or migration behavior, and
  restart/replay tests.
- Security gate: credentials, paths, ids, payload sizes, queue limits, and
  listener exposure are validated at ingress and represented internally as
  trusted domain types.
- Dependency gate: new crates or host tooling require owner, transitive-cost,
  feature, audit, and release-artifact review.
- Tooling gate: implementation PRs must define or use canonical formatter,
  lint, typecheck, test, doctest, feature, audit, and artifact validation
  commands. Rust stages must include `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  targeted package tests, `cargo test --workspace --doc`, and feature checks
  required by touched public feature contracts unless a repo-owned equivalent
  is recorded in the stage-start report.
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
- Settings ownership gate: persistent global settings must be owned by the
  workbench Settings page. Legacy side-panel, runtime-manager, and server
  settings surfaces must be relocated, embedded, or retired when Stage `11`
  lands. Feature-local helper settings may remain colocated only when they do
  not own global persistent configuration.
- Artifact payload gate: image, audio, video, 3D, large table, and generic
  binary payload bodies must use ArtifactStore descriptors plus binary-safe
  retrieval/streaming. Raising JSON value caps is not an acceptable substitute.
- Managed redistributables gate: runtime sidecars, tool binaries, and native
  library artifacts may share infrastructure, but DTOs, tests, and code names
  must preserve distinct product categories and avoid runtime-only semantics
  for tools/libraries.
- OCIO native boundary gate: OCIO loading must be isolated behind a safe
  adapter with ABI/version validation, documented lifetime/threading/shutdown
  behavior, fail-closed capability reporting, and no unsafe FFI code in domain
  logic.
- Media dependency supply-chain gate: managed media dependency catalogs must
  validate checksum/signature, license/redistribution metadata, source owner,
  platform support, archive kind, expected files, and rejected arbitrary
  download URLs.
- Release gate: public or binding-facing changes require changelog or migration
  notes, explicit artifact naming, checksums, SBOM expectations where released,
  and version-matched native/binding packages.
- ADR checkpoint gate: every stage that first implements an
  architecture-defining decision from `00-overview-and-boundaries.md` must
  create or update the matching ADR before the stage is considered complete.

## Per-Plan Findings

- `00`: now includes cross-cutting standards gates, affected structured
  contracts, risks, and completion criteria.
- `01`: now includes durable artifact scope, credential/security notes,
  reconnect/takeover concurrency constraints, and recovery verification.
- `02`: now includes canonical contract ownership, host-local semantics
  rejection, discovery DTO documentation, and graph compatibility tests.
- `03`: now includes runtime task ownership, cancellation, progress, guarantee
  classification, and shutdown verification.
- `04`: now includes ledger persistence, attribution history projections, GUI
  diagnostics/history requirements, retention, privacy, dependency, and
  replay/recovery requirements.
- `05`: now includes migration artifacts, clean upgrade or typed rejection of
  old persisted workflows, removal of temporary compatibility surfaces, release
  notes, and composed-node trace preservation.
- `06`: now includes binding architecture, unsafe isolation, artifact version
  matching, cross-platform release packaging, host-lane verification, and
  language-native C#/Python/BEAM test requirements.
- `08`: defines the stage-start implementation readiness gate so source edits
  begin only after plan, standards, worktree, verification, and commit-boundary
  checks pass.
- `09`: defines the stage-end refactor decision and execution gate so each next
  stage starts from a standards-compliant touched-file baseline.
- `10`: defines the phased parallel implementation scaffold required before a
  stage can use concurrent workers.
- `11`: adds the active extension for ArtifactStore, canonical Settings
  ownership, managed media dependencies, OCIO FFI isolation, typed format DTOs,
  and base64/media JSON migration.
- `12`: imports the run-centric GUI workbench plan set into execution-platform
  ownership, including Scheduler-default navigation, active-run page context,
  page projection requirements, frontend service boundaries, review findings,
  rollout gates, and source-audit requirements.

## Residual Risks

- Stage `01` through Stage `06` architecture decisions are now represented by
  ADR-005 through ADR-010. Future execution-platform work must update or
  supersede those ADRs when it changes durable attribution, node contracts,
  runtime observability, diagnostics ledger persistence, composition/migration,
  or binding projection ownership.
- Stage `06` support tiers are recorded in ADR-010, but the Stage `06` closeout
  is stale after the 2026-04-29 audit. Current risk is not only toolchain and
  artifact availability: UniFFI/C# request-shape drift must be fixed before
  supported C# execution-session surfaces are considered complete. Python stays
  unsupported until a real generated/native package and import/load smoke
  exists, and BEAM stays experimental on hosts without `mix` smoke coverage.
- Stage `04` recorded SQLite ledger dependency, linking, migration, audit, and
  release-artifact impact before implementation. Future storage-engine changes
  require a new plan update or ADR rather than editing the ledger crate in
  place.
- Stage `01` recorded SQLite attribution dependency, linking, migration, audit,
  and release-artifact impact before implementation. Future attribution storage
  changes require a plan update or ADR because the durable schema is now an
  implemented artifact.
- The root `../../../DIAGNOSTICS-MODEL-LICENSE-USAGE.md` remains outside `docs/` because
  it was requested as a root orientation document. It should stay short and
  point into `docs/` for durable planning details.
- `LAUNCHER-STANDARDS.md` does not directly change these plan files, but any
  implementation that adds canonical verification commands should expose them
  through `launcher.sh` or explicitly document why they remain workspace-native.
- Some completed stage plans retain historical progress notes and future-tense
  implementation requirements below their completion summaries. Those notes are
  useful audit history, but future readers should treat the latest
  implementation progress, coordination ledger, and ADR entries as authoritative
  when they conflict with earlier planning language.

## Verification

- All numbered plans have explicit affected contract/artifact scope.
- All implementation categories from the standards are represented by at least
  one compliance gate.
- The review calls out standards that are indirect for this work, such as
  launcher, commit, frontend, accessibility, and release standards.
- The review does not weaken or duplicate the authoritative standards; it
  maps this plan set to them.
- 2026-04-25 Stage `07` review compared Stage `01` through Stage `06`
  completion records, ADR index entries, support-tier records, and current
  dirty worktree state. No source-code verification was required because this
  stage changed review/status documentation only.
- 2026-04-29 audit update: source-code verification has since shown Stage `06`
  binding verification failures. Stage `07` is therefore documentation-present
  but status-stale until Stage `06` is corrected and this standards review is
  refreshed against the corrected evidence.

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
- Stage `07` implementation is complete when stale compliance-review residual
  risks are reconciled with completed stage evidence, any discovered ledger
  drift is corrected, and the documentation-only diff is committed separately
  from unrelated asset changes.
