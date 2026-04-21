# Rust Warning Baseline

Last refreshed: 2026-04-21

Command used for the baseline:

```bash
cargo check --workspace --all-features --message-format short
```

The command completes successfully, but the workspace currently emits 99
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
| `node-engine` | 2 | Remove unused helpers | Delete unused multi-demand test helpers or make tests consume them. |
| `workflow-nodes` | 7 | Remove simple unused fields/imports; gate experimental model-provider executor | Remove unused Ollama response fields/imports; either register the model-provider executor or gate it behind an experimental feature. |
| `pantograph-workflow-service` | 1 | Remove or test-scope helper | Remove the unused wrapper or move it behind `#[cfg(test)]`. |
| `pantograph-embedded-runtime` | 2 | Remove unused imports | Drop unused public imports when the dirty embedded-runtime branch is normalized. |
| `pantograph-uniffi` | 1 | Intentional retained ownership field | Rename/comment the field or add a narrow allow because it holds the executor alive for exported UniFFI resources. |
| `pantograph` Tauri binary | 80 | Remove migrated/stale local workflow and server-discovery code | Delete superseded workflow local DTO/validation/registry/connection helpers and unused discovery paths after confirming no command references remain. |
| `pantograph_rustler` | 6 | External macro/dependency exception | Resolve by upgrading/fixing `rustler::resource!`, or add a scoped lint exception with dependency rationale. |

## Detailed Classification

### `crates/node-engine`

| Location | Warning | Classification | Next action |
| --- | --- | --- | --- |
| `src/engine/multi_demand.rs:225` | `DemandExecutionBudget::sequential` unused | Remove/use | Delete if no multi-demand tests need sequential-mode construction. If the helper captures intended behavior, consume it from a focused test. |
| `src/engine/multi_demand.rs:290` | `DemandBatchExecutionOutcome::completed_targets` unused | Remove/use | Delete if the public outcome API no longer needs a completed-target accessor; otherwise add a focused assertion through the local test boundary. |

### `crates/workflow-nodes`

| Location | Warning | Classification | Next action |
| --- | --- | --- | --- |
| `src/processing/ollama_inference.rs:12` | unused `Serialize` import | Remove | Delete the import. |
| `src/processing/ollama_inference.rs:19` | `OllamaResponse.done` never read | Remove/use | Remove the field if streaming completion state is not consumed; serde ignores unknown response fields by default. |
| `src/processing/ollama_inference.rs:21` | `OllamaResponse.context` never read | Remove/use | Remove the field unless the node needs to persist or chain Ollama context tokens. |
| `src/input/model_provider.rs:162` | `ModelProviderExecutor` never constructed | Gate/use | Treat as experimental until registered by the node registry or moved behind a feature. |
| `src/input/model_provider.rs:165` | `ModelProviderExecutor::factory` unused | Gate/use | Same as the executor: register, feature-gate, or delete the inactive implementation. |
| `src/input/model_provider.rs:170` | `ModelProviderExecutorFactory` never constructed | Gate/use | Same as the executor. |
| `src/input/model_provider.rs:272` and `:277` | `ResolvedModel` and `resolve_with_library` unused | Gate/use | These are only reachable through the inactive executor; resolve with the executor decision. |

### `crates/pantograph-workflow-service`

| Location | Warning | Classification | Next action |
| --- | --- | --- | --- |
| `src/graph/session_contract.rs:76` | `build_graph_session_response` unused | Remove/test-scope | Delete the wrapper if all callers use state-aware projection; otherwise make it test-only. |

### `crates/pantograph-embedded-runtime`

| Location | Warning | Classification | Next action |
| --- | --- | --- | --- |
| `src/lib.rs:6` | unused `node_engine::ExecutorExtensions` import | Remove | Delete after preserving the current dirty embedded-runtime work. |
| `src/lib.rs:15` | unused `tokio::sync::RwLock` import | Remove | Delete after preserving the current dirty embedded-runtime work. |

### `crates/pantograph-uniffi`

| Location | Warning | Classification | Next action |
| --- | --- | --- | --- |
| `src/lib.rs:586` | `task_executor` field never read | Intentionally retained/use | This appears to retain executor ownership for exported runtime resources. Make the ownership contract explicit by reading it through a small accessor, renaming to `_task_executor`, or applying a narrow lint allow with a comment. |

### `src-tauri` / `pantograph` binary

| Location | Warning group | Classification | Next action |
| --- | --- | --- | --- |
| `src/agent/rag/mod.rs`, `src/agent/rag/manager.rs` | unused `RagManager` re-export and `store_path` | Remove/use | Remove the unused export/method unless a command or diagnostic surface needs it. |
| `src/constants.rs` | unused inference/server constants | Remove | Delete stale constants after confirming no external config contract references them. |
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
