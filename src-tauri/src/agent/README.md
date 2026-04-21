# src-tauri/src/agent

Desktop assistant support modules.

## Purpose
This directory owns documentation indexing, retrieval, prompt enrichment, and
tool integration used by the backend assistant runtime in the desktop app.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Agent module exports and high-level wiring. |
| `types.rs` | Agent request/response and shared DTOs. |
| `prompt.rs` | Prompt assembly helpers. |
| `chunker.rs` | Documentation/content chunking utilities. |
| `docs.rs` | Documentation loading and source management. |
| `docs_index.rs` | Index construction and lookup support. |
| `embeddings.rs` | Embedding generation and vectorization helpers. |
| `enricher.rs` | Agent context enrichment pipeline. |
| `enricher_svelte.rs` | Svelte-specific enrichment support. |
| `rag/` | Retrieval-augmented generation storage and query helpers. |
| `tools/` | Agent filesystem/Tailwind tool implementations. |

## Problem
The assistant runtime needs app-local documentation and tools, but those helpers
must not become hidden workflow or runtime policy owners.

## Constraints
- Agent modules may read repo/app documentation and tool inputs.
- Tool side effects must be validated and bounded.
- Retrieval state must stay explicit and not replace product source truth.
- Public command surfaces should keep errors actionable.

## Decision
Keep desktop assistant support under this Tauri module while preserving clear
sub-boundaries for RAG and tools. Reusable model/runtime execution remains in
backend crates.

## Alternatives Rejected
- Mix assistant tools into workflow-node execution: rejected because assistant
  tooling has desktop-local permissions and validation needs.
- Put RAG/index state in frontend stores: rejected because indexing and storage
  are backend responsibilities.

## Invariants
- Tool writes must pass validation before modifying files.
- Retrieval inputs and generated context must remain traceable to sources.
- Agent helpers should not bypass workflow/runtime service contracts.

## Revisit Triggers
- Agent tools become workflow-executable production nodes.
- RAG storage moves to a shared backend crate.
- Assistant APIs become a supported external interface.

## Dependencies
**Internal:** Tauri app state, RAG modules, tool modules, and local docs/source
trees.

**External:** embedding/vector storage dependencies and filesystem APIs.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
use crate::agent::types::AgentRequest;
```

## API Consumer Contract
- Inputs: agent prompts, document sources, retrieval settings, and tool
  requests.
- Outputs: enriched prompts, retrieval context, tool results, and agent DTOs.
- Lifecycle: indexing/retrieval state is created by backend services and used
  during agent requests.
- Errors: validation, indexing, embedding, and tool errors must remain
  distinguishable.
- Versioning: DTO changes require command/frontend consumers to migrate
  together.

## Structured Producer Contract
- Stable fields: agent DTOs, retrieval result keys, tool result payloads, and
  source references are machine-consumed.
- Defaults: prompt/retrieval defaults must stay documented near their owners.
- Enums and labels: tool names, source labels, and result statuses carry
  behavior.
- Ordering: retrieval results preserve ranking order.
- Compatibility: tool payload changes affect assistant command consumers.
- Regeneration/migration: update tool docs, command consumers, and tests
  together when payloads change.

## Testing
```bash
cargo test --manifest-path src-tauri/Cargo.toml agent
```

## Notes
- Tool execution hardening overlaps with the M2 tool-loop/tool-executor plan.
