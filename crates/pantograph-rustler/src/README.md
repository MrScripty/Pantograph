# crates/pantograph-rustler/src

Rustler NIF source boundary for Pantograph workflow APIs.

## Purpose
This directory owns the BEAM-facing Rustler implementation for selected
Pantograph workflow and orchestration APIs. It keeps NIF entrypoints, resource
wrappers, scheduling annotations, callback bridges, and BEAM-safe error mapping
outside core workflow crates.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | Public NIF facade, resource wrappers, exported entrypoints, and module load wiring. |
| `elixir_data_graph_executor.rs` | Rustler-specific orchestration data-graph bridge into backend workflow execution. |
| `resource_registration.rs` | NIF load-time Rustler resource registration boundary. |
| `type_parsing_contract.rs` | String-to-enum parsing helpers behind public type-parsing NIFs. |
| `workflow_event_contract.rs` | Workflow-event JSON serialization helpers for the BEAM event channel. |
| `workflow_graph_contract.rs` | Workflow graph JSON CRUD and validation helpers behind public graph NIFs. |
| `workflow_host_contract.rs` | Frontend-HTTP request/response and workflow-error envelope helpers. |

## Problem
BEAM consumers need native Pantograph workflow behavior without embedding Rust
implementation details or reimplementing workflow policy in Elixir. Rustler
adds resource and scheduler constraints that should not leak into core crates.

## Constraints
- Core workflow crates must not depend on Rustler.
- Long-running work must use appropriate dirty schedulers or owned async
  resources.
- Frontend HTTP behavior is optional and isolated behind a feature.
- Request/response JSON contracts are owned by `pantograph-workflow-service`.
- BEAM-facing resource and callback behavior must remain explicit.

## Decision
Keep the Rustler source surface as a thin binding facade. It delegates workflow
behavior to backend crates and owns only BEAM conversion, NIF resource
registration, callback transport, and feature-gated adapter calls.

## Alternatives Rejected
- Put Rustler macros in core workflow crates: rejected because BEAM runtime
  details would leak into reusable Rust APIs.
- Mirror every internal Rust function as a NIF: rejected because binding
  surfaces should be curated public contracts.
- Implement workflow policy in Elixir callbacks: rejected because backend Rust
  owns workflow semantics and runtime truth.

## Invariants
- Exported NIF names remain stable unless BEAM consumers migrate together.
- Workflow JSON responses and error envelopes preserve backend-owned service
  semantics.
- Resource registration stays centralized in `resource_registration.rs`.
- Callback/event JSON serialization preserves backend event labels and order.
- Frontend HTTP NIFs are unavailable unless the `frontend-http` feature is
  enabled.

## Revisit Triggers
- Rustler resource macro behavior changes and removes current warning pressure.
- BEAM runtime supervision requires a dedicated lifecycle manager.
- Public NIF support tiers or exported function names change.
- Direct embedded runtime support is added to Rustler.

## Dependencies
**Internal:** `node-engine`, `pantograph-workflow-service`,
`pantograph-frontend-http-adapter`, `inference`, and `workflow-nodes`.

**External:** `rustler`, `tokio`, `async-trait`, `graph-flow`, `serde`,
`serde_json`, `log`, and `pumas-library`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
The Rust crate is loaded by the BEAM module configured in `rustler::init!`:

```elixir
Pantograph.Native.workflow_validate(graph_json)
```

## API Consumer Contract
- Inputs: BEAM terms, JSON strings, resource handles, callback PIDs, and
  feature-gated frontend HTTP parameters.
- Outputs: BEAM-safe terms, JSON response strings, resource handles, and
  callback messages.
- Lifecycle: BEAM code loads the NIF, creates resources, invokes exported
  functions, and drops resources when no longer referenced.
- Errors: Rust errors map to `rustler::Error::Term` values or structured JSON
  envelopes.
- Versioning: exported NIF names, atoms, resource shapes, and JSON payload
  semantics are public binding contracts for BEAM consumers.

## Structured Producer Contract
- Stable fields: exported NIF names, atoms, JSON payload shapes, callback event
  tags, and resource handle semantics are machine-consumed by host code.
- Defaults: default features expose no frontend HTTP workflow surface.
- Enums and labels: type-parsing labels, atoms, and event tags carry semantic
  meaning.
- Ordering: event messages preserve backend workflow event ordering as observed
  by the bridge.
- Compatibility: changing exported NIF names or resource shapes is breaking for
  BEAM consumers.
- Regeneration/migration: NIF surface changes must update host docs, native
  tests, BEAM smoke paths, release notes, and this README together.

## Testing
```bash
cargo test -p pantograph_rustler
```

## Notes
- `resource_registration.rs` carries a scoped `non_local_definitions` lint
  exception for the current `rustler::resource!` expansion; remove it when
  Rustler exposes a warning-clean registration API.
- `lib.rs` remains over the decomposition threshold and is tracked in the
  standards compliance plan.
