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

`workflow-nodes` owns concrete built-in primitive node registrations.
Primitive descriptors continue to project into `NodeTypeContract` without
serialization changes.

The existing `tool-loop` authoring surface is not represented as a static
composed contract over `llm-inference`, `tool-executor`, and turn-state
control nodes. That static composed mapping is retired because it would keep
the broad `llm-inference` descriptor alive as an executable compatibility
path. `tool-loop` remains a stable authoring descriptor, but runtime behavior
must be implemented through scheduler-owned agent loop orchestration once the
generic inference descriptor and task-level scheduler contracts can materialize
one inference turn.

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

- Composed-node DTOs remain available for future authoring surfaces where a
  static internal graph is the canonical design, but `workflow-nodes` does not
  currently expose a built-in composed `tool-loop` registration.
- Scheduler-owned `tool-loop` orchestration must preserve primitive execution
  facts without routing through a static all-port `llm-inference` fallback.
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
