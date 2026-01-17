<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { AgentNodeData } from '../../types/nodes';

  export let data: AgentNodeData;

  $: status = data.status || 'idle';
  $: modelName = data.modelName || 'Local VLM';
  $: maxTurns = data.maxTurns || 5;

  $: statusColor = {
    idle: 'bg-neutral-500',
    running: 'bg-blue-500 animate-pulse',
    success: 'bg-green-500',
    error: 'bg-red-500',
  }[status];

  $: statusText = {
    idle: 'Idle',
    running: 'Running...',
    success: 'Success',
    error: 'Error',
  }[status];
</script>

<div class="node-container bg-neutral-800 border border-green-600/50 rounded-lg p-4 min-w-[220px]">
  <div class="flex items-center gap-2 mb-3">
    <div class="w-8 h-8 rounded bg-green-600 flex items-center justify-center">
      <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
      </svg>
    </div>
    <div>
      <span class="text-sm font-medium text-neutral-200">{data.label}</span>
      <div class="flex items-center gap-1.5 mt-0.5">
        <span class="w-2 h-2 rounded-full {statusColor}"></span>
        <span class="text-xs text-neutral-400">{statusText}</span>
      </div>
    </div>
  </div>

  <div class="space-y-2 text-xs">
    <div class="flex justify-between items-center text-neutral-400">
      <span>Model:</span>
      <span class="text-neutral-200 font-mono">{modelName}</span>
    </div>
    <div class="flex justify-between items-center text-neutral-400">
      <span>Max Turns:</span>
      <span class="text-neutral-200 font-mono">{maxTurns}</span>
    </div>
  </div>

  <!-- Input handles on the left -->
  <Handle type="target" position={Position.Left} id="user-input" style="top: 25%;" class="!bg-blue-500 !w-3 !h-3" />
  <Handle type="target" position={Position.Left} id="system-prompt" style="top: 50%;" class="!bg-purple-500 !w-3 !h-3" />
  <Handle type="target" position={Position.Left} id="tools" style="top: 75%;" class="!bg-amber-500 !w-3 !h-3" />

  <!-- Output handle on the right -->
  <Handle type="source" position={Position.Right} id="result" class="!bg-green-500 !w-3 !h-3" />
</div>

<style>
  .node-container {
    box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.3);
  }
</style>
