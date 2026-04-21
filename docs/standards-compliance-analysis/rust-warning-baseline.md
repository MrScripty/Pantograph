# Rust Warning Baseline

Last refreshed: 2026-04-21

Command used for the baseline:

```bash
cargo check --workspace --all-features --message-format short
```

The command completes successfully, but the workspace currently emits 2
warnings. This document classifies the warning debt required by M7 before
`cargo clippy --workspace --all-targets --all-features -- -D warnings` can
become a blocking quality gate.

The classification follows the updated standards expectations:

- tooling gates should fail on warnings once baseline debt is resolved;
- temporary lint debt needs a committed baseline and ratchet path;
- dead code should be removed, used, feature-gated, or explicitly documented.

## Summary

| Crate/target | Count | Primary category | Resolution path |
| --- | ---: | --- | --- |
| `pantograph-embedded-runtime` | 2 | Remove unused imports | Drop unused public imports when the dirty embedded-runtime branch is normalized. |
| `pantograph` Tauri binary | 0 | Complete | Tauri-local warning debt has been removed; keep command adapters on backend-owned DTOs and service contracts. |
| `pantograph_rustler` | 0 | Scoped external macro exception | Keep the narrow `rustler::resource!` lint exception documented until Rustler exposes a warning-clean registration API. |

## Resolved Warnings

### `crates/node-engine`

| Location | Warning | Classification | Next action |
| --- | --- | --- | --- |
| `src/engine/multi_demand.rs:225` | `DemandExecutionBudget::sequential` unused | Test-scope | Resolved by compiling the helper only for tests, where the sequential dispatch-plan tests consume it. |
| `src/engine/multi_demand.rs:290` | `DemandBatchExecutionOutcome::completed_targets` unused | Test-scope | Resolved by compiling the accessor only for tests, where batch outcome ordering assertions consume it. |

### `crates/workflow-nodes`

| Location | Warning | Classification | Resolution |
| --- | --- | --- | --- |
| `src/processing/ollama_inference.rs:12` | unused `Serialize` import | Remove | Resolved by importing only `Deserialize`. |
| `src/processing/ollama_inference.rs:19` | `OllamaResponse.done` never read | Remove | Resolved by omitting the unconsumed response field; serde ignores unknown Ollama fields. |
| `src/processing/ollama_inference.rs:21` | `OllamaResponse.context` never read | Remove | Resolved by omitting the unconsumed response field; context-token reuse is not part of this node contract. |

### `crates/pantograph-workflow-service`

| Location | Warning | Classification | Resolution |
| --- | --- | --- | --- |
| `src/graph/session_contract.rs:76` | `build_graph_session_response` unused | Test-scope | Resolved by compiling the compatibility wrapper only for session-contract tests; production response assembly uses the state-aware projection helper. |

### `crates/pantograph-uniffi`

| Location | Warning | Classification | Resolution |
| --- | --- | --- | --- |
| `src/lib.rs:586` | `task_executor` field never read | Remove | Resolved by deleting the inactive no-op task executor field and helper from `FfiWorkflowEngine`; the binding object does not expose task execution. |

### `src-tauri` Constants

| Location | Warning | Classification | Resolution |
| --- | --- | --- | --- |
| `src/constants.rs:19` | `SERVER_STARTUP_SECS` unused | Remove | Resolved by deleting the unused timeout module. |
| `src/constants.rs:25` | `CONTEXT_SIZE` unused | Remove | Resolved by deleting the stale default; active inference defaults stay with their owning config/runtime paths. |
| `src/constants.rs:35` | `AUTO` unused | Remove | Resolved by deleting the unused device-type module. |
| `src/constants.rs:41` | `LOCAL` unused | Remove | Resolved by deleting the unused host module. |

### `src-tauri` RAG Manager

| Location | Warning | Classification | Resolution |
| --- | --- | --- | --- |
| `src/agent/rag/mod.rs:15` | unused `RagManager` re-export | Remove | Resolved by exporting only the shared handle and constructor; tests use `create_rag_manager`. |
| `src/agent/rag/manager.rs:454` | `store_path` unused | Remove | Resolved by deleting the private accessor; storage paths remain internal to manager commands/DTOs. |

### `src-tauri` Health Monitor

| Location | Warning | Classification | Resolution |
| --- | --- | --- | --- |
| `src/llm/health_monitor.rs:288` | `consecutive_failures` unused | Remove | Resolved by deleting the unused accessor; structured health results still carry failure counts for command/debug consumers. |

### `src-tauri` Server Discovery

