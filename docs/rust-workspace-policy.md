# Rust Workspace Policy

This document records the repo-local Rust policy required by the standards
compliance plan.

## Package Metadata

Rust workspace crates inherit shared `version`, `edition`, `rust-version`,
`license`, and `repository` metadata from the root `Cargo.toml` unless the
crate has a product-specific reason to override it. The Tauri application keeps
its app version separate, but inherits the shared toolchain, license, and
repository metadata.

All Rust workspace members declare `publish = false`. This repository is not
currently treating any crate as independently published crates.io API.

## Lints

Every workspace member opts into `[workspace.lints]` through:

```toml
[lints]
workspace = true
```

The root policy denies repo-owned `unsafe` code by default. Clippy lint policy
starts as a ratchet: debug macros and TODOs warn, while unsafe documentation
lints are denied so any future exception must be documented.

Rust warning cleanup history remains tracked in
`docs/standards-compliance-analysis/rust-warning-baseline.md`; the current M7
baseline is zero `cargo check` warnings.

## Dependency Inheritance

Dependencies used by two or more Rust workspace members belong in the root
`[workspace.dependencies]` table. Member crates still declare every dependency
they directly use, but shared versions are referenced with `.workspace = true`
or `{ workspace = true }`.

The current shared dependency set includes common async, serialization,
compression, runtime, testing, and graph crates. Single-owner dependencies stay
in the owning crate manifest unless another member starts using them.

When adding a dependency:

- Put it in the owning member manifest if only one crate uses it.
- Move it to `[workspace.dependencies]` once two or more members need the same
  crate.
- Keep optional feature ownership in the member manifest, even when the version
  is inherited from the workspace.
- Do not use workspace inheritance to hide a dependency that a member actually
  owns and imports.

## Warning Ratchet

Current policy:

- `cargo check --workspace --all-features` and
  `cargo check --workspace --no-default-features` must compile.
- `unused`, `dead_code`, and dependency macro warnings are expected to remain
  at zero for `cargo check`.
- `clippy -D warnings` is not a blocking gate until M7 resolves the
  clippy-specific findings exposed after the rustc warning baseline reached
  zero. The audit has cleared `inference`, `node-engine`, and
  `workflow-nodes`, `pantograph-workflow-service`, and
  `pantograph-frontend-http-adapter`; the current workspace run now stops in
  `pantograph-embedded-runtime`.
- New policy lints may be denied only when they are known not to fail the
  current workspace.

Ratchet sequence:

1. Keep `docs/standards-compliance-analysis/rust-warning-baseline.md` current
   as the zero-warning history for `cargo check`.
2. Add a non-regression check for the zero rustc warning baseline.
3. Resolve clippy-specific findings separately from rustc warning cleanup.
4. Promote clippy to `-D warnings` only after the clippy audit is clean or
   explicitly machine-enforced.

## Unsafe Exceptions

New unsafe code is not allowed unless a future change first introduces a narrow
exception with:

- The owning crate and module.
- The exact operation that requires unsafe code.
- The invariant that makes the unsafe block valid.
- A safe wrapper boundary and tests that exercise the wrapper.
- A review note explaining why a safe alternative is not practical.
- A lint exception scoped to the smallest possible module or item.

Do not add crate-wide or workspace-wide unsafe exceptions.

## Platform Cfg Policy

Non-test platform `cfg` blocks are allowed only when they stay in one of these
categories:

- Thin platform adapter modules such as managed-runtime `llama.cpp` and Ollama
  platform selectors.
- Small platform filesystem/process affordances, for example Unix symlink,
  permission, PID, or port-inspection helpers.
- Compile-time native artifact naming for binding packaging.
- Cargo feature gates that do not branch on operating system or architecture.

The April 21 review found current non-test platform `cfg` usage in:

- `crates/inference/src/managed_runtime/llama_cpp_platform/`
- `crates/inference/src/managed_runtime/ollama_platform/`
- `crates/inference/src/managed_runtime/archive.rs`
- `src-tauri/src/llm/port_manager.rs`
- `src-tauri/src/llm/server_discovery.rs`
- `crates/workflow-nodes/src/system/process.rs`
- `crates/node-engine/src/path_validation.rs`
- `crates/pantograph-uniffi/src/runtime.rs`

Those fit the exception rule today. If any platform branch grows into business
logic, move it behind a named platform module or adapter before adding new
behavior.

## Benchmark Policy

Performance claims and hot-path changes must include Criterion evidence unless
the change is explicitly documentation-only or the performance impact is not a
goal.

Benchmark requirements:

- Place benchmarks under the owning crate's `benches/` directory.
- Use Criterion for timing and report the baseline command and comparison
  target.
- Benchmark the smallest stable public or crate-private boundary that reflects
  the claim.
- Keep generated benchmark output out of source control.
- Record any hardware, runtime feature, model, or dataset assumption needed to
  interpret the result.

Do not use ad hoc wall-clock logs as the only evidence for a performance claim.
