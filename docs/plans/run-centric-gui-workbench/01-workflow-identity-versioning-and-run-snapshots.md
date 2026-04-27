# 01: Workflow Identity, Versioning, And Run Snapshots

## Status

In progress. First identity-validation slice implemented. Workflow identity
grammar and saved-graph boundary validation now exist before broader
workflow-version registry work.

## Objective

Introduce backend-owned workflow identity, workflow execution versioning,
presentation revisions, semantic version/fingerprint conflict enforcement, and
immutable run snapshots so queued and historic runs can be audited without
storing duplicate graphs for unchanged workflow versions.

## Scope

### In Scope

- Stable validated workflow identity.
- Workflow semantic versions.
- Workflow execution fingerprints based on normalized executable topology and
  node versions.
- Strict rejection when semantic version and execution fingerprint disagree.
- Mandatory node semantic version and behavior fingerprint capture for
  executable workflow versions.
- Presentation revision model for display metadata.
- Immutable queued/future run snapshot contracts.
- Stable event correlation identifiers required by the typed diagnostic event
  ledger.
- Breaking cutover strategy for existing workflow/run records that do not meet
  the new identity/version invariants.
- Diagnostics filter keys for workflow/node versions.

### Out of Scope

- Full frontend page implementation.
- Scheduler estimates/events beyond fields needed in the run snapshot.
- Retention cleanup behavior.
- Iroh distributed execution.
- Node Lab authoring semantics.

## Inputs

### Problem

Diagnostics must not mix data from different executable workflow/node
conditions as if they were the same. A run must reference the workflow version,
node versions, model choices, runtime versions, scheduler policy, retention
policy, graph settings, and inputs that existed when the run was queued.

### Constraints

- Workflow version identity is graph topology plus node versions.
- Model choice, runtime choice, scheduler policy, inputs, priority, and target
  device are run context, not workflow-version identity.
- Node placement and display metadata do not affect diagnostics identity.
- Queued runs are immutable.
- Run snapshot identifiers, workflow version identifiers, and node version
  identifiers become required correlation fields for typed diagnostic events
  where the event is run-scoped.
- The server may accept the client/user-supplied stable workflow identity and
  semantic version label, but it must compute execution fingerprints itself and
  reject semantic-version/fingerprint conflicts.
- The backend must not claim to infer correct semantic-version intent
  (`major`, `minor`, `patch`) from graph changes unless a later dedicated
  policy is designed. Fingerprints are the correctness identity; semantic
  versions are explicit labels constrained by fingerprints.

### Assumptions

- Backwards compatibility is not required for existing saved workflow files,
  diagnostics history, or run records.
- Existing records that cannot satisfy the new workflow identity and version
  invariants may be deleted, ignored, or regenerated during the cutover.
- Node behavior fingerprints are projected from node contracts through an
  explicit `NodeBehaviorVersion` fact. Producers may provide a digest or allow
  the contract crate to derive one from the typed contract payload, but
  executable nodes must carry semantic contract versions before they can
  participate in versioned execution.

### Dependencies

- `pantograph-workflow-service` workflow/run contracts.
- `pantograph-runtime-attribution` workflow/run identity.
- `pantograph-node-contracts` node identity and contract metadata.
- `pantograph-diagnostics-ledger` run summaries and query filters.
- `diagnostic-event-ledger-architecture.md` event correlation requirements.
- Existing graph persistence/session loading code.

### Risks

| Risk | Impact | Mitigation |
| ---- | ------ | ---------- |
| Hashing non-canonical graph payloads creates false versions. | High | Define canonical executable topology normalization and test ordering-insensitive cases. |
| Old workflow/run records are accidentally mixed with new versioned data. | High | Use a breaking schema/contract cutover and reject or purge records that cannot satisfy new invariants. |
| Semantic versioning is treated as authoritative without fingerprints. | High | Enforce fingerprints as correctness identity and reject conflicts. |
| Presentation revisions are accidentally used for diagnostics grouping. | Medium | Keep execution version and presentation revision as separate DTO fields and query filters. |
| Event producers cannot correlate diagnostics to immutable run/version context. | High | Resolve run snapshot and workflow/node version ids before queue insertion and expose them to typed event builders. |
| The system appears to infer semantic version correctness automatically. | Medium | Treat semantic versions as explicit labels and fingerprints as correctness identity; reject conflicts rather than guessing major/minor/patch. |
| Missing node version facts are accepted into executable fingerprints. | High | Reject or quarantine nodes without behavior version facts before queueing versioned runs. |

