# 03: Managed Runtime Observability

## Purpose

Route standard node execution through a runtime-created context so ordinary
nodes get diagnostics, attribution, cancellation, progress, and guarantee
classification without node-authored boilerplate.

## Implementation Readiness Status

Ready for stage-start preflight after stages `01` and `02` are complete and
their stage-end refactor gates have been recorded.

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
