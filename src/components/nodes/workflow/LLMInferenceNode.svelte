<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { NodeDefinition, NodeExecutionState } from '../../../services/workflow/types';
  import { nodeExecutionStates } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      definition?: NodeDefinition;
      label?: string;
      modelName?: string;
      streamContent?: string;
    };
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let executionState = $derived($nodeExecutionStates.get(id) || 'idle');
  let modelName = $derived(data.modelName || 'Local LLM');
  let streamContent = $derived(data.streamContent || '');

  let statusColor = $derived(
    {
      idle: 'bg-neutral-500',
      running: 'bg-green-500 animate-pulse',
      success: 'bg-green-500',
      error: 'bg-red-500',
    }[executionState]
  );

  let statusText = $derived(
    {
      idle: 'Idle',
      running: 'Running...',
      success: 'Complete',
      error: 'Error',
    }[executionState]
  );
</script>

<div
  class="llm-node bg-neutral-800 rounded-lg min-w-[240px] border border-green-600/50"
  class:selected
>
  <div class="flex items-center gap-2 px-3 py-2 bg-green-600/20 rounded-t-lg border-b border-green-600/30">
    <div class="w-6 h-6 rounded bg-green-600 flex items-center justify-center">
      <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
      </svg>
    </div>
    <div class="flex-1">
      <span class="text-sm font-medium text-neutral-200">{data.label || 'LLM Inference'}</span>
      <div class="flex items-center gap-1.5 mt-0.5">
        <span class="w-2 h-2 rounded-full {statusColor}"></span>
        <span class="text-xs text-neutral-400">{statusText}</span>
      </div>
    </div>
  </div>

  <div class="px-3 py-2 space-y-2">
    <div class="flex justify-between items-center text-xs">
      <span class="text-neutral-400">Model:</span>
      <span class="text-neutral-200 font-mono">{modelName}</span>
    </div>

    {#if streamContent}
      <div class="mt-2 p-2 bg-neutral-900 rounded text-xs text-neutral-300 max-h-20 overflow-y-auto">
        {streamContent}
      </div>
    {/if}
  </div>

  <!-- Input Handles -->
  <Handle
    type="target"
    position={Position.Left}
    id="prompt"
    style="top: 48px; background: #3b82f6; width: 12px; height: 12px; border: 2px solid #262626;"
  />
  <span class="absolute text-[10px] text-neutral-400 pointer-events-none" style="top: 42px; left: 16px;">
    Prompt
  </span>

  <Handle
    type="target"
    position={Position.Left}
    id="system_prompt"
    style="top: 76px; background: #22c55e; width: 12px; height: 12px; border: 2px solid #262626;"
  />
  <span class="absolute text-[10px] text-neutral-400 pointer-events-none" style="top: 70px; left: 16px;">
    System
  </span>

  <!-- Output Handles -->
  <Handle
    type="source"
    position={Position.Right}
    id="response"
    style="top: 48px; background: #22c55e; width: 12px; height: 12px; border: 2px solid #262626;"
  />
  <span class="absolute text-[10px] text-neutral-400 pointer-events-none text-right" style="top: 42px; right: 16px;">
    Response
  </span>

  <Handle
    type="source"
    position={Position.Right}
    id="stream"
    style="top: 76px; background: #06b6d4; width: 12px; height: 12px; border: 2px solid #262626;"
  />
  <span class="absolute text-[10px] text-neutral-400 pointer-events-none text-right" style="top: 70px; right: 16px;">
    Stream
  </span>
</div>

<style>
  .llm-node {
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
  }

  .llm-node.selected {
    border-color: #4f46e5;
    box-shadow: 0 0 0 2px #4f46e5;
  }
</style>
