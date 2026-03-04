# crates/pantograph-uniffi/src

## Purpose
UniFFI adapter surface for Pantograph workflow APIs.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | UniFFI exports and adapter implementation delegating to shared service contracts. |
| `bin/` | Binding generation helper utilities. |

## Workflow Export Modes

Default (`no features`):
- No URL/HTTP workflow exports.
- Headless Rust hosts should call `pantograph-workflow-service` directly.

`frontend-http` feature:
- `frontend_http_workflow_run(base_url, request_json, pumas_api?) -> response_json`
- `frontend_http_workflow_get_capabilities(base_url, request_json, pumas_api?) -> response_json`
- `frontend_http_workflow_create_session(base_url, request_json, pumas_api?) -> response_json`
- `frontend_http_workflow_run_session(base_url, request_json, pumas_api?) -> response_json`
- `frontend_http_workflow_close_session(request_json) -> response_json`

## Dependencies
- Internal: `pantograph-workflow-service`, `node-engine`.
- Frontend HTTP (optional): `pantograph-frontend-http-adapter`.
- Host/runtime: optional `pumas-library`.

## Notes

- Frontend HTTP behavior is isolated in `pantograph-frontend-http-adapter`.
