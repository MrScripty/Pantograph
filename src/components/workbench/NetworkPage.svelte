<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw } from 'lucide-svelte';
  import type { WorkflowLocalNetworkStatusQueryResponse } from '../../services/workflow/types';
  import { workflowService } from '../../services/workflow/WorkflowService';

  let status = $state<WorkflowLocalNetworkStatusQueryResponse | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  function formatBytes(bytes: number): string {
    if (bytes >= 1_073_741_824) {
      return `${(bytes / 1_073_741_824).toFixed(1)} GiB`;
    }
    if (bytes >= 1_048_576) {
      return `${(bytes / 1_048_576).toFixed(1)} MiB`;
    }
    if (bytes >= 1_024) {
      return `${(bytes / 1_024).toFixed(1)} KiB`;
    }
    return `${bytes} B`;
  }

  async function refreshStatus(): Promise<void> {
    loading = true;
    error = null;
    try {
      status = await workflowService.queryLocalNetworkStatus({
        include_disks: true,
        include_network_interfaces: true,
      });
    } catch (statusError) {
      error = statusError instanceof Error ? statusError.message : String(statusError);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void refreshStatus();
  });
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950">
  <div class="flex shrink-0 items-center justify-between border-b border-neutral-800 px-4 py-3">
    <div>
      <h1 class="text-base font-semibold text-neutral-100">Network</h1>
      <div class="mt-1 text-xs text-neutral-500">
        {status?.local_node.display_name ?? 'Local Pantograph'}
      </div>
    </div>
    <button
      type="button"
      class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-sm text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
      onclick={() => refreshStatus()}
      disabled={loading}
    >
      <RefreshCw size={14} aria-hidden="true" class={loading ? 'animate-spin' : ''} />
      Refresh
    </button>
  </div>

  {#if error}
    <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{error}</div>
  {/if}

  <div class="min-h-0 flex-1 overflow-auto p-6">
    {#if !status && loading}
      <div class="text-sm text-neutral-500">Loading local status</div>
    {:else if status}
      <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <div class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
          <div class="text-xs uppercase tracking-[0.22em] text-neutral-500">Transport</div>
          <div class="mt-2 text-lg font-semibold text-neutral-100">{status.local_node.transport_state}</div>
          <div class="mt-2 text-xs text-neutral-500">{status.peer_nodes.length} peers</div>
        </div>
        <div class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
          <div class="text-xs uppercase tracking-[0.22em] text-neutral-500">CPU</div>
          <div class="mt-2 text-lg font-semibold text-neutral-100">
            {status.local_node.system.cpu.logical_core_count} cores
          </div>
          <div class="mt-2 text-xs text-neutral-500">
            {status.local_node.system.cpu.average_usage_percent?.toFixed(1) ?? '0.0'}% average
          </div>
        </div>
        <div class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
          <div class="text-xs uppercase tracking-[0.22em] text-neutral-500">Memory</div>
          <div class="mt-2 text-lg font-semibold text-neutral-100">
            {formatBytes(status.local_node.system.memory.used_bytes)}
          </div>
          <div class="mt-2 text-xs text-neutral-500">
            {formatBytes(status.local_node.system.memory.available_bytes)} available
          </div>
        </div>
        <div class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
          <div class="text-xs uppercase tracking-[0.22em] text-neutral-500">Scheduler</div>
          <div class="mt-2 text-lg font-semibold text-neutral-100">
            {status.local_node.scheduler_load.active_run_count} active
          </div>
          <div class="mt-2 text-xs text-neutral-500">
            {status.local_node.scheduler_load.queued_run_count} queued
          </div>
        </div>
      </div>
    {:else}
      <div class="text-sm text-neutral-500">Local status unavailable</div>
    {/if}
  </div>
</section>
