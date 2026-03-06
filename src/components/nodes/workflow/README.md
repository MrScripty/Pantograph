# src/components/nodes/workflow

## Purpose
Workflow node components render the Pantograph-specific UI for dataflow nodes that
appear on the workflow canvas. This directory exists so node rendering,
node-local interaction logic, and workflow-specific presentation rules stay close
to the workflow graph runtime instead of being spread across generic canvas code.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `AudioOutputNode.svelte` | Renders playback controls for streamed and final audio outputs, including rerun cleanup of execution-local playback state. |
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

## Decision
Keep node-specific runtime behavior inside the component that owns the UI, but
drive run-boundary resets from shared workflow state. `AudioOutputNode.svelte`
therefore handles playback resources locally while relying on run-start store
cleanup to clear execution-local audio fields between workflow runs.

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
