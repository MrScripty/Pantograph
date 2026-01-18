# Pantograph

Transform drawings into Svelte GUI's with fully local AI.

This is a stand-alone feature demo for Studio Whip, prototyping how GUI creation and editing works. Users draw images of what they want, and provide a short text prompt describing it. The AI sees the drawing and generates the matching Svelte UI elements.

## Features

- **Drawing Canvas**: Freehand drawing with customizable tools
- **Vision LLM Integration**: Send your drawings to a vision-capable LLM for analysis
  - Connect to external OpenAI-compatible servers (e.g., LM Studio)
  - Or use a bundled llama.cpp sidecar with your own model files
- **Streaming Responses**: Real-time streaming of LLM responses in a side panel

## Prerequisites

Install Node.js (for npm) and the Rust toolchain (Cargo).

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Install Tauri system dependencies (includes `libsoup-2.4` and `javascriptcoregtk-4.0`):

```bash
# Debian/Ubuntu
sudo apt install pkg-config libsoup2.4-dev libjavascriptcoregtk-4.0-dev

# Fedora
sudo dnf install pkgconf-pkg-config libsoup-devel javascriptcoregtk4.0-devel

# Arch
sudo pacman -S pkgconf libsoup2 webkit2gtk
```

## Development (Desktop)

```bash
npm install
npm run dev:desktop
```

Requires the Rust toolchain and Tauri system dependencies for your OS.
Runtime assets are local (no CDN), so the app is offline-safe once dependencies are installed.

## Development (Web Preview)

```bash
npm run dev
```

## Build Desktop App

```bash
npm run build:desktop
```

## Launcher

```bash
./launcher.sh
```

## Using the Vision LLM

### Option 1: External Server (e.g., LM Studio)

1. Start LM Studio and load a vision-capable model (e.g., GLM-4.6V-Flash)
2. Enable the local server in LM Studio (default: `http://localhost:1234`)
3. In Pantograph, open the side panel and enter the server URL
4. Click "Connect"
5. Draw on the canvas, enter a prompt, and click "Go"

### Option 2: Bundled llama.cpp Sidecar

1. Download `llama-server` from [llama.cpp releases](https://github.com/ggerganov/llama.cpp/releases)
2. Rename it with the target triple suffix and place in `src-tauri/binaries/`:
   - Linux: `llama-server-x86_64-unknown-linux-gnu`
   - macOS Intel: `llama-server-x86_64-apple-darwin`
   - macOS Apple Silicon: `llama-server-aarch64-apple-darwin`
   - Windows: `llama-server-x86_64-pc-windows-msvc.exe`
3. Download a vision model with mmproj file (e.g., GLM-4.6V-Flash GGUF)
4. In the side panel, enter the model and mmproj paths, then click "Start Sidecar"

## Recommended Vision Models

- **GLM-4.6V-Flash**: Full llama.cpp support with mmproj file. This is the model being used for development testing.
- Any vision model compatible with LM Studio or llama.cpp

## Keyboard Shortcuts

| Shortcut       | Action                                     |
| -------------- | ------------------------------------------ |
| `Ctrl+Z`       | Undo canvas drawing stroke                 |
| `Ctrl+Shift+Z` | Unified undo (unhide commits, etc.)        |
| `Alt+Ctrl+Z`   | Undo component change (git)                |
| `Ctrl+Y`       | Redo component change (git)                |
| `Alt+Ctrl+Y`   | Unified redo                               |
| `Tab`          | Toggle between Draw and Interact modes     |
| `Ctrl+\``      | Toggle between Canvas and Node Graph views |

## Commit Timeline

A minimal commit timeline appears above the toolbar when you have generated components. Hover to expand it.

| Action                         | Effect                                            |
| ------------------------------ | ------------------------------------------------- |
| **Click** a commit node        | Soft delete (hide) - can undo with `Ctrl+Shift+Z` |
| **Ctrl+Click** a commit node   | Hard delete (permanent, with confirmation)        |
| **Double-click** a commit node | Checkout that commit                              |

**Note:** Soft-deleted commits are automatically hard-deleted after 32 undo steps to keep history clean.

## Tech Stack

- **Frontend**: Svelte 5, TypeScript, Tailwind CSS
- **Backend**: Tauri 2.9, Rust
- **Build**: Vite
