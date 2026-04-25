# 03: Managed Runtime Observability

## Purpose

Route standard node execution through a runtime-created context so ordinary
nodes get diagnostics, attribution, cancellation, progress, and guarantee
classification without node-authored boilerplate.

## Implementation Readiness Status

Ready for stage-start preflight after stages `01` and `02` are complete and
their stage-end refactor gates have been recorded.

## Implementation Notes

### 2026-04-24 Stage-Start Report

- Selected stage: Stage `03`, managed runtime observability.
- Current branch: `main`.
- Stage base: `41f30685`, the Stage `02` closeout commit.
- Prior-stage gates: Stage `01` and Stage `02` coordination ledgers record
  complete wave integration and stage-end refactor gate outcomes of
  `not_warranted`.
- Git status before implementation: unrelated asset changes only:
  deleted `assets/3c842e69-080c-43ad-a9f0-14136e18761f.jpg`, deleted
  `assets/grok-image-6c435c73-11b8-4dcf-a8b2-f2735cc0c5d3.png`, deleted
  `assets/grok-image-e5979483-32c2-4cf5-b32f-53be66170132.png`,
  untracked `assets/banner_3.jpg`, `assets/banner_3.png`,
  `assets/github_social.jpg`, and `assets/reject/`.
- Dirty-file overlap: none. Stage `03` implementation must not touch
  `assets/`.
- Standards reviewed through the execution-platform standards map:
  `PLAN-STANDARDS.md`, `ARCHITECTURE-PATTERNS.md`,
  `CODING-STANDARDS.md`, `DOCUMENTATION-STANDARDS.md`,
  `TESTING-STANDARDS.md`, `CONCURRENCY-STANDARDS.md`,
  `TOOLING-STANDARDS.md`, `DEPENDENCY-STANDARDS.md`,
  `COMMIT-STANDARDS.md`, `languages/rust/RUST-API-STANDARDS.md`,
  `languages/rust/RUST-ASYNC-STANDARDS.md`, and
  `languages/rust/RUST-TOOLING-STANDARDS.md`.
- Intended Wave `02` write set:
  `crates/pantograph-embedded-runtime/src/` context, managed capability,
  diagnostics projection, cancellation/progress, guarantee, lifecycle, tests,
  README, and public facade exports when required.
- Adjacent write set only if required by existing call sites:
  `crates/node-engine/src/events/`, `crates/node-engine/src/engine/`, and
  `crates/pantograph-workflow-service/src/scheduler/`.
- Forbidden write set for this stage unless the plan is updated first:
  `crates/pantograph-diagnostics-ledger/`, host binding generation, GUI
  diagnostics views, and durable model/license ledger query or storage code.
- Runtime inventory:
  - `crates/pantograph-embedded-runtime/src/workflow_runtime.rs` currently owns
    workflow execution diagnostics snapshots, runtime lifecycle projection,
    model-target shaping, and registry reconciliation.
  - `crates/pantograph-embedded-runtime/src/runtime_capabilities.rs` currently
    maps runtime facts into workflow runtime capabilities but does not yet
    provide per-node managed capability handles.
  - `crates/pantograph-embedded-runtime/src/workflow_execution_session_execution.rs`
    owns keep-alive workflow executor reuse keyed by execution-session ids and
    must remain separate from durable client/session/bucket/run attribution.
  - `crates/node-engine/src/events/contract.rs` exposes low-level
    `WorkflowEvent` variants for workflow/task lifecycle, waiting-for-input,
    progress, stream, and graph mutation events.
  - `crates/node-engine/src/engine/execution_events.rs` emits demand-level
    task started, waiting-for-input, and completed events.
  - `crates/pantograph-workflow-service/src/scheduler/store_diagnostics.rs`
    owns scheduler queue/runtime-capacity diagnostics and remains
    backend-owned.
- Event adaptation decision:
  - Stage `03` adapts node-engine `WorkflowEvent` task lifecycle, progress,
    waiting-for-input, stream, and graph mutation facts as low-level execution
    inputs when available.
  - The embedded runtime owns enriched node execution diagnostics, attribution
    context, guarantee classification, managed capability routing facts,
    cancellation/progress handles, and lineage projection.
  - Node-engine events must not become the canonical owner of durable
    attribution, compliance meaning, guarantee policy, or host binding
    projections.
  - Missing runtime-owned baseline events are added at the embedded-runtime
    wrapper/context boundary rather than by requiring ordinary node code to
    emit diagnostics manually.
