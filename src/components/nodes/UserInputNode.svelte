<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { UserInputNodeData } from '../../types/nodes';

  export let data: UserInputNodeData;
</script>

<div class="node-container bg-neutral-800 border border-neutral-600 rounded-lg p-3 w-[200px]">
  <div class="flex items-center gap-2 mb-2">
    <div class="w-6 h-6 rounded bg-blue-600 flex items-center justify-center">
      <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
      </svg>
    </div>
    <span class="text-sm font-medium text-neutral-200">{data.label}</span>
  </div>

  <div class="text-xs text-neutral-400 space-y-1">
    <div class="flex items-center gap-1">
      <span class="text-neutral-500">Image:</span>
      <span class="text-neutral-300">Canvas Drawing</span>
    </div>
    <div class="flex items-center gap-1">
      <span class="text-neutral-500">Text:</span>
      <span class="text-neutral-300 truncate max-w-[120px]">{data.promptText || 'User prompt'}</span>
    </div>
  </div>

  <Handle type="source" position={Position.Right} id="output" class="!bg-blue-500 !w-3 !h-3" />
</div>

<style>
  .node-container {
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
  }
</style>
