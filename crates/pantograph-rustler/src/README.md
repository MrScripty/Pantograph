# crates/pantograph-rustler/src

## Purpose
Rustler NIF adapter surface for Pantograph workflow APIs.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | NIF implementations, resources, and adapter logic delegating to shared service contracts. |

## Workflow NIF Modes

Default (`no features`):
- No URL/HTTP workflow NIF surface.
- Headless Rust hosts should use `pantograph-workflow-service` directly.

`frontend-http` feature:
- `frontend_http_workflow_run/3`
- `frontend_http_workflow_get_capabilities/3`
- `frontend_http_workflow_create_session/3`
- `frontend_http_workflow_run_session/3`
- `frontend_http_workflow_close_session/1`

## Dependencies
- Internal: `pantograph-workflow-service`, `node-engine`.
- Frontend HTTP (optional): `pantograph-frontend-http-adapter`.
- Host/runtime: optional `pumas-library`.

## Notes

- Frontend HTTP behavior is isolated in `pantograph-frontend-http-adapter`.
