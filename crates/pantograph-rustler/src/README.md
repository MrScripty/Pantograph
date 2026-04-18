# crates/pantograph-rustler/src

## Purpose
Rustler NIF adapter surface for Pantograph workflow APIs.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | NIF entrypoints, resource wrappers, and BEAM-facing adapter composition. |
| `elixir_data_graph_executor.rs` | Focused orchestration data-graph bridge from Rustler into backend-owned workflow execution. |
| `resource_registration.rs` | Focused Rustler resource registration boundary used during NIF load. |
| `workflow_event_contract.rs` | Focused workflow-event JSON serialization helpers for the BEAM event channel. |
| `workflow_host_contract.rs` | Focused frontend-HTTP request/response and workflow-error envelope helpers for Rustler. |

## Workflow NIF Modes

Default (`no features`):
- No URL/HTTP workflow NIF surface.
- Headless Rust hosts should use `pantograph-workflow-service` directly.

`frontend-http` feature:
- `frontend_http_workflow_run/3`
- `frontend_http_workflow_get_capabilities/3`
- `frontend_http_workflow_preflight/3`
- `frontend_http_workflow_create_session/3`
- `frontend_http_workflow_run_session/3`
- `frontend_http_workflow_close_session/3`
- `frontend_http_workflow_get_session_status/1`
- `frontend_http_workflow_list_session_queue/1`
- `frontend_http_workflow_cancel_session_queue_item/1`
- `frontend_http_workflow_reprioritize_session_queue_item/1`
- `frontend_http_workflow_set_session_keep_alive/3`

## Dependencies
- Internal: `pantograph-workflow-service`, `node-engine`.
- Frontend HTTP (optional): `pantograph-frontend-http-adapter`.
- Host/runtime: optional `pumas-library`.

## Notes

- Frontend HTTP behavior is isolated in `pantograph-frontend-http-adapter`.
- The request and response JSON contracts are owned by
  `pantograph-workflow-service`; the Rustler layer only parses boundary JSON,
  delegates to the Rust service/adapter crates, and returns backend-owned
  response or error-envelope JSON back to the BEAM.
- Workflow-event JSON serialization is isolated in
  `workflow_event_contract.rs`, and frontend-HTTP request/error helpers are
  isolated in `workflow_host_contract.rs`, so the public NIF surface can stay
  facade-first while touched boundary logic remains decomposed.
- The orchestration data-graph bridge now lives in
  `elixir_data_graph_executor.rs`, isolating the Rustler-specific callback
  bridge from the NIF facade while keeping the actual async execution contract
  backend-owned.
- NIF load-time resource registration now lives in
  `resource_registration.rs`, isolating the Rustler resource macro boundary
  from the public NIF facade and removing the previous ignored-return warning
  pattern from `lib.rs`.
