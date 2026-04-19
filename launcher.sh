#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

SCRIPT_NAME="$(basename "$0")"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

APP_BIN="pantograph"
RELEASE_BIN_CANDIDATES=(
  "./target/release/${APP_BIN}"
  "./target/release/${APP_BIN}.exe"
  "./src-tauri/target/release/${APP_BIN}"
  "./src-tauri/target/release/${APP_BIN}.exe"
)
VENV_DIR="$ROOT_DIR/.venv"
VENV_PYTHON="$VENV_DIR/bin/python3"

EXIT_SUCCESS=0
EXIT_OPERATION_FAILED=1
EXIT_USAGE_ERROR=2
EXIT_MISSING_DEP=3
EXIT_MISSING_RELEASE_ARTIFACT=4

INSTALL_DEPENDENCIES=("npm" "cargo" "python3" "node_modules" "venv" "python_base_requirements" "python_diffusion_requirements")
RUNTIME_DEPENDENCIES=("npm" "cargo" "python3" "node_modules" "venv")

# Raise file descriptor limits for local dev watchers where possible.
ulimit -n 65536 2>/dev/null || ulimit -n 16384 2>/dev/null || ulimit -n 4096 2>/dev/null || true

usage() {
  cat <<EOF
Pantograph launcher for install, build, and runtime operations.

Usage:
  ./${SCRIPT_NAME} --help
  ./${SCRIPT_NAME} --install
  ./${SCRIPT_NAME} --build
  ./${SCRIPT_NAME} --build-release
  ./${SCRIPT_NAME} --release-smoke
  ./${SCRIPT_NAME} --run [-- <app args...>]
  ./${SCRIPT_NAME} --run-release [-- <app args...>]

Required action flags (choose exactly one):
  --run            Run the desktop app in development mode
  --run-release    Run the release binary artifact directly
  --build          Build development artifacts
  --build-release  Build release artifacts
  --release-smoke  Run the bounded redistributables smoke against a built release artifact
  --install        Install/verify dependencies
  --help           Print this help and exit

Examples:
  ./${SCRIPT_NAME} --install
  ./${SCRIPT_NAME} --build
  ./${SCRIPT_NAME} --build-release
  ./${SCRIPT_NAME} --release-smoke
  ./${SCRIPT_NAME} --run
  ./${SCRIPT_NAME} --run -- --verbose
  ./${SCRIPT_NAME} --run-release
  ./${SCRIPT_NAME} --run-release -- --help

Exit codes:
  ${EXIT_SUCCESS} success
  ${EXIT_OPERATION_FAILED} operation failed
  ${EXIT_USAGE_ERROR} usage error
  ${EXIT_MISSING_DEP} missing dependency for runtime
  ${EXIT_MISSING_RELEASE_ARTIFACT} missing release artifact for --run-release
  130 interrupted
EOF
}

log() {
  printf '[launcher] %s\n' "$*"
}

die() {
  log "error: $*"
  exit "$EXIT_OPERATION_FAILED"
}

die_usage() {
  log "usage error: $*"
  usage
  exit "$EXIT_USAGE_ERROR"
}

check_requirements() {
  local req_file="$1"
  while IFS= read -r line; do
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    [[ -z "${line// /}" ]] && continue

    local pkg
    if [[ "$line" =~ ^https?:// ]]; then
      local filename="${line##*/}"
      pkg="${filename%%-*}"
    else
      pkg="${line%% @*}"
      pkg="${pkg%%[>=<\[]*}"
      pkg="${pkg// /}"
    fi
    case "$pkg" in
      Pillow) pkg="PIL" ;;
      *) pkg="${pkg//-/_}" ;;
    esac

    if ! "$VENV_PYTHON" -c "import $pkg" >/dev/null 2>&1; then
      return 1
    fi
  done < "$req_file"
  return 0
}

check_npm() { command -v npm >/dev/null 2>&1; }
install_npm() { die "npm is required. Install Node.js/npm, then rerun --install"; }

check_cargo() { command -v cargo >/dev/null 2>&1; }
install_cargo() { die "cargo is required. Install Rust toolchain, then rerun --install"; }

check_python3() { command -v python3 >/dev/null 2>&1; }
install_python3() { die "python3 is required. Install Python 3.10+, then rerun --install"; }

check_node_modules() { [[ -d "$ROOT_DIR/node_modules" ]]; }
install_node_modules() { npm install; }

check_venv() { [[ -x "$VENV_PYTHON" ]]; }
install_venv() {
  check_python3 || return 1
  python3 -m venv "$VENV_DIR"
  "$VENV_PYTHON" -m pip install --upgrade pip -q
}

check_python_base_requirements() {
  check_venv || return 1
  check_requirements "$ROOT_DIR/requirements.txt"
}
install_python_base_requirements() {
  check_venv || install_venv
  "$VENV_PYTHON" -m pip install -r "$ROOT_DIR/requirements.txt"
}

check_python_diffusion_requirements() {
  check_venv || return 1
  check_requirements "$ROOT_DIR/requirements-diffusion.txt"
}
install_python_diffusion_requirements() {
  check_venv || install_venv
  "$VENV_PYTHON" -m pip install -r "$ROOT_DIR/requirements-diffusion.txt"
}

