# Wave 02: Conversion Executor

## Objective

Add deterministic managed process execution scaffolding to
`pantograph-media-conversion` without requiring real converter binaries.

## Primary Write Set

- `crates/pantograph-media-conversion/src/**`
- `crates/pantograph-media-conversion/Cargo.toml` only if a dependency is
  justified
- `../reports/wave-02-worker-conversion-executor.md`

## Forbidden Files

- `crates/inference/**`
- `crates/pantograph-workflow-service/**`
- frontend files
- generated bindings and generated package output
- root manifests and lockfiles
- `.pantograph/**`
- `assets/**`

## Requirements

- Add a process-runner abstraction suitable for later managed executable paths.
- Validate executable paths as host-supplied absolute executable paths, not
  shell command strings.
- Do not shell through user-supplied command text.
- Keep stderr summaries bounded.
- Expose timeout, cancellation, process failure, dependency unavailable, and
  I/O failure through typed errors.
- Cover behavior with fake process-runner tests.

## Verification

- `cargo test -p pantograph-media-conversion`
- `cargo fmt --all -- --check`

## Report

Write `../reports/wave-02-worker-conversion-executor.md` with changed files,
verification, skipped checks, deferred work, and any cross-boundary needs.

## Integration

Integrate before Wave `03` lease attribution so process execution contracts are
available for later host-owned conversion wiring.
