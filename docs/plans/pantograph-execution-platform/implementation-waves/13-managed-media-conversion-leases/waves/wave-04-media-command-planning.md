# Wave 04: Media Command Planning

## Objective

Add media-type command planning and capability coverage for managed conversion
tools without wiring workflow execution yet.

## Primary Write Set

- `crates/pantograph-media-conversion/src/**`
- `crates/pantograph-media-conversion/README.md`
- `crates/pantograph-media-conversion/src/README.md`
- `../reports/wave-04-worker-media-command-planning.md`

## Forbidden Files

- `crates/inference/**`
- `crates/pantograph-workflow-service/**`
- frontend files
- generated bindings and generated package output
- root manifests and lockfiles
- `.pantograph/**`
- `assets/**`

## Requirements

- Add deterministic command-plan builders for image, audio, and video output
  targets using the existing conversion target metadata.
- Select the expected managed dependency id for each planned tool.
- Treat unsupported 3D conversion as a typed fail-closed planning error until a
  concrete managed 3D converter exists.
- Keep command planning free of ArtifactStore paths and host filesystem
  assumptions.
- Preserve separate argv vectors; do not create shell command strings.
- Cover defaults and explicit target fields with unit tests.

## Verification

- `cargo test -p pantograph-media-conversion`
- `cargo fmt --all -- --check`
- `npm run traceability`

## Report

Write `../reports/wave-04-worker-media-command-planning.md` with changed files,
verification, skipped checks, deferred work, and any cross-boundary needs.

## Integration

Integrate before workflow-service/Tauri wiring so host conversion code can use
typed command plans instead of ad hoc argument strings.
