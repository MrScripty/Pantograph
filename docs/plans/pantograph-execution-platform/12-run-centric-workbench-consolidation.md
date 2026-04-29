# 12: Run-Centric Workbench Consolidation

## Purpose

Consolidate the former `run-centric-gui-workbench` plan set into the
execution-platform plan set so future implementation has one canonical planning
path.

This stage does not make the older workbench directory authoritative. It imports
its product requirements, frontend workflow, diagnostic projection constraints,
review findings, and verification gates into execution-platform ownership. The
supporting review records that remain useful after consolidation live under
`reviews/run-centric-workbench/` so the former plan directory can be deleted.

## Status

In progress.

Stage `12` depends on the completed execution-platform stages `01` through `05`
and the repaired Stage `06` binding correction. Stage `11`
ArtifactStore/settings/media-dependency work has enough workbench-facing API
surface for Stage `12` pages, but Stage `11` is not fully closed while
producer-specific preview streams, diffusion child/revision artifacts, active
converter/library version capture, and OpenColorIO ABI validation remain.

Current Stage `12` implementation has completed source-audit/crosswalk,
Scheduler-default shell navigation, transient active-run ownership, current
page projection wiring, truthful Network/Node Editor states, Settings ownership
for persistent ArtifactStore and diagnostics retention policy, and focused
frontend/backend verification. Stage `12` remains open for stage-end refactor
gate evidence, optional GUI/screenshot coverage if tooling is added, and
follow-up coordination with the remaining Stage `11` media/API parity gaps.

## Consolidation Coverage Matrix

Former run-centric source names are retained here only as migration labels. The
canonical requirements are in this execution-platform plan set, and the useful
supporting review records have been moved to
`reviews/run-centric-workbench/`.

| Former Run-Centric Source | Execution-Platform Coverage |
| ------------------------- | --------------------------- |
| `README.md` | This file imports the workbench decision, invariants, active-run pipeline, Settings ownership, artifact/media refinements, and managed dependency refinements. |
| `00-overview-and-boundaries.md` | Covered by `00-overview-and-boundaries.md`, this file, and Stage `11` for ArtifactStore/settings/media dependencies. |
| `01-workflow-identity-versioning-and-run-snapshots.md` | Covered by Stage `01`, Stage `02`, Stage `04`, and this file's identity/versioning acceptance gates. |
| `02-scheduler-estimates-events-and-control.md` | Covered by this file's scheduler/run-list requirements and Stage `04` typed event/projection requirements. |
| `03-diagnostics-retention-and-audit-ledgers.md` | Covered by Stage `04`, Stage `11`, and this file's retention/audit/I/O acceptance gates. |
| `diagnostic-event-ledger-architecture.md` | Covered by Stage `04` and this file's event-ledger/projection rules, including incremental materialized projections. |
| `04-api-projections-and-frontend-data-boundaries.md` | Covered by this file's API projection and frontend service boundaries, plus Stage `06` and Stage `11` for binding/API media surfaces. |
| `05-app-shell-active-run-navigation.md` | Covered by this file's app shell, routing, toolbar, active-run, and old-surface retirement requirements. |
| `06-run-centric-page-implementations.md` | Covered by this file's page implementation requirements for Scheduler, Diagnostics, Graph, I/O Inspector, Library, Network, Node Editor, and Settings. |
| `07-verification-rollout-and-refactor-gates.md` | Covered by this file's verification, rollout, source-audit, and stage-gate requirements, plus `08`, `09`, and `10`. |

## Moved Review Records

These review records are now part of execution-platform planning evidence:

| Review Record | Execution-Platform Use |
| ------------- | ---------------------- |
| `reviews/run-centric-workbench/README.md` | Index for the moved review records and their non-authoritative rationale role. |
| `reviews/run-centric-workbench/architecture-requirements-against-current-code.md` | Architecture requirement import and implementation-wave source-audit input. |
| `reviews/run-centric-workbench/architecture-compatibility-risk-review.md` | Compatibility/risk import, anti-regression rules, and re-plan trigger input. |
| `reviews/run-centric-workbench/blast-radius-analysis.md` | Blast-radius import, wave planning rules, and verification/source-audit input. |
| `reviews/run-centric-workbench/diagnostics-code-against-event-ledger.md` | Diagnostic event ledger migration detail and source-audit checks for raw event/read-model drift. |
| `reviews/run-centric-workbench/plan-continuity-review.md` | Core invariants, event family ownership rules, and semantic-version/fingerprint strictness checklist. |
| `reviews/run-centric-workbench/projection-materialization-standards-pass.md` | Materialized projection cursor rules, warm-drain ownership risk, and non-trivial event-count performance verification. |
| `reviews/run-centric-workbench/requirements-coverage-review.md` | Requirement-by-requirement audit trail used to validate this consolidation. |

