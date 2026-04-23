#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HARNESS_DIR="$REPO_ROOT/bindings/beam/pantograph_native_smoke"

log() {
  printf '[rustler-beam-smoke] %s\n' "$*"
}

die() {
  log "FAIL: $*"
  exit 1
}

require_command() {
  local cmd="$1"
  command -v "$cmd" >/dev/null 2>&1 || die "missing required command: $cmd"
}

resolve_default_nif_path() {
  case "$(uname -s)" in
    Linux)
      printf '%s\n' "$REPO_ROOT/target/debug/libpantograph_rustler.so"
      ;;
    Darwin)
      printf '%s\n' "$REPO_ROOT/target/debug/libpantograph_rustler.dylib"
      ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
      printf '%s\n' "$REPO_ROOT/target/debug/pantograph_rustler.dll"
      ;;
    *)
      die "unsupported OS for default Rustler NIF path resolution: $(uname -s)"
      ;;
  esac
}

main() {
  require_command cargo
  require_command mix
  require_command elixir
  require_command erl

  log "building pantograph_rustler debug NIF artifact"
  cargo build -p pantograph_rustler

  local nif_path
  nif_path="${PANTOGRAPH_RUSTLER_NIF_PATH:-$(resolve_default_nif_path)}"

  [[ -f "$nif_path" ]] || die "expected NIF artifact not found at $nif_path"
  [[ -d "$HARNESS_DIR" ]] || die "BEAM smoke harness not found at $HARNESS_DIR"

  log "running Mix smoke harness against $nif_path"
  (
    cd "$HARNESS_DIR"
    PANTOGRAPH_RUSTLER_NIF_PATH="$nif_path" mix test
  )

  log "PASS: BEAM smoke harness loaded pantograph_rustler"
}

main "$@"
