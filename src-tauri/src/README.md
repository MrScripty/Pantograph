# src-tauri/src

Tauri desktop backend source boundary.

## Purpose
This directory owns Pantograph's desktop composition root, command transport,
workflow host integration, local runtime wiring, and app-specific backend
services. It adapts desktop state into backend-owned crates without becoming
the canonical owner of workflow, runtime registry, or node execution policy.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `main.rs` | Thin crate-root launcher, module declarations, logging bootstrap, and fatal startup error reporting. |
| `app_setup.rs` | Tauri builder composition, managed state registration, setup-time resource initialization, and command registration. |
| `app_lifecycle.rs` | Window lifecycle shutdown hook that stops owned workers, invalidates loaded workflow runtimes, and syncs runtime-registry state. |
| `app_tasks.rs` | App-owned async task registry for startup/setup work that must be stopped during shutdown. |
| `config.rs` | Desktop configuration structures and persistence integration. |
| `constants.rs` | Tauri backend constants shared across modules. |
| `agent/` | Assistant documentation, retrieval, enrichment, and tool support. |
| `bin/` | Developer/runtime helper binaries compiled from the Tauri crate. |
| `hotload_sandbox/` | Runtime Svelte component validation and sandbox helpers. |
| `llm/` | LLM gateway, runtime registry, model server, and related command adapters. |
| `workflow/` | Tauri workflow command transport and desktop runtime integration. |

## Problem
The desktop backend composes many long-lived services and transport adapters.
Without an explicit boundary, Tauri modules can accumulate product policy that
belongs in backend crates such as workflow service, runtime registry, inference,
or embedded runtime.

## Constraints
- Tauri command handlers are adapters over backend-owned contracts.
- Startup/shutdown and spawned task ownership must be explicit.
- Desktop-only state may be composed here, but durable workflow/runtime policy
  must stay in backend crates.
- Public command payloads must remain aligned with frontend TypeScript
  consumers.

## Decision
Keep desktop composition and command wiring in `src-tauri/src`. Move reusable
workflow, runtime, inference, and binding behavior into workspace crates, then
let Tauri inject app state and translate command payloads.
Session-scoped node group commands are registered here only as transport
entrypoints; collapsed group mutation policy lives in
`pantograph-workflow-service`.
Workflow save/load/list commands likewise delegate to the service graph store;
Tauri no longer keeps a parallel workflow persistence/path-validation module.
Window close shutdown now lives in `app_lifecycle.rs`, and startup composition
now lives in `app_setup.rs`, so `main.rs` stays a small crate-root launcher.
Startup resource failures flow through `app_setup::run_app()` and the Tauri
setup result with logged context instead of production `expect(...)` panics.
Startup/setup async tasks are registered in `app_tasks.rs` and drained during
window shutdown before runtime workers and model processes are stopped.
Window shutdown also stops health monitoring and any tracked automatic recovery
task before workflow cleanup and runtime process shutdown.
`main.rs` no longer registers a Tauri-local workflow execution manager; edit
session undo/redo and execution state are injected through the backend-owned
workflow service instead.
Strict Rust lint cleanup in this tree should prefer narrow type/signature
improvements and backend-owned grouping DTOs over local adapter exceptions.

## Alternatives Rejected
- Put workflow/runtime policy directly in Tauri commands: rejected because
  embedded/runtime bindings and tests need the same behavior without desktop
  transport.
- Move all desktop state into backend crates: rejected because Tauri owns app
  handles, windows, command registration, and desktop-specific lifecycle.

## Invariants
- `main.rs` should remain a thin launcher and module declaration surface.
- `app_setup.rs` owns desktop composition wiring and must not accumulate
  workflow, runtime registry, or node execution policy.
- Command modules must preserve backend error categories.
- Long-lived tasks and process handles need owned shutdown paths.
- Window-close cleanup must route through `app_lifecycle.rs` rather than inline
  shutdown policy in `main.rs`.
