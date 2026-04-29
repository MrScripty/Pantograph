# managed_redistributables

## Purpose

This directory owns Pantograph-managed media redistributables that are not
runtime sidecars. It covers tool binaries such as `ffmpeg`, `ocioconvert`, and
`oiiotool`, plus native library/artifact dependencies such as OpenColorIO.

## Contents

| File | Description |
| ---- | ----------- |
| `catalog.rs` | Static catalog entries, platform metadata, support checks, and expected-file validation. |
| `contracts.rs` | Public DTOs for managed media dependencies, status projection, state, and leases. |
| `operations.rs` | Status, install-from-staging, select/default/activate, lease, and remove operations. |
| `paths.rs` | App-owned managed-dependency paths, platform keys, expected file paths, and timestamp helpers. |
| `state.rs` | Schema-versioned durable JSON state load/save and state-entry helpers. |

## Invariants

- Readiness is based only on app-owned managed dependency files, never host
  `PATH` or system library probing.
- Tool binaries and native library artifacts use managed redistributable terms,
  not managed runtime sidecar names.
- Network download and checksum verification are intentionally outside this
  module until source artifacts are pinned.

## Problem

Pantograph needs media conversion dependencies such as `ffmpeg`,
`ocioconvert`, `oiiotool`, and OpenColorIO without treating them as inference
runtime sidecars or trusting whatever binary happens to be on the host `PATH`.
The Library/Settings surface needs status and actions that are explicit,
auditable, and product-owned.

## Constraints

- Runtime sidecars, tool binaries, and native library artifacts remain distinct
  product categories even when they share install-state helpers.
- Managed media dependency readiness must come from app-owned files and durable
  state, not ambient system discovery.
- OpenColorIO activation is represented as a native library/artifact boundary;
  unsafe ABI loading is outside the current scaffold.
- Network downloads and checksum validation require pinned source metadata
  before becoming active operations.

## Decision

Create a managed redistributables module for non-runtime media dependencies.
The module owns catalog entries, expected-file validation, durable selected and
active version state, local staging installs, and lease planning for conversion
jobs.

## Alternatives Rejected

- Model media tools as managed runtimes: rejected because conversion tools are
  dependencies of artifact conversion, not scheduler-loadable inference
  runtimes.
- Probe host `PATH` for readiness: rejected because it is not auditable and
  cannot support repeatable Library-managed installs.
- Load OpenColorIO directly in the first scaffold: rejected because native ABI
  safety needs a focused FFI boundary before dynamic loading is enabled.

## Revisit Triggers

- Pinned download URLs and checksums are added for managed media dependencies.
- OpenColorIO ABI validation or dynamic loading is implemented.
- Conversion jobs need concurrent lease accounting across multiple processes.

## Dependencies

**Internal:** inference managed dependency paths/state helpers and workflow
ArtifactStore format capability projections.

**External:** app-owned filesystem storage for staged/installed artifacts.

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`
- Reason: workbench Settings consumes backend-owned media dependency status and
  must not infer conversion capability from the host environment.
- Revisit trigger: Settings or output nodes probe media dependencies directly.

## Usage Examples

```rust
use inference::managed_redistributables::{
    managed_redistributable_status, ManagedRedistributableId,
};

let status = managed_redistributable_status(root, ManagedRedistributableId::Ffmpeg)?;
```

## API Consumer Contract

- Inputs: managed redistributable ids, local staging directories, selected
  versions, and activation/default commands.
- Outputs: catalog metadata, install/readiness state, missing expected files,
  active/default version facts, and conversion dependency leases.
- Lifecycle: callers stage files, install them into app-owned storage, select
  or activate versions, and remove inactive versions through backend commands.
- Errors: unsupported platform, missing expected files, invalid ids, and active
  lease conflicts must be reported explicitly.

## Structured Producer Contract

- Catalog and status DTOs are machine-consumed by workflow-service bindings and
  the workbench Settings page.
- State files are durable selected/default/active-version records and require
  schema-versioned migrations if their shape changes.
- Lease records protect active conversion dependency versions from removal
  during jobs.
