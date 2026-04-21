# src-tauri/src/hotload_sandbox

Desktop hot-load sandbox validation boundary.

## Purpose
This directory owns backend helpers for validating and sandboxing runtime
Svelte component code before it is exposed to the desktop hot-load path.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `mod.rs` | Hotload sandbox module exports. |
| `runtime_sandbox.rs` | Runtime sandbox setup and execution helpers. |
| `svelte_validator.rs` | Svelte-specific validation logic. |

## Problem
Pantograph can work with runtime-generated Svelte components, but those assets
need validation and sandbox boundaries so generated code does not bypass app
constraints.

## Constraints
- Runtime-generated component files live under `src/generated`, while history
  metadata lives under `.pantograph/generated-components.git/`.
- Svelte validation must happen before hot-loaded components are trusted by the
  UI.
- Sandbox behavior belongs in backend helpers, not ad hoc frontend checks.

## Decision
Keep hot-load sandbox validation in this Tauri module while tracking the
generated-state storage contract in source docs. The module validates generated
component assets but does not own general workflow/runtime policy.

## Alternatives Rejected
- Trust generated Svelte without backend validation: rejected because runtime
  code needs bounded checks.
- Move generated component history into this module: rejected because version
  history is owned by the generated-component command boundary.

## Invariants
- Hot-loaded components must pass validation before use.
- Generated component history metadata stays outside `src/`.
- Sandbox helpers should not mutate workflow graph truth.

## Revisit Triggers
- Generated component history moves away from the repo-local `.pantograph`
  storage path.
- Generated component validation becomes a shared service or CLI.
- Hot-load sandbox policy changes from validation-only to execution isolation.

## Dependencies
**Internal:** generated component workspace, Tauri command paths, and frontend
hotload sandbox services.

**External:** Svelte validation/tooling and filesystem APIs.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
use crate::hotload_sandbox::svelte_validator;
```

## API Consumer Contract
- Inputs: generated component source and sandbox configuration.
- Outputs: validation results and sandbox diagnostics.
- Lifecycle: validation runs per generated component/update.
- Errors: syntax, policy, and filesystem errors should stay distinguishable.
- Versioning: validation result shape changes require frontend hotload
  consumers to migrate.

## Structured Producer Contract
- Stable fields: validation result keys, diagnostics, and component identifiers
  are machine-consumed by hotload UI paths.
- Defaults: sandbox defaults must be explicit near validator/runtime owners.
- Enums and labels: validation status labels carry behavior.
- Ordering: diagnostics should preserve validator emission order.
- Compatibility: generated component state may outlive a single app run.
- Regeneration/migration: update generated-state docs, validators, UI
  consumers, and tests together when validation contracts change.

## Testing
```bash
cargo test --manifest-path src-tauri/Cargo.toml hotload_sandbox
```

## Notes
- Generated history metadata is stored outside `src/`; future storage changes
  should update this README and `src/generated/README.md` together.
