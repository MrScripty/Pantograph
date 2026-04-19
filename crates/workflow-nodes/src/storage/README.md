# `crates/workflow-nodes/src/storage`

## Responsibility

This directory defines storage-oriented workflow-node descriptors and their
local task wrappers. It owns node metadata such as names, ports, categories,
and lightweight task wiring for file nodes plus descriptor-only registration
for KV-cache nodes.

It does not own the authoritative backend execution path for KV-cache behavior.
The real KV save/load/truncate execution logic is owned by backend executor
code in `node-engine`, and the KV task bodies in this directory now fail fast
unless they are run through `CoreTaskExecutor`.

## Module Boundaries

| File | Responsibility |
| --- | --- |
| `mod.rs` | Public facade for storage-node descriptors. |
| `read_file.rs` | File-read node descriptor and local task wrapper. |
| `write_file.rs` | File-write node descriptor and local task wrapper. |
| `kv_cache_save.rs` | KV-save node descriptor that directs execution to `CoreTaskExecutor`. |
| `kv_cache_load.rs` | KV-load node descriptor that directs execution to `CoreTaskExecutor`. |
| `kv_cache_truncate.rs` | KV-truncate node descriptor that directs execution to `CoreTaskExecutor`. |

## Ownership Rules

- Keep storage-node descriptors aligned with backend-owned contracts exported by
  the execution/runtime crates.
- Do not introduce backend KV business logic here. If KV behavior grows, move
  or reuse backend executor helpers instead of duplicating runtime ownership in
  this crate.
- KV task wrappers must remain descriptor-only. Direct execution should fail
  loudly and direct callers to `CoreTaskExecutor`.

## Standards Notes

- Port contracts should stay explicit and typed so future workflow graph
  validation can distinguish KV artifacts from generic JSON.
- Frontend and Tauri layers should consume backend-declared contracts rather
  than re-deriving storage semantics locally.
- Update this README whenever a storage-node boundary changes.
