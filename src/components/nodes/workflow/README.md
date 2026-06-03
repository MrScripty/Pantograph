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
| `AudioOutputNode.svelte` | Renders playback controls for streamed and final audio outputs, including ArtifactStore stream-reference preview reads, rerun cleanup of execution-local playback state, and explicit artifact format overrides. |
| `ImageOutputNode.svelte` | Renders image output previews and explicit artifact format overrides for image artifacts. |
| `PointCloudOutputNode.svelte` | Renders point-cloud previews and explicit artifact format overrides for 3D artifacts. |
| `DependencyEnvironmentActivityLog.svelte` | Renders the dependency environment activity log and owns log auto-scroll behavior. |
| `DependencyEnvironmentBindingsPanel.svelte` | Renders binding selection, manual override summary controls, and per-requirement override fields. |
| `DependencyEnvironmentModeControls.svelte` | Renders the dependency environment automatic/manual mode selector. |
| `DependencyEnvironmentNodeHeader.svelte` | Renders the dependency environment node icon and title label. |
| `DependencyEnvironmentNode.svelte` | Presents dependency resolution, check, install, activity, and override controls for model-backed environment setup. |
| `DependencyEnvironmentRefPanel.svelte` | Renders the resolved dependency environment reference state, environment id, and Python executable. |
| `DependencyEnvironmentStatusPanel.svelte` | Renders dependency state badges, status messages, and command buttons for dependency actions. |
| `dependencyEnvironmentActions.ts` | Builds backend action payloads and wraps dependency action execution from upstream model state and dependency override selections. |
| `dependencyEnvironmentActivityListener.ts` | Wires dependency activity events, initial persistence, and automatic mode startup for the dependency environment node. |
| `dependencyEnvironmentDisplay.ts` | Formats dependency badges, backend codes, activity timestamps, and activity-log events. |
| `dependencyEnvironmentNodeState.ts` | Projects dependency environment node-local state, persistence payloads, action responses, and retained activity logs. |
| `dependencyEnvironmentOverrides.ts` | Parses, merges, looks up, reads, clears, summarizes, mutates dependency override patches, and projects override form values. |
| `dependencyEnvironmentSelection.ts` | Filters and toggles dependency binding selection state for the environment node UI. |
| `dependencyEnvironmentSources.ts` | Resolves connected upstream model, requirement, and manual override inputs from workflow graph state. |
| `dependencyEnvironmentState.ts` | Re-exports the dependency environment helper modules for stable component and test imports. |
| `dependencyEnvironmentState.test.ts` | Unit coverage for dependency environment override parsing, merge, lookup, and label helpers. |
| `dependencyEnvironmentTypes.ts` | Defines dependency environment frontend contracts, node props, and node data shapes that mirror backend payloads. |
| `audioOutputState.ts` | Defines the execution-local audio runtime keys and helper logic that maps backend completion metadata into output-node playback state. |
| `NumberInputNode.svelte` | Renders a metadata-driven numeric editor that adopts downstream default values and range constraints. |
| `PumaLibNode.svelte` | Presents model-library selection and persists canonical `pumas_model_ref` identity for downstream inference planning. |
| `pumaLibNodeState.ts` | Projects selectable Pumas model options into path-free node data for `PumaLibNode.svelte`. |
| `pumaLibNodeState.test.ts` | Unit coverage for Puma-Lib model option projection and rejection of path-shaped option values. |
| `primitiveInputMetadata.ts` | Shared helpers that resolve downstream port metadata and normalize primitive editor defaults. |
| `selectionInputState.ts` | Shared selection-input state helpers, including provider-backed unset/stale presentation and static allowed-values default adoption. |
| `selectionInputProviderOptions.ts` | Builds backend-owned provider option queries for selection inputs and discards stale async option responses when target context changes. |
| `TextOutputNode.svelte` | Displays terminal text values and streaming text updates from workflow execution. |
| `AudioInputNode.svelte` | Captures user-selected audio files and writes stable input data into node configuration. |
| `GenericNode.svelte` | Fallback renderer for workflow node types that do not need specialized UI. |
| `inferencePayloadDisplay.ts` | Projects backend-neutral inference payload role metadata into compact task, diagnostics, usage, cache-handle, model-fact, and option display rows for canonical inference nodes. |
| `inferenceValidationDisplay.ts` | Projects backend validation, drift, update-proposal, and supported apply-action presentation for canonical inference nodes without owning graph patches. |

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
- The `dependency_environment_sidecar` port data type is a backend-owned
  association marker. Components may render it as a typed handle, but they must
  not use it to synthesize dependency requests, model paths, platform context,
  or scheduler admission state.
