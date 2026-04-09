#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ -n "${PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH:-}" && -z "${PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH:-}" ]]; then
  echo "PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH is deprecated for this smoke." >&2
  echo "Use PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH so it is explicit that the path populates the Puma-Lib node." >&2
  export PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH="$PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH"
fi

if [[ -z "${PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH:-}" ]]; then
  cat >&2 <<'EOF'
Missing required environment variable: PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH

Set it to the same local diffusers-style model entry path that a selected
Puma-Lib node would emit on its model_path port. Example:

  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH=/models/tiny-sd-turbo \
  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID=diffusion/cc-nms/tiny-sd-turbo \
  PANTOGRAPH_PYTHON_EXECUTABLE=.venv/bin/python \
  ./scripts/check-uniffi-csharp-diffusion-smoke.sh

The selected Python executable must be able to import the Pantograph diffusion
worker dependencies: torch, diffusers, transformers, accelerate, and Pillow.
EOF
  exit 2
fi

if [[ ! -e "$PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH" ]]; then
  echo "PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH does not exist: $PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH" >&2
  exit 2
fi

export PANTOGRAPH_CSHARP_SMOKE_MODE=diffusion
export PANTOGRAPH_DIFFUSION_SMOKE_OUTPUT="${PANTOGRAPH_DIFFUSION_SMOKE_OUTPUT:-$repo_root/target/csharp-runtime-smoke/diffusion-smoke.png}"

./scripts/check-uniffi-csharp-smoke.sh

echo "Verified generated C# direct diffusion smoke output: $PANTOGRAPH_DIFFUSION_SMOKE_OUTPUT"
