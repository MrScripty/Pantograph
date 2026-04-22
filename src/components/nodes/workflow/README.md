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
| `DiffusionInferenceNode.svelte` | Shows execution and dependency state for process-backed diffusion image generation. |
| `DependencyEnvironmentActivityLog.svelte` | Renders the dependency environment activity log and owns log auto-scroll behavior. |
| `DependencyEnvironmentBindingsPanel.svelte` | Renders binding selection, manual override summary controls, and per-requirement override fields. |
| `DependencyEnvironmentModeControls.svelte` | Renders the dependency environment automatic/manual mode selector. |
| `DependencyEnvironmentNode.svelte` | Presents dependency resolution, check, install, activity, and override controls for model-backed environment setup. |
| `DependencyEnvironmentRefPanel.svelte` | Renders the resolved dependency environment reference state, environment id, and Python executable. |
| `DependencyEnvironmentStatusPanel.svelte` | Renders dependency state badges, status messages, and command buttons for dependency actions. |
| `dependencyEnvironmentActions.ts` | Builds backend action payloads and wraps dependency action execution from upstream model state and dependency override selections. |
| `dependencyEnvironmentActivityListener.ts` | Wires dependency activity events, initial persistence, and automatic mode startup for the dependency environment node. |
| `dependencyEnvironmentDisplay.ts` | Formats dependency badges, backend codes, and activity-log events. |
| `dependencyEnvironmentNodeState.ts` | Projects dependency environment node-local state, persistence payloads, action responses, and retained activity logs. |
| `dependencyEnvironmentOverrides.ts` | Parses, merges, looks up, reads, clears, summarizes, and mutates dependency override patches. |
| `dependencyEnvironmentSelection.ts` | Filters and toggles dependency binding selection state for the environment node UI. |
| `dependencyEnvironmentSources.ts` | Resolves connected upstream model, requirement, and manual override inputs from workflow graph state. |
| `dependencyEnvironmentState.ts` | Re-exports the dependency environment helper modules for stable component and test imports. |
| `dependencyEnvironmentState.test.ts` | Unit coverage for dependency environment override parsing, merge, lookup, and label helpers. |
| `dependencyEnvironmentTypes.ts` | Defines dependency environment frontend contracts, node props, and node data shapes that mirror backend payloads. |
| `ExpandSettingsNode.svelte` | Displays the effective passthrough value for each model-derived inference setting while the shared base node renders matching override input/output handles from dynamic port metadata. |
| `expandSettingsDisplay.ts` | Resolves the effective visible expand-setting value from live connected overrides, runtime passthrough data, and schema defaults. |
| `audioOutputState.ts` | Defines the execution-local audio runtime keys and helper logic that maps backend completion metadata into output-node playback state. |
| `NumberInputNode.svelte` | Renders a metadata-driven numeric editor that adopts downstream default values and range constraints. |
| `PumaLibNode.svelte` | Presents model-library selection and routes model metadata into the correct downstream inference node type. |
| `primitiveInputMetadata.ts` | Shared helpers that resolve downstream port metadata and normalize primitive editor defaults. |
| `TextOutputNode.svelte` | Displays terminal text values and streaming text updates from workflow execution. |
| `AudioInputNode.svelte` | Captures user-selected audio files and writes stable input data into node configuration. |
| `AudioGenerationNode.svelte` | Shows execution and dependency status for Stable Audio generation nodes. |
| `RerankerNode.svelte` | Presents query, candidate-document, and ranked-output state for GGUF reranker execution. |
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
- Model-derived port metadata arrives from backend-owned graph state, so
  workflow node components must render additive handles from `data.definition`
  rather than inventing their own durable port lists.
- Audio playback must support both low-latency stream playback and final-audio
  controls while cleaning up timers and `AudioContext` resources deterministically.
- Final generated audio may arrive before browser metadata resolves, so the UI
  must honor backend-provided duration metadata instead of relying only on
  `HTMLAudioElement.duration`.
- Stable Audio generation is batch-only in the current runtime, so the UI must
  not imply that its output will arrive as playable stream chunks mid-generation.
- Embedded node controls must remain labelled and graph-safe: icon-only or
  image-only buttons need accessible names, and pointer handlers must not leak
  canvas drag/pan gestures.

