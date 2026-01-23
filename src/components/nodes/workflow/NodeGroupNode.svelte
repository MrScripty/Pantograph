<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import type { PortDefinition } from '../../../services/workflow/types';
  import type { PortMapping, NodeGroup } from '../../../services/workflow/groupTypes';
  import { expandedGroupId, tabIntoGroup } from '../../../stores/workflowStore';

  interface Props {
    id: string;
    data: {
      group: NodeGroup;
      label?: string;
    } & Record<string, unknown>;
    selected?: boolean;
  }

  let { id, data, selected = false }: Props = $props();

  let group = $derived(data.group);
  let label = $derived(data.label || group?.name || 'Group');
  let nodeCount = $derived(group?.nodes?.length || 0);
  let isExpanded = $derived($expandedGroupId === id);

  // Convert port mappings to port definitions for rendering handles
  let inputs = $derived<PortDefinition[]>(
    (group?.exposed_inputs || []).map((mapping: PortMapping) => ({
      id: mapping.group_port_id,
      label: mapping.group_port_label,
      data_type: mapping.data_type,
      required: false,
      multiple: false,
    }))
  );

  let outputs = $derived<PortDefinition[]>(
    (group?.exposed_outputs || []).map((mapping: PortMapping) => ({
      id: mapping.group_port_id,
      label: mapping.group_port_label,
      data_type: mapping.data_type,
      required: false,
      multiple: false,
    }))
  );

  const typeColors: Record<string, string> = {
    string: '#22c55e',
    prompt: '#3b82f6',
    number: '#f59e0b',
    boolean: '#ef4444',
    image: '#8b5cf6',
    audio: '#f472b6',
    stream: '#06b6d4',
    json: '#f97316',
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

  function handleDoubleClick() {
    if (group) {
      tabIntoGroup(id);
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="node-group bg-gradient-to-br from-purple-900/50 to-indigo-900/50 rounded-lg min-w-[200px] relative"
  class:selected
  class:expanded={isExpanded}
  ondblclick={handleDoubleClick}
>
  <!-- Group Header -->
  <div class="group-header px-3 py-2 bg-purple-800/50 rounded-t-lg border-b border-purple-600/50 flex items-center gap-2">
    <svg class="w-4 h-4 text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
    </svg>
    <span class="text-sm font-medium text-purple-200">{label}</span>
    <span class="text-xs text-purple-400 ml-auto">{nodeCount} nodes</span>
  </div>

  <!-- Ports Section -->
  <div class="ports-section px-3 py-2">
    <div class="ports-grid" style="min-height: {Math.max(inputs.length, outputs.length, 1) * 20}px;">
      <!-- Input labels (left column) -->
      <div class="input-labels flex flex-col gap-1">
        {#each inputs as input}
          <span class="text-[10px] text-purple-300/70 h-4 leading-4" title="{input.data_type}">
            {input.label}
          </span>
        {/each}
      </div>
      <!-- Output labels (right column) -->
      <div class="output-labels flex flex-col gap-1 text-right">
        {#each outputs as output}
          <span class="text-[10px] text-purple-300/70 h-4 leading-4" title="{output.data_type}">
            {output.label}
          </span>
        {/each}
      </div>
    </div>
  </div>

  <!-- Double-click hint -->
  <div class="hint-section px-3 py-2 border-t border-purple-700/30">
    <span class="text-[10px] text-purple-400/60">Double-click to expand</span>
  </div>

  <!-- Handles positioned absolutely on edges -->
  {#each inputs as input, i}
    {@const yPos = 52 + i * 20}
    <Handle
      type="target"
      position={Position.Left}
      id={input.id}
      style="top: {yPos}px; background: {getPortColor(input)}; width: 10px; height: 10px; border: 2px solid #262626;"
    />
  {/each}

  {#each outputs as output, i}
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

  :global(.node-group .svelte-flow__handle) {
    border-radius: 50%;
  }
</style>
