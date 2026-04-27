# Architecture Compatibility And Risk Review

## Status

Draft compatibility review. Not an implementation plan replacement.

Last updated: 2026-04-27.

## Compatibility Policy

Backwards compatibility is not required for this workbench architecture change.
Existing saved workflow files, run history, diagnostics rows, and overloaded
fingerprint fields may be invalidated, deleted, ignored, or regenerated if they
cannot satisfy the new contracts.

The plan should prefer a clean breaking cutover over long-term compatibility
shims.

## Purpose

Re-examine the current codebase against the architecture required by the
run-centric GUI workbench plans, with emphasis on compatibility, blast radius,
standards-driven refactors, and regression risk.

This review answers four questions:

- Is the current architecture compatible with the required architecture?
- Do the required changes affect systems outside the obvious edit areas?
- Will standards compliance require additional refactors during or after
  implementation?
- Which regressions or code-design issues are likely if implementation is not
  staged carefully?

## Executive Assessment

The current architecture is partially compatible, but the workbench should not
be implemented as a frontend-first shell over the existing session diagnostics.
The existing code has useful foundations: backend-owned graph mutation,
session queues, timing observations, diagnostics projections, runtime
capability inspection, managed-runtime views, and frontend service/store
boundaries. Those foundations can be extended.

The missing pieces are architectural, not only UI features:

- workflow execution-version registry
- immutable run snapshot ledger
- typed diagnostic event ledger plus durable global scheduler run/estimate/read
  models
- version-aware diagnostics keys
- I/O artifact retention metadata
- Library/Pumas audit records
- local system/network-node facts
- additive API projections for workbench pages

The highest cutover risk is leaving existing `graph_fingerprint` semantics
partially active. Today that field is a topology/type revision token used by
backend timing, frontend diagnostics, Svelte graph synchronization, connection
intent, tests, and mocks. The clean target is to replace this overloaded model
with explicit topology, execution, workflow-version, and presentation-revision
fields in Stage `01`, then update all active consumers in the same cutover.

## Compatibility Matrix

