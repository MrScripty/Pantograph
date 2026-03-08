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
| `diffusion_inference.rs` | Declares the graph contract for process-backed diffusion generation, including optional dependency-environment handoff. |
| `pytorch_inference.rs` | Defines the general PyTorch inference contract used for text-generation style models. |
| `audio_generation.rs` | Declares the Stable Audio generation node contract. |
| `dependency_environment.rs` | Exposes dependency resolution and environment materialization as an explicit workflow step. |
| `expand_settings.rs` | Turns inference-setting schemas into graph-visible override ports. |
| `json_filter.rs` | Filters JSON payloads without leaving the workflow graph. |
| `vision_analysis.rs` | Declares image-to-text style vision analysis contracts. |

## Problem
Workflow graphs need stable processing-node contracts across Rust, Python, and
frontend layers. Without explicit descriptors, runtime-only fields such as
dependency environment references or model-specific override settings become
hidden behavior that the graph cannot express safely.

## Constraints
- Processing descriptors must stay host-agnostic even when execution is
  delegated to Tauri task executors or Python sidecars.
- Contract changes must remain additive so saved workflows continue to load.
- Dependency/runtime metadata used by Python-backed nodes must be graph-visible
  when workflows need to stage environment readiness explicitly.

## Decision
Keep descriptors in this directory as the graph-visible contract layer and let
host executors implement the runtime behavior. Python-backed diffusion nodes now
declare an optional `environment_ref` input so dependency-environment workflows
can express the handoff explicitly instead of relying on hidden runtime fields.

## Alternatives Rejected
- Leave dependency environment handoff as an undocumented runtime-only input.
  Rejected because the graph could not wire or validate the contract explicitly.
- Move Python-backed node descriptors into host-specific modules.
  Rejected because workflow consumers need the same metadata regardless of host.

## Invariants
- Descriptor metadata remains the source of truth for graph-visible ports.
- Python-backed node contracts must stay additive across releases.
- Dependency environment handoff, when used, is represented as structured JSON
  rather than opaque string flags.

## Revisit Triggers
- Another runtime requires a different environment handoff contract than the
  current JSON `environment_ref`.
- Saved workflows need migration because a processing-node port meaning changes.

## Dependencies
**Internal:** `node_engine` task metadata, host task executors in Pantograph,
and workflow frontend port synchronization.

**External:** none directly; runtime-specific dependencies are owned by the
host executors that consume these descriptors.

## Usage Examples
```rust
let meta = DiffusionInferenceTask::descriptor();
assert!(meta.inputs.iter().any(|p| p.id == "environment_ref"));
```

## API Consumer Contract
- Host executors must honor the declared input/output ports for processing
  nodes, including optional additive ports such as `environment_ref`.
- Python-backed nodes may fail when required host runtime dependencies are not
  configured; those failures surface as task execution errors.

## Structured Producer Contract
- Descriptor metadata in this directory is machine-consumed by workflow
  registries, graph validation, and frontend node renderers.
- New ports may be added additively; existing port IDs and meanings must remain
  stable for saved workflows.
