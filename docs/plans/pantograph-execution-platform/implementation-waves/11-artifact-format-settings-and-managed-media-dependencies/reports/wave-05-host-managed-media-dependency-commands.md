# Wave 05 Host Managed Media Dependency Commands

## Scope

Added the Tauri/headless command surface for managed media dependencies owned by
the `inference` managed redistributables boundary. This slice exposes status and
state-transition operations for `ffmpeg`, `ocioconvert`, `oiiotool`, and
OpenColorIO without adding frontend controls or changing shared DTO contracts.

## Changed Files

- `src-tauri/src/workflow/headless_workflow_commands.rs`
- `src-tauri/src/workflow/commands.rs`
- `src-tauri/src/workflow/headless_workflow_commands_tests.rs`
- `src-tauri/src/workflow/headless_workflow_commands_tests/managed_media_dependencies.rs`
- `src-tauri/src/app_setup.rs`

## Implemented Commands

- `workflow_list_managed_media_dependencies`
- `workflow_managed_media_dependency_status`
- `workflow_install_managed_media_dependency_from_staging`
- `workflow_select_managed_media_dependency_version`
- `workflow_set_default_managed_media_dependency_version`
- `workflow_activate_managed_media_dependency_version`
- `workflow_remove_managed_media_dependency_version`

## Verification

- `cargo test -p pantograph managed_media_dependency_helpers_project_status_and_actions`
- `cargo check -p pantograph`
- `rustfmt --edition 2021 --check src-tauri/src/workflow/commands.rs src-tauri/src/workflow/headless_workflow_commands.rs src-tauri/src/workflow/headless_workflow_commands_tests.rs src-tauri/src/workflow/headless_workflow_commands_tests/managed_media_dependencies.rs src-tauri/src/app_setup.rs`

## Notes

The test uses a temporary app-data root and staged expected files so the command
helpers exercise the same managed redistributable install/select/default/
activate/remove path used by the GUI commands.
