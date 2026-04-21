# managed_runtime/managed_binaries

Reserved non-source boundary for managed runtime binary artifacts.

## Purpose
This directory currently contains no source files and is not referenced by the
managed-runtime implementation. It is documented to make the source-root
boundary explicit: managed runtime binaries and downloaded archives must live in
app data/runtime storage, not under `src/`.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `README.md` | Boundary marker explaining why runtime artifacts must not be stored here. |

## Problem
An empty `managed_binaries/` directory exists below a source directory. Without
documentation it is ambiguous whether the directory is a source module, a
runtime artifact cache, or stale local state.

## Constraints
- Source roots must not contain downloaded runtime archives or binaries.
- Managed runtime install state is owned by backend state under app data.
- Git should not track generated binary artifacts here.
- Build and release logic must not depend on this directory existing.

## Decision
Treat this directory as a reserved boundary marker only while M1/M3 cleanup
continues. Do not add runtime artifacts here. If no future source role is
accepted, remove the directory marker in a focused cleanup commit.

## Alternatives Rejected
- Use this path for managed runtime installs: rejected because runtime
  artifacts belong in app data and would violate source-root policy.
- Leave the directory undocumented: rejected because source-directory README
  enforcement would keep reporting it as ambiguous.

## Invariants
- No binaries, archives, extracted trees, or generated state belong here.
- The managed-runtime code must continue using configured app-data roots.
- Any future source role for this directory requires a new ownership decision.

## Revisit Triggers
- M3 lifecycle cleanup removes this reserved path.
- Managed runtime packaging introduces source-owned fixture binaries.
- Tooling starts depending on this directory as an input.

## Dependencies
**Internal:** none.

**External:** none.

## Related ADRs
- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md`

## Usage Examples
Do not write runtime artifacts here:

```text
managed runtime installs -> app data, not crates/inference/src/managed_runtime/managed_binaries
```

## API Consumer Contract
- Inputs: none.
- Outputs: none.
- Lifecycle: reserved marker only.
- Errors: any code that tries to use this path as runtime storage should be
  treated as a bug.
- Versioning: removal of this marker is a repository cleanup, not a public API
  change.

## Structured Producer Contract
- None.
- Reason: this directory must not produce or contain generated artifacts.
- Revisit trigger: source-owned binary fixtures are intentionally introduced.

## Testing
```bash
rg -n "managed_binaries" crates/inference/src
```

## Notes
- Additional issue recorded: this empty directory should be removed if no
  source-owned fixture role is accepted during managed-runtime cleanup.
