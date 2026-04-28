<script lang="ts">
  import { onMount } from 'svelte';
  import { RefreshCw } from 'lucide-svelte';
  import type {
    LibraryUsageProjectionRecord,
    NodeStatusProjectionRecord,
    ProjectionStateRecord,
    SchedulerTimelineProjectionRecord,
  } from '../../services/diagnostics/types';
  import type { WorkflowLocalNetworkStatusQueryResponse } from '../../services/workflow/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { activeWorkflowRun } from '../../stores/workbenchStore';
  import {
    buildNetworkFactRows,
    buildSelectedRunExecutionRows,
    buildSelectedRunPlacementRows,
    buildSelectedRunResourceRows,
    formatCpuUsage,
    formatNetworkBytes,
    formatNetworkProjectionFreshness,
    formatNetworkTimestamp,
    formatSchedulerLoad,
    findSelectedRunPlacement,
    formatSelectedRunLocalState,
    formatSessionLoad,
    formatTransportState,
  } from './networkPagePresenters';
  import { formatLibraryProjectionFreshness } from './libraryUsagePresenters';
  import {
    formatSchedulerProjectionFreshness,
    formatSchedulerTimelineKind,
    formatSchedulerTimelineSource,
    formatSchedulerTimestamp,
    schedulerTimelinePayloadLabel,
  } from './schedulerPagePresenters';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

  let status = $state<WorkflowLocalNetworkStatusQueryResponse | null>(null);
  let selectedRunResources = $state<LibraryUsageProjectionRecord[]>([]);
  let selectedRunResourceProjectionState = $state<ProjectionStateRecord | null>(null);
  let selectedRunNodeStatuses = $state<NodeStatusProjectionRecord[]>([]);
  let selectedRunNodeStatusProjectionState = $state<ProjectionStateRecord | null>(null);
  let timelineEvents = $state<SchedulerTimelineProjectionRecord[]>([]);
  let timelineProjectionState = $state<ProjectionStateRecord | null>(null);
  let loading = $state(false);
  let resourceLoading = $state(false);
  let nodeStatusLoading = $state(false);
  let timelineLoading = $state(false);
  let error = $state<string | null>(null);
  let resourceError = $state<string | null>(null);
  let nodeStatusError = $state<string | null>(null);
  let timelineError = $state<string | null>(null);
  let localNode = $derived(status?.local_node ?? null);
  let factRows = $derived(localNode ? buildNetworkFactRows(localNode) : []);
  let selectedRunPlacement = $derived(
    localNode ? findSelectedRunPlacement(localNode, $activeWorkflowRun?.workflow_run_id) : null,
  );
  let selectedRunPlacementRows = $derived(buildSelectedRunPlacementRows(selectedRunPlacement));
  let selectedRunResourceRows = $derived(buildSelectedRunResourceRows(selectedRunResources));
  let selectedRunExecutionRows = $derived(buildSelectedRunExecutionRows(selectedRunNodeStatuses));
  let resourceRequestSerial = 0;
  let nodeStatusRequestSerial = 0;
  let timelineRequestSerial = 0;

  function activeRunId(): string | null {
    return $activeWorkflowRun?.workflow_run_id ?? null;
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
      error = formatWorkflowCommandError(statusError);
    } finally {
      loading = false;
    }
  }

  async function refreshTimeline(runId = activeRunId()): Promise<void> {
    const requestSerial = ++timelineRequestSerial;
    timelineError = null;

    if (!runId) {
      timelineEvents = [];
      timelineProjectionState = null;
      timelineLoading = false;
      return;
    }

    timelineLoading = true;
    try {
      const response = await workflowService.querySchedulerTimeline({
        workflow_run_id: runId,
        limit: 24,
      });
      if (requestSerial !== timelineRequestSerial) {
        return;
      }
      timelineEvents = response.events;
      timelineProjectionState = response.projection_state;
    } catch (refreshError) {
      if (requestSerial !== timelineRequestSerial) {
        return;
      }
      timelineError = formatWorkflowCommandError(refreshError);
      timelineEvents = [];
      timelineProjectionState = null;
    } finally {
      if (requestSerial === timelineRequestSerial) {
        timelineLoading = false;
      }
    }
  }

  async function refreshSelectedRunResources(runId = activeRunId()): Promise<void> {
    const requestSerial = ++resourceRequestSerial;
    resourceError = null;

    if (!runId) {
      selectedRunResources = [];
      selectedRunResourceProjectionState = null;
      resourceLoading = false;
      return;
    }

    resourceLoading = true;
    try {
      const response = await workflowService.queryLibraryUsage({
        workflow_run_id: runId,
        limit: 24,
      });
      if (requestSerial !== resourceRequestSerial) {
        return;
      }
      selectedRunResources = response.assets;
      selectedRunResourceProjectionState = response.projection_state;
    } catch (refreshError) {
      if (requestSerial !== resourceRequestSerial) {
        return;
      }
      resourceError = formatWorkflowCommandError(refreshError);
      selectedRunResources = [];
      selectedRunResourceProjectionState = null;
    } finally {
      if (requestSerial === resourceRequestSerial) {
        resourceLoading = false;
      }
    }
  }

  async function refreshSelectedRunNodeStatuses(runId = activeRunId()): Promise<void> {
    const requestSerial = ++nodeStatusRequestSerial;
    nodeStatusError = null;

    if (!runId) {
      selectedRunNodeStatuses = [];
      selectedRunNodeStatusProjectionState = null;
      nodeStatusLoading = false;
      return;
    }

    nodeStatusLoading = true;
    try {
      const response = await workflowService.queryNodeStatus({
        workflow_run_id: runId,
        limit: 48,
      });
      if (requestSerial !== nodeStatusRequestSerial) {
        return;
      }
      selectedRunNodeStatuses = response.nodes;
      selectedRunNodeStatusProjectionState = response.projection_state;
    } catch (refreshError) {
      if (requestSerial !== nodeStatusRequestSerial) {
        return;
      }
      nodeStatusError = formatWorkflowCommandError(refreshError);
      selectedRunNodeStatuses = [];
      selectedRunNodeStatusProjectionState = null;
    } finally {
      if (requestSerial === nodeStatusRequestSerial) {
        nodeStatusLoading = false;
      }
    }
  }

  async function refreshNetworkPage(): Promise<void> {
    await Promise.all([
      refreshStatus(),
      refreshTimeline(),
      refreshSelectedRunResources(),
      refreshSelectedRunNodeStatuses(),
    ]);
  }

  onMount(() => {
    void refreshStatus();
  });

  $effect(() => {
    const runId = activeRunId();
    void refreshTimeline(runId);
    void refreshSelectedRunResources(runId);
    void refreshSelectedRunNodeStatuses(runId);
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
      onclick={() => refreshNetworkPage()}
      disabled={loading || timelineLoading || resourceLoading || nodeStatusLoading}
    >
      <RefreshCw
        size={14}
        aria-hidden="true"
        class={loading || timelineLoading || resourceLoading || nodeStatusLoading ? 'animate-spin' : ''}
      />
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
              <span class="font-mono text-neutral-200">{$activeWorkflowRun.workflow_run_id}</span>
              <span class="ml-2 text-neutral-500">{formatSelectedRunLocalState(localNode, $activeWorkflowRun.workflow_run_id)}</span>
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
            {#if $activeWorkflowRun}
              <section class="rounded border border-neutral-800 bg-neutral-900/50 p-4">
                <h2 class="text-sm font-semibold text-neutral-100">Selected Run Placement</h2>
                <dl class="mt-4 grid gap-3 text-xs sm:grid-cols-2">
                  {#each selectedRunPlacementRows as row (row.label)}
                    <div>
                      <dt class="text-neutral-500">{row.label}</dt>
                      <dd
                        class={`mt-1 truncate text-neutral-200 ${row.mono ? 'font-mono' : ''}`}
                        title={row.value}
                      >
                        {row.value}
                      </dd>
                    </div>
                  {/each}
                </dl>
              </section>

              <section class="rounded border border-neutral-800 bg-neutral-900/50">
                <div class="flex items-start justify-between gap-3 border-b border-neutral-800 px-4 py-3">
                  <div class="min-w-0">
                    <h2 class="text-sm font-semibold text-neutral-100">Selected Run Execution</h2>
                    <div class="mt-1 truncate text-xs text-neutral-500">
                      {formatNetworkProjectionFreshness(selectedRunNodeStatusProjectionState)}
                    </div>
                  </div>
                  {#if nodeStatusLoading}
                    <RefreshCw size={12} aria-hidden="true" class="mt-1 shrink-0 animate-spin text-neutral-500" />
                  {/if}
                </div>
                {#if nodeStatusError}
                  <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{nodeStatusError}</div>
                {/if}
                {#if nodeStatusLoading && selectedRunExecutionRows.length === 0}
                  <div class="px-4 py-6 text-sm text-neutral-500">Loading selected-run execution state</div>
                {:else if selectedRunExecutionRows.length === 0}
                  <div class="px-4 py-6 text-sm text-neutral-500">No selected-run node status projected</div>
                {:else}
                  <div class="overflow-auto">
                    <table class="w-full min-w-[42rem] text-left text-sm">
                      <thead class="bg-neutral-950 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                        <tr class="border-b border-neutral-800">
                          <th class="px-4 py-3 font-medium">Node</th>
                          <th class="px-3 py-3 font-medium">Status</th>
                          <th class="px-3 py-3 font-medium">Runtime</th>
                          <th class="px-4 py-3 font-medium">Model</th>
                        </tr>
                      </thead>
                      <tbody class="divide-y divide-neutral-900">
                        {#each selectedRunExecutionRows as row (row.nodeId)}
                          <tr>
                            <td class="max-w-[18rem] px-4 py-2">
                              <div class="truncate font-mono text-xs text-neutral-200" title={row.nodeId}>{row.nodeId}</div>
                            </td>
                            <td class="px-3 py-2 text-neutral-400">{row.status}</td>
                            <td class="max-w-[18rem] px-3 py-2">
                              <div class="truncate text-neutral-400" title={row.runtime}>{row.runtime}</div>
                            </td>
                            <td class="max-w-[18rem] px-4 py-2">
                              <div class="truncate text-neutral-400" title={row.model}>{row.model}</div>
                            </td>
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  </div>
                {/if}
              </section>

              <section class="rounded border border-neutral-800 bg-neutral-900/50">
                <div class="flex items-start justify-between gap-3 border-b border-neutral-800 px-4 py-3">
                  <div class="min-w-0">
                    <h2 class="text-sm font-semibold text-neutral-100">Selected Run Resources</h2>
                    <div class="mt-1 truncate text-xs text-neutral-500">
                      {formatLibraryProjectionFreshness(selectedRunResourceProjectionState)}
                    </div>
                  </div>
                  {#if resourceLoading}
                    <RefreshCw size={12} aria-hidden="true" class="mt-1 shrink-0 animate-spin text-neutral-500" />
                  {/if}
                </div>
                {#if resourceError}
                  <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{resourceError}</div>
                {/if}
                {#if resourceLoading && selectedRunResourceRows.length === 0}
                  <div class="px-4 py-6 text-sm text-neutral-500">Loading selected-run resources</div>
                {:else if selectedRunResourceRows.length === 0}
                  <div class="px-4 py-6 text-sm text-neutral-500">No selected-run Library usage projected</div>
                {:else}
                  <div class="overflow-auto">
                    <table class="w-full min-w-[42rem] text-left text-sm">
                      <thead class="bg-neutral-950 text-[11px] uppercase tracking-[0.18em] text-neutral-500">
                        <tr class="border-b border-neutral-800">
                          <th class="px-4 py-3 font-medium">Asset</th>
                          <th class="px-3 py-3 font-medium">Category</th>
                          <th class="px-3 py-3 font-medium">Cache</th>
                          <th class="px-3 py-3 font-medium">Network</th>
                          <th class="px-4 py-3 font-medium">Run Access</th>
                        </tr>
                      </thead>
                      <tbody class="divide-y divide-neutral-900">
                        {#each selectedRunResourceRows as resource (resource.assetId)}
                          <tr>
                            <td class="max-w-[20rem] px-4 py-2">
                              <div class="truncate font-mono text-xs text-neutral-200" title={resource.assetId}>
                                {resource.assetId}
                              </div>
                            </td>
                            <td class="px-3 py-2 text-neutral-400">{resource.category}</td>
                            <td class="px-3 py-2 text-neutral-400">{resource.cacheStatus}</td>
                            <td class="px-3 py-2 text-neutral-400">{resource.networkBytes}</td>
                            <td class="px-4 py-2 font-mono text-xs text-neutral-300">{resource.accessCount}</td>
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  </div>
                {/if}
              </section>
            {/if}

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
              <div class="flex items-start justify-between gap-3 border-b border-neutral-800 px-4 py-3">
                <div class="min-w-0">
                  <h2 class="text-sm font-semibold text-neutral-100">Selected Run Events</h2>
                  <div class="mt-1 truncate text-xs text-neutral-500">
                    {formatSchedulerProjectionFreshness(timelineProjectionState)}
                  </div>
                </div>
                {#if timelineLoading}
                  <RefreshCw size={12} aria-hidden="true" class="mt-1 shrink-0 animate-spin text-neutral-500" />
                {/if}
              </div>
              {#if timelineError}
                <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{timelineError}</div>
              {/if}
              {#if !$activeWorkflowRun}
                <div class="px-4 py-6 text-sm text-neutral-500">No active run selected</div>
              {:else if timelineLoading && timelineEvents.length === 0}
                <div class="px-4 py-6 text-sm text-neutral-500">Loading selected-run events</div>
              {:else if timelineEvents.length === 0}
                <div class="px-4 py-6 text-sm text-neutral-500">No selected-run scheduler events projected</div>
              {:else}
                <div class="divide-y divide-neutral-900">
                  {#each timelineEvents as event (event.event_id)}
                    <div class="px-4 py-3">
                      <div class="flex flex-wrap items-center gap-2">
                        <span class="text-sm font-medium text-neutral-100">{formatSchedulerTimelineKind(event)}</span>
                        <span class="rounded border border-neutral-800 px-1.5 py-0.5 text-[11px] text-neutral-500">
                          {schedulerTimelinePayloadLabel(event)}
                        </span>
                      </div>
                      <div class="mt-1 text-xs text-neutral-500">
                        {formatSchedulerTimelineSource(event)} · {formatSchedulerTimestamp(event.occurred_at_ms)}
                      </div>
                      <div class="mt-1 text-xs text-neutral-300">{event.summary}</div>
                      {#if event.detail}
                        <div class="mt-1 text-xs text-neutral-500">{event.detail}</div>
                      {/if}
                    </div>
                  {/each}
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
