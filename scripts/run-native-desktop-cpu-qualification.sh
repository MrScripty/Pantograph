#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
export ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1
export PANTOGRAPH_NATIVE_CPU_EVIDENCE_DIR="${PANTOGRAPH_NATIVE_CPU_EVIDENCE_DIR:?isolated evidence directory is required}"
python3 scripts/check-native-desktop-cpu-prerequisites.py
fixture_parent="$(mktemp -d "${TMPDIR:-/tmp}/pantograph-native-cpu.XXXXXXXX")"
cleanup() { rm -rf "$fixture_parent"; }
trap cleanup EXIT
export PANTOGRAPH_NATIVE_CPU_FIXTURE_ROOT="$fixture_parent/fixture"
python3 scripts/prepare-native-desktop-cpu-fixture.py "$PANTOGRAPH_NATIVE_CPU_FIXTURE_ROOT" > "$PANTOGRAPH_NATIVE_CPU_EVIDENCE_DIR/provisioning.json"
export PANTOGRAPH_GUI_SMOKE_PROJECT_ROOT="$PANTOGRAPH_NATIVE_CPU_FIXTURE_ROOT/project"
export PUMAS_LIBRARY_PATH="$PANTOGRAPH_NATIVE_CPU_FIXTURE_ROOT/synthetic-library"
export XDG_CONFIG_HOME="$fixture_parent/config"
export XDG_DATA_HOME="$fixture_parent/data"
export XDG_CACHE_HOME="$fixture_parent/cache"
# The display is cloud-owned; existing WebKit process sandboxing is retained.
xvfb-run -a node_modules/.bin/wdio run tests/e2e/native-desktop-cpu/wdio.conf.mjs
