# .pantograph

Repository-local Pantograph workflow and orchestration data boundary.

## Purpose
This directory stores checked-in default workflow and orchestration examples
plus ignored user-created local data. It documents the split between
source-controlled starter artifacts and runtime/user state that must not be
accidentally committed.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `workflows/` | Saved workflow JSON examples and ignored user-created workflow files. |
| `orchestrations/` | Saved orchestration JSON examples and ignored user-created orchestration files. |
| `generated-components.git/` | Ignored generated-component history Git directory used with `src/generated/` as its work tree. |

## Problem
Pantograph keeps local workflow data near the repo for developer workflows, but
the directory also contains large user-created JSON files. Without explicit
producer rules, tracked examples and ignored user data are easy to confuse.

## Constraints
- User-created workflow, orchestration, and generated-component history files
  remain ignored by default.
- Tracked examples must remain intentional and reviewable.
- JSON files must stay compatible with the workflow/orchestration loaders that
  consume them.
- Generated-component history must stay outside `src/` while preserving
  undo/redo behavior for the hot-load workspace.
- README marker files are the only broadly unignored documentation artifacts in
  ignored data directories.

## Decision
Document `.pantograph` as a mixed structured-data boundary. Keep selected
default examples tracked, keep user-created data ignored, and add nested README
files to explain the producer contracts for each data family. Store generated
component undo/redo Git metadata in the ignored
`.pantograph/generated-components.git/` directory instead of nesting Git
metadata under `src/generated/`.

## Alternatives Rejected
- Track all local workflow data: rejected because user-generated files are
  large, machine-specific, and not all suitable as examples.
- Ignore the entire directory without documentation: rejected because tracked
  examples already exist and need compatibility rules.
- Keep generated-component Git metadata under `src/generated/`: rejected
  because source directories must remain traceable by the outer repository.

## Invariants
- Default examples are reviewed as source-controlled structured artifacts.
- Local user data remains ignored unless a file is deliberately promoted as a
  default example.
- Workflow and orchestration JSON changes must be compatible with the loaders
  that read them.
- Generated-component history is local runtime state and must not be committed
  to the outer repository.

## Revisit Triggers
- Saved workflow data moves to a separate examples package.
- Runtime storage moves out of the repo-local `.pantograph` path.
- Generated-component history moves into an application data directory or
  backend-owned history store.
- A schema validator is introduced for saved workflow/orchestration JSON.

## Dependencies
**Internal:** workflow and orchestration loaders, frontend template/service
code, backend graph DTOs, and generated-component versioning commands.

**External:** JSON tooling and the Git CLI used for generated-component
history.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```bash
git ls-files .pantograph
```

## API Consumer Contract
- Inputs: saved workflow/orchestration JSON consumed by Pantograph loaders.
- Outputs: example graphs and orchestration records available to local app
  workflows.
- Lifecycle: tracked examples are versioned with the repo; ignored user files
  are local runtime/developer state.
- Errors: malformed JSON or schema drift should fail at loader boundaries, not
  be silently repaired here.
- Versioning: tracked examples must migrate with workflow/orchestration DTO
  changes.

## Structured Producer Contract
- Stable fields: workflow graph ids, node ids, edge ids, orchestration ids, and
  version fields are machine-consumed. Generated-component Git object data is
  local runtime state rather than a reviewed structured source artifact.
- Defaults: loader defaults should be documented when fields are omitted.
- Enums and labels: node type ids, port ids, backend ids, and orchestration
  labels carry runtime behavior.
- Ordering: node/edge arrays should stay deterministic for reviewable diffs.
- Compatibility: tracked examples may be opened by older local builds during
  development and should change deliberately.
- Regeneration/migration: schema-affecting changes require loader updates,
  example JSON migration, and README updates in the same slice. Generated
  component history migrations require updates to Tauri versioning commands and
  `src/generated/README.md`.

## Testing
```bash
rg -n '"nodes"|"edges"|"orchestration"' .pantograph
```

## Notes
- `.gitignore` intentionally ignores user-created data while allowing README
  marker files and explicitly tracked defaults.
- `.pantograph/generated-components.git/` is ignored runtime history metadata
  and should be recreated or migrated by Tauri versioning commands.
