# src/generated

Runtime-generated Svelte component workspace.

## Purpose
This directory is the Vite-visible working tree for Svelte components generated
at runtime by the agent/hot-load flow. It keeps generated component source files
under the frontend module graph while storing undo/redo Git metadata outside
`src/`.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `.gitignore` | Runtime workspace ignore rules for temporary validation files. |
| `.gitkeep` | Marker that keeps this generated workspace present in fresh checkouts. |
| `*.svelte` | Ignored runtime-authored component files loaded by the hot-load sandbox. |
| `*/` | Ignored runtime-authored component subdirectories. |

## Problem
Generated Svelte files must be importable by Vite from a stable path, but Git
history metadata inside a source directory breaks source traceability and makes
the outer repository treat runtime state like source.

## Constraints
- Vite and frontend hot-load services still import generated components from
  `/src/generated/`.
- Runtime-authored component files remain ignored by the outer repository.
- Generated component undo/redo history is stored in the ignored
  `.pantograph/generated-components.git/` Git directory.
- Backend/Tauri write commands own history mutation; frontend code only renders
  generated components after validation.

## Decision
Keep `src/generated/` as the generated component working tree and move the Git
history directory to `.pantograph/generated-components.git/`. Track this README
and marker files in the outer repository so the source boundary is documented
without committing runtime-authored components.

## Alternatives Rejected
- Track generated Svelte files in the outer repository: rejected because they
  are runtime/user state and can contain large or machine-specific content.
- Keep nested Git metadata under `src/generated/`: rejected because source
  directories must be inspectable by repository tooling without hidden nested
  repositories.
- Move generated components entirely outside `src/`: rejected for this slice
  because existing Vite dynamic imports and HMR paths depend on `/src/generated/`.

## Invariants
- Generated component files are runtime state, not hand-maintained source.
- The outer repository tracks only marker/documentation files in this directory.
- Undo/redo history commands use `.pantograph/generated-components.git/` with
  this directory as the Git work tree.
- Validation must run before generated components are rendered in the app.

## Revisit Triggers
- Vite import paths are replaced with a manifest or virtual module provider.
- Generated component history moves into a backend-owned non-Git store.
- Generated components become distributable plugin artifacts.

## Dependencies
**Internal:** Tauri generated-component write/version commands, frontend
hot-load sandbox services, Vite generated component plugin, and design-system
validation scripts.

**External:** Git CLI, Vite dynamic imports, Svelte compiler behavior, and local
filesystem APIs.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```ts
const modules = import.meta.glob('/src/generated/**/*.svelte');
```

## API Consumer Contract
- Inputs: runtime-authored `.svelte` files written by Tauri/agent commands.
- Outputs: generated component modules consumed by the hot-load sandbox.
- Lifecycle: files are created, updated, checked out, or deleted by generated
  component history commands during a developer/app session.
- Errors: filesystem, Git, validation, and module import failures must remain
  distinguishable for the hot-load UI.
- Versioning: generated component file layout changes require Vite, Tauri
  command, and sandbox service updates in the same slice.

## Structured Producer Contract
- Stable fields: relative component paths and Svelte module exports are
  machine-consumed by Vite and the hot-load registry.
- Defaults: generated files inherit sandbox defaults from frontend hot-load
  configuration.
- Enums and labels: component ids and validation status labels carry behavior.
- Ordering: component listing should remain deterministic where shown in UI.
- Compatibility: existing generated components may survive local app upgrades.
- Regeneration/migration: history metadata belongs in
  `.pantograph/generated-components.git/`; generated file migrations must update
  the history commands and this README together.

## Testing
```bash
npm run lint:full
TRACEABILITY_STAGED_ONLY=1 ./scripts/check-decision-traceability.sh
```

## Notes
- `.gitignore` keeps runtime-authored component files out of the outer
  repository while allowing this README and marker files to be tracked.