## Definition of Done

- Workflow identity validation exists at submission boundaries.
- Workflow execution fingerprints are computed by backend code from normalized
  executable topology and node versions.
- Workflow semantic version/fingerprint conflicts are rejected with explicit
  errors.
- Semantic version labels are not auto-classified as major/minor/patch by
  heuristics unless a later documented policy explicitly owns that behavior.
- Executable workflow versions do not accept silent unknown node-version
  placeholders.
- Queued runs reference workflow execution version immediately.
- Run snapshots include versioned workflow/node/model/runtime/policy/context
  fields needed for auditing.
- Run snapshots provide the stable correlation fields used by typed diagnostic
  event envelope validation.
- Historic graph view consumers can request the workflow version used by a run.
- Old graph/run/fingerprint assumptions are removed or isolated behind explicit
  cutover cleanup.
- Existing tests and new version/cutover tests pass for touched crates.

## Implementation Wave Split

Stage `01` has repo-wide blast radius and should not be implemented as one
large patch. Before coding, split it into waves with explicit write sets:

1. Workflow identity grammar and boundary validation.
2. Mandatory node semantic version and behavior digest facts.
3. Replacement of old active `graph_fingerprint` execution semantics with
   topology, execution, workflow-version, and presentation-revision fields.
4. Workflow version registry and semantic-version/fingerprint conflict
   rejection.
5. Immutable run snapshot ledger and queue submission cutover.
6. Diagnostics, attribution, adapter, frontend DTO, mock, template, and test
   cutover.
7. Source audit proving old fields are removed or quarantined with documented
   non-execution meaning.

## Milestones

### Milestone 1: Contract And ADR Freeze

**Goal:** Freeze terminology and ownership before schema/API changes.

**Tasks:**

- [ ] Decide whether to add a new ADR for workflow version registry ownership.
- [x] Define workflow identity validation rules and error categories.
- [x] Define canonical executable topology inputs and exclusions.
- [ ] Define presentation revision contract and its relationship to execution
  versions.
- [ ] Define run snapshot fields and breaking cutover cleanup strategy.
- [ ] Define run snapshot and workflow/node version fields that event payload
  validation will require for run-scoped diagnostic events.
- [ ] Define replacement field names for old overloaded graph revision and
  execution identity concepts.

**Verification:**

- Contract tests or compile-only DTO tests cover required fields and error
  variants.
- Documentation links back to the requirements file and this stage.

**Status:** In progress. Workflow identity grammar has a first implementation
in `pantograph-workflow-service`, and node behavior version facts are now
available from `pantograph-node-contracts`. Canonical executable topology
projection is implemented, and workflow-version registry ownership now sits in
the durable attribution store. Remaining Milestone 1 snapshot and presentation
revision decisions are still open.

### Milestone 2: Workflow Version Registry

**Goal:** Add backend registry behavior for workflow identity, semantic
version, execution fingerprint, and presentation revision lookup.

**Tasks:**

- [ ] Add canonical fingerprint generation for executable graph topology.
- [x] Store or project workflow versions under stable workflow identity.
- [x] Reuse an existing workflow version when identity and fingerprint match.
- [x] Reject same semantic version with different fingerprint.
- [ ] Track presentation revisions separately from execution versions.
- [ ] Replace old active diagnostics grouping keys with workflow execution
  version ids.

**Verification:**

- Unit tests cover same topology with different JSON ordering.
- Unit tests cover node-version changes creating a new execution version.
- Unit tests cover display-only changes not changing execution version.
- Unit tests cover semantic version conflict rejection.

**Status:** In progress. Durable attribution storage now owns
workflow-version records and strict semantic-version/fingerprint conflict
checks; diagnostics and presentation revision cutover remain open.

### Milestone 3: Run Submission And Immutability

**Goal:** Ensure submitted future/queued runs immediately reference immutable
workflow version and run context.

**Tasks:**

- [x] Update run submission path to resolve or create workflow version before
  queue insertion.
- [ ] Attach run snapshot fields for model/runtime choices, graph settings,
  scheduler policy, retention policy, session, bucket, and immutable input
  references.
