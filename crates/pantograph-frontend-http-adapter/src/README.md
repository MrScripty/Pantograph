# crates/pantograph-frontend-http-adapter/src

## Purpose
Explicit frontend HTTP workflow host adapter for modular GUI surfaces.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | URL-based `WorkflowHost` implementation plus shared HTTP error-envelope and output-payload parsing helpers. |

## Problem
Headless API bindings were mixing frontend HTTP transport concerns with
framework embedding concerns, which blurred integration boundaries.

## Constraints
- Keep `pantograph-workflow-service` as the contract and orchestration source of truth.
- Keep HTTP transport explicitly opt-in for frontend/modular GUI use.
- Avoid any dependency direction from service/domain layers back into frontend adapters.

## Decision
Place OpenAI-compatible HTTP host wiring in a dedicated adapter crate that
implements `WorkflowHost`.

## Alternatives Rejected
- Keep duplicated HTTP host code in each binding crate: rejected due to drift risk.
- Keep HTTP transport in core service crate: rejected due to boundary coupling.

## Invariants
- This crate is transport/infrastructure only; no workflow business rules.
- This crate is not required for headless Rust API usage.
- Non-2xx HTTP responses must be translated through backend-owned workflow
  error envelopes instead of adapter-local error taxonomies.
- Backend-owned cancellation envelopes must stay distinct from timeout
  envelopes; this adapter must not rewrite `cancelled` into
  `runtime_timeout`.

## Revisit Triggers
- A second frontend transport needs the same host semantics.
- HTTP payload contract changes require adapter-level redesign.

## Dependencies
**Internal:** `pantograph-workflow-service`  
**External:** `reqwest`, `pumas-library`

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
```rust
let host = pantograph_frontend_http_adapter::FrontendHttpWorkflowHost::with_defaults(
    "http://127.0.0.1:8081".to_string(),
    None,
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
)?;
```
