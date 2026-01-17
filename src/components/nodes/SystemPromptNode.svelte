<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { SystemPromptNodeData } from '../../types/nodes';

  export let data: SystemPromptNodeData;

  // Truncate prompt for preview
  function truncate(text: string | undefined, maxLength: number = 50): string {
    if (!text) return 'Click to edit...';
    if (text.length <= maxLength) return text;
    return text.slice(0, maxLength) + '...';
  }
</script>

<div
  class="node-container bg-neutral-800 border border-purple-600/50 rounded-lg p-3 w-[200px] cursor-pointer hover:border-purple-500 transition-colors"
  onclick={() => data.onEdit?.()}
  onkeydown={(e) => e.key === 'Enter' && data.onEdit?.()}
  role="button"
  tabindex="0"
>
  <div class="flex items-center gap-2 mb-2">
    <div class="w-6 h-6 rounded bg-purple-600 flex items-center justify-center">
      <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
      </svg>
    </div>
    <span class="text-sm font-medium text-neutral-200">{data.label}</span>
    <svg class="w-3 h-3 text-purple-400 ml-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
    </svg>
  </div>

  <div class="text-xs text-neutral-400 bg-neutral-900/50 rounded p-2 max-h-16 overflow-hidden">
    {truncate(data.promptPreview, 100)}
  </div>

  <div class="text-xs text-purple-400 mt-2 text-center">
    Click to edit
  </div>

  <Handle type="source" position={Position.Right} id="output" class="!bg-purple-500 !w-3 !h-3" />
</div>

<style>
  .node-container {
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
  }
</style>
