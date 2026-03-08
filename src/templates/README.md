# src/templates

## Purpose
This directory contains app-bundled UI and workflow templates that Pantograph
ships as local starter assets. The boundary exists so reusable starter content
can be versioned alongside the app without mixing it into runtime service code.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `Button.svelte` | Base template used when generating or scaffolding button UIs. |
| `Card.svelte` | Card starter template used by UI-generation flows. |
| `Input.svelte` | Input starter template for generated form controls. |
| `Modal.svelte` | Modal starter template for generated overlay flows. |
| `workflows/` | Bundled workflow graph templates such as the tiny-sd-turbo text-to-image starter. |

## Problem
Pantograph needs local starter artifacts for both code-generation flows and
workflow bootstraps. Those artifacts must be versioned with the app so shipped
templates stay aligned with the current runtime contracts.

## Constraints
- Templates in this directory must remain static assets that can be bundled by
  the frontend build.
- Workflow templates must stay compatible with the workflow DTO contracts and
  built-in node definitions.
- Changes here can affect saved user bootstraps, so template semantics should
  evolve additively where possible.

## Decision
Keep starter Svelte component templates and workflow templates together under
`src/templates/`, while leaving loading/orchestration behavior in the workflow
service layer. This keeps asset ownership local without turning this directory
into a service boundary.

## Alternatives Rejected
- Move bundled workflow templates into the service layer.
  Rejected because service code should load templates, not own the static
  assets themselves.
- Keep generated UI templates and workflow templates in unrelated roots.
  Rejected because both are shipped starter assets consumed by the frontend.

## Invariants
- Assets in this directory are static bundled resources, not generated at
  runtime.
- Workflow template JSON must remain valid against the frontend workflow DTO
  contracts.
- Template file names must stay stable enough for static imports in the app.

## Revisit Triggers
- Workflow templates need remote updates or marketplace-style delivery.
- Generated UI templates start requiring build-time code generation.

## Dependencies
**Internal:** `src/services/workflow/templateService.ts`, frontend bundler
static asset imports, and workflow DTO contracts.

**External:** none.

## Usage Examples
```ts
import tinySdTurboTemplate from './workflows/tiny-sd-turbo-text-to-image.json';
```

## API Consumer Contract
None.
Reason: this directory stores bundled assets rather than a host-facing API.
Revisit trigger: external tools or plugins begin importing template assets
directly as a supported interface.

## Structured Producer Contract
- Workflow template JSON files must conform to the `WorkflowTemplate` shape
  consumed by `templateService.ts`.
- Template IDs are defined by the importing service, while file contents carry
  stable `name`, `description`, `version`, orchestration, and data-graph data.
- Changes that break existing built-in template loading require matching
  service updates in the same logical slice.