- Durable ledger boundary: Stage `03` may define transient runtime facts and
  Stage `04` ledger-facing DTOs/traits if needed, but it must not implement
  durable model/license ledger persistence or queries.
- Start outcome: `ready_with_recorded_assumptions`.
- Recorded assumptions:
  - Wave `02` may be executed serially by the host in this shared workspace
    when subagents are not explicitly authorized; the recorded worker write
    sets and reports still apply.
  - The first logical implementation step is the
    `runtime-context-capabilities` slice: add runtime-created
    `NodeExecutionContext`, execution input/output/error/result contracts,
    managed capability traits, lineage context, and focused tests in
    `pantograph-embedded-runtime`.
  - No new third-party dependency is expected for the first slice. If a
    dependency becomes necessary, stop and record dependency-standard review
    before editing manifests.
  - Public facade exports and workspace manifests are host-owned and will be
    edited only when needed to expose implemented runtime contracts.
- Expected verification for the first logical step:
  `cargo test -p pantograph-embedded-runtime`,
  `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-embedded-runtime --all-targets -- -D warnings`.
- Expected Stage `03` verification remains the command set listed in
  `Verification Commands`.

### 2026-04-24 Wave 02 Runtime Context Capabilities Progress

- Added `crates/pantograph-embedded-runtime/src/node_execution.rs`,
  `node_execution_capabilities.rs`, and `node_execution_tests.rs` as the
  embedded-runtime-owned context, managed capability, and focused test
  modules.
- Added crate-local path dependencies on `pantograph-node-contracts` and
  `pantograph-runtime-attribution` so the runtime context carries canonical
  effective contracts and durable attribution ids directly.
- Implemented execution contracts:
  `NodeExecutionContext`, `NodeExecutionContextInput`, `NodeExecutionInput`,
  `NodeExecutionOutput`, `NodeExecutionResult`, `NodeExecutionError`, and
  `NodeOutputSummary`.
- Implemented runtime-owned lifecycle handles and context:
  `NodeCancellationToken`, `NodeProgressHandle`, `NodeProgressEvent`, and
  `NodeLineageContext`.
- Implemented managed capability route contracts:
  `ManagedCapabilityKind`, `ManagedCapabilityRoute`,
  `NodeManagedCapabilities`, `ModelExecutionCapability`,
  `ResourceAccessCapability`, `CacheCapability`, `ProgressCapability`,
  `DiagnosticsCapability`, and `ExternalToolCapability`.
- Implemented guarantee classification through `NodeExecutionGuarantee` and
  `NodeExecutionGuaranteeEvidence`.
- Updated the embedded-runtime public facade and source README to expose and
  document the new runtime context boundary.
- Decomposition review: the initial combined context/capability/test module
  exceeded 500 lines, so it was split into focused sibling modules before
  commit.
- Verification passed:
  `cargo test -p pantograph-embedded-runtime node_execution`,
  `cargo check -p pantograph-embedded-runtime`,
  `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-embedded-runtime --all-targets -- -D warnings`.
- Full `cargo test -p pantograph-embedded-runtime` is not clean in this
  environment. The new `node_execution` tests passed, but the package suite
  still reports existing Pumas SQLite read-only database failures and older
  workflow-run fixture failures where callers supply backend-owned run ids.
- Remaining Wave `02` work: adapt baseline runtime diagnostics events from
  scheduler/runtime/node-engine facts and wire cancellation/progress/guarantee
  behavior into execution paths.

### 2026-04-24 Wave 02 Diagnostics Event Adapter Progress

- Added `crates/pantograph-embedded-runtime/src/node_execution_diagnostics.rs`
  and focused diagnostics adapter tests.
- Implemented transient `NodeExecutionDiagnosticEvent` and
  `NodeExecutionDiagnosticEventKind` DTOs for runtime-owned node diagnostics.
- Implemented `adapt_node_engine_diagnostic_event` to enrich node-engine task
  lifecycle, waiting-for-input, progress, stream, workflow cancellation,
  graph-modified, and incremental-execution events with runtime attribution,
  workflow id, node id/type, attempt, contract version/digest, lineage, and
  guarantee context.
- Captured output summaries from effective output port contracts for completed
  and stream events without implementing durable ledger storage.
