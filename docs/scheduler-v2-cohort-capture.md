# Store-owned cohort capture and evidence coverage

This additive workflow-service API captures the represented runtime population
needed by the pure bounded cohort kernel. It does not call that kernel, install a
selector, collect profiles, reserve resources, admit successors, or change default
dispatch. The existing two-task lookahead and serial-Ready APIs are unchanged.

## Read-only entry point

A native application can explicitly construct
`WorkflowService::new_with_cohort_coverage(provider)` and query
`workflow_cohort_evidence_coverage(session, run, first_task)`.
`WorkflowService::new()` has no provider and reports capability false. Even an
injected provider defaults to capability false unless it explicitly implements
`supports_cohort_coverage()`.

The dedicated provider is separate from dispatch candidate collection, which may
create provisional reservations. Its callbacks must be bounded, nonblocking and
read-only. It returns a complete placement universe for every captured task,
including one or two capability-valid, serialized, single-device descriptors per
task. A third placement, omitted task group, duplicate, incompatible constraint,
empty identity or incomplete-universe assertion refuses the query. The service
cannot independently discover or certify native capabilities: this remains the
trusted injected owner's responsibility.

The result is `Disabled`, a typed `Refused` reason, or `Incomplete` coverage. No
result contains a selected action, lease, resource-fit assertion, cleanup
acknowledgement or forecast Ready proof. The snapshot retains only the first
task's genuine persisted admission proof.

## Frozen population and invalidation

Under one store lock the capture includes every unfinished runtime-inference
obligation in the active workflow/run, with no skipping or truncation. One to
four tasks are supported; the first must already be Ready with its persisted,
matching dependency proof. Future members can be Ready, AwaitingInputs or
WaitingDependencyReadiness; running work and other unsupported runtime states
refuse the capture. Non-runtime running work also refuses a serialized capture.

Each member retains its task/descriptor, state/version, complete dependency and
input bindings, already materialized typed inputs and exact selected-value
identities. Internal outputs that do not exist are represented solely by symbolic
source/target bindings. Their shape, value, size and transfer cost remain unknown.
Known model bindings that the execution mapper intentionally omits still have an
exact input identity, so changing one invalidates the snapshot.

Internal dependencies must form a DAG. External prerequisites must be represented
in the same graph with Completed records; unknown records, unfinished external
work, malformed bindings and pending pair cleanup refuse the capture. Independent
members, chains, fan-out and joins do not require a single-successor topology.

The store fingerprint binds the entire bounded graph, all records and persisted
readiness proofs. Members are exposed in task-id order and the actual admitted
first task is separately identified. Provider calls happen after releasing the
store lock. The service then reacquires the lock, recaptures and compares the
whole population, and checks every live owner epoch without callbacks. An owner
must advance its monotonic epoch on placement, capability or residency changes
and never reuse an old value. These guards are observation invalidation, not
resource custody; the returned inspection is an as-of observation and authorizes
no later execution.

## Bounds and evidence honesty

The service reuses existing per-task/proof/record limits and the 128-task graph
and record limit. There is also a 256 KiB aggregate capture serialization budget,
charged while fingerprinting the graph/records/proofs, selected known input
values and final snapshot. A streaming writer refuses oversize data without
allocating a serialized copy. Input lookup uses bounded bindings (16), bounded
output lists (32), and indexed task records/results; it never clones or scans the
entire result population. Provider lookup is bounded by four task groups and two
placements. Returned coverage also has a 256 KiB serialization cap.

Coverage explicitly names missing workload qualification, symbolic output shape
when applicable, setup/transfer/execution/cleanup/retention/reload timing,
release/reconciliation and conditional capacity. All native coverage in this
slice remains incomplete. No current CPU calibration, coarse traces, private
lifecycle observer or stale sample is relabeled as this evidence. Missing stages
are not zero-filled, and future output shape or capacity is never inferred.
There is deliberately no API to submit a supposedly complete matrix here. The
pure kernel's synthetic complete matrices remain fixtures; a trusted native
path-conditioned evidence producer is separate future work.

No POC source was copied. Main, native dispatch, existing calibration and private
instrumentation are outside this change.
