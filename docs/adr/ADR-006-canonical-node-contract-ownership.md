# ADR-006: Canonical Node Contract Ownership

## Status

Accepted.

## Context

Pantograph previously exposed node and port meaning through several nearby
shapes: `node-engine` task metadata, workflow-service graph DTOs, dynamic
`node.data.definition` overlays, and binding validation helpers. That made it
easy for GUI, bindings, or execution descriptors to drift into separate
compatibility and dynamic-shape semantics.

## Decision

`pantograph-node-contracts` owns canonical node type ids, node instance ids,
port ids, node contracts, effective node contracts, port value types,
compatibility checks, and structured rejection diagnostics.

`workflow-nodes` may define concrete executable node descriptors, but those
descriptors are projected into `NodeTypeContract` records before graph
authoring, workflow-service validation, or binding validation consumes them.

`node-engine` remains the execution engine and execution descriptor consumer. It
does not own canonical GUI, graph-authoring, or binding node semantics.

`pantograph-workflow-service` owns application-level projection: node
definition discovery, effective contract resolution from graph node overlays,
connection validation, and binding-facing workflow graph validation all consume
`pantograph-node-contracts` and expose additive DTO projections.

Host adapters, Tauri commands, UniFFI, Rustler, HTTP adapters, and GUI code may
forward these projections. They must not invent local node catalogs,
compatibility rules, or dynamic port-shape semantics.

## Consequences

- Graph-authoring and binding consumers get one backend-owned source for node
  and port semantics.
- Dynamic node shape must be represented as `EffectiveNodeContract` plus
  diagnostics before it is rendered or validated.
- Execution descriptors can remain optimized for runtime dispatch without
  becoming GUI or binding schemas.
- Binding validation can convert legacy graph JSON into workflow-service graph
  DTOs, but validation policy stays in workflow-service and
  `pantograph-node-contracts`.
- Future composed-node and saved-workflow migration work must preserve stable
  node and port ids and route compatibility decisions through the canonical
  contract layer.
