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


  function handleOpenGroup() {
    if (group) {
      tabIntoGroup(id);
    }
  }
</script>

<div
  class="node-group rounded min-w-[200px] relative"
  class:selected
  class:expanded={isExpanded}
>
  <!-- Group Header -->
  <div class="group-header px-3 py-2 flex items-center gap-2">
    <svg class="w-4 h-4 text-pg-text-muted" aria-hidden="true" fill="none" viewBox="0 0 24 24" stroke="currentColor">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
    </svg>
    <span class="group-title">{label}</span>
    <span class="group-count ml-auto">{nodeCount} nodes</span>
  </div>

  <!-- Ports Section -->
  <div class="ports-section px-3 py-2">
    <div class="ports-grid" style="min-height: {Math.max(inputs.length, outputs.length, 1) * 20}px;">
      <!-- Input labels (left column) -->
      <div class="input-labels flex flex-col gap-1">
        {#each inputs as input (input.id)}
          <span class="port-label h-4 leading-4" title="{input.data_type}">
            {input.label}
          </span>
        {/each}
      </div>
      <!-- Output labels (right column) -->
      <div class="output-labels flex flex-col gap-1 text-right">
        {#each outputs as output (output.id)}
          <span class="port-label h-4 leading-4" title="{output.data_type}">
            {output.label}
          </span>
        {/each}
      </div>
    </div>
  </div>

  <!-- Double-click hint -->
  <div class="hint-section px-3 py-2 flex items-center justify-between">
    <span class="text-[10px] text-pg-text-muted">Open group to edit internals</span>
    <button type="button" class="open-group-btn text-[10px]" onclick={handleOpenGroup}>
      Open
    </button>
  </div>

  <!-- Handles positioned absolutely on edges -->
  {#each inputs as input, i (input.id)}
    {@const yPos = 52 + i * 20}
    <Handle
      type="target"
      position={Position.Left}
      id={input.id}
      style="top: {yPos}px; width: 10px; height: 10px;"
    />
  {/each}

  {#each outputs as output, i (output.id)}
    {@const yPos = 52 + i * 20}
    <Handle
      type="source"
      position={Position.Right}
      id={output.id}
      style="top: {yPos}px; width: 10px; height: 10px;"
    />
  {/each}
</div>

<style>
  /* Token colours only. The 2px border, 37px header and ports padding place
     the handles at 52px + 20px per port; keep them in step. */
  .node-group {
    background: var(--pg-panel);
    border: 2px dashed var(--pg-border-strong);
    color: var(--pg-text);
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.25);
  }

  .node-group.selected {
    border-color: var(--pg-text);
    box-shadow: 0 0 0 1px var(--pg-text);
  }

  .node-group.expanded {
    border-style: solid;
  }

  .group-header {
    border-bottom: 1px solid var(--pg-border);
    font-size: 14px;
    line-height: 20px;
  }

  .group-title {
    font-size: 12.5px;
    font-weight: 700;
    color: var(--pg-text-bright);
  }

  .group-count {
    font-size: 11px;
    color: var(--pg-text-muted);
  }

  .port-label {
    font-size: 10.5px;
    color: var(--pg-text-soft);
  }

  .hint-section {
    border-top: 1px solid var(--pg-border);
  }

  .ports-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .open-group-btn {
    border: 1px solid var(--pg-border);
    background: var(--pg-panel-raised);
    color: var(--pg-text-soft);
    border-radius: 4px;
    padding: 0.15rem 0.45rem;
    line-height: 1.2;
    cursor: pointer;
  }

  .open-group-btn:hover {
    background: var(--pg-panel-selected);
    color: var(--pg-text);
  }

  :global(.node-group .svelte-flow__handle) {
    border-radius: 50%;
    background: var(--pg-panel-head);
    border: 1.5px solid var(--pg-border-strong);
  }
</style>
