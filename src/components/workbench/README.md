# Workbench

## Purpose

The run-centric workbench is the only root workspace of the Pantograph desktop
GUI (ADR-014). This directory holds the shell, the pages that hang off its tab
bar, and the pure presenters that turn backend projections into what each page
renders.

## Contents

| Path | Responsibility |
| --- | --- |
| `WorkbenchShell.svelte` | Header (brand, tab groups, run-context picker, engine line, Settings, Assistant toggle) and the page switch. |
| `*Page.svelte` | One page per tab. Each consumes typed projection services and its own presenters. |
| `*Presenters.ts` + `*.test.ts` | Pure projection-to-view logic for one page, tested with the Node test runner. |
| `shell/` | Run-context picker, its run-list source and the run-id clipboard shortcuts. See `shell/README.md`. |
| `ui/` | Shared presentation parts every page composes (panels, rows, glyphs, segments, lanes, log lines). See `ui/README.md`. |
| `GraphPage.svelte`, `RunGraphSnapshot.svelte`, `runGraphPresenters.ts`, `graphInspectionPresenters.ts` | Graph tab: edit mode over the editable graph stores, read-only run mode over the run graph and node status projections (ADR-014). |
| `SessionsPage.svelte` | Sessions tab: saved workflows and orchestrations shown as sessions with their runs from the run-context run list. No session service exists, so session actions are disabled (see `docs/plans/workbench-redesign/reports/m8-sessions.md`). |

## Problem

Pages used to repeat the active run in their own headers, style themselves
with per-page Tailwind greys and `!important` overrides, and each invent status
colours. The approved redesign (`docs/design/workbench-redesign-spec.md`) needs
one shell, one run selection control and one set of parts so pages built in
parallel come out as one system.

## Constraints

- ADR-014: the active run is transient frontend state in
  `src/stores/workbenchStore.ts`; pages read typed projection services and
  never query `diagnostic_events` or parse `payload_json` for primary facts.
- Historic run graphs render in an isolated read-only surface and never touch
  the editable graph store.
- Svelte 5 runes and snippets; Tailwind 3 with the `pg-*` tokens from
  `src/styles.css`; native interactive elements only
  (`scripts/check-svelte-a11y.mjs`).

## Decision

The shell owns navigation and the run context; pages own their projection and
layout; `ui/` owns visual primitives. Pages compose `ui/` parts and call the
run-context actions in `shell/runContextStore.ts`; they do not build their own
selection controls or status colours. The heading set of this README is
required by `scripts/check-decision-traceability.sh` (issue WR-001 in
`docs/plans/workbench-redesign/issues.md`); its content is the boundary
description for this directory.

## Alternatives Rejected

- Keeping the icon rail and a per-page "Active Run" label: it showed the
  selection twice and gave no grouping of tabs.
- Letting each page style its own rows and glyphs: the redesign mockups depend
  on one glyph vocabulary and one row shape across tabs.

## Invariants

- Graph run mode never writes the editable graph store; it reads only the run graph and node status projections for the selected run.
- The Scheduler reads the shell's `runContextRunList` as its only run-list source; it never queries `workflow_run_list_query` itself (WR-005). Diagnostics, I/O Inspector, Network, Library and Sessions keep their own projection queries and never read the diagnostic ledger directly (ADR-014).

- Scheduler is the default page; page ids and their order come from
  `WORKBENCH_PAGES` in `src/stores/workbenchStore.ts`.
- Exactly one run-context picker is mounted (by the shell); pages open it with
  `openRunContextPicker()` instead of rendering their own.
- While the engine is unreachable the page area dims to 72% opacity and pages
  keep their last projection instead of blanking.
- Presenters stay pure: no services, stores, DOM or timers.

## Revisit Triggers

- A page needs a projection field the backend does not expose (record it in
  the plan's `issues.md` before working around it).
- A second root workspace is proposed alongside the workbench.

## Dependencies

**Internal:** `src/stores/workbenchStore.ts`, `src/stores/panelStore.ts`,
`src/services/workflow/WorkflowService.ts` and the diagnostics projection types
in `src/services/diagnostics/types.ts`.

**External:** `svelte`, `lucide-svelte`, `@xyflow/svelte` (Graph page only).

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples

```svelte
<script lang="ts">
  import { activeWorkflowRun } from '../../stores/workbenchStore';
  import { openRunContextPicker } from './shell/runContextStore';
  import { DimensionLine, EmptyState } from './ui';
</script>

{#if $activeWorkflowRun}
  <DimensionLine label="run_…c1f90ae7, failed 19:35:42 after 22 s" />
{:else}
  <EmptyState
    question="Which run do you want to look into?"
    sentence="Diagnostics shows the events, errors and placement of one run."
    actionLabel="Pick a run"
    onaction={() => openRunContextPicker()}
  />
{/if}
```
