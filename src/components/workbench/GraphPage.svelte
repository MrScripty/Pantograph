<script lang="ts">
  import NodePalette from '../NodePalette.svelte';
  import WorkflowGraph from '../WorkflowGraph.svelte';
  import WorkflowToolbar from '../WorkflowToolbar.svelte';
  import { isReadOnly } from '../../stores/graphSessionStore';
  import { activeWorkflowRun } from '../../stores/workbenchStore';
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950">
  <WorkflowToolbar showDiagnosticsToggle={false} />
  {#if $activeWorkflowRun}
    <div class="border-b border-neutral-800 bg-neutral-900/60 px-4 py-2 text-xs text-neutral-400">
      Selected run: <span class="font-mono text-neutral-200">{$activeWorkflowRun.workflow_run_id}</span>
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
