# ADR-007: Managed Runtime Observability Ownership

## Status

Accepted.

## Context

Pantograph needs ordinary node execution to produce backend-owned diagnostics,
attribution, cancellation, progress, lineage, and guarantee classification
without requiring every node implementation to emit observability boilerplate.

`node-engine` already produces lower-level workflow and task events, while
Stages 01 and 02 established durable runtime attribution and canonical node
contracts. Stage 03 must connect those facts at the embedded-runtime boundary
without moving compliance meaning into host adapters, node-engine internals, or
individual node code.

## Decision

`pantograph-embedded-runtime` owns managed node execution observability.
It defines and exports runtime-created node execution context, cancellation,
progress, lineage, managed capability routing, transient diagnostic events, and
execution guarantee classification.

Runtime-created `NodeExecutionContext` values carry Stage 01 attribution and
Stage 02 effective node contract references. Managed capability routes attach
that same context to model, resource, cache, progress, diagnostics, and
external-tool calls.

`node-engine` remains a low-level execution event producer. The embedded
runtime may adapt node-engine task lifecycle, progress, stream,
waiting-for-input, graph mutation, and cancellation events, but those events are
input facts rather than the owner of durable attribution, compliance meaning,
guarantee policy, or host binding projections.

Cancellation, progress, attempt state, and baseline diagnostic collection have
one runtime owner. Standard node execution paths use runtime-created handles and
recorders so start, completion, failure, cancellation, progress, and reduced
guarantee events can be observed without node-authored diagnostics calls.

Durable model/license ledger storage and query implementation remains outside
Stage 03. Stage 03 may define transient runtime facts and ledger-facing inputs,
but Stage 04 owns persistence and durable query projections.

## Consequences

- Ordinary node implementations do not own baseline diagnostics, durable
  attribution, or guarantee classification.
- Reduced observability is represented explicitly through
  `NodeExecutionGuarantee` rather than being silently projected as complete
  compliance data.
- Host adapters and bindings project backend-owned observability records
  instead of inventing local runtime diagnostics semantics.
- Future scheduler integration must register runtime-created node contexts
  before adapting node-engine execution events.
- Stage 04 can consume runtime-owned transient facts through a ledger boundary
  without retrofitting node-authored diagnostics boilerplate.
