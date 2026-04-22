<script lang="ts">
  interface Props {
    activityLog: string[];
    isBusy: boolean;
    onClear: () => void;
  }

  let { activityLog, isBusy, onClear }: Props = $props();
  let activityLogContainer: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (!activityLogContainer || activityLog.length === 0) return;
    activityLogContainer.scrollTop = activityLogContainer.scrollHeight;
  });
</script>

<div class="rounded border border-neutral-700 bg-neutral-950/50 px-2 py-1 space-y-1">
  <div class="flex items-center gap-2">
    <span class="text-[10px] text-neutral-300">Activity Log</span>
    <span class="ml-auto text-[9px] text-neutral-500">{activityLog.length} line(s)</span>
    <button
      type="button"
      class="text-[9px] text-neutral-500 hover:text-neutral-300 disabled:opacity-50"
      onclick={onClear}
      disabled={isBusy || activityLog.length === 0}
    >
      Clear
    </button>
  </div>
  <div
    bind:this={activityLogContainer}
    class="copyable-activity-log nodrag nopan nowheel h-28 overflow-y-auto rounded border border-neutral-800 bg-black/40 px-2 py-1 font-mono text-[9px] leading-4 text-neutral-300"
  >
    {#if activityLog.length === 0}
      <div class="text-neutral-500">No activity yet. Use Run/Resolve/Check/Install to capture logs.</div>
    {:else}
      {#each activityLog as line, i (`${i}:${line}`)}
        <div class="whitespace-pre-wrap break-words">{line}</div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .copyable-activity-log {
    user-select: text;
    -webkit-user-select: text;
    cursor: text;
  }
</style>
