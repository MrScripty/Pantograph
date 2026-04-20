<script lang="ts">
  import {
    HealthMonitorService,
    type HealthMonitorState,
  } from '../../services/HealthMonitorService';

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

  let healthStatusColor = $derived(
    healthState.lastResult
      ? HealthMonitorService.getStatusColor(healthState.lastResult.status)
      : 'text-neutral-500'
  );

  let healthStatusLabel = $derived(
    healthState.lastResult
      ? HealthMonitorService.getStatusLabel(healthState.lastResult.status)
      : 'Unknown'
  );
</script>

<div>
  <button
    type="button"
    class="w-full rounded px-1 py-1 text-xs transition-colors hover:bg-neutral-800/50"
    onclick={() => (showHealthDetails = !showHealthDetails)}
  >
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <span class="text-neutral-500">Health:</span>
        <span class={healthStatusColor}>{healthStatusLabel}</span>
        {#if healthState.lastResult?.response_time_ms}
          <span class="text-[10px] text-neutral-600">
            ({healthState.lastResult.response_time_ms}ms)
          </span>
        {/if}
      </div>
      <svg
        class={`h-3 w-3 text-neutral-500 transition-transform ${showHealthDetails ? 'rotate-180' : ''}`}
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M19 9l-7 7-7-7"
        />
      </svg>
    </div>
  </button>

  {#if showHealthDetails}
    <div class="mt-2 space-y-2 border-l-2 border-neutral-700 pl-2">
      {#if healthState.lastResult}
        <div class="space-y-1 text-[10px] text-neutral-500">
          <div>Consecutive failures: {healthState.lastResult.consecutive_failures}</div>
          <div>Last check: {new Date(healthState.lastResult.timestamp).toLocaleTimeString()}</div>
          {#if healthState.lastResult.error}
            <div class="text-red-400">Error: {healthState.lastResult.error}</div>
          {/if}
        </div>
      {/if}

      <div class="flex gap-2">
        <button
          type="button"
          onclick={onCheckNow}
          class="flex-1 rounded bg-neutral-700 px-2 py-1 text-[10px] transition-colors hover:bg-neutral-600"
        >
          Check Now
        </button>
        {#if healthState.lastResult?.healthy === false}
          <button
            type="button"
            onclick={onTriggerRecovery}
            disabled={healthState.isRecovering}
            class="flex-1 rounded bg-yellow-700 px-2 py-1 text-[10px] transition-colors hover:bg-yellow-600 disabled:opacity-50"
          >
            {healthState.isRecovering ? 'Recovering...' : 'Recover'}
          </button>
        {/if}
      </div>

      <div class="text-[10px] text-neutral-600">
        {#if healthState.isRunning}
          <span class="text-green-500">● Monitoring active</span>
        {:else}
          <span class="text-neutral-500">○ Monitoring inactive</span>
        {/if}
      </div>
    </div>
  {/if}
</div>
