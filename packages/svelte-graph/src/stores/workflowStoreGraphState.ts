import type { Edge, Node } from '@xyflow/svelte';
import { derived, get, writable, type Readable, type Writable } from 'svelte/store';

import { buildDerivedGraph } from '../graphRevision.ts';
import type { NodeDefinition, WorkflowGraph } from '../types/workflow.js';
import { buildDefaultWorkflowGraphState } from './defaultWorkflowGraph.ts';
import {
  appendNodeStreamContentOverlay,
  clearNodeRuntimeOverlayKeys,
  clearNodeStreamContentOverlay,
  mergeNodeRuntimeOverlays,
  setNodeStreamContentOverlay,
  updateNodeRuntimeOverlay,
} from './runtimeData.ts';
import {
  materializeWorkflowGraphSnapshot,
  projectWorkflowGraphStoreState,
  type WorkflowGraphMaterialization,
} from './workflowStoreMaterialization.ts';

export interface WorkflowStoreGraphState {
  applyWorkflowGraph: (graph: WorkflowGraph) => WorkflowGraphMaterialization;
  appendStreamContent: (nodeId: string, chunk: string) => void;
  clearGraph: () => void;
  clearNodeRuntimeData: (keys: string[]) => void;
  clearRuntimeOverlays: () => void;
  clearStreamContent: () => void;
  derivedGraph: Writable<WorkflowGraph['derived_graph']>;
  edges: Writable<Edge[]>;
  loadDefaultWorkflow: (definitions: NodeDefinition[]) => void;
  nodes: Readable<Node[]>;
  setStreamContent: (nodeId: string, content: string) => void;
  structuralNodes: Writable<Node[]>;
  updateNodeRuntimeData: (nodeId: string, data: Record<string, unknown>) => void;
  workflowGraph: Readable<WorkflowGraph>;
}

export function createWorkflowStoreGraphState(params: {
  nodeDefinitions: Readable<NodeDefinition[]>;
  selectedNodeIds: Readable<string[]>;
}): WorkflowStoreGraphState {
  const structuralNodes = writable<Node[]>([]);
  const nodeRuntimeOverlays = writable<Map<string, Record<string, unknown>>>(new Map());
  const nodes = derived(
    [structuralNodes, nodeRuntimeOverlays],
    ([$structuralNodes, $nodeRuntimeOverlays]) =>
      mergeNodeRuntimeOverlays($structuralNodes, $nodeRuntimeOverlays),
  );
  const edges = writable<Edge[]>([]);
  const derivedGraph = writable<WorkflowGraph['derived_graph']>(undefined);

  const workflowGraph = derived(
    [structuralNodes, edges, derivedGraph],
    ([$nodes, $edges, $derivedGraph]): WorkflowGraph =>
      projectWorkflowGraphStoreState({
        nodes: $nodes,
        edges: $edges,
        derivedGraph: $derivedGraph,
      }),
  );

  function resolveDerivedGraph(graph: WorkflowGraph): WorkflowGraph['derived_graph'] {
    return graph.derived_graph ?? buildDerivedGraph(graph);
  }

  function applyWorkflowGraph(graph: WorkflowGraph): WorkflowGraphMaterialization {
    const materialized = materializeWorkflowGraphSnapshot({
      graph,
      definitions: get(params.nodeDefinitions),
      selectedNodeIds: get(params.selectedNodeIds),
    });
    structuralNodes.set(materialized.graphNodes);
    edges.set(materialized.graphEdges);
    derivedGraph.set(resolveDerivedGraph(materialized.graph));
    return materialized;
  }

  function clearRuntimeOverlays(): void {
    nodeRuntimeOverlays.set(new Map());
  }

  function clearGraph(): void {
    structuralNodes.set([]);
    edges.set([]);
    clearRuntimeOverlays();
    derivedGraph.set(
      buildDerivedGraph({
        nodes: [],
        edges: [],
      }),
    );
  }

  function loadDefaultWorkflow(definitions: NodeDefinition[]): void {
    clearRuntimeOverlays();
    const defaultWorkflow = buildDefaultWorkflowGraphState(definitions);
    structuralNodes.set(defaultWorkflow.nodes);
    edges.set(defaultWorkflow.edges);
    derivedGraph.set(resolveDerivedGraph(defaultWorkflow.graph));
  }

  function updateNodeRuntimeData(nodeId: string, data: Record<string, unknown>): void {
    nodeRuntimeOverlays.update((overlays) => updateNodeRuntimeOverlay(overlays, nodeId, data));
  }

  function clearNodeRuntimeData(keys: string[]): void {
    if (keys.length === 0) return;

    nodeRuntimeOverlays.update((overlays) => clearNodeRuntimeOverlayKeys(overlays, keys));
  }

  function appendStreamContent(nodeId: string, chunk: string): void {
    nodeRuntimeOverlays.update((overlays) =>
      appendNodeStreamContentOverlay(overlays, nodeId, chunk),
    );
  }

  function setStreamContent(nodeId: string, content: string): void {
    nodeRuntimeOverlays.update((overlays) =>
      setNodeStreamContentOverlay(overlays, nodeId, content),
    );
  }

  function clearStreamContent(): void {
    nodeRuntimeOverlays.update(clearNodeStreamContentOverlay);
  }

  return {
    applyWorkflowGraph,
    appendStreamContent,
    clearGraph,
    clearNodeRuntimeData,
    clearRuntimeOverlays,
    clearStreamContent,
    derivedGraph,
    edges,
    loadDefaultWorkflow,
    nodes,
    setStreamContent,
    structuralNodes,
    updateNodeRuntimeData,
    workflowGraph,
  };
}
