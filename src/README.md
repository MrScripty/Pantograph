# src

Frontend application source for Pantograph.

## Purpose
This directory owns the Svelte frontend application shell, user-facing workflow
UI, frontend services, stores, templates, and shared browser utilities. It
adapts backend-owned workflow/runtime contracts into interactive views without
becoming the source of truth for graph mutation, runtime readiness, or
execution identity.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| `App.svelte` | Application root component and top-level layout composition. |
| `main.ts` | Browser entrypoint that mounts the Svelte application. |
| `styles.css` | Application-wide CSS and Tailwind base styles. |
| `constants.ts` | Frontend constants shared by app-level modules. |
| `types.ts` | App-level TypeScript type aliases. |
| `vite-env.d.ts` | Vite environment type declarations. |
| `backends/` | Frontend backend adapter selection and transport abstractions. |
| `components/` | Reusable and workflow-specific Svelte UI components. |
| `config/` | Frontend configuration metadata and architecture descriptors. |
| `features/` | Feature-level frontend module entrypoints. |
| `generated/` | Runtime-generated Svelte component workspace owned by the hot-load sandbox; marker docs are tracked while runtime-authored components are ignored. |
| `lib/` | Frontend libraries, design-system helpers, and hot-load sandbox support. |
| `registry/` | Frontend node/component registry wiring. |
| `services/` | Frontend service adapters for agent, diagnostics, managed runtime, and workflow APIs. |
| `shared/` | Shared frontend components, stores, and utility exports. |
| `stores/` | Application-level Svelte stores and workflow state adapters. |
| `templates/` | Frontend-authored workflow templates and template metadata. |
| `types/` | Domain-specific frontend type declarations. |

## Problem
The frontend must render complex workflow, runtime, diagnostics, and generated
component surfaces while consuming backend-owned contracts. Unclear ownership
would let UI code rebuild workflow truth locally or leave generated component
state ambiguous under a source root.

## Constraints
- Backend services own workflow mutation, execution/session identity, runtime
  readiness, and diagnostics facts.
- Frontend services may normalize transport payloads but must not invent second
  sources of truth.
- Generated component files under `src/generated/` remain runtime state, while
  undo/redo Git metadata is stored outside `src/` in
  `.pantograph/generated-components.git/`.
- Templates and generated component metadata are machine-consumed and require
  explicit producer contracts.
- UI state must remain responsive and deterministic across workflow reruns.

## Decision
Keep browser presentation, command invocation, and UI-local interaction state in
this source tree. Treat backend DTOs as authoritative for durable workflow and
runtime facts. Keep `src/generated/` as a documented generated-component
working tree with externalized history metadata and tracked marker docs.

## Alternatives Rejected
- Let frontend stores reconstruct canonical graph mutations: rejected because
  backend-owned mutation responses are required for no-optimistic-update
  behavior.
- Track runtime-authored generated Svelte files in the outer repo: rejected
  because generated components are local runtime/user state, while marker docs
  are now tracked separately.
- Move all UI state into backend DTOs: rejected because browser-only
  interaction state and media resources belong in component/store layers.

## Invariants
- Backend-owned workflow responses drive durable graph and execution state.
- UI components may own browser resources, transient form state, and display
  affordances only.
- `src/generated/` tracks marker documentation only; runtime-authored
  components remain ignored by the outer repo.
- Template and generated metadata changes must document compatibility and
  regeneration expectations.
- Transport services should preserve backend error categories and avoid
  genericizing expected workflow rejections.

## Revisit Triggers
- Generated component history moves away from the repo-local
  `.pantograph/generated-components.git/` store.
- Frontend services start owning decisions that should be returned by backend
  workflow/runtime APIs.
- A plugin/extension system begins consuming frontend components as a public
  API.
- Saved template schemas or generated component manifests change shape.

## Dependencies
**Internal:** package workspace modules under `packages/`, Tauri command
bindings, frontend stores, workflow services, design-system helpers, and
template data in this source tree.

**External:** Svelte 5, Vite, Tauri JavaScript APIs, `@xyflow/svelte`,
`lucide-svelte`, Tiptap, Three.js, and browser platform APIs.

## Related ADRs
- `docs/adr/ADR-001-headless-embedding-service-boundary.md`
- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md`

## Usage Examples
```ts
import App from './App.svelte';
import './styles.css';
```

## API Consumer Contract
- Inputs: backend DTOs, Tauri command responses, workflow events, user
  interactions, saved templates, and generated component manifests.
- Outputs: rendered UI, frontend command invocations, and browser-local
  interaction state.
- Lifecycle: Svelte components mount/unmount with the app; services subscribe to
  backend events and must clean up browser resources on teardown. Workbench
  pages query backend projections directly for run diagnostics instead of
  starting app-level diagnostics polling.
- Errors: backend error categories should remain visible to callers and UI
  presenters instead of being collapsed into generic transport failures.
- Versioning: frontend type and service changes must migrate backend DTO
  consumers, templates, and tests together when serialized shapes change.

## Structured Producer Contract
- Stable fields: workflow templates, generated component metadata, frontend
  registry entries, and serialized store fixtures are machine-consumed.
- Defaults: omitted frontend template fields must match backend/service
  defaults or be normalized before persistence.
- Enums and labels: node type ids, port ids, runtime state labels, and template
  ids are semantic contract values.
- Ordering: template node/edge ordering and generated component history ordering
  must remain deterministic where rendered or replayed.
- Compatibility: saved templates and generated component state may survive app
  upgrades, so field changes require migration notes.
- Regeneration/migration: generated component history metadata is stored in
  `.pantograph/generated-components.git/`; future backend-owned history stores
  must migrate Tauri commands, frontend hot-load services, and marker docs
  together.

## Testing
```bash
npm run lint:full
npm run typecheck
npm run test:frontend
```

## Notes
- `src/generated/` now has tracked marker documentation; generated component
  source files and history metadata remain ignored runtime state.
