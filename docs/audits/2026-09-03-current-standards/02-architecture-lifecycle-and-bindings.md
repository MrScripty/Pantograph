# Focused Audit: Architecture, Lifecycle, And Bindings

Implementation plan: [Architecture, lifecycle, and bindings remediation](../../plans/current-standards-remediation/architecture-lifecycle-and-bindings/plan.md)

## Scope

This audit covers execution authority, scheduler/runtime-host separation,
binding adapters, process and task ownership, shutdown, event delivery, and
failure semantics.

Applicable current standards are Architecture, Contracts, Concurrency,
Resilience, Diagnostics, Interop, Language Bindings, Rust API, Rust Async, and
Verification.

## Assessment

Pantograph's target architecture is sound and newer scheduler components often
model lifecycle correctly. The remaining violations are concentrated in
compatibility and direct-execution paths. They should be removed or adapted to
the target owners rather than generalized into another permanent layer.

## Findings

### ARC-01 — High: direct inference remains a second execution authority

[core_executor.rs](../../../crates/node-engine/src/core_executor.rs#L92) owns an
InferenceGateway and directly executes text, embedding, rerank, audio, image,
fallback, and unload operations. Rustler publicly constructs and exposes that
path in pantograph-rustler/src/lib.rs near lines 252-340.

This conflicts with the accepted target in
[ARCHITECTURE.md](../../../ARCHITECTURE.md): every public run should enter a
scheduler-backed session and node-engine should remain runtime-neutral.

This is acknowledged migration debt, not an accidental undocumented design.
Compliance still requires one executable owner, an explicit retirement
boundary, and deletion of unsupported routes.

### ARC-02 — High: Rustler owns alternate runtimes and blocking

Verified binding-owned runtime construction and block_on behavior appears in:

- pantograph-rustler/src/executor_nifs.rs near lines 27-123;
- workflow_host_contract.rs near lines 34-97; and
- orchestration_execution_nifs.rs near lines 12-110.

A deprecated public operation also blocks a BEAM DirtyCpu scheduler, while a
demand path spawns work, discards its handle, ignores result delivery, and
immediately returns success.

Binding adapters should project a composition-owned runtime and typed
operation outcomes. They should not become lifecycle or execution-policy
owners.

### ARC-03 — High: boundary conversions and event delivery lose meaning

- Rustler turns malformed node JSON into a default value and reports a
  successful mutation.
- UniFFI turns malformed node JSON into Value::Null for add and update.
- Rustler narrows cache counts from usize to u32 without checked conversion.
- [workflow_event_bridge.rs](../../../crates/pantograph-uniffi/src/workflow_event_bridge.rs#L8)
  uses an unbounded Vec and silently drops an event when try_write contends,
  while returning success.

These are verified violations of typed invalid input, range preservation,
capacity/overflow ownership, and observable delivery failure.

### ARC-04 — High: standard process spawning detaches lifecycle work

[process.rs](../../../crates/inference/src/process.rs#L158) launches a blocking
process inside an async operation, spawns stdout, stderr, and monitor tasks,
and discards all three handles. The returned handle owns the child only. Send
failures are ignored and lifecycle errors are flattened to strings.

The owner cannot observe task panic or drain all work during shutdown.

### ARC-05 — High: shutdown outcomes are erased

Examples include:

- stale-session cleanup discarding signal and join outcomes;
- dependency auto-resume logging task failure but returning successful unit;
- embedded-runtime shutdown logging invalidation failure and continuing with a
  void outcome; and
- UniFFI exporting that shutdown as void.

Shutdown must preserve complete, incomplete, and failed terminal outcomes
through the public boundary.

### ARC-06 — Medium: transition machinery needs a deletion audit

The repository contains 104 dead-code allowances and several core transition
modules with thousands of lines. Counts are not standards violations. Combined
with the verified duplicate execution routes, they are evidence that a
composed-design and deletion review is warranted.

The review should ask what knowledge every adapter, registry, facade, feature,
and compatibility path lets callers ignore, and what complexity disappears if
it is deleted.

## Preserved Strengths

- Scheduler queue and inference-validation owners track handles, close
  admission, cancel, drain, and report terminal failures.
- Scheduler, runtime-registry, runtime-host, workflow-service, and inference
  responsibilities are explicitly documented.
- Runtime and scheduler contracts use validated identifiers and typed
  diagnostics extensively.
- Repo-owned unsafe code is denied and no unsafe block was found.

## Follow-Up Audit Boundaries

1. Enumerate every public execution entry point and map it to one execution
   owner.
2. Inventory every production spawn, thread, process, callback, subscription,
   and blocking call with admission, cancellation, drain, and terminal state.
3. Audit every binding conversion for malformed, unsupported, unavailable,
   overflow, and shutdown outcomes.
4. Define bounded event delivery and overflow behavior for every host.
5. Perform composed-design admission on the retained scheduler/runtime/binding
   artifact, then identify deletable compatibility machinery.

The next implementation plan should split scheduler cutover, binding repair,
and process lifecycle into independently verifiable slices.
