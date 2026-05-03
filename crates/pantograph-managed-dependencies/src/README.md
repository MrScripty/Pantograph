# pantograph-managed-dependencies

## Purpose

`pantograph-managed-dependencies` owns the neutral managed dependency contract
surface for runtime sidecars, media tools, and native artifacts. It exists so
inference, media conversion, UniFFI, embedded runtime, and workflow adapters can
share status, command, lease, and activation facts without importing inference
implementation modules.

## Contents

| File | Description |
| ---- | ----------- |
| `lib.rs` | Public DTOs, IDs, categories, status states, lease records, command facts, operation scopes, and crate-private JSON shape tests. |

## Constraints

- This crate is a contracts boundary. It must not perform installs, filesystem
  mutation, downloads, process spawning, scheduler admission, runtime
  reservation, workflow policy, or frontend projection.
- Runtime sidecar owners provide runtime command facts through this contract.
- Media conversion owners provide ffmpeg/OIIO/OCIO command and native artifact
  facts through this contract.
- Consumers may display or adapt these facts, but must not infer scheduler
  selection or workflow execution policy from them.

## Dependencies

**Internal:** None.

**External:** `serde` from the workspace.

## Revisit Triggers

- Inference managed runtime state migrates into this crate or an implementation
  crate below this contract.
- Media conversion becomes the direct owner of ffmpeg/OIIO/OCIO executable
  resolution.
- Native artifact activation needs platform-specific ABI validation facts that
  exceed the current status and activation projection fields.
