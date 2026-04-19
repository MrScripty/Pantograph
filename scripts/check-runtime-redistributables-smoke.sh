#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

release_bin="${1:-${PANTOGRAPH_RELEASE_BINARY:-}}"

if [[ -z "$release_bin" ]]; then
  for candidate in \
    "./target/release/pantograph" \
    "./target/release/pantograph.exe" \
    "./src-tauri/target/release/pantograph" \
    "./src-tauri/target/release/pantograph.exe"; do
    if [[ -x "$candidate" ]]; then
      release_bin="$candidate"
      break
    fi
  done
fi

if [[ -z "$release_bin" || ! -x "$release_bin" ]]; then
  echo "[runtime-redistributables-smoke] missing release artifact; build with ./launcher.sh --build-release first" >&2
  exit 1
fi

echo "[runtime-redistributables-smoke] release artifact: $release_bin"
echo "[runtime-redistributables-smoke] note: Pantograph does not yet expose a headless desktop release-smoke entrypoint, so this smoke verifies the built artifact exists and then runs targeted managed-runtime contract tests."

cargo test -p pantograph-embedded-runtime managed_runtime_manager::tests::manager_list_projects_install_history_and_selection -- --exact
cargo test -p pantograph-embedded-runtime tests::workflow_preflight_blocks_interrupted_runtime_job_after_restart -- --exact
cargo test --manifest-path src-tauri/Cargo.toml workflow::diagnostics::tests::runtime_snapshot_preserves_managed_runtime_views -- --exact

echo "[runtime-redistributables-smoke] managed runtime contract smoke passed"
