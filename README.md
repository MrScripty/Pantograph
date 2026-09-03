# Pantograph

![banner_3](https://github.com/user-attachments/assets/32b9a8c3-39b1-4fdf-ae55-c0ea9d850929)


Pantograph is a local-first, Rust-native framework with an optional desktop app.
It provides unified local inference, node-based workflows, resource-aware
runtime scheduling, real-time observability, and diagnostic tracing for AI
pipelines. Host integrations consume the native Rust interface or projections
through UniFFI, Rustler, and optional HTTP adapters.

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for the current module map, target
scheduler-owned task execution model, ownership rules, active transition, and
links to authoritative ADRs and implementation status.

The [current standards audit](docs/audits/2026-09-03-current-standards/README.md)
records the repository-wide compliance baseline and routes its findings into
focused follow-up audits. Its
[remediation portfolio](docs/plans/current-standards-remediation/plan.md)
coordinates the resulting implementation plans.

## Quick Start

1. Clone the repository.
2. Install dependencies:
   ```bash
   npm install
   ```
3. Run the desktop app:
   ```bash
   npm run dev:desktop
   ```

## Installation

### Prerequisites

- Node.js (for `npm`)
- Rust toolchain (`cargo`, `rustc`)
- Tauri system libraries for your OS

Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Install Tauri system dependencies:

```bash
# Debian/Ubuntu
sudo apt install pkg-config libsoup2.4-dev libjavascriptcoregtk-4.0-dev

# Fedora
sudo dnf install pkgconf-pkg-config libsoup-devel javascriptcoregtk4.0-devel

# Arch
sudo pacman -S pkgconf libsoup2 webkit2gtk
```

Install project dependencies:

```bash
npm install
```

## Usage

### Desktop Mode (recommended)

```bash
npm run dev:desktop
```

### Web Preview

```bash
npm run dev
```

The Vite dev server binds to `127.0.0.1` by default. For an intentional LAN
preview, set `PANTOGRAPH_VITE_HOST=0.0.0.0` when starting the server.

### Build Desktop App

```bash
npm run build:desktop
```

### Launcher Script

```bash
./launcher.sh --help
./launcher.sh --test
./launcher.sh --release-smoke
```

### Vision Backend Options

- External OpenAI-compatible server (for example LM Studio)
- Bundled `llama.cpp` sidecar with local model files

## Development

### Prerequisites

- Node.js + npm matching `.node-version` and `package.json`
- Rust toolchain matching `rust-toolchain.toml`
- Python matching `.python-version` for Python-backed smoke paths
- Tauri system dependencies (above)

See [Development](docs/development.md) for toolchain ownership, setup caveats,
and the current verification status.

### Useful Commands

```bash
# Frontend type and configured test checks
npm run typecheck
npm run test:frontend

# Rust workspace checks
cargo fmt --all -- --check
cargo check --workspace --no-default-features
```

These commands prove only their stated scopes. Pantograph does not currently
have one green repository-wide compliance command; see the
[verification audit](docs/audits/2026-09-03-current-standards/04-verification-and-tooling.md).

### Runtime Separation

Python-backed model execution is out-of-process and externally provisioned.
See [Runtime Operations](docs/runtime-operations.md) for interpreter selection,
runtime inspection, recovery, and current security/lifecycle limitations.

### Headless Workflow API

Pantograph exposes a Rust-first service with UniFFI, Rustler, and optional HTTP
projections. See [Headless Workflow Integration](docs/headless-workflow.md) for
the supported ownership model, session flow, exact contract sources, and known
transition boundaries.

## Project Structure

| Path | Description |
| ---- | ----------- |
| `ARCHITECTURE.md` | Current architecture, target execution model, ownership, and status entry points |
| `src/` | Frontend Svelte app, UI components, stores, and services |
| `src-tauri/src/` | Tauri backend commands and runtime wiring |
| `crates/` | Shared Rust crates (`inference`, `node-engine`, `workflow-nodes`, bindings) |
| `packages/svelte-graph/src/` | Reusable graph editor package modules |
| `scripts/` | Validation and tooling scripts |
| `docs/` | Current guides, decisions, audits, and implementation plans |

## Contributing

1. Create a focused branch for one logical change.
2. Follow coding, tooling, accessibility, and documentation standards.
3. Run the smallest checks that decide the affected behavior and contracts.
4. Use Conventional Commits for all commits.

## License

The repository currently contains Apache-2.0 license material while Cargo
metadata declares `MIT OR Apache-2.0`. Resolve that mismatch before
distribution; see [Release](docs/release.md).
