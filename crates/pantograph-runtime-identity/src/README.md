# crates/pantograph-runtime-identity/src

## Purpose
This directory contains Pantograph's shared runtime and backend identity
normalization helpers. The boundary exists so workflow service, embedded
runtime, runtime registry, inference, and Tauri adapters can all reuse one
canonical mapping for runtime ids, backend keys, display labels, and alias
sets instead of maintaining divergent host-local lookup tables.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Defines the canonical runtime-id, engine/backend-key, display-name, and alias helpers consumed by runtime capability, diagnostics, and registry paths. |

## Problem
Pantograph has several layers that need to talk about the same runtime using
slightly different identifiers. Gateway lifecycle paths, workflow capability
payloads, runtime-registry observations, technical-fit candidates, and
frontend-facing diagnostics all need to agree on what `llama.cpp`,
`llama_cpp`, `llamacpp`, `pytorch`, `torch`, and dedicated embedding runtimes
mean. Without a shared identity boundary, Phase 7 runtime-adapter unification
would keep reintroducing drift between backend-owned facts and adapter-local
projection code.

## Constraints
- The helpers in this directory must stay pure and dependency-light so they can
  be reused by multiple workspace crates.
- Unknown runtime ids must be preserved rather than rejected so additive runtime
  support does not require a breaking normalization change first.
- Canonicalization must remain compatible with existing workflow capability,
  runtime-registry, and diagnostics contracts that already depend on stable
  runtime ids and backend-key aliases.
- This directory may normalize identity semantics, but it must not grow runtime
  lifecycle policy, health interpretation, or registry ownership logic.

## Decision
Keep runtime identity normalization in a dedicated shared crate with a single
`lib.rs` entrypoint. The crate owns canonical runtime ids, engine/backend-key
normalization, runtime display labels, and alias-set construction. Producer
mapping, runtime capability publication, diagnostics projection, and registry
reconciliation consume these helpers instead of restating alias logic in their
own modules.

## Alternatives Rejected
- Duplicate alias tables in each runtime-related crate.
  Rejected because Pantograph already has multiple consumers, and drift between
  workflow, registry, and Tauri layers would undermine Phase 7 convergence.
- Move identity helpers into `src-tauri`.
  Rejected because identity normalization is backend-owned shared logic, not a
  desktop-only transport concern.

## Invariants
- Canonical runtime ids and backend keys are computed here before runtime facts
  cross capability, diagnostics, or registry boundaries.
- Unknown runtime ids remain pass-through values after trimming instead of being
  collapsed into unrelated known ids.
- Alias helpers may add additive synonyms, but they must not silently change the
  meaning of an established canonical runtime id.
- This directory remains pure normalization logic and does not become the owner
  of runtime status, health, reconnect, or retention policy.

## Revisit Triggers
- A new runtime family needs richer structured identity metadata than the
  current helper functions can represent safely.
- Runtime identity must become part of a generated cross-language contract
  rather than a Rust-only helper boundary.
- Multiple files or responsibilities appear here and `lib.rs` stops being a
  clear single-responsibility module.

## Dependencies
**Internal:** None. This directory is intended to stay reusable by runtime,
workflow, and adapter crates without importing their policy modules.

**External:** Rust standard library collections and string handling only.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- Reason: runtime-registry convergence depends on one backend-owned identity
  contract so observations, capability payloads, and diagnostics do not drift
  across crates.
- Revisit trigger: a future ADR changes where canonical runtime identity is
  owned or requires additional machine-consumable identity metadata.

## Usage Examples
```rust
use pantograph_runtime_identity::{
    canonical_runtime_backend_key, canonical_runtime_id, runtime_backend_key_aliases,
    runtime_display_name,
};

let runtime_id = canonical_runtime_id("llama.cpp");
assert_eq!(runtime_id, "llama_cpp");

let backend_key = canonical_runtime_backend_key("PyTorch");
assert_eq!(backend_key, "pytorch");

let aliases = runtime_backend_key_aliases(
    runtime_display_name("pytorch").unwrap(),
    "pytorch",
);
assert!(aliases.contains(&"torch".to_string()));
```

## API Consumer Contract
- Callers may pass user-facing names, backend keys, or previously stored runtime
  ids into the canonicalization helpers.
- `canonical_runtime_id` trims input, normalizes known aliases, and preserves
  unknown non-empty ids as caller-visible values.
- `canonical_runtime_backend_key` and `canonical_engine_backend_key` normalize
  the known backend/engine alias families used by workflow and runtime layers.
- `runtime_display_name` returns a stable display label only for known runtime
  ids; callers must handle `None` for additive or unknown runtimes.
- Compatibility policy: new aliases may be added, but changing the canonical
  output for an already-supported runtime requires coordinated contract review.

## Structured Producer Contract
- The canonical runtime-id producer contract currently covers known values such
  as `llama_cpp`, `llama.cpp.embedding`, `pytorch`, `onnx-runtime`,
  `stable_audio`, `diffusers`, `ollama`, and `candle`.
- Backend-key alias sets are additive and are intended for capability matching,
  diagnostics lookup, and registry reconciliation rather than as a transport for
  lifecycle state.
- `runtime_display_name` is a presentation helper layered over canonical ids; it
  must not become a substitute source of truth for runtime identity.
- If this directory adds a new canonical runtime family, all downstream
  capability, registry, and diagnostics consumers must update in the same
  change set so the identity contract stays aligned.