## Objective

Pantograph GUI starts as a run-centric workbench. Scheduler is the default page.
Runs are the shared context across Scheduler, Diagnostics, Graph, I/O Inspector,
Library, Network, Node Editor, and Settings. Backend-owned facts define durable
truth; the frontend owns only transient presentation state such as selected page,
active-run selection, filters, sorting, and layout.

The workbench must make a clean pipeline for current and future features:

- Scheduler: dense current/future/historic run table, estimates, queue state,
  status, timing, scheduler decisions, model/runtime/cache state, and scoped
  controls.
- Diagnostics: run/system diagnostics over workflow, node, model, runtime,
  scheduler, retention, and version filters.
- Graph: workflow editor plus historic run graph projection as it existed at
  run time.
- I/O Inspector: workflow and node input/output inspection, artifact gallery,
  streaming/finalized/expired/deleted states, read-only retention policy/status
  visibility, cleanup operations, and binary-safe artifact previews.
- Library: Pumas/Pantograph assets, models, runtimes, workflows, nodes, managed
  dependencies, usage audit, active-run asset highlighting, and management
  actions.
- Network: local-only system/node capability and load view now, with future
  Iroh peer discovery, pairing, and trust slots.
- Node Editor: reserved truthful future page for node authoring/local-agent
  tooling without implying unavailable support exists.
- Settings: canonical persistent settings owner for artifact persistence,
  output format defaults, runtime/server/dependency configuration, and future
  global workbench settings.

## Scope

In scope:

- Canonical execution-platform ownership of the former run-centric workbench
  requirements and moved review evidence.
- Scheduler-default workbench shell, toolbar navigation, transient active-run
  state, and old root-mode relocation or retirement.
- Backend-owned API/materialized projection contracts needed by Scheduler,
  Diagnostics, Graph, I/O Inspector, Library, Network, Node Editor, and
  Settings pages.
- Page implementation requirements, accessibility checks, source audits, and
  stage-gate criteria for the workbench.
- Dependency coordination with reopened Stage `06` binding verification and
  Stage `11` ArtifactStore/settings/media-dependency work.

Out of scope:

- Reintroducing the retired run-centric plan directory as an authoritative
  source.
- Implementing Iroh peer discovery or distributed execution.
- Implementing Node Editor local-agent authoring before its platform
  dependencies exist.
- Passing artifact media bodies through inline JSON or exposing physical
  storage locations to clients.
- Creating another persistent global settings owner outside the workbench
  Settings page.

## Core Invariants

- Scheduler is the GUI landing page.
- Active-run selection is frontend session state and is not persisted.
- Queued/future runs are immutable after submission; changes require cancel and
  resubmit.
- Workflow identity is stable, user named, validated, and rejected with explicit
  errors when invalid.
- Workflow version identity is normalized executable topology plus node
  behavior versions. Display metadata has separate presentation revisions.
- Semantic versions are explicit labels; backend-computed fingerprints are the
  correctness identity. Semantic-version/fingerprint conflicts fail closed.
- Run snapshots capture workflow version, node versions, model choices, runtime
  versions, scheduler policy, retention policy, graph settings, inputs, and
  selected output format settings at queue time.
- Scheduler alone owns reservations, placement, model load/unload, retries,
  fallback, delay, and execution admission.
- Normal clients can affect only their own session/bucket queue entries.
  Pantograph GUI is the privileged developer/admin surface for all-session
  controls.
- Diagnostics, scheduler timelines, I/O observations, retention changes,
  Library/Pumas audits, runtime state, and Network local-node facts use the
  typed diagnostic event ledger pattern.
- Normal GUI/API reads consume durable materialized projections with
  projection-version and event-sequence cursors. Full replay is a migration,
  repair, projection-version-change, admin, or test path only.