- [ ] Make run snapshot id, workflow version id, node version set, client,
  session, bucket, and policy ids available to typed diagnostic event builders.
- [ ] Make update-before-execution behavior explicit: cancel and resubmit.
- [ ] Preserve scoped client cancellation rules from existing attribution work.

**Verification:**

- Integration tests cover queued run retaining original version after the
  editable workflow changes.
- Tests cover clone/resubmit producing a new run instead of mutating the old
  run.

**Status:** In progress. Queued session runs now resolve workflow versions and
persist snapshots before scheduler admission. Public run contracts now require
the caller to provide `workflow_semantic_version`; remaining work is to fill the
full audit context and event-builder correlation fields.

### Milestone 4: Diagnostics And Graph Consumers

**Goal:** Make downstream consumers use workflow execution versions without
  relying on frontend repair.

**Tasks:**

- [ ] Add workflow/node version filters to diagnostics query contracts.
- [ ] Make diagnostics query contracts consume projections derived from typed
  event ledger correlation fields rather than raw mutable graph state.
- [ ] Add run-to-workflow-version projection for Graph page consumers.
- [ ] Remove or quarantine old graph-fingerprint-only diagnostics grouping.
- [ ] Update READMEs for changed host-facing contracts.

**Verification:**

- Diagnostics query tests cover workflow-version filtering.
- Graph projection tests cover historic run version lookup.

**Status:** Not started.

## Public Facade Preservation Note

Prefer contract-first replacement over additive compatibility. Existing
workflow/run facades may be deleted or renamed when they carry ambiguous
identity semantics. Any API that returns a current workflow graph for a
historic run must be replaced before the Graph page depends on it.

## Ownership And Lifecycle Note

Run submission owns the version-resolution transaction boundary for Stage
`01`. Resolving or creating the workflow execution version, creating the
immutable run snapshot, and inserting the queued run must be treated as one
durable operation or an explicitly idempotent sequence with tested recovery.

Implementation must document the storage owner, lock ordering, cancellation
safety, and retry behavior before queue insertion is changed. A cancelled or
failed submission must not leave an executable queued run without a matching
workflow version, node version set, and run snapshot. A retry of the same
submission must either reuse the same resolved workflow version or return a
clear validation/conflict error; it must not create duplicate semantic-version
records for the same execution fingerprint.

## Re-Plan Triggers

- Existing persisted workflow data cannot be cleanly deleted, ignored, or
  regenerated during cutover.
- Node version/fingerprint data is not available from node contracts.
- Workflow identity validation conflicts with the desired stable identity
  grammar.
- A storage owner cannot be chosen cleanly between workflow service and
  diagnostics ledger.

## Completion Summary

### Completed

- 2026-04-27: Stage-start gate completed for the first Stage `01` slice.
  Dirty files existed outside the write set (`.pantograph/` workflow output,
  diagnostics SQLite, and `assets/` files), but no dirty source/test/config
  files overlapped the selected workflow-service identity slice.
- 2026-04-27: Added `WorkflowIdentity` grammar in
  `pantograph-workflow-service`, routed `validate_workflow_id` through it, and
  changed filesystem workflow save/load/list/delete boundaries to reject or
  skip incompatible workflow file stems instead of silently sanitizing names.
- 2026-04-27: Added `NodeBehaviorVersion` in
  `pantograph-node-contracts`, made `NodeTypeContract::validate` require
  semantic contract versions and BLAKE3 behavior digests, and updated
  built-in contract producers plus legacy migration metadata to semantic
  versions.
- 2026-04-27: Added `WorkflowExecutableTopology` in
  `pantograph-workflow-service`. Execution fingerprints now have an explicit
  BLAKE3 projection that includes sorted node ids, node types, node behavior
  versions, and sorted port connections while excluding node positions, node
  data, edge ids, derived graph caches, and display metadata.
- 2026-04-27: Added durable workflow-version records to
  `pantograph-runtime-attribution` and a `WorkflowService`
  `resolve_workflow_graph_version` facade. The registry reuses matching
  workflow id/fingerprint/version rows and rejects both semantic-version and
  execution-fingerprint disagreements.
