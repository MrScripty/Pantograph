# pantograph-frontend-http-adapter

Optional HTTP workflow host adapter for Pantograph frontend-modular surfaces.

## Purpose
This crate implements the workflow host contract over HTTP while keeping
transport behavior outside `pantograph-workflow-service`. The boundary exists
so frontend HTTP integration is explicit, optional, and unable to become the
owner of workflow business rules.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Crate manifest for the optional HTTP adapter. |
| `src/` | Adapter implementation and source-level README. |

## Problem
Some modular GUI or frontend surfaces need to call a workflow host through
OpenAI-compatible HTTP-style transport. Without a dedicated adapter crate, that
transport code would be duplicated in binding crates or mixed into the core
service layer.

## Constraints
- Keep workflow contracts and error semantics owned by
  `pantograph-workflow-service`.
- Keep this adapter optional.
- Preserve backend-owned error envelopes without translating cancellation or
  invalid-request semantics incorrectly.
- Avoid app-specific policy.

## Decision
Expose a narrow `WorkflowHost` implementation backed by `reqwest` and shared
workflow-service DTOs. Binding crates may depend on this adapter only when
their feature surface explicitly enables frontend HTTP behavior.

## Alternatives Rejected
- Keep HTTP host code in each binding crate: rejected because response and
  error-envelope handling would drift.
- Put HTTP transport in workflow service: rejected because service contracts
  should be framework and transport agnostic.

## Invariants
- This crate is transport/infrastructure only.
- Non-2xx responses are mapped through backend-owned workflow errors.
- `cancelled`, `invalid_request`, and runtime timeout envelopes remain
  semantically distinct.
- The adapter does not own runtime selection, graph mutation, or scheduler
  policy.

## Revisit Triggers
- A second HTTP transport variant needs shared behavior.
- Workflow-service error envelopes change shape.
- Frontend HTTP becomes deprecated in favor of direct embedded runtime calls.

## Dependencies
**Internal:** `pantograph-workflow-service`, `pantograph-runtime-identity`, and
optional `pumas-library`.

**External:** `async-trait`, `serde_json`, `reqwest`, and `thiserror`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
let pumas_api = maybe_pumas_api();
let host = pantograph_frontend_http_adapter::FrontendHttpWorkflowHost::with_defaults(
    "http://127.0.0.1:8081".to_string(),
    pumas_api,
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
)?;
```

## API Consumer Contract
- Inputs: base URL, workflow request DTOs/JSON, and optional Pumas API handles.
- Outputs: workflow-service response DTOs/JSON and typed workflow-service
  errors.
- Lifecycle: callers construct the host and use it for request forwarding; this
  crate does not own server startup or shutdown.
- Errors: transport failures and backend error envelopes are mapped to service
  errors while preserving backend categories.
- Versioning: adapter behavior follows workflow-service contract versions.

## Structured Producer Contract
- None.
- Reason: this crate consumes and forwards structured workflow payloads but does
  not publish independent schemas or manifests.
- Revisit trigger: the adapter starts generating OpenAPI, JSON Schema, or
  saved transport manifests.

## Testing
```bash
cargo test -p pantograph-frontend-http-adapter
```

## Notes
- Keep this crate out of default binding surfaces unless frontend HTTP support
  is intentionally exposed.