## Decision
Keep node-specific runtime behavior inside the component that owns the UI, but
drive run-boundary resets from shared workflow state. `AudioOutputNode.svelte`
therefore handles playback resources locally while relying on run-start store
cleanup to clear execution-local audio fields between workflow runs. Final audio
duration is treated as a produced runtime contract (`audio_duration_seconds`)
that the toolbar forwards from node outputs into the output node so scrub/replay
controls do not depend solely on browser metadata timing. `AudioGenerationNode`
also surfaces the batch-only capability boundary so users can distinguish Stable
Audio final renders from ONNX-backed live chunk playback. `PumaLibNode.svelte`
also owns the UI-side routing hints that send diffusion models to
`diffusion-inference` and reranker models to the dedicated reranker node
instead of the text-only PyTorch or llama.cpp generation nodes.
`ExpandSettingsNode.svelte` stays presentation-only: it shows schema details and
the effective value currently flowing through each setting, while
override-capable handles come from the shared node definition supplied by the
workflow stores.
`DependencyEnvironmentNode.svelte` keeps UI state and backend actions in the
component, while dependency contracts and pure override state helpers live in
`dependencyEnvironmentTypes.ts`, `dependencyEnvironmentActions.ts`,
`dependencyEnvironmentActivityListener.ts`, `dependencyEnvironmentNodeState.ts`,
`dependencyEnvironmentOverrides.ts`, `dependencyEnvironmentSelection.ts`,
`dependencyEnvironmentDisplay.ts`, and `dependencyEnvironmentSources.ts` so
payload projection, node prop/data contracts, node-local state projection,
upstream requirement adoption, backend action execution bracketing, mount-time
activity listener setup, graph-input projection, binding selection, override
reads and scope clears, parsing, merge, and formatting behavior can be tested
without mounting the node.
`dependencyEnvironmentState.ts` remains as a stable re-export surface for
component and test imports.
The activity log panel lives in `DependencyEnvironmentActivityLog.svelte` so
scroll handling and copyable log styling stay separate from dependency action
state.
The dependency action status panel lives in
`DependencyEnvironmentStatusPanel.svelte`, while the parent keeps backend action
dispatch and persistence ownership.
The resolved environment reference display lives in
`DependencyEnvironmentRefPanel.svelte`.
Binding selection and structured override form controls live in
`DependencyEnvironmentBindingsPanel.svelte`, while the parent owns state
mutation callbacks and persistence.
The automatic/manual mode selector lives in
`DependencyEnvironmentModeControls.svelte`, keeping mode rendering separate from
node persistence.

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
- Specialized node components must mirror canonical backend-owned port names so
  template graphs and execution bindings do not depend on UI-local aliases.
- `ExpandSettingsNode.svelte` must not hardcode durable override handles; it
  renders whatever additive inputs/outputs arrive in the backend-owned node
  definition.
- `ExpandSettingsNode.svelte` must display the connected override value when one
  is available, otherwise the last runtime passthrough value or schema default.
- `DependencyEnvironmentNode.svelte` must keep dependency override parsing and
  merge semantics aligned with the backend patch contract in
  `dependencyEnvironmentOverrides.ts`.
- `dependencyEnvironmentOverrides.ts` owns displayed override values, scope
  clears, summary counts, and local override checks; the Svelte component must
  only assign returned patch arrays and persist state.
- `dependencyEnvironmentSelection.ts` owns binding filtering and selection
  toggles; the Svelte component must not duplicate selected-binding rules
  inline.
- `dependencyEnvironmentNodeState.ts` owns dependency node persistence payloads,
  backend action response projection, and upstream requirement adoption; the
  Svelte component must not duplicate that state-shape mapping inline.
- `DependencyEnvironmentActivityLog.svelte` owns log auto-scroll behavior and
  must not trigger graph drag or pan gestures.
- `dependencyEnvironmentActivityListener.ts` owns dependency activity event
  listener setup, initial persistence, automatic mode startup, and listener
  failure log formatting.
- `dependencyEnvironmentActions.ts` owns action payload construction, busy-state
  bracketing, backend response application, and failure log formatting for
  dependency action commands.
- `DependencyEnvironmentStatusPanel.svelte` emits command callbacks without
  invoking backend APIs directly.
- `DependencyEnvironmentBindingsPanel.svelte` emits form and selection callbacks
  without writing node data directly.
- `DependencyEnvironmentModeControls.svelte` emits mode changes without writing
  node data directly.
- Image and media preview controls must expose accessible names even when the
  visible content is an image or icon rather than text.

## Revisit Triggers
- Another output node needs the same rerun-reset pattern and the logic starts to
  duplicate across components.
- Workflow events gain execution identifiers, allowing stale-event rejection to
  move out of the component layer.
- Product requirements change so streamed audio must also support seekable replay.
- More inference-family nodes need shared execution-status rendering and the
  specialized node components start repeating the same state layout.

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
