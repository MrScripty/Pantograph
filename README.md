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

Pinned toolchain ownership and update policy are documented in
`docs/toolchain-policy.md`.

### Useful Commands

```bash
# Lint (configured scope)
npm run lint

# Full lint scan
npm run lint:full

# Critical anti-pattern gate (src/ + packages/)
npm run lint:critical

# Focused Svelte accessibility gate
npm run lint:a11y

# No-new-debt gate for critical anti-patterns and decision traceability
npm run lint:no-new

# Rust formatting audit
npm run format:check

# Type check
npm run typecheck

# Tests
npm test

# Runtime separation guard (no compile-time Python linkage)
npm run test:runtime-separation

# Opt-in BEAM / Rustler host smoke
npm run test:rustler-beam-smoke

# Configured quality gates
npm run check

# Canonical local quality gate
./launcher.sh --test
```

Testing placement, cross-layer acceptance requirements, and release-smoke CI
strategy are documented in `docs/testing-and-release-strategy.md`.

### Runtime Separation

Python-backed model execution is intentionally out-of-process and externally provisioned.
See `docs/python-runtime-separation.md` for configuration and migration details.
For a local diffusion worker smoke path, run:

```bash
./.venv/bin/python scripts/diffusion_cli_smoketest.py --model-path /path/to/tiny-sd-turbo
```

### Headless Workflow API

Pantograph exposes a Rust-first headless workflow API for host integrations
through `crates/pantograph-workflow-service`:

- `workflow_get_capabilities`
- `workflow_get_io`
- `workflow_preflight`
- `create_workflow_execution_session`
- `run_workflow_execution_session`
- `close_workflow_execution_session`
- `workflow_get_execution_session_status`
- `workflow_list_execution_session_queue`
- `workflow_cancel_execution_session_queue_item`
- `workflow_reprioritize_execution_session_queue_item`
- `workflow_set_execution_session_keep_alive`

Integration boundary:

- Headless hosts should integrate with the core API/service crate directly.
- `src-tauri` commands are desktop app transport adapters, not the headless API.
- HTTP binding exports are opt-in frontend adapters for modular standalone GUI
  hosting (`frontend-http` in UniFFI and Rustler).
- Recommended headless flow: inspect I/O and preflight, create an execution
  session, submit work through that session, inspect its scheduler-owned state,
  and then close it or keep it alive.

## Project Structure

| Path | Description |
| ---- | ----------- |
| `ARCHITECTURE.md` | Current architecture, target execution model, ownership, and status entry points |
| `src/` | Frontend Svelte app, UI components, stores, and services |
| `src-tauri/src/` | Tauri backend commands and runtime wiring |
| `crates/` | Shared Rust crates (`inference`, `node-engine`, `workflow-nodes`, bindings) |
| `packages/svelte-graph/src/` | Reusable graph editor package modules |
| `scripts/` | Validation and tooling scripts |
| `docs/` | Architecture and process documentation |

## Contributing

1. Create a focused branch for one logical change.
2. Follow coding, tooling, accessibility, and documentation standards.
3. Run `./launcher.sh --test` and relevant targeted checks before opening a PR.
4. Use Conventional Commits for all commits.

## License

Workspace crates declare `MIT OR Apache-2.0` in Cargo metadata. Review individual package metadata for any exceptions.
