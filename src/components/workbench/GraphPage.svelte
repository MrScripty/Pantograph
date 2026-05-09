<script lang="ts">
  import NodePalette from '../NodePalette.svelte';
  import WorkflowGraph from '../WorkflowGraph.svelte';
  import WorkflowToolbar from '../WorkflowToolbar.svelte';
  import { graphSessionError, isReadOnly } from '../../stores/graphSessionStore';
  import { activeWorkflowRun } from '../../stores/workbenchStore';
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950">
  <WorkflowToolbar />
  {#if $activeWorkflowRun}
    <div class="border-b border-neutral-800 bg-neutral-900/60 px-4 py-2 text-xs text-neutral-400">
      Editing the current workflow. Selected run remains
      <span class="font-mono text-neutral-200">{$activeWorkflowRun.workflow_run_id}</span>
      for Scheduler, Diagnostics, I/O Inspector, Library, and Network pages.
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
</section>
