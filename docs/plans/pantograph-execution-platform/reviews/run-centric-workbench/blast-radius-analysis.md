# Blast Radius Analysis

## Status

Draft source blast-radius review. Not an implementation plan replacement.

Last updated: 2026-04-27.

## Purpose

Re-examine the run-centric GUI workbench plans against the current codebase and
record how far each plan stage reaches. This file is intended to prevent
implementation from treating broad architectural changes as local feature work.

## Compatibility Policy

Backwards compatibility is not required for this workbench architecture change.
Existing saved workflow files, old diagnostics rows, old attribution rows, old
frontend graph fingerprints, and current canvas/workflow shell assumptions may
be invalidated, deleted, ignored, or regenerated when the clean contracts land.

The blast-radius concern is therefore not preserving old behavior. The concern
is avoiding a mixed architecture where old and new identity, scheduler,
diagnostics, and API meanings coexist in active code.

## Executive Assessment

The largest blast radius is Stage `01`: workflow identity, workflow versions,
node versions, execution fingerprints, and immutable run snapshots. It is
repo-wide because the current `graph_fingerprint` is used as a graph sync token,
diagnostics grouping key, run summary field, frontend derived graph field, mock
backend assumption, and test fixture value.

The second largest blast radius is the combined Scheduler/Diagnostics/API work
in Stages `02` through `04`. Current scheduler state is session-local and
in-memory, while the target workbench needs global queued/running/historic run
lists, pre-run estimates, typed model load/unload events, retention policy
state, I/O artifact metadata, and Library/Pumas audit records.

Stage `03` now has a clearer architectural shape: a typed append-only
diagnostic event ledger plus rebuildable projections. That reduces future
schema churn, but it expands the initial blast radius because scheduler,
runtime, node execution, Pumas/Library, retention cleanup, projections, and
tests must agree on the event envelope and validation rules.

The frontend shell and page work in Stages `05` and `06` should be large but
more controllable if the backend projections land first. If implemented before
the backend cutovers, the frontend will likely create local truth that has to be
removed later.

## Blast Radius Matrix

| Stage | Primary blast radius | Secondary blast radius | Regression risk | Required plan adjustment |
| --- | --- | --- | --- | --- |
| `00` Boundaries | Plan docs, ADR references, source README ownership notes. | None expected in source until implementation starts. | Low. The risk is unclear cutover language. | Keep no-compatibility and no-mixed-identity rules visible in every stage. |
| `01` Identity, versioning, run snapshots | `pantograph-workflow-service`, `pantograph-diagnostics-ledger`, `pantograph-runtime-attribution`, `pantograph-node-contracts`, `workflow-nodes`, `packages/svelte-graph`, Tauri workflow commands, frontend workflow/diagnostics services. | Embedded runtime, Rustler/UniFFI/frontend HTTP adapters, templates, mocks, contract tests. | Very high. Old graph fingerprint semantics can silently pollute new diagnostics and workflow-version records. | Split Stage `01` into implementation waves before coding: identity grammar, node version facts, execution fingerprint, version registry, run snapshot ledger, projection cutover, old-field audit. |
| `02` Scheduler estimates/events/control | Workflow service scheduler store/policy/contracts, session execution API, embedded runtime lifecycle/preflight, runtime registry, typed scheduler event builders. | Tauri workflow commands, frontend scheduler services, runtime manager surfaces, tests around queue behavior. | High. Event writes or estimates added inside existing locks can create deadlocks, scheduler latency regressions, or invalid event records. | Require scheduler event ownership, typed event validation, and lock-boundary design before source edits. |
| `03` Diagnostics, retention, audit ledgers | Diagnostics ledger schema/repository/SQLite rows, typed event envelope/builders, projection rebuilds, workflow trace/timing, node execution ledger, runtime attribution, Pumas/Library command wrappers. | Retention cleanup lifecycle, artifact storage paths, frontend diagnostics and I/O inspector types. | High. Payload retention can create disk/privacy problems; incomplete event validation can create unsafe arbitrary metadata or misleading metrics. | Add typed event ledger, artifact projections, audited Library boundary, and projection rebuild tests before page work. Keep payload retention separate from audit retention. |
| `04` API projections | Tauri command registration and DTOs, frontend HTTP adapter, Rustler/UniFFI bindings, frontend services/types/stores. | Tests across Rust and TypeScript, generated or manually mirrored DTOs, app setup. | Medium-high. Existing Rust/TypeScript drift already exists in diagnostics scheduler DTOs, and raw event rows could leak through page APIs. | Add DTO drift gate, projection contract tests, and raw-event API boundary before new page components depend on the APIs. |
| `05` App shell and active-run navigation | `src/App.svelte`, view mode store, workflow toolbar, side panel/top bar, diagnostics lifecycle, graph restore, drawing/canvas surfaces, shortcut handling. | Generated component workspace, hotload container, graph stores, global overlays. | Medium-high. Partial migration can leave two competing app shells and conflicting shortcuts. | Use one workbench shell boundary. Explicitly relocate or retire current canvas/workflow mode behavior. |
| `06` Run-centric pages | Scheduler table, Diagnostics, Graph, I/O Inspector, Library, Network, Node Lab route slots and services. | Existing diagnostics components, runtime manager components, node palettes, Pumas node UI, managed runtime services. | Medium. Page code can remain clean if it renders backend projections; high if it invents missing facts locally. | Keep page stores selection/filter-only and require backend projections before displaying authoritative data. |
| `07` Verification and rollout | Test commands, module READMEs, ADR updates, old-field search gates, worktree hygiene. | All implementation stages. | Medium. Without explicit audit gates, old identity and scheduler assumptions will remain. | Add mandatory source-search gates for old active fields and mixed route semantics. |

