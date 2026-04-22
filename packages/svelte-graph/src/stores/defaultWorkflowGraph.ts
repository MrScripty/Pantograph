import type { Edge, Node } from '@xyflow/svelte';

import type { NodeDefinition, WorkflowGraph } from '../types/workflow.js';

export interface DefaultWorkflowGraphState {
  nodes: Node[];
  edges: Edge[];
  graph: WorkflowGraph;
}

function findDefaultDefinition(
  definitions: NodeDefinition[],
  nodeType: string,
): NodeDefinition | undefined {
  return definitions.find((definition) => definition.node_type === nodeType);
}

export function buildDefaultWorkflowGraphState(
  definitions: NodeDefinition[],
): DefaultWorkflowGraphState {
  const textInputDef = findDefaultDefinition(definitions, 'text-input');
  const llmDef = findDefaultDefinition(definitions, 'llm-inference');
  const outputDef = findDefaultDefinition(definitions, 'text-output');

  const nodes: Node[] = [
    {
      id: 'user-input',
      type: 'text-input',
      position: { x: 50, y: 150 },
      data: { label: 'User Input', text: '', definition: textInputDef },
    },
    {
      id: 'llm',
      type: 'llm-inference',
      position: { x: 350, y: 150 },
      data: { label: 'LLM Inference', definition: llmDef },
    },
    {
      id: 'output',
      type: 'text-output',
      position: { x: 650, y: 150 },
      data: { label: 'Output', text: '', definition: outputDef },
    },
  ];

  const edges: Edge[] = [
    {
      id: 'input-to-llm',
      source: 'user-input',
      sourceHandle: 'text',
      target: 'llm',
      targetHandle: 'prompt',
    },
    {
      id: 'llm-to-output',
      source: 'llm',
      sourceHandle: 'response',
      target: 'output',
      targetHandle: 'text',
    },
  ];

  const graph: WorkflowGraph = {
    nodes: [
      {
        id: 'user-input',
        node_type: 'text-input',
        position: { x: 50, y: 150 },
        data: { label: 'User Input', text: '', definition: textInputDef },
      },
      {
        id: 'llm',
        node_type: 'llm-inference',
        position: { x: 350, y: 150 },
        data: { label: 'LLM Inference', definition: llmDef },
      },
      {
        id: 'output',
        node_type: 'text-output',
        position: { x: 650, y: 150 },
        data: { label: 'Output', text: '', definition: outputDef },
      },
    ],
    edges: [
      {
        id: 'input-to-llm',
        source: 'user-input',
        source_handle: 'text',
        target: 'llm',
        target_handle: 'prompt',
      },
      {
        id: 'llm-to-output',
        source: 'llm',
        source_handle: 'response',
        target: 'output',
        target_handle: 'text',
      },
    ],
  };

  return { nodes, edges, graph };
}
