<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw } from 'lucide-svelte';
  import type { WorkflowLocalNetworkStatusQueryResponse } from '../../services/workflow/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { activeWorkflowRun } from '../../stores/workbenchStore';
  import {
    buildNetworkFactRows,
    formatCpuUsage,
    formatNetworkBytes,
    formatNetworkTimestamp,
    formatSchedulerLoad,
    formatSessionLoad,
    formatTransportState,
  } from './networkPagePresenters';

  let status = $state<WorkflowLocalNetworkStatusQueryResponse | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let localNode = $derived(status?.local_node ?? null);
  let factRows = $derived(localNode ? buildNetworkFactRows(localNode) : []);

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
        {localNode?.display_name ?? 'Local Pantograph'}
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
    {:else if localNode}
      <div class="space-y-4">
        <div class="flex flex-wrap items-center justify-between gap-3 border-b border-neutral-900 pb-4">
          <div class="min-w-0">
            <div class="truncate text-sm font-semibold text-neutral-100" title={localNode.node_id}>
              {localNode.display_name}
            </div>
            <div class="mt-1 text-xs text-neutral-500">
              Captured {formatNetworkTimestamp(localNode.captured_at_ms)}
            </div>
          </div>
          {#if $activeWorkflowRun}
            <div class="max-w-full truncate rounded border border-neutral-800 px-3 py-1.5 text-xs text-neutral-400">
              Selected run <span class="font-mono text-neutral-200">{$activeWorkflowRun.workflow_run_id}</span>
            </div>
          {/if}
        </div>

        {#if localNode.degradation_warnings.length > 0}
          <div class="rounded border border-amber-900 bg-amber-950/30 p-4">
            <h2 class="text-sm font-semibold text-amber-100">Degraded Metrics</h2>
            <ul class="mt-2 space-y-1 text-sm text-amber-200">
              {#each localNode.degradation_warnings as warning (warning)}
                <li>{warning}</li>
              {/each}
            </ul>
          </div>
        {/if}

        <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
          <div class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
            <div class="text-xs uppercase tracking-[0.22em] text-neutral-500">Transport</div>
            <div class="mt-2 text-lg font-semibold text-neutral-100">
              {formatTransportState(localNode.transport_state)}
            </div>
            <div class="mt-2 text-xs text-neutral-500">{status.peer_nodes.length} peers</div>
          </div>
          <div class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
            <div class="text-xs uppercase tracking-[0.22em] text-neutral-500">CPU</div>
            <div class="mt-2 text-lg font-semibold text-neutral-100">
              {localNode.system.cpu.logical_core_count} cores
            </div>
            <div class="mt-2 text-xs text-neutral-500">
              {formatCpuUsage(localNode.system.cpu.average_usage_percent)}
            </div>
          </div>
          <div class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
            <div class="text-xs uppercase tracking-[0.22em] text-neutral-500">Memory</div>
            <div class="mt-2 text-lg font-semibold text-neutral-100">
              {formatNetworkBytes(localNode.system.memory.used_bytes)}
            </div>
            <div class="mt-2 text-xs text-neutral-500">
              {formatNetworkBytes(localNode.system.memory.available_bytes)} available
            </div>
          </div>
          <div class="rounded border border-neutral-800 bg-neutral-900/60 p-4">
            <div class="text-xs uppercase tracking-[0.22em] text-neutral-500">Scheduler</div>
            <div class="mt-2 text-lg font-semibold text-neutral-100">
              {formatSchedulerLoad(localNode)}
            </div>
            <div class="mt-2 text-xs text-neutral-500">
              {formatSessionLoad(localNode)}
            </div>
          </div>
        </div>

        <div class="grid gap-4 xl:grid-cols-[24rem_minmax(0,1fr)]">
          <section class="rounded border border-neutral-800 bg-neutral-900/50 p-4">
            <h2 class="text-sm font-semibold text-neutral-100">Local Capabilities</h2>
            <dl class="mt-4 space-y-3 text-xs">
              {#each factRows as row (row.label)}
                <div>
                  <dt class="text-neutral-500">{row.label}</dt>
                  <dd class="mt-1 truncate text-neutral-200" title={row.value}>{row.value}</dd>
                </div>
              {/each}
            </dl>
          </section>

          <div class="space-y-4">
            <section class="rounded border border-neutral-800 bg-neutral-900/50">
              <div class="border-b border-neutral-800 px-4 py-3">
                <h2 class="text-sm font-semibold text-neutral-100">Disks</h2>
              </div>
              {#if localNode.system.disks.length === 0}
                <div class="px-4 py-6 text-sm text-neutral-500">Disk metrics unavailable</div>
              {:else}
                <div class="overflow-auto">
                  <table class="w-full min-w-[36rem] text-left text-sm">
                    <thead class="bg-neutral-950 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                      <tr class="border-b border-neutral-800">
                        <th class="px-4 py-3 font-medium">Name</th>
                        <th class="px-3 py-3 font-medium">Mount</th>
                        <th class="px-3 py-3 font-medium">Total</th>
                        <th class="px-4 py-3 font-medium">Available</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-neutral-900">
                      {#each localNode.system.disks as disk (disk.name + disk.mount_point)}
                        <tr>
                          <td class="px-4 py-2 text-neutral-200">{disk.name}</td>
                          <td class="px-3 py-2 font-mono text-xs text-neutral-400">{disk.mount_point}</td>
                          <td class="px-3 py-2 text-neutral-400">{formatNetworkBytes(disk.total_bytes)}</td>
                          <td class="px-4 py-2 text-neutral-400">{formatNetworkBytes(disk.available_bytes)}</td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </section>

            <section class="rounded border border-neutral-800 bg-neutral-900/50">
              <div class="border-b border-neutral-800 px-4 py-3">
                <h2 class="text-sm font-semibold text-neutral-100">Network Interfaces</h2>
              </div>
              {#if localNode.system.network_interfaces.length === 0}
                <div class="px-4 py-6 text-sm text-neutral-500">Interface metrics unavailable</div>
              {:else}
                <div class="overflow-auto">
                  <table class="w-full min-w-[32rem] text-left text-sm">
                    <thead class="bg-neutral-950 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                      <tr class="border-b border-neutral-800">
                        <th class="px-4 py-3 font-medium">Name</th>
                        <th class="px-3 py-3 font-medium">Received</th>
                        <th class="px-4 py-3 font-medium">Transmitted</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-neutral-900">
                      {#each localNode.system.network_interfaces as networkInterface (networkInterface.name)}
                        <tr>
                          <td class="px-4 py-2 text-neutral-200">{networkInterface.name}</td>
                          <td class="px-3 py-2 text-neutral-400">
                            {formatNetworkBytes(networkInterface.total_received_bytes)}
                          </td>
                          <td class="px-4 py-2 text-neutral-400">
                            {formatNetworkBytes(networkInterface.total_transmitted_bytes)}
                          </td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </section>

            <section class="rounded border border-neutral-800 bg-neutral-900/50">
              <div class="border-b border-neutral-800 px-4 py-3">
                <h2 class="text-sm font-semibold text-neutral-100">Peers</h2>
              </div>
              {#if status.peer_nodes.length === 0}
                <div class="px-4 py-6 text-sm text-neutral-500">No trusted peers connected</div>
              {:else}
                <div class="divide-y divide-neutral-900">
                  {#each status.peer_nodes as peer (peer.node_id)}
                    <div class="px-4 py-3">
                      <div class="font-mono text-sm text-neutral-200">{peer.node_id}</div>
                      <div class="mt-1 text-xs text-neutral-500">{formatTransportState(peer.transport_state)}</div>
                    </div>
                  {/each}
                </div>
              {/if}
            </section>
          </div>
        </div>
      </div>
    {:else}
      <div class="text-sm text-neutral-500">Local status unavailable</div>
    {/if}
  </div>
</section>
