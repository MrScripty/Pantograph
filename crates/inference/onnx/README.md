# crates/inference/onnx

## Purpose
Python worker implementation for ONNX text-to-audio inference in Pantograph.
Current target model family is KittenTTS ONNX bundles.

## Runtime Contract
- Input: `model_path` (required), `prompt` (required), plus model-specific keys
  supplied from `inference_settings` (for example `voice`, `speed`,
  `clean_text`, `sample_rate`).
- Output:
  - `audio` (base64 WAV, complete clip)
  - `duration_seconds`
  - `sample_rate`
  - `stream` (array of audio stream chunks)
  - `voice_used`
  - `speed_used`

## Model Bundle Layout
- Accepts either:
  - Path to `*.onnx` file with sibling `voices.npz`
  - Directory containing `*.onnx` and `voices.npz`

## Stream Chunk Schema
- `type`: `audio_chunk`
- `mode`: `append` or `replace`
- `audio_base64`: base64 WAV payload for the chunk
- `duration_seconds`
- `sample_rate`
- `mime_type`: currently `audio/wav`
- `sequence`: monotonic chunk index
- `is_final`: true on terminal chunk

## Dependencies
Expected Python environment packages:
- `kittentts`
- `onnxruntime`
- `numpy`
- `phonemizer`
- `soundfile`

`phonemizer` may require OS-level speech backends (for example `espeak-ng`)
to be installed and visible in the selected dependency environment.
