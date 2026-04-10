# crates/pantograph-uniffi/src

## Purpose
UniFFI adapter surface for Pantograph workflow APIs.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | UniFFI exports and adapter implementation delegating to shared service contracts. |
| `runtime.rs` | Direct embedded-runtime UniFFI object for workflow/session execution without HTTP. |
| `bin/` | Binding generation helper utilities. |

## Workflow Export Modes

Default:
- Builds the Pantograph headless native shared library.
- Exports `FfiPantographRuntime`.
- `FfiPantographRuntime` wraps `pantograph-embedded-runtime` and calls the Rust workflow service directly.
- No URL/HTTP workflow exports.

`no-default-features`:
- Keeps only the graph CRUD / orchestration / Pumas adapter surface.
- Does not export workflow/session transport helpers.

`embedded-runtime` feature:
- `FfiPantographRuntime::new(config, pumas_api?)`
- `workflow_run(request_json) -> response_json`
- `workflow_get_capabilities(request_json) -> response_json`
- `workflow_get_io(request_json) -> response_json`
- `workflow_preflight(request_json) -> response_json`
- `workflow_create_session(request_json) -> response_json`
- `workflow_run_session(request_json) -> response_json`
- `workflow_close_session(request_json) -> response_json`
- `workflow_get_session_status(request_json) -> response_json`
- `workflow_list_session_queue(request_json) -> response_json`
- `workflow_cancel_session_queue_item(request_json) -> response_json`
- `workflow_reprioritize_session_queue_item(request_json) -> response_json`
- `workflow_set_session_keep_alive(request_json) -> response_json`
- `workflow_graph_save(request_json) -> response_json`
- `workflow_graph_load(request_json) -> response_json`
- `workflow_graph_list() -> response_json`
- `workflow_graph_create_edit_session(request_json) -> response_json`
- `workflow_graph_close_edit_session(request_json) -> response_json`
- `workflow_graph_get_edit_session_graph(request_json) -> response_json`
- `workflow_graph_get_undo_redo_state(request_json) -> response_json`
- `workflow_graph_update_node_data(request_json) -> response_json`
- `workflow_graph_update_node_position(request_json) -> response_json`
- `workflow_graph_add_node(request_json) -> response_json`
- `workflow_graph_remove_node(request_json) -> response_json`
- `workflow_graph_add_edge(request_json) -> response_json`
- `workflow_graph_remove_edge(request_json) -> response_json`
- `workflow_graph_undo(request_json) -> response_json`
- `workflow_graph_redo(request_json) -> response_json`
- `workflow_graph_get_connection_candidates(request_json) -> response_json`
- `workflow_graph_connect(request_json) -> response_json`
- `workflow_graph_insert_node_and_connect(request_json) -> response_json`
- `workflow_graph_preview_node_insert_on_edge(request_json) -> response_json`
- `workflow_graph_insert_node_on_edge(request_json) -> response_json`

`frontend-http` feature:
- `frontend_http_workflow_run(base_url, request_json, pumas_api?) -> response_json`
- `frontend_http_workflow_get_capabilities(base_url, request_json, pumas_api?) -> response_json`
- `frontend_http_workflow_create_session(base_url, request_json, pumas_api?) -> response_json`
- `frontend_http_workflow_run_session(base_url, request_json, pumas_api?) -> response_json`
- `frontend_http_workflow_close_session(request_json) -> response_json`

## Dependencies
- Internal: `pantograph-workflow-service`, `pantograph-embedded-runtime`, `node-engine`.
- Frontend HTTP (optional): `pantograph-frontend-http-adapter`.
- Host/runtime: optional `pumas-library`.

## Notes

- Frontend HTTP behavior is isolated in `pantograph-frontend-http-adapter`.
- Native embedding behavior is isolated in `pantograph-embedded-runtime`.
- `scripts/check-uniffi-embedded-runtime-surface.sh` verifies that the direct
  runtime object is present in the compiled UniFFI metadata.
- `scripts/check-uniffi-csharp-smoke.sh` uses `uniffi-bindgen-cs` to generate
  C# into `target/uniffi/csharp/`, compile it, and run a direct-runtime C#
  workflow/session smoke harness against the Pantograph headless native library.
- `scripts/package-uniffi-csharp-artifacts.sh` builds the release native
  library, generates C# from that library, and creates CI-ready C# and native
  runtime zip artifacts under `target/bindings-package/artifacts/`.
- `scripts/check-packaged-csharp-quickstart.sh` compiles the packaged C#
  quickstart against the packaged generated binding without restoring NuGet
  packages.
- The checked-in bindgen helper currently supports the official UniFFI 0.28
  generator set. Use the separate `uniffi-bindgen-cs` CLI for C# generation.
