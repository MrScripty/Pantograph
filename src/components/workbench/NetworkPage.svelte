<script lang="ts">
  import { onMount } from 'svelte';
  import type {
    LibraryUsageProjectionRecord,
    NodeStatusProjectionRecord,
    ProjectionStateRecord,
    SchedulerTimelineProjectionRecord,
  } from '../../services/diagnostics/types';
  import { subscribeDiagnosticsProjectionInvalidations } from '../../services/workflow/WorkflowProjectionSubscriptionService';
  import type { WorkflowLocalNetworkStatusQueryResponse } from '../../services/workflow/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { activeWorkflowRun } from '../../stores/workbenchStore';
  import {
    buildLocalDeviceRows,
    buildLocalDiskRows,
    buildLocalIdentityRows,
    buildLocalInterfaceRows,
    buildLocalNodeHeader,
    buildLocalStatCells,
    buildNetworkDimensionLine,
    buildNetworkNodeGroups,
    buildPeerDetail,
    buildSelectedRunEventLines,
    buildSelectedRunExecutionRows,
    buildSelectedRunOnLocalNode,
    buildSelectedRunResourceRows,
    describeLocalStatusFailure,
    describeRunProjectionFailure,
    describeSelectedRunForPeer,
    DEVICE_MEMORY_NOT_REPORTED,
    formatNetworkProjectionFreshness,
    groupDegradationWarnings,
    resolveSelectedNodeId,
    type NetworkNodeEntry,
  } from './networkPagePresenters';
  import { openRunContextPicker } from './shell/runContextStore';
  import {
    Chip,
    DimensionLine,
    GroupHeader,
    KeyValue,
    ListRow,
    LogLine,
    Panel,
    StatusGlyph,
  } from './ui';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

  const SEGMENT_UNAVAILABLE_TITLE =
    'The network status does not say who owns a peer yet, so nodes cannot be filtered by owner.';
  const PAIR_UNAVAILABLE_TITLE = 'Pairing is not available in this build yet.';
  const FORGET_UNAVAILABLE_TITLE = 'Forgetting a peer is not available in this build yet.';

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
  let subscriptionError = $state<string | null>(null);
  let resourceError = $state<string | null>(null);
  let nodeStatusError = $state<string | null>(null);
  let timelineError = $state<string | null>(null);

  // Presentation state owned by this page.
  let requestedNodeId = $state<string | null>(null);
  let localExpanded = $state(true);
  let peersExpanded = $state(true);
  let eventsExpanded = $state(false);

  let localNode = $derived(status?.local_node ?? null);
  let peerNodes = $derived(status?.peer_nodes ?? []);
  let groups = $derived(localNode ? buildNetworkNodeGroups(localNode, peerNodes) : null);
  let selectedNodeId = $derived(groups ? resolveSelectedNodeId(groups, requestedNodeId) : null);
  let selectedPeer = $derived(
    groups && selectedNodeId !== groups.local.nodeId
      ? (peerNodes.find((peer) => peer.node_id === selectedNodeId) ?? null)
      : null,
  );
  let dimensionLine = $derived(buildNetworkDimensionLine(localNode, peerNodes, loading));
  let busy = $derived(loading || timelineLoading || resourceLoading || nodeStatusLoading);

  let localHeader = $derived(localNode ? buildLocalNodeHeader(localNode, peerNodes.length) : null);
  let statCells = $derived(localNode ? buildLocalStatCells(localNode) : []);
  let degradations = $derived(localNode ? groupDegradationWarnings(localNode) : null);
  let deviceRows = $derived(localNode ? buildLocalDeviceRows(localNode) : []);
  let diskRows = $derived(localNode ? buildLocalDiskRows(localNode) : []);
  let interfaceRows = $derived(localNode ? buildLocalInterfaceRows(localNode) : []);
  let identityRows = $derived(localNode ? buildLocalIdentityRows(localNode) : []);
  let peerDetail = $derived(selectedPeer ? buildPeerDetail(selectedPeer) : null);

  let selectedRunOnNode = $derived(
    localNode && $activeWorkflowRun ? buildSelectedRunOnLocalNode(localNode, $activeWorkflowRun) : null,
  );
  let selectedRunOnPeer = $derived(
    localNode && peerDetail && $activeWorkflowRun
      ? describeSelectedRunForPeer(localNode, $activeWorkflowRun, peerDetail.header.title)
      : null,
  );
  let executionRows = $derived(buildSelectedRunExecutionRows(selectedRunNodeStatuses));
  let resourceRows = $derived(buildSelectedRunResourceRows(selectedRunResources));
  let eventLines = $derived(buildSelectedRunEventLines(timelineEvents));
  let statusFailure = $derived(
    error ? describeLocalStatusFailure(error, localNode?.captured_at_ms ?? null) : null,
  );

  let statusRequestSerial = 0;
  let resourceRequestSerial = 0;
  let nodeStatusRequestSerial = 0;
  let timelineRequestSerial = 0;
  let projectionUnsubscribe: (() => void) | null = null;

  function activeRunId(): string | null {
    return $activeWorkflowRun?.workflow_run_id ?? null;
  }

  async function refreshStatus(): Promise<void> {
    const requestSerial = ++statusRequestSerial;
    loading = true;
    try {
      const response = await workflowService.queryLocalNetworkStatus({
        include_disks: true,
        include_network_interfaces: true,
      });
      if (requestSerial !== statusRequestSerial) {
        return;
      }
      status = response;
      error = null;
    } catch (statusError) {
      if (requestSerial !== statusRequestSerial) {
        return;
      }
      // Keep the last status; the panel says it is from an earlier capture.
      error = formatWorkflowCommandError(statusError);
    } finally {
      if (requestSerial === statusRequestSerial) {
        loading = false;
      }
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

  function recordSubscriptionError(failure: unknown): void {
    subscriptionError = formatWorkflowCommandError(failure);
  }

  async function refreshFromProjectionInvalidation(projectionKind: string): Promise<void> {
    if (projectionKind === 'scheduler_timeline') {
      await refreshTimeline();
      return;
    }
    if (projectionKind === 'library_usage') {
      await refreshSelectedRunResources();
      return;
    }
    if (projectionKind === 'node_status') {
      await refreshSelectedRunNodeStatuses();
    }
  }

  onMount(() => {
    let disposed = false;
    void refreshStatus();
    void subscribeDiagnosticsProjectionInvalidations({
      projections: ['scheduler_timeline', 'library_usage', 'node_status'],
      getActiveRunId: activeRunId,
      refresh: (event) => refreshFromProjectionInvalidation(event.projection_kind),
      onRefreshError: recordSubscriptionError,
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }
        projectionUnsubscribe = nextUnlisten;
      })
      .catch(recordSubscriptionError);

    return () => {
      disposed = true;
      projectionUnsubscribe?.();
      projectionUnsubscribe = null;
    };
  });

  $effect(() => {
    const runId = activeRunId();
    void refreshTimeline(runId);
    void refreshSelectedRunResources(runId);
    void refreshSelectedRunNodeStatuses(runId);
  });
</script>

{#snippet nodeRow(entry: NetworkNodeEntry)}
  <ListRow
    glyph={entry.glyph}
    name={entry.name}
    version={entry.nodeId}
    context={entry.line1}
    detail={entry.line2}
    time={entry.time}
    title={entry.timeTitle ? `${entry.name}, ${entry.transportLabel}. ${entry.timeTitle}` : `${entry.name}, ${entry.transportLabel}`}
    selected={entry.nodeId === selectedNodeId}
    onselect={() => (requestedNodeId = entry.nodeId)}
  />
{/snippet}

{#snippet degraded(lines: string[])}
  {#each lines as line (line)}
    <p class="pg-muted pg-small net-note">{line}</p>
  {/each}
{/snippet}

{#snippet sectionStatus(loadingNow: boolean, failure: string | null, freshness: string)}
  <span class="pg-muted net-fresh">{loadingNow ? 'Refreshing' : freshness}</span>
  {#if failure}<p class="pg-muted pg-small net-note">{failure}</p>{/if}
{/snippet}

<div class="pg-main h-full">
  <DimensionLine label={dimensionLine} />

  <div class="net-body">
    <Panel label="Nodes" width="340px">
      {#snippet top()}
        <div class="pg-seg net-seg" role="group" aria-label="Show">
          <button type="button" aria-pressed="true">
            All nodes{#if groups}<span class="pg-n">{1 + groups.peers.length}</span>{/if}
          </button>
          <button type="button" aria-pressed="false" disabled title={SEGMENT_UNAVAILABLE_TITLE}>Shared with me</button>
          <button type="button" aria-pressed="false" disabled title={SEGMENT_UNAVAILABLE_TITLE}>Mine</button>
        </div>
      {/snippet}

      {#if groups}
        <GroupHeader
          label="Local"
          count={1}
          expanded={localExpanded}
          controls="network-local-rows"
          ontoggle={() => (localExpanded = !localExpanded)}
        />
        <div id="network-local-rows" hidden={!localExpanded}>
          {@render nodeRow(groups.local)}
        </div>
        <GroupHeader
          label="Peers"
          count={groups.peers.length}
          expanded={peersExpanded}
          controls="network-peer-rows"
          ontoggle={() => (peersExpanded = !peersExpanded)}
        />
        <div id="network-peer-rows" hidden={!peersExpanded}>
          {#each groups.peers as peer (peer.nodeId)}
            {@render nodeRow(peer)}
          {:else}
            <p class="pg-muted pg-small net-listnote">No peers are known to this node.</p>
          {/each}
        </div>
      {:else}
        <p class="pg-muted pg-small net-listnote">
          {loading ? 'Reading the local status.' : 'Nodes appear here once the local status can be read.'}
        </p>
      {/if}

      {#snippet footer()}
        <button class="pg-btn pg-btn-sm" type="button" disabled title={PAIR_UNAVAILABLE_TITLE}>Pair a node</button>
        <span class="pg-muted pg-small net-trust">Trusted peers only</span>
      {/snippet}
    </Panel>

    <Panel label="Node detail">
      {#if !localNode}
        <div class="net-head">
          <div class="net-headtext">
            <div class="pg-title">Network</div>
            <p class="pg-muted pg-small net-sub">
              {statusFailure ?? 'Reading the local status.'}
            </p>
          </div>
          <div class="net-actions">
            <button class="pg-btn" type="button" disabled={busy} onclick={() => refreshNetworkPage()}>
              {busy ? 'Refreshing' : 'Refresh'}
            </button>
          </div>
        </div>
      {:else if peerDetail}
        <div class="net-head">
          <div class="net-headtext">
            <div class="pg-title">{peerDetail.header.title} <span class="pg-muted">{peerDetail.header.nodeId}</span></div>
            <div class="pg-muted pg-small net-sub">{peerDetail.header.subtitle}</div>
            <div class="net-chips">
              <Chip glyph={peerDetail.header.glyph}>{peerDetail.header.transportLabel}</Chip>
              {#each peerDetail.header.chips as chip (chip)}<Chip>{chip}</Chip>{/each}
            </div>
          </div>
          <div class="net-actions">
            <button class="pg-btn" type="button" disabled={busy} onclick={() => refreshNetworkPage()}>
              {busy ? 'Refreshing' : 'Refresh'}
            </button>
            <button class="pg-btn pg-btn-danger" type="button" disabled title={FORGET_UNAVAILABLE_TITLE}>Forget node</button>
          </div>
        </div>
        {#if statusFailure}<p class="pg-muted pg-small net-headnote">{statusFailure}</p>{/if}

        <div class="pg-sec">
          <h3>Details</h3>
          <p class="pg-muted pg-small">{peerDetail.note}</p>
        </div>

        {#if selectedRunOnPeer}
          <div class="pg-sec">
            <h3>{selectedRunOnPeer.heading}</h3>
            <p class="pg-muted pg-small">{selectedRunOnPeer.sentence}</p>
          </div>
        {/if}

        <div class="pg-sec">
          <h3>Identity</h3>
          <KeyValue rows={peerDetail.identityRows} />
        </div>
      {:else if localHeader && degradations}
        <div class="net-head">
          <div class="net-headtext">
            <div class="pg-title">{localHeader.title} <span class="pg-muted">{localHeader.nodeId}</span></div>
            <div class="pg-muted pg-small net-sub">{localHeader.subtitle}</div>
            <div class="net-chips">
              <Chip glyph={localHeader.glyph}>{localHeader.transportLabel}</Chip>
              {#each localHeader.chips as chip (chip)}<Chip>{chip}</Chip>{/each}
            </div>
          </div>
          <div class="net-actions">
            <button class="pg-btn" type="button" disabled={busy} onclick={() => refreshNetworkPage()}>
              {busy ? 'Refreshing' : 'Refresh'}
            </button>
          </div>
        </div>
        {#if statusFailure}<p class="pg-muted pg-small net-headnote">{statusFailure}</p>{/if}
        {#if subscriptionError}
          <p class="pg-muted pg-small net-headnote">
            Live updates for the selected run are off ({subscriptionError}). Use Refresh to update.
          </p>
        {/if}
        {#if degradations.node.length > 0}
          <div class="net-headnote">{@render degraded(degradations.node)}</div>
        {/if}

        <div class="net-stats">
          {#each statCells as cell (cell.id)}
            <div class="net-stat">
              <div class="net-stat-v">{cell.value}</div>
              <div class="net-stat-l">{cell.label}</div>
              {#if cell.meterPercent !== null}
                <div class="pg-meter net-meter" aria-hidden="true"><i style:width="{cell.meterPercent}%"></i></div>
              {/if}
              {#if cell.id === 'cpu'}{@render degraded(degradations.cpu)}{/if}
              {#if cell.id === 'memory'}{@render degraded(degradations.memory)}{/if}
            </div>
          {/each}
        </div>

        <div class="pg-sec">
          <h3>Devices</h3>
          <!--
            WR-006: the local status reports GPU availability only
            (WorkflowLocalGpuMetrics is { available, reason }). When the
            projection adds per-device memory totals and reservations, render
            them here with the shared DeviceLanes part instead of this KeyValue.
          -->
          <KeyValue rows={deviceRows} label="GPU" />
          <p class="pg-muted pg-small net-note">{DEVICE_MEMORY_NOT_REPORTED}</p>
          {@render degraded(degradations.devices)}
        </div>

        <div class="pg-sec">
          {#if !$activeWorkflowRun || !selectedRunOnNode}
            <h3>Selected run</h3>
            <p class="pg-muted pg-small">
              No run selected. Pick one to see its placement, execution and resources on this node.
            </p>
            <button class="pg-btn pg-btn-sm net-pick" type="button" onclick={() => openRunContextPicker()}>Pick a run</button>
          {:else}
            <h3>{selectedRunOnNode.heading}</h3>
            <dl class="pg-kv">
              <dt>Run</dt>
              <dd class="pg-mono" data-run-id={selectedRunOnNode.runId} title="Ctrl+C copies the run id">{selectedRunOnNode.runId}</dd>
              <dt>State</dt>
              <dd>{selectedRunOnNode.state}</dd>
              {#each selectedRunOnNode.rows as row (row.key)}
                <dt>{row.key}</dt>
                <dd class:pg-mono={row.mono}>{row.value}</dd>
              {/each}
            </dl>
            {#if selectedRunOnNode.note}
              <p class="pg-muted pg-small net-note">{selectedRunOnNode.note}</p>
            {/if}

            <div class="net-subhead">
              <h4>Execution</h4>
              {@render sectionStatus(
                nodeStatusLoading,
                nodeStatusError ? describeRunProjectionFailure('node status for this run', nodeStatusError) : null,
                formatNetworkProjectionFreshness(selectedRunNodeStatusProjectionState),
              )}
            </div>
            {#if executionRows.length === 0}
              {#if !nodeStatusError}
                <p class="pg-muted pg-small">
                  {nodeStatusLoading ? 'Reading node status.' : 'No node status is projected for this run yet.'}
                </p>
              {/if}
            {:else}
              <div class="net-tblwrap">
                <table class="pg-tbl net-tbl">
                  <thead>
                    <tr><th scope="col">Node</th><th scope="col">Status</th><th scope="col">Runtime</th><th scope="col">Model</th></tr>
                  </thead>
                  <tbody>
                    {#each executionRows as row (row.nodeId)}
                      <tr>
                        <td class="pg-mono net-cell" title={row.nodeId}>{row.nodeId}</td>
                        <td class="net-status"><StatusGlyph state={row.glyph} small />{row.status}</td>
                        <td class="net-cell" title={row.runtime}>{row.runtime}</td>
                        <td class="net-cell" title={row.model}>{row.model}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {/if}

            <div class="net-subhead">
              <h4>Resources</h4>
              {@render sectionStatus(
                resourceLoading,
                resourceError ? describeRunProjectionFailure('library usage for this run', resourceError) : null,
                formatNetworkProjectionFreshness(selectedRunResourceProjectionState),
              )}
            </div>
            {#if resourceRows.length === 0}
              {#if !resourceError}
                <p class="pg-muted pg-small">
                  {resourceLoading ? 'Reading library usage.' : 'No library usage is projected for this run yet.'}
                </p>
              {/if}
            {:else}
              <div class="net-tblwrap">
                <table class="pg-tbl net-tbl">
                  <thead>
                    <tr>
                      <th scope="col">Asset</th><th scope="col">Category</th><th scope="col">Cache</th>
                      <th scope="col" class="net-r">Network</th><th scope="col" class="net-r">Reads</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each resourceRows as row (row.assetId)}
                      <tr>
                        <td class="pg-mono net-cell" title={row.assetId}>{row.assetId}</td>
                        <td>{row.category}</td>
                        <td>{row.cacheStatus}</td>
                        <td class="pg-num">{row.networkBytes}</td>
                        <td class="pg-num">{row.accessCount}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {/if}
          {/if}
        </div>

        {#if $activeWorkflowRun}
          <div class="pg-sec">
            <h3 class="net-disclosure-h">
              <button
                type="button"
                class="net-disclosure"
                aria-expanded={eventsExpanded}
                aria-controls="network-run-events"
                onclick={() => (eventsExpanded = !eventsExpanded)}
              >
                <span class="pg-chev" aria-hidden="true">{eventsExpanded ? '▾' : '▸'}</span>
                Events <span class="pg-n">{eventLines.length}</span>
              </button>
              <span class="pg-muted net-fresh">
                {timelineLoading ? 'Refreshing' : formatNetworkProjectionFreshness(timelineProjectionState)}
              </span>
            </h3>
            <div id="network-run-events" hidden={!eventsExpanded}>
              {#if timelineError}
                <p class="pg-muted pg-small">{describeRunProjectionFailure('scheduler events for this run', timelineError)}</p>
              {:else if eventLines.length === 0}
                <p class="pg-muted pg-small">
                  {timelineLoading ? 'Reading scheduler events.' : 'No scheduler events are projected for this run yet.'}
                </p>
              {:else}
                <div class="pg-log" role="log" aria-label="Scheduler events for the selected run">
                  {#each eventLines as line (line.id)}
                    <LogLine time={line.time} level={line.level} kind={line.kind} message={line.message} source={line.source} />
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {/if}

        <div class="net-split">
          <div class="pg-sec net-split-cell">
            <h3>Disks</h3>
            {#if diskRows.length === 0}
              <p class="pg-muted pg-small">Disk metrics are not reported.</p>
            {:else}
              <table class="pg-tbl net-tbl">
                <thead><tr><th scope="col">Mount</th><th scope="col" class="net-r">Free</th><th scope="col" class="net-r">Total</th></tr></thead>
                <tbody>
                  {#each diskRows as disk (disk.key)}
                    <tr>
                      <td class="pg-mono net-cell" title={disk.name}>{disk.mount}</td>
                      <td class="pg-num">{disk.free}</td>
                      <td class="pg-num pg-muted">{disk.total}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
            {@render degraded(degradations.disks)}
          </div>
          <div class="pg-sec net-split-cell net-split-right">
            <h3>Interfaces</h3>
            {#if interfaceRows.length === 0}
              <p class="pg-muted pg-small">Interface metrics are not reported.</p>
            {:else}
              <table class="pg-tbl net-tbl">
                <thead><tr><th scope="col">Name</th><th scope="col" class="net-r">Received</th><th scope="col" class="net-r">Sent</th></tr></thead>
                <tbody>
                  {#each interfaceRows as networkInterface (networkInterface.name)}
                    <tr>
                      <td class="pg-mono net-cell">{networkInterface.name}</td>
                      <td class="pg-num">{networkInterface.received}</td>
                      <td class="pg-num">{networkInterface.sent}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
            {@render degraded(degradations.interfaces)}
          </div>
        </div>

        <div class="pg-sec">
          <h3>Identity</h3>
          <KeyValue rows={identityRows} />
        </div>
      {/if}
    </Panel>
  </div>
</div>

<style>
  .net-body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 14px;
    align-items: stretch;
  }

  .net-seg button:disabled {
    cursor: not-allowed;
    color: var(--pg-text-muted);
    opacity: 0.7;
  }

  .net-seg button:disabled:hover {
    background: transparent;
    color: var(--pg-text-muted);
  }

  .net-listnote {
    padding: 10px 12px;
    margin: 0;
  }

  .net-trust {
    margin-left: auto;
  }

  .net-head {
    padding: 16px 18px 12px;
    display: flex;
    gap: 16px;
    align-items: flex-start;
    flex-wrap: wrap;
  }

  .net-headtext {
    flex: 1 1 260px;
    min-width: 0;
  }

  .net-sub {
    margin: 2px 0 0;
  }

  .net-chips {
    margin-top: 10px;
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .net-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .net-headnote {
    margin: 0;
    padding: 0 18px 10px;
  }

  .net-headnote .net-note {
    margin-top: 0;
  }

  .net-note {
    margin: 6px 0 0;
  }

  .net-stats {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(170px, 100%), 1fr));
    border-top: 1px solid var(--pg-border);
  }

  .net-stat {
    padding: 12px 18px;
    border-left: 1px solid var(--pg-border);
    min-width: 0;
  }

  .net-stat:first-child {
    border-left: 0;
  }

  .net-stat-v {
    font-family: var(--pg-font-serif);
    font-size: 22px;
    line-height: 1.1;
    color: var(--pg-text-bright);
  }

  .net-stat-l {
    font-size: 12px;
    color: var(--pg-text-muted);
    margin-top: 3px;
  }

  .net-meter {
    margin-top: 8px;
  }

  .net-pick {
    margin-top: 10px;
  }

  .net-subhead {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
    margin: 16px 0 6px;
  }

  .net-subhead h4 {
    margin: 0;
    font-size: 12.5px;
    font-weight: 700;
    color: var(--pg-text-soft);
  }

  .net-subhead .net-note {
    flex-basis: 100%;
    margin: 0;
  }

  .net-fresh {
    font-family: var(--pg-font-mono);
    font-size: 11px;
    font-weight: 400;
  }

  .net-tblwrap {
    overflow-x: auto;
    border: 1px solid var(--pg-border-soft);
    border-radius: 4px;
  }

  .net-tbl {
    font-size: 12.5px;
  }

  .net-tbl th {
    position: static;
  }

  .net-r {
    text-align: right;
  }

  .net-cell {
    max-width: 18rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .net-status {
    white-space: nowrap;
  }

  .net-status :global(.pg-g) {
    margin-right: 6px;
    vertical-align: 0;
  }

  .net-disclosure-h {
    display: flex;
    align-items: baseline;
    gap: 10px;
  }

  .net-disclosure {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  .net-disclosure .pg-chev {
    font-size: 10px;
    color: var(--pg-border-strong);
    width: 10px;
  }

  #network-run-events {
    margin-top: 4px;
  }

  .net-split {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(280px, 100%), 1fr));
    border-top: 1px solid var(--pg-border);
  }

  .net-split-cell {
    border-top: 0;
    min-width: 0;
  }

  .net-split-right {
    border-left: 1px solid var(--pg-border);
  }
</style>
