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
- `node-engine` owns execution-path behavior for KV save/load/truncate nodes.
- Workflow-node descriptor code may describe KV ports, but it must not become a
  parallel KV execution owner.

## Standards Notes

- Keep backend business logic in Rust backend crates.
- Grow the facade additively where possible; prefer extracting focused helper
  modules over widening already-oversized insertion points.
- When this directory changes, keep the executable contract and README aligned
  with the roadmap and the Phase 3 implementation plan.
