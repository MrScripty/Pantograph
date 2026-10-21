#!/usr/bin/env bash
# Public scheduler integration using committed untrained weights; never T3.
set -euo pipefail
source_sha="${1:?usage: qualify-cpu-text-workflow.sh EXPECTED_SOURCE_SHA NEW_ABSOLUTE_EVIDENCE_ROOT}"
evidence_root="${2:?provide a new absolute evidence root}"
if [[ ! "$source_sha" =~ ^[0-9a-f]{40}$ ]] || [[ "$(git rev-parse HEAD)" != "$source_sha" ]]; then
  echo 'CPU text integration requires the exact requested source SHA' >&2
  exit 1
fi
if [[ -n "$(git status --porcelain --untracked-files=normal)" ]]; then
  echo "CPU text commit qualification requires a clean checkout" >&2
  exit 1
fi
if [[ "$evidence_root" != /* ]] || [[ -e "$evidence_root" ]]; then
  echo 'CPU text integration requires a new absolute evidence directory' >&2
  exit 1
fi
mkdir "$evidence_root"
git rev-parse HEAD > "$evidence_root/source-sha.txt"
git status --porcelain > "$evidence_root/source-status.txt"
sha256sum crates/pantograph-embedded-runtime/examples/cpu_text_generation_workflow_probe.rs \
  crates/pantograph-embedded-runtime/Cargo.toml scripts/qualify-cpu-text-workflow.sh \
  > "$evidence_root/source-sha256.txt"
export HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1 OMP_NUM_THREADS=2 MKL_NUM_THREADS=2
export PYTHONPATH="${PYTHONPATH:-}"
# PYO3_PYTHON must name the same prepared environment used by execution.
if [[ -n "${PYO3_PYTHON:-}" ]]; then
  python_site_dir="$("$PYO3_PYTHON" -c 'import sysconfig; print(sysconfig.get_path("purelib"))')"
  export PYTHONPATH="$python_site_dir${PYTHONPATH:+:$PYTHONPATH}"
fi
timeout 600 cargo build --locked -p pantograph-embedded-runtime \
  --features backend-pytorch,host-dependency-inventory \
  --example cpu_text_generation_workflow_probe 2>&1 | tee "$evidence_root/build.log"
target_root="${CARGO_TARGET_DIR:-target}"
for scenario in generate failure cancel; do
  probe_root="$evidence_root/$scenario"
  XDG_CONFIG_HOME="$probe_root/config" XDG_DATA_HOME="$probe_root/data" \
    XDG_CACHE_HOME="$probe_root/cache" PUMAS_REGISTRY_DB_PATH="$probe_root/registry.db" \
    timeout 240 "$target_root/debug/examples/cpu_text_generation_workflow_probe" \
    "$probe_root" --synthetic "$scenario" 2>&1 | tee "$evidence_root/$scenario.log"
done
