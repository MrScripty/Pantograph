# Components

## Purpose

Svelte components of the Pantograph desktop GUI that are not pages. The
workbench shell and its pages live in `workbench/`; this directory holds the
graph editor surface the Graph page embeds, the app settings panels the
Settings page embeds, the side panel, and the pure helpers those components
use.

## Contents

| Path | Responsibility |
| --- | --- |
| `workbench/` | Run-centric workbench shell, pages, presenters and shared `ui/` parts (ADR-014). See `workbench/README.md`. |
| `WorkflowGraph*.svelte`, `WorkflowGraph.css`, `workflow*.ts` + tests, `edgeInsertInteraction.ts`, `reconnectInteraction.ts`, `WorkflowContainerBoundary.svelte`, `WorkflowEdgeInsertPreviewMarker.svelte`, `WorkflowToolbar.svelte`, `WorkflowPersistenceControls.svelte`, `NodePalette.svelte` | Graph editor canvas and its controls, with pure interaction helpers. |
| `nodes/`, `edges/`, `orchestration/` | Node cards, the reconnectable edge, and orchestration graph nodes. See their READMEs. |
| `ServerStatus.svelte`, `BackendSelector.svelte`, `ModelConfig.svelte`, `DeviceConfig.svelte`, `RagStatus.svelte`, `SandboxSettings.svelte` | App settings panels the Settings page shows under "Runtime and app settings". Each is a collapsible `pg-card`; only one is open at a time (`src/stores/accordionStore.ts`). |
| `server-status/`, `runtime-manager/` | Parts of `ServerStatus` and the managed runtime and dependency panels. See their READMEs. |
| `deviceConfigPresenters.ts`, `deviceConfigRefreshScope.ts` + tests | Pure device option logic and the scoped device refresh owner for `DeviceConfig`. |
| `SidePanel.svelte`, `side-panel/`, `ChunkPreview.svelte` | Assistant side panel and the RAG chunk preview overlay mounted by `App.svelte`. |
| `Canvas.svelte`, `Toolbar.svelte`, `Rulers.svelte`, `ClearButton.svelte`, `TopBar.svelte`, `HotLoadContainer.svelte`, `ZoomTransition.svelte`, `UnifiedGraphView.svelte`, `NavigationBreadcrumb.svelte`, `NodeGroupEditor.svelte`, `GroupPortMapper.svelte`, `CommitTimeline.svelte` | Drawing, hot-load and graph navigation components from the pre-workbench app; several are only reachable through `src/features/*` barrels or `src/config/architecture.ts`. |

## Problem

The GUI started as a single canvas app with a side panel; the run-centric
workbench (ADR-014) became the root and pages were rebuilt on shared parts.
Components in this directory are reused by those pages, so they must follow
the workbench look and the pages' data rules without moving into
`workbench/`, which holds pages and their presenters only.

## Constraints

- Services own backend data: components call `src/services/*` and subscribe to
  their state, and never rebuild backend truth (ADR-003 for managed runtimes,
  ADR-014 for run projections).
- Workbench look: `pg-*` classes and `--pg-*` tokens from `src/styles.css` and
  the parts in `workbench/ui`; status as text beside a glyph; no cyan,
  uppercase tracked labels, pulsing or spinners in restyled components.
- Svelte 5 runes and snippets; native interactive elements with names
  (`scripts/check-svelte-a11y.mjs`).
- Pure logic sits in `*.ts` modules with Node test runner tests.

## Decision

Keep shared non-page components here, grouped by subdirectory when a
component has several parts, and restyle them in place onto the workbench
parts rather than forking page-specific copies. The settings panels remain
self-contained (they load their own service state on mount) so the Settings
page embeds them without wiring.

## Alternatives Rejected

- Moving the settings panels into `workbench/`: they are not pages, and
  `ServerStatus` and `ManagedRuntimePanel` are also exported through
  `src/features/*` barrels.
- Forking restyled copies for the Settings page: two copies of each panel's
  service wiring would drift.

## Invariants

- Each settings panel owns its subscriptions: it subscribes on mount and
  unsubscribes on destroy.
- Settings panel element ids (`vlm-model-path`, `compute-device-select`,
  `validation-mode-select`, ...) are stable; the a11y labels depend on them.
- Graph handle geometry is a contract with saved graphs (see `nodes/README.md`).

## Revisit Triggers

- A component here gains a second page consumer with different needs; give it
  props instead of copying it.
- The pre-workbench drawing components lose their last consumer; remove them.

## Dependencies

**Internal:** `src/services/*`, `src/stores/*`, `workbench/ui`,
`src/styles.css`.

**External:** `svelte`, `@xyflow/svelte` (graph editor), `@tauri-apps/api`
(services), `lucide-svelte` (side panel, architecture nodes, group editor).

## Related ADRs

- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md`
- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples

```svelte
<script lang="ts">
  import DeviceConfig from '../DeviceConfig.svelte';
  import ModelConfig from '../ModelConfig.svelte';
  import ServerStatus from '../ServerStatus.svelte';
</script>

<ServerStatus showManagedRuntimePanel={false} />
<ModelConfig />
<DeviceConfig />
```
