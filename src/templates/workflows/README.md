# src/templates/workflows

## Purpose
This directory contains bundled workflow-template JSON files that Pantograph can
load as starter graphs. The boundary exists so shipped workflow examples remain
versioned with the app and reviewable as structured artifacts.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `gguf-reranker-workflow.json` | Minimal local reranking starter that wires `puma-lib`, query/document text inputs, the dedicated reranker node, and text output for GGUF reranker models such as `Qwen3-Reranker-4B-GGUF`. |
| `svelte-code-agent.json` | Multi-graph agent workflow template used to scaffold Svelte code-generation flows. |
| `tiny-sd-turbo-text-to-image.json` | Minimal local text-to-image starter that wires `puma-lib`, direct diffusion inference, and image output for imported bundles such as tiny-sd-turbo. |

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
the frontend template service. The tiny-sd-turbo template deliberately uses the
same direct `puma-lib -> diffusion-inference` path that Pantograph can execute
today for imported bundles without Pumas dependency bindings, rather than
shipping a starter graph that stalls on an unresolved dependency-environment
step.

## Alternatives Rejected
- Generate workflow templates dynamically in code.
  Rejected because structured JSON is easier to review, diff, and validate.
- Ship a text-to-image template that inserts dependency-environment even when
  imported bundles have no dependency bindings yet.
  Rejected because it teaches a starter graph that cannot execute for the
  currently supported imported tiny-sd-turbo path.

## Invariants
- Template JSON must deserialize into the frontend `WorkflowTemplate` shape.
- Built-in text-to-image templates must use declared node ports and may omit
  optional `environment_ref` handoff when the recommended starter path relies on
  Pantograph's local Python fallback.
- Example workflows should remain small enough to serve as operator references.
- Reranker starter workflows may use additive compatibility inputs such as
  `documents_json` only when the canonical structured port is still awkward to
  author with current built-in input nodes.

## Revisit Triggers
- Built-in templates need schema validation tooling beyond JSON parse checks.
- Pumas dependency bindings become available for imported diffusion bundles and
  the recommended starter path should reintroduce explicit dependency
  environment staging.

## Dependencies
**Internal:** `src/services/workflow/templateService.ts`, workflow DTOs, and
the node descriptors served by the Rust backend.

**External:** none.

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
  supports today; they must not imply unsupported generic-inference reranking.
