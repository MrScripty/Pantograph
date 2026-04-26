import test from 'node:test';
import assert from 'node:assert/strict';
import type { Edge, Node } from '@xyflow/svelte';

import type { NodeDefinition, WorkflowGraph } from '../types/workflow.ts';
import {
  materializeWorkflowGraphSnapshot,
  projectWorkflowGraphStoreState,
} from './workflowStoreMaterialization.ts';

const textInputDefinition = {
  node_type: 'text-input',
  category: 'input',
  label: 'Text Input',
  description: 'Provides text input',
  io_binding_origin: 'client_session',
  inputs: [],
  outputs: [
    {
      id: 'text',
      label: 'Text',
      data_type: 'string',
      required: false,
      multiple: false,
    },
  ],
  execution_mode: 'manual',
} satisfies NodeDefinition;

test('projectWorkflowGraphStoreState converts SvelteFlow state into workflow graph shape', () => {
  const graph = projectWorkflowGraphStoreState({
    nodes: [
      {
        id: 'node-a',
        type: 'text-input',
        position: { x: 10, y: 20 },
        data: { text: 'hello' },
      },
    ] as Node[],
    edges: [
      {
        id: 'edge-a',
        source: 'node-a',
        sourceHandle: null,
        target: 'node-b',
        targetHandle: undefined,
      },
    ] as Edge[],
    derivedGraph: {
      schema_version: 1,
      graph_fingerprint: 'fingerprint-a',
      consumer_count_map: {},
    },
  });

  assert.deepEqual(graph, {
    nodes: [
      {
        id: 'node-a',
        node_type: 'text-input',
        position: { x: 10, y: 20 },
        data: { text: 'hello' },
      },
    ],
    edges: [
      {
        id: 'edge-a',
        source: 'node-a',
        source_handle: 'output',
        target: 'node-b',
        target_handle: 'input',
      },
    ],
    derived_graph: {
      schema_version: 1,
      graph_fingerprint: 'fingerprint-a',
      consumer_count_map: {},
    },
  });
});

test('projectWorkflowGraphStoreState strips runtime-only display fields', () => {
  const graph = projectWorkflowGraphStoreState({
    nodes: [
      {
        id: 'node-a',
        type: 'text-output',
        position: { x: 10, y: 20 },
        data: {
          label: 'Text Output',
          streamContent: 'partial',
          audio: 'base64-audio',
          audio_mime: 'audio/wav',
        },
      },
    ] as Node[],
    edges: [],
    derivedGraph: undefined,
  });

  assert.deepEqual(graph.nodes[0].data, {
    label: 'Text Output',
  });
});

test('materializeWorkflowGraphSnapshot applies definitions and selected node ids', () => {
  const graph = {
    nodes: [
      {
        id: 'node-a',
        node_type: 'text-input',
        position: { x: 10, y: 20 },
        data: { text: 'hello' },
      },
    ],
    edges: [
      {
        id: 'edge-a',
        source: 'node-a',
        source_handle: 'text',
        target: 'node-b',
        target_handle: 'input',
      },
    ],
  } satisfies WorkflowGraph;

  const materialized = materializeWorkflowGraphSnapshot({
    graph,
    definitions: [textInputDefinition],
    selectedNodeIds: ['node-a'],
  });

  assert.equal(materialized.graph, graph);
  assert.equal(materialized.graphNodes[0].selected, true);
  assert.equal(materialized.graphNodes[0].data.definition, textInputDefinition);
  assert.deepEqual(materialized.graphEdges[0], {
    id: 'edge-a',
    source: 'node-a',
    sourceHandle: 'text',
    target: 'node-b',
    targetHandle: 'input',
  });
});
