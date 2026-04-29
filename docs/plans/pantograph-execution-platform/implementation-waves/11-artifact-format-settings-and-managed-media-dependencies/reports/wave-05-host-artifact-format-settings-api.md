# Wave 05 Host Report: Artifact Format Settings API

## Scope

Add the backend settings surface needed by the canonical workbench Settings
page and output-node format selectors without moving persistence into the
frontend.

## Files Changed

- `crates/pantograph-workflow-service/src/lib.rs`
- `crates/pantograph-workflow-service/src/workflow.rs`
- `crates/pantograph-workflow-service/src/workflow/artifact_contracts.rs`
- `crates/pantograph-workflow-service/src/workflow/artifact_settings_api.rs`
- `crates/pantograph-workflow-service/src/workflow/service_config.rs`
- `crates/pantograph-workflow-service/tests/artifact_format_settings.rs`
- `src-tauri/src/app_setup.rs`
- `src-tauri/src/workflow/commands.rs`
- `src-tauri/src/workflow/headless_workflow_commands.rs`
- `docs/plans/pantograph-execution-platform/11-artifact-format-settings-and-managed-media-dependencies.md`
- `docs/plans/pantograph-execution-platform/implementation-waves/11-artifact-format-settings-and-managed-media-dependencies/coordination-ledger.md`

## Result

- Added backend query/update DTOs for `ArtifactFormatSettings`.
- Added a workflow-service settings API that validates image, audio, video,
  and 3D defaults against backend-owned conversion capabilities.
- Persisted GUI-owned settings under `.pantograph/artifact-format-settings.json`
  through the canonical workflow service configured at app startup.
- Added Tauri commands for settings query, settings update, and format
  capability projection.
- Added focused tests for required defaults, persistence/reload, and invalid
  setting rejection.

## Deferred Work

- The workbench Settings page still needs to consume these commands.
- Output nodes still need format selectors backed by the capability projection.
- Managed redistributable status/action commands and final binding parity are
  still open.

## Verification

- `cargo test -p pantograph-workflow-service --test artifact_format_settings`
- `cargo check -p pantograph`
- Targeted `rustfmt --edition 2021` on files touched by this slice.