## Stage `01` Detailed Blast Radius

Stage `01` is the architectural foundation and should be treated as a breaking
contract cutover.

Current workflow identity is loose and scattered:

- `validate_workflow_id` only rejects empty strings in
  `crates/pantograph-workflow-service/src/workflow/validation.rs:17`.
- Filesystem persistence sanitizes workflow names independently and allows
  spaces in `crates/pantograph-workflow-service/src/graph/persistence.rs:146`.
- Loading derives `metadata.id` from the file stem in
  `crates/pantograph-workflow-service/src/graph/persistence.rs:221`.

Current execution identity is also overloaded:

- The frontend graph revision hash uses node id, node type, and edge endpoints
  in `packages/svelte-graph/src/graphRevision.ts:19`.
- The same derived graph field is exposed as `graph_fingerprint` in
  `packages/svelte-graph/src/graphRevision.ts:57`.
- Diagnostics timing rows store `graph_fingerprint` in
  `crates/pantograph-diagnostics-ledger/src/schema.rs:174`.
- Diagnostics run summaries store `graph_fingerprint` in
  `crates/pantograph-diagnostics-ledger/src/schema.rs:209`.

Node-version support is only partial:

- `NodeTypeContract` has optional `contract_version` and `contract_digest` in
  `crates/pantograph-node-contracts/src/lib.rs:428`.
- Workflow graph fingerprints do not require those fields, so current
  diagnostics cannot reliably distinguish node-code behavior changes.

Run attribution is too thin for the target audit model:

- Runtime attribution `workflow_runs` stores workflow id, client/session,
  bucket, status, and timestamps, but not workflow version, node versions,
  model/runtime versions, scheduler policy facts, or parameter facts in
  `crates/pantograph-runtime-attribution/src/schema.rs:82`.

Affected source areas:

- `crates/pantograph-workflow-service/src/graph/*`
- `crates/pantograph-workflow-service/src/workflow/*`
- `crates/pantograph-workflow-service/src/trace/*`
- `crates/pantograph-diagnostics-ledger/src/*`
- `crates/pantograph-runtime-attribution/src/*`
- `crates/pantograph-node-contracts/src/*`
- `crates/workflow-nodes/src/*`
- `crates/node-engine/src/*`
- `crates/pantograph-embedded-runtime/src/*`
- `crates/pantograph-rustler/src/*`
- `crates/pantograph-uniffi/src/*`
- `crates/pantograph-frontend-http-adapter/src/*`
- `packages/svelte-graph/src/*`
- `src/services/workflow/*`
- `src/services/diagnostics/*`
- `src/stores/*`
- `src/components/WorkflowGraph.svelte`
- `src/templates/workflows/*`

Recommended Stage `01` waves:

1. Add one workflow identity domain type and reject invalid identities at every
   workflow submission, save/load, API, and frontend service boundary.
2. Make node semantic version and behavior digest mandatory in node contracts
   used for executable workflows.
3. Replace `graph_fingerprint` as active execution identity with explicit
   topology, execution, workflow-version, and presentation-revision fields.
4. Add workflow version registry and strict semantic-version/fingerprint
   disagreement rejection.
5. Add immutable run snapshot records before enqueueing.
6. Cut over diagnostics and attribution schemas to workflow-version and
   run-snapshot identifiers.
