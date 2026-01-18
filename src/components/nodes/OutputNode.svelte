<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { OutputNodeData } from '../../types/nodes';

  interface Props {
    data: OutputNodeData;
  }

  let { data }: Props = $props();

  function truncate(text: string | undefined, maxLength: number = 60): string {
    if (!text) return 'No output yet';
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + '...';
  }
</script>

<div class="node-container bg-neutral-800 border border-cyan-600/50 rounded-lg p-3 min-w-[180px]">
  <div class="flex items-center gap-2 mb-2">
    <div class="w-6 h-6 rounded bg-cyan-600 flex items-center justify-center">
      <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
    </div>
    <span class="text-sm font-medium text-neutral-200">{data.label}</span>
  </div>

  <div class="text-xs text-neutral-400 space-y-2">
    {#if data.componentPath}
      <div class="flex items-center gap-1">
        <svg class="w-3 h-3 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        <span class="text-green-400 font-mono">{data.componentPath}</span>
      </div>
    {/if}
    <div class="bg-neutral-900/50 rounded p-2 max-h-16 overflow-hidden">
      {truncate(data.lastOutput)}
    </div>
  </div>

  <Handle type="target" position={Position.Left} id="input" class="!bg-cyan-500 !w-3 !h-3" />
</div>

<style>
  .node-container {
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
  }
</style>
