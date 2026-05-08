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
| `audio_generation.rs` | Declares the Stable Audio generation node contract. |
| `dependency_environment.rs` | Exposes dependency resolution and environment materialization as an explicit workflow step. |
| `expand_settings.rs` | Declares the passthrough node that exposes inference-setting schemas as matching override-capable input/output ports. |
| `json_filter.rs` | Filters JSON payloads without leaving the workflow graph. |
| `inference.rs` | Declares the canonical `llm-inference` graph contract for text/chat, embedding, rerank, audio transcription, image generation, model refs, package facts, and graph-visible generated-image output. |
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
host executors implement the runtime behavior. Retired direct inference
descriptors, including old diffusion, llama.cpp, embedding, and reranker
shapes, must not re-enter this directory; canonical `llm-inference` task
metadata owns text/chat, embedding, rerank, audio-transcription, and
image-generation graph contracts. Its `Task::run` implementation fails closed
so standalone graph execution cannot bypass the host typed inference gateway.
The old descriptor-local `base_url`/model generation config has been removed;
generation and task options must flow through canonical graph ports and typed
inference requests.
`expand_settings.rs` follows the same contract-first rule: model-specific
settings stay graph-visible as matching optional input/output ports while the
schema itself still passes through unchanged for downstream inference merging.
Compatible text-generation
descriptors now also reserve explicit `kv_cache_in` and `kv_cache_out` ports
using the first-class `kv_cache` graph type so KV reuse remains graph-visible
instead of hiding behind generic JSON ports.

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
- KV-cache reuse, when exposed by processing nodes, uses explicit `kv_cache`
  ports rather than generic `json` ports.
- Expand-settings contracts must preserve the static `inference_settings`
  passthrough while keeping per-setting override ports additive and keyed by the
  source schema.
- Rerank outputs must be exposed through canonical `llm-inference` ranked-result
  fields so saved workflows and templates can consume them without
  endpoint-specific parsing logic.
- Canonical `llm-inference` image-generation descriptors must expose the first
  generated image on the `image` output while keeping the full typed result
  envelope on `results`.
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
- New ports may be added additively; existing port IDs and meanings must remain
  stable for saved workflows.
- `expand-settings` publishes `inference_settings` as the authoritative schema
  payload and may add per-setting ports keyed exactly by the upstream schema
  `key` values.