7. Run a source audit that either removes old active `graph_fingerprint`
   semantics or quarantines the field as presentation/topology-only.

## Stage `02` Detailed Blast Radius

The current scheduler is session-local and in-memory:

- `enqueue_run` generates the run id and stores request fields directly in the
  queue in `crates/pantograph-workflow-service/src/scheduler/store_queue.rs:62`.
- Session execution polls for admission, preflights, loads the runtime, runs,
  and finishes in
  `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs:74`.

That is not enough for the target Scheduler page, which needs future queued
runs, historic runs, global/admin actions, client-scoped authority, estimates,
delay reasons, and model load/unload events.

Affected source areas:

- `crates/pantograph-workflow-service/src/scheduler/*`
- `crates/pantograph-workflow-service/src/workflow/session_execution_api.rs`
- `crates/pantograph-workflow-service/src/workflow/session_queue_api.rs`
- `crates/pantograph-workflow-service/src/technical_fit.rs`
- `crates/pantograph-embedded-runtime/src/embedded_runtime_lifecycle.rs`
- `crates/pantograph-embedded-runtime/src/runtime_registry.rs`
- `crates/pantograph-embedded-runtime/src/workflow_scheduler_diagnostics.rs`
- `crates/pantograph-runtime-registry/src/*`
- `src-tauri/src/workflow/workflow_execution_*`
- `src-tauri/src/workflow/headless_*`
- `src/services/workflow/*`
- `src/components/diagnostics/DiagnosticsScheduler.svelte`

Main design risk: scheduler event writes, estimate calculations, and runtime
load/unload recording must not run while holding the existing session-store
lock. The implementation plan needs an explicit lock-boundary rule and event
emission pattern before Stage `02` source work starts.

With the typed event ledger decision, scheduler events also need event-kind
allowlists, typed payloads, schema versions, source validation, retention and
privacy classes, and tests that reject malformed events.

## Stage `03` Detailed Blast Radius

Diagnostics and retention work affects more than diagnostics pages.

The diagnostics ledger already has a retention policy table and timing/run
summary tables, but there is no typed event envelope, event-kind allowlist,
payload schema-version model, I/O artifact metadata projection, or durable
workflow-version-aware query key. Timing rows are currently indexed around
workflow id and graph fingerprint, which is the wrong long-term key for
version-aware comparisons.

Affected source areas:

- `crates/pantograph-diagnostics-ledger/src/schema.rs`
- `crates/pantograph-diagnostics-ledger/src/records.rs`
- `crates/pantograph-diagnostics-ledger/src/repository.rs`
- `crates/pantograph-diagnostics-ledger/src/sqlite/*`
- typed event builder modules added during implementation
- projection rebuild modules added during implementation
- `crates/pantograph-workflow-service/src/trace/*`
- `crates/pantograph-workflow-service/src/workflow/diagnostics_api.rs`
- `crates/pantograph-workflow-service/src/workflow/io_contract.rs`
- `crates/pantograph-embedded-runtime/src/node_execution_ledger.rs`
- `crates/pantograph-embedded-runtime/src/node_execution_diagnostics.rs`
- `src-tauri/src/workflow/diagnostics/*`
- `src-tauri/src/workflow/puma_lib_commands.rs`
- `src-tauri/src/workflow/model_dependency_commands.rs`
- `crates/workflow-nodes/src/input/puma_lib.rs`
- `src/components/nodes/workflow/PumaLibNode.svelte`
- `packages/svelte-graph/src/components/nodes/PumaLibNode.svelte`

Main design risk: I/O payloads may be too large or sensitive to retain, while
audit metadata must remain queryable. Stage `03` should keep payload retention
policy, artifact metadata, and audit ledger retention as separate concepts.
It should also reject any diagnostic write path that tries to bypass typed
payload validation.

## Stage `04` Detailed Blast Radius

API projection work crosses Rust, Tauri, TypeScript, Rustler, UniFFI, and any
future HTTP adapter.

A current drift example exists now:

- Rust `DiagnosticsSchedulerSnapshot` includes `diagnostics` in
  `src-tauri/src/workflow/diagnostics/types.rs:302`.
- Frontend `DiagnosticsSchedulerSnapshot` does not expose that field in
  `src/services/diagnostics/types.ts:154`.

Affected source areas:

