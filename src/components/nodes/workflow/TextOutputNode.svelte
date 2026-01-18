<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { NodeDefinition, NodeExecutionState } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

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

  let executionState = $derived($nodeExecutionStates.get(id) || 'idle');
  let text = $derived(data.text || '');

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-cyan-500 animate-pulse',
      success: 'bg-cyan-500',
      error: 'bg-red-500',
    }[executionState]
  );
</script>

<div
  class="output-node bg-neutral-800 rounded-lg min-w-[220px] border border-cyan-600/50"
  class:selected
>
  <div class="flex items-center gap-2 px-3 py-2 bg-cyan-600/20 rounded-t-lg border-b border-cyan-600/30">
    <div class="w-6 h-6 rounded bg-cyan-600 flex items-center justify-center">
      <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
      </svg>
    </div>
    <div class="flex-1">
      <span class="text-sm font-medium text-neutral-200">{data.label || 'Text Output'}</span>
    </div>
    <span class="w-2 h-2 rounded-full {statusColor}"></span>
  </div>

  <div class="px-3 py-2">
    {#if text}
      <div class="p-2 bg-neutral-900 rounded text-xs text-neutral-300 max-h-32 overflow-y-auto whitespace-pre-wrap">
        {text}
      </div>
    {:else}
      <div class="text-xs text-neutral-500 italic">
        No output yet
      </div>
    {/if}
  </div>

  <!-- Input Handle -->
  <Handle
    type="target"
    position={Position.Left}
    id="text"
    style="top: 50%; background: #22c55e; width: 12px; height: 12px; border: 2px solid #262626;"
  />
  <span class="absolute text-[10px] text-neutral-400 pointer-events-none" style="top: calc(50% - 6px); left: 16px;">
    Text
  </span>
</div>

<style>
  .output-node {
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
  }

  .output-node.selected {
    border-color: #4f46e5;
    box-shadow: 0 0 0 2px #4f46e5;
  }
</style>
