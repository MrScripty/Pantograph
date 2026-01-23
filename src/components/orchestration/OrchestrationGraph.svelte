<script lang="ts">
  import { SvelteFlow, Background, Controls, MiniMap } from '@xyflow/svelte';
  import type { Node, Edge, NodeTypes, Connection } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';

  import {
    orchestrationFlowNodes,
    orchestrationFlowEdges,
    selectedOrchestrationNodeId,
    selectOrchestrationNode,
    addOrchestrationEdge,
    removeOrchestrationEdge,
    removeOrchestrationNode,
    updateOrchestrationNodePosition,
    currentOrchestration,
  } from '../../stores/orchestrationStore';

  import StartNode from './StartNode.svelte';
  import EndNode from './EndNode.svelte';
  import ConditionNode from './ConditionNode.svelte';
  import LoopNode from './LoopNode.svelte';
  import DataGraphNode from './DataGraphNode.svelte';
  import MergeNode from './MergeNode.svelte';

  // Props
  interface Props {
    /** Handler for double-clicking a node (e.g., to zoom into DataGraph) */
    onNodeDoubleClick?: (nodeId: string) => void;
  }

  let { onNodeDoubleClick }: Props = $props();

  // Node type mapping for SvelteFlow
  const nodeTypes: NodeTypes = {
    start: StartNode,
    end: EndNode,
    condition: ConditionNode,
    loop: LoopNode,
    data_graph: DataGraphNode,
    merge: MergeNode,
  };

  // Local reactive state for SvelteFlow
  let nodes = $state<Node[]>([]);
  let edges = $state<Edge[]>([]);

  // Sync from store to local state
  $effect(() => {
    nodes = $orchestrationFlowNodes;
  });

  $effect(() => {
    edges = $orchestrationFlowEdges;
  });

  // Track click times for double-click detection
  let lastClickTime = 0;
  let lastClickNodeId: string | null = null;
  const DOUBLE_CLICK_THRESHOLD = 300; // ms

  // Handle node selection and double-click
  function handleNodeClick({ node }: { node: Node }) {
    const now = Date.now();
    const isDoubleClick =
      lastClickNodeId === node.id &&
      (now - lastClickTime) < DOUBLE_CLICK_THRESHOLD;

    if (isDoubleClick && onNodeDoubleClick) {
      onNodeDoubleClick(node.id);
    } else {
      selectOrchestrationNode(node.id);
    }

    lastClickTime = now;
    lastClickNodeId = node.id;
  }

  // Handle pane click (deselect)
  function handlePaneClick() {
    selectOrchestrationNode(null);
  }

  // Handle new connection
  async function handleConnect(connection: Connection) {
    if (connection.source && connection.target) {
      try {
        await addOrchestrationEdge(
          connection.source,
          connection.sourceHandle ?? 'next',
          connection.target,
          connection.targetHandle ?? 'input'
        );
      } catch (error) {
        console.error('Failed to add edge:', error);
      }
    }
  }

  // Handle deletion of nodes and edges
  async function handleDelete({ nodes: deletedNodes, edges: deletedEdges }: { nodes: Node[]; edges: Edge[] }) {
    // Handle edge deletion
    for (const edge of deletedEdges) {
      try {
        await removeOrchestrationEdge(edge.id);
      } catch (error) {
        console.error('Failed to remove edge:', error);
      }
    }

    // Handle node deletion
    for (const node of deletedNodes) {
      try {
        await removeOrchestrationNode(node.id);
      } catch (error) {
        console.error('Failed to remove node:', error);
      }
    }
  }

  // Handle node drag end
  async function handleNodeDragStop({ targetNode }: { targetNode: Node }) {
    try {
      await updateOrchestrationNodePosition(targetNode.id, targetNode.position.x, targetNode.position.y);
    } catch (error) {
      console.error('Failed to update node position:', error);
    }
  }
</script>

<div class="orchestration-graph">
  {#if $currentOrchestration}
    <div class="graph-header">
      <h2>{$currentOrchestration.name}</h2>
      {#if $currentOrchestration.description}
        <p class="description">{$currentOrchestration.description}</p>
      {/if}
    </div>

    <div class="graph-container">
      <SvelteFlow
        {nodes}
        {edges}
        {nodeTypes}
        fitView
        onnodeclick={handleNodeClick}
        onpaneclick={handlePaneClick}
        onconnect={handleConnect}
        ondelete={handleDelete}
        onnodedragstop={handleNodeDragStop}
      >
        <Background />
        <Controls />
        <MiniMap />
      </SvelteFlow>
    </div>
  {:else}
    <div class="empty-state">
      <p>No orchestration loaded</p>
      <p class="hint">Create a new orchestration or select one from the list</p>
    </div>
  {/if}
</div>

<style>
  .orchestration-graph {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: #1a1a1a;
  }

  .graph-header {
    padding: 12px 16px;
    border-bottom: 1px solid #333;
    background-color: #252525;
  }

  .graph-header h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: #e5e5e5;
  }

  .graph-header .description {
    margin: 4px 0 0 0;
    font-size: 12px;
    color: #888;
  }

  .graph-container {
    flex: 1;
    min-height: 0;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #888;
  }

  .empty-state p {
    margin: 4px 0;
  }

  .empty-state .hint {
    font-size: 12px;
    color: #666;
  }

  /* Style SvelteFlow within our component */
  .graph-container :global(.svelte-flow) {
    background-color: #1a1a1a;
  }

  .graph-container :global(.svelte-flow__node) {
    font-family: inherit;
  }

  .graph-container :global(.svelte-flow__edge-path) {
    stroke: #666;
    stroke-width: 2;
  }

  .graph-container :global(.svelte-flow__edge.selected .svelte-flow__edge-path) {
    stroke: #3b82f6;
  }

  .graph-container :global(.svelte-flow__handle) {
    width: 12px;
    height: 12px;
    border: 2px solid #333;
    background-color: #666;
  }

  .graph-container :global(.svelte-flow__handle:hover) {
    background-color: #3b82f6;
  }
</style>