- `src-tauri/src/app_setup.rs`
- `src-tauri/src/workflow/commands.rs`
- `src-tauri/src/workflow/headless_workflow_commands.rs`
- `src-tauri/src/workflow/diagnostics/types.rs`
- `src/services/workflow/types.ts`
- `src/services/diagnostics/types.ts`
- `src/services/workflow/WorkflowService.ts`
- `src/stores/diagnosticsStore.ts`
- `crates/pantograph-frontend-http-adapter/src/lib.rs`
- `crates/pantograph-rustler/src/frontend_http_nifs.rs`
- `crates/pantograph-uniffi/src/frontend_http.rs`

Main design risk: manually maintained projection DTOs can drift as the API
surface grows. Stage `04` should require paired Rust/TypeScript projection
tests or a generated/schema-checked DTO path before page implementation.
Normal page APIs should expose ledger-derived projections, not raw typed event
rows. Raw event inspection should remain a separate privileged developer/admin
surface if implemented later.

## Stage `05` Detailed Blast Radius

The app shell currently mixes drawing UI, workflow UI, diagnostics startup,
graph restore, global shortcuts, and always-visible panels in one file:

- `src/App.svelte:62` starts diagnostics and restores generated workspace and
  graph state.
- `src/App.svelte:120` toggles between canvas and workflow modes.
- `src/App.svelte:144` renders the canvas surface.
- `src/App.svelte:153` renders the workflow graph surface and diagnostics
  panel.

Affected source areas:

- `src/App.svelte`
- `src/stores/viewModeStore.ts`
- `src/components/WorkflowToolbar.svelte`
- `src/components/WorkflowGraph.svelte`
- `src/components/diagnostics/DiagnosticsPanel.svelte`
- `src/components/SidePanel.svelte`
- `src/components/TopBar.svelte`
- canvas/drawing components imported by the current shell
- hotload/generated-component workspace services
- graph session stores and restore paths

Main design risk: a partial shell migration can leave old mode toggles,
shortcuts, and lifecycle hooks active under the new workbench. Stage `05`
should explicitly choose where the drawing-to-Svelte UI lives or retire it from
the default GUI path.

## Stage `06` Detailed Blast Radius

Page implementation should be frontend-heavy only after Stages `01` through
`04` have made backend facts authoritative.

Affected source areas:

- scheduler page components and run table state
- diagnostics page components under `src/components/diagnostics/*`
- graph page wrappers around `WorkflowGraph.svelte`
- new I/O Inspector components and artifact renderers
- Library/Pumas UI around existing Pumas nodes and runtime/model services
- Network page local-node statistics provider and future Iroh placeholder
- Node Lab route slot and disabled/future-state UI

Main design risk: pages can easily recreate scheduler, retention, Library, or
diagnostic truth locally. The plan should repeat that page stores may own only
selection, filters, sort, layout, and transient UI state.

## Stage `07` Required Source Gates

Before each stage is marked complete, the implementation should run source
audits matching the stage. The exact commands can evolve, but the gates should
cover these active-use checks:

- `graph_fingerprint`, `derived_graph`, `currentGraphFingerprint`, and
  `computeGraphFingerprint` after Stage `01`.
- `workflow_id`, workflow version ids, run snapshot ids, and semantic version
  conflict errors after Stage `01`.
- scheduler queue/event/estimate terminology after Stage `02`.
- retention, artifact, typed event builders, payload schemas, and Pumas/Library
  audit API usage after Stage `03`.
- Rust/TypeScript DTO projection fields after Stage `04`.
- `viewMode`, canvas/workflow mode toggles, and old shell shortcut ownership
  after Stage `05`.

These gates should distinguish accepted quarantined fields from active old
semantics. A field name may remain only if its new meaning is documented in the
owning module README and tests.

## Plan Adjustments Applied

- Stage `01` now includes an implementation-wave split before coding starts.
- Stage `02` now includes scheduler lock-boundary and event-emission rules.
- Stage `02` now depends on the shared typed event ledger bootstrap before
  durable scheduler event persistence and must not create a scheduler-specific
  event repository.
- Stage `03` now uses a typed diagnostic event ledger with strict backend-owned
  writers, validation, and rebuildable projections.
- Stage `04` now includes DTO drift verification gates.
- Stage `05` now includes shell cutover acceptance for old modes, shortcuts,
  and lifecycle hooks.
- Stage `06` now repeats that page stores must not own authoritative backend
  facts.
- Stage `07` now includes source-search gates for each broad contract cutover.

## Conclusion

The current plans can lead to a clean architecture if the implementation treats
the first four stages as contract and storage cutovers before page work. The
main architecture hazard is not regression against old data. The hazard is
leaving partially active old meanings beside the new workflow-version,
run-snapshot, scheduler-event, and diagnostics-retention contracts.
