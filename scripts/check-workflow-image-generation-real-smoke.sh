#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ -n "${PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH:-}" || -n "${PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH:-}" ]]; then
  cat >&2 <<'EOF'
PANTOGRAPH_DIFFUSION_SMOKE_MODEL_PATH and PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_PATH are retired.

Use PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID and the canonical Pumas model
package/load-target path. This smoke intentionally refuses direct model paths.
EOF
  exit 2
fi

if [[ -z "${PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID:-}" ]]; then
  cat >&2 <<'EOF'
Missing required environment variable: PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID

This opt-in smoke requires a locally provisioned Pumas Diffusers model and a
Python executable that can import torch, diffusers, transformers, accelerate,
and Pillow. Example:

  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_MODEL_ID=diffusion/cc-nms/tiny-sd-turbo \
  PANTOGRAPH_DIFFUSION_SMOKE_PUMAS_ARTIFACT_ID=diffusers \
  PANTOGRAPH_PYTHON_EXECUTABLE=.venv/bin/python \
  ./scripts/check-workflow-image-generation-real-smoke.sh

The smoke runs the canonical workflow graph shape check, the generated
C#/native-runtime real Diffusers session smoke, the desktop Tauri
backend-pytorch build gate, and focused frontend command checks for editor
navigation/diagnostic preservation.
EOF
  exit 2
fi

node scripts/check-current-image-workflow-smoke.mjs
cargo check --manifest-path src-tauri/Cargo.toml --features backend-pytorch
./scripts/check-uniffi-csharp-diffusion-smoke.sh
npm run test:frontend -- workflowToolbarEvents WorkflowService.commands

echo "Verified real image-generation smoke prerequisites, desktop PyTorch build surface, runtime execution, artifact output, and workflow-editor command projections."
