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
echo "[runtime-redistributables-smoke] running headless release contract smoke for managed runtimes, current image workflow shape, Pumas resolution, stale diagnostics, and image artifact retention."

cargo test -p pantograph-embedded-runtime managed_runtime_manager::tests::manager_list_projects_install_history_and_selection -- --exact
cargo test --manifest-path src-tauri/Cargo.toml workflow::diagnostics::tests::runtime_snapshot_preserves_managed_runtime_views -- --exact
node "$repo_root/scripts/check-current-image-workflow-smoke.mjs"
cargo test -p workflow-nodes test_inventory_collects_all_builtins --lib
cargo test -p pantograph-embedded-runtime puma_lib_execution_rebinds_stale_model_path_from_selector_access_without_pumas_api --lib
cargo test -p pantograph-workflow-service inspection_projection_returns_stable_stale_graph_diagnostics --lib
cargo test -p pantograph-workflow-service workflow_io_artifact_query_reads_refreshed_projection --lib

echo "[runtime-redistributables-smoke] release contract smoke passed"