- Event payloads are typed, bounded, backend-authored, and validated before
  persistence. Raw stream chunks and media bodies do not belong in event JSON.
- Artifact payload bodies are backend-owned and location opaque. Clients receive
  typed descriptors, artifact ids, stream ids, lifecycle state, retention state,
  and binary-safe retrieval/stream handles.
- JSON workflow/API values remain descriptor/control-plane data. Image, audio,
  video, 3D, large table, and generic binary bodies are not transported inline
  as JSON.
- Retention may expire or delete payload bodies, but audit metadata remains
  queryable according to retention/privacy policy.
- Settings page owns global persistent settings. Feature-local helper settings
  may stay near the feature only when that placement is directly required for
  usability and the setting is not global persistent configuration.
- Managed redistributables preserve product categories: runtime sidecars, tool
  binaries, and native library artifacts may share infrastructure but must not
  be misrepresented as one category.

## API And Projection Requirements

The workbench consumes backend-owned projections rather than raw storage rows or
frontend-inferred state.

Required projections include:

- run list and run detail
- current run status
- scheduler estimate and scheduler timeline
- scheduler queue controls and authority error responses
- workflow version and presentation revision graph projection
- node status/runtime overlay projection
- diagnostics summary and workflow-version performance summary
- I/O artifact gallery, artifact metadata, stream/finalization state, retention
  state, and consume acknowledgement state
- Library/Pumas usage, audit, cache, and asset-management state
- local Network/system node capability, load, cache, runtime, model, and device
  state
- global retention and artifact persistence policy
- artifact format settings and capability options
- managed dependency status/actions for runtimes, ffmpeg, `ocioconvert`,
  `oiiotool`, and OpenColorIO

Projection APIs must expose freshness/catching-up state where a warm projection
can lag. Page reads must not rebuild projections or replay all events during
normal navigation.

Backend errors must remain typed through API/frontend service adapters,
including invalid workflow identity, semantic-version/fingerprint conflict,
missing node behavior version, unauthorized queue action, unsupported format,
unsupported codec, unavailable conversion backend, invalid quality/bitrate,
invalid color-management setting, artifact unavailable, artifact finalizing,
artifact expired, artifact deleted, stream failed, consume acknowledgement
rejected, retention failure, and unauthorized artifact access.

## Scheduler Requirements

Scheduler must produce estimates before run execution. Estimates may begin as
rule-based and low-confidence, but they must be explicit about confidence,
timestamp, policy, facts used, and reason codes.

The scheduler derives resource needs from backend facts, including node
metadata, model metadata, file sizes, runtime metadata, graph settings,
historical diagnostics, local/future node capabilities, current load, cache
state, and queue state. Workflows do not declare authoritative resources.

Scheduler events must be typed diagnostic events for submission, estimate,
queue placement, delay, promotion, cancellation, admission, reservation,
runtime/device selection, model load/unload, retry, fallback, client action,
and admin override. Scheduler projections may join `run.*` and `node.*`
lifecycle events for timeline visibility, but scheduler producers must not
duplicate terminal run lifecycle truth.

## Diagnostic Event Ledger Requirements

The typed diagnostic event ledger is the source of audit truth. Event envelope
fields include monotonic `event_seq`, event id/kind/schema/source timestamps,
run/workflow/version/node/model/runtime/client/session/bucket/policy
correlation fields where required, privacy class, retention class, payload hash,
payload size, payload reference, and bounded payload JSON.

Initial event families are:

- `scheduler.*`
- `run.*`
- `node.*`
- `io.*`
- `library.*`
- `runtime.*`
- `retention.*`

Every event kind defines required envelope fields, typed payload structure,
schema version, allowed source components, privacy class, retention class,
maximum embedded payload size, payload reference rules, and scope.

Projection storage follows the durable materialized read-model pattern:

- append typed event once
- update hot projections synchronously or near-synchronously
- update warm projections asynchronously or lazily
- store `projection_name`, `projection_version`, `last_applied_event_seq`,
  status, and rebuild metadata
- apply only events after the stored cursor during normal operation
- perform full rebuild only for migration, repair, projection-version change,
  admin maintenance, or tests

Hot projections include run list, run detail, current run status, scheduler
timeline, scheduler estimates, and active-run I/O metadata. Warm projections
include workflow-version performance, model/runtime comparison facets, Library
usage counts, retention completeness, and aggregate diagnostics. Cold rebuilds
cover full diagnostics summary rebuilds, all-runs artifact gallery rebuilds,
and historical aggregate recomputation.

