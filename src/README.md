# Frontend source root

## Purpose

The Svelte 5 frontend of the Pantograph desktop app (Tauri 2 webview). This
root holds the mount point, the global stylesheet with the workbench design
tokens, and the directories for components, stores, services and generated UI.

## Contents

| Path | Responsibility |
| --- | --- |
| `main.ts`, `App.svelte` | Mount the app; `App.svelte` renders the workbench shell, the Assistant side panel and global overlays. |
| `styles.css` | Fonts, `--pg-*` design tokens, global scrollbar and focus styles, and the `pg-*` component classes the workbench parts use. |
| `components/` | UI components; `components/workbench/` is the run-centric workbench (see its README). |
| `stores/` | Svelte stores for frontend-owned state (see `stores/README.md`). |
| `services/` | Typed clients for Tauri commands and events, including the workflow projection services. |
| `generated/` | Runtime-authored components; excluded from tests and lint rules that target hand-written code. |

## Problem

The webview needs one place that defines how the app looks (tokens, fonts,
focus and scrollbars) and one boundary through which it talks to the Rust
host, so pages can be rebuilt independently without forking either.

## Constraints

- The desktop app must render the same offline, so fonts are bundled from
  `@fontsource/*` (SIL OFL 1.1) rather than loaded from a CDN.
- Colours come from the `--pg-*` tokens in `styles.css`; Tailwind exposes them
  as `pg-*` colours (`tailwind.config.cjs`). The `neutral` scale stays only for
  pages not yet restyled and is tuned to the same greys; no `!important`
  colour overrides.
- Host access goes through `services/`; components do not call `invoke`
  directly for workflow facts.

## Decision

Global styling lives in `styles.css` only: tokens on `:root`, element
baselines, and `pg-*` primitives in `@layer components` ported from
`docs/design/mockups/boards/shell.css`. The heading set of this README is
required by `scripts/check-decision-traceability.sh` (issue WR-001 in
`docs/plans/workbench-redesign/issues.md`).

## Alternatives Rejected

- Keeping the `!important` neutral overrides: they fought every new class and
  hid which palette a page was using.
- System font stacks only: they render differently per machine; they remain
  the fallback in each `font-family` stack.

## Invariants

- `:focus-visible` draws a 2px accent outline on every focusable element.
- Scrollbars are thin and themed everywhere.
- The `body` background is `var(--pg-ground)`; there is no gradient body
  background.
- One remaining `!important` block in `styles.css` targets package graph
  components (`.svelte-flow`, `.base-node`, ...) until the Graph milestone
  restyles them.

## Revisit Triggers

- A light theme or user-selectable accent is requested (tokens would need a
  second set).
- The Graph milestone removes the literal darks in `packages/svelte-graph`, so
  the remaining override block can go.

## Dependencies

**Internal:** `packages/svelte-graph` (graph editor components).

**External:** `svelte`, `@tauri-apps/api`, `tailwindcss`, `lucide-svelte`,
`@fontsource/newsreader`, `@fontsource/atkinson-hyperlegible`,
`@fontsource/jetbrains-mono`.

## Related ADRs

- `docs/adr/ADR-014-run-centric-workbench-projection-boundary.md`

## Usage Examples

```svelte
<!-- Token-backed Tailwind colours and the shared component classes. -->
<section class="pg-panel">
  <h2 class="pg-title">ingest-docs-rag <span class="pg-muted">v1.4.0</span></h2>
  <p class="text-pg-text-muted pg-small">Queued, #2 in queue</p>
  <button class="pg-btn pg-btn-primary" type="button">Pick a run</button>
</section>
```

## API Consumer Contract

- The frontend consumes the Tauri commands and events of the Rust host through
  `services/` (for example `workflow_run_list_query` via
  `workflowService.queryRunList`) and treats their typed responses as the only
  authority for workflow, scheduler and diagnostics facts (ADR-014).
- Failures arrive as `WorkflowServiceError`; code `transport_error` means the
  host could not be reached, and the workbench then shows the engine as
  unreachable and keeps the last projection on screen.
- Outside a Tauri host (plain browser) every command fails with
  `transport_error`; the app still renders its shell and empty or error states.

## Structured Producer Contract

- The frontend produces typed command requests for the host
  (`services/workflow/*`), shaped by the request types in
  `services/diagnostics/types.ts` and `services/workflow/types.ts`; it never
  writes ledger rows or projections itself.
- `styles.css` produces the `--pg-*` custom properties and `pg-*` classes that
  components consume; renaming or removing one is a coordinated change across
  `components/workbench/ui/` and the pages using it.
