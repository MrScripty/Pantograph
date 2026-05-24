# pantograph-scheduler src

## Purpose
This directory contains the Rust source for Pantograph's scheduler-owned
dynamic task dispatch contracts. The boundary exists so queue state, resource
admission, dependency readiness, batching, dispatch decisions, runtime handoff,
and lifecycle ownership stay in one scheduler crate instead of drifting into
graph editing, node execution, frontend adapters, or runtime hosts.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Curated public facade for validated scheduler DTOs and policy helper entry points. |
| `intent.rs` | Path-free task intent submitted by ready workflow DAG nodes. |
| `capability.rs` | Backend-owned option and availability hints for graph/editor consumers. |
| `readiness.rs` | Scheduler admission policy contracts for dependency readiness and retry/defer/fail decisions. |
| `dispatch.rs` | Scheduler-selected runtime/device/model/reservation/batch execution decision contract. |
| `handoff.rs` | Runtime-host handoff envelope that carries readiness proof and optional dispatch decision. |
| `resource.rs` | Platform-neutral resource observation, residency, reservation, and fit contracts. |
| `queue.rs` | Durable phase-aware task state, typed runtime/non-runtime execution intent, and idempotent transition replay contract. |
| `lifecycle.rs` and `supervision.rs` | User-facing lifecycle diagnostics and scheduler service ownership contracts. |

## Problem
Workflow tasks may be admitted, deferred, batched, retried, or executed across
multiple users and devices while resource availability and dependency readiness
change. Keeping these contracts together gives Pantograph one place to reason
about scheduler-owned facts without exposing executable model paths to graph,
node-engine, or frontend code.

## Constraints
- Source modules must remain path-free and must not inspect Pumas filesystem
  layout, join model paths, or construct executable load targets.
- Scheduler policy owns runtime/device/resource/dependency/batching decisions;
  other crates may only submit typed intent or consume validated facts.
- Public DTOs are persisted or transported contracts and must keep explicit
  contract versions, typed ids, typed diagnostics, and validated wrappers.
- Core policy helpers remain synchronous unless a later slice records the I/O
  operation that requires an async shell.
- Platform-specific resource collection must live behind observer
  implementations; scheduler business logic consumes platform-neutral
  observations only.

## Decision
Use small modules by contract family and export them through `lib.rs`. This
keeps the public surface discoverable while allowing scheduler algorithms to
change behind stable typed contracts.

## Alternatives Rejected
- Put dispatch and admission contracts in node-engine: rejected because graph
  execution should submit task intent, not own scheduling policy.
- Put resource and runtime selection facts in runtime adapters: rejected
  because runtime adapters consume scheduler decisions and must not become a
  second source of scheduling truth.
- Keep a compatibility DTO for legacy dependency preflight output: rejected
  because Milestone 5a requires replacement contracts and Milestone 5b owns
  deletion of `ModelRefV2` and path-shaped success paths.

## Invariants
- `SchedulableTaskIntent` is the only runtime task request shape accepted by
  scheduler readiness, resource, batching, dispatch, and handoff policy; it
  must not contain local paths or executable Pumas load targets.
- `SchedulerTaskExecutionIntent` separates runtime executable state from
  non-runtime executable state. Non-runtime task intent may drive node-engine
  adapter execution, but it must not be accepted by runtime readiness,
  resource, batching, dispatch, or handoff policy.
- `SchedulerRuntimeHandoff` may carry dispatch facts only through
  `SchedulerDispatchDecision`.
- Readiness proof, task state, resource snapshots, batching decisions, and
  lifecycle diagnostics are separate contracts; none of them may silently stand
  in for another.
- Runtime choices from a graph are hard constraints only when explicitly
  provided; otherwise scheduler policy selects the runtime.
- All successful scheduler-facing shapes must validate into typed wrappers
  before policy or host handoff consumes them.

## Revisit Triggers
- A scheduler algorithm needs to own I/O directly instead of consuming a
  validated observation, preflight result, or host shell result.
- A new model family or runtime requires untyped JSON maps instead of a typed
  trait/value extension.
- Runtime host execution needs data that cannot be represented as a
  `SchedulerDispatchDecision` plus host-local Pumas load-target resolution.

## Dependencies
**Internal:** `pantograph-dependency-planning` for path-free dependency
preflight proof and environment refs; `pumas-models` for authoritative
`PumasModelRef`; `pantograph-runtime-identity` for typed runtime/device ids.

**External:** `serde` for transported DTOs and Rust standard library types for
validation and arithmetic.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```rust
use pantograph_scheduler::{
    SchedulerRuntimeHandoff, ValidatedSchedulerRuntimeHandoff,
};

let raw: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
    "../tests/fixtures/runtime_handoff_readiness_admitted.json"
))?;
let _validated = ValidatedSchedulerRuntimeHandoff::try_from(raw)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## API Consumer Contract
- Inputs: raw scheduler DTOs decoded from graph, IPC, queue, ledger, or host
  boundaries. Consumers must validate them into the matching
  `Validated*` wrapper before using the data for scheduler policy or handoff.
- Outputs: typed scheduler DTOs and helper results that carry explicit
  contract versions, typed ids, bounded diagnostics, and no executable paths.
- Lifecycle: this source directory defines contracts and pure policy helpers;
  long-running work is represented by supervision contracts and owned by the
  scheduler lifecycle service outside these DTO modules.
- Errors: validation returns `SchedulerContractError`; scheduler policy errors
  must use typed diagnostics rather than string parsing or silent fallback.
- Compatibility: breaking public DTO changes require fixture, README, adapter,
  binding, and migration-plan updates in the same slice.

## Structured Producer Contract
- Stable fields are the public serde field names, contract version constants,
  typed ids, enum variant semantics, and validated wrapper behavior exported by
  `lib.rs`.
- Defaults are explicit per DTO. Omitted runtime/device constraints mean
  scheduler policy decides; omitted dispatch decisions are valid only for
  readiness-admitted handoff.
- Enum order is not stable. Variant names and meanings are stable for
  persisted consumers unless a migration plan says otherwise.
- Fixtures under `tests/fixtures/` must be updated when a persisted DTO shape
  changes.
- New scheduler traits, diagnostics, states, or runtime facts must be added as
  typed fields or enum variants, not incidental metadata maps.