- 2026-04-27: Added durable immutable workflow-run snapshot storage to
  `pantograph-runtime-attribution`. Snapshots capture the workflow run id,
  workflow version id, semantic version, execution fingerprint, execution
  session id, priority, timeout, serialized inputs, output targets, and runtime
  override selection.
- 2026-04-27: Changed queued workflow execution session submission to generate
  the backend run id before enqueue and record the workflow version/run
  snapshot before scheduler admission when attribution storage is configured.
- 2026-04-27: Added explicit `workflow_semantic_version` to generic and
  session run request contracts, validate it at the workflow-service boundary,
  and use it for queued run snapshot version resolution instead of a temporary
  default.
- 2026-04-27: Expanded workflow-run snapshots with session kind, usage
  profile, keep-alive state, retention policy, and scheduler policy facts
  currently owned by the session scheduler.
- 2026-04-27: Added typed optional workflow-version id and semantic-version
  correlation fields to the diagnostics usage ledger, plus workflow-service
  query filters for those fields. Node version filters remain pending.

### Deviations

- The first run-snapshot storage contract captures the queue/session fields
  available today. Full model/runtime, graph-settings, client, and bucket
  fields still need to be filled during queue cutover.
- Workflow-version registry ownership is implemented in the attribution store
  without a standalone ADR. The choice is documented here and in crate READMEs
  because the registry must share the future run snapshot transaction boundary.

### Follow-Ups

- Define presentation revision storage and API fields.
- Define run snapshot schema and queue submission transaction wiring.

### Verification Summary

- 2026-04-27: Initial combined test command
  `cargo test -p pantograph-workflow-service identity persistence_tests --lib`
  was malformed because Cargo accepts one test filter before `--`; reran the
  filters separately.
- 2026-04-27: `cargo test -p pantograph-workflow-service --lib identity`
  passed.
- 2026-04-27: `cargo test -p pantograph-workflow-service --lib
  persistence_tests` passed.
- 2026-04-27: `cargo test -p pantograph-workflow-service` passed.
- 2026-04-27: `cargo test -p pantograph-node-contracts` passed.
- 2026-04-27: `cargo test -p workflow-nodes` passed.
- 2026-04-27: `cargo test -p pantograph-workflow-service canonicalization`
  passed.
- 2026-04-27: `cargo test -p pantograph-workflow-service` passed after the
  semantic node contract-version cutover.
- 2026-04-27: `cargo test -p pantograph-workflow-service
  executable_topology` passed.
- 2026-04-27: `cargo test -p pantograph-workflow-service` passed after adding
  executable topology contracts.
- 2026-04-27: `cargo test -p pantograph-runtime-attribution` passed after
  adding workflow-version registry storage.
- 2026-04-27: `cargo test -p pantograph-workflow-service workflow_version`
  passed.
- 2026-04-27: `cargo test -p pantograph-runtime-attribution` passed after
  adding workflow-run snapshot storage.
- 2026-04-27: `cargo test -p pantograph-workflow-service
  workflow_execution_session_run_records_snapshot_before_execution` passed.
- 2026-04-27: `cargo test -p pantograph-workflow-service` passed after making
  `workflow_semantic_version` an explicit generic/session run request field.
- 2026-04-27: `cargo check -p pantograph-frontend-http-adapter -p
  pantograph_rustler -p pantograph-uniffi -p pantograph-embedded-runtime`
  passed after updating adapter and embedded runtime request construction.
- 2026-04-27: `cargo test -p pantograph-runtime-attribution` passed after
  adding scheduler/session context fields to workflow-run snapshots.
- 2026-04-27: `cargo test -p pantograph-workflow-service` passed after
  projecting scheduler/session context into queued run snapshots.
- 2026-04-27: `cargo test -p pantograph-diagnostics-ledger` passed after
  adding workflow-version correlation fields and filters to usage diagnostics.
- 2026-04-27: `cargo test -p pantograph-workflow-service diagnostics` passed
  after exposing workflow-version diagnostics query filters through the
  workflow-service facade.
- 2026-04-27: `cargo check -p pantograph-embedded-runtime` passed after
  preserving embedded runtime usage-event construction with optional workflow
  version fields.

### Traceability Links

- Requirement sections: Workflow Identity Requirements, Workflow Versioning
  Requirements, Semantic Version and Fingerprint Requirements, Presentation
  Revision Requirements, Run Immutability and Audit Snapshot.
