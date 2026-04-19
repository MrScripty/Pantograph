# src-tauri/src/llm/commands

## Purpose
This directory contains thin Tauri command adapters for Pantograph's LLM and
runtime-management surfaces. These files accept desktop transport inputs,
resolve app-owned Tauri state such as `AppHandle` or shared gateway handles,
and forward requests onto backend-owned Rust contracts.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `agent.rs` | Agent-facing command transport that adapts Tauri requests onto backend orchestration helpers. |
| `backend.rs` | Backend selection and capability commands that forward onto shared gateway/runtime services. |
| `binary.rs` | Managed-runtime redistributable transport. It must stay a thin adapter over the backend-owned manager contract in `pantograph-embedded-runtime` rather than calling `inference` internals directly. |
| `config.rs` | App/config command transport that coordinates persisted configuration with shared workflow-service state. |
| `docs.rs` | Documentation and chunking command transport. |
| `embedding.rs` | Embedding-mode command transport and app-level coordination glue. |
| `health.rs` | Health-monitor and recovery-status transport over shared host services. |
| `mod.rs` | Module wiring and public re-exports for the Tauri invoke surface. |
| `port.rs` | Port-management transport helpers. |
| `rag.rs` | RAG command transport and registry-sync composition. |
| `registry.rs` | Runtime-registry and runtime-debug Tauri command entrypoints. |
| `registry/` | Focused helpers extracted from the runtime-registry command boundary. |
| `sandbox.rs` | Sandbox configuration transport. |
| `server.rs` | LLM server lifecycle commands that compose shared gateway and registry state. |
| `shared.rs` | Shared Tauri-only helper functions and aliases used by multiple command modules. |
| `version.rs` | Component history/versioning transport. |
| `vision.rs` | Vision/image prompt transport. |

## Boundary Rules
- Command handlers in this directory are transport adapters, not runtime-policy
  owners.
- Business logic for workflow execution, runtime readiness, redistributable
  state, version selection, and runtime-registry policy must stay in backend
  crates such as `pantograph-embedded-runtime`, `pantograph-workflow-service`,
  `pantograph-runtime-registry`, or `inference`.
- If a command needs new behavior, prefer adding an additive backend service or
  contract first, then let the Tauri command call that contract.
- Tauri-local logic here may normalize request envelopes, acquire app-owned
  state, and translate backend errors into command-level strings or JSON
  envelopes, but it must not derive a second source of truth for runtime state.

## Design Decisions
- Keep files in this directory scoped to one transport boundary each.
- Prefer explicit helper modules when a command surface grows DTO or test-only
  support logic, as `registry/` already does.
- Command handlers that affect embedding-runtime availability should reuse the
  shared host-side RAG sync helper instead of caching embedding endpoints with
  command-local logic.
- Managed-runtime redistributable commands should expose backend-owned view
  payloads, lifecycle mutations, and progress events rather than leaking
  `inference` implementation details into Tauri-specific branching.
- Managed-runtime pause, resume, and destructive cancel semantics must remain
  backend-owned. Tauri forwards those requests onto the backend manager and
  must not reinterpret retained-artifact state locally.

## Dependencies
**Internal:** `src-tauri/src/main.rs`, neighboring command modules, shared
gateway/runtime-registry aliases under `src-tauri/src/llm`, and backend crates
that own the actual runtime/workflow logic.

**External:** Tauri command/runtime APIs plus serde for transport payloads.

## Invariants
- `binary.rs` must remain a thin transport boundary over the backend-owned
  managed-runtime manager contract.
- `registry.rs` may aggregate debug facts for inspection, but runtime registry
  policy must remain owned by backend crates.
- Commands must not create replacement long-lived services on demand when the
  app composition root already owns a shared instance.
