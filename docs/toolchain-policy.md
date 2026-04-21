# Toolchain Policy

Pantograph pins its local and CI toolchains so builds, tests, and release
artifacts do not drift across developer machines or runners.

## Pinned Versions

| Tool | Version | Pin |
| ---- | ------- | --- |
| Rust | `1.92.0` | `rust-toolchain.toml` and Cargo `rust-version = "1.92"` |
| Node.js | `24.12.0` | `.node-version` and `package.json` `engines.node` |
| npm | `11.6.2` | `package.json` `packageManager` and `engines.npm` |
| Python | `3.12.3` | `.python-version` |

These versions match the active development environment used when the standards
compliance pass added toolchain pinning.

## Ownership

- `rust-toolchain.toml` is the source of truth for `rustup` and CI Rust
  installation.
- `Cargo.toml` keeps the Rust package `rust-version` aligned with the pinned
  toolchain until a separate MSRV review lowers it deliberately.
- `.node-version` is the source of truth for Node version managers.
- `package.json` records the npm version and rejects accidental Node/npm drift
  in package-manager aware tooling.
- `.python-version` records the Python version used for local virtual
  environments and Python-backed smoke paths.

## Update Policy

Toolchain updates must be deliberate changes that update all matching pins in
the same commit. The minimum verification for a toolchain bump is:

- `npm run lint:no-new`
- `npm run typecheck`
- `npm run test:frontend`
- `cargo check --workspace --no-default-features`
- `cargo check --workspace --all-features`

Run targeted runtime or binding smoke checks when the toolchain bump touches
their execution surface.
