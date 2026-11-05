<script module lang="ts">
  import type { GraphPageModeRequest as PersistedModeRequest } from './graphInspectionPresenters';

  // The explicit mode choice outlives a tab switch (the shell unmounts pages);
  // it is keyed by run id, so selecting another run returns to the default.
  let persistedModeRequest: PersistedModeRequest | null = null;
</script>

<script lang="ts">
  import { untrack } from 'svelte';
  import NodePalette from '../NodePalette.svelte';
  import WorkflowGraph from '../WorkflowGraph.svelte';
  import WorkflowToolbar from '../WorkflowToolbar.svelte';
  import type { NodeDefinition, WorkflowRunGraphProjection } from '../../services/workflow/types';
  import { subscribeDiagnosticsProjectionInvalidations } from '../../services/workflow/WorkflowProjectionSubscriptionService';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import {
    currentGraphId,
    currentGraphName,
    currentGraphType,
    isReadOnly,
    switchGraph,
  } from '../../stores/graphSessionStore';
  import { deleteSelection, isDirty, nodes, selectedNodeIds } from '../../stores/workflowStore';
  import {
    activeWorkflowRun,
    focusWorkflowDiagnostics,
    setWorkbenchPage,
  } from '../../stores/workbenchStore';
  import RunGraphSnapshot from './RunGraphSnapshot.svelte';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';
  import {
    formatRunDimensionLabel,
    formatRunModeLabel,
    resolveGraphPageMode,
    runIdSuffix,
    type GraphPageMode,
    type GraphPageModeRequest,
  } from './graphInspectionPresenters';
  import {
    buildRunGraphNodeFacts,
    buildRunGraphNodeStatusMap,
    createRunGraphRequestGate,
    formatRunGraphTimestamp,
    resolveRunGraphPresentationLabel,
    resolveRunGraphSelectedNodeId,
    type RunGraphNodeStatusByNodeId,
  } from './runGraphPresenters';
  import { openRunContextPicker } from './shell/runContextStore';
  import Chip from './ui/Chip.svelte';
  import EmptyState from './ui/EmptyState.svelte';
  import ErrorCard from './ui/ErrorCard.svelte';
  import KeyValue from './ui/KeyValue.svelte';
  import SegmentedControl from './ui/SegmentedControl.svelte';
  import TwoStepButton from './ui/TwoStepButton.svelte';
  import { glyphStateForStatus, glyphStateLabel } from './ui/statusGlyph';
  import type { SegmentOption } from './ui/types';

  const NODE_STATUS_LIMIT = 250;

  type RunGraphLoadStatus = 'idle' | 'loading' | 'ready' | 'missing' | 'error';

  interface RunGraphState {
    runId: string | null;
    status: RunGraphLoadStatus;
    runGraph: WorkflowRunGraphProjection | null;
    nodeStatuses: RunGraphNodeStatusByNodeId;
    statusesAvailable: boolean;
    statusError: string | null;
    error: string | null;
  }

  const EMPTY_RUN_GRAPH_STATE: RunGraphState = {
    runId: null,
    status: 'idle',
    runGraph: null,
    nodeStatuses: {},
    statusesAvailable: true,
    statusError: null,
    error: null,
  };

  // Presentation state owned by this page. The run graph below is a read-only
  // projection copy (ADR-014); nothing here writes the editable graph store.
  let modeRequest = $state<GraphPageModeRequest | null>(persistedModeRequest);
  let paletteOpen = $state(true);
  let propertiesOpen = $state(true);
  let selectedRunNodeId = $state<string | null>(null);
  let runState = $state<RunGraphState>({ ...EMPTY_RUN_GRAPH_STATE });
  let liveStatusUpdates = $state<'connecting' | 'live' | 'unavailable'>('connecting');

  // One gate for full loads, one for status refreshes: a refresh must not
  // supersede the graph load it depends on, and a new load supersedes both.
  const loadGate = createRunGraphRequestGate();
  const statusGate = createRunGraphRequestGate();

  let activeRun = $derived($activeWorkflowRun);
  let runId = $derived(activeRun?.workflow_run_id ?? null);
  let loadedWorkflowId = $derived($currentGraphType === 'workflow' ? $currentGraphId : null);
  let resolution = $derived(
    resolveGraphPageMode({ activeRun, loadedWorkflowId, request: modeRequest }),
  );
  let mode = $derived(resolution.mode);
  let runGlyph = $derived(glyphStateForStatus(activeRun?.status));
  let runStatusLabel = $derived(activeRun?.status ? glyphStateLabel(runGlyph) : 'Selected');

  let modeOptions = $derived<SegmentOption<GraphPageMode>[]>([
    { value: 'edit', label: 'Edit workflow' },
    { value: 'run', label: formatRunModeLabel(runId) },
  ]);

  let shownRunGraph = $derived(
    runState.status === 'ready' && runState.runId === runId ? runState.runGraph : null,
  );
  let effectiveRunNodeId = $derived(
    shownRunGraph
      ? resolveRunGraphSelectedNodeId(selectedRunNodeId, shownRunGraph.graph, runState.nodeStatuses)
      : null,
  );
  let runFacts = $derived(
    shownRunGraph && effectiveRunNodeId
      ? buildRunGraphNodeFacts(
          shownRunGraph,
          effectiveRunNodeId,
          runState.nodeStatuses,
          runState.statusesAvailable,
        )
      : null,
  );
  let runDimLabel = $derived(
    !runId
      ? ''
      : shownRunGraph
        ? formatRunDimensionLabel(shownRunGraph.graph.nodes.length, runId, runStatusLabel)
        : `Run ${runIdSuffix(runId)}, ${runStatusLabel.toLowerCase()}`,
  );

  let editNode = $derived(
    $selectedNodeIds.length === 1
      ? ($nodes.find((node) => node.id === $selectedNodeIds[0]) ?? null)
      : null,
  );
  let editDefinition = $derived(
    (editNode?.data as { definition?: NodeDefinition } | undefined)?.definition ?? null,
  );
  let editNodeLabel = $derived(
    editNode
      ? String((editNode.data as { label?: unknown }).label ?? '') || editDefinition?.label || editNode.id
      : '',
  );

  // Run mode owns the run graph load and the node-status subscription for the
  // selected run. Leaving run mode, changing run or unmounting stops both and
  // supersedes any load still in flight.
  $effect(() => {
    const id = mode === 'run' ? runId : null;
    if (!id) {
      loadGate.invalidate();
      statusGate.invalidate();
      return;
    }

    untrack(() => {
      void loadRunGraph(id);
    });

    let disposed = false;
    let unlisten: (() => void) | null = null;
    liveStatusUpdates = 'connecting';
    subscribeDiagnosticsProjectionInvalidations({
      projections: ['node_status'],
      getActiveRunId: () => id,
      refresh: () => refreshNodeStatuses(id),
      onRefreshError: (error) => {
        // Keep the last statuses on screen and say the refresh failed.
        if (!disposed && runState.runId === id && runState.status === 'ready') {
          runState = { ...runState, statusError: formatWorkflowCommandError(error) };
        }
      },
    })
      .then((nextUnlisten) => {
        if (disposed) {
          nextUnlisten();
          return;
        }
        unlisten = nextUnlisten;
        liveStatusUpdates = 'live';
      })
      .catch(() => {
        if (!disposed) liveStatusUpdates = 'unavailable';
      });

    return () => {
      disposed = true;
      unlisten?.();
      unlisten = null;
      loadGate.invalidate();
      statusGate.invalidate();
    };
  });

  async function loadRunGraph(id: string): Promise<void> {
    const token = loadGate.begin();
    statusGate.invalidate();
    runState = { ...EMPTY_RUN_GRAPH_STATE, runId: id, status: 'loading' };

    const [graphResult, statusResult] = await Promise.allSettled([
      workflowService.queryRunGraph({ workflow_run_id: id }),
      workflowService.queryNodeStatus({ workflow_run_id: id, limit: NODE_STATUS_LIMIT }),
    ]);
    if (!loadGate.isCurrent(token)) {
      return; // superseded by another run, a reload or leaving run mode
    }

    if (graphResult.status === 'rejected') {
      runState = {
        ...EMPTY_RUN_GRAPH_STATE,
        runId: id,
        status: 'error',
        error: formatWorkflowCommandError(graphResult.reason),
      };
      return;
    }

    const runGraph = graphResult.value.run_graph ?? null;
    runState = {
      runId: id,
      status: runGraph ? 'ready' : 'missing',
      runGraph,
      nodeStatuses:
        statusResult.status === 'fulfilled' ? buildRunGraphNodeStatusMap(statusResult.value.nodes) : {},
      statusesAvailable: statusResult.status === 'fulfilled',
      statusError:
        statusResult.status === 'rejected' ? formatWorkflowCommandError(statusResult.reason) : null,
      error: null,
    };
  }

  async function refreshNodeStatuses(id: string): Promise<void> {
    const token = statusGate.begin();
    const response = await workflowService.queryNodeStatus({
      workflow_run_id: id,
      limit: NODE_STATUS_LIMIT,
    });
    if (!statusGate.isCurrent(token) || runState.runId !== id || runState.status !== 'ready') {
      return;
    }
    runState = {
      ...runState,
      nodeStatuses: buildRunGraphNodeStatusMap(response.nodes),
      statusesAvailable: true,
      statusError: null,
    };
  }

  function requestMode(next: GraphPageModeRequest): void {
    modeRequest = next;
    persistedModeRequest = next;
  }

  function setMode(next: GraphPageMode): void {
    if (next === 'run' && !runId) {
      openRunContextPicker();
      return;
    }
    requestMode({ runId, mode: next });
  }

  function showRun(): void {
    requestMode({ runId, mode: 'run' });
  }

  function handleWorkflowPicked(): void {
    requestMode({ runId, mode: 'edit' });
  }

  async function editCurrentVersion(): Promise<void> {
    const workflowId = activeRun?.workflow_id?.trim();
    const requestedForRun = runId;
    if (!workflowId) return;
    if (workflowId === loadedWorkflowId) {
      requestMode({ runId: requestedForRun, mode: 'edit' });
      return;
    }
    if ($isDirty && !confirm('Discard unsaved changes?')) return;
    const loaded = await switchGraph(workflowId, 'workflow');
    // A failure is explained by graphSessionError under the toolbar.
    if (loaded && runId === requestedForRun) {
      requestMode({ runId: requestedForRun, mode: 'edit' });
    }
  }

  function openIoInspector(): void {
    setWorkbenchPage('io_inspector');
  }

  function openNodeDiagnostics(): void {
    if (!activeRun) return;
    focusWorkflowDiagnostics(
      {
        workflow_run_id: activeRun.workflow_run_id,
        workflow_id: activeRun.workflow_id ?? null,
        workflow_version_id: activeRun.workflow_version_id ?? null,
        workflow_semantic_version: activeRun.workflow_semantic_version ?? null,
        status: activeRun.status ?? null,
      },
      {
        node_id: runFacts?.nodeId ?? null,
        diagnostic_event_id: runFacts?.errorEventId ?? null,
      },
    );
  }

  function removeEditNode(nodeId: string): void {
    void deleteSelection([nodeId], []);
  }
