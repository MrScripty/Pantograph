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
  import { graphSessionError, isReadOnly } from '../../stores/graphSessionStore';
  import { activeWorkflowRun, diagnosticsFocus } from '../../stores/workbenchStore';
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
  let loadingRunInspection = $state(false);
  let runInspectionError = $state<string | null>(null);
  let lastRunId = $state<string | null>(null);
  let runInspectionRequestSerial = 0;
  let artifactSummaries = $derived(buildRunGraphNodeArtifactSummaries(runArtifacts));
  let nodeStatuses = $derived(buildRunGraphNodeStatusMap(runNodeStatuses));
  let focusedDiagnosticEventId = $derived(
    $diagnosticsFocus?.workflow_run_id === $activeWorkflowRun?.workflow_run_id
      ? ($diagnosticsFocus?.diagnostic_event_id ?? null)
      : null,
  );

  function activeRunId(): string | null {
    return $activeWorkflowRun?.workflow_run_id ?? null;
  }

  async function refreshRunInspection(runId = activeRunId()): Promise<void> {
    const requestSerial = ++runInspectionRequestSerial;
    runInspectionError = null;

    if (!runId) {
      runGraph = null;
      runArtifacts = [];
      runNodeStatuses = [];
      loadingRunInspection = false;
      return;
    }

    loadingRunInspection = true;
    try {
      const response = await workflowService.queryRunInspection({
        workflow_run_id: runId,
        artifact_limit: 250,
      });
      if (requestSerial !== runInspectionRequestSerial) {
        return;
      }
      runGraph = response.run_graph ?? null;
      runArtifacts = response.io_artifacts;
      runNodeStatuses = response.node_statuses;
    } catch (error) {
      if (requestSerial !== runInspectionRequestSerial) {
        return;
      }
      runInspectionError = formatWorkflowCommandError(error);
      runGraph = null;
      runArtifacts = [];
      runNodeStatuses = [];
    } finally {
      if (requestSerial === runInspectionRequestSerial) {
        loadingRunInspection = false;
      }
    }
  }

  function refreshRunSnapshot(): void {
    void refreshRunInspection();
  }

  $effect(() => {
    const runId = activeRunId();
    if (runId === lastRunId) {
      return;
    }

    lastRunId = runId;
    mode = runId ? 'run_snapshot' : 'editor';
    void refreshRunInspection(runId);
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
            disabled={loadingRunInspection}
          >
            <RefreshCw
              size={14}
              aria-hidden="true"
              class={loadingRunInspection ? 'animate-spin' : ''}
            />
            Refresh
          </button>
        {/if}
      </div>
    </div>
  {/if}

  {#if !$activeWorkflowRun || mode === 'editor'}
    <WorkflowToolbar />
    {#if $activeWorkflowRun}
      <div class="border-b border-neutral-800 bg-neutral-900/60 px-4 py-2 text-xs text-neutral-400">
        Editing the current workflow. Selected run remains
        <span class="font-mono text-neutral-200">{$activeWorkflowRun.workflow_run_id}</span>
        for other workbench pages.
      </div>
    {/if}
    {#if $graphSessionError}
      <div class="border-b border-red-900 bg-red-950/60 px-4 py-2 text-sm text-red-200">
        {$graphSessionError}
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
  {:else if loadingRunInspection && !runGraph}
    <div class="flex min-h-0 flex-1 items-center justify-center text-sm text-neutral-500">
      Loading run graph
    </div>
  {:else if runInspectionError}
    <div class="border-b border-red-900 bg-red-950/50 px-4 py-2 text-sm text-red-200">{runInspectionError}</div>
    <div class="flex min-h-0 flex-1 items-center justify-center text-sm text-neutral-500">
      Run graph unavailable
    </div>
  {:else if !runGraph}
    <div class="flex min-h-0 flex-1 items-center justify-center text-sm text-neutral-500">
      No versioned graph captured for this run
    </div>
  {:else}
    <RunGraphSnapshot {runGraph} {artifactSummaries} {nodeStatuses} {focusedDiagnosticEventId} />
  {/if}
</section>
