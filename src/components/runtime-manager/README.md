# Runtime manager components

## Purpose

The GUI for managed inference runtimes and managed dependencies: list the
runtimes the backend reports, install, pause, cancel, remove and select their
versions, and show the state of every managed dependency. Rendered in the
Settings page ("Managed inference runtimes") and, when
`showManagedRuntimePanel` is true, inside `../ServerStatus.svelte`.

## Contents

| File | Responsibility |
| --- | --- |
| `ManagedRuntimePanel.svelte` | Loads the runtime list and catalogs once on mount, subscribes to `managedRuntimeService` updates, renders one card per runtime. |
| `ManagedRuntimeCard.svelte` | Per-runtime actions (install, remove, pause, cancel, select, set default) and the derived labels the child parts show. |
| `ManagedRuntimeHeader.svelte`, `ManagedRuntimeSummaryGrid.svelte` | Name, readiness glyph and text, selection summary, unavailable reason, last action error. |
| `ManagedRuntimeJobPanel.svelte` | Active transfer: state, progress meter, retained artifact, pause and cancel or discard. |
| `ManagedRuntimeCatalogPanel.svelte` | Version policy selects and the versions table with per-version install and uninstall. |
| `ManagedRuntimeActivityPanel.svelte` | Install history, missing files, install and remove runtime. |
| `ManagedBinaryOverviewPanel.svelte` | Read-only "Managed dependency state" list grouped by dependency category. |
| `managedRuntimePresenters.ts` + `.test.ts` | Pure glyph mapping for readiness and job states and sentence-case state words. |

## Problem

Runtime install and readiness facts are backend-owned (ADR-003); the GUI only
needs to show them and forward user actions. These panels sat in the old side
panel with their own neutral, blue, amber and red Tailwind classes, spinners
and coloured error boxes, which clashed with the workbench redesign once they
moved into Settings (issue WR-M9-2).

## Constraints

- ADR-003 and ADR-007: catalog, install state, version policy, readiness and
  job progress come from `src/services/managedRuntime`; these components never
  derive install truth themselves.
- Workbench look: `pg-*` classes and tokens from `src/styles.css` and the
  shared parts in `../workbench/ui` (`StatusGlyph`, `Chip`); status is always
  in text beside a glyph; no spinners, pulsing, uppercase tracked labels, cyan
  or red error boxes.
- Native controls with names; per-row Install and Uninstall buttons carry the
  version in a screen-reader-only span so each has a distinct name.

## Decision

Keep the existing component split and data flow and restyle the markup only.
State-to-glyph mapping lives in `managedRuntimePresenters.ts` so it is tested
and shared by the header, job panel and dependency overview. Errors render as
muted sentences with `role="alert"` next to the action that failed.

## Alternatives Rejected

- Rebuilding the panels on the Settings page presenters: the runtime data
  model and actions are separate from the workflow settings the page projects,
  and the panel is also embedded in `ServerStatus`.
- Mapping readiness strings through `glyphStateForStatus`: that map covers run
  and node statuses; runtime readiness words such as `missing` or
  `extracting` would all fall through to idle.

## Invariants

- `ManagedRuntimePanel` loads once on mount and unsubscribes from
  `managedRuntimeService` on destroy.
- Buttons keep their original `disabled` conditions and handlers; Remove
  runtime, Uninstall and Cancel download stay single-click (no confirm step).
- Every readiness and job state maps to a glyph (`managedRuntimePresenters.test.ts`).

## Revisit Triggers

- A new runtime readiness or job state is added to
  `src/services/managedRuntime/types.ts`.
- A second managed runtime family lands and the cards need grouping.
- Destructive runtime actions get a confirm step (use the shared
  `TwoStepButton`).

## Dependencies

**Internal:** `src/services/managedRuntime`, `../workbench/ui`,
`src/styles.css`.

**External:** `svelte`.

## Related ADRs

- `docs/adr/ADR-003-runtime-redistributables-manager-boundary.md`
- `docs/adr/ADR-007-managed-runtime-observability-ownership.md`

## Usage Examples

```svelte
<script lang="ts">
  import ManagedBinaryOverviewPanel from '../runtime-manager/ManagedBinaryOverviewPanel.svelte';
  import ManagedRuntimePanel from '../runtime-manager/ManagedRuntimePanel.svelte';
</script>

<ManagedRuntimePanel />
<ManagedBinaryOverviewPanel />
```
