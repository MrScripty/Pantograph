<script lang="ts">
  import { RefreshCw } from 'lucide-svelte';
  import NodePalette from '../NodePalette.svelte';
  import WorkflowGraph from '../WorkflowGraph.svelte';
  import WorkflowToolbar from '../WorkflowToolbar.svelte';
  import type {
    IoArtifactProjectionRecord,
    NodeStatusProjectionRecord,
  } from '../../services/diagnostics/types';
  import type { WorkflowRunGraphProjection } from '../../services/workflow/types';
  import { workflowService } from '../../services/workflow/WorkflowService';
  import { isReadOnly } from '../../stores/graphSessionStore';
  import { activeWorkflowRun } from '../../stores/workbenchStore';
  import RunGraphSnapshot from './RunGraphSnapshot.svelte';
  import {
    buildRunGraphNodeArtifactSummaries,
    buildRunGraphNodeStatusMap,
  } from './runGraphPresenters';
  import { formatWorkflowCommandError } from './workflowErrorPresenters';

  type GraphPageMode = 'run_snapshot' | 'editor';

  let mode = $state<GraphPageMode>('editor');
  let runGraph = $state<WorkflowRunGraphProjection | null>(null);
  let runArtifacts = $state<IoArtifactProjectionRecord[]>([]);
  let runNodeStatuses = $state<NodeStatusProjectionRecord[]>([]);
  let loadingRunGraph = $state(false);
  let loadingRunArtifacts = $state(false);
  let loadingRunNodeStatuses = $state(false);
  let runGraphError = $state<string | null>(null);
  let runArtifactError = $state<string | null>(null);
  let runNodeStatusError = $state<string | null>(null);
  let lastRunId = $state<string | null>(null);
  let runGraphRequestSerial = 0;
  let runArtifactRequestSerial = 0;
  let runNodeStatusRequestSerial = 0;
  let artifactSummaries = $derived(buildRunGraphNodeArtifactSummaries(runArtifacts));
  let nodeStatuses = $derived(buildRunGraphNodeStatusMap(runNodeStatuses));

  function activeRunId(): string | null {
    return $activeWorkflowRun?.workflow_run_id ?? null;
  }

  async function refreshRunGraph(runId = activeRunId()): Promise<void> {
    const requestSerial = ++runGraphRequestSerial;
    runGraphError = null;

    if (!runId) {
      runGraph = null;
      runArtifacts = [];
      runNodeStatuses = [];
      loadingRunGraph = false;
      return;
    }

    loadingRunGraph = true;
    try {
      const response = await workflowService.queryRunGraph({
        workflow_run_id: runId,
      });
      if (requestSerial !== runGraphRequestSerial) {
        return;
      }
      runGraph = response.run_graph ?? null;
    } catch (error) {
      if (requestSerial !== runGraphRequestSerial) {
        return;
      }
      runGraphError = formatWorkflowCommandError(error);
      runGraph = null;
    } finally {
      if (requestSerial === runGraphRequestSerial) {
        loadingRunGraph = false;
      }
    }
  }

  async function refreshRunArtifacts(runId = activeRunId()): Promise<void> {
    const requestSerial = ++runArtifactRequestSerial;
    runArtifactError = null;

    if (!runId) {
      runArtifacts = [];
      loadingRunArtifacts = false;
      return;
    }

    loadingRunArtifacts = true;
    try {
      const response = await workflowService.queryIoArtifacts({
        workflow_run_id: runId,
        limit: 250,
      });
      if (requestSerial !== runArtifactRequestSerial) {
        return;
      }
      runArtifacts = response.artifacts;
    } catch (error) {
      if (requestSerial !== runArtifactRequestSerial) {
        return;
      }
      runArtifactError = formatWorkflowCommandError(error);
      runArtifacts = [];
    } finally {
      if (requestSerial === runArtifactRequestSerial) {
        loadingRunArtifacts = false;
      }
    }
  }

  async function refreshRunNodeStatuses(runId = activeRunId()): Promise<void> {
    const requestSerial = ++runNodeStatusRequestSerial;
    runNodeStatusError = null;

    if (!runId) {
      runNodeStatuses = [];
      loadingRunNodeStatuses = false;
      return;
    }

    loadingRunNodeStatuses = true;
    try {
      const response = await workflowService.queryNodeStatus({
        workflow_run_id: runId,
        limit: 250,
      });
      if (requestSerial !== runNodeStatusRequestSerial) {
        return;
      }
      runNodeStatuses = response.nodes;
    } catch (error) {
      if (requestSerial !== runNodeStatusRequestSerial) {
        return;
      }
      runNodeStatusError = formatWorkflowCommandError(error);
      runNodeStatuses = [];
    } finally {
      if (requestSerial === runNodeStatusRequestSerial) {
        loadingRunNodeStatuses = false;
      }
    }
  }

  function refreshRunSnapshot(): void {
    void refreshRunGraph();
    void refreshRunArtifacts();
    void refreshRunNodeStatuses();
  }

  $effect(() => {
    const runId = activeRunId();
    if (runId === lastRunId) {
      return;
    }

    lastRunId = runId;
    mode = runId ? 'run_snapshot' : 'editor';
    void refreshRunGraph(runId);
    void refreshRunArtifacts(runId);
    void refreshRunNodeStatuses(runId);
  });
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950">
  {#if $activeWorkflowRun}
    <div class="flex shrink-0 items-center justify-between gap-4 border-b border-neutral-800 px-4 py-3">
      <div class="min-w-0">
        <h1 class="text-base font-semibold text-neutral-100">Graph</h1>
        <div class="mt-1 truncate text-xs text-neutral-500">
          {$activeWorkflowRun.workflow_run_id}
        </div>
      </div>
      <div class="flex shrink-0 items-center gap-2">
        <div class="inline-flex overflow-hidden rounded border border-neutral-800">
          <button
            type="button"
            class={`px-3 py-1.5 text-sm transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 ${mode === 'run_snapshot' ? 'bg-cyan-950 text-cyan-100' : 'text-neutral-400 hover:bg-neutral-900 hover:text-neutral-100'}`}
            onclick={() => {
              mode = 'run_snapshot';
            }}
          >
            Run Snapshot
          </button>
          <button
            type="button"
            class={`border-l border-neutral-800 px-3 py-1.5 text-sm transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 ${mode === 'editor' ? 'bg-cyan-950 text-cyan-100' : 'text-neutral-400 hover:bg-neutral-900 hover:text-neutral-100'}`}
            onclick={() => {
              mode = 'editor';
            }}
          >
            Current Editor
          </button>
        </div>
        {#if mode === 'run_snapshot'}
          <button
            type="button"
            class="inline-flex items-center gap-2 rounded border border-neutral-700 px-3 py-1.5 text-sm text-neutral-300 transition-colors hover:border-neutral-500 hover:text-neutral-100 focus-visible:outline focus-visible:outline-2 focus-visible:outline-cyan-400 disabled:opacity-50"
            onclick={refreshRunSnapshot}
            disabled={loadingRunGraph || loadingRunArtifacts || loadingRunNodeStatuses}
          >
            <RefreshCw
              size={14}
              aria-hidden="true"
              class={loadingRunGraph || loadingRunArtifacts || loadingRunNodeStatuses ? 'animate-spin' : ''}
            />
            Refresh
          </button>
        {/if}
      </div>
    </div>
  {/if}

  {#if !$activeWorkflowRun || mode === 'editor'}
    <WorkflowToolbar showDiagnosticsToggle={false} />
    {#if $activeWorkflowRun}
      <div class="border-b border-neutral-800 bg-neutral-900/60 px-4 py-2 text-xs text-neutral-400">
        Editing the current workflow. Selected run remains
        <span class="font-mono text-neutral-200">{$activeWorkflowRun.workflow_run_id}</span>
        for other workbench pages.
      </div>
    {/if}
    <div class="flex min-h-0 flex-1 overflow-hidden">
      {#if !$isReadOnly}
        <NodePalette />
      {/if}
      <div class="min-w-0 flex-1">
        <WorkflowGraph />
      </div>
    </div>
  {:else if loadingRunGraph && !runGraph}
    <div class="flex min-h-0 flex-1 items-center justify-center text-sm text-neutral-500">
      Loading run graph
    </div>
  {:else if runGraphError}
    <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{runGraphError}</div>
    <div class="flex min-h-0 flex-1 items-center justify-center text-sm text-neutral-500">
      Run graph unavailable
    </div>
  {:else if !runGraph}
    <div class="flex min-h-0 flex-1 items-center justify-center text-sm text-neutral-500">
      No versioned graph captured for this run
    </div>
  {:else}
    {#if runArtifactError}
      <div class="border-b border-amber-900 bg-amber-950/50 px-4 py-2 text-sm text-amber-100">
        I/O artifact overlays unavailable: {runArtifactError}
      </div>
    {/if}
    {#if runNodeStatusError}
      <div class="border-b border-amber-900 bg-amber-950/50 px-4 py-2 text-sm text-amber-100">
        Node status overlays unavailable: {runNodeStatusError}
      </div>
    {/if}
    <RunGraphSnapshot {runGraph} {artifactSummaries} {nodeStatuses} />
  {/if}
</section>
