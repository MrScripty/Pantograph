# Wave 04 Host Report: GUI ArtifactStore Wiring

## Scope

Wire the Tauri GUI's canonical `WorkflowService` instance to a project-local
ArtifactStore so workflow submissions that produce descriptor-backed media can
use the regular scheduler/execution path without falling back to inline JSON.

## Files Changed

- `src-tauri/src/app_setup.rs`
- `docs/plans/pantograph-execution-platform/11-artifact-format-settings-and-managed-media-dependencies.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/coordination-ledger.md`

## Result

- The GUI startup path now opens `.pantograph/artifacts` with the current
  global default ArtifactStore policy.
- The shared workflow service is constructed with both the ArtifactStore and
  diagnostics ledger.
- Media outputs converted by the workflow-service execution boundary can be
  persisted as artifacts instead of failing with `artifact store is not
  configured`.

## Deferred Work

- Persisted workbench Settings remain the future canonical owner for artifact
  policy values.
- API/binding/frontend access to artifact reads, downloads, consume
  acknowledgement, and policy changes remains assigned to Wave `05`.

## Verification

- `cargo check -p pantograph`
- `cargo test -p pantograph-workflow-service artifact_output_conversion`
- `cargo fmt --all -- --check`
