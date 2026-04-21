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
| `version.rs` | Generated-component history/versioning transport using `src/generated/` as the work tree and `.pantograph/generated-components.git/` for Git metadata. |
| `vision.rs` | Vision/image prompt transport. |

## Problem
Tauri invoke commands expose runtime, model, agent, and generated-component
operations to the frontend. Without a strict adapter boundary these commands
can accumulate backend policy, duplicate runtime state, or hide local runtime
metadata inside source directories.

## Constraints
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
- Generated-component version commands must keep Git history metadata outside
  `src/` while preserving `/src/generated/` as the Vite work tree.

## Decision
Keep these files as host-facing Tauri adapters over backend services and
runtime helpers. Use focused helper modules for oversized command surfaces, and
store generated-component history metadata in `.pantograph/generated-components.git/`
instead of nesting Git state under `src/generated/`.
Generated-component history helpers accept borrowed path inputs internally so
command transport can stay focused on invoke state and payload mapping.

## Alternatives Rejected
- Move backend runtime policy into Tauri commands: rejected because backend
  crates own runtime readiness, registry policy, and workflow semantics.
- Keep generated-component Git metadata under `src/generated/`: rejected
  because source directories need tracked marker documentation and must not
  contain nested repository metadata.
- Put generated-component history into frontend stores: rejected because file
  mutation and Git operations belong in the desktop/backend layer.

## Invariants
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
- Generated-component history commands use `.pantograph/generated-components.git/`
  with `src/generated/` as the work tree.
- Command helper cleanup for strict clippy must not alter public invoke
  payloads or generated-component history semantics.

## Revisit Triggers
- A Tauri command begins deriving workflow/runtime policy that backend services
  should return.
- Generated-component history moves into an application data directory or
  backend-owned non-Git store.
- Command payloads become generated schemas shared across host bindings.

## Dependencies
**Internal:** `src-tauri/src/main.rs`, neighboring command modules, shared
gateway/runtime-registry aliases under `src-tauri/src/llm`, and backend crates
that own the actual runtime/workflow logic. Generated-component versioning also
depends on `src/generated/` marker docs and `.pantograph` runtime data rules.

**External:** Tauri command/runtime APIs, serde for transport payloads, local
filesystem APIs, and the Git CLI for generated-component history.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```rust
use crate::llm::commands::version::git_for_generated_history;
```

## API Consumer Contract
- Inputs: Tauri invoke payloads, app state handles, runtime ids, generated
  component paths, and command-specific DTOs.
- Outputs: serialized command responses, progress events, generated-component
  history views, and command-level error strings.
- Lifecycle: commands execute per invoke request and must reuse app-composed
  services for long-lived runtime state.
- Errors: backend error categories should be preserved or mapped explicitly
  rather than flattened into unrelated transport messages.
- Versioning: command payload changes require frontend service/store updates
  and tests in the same slice.

## Testing
```bash
cargo test --manifest-path src-tauri/Cargo.toml llm::commands
```

## Notes
- `version.rs` may migrate legacy `src/generated/.git/` metadata into
  `.pantograph/generated-components.git/` when generated-component writes run.
