# Workbench UI parts

## Purpose

The shared presentation parts of the redesigned workbench. Every page builds
its layout from these so the tabs read as one system: same panels, rows,
glyphs, segments, chips, lanes, log lines and empty states.

## Contents

| File | Responsibility |
| --- | --- |
| `StatusGlyph.svelte`, `statusGlyph.ts` | The glyph vocabulary and the one mapping from backend status strings to glyph states. |
| `DimensionLine.svelte` | Page summary line (rule with ticks and a centred mono label). |
| `Panel.svelte` | Bordered panel; column variant with `top`, scrolling `children` and `footer` snippets. |
| `GroupHeader.svelte`, `ListRow.svelte` | Sticky collapsible group header and the selectable list row. |
| `SegmentedControl.svelte`, `Chip.svelte`, `FilterChip.svelte` | Status segments with counts, static pills, removable filter pills. |
| `DeviceLanes.svelte` | Per-device memory lanes with running, waiting, next and free blocks. |
| `KeyValue.svelte`, `LogLine.svelte`, `ErrorCard.svelte` | Key/value grid, one timeline line, in-place error explanation. |
| `TwoStepButton.svelte`, `EmptyState.svelte`, `Toast.svelte` | Destructive confirm with reset, centred empty state, transient status message. |
| `types.ts`, `index.ts` | Data shapes the parts accept and the barrel export. |

## Problem

The mockups (`docs/design/mockups/boards/*.dc.html`) express status, selection
and grouping the same way on every tab. Eight pages are rebuilt in parallel;
without shared parts each would re-derive the CSS and the status mapping and
drift apart.

## Constraints

- Visual values come from `docs/design/mockups/boards/shell.css`, ported to the
  `pg-*` classes in `src/styles.css` (`@layer components`) and the `--pg-*`
  tokens. Parts use those classes; they do not hard-code colours.
- No pulsing animation, no uppercase tracked labels, no cyan; copy is sentence
  case and uses domain words.
- Native interactive elements only, each with a name; `aria-current` on
  selected rows, `aria-pressed` on toggles and segments, `aria-expanded` on
  disclosures; glyphs are `aria-hidden` and the status is always in text.

## Decision

Parts take data through props and snippets and own no domain state; pages and
their presenters decide what to show. Status strings map to glyphs only in
`statusGlyph.ts`, whose class names are literal so Tailwind keeps them. Page
agents treat this directory as read-only and report needed changes to the
integration owner (plan, Concurrent Work). The heading set of this README is
required by `scripts/check-decision-traceability.sh` (issue WR-001).

## Alternatives Rejected

- A third-party component library: it would bring its own visual language and
  dependency weight for parts that are a few lines of markup each.
- Tailwind utility strings copied into each page: the values would drift and
  the glyph vocabulary would be re-implemented per page.

## Invariants

- `glyphStateForStatus` maps `accepted`, `future`, `scheduled`, `queued`,
  `delayed` to queued; `running` to running; `completed` to succeeded;
  `failed` to failed; `cancelled` to cancelled; `waiting` to waiting; anything
  unknown to idle.
- `ListRow` with a `runId` carries `data-run-id`, so Ctrl+C over it copies the
  id (see `../shell/runContextClipboard.ts`).
- `TwoStepButton` returns to its first step after `resetMs` (4 s), when
  disabled, or when unmounted.
- `Toast` keeps its live region mounted so each message is announced.

## Revisit Triggers

- A backend status or log level appears that the glyph vocabulary cannot
  express.
- Two or more pages need the same composite (for example a filter bar); add it
  here rather than copying it.

## Dependencies

**Internal:** `src/styles.css` (`pg-*` classes and tokens).

**External:** `svelte` (snippets, runes).

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md` (pages using
  these parts still read only typed projections).

## Usage Examples

```svelte
<script lang="ts">
  import { GroupHeader, ListRow, Panel, SegmentedControl, TwoStepButton, glyphStateForStatus } from './ui';
</script>

<Panel label="Runs" width="380px">
  {#snippet top()}
    <SegmentedControl label="Status" options={statusOptions} value={status} onchange={(next) => (status = next)} />
  {/snippet}
  <GroupHeader label="Running" count={running.length} expanded={open} ontoggle={() => (open = !open)} />
  {#each running as run (run.workflow_run_id)}
    <ListRow
      glyph={glyphStateForStatus(run.status)}
      name={run.workflow_id}
      version="v1.4.0"
      statusLabel="Running"
      context=", embed_chunks 61%"
      detail="llama.cpp on cuda:0"
      time="20:05:51"
      runId={run.workflow_run_id}
      selected={run.workflow_run_id === selectedId}
      onselect={() => selectRunContext(run)}
    />
  {/each}
  {#snippet footer()}
    <TwoStepButton label="Cancel run" confirmLabel="Confirm cancel" onconfirm={cancelRun} />
  {/snippet}
</Panel>
```
