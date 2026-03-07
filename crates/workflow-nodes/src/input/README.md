# crates/workflow-nodes/src/input

## Purpose
This directory contains workflow input-node descriptors and host-bridge stubs.
These nodes define the graph-facing contracts for user input, model selection,
and library-provided metadata before host-specific executors take over.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `audio_input.rs` | Audio file input descriptor. |
| `image_input.rs` | Image input descriptor for workflow-provided image payloads. |
| `model_provider.rs` | Generic model-provider node contract. |
| `puma_lib.rs` | Puma-Lib model selector that emits model path, dependency requirements, and inference settings metadata. |
| `text_input.rs` | Plain text input descriptor. |
| `number_input.rs` | Numeric input descriptor. |
| `boolean_input.rs` | Boolean input descriptor. |

## Problem
Workflow graphs need stable input contracts that can be shared across hosts.
Model-selection nodes in particular must emit enough metadata for downstream
routing without hardcoding runtime choices into the UI or executor.

## Constraints
- Input descriptors must stay host-agnostic.
- Host-owned nodes such as `puma-lib` still need discoverable metadata for the
  frontend and dependency preflight.
- Model metadata fallbacks must stay additive so older Pumas records continue to
  resolve.

## Decision
Keep input nodes as descriptor-first modules. `puma_lib.rs` emits model path,
`task_type_primary`, backend hints, dependency requirements, and inference
settings so downstream routing can distinguish text, audio, and diffusion flows.

## Invariants
- Input nodes do not own runtime execution side effects.
- `puma-lib` metadata is the primary workflow-facing bridge from Pumas-Library
  into Pantograph routing.
- Fallback task inference must remain conservative and deterministic.

## Revisit Triggers
- Model-selection metadata grows beyond what one node should emit directly.
- `image_input.rs` becomes part of first-class img2img or inpaint execution and
  needs a richer contract.

## Dependencies
**Internal:** `node_engine` task metadata and inventory registration.

**External:** optional `pumas_library` APIs behind the `model-library` feature.

## API Consumer Contract
- Consumers should treat these modules as node descriptor sources, not direct
  execution APIs.
- `puma-lib` outputs are append-only workflow metadata contracts.

## Structured Producer Contract
- `puma-lib` emits `model_path`, `inference_settings`, and
  `dependency_requirements`.
- Diffusion models should resolve to `text-to-image` when explicit metadata is
  missing but `model_type == diffusion`.