## Artifact, Retention, And Format Requirements

ArtifactStore owns physical payload bodies and streaming lifecycle. It must hide
whether a payload is in memory, on disk, or future remote storage.

Artifacts cover workflow inputs, workflow outputs, node inputs, node outputs,
intermediate artifacts, final artifacts, image, audio, video, 3D, large table,
text/JSON descriptors, and generic binary payloads.

Artifact descriptors include kind, media type, byte length, content hash,
lifecycle state, retention state, storage opacity, producer/consumer facts,
format metadata, and binary-safe retrieval or stream handles.

Streaming assets such as diffusion passes and audio/video chunks use stream
state and artifact lifecycle events. Stream chunks and raw bytes stay out of
diagnostic event JSON.

Global artifact persistence policy includes TTL, maximum disk bytes, maximum
memory bytes, maximum single artifact bytes, memory-to-disk spill threshold,
delete-on-consume mode, cleanup status, and consume acknowledgement semantics.
Initial policy is global but should leave room for future granularity.

Output format defaults are backend-owned settings:

- image: OpenImageIO-supported formats, OpenColorIO color management, JPEG
  default at 75 percent quality, sRGB default color profile
- audio: ffmpeg-backed Ogg Opus default at 96 kbps, with Vorbis, WAV, MP3,
  AIFF, and FLAC support
- video: ffmpeg-backed SVT-AV1 CRF 32 8-bit default
- 3D: GLB default, with glTF and OBJ support

Output nodes expose backend-provided format selectors and record actual
per-run/per-node format choices in run snapshots, artifact descriptors, and
diagnostics metadata.

## Workbench Shell Requirements

The top-level GUI shell replaces the old canvas/workflow mode switch with page
navigation. It opens to Scheduler and includes Scheduler, Diagnostics, Graph,
I/O Inspector, Library, Network, Node Editor, and Settings. Navigation controls
must be accessible, keyboard reachable, and visually stable.

The active-run store contains only transient selected-run identity/context and
top-bar summary state. Pages receive active-run/no-active-run context and fetch
durable facts from backend services.

Existing drawing-to-Svelte and legacy graph entry points must be relocated into
the workbench, converted into a Library/Node Editor tool, or intentionally
retired with tests and documentation updated.

No page may mutate backend-owned state optimistically. Queue actions, retention
policy updates, Library actions, managed dependency changes, artifact consume
acknowledgements, and settings changes wait for backend confirmation and then
refresh projections.

## Page Requirements

Scheduler page:

- dense spreadsheet-like current/future/historic run table
- queued and scheduled runs as first-class rows
- active-run selection
- key metrics: status, duration, queue wait, estimates, priority, bucket,
  session, workflow version, runtime/model/cache state, and scheduler reason
- scoped client controls and privileged GUI admin controls

Diagnostics page:

- selected-run history from receipt through queue, planning, admission,
  execution, node diagnostics, completion/failure, and total duration
- filter by workflow version, node version, model, runtime, scheduler policy,
  retention policy, bucket, session, and time
- system diagnostics without active run where backend projections support it

Graph page:

- graph editor for current authoring
- selected historic run graph rendered from workflow version and presentation
  revision, not from current editable workflow by accident
- output-node format selectors from backend capabilities

I/O Inspector:

- workflow-level and node-level input/output browsing
- artifact gallery for text, JSON, image, audio, video, 3D/file, table, and
  generic binary descriptors
- retained, streaming, finalizing, expired, deleted, metadata-only, external,
  truncated, and too-large states
- binary-safe preview retrieval/streaming and cleanup of object URLs/readers on
  artifact change, navigation, and unmount
- no-active-run browsing of retained artifacts where backend projections allow
  it
- read-only global retention/artifact persistence policy visibility plus
  cleanup operations; persistent policy edits live on Settings

Library:

- Pumas and Pantograph-owned assets
- search/download/delete/manage actions through backend APIs
- active-run asset highlighting
- model/runtime/workflow/node/managed-dependency usage audit
- network bytes/cache hit/miss facts where available

Network:

- local-only system and node capability/load/cache/runtime/model/device state
  before Iroh exists
