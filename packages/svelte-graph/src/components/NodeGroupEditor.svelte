<script lang="ts">
  import {
    SvelteFlow,
    Controls,
    MiniMap,
    type NodeTypes,
    type EdgeTypes,
    type Node,
    type Edge,
    type Connection,
  } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import { get } from 'svelte/store';

  import { useGraphContext } from '../context/useGraphContext.js';
  import type { NodeGroup, PortMapping } from '../types/groups.js';
  import type { GraphEdge } from '../types/workflow.js';
  import ReconnectableEdge from './edges/ReconnectableEdge.svelte';

  const { registry, stores } = useGraphContext();

  interface Props {
    group: NodeGroup;
    onSave: (nodes: Node[], edges: Edge[], exposedInputs: PortMapping[], exposedOutputs: PortMapping[]) => void;
    onCancel: () => void;
  }

  let { group, onSave, onCancel }: Props = $props();

  const { nodeDefinitions: nodeDefsStore } = stores.workflow;

  const edgeTypes: EdgeTypes = (registry.edgeTypes || { reconnectable: ReconnectableEdge }) as unknown as EdgeTypes;
  const nodeTypes: NodeTypes = registry.nodeTypes as unknown as NodeTypes;

  let nodes = $state.raw<Node[]>(
    group.nodes.map((n) => {
      const definition = get(nodeDefsStore).find((d) => d.node_type === n.node_type);
      return {
        id: n.id,
        type: n.node_type,
        position: n.position,
        data: {
          ...n.data,
          definition,
        },
      };
    })
  );

  let edges = $state.raw<Edge[]>(
    group.edges.map((e) => ({
      id: e.id,
      source: e.source,
      sourceHandle: e.source_handle,
      target: e.target,
      targetHandle: e.target_handle,
    }))
  );

  let exposedInputs = $state<PortMapping[]>([...group.exposed_inputs]);
  let exposedOutputs = $state<PortMapping[]>([...group.exposed_outputs]);

  async function handleConnect(connection: Connection) {
    const newEdge: Edge = {
      id: `${connection.source}-${connection.sourceHandle}-${connection.target}-${connection.targetHandle}`,
      source: connection.source!,
      sourceHandle: connection.sourceHandle!,
      target: connection.target!,
      targetHandle: connection.targetHandle!,
    };

    edges = [...edges, newEdge];
  }

  function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: Node[]; edges: Edge[] }) {
    if (deletedNodes.length > 0) {
      const deletedIds = new Set(deletedNodes.map((n) => n.id));
      nodes = nodes.filter((n) => !deletedIds.has(n.id));
      edges = edges.filter((e) => !deletedIds.has(e.source) && !deletedIds.has(e.target));
      exposedInputs = exposedInputs.filter((p) => !deletedIds.has(p.internal_node_id));
      exposedOutputs = exposedOutputs.filter((p) => !deletedIds.has(p.internal_node_id));
    }

    if (deletedEdges.length > 0) {
      const deletedIds = new Set(deletedEdges.map((e) => e.id));
      edges = edges.filter((e) => !deletedIds.has(e.id));
    }
  }

  function handleSave() {
    onSave(nodes, edges, exposedInputs, exposedOutputs);
  }

  function handleBack() {
    stores.workflow.collapseGroup();
    onCancel();
  }
</script>