| Required architecture | Current compatibility | Cross-system effects | Standards/refactor risk | Regression risk | Required mitigation |
| --- | --- | --- | --- | --- | --- |
| Stable workflow identity with strict validation | Low. Workflow id validation is currently non-empty only, while persistence sanitizes names differently. | Saved workflow files, workflow list/load, capability lookup, edit sessions, tests, frontend `currentGraphId`. | Needs centralized domain validator at API boundaries. | Existing saved workflows with spaces or sanitized names become invalid. | Add domain identity type, explicit rejection errors, cleanup/regeneration command, and cutover tests. |
| Workflow version = topology + node versions | Medium-low. Node contracts have optional `contract_version` and `contract_digest`, but graph fingerprints do not use them. | Node registry, workflow nodes, graph persistence, diagnostics timing, frontend graph types, mocks. | Needs immutable contract projection and README/ADR ownership update. | Node code changes may be invisible to diagnostics if version data is absent. | Replace active execution identity with workflow execution fingerprint and require node version facts before downstream stages. |
| Semantic version plus fingerprint strictness | Low. No workflow version registry or uniqueness constraint exists. | Workflow service storage, diagnostics, attribution, frontend API types. | Requires durable repository and cutover plan. | Conflicting semver/fingerprint could silently pollute diagnostics without strict constraint. | Implement registry before diagnostics filtering; reject conflicts at submission. |
| Queued runs immutable with version references | Low. Queue items store in-memory request fields only. | Scheduler queue, edit-session run path, attribution, diagnostics, retention. | Needs run snapshot owner and transaction ordering. | Editable graph changes after queueing could be mixed into the wrong run. | Resolve version and persist snapshot before enqueue; queue stores snapshot id. |
| Scheduler-first dense global run list | Low. Current scheduler is session-scoped and in-memory. | Tauri commands, frontend workflow service, diagnostics store, scheduler components. | Needs backend read model instead of component reconstruction. | GUI may show incomplete or current-session-only history. | Add global scheduler run projection backed by durable run/scheduler stores. |
| Pre-run estimates and scheduler events | Medium-low. Runtime requirements/capabilities exist, but no typed event ledger. | Scheduler policy, runtime lifecycle, model load/unload, diagnostics, UI. | Needs event ownership, typed payload validation, replay/idempotency tests, non-blocking writes. | Scheduler latency, deadlocks, or unvalidated event drift. | Emit typed events outside critical locks through narrow backend event builders. |
| Typed diagnostic event ledger | Low. Diagnostics storage exists, but no typed append-only event envelope or allowlisted event families exist. | Scheduler, workflow service, embedded runtime, node execution, diagnostics ledger, Pumas/Library wrappers, projections. | Needs strict schema-versioned payloads, validation, source ownership, and projection rebuild tests. | Free-form metadata or raw event APIs can weaken security and make future queries unreliable. | Use flexible storage shape with strict typed write contract and rebuildable projections. |
| I/O Inspector with retention policy | Low. Bindings carry payloads, but no typed artifact event/projection model exists. | Node execution, diagnostics ledger, storage cleanup, frontend media rendering. | Needs security/storage policy and cleanup lifecycle docs. | Large payload persistence can create disk, privacy, and performance regressions. | Store metadata and bounded artifacts by policy; retain audit metadata after payload deletion. |
| Pumas/Library audit | Medium-low. Pumas helpers and managed runtime surfaces exist, but access is not centrally audited. | Pumas command helpers, dependency resolution, model install/delete/search, scheduler estimates. | Needs centralized typed audit wrapper to avoid partial coverage. | Library usage metrics become incomplete or misleading. | Route Library/Pumas actions through typed audited service boundary. |
| Network page with local node stats | Low. Managed runtimes exist, but no local compute/network-node projection. | Runtime manager, app setup, platform metrics dependencies, future Iroh. | Needs cross-platform dependency review and privacy constraints. | Platform-specific failures if metrics code is wired directly into pages. | Add optional backend system-stats provider with degraded states. |
| Page-based workbench shell | Medium. `App.svelte` is the shell owner, but still uses canvas/workflow modes. | Keyboard shortcuts, diagnostics lifecycle, graph context, drawing UI, generated component tools. | Needs frontend boundary split and README updates. | Mixed old/new navigation if replaced only partially. | Build one workbench shell and explicitly relocate or retire old surfaces. |

## Current Compatibility Notes

### Identity And Persistence

Workflow identity is currently loose in more than one place:

- `validate_workflow_id` accepts any non-empty string
  (`crates/pantograph-workflow-service/src/workflow/validation.rs`).
- Filesystem persistence converts unsupported name characters to underscores
  and allows spaces in filenames
  (`crates/pantograph-workflow-service/src/graph/persistence.rs:146`).
- Loading/listing derives `metadata.id` from the file stem
  (`crates/pantograph-workflow-service/src/graph/persistence.rs:221`).
- Capability lookup has a separate workflow stem sanitizer
  (`crates/pantograph-workflow-service/src/capabilities.rs`).

Compatibility concern: strict identity validation will affect saved workflow
files, user-visible names, workflow capability lookup, and active graph ids.
Because legacy support is not required, Stage `01` should replace scattered
validation with one domain identity type and explicitly invalidate, delete, or
regenerate files that do not pass the new grammar.

### Graph Fingerprints And Execution Versions

The current graph fingerprint intentionally ignores display metadata and hashes
node id, node type, and edge endpoints using FNV-1a:

- backend calculation:
  `crates/pantograph-workflow-service/src/capabilities.rs`
- frontend calculation:
  `packages/svelte-graph/src/graphRevision.ts:19`

That fingerprint is already a revision token for graph sync and diagnostics:

- `src/stores/diagnosticsStore.ts` clears timing history when graph
  fingerprint changes.