- Startup/setup failures for required resources must return logged errors
  instead of panicking.
- Startup/setup tasks spawned from the composition root must be tracked by the
  app task registry.
- Window shutdown must stop health and recovery background tasks before
  tearing down workflow runtimes and model processes.
- Tauri-local DTOs should migrate toward shared backend contracts where
  practical.
- `constants.rs` should contain only shared values with active consumers; stale
  defaults and one-off literals should be deleted or moved to the owning module.
- Command registrations for graph edits, including node group mutations, must
  return backend-owned service snapshots instead of adapter-owned graph facts.
- Workflow persistence command registrations must delegate path validation and
  file IO policy to the service graph store.
- Retention cleanup command registration must delegate artifact expiration and
  audit-event emission to the workflow service; Tauri only transports the
  typed request/response.
- Pumas model delete command registration must delegate deletion to Pumas and
  audit-event emission to the workflow service; Tauri validates auditable
  identifiers and transports the typed response.
- Pumas HuggingFace search command registration must delegate search to Pumas
  and audit-event emission to the workflow service; Tauri validates query
  bounds and transports the typed response.
- Pumas HuggingFace download-start command registration must delegate download
  startup to Pumas and audit-event emission to the workflow service; Tauri
  validates auditable repository identifiers and transports the typed response.
- Desktop composition must not register parallel workflow execution-state
  managers when the workflow service owns the active session state.
- Mechanical lint fixes must not change command payload shape, runtime
  ownership, or backend error categories.

## Cargo Feature Contract
| Feature | Default | Contract |
| ------- | ------- | -------- |
| `backend-llamacpp` | Yes | Enables llama.cpp support in the desktop app and embedded runtime. |
| `backend-ollama` | Yes | Enables Ollama support in the desktop app and embedded runtime. |
| `backend-candle` | Yes | Enables local Candle support plus optional Candle/tokenizer/http streaming dependencies. |
| `backend-pytorch` | No | Enables Python/PyTorch backend support and node-engine PyTorch nodes. Requires host Python dependencies. |
| `backend-audio` | No | Enables Python-backed audio node support. Requires host audio Python dependencies. |

Defaults match the current desktop local-backend product set. Python-backed
families remain explicit because they require host interpreter packages and
larger runtime setup.

## Revisit Triggers
- `main.rs` composition extraction changes module ownership.
- A command surface becomes supported outside the desktop app.
- Runtime lifecycle supervision moves into a dedicated backend service.

## Dependencies
**Internal:** workspace Rust crates, frontend command consumers, Tauri command
modules, workflow/runtime/inference services.

**External:** Tauri 2, Tokio, serde, and platform process/filesystem APIs.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md`

## Usage Examples
```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![])
    .run(tauri::generate_context!())?;
```

## API Consumer Contract
- Inputs: Tauri invoke payloads, app state, filesystem paths, runtime handles,
  and backend service requests.
- Outputs: command responses, events, windows, logs, and persisted desktop app
  state.
- Lifecycle: Tauri creates shared services during setup and must release owned
  tasks/processes during shutdown.
- Errors: backend errors should stay categorized when projected across command
  boundaries.
- Versioning: command payload changes require frontend TypeScript and tests to
  migrate together.

## Structured Producer Contract
- Stable fields: command response DTOs, event payloads, config state, and saved
  runtime/workflow projections are machine-consumed.
- Defaults: desktop defaults must be documented near configuration owners.
- Enums and labels: runtime ids, command labels, and event names carry behavior.
- Ordering: event delivery order should preserve backend producer order where
  visible to the frontend.
- Compatibility: persisted desktop state and command payloads may outlive a
  single process run.
- Regeneration/migration: command DTO changes require frontend services, tests,
  and docs updates in the same slice.

## Testing
```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Notes
- M3 tracks deeper startup/shutdown and task-supervision cleanup.
