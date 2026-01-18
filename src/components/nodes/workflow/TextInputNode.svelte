<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { NodeDefinition } from '../../../services/workflow/types';
  import { updateNodeData } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      text?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let text = $state(data.text || '');

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    text = target.value;
    updateNodeData(id, { text });
  }
</script>

<div
  class="text-input-node bg-neutral-800 rounded-lg min-w-[220px] border border-blue-600/50"
  class:selected
>
  <div class="flex items-center gap-2 px-3 py-2 bg-blue-600/20 rounded-t-lg border-b border-blue-600/30">
    <div class="w-6 h-6 rounded bg-blue-600 flex items-center justify-center">
      <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
      </svg>
    </div>
    <span class="text-sm font-medium text-neutral-200">{data.label || 'Text Input'}</span>
  </div>

  <div class="px-3 py-2">
    <textarea
      class="w-full bg-neutral-900 border border-neutral-600 rounded px-2 py-1 text-sm text-neutral-200 resize-none focus:outline-none focus:border-blue-500"
      rows="3"
      placeholder="Enter text..."
      value={text}
      oninput={handleInput}
    ></textarea>
  </div>

  <!-- Output Handle -->
  <Handle
    type="source"
    position={Position.Right}
    id="text"
    style="top: 50%; background: #22c55e; width: 12px; height: 12px; border: 2px solid #262626;"
  />
  <span class="absolute text-[10px] text-neutral-400 pointer-events-none" style="top: calc(50% - 6px); right: 16px;">
    Text
  </span>
</div>

<style>
  .text-input-node {
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
  }

  .text-input-node.selected {
    border-color: #4f46e5;
    box-shadow: 0 0 0 2px #4f46e5;
  }
</style>
