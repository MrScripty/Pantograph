# 03: Managed Runtime Observability

## Purpose

Route standard node execution through a runtime-created context so ordinary
nodes get diagnostics, attribution, cancellation, progress, and guarantee
classification without node-authored boilerplate.

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
  -> persist model/license usage records when applicable
  -> emit completion or failure event
  -> publish diagnostics projections
```

Node authors should not manually perform attribution, baseline diagnostics,
license tracking, or output-measurement steps.

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

- Decide which crate owns runtime execution context and managed capabilities.
- Create `NodeExecutionContext` or equivalent.
- Inject attribution, cancellation, progress, diagnostics, and lineage context.
- Ensure start/completion/failure events are emitted for ordinary nodes.
- Capture input and output summaries from contracts and runtime facts.
- Classify execution guarantee level for managed and escape-hatch paths.
- Record decomposition targets for any oversized files touched by the work.

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
