# Server status parts

## Purpose

The parts `../ServerStatus.svelte` (the "LLM server" card in Settings) and
`../BackendSelector.svelte` compose: connection mode, external server form,
backend toggles and capability chips, runtime lifecycle snapshots and the
health check disclosure.

## Contents

| File | Responsibility |
| --- | --- |
| `ConnectionModeTabs.svelte` | External or Sidecar, through the shared `SegmentedControl`. |
| `ExternalConnectionPanel.svelte` | Server URL and API key fields, Connect and Disconnect. |
| `BackendOptionList.svelte` | One toggle per backend (`aria-pressed` on the running one) with its availability summary. |
| `BackendCapabilityBadges.svelte` | Capability `Chip`s for the active backend. |
| `RuntimeSnapshotGrid.svelte` | Active and embedding runtime lifecycle rows (`KeyValue`). |
| `HealthStatusPanel.svelte` | Health disclosure: last check, Check now, Recover, monitoring state. |
| `serverStatusPresenters.ts` + `.test.ts` | Pure glyph mapping for the server connection and health states. |

## Problem

The parts carried per-colour Tailwind classes (green, purple, amber, cyan,
blue, red), a spinner and uppercase labels, and `HealthMonitorService` returns
Tailwind colour strings for health. Under the workbench redesign status is a
glyph plus words, and colour alone may not carry meaning.

## Constraints

- Server, config and health state come from `LLMService`, `ConfigService` and
  `HealthMonitorService`; parts receive it through props and own no service
  subscriptions.
- `pg-*` classes and `../workbench/ui` parts only; glyphs are decorative and
  every status is also written out.
- Every input has a visible label; disclosures carry `aria-expanded`.

## Decision

Map server and health status to glyph states in `serverStatusPresenters.ts`
instead of using `HealthMonitorService.getStatusColor` (left in place for any
other caller). Parts keep their props and bindings unchanged so
`ServerStatus.svelte` and `BackendSelector.svelte` need no wiring changes.

## Alternatives Rejected

- Translating `getStatusColor` class strings into glyphs: it couples the view
  to Tailwind colour names the redesign removes.
- Merging the parts back into `ServerStatus.svelte`: the file would pass the
  size and responsibility limits again.

## Invariants

- Props, bindings and callbacks are unchanged; parts do not call services
  except `HealthMonitorService.getStatusLabel` for the health text.
- `serverConnectionGlyph` and `healthStatusGlyph` cover every input
  (`serverStatusPresenters.test.ts`).

## Revisit Triggers

- `HealthStatus` gains a new variant.
- `BackendSelector.svelte` is restyled (it still uses neutral classes and an
  uppercase heading; issue in `docs/plans/workbench-redesign/reports/m9c-settings-panels.md`).

## Dependencies

**Internal:** `src/services/HealthMonitorService.ts`,
`src/services/ConfigService.ts` (types), `../workbench/ui`, `src/styles.css`.

**External:** `svelte`.

## Related ADRs

- `docs/adr/ADR-002-runtime-registry-ownership-and-lifecycle.md` (runtime
  lifecycle snapshots shown by `RuntimeSnapshotGrid`).

## Usage Examples

```svelte
<script lang="ts">
  import ConnectionModeTabs from './server-status/ConnectionModeTabs.svelte';
  import HealthStatusPanel from './server-status/HealthStatusPanel.svelte';

  let connectionType: 'external' | 'sidecar' = $state('sidecar');
  let showHealthDetails = $state(false);
</script>

<ConnectionModeTabs bind:connectionType />
<HealthStatusPanel {healthState} bind:showHealthDetails onCheckNow={check} onTriggerRecovery={recover} />
```
