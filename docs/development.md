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

## ONNX Runtime provisioning

Cargo builds do not download ONNX Runtime. The Pumas dependency uses dynamic
loading; metadata and host lifecycle use does not require a native SDK. The
consumer feature gate (`python3 scripts/check-onnx-no-build-download.py`) checks
additive features and keeps the manifest, lockfile and CI checkout pin aligned.

Before ONNX execution, separately provision and verify Microsoft's ONNX Runtime
1.24.2 for the host target (C API 24). Record the official archive and library
hashes and preserve upstream notices. Set `ORT_DYLIB_PATH` to the absolute library
file for that invocation, or stage the full SDK closure beside the executable.
This is runtime configuration; `ORT_LIB_PATH` and download-suppression variables
are not substitutes. Missing/invalid selection returns a typed `runtime_library`
error. Build and test jobs must consume an already provisioned SDK when execution
needs it; they must not acquire one. Native inference and extracted-package
qualification are separate from successful compilation and metadata tests.

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

`cargo test --locked -p pantograph-app-config` executes the production persisted
AppConfig loader, save/restore, and startup registry composition used by desktop
setup. These tests need no GTK/WebKit or inference runtime. They exercise real
filesystem failures and shared admission, but do not qualify Tauri setup, IPC,
or a running desktop process. Desktop build prerequisites remain required for
those checks.

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
- Decision traceability checks declared decision-to-guide impacts and local
  references; semantic documentation coverage still requires review. See
  [gate operation](../scripts/README.md#decision-traceability-operation).

Those limitations are active work, not setup exceptions. See the
[current remediation portfolio](plans/current-standards-remediation/plan.md).
