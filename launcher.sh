#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

VENV_DIR="$ROOT_DIR/.venv"
VENV_PYTHON="$VENV_DIR/bin/python3"

# Increase file descriptor limit to prevent "too many open files" errors
# Vite + Tauri both watch files, requiring many open handles
ulimit -n 65536 2>/dev/null || ulimit -n 16384 2>/dev/null || ulimit -n 4096 2>/dev/null || true

# --- Helper: check if a requirements file is satisfied ---
check_requirements() {
  local req_file="$1"
  while IFS= read -r line; do
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    [[ -z "${line// /}" ]] && continue
    local pkg
    if [[ "$line" =~ ^https?:// ]]; then
      # URL-based requirement: extract package name from wheel filename
      # e.g. .../flash_attn-2.8.3+cu128torch2.10-cp312-cp312-linux_x86_64.whl
      local filename="${line##*/}"
      pkg="${filename%%-*}"          # everything before first dash
    else
      # Standard requirement: strip version/url specifiers
      pkg="${line%% @*}"             # strip " @ git+..." suffix
      pkg="${pkg%%[>=<\[]*}"         # strip version specifiers
      pkg="${pkg// /}"               # strip whitespace
    fi
    pkg="${pkg//-/_}"                # dashes to underscores for import
    if ! "$VENV_PYTHON" -c "import $pkg" 2>/dev/null; then
      return 1
    fi
  done < "$req_file"
  return 0
}

# --- Helper: install from a requirements file if needed ---
install_requirements() {
  local req_file="$1"
  local label="$2"

  if check_requirements "$req_file"; then
    echo "[python] $label dependencies satisfied, skipping"
  else
    echo "[python] Installing $label dependencies..."
    "$VENV_PYTHON" -m pip install -r "$req_file"
  fi
}

# --- Helper: list available extras ---
list_extras() {
  local found=false
  for f in "$ROOT_DIR"/requirements-*.txt; do
    [ -f "$f" ] || continue
    found=true
    local name desc
    name="$(basename "$f" .txt)"
    name="${name#requirements-}"
    desc="$(head -1 "$f" | sed 's/^#[[:space:]]*//')"
    printf "  %-12s %s\n" "$name" "$desc"
  done
  if [ "$found" = false ]; then
    echo "  (none)"
  fi
}

# --- Install mode ---
if [ "${1:-}" = "--install" ]; then
  shift
  EXTRAS=("${@}")

  echo "=== Pantograph dependency installer ==="

  # 1. Node modules (always run to pick up new packages)
  echo "[npm] Installing node dependencies..."
  npm install

  # 2. Python venv
  SYSTEM_PYTHON="$(command -v python3 2>/dev/null || true)"
  if [ -z "$SYSTEM_PYTHON" ]; then
    echo "[python] ERROR: python3 not found. Install Python 3.10+ first."
    exit 1
  fi

  if [ ! -d "$VENV_DIR" ]; then
    echo "[python] Creating venv at $VENV_DIR..."
    "$SYSTEM_PYTHON" -m venv "$VENV_DIR"
    "$VENV_PYTHON" -m pip install --upgrade pip -q
  else
    echo "[python] venv exists at $VENV_DIR"
  fi

  # 3. Base requirements
  install_requirements "$ROOT_DIR/requirements.txt" "base"

  # 4. Extras
  if [ ${#EXTRAS[@]} -eq 0 ]; then
    echo ""
    echo "Extras available (install with: ./launcher.sh --install <extra> ...):"
    list_extras
  else
    for extra in "${EXTRAS[@]}"; do
      if [ "$extra" = "all" ]; then
        for f in "$ROOT_DIR"/requirements-*.txt; do
          [ -f "$f" ] || continue
          ename="$(basename "$f" .txt)"
          ename="${ename#requirements-}"
          install_requirements "$f" "$ename"
        done
        break
      fi

      req_file="$ROOT_DIR/requirements-${extra}.txt"
      if [ ! -f "$req_file" ]; then
        echo "[python] ERROR: Unknown extra '$extra'. Available:"
        list_extras
        exit 1
      fi
      install_requirements "$req_file" "$extra"
    done
  fi

  echo ""
  echo "=== Install complete ==="
  echo "Run ./launcher.sh to start development server"
  exit 0
fi

# --- Activate venv for PyO3 if available ---
if [ -x "$VENV_PYTHON" ]; then
  export PYO3_PYTHON="$VENV_PYTHON"
  export VIRTUAL_ENV="$VENV_DIR"
  export PATH="$VENV_DIR/bin:$PATH"
fi

# --- Ensure node_modules ---
if [ ! -d "node_modules" ]; then
  npm install
fi

# --- Launch ---
if [ "${1:-}" = "--release" ]; then
  npm run build:desktop
else
  npm run dev:desktop
fi
