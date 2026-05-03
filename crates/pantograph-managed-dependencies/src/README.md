# pantograph-managed-dependencies

## Purpose

`pantograph-managed-dependencies` owns the neutral managed dependency contract
surface for runtime sidecars, media tools, and native artifacts. It also owns
the current media redistributable catalog, install-state, activation, lease,
and removal implementation for ffmpeg/OIIO/OCIO dependencies while runtime
sidecar implementation remains in inference.

## Contents

| File | Description |
| ---- | ----------- |
| `lib.rs` | Public DTOs, IDs, categories, status states, lease records, command facts, operation scopes, and crate-private JSON shape tests. |
| `redistributables.rs` | Public media redistributable compatibility API, re-exporting catalog, state, status, install, activation, lease, and removal operations. |
| `redistributables/` | Media tool/native artifact catalog, persisted state, path, and operation implementation moved out of inference. |

## Constraints

- Runtime sidecar DTOs remain contract-only until runtime state moves.
- Media redistributable operations may mutate app-owned dependency state and
  install directories, but must not perform network downloads or process
  spawning.
- This crate must not own scheduler admission, runtime reservation, workflow
  policy, frontend projection, or converter process execution.
- Runtime sidecar owners provide runtime command facts through this contract.
- Media conversion owners provide ffmpeg/OIIO/OCIO command and native artifact
  facts through this contract.
- Consumers may display or adapt these facts, but must not infer scheduler
  selection or workflow execution policy from them.

## Dependencies

**Internal:** None.

**External:** `serde`, `serde_json`, and `uuid` from the workspace.

## Revisit Triggers

- Inference managed runtime state migrates into this crate or an implementation
  crate below this contract.
- Media conversion becomes the direct owner of ffmpeg/OIIO/OCIO executable
  resolution and no longer needs compatibility redistributable APIs.
- Native artifact activation needs platform-specific ABI validation facts that
  exceed the current status and activation projection fields.
