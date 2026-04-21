# crates/workflow-nodes/src/input

## Purpose
This directory contains workflow input-node descriptors and host-bridge stubs.
These nodes define the graph-facing contracts for user input, model selection,
and library-provided metadata before host-specific executors take over.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `puma_lib.rs` | Host-bridged model selector that publishes routing and dependency metadata from Pumas into workflow graphs. |
| `model_provider.rs` | Generic model selector descriptor/task used when the workflow is not backed by the Pantograph/Pumas library path. It does not own a separate `NodeExecutor` implementation. |
| `text_input.rs` | Freeform text source for prompts and other string inputs. |
| `number_input.rs` | Numeric source node that adopts downstream defaults and constraints. |
| `boolean_input.rs` | Boolean source node for true/false workflow settings. |
| `image_input.rs` | Image payload source for image-consuming workflows. |
| `audio_input.rs` | Audio payload source for audio-consuming workflows. |

## Problem
Workflow graphs need stable input contracts that can be shared across hosts.
Model-selection nodes in particular must emit enough metadata for downstream
routing without hardcoding runtime choices into the UI or executor.

## Constraints
- Input descriptors must stay host-agnostic.
- Host-owned nodes such as `puma-lib` still need discoverable metadata for the
  frontend and dependency preflight.
- Runtime-executable model facts must come from the Pumas execution descriptor
  when a model can resolve one.
- Model metadata fallbacks must stay additive so older Pumas records continue to
  resolve when descriptor lookup is unavailable.

## Decision
Keep input nodes as descriptor-first modules. `puma_lib.rs` emits model path,
`task_type_primary`, backend hints, dependency requirements, and inference
settings so downstream routing can distinguish text, audio, and diffusion
flows. For `puma-lib`, Pantograph preserves the graph-facing `model_path`,
`model_type`, and `task_type_primary` facade, but it should source those values
from Pumas `ModelExecutionDescriptor` whenever a `model_id` is available and
descriptor resolution succeeds. Record metadata remains a display/fallback
contract only, not the runtime source of truth.

## Alternatives Rejected
- Keep an unregistered `model-provider` `NodeExecutor` in this crate.
  Rejected because active model-provider execution is already owned by
  `node-engine` core executor handlers, leaving the workflow-nodes executor as
  dead code.
- Move Pumas-backed model selection into generic `model-provider`.
  Rejected because `puma-lib` owns the Pumas-specific workflow metadata and
  dependency contract.

## Invariants
- Input nodes do not own runtime execution side effects.
- Generic `model-provider` remains a descriptor/task contract; active runtime
  projection for model-provider nodes is owned by `node-engine` core executor
  handlers rather than an unregistered workflow-nodes executor.
- `puma-lib` metadata is the primary workflow-facing bridge from Pumas-Library
  into Pantograph routing.
- Pantograph must not infer Pumas runtime bundle semantics from projected
  metadata when an execution descriptor is available.
- Fallback task inference must remain conservative and deterministic.

## Revisit Triggers
- Model-selection metadata grows beyond what one node should emit directly.
- `image_input.rs` becomes part of first-class img2img or inpaint execution and
  needs a richer contract.

## Dependencies
**Internal:** `node_engine` task metadata and inventory registration.

**External:** optional `pumas_library` APIs behind the `model-library` feature.

## Related ADRs
- None identified as of 2026-04-21.
- Reason: the model-provider executor cleanup removes an inactive
  implementation and preserves the existing crate ownership boundary.
- Revisit trigger: generic model-provider execution becomes a public,
  host-bridged contract separate from `node-engine` core executor handling.

## Usage Examples
```rust
let metadata = ModelProviderTask::descriptor();
assert_eq!(metadata.node_type, "model-provider");
```

## API Consumer Contract
- Consumers should treat these modules as node descriptor sources, not direct
  execution APIs.
- `puma-lib` outputs are append-only workflow metadata contracts.
- `puma-lib` preserves the `model_path` facade, but hosts may source that value
  from Pumas execution descriptors instead of raw library record paths.
- Consumers must not assume Pantograph inferred runtime-executable paths from
  `metadata.json`; the host may rebind the same facade from the upstream
  execution descriptor contract.

## Structured Producer Contract
- `puma-lib` emits `model_path`, `model_id`, `model_type`,
  `task_type_primary`, `backend_key`, `recommended_backend`, `platform_context`,
  `selected_binding_ids`, `dependency_bindings`,
  `dependency_requirements_id`, `inference_settings`, and
  `dependency_requirements`.
- When `ModelExecutionDescriptor` resolution succeeds, `model_path` must be the
  executable `entry_path`, `model_type` should prefer the descriptor model
  type, and `task_type_primary` should prefer descriptor task data unless more
  explicit task metadata is present.
- Metadata fields such as `bundle_format`, `storage_kind`, and `entry_path` are
  compatibility fallbacks only. They are not the authoritative runtime contract
  for executable model selection.
- Diffusion models should resolve to `text-to-image` when explicit metadata is
  missing but `model_type == diffusion`.