- `src/components/WorkflowGraph.svelte` uses graph fingerprint to sync
  SvelteFlow state.
- timing observations and run summaries are keyed by `graph_fingerprint`.
- frontend tests and mock backends assert the current fingerprint shape.

Cutover concern: the planned execution version cannot remain semantically
merged with `graph_fingerprint`. Stage `01` should split the concept into
explicit fields, for example:

- `graph_topology_fingerprint` for graph revision
  synchronization
- `execution_fingerprint` for workflow version identity
- `workflow_version_id` for diagnostics and run snapshot grouping
- `presentation_revision_id` for layout/display history

### Node Version Facts

`pantograph-node-contracts` already has optional `contract_version` and
`contract_digest` on `NodeTypeContract`
(`crates/pantograph-node-contracts/src/lib.rs:413`). That is a useful hook,
but it is not yet sufficient for strict workflow versioning because current
workflow graph DTOs and frontend `NodeDefinition` do not carry required node
semantic versions or behavior fingerprints.

Cutover concern: making node versions mandatory will affect node definition
producers, workflow-node descriptors, graph persistence, frontend types, mocks,
and tests. Stage `01` should either require node version/digest facts at the
new boundary or reject nodes that cannot provide them; later stages should not
carry fallback node identity logic.

### Scheduler And Queue State

The scheduler is currently session-local and in-memory. `enqueue_run` generates
a run id and stores request data directly in the queue. The run waits for
admission in a polling loop and then executes the queued request. This is
compatible with a session diagnostics panel, not with the planned Scheduler
page that lists future, queued, running, and historic runs across sessions.

Compatibility concern: widening the existing queue DTO will not be enough.
The workbench needs durable global read models:

- run snapshot record
- scheduler estimate record
- typed scheduler event record
- run status projection
- run action authority record

These can be populated from existing scheduler paths, but should not be
stored only inside `WorkflowExecutionSessionStore`.

Scheduler event records should use the typed diagnostic event ledger pattern:
allowlisted event kinds, schema-versioned Rust payloads, backend-only event
builders, source validation, privacy/retention classes, and projections for
normal GUI consumption.

### Typed Diagnostic Events

The current diagnostics ledger has durable timing and usage tables, but it does
not yet have a general typed event envelope. The new architecture should avoid
both extremes: a bespoke table/API for every new fact, and unvalidated
arbitrary JSON.

Compatibility concern: future diagnostics should be additive without weakening
security. Stage `03` should introduce a typed append-only event ledger with
strict backend-owned writers, validated payload schema versions, source
ownership, retention/privacy classification, payload size/ref rules, and
rebuildable projections. Raw event rows should not become the normal page API.

### Diagnostics And DTO Drift

The Rust diagnostics scheduler snapshot includes scheduler diagnostics
(`src-tauri/src/workflow/diagnostics/types.rs:302`), but the frontend
`DiagnosticsSchedulerSnapshot` does not expose a matching `diagnostics` field
(`src/services/diagnostics/types.ts:154`). This is a small current drift and a
warning sign for the larger workbench projection work.

Compatibility concern: adding more page projections manually in parallel Rust
and TypeScript types will increase drift risk. At minimum, each new projection
needs paired Rust tests and frontend type/normalization tests. A generated or
schema-checked DTO path should be considered before the projection surface
grows.

### Frontend Shell

`App.svelte` starts diagnostics, loads generated components, restores the last
graph, registers global shortcuts, and switches between drawing and workflow
views. The workbench can use this as a migration point, but the change affects
systems not directly related to the new pages:

- drawing-to-Svelte UI and hot-loaded component workspace
- global undo/redo shortcuts
- graph context initialization
- diagnostics subscription lifecycle
- graph session restore
- `SidePanel`, `TopBar`, and current canvas-specific overlays