- future peer, pairing, trust, and network node id structure
- no overclaim that distributed execution is implemented

Node Editor:

- reserved/future truthful disabled state for local-agent node authoring
- no implied runtime authoring support until dependent platform features exist

Settings:

- canonical persistent settings owner
- artifact persistence policy
- artifact format defaults and conversion capabilities
- runtime/server/dependency controls relocated, embedded, or retired from old
  independent surfaces
- Library-managed OCIO, ffmpeg, `ocioconvert`, and `oiiotool` status/actions
  through backend DTOs

## Review Finding Import

The prior run-centric review files are imported as implementation constraints:

- Architecture requirements: implementation must account for workflow identity,
  scheduler authority, diagnostic ledger/projection ownership, API/frontend
  boundaries, shell migration, page implementation, and verification gates
  against current code.
- Architecture compatibility/risk: no legacy compatibility is required, but
  changes must avoid mixed execution identity, duplicate event truth, DTO drift,
  read-time mutation, security regressions, and frontend state ownership leaks.
- Blast radius: implementation touches workflow-service contracts, diagnostics
  ledger/schema, scheduler, runtime registry, embedded runtime, Tauri/HTTP/API
  adapters, frontend services/stores/components, graph persistence, tests, and
  docs. Each slice must record write sets and verification.
- Diagnostics-code review: current trace, scheduler diagnostics, Tauri stores,
  headless projections, event transport, embedded runtime diagnostics, and
  frontend diagnostics must converge through typed event/projection boundaries
  instead of adding bespoke state paths.
- Plan continuity: event family ownership, semantic-version/fingerprint
  strictness, mandatory node versions, and DTO consistency are non-negotiable.
- Projection materialization: durable projections use cursors and performance
  tests with non-trivial event counts.
- Requirements coverage: active-run workbench, versioned diagnostics, scheduler
  audit, I/O retention, Library/Pumas audit, local Network view, Settings
  ownership, ArtifactStore, and media/dependency requirements must all be
  represented before implementation is considered ready.
- Moved review records: implementation must use the review appendix for
  source-audit terms, current-code risk notes, event-envelope migration detail,
  projection materialization controls, and requirement-by-requirement evidence.

## Implementation Waves

Before coding Stage `12`, create implementation-wave files with non-overlapping
write sets. Default sequence:

1. Source audit and final crosswalk from run-centric docs to execution-platform
   tasks.
2. API/projection contract correction for any gaps remaining after Stages
   `01` through `06` and Stage `11`.
3. Workbench shell and active-run navigation.
4. Scheduler, Diagnostics, Graph, I/O Inspector, Library, Network, Node Editor,
   and Settings page implementation.
5. Frontend services/stores/type parity and generated/schema validation where
   available.
6. Verification, accessibility, Playwright/screenshot checks, source-audit
   closure, and stage-end refactor gate.

## Tasks

### Milestone 1: Source Audit And Crosswalk Closure

- [x] Read this plan, Stage `06`, Stage `11`, and the moved review records in
  `reviews/run-centric-workbench/`.
- [x] Verify each former run-centric requirement maps to an
  execution-platform implementation task or is explicitly obsolete with reason.
- [x] Record Stage `06` and Stage `11` dependency status before selecting a
  workbench implementation slice.

### Milestone 2: API And Projection Readiness

- [x] Close current run, scheduler, diagnostics, I/O, Library, Network,
  Settings, and artifact DTO gaps used by implemented page components.
- [x] Confirm hot/warm projection freshness behavior and no-full-replay
  navigation behavior are represented in backend services.
- [x] Choose DTO parity enforcement through generated bindings or paired
  Rust/TypeScript contract tests for every new page projection.
- [x] Strengthen shared-fixture/generated DTO parity for current Network and
  Settings workbench surfaces.
- [ ] Extend parity strengthening to remaining Stage `11` media surfaces before
  promoting those APIs as complete external contracts.

### Milestone 3: Workbench Shell And Active-Run Navigation

- [x] Replace old root mode navigation with Scheduler-default workbench
  navigation.
- [x] Keep active-run selection as frontend session state only.
- [x] Relocate, embed, or retire drawing-to-Svelte and legacy graph entry
  points with tests and documentation updated.

### Milestone 4: Page Implementations

- [x] Implement Scheduler, Diagnostics, Graph, I/O Inspector, Library, Network,
  Node Editor, and Settings pages against backend-owned projections.
