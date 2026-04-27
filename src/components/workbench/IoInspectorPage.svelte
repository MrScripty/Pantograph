<script lang="ts">
  import { activeWorkflowRun } from '../../stores/workbenchStore';
</script>

<section class="flex h-full min-h-0 flex-col bg-neutral-950 p-6">
  <h1 class="text-base font-semibold text-neutral-100">I/O Inspector</h1>
  {#if $activeWorkflowRun}
    <div class="mt-4 rounded border border-neutral-800 bg-neutral-900/60 p-4 text-sm text-neutral-300">
      <div class="text-xs uppercase tracking-[0.22em] text-neutral-500">Selected Run</div>
      <div class="mt-2 font-mono text-neutral-100">{$activeWorkflowRun.workflow_run_id}</div>
    </div>
  {:else}
    <div class="mt-4 rounded border border-neutral-800 bg-neutral-900/60 p-4 text-sm text-neutral-500">
      No active run selected
    </div>
  {/if}
</section>