- Audio playback must support final-audio controls while cleaning up timers and
  `AudioContext` resources deterministically.
- Final generated audio may arrive before browser metadata resolves, so the UI
  must honor backend-provided duration metadata instead of relying only on
  `HTMLAudioElement.duration`.
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
controls do not depend solely on browser metadata timing.
`PumaLibNode.svelte` must hand model identity to canonical `llm-inference`
task shapes instead of routing packages to retired direct inference nodes.
The retired direct `AudioGenerationNode.svelte`, `OnnxInferenceNode.svelte`,
and `DepthEstimationNode.svelte` components have been removed. Audio,
ONNX-backed, and depth task families are exposed through backend-resolved
generic inference descriptors instead of specialized graph-visible inference
components.
The retired direct `PyTorchInferenceNode.svelte`, `LlamaCppInferenceNode.svelte`,
and `RerankerNode.svelte` components have been removed. Canonical inference
workflows render through `LLMInferenceNode.svelte` and backend descriptor
metadata instead of path-era direct runtime node components.
The retired `ExpandSettingsNode.svelte` component and display helper have been
removed. Model-specific inference options are rendered from backend descriptor
ports rather than frontend-owned settings expansion.
`LLMInferenceNode.svelte` displays canonical inference task and payload-role
facts through `inferencePayloadDisplay.ts`, which reads only backend-neutral
`inference_payloads` metadata from the node definition and does not render
backend keys, runtime ids, scheduler policy, raw paths, prompts, or result
bodies.
`PumaLibNode.svelte` is model-ref-only authoring UI. It may display a model id
or option label, but it must not persist `modelPath`, `model_path`, dependency
requirements, runtime hints, or package facts as graph data.
`DependencyEnvironmentNode.svelte` keeps UI state and emits dependency action
requests to the graph coordinator. It must not build backend dependency
requests, call Tauri directly, or derive graph session/revision context.
Dependency contracts and pure override state helpers live in
`dependencyEnvironmentTypes.ts`, `dependencyEnvironmentActions.ts`,
`dependencyEnvironmentActivityListener.ts`, `dependencyEnvironmentNodeState.ts`,
`dependencyEnvironmentOverrides.ts`, `dependencyEnvironmentSelection.ts`,
`dependencyEnvironmentDisplay.ts`, and `dependencyEnvironmentSources.ts` so
node prop/data contracts, node-local state projection, upstream requirement
adoption, backend action execution bracketing, mount-time activity listener
setup, graph-input projection, binding selection, override reads and scope
clears, override form value projection, parsing, merge, timestamps, and
formatting behavior can be tested without mounting the node.
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
The dependency environment node header lives in
`DependencyEnvironmentNodeHeader.svelte`, keeping icon and title markup separate
from node state orchestration.
`SelectionInputNode.svelte` loads provider-backed options through
`selectionInputProviderOptions.ts` and the shared `portOptionsCache.ts` service.
The component passes only stable provider context references, including the
current descriptor fingerprint when validation projections expose one, to the
backend. It keeps the selected graph value explicit and ignores late option
responses when the target descriptor/model/runtime context has already changed.
Image, audio, and point-cloud output nodes load backend-owned artifact format
defaults and capabilities through the workflow service. Their format selectors
store only explicit per-node overrides in `artifact_format_override`; a missing
or `null` override means the node uses the canonical backend Settings defaults.
`AudioOutputNode.svelte` accepts transient audio stream chunks either as legacy
inline `audio_base64` payloads or as ArtifactStore stream references. Stream
references keep `artifact_id`, `stream_handle`, byte range, lifecycle, sequence,
and finality metadata in runtime data while the component reads bytes lazily with
`workflowService.readArtifactStream` only when it needs browser preview playback.

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
- Streamed audio event handlers must not require inline `audio_base64`; they
  preserve ArtifactStore stream-reference metadata and let preview components
  fetch referenced stream bytes on demand.
- Specialized node components must mirror canonical backend-owned port names so
  template graphs and execution bindings do not depend on UI-local aliases.
- `LLMInferenceNode.svelte` must keep task and diagnostics display derived from
  `inference_payloads` role metadata only; it must not inspect backend runtime
  internals or infer scheduler/runtime selection.