- Updated the embedded-runtime public facade and source README to expose and
  document the transient diagnostics adapter.
- Verification passed:
  `cargo test -p pantograph-embedded-runtime node_execution_diagnostics`,
  `cargo check -p pantograph-embedded-runtime`,
  `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-embedded-runtime --all-targets -- -D warnings`.
### 2026-04-24 Wave 02 Cancellation Progress Guarantee Progress

- Added `NodeExecutionDiagnosticsRecorder` as an event-sink recorder that can
  sit on node-engine execution paths, forward original workflow events, and
  collect enriched runtime-owned diagnostics for registered
  `NodeExecutionContext` values.
- The recorder adapts progress, cancellation, lifecycle, stream, graph, and
  incremental execution events without requiring ordinary node code to emit
  diagnostics manually.
- Added tests proving the recorder forwards original node-engine events,
  captures adapted diagnostics, and preserves reduced guarantee classification
  on cancellation events.
- Verification passed:
  `cargo test -p pantograph-embedded-runtime node_execution_diagnostics`,
  `cargo check -p pantograph-embedded-runtime`,
  `cargo fmt --all -- --check`, and
  `cargo clippy -p pantograph-embedded-runtime --all-targets -- -D warnings`.
- Wave `02` runtime context, diagnostics adaptation, and
  cancellation/progress/guarantee slices are integrated. Remaining Stage `03`
  work moves to Wave `03`: final integration review, ADR, final verification,
  and stage-end refactor gate.

### 2026-04-24 Wave 03 Integration And Closeout

- Added `docs/adr/ADR-007-managed-runtime-observability-ownership.md` to freeze
  embedded-runtime ownership of runtime-created node execution context, managed
  capabilities, transient diagnostics, cancellation/progress lifecycle, and
  guarantee classification.
- Updated the ADR index so the Stage `03` observability boundary has the same
  stable traceability as Stage `01` attribution and Stage `02` node contracts.
- Repaired stale embedded-runtime test fixtures that supplied caller-owned
  `run_id` values to public `workflow_run` calls. The fixture updates preserve
  the Stage `01` rule that workflow run ids are backend-owned and restored the
  full `pantograph-embedded-runtime` package suite.
- Verification passed:
  `cargo test -p pantograph-embedded-runtime`,
  `cargo test -p node-engine`,
  `cargo test -p pantograph-workflow-service`,
  `cargo check --workspace --all-features`,
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and
  `cargo test --workspace --doc`.
- Stage-end refactor gate outcome: `not_warranted`.
- Stage-end gate review source:
  `git diff --name-only 7d153f82...HEAD` plus Wave `03` files that were still
  uncommitted before the closeout commit.
- Touched source files stayed below the 500-line decomposition review trigger:
  `node_execution.rs` 397 lines, `node_execution_capabilities.rs` 193 lines,
  `node_execution_tests.rs` 219 lines, `node_execution_diagnostics.rs` 367
  lines, `node_execution_diagnostics_tests.rs` 277 lines,
  `runtime_preflight_tests.rs` 317 lines, and
  `workflow_run_execution_tests.rs` 377 lines.
- Applicable standards groups reviewed: planning/documentation,
  architecture/coding, Rust API and async, testing/tooling,
  security/dependencies, interop/bindings, and release/cross-platform.
- Findings: Stage `03` changes are already split by runtime-context,
  capability, diagnostics-adapter, recorder, and test responsibilities;
  durable model/license ledger storage remains out of scope until Stage `04`;
  no additional standards refactor is required before the next numbered stage.

## Type Families To Define

### Execution Types

- `NodeExecutionContext`
- `NodeExecutionInput`
- `NodeExecutionOutput`
- `NodeExecutionResult`
- `NodeExecutionError`
- `NodeCancellationToken`
- `NodeProgressHandle`
- `NodeLineageContext`
- `NodeExecutionGuarantee`

### Managed Capability Types

- `ModelExecutionCapability`
- `ResourceAccessCapability`
- `CacheCapability`
- `ProgressCapability`
- `DiagnosticsCapability`
- `ExternalToolCapability`

## Implementation Decisions

### Runtime Ownership

- `crates/pantograph-embedded-runtime` owns `NodeExecutionContext`, managed
  capability routing, baseline diagnostics emission, cancellation/progress
  handles, lineage context, and guarantee classification.
