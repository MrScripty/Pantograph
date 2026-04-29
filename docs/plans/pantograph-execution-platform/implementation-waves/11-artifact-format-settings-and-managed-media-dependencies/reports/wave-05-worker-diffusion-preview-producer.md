# Wave 05 Worker Diffusion Preview Producer

## Scope

Implemented a bounded producer-side diffusion preview emission slice in the
Python bridge and PyTorch worker only.

## Changes

- Threaded `emit_stream` through the embedded Python bridge for
  `diffusion-inference` nodes.
- Added best-effort diffusion step callback attachment in the PyTorch worker.
- Supports the newer diffusers `callback_on_step_end` path when the pipeline
  signature advertises it, with `latents` requested only when the pipeline
  exposes that callback tensor input.
- Supports the older `callback` / `callback_steps` path when advertised by the
  pipeline signature.
- Emits at most eight preview events per generation.
- Preview stream chunks use bounded metadata:
  - `type: "diffusion_preview"`
  - `image_base64`
  - `media_type`
  - `sequence`
  - `revision_index`
  - `step`
  - `total_steps`
  - `preview_role: "revision"`
  - `artifact_role: "diffusion_preview"`
  - `is_final: false`

Final diffusion outputs are unchanged and still return the final image payload
from the synchronous generation result.

## Preview Extraction

Preview image extraction is intentionally best-effort. The worker emits only
when the callback payload can be converted safely without changing the final
generation path:

- PIL image payloads in callback kwargs are encoded directly.
- Latent tensor payloads are decoded only when the loaded pipeline exposes both
  `vae.decode` and `image_processor.postprocess`.
- Callback extraction and stream emission failures are swallowed so preview
  support cannot fail final image generation.

Pipelines that support step callbacks but do not expose compatible image or
latent callback data will produce no preview chunks in this slice.

## Verification

- `python3 -m py_compile crates/pantograph-embedded-runtime/src/python_runtime_bridge.py crates/inference/torch/worker.py`
- Direct bridge module import smoke passed.

The local environment does not have `numpy` installed, so a direct torch worker
module import smoke could not run here. No existing focused Python test harness
was present for these files.
