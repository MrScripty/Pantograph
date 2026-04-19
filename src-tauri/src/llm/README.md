# src-tauri/src/llm

## Purpose
This directory contains Pantograph's Tauri-side LLM composition and transport
layer. It wires desktop commands, gateway-backed runtime lifecycle operations,
health/recovery helpers, and startup/config adaptation onto the backend-owned
runtime contracts exposed by `crates/inference`.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `gateway.rs` | Tauri-facing wrapper around `inference::InferenceGateway` that adapts app-state wiring and startup helpers without replacing the backend facade. |
| `commands/` | Tauri command handlers for backend selection, server lifecycle, config reads/writes, runtime-status queries, and thin redistributable-manager transport over backend-owned contracts. |
| `runtime_registry.rs` | Tauri adapter that translates backend lifecycle facts into the backend-owned runtime-registry crate. |
| `rag_sync.rs` | Host-only helper that keeps the Tauri RAG consumer aligned with gateway-owned embedding runtime availability. |
| `health_monitor.rs` | App-owned health polling loop that maps HTTP probe results onto backend-owned health assessment and emits desktop events. |
| `recovery.rs` | Recovery orchestration that reacts to runtime failures and retries through the shared gateway. |
| `startup.rs` | Shared startup request construction and model-path resolution for Tauri-side runtime launches. |
| `process_tauri.rs` | Tauri-specific process spawning bridge used when the app must launch managed runtimes. |
| `server_discovery.rs` | Local runtime/server registration and discovery helpers used by the desktop host. |

## Problem
Pantograph's desktop app still needs a native composition layer for runtime
startup, server connection, health monitoring, and user-triggered backend
control. That host wiring must remain thin enough that runtime lifecycle facts
continue to come from the backend, while app-specific coordination stays in the
desktop composition layer.

## Constraints
- The backend-owned `InferenceGateway` contract must remain the execution
  facade.
- Tauri commands must not become the owner of app-global runtime residency,
  admission, retention, or eviction policy.
- Health/recovery helpers must consume backend facts rather than deriving their
  own runtime truth model.
- Host-owned consumers that cache embedding endpoints, such as the RAG manager,
  must refresh from gateway facts after runtime lifecycle changes rather than
  persisting adapter-local availability guesses.
- Startup and config mapping must stay compatible with existing GUI command
  surfaces until an explicit contract change is approved.

## Decision
Keep `src-tauri/src/llm` as the desktop host adapter and app-composition layer
for runtime control. The Tauri app creates and manages the shared gateway in
`src-tauri/src/main.rs`, injects it into command modules, and uses this
directory for transport mapping, Tauri-specific spawning, and app-owned
monitoring loops. The shared `RuntimeRegistry` is still created from the app
composition root and injected through this layer, but the registry state
machine now lives in `crates/pantograph-runtime-registry` so transport code
does not own runtime policy.

## Alternatives Rejected
- Move runtime policy into `gateway.rs`.
  Rejected because `InferenceGateway` must stay the execution facade and source
  of backend lifecycle facts, not the owner of Pantograph scheduler policy.
- Let workflow commands own runtime health and recovery decisions directly.
  Rejected because runtime lifecycle coordination is a shared concern across the
  desktop host, not a workflow-only policy boundary.

## Invariants
- `src-tauri/src/main.rs` is the current desktop composition root for the shared
  gateway and related host-owned runtime services.
- Tauri commands in this directory adapt host calls onto backend contracts; they
  do not redefine runtime lifecycle truth.
- Managed-runtime redistributable command surfaces must forward onto the
  backend-owned manager/view contract exposed by `pantograph-embedded-runtime`
  rather than branching directly on `inference` install-state helpers inside
  Tauri.
- New managed runtime families must extend the existing managed-runtime
  transport surfaces and shared frontend service boundary; do not add
  runtime-specific Tauri command modules or GUI-only state ownership just
  because one runtime needs a new install or selection flow.
- Health and recovery flows must operate through shared gateway-backed state,
  not independent adapter-local state machines.
- Recovery retry loops may gather host facts such as port availability, but
  alternate-port fallback and clean-restart sequencing must come from backend
  recovery helpers rather than from Tauri-local branching.
- Host-owned caches of embedding runtime availability must be synchronized from
  gateway facts whenever lifecycle commands or recovery change the active
  embedding producer.
- Runtime-registry injection passes through this layer, but runtime residency
  and admission policy must not be implemented in command handlers or other
  Tauri transport modules.
- Recovery orchestration may perform host-specific restart steps here, but the
  “run transition, then reconcile registry” sequencing must stay in backend
  lifecycle helpers rather than as a separate adapter-local sync step.
- Adapter-level registry tests in this directory should pin that shared
  transition helper for both successful and failed host transitions so Tauri
  wrappers cannot silently reintroduce post-transition registry drift.

## Revisit Triggers
- A non-Tauri app root needs the same runtime composition logic and this
  directory stops being a clear desktop-only adapter boundary.
- Runtime-registry ownership now lives in
  `crates/pantograph-runtime-registry`; this layer should remain a thin adapter
  over that crate.
