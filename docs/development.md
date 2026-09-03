# Development

This guide records the minimum current setup and verification entry points.
Exact behavior claims and their required evidence are being normalized by the
[verification remediation plan](plans/current-standards-remediation/verification-and-tooling/plan.md).

## Toolchains

| Tool | Authoritative pin |
| --- | --- |
| Rust | `rust-toolchain.toml` and workspace `rust-version` |
| Node.js | `.node-version` and `package.json` |
| npm | `package.json#packageManager` |
| Python | `.python-version` |

Use the manifest and lockfile belonging to each ecosystem. The root
`Cargo.lock` is the Rust workspace lock; `src-tauri/Cargo.lock` is obsolete and
is scheduled for removal by the dependency remediation plan.

## Setup

```bash
npm install
./launcher.sh --install
```

`npm install` installs the frontend/tooling dependencies. The launcher can
provision the project Python environment and other declared local prerequisites.
Do not treat an existing `node_modules` directory or an importable Python module
as proof that all declared versions are satisfied; that contract is still under
remediation.

Platform-specific Tauri prerequisites are maintained in the
[official Tauri setup guide](https://v2.tauri.app/start/prerequisites/) and the
CI bootstrap in `.github/workflows/quality-gates.yml`.

## Useful Checks

```bash
cargo fmt --all -- --check
cargo check --workspace --no-default-features
cargo check --workspace --all-features
npm run typecheck
npm run test:frontend
npm test
```

Use targeted `cargo test -p <crate>` commands for affected Rust owners and the
specialized scripts under `scripts/` for binding, runtime, GUI, and packaging
paths.

There is no single green command that currently proves repository-wide
standards compliance. The [current audit baseline](audits/2026-09-03-current-standards/04-verification-and-tooling.md)
records which checks pass, which fail, and where test discovery is incomplete.
Do not upgrade a passing subset into a broader claim.

## Source Layout

- `crates/` contains the Rust workspace; see [the crate map](../crates/README.md).
- `src/` contains the Svelte application.
- `src-tauri/` composes the desktop host and transport adapters.
- `packages/svelte-graph/` contains the reusable graph-editor package.
- `bindings/` contains host-language examples and smoke consumers.
- `scripts/` contains repository orchestration and specialized checks.

Documentation follows ownership rather than directory shape. Add a local
README only when it serves a real consumer or operator at that boundary; source
directories do not require inventories or fixed headings.

## Current Limitations

- Frontend test registration is manually curated and omits tracked tests.
- Strict workspace Clippy and the current frontend lint/accessibility gates are
  not green.
- Release smoke does not yet prove the packaged artifact.
- Decision traceability still implements a retired README-per-directory model.

Those limitations are active work, not setup exceptions. See the
[current remediation portfolio](plans/current-standards-remediation/plan.md).
