import test from 'node:test';
import assert from 'node:assert/strict';

import {
  buildConnectionIntentState,
  edgeToGraphEdge,
  isWorkflowConnectionValid,
} from './workflowConnections.ts';
import type { NodeDefinition, PortDataType } from '../services/workflow/types.ts';

const isSamePortType = (source: PortDataType, target: PortDataType) => source === target;

const sourceDefinition: NodeDefinition = {
  node_type: 'source',
  category: 'input',
  label: 'Source',
  description: 'Source node',
  io_binding_origin: 'client_session',
  execution_mode: 'manual',
  inputs: [],
  outputs: [
    {
      id: 'out',
      label: 'Out',
      data_type: 'number',
      required: false,
      multiple: false,
    },
  ],
};

const targetDefinition: NodeDefinition = {
  node_type: 'target',
  category: 'processing',
  label: 'Target',
  description: 'Target node',
  io_binding_origin: 'client_session',
  execution_mode: 'manual',
  inputs: [
    {
      id: 'number',
      label: 'Number',
      data_type: 'number',
      required: false,
      multiple: false,
    },
    {
      id: 'text',
      label: 'Text',
      data_type: 'string',
      required: false,
      multiple: false,
    },
  ],
  outputs: [],
};

test('edgeToGraphEdge normalizes default edge handles for backend commits', () => {
  assert.deepEqual(
    edgeToGraphEdge({
      id: 'edge-a',
      source: 'source-a',
      target: 'target-a',
    }),
    {
      id: 'edge-a',
      source: 'source-a',
      source_handle: 'output',
      target: 'target-a',
      target_handle: 'input',
    },
  );
});

test('buildConnectionIntentState projects backend candidates into app store state', () => {
  const intent = buildConnectionIntentState({
    graph_revision: 'rev-a',
    revision_matches: true,
    source_anchor: { node_id: 'source-a', port_id: 'out' },
    compatible_nodes: [
      {
        node_id: 'target-a',
        node_type: 'target',
        node_label: 'Target',
        position: { x: 0, y: 0 },
        anchors: [
          {
            port_id: 'number',
            port_label: 'Number',
            data_type: 'number',
            multiple: false,
          },
        ],
      },
    ],
    insertable_node_types: [],
  });

  assert.deepEqual(intent.compatibleNodeIds, ['target-a']);
  assert.deepEqual(intent.compatibleTargetKeys, ['target-a:number']);
  assert.equal(intent.graphRevision, 'rev-a');
});

test('isWorkflowConnectionValid uses active intent when it matches the source anchor', () => {
  const intent = {
    sourceAnchor: { node_id: 'source-a', port_id: 'out' },
    graphRevision: 'rev-a',
    compatibleNodeIds: ['target-a'],
    compatibleTargetKeys: ['target-a:number'],
    insertableNodeTypes: [],
  };

  assert.equal(
    isWorkflowConnectionValid(
      {
        source: 'source-a',
        sourceHandle: 'out',
        target: 'target-a',
        targetHandle: 'number',
      },
      [],
      intent,
      isSamePortType,
    ),
    true,
  );
  assert.equal(
    isWorkflowConnectionValid(
      {
        source: 'source-a',
        sourceHandle: 'out',
        target: 'target-a',
        targetHandle: 'text',
      },
      [],
      intent,
      isSamePortType,
    ),
    false,
  );
});

test('isWorkflowConnectionValid falls back to supplied port compatibility', () => {
  const graphNodes = [
    { id: 'source-a', data: { definition: sourceDefinition } },
    { id: 'target-a', data: { definition: targetDefinition } },
  ];

  assert.equal(
    isWorkflowConnectionValid(
      {
        source: 'source-a',
        sourceHandle: 'out',
        target: 'target-a',
        targetHandle: 'number',
      },
      graphNodes,
      null,
      isSamePortType,
    ),
    true,
  );
  assert.equal(
    isWorkflowConnectionValid(
      {
        source: 'source-a',
        sourceHandle: 'out',
        target: 'target-a',
        targetHandle: 'text',
      },
      graphNodes,
      null,
      isSamePortType,
    ),
    false,
  );
});
