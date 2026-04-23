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
| `lib.rs` | Public NIF facade, exported entrypoints, and module load wiring. |
| `binding_types.rs` | BEAM-facing enum and struct declarations used by NIF signatures. |
| `callback_bridge.rs` | BEAM callback task executor, core-first fallback executor, event sink, and pending callback response state. |
| `elixir_data_graph_executor.rs` | Rustler-specific orchestration data-graph bridge into backend workflow execution. |
| `executor_nifs.rs` | Workflow executor resource construction, inference gateway setup, demand, cache, graph snapshot, and context I/O helpers behind exported NIF wrappers. |
| `frontend_http_nifs.rs` | Feature-gated frontend HTTP workflow/session implementation helpers behind exported NIF wrappers. |
| `orchestration_execution_nifs.rs` | Orchestration execution, inference-backed orchestration execution, and data-graph insertion helpers behind exported NIF wrappers. |
| `orchestration_store_nifs.rs` | Orchestration store resource creation and JSON CRUD helpers behind exported NIF wrappers. |
| `pumas_nifs.rs` | Pumas model-library resource, executor extension, download/import, and system-info helpers behind exported NIF wrappers. |
| `registry_nifs.rs` | Node registry, executor extension, and port-option query helpers behind exported NIF wrappers. |
| `resource_registration.rs` | NIF load-time Rustler resource registration boundary. |
| `resources.rs` | ResourceArc wrapper declarations for executor, orchestration, registry, Pumas, extensions, and inference gateway state. |
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
- Orchestration store JSON CRUD behavior stays in `orchestration_store_nifs.rs`;
  `lib.rs` keeps only the exported orchestration NIF wrappers.
- Orchestration execution, inference-backed orchestration execution, and
  data-graph insertion behavior stays in `orchestration_execution_nifs.rs`;
  `lib.rs` keeps only the exported orchestration execution NIF wrappers.
- Node registry and extension setup behavior stays in `registry_nifs.rs`;
  `lib.rs` keeps only the exported registry/extension NIF wrappers.
- Pumas model-library behavior stays in `pumas_nifs.rs`; `lib.rs` keeps only
  the exported Pumas and related executor-extension NIF wrappers.
- Executor construction, inference gateway setup, demand, cache, graph
  snapshots, and context I/O behavior stays in `executor_nifs.rs`; `lib.rs`
  keeps only the exported executor NIF wrappers.
- BEAM DTO and `ResourceArc` wrapper declarations stay outside `lib.rs` so the
  facade remains focused on exported NIF behavior and load wiring.
- Callback/event JSON serialization preserves backend event labels and order.
- Callback bridge state and BEAM event delivery stay in `callback_bridge.rs`;
  `lib.rs` keeps only the exported callback NIF wrappers.
- Event contract tests must construct the current backend event shape,
  including additive graph memory-impact fields, even when the BEAM projection
  only asserts the stable legacy fields.
- Async frontend-HTTP tests that temporarily change process current-directory
  state must serialize through an async-aware test mutex instead of holding a
  synchronous guard across workflow awaits.
- Frontend HTTP NIFs are unavailable unless the `frontend-http` feature is
  enabled, and their backend service dispatch stays in `frontend_http_nifs.rs`.

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
cargo check -p pantograph_rustler
```

Direct Rust test binaries currently fail to link outside a BEAM-backed NIF test
harness because the Erlang `enif_*` symbols are host-supplied.

## Notes
- `resource_registration.rs` carries a scoped `non_local_definitions` lint
  exception for the current `rustler::resource!` expansion; remove it when
  Rustler exposes a warning-clean registration API.
- `lib.rs` remains over the decomposition threshold and is tracked in the
  standards compliance plan.