check_dep() { "check_$1"; }
install_dep() { "install_$1"; }

install_dependencies() {
  local dep
  for dep in "${INSTALL_DEPENDENCIES[@]}"; do
    if check_dep "$dep"; then
      log "[ok] $dep already satisfied"
      continue
    fi

    log "[install] $dep missing; installing"
    if install_dep "$dep"; then
      if check_dep "$dep"; then
        log "[done] $dep installed"
      else
        log "[error] $dep install failed verification"
        exit "$EXIT_OPERATION_FAILED"
      fi
    else
      log "[error] $dep install failed"
      exit "$EXIT_OPERATION_FAILED"
    fi
  done
}

ensure_runtime_dependencies() {
  local dep
  for dep in "${RUNTIME_DEPENDENCIES[@]}"; do
    if ! check_dep "$dep"; then
      log "missing dependency: $dep"
      log "run ./${SCRIPT_NAME} --install first"
      exit "$EXIT_MISSING_DEP"
    fi
  done
}

activate_python_env() {
  if check_venv; then
    export PYO3_PYTHON="$VENV_PYTHON"
    if [[ -z "${PANTOGRAPH_PYTHON_EXECUTABLE:-}" ]]; then
      export PANTOGRAPH_PYTHON_EXECUTABLE="$VENV_PYTHON"
    fi
    export VIRTUAL_ENV="$VENV_DIR"
    export PATH="$VENV_DIR/bin:$PATH"
  fi
}

build_app() {
  local mode="$1"
  ensure_runtime_dependencies
  activate_python_env

  case "$mode" in
    dev)
      log "[build] building development artifacts"
      npm run build
      ;;
    release)
      log "[build] building release artifacts"
      npm run build:desktop
      ;;
    *)
      die_usage "invalid build mode: $mode"
      ;;
  esac
}

run_app() {
  ensure_runtime_dependencies
  activate_python_env
  log "[run] starting development runtime"
  if [[ ${#RUN_ARGS[@]} -gt 0 ]]; then
    exec npm run dev:desktop -- "${RUN_ARGS[@]}"
  fi
  exec npm run dev:desktop
}

run_release_app() {
  ensure_runtime_dependencies
  activate_python_env

  local release_bin=""
  if ! release_bin="$(resolve_release_artifact)"; then
    log "missing release artifact"
    log "expected one of:"
    local candidate=""
    for candidate in "${RELEASE_BIN_CANDIDATES[@]}"; do
      log "  $candidate"
    done
    log "run ./${SCRIPT_NAME} --build-release first"
    exit "$EXIT_MISSING_RELEASE_ARTIFACT"
  fi

  log "[run] starting release binary: $release_bin"
  if [[ ${#RUN_ARGS[@]} -gt 0 ]]; then
    exec "$release_bin" "${RUN_ARGS[@]}"
  fi
  exec "$release_bin"
}

resolve_release_artifact() {
  local candidate=""
  for candidate in "${RELEASE_BIN_CANDIDATES[@]}"; do
    if [[ -x "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  return 1
}

run_release_smoke() {
  ensure_runtime_dependencies
  activate_python_env

  local release_bin=""
  if ! release_bin="$(resolve_release_artifact)"; then
    log "missing release artifact"
    log "expected one of:"
    local candidate=""
    for candidate in "${RELEASE_BIN_CANDIDATES[@]}"; do
      log "  $candidate"
    done
    log "run ./${SCRIPT_NAME} --build-release first"
    exit "$EXIT_MISSING_RELEASE_ARTIFACT"
  fi

  log "[smoke] running runtime redistributables smoke against $release_bin"
  "$ROOT_DIR/scripts/check-runtime-redistributables-smoke.sh" "$release_bin"
}

ACTION=""
RUN_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --run|--run-release|--build|--build-release|--release-smoke|--install|--help)
      if [[ -n "$ACTION" ]]; then
        die_usage "exactly one action flag is allowed"
      fi
      ACTION="$1"
      shift
      ;;
    --)
      if [[ "$ACTION" != "--run" && "$ACTION" != "--run-release" ]]; then
        die_usage "'--' is only valid with --run or --run-release"
      fi
      shift
      RUN_ARGS=("$@")
      break
      ;;
    --*)
      die_usage "unknown flag: $1"
      ;;
    *)
      die_usage "positional arguments are not allowed: $1"
      ;;
  esac
done

if [[ -z "$ACTION" ]]; then
  die_usage "missing action flag"
fi

case "$ACTION" in
  --help)
    usage
    exit "$EXIT_SUCCESS"
    ;;
  --install)
    install_dependencies
    exit "$EXIT_SUCCESS"
    ;;
  --build)
    build_app dev
    ;;
  --build-release)
    build_app release
    ;;
  --release-smoke)
    run_release_smoke
    ;;
  --run)
    run_app
    ;;
  --run-release)
    run_release_app
    ;;
  *)
    die_usage "invalid action: $ACTION"
    ;;
esac
