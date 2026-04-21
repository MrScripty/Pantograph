# inference/torch

Python worker modules for PyTorch text and audio-capable model execution.

## Purpose
This directory contains Python modules loaded by the Rust inference crate
through PyO3. The boundary exists so model-specific PyTorch generation logic
can stay isolated from Rust orchestration while Rust remains the owner of
process/runtime lifecycle and public inference contracts.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `worker.py` | PyO3-facing worker entrypoints, loaded-model state, and dispatch into generation helpers. |
| `autoregressive.py` | Autoregressive HuggingFace generation and streaming helpers. |
| `block_diffusion.py` | dLLM, SDAR, TraDo, and masked block-diffusion generation helpers. |

## Problem
PyTorch generation paths require Python ecosystem libraries and model-specific
state that should not be mixed into Rust backend orchestration. At the same
time, these modules are source code consumed by Rust and need explicit
ownership rules.

## Constraints
- Rust owns public inference contracts and lifecycle entrypoints.
- Python module globals hold one loaded model state per worker.
- Sibling imports must work both from filesystem loading and PyO3 embedding.
- Worker responses must stay JSON/base64-safe for Rust callers.

## Decision
Keep PyTorch-specific generation in this Python worker directory. Rust callers
load `worker.py`, invoke stable functions, and treat helper modules as private
implementation details behind the worker contract.

## Alternatives Rejected
- Port all generation paths to Rust immediately: rejected because current
  supported models rely on Python/PyTorch ecosystem behavior.
- Let UI or Tauri call Python directly: rejected because inference lifecycle
  and error mapping must remain backend-owned.

## Invariants
- Public Python functions are called only through Rust/PyO3-owned boundaries.
- Module-level loaded state must be cleared by unload paths before switching
  model families.
- Generated audio and text payloads must remain serializable for Rust callers.
- Sibling helper modules must not own process lifecycle or backend selection.

## Revisit Triggers
- Rust-native inference replaces a model family.
- Worker state needs concurrent multi-model support.
- Python dependency packaging changes for release builds.

## Dependencies
**Internal:** Rust PyO3 loader paths in `crates/inference`.

**External:** `torch`, `transformers`, `numpy`, `soundfile`, and model-family
specific Python dependencies.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
The worker is loaded by Rust; direct Python execution is not the supported
product path:

```python
load_model("/path/to/model", device="auto")
generate("hello", max_tokens=64)
```

## API Consumer Contract
- Inputs: model paths, device labels, prompts, generation options, and encoded
  payloads supplied by Rust.
- Outputs: JSON-serializable text/audio results, streaming chunks, and loaded
  model metadata.
- Lifecycle: Rust loads the module, calls load/generate/unload functions, and
  owns when workers are created or dropped.
- Errors: Python exceptions cross the PyO3 boundary and must remain actionable
  for Rust error mapping.
- Versioning: function names and return shapes are Rust-consumed contracts.

## Structured Producer Contract
- Stable fields: returned dictionaries, base64 media fields, and streaming
  chunk keys are machine-consumed by Rust.
- Defaults: worker defaults for device and generation parameters must stay
  documented or be overridden by Rust before use.
- Enums and labels: device labels and model type labels are semantic inputs.
- Ordering: streaming chunks must preserve generation order.
- Compatibility: response-shape changes require Rust caller and test updates.
- Regeneration/migration: update Rust embedding code, Python packaging, and
  this README together when worker contracts change.

## Testing
```bash
cargo test -p inference
```

## Notes
- `__pycache__/` directories are generated Python cache output and must remain
  ignored by Git.
