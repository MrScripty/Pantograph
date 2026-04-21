# inference/depth

Python worker modules for depth-estimation inference.

## Purpose
This directory contains the DepthPro Python worker loaded by Rust through PyO3.
It isolates depth-estimation model loading and image processing from Rust
backend orchestration while preserving a stable Rust-consumed worker contract.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `worker.py` | PyO3-facing depth worker entrypoints and loaded-model state. |
| `depth_estimation.py` | DepthPro model loading and depth-map generation helpers. |

## Problem
Depth estimation depends on Python model libraries and image-processing
pipelines. Mixing that logic into Rust would increase backend complexity, while
letting adapters call Python directly would bypass backend-owned inference
lifecycle.

## Constraints
- Rust owns public inference contracts, worker loading, and error projection.
- Python globals hold one loaded depth model at a time.
- Inputs and outputs must remain JSON/base64-safe.
- Worker imports must work when loaded from disk by PyO3.

## Decision
Keep depth-specific Python implementation here behind `worker.py`. Rust treats
the worker as an internal implementation detail of the inference crate.

## Alternatives Rejected
- Put depth estimation in the desktop app layer: rejected because inference is
  a backend capability and should be reusable by non-Tauri hosts.
- Merge depth helpers into the PyTorch text worker: rejected because model
  lifecycle and input/output contracts differ.

## Invariants
- `worker.py` remains the Rust-facing module.
- Loaded model metadata must reflect the active model path and device.
- Depth outputs remain encoded and structured for Rust callers.
- Python helper modules do not own runtime selection or process lifecycle.

## Revisit Triggers
- Depth inference moves to a Rust-native or sidecar process implementation.
- Multi-model depth loading is required.
- Output formats expand beyond current depth map and point-cloud payloads.

## Dependencies
**Internal:** Rust PyO3 loader paths in `crates/inference`.

**External:** `torch`, DepthPro/model libraries, and image-processing
dependencies used by `depth_estimation.py`.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`

## Usage Examples
The worker is loaded by Rust; direct Python execution is not the supported
product path:

```python
load_model("/path/to/depth-model", device="auto")
estimate_depth(image_base64)
```

## API Consumer Contract
- Inputs: model path, device label, and base64-encoded image payload.
- Outputs: JSON-serializable model metadata and depth-estimation results.
- Lifecycle: Rust loads, invokes, and unloads the worker module.
- Errors: missing model state and model-library failures propagate as Python
  exceptions for Rust mapping.
- Versioning: worker function names and return keys are Rust-consumed
  contracts.

## Structured Producer Contract
- Stable fields: depth result dictionaries and encoded media fields are
  machine-consumed by Rust.
- Defaults: `device="auto"` selects CUDA, MPS, then CPU in worker order.
- Enums and labels: device labels are semantic inputs.
- Ordering: point-cloud/depth arrays must preserve helper-defined ordering.
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
