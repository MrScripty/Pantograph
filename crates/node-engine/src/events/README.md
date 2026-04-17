# crates/node-engine/src/events

## Purpose
This directory owns the backend workflow event contract for `node-engine` and
the in-process sink implementations that collect, broadcast, or fan out those
events. It exists so execution semantics can evolve without pushing more
responsibility into a single monolithic `events.rs` file or into transport
adapters.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `contract.rs` | Canonical `WorkflowEvent` contract and timestamp helpers. |
| `sinks.rs` | `EventSink` trait, error type, and built-in sink implementations. |
| `tests.rs` | Focused sink and contract tests behind the stable `events` facade. |

## Problem
Workflow execution, diagnostics, bindings, and Tauri transport all depend on
the same event contract, but they should not all grow the same source file.
Keeping the contract and sink implementations separated makes it easier to add
event vocabulary or transport-specific adapters without obscuring ownership of
the backend-owned event semantics.

## Constraints
- Event semantics stay backend-owned in Rust and must not be redefined by
  transport layers.
- Additive fields such as producer timestamps must remain compatible with
  existing consumers.
- Sink implementations must remain generic and in-process; they cannot assume a
  particular frontend or host runtime.

## Decision
Keep `crate::events` as the stable public facade while splitting the event
contract and sink implementations into focused submodules. This preserves
existing imports for callers while creating standards-compliant insertion
points for later Phase 5 event-contract work and Phase 2 execution refactors.

## Alternatives Rejected
- Keeping all event contract and sink logic in `events.rs`.
  Rejected because the file already exceeded decomposition thresholds and was
  the shared insertion point for multiple roadmap phases.
- Moving sink implementations into transport crates.
  Rejected because event collection and fan-out are backend execution concerns,
  not adapter-owned logic.

## Invariants
- `crate::events` remains the stable import path for existing callers.
- `WorkflowEvent` stays the canonical execution event contract for backend
  producers.
- Built-in sinks remain transport-agnostic and safe to use in tests or
  headless runtimes.

## Revisit Triggers
- Event persistence or durable replay moves sink ownership into a different
  backend package.
- The event contract gains enough variants or helpers to justify further
  splitting `contract.rs`.
- A second runtime host needs a distinct sink family with lifecycle semantics
  that do not fit the current in-process abstractions.

## Dependencies
**Internal:** `crate::engine`, `crate::orchestration`, and any execution or
adapter layer that consumes `node_engine::WorkflowEvent`.

**External:** `serde`, `serde_json`, and `tokio::sync::broadcast`.

## Related ADRs
- None identified as of 2026-04-17.
- Reason: This directory decomposition preserves the existing event ownership
  boundary rather than creating a new architecture boundary.
- Revisit trigger: Durable event storage or externalized event buses become a
  first-class backend architecture concern.

## Usage Examples
```rust
use node_engine::{EventSink, VecEventSink, WorkflowEvent};

let sink = VecEventSink::new();
sink.send(WorkflowEvent::task_progress("node-a", "exec-1", 0.5, None))
    .expect("event should be collected");
```

## API Consumer Contract
- Consumers import events through `node_engine::events` or the re-exports in
  `node_engine`.
- Sink implementations operate synchronously from the caller's perspective and
  return `EventError` when a send fails.
- Broadcast sinks may drop events when no receiver is listening; callers should
  not treat that as a workflow failure.

## Structured Producer Contract
- `WorkflowEvent` uses serde tagged-enum encoding with `type` and camelCase
  field names.
- `occurred_at_ms` is additive and optional; consumers must tolerate its
  absence on older producers.
- Event variants and field names should evolve append-only where practical so
  transport and binding consumers can stay compatible.
