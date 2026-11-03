#!/usr/bin/env bash
# Run from repository root using its ordinary supported toolchain and qualified
# CPU Python environment. This script does not install dependencies or weights.
set -euo pipefail
export ORT_SKIP_DOWNLOAD=1 HF_HUB_OFFLINE=1 TRANSFORMERS_OFFLINE=1
chat_python_lib="$(python -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR"))')"
export LD_LIBRARY_PATH="$chat_python_lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
chat_scope=(-p inference -p pantograph-embedded-runtime -p pantograph-inference-interface-contracts -p pantograph-runtime-host-contracts -p pantograph-workflow-service --features inference/backend-pytorch,inference/backend-candle,inference/std-process,pantograph-embedded-runtime/backend-pytorch)
chat_features="$(mktemp)"
trap 'rm -f "$chat_features"' EXIT
cargo tree --locked --offline --target all --edges features "${chat_scope[@]}" > "$chat_features"
if rg -q 'download-binaries|Pumas-Library.*2243a2b' "$chat_features"; then
  echo 'Forbidden dependency or build-download feature is effective' >&2
  exit 1
fi
rg -q 'ort feature "load-dynamic"' "$chat_features"
rg -q 'ort-sys feature "disable-linking"' "$chat_features"
cat "$chat_features"
python crates/inference/tests/fixtures/tiny_chat_gpt2/qualify.py
python -m unittest discover -s crates/inference/torch/tests -v
cargo test --locked --offline "${chat_scope[@]}"
cargo test --locked --offline "${chat_scope[@]}" saved_cpu_chat_graph_runs_native_owner_with_template_controls_and_scope_isolation -- --ignored --nocapture
cargo clippy --locked --offline "${chat_scope[@]}" --all-targets -- -D warnings
cargo fmt --all -- --check
npm run test:frontend
npm run typecheck
npm run build
# Select explicit range refs or TRACEABILITY_STAGED_ONLY=1 for this gate.
npm run lint:no-new
./scripts/check-scheduler-only-workflow-execution.sh
