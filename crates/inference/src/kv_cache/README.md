# `crates/inference/src/kv_cache`

## Purpose

This directory owns Pantograph's backend KV-cache primitive. It defines the
persisted artifact format, the executable handle that workflows and session
memory may reference, the compatibility contract used to validate reuse, and
the storage/codec abstractions used by inference runtimes.

The directory does not own workflow scheduling, workflow-session memory, or
frontend transport behavior. Those systems may hold indirect references to KV
artifacts, but the inference KV store remains the single cache owner.

## Contents

| File | Responsibility |
| --- | --- |
| `mod.rs` | Public facade for the KV-cache subsystem. |
| `types.rs` | Backend-owned DTOs for persisted KV metadata, executable handles, compatibility fingerprints, usage modes, and truncation markers. |
| `codec.rs` | Runtime-specific codec trait for capture, restore, and truncation of opaque KV bytes. |
| `error.rs` | Typed KV-cache errors. |
| `storage.rs` | Low-level storage backends for in-memory and disk persistence. |
| `store.rs` | Store orchestration, retention, and metadata lookup over the storage backends. |
| `store_tests.rs` | Crate-local KV-store regression coverage for compatibility, marker, truncation, and retention behavior. |

## Problem

Pantograph needs one backend-owned KV-cache boundary that can persist, validate,
truncate, and reuse opaque runtime artifacts without letting workflow/session
layers grow parallel cache implementations.

## Constraints

- KV artifacts are opaque runtime bytes and must be manipulated only through
  backend codecs.
- Executable reuse must enforce strict model and runtime compatibility.
- Legacy metadata without runtime fingerprints must remain readable while being
  rejected for executable reuse.
- Workflow and session layers may hold references to KV artifacts, but they
  must not own cache retention or compatibility policy.

## Decision

Keep the KV-cache subsystem in this focused directory. `store.rs` owns
production storage orchestration and compatibility checks, while
`store_tests.rs` owns crate-local regression coverage for marker management,
truncation, runtime compatibility, and retention.

## Alternatives Rejected

- Reimplementing KV artifact storage in workflow/session layers.
  Rejected because executable compatibility and retention policy must stay
  backend-owned.
- Letting runtimes bypass `KvCacheStore` for executable handles.
  Rejected because reusable artifact validation must stay centralized.

## Invariants

- `KvCacheHandle` is the workflow-facing contract. It is the artifact that
  graph execution, workflow-session memory, and diagnostics may pass around.
- Compatibility is strict. Reuse must match the same model fingerprint and the
  same runtime/tokenizer fingerprint.
- Missing runtime fingerprints in legacy metadata are treated as not reusable
  through executable KV handles.
- Session memory may store indirect references to KV artifacts, but it must not
  become a second cache implementation.
- `KvCacheStore::load_for_execution` and `KvCacheStore::load_handle` are the
  backend-owned executable-reuse gates. They enforce the same model/runtime
  compatibility rules used by both load-time validation and live restore.
- `KvCacheStore::prune_to_max_entries` provides explicit oldest-first bounded
  retention semantics for the real store. Workflow/session layers may request
  pruning, but they must not reimplement their own cache-eviction policy.
- KV-store behavior tests stay in `store_tests.rs` so `store.rs` remains
  focused on production storage orchestration and compatibility checks.

## Revisit Triggers

- A second persisted KV artifact family needs materially different metadata or
  compatibility rules.
- Runtime-owned truncation/capture behavior becomes rich enough to justify
  another focused helper module beneath `store.rs`.
- Workflow/session consumers need durable retention semantics that cannot be
  expressed through oldest-first pruning.

## Dependencies

**Internal:** `codec.rs`, `error.rs`, `storage.rs`, and `types.rs` in this
directory, plus backend runtime adapters that provide `KvCacheCodec`
implementations.

**External:** async runtime support and filesystem access used by the storage
backends.

## Related ADRs

- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Reason: KV-cache reuse remains a backend execution concern below
  application-level runtime policy.

## Usage Examples

```rust
use inference::kv_cache::KvCacheStore;
```

## API Consumer Contract

- Runtime implementations provide the actual capture, restore, and truncation
  behavior through `KvCacheCodec`.
- The inference backend and gateway layers expose backend-owned runtime
  fingerprint, model fingerprint, and live slot persistence hooks before
  `node-engine` consumes them.
- Backend-owned truncation routes through the inference gateway. The executor
  validates the cache against the active runtime, then delegates byte
  truncation to the backend instead of baking truncation rules into
  `node-engine`.
- Runtime-requirement and diagnostics consumers should refer to the canonical
  `kv_cache` extension name when describing this capability.

## Structured Producer Contract

- The first concrete runtime adapter is llama.cpp slot persistence. It owns the
  live-runtime save/restore boundary; later workflow execution slices should
  compose through that adapter instead of duplicating HTTP slot logic. Its slot
  snapshots still do not provide a truncation codec, so truncate requests fail
  from the backend boundary with an explicit unsupported reason.
- PyTorch now exposes backend-owned KV runtime/model identity and worker
  snapshot primitives for `dllm`-style live caches. `pytorch-inference` now
  uses those hooks to restore compatible `kv_cache_in` artifacts and capture
  fresh `kv_cache_out` artifacts through the shared KV store contract. Broader
  workflow-session and partial-rerun reuse still belongs to later roadmap
  slices.
- Structured KV execution diagnostics are emitted from backend execution paths
  and flow through workflow trace plus diagnostics transport as additive facts.
  This directory still owns only artifact/compatibility contracts; it does not
  own adapter-local formatting or reuse-policy decisions.
- `node-engine` owns execution-path behavior for KV save/load/truncate nodes.
- Workflow-node descriptor code may describe KV ports, but it must not become a
  parallel KV execution owner.