Cutover concern: replacing `App.svelte` in one implementation stage can tangle
unrelated drawing, graph, diagnostics, and shortcut behavior. The app shell
should introduce one workbench boundary, then explicitly relocate or retire
current canvas and graph surfaces instead of keeping parallel mode semantics.

## Systems Affected Outside Direct Feature Edits

The implementation will affect these non-obvious systems:

- `packages/svelte-graph`: graph revision, workflow types, mock backend,
  connection intent, tests.
- `crates/pantograph-node-contracts`: node version/digest as execution
  identity inputs.
- `crates/workflow-nodes`: descriptors need version/digest population.
- `crates/pantograph-workflow-service`: graph persistence, session queue,
  workflow host traits, scheduler store, graph edit sessions.
- `crates/pantograph-diagnostics-ledger`: timing observations, retention,
  run summaries, typed event envelope, event validation, projection rebuilds,
  artifact projections, and audit projections.
- `crates/pantograph-runtime-attribution`: workflow run records must reference
  workflow versions and scheduler/admin actions.
- `src-tauri/src/workflow`: command registration, diagnostics projection,
  Pumas helpers, runtime diagnostics transport.
- `src/services`: workflow/diagnostics service surfaces should split rather
  than become one large workbench service.
- `src/stores`: active-run selection may be local, but durable run/scheduler
  data must remain backend-owned.
- `src/components/diagnostics`: existing session panels need replacement or
  conversion to run/version-aware views.
- app setup and shutdown: any background pruning/event replay tasks need
  explicit lifecycle ownership.

## Standards Compliance Risks

### Architecture Patterns

Backend-owned data standards require the frontend to render scheduler, run,
retention, version, and audit facts from backend projections. A mock-first page
implementation would violate that boundary unless clearly limited to
non-authoritative prototyping.

Mitigation: implement contracts/read models before page claims, and keep
frontend stores limited to page selection, active run id, filters, and
presentation state.

### Security Standards

Workflow identity validation is currently scattered. Strict identity,
Library/Pumas operations, and I/O artifact retention all need boundary
validation and centralized path/resource checks.

Mitigation: add domain validators and audited service wrappers instead of
inline regex/path checks in command handlers or Svelte services.

### Concurrency Standards

Scheduler event writes, run snapshot writes, estimate calculation, retention
cleanup, and background pruning all touch async execution paths. Risk increases
if new persistence writes happen while scheduler/session locks are held.

Mitigation: copy immutable event facts while under lock, release the lock, then
persist. Background tasks need tracked startup/shutdown, cancellation, and
replay/idempotency tests.

### Frontend Standards

The future Scheduler table and I/O gallery will probably need subscriptions or
event-driven refresh. Global high-frequency polling would violate frontend
synchronization standards.

Mitigation: use backend events or scoped refresh requests with deterministic
cleanup. Add timer cleanup tests only where polling is unavoidable.

### Documentation Standards

Implementation will create or reshape source directories and durable
contracts. Standards require README or ADR updates for ownership changes.

Required documentation during implementation:

- ADR for workflow version registry and semantic-version/fingerprint
  strictness.
- ADR or README decision for typed diagnostic event ledger ownership.
- README updates for new workflow-service modules and diagnostics ledger
  schema owners.
- README updates for frontend workbench feature directories.
- API consumer and structured producer contracts for new projection modules.

### Testing Standards

This work crosses Rust backend, Tauri transport, TypeScript services, Svelte
stores, and UI components. Typecheck and unit tests alone are not enough.

Required acceptance checks:

- submission creates a workflow version and immutable run snapshot before
  queueing
- queued run remains tied to old version after graph edit
- scheduler estimate/event appears in global Scheduler projection
- diagnostics filter by workflow version and do not mix graph topology
  fingerprints with execution identity
- I/O artifact metadata remains after payload retention cleanup
- frontend active-run selection drives Diagnostics, Graph, I/O, and Library
  projections from backend data

## Likely Refactors

These refactors are likely required for standards compliance. They should be
planned as implementation work, not deferred as cleanup after pages exist.

