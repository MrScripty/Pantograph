# .pantograph/workflows

Saved workflow JSON examples and local user workflow data.

## Purpose
This directory stores workflow graph JSON files. A small set of default examples
is tracked for review and smoke usage; user-created workflow files remain
ignored by default.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `coding-agent.json` | Tracked default coding-agent workflow example. |
| `juggernaut-x-v10-sdxl.json` | Tracked default diffusion workflow example. |
| `tiny-sd-turbo-diffusion.json` | Tracked default tiny-sd-turbo diffusion workflow example. |
| `*.json` | Ignored local user workflow data unless explicitly promoted as a default. |

## Problem
Saved workflow JSON is both product data and developer-local state. The repo
needs curated examples without accidentally collecting every local workflow
created during development.

## Constraints
- User workflow files are ignored by `.gitignore`.
- Tracked examples should use lowercase hyphenated filenames when promoted.
- Workflow graphs must match backend graph DTO and node descriptor contracts.
- Large model-specific examples should be reviewed for size and relevance
  before tracking.

## Decision
Track only intentional default workflow examples and keep this README as the
directory-level producer contract. Promote additional examples only when they
are small enough to review and represent supported runtime behavior.

## Alternatives Rejected
- Track every saved workflow: rejected because local workflow data is
  user-specific and can be very large.
- Move examples into `src/templates/workflows`: rejected for currently saved
  runtime workflow files that exercise app-local persistence paths rather than
  bundled frontend starter templates.

## Invariants
- Tracked workflow JSON must deserialize through current workflow loaders.
- Node ids, edge ids, type ids, and port ids remain semantically meaningful.
- Examples must reflect supported execution paths rather than aspirational
  graph shapes.

## Revisit Triggers
- Workflow JSON receives a formal schema.
- Default examples move to a dedicated fixtures package.
- The app no longer stores local workflows under `.pantograph/workflows`.

## Dependencies
**Internal:** workflow persistence commands, frontend workflow services, node
descriptors, and backend graph DTOs.

**External:** JSON tooling.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```bash
git ls-files .pantograph/workflows
```

## API Consumer Contract
- Inputs: workflow JSON files consumed by app persistence/loading paths.
- Outputs: saved graph definitions used as local examples or user state.
- Lifecycle: tracked examples migrate with the repo; ignored files remain local
  user/developer state.
- Errors: invalid workflow JSON should fail through workflow loading paths.
- Versioning: graph schema changes must migrate tracked examples together.

## Structured Producer Contract
- Stable fields: graph ids, nodes, edges, node type ids, port ids, and metadata
  are machine-consumed by workflow loaders.
- Defaults: omitted optional graph fields must match loader defaults.
- Enums and labels: node type ids, task types, backend ids, and port labels
  carry execution behavior.
- Ordering: node and edge arrays should remain deterministic for reviewable
  diffs.
- Compatibility: examples must stay compatible with current node descriptors
  and workflow persistence code.
- Regeneration/migration: update loaders, examples, tests, and this README
  together when workflow JSON shape changes.

## Testing
```bash
rg -n '"nodes"|"edges"' .pantograph/workflows
```

## Notes
- `.gitignore` allows this README and explicitly tracked defaults while keeping
  ordinary user workflow files ignored.
