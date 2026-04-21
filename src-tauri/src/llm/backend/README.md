# src-tauri/src/llm/backend

Desktop LLM backend adapter module.

## Purpose
This directory owns the Tauri-side module boundary for LLM backend gateway
integration. It keeps desktop command code pointed at shared backend services
instead of scattering backend selection and process assumptions across
commands.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | LLM backend module exports and adapter wiring. |

## Problem
LLM gateway and process behavior touch runtime registry, model server, and
workflow execution paths. The desktop layer needs a clear adapter module so it
does not become the owner of runtime/backend policy.

## Constraints
- Runtime/backend selection policy belongs in backend services and registry
  crates.
- Tauri modules may hold shared handles but should not duplicate runtime state.
- Command responses must preserve backend error categories.

## Decision
Keep this module as LLM backend adapter wiring for the desktop app. Shared
runtime and inference behavior remains in workspace crates.

## Alternatives Rejected
- Put backend selection logic in every command: rejected because command-local
  policy would drift.
- Move Tauri shared handles into inference crates: rejected because app handles
  and desktop lifecycle belong in Tauri.

## Invariants
- Backend adapters consume shared runtime/gateway state.
- Runtime registry and inference crates own backend availability facts.
- Desktop command code should not infer lifecycle state from process paths.

## Revisit Triggers
- LLM backend adapter logic moves to `pantograph-embedded-runtime`.
- Runtime registry becomes the only source for LLM backend handles.
- Backend selection APIs become host-agnostic.

## Dependencies
**Internal:** LLM gateway/runtime registry modules and backend workspace crates.

**External:** Tauri app state and async runtime APIs.

## Related ADRs
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`
- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md`

## Usage Examples
```rust
use crate::llm::backend;
```

## API Consumer Contract
- Inputs: shared LLM gateway/runtime state and command-layer requests.
- Outputs: backend adapter handles and command-ready projections.
- Lifecycle: shared state is created by Tauri setup and consumed by commands.
- Errors: backend selection and runtime errors should preserve original
  categories.
- Versioning: adapter surface changes require command consumers to migrate
  together.

## Structured Producer Contract
- Stable fields: backend ids, runtime labels, and command projection fields are
  machine-consumed by frontend services.
- Defaults: default backend selection must come from backend-owned state.
- Enums and labels: backend/runtime ids carry behavior.
- Ordering: runtime/backend lists should preserve backend-provided ordering.
- Compatibility: frontend command consumers depend on response labels.
- Regeneration/migration: update commands, frontend services, and tests with
  backend projection changes.

## Testing
```bash
cargo test --manifest-path src-tauri/Cargo.toml llm
```

## Notes
- Runtime policy migration is tracked in M3 and M5.
