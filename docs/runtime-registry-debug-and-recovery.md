# Runtime Registry Debug And Recovery

## Purpose
This document explains how Pantograph exposes runtime-registry, runtime-debug,
targeted-reclaim, and recovery state during Milestone 6. It exists so
developers and operators can inspect runtime posture without misreading Tauri
transport helpers as the owner of runtime policy or recovery truth.

## Scope

### In Scope

- runtime-registry snapshot inspection
- aggregate runtime debug snapshot inspection
- targeted reclaim behavior
- recovery and restart synchronization behavior
- Milestone 6 rollout-safety posture

### Out of Scope

- admission or technical-fit scoring policy design
- scheduler-v2 planning
- frontend UX design for diagnostics views
- distributed or multi-host runtime coordination

## Runtime Debug Surfaces

Pantograph currently exposes three relevant Tauri command surfaces:

1. `get_runtime_registry_snapshot`
   Returns the current backend-owned `RuntimeRegistrySnapshot` after
   synchronizing from the shared gateway.
2. `get_runtime_debug_snapshot`
   Returns a transport-only aggregate view that includes:
   - synced `ServerModeInfo`
   - synced `RuntimeRegistrySnapshot`
   - health monitor status and last check
   - recovery manager state
   - workflow runtime diagnostics
   - workflow scheduler diagnostics
   - optional workflow trace data
3. `reclaim_runtime_registry_runtime`
   Requests a backend-owned reclaim disposition for a specific runtime id and
   returns both the reclaim result and the updated registry snapshot.

These commands live in `src-tauri/src/llm/commands/registry.rs`, but they do
not own runtime policy. They aggregate or forward backend-owned data:

- registry state and reclaim disposition come from
  `pantograph-runtime-registry`
- sync, restore, and reclaim coordination come from
  `pantograph-embedded-runtime::runtime_registry`
- workflow runtime/scheduler/trace facts come from
  `pantograph-workflow-service` and `pantograph-embedded-runtime`

## Recovery Ownership

Automatic and manual recovery are coordinated from `src-tauri/src/llm/recovery.rs`,
but restart planning and runtime-registry reconciliation are backend-owned.

The current path is:

1. Tauri health monitoring or a manual command triggers recovery.
2. `RecoveryManager` selects a strategy and restart timing.
3. Backend-owned restart planning comes from
   `pantograph_embedded_runtime::runtime_recovery`.
4. Producer stop, restore, and post-restart registry synchronization reuse the
   shared runtime-registry helper paths rather than host-local bookkeeping.
5. Runtime debug and registry snapshot reads re-synchronize from those backend
   helpers before returning state to the caller.

The important rule is that recovery does not "fix up" registry state by
manually editing Tauri-local caches. It converges state by reusing the same
backend-owned stop, restore, sync, and reclaim helpers that other runtime
paths already use.

## Targeted Reclaim Semantics

Targeted reclaim is an operational inspection and cleanup tool, not a second
eviction engine. The host asks the registry whether a runtime should:

- remain retained
- be marked stopped in the registry only
- stop a live producer before final reconciliation

The reclaim action is backend-owned and returned as
`RuntimeReclaimDisposition`. Tauri only forwards the request and maps the
result into a command response.

## Rollout-Safety Decision

Milestone 6 does not add a new rollout toggle.

Reason:
- the runtime debug and reclaim surfaces are additive inspection/operational
  wrappers over backend-owned state that is already present in the app
- adding a new flag would create configuration and lifecycle semantics that are
  harder to reason about than the current always-available internal tooling
- existing recovery settings in `RecoveryConfig` are operational controls for
  recovery behavior, not a staged rollout switch for the Milestone 6 features

Revisit triggers:
- a non-Tauri host cannot support one of these commands safely
- a future release needs staged rollout for a new persisted diagnostics
  artifact
- a command becomes user-facing in a way that needs compatibility gating

## Operator Guidance

- Treat runtime-registry snapshots as the authoritative source for current
  runtime residency and reclaim posture.
- Use the aggregate runtime debug snapshot when you need one synchronized view
  across gateway mode, health, recovery, runtime diagnostics, and optional
  workflow trace state.
- If reclaim or recovery behavior looks inconsistent, inspect the backend-owned
  registry snapshot first rather than inferring state from stale UI overlays.
- When investigating restore or restart issues, prefer the synchronized helper
  paths over ad hoc manual stop/start sequences so registry state converges
  through the supported backend-owned flow.

## Traceability

- Runtime-registry ownership ADR:
  `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Runtime host adapter boundary:
  `src-tauri/src/llm/README.md`
- Registry policy boundary:
  `crates/pantograph-runtime-registry/src/README.md`
- Milestone 6 execution plan:
  `docs/completed-plans/IMPLEMENTATION-PLAN-pantograph-milestone-6-diagnostics-documentation-rollout-safety.md`
