<script lang="ts">
  import { activeWorkflowRun } from '../../stores/workbenchStore';
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950 p-6">
  <h1 class="text-base font-semibold text-neutral-100">Library</h1>
  <div class="mt-4 rounded border border-neutral-800 bg-neutral-900/60 p-4 text-sm text-neutral-400">
    {#if $activeWorkflowRun}
      Run-scoped Library usage: <span class="font-mono text-neutral-100">{$activeWorkflowRun.workflow_run_id}</span>
    {:else}
      Library usage
    {/if}
  </div>
</section>
