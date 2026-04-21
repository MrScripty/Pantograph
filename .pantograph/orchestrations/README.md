# .pantograph/orchestrations

Saved orchestration JSON examples and local user orchestration data.

## Purpose
This directory stores orchestration JSON files that pair workflow graphs with
execution/orchestration metadata. Tracked examples provide reviewable defaults;
ordinary local orchestration files remain ignored by default.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `juggernaut-x-v10-sdxl-orch.json` | Tracked diffusion orchestration example. |
| `tiny-sd-turbo-diffusion-orch.json` | Tracked tiny-sd-turbo orchestration example. |
| `*.json` | Ignored local user orchestration data unless explicitly promoted. |

## Problem
Orchestration records are structured runtime data and can drift from workflow
graph examples. The repo needs a documented path for curated examples without
turning local user state into source.

## Constraints
- User orchestration files are ignored by `.gitignore`.
- Tracked examples must correspond to supported workflow examples or documented
  runtime paths.
- Orchestration ids and graph references must remain deterministic enough for
  review and test use.
- Large local orchestration data should not be promoted without a clear fixture
  role.

## Decision
Track only intentional orchestration examples and document their producer
contract here. Keep local user data ignored and migrate tracked examples with
workflow/orchestration DTO changes.

## Alternatives Rejected
- Track every orchestration record: rejected because local runtime state is
  user-specific.
- Keep orchestration examples undocumented: rejected because they are
  machine-consumed structured data.

## Invariants
- Tracked orchestration JSON must deserialize through current orchestration
  loaders.
- Example orchestration records must reference supported workflow/runtime
  behavior.
- JSON ordering should stay stable enough for reviewable diffs.

## Revisit Triggers
- Orchestration JSON receives a formal schema.
- Examples move to a dedicated fixtures package.
- Runtime orchestration storage moves out of `.pantograph/orchestrations`.

## Dependencies
**Internal:** orchestration loaders, workflow persistence, and backend graph DTOs.

**External:** JSON tooling.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```bash
git ls-files .pantograph/orchestrations
```

## API Consumer Contract
- Inputs: orchestration JSON files consumed by orchestration loading paths.
- Outputs: saved orchestration records used as examples or local state.
- Lifecycle: tracked examples migrate with the repo; ignored files remain local
  user/developer state.
- Errors: invalid orchestration JSON should fail through orchestration loading
  paths.
- Versioning: orchestration schema changes must migrate tracked examples
  together.

## Structured Producer Contract
- Stable fields: orchestration ids, graph references, node/task ids, and runtime
  metadata are machine-consumed.
- Defaults: omitted optional fields must match loader defaults.
- Enums and labels: orchestration state labels, node/task ids, and backend ids
  carry behavior.
- Ordering: arrays should remain deterministic for reviewable diffs.
- Compatibility: examples must stay compatible with current orchestration and
  workflow loaders.
- Regeneration/migration: update loaders, examples, tests, and this README
  together when orchestration JSON shape changes.

## Testing
```bash
rg -n '"orchestration"|"nodes"|"edges"' .pantograph/orchestrations
```

## Notes
- `.gitignore` allows this README and explicitly tracked defaults while keeping
  ordinary user orchestration files ignored.
