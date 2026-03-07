# src/components/nodes/workflow

## Purpose
Workflow node components render the Pantograph-specific UI for dataflow nodes that
appear on the workflow canvas. This directory exists so node rendering,
node-local interaction logic, and workflow-specific presentation rules stay close
to the workflow graph runtime instead of being spread across generic canvas code.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `BooleanInputNode.svelte` | Renders a metadata-driven boolean editor that can bind to any downstream boolean-compatible setting. |
| `AudioOutputNode.svelte` | Renders playback controls for streamed and final audio outputs, including rerun cleanup of execution-local playback state. |
| `audioOutputState.ts` | Defines the execution-local audio runtime keys and helper logic that maps backend completion metadata into output-node playback state. |
| `NumberInputNode.svelte` | Renders a metadata-driven numeric editor that adopts downstream default values and range constraints. |
| `primitiveInputMetadata.ts` | Shared helpers that resolve downstream port metadata and normalize primitive editor defaults. |
| `TextOutputNode.svelte` | Displays terminal text values and streaming text updates from workflow execution. |
| `AudioInputNode.svelte` | Captures user-selected audio files and writes stable input data into node configuration. |
| `AudioGenerationNode.svelte` | Shows execution and dependency status for Stable Audio generation nodes. |
| `GenericNode.svelte` | Fallback renderer for workflow node types that do not need specialized UI. |

## Problem
Workflow execution mixes durable node configuration with transient runtime data
such as stream chunks, progress, and terminal outputs. These components must
render that runtime state without leaking execution-local UI state across reruns
or requiring the whole workflow view to remount.

## Constraints
- Node components run inside a draggable, pannable graph canvas, so embedded
  controls must not accidentally trigger graph gestures.
- Runtime updates arrive through workflow events and store mutations; components
  must react to those updates declaratively instead of polling.
- Audio playback must support both low-latency stream playback and final-audio
  controls while cleaning up timers and `AudioContext` resources deterministically.
- Final generated audio may arrive before browser metadata resolves, so the UI
  must honor backend-provided duration metadata instead of relying only on
  `HTMLAudioElement.duration`.
- Stable Audio generation is batch-only in the current runtime, so the UI must
  not imply that its output will arrive as playable stream chunks mid-generation.

## Decision
Keep node-specific runtime behavior inside the component that owns the UI, but
drive run-boundary resets from shared workflow state. `AudioOutputNode.svelte`
therefore handles playback resources locally while relying on run-start store
cleanup to clear execution-local audio fields between workflow runs. Final audio
duration is treated as a produced runtime contract (`audio_duration_seconds`)
that the toolbar forwards from node outputs into the output node so scrub/replay
controls do not depend solely on browser metadata timing. `AudioGenerationNode`
also surfaces the batch-only capability boundary so users can distinguish Stable
Audio final renders from ONNX-backed live chunk playback.

## Alternatives Rejected
- Reset audio output state only by remounting the workflow view.
  Rejected because reruns in the same workflow would remain broken and cleanup
  would depend on incidental navigation behavior.
- Move all playback state into a global store.
  Rejected because browser audio resources and DOM playback controls are owned by
  the component instance and are simpler to manage there.

## Invariants
- Node configuration entered by the user must survive reruns; only execution-local
  audio state may be cleared automatically.
- `AudioOutputNode.svelte` must stop timers and close buffered stream playback
  resources on rerun reset and component teardown.
- Final-audio controls such as seek, replay, and loop remain tied to final audio
  payloads, not transient stream chunks.
- Workflow completion handlers must forward final audio metadata together with
  the audio payload so output playback stays seekable even when metadata loading
  lags in the browser.

## Revisit Triggers
- Another output node needs the same rerun-reset pattern and the logic starts to
  duplicate across components.
- Workflow events gain execution identifiers, allowing stale-event rejection to
  move out of the component layer.
- Product requirements change so streamed audio must also support seekable replay.

## Dependencies
**Internal:** `src/stores/workflowStore.ts`, `src/components/nodes/BaseNode.svelte`,
workflow event handling in `src/components/WorkflowToolbar.svelte`.

**External:** Svelte 5 runes, browser audio APIs (`HTMLAudioElement`,
`AudioContext`), and `@xyflow/svelte` through the surrounding graph renderer.

## Related ADRs
- None.
- Reason: no ADR currently records node-level runtime ownership for workflow
  output components.
- Revisit trigger: this directory takes on broader cross-layer execution or
  contract responsibilities.

## Usage Examples
```ts
import AudioOutputNode from '../components/nodes/workflow/AudioOutputNode.svelte';

const nodeTypes = {
  'audio-output': AudioOutputNode,
};
```

## API Consumer Contract (Host-Facing Modules)
None.
Reason: these components are internal frontend renderers, not a host-facing API
or cross-process boundary.
Revisit trigger: a plugin or extension surface begins consuming these node
components directly.

## Structured Producer Contract (Machine-Consumed Modules)
None.
Reason: this directory consumes workflow runtime data but does not define a
persisted machine-readable artifact format of its own.
Revisit trigger: components in this directory start generating saved metadata,
templates, or structured payloads consumed elsewhere.
