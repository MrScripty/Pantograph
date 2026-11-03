<script lang="ts">
  import {
    HealthMonitorService,
    type HealthMonitorState,
  } from '../../services/HealthMonitorService';
  import { KeyValue, StatusGlyph, type KeyValueRow } from '../workbench/ui';
  import { healthStatusGlyph } from './serverStatusPresenters';

  type Props = {
    healthState: HealthMonitorState;
    showHealthDetails: boolean;
    onCheckNow: () => Promise<void>;
    onTriggerRecovery: () => Promise<void>;
  };

  let {
    healthState,
    showHealthDetails = $bindable(),
    onCheckNow,
    onTriggerRecovery,
  }: Props = $props();

  let healthGlyph = $derived(healthStatusGlyph(healthState.lastResult?.status));

  let healthStatusLabel = $derived(
    healthState.lastResult
      ? HealthMonitorService.getStatusLabel(healthState.lastResult.status)
      : 'Unknown'
  );

  let healthRows = $derived.by((): KeyValueRow[] => {
    const result = healthState.lastResult;
    if (!result) {
      return [];
    }
    const rows: KeyValueRow[] = [
      { key: 'Consecutive failures', value: String(result.consecutive_failures), mono: true },
      { key: 'Last check', value: new Date(result.timestamp).toLocaleTimeString(), mono: true },
    ];
    if (result.error) {
      rows.push({ key: 'Error', value: result.error });
    }
    return rows;
  });
</script>

<div>
  <button
    type="button"
    class="flex w-full items-center gap-2 rounded px-1 py-1 text-left pg-small hover:bg-pg-panel-raised"
    aria-expanded={showHealthDetails}
    onclick={() => (showHealthDetails = !showHealthDetails)}
  >
    <span class="pg-muted">Health</span>
    <StatusGlyph state={healthGlyph} small />
    <span class="min-w-0 flex-1 truncate">{healthStatusLabel}</span>
    {#if healthState.lastResult?.response_time_ms}
      <span class="pg-mono pg-muted">{healthState.lastResult.response_time_ms} ms</span>
    {/if}
    <span class="pg-muted text-[11px]" aria-hidden="true">{showHealthDetails ? '▴' : '▾'}</span>
  </button>

  {#if showHealthDetails}
    <div class="mt-2 flex flex-col gap-2 border-l-2 border-pg-border pl-3">
      {#if healthRows.length > 0}
        <KeyValue rows={healthRows} label="Health check" />
      {/if}

      <div class="flex gap-2">
        <button type="button" onclick={onCheckNow} class="pg-btn pg-btn-sm">
          Check now
        </button>
        {#if healthState.lastResult?.healthy === false}
          <button
            type="button"
            onclick={onTriggerRecovery}
            disabled={healthState.isRecovering}
            class="pg-btn pg-btn-sm"
          >
            {healthState.isRecovering ? 'Recovering...' : 'Recover'}
          </button>
        {/if}
      </div>

      <p class="m-0 flex items-center gap-1.5 pg-muted pg-small">
        <StatusGlyph state={healthState.isRunning ? 'succeeded' : 'idle'} small />
        {healthState.isRunning ? 'Monitoring active' : 'Monitoring inactive'}
      </p>
    </div>
  {/if}
</div>
