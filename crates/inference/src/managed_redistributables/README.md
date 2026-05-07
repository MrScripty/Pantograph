# managed_redistributables

## Purpose

This directory is now an inference compatibility adapter for
Pantograph-managed media redistributables. The implementation owner moved to
`pantograph-managed-dependencies`; inference keeps only private neutral
projection helpers for aggregate managed-dependency status.

## Contents

| File | Description |
| ---- | ----------- |
| `neutral_contracts.rs` | Adapts managed redistributable status from `pantograph-managed-dependencies` into neutral `ManagedDependencyStatus` DTOs. |

## Invariants

- Readiness is based only on app-owned managed dependency files, never host
  `PATH` or system library probing.
- Tool binaries and native library artifacts use managed redistributable terms,
  not managed runtime sidecar names.
- Network download and checksum verification are intentionally outside the
  redistributable owner until source artifacts are pinned.

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

Keep this directory as a thin compatibility adapter. Catalog entries,
expected-file validation, durable selected/default/active version state, local
staging installs, leases, and removal operations are owned by
`pantograph-managed-dependencies`.

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

**Internal:** `pantograph-managed-dependencies` redistributable APIs and
neutral managed dependency DTOs.

**External:** app-owned filesystem storage for staged/installed artifacts.

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`
- Reason: workbench Settings consumes backend-owned media dependency status and
  must not infer conversion capability from the host environment.
- Revisit trigger: Settings or output nodes probe media dependencies directly.

## Usage Examples

```rust
use inference::managed_dependency_status;
use pantograph_managed_dependencies::ManagedRedistributableId;

let status = managed_dependency_status(root, ManagedRedistributableId::Ffmpeg);
```

## API Consumer Contract

- Inputs: managed redistributable ids passed to inference-owned projection
  helpers.
- Outputs: neutral managed-dependency status facts for media tools and native
  artifacts.
- Lifecycle: install, selection, activation, removal, and lease operations are
  consumed from `pantograph-managed-dependencies`, not this inference adapter.
- Errors: unsupported platform, missing expected files, invalid ids, and active
  lease conflicts must be reported by the managed-dependency owner before
  projection.

## Structured Producer Contract

- Catalog and status DTOs are imported from `pantograph-managed-dependencies`
  internally and projected into neutral managed-dependency DTOs.
- State files are durable selected/default/active-version records owned by the
  managed-dependency crate.
- Lease records protect active conversion dependency versions from removal
  during jobs and are owned by the managed-dependency crate.
