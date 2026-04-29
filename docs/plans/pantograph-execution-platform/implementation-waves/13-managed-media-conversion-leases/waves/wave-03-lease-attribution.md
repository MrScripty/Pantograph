# Wave 03: Lease Attribution

## Objective

Harden `inference` media dependency lease plans so they can be mapped into
per-conversion attribution records after real converter invocation is wired.

## Primary Write Set

- `crates/inference/src/managed_media_dependencies.rs`
- `crates/inference/tests/managed_media_dependencies.rs`
- `../reports/wave-03-worker-lease-attribution.md`

## Forbidden Files

- `crates/pantograph-media-conversion/**`
- `crates/pantograph-workflow-service/**`
- frontend files
- generated bindings and generated package output
- root manifests and lockfiles
- `.pantograph/**`
- `assets/**`

## Requirements

- Preserve dependency id, active version, lease id, install root, and expected
  files for each acquired conversion dependency.
- Add or validate a holder convention suitable for workflow run, node, port,
  and conversion ids.
- Prove rollback releases already-acquired leases when a later dependency in a
  multi-dependency plan fails.
- Prove release removes all dependency leases after a successful plan.
- Do not introduce a dependency on `pantograph-media-conversion`; record mapping
  needs in the report instead.

## Verification

- `cargo test -p inference --test managed_media_dependencies`
- `cargo fmt --all -- --check`

## Report

Write `../reports/wave-03-worker-lease-attribution.md` with changed files,
verification, skipped checks, deferred work, and any cross-boundary needs.

## Integration

Integrate after Wave `02` so attribution hardening can be checked against the
frozen conversion executor contract without overlapping file edits.
