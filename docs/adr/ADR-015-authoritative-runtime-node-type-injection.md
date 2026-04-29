# ADR-015: Authoritative Runtime Node Type Injection

## Status
Accepted

## Context
Pantograph node execution historically allowed `CoreTaskExecutor` to infer a
node type from the task id when `_data.node_type` was absent. That fallback is
fragile for GUI-authored node ids such as `prompt-input`,
`negative-prompt-input`, or `steps-input`: stripping the final suffix produces
`prompt`, `negative-prompt`, or `steps` instead of the graph-declared
`text-input` or `number-input`.

The scheduler-only execution path exposed this fragility because saved workflow
graphs can carry arbitrary stable node ids while the runtime executor still
needs a trusted node type for routing to core or host-specific handlers.

## Decision
The node engine preparation layer must inject the authoritative graph node type
into each task's `_data.node_type` payload before execution. Runtime dispatch
must treat the graph `node_type` field as the source of truth when present in
prepared inputs, not derive behavior from user-facing or GUI-generated node ids.

Null node data is normalized to an empty `_data` object with `node_type` during
preparation. Existing object node data is preserved and its `node_type` field is
overwritten with the graph-declared type so persisted metadata cannot spoof the
executor route.

The task-id suffix fallback remains only as a compatibility behavior for
manually constructed executor calls that bypass graph preparation. It is not
the supported path for workflow execution.

## Consequences

### Positive
- GUI-authored workflows can use readable, stable node ids without breaking
  executor routing.
- Scheduler-owned workflow execution no longer depends on node id naming
  conventions.
- Input snapshots and node-memory fingerprints include the graph node type,
  making cache and diagnostic state more explicit.

### Negative
- Existing input fingerprint expectations must account for `_data.node_type`.
- Prepared input snapshots change shape for null node data.

### Neutral
- Host-specific executors still handle only platform-owned node types.
- Saved workflow graph data does not need migration; the runtime injects the
  execution metadata during preparation.

## Guardrails
- Regression coverage must include a scheduler-submitted workflow whose input
  node id does not end with the node type, such as `prompt-input`.
- Node-memory snapshot tests must assert the explicit `_data.node_type`
  fingerprint shape.
