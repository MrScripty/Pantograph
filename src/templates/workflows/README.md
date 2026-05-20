# src/templates/workflows

## Purpose
This directory contains bundled workflow-template JSON files that Pantograph can
load as starter graphs. The boundary exists so shipped workflow examples remain
versioned with the app and reviewable as structured artifacts.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `gguf-reranker-workflow.json` | Minimal local reranking starter that wires `puma-lib`, query/document text inputs, canonical `llm-inference` with `task_kind = rerank`, and text output for GGUF reranker models such as `Qwen3-Reranker-4B-GGUF`. |
| `svelte-code-agent.json` | Multi-graph agent workflow template used to scaffold Svelte code-generation flows. |
| `tiny-sd-turbo-text-to-image.json` | Minimal local text-to-image starter that wires `puma-lib`, canonical `llm-inference` with `task_kind = image_generation`, and image output for imported bundles such as tiny-sd-turbo. |

## Problem
Starter workflows must demonstrate real graph shapes that match current node
contracts. Without checked-in examples, users and maintainers have no shared
baseline for how Pantograph expects multi-node workflows such as local
text-to-image generation or local GGUF reranking to be wired.

## Constraints
- Templates must stay valid JSON assets that the frontend can import directly.
- Node IDs, port IDs, and graph DTO fields must stay aligned with workflow
  registry contracts.
- Templates should favor minimal, reviewable graphs over product-complete demos.

## Decision
Store built-in workflow templates here as JSON and import them statically into
the frontend template service. Text-to-image starters use the same canonical
`puma-lib -> llm-inference` model-reference and inference-settings handoff as
other inference tasks, with `task_kind = image_generation` and a graph-visible
`image` output connected to `image-output`. Runtime selection remains
scheduler-owned unless a template explicitly demonstrates a hard runtime
requirement.

## Alternatives Rejected
- Generate workflow templates dynamically in code.
  Rejected because structured JSON is easier to review, diff, and validate.
- Keep text-to-image starters on the direct `diffusion-inference` node after
  canonical image generation gained an `image` output.
  Rejected because it would keep new starter workflows on a superseded graph
  shape and bypass the Pumas package-facts boundary.

## Invariants
- Template JSON must deserialize into the frontend `WorkflowTemplate` shape.
- Built-in templates must not use retired direct inference node types such as
  `diffusion-inference`, `llamacpp-inference`, `pytorch-inference`,
  `ollama-inference`, dedicated `embedding`, or dedicated `reranker`.
- Built-in text-to-image templates must use canonical `llm-inference` with
  `task_kind = image_generation`, carry `pumas_model_ref` and
  `inference_settings` from `puma-lib`, and connect the canonical `image`
  output into `image-output`.
- Built-in templates must not use retired graph-visible backend/runtime
  selection fields such as `backend_key`, `runtime_hint`, resolved Pumas
  package facts, or raw model-source ports. The scheduler is the only runtime
  selection authority.
- Example workflows should remain small enough to serve as operator references.
- Reranker starter workflows may use additive compatibility inputs such as
  `documents_json` only when the canonical structured port is still awkward to
  author with current built-in input nodes.

## Revisit Triggers
- Built-in templates need schema validation tooling beyond JSON parse checks.
- The canonical inference contract replaces direct task-specific node types
  with a more specific image-generation node contract.

## Dependencies
**Internal:** `src/services/workflow/templateService.ts`, workflow DTOs, and
the node descriptors served by the Rust backend.

**External:** None.
Reason: bundled templates are local assets consumed by Pantograph itself.
Revisit trigger: templates are loaded from remote catalogs or plugin packages.

## Related ADRs
None.
Reason: bundled templates are an internal starter-graph asset boundary rather
than an architecture decision record surface.
Revisit trigger: template loading becomes an external SDK or plugin surface.

## Usage Examples
```ts
import tinySdTurboTemplate from './tiny-sd-turbo-text-to-image.json';
```

## API Consumer Contract
None.
Reason: these are bundled assets consumed internally by Pantograph.
Revisit trigger: template loading becomes an external SDK or plugin surface.

## Structured Producer Contract
- Each file defines one `WorkflowTemplate` object with stable top-level fields:
  `name`, `description`, `version`, `orchestration`, and `dataGraphs`.
- Data-graph `nodes` and `edges` must match the workflow DTO field names used by
  `templateService.ts`.
- Template changes that rely on new node contracts must land with the matching
  descriptor/runtime changes in the same implementation slice.
- Template examples must reflect the backend execution path Pantograph actually
  supports today; they must not imply unsupported generic-inference reranking or
  bypass the canonical image-generation inference contract.