- `LLMInferenceNode.svelte` may render backend graph-validation summary
  overlays for the current graph revision, but those overlays are display-only;
  submit gating, scheduler admission, and runtime execution must continue to
  consume backend validation authority directly.
- Drift and update-proposal badges on `LLMInferenceNode.svelte` must be derived
  only from backend projection overlays. The component must not compare ports,
  create patch operations, or mutate graph data itself. Supported apply actions
  must go through the graph-level inference-interface update coordinator and
  backend graph-patch API.
- Inference-interface update previews may list backend drift messages and
  backend operation counts for node-local review. They must not become graph
  mutation authority or recreate backend patch previews from frontend
  comparisons.
- `PumaLibNode.svelte` must consume the shared Pumas model-option cache from
  `src/services/workflow/pumaModelOptionsCache.ts`; selector cursor handoff and
  invalidation logic belong in that service, not in component module state.
- Retired expand-settings presentation helpers must not be restored; backend
  descriptors and authored snapshots are the only inference option source.
- `SelectionInputNode.svelte` may auto-adopt defaults for static
  `allowed_values` ports, but provider-backed ports must render unset or stale
  values without writing planner defaults into graph data.
- Provider-backed `SelectionInputNode.svelte` option loads must be one-shot,
  event-driven reactions to target context changes. They must use
  `selectionInputProviderOptions.ts` and discard stale async responses instead
  of polling, hardcoding backend-owned options, or mutating graph data when
  options arrive.
- Provider-backed option queries must carry descriptor fingerprint when present
  so cache invalidation follows backend descriptor changes instead of only
  model/runtime UI state.
- `DependencyEnvironmentNode.svelte` must keep dependency override parsing and
  merge semantics aligned with the backend patch contract in
  `dependencyEnvironmentOverrides.ts`.
- `DependencyEnvironmentNode.svelte` must delegate resolve/check/install to the
  graph-level dependency action coordinator. The graph coordinator supplies
  session and revision identity; workflow-service owns freshness checks and
  dependency request derivation.
- `dependencyEnvironmentOverrides.ts` owns displayed override values, scope
  clears, summary counts, local override checks, override timestamps, and form
  value projection; the Svelte component must only assign returned patch arrays
  and persist state.
- `dependencyEnvironmentSelection.ts` owns binding filtering and selection
  toggles; the Svelte component must not duplicate selected-binding rules
  inline.
- `dependencyEnvironmentNodeState.ts` owns dependency node persistence payloads
  and upstream requirement adoption; the Svelte component must not duplicate
  that state-shape mapping inline.
- `DependencyEnvironmentNode.svelte` may seed local state from the incoming
  node data once, but that initialization must be an explicit snapshot so
  subsequent prop updates are handled by the node's persistence and upstream
  adoption flows instead of hidden reactive capture.
- `DependencyEnvironmentActivityLog.svelte` owns log auto-scroll behavior and
  must not trigger graph drag or pan gestures.
- `dependencyEnvironmentActivityListener.ts` owns dependency activity event
  listener setup, initial persistence, automatic mode startup, and listener
  failure log formatting.
- `dependencyEnvironmentActions.ts` owns action payload construction, busy-state
  bracketing, backend response application, and failure log formatting for
  dependency action commands.
- `dependencyEnvironmentDisplay.ts` owns activity timestamp formatting so
  dependency log rendering remains testable outside the Svelte component.
- `DependencyEnvironmentStatusPanel.svelte` emits command callbacks without
  invoking backend APIs directly.
- `DependencyEnvironmentBindingsPanel.svelte` emits form and selection callbacks
  without writing node data directly, and any visible override labels must stay
  associated with concrete input ids so the manual override panel remains
  keyboard and screen-reader navigable.
- `DependencyEnvironmentModeControls.svelte` emits mode changes without writing
  node data directly.
- `DependencyEnvironmentNodeHeader.svelte` owns dependency environment header
  icon and label markup.
- Image and media preview controls must expose accessible names even when the
  visible content is an image or icon rather than text.
- Output-node artifact format selectors must not persist backend defaults into
  graph data unless the user explicitly chooses an override for that node.
- Output-node artifact format overrides are graph data, so run snapshots can
  capture them together with the workflow version while persistent defaults
  remain owned by the workbench Settings page and backend APIs.
- `ImageOutputNode.svelte` must treat only backdrop clicks as dialog-dismiss
  input; clicks inside the modal content stay local so expanded previews do not
  close while the user is interacting with the image or close button.

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