</script>

{#snippet modeSwitch()}
  <span class="gp-mode" data-run-id={runId ?? undefined}>
    <SegmentedControl label="What to show" compact options={modeOptions} value={mode} onchange={setMode} />
  </span>
{/snippet}

{#snippet panelToggles()}
  {#if mode === 'edit'}
    <button
      type="button"
      class="pg-tgl"
      aria-pressed={paletteOpen && !$isReadOnly}
      aria-disabled={$isReadOnly ? 'true' : undefined}
      title={$isReadOnly ? 'System graphs are read-only, so there are no nodes to add' : 'Show or hide the node palette'}
      onclick={() => {
        if (!$isReadOnly) paletteOpen = !paletteOpen;
      }}
    >
      Nodes
    </button>
  {/if}
  <button
    type="button"
    class="pg-tgl"
    aria-pressed={propertiesOpen}
    title="Show or hide node properties"
    onclick={() => (propertiesOpen = !propertiesOpen)}
  >
    Properties
  </button>
{/snippet}

{#snippet portList(ports: { direction: string; id: string; label: string; dataType: string | null }[])}
  {#if ports.length === 0}
    <p class="pg-small pg-muted gp-p">This node has no ports.</p>
  {:else}
    <dl class="pg-kv gp-ports">
      {#each ports as port (`${port.direction}:${port.id}`)}
        <dt>{port.direction}</dt>
        <dd><span class="pg-mono">{port.label}</span>{#if port.dataType}<span class="pg-mono pg-muted gp-type">{port.dataType}</span>{/if}</dd>
      {/each}
    </dl>
  {/if}
{/snippet}

<section class="pg-main gp" data-testid="workflow-editor-graph-page">
  <WorkflowToolbar
    {mode}
    run={runId
      ? {
          runId,
          workflowId: activeRun?.workflow_id ?? null,
          version: activeRun?.workflow_semantic_version ?? null,
          glyph: runGlyph,
        }
      : null}
    {runDimLabel}
    {modeSwitch}
    {panelToggles}
    onworkflowpicked={handleWorkflowPicked}
    oneditcurrent={() => void editCurrentVersion()}
  />

  {#if resolution.workflowMismatch && activeRun?.workflow_id}
    <p class="gp-note">
      The selected run is of <b>{activeRun.workflow_id}</b>, not {$currentGraphName}.
      <button type="button" class="pg-tgl gp-inline" onclick={showRun}>Show {activeRun.workflow_id} as run</button>
      or keep editing {$currentGraphName}.
    </p>
  {/if}

  <div class="gp-body">
    {#if mode === 'edit'}
      {#if paletteOpen && !$isReadOnly}
        <NodePalette />
      {/if}

      <div class="gp-canvas">
        <WorkflowGraph />
      </div>

      {#if propertiesOpen}
        <section class="pg-panel pg-col gp-props" aria-label="Node properties">
          {#if editNode}
            {@const editNodeId = editNode.id}
            <div class="pg-scroll">
              <div class="gp-head">
                <div class="pg-subtitle gp-title">{editNodeLabel}</div>
                <div class="pg-mono pg-muted gp-id">{editNode.type ?? ''} · {editNode.id}</div>
              </div>
              <div class="pg-sec gp-sec">
                <h3>Settings</h3>
                <p class="pg-small pg-muted gp-p">Edit this node's settings on its card in the canvas.</p>
              </div>
              <div class="pg-sec gp-sec">
                <h3>Ports</h3>
                {@render portList([
                  ...(editDefinition?.inputs ?? []).map((port) => ({ direction: 'in', id: port.id, label: port.label, dataType: port.data_type })),
                  ...(editDefinition?.outputs ?? []).map((port) => ({ direction: 'out', id: port.id, label: port.label, dataType: port.data_type })),
                ])}
              </div>
            </div>
            <div class="pg-pfoot">
              <button
                type="button"
                class="pg-btn pg-btn-sm"
                aria-disabled="true"
                title="Duplicating a node is not available in the editor yet."
              >
                Duplicate
              </button>
              <TwoStepButton
                class="gp-push"
                label="Remove node"
                confirmLabel="Confirm remove"
                disabled={$isReadOnly}
                disabledReason="System graphs are read-only."
                onconfirm={() => removeEditNode(editNodeId)}
              />
            </div>
          {:else}
            <p class="pg-empty">
              {$selectedNodeIds.length > 1
                ? `${$selectedNodeIds.length} nodes are selected. Select one node to see its ports.`
                : 'Select a node to see its ports.'}
            </p>
          {/if}
        </section>
      {/if}
    {:else}
      <div class="gp-canvas">
        {#if runState.status === 'error' && runState.runId === runId}
          <div class="pg-panel gp-fill">
            <ErrorCard
              title="Could not load the graph of this run"
              explanation={runState.error ?? 'The run graph query failed.'}
              suggestion="check that the local engine is running, then load it again."
              actionLabel="Load again"
              onaction={() => runId && void loadRunGraph(runId)}
            />
          </div>
        {:else if runState.status === 'missing' && runState.runId === runId}
          <EmptyState
            question="This run has no captured graph."
            sentence="Runs record a graph snapshot when they are submitted; this one has none to show. The current workflow is still available to edit."
            actionLabel="Edit workflow"
            onaction={() => setMode('edit')}
          />
        {:else if shownRunGraph}
          <RunGraphSnapshot
            runGraph={shownRunGraph}
            nodeStatuses={runState.nodeStatuses}
            statusesAvailable={runState.statusesAvailable}
            selectedNodeId={effectiveRunNodeId}
            onSelectNode={(nodeId) => (selectedRunNodeId = nodeId)}
          />
        {:else}
          <div class="pg-panel gp-fill gp-wait" role="status">Loading the graph as this run used it…</div>
        {/if}
      </div>

      {#if propertiesOpen}
        <section class="pg-panel pg-col gp-props" aria-label="Node properties">
          {#if runFacts && shownRunGraph}
            <div class="pg-scroll">
              <div class="gp-head">
                <div class="pg-subtitle gp-title">{runFacts.label}</div>
                <div class="pg-mono pg-muted gp-id">{runFacts.nodeType} · {runFacts.nodeId}</div>
                <div class="gp-chips"><Chip glyph={runFacts.glyph}>{runFacts.stateLabel}</Chip></div>
              </div>
              <div class="pg-sec gp-sec">
                <h3>Settings</h3>
                {#if runFacts.settings.length > 0}
                  <KeyValue rows={runFacts.settings} label="Settings as the run used them" />
                {/if}
                {#if runFacts.settingsNote}
                  <p class="pg-small pg-muted gp-p">{runFacts.settingsNote}</p>
                {/if}
                <p class="pg-small pg-muted gp-p">
                  Values as they were when the run started. Switch to Edit workflow to change them.
                </p>
              </div>
              <div class="pg-sec gp-sec">
                <h3>Ports</h3>
                {@render portList(runFacts.ports)}
              </div>
              <div class="pg-sec gp-sec">
                <h3>In this run</h3>
                <p class="pg-small gp-p">{runFacts.runSentence}</p>
                {#if runFacts.runRows.length > 0}
                  <KeyValue rows={runFacts.runRows} label="Node facts in this run" />
                {/if}
                {#each runFacts.staleMessages as message, index (index)}
                  <p class="pg-small pg-muted gp-p">Stale graph fact: {message}</p>
                {/each}
                <div class="gp-actions">
                  <button type="button" class="pg-btn pg-btn-sm" onclick={openIoInspector}>I/O</button>
                  <button type="button" class="pg-btn pg-btn-sm" onclick={openNodeDiagnostics}>Diagnostics</button>
                </div>
              </div>
              <div class="pg-sec gp-sec">
                <h3>Snapshot</h3>
                <p class="pg-mono pg-small gp-runid" data-run-id={shownRunGraph.workflow_run_id} title="Ctrl+C copies the run id">
                  {shownRunGraph.workflow_run_id}
                </p>
                <KeyValue
                  label="Snapshot facts"
                  rows={[
                    { key: 'Workflow', value: `${shownRunGraph.workflow_id} v${shownRunGraph.workflow_semantic_version}` },
                    { key: 'Captured', value: formatRunGraphTimestamp(shownRunGraph.snapshot_created_at_ms) },
                    { key: 'Layout', value: resolveRunGraphPresentationLabel(shownRunGraph) },
                    ...runFacts.contractRows,
                  ]}
                />
                {#if runState.statusError}
                  <p class="pg-small pg-muted gp-p">
                    {runState.statusesAvailable ? 'Node status could not be refreshed' : 'Node status is unavailable'}: {runState.statusError}
                  </p>
                {:else if liveStatusUpdates === 'unavailable'}
                  <p class="pg-small pg-muted gp-p">Node status updates arrive only when the run is opened again.</p>
                {/if}
              </div>
            </div>
          {:else}
            <p class="pg-empty">
              {shownRunGraph ? 'Select a node to see its facts in this run.' : 'Node facts appear once the run graph has loaded.'}
            </p>
          {/if}
        </section>
      {/if}
    {/if}
  </div>
</section>

<style>
  .gp {
    height: 100%;
    gap: 8px;
  }

  .gp-mode {
    display: inline-flex;
    flex: none;
  }

  .gp-note {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 12.5px;
    color: var(--pg-text-muted);
    flex: none;
  }

  .gp-note b {
    color: var(--pg-text);
  }

  .gp-inline {
    height: 22px;
  }

  .gp-body {
    flex: 1;
    min-height: 0;
    display: flex;
    gap: 10px;
    align-items: stretch;
  }

  .gp-canvas {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
  }

  .gp-canvas > :global(*) {
    flex: 1;
    min-width: 0;
  }

  .gp-fill {
    padding: 24px;
  }

  .gp-wait {
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--pg-text-muted);
  }

  .gp-props {
    flex: 0 0 300px;
    min-height: 0;
  }

  .gp-head {
    padding: 14px 14px 10px;
  }

  .gp-title {
    overflow-wrap: anywhere;
  }

  .gp-id {
    margin-top: 2px;
    font-size: 11.5px;
    overflow-wrap: anywhere;
  }

  .gp-chips {
    margin-top: 8px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .gp-sec {
    padding: 12px 14px;
  }

  .gp-p {
    margin: 6px 0 0;
  }

  .gp-ports {
    font-size: 12.5px;
  }

  .gp-type {
    margin-left: 6px;
    font-size: 11.5px;
  }

  .gp-actions {
    margin-top: 10px;
    display: flex;
    gap: 6px;
  }

  .gp-runid {
    margin: 0 0 8px;
    overflow-wrap: anywhere;
    color: var(--pg-text-soft);
  }

  .gp-props :global(.gp-push) {
    margin-left: auto;
  }
</style>
