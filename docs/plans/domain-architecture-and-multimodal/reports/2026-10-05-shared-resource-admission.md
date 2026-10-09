# Shared backing resource admission

Branch `feat/scheduler-shared-resource-admission` starts at frozen qualification
commit `6aa6b717630593700f92c74bdfb91ee7f5e30f37`, tree
`85c340bddd973230118528dacda632aa0521aa10`. Parent retains review, PR and hosted
qualification. Frozen sampling/qualification/PR54 refs and dependency pins are
unchanged. No ONNX retry, substitution, native test execution or model download.

## Gap and scope

The scheduler thesis (`pantograph_scheduler_thesis.md`, Library
`libfile_6c1d9aa6e1f08191b4a14e0de0846220`, version 1, sections 2 and 4) identifies
per-runtime accounting as a prerequisite gap for safe heterogeneous placement.
The v2 evaluation (`libfile_8b6d205bd260819187ef9fbac029a243`) demonstrates shared
physical domains in a simulator, not Pantograph or measured hardware.
Current production reservation evaluation/selected commitment already exist,
but two runtimes can independently spend the same configured backing capacity.
This is the bounded admission capability selected before automatic placement or
completion ranking; it does not port the simulator or invent timing samples.

Acceptance: alternatives do not allocate leases; concurrent 80-byte requests from
two runtimes sharing 100 bytes admit exactly one; RAM and VRAM can share one
unified-memory pool and are summed; independent pools stay independent; local
budgets and shared margins both constrain admission; failed replacements preserve
their predecessor; provisional rollback/transfer cannot release protected capacity
early; unsafe configuration changes and overflowing claims reject atomically.

## Implementation

The existing registry owns explicit resource-domain configuration. Each binding
maps a canonical runtime/resource-kind pair to one domain. Multiple runtimes and
both logical memory kinds can share a domain. Bindings are fixed after creation;
capacity/margin updates must cover live claims. No secondary reservation store,
allocator, background task, configuration file or persistence schema is added.

Read-only evaluations add domain requested/reserved/available bytes. The existing
authoritative acquisition path checks these capacities under its lock after
runtime-local admission. Provisional publication also checks componentwise
maximum old/new claims, protecting unified-memory rollback. Existing release and
custody transfer reduce held claims through the same reservation authority.
The embedded host classifies shared-capacity rejection as unavailable admission,
with explicit configuration/overflow error mappings. Durable usage guidance lives
in [runtime operations](../../../runtime-operations.md#shared-resource-admission).

This is an opt-in production registry API. Existing composition does not yet
provide authoritative physical bindings/capacities, so no automatic desktop
activation or hardware safety claim is made. Operators/composers must configure
known domains before using them. Scope is declared reservation envelopes within
one registry, not external allocations or retained model memory without leases.
Automatic device discovery, per-device subdivision within one runtime, persistent
domain recovery, alias sharing, calibrated ranking and full residency accounting
remain separate capabilities. Unconfigured resources retain their current behavior.

## Verification

`cargo test --locked -p pantograph-runtime-registry -p pantograph-scheduler`:
106 registry and 133 scheduler tests passed, zero failures or ignored tests.
Eleven new public-API domain tests run real registry state, custody lifecycle and
thread contention without ONNX or inference linking. Registry/scheduler all-target
Clippy with `-D warnings` passed. Embedded host library/test Clippy also passed
without warnings as a compiler check. Formatting, whitespace, critical and
scheduler-only surface gates passed; staged/range traceability is checked before
source upload. No native host test is included in these execution counts.

Logs: `/tmp/shared-resource-tests.log`, `/tmp/shared-resource-clippy.log`,
`/tmp/shared-resource-consumer-compile.log`. Downstream host compilation uses
`ORT_SKIP_DOWNLOAD=1`, normal embedded defaults plus `backend-pytorch`, and
`--lib --tests`; it does not link or execute native host tests. Earlier sampling
native qualification remains blocked by the pinned ONNX HTTP 403.
