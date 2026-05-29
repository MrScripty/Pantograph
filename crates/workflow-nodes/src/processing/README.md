# crates/workflow-nodes/src/processing

## Purpose
This directory contains processing-node descriptors and host-routed execution
contracts for workflow steps that transform inputs into model outputs, derived
artifacts, or filtered values. The boundary exists so graph-visible processing
contracts stay explicit even when execution is delegated to Pantograph host
adapters such as the Python runtime.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `dependency_environment.rs` | Declares the dependency-environment sidecar/control node that associates dependency actions with one inference node. |
| `json_filter.rs` | Filters JSON payloads without leaving the workflow graph. |
| `inference.rs` | Declares the canonical `llm-inference` bootstrap contract for model reference and scheduler constraint inputs before backend descriptor resolution. |

## Problem
Workflow graphs need stable processing-node contracts across Rust, Python, and
frontend layers. Without explicit descriptors, runtime-only fields such as
dependency environment references or model-specific override settings become
hidden behavior that the graph cannot express safely.

## Constraints
- Processing descriptors must stay host-agnostic even when execution is
  delegated to Tauri task executors or Python sidecars.
- Current canonical contract changes must preserve declared port meanings.
  Retired legacy ports are removed instead of kept as compatibility aliases.
- Dependency/runtime metadata used by inference runtimes must come from backend
  validation, dependency planning, scheduler admission, and runtime handoff
  contracts rather than graph-visible path-shaped processing nodes.
- Canonical inference runtime selection is graph-visible only as optional
  scheduler intent. An omitted `runtime` input leaves runtime choice to
  scheduler policy, while an explicit value is a hard scheduler requirement
  after backend capability validation.

## Decision
Keep descriptors in this directory as the graph-visible contract layer and let
host executors implement the runtime behavior. Retired direct inference
descriptors, including old diffusion, llama.cpp, embedding, and reranker
shapes plus the old direct `vision-analysis` image-understanding node, must not
re-enter this directory. Canonical `llm-inference` static metadata is now only
the bootstrap surface for `pumas_model_ref`, optional task kind, optional
runtime/device constraints, and diagnostics. Text/chat, embedding, rerank,
audio-transcription, image-understanding, image-generation, sampler, and result
ports must be resolved by workflow-service descriptors and authored snapshots.
Its `Task::run` implementation fails closed so standalone graph execution
cannot bypass the host typed inference gateway.
The optional `runtime` input is not projected into node-engine execution,
worker envelopes, or inference requests directly; scheduler-produced execution
decisions are the only source of selected runtime facts.
The dependency-environment descriptor is a control/manual sidecar. It exposes
only user-authored sidecar choices plus the typed
`dependency_environment_sidecar` association output; it does not consume
`pumas_model_ref`, model facts, backend keys, platform context, dependency
requirements, or environment references as graph ports. Workflow-service derives
dependency requests from the associated inference node and backend validation
state.
The old descriptor-local `base_url`/model generation config has been removed;
generation and task options must flow through descriptor-authored graph ports
and typed inference requests.
The previous static `llm-inference.denoising_scheduler` port and provider are
retired. Denoising scheduler choices are descriptor-backed option sets owned by
the inference-interface resolver; explicit scheduler acceptance remains a
planner decision.
The retired `expand-settings` passthrough node is no longer registered.
Model-specific inference options must come from backend descriptors and
authored snapshots instead of a frontend-owned settings expansion path.
The retired direct `audio-generation`, `onnx-inference`, and
`depth-estimation` graph descriptors are no longer registered. Their task
families remain runtime/backend capabilities for canonical inference
descriptors and scheduler dispatch, but they are not authorable processing
nodes and must not reintroduce graph-visible `model_path`, `environment_ref`,
or `inference_settings` ports.
Compatible text-generation
descriptors now also reserve explicit `kv_cache_in` and `kv_cache_out` ports
using the first-class `kv_cache` graph type so KV reuse remains graph-visible
instead of hiding behind generic JSON ports.
Dependency-environment sidecar association uses the first-class
`dependency_environment_sidecar` port type. Processing descriptors must not
represent that association with `json`, `any`, `component`, or runtime
environment-ref data ports.

## Alternatives Rejected
- Leave dependency environment handoff as an undocumented runtime-only input.
  Rejected because the graph could not wire or validate the contract explicitly.
- Move Python-backed node descriptors into host-specific modules.
  Rejected because workflow consumers need the same metadata regardless of host.

## Invariants
- Descriptor metadata remains the source of truth for graph-visible ports.
- Python-backed node contracts must preserve canonical port meanings across
  releases. Retired path-shaped inputs are removed rather than maintained as
  compatibility shims.
- Dependency environment association uses the exact-only
  `dependency_environment_sidecar` port type rather than structured JSON,
  opaque string flags, or runtime environment-ref ports.
- KV-cache reuse, when exposed by processing nodes, uses explicit `kv_cache`
  ports rather than generic `json` ports.
- Static `llm-inference` descriptors must not expose prompt, text, image,
  sampler, rerank, audio, cache, usage, or result ports. Those ports come only
  from backend-resolved descriptors and authored snapshots.
- `expand-settings` must not be restored as an alternate inference-interface
  source. Existing `inference_settings` uses are removal or descriptor-backed
  rewrite targets in the inference-interface milestone.
- Direct `audio-generation`, `onnx-inference`, and `depth-estimation`
  processing nodes must not be restored as graph-visible inference entrypoints.
  Add or change those task families through backend-resolved
  `llm-inference` descriptors and scheduler/runtime dispatch contracts.
- Denoising scheduler option rows must come from descriptor-backed typed option
  sets and Pumas package facts. They must not write executable defaults into
  graph data or bypass planner diagnostics.
- JSON-filter configuration defaults remain the derived empty-path/empty-default
  contract so descriptor consumers and task construction share one default
  shape.

## Revisit Triggers
- Another runtime requires a different environment handoff contract than the
  current JSON `environment_ref`.
- Saved workflows need migration because a processing-node port meaning changes.

## Dependencies
**Internal:** `node_engine` task metadata, host task executors in Pantograph,
and workflow frontend port synchronization.

**External:** none directly; runtime-specific dependencies are owned by the
host executors that consume these descriptors.
- Reason: processing descriptors define graph contracts while host executors
  own runtime-specific packages and endpoints.
- Revisit trigger: a processing node begins owning runtime integration directly
  rather than declaring a host-consumed descriptor.

## Related ADRs
None.
Reason: this directory documents local descriptor contracts; no separate
architecture decision has been needed beyond the implementation plan.
Revisit trigger: processing descriptors move into a generated registry or a
host-specific runtime crate.

## Usage Examples
```rust
let meta = InferenceTask::descriptor();
assert_eq!(meta.node_type, "llm-inference");
```

## API Consumer Contract
- Host executors must honor the declared input/output ports for processing
  nodes, including optional additive ports such as `environment_ref`.
- Python-backed nodes may fail when required host runtime dependencies are not
  configured; those failures surface as task execution errors.

## Structured Producer Contract
- Descriptor metadata in this directory is machine-consumed by workflow
  registries, graph validation, and frontend node renderers.
- New static processing ports may be added additively only when they are not
  model/task-specific inference interface ports. Inference-specific ports must
  be added through the descriptor resolver and authored snapshot contract.
- Future option presentation must consume backend descriptors or be removed.
