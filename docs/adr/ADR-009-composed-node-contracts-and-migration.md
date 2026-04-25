# ADR-009: Composed Node Contracts And Migration

## Status

Accepted.

## Context

Pantograph needs higher-level graph authoring surfaces without hiding the
primitive runtime facts required for diagnostics, attribution, and durable
model/license usage history.

Earlier execution-platform stages established durable attribution, canonical
node contracts, runtime-created execution context, and model/license ledger
records. Stage 05 connects those boundaries to composed authoring surfaces and
saved-workflow migration behavior.

Existing workflows may contain legacy node ids or volatile derived graph
projections. Those artifacts must be upgraded, regenerated, or rejected with
typed diagnostics instead of silently changing behavior or preserving
indefinite compatibility shims.

## Decision

`pantograph-node-contracts` owns canonical composed-node and migration
semantics. It defines composed external contracts, internal primitive graph
mappings, external-to-internal port mappings, primitive trace policy, contract
upgrade records, upgrade outcomes, changed node/port records, diagnostics
lineage policy, and typed rejection diagnostics.

`workflow-nodes` owns concrete built-in primitive and composed node
registrations. Primitive descriptors continue to project into
`NodeTypeContract` without serialization changes. Built-in composed authoring
surfaces are exposed through `builtin_composed_node_contracts()`.

The existing `tool-loop` authoring surface is represented as a composed
contract over primitive `llm-inference`, `tool-executor`, and turn-state
control nodes. This preserves the stable external `tool-loop` ports while
making the primitive trace policy explicit.

`pantograph-embedded-runtime` owns runtime composed-parent lineage projection.
Runtime-created primitive execution contexts use `NodeLineageContext` helpers
to carry parent composed node ids, composed parent stacks, and lineage segment
metadata into transient diagnostics and durable ledger events.

`pantograph-workflow-service` owns saved-workflow migration use cases. It
emits migration-aware canonicalization results for legacy upgrades and keeps
volatile graph projections regenerable from canonical graph state.

Compatibility projections are temporary migration internals only. They must not
remain as public node, port, GUI, binding, or runtime semantics after an
artifact is upgraded or rejected.

Stage 05 does not implement host bindings or GUI redesign.

## Consequences

- Composed nodes improve authoring without erasing primitive execution facts.
- Model/license usage records continue to point at primitive model execution
  and can include composed-parent lineage.
- Saved-workflow upgrades have explicit records for changed node/port ids and
  lineage behavior.
- Volatile projections can be regenerated rather than persisted as
  compatibility state.
- Unmigratable artifacts must fail with typed diagnostics rather than silent
  behavior changes.
- Future GUI and binding work must consume backend-owned composed contracts
  and migration records instead of reconstructing composition locally.
