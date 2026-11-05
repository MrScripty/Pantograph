# M4 Network page report

Agent: Opus 5.5 page agent, 2026-11-03. Board: `docs/design/mockups/boards/Network.dc.html`.

## Changed files

| File | Change |
| --- | --- |
| `src/components/workbench/NetworkPage.svelte` | Rewritten to the board: dimension line, 340px Nodes panel (segment, Local and Peers groups, footer), node detail panel (header, chips, four stat cells, Devices, selected run on this node with Execution and Resources tables, collapsible Events log, Disks and Interfaces, Identity), peer detail. Same service calls, same refresh ownership. |
| `src/components/workbench/networkPagePresenters.ts` | New pure projection for the layout: node list entries and transport glyphs, selected-node fallback, dimension line, degradation routing, stat cells, device/disk/interface/identity rows, peer detail, selected-run placement (local and peer), execution/resource rows with glyphs, timeline events as log lines, failure sentences. Removed presenters that only the old page used, including the Tailwind cyan/amber class helpers. |
| `src/components/workbench/networkPagePresenters.test.ts` | 16 tests replacing the old 8 (see Verification). |
| `docs/plans/workbench-redesign/reports/tools/fixture-network.js` | New walkthrough fixture (per the caller's instruction, next to `fixture.js`). Wraps the base host: delegates `workflow_run_list_query` to `fixture.js` and derives placements from those runs; answers `workflow_local_network_status_query`, `workflow_scheduler_timeline_query`, `workflow_node_status_query`, `workflow_library_usage_query`, `plugin:event|listen` and `plugin:event|unlisten`; one local node and three peers (connected, degraded, pairing required). |
| `docs/plans/workbench-redesign/reports/m4-network.md` | This report. |

## What the page does

- Data authority unchanged: `queryLocalNetworkStatus` once on mount and on Refresh;
  `querySchedulerTimeline`, `queryLibraryUsage`, `queryNodeStatus` for the active
  run, re-run when the active run changes and on `scheduler_timeline`,
  `library_usage`, `node_status` invalidations through the one existing
  subscription (owned by `onMount`, removed on destroy, late unlisten handled).
  The only lifecycle change: the status query now has the same per-scope
  request serial as the other three queries, so an overlapping older status
  result is not applied. Subscription failures now have their own state instead
  of overwriting the status error.
- Left list: local node first (glyph running, "local" on the right), then peers
  in projection order. Transport glyphs: connected → succeeded, degraded →
  cancelled, pairing required → queued, local only → running,
  peer networking unavailable → idle. The right column shows the peer's
  last-seen time (latency is not in the projection). "Shared with me" and "Mine"
  are disabled native buttons with a title; "Pair a node" is disabled with a
  title; "Trusted peers only" is muted text. Groups use `GroupHeader`
  (`aria-expanded`, `aria-controls`); rows use `ListRow` (`aria-current`).
  If a selected peer disappears from the projection, the selection falls back
  to the local node.
- Dimension line: "N nodes, c connected, d degraded, p waiting to be paired."
  The GPU count and VRAM part is omitted because the projection does not report
  it. The local node counts as connected unless its transport is degraded.
- Devices (WR-006): GPU availability and reason as a `KeyValue`, then the muted
  sentence "Per-device memory is not reported by this engine build." A comment
  at that spot names `DeviceLanes` as the component for when device memory
  exists. No lanes are rendered.
- Selected run on the local node: "<workflow> on this node" with Run (mono,
  `data-run-id`, Ctrl+C copies), State, and the placement facts (runtime
  session and model cache, session id, backends, models). When the run has no
  local placement, a muted sentence explains that the local status lists only
  running and queued runs. Execution and Resources keep their tables
  (`pg-tbl`, status glyph plus text) with the projection cursor next to each
  sub-heading. Events are `LogLine`s in a `role="log"` container behind a
  disclosure button (collapsed by default). With no run selected, the section
  says so and offers "Pick a run" (opens the shared picker).
- Peer detail: title, node id, last seen, transport chip, Refresh, "Forget node"
  disabled with a title; a Details sentence saying this build reports only
  transport and last-seen for peers (or that a pairing-required node stays
  hidden until paired); the selected run as a sentence (running or queued on
  the local node, or "this engine build does not report run placement on
  peers"); Identity.
- Degraded metrics: the projection's free-text warnings are routed by keyword to
  the section they qualify (CPU and memory cells, Devices, Disks, Interfaces;
  anything else under the header) and render as muted text there. A warning
  that repeats the GPU reason is dropped because Devices already shows it.
- Errors: status, node-status, library-usage and timeline failures render as
  muted sentences in the panel or section they affect; a failed refresh keeps
  the last status and says which capture is shown. No banners, no red. The
  shell owns the engine-unavailable line and dimming.
- Removed: lucide spinner (`animate-spin`), uppercase tracked labels, cyan,
  amber and red panels.

## Verification

Run from the repo root on 2026-11-03.

```
$ npm run typecheck
> Pantograph@0.0.0 typecheck
> tsc --project tsconfig.json --noEmit --pretty false
(exit 0, no diagnostics)
```

(An earlier run during the session failed only in `diagnosticsPagePresenters.test.ts`,
M2's files mid-edit; the final run is clean.)

```
$ npm run test:frontend
ℹ tests 675
ℹ pass 675
ℹ fail 0
```

Network tests (`node --experimental-strip-types --test src/components/workbench/networkPagePresenters.test.ts`):
16 pass, covering formatters (bytes up to TiB, clock), transport glyph mapping,
list order and local-first, selected-node fallback, dimension line counts and
omissions, degradation routing and de-duplication, stat cells including
unreported CPU usage and zero memory total, device/disk/interface/identity rows,
peer detail for connected and pairing-required peers, selected-run placement
(placed, from placement workflow id, finished, listed without placement), peer
run sentence, execution glyphs, resource rows, timeline log lines and levels,
failure sentences.

```
$ npm run lint:a11y
svelte accessibility gate failed
src/components/workbench/IoInspectorPage.svelte:1024 [reviewed-a11y-ignore] Svelte a11y ignore comments require an adjacent a11y-reviewed reason.

$ npm run lint:critical
critical anti-pattern gate failed
src/components/workbench/IoInspectorPage.svelte:508:20 [no-append-remove-child] Disallow appendChild/removeChild DOM mutation
```

Both failures are in M3's file (WR-007); nothing reported for the Network files.

```
$ npx eslint src/components/workbench/NetworkPage.svelte src/components/workbench/networkPagePresenters.ts src/components/workbench/networkPagePresenters.test.ts --max-warnings 0
(exit 0)

$ TRACEABILITY_BASE_REF=codex/pantograph-rpc-workflow-2026-09-26 ./scripts/check-decision-traceability.sh
No changed files detected for decision traceability check.
(exit 0; per WR-003 range mode proves nothing for uncommitted work. No new
directory was added; src/components/workbench/README.md exists.)

$ npx vite build --outDir <scratch>/m4/dist-real
✓ built in 9.75s
```

The first real-tree build attempts failed in `DiagnosticsPage.svelte` (M2
mid-edit: `filterDiagnosticsTimelineEvents` not exported). Interim screenshots
came from a scratch copy with only the two Diagnostics files restored from
HEAD; the final build and all screenshots below are from the real working tree.
No Svelte compiler warnings mention `NetworkPage`.

Walkthrough: `python3 -m http.server 8794 --directory <scratch>/m4/dist-real`,
`CDP_PORT=9344 node cdp-init.cjs <scratch>/m4/steps-network.json` with
`fixture.js` then `fixture-network.js`; server stopped afterwards. Logged
checks: sections `Devices`, `ingest-docs-rag on this node`, `Events 7`,
`Disks`, `Interfaces`, `Identity`; Events disclosure toggled to
`aria-expanded="true"` with 7 log lines; the detail panel scrolls inside
(`scrollHeight > clientHeight`) while the document stays 900px tall for a 900px
viewport; disabled controls and titles:
`Shared with me | The network status does not say who owns a peer yet, so nodes cannot be filtered by owner.`,
`Mine | (same)`, `Pair a node | Pairing is not available in this build yet.`,
`Forget node | Forgetting a peer is not available in this build yet.`

## Screenshots

All under `/tmp/claude-1000/-media-jeremy-OrangeCream-Linux-Software-repos-owned-ai-systems-Pantograph/26f5ffd8-b094-44e6-bf07-1b07d5fc8def/scratchpad/m4/`:

- `network-no-run.png`: local node, no run selected (fixture).
- `network-local-run.png`: local node with `ingest-docs-rag` selected: stats with the CPU degradation note, Devices sentence, placement, Execution table.
- `network-events.png`: scrolled detail: Resources table, Events expanded (one warn line), Disks, Interfaces, Identity.
- `network-local-bottom.png`: detail scrolled to the end.
- `network-peer.png`: degraded peer (Laptop) with the selected-run sentence.
- `network-pairing.png`: pairing-required peer.
- `network-status-failed.png`: base fixture only (status query rejected): muted sentence in both panels, engine line online.
- `network-plain-browser.png`: no host at all: shell shows "Engine unreachable", page shows the muted sentence.
- `board.png`: the board rendered for comparison.

## Ledger entry text

> **2026-11-03, M4 Network (Opus 5.5 page agent).** `NetworkPage.svelte`
> rebuilt to the Network board from the existing projections (local network
> status, node status, library usage, scheduler timeline); refresh ownership
> unchanged except a request serial on the status query. New pure presenters in
> `networkPagePresenters.ts` with 16 tests (675 frontend tests pass). Devices
> show GPU availability and reason with the WR-006 sentence; no lanes. Peers
> listed with transport glyphs; owner segments, pairing and forget are disabled
> with titles. Degraded metrics render as muted text inside the affected
> section. Gates: typecheck pass; eslint on the three files pass; lint:a11y and
> lint:critical fail only on WR-007 (IoInspectorPage); traceability "No changed
> files" (WR-003); vite build pass. Headless CDP walkthrough with
> `reports/tools/fixture-network.js`: screenshots of the local node with and
> without a run, events expanded, a degraded and a pairing-required peer, the
> failed-status and plain-browser states (paths in `reports/m4-network.md`).

## Issue rows

IDs are placeholders for the integration owner to number.

| ID | Severity | Evidence | Relationship | Owner / boundary | Disposition | Verification | Revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WR-M4-1 | medium | `WorkflowPeerNetworkNodeStatus` is `{ node_id, display_name, transport_state, last_seen_at_ms }` (`src/services/workflow/types.ts`); the backend returns `peer_nodes: Vec::new()` (`crates/pantograph-workflow-service/src/workflow/local_network_api.rs`). No peer CPU, memory, devices, disks, interfaces, latency, ownership or run placement | Network peers (A2) | backend network status projection | defer: peer rows say "Metrics not reported for peers"; the right column shows last-seen time instead of latency; peer detail explains in one sentence; "Shared with me" and "Mine" are disabled with a title; the selected run on a peer is a sentence that placement on peers is not reported | `network-peer.png`, `network-pairing.png`; presenter tests for peer entries and the peer run sentence | projection adds peer metrics, latency, ownership or remote placement |
| WR-M4-2 | low | No pairing or forget-peer command exists in `WorkflowService` / `WorkflowProjectionService` | Network footer and peer header actions | backend peer management | defer: "Pair a node" and "Forget node" render disabled with a title | walkthrough log of disabled controls and titles | a pairing or forget command is added |
| WR-M4-3 | low | `degradation_warnings` is `string[]` without a section or code; the page routes warnings by keyword (gpu/cuda/device → Devices, peer/transport → node, cpu, memory, disk, network/interface) in `groupDegradationWarnings` | Degraded metrics placement | backend network status projection | defer: keyword routing with unmatched warnings under the header; covered by tests | `groupDegradationWarnings` test | the projection gives warnings a typed section or code |
| WR-006 (evidence) | n/a | Network Devices shows "Per-device memory is not reported by this engine build." under the GPU availability KeyValue; the dimension line omits the GPU/VRAM clause | existing WR-006 | n/a | add the screenshot as WR-006 verification for M4 | `network-local-run.png` | unchanged |

## Shared-part changes requested from the integration owner

None were made; the page works around each locally.

1. `SegmentedControl`: per-option `disabled` and `title` in `SegmentOption`. The
   page renders its own `pg-seg` group to disable "Shared with me" and "Mine".
2. `src/styles.css`: a `.pg-seg button:disabled` rule (muted, `not-allowed`, no
   hover tint). The page carries a scoped copy (`.net-seg button:disabled`).
3. `KeyValueRow`: an optional `runId` that adds `data-run-id` to the value. The
   run section renders its own `dl.pg-kv` so the run id copies with Ctrl+C.
4. `pg-tbl`: a non-sticky variant for small tables inside a scrolling detail
   panel (the board uses `position: static` on those headers). The page sets
   `.net-tbl th { position: static }`.

## Notes for integration

- `networkPagePresenters.ts` still imports `formatLibraryAssetCategory` and
  `formatLibraryBytes` from `libraryUsagePresenters.ts` (M7's write set), as the
  old page did. If M7 renames them, the Network presenters need the new names.
  The scheduler presenter imports the old page used are gone; freshness comes
  from `projectionFreshness.ts`.
- Execution rows show "Runtime unavailable" / "Model unavailable" for nodes with
  no runtime or model, as before; the board has no equivalent table.
