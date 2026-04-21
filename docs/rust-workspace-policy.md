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

Existing Rust warning debt remains a tracked baseline until M7 classifies each
warning as remove, use, feature-gate, or intentionally retained.

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
