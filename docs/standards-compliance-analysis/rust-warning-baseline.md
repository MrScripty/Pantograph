# Rust Warning Baseline

Last refreshed: 2026-04-21

Command used for the baseline:

```bash
cargo check --workspace --all-features --message-format short
```

The command completes successfully, but the workspace currently emits 82
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
| `pantograph` Tauri binary | 80 | Remove migrated/stale local workflow and server-discovery code | Delete superseded workflow local DTO/validation/registry/connection helpers and unused discovery paths after confirming no command references remain. |
| `pantograph_rustler` | 6 | External macro/dependency exception | Resolve by upgrading/fixing `rustler::resource!`, or add a scoped lint exception with dependency rationale. |

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

### `crates/workflow-nodes` Model Provider

| Location | Warning | Classification | Resolution |
| --- | --- | --- | --- |
| `src/input/model_provider.rs:162` | `ModelProviderExecutor` never constructed | Remove | Resolved by deleting the unregistered executor implementation; active model-provider projection is owned by `node-engine` core executor handlers. |
| `src/input/model_provider.rs:165` | `ModelProviderExecutor::factory` unused | Remove | Resolved with the inactive executor removal. |
| `src/input/model_provider.rs:170` | `ModelProviderExecutorFactory` never constructed | Remove | Resolved with the inactive executor removal. |
| `src/input/model_provider.rs:272` and `:277` | `ResolvedModel` and `resolve_with_library` unused | Remove | Resolved by deleting the executor-only library resolver path. Pumas-backed selection remains in `puma-lib` and setup helpers. |

## Active Warnings

### `crates/pantograph-embedded-runtime`

| Location | Warning | Classification | Next action |
| --- | --- | --- | --- |
| `src/lib.rs:6` | unused `node_engine::ExecutorExtensions` import | Remove | Delete after preserving the current dirty embedded-runtime work. |
| `src/lib.rs:15` | unused `tokio::sync::RwLock` import | Remove | Delete after preserving the current dirty embedded-runtime work. |

### `src-tauri` / `pantograph` binary

| Location | Warning group | Classification | Next action |
| --- | --- | --- | --- |
| `src/llm/health_monitor.rs` | `consecutive_failures` unused | Remove/use | Expose through diagnostics if still useful; otherwise delete. |
| `src/llm/server_discovery.rs` | registry file, DTOs, discovery type, helpers, and display function unused | Remove/feature-gate | The module is still exported but has no active consumers. Delete the module or gate it behind a documented local-server-discovery feature. |
| `src/workflow/connection_intent.rs` | local connection candidate/commit/insert helpers unused | Remove | Superseded by `pantograph-workflow-service::graph::connection_intent`; delete after command imports are confirmed clean. |
| `src/workflow/effective_definition.rs` | local effective-definition resolver unused | Remove | Superseded by workflow-service graph definitions; delete with the stale local workflow type cleanup. |
| `src/workflow/events.rs` | workflow event constructor helpers unused | Remove/use | Delete inactive constructors or move expected construction through the backend-owned event adapter. |
| `src/workflow/execution_manager.rs` and `execution_manager/state.rs` | execution state, undo/redo, cleanup, and accessors mostly unused | Remove/replace | Confirm Tauri state still needs `ExecutionManager`; then delete migrated session-manager logic or route remaining commands through backend-owned workflow sessions. |
| `src/workflow/registry.rs` | local node registry conversion layer unused | Remove | Superseded by workflow-service graph registry and direct `node_engine::NodeRegistry` command state. |
| `src/workflow/types.rs` | local workflow DTOs, graph helpers, fingerprint helpers, file metadata unused | Remove | Delete once stale local connection/validation/registry modules are removed and command DTO imports are migrated. |
| `src/workflow/validation.rs` | local validator and helper unused | Remove | Superseded by workflow-service and node-engine validation paths. |

### `crates/pantograph-rustler`

| Location | Warning | Classification | Next action |
| --- | --- | --- | --- |
| `src/resource_registration.rs:9-14` | `rustler::resource!` emits `non_local_definitions` | External dependency exception | Prefer a `rustler` update or upstream-compatible registration API change. If blocked, add the narrowest lint exception around resource registration with the macro rationale and removal trigger. |

## Ratchet Plan

1. Remove simple unused imports, fields, constants, and wrappers in isolated
   commits.
2. Delete the stale Tauri-local workflow connection, registry, validation, and
   type modules once command imports are confirmed to use backend-owned
   workflow-service contracts.
3. Decide whether server discovery is active product behavior. Delete it if not;
   otherwise put it behind an explicit feature and command surface.
4. Resolve intentional/external exceptions with narrow comments or lint allows.
5. Re-run all-features and no-default-features checks, record the new warning
   count, and promote `clippy -D warnings` once the baseline reaches zero or a
   machine-enforced exception list exists.
