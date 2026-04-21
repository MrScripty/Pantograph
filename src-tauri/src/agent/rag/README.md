# src-tauri/src/agent/rag

Retrieval-augmented generation storage boundary for the desktop assistant.

## Purpose
This directory owns agent RAG storage, indexing, query DTOs, and error mapping
for desktop assistant context retrieval.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | RAG module exports. |
| `types.rs` | Retrieval request/response and indexed document DTOs. |
| `error.rs` | RAG-specific error types and conversions. |
| `lancedb.rs` | LanceDB/vector storage integration. |
| `manager.rs` | RAG manager orchestration for indexing and retrieval. |

## Problem
Assistant context retrieval needs durable/vector-backed indexing that is
separate from prompt assembly and file tools. Without a dedicated boundary,
storage errors and ranking semantics can blur into generic agent logic.

## Constraints
- Indexed content must retain source traceability.
- Retrieval ranking order is semantically meaningful.
- Storage failures need clear error categories.
- RAG state must not become canonical product documentation.

## Decision
Keep RAG storage and retrieval orchestration here behind typed manager and DTO
contracts. Agent prompt enrichment consumes this boundary rather than accessing
storage directly.

## Alternatives Rejected
- Store RAG state in frontend memory: rejected because retrieval should be
  backend-owned and durable where configured.
- Let prompt assembly query storage directly: rejected because ranking and
  storage errors need a focused boundary.

## Invariants
- Retrieval results preserve rank order and source references.
- Index updates must not silently discard source metadata.
- Storage-specific errors are converted into RAG error categories.

## Revisit Triggers
- RAG moves to a shared backend crate.
- Retrieval schemas become shared with workflow nodes.
- A remote/vector service replaces local storage.

## Dependencies
**Internal:** agent types, embeddings helpers, and local document sources.

**External:** LanceDB/vector storage and filesystem APIs.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
use crate::agent::rag::RagManager;
```

## API Consumer Contract
- Inputs: documents/chunks, embedding vectors, query strings, and retrieval
  limits.
- Outputs: ranked retrieval results with source metadata.
- Lifecycle: managers own index/update/query operations for a configured store.
- Errors: storage, embedding, and validation failures remain distinct.
- Versioning: retrieval DTO changes require agent consumers and tests to
  migrate together.

## Structured Producer Contract
- Stable fields: document ids, chunk ids, scores, source paths, and text spans
  are machine-consumed.
- Defaults: ranking and limit defaults should be explicit.
- Enums and labels: source kinds and error labels carry behavior.
- Ordering: result order is rank order.
- Compatibility: persisted index records may need migration when DTOs change.
- Regeneration/migration: update index builders, query consumers, and tests
  together when record shapes change.

## Testing
```bash
cargo test --manifest-path src-tauri/Cargo.toml rag
```

## Notes
- Keep source attribution with retrieved context.