- Health/recovery behavior grows beyond simple adapter orchestration and needs
  an ADR-level boundary split.

## Dependencies
**Internal:** `src-tauri/src/main.rs`, `src-tauri/src/config`, `crates/inference`,
and the workflow command layer that consumes shared gateway state.

**External:** Tauri state management/runtime, serde, and desktop process-spawn
capabilities required by managed runtimes.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md`
- Reason: ADR-001 freezes the service/adapter split, ADR-002 freezes this
  directory as a host adapter/composition layer rather than the owner of the
  planned runtime-policy layer, and ADR-003 freezes redistributable lifecycle
  ownership in backend Rust rather than in Tauri transport.
- Revisit trigger: the desktop host no longer owns the composition root for
  runtime services.

## Usage Examples
```rust
let gateway: SharedGateway = Arc::new(InferenceGateway::new(spawner));
tauri::async_runtime::block_on(async { gateway.init().await });
app.manage(gateway);
```

## API Consumer Contract
- Tauri commands and app startup code consume the shared gateway through the
  `SharedGateway` alias exported from this directory.
- Command handlers return backend-owned runtime status and capability facts; the
  GUI should treat those as authoritative over local inference about runtime
  state.
- Runtime-debug command surfaces may aggregate synced registry, lifecycle,
  health, recovery, and latest workflow diagnostics facts for internal
  debugging, but that aggregation must remain transport-only and must not
  redefine runtime policy in the host layer. When workflow-specific filters are
  needed, this layer should reuse the shared workflow diagnostics projection
  helper that also backs workflow diagnostics command reads rather than
  building a second local diagnostics path. Optional workflow trace reads must
  likewise reuse the shared backend workflow trace snapshot helper so Tauri
  does not start owning trace assembly or fallback logic.
- Runtime-debug request normalization may trim filter values at the command
  boundary, but blank-filter rejection and the accepted filter vocabulary must
  stay aligned with the backend-owned trace/diagnostics contracts. If
  `workflow_name` is supported for a combined diagnostics-plus-trace read, that
  support must come from the backend trace contract rather than from a
  Tauri-only filter path.
- Long-lived host services such as health monitoring and recovery must be
  started and stopped by the app composition root or another explicit owner,
  not by arbitrary UI calls. Command handlers may invoke those managed
  services, but they must not create replacement service instances on demand.
- Milestone 6 does not add a new rollout toggle for runtime debug or targeted
  reclaim transport. These command surfaces stay additive and always available
  to the desktop host because they forward already-owned backend state rather
  than introducing a second policy boundary.
- Compatibility policy is additive: command surfaces may grow, but existing
  backend-owned status shapes should remain stable unless an explicit contract
  change is approved.

## Structured Producer Contract
- `gateway.rs` exposes backend-owned lifecycle and capability facts through the
  Tauri host; adapter code must preserve canonical runtime ids and backend keys
  instead of inventing local aliases.
- `runtime_registry.rs` may translate either full gateway mode snapshots or
  single producer-specific runtime snapshots into backend-owned registry
  observations, but it must not become the owner of lifecycle or retention
  policy.
- `commands/registry.rs` may aggregate runtime mode, registry, health,
  recovery, workflow diagnostics, and optional workflow trace facts into one
  debug response, but it must do so by reusing backend-owned snapshot helpers
  and must not cache or redefine runtime truth locally.
- `health_monitor.rs` may own polling cadence, HTTP transport, and desktop
  event emission, but degraded/unhealthy threshold interpretation must come
  from `crates/pantograph-embedded-runtime::runtime_health`.
- When health polling needs to project runtime failure into the shared runtime
  registry, this layer must route that projection through
  `crates/pantograph-embedded-runtime::runtime_registry` rather than mutating
  registry state from host-local policy.
- Dedicated embedding runtime health polling may reuse host HTTP transport, but
  any active-vs-embedding unhealthy projection still belongs to backend
  `runtime_registry` helpers rather than to Tauri-local branching.
- When this layer persists host-observed health overlays for later registry
  synchronization, it must treat them as backend-owned facts keyed by runtime
  id plus runtime instance id and clear them on lifecycle-changing transitions
  rather than inventing a Tauri-local health state machine.
- When this directory synchronizes registry state from the shared gateway, it
  must use the richer Tauri `mode_info()` snapshot rather than the narrower
  core-gateway view, and it should convert that snapshot into the backend-owned
  `HostRuntimeModeSnapshot` contract so dedicated embedding-sidecar facts are
  not dropped.
- Command payloads emitted from this directory are transport wrappers around
  backend/runtime contracts, not a separate policy schema.
- Existing `RecoveryConfig` fields control recovery behavior only; they are not
  a feature-gating mechanism for runtime-registry visibility or targeted
  reclaim.
- Health/recovery overlays may add host-only fields, but they must not mutate
  the meaning of backend-owned lifecycle facts.
- This directory must distinguish raw backend facts from registry policy
  decisions owned by `crates/pantograph-runtime-registry`.
