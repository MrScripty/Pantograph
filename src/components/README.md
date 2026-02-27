# src/components

## Purpose
Svelte UI components for the frontend experience, organized by feature and composition boundaries.

## Contents
| File/Folder | Description |
| ----------- | ----------- |
| BackendSelector.svelte | Source file used by modules in this directory. |
| BinaryDownloader.svelte | Source file used by modules in this directory. |
| Canvas.svelte | Source file used by modules in this directory. |
| ChunkPreview.svelte | Source file used by modules in this directory. |
| ClearButton.svelte | Source file used by modules in this directory. |
| CommitTimeline.svelte | Source file used by modules in this directory. |
| DeviceConfig.svelte | Source file used by modules in this directory. |
| GraphSelector.svelte | Source file used by modules in this directory. |
| GroupPortMapper.svelte | Source file used by modules in this directory. |
| HotLoadContainer.svelte | Source file used by modules in this directory. |
| ModelConfig.svelte | Source file used by modules in this directory. |
| NavigationBreadcrumb.svelte | Source file used by modules in this directory. |
| NodeGroupEditor.svelte | Source file used by modules in this directory. |
| NodePalette.svelte | Source file used by modules in this directory. |
| RagStatus.svelte | Source file used by modules in this directory. |
| Rulers.svelte | Source file used by modules in this directory. |
| SandboxSettings.svelte | Source file used by modules in this directory. |
| ServerStatus.svelte | Source file used by modules in this directory. |
| SidePanel.svelte | Source file used by modules in this directory. |
| Toolbar.svelte | Source file used by modules in this directory. |
| TopBar.svelte | Source file used by modules in this directory. |
| UnifiedGraphView.svelte | Source file used by modules in this directory. |
| WorkflowGraph.svelte | Source file used by modules in this directory. |
| WorkflowToolbar.svelte | Source file used by modules in this directory. |
| ZoomTransition.svelte | Source file used by modules in this directory. |
| edges/ | Subdirectory containing related implementation details. |
| nodes/ | Subdirectory containing related implementation details. |
| orchestration/ | Subdirectory containing related implementation details. |
| side-panel/ | Subdirectory containing related implementation details. |

## Design Decisions
- Keep files in this directory scoped to a single responsibility boundary.
- Prefer explicit module boundaries over cross-cutting utility placement.
- Maintain predictable naming so callers can discover related modules quickly.

## Dependencies
**Internal:** Neighboring modules in this source tree and the nearest package/crate entry points.
**External:** Dependencies declared in the corresponding manifest files.

## Usage Examples
```ts
// Example: import API from this directory.
import { value } from './module';
```
