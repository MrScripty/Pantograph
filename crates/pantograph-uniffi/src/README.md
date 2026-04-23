# crates/pantograph-uniffi/src

UniFFI source boundary for Pantograph native workflow bindings.

## Purpose
This directory owns the UniFFI-facing wrapper implementation for Pantograph
workflow and embedded-runtime APIs. It keeps FFI-safe DTO conversion, generated
binding metadata, async bridging, and host-language error mapping separate from
core workflow/runtime crates.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `lib.rs` | UniFFI exports, wrapper DTOs, legacy graph/orchestration surface, adapter delegation, and test module wiring. The legacy workflow engine owns graph CRUD, cache inspection, and event buffering only. |
| `lib_tests.rs` | Crate-local UniFFI facade tests, event projection tests, and feature-gated frontend HTTP binding contract tests. |
| `runtime.rs` | Direct `FfiPantographRuntime` wrapper over `pantograph-embedded-runtime`. |
| `runtime_tests.rs` | Direct embedded-runtime binding integration tests and runtime fixture helpers. |
| `workflow_event_bridge.rs` | Internal buffered workflow-event sink and backend event label projection used by the legacy workflow-engine binding object. |
| `bin/` | Binding generation helper utilities for supported UniFFI generator flows. |

## Problem
Native consumers need a stable shared library and generated bindings, but core
Rust workflow/runtime types are not all FFI-safe. Binding-specific conversion
and generated-language constraints must not leak into product-native backend
contracts.

## Constraints
- Core crates must compile and test without UniFFI.
- Exported DTOs and errors must be FFI-safe.
- Generated bindings must match the native library build.
- Frontend HTTP exports are optional and feature-gated.
- Direct embedded-runtime exports delegate to `pantograph-embedded-runtime`.

## Decision
Keep UniFFI exports in this source directory. The binding layer owns conversion
and error projection, while workflow semantics stay in
`pantograph-workflow-service` and runtime composition stays in
`pantograph-embedded-runtime`.

## Alternatives Rejected
- Annotate core workflow types directly for UniFFI: rejected because FFI
  concerns would leak inward and several types are not binding-safe.
- Use Tauri IPC as the native embedding API: rejected because non-desktop
  consumers need a product-native library.
- Hand-maintain generated host-language bindings: rejected because generated
  bindings must come from the compiled native artifact.

## Invariants
- `FfiPantographRuntime` wraps the direct embedded runtime when the
  `embedded-runtime` feature is enabled.
- Direct embedded-runtime binding tests stay in `runtime_tests.rs`; `runtime.rs`
  keeps only exported runtime wrapper methods, conversion helpers, and test
  module wiring.
- Request/response JSON contracts remain backend-owned by
  `pantograph-workflow-service`.
- `frontend-http` exports delegate to `pantograph-frontend-http-adapter`.
- Test fixtures for canonical workflow events must include all backend-owned
  event fields, including additive graph memory-impact metadata, so binding
  tests compile against the current `node-engine` contract.
- Crate-local tests stay in `lib_tests.rs`; `lib.rs` keeps only the test module
  declaration so exported binding definitions remain navigable.
- Buffered workflow-event delivery for the legacy engine object stays in
  `workflow_event_bridge.rs`; the exported `FfiWorkflowEvent` record stays in
  `lib.rs` to preserve binding metadata shape.
- Generated bindings and native library artifacts must be produced from the
  same build input.
- Public exported methods should map to documented host-language use cases.
- Binding objects should not retain placeholder runtime executors. If a binding
  surface exposes execution, it must own a real host/runtime execution contract
  or delegate to the embedded runtime wrapper.

## Revisit Triggers
- A supported host language needs a different binding framework.
- Native artifact naming or packaging changes.
- FFI wrapper conversions become too broad for one facade file.
- Public binding support tiers change.

## Dependencies
**Internal:** `node-engine`, `pantograph-workflow-service`,
`pantograph-embedded-runtime`, optional `pantograph-frontend-http-adapter`, and
optional `inference`.

**External:** `uniffi`, `tokio`, `async-trait`, `graph-flow`, `serde`,
`serde_json`, `thiserror`, and `pumas-library`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/embedded-runtime-extraction-plan.md`
- `docs/headless-native-bindings.md`

## Usage Examples
```rust
let runtime = FfiPantographRuntime::new(config, optional_pumas_api).await?;
```

## API Consumer Contract
- Inputs: FFI-safe records, strings containing workflow JSON, runtime config
  records, generated host-language method calls, and optional Pumas handles.
- Outputs: generated binding methods, native shared library exports, response
  JSON, and FFI-safe error categories.
- Lifecycle: host code loads the generated binding and native library from the
  same build, creates runtime objects, and shuts them down through exported
  lifecycle APIs as those mature.
- Errors: Rust errors are mapped into binding-safe error values or response
  envelopes.
- Versioning: generated bindings and native library artifacts must come from
  the same release input; exported method changes require host-language smoke
  updates.

## Structured Producer Contract
- Stable fields: generated binding metadata, native library name, exported
  record fields, method names, and feature-gated export sets are
  machine-consumed.
- Defaults: default features expose embedded runtime and selected backend
  families.
- Enums and labels: FFI enum variants and exported method names are public
  binding labels.
- Ordering: generated file ordering is not semantic, but generated metadata
  must match the compiled library.
- Compatibility: consumers must not mix generated bindings and native
  libraries from different builds.
- Regeneration/migration: every API change requires binding regeneration,
  native wrapper tests, host-language smoke tests, package scripts, and this
  README to update together.

## Testing
```bash
cargo test -p pantograph-uniffi
./scripts/check-uniffi-embedded-runtime-surface.sh
./scripts/check-uniffi-csharp-smoke.sh
```

## Notes
- `lib.rs` remains over the decomposition threshold after moving crate-local
  tests and is tracked in the standards compliance plan.
