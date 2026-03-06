# src/components/nodes/workflow

## Purpose
Svelte UI components for the frontend experience, organized by feature and composition boundaries.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| AgentToolsNode.svelte | Source file used by modules in this directory. |
| AudioGenerationNode.svelte | Source file used by modules in this directory. |
| AudioInputNode.svelte | Source file used by modules in this directory. |
| AudioOutputNode.svelte | Source file used by modules in this directory. |
| DepthEstimationNode.svelte | Source file used by modules in this directory. |
| DiffusionInferenceNode.svelte | Source file used by modules in this directory. |
| ExpandSettingsNode.svelte | Source file used by modules in this directory. |
| GenericNode.svelte | Source file used by modules in this directory. |
| ImageOutputNode.svelte | Source file used by modules in this directory. |
| LLMInferenceNode.svelte | Source file used by modules in this directory. |
| LinkedInputNode.svelte | Source file used by modules in this directory. |
| LlamaCppInferenceNode.svelte | Source file used by modules in this directory. |
| MaskedTextInputNode.svelte | Source file used by modules in this directory. |
| ModelProviderNode.svelte | Source file used by modules in this directory. |
| NodeGroupNode.svelte | Source file used by modules in this directory. |
| OnnxInferenceNode.svelte | Source file used by modules in this directory. |
| OllamaInferenceNode.svelte | Source file used by modules in this directory. |
| PointCloudOutputNode.svelte | Source file used by modules in this directory. |
| PumaLibNode.svelte | Source file used by modules in this directory. |
| PyTorchInferenceNode.svelte | Source file used by modules in this directory. |
| TextInputNode.svelte | Source file used by modules in this directory. |
| TextOutputNode.svelte | Source file used by modules in this directory. |

## Design Decisions
- Keep files in this directory scoped to a single responsibility boundary.
- Prefer explicit module boundaries over cross-cutting utility placement.
- Maintain predictable naming so callers can discover related modules quickly.
- `AudioOutputNode.svelte` supports buffered stream playback for low-latency
  `audio_stream` chunks while preserving final-audio seek/volume controls.

## Dependencies
**Internal:** Neighboring modules in this source tree and the nearest package/crate entry points.
**External:** Dependencies declared in the corresponding manifest files.

## Usage Examples
```ts
// Example: import API from this directory.
import { value } from './module';
```
