<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { PortDefinition } from '../../../services/workflow/types';
  import { groupDisplay } from './groupDisplayDiagnostics';
  import GroupJsonFilterPathEditor from './GroupJsonFilterPathEditor.svelte';
  import { currentSessionId, isReadOnly } from '../../../stores/graphSessionStore';
  import { expandedGroupId, isEditing, tabIntoGroup } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      group?: unknown;
      label?: string;
    } & Record<string, unknown>;
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let editPaths = $state(false);
  let display = $derived(groupDisplay(data));
  let label = $derived(display.label);
  let nodeCount = $derived(display.nodeCount);
  let isExpanded = $derived($expandedGroupId === id);
  let inputs = $derived(display.inputs);
  let outputs = $derived(display.outputs);
  let filterNodes = $derived(display.nodes.filter(node => node.node_type === 'json-filter'));

  const typeColors: Record<string, string> = {
    string: '#22c55e',
    prompt: '#3b82f6',
    number: '#f59e0b',
    boolean: '#ef4444',
    image: '#8b5cf6',
    audio: '#f472b6',
    audio_stream: '#0ea5e9',
    stream: '#06b6d4',
    json: '#f97316',
    kv_cache: '#84cc16',
    component: '#ec4899',
    document: '#14b8a6',
    tools: '#d97706',
    embedding: '#6366f1',
    vector_db: '#a855f7',
    any: '#6b7280',
  };

  function getPortColor(port: PortDefinition): string {
    return typeColors[port.data_type] || typeColors.any;
  }

  function handleOpenGroup() {
    if (!display.diagnostics.length) {
      tabIntoGroup(id);
    }
  }
</script>

<div
  class="node-group bg-gradient-to-br from-purple-900/50 to-indigo-900/50 rounded-lg min-w-[200px] relative"
  class:selected
  class:expanded={isExpanded}
>
  <!-- Group Header -->
  <div class="group-header px-3 py-2 bg-purple-800/50 rounded-t-lg border-b border-purple-600/50 flex items-center gap-2">
    <svg class="w-4 h-4 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
    </svg>
    <span class="text-sm font-medium text-purple-200">{label}</span>
    <span class="text-xs text-purple-400 ml-auto">{nodeCount} nodes</span>
  </div>

  {#if display.diagnostics.length}
    <div class="nodrag nopan nowheel px-3 py-2 text-xs text-amber-200" role="status" data-testid="group-display-diagnostics">
      <p class="font-semibold">Group editor unavailable</p>
      <ul class="mt-2 space-y-2">
        {#each display.diagnostics as diagnostic, index (index)}
          <li><code>{diagnostic.code}</code>: <code>{diagnostic.field}</code><br />{diagnostic.message}</li>
        {/each}
      </ul>
      <p class="mt-2">{display.diagnostics[0].hint}</p>
      <p class="mt-2">This display check does not determine whether the workflow can execute.</p>
    </div>
  {/if}

  <!-- Ports Section -->
  <div class="ports-section px-3 py-2">
    <div class="ports-grid" style="min-height: {Math.max(inputs.length, outputs.length, 1) * 20}px;">
      <!-- Input labels (left column) -->
      <div class="input-labels flex flex-col gap-1">
        {#each inputs as input (input.id)}
          <span class="text-[10px] text-purple-300/70 h-4 leading-4" title="{input.data_type}">
            {input.label}
          </span>
        {/each}
      </div>
      <!-- Output labels (right column) -->
      <div class="output-labels flex flex-col gap-1 text-right">
        {#each outputs as output (output.id)}
          <span class="text-[10px] text-purple-300/70 h-4 leading-4" title="{output.data_type}">
            {output.label}
          </span>
        {/each}
      </div>
    </div>
  </div>

  <!-- Double-click hint -->
  <div class="hint-section px-3 py-2 border-t border-purple-700/30 flex items-center justify-between">
    <span class="text-[10px] text-purple-400/60">Open group to edit internals</span>
    <button type="button" class="open-group-btn text-[10px] disabled:opacity-50" onclick={handleOpenGroup} disabled={display.diagnostics.length > 0}>
      Open
    </button>
  </div>

  {#if $isEditing && !$isReadOnly && filterNodes.length > 0}
    <div class="nodrag nopan nowheel px-3 py-2 border-t border-purple-700/30">
      <button type="button" class="open-group-btn text-xs" onclick={() => { editPaths = !editPaths; }}
        aria-expanded={editPaths}>JSON Filter paths</button>
      {#if editPaths}
        {#each filterNodes as node (node.id)}
          {#key JSON.stringify([$currentSessionId, id, node.id, node.node_type])}
            <div class="mt-3 space-y-2" data-testid="group-json-filter-editor">
              <p class="text-xs text-purple-200">{node.id}</p>
              <GroupJsonFilterPathEditor groupId={id} {node} sessionId={$currentSessionId} />
            </div>
          {/key}
        {/each}
      {/if}
    </div>
  {/if}

  <!-- Handles positioned absolutely on edges -->
  {#each inputs as input, i (input.id)}
    {@const yPos = 52 + i * 20}
    <Handle
      type="target"
      position={Position.Left}
      id={input.id}
      style="top: {yPos}px; background: {getPortColor(input)}; width: 10px; height: 10px; border: 2px solid #262626;"
    />
  {/each}

  {#each outputs as output, i (output.id)}
    {@const yPos = 52 + i * 20}
    <Handle
      type="source"
      position={Position.Right}
      id={output.id}
      style="top: {yPos}px; background: {getPortColor(output)}; width: 10px; height: 10px; border: 2px solid #262626;"
    />
  {/each}
</div>

<style>
  .node-group {
    border: 2px dashed #7c3aed;
    box-shadow: 0 4px 6px -1px rgba(139, 92, 246, 0.2);
  }

  .node-group.selected {
    border-color: #a78bfa;
    box-shadow: 0 0 0 2px #a78bfa, 0 4px 6px -1px rgba(139, 92, 246, 0.3);
  }

  .node-group.expanded {
    border-style: solid;
    border-color: #c4b5fd;
  }

  .ports-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .open-group-btn {
    border: 1px solid rgba(167, 139, 250, 0.45);
    background: rgba(124, 58, 237, 0.2);
    color: #ddd6fe;
    border-radius: 9999px;
    padding: 0.15rem 0.45rem;
    line-height: 1.2;
    cursor: pointer;
  }

  .open-group-btn:hover {
    background: rgba(124, 58, 237, 0.35);
    border-color: rgba(167, 139, 250, 0.8);
  }

  :global(.node-group .svelte-flow__handle) {
    border-radius: 50%;
  }
</style>
