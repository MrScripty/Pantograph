# inference/audio

Python worker modules for Stable Audio generation.

## Purpose
This directory contains the Stable Audio Python worker loaded by Rust through
PyO3. It isolates audio model loading and waveform generation while Rust keeps
ownership of inference lifecycle, request routing, and public contracts.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `worker.py` | PyO3-facing audio worker entrypoints and loaded-model state. |
| `stable_audio.py` | Stable Audio model loading and text-to-audio generation helpers. |

## Problem
Stable Audio generation depends on Python audio and diffusion libraries. The
implementation needs to stay close to those libraries without leaking Python
lifecycle or media encoding decisions into adapters.

## Constraints
- Rust owns public inference contracts and worker lifecycle.
- Python globals hold one loaded audio model at a time.
- Audio output must be encoded for Rust and host-language consumers.
- Duration, step, guidance, and seed defaults must remain intentional.

## Decision
Keep audio-specific Python implementation here behind `worker.py`. Rust loads
the worker as an internal inference implementation and projects results through
backend-owned DTOs.

## Alternatives Rejected
- Place audio generation in frontend code: rejected because model execution and
  media encoding are backend responsibilities.
- Merge audio generation into the text PyTorch worker: rejected because audio
  dependencies and output contracts differ.

## Invariants
- `worker.py` remains the Rust-facing module.
- Generated audio is returned as base64 WAV with sample-rate metadata.
- Worker defaults must not silently change release behavior.
- Helper modules do not own runtime selection, process spawning, or adapter
  response shapes.

## Revisit Triggers
- Audio generation moves to a sidecar service.
- More than one audio model must be loaded concurrently.
- Output format support expands beyond WAV/base64 payloads.

## Dependencies
**Internal:** Rust PyO3 loader paths in `crates/inference`.

**External:** `torch`, `torchaudio`, `stable_audio_tools`, and model-specific
audio dependencies.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
The worker is loaded by Rust; direct Python execution is not the supported
product path:

```python
load_model("/path/to/stable-audio-model", device="auto")
generate_audio_from_text("short piano phrase", duration=8.0)
```

## API Consumer Contract
- Inputs: model path, device label, text prompt, duration, step count,
  guidance scale, and seed.
- Outputs: JSON-serializable model metadata and base64 WAV audio result.
- Lifecycle: Rust loads, invokes, and unloads the worker module.
- Errors: missing model state and model-library failures propagate as Python
  exceptions for Rust mapping.
- Versioning: worker function names and return keys are Rust-consumed
  contracts.

## Structured Producer Contract
- Stable fields: `audio_base64`, `duration_seconds`, and `sample_rate` are
  machine-consumed by Rust and host projections.
- Defaults: generation defaults in `worker.py` are part of observable behavior
  unless Rust overrides them.
- Enums and labels: device labels are semantic inputs.
- Ordering: generated audio samples preserve model output order.
- Compatibility: output-shape changes require Rust caller and test updates.
- Regeneration/migration: update Rust embedding code, Python packaging, and
  this README together when worker contracts change.

## Testing
```bash
cargo test -p inference
```

## Notes
- `__pycache__/` directories are generated Python cache output and must remain
  ignored by Git.