1. Add domain value types for workflow identity, workflow version id,
   execution fingerprint, presentation revision id, and scheduler event id.
2. Replace graph revision/execution identity overloads in backend and frontend
   DTOs.
3. Introduce a repository boundary for workflow versions and run snapshots.
4. Add typed diagnostic event ledger and scheduler/run read-model projections
   separate from the in-memory session queue.
5. Split frontend workflow service methods by projection area if
   `WorkflowService.ts` starts accumulating Scheduler, I/O, Library, Network,
   and Node Lab APIs.
6. Replace manual Rust/TypeScript DTO drift with either generated contracts or
   focused contract tests for every new projection.
7. Move workbench shell state into a dedicated feature boundary while
   relocating or retiring current drawing and workflow surfaces.
8. Centralize Pumas/Library audit entrypoints through typed event builders
   instead of auditing individual commands opportunistically.
9. Add retention cleanup ownership and lifecycle docs before storing I/O
   artifacts.

## Regression Risks

### High Risk

- Leaving old `graph_fingerprint` semantics active beside new execution
  fingerprints.
- Tightening workflow identity validation without deleting, ignoring, or
  regenerating invalid saved workflow ids and filenames.
- Queueing runs before durable version/snapshot writes succeed.
- Storing full node I/O payloads by default without size, privacy, or
  retention controls.
- Accepting arbitrary diagnostic metadata or raw event writes from feature code.
- Writing scheduler events while holding scheduler/session locks.
- Replacing the root app shell in one pass.

### Medium Risk

- Manual DTO additions drifting between Rust and TypeScript.
- Treating edit-session runs and headless workflow-session runs as identical
  before their queue/version paths are unified.
- Adding Network/system metrics directly to frontend pages instead of backend
  projections.
- Auditing only visible Library UI actions while background dependency
  resolution and scheduler access skip the audit path.
- Allowing GUI admin actions to share normal client queue mutation endpoints
  without explicit authority modeling.

### Lower Risk

- Adding active-run selection as transient frontend state.
- Adding placeholder Network and Node Lab route slots if they do not claim
  backend-owned facts that do not yet exist.
- Retiring the current drawing UI through an explicit shell cutover.

## Recommended Plan Changes

1. Add an explicit breaking cutover milestone before Stage `01`
   implementation: replace overloaded `graph_fingerprint` semantics with
   explicit topology, execution, workflow-version, and presentation-revision
   fields.
2. Add a "projection contract gate" before Stage 04: each new backend
   projection must have matching TypeScript type coverage and at least one
   cross-layer test or fixture.
3. Add a "typed event gate" before Stage `03` implementation: every new
   diagnostic event kind needs a typed payload, schema version, allowed source
   owner, validation tests, privacy/retention classification, and projection
   ownership.
4. Add a "ledger bootstrap gate" before durable Stage `02` scheduler event
   persistence: scheduler event producers may be designed first, but durable
   persistence must use the shared typed event ledger append/validation
   boundary.
5. Add a "scheduler persistence gate" before Stage 06: the Scheduler page must
   consume a durable/global run projection, not the existing session queue
   snapshot.
6. Add a "payload retention gate" before I/O Inspector implementation:
   artifact metadata, size caps, retention cleanup lifecycle, and redaction
   policy must be defined first.
7. Add a "shell cutover gate" before Stage `05`: current canvas and graph
   surfaces must be explicitly relocated into the workbench or retired with
   tests/docs updated.

## Conclusion

Implementation is feasible, but only if treated as a staged architecture
change. The broadest effects are not the new pages themselves; they are the new
identity, versioning, run snapshot, typed diagnostic event, retention, and
audit contracts that the pages require. The cleanest path is a Stage `01`
breaking contract cutover, a Stage `03` typed event ledger/projection cutover,
durable read models next, frontend shell/pages after that, with cross-layer
verification at each boundary.
