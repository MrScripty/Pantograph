# Wave 06: Host Conversion Integration

## Purpose

Wire real managed media conversion into workflow artifact output handling while
preserving the host boundary established by Stage `13`.

## Scope

- Add an optional host-agnostic conversion executor injection point to
  `WorkflowService`.
- Use the injected executor when a selected artifact output format requires
  conversion instead of returning the previous `transcoding is not implemented`
  error.
- Populate descriptor and diagnostics conversion metadata from conversion
  results.
- Implement a host-owned adapter that maps managed media dependency plans,
  active-version leases, command plans, and managed executable paths into
  `pantograph-media-conversion` requests.
- Release acquired leases on success and failure.

## Worker Split

| Worker | Scope | Write Set |
| ------ | ----- | --------- |
| Workflow-service conversion hook | Optional conversion executor field, async artifact conversion handoff, fake-executor tests, and workflow-service README/report updates. | `crates/pantograph-workflow-service/**`, targeted Stage `13` report. |
| Host adapter inspection | Read-only inspection of Tauri/inference composition roots, managed dependency APIs, executable path resolution, and safe write sets for the adapter worker. | No writes. |
| Host adapter implementation | To be launched after inspection confirms executable-path ownership and injection location. | Expected `src-tauri/src/workflow/**` plus focused tests and docs, excluding workflow-service and inference internals unless a missing API requires a separate contract slice. |

## Non-Goals

- Changing ArtifactStore retention policy.
- Adding unmanaged system PATH probing.
- GUI rendering of conversion state.
- 3D conversion if no managed 3D converter dependency exists.

## Verification

- `cargo test -p pantograph-workflow-service artifact_output_conversion`
- Host adapter tests selected after inspection.
- `cargo fmt --all -- --check`
- `npm run traceability`
