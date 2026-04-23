# pantograph-uniffi

UniFFI wrapper crate for Pantograph native workflow/runtime bindings.

## Purpose
This crate exposes curated Pantograph Rust APIs through UniFFI-generated
host-language bindings. The boundary exists to keep binding-specific type
conversion, error flattening, async bridging, and bindgen tooling separate from
core workflow and runtime crates.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Cargo.toml` | Crate manifest, cdylib/lib configuration, bindgen binary, and binding feature flags. |
| `src/` | UniFFI wrapper implementation, direct runtime wrapper, and source-level README. |

## Problem
Foreign-language consumers need a stable native Pantograph library and generated
binding files. Exposing core crates directly would leak Rust-specific types and
binding framework constraints into the product API.

## Constraints
- Core crates must compile and test without UniFFI.
- Wrapper DTOs must be FFI-safe.
- Generated bindings are artifacts, not hand-maintained source.
- Native library and generated bindings must be version-matched.
- Binding support tiers must be documented as M5 hardening proceeds.

## Decision
Keep UniFFI exports in this wrapper crate. The direct runtime surface delegates
to `pantograph-embedded-runtime`, and optional frontend HTTP exports delegate to
`pantograph-frontend-http-adapter`. This crate owns conversion and error
mapping, not workflow semantics.

## Alternatives Rejected
- Annotate every core type directly for UniFFI: rejected because several core
  types are not FFI-safe and binding concerns would leak inward.
- Use Tauri IPC as the native embedding API: rejected because non-desktop
  consumers need a product-native shared library.
- Hand-maintain C# binding files: rejected because generated bindings should
  come from one compiled native artifact.

## Invariants
- Core workflow/runtime logic does not depend on this crate.
- Every exported entry point maps to a documented consumer use case.
- Errors are flattened at the binding boundary.
- Public binding changes require generated host-language smoke coverage.

## Cargo Feature Contract
| Feature | Default | Contract |
| ------- | ------- | -------- |
| `embedded-runtime` | Yes | Exposes the direct embedded runtime wrapper and implies llama.cpp runtime support. |
| `backend-llamacpp` | Via `embedded-runtime` | Enables llama.cpp runtime dependencies for embedded execution. |
| `backend-ollama` | Yes | Enables Ollama runtime dependencies for embedded execution. |
| `backend-candle` | Yes | Enables Candle runtime dependencies for embedded execution. |
| `backend-pytorch` | No | Enables PyTorch runtime dependencies. Requires Python/PyTorch runtime availability. |
| `backend-audio` | No | Enables Python-backed audio runtime support. Requires audio Python dependencies. |
| `frontend-http` | No | Exposes optional frontend HTTP workflow host exports. |
| `cli` | No | Enables the UniFFI bindgen helper binary. |
| `runtime-deps` | Internal glue | Activates optional embedded-runtime and inference dependencies for backend features. |

Defaults expose the product-native embedded runtime plus selected local backend
families. Frontend HTTP and Python-backed families remain explicit opt-ins.

## Revisit Triggers
- A supported host language needs a different binding framework.
- Product-native artifact naming changes.
- FFI wrapper conversions become too broad for one facade file.
- Support tiers change for exported APIs.

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
Generate C# bindings from the compiled native library:

```bash
cargo build -p pantograph-uniffi --features embedded-runtime --lib
uniffi-bindgen-cs target/debug/libpantograph_headless.so \
  --crate pantograph_headless \
  --out-dir target/uniffi/csharp \
  --namespace uniffi.pantograph_headless
```

## API Consumer Contract
- Inputs: FFI-safe records, strings containing workflow JSON, runtime config
  records, and generated host-language calls.
- Outputs: generated binding methods, native shared library exports, response
  JSON, and FFI-safe error categories.
- Lifecycle: host code loads the generated binding and native library from the
  same build, creates runtime objects, and shuts them down through exported
  lifecycle APIs as those mature.
- Errors: core errors are mapped into binding-safe error values or response
  envelopes.
- Versioning: generated bindings and native library artifacts must come from
  the same release input.

## Binding Support Tiers
| Tier | Surface | Contract |
| ---- | ------- | -------- |
| Supported | `version()`, `validate_workflow_json`, `validate_orchestration_json`, and the `embedded-runtime` `FfiPantographRuntime` workflow/session/graph methods. | Product-native host integrations may rely on these when native library and generated bindings are version-matched. |
| Experimental | Legacy in-memory graph/executor/orchestration object facade and `frontend-http` exports. | Available for integration work, but method shape or support tier can change with coordinated docs/tests. |
| Internal-only | Wrapper records, conversion helpers, `runtime-deps`, and bindgen implementation details. | Not a standalone host API; consumers should use generated methods rather than depending on helper layout. |

Product-native artifact names are `libpantograph_headless.so`,
`libpantograph_headless.dylib`, or `pantograph_headless.dll`, depending on
platform. Generated bindings must be produced from the same native library build
and package version. The host-visible `version()` export returns
`CARGO_PKG_VERSION` for that native library.

## Structured Producer Contract
- Stable fields: generated binding files, UniFFI metadata, native library name,
  exported record fields, and feature-gated method names are machine-consumed.
- Defaults: default features currently expose embedded runtime and selected
  backend families.
- Enums and labels: FFI enum variants and method names are public binding
  contract labels.
- Ordering: generated file ordering is not a semantic contract.
- Compatibility: binding package consumers must not mix generated files and
  native libraries from different builds.
- Regeneration/migration: every API change requires binding regeneration,
  native wrapper tests, host-language smoke tests, and docs updates.

## Testing
```bash
cargo test -p pantograph-uniffi
./scripts/check-uniffi-csharp-smoke.sh
```

## Notes
- `src/lib.rs` remains near the decomposition threshold after moving
  crate-local tests into `src/lib_tests.rs` and internal event buffering into
  `src/workflow_event_bridge.rs`; direct runtime tests now live in
  `src/runtime_tests.rs`, and the remaining facade split is tracked in the
  standards compliance plan.