<div class="group-editor-container">
  <!-- Header bar -->
  <div class="editor-header">
    <button class="back-btn" onclick={handleBack}>
      <svg width="20" height="20" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
      </svg>
      Back
    </button>

    <div class="editor-title-row">
      <svg class="title-icon" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
      </svg>
      <span class="editor-title">Editing: {group.name}</span>
    </div>

    <div class="editor-actions">
      <span class="node-count">{nodes.length} nodes</span>
      <button class="save-btn" onclick={handleSave}>Save & Close</button>
    </div>
  </div>

  <!-- Graph editor -->
  <div class="editor-canvas">
    <SvelteFlow
      bind:nodes
      bind:edges
      {nodeTypes}
      {edgeTypes}
      fitView
      fitViewOptions={{ maxZoom: 1, padding: 0.2 }}
      nodesConnectable={true}
      elementsSelectable={true}
      nodesDraggable={true}
      panOnDrag={true}
      zoomOnScroll={true}
      minZoom={0.25}
      maxZoom={2}
      deleteKey="Delete"
      onconnect={handleConnect}
      ondelete={handleDelete}
      defaultEdgeOptions={{
        type: 'reconnectable',
        animated: false,
        style: 'stroke: #7c3aed; stroke-width: 2px;',
        interactionWidth: 20,
        selectable: true,
        focusable: true,
      }}
    >
      <Controls />
      <MiniMap
        nodeColor={() => '#7c3aed'}
        maskColor="rgba(139, 92, 246, 0.1)"
      />
    </SvelteFlow>

    <!-- Exposed ports indicator -->
    <div class="ports-indicator">
      <div class="ports-indicator-title">Exposed Ports</div>
      <div class="ports-indicator-grid">
        <div>
          <div class="ports-indicator-label">Inputs ({exposedInputs.length})</div>
          {#each exposedInputs as input}
            <div class="ports-indicator-item">{input.group_port_label}</div>
          {/each}
          {#if exposedInputs.length === 0}
            <div class="ports-indicator-empty">None</div>
          {/if}
        </div>
        <div>
          <div class="ports-indicator-label">Outputs ({exposedOutputs.length})</div>
          {#each exposedOutputs as output}
            <div class="ports-indicator-item">{output.group_port_label}</div>
          {/each}
          {#if exposedOutputs.length === 0}
            <div class="ports-indicator-empty">None</div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .group-editor-container {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  /* --- Header --- */
  .editor-header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 1rem;
    background-color: rgba(88, 28, 135, 0.3);
    border-bottom: 1px solid rgba(147, 51, 234, 0.3);
  }

  .back-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: none;
    border: none;
    color: #d8b4fe;
    cursor: pointer;
    font-size: inherit;
    transition: color 150ms;
  }

  .back-btn:hover {
    color: #f3e8ff;
  }

  .editor-title-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .title-icon {
    width: 1.25rem;
    height: 1.25rem;
    color: #c084fc;
  }

  .editor-title {
    font-size: 1.125rem;
    font-weight: 500;
    color: #e9d5ff;
  }

  .editor-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .node-count {
    font-size: 0.875rem;
    color: #c084fc;
  }

  .save-btn {
    padding: 0.25rem 0.75rem;
    font-size: 0.875rem;
    background-color: #9333ea;
    border: none;
    border-radius: 0.25rem;
    color: white;
    cursor: pointer;
    transition: background-color 150ms;
  }

  .save-btn:hover {
    background-color: #a855f7;
  }

  /* --- Canvas --- */
  .editor-canvas {
    flex: 1;
    position: relative;
  }

  /* --- Ports Indicator --- */
  .ports-indicator {
    position: absolute;
    bottom: 1rem;
    left: 1rem;
    background-color: rgba(38, 38, 38, 0.9);
    border-radius: 0.5rem;
    padding: 0.75rem;
    font-size: 0.875rem;
  }

  .ports-indicator-title {
    color: #d8b4fe;
    font-weight: 500;
    margin-bottom: 0.5rem;
  }

  .ports-indicator-grid {
    display: flex;
    gap: 1rem;
  }

  .ports-indicator-label {
    color: #a3a3a3;
    font-size: 0.75rem;
    margin-bottom: 0.25rem;
  }

  .ports-indicator-item {
    color: #c084fc;
    font-size: 0.75rem;
  }

  .ports-indicator-empty {
    color: #737373;
    font-size: 0.75rem;
  }

  /* --- SvelteFlow overrides --- */
  :global(.group-editor-container .svelte-flow) {
    background-color: transparent !important;
    background-image: none !important;
  }

  :global(.group-editor-container .svelte-flow__background) {
    display: none !important;
  }

  :global(.group-editor-container .svelte-flow__edge-path) {
    stroke: #7c3aed;
    stroke-width: 2px;
  }

  :global(.group-editor-container .svelte-flow__edge.selected .svelte-flow__edge-path) {
    stroke: #a78bfa;
    stroke-width: 3px;
  }

  :global(.group-editor-container .svelte-flow__controls) {
    background-color: #262626;
    border: 1px solid #7c3aed;
    border-radius: 8px;
  }

  :global(.group-editor-container .svelte-flow__controls-button) {
    background-color: #262626;
    border-color: #7c3aed;
    color: #a3a3a3;
  }

  :global(.group-editor-container .svelte-flow__controls-button:hover) {
    background-color: #7c3aed;
  }

  :global(.group-editor-container .svelte-flow__minimap) {
    background-color: #171717;
    border: 1px solid #7c3aed;
    border-radius: 8px;
  }
</style>