- [x] Keep Network and Node Editor truthful when future Iroh or local-agent
  features are not implemented.
- [x] Use ArtifactStore and Settings APIs from Stage `11` for binary-safe
  previews and global persistent settings.

### Milestone 5: Verification And Refactor Gate

- [ ] Run backend projection, frontend type/unit/lint, accessibility, and GUI
  checks listed in this plan. Backend/frontend/a11y/build gates have passed;
  GUI/screenshot checks remain pending because no Playwright/equivalent harness
  is currently present.
- [x] Run source audits for raw event reads, page-load full replay, inline
  media JSON, duplicate settings owners, and old root navigation ambiguity.
- [ ] Apply `09-stage-end-refactor-gate.md` and record any required follow-up
  plan before marking Stage `12` complete.

## Verification

- `cargo test -p pantograph-runtime-attribution`
- `cargo test -p pantograph-node-contracts`
- `cargo test -p pantograph-diagnostics-ledger`
- `cargo test -p pantograph-workflow-service diagnostics`
- `cargo test -p pantograph-workflow-service session_execution`
- `cargo test -p pantograph-uniffi` after Stage `06` is corrected
- frontend unit/type/lint checks for workbench stores, services, presenters,
  and pages
- Playwright or equivalent GUI checks for Scheduler default route, navigation,
  active-run propagation, no-active-run states, dense table stability,
  artifact preview lifecycle, Settings ownership, and accessibility
- projection rebuild/performance tests with non-trivial event counts
- source audits proving old canvas/workflow root mode ambiguity, raw event page
  reads, full-replay page reads, inline media JSON payload paths, duplicate
  persistent settings owners, unmanaged ffmpeg/OIIO/OCIO probing, and
  optimistic global setting mutations are removed or intentionally quarantined

## Completion Criteria

- Every former run-centric source listed in the coverage matrix is represented
  in an execution-platform implementation task or explicitly marked obsolete
  with reason.
- Every useful review record has been moved under
  `reviews/run-centric-workbench/`; Stage `12` implementation does not require
  the old run-centric plan directory to exist.
- Stage `06` binding verification has been corrected or the affected binding
  surfaces are explicitly excluded from the implemented slice.
- Stage `11` ArtifactStore/settings/media-dependency contracts are implemented
  before pages require binary-safe artifact previews or Settings format
  controls.
- Scheduler is the default GUI page.
- Active-run navigation works across all workbench pages without backend
  persistence.
- All page data comes from backend-owned projections or Settings services.
- Artifact previews and streams use ArtifactStore APIs, not inline media JSON.
- Settings is the only canonical persistent settings page.
- Verification commands, frontend checks, accessibility checks, source audits,
  and stage-end refactor gate pass or have recorded, standards-compliant
  follow-up plans.

## Risks And Mitigations

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Workbench pages start before backend projections exist. | High | Require Milestone 2 projection readiness before page implementation and keep page stores selection/filter-only. |
| Stage `12` bypasses unresolved Stage `06` or Stage `11` dependencies. | High | Record dependency status and explicitly exclude affected binding/media surfaces when they are not ready. |
| Frontend reconstructs authoritative facts from raw events. | High | Use backend materialized projections and source-audit raw event page reads before completion. |
| Active-run selection becomes persisted backend state. | Medium | Keep active-run state frontend-transient and test no-restart persistence behavior. |
| Old root modes remain alongside the workbench shell. | Medium | Require relocation, embedding, or retirement of legacy surfaces with tests and docs updated. |
| Settings ownership fragments across old panels and the new page. | High | Make Settings the only persistent global settings owner and source-audit duplicate owners. |
| Network or Node Editor pages imply unavailable future features. | Medium | Render local-only or disabled/future states until platform support exists. |

## Re-Plan Triggers

- A former run-centric source requirement is found that is not covered by this
  file, Stage `11`, or existing execution-platform stages.
- Stage `06` binding correction changes the API shape needed by workbench
  services.
- Stage `11` changes ArtifactStore, Settings, or managed dependency contracts.
- Frontend implementation discovers that existing graph/drawing/runtime-manager
  surfaces cannot be relocated, embedded, or retired without a separate plan.
- Network/Iroh or Node Editor authoring work becomes active rather than a
  reserved/future page slot.