- `crates/node-engine` remains a lower-level execution helper only where its
  existing abstractions are useful. It must not become the owner of durable
  attribution, diagnostics meaning, or binding projection semantics.
- `crates/pantograph-workflow-service` creates workflow runs before scheduling
  and calls into the embedded runtime with already-resolved attribution.
- Node implementation crates receive a runtime-created context. They do not
  receive raw client/session/bucket/run ids as trusted arguments.

### Async And Lifecycle Decision

- Runtime creation belongs in product composition roots or service wiring, not
  in reusable library constructors.
- The embedded runtime owns spawned node tasks through tracked handles,
  `JoinSet`, or an equivalent task tracker.
- Cancellation, timeout, retry, progress, and attempt state have one lifecycle
  owner in the embedded runtime.
- Baseline diagnostics are emitted by scheduler/context wrappers before and
  after node invocation, not by normal node code.
- Task panic and cancellation paths are classified at the runtime lifecycle
  owner and surfaced through diagnostics rather than being swallowed.

### Diagnostics Retention Decision

- Stage `03` owns transient runtime trace diagnostics and persisted run indexes
  only where needed to correlate live traces to workflow runs.
- Durable compliance ledger records are stage `04`; stage `03` may define the
  guarantee and event DTOs needed by that ledger but must not complete the
  model/license ledger early.

### Guarantee Classification Decision

- The runtime assigns `NodeExecutionGuarantee` for every standard node attempt.
- `managed_full` requires resolved attribution, effective contract reference,
  runtime-created context, baseline lifecycle events, and managed capability
  routing for applicable model/resource/tool calls.
- `managed_partial` requires explicit unavailable measurement or capability
  facts.
- `escape_hatch_detected` is used when runtime-mediated escape hatches are
  exercised.
- `unsafe_or_unobserved` is used when required runtime ownership cannot be
  proven.

## Managed Execution Flow

```text
resolve workflow run attribution
  -> resolve graph and effective node contracts
  -> schedule runnable node
  -> create NodeExecutionContext
  -> emit backend-owned execution-start event
  -> invoke node execution logic
  -> route model/resource/cache/tool calls through managed capabilities
  -> capture output summaries and direct model output measurements
  -> submit model/license usage facts through the Stage 04 ledger trait when available
  -> emit completion or failure event
  -> publish diagnostics projections
```

Node authors should not manually perform attribution, baseline diagnostics,
license tracking, or output-measurement steps.

Stage `03` may define and emit the runtime facts needed by the durable ledger,
but it must not implement durable model/license ledger storage. Persistence
belongs to Stage `04`.

## Runtime Trace Diagnostics

The runtime must automatically emit baseline trace events for standard nodes:

- node execution started
- node execution completed
- node execution failed
- node execution skipped or blocked
- node cancellation observed
- node progress update
- input summary captured
- output summary captured
- effective contract resolved or changed
- connection or validation rejection
- execution guarantee changed

Every baseline trace event must carry:

- `client_id`
- `client_session_id`
- `bucket_id`
- `workflow_id`
- `workflow_run_id`
- `node_id`
- `node_type`
- relevant `port_id` values when the event is port-specific
- effective contract version or digest when available
- timestamp
- execution attempt or visit count where relevant
- parent composed-node context where relevant

## Guarantee Levels

- `managed_full`: managed runtime path with complete required attribution and
  measurement facts.
- `managed_partial`: managed runtime path with explicit unavailable measurement
  fields.
- `escape_hatch_detected`: runtime-mediated escape hatch reduced guarantees.
- `unsafe_or_unobserved`: required observability was bypassed or could not be
  proven.

Reduced-guarantee records must never be presented as complete compliance data.

## Affected Structured Contracts And Persisted Artifacts

- Runtime trace event DTOs, node execution result records, guarantee
  classifications, progress events, cancellation observations, and lineage
  projections.
- Any persisted run indexes needed to correlate live traces with durable usage
  records.

## Standards Compliance Notes

- Rust async compliance requires runtime creation to stay in composition roots,
  every spawned task to have a lifecycle owner, cancellation to be propagated
  through explicit tokens, and shutdown to drain or abort owned work
  deliberately.
- Concurrency compliance requires progress, cancellation, retry, timeout, and
  node attempt state to have one owner. Shared mutable state must not be held
  across await points unless the lock type and transaction boundary justify it.
- Rust API compliance requires typed execution errors, explicit guarantee
  enums, validated context ids, and `Result`-based public APIs rather than
  panics for recoverable node/runtime failures.
