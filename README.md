# Pantograph

Pantograph is a local-first desktop app that turns sketches and prompts into editable Svelte UI.

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

### Build Desktop App

```bash
npm run build:desktop
```

### Launcher Script

```bash
./launcher.sh --help
```

### Vision Backend Options

- External OpenAI-compatible server (for example LM Studio)
- Bundled `llama.cpp` sidecar with local model files

## Development

### Prerequisites

- Node.js + npm
- Rust toolchain
- Tauri system dependencies (above)

### Useful Commands

```bash
# Lint (configured scope)
npm run lint

# Full lint scan
npm run lint:full

# Critical anti-pattern gate (src/ + packages/)
npm run lint:critical

# Type check
npm run typecheck

# Tests
npm test

# Runtime separation guard (no compile-time Python linkage)
npm run test:runtime-separation

# All quality gates
npm run check
```

### Runtime Separation

Python-backed model execution is intentionally out-of-process and externally provisioned.
See `docs/python-runtime-separation.md` for configuration and migration details.

### Headless Embedding API

Pantograph also exposes a versioned, Rust-first headless embedding API for host integrations:

- `embed_objects_v1`
- `get_embedding_workflow_capabilities_v1`

Reference docs:

- Contract: `docs/headless-embedding-api-v1.md`
- Migration guide: `docs/headless-embedding-migration.md`
- Service boundary ADR: `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- Implementation notes: `docs/headless-embedding-implementation-notes.md`

## Project Structure

| Path | Description |
| ---- | ----------- |
| `src/` | Frontend Svelte app, UI components, stores, and services |
| `src-tauri/src/` | Tauri backend commands and runtime wiring |
| `crates/` | Shared Rust crates (`inference`, `node-engine`, `workflow-nodes`, bindings) |
| `packages/svelte-graph/src/` | Reusable graph editor package modules |
| `scripts/` | Validation and tooling scripts |
| `docs/` | Architecture and process documentation |

## Contributing

1. Create a focused branch for one logical change.
2. Follow coding, tooling, accessibility, and documentation standards.
3. Run `npm run check` and relevant targeted Rust tests before opening a PR.
4. Use Conventional Commits for all commits.

## License

Workspace crates declare `MIT OR Apache-2.0` in Cargo metadata. Review individual package metadata for any exceptions.
