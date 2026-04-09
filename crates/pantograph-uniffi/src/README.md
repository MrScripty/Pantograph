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
- The checked-in bindgen helper currently supports the official UniFFI 0.28
  generator set. It does not currently generate C#; add an explicit C#
  generator/tooling path before claiming a generated C# package.
