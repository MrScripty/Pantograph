#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ -n "${PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH:-}" || -n "${PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH:-}" ]]; then
  cat >&2 <<'EOF'
PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH and PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH are retired for this smoke.

Use PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID and ensure the selected model is
available through the canonical Pumas/runtime planning path.
EOF
  exit 2
fi

if [[ -z "${PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID:-}" ]]; then
  cat >&2 <<'EOF'
Missing required environment variable: PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID

Set it to the Pumas model id selected by the Puma-Lib node. Example:

  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID=diffusion/cc-nms/tiny-sd-turbo \
  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID=diffusers \
  PANTOGRAPH_PYTHON_EXECUTABLE=.venv/bin/python \
  ./scripts/check-uniffi-csharp-diffusion-smoke.sh

The selected Python executable must be able to import the Pantograph diffusion
worker dependencies: torch, diffusers, transformers, accelerate, and Pillow.
EOF
  exit 2
fi

export PANTOGRAPH_CSHARP_SMOKE_MODE=diffusion
export PANTOGRAPH_DIFFUSION_SMOKE_OUTPUT="${PANTOGRAPH_DIFFUSION_SMOKE_OUTPUT:-$repo_root/target/csharp-runtime-smoke/diffusion-smoke.png}"

./scripts/check-uniffi-csharp-smoke.sh

echo "Verified generated C# direct diffusion smoke output: $PANTOGRAPH_DIFFUSION_SMOKE_OUTPUT"