- Observability compliance requires runtime-owned spans/events for start,
  completion, failure, skipped/blocked, cancellation, progress, contract
  change, and guarantee-change paths without node-authored boilerplate.
- Testing compliance requires no-boilerplate node tests, cancellation tests,
  timeout tests, failure attribution tests, and escape-hatch downgrade tests.

## Risks And Mitigations

- Risk: managed capabilities are bypassed by convenience APIs. Mitigation:
  require guarantee classification and make bypasses visible in diagnostics.
- Risk: node execution introduces untracked background tasks. Mitigation:
  require a runtime lifecycle owner for any spawned work and verify shutdown.
- Risk: observability requires node authors to call diagnostics APIs manually.
  Mitigation: baseline events are emitted by the scheduler and context wrapper,
  not by node code.

## Tasks

- Implement runtime execution context and managed capabilities in
  `pantograph-embedded-runtime`.
- Create `NodeExecutionContext` or equivalent.
- Inject attribution, cancellation, progress, diagnostics, and lineage context.
- Ensure start/completion/failure events are emitted for ordinary nodes.
- Capture input and output summaries from contracts and runtime facts.
- Classify execution guarantee level for managed and escape-hatch paths.
- Record decomposition targets for any oversized files touched by the work.

## Intended Write Set

- Primary:
  - `crates/pantograph-embedded-runtime/`
- Adjacent only if required by existing call sites:
  - `crates/node-engine/`
  - `crates/pantograph-workflow-service/`
  - `crates/workflow-nodes/`
- Forbidden for this stage unless the plan is updated first:
  - host binding generation
  - durable model/license ledger queries
  - GUI diagnostics views

## Existing Code Impact

- `crates/pantograph-embedded-runtime/src/workflow_runtime.rs` currently builds
  workflow execution diagnostics snapshots from scheduler and runtime facts.
  Stage `03` should extend this path into runtime-created node execution
  context and baseline node lifecycle events rather than adding diagnostics
  calls to ordinary nodes.
- `crates/pantograph-embedded-runtime/src/workflow_session_execution.rs`
  currently owns warm workflow executor reuse keyed by workflow session id.
  Stage `03` must separate that internal workflow-session execution cache from
  durable client/session/bucket/run attribution resolved in stage `01`.
- `crates/pantograph-workflow-service/src/scheduler/store_diagnostics.rs`
  and scheduler DTOs already expose scheduler snapshots. Stage `03` must keep
  scheduler diagnostics backend-owned while adding node-level context and
  guarantee classification.
- `crates/node-engine/src/events/` and `crates/node-engine/src/engine/` already
  produce lower-level workflow events. Stage `03` must decide whether to adapt
  those events into runtime-managed baseline diagnostics or replace them at the
  embedded-runtime boundary; node-engine must not become the owner of durable
  attribution or compliance semantics.

## Verification Commands

Expected stage verification:

```bash
cargo test -p pantograph-embedded-runtime
cargo test -p node-engine
cargo check --workspace --all-features
```

If workflow-service scheduling integration is touched, also run:

```bash
cargo test -p pantograph-workflow-service
```

Stage completion also requires the Rust baseline verification from
`RUST-TOOLING-STANDARDS.md` unless the stage-start report records an existing
repo-owned equivalent:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --doc
```

## Verification

- A minimal node with no diagnostics code still produces baseline diagnostics.
- Failures preserve node, port, contract, and run attribution.
- Cancellation and timeout behavior are observable from backend facts.
- Escape-hatch paths are detectable and visibly reduce guarantee level.
- Runtime shutdown drains or aborts owned node tasks and reports cancellation or
  panic paths at the lifecycle owner.

## Completion Criteria

- Ordinary nodes get baseline diagnostics without node-authored diagnostics
  boilerplate.
- Managed capabilities automatically carry attribution and observability
  context.
- The stage-start implementation gate in
  `08-stage-start-implementation-gate.md` is recorded before source edits.
- The stage-end refactor gate in `09-stage-end-refactor-gate.md` is completed
  or explicitly recorded as not warranted for this stage.

## Re-Plan Triggers

- Ordinary nodes must explicitly emit baseline diagnostics to be observable.
- Node execution requires global runtime creation inside library crates.
- Cancellation or progress cannot be represented without shared mutable state
  that violates the concurrency standards.
