# pantograph-rustler

Rustler NIF wrapper crate for Pantograph workflow/runtime integration on the
BEAM.

## Purpose
This crate exposes selected Pantograph workflow APIs to Elixir/Erlang through
Rustler NIFs. The boundary exists to keep BEAM resource wrappers, NIF scheduling,
callback bridging, and error-term mapping separate from backend workflow and
runtime semantics.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Crate manifest, cdylib configuration, and Rustler feature flags. |
| `src/` | NIF facade, resource wrappers, helper modules, and source-level README. |

## Problem
BEAM consumers need native workflow execution and callback integration without
embedding Rust implementation details directly into Elixir code. Without a
wrapper crate, Rustler-specific scheduling and resource lifecycle would leak
into core workflow crates.

## Constraints
- Core crates must not depend on Rustler.
- Long-running work must not block ordinary BEAM schedulers.
- NIF resources must own or reference runtime state safely.
- Callback/event semantics must be documented for host-language consumers.
- Binding support tiers must be documented as M5 hardening proceeds.

## Decision
Keep Rustler exports in this wrapper crate. The NIF facade delegates workflow
behavior to `node-engine`, `pantograph-workflow-service`, `workflow-nodes`, and
optional adapter crates while owning BEAM-facing conversion and resource
registration.

## Alternatives Rejected
- Put Rustler annotations in core crates: rejected because BEAM runtime details
  would leak into reusable Rust contracts.
- Make the Rustler surface mirror every internal function: rejected because
  binding APIs should be curated client contracts.
- Implement workflow policy in Elixir callbacks: rejected because backend Rust
  owns workflow semantics and runtime truth.

## Invariants
- Core workflow semantics remain backend-owned.
- Rustler NIFs either run on appropriate dirty schedulers or delegate work
  through owned runtime resources.
- Error mapping returns BEAM-safe terms without exposing third-party Rust error
  types.
- Event/callback behavior must preserve backend event semantics.

## Cargo Feature Contract
| Feature | Default | Contract |
| ------- | ------- | -------- |
| `frontend-http` | No | Enables NIFs and parameters that delegate workflow host behavior through `pantograph-frontend-http-adapter`. |

The default Rustler feature set intentionally exposes no frontend HTTP workflow
surface. NIF consumers must opt into HTTP adapter behavior explicitly.

## Revisit Triggers
- Rustler resource registration warnings require a crate-level lint exception
  or dependency upgrade.
- BEAM callback execution needs a stronger lifecycle supervisor.
- Public NIF support tiers change.
- The facade split changes exported function names.

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
- Lifecycle: BEAM code loads the NIF, creates resources, and drops resources
  when no longer referenced; runtime shutdown semantics must be hardened under
  M3/M5 lifecycle work.
- Errors: Rust errors are mapped to `rustler::Error::Term` values or structured
  JSON envelopes.
- Versioning: exported NIF names and resource shapes are public binding
  contracts for BEAM consumers.

## Binding Support Tiers
| Tier | Surface | Contract |
| ---- | ------- | -------- |
| Supported | `version()`, workflow JSON graph helpers, executor resources, orchestration store operations, node registry operations, callback response/error NIFs, and Pumas API resource operations. | BEAM hosts may rely on these names and JSON/resource shapes with native NIF and Elixir wrapper versions matched. |
| Experimental | `frontend-http` feature exports and callback/orchestration paths that still need stronger lifecycle supervision. | Available for integration work; support tier can change with coordinated host smoke tests. |
| Internal-only | Rust resource wrapper structs, registration helpers, parsing helper modules, and non-NIF implementation functions. | Not part of the BEAM API even when compiled into the native library. |

The native artifact name is `pantograph_rustler` as configured by the Rustler
crate and loaded by `Elixir.Pantograph.Native`. The host-visible `version()`
NIF returns `CARGO_PKG_VERSION`; BEAM wrappers and native artifacts must come
from the same package version.

## Structured Producer Contract
- Stable fields: exported NIF names, JSON payload shapes, event message tags,
  and resource handle semantics are machine-consumed by host code.
- Defaults: the default feature set exposes no frontend HTTP workflow surface.
- Enums and labels: atoms, NIF names, and event tags carry semantic meaning.
- Ordering: event messages preserve backend workflow event ordering as observed
  by the bridge.
- Compatibility: changing exported NIF names or resource shapes is breaking for
  BEAM consumers.
- Regeneration/migration: NIF surface changes must update host docs, native
  tests, BEAM smoke paths, and release notes together.

## Testing
```bash
cargo check -p pantograph_rustler
```

Direct `cargo test -p pantograph_rustler` currently links against Rustler NIF
symbols that are supplied by the BEAM runtime in host execution. Track a
dedicated BEAM-backed test harness before treating crate-local Rust tests as the
canonical binding verification path.

## Notes
- `src/resource_registration.rs` carries a scoped `non_local_definitions` lint
  exception for the current `rustler::resource!` expansion; remove it when
  Rustler exposes a warning-clean registration API.
- `src/lib.rs` is still over the decomposition threshold, but BEAM binding DTOs,
  resource wrappers, callback/event transport, executor/inference-gateway
  dispatch, frontend HTTP dispatch, orchestration store JSON CRUD,
  orchestration execution dispatch, registry/extension dispatch, and Pumas
  model-library dispatch now live in focused source modules as part of the
  facade split tracked by the standards compliance plan.
