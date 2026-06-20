#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

fail() {
  printf 'workflow editor image-generation GUI smoke preflight failed: %s\n' "$1" >&2
  exit 2
}

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    fail "missing required environment variable: ${name}"
  fi
}

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    fail "missing required command on PATH: ${command_name}"
  fi
}

if [[ -n "${PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH:-}" || -n "${PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH:-}" ]]; then
  cat >&2 <<'EOF'
PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH and PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH are retired.

Use PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID and
PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID so the desktop GUI smoke exercises
the canonical Pumas model package/load-target path.
EOF
  exit 2
fi

require_env PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID
require_env PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID
require_env PANTOGRAPH_PYTHON_EXECUTABLE

if [[ ! -x "${PANTOGRAPH_PYTHON_EXECUTABLE}" ]]; then
  fail "PANTOGRAPH_PYTHON_EXECUTABLE is not executable: ${PANTOGRAPH_PYTHON_EXECUTABLE}"
fi

require_command tauri-driver

case "$(uname -s)" in
  Linux)
    require_command WebKitWebDriver
    if [[ -z "${DISPLAY:-}" && -z "${WAYLAND_DISPLAY:-}" ]]; then
      fail "Linux GUI display is unavailable; set DISPLAY or WAYLAND_DISPLAY"
    fi
    ;;
  *)
    fail "this scaffolded GUI smoke currently supports Linux only"
    ;;
esac

if [[ ! -x node_modules/.bin/wdio ]]; then
  fail "WebdriverIO is not installed; run npm ci before this smoke"
fi

exec node_modules/.bin/wdio run tests/e2e/workflow-editor-image-generation/wdio.conf.mjs
