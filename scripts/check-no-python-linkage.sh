#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

contains_pattern() {
  local pattern="$1"
  if command -v rg >/dev/null 2>&1; then
    rg -q -e "$pattern"
  else
    grep -E -q -- "$pattern"
  fi
}

contains_pattern_ci() {
  local pattern="$1"
  if command -v rg >/dev/null 2>&1; then
    rg -q -i -e "$pattern"
  else
    grep -E -q -i -- "$pattern"
  fi
}

echo "[check] verifying pantograph dependency graph excludes pyo3"
if cargo tree -p pantograph --manifest-path src-tauri/Cargo.toml | contains_pattern '\\bpyo3\\b'; then
  echo "FAIL: pyo3 detected in pantograph dependency graph"
  exit 1
fi

echo "[check] building pantograph binary for linkage inspection"
cargo build --manifest-path src-tauri/Cargo.toml -p pantograph

BIN_PATH="target/debug/pantograph"
if [[ ! -f "$BIN_PATH" ]]; then
  echo "FAIL: expected binary not found at $BIN_PATH"
  exit 1
fi

case "$(uname -s)" in
  Linux)
    echo "[check] linux linkage scan (ldd)"
    if ldd "$BIN_PATH" | contains_pattern_ci 'libpython|python[0-9]'; then
      echo "FAIL: python shared library linkage detected in pantograph binary"
      exit 1
    fi
    ;;
  Darwin)
    if command -v otool >/dev/null 2>&1; then
      echo "[check] macOS linkage scan (otool -L)"
      if otool -L "$BIN_PATH" | contains_pattern_ci 'libpython|python[0-9]'; then
        echo "FAIL: python shared library linkage detected in pantograph binary"
        exit 1
      fi
    else
      echo "[check] skipping macOS linkage scan: otool not available"
    fi
    ;;
  *)
    echo "[check] unsupported OS for dynamic linker scan, tree check completed"
    ;;
esac

echo "PASS: pantograph remains separated from compile-time python linkage"
