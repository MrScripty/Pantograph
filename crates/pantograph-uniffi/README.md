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
- `src/lib.rs` is over the decomposition threshold and remains tracked in the
  standards compliance plan.
