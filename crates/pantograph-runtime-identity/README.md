# pantograph-runtime-identity

Shared runtime identity normalization crate for Pantograph backend surfaces.

## Purpose
This crate owns pure runtime-id, backend-key, display-label, and alias
normalization helpers used by inference, embedded runtime, workflow service,
runtime registry, and adapters. The crate boundary exists so runtime identity
semantics are not duplicated across backend producers or transport layers.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Minimal crate manifest for the dependency-light identity helper package. |
| `src/` | Rust source and source-level README for canonical identity helper behavior. |

## Problem
Pantograph has several runtime producers and consumers that refer to the same
runtime with different spellings. Without one shared normalization boundary,
diagnostics, runtime registry state, technical-fit selection, and workflow
capability payloads can disagree about which backend is being described.

## Constraints
- Keep the crate dependency-free so every runtime-facing crate can use it.
- Preserve unknown runtime ids instead of collapsing them into known families.
- Add aliases compatibly; do not silently redefine established canonical ids.
- Do not add runtime lifecycle, health, retention, or scheduler policy here.

## Decision
Keep runtime identity as a small Rust library crate with a curated public API
from `src/lib.rs`. Other crates import these helpers rather than maintaining
their own alias tables or display-name maps.

## Alternatives Rejected
- Duplicate identity maps in each runtime producer: rejected because drift has
  direct user-facing impact in diagnostics and runtime selection.
- Move identity helpers into `src-tauri`: rejected because runtime identity is
  backend-owned shared logic, not desktop transport behavior.

## Invariants
- The crate remains pure normalization logic.
- Canonical ids and aliases are stable once consumed by workflow, registry, or
  diagnostics contracts.
- Unknown ids remain representable.
- New runtime families update this crate before adapters project display labels
  or backend keys.

## Revisit Triggers
- Runtime identity needs structured metadata beyond helper functions.
- Identity contracts become generated cross-language schemas.
- A consumer needs runtime policy data that should instead live in a runtime
  state or registry crate.

## Dependencies
**Internal:** None.

**External:** Rust standard library only.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```rust
let canonical = pantograph_runtime_identity::canonical_runtime_id("llamacpp");
```

## API Consumer Contract
- Inputs: runtime ids, backend keys, and display labels supplied by backend
  producers or adapters.
- Outputs: canonical strings and alias sets used by workflow, diagnostics, and
  registry consumers.
- Lifecycle: all functions are synchronous and side-effect free.
- Errors: unknown ids are preserved rather than returned as errors.
- Versioning: alias additions are compatible; changing an existing canonical id
  is a contract change.

## Structured Producer Contract
- None.
- Reason: this crate does not publish generated schemas, manifests, or saved
  artifacts.
- Revisit trigger: runtime identity metadata becomes machine-generated or
  exported as a schema for non-Rust consumers.

## Testing
Run the crate tests from the workspace root:

```bash
cargo test -p pantograph-runtime-identity
```

## Notes
- Keep this crate small. If behavior starts depending on live runtime state,
  it belongs in `pantograph-runtime-registry` or an application runtime crate.
