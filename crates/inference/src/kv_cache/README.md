# `crates/inference/src/kv_cache`

## Responsibility

This directory owns Pantograph's backend KV-cache primitive. It defines the
persisted artifact format, the executable handle that workflows and session
memory may reference, the compatibility contract used to validate reuse, and
the storage/codec abstractions used by inference runtimes.

The directory does not own workflow scheduling, workflow-session memory, or
frontend transport behavior. Those systems may hold indirect references to KV
artifacts, but the inference KV store remains the single cache owner.

## Module Boundaries

| File | Responsibility |
| --- | --- |
| `mod.rs` | Public facade for the KV-cache subsystem. |
| `types.rs` | Backend-owned DTOs for persisted KV metadata, executable handles, compatibility fingerprints, usage modes, and truncation markers. |
| `codec.rs` | Runtime-specific codec trait for capture, restore, and truncation of opaque KV bytes. |
| `error.rs` | Typed KV-cache errors. |
| `storage.rs` | Low-level storage backends for in-memory and disk persistence. |
| `store.rs` | Store orchestration, retention, and metadata lookup over the storage backends. |

## Ownership Rules

- `KvCacheHandle` is the workflow-facing contract. It is the artifact that
  graph execution, workflow-session memory, and diagnostics may pass around.
- Compatibility is strict. Reuse must match the same model fingerprint and the
  same runtime/tokenizer fingerprint.
- Missing runtime fingerprints in legacy metadata are treated as not reusable
  through executable KV handles.
- Session memory may store indirect references to KV artifacts, but it must not
  become a second cache implementation.

## Integration Expectations

- Runtime implementations provide the actual capture, restore, and truncation
  behavior through `KvCacheCodec`.
- The inference backend and gateway layers expose backend-owned runtime
  fingerprint, model fingerprint, and live slot persistence hooks before
  `node-engine` consumes them.
- Backend-owned truncation now also routes through the inference gateway. The
  executor validates the cache against the active runtime, then delegates byte
  truncation to the backend instead of baking truncation rules into
  `node-engine`.
- The first concrete runtime adapter is llama.cpp slot persistence. It owns the
  live-runtime save/restore boundary; later workflow execution slices should
  compose through that adapter instead of duplicating HTTP slot logic. Its slot
  snapshots still do not provide a truncation codec, so truncate requests fail
  from the backend boundary with an explicit unsupported reason.
- `node-engine` owns execution-path behavior for KV save/load/truncate nodes.
- Workflow-node descriptor code may describe KV ports, but it must not become a
  parallel KV execution owner.
- `KvCacheStore::load_for_execution` and `KvCacheStore::load_handle` are the
  backend-owned executable-reuse gates. They enforce the same model/runtime
  compatibility rules used by both load-time validation and live restore.
- `KvCacheStore::prune_to_max_entries` provides explicit oldest-first bounded
  retention semantics for the real store. Workflow/session layers may request
  pruning, but they must not reimplement their own cache-eviction policy.

## Standards Notes

- Keep backend business logic in Rust backend crates.
- Grow the facade additively where possible; prefer extracting focused helper
  modules over widening already-oversized insertion points.
- When this directory changes, keep the executable contract and README aligned
  with the roadmap and the Phase 3 implementation plan.
