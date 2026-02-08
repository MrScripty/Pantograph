#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

# Increase file descriptor limit to prevent "too many open files" errors
# Vite + Tauri both watch files, requiring many open handles
ulimit -n 65536 2>/dev/null || ulimit -n 16384 2>/dev/null || ulimit -n 4096 2>/dev/null || true

if [ ! -d "node_modules" ]; then
  npm install
fi

if [ "${1:-}" = "--release" ]; then
  npm run build:desktop
else
  npm run dev:desktop
fi