| Location | Warning group | Classification | Resolution |
| --- | --- | --- | --- |
| `src/llm/server_discovery.rs` | unused registration DTOs, registry file constant, discovery owner, helpers, and `get_process_info` import | Remove | Resolved by deleting the unconsumed desktop-local discovery registry module and removing it from `llm::mod`; future discovery must return through active command/backend runtime-registry contracts. |

### `src-tauri` Legacy Graph Policy

| Location | Warning group | Classification | Resolution |
| --- | --- | --- | --- |
| `src/workflow/connection_intent.rs`, `effective_definition.rs`, `validation.rs` | unused local connection-intent, effective-definition, and validation helpers | Remove | Resolved by deleting the stale Tauri-local graph policy modules; active editing wrappers delegate to `pantograph-workflow-service`. |

### `src-tauri` Legacy Registry Mirror

| Location | Warning group | Classification | Resolution |
| --- | --- | --- | --- |
| `src/workflow/registry.rs` | unused local node registry conversion mirror | Remove | Resolved by deleting the stale Tauri-local registry module; active definition commands use `pantograph-workflow-service::NodeRegistry` and port-option paths use active `node_engine::NodeRegistry` state. |

### `src-tauri` Workflow Event Constructors

| Location | Warning group | Classification | Resolution |
| --- | --- | --- | --- |
| `src/workflow/events.rs` | unused workflow event constructor helpers | Remove | Resolved by deleting inactive convenience constructors and constructing enum variants directly in serialization tests; production snapshot constructors remain for active diagnostics paths. |

### `src-tauri` Legacy Execution Manager

| Location | Warning group | Classification | Resolution |
| --- | --- | --- | --- |
| `src/workflow/execution_manager.rs` and `src/workflow/execution_manager/state.rs` | unused Tauri-local execution state, undo/redo, stale cleanup, and accessors | Remove | Resolved by deleting the stale desktop-local execution manager and removing its managed Tauri state injection; undo/redo and session execution state stay with `pantograph-workflow-service`. |

### `src-tauri` Legacy Workflow Type Mirror

| Location | Warning group | Classification | Resolution |
| --- | --- | --- | --- |
| `src/workflow/types.rs` | unused local workflow DTOs, graph helpers, file metadata, and duplicated connection payloads | Remove | Resolved by deleting the stale Tauri-local type mirror and moving the last workflow event/group command references to `pantograph-workflow-service` graph and port DTOs. |

### `crates/workflow-nodes` Model Provider

| Location | Warning | Classification | Resolution |
| --- | --- | --- | --- |
| `src/input/model_provider.rs:162` | `ModelProviderExecutor` never constructed | Remove | Resolved by deleting the unregistered executor implementation; active model-provider projection is owned by `node-engine` core executor handlers. |
| `src/input/model_provider.rs:165` | `ModelProviderExecutor::factory` unused | Remove | Resolved with the inactive executor removal. |
| `src/input/model_provider.rs:170` | `ModelProviderExecutorFactory` never constructed | Remove | Resolved with the inactive executor removal. |
| `src/input/model_provider.rs:272` and `:277` | `ResolvedModel` and `resolve_with_library` unused | Remove | Resolved by deleting the executor-only library resolver path. Pumas-backed selection remains in `puma-lib` and setup helpers. |

### `crates/pantograph-rustler`

| Location | Warning | Classification | Resolution |
| --- | --- | --- | --- |
| `src/resource_registration.rs` | `rustler::resource!` emits `non_local_definitions` under the current Rustler macro expansion | Scoped external macro exception | Resolved by adding a narrow `#[allow(non_local_definitions)]` on the load-time resource registration function with removal guidance in the rustler README files. |

## Active Warnings

### `crates/pantograph-embedded-runtime`

| Location | Warning | Classification | Next action |
| --- | --- | --- | --- |
| `src/lib.rs:6` | unused `node_engine::ExecutorExtensions` import | Remove | Delete after preserving the current dirty embedded-runtime work. |
| `src/lib.rs:15` | unused `tokio::sync::RwLock` import | Remove | Delete after preserving the current dirty embedded-runtime work. |

## Ratchet Plan

1. Remove simple unused imports, fields, constants, and wrappers in isolated
   commits.
2. Keep Tauri-local workflow type, event, registry, graph-policy, and
   execution-manager mirrors deleted; command imports should use backend-owned
   workflow-service contracts.
3. Keep desktop-local server discovery, graph policy, and registry mirrors
   deleted unless a future feature reintroduces them through an explicit command
   and backend-owned contract.
4. Remove the Rustler macro lint exception when upstream resource registration
   no longer emits `non_local_definitions`.
5. Re-run all-features and no-default-features checks, record the new warning
   count, and promote `clippy -D warnings` once the baseline reaches zero or a
   machine-enforced exception list exists.
